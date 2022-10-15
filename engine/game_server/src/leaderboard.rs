// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::liveboard::LiveboardRepo;
use crate::player::PlayerRepo;
use actix::{
    ActorFutureExt, ActorStreamExt, Context as ActorContext, ContextFutureSpawner, Handler,
    WrapFuture, WrapStream,
};
use core_protocol::dto::LeaderboardDto;
use core_protocol::get_unix_time_now;
use core_protocol::id::PeriodId;
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{LeaderboardResponse, LeaderboardUpdate};
use futures::stream::FuturesUnordered;
use log::error;
use server_util::database_schema::{GameIdScoreType, ScoreItem, ScoreType};
use server_util::rate_limiter::RateLimiter;
use std::collections::{BinaryHeap, HashMap};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

/// Manages updating, saving, and loading leaderboards.
pub struct LeaderboardRepo<G: GameArenaService> {
    /// Stores cached leaderboards from database and whether they were changed.
    leaderboards: [(Arc<[LeaderboardDto]>, bool); std::mem::variant_count::<PeriodId>()],
    /// Scores that should be committed to database.
    pending: HashMap<(PlayerAlias, PeriodId), u32>,
    take_pending_rate_limit: RateLimiter,
    read_database_rate_limit: RateLimiter,
    _spooky: PhantomData<G>,
}

impl<G: GameArenaService> LeaderboardRepo<G> {
    pub fn new() -> Self {
        Self {
            leaderboards: [
                (Vec::new().into(), false),
                (Vec::new().into(), false),
                (Vec::new().into(), false),
            ],
            pending: HashMap::new(),
            take_pending_rate_limit: RateLimiter::new(Duration::from_secs(60), 0),
            read_database_rate_limit: RateLimiter::new(Duration::from_secs(110), 0),
            _spooky: PhantomData,
        }
    }

    /// Gets a cached leaderboard.
    pub fn get(&self, period_id: PeriodId) -> &Arc<[LeaderboardDto]> {
        &self.leaderboards[period_id as usize].0
    }

    /// Leaderboard relies on an external source of data, such as a database.
    pub fn put_leaderboard(&mut self, period_id: PeriodId, leaderboard: Arc<[LeaderboardDto]>) {
        if &leaderboard != self.get(period_id) {
            self.leaderboards[period_id as usize] = (leaderboard, true);
        }
    }

    /// Computes minimum score to earn a place on the given leaderboard.
    fn minimum_score(&self, period_id: PeriodId) -> u32 {
        self.get(period_id)
            .get(G::LEADERBOARD_SIZE - 1)
            .map(|dto| dto.score)
            .unwrap_or(0)
    }

    /// Process liveboard scores to potentially be added to the leaderboard.
    pub(crate) fn process(&mut self, liveboard: &LiveboardRepo<G>, players: &PlayerRepo<G>) {
        let liveboard_items = liveboard.get();

        // Must be sorted in reverse.
        debug_assert!(liveboard_items.is_sorted_by_key(|dto| u32::MAX - dto.score));

        if players.real_players_live < G::LEADERBOARD_MIN_PLAYERS {
            return;
        }

        for period_id in PeriodId::iter() {
            let minimum_score = self.minimum_score(period_id);

            for dto in liveboard_items.iter() {
                if dto.score < minimum_score {
                    // Sorted, so this iteration is not going to produce any more sufficient scores.
                    break;
                }

                if let Some(player) = players.borrow_player(dto.player_id) {
                    if player.is_bot() {
                        // Bots are never on the leaderboard, regardless of whether they are on the liveboard.
                        continue;
                    }

                    let alias = player.alias();
                    let entry = self.pending.entry((alias, period_id)).or_insert(0);
                    *entry = dto.score.max(*entry);
                } else {
                    // TODO: Is this legitimately possible?
                    debug_assert!(false, "player from liveboard doesn't exist");
                }
            }
        }
    }

