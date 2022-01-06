// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::client::ClientState;
use crate::database::{Database, MetricsItem, Score, ScoreType, SessionItem};
use crate::repo::*;
use crate::server::ServerState;
use actix::prelude::*;
use actix::Recipient;
use core_protocol::dto::LeaderboardDto;
use core_protocol::id::PeriodId;
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{ClientUpdate, ServerUpdate};
use core_protocol::*;
use futures::stream::futures_unordered::FuturesUnordered;
use log::{error, info, warn};
use server_util::observer::*;
use std::collections::hash_map::HashMap;
use std::lazy::OnceCell;
use std::process;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use std::time::Duration;

const DB_SESSION_TIMER_SECS: u64 = 60;
const DB_METRICS_MILLIS: u64 = 60 * 60 * 1000;

/// Putting these in an actor is very tricky. They won't have a static lifetime, and that makes
/// it hard to use async functions.
static mut DATABASE: OnceCell<Database> = OnceCell::new();

pub struct Core {
    /// Optional file to append chat logs as CSV.
    pub chat_log: Option<String>,
    /// Inhibits writing to the database.
    pub database_read_only: bool,
    pub clients: HashMap<Recipient<ObserverUpdate<ClientUpdate>>, ClientState>,
    /// The stop time of the last metrics saved to database, used to avoid double-counting.
    pub metrics_stop: Option<UnixTime>,
    pub repo: Repo,
    pub servers: HashMap<Recipient<ObserverUpdate<ServerUpdate>>, ServerState>,
    /// Control HTTP redirection at the highest level (affects both core and bundled game server).
    pub redirect_server_id: Option<&'static AtomicU8>,
}

impl Core {
    /// Creates a new core, with various options. Unsafe to call more than once.
    pub async fn new(
        chat_log: Option<String>,
        database_read_only: bool,
        redirect_server_id: Option<&'static AtomicU8>,
    ) -> Self {
        // SAFETY: Only happens once.
        unsafe {
            let _ = DATABASE.set(Database::new().await);
        }

        Self {
            chat_log,
            database_read_only,
            clients: HashMap::new(),
            metrics_stop: None,
            repo: Repo::new(),
            servers: HashMap::new(),
            redirect_server_id,
        }
    }

    /// Returns a static reference to the database singleton.
    pub fn database() -> &'static Database {
        // SAFETY: Only initialized once, then immutable.
        unsafe { DATABASE.get().unwrap() }
    }
}

impl Actor for Core {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Core started");

        ctx.set_mailbox_capacity(256);
        self.start_timers(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        error!("Core stopped");

        // A process without this actor running should be restarted immediately.
        process::exit(1);
    }
}

impl Core {
    fn start_timers(&self, ctx: &mut <Self as Actor>::Context) {
        self.start_admin_timers(ctx);
        self.start_db_timers(ctx);
        self.start_client_timers(ctx);
        self.start_server_timers(ctx);
    }

    fn start_db_timers(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_secs(DB_SESSION_TIMER_SECS), |act, ctx| {
            // Update leaderboard with database.
            {
                let stream: FuturesUnordered<_> = act
                    .repo
                    .get_liveboards(false)
                    .into_iter()
                    .map(|(arena_id, game_id, liveboard, leaderboard_worthy)| {
                        let mut player_scores: Vec<Score> = liveboard
                            .into_iter()
                            .filter_map(|item| {
                                if let Some(name) =
                                    act.repo.player_id_to_name(arena_id, item.player_id)
                                {
                                    Some(Score {
                                        alias: name.to_string(),
                                        score: item.score,
                                    })
                                } else {
                                    warn!("Missing name in leaderboard");
                                    None
                                }
                            })
                            .collect();

                        if !leaderboard_worthy || act.database_read_only {
                            // Don't actually update any scores (but still read leaderboard).
                            warn!("Would have written to leaderboard database, but was inhibited");
                            player_scores.clear();
                        }

                        async move {
                            (
                                arena_id,
                                Core::database()
                                    .update_leaderboard(game_id, player_scores)
                                    .await,
                            )
                        }
                    })
                    .collect();

                stream
                    .into_actor(act)
                    .map(|(arena_id, result), act, _| {
                        match result {
                            Ok(leaderboard) => {
                                for (score_type, scores) in leaderboard.into_iter() {
                                    let period = match score_type {
                                        ScoreType::PlayerDay => PeriodId::Daily,
                                        ScoreType::PlayerWeek => PeriodId::Weekly,
                                        ScoreType::PlayerAllTime => PeriodId::AllTime,
                                        _ => continue, // never happens
                                    };

                                    let arc: Arc<[_]> = scores
                                        .into_iter()
                                        .map(|score| LeaderboardDto {
                                            alias: PlayerAlias::new(&score.alias),
                                            score: score.score,
                                        })
                                        .collect();

                                    act.repo.put_leaderboard(arena_id, arc, period)
                                }
                            }
                            Err(e) => error!("Error putting leaderboard: {:?}", e),
                        }
                    })
                    .finish()
                    .spawn(ctx);
            }

            // Put sessions to database.
            if act.database_read_only {
                warn!("Would have written to sessions database, but was inhibited");
            } else {
                let stream = FuturesUnordered::new();

                for (arena_id, session_id, session) in act
                    .repo
                    .iter_recently_modified_sessions(DB_SESSION_TIMER_SECS * 1000)
                {
                    if let Some(server_id) = session.server_id {
                        stream.push(Core::database().put_session(SessionItem {
                            alias: session.alias,
                            arena_id,
                            date_created: session.date_created,
                            date_previous: session.date_previous,
                            date_renewed: session.date_renewed,
                            date_terminated: session.date_terminated,
                            game_id: session.game_id,
                            player_id: session.player_id,
                            plays: session.previous_plays + session.plays.len() as u32,
                            previous_id: session.previous_id,
                            user_agent_id: session.user_agent_id,
                            referrer: session.referrer,
                            server_id,
                            session_id,
                        }));
                    }
                }

                stream
                    .into_actor(act)
                    .map(|res, _, _| {
                        if let Err(e) = res {
                            error!("error putting session: {:?}", e)
                        }
                    })
                    .finish()
                    .spawn(ctx);
            }
        });

        ctx.run_interval(Duration::from_millis(DB_METRICS_MILLIS), |act, ctx| {
            if act.database_read_only {
                warn!("Would have written to metrics database, but was inhibited");
            } else {
                let stream = FuturesUnordered::new();
                let metrics_stop = get_unix_time_now();
                let metrics_start = act.metrics_stop.unwrap_or(metrics_stop - DB_METRICS_MILLIS);
                act.metrics_stop = Some(metrics_stop);

                for (game_id, _) in act.repo.get_game_ids().iter() {
                    if let Some(metrics) = act.repo.get_metrics(
                        game_id,
                        Some(metrics_start),
                        Some(metrics_stop),
                        &|_| true,
                    ) {
                        stream.push(Core::database().update_metrics(MetricsItem {
                            game_id: *game_id,
                            timestamp: (get_unix_time_now() / DB_METRICS_MILLIS)
                                * DB_METRICS_MILLIS,
                            metrics,
                        }))
                    }
                }

                stream
                    .into_actor(act)
                    .map(|res, _, _| {
                        if let Err(e) = res {
                            error!("error putting metrics: {:?}", e)
                        }
                    })
                    .finish()
                    .spawn(ctx);
            }
        });
    }
}