    /// Returns scores pending database commit, draining them in the process. Rate limited.
    pub fn take_pending(&mut self) -> Option<impl Iterator<Item = ScoreItem> + '_> {
        if self.pending.is_empty() || self.take_pending_rate_limit.should_limit_rate() {
            None
        } else {
            let now_seconds = get_unix_time_now() / 1000;

            Some(
                self.pending
                    .drain()
                    .map(move |((alias, period_id), score)| {
                        let score_type = match period_id {
                            PeriodId::AllTime => ScoreType::PlayerAllTime,
                            PeriodId::Daily => ScoreType::PlayerDay,
                            PeriodId::Weekly => ScoreType::PlayerWeek,
                        };

                        ScoreItem {
                            game_id_score_type: GameIdScoreType {
                                game_id: G::GAME_ID,
                                score_type,
                            },
                            alias: alias.to_string(),
                            score,
                            ttl: score_type.period().map(|period| now_seconds + period),
                        }
                    }),
            )
        }
    }

    /// Reads leaderboards from database. Can call frequently, but will only read on a rate limited
    /// basis.
    pub fn update_from_database(
        infrastructure: &mut Infrastructure<G>,
        ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        if infrastructure
            .leaderboard
            .read_database_rate_limit
            .should_limit_rate()
        {
            return;
        }

        for period_id in PeriodId::iter() {
            infrastructure
                .database()
                .read_scores_by_type(GameIdScoreType {
                    game_id: G::GAME_ID,
                    score_type: match period_id {
                        PeriodId::Daily => ScoreType::PlayerDay,
                        PeriodId::Weekly => ScoreType::PlayerWeek,
                        PeriodId::AllTime => ScoreType::PlayerAllTime,
                    },
                })
                .into_actor(infrastructure)
                .map(move |res, act, _| match res {
                    Ok(scores) => {
                        let heap: BinaryHeap<LeaderboardDto> = scores
                            .into_iter()
                            .map(|score| LeaderboardDto {
                                alias: PlayerAlias::new_sanitized(score.alias.as_str()),
                                score: score.score,
                            })
                            .collect();

                        let leaderboard =
                            heap.into_iter_sorted().take(G::LEADERBOARD_SIZE).collect();

                        act.leaderboard.put_leaderboard(period_id, leaderboard)
                    }
                    Err(e) => {
                        error!("error reading leaderboard scores: {:?}", e);
                    }
                })
                .spawn(ctx);
        }
    }

    pub fn update_to_database(
        infrastructure: &mut Infrastructure<G>,
        ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        let database = infrastructure.database();
        let stream: Option<FuturesUnordered<_>> =
            infrastructure
                .leaderboard
                .take_pending()
                .map(|pending_scores| {
                    pending_scores
                        .map(|pending_score| database.update_score(pending_score))
                        .collect()
                });
        if let Some(stream) = stream {
            stream
                .into_actor(infrastructure)
                .map(|res, _act, _| {
                    if let Err(e) = res {
                        error!("error putting leaderboard score: {:?}", e);
                    }
                })
                .finish()
                .spawn(ctx);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (PeriodId, &Arc<[LeaderboardDto]>)> {
        self.leaderboards
            .iter()
            .enumerate()
            .map(|(i, (leaderboard, _))| (PeriodId::from(i), leaderboard))
    }

    /// Reads off changed leaderboards, *without* the changed flag in the process.
    pub fn deltas_nondestructive(
        &self,
    ) -> impl Iterator<Item = (PeriodId, &Arc<[LeaderboardDto]>)> {
        self.leaderboards
            .iter()
            .enumerate()
            .filter_map(|(i, (leaderboard, changed))| {
                if *changed {
                    Some((PeriodId::from(i), leaderboard))
                } else {
                    None
                }
            })
    }

    /// Reads off changed leaderboards, clearing the changed flag in the process.
    /// Don't use this when there are multiple arenas!
    /*
    pub fn delta_destructive(&mut self) -> impl Iterator<Item = (PeriodId, &Arc<[LeaderboardDto]>)> {
        self.leaderboards
            .iter_mut()
            .enumerate()
            .filter_map(|(i, (leaderboard, changed))| {
                if *changed {
                    *changed = false;
                    Some((PeriodId::from(i), &*leaderboard))
                } else {
                    None
                }
            })
    }
     */

    /// Clear all the delta flags (such as if clients have been updated).
    pub fn clear_deltas(&mut self) {
        for (_, changed) in self.leaderboards.iter_mut() {
            *changed = false;
        }
    }

    /// Gets leaderboard for new players.
    pub fn initializers(&self) -> impl Iterator<Item = LeaderboardUpdate> + '_ {
        self.iter().filter_map(|(period_id, leaderboard)| {
            if leaderboard.is_empty() {
                None
            } else {
                Some(LeaderboardUpdate::Updated(
                    period_id,
                    Arc::clone(leaderboard),
                ))
            }
        })
    }
}

/// Asks the server if it and the underlying hardware and OS are healthy.
#[derive(actix::Message)]
#[rtype(result = "LeaderboardResponse")]
pub struct LeaderboardRequest;

/// Reports whether infrastructure is healthy (hardware and actor are running properly).
impl<G: GameArenaService> Handler<LeaderboardRequest> for Infrastructure<G> {
    type Result = LeaderboardResponse;

    fn handle(&mut self, _request: LeaderboardRequest, _: &mut Self::Context) -> Self::Result {
        let local_players = self.context_service.context.players.real_players_live as u32;

        LeaderboardResponse {
            leaderboard: Arc::clone(self.leaderboard.get(PeriodId::AllTime)),
            players: self
                .system
                .as_ref()
                .map(|s| {
                    s.servers
                        .iter()
                        .map(|(_, d)| d.status.player_count().unwrap_or_default())
                        .sum::<u32>()
                })
                .unwrap_or_default()
                .max(local_players),
        }
    }
}
