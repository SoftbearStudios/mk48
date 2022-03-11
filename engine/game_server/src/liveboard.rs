// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::player::PlayerRepo;
use crate::team::TeamRepo;
use crate::util::diff_small_n;
use core_protocol::dto::LiveboardDto;
use core_protocol::id::PlayerId;
use core_protocol::rpc::LiveboardUpdate;
use server_util::rate_limiter::RateLimiter;
use std::collections::BinaryHeap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

/// Manages the live leaderboard of an arena.
pub struct LiveboardRepo<G: GameArenaService> {
    /// Stores previous liveboard for diffing.
    previous: Arc<[LiveboardDto]>,
    update_rate_limiter: RateLimiter,
    _spooky: PhantomData<G>,
}

impl<G: GameArenaService> LiveboardRepo<G> {
    pub fn new() -> Self {
        Self {
            previous: Vec::new().into(),
            update_rate_limiter: RateLimiter::new(Duration::from_secs(1), 0),
            _spooky: PhantomData,
        }
    }

    /// Compute the current liveboard.
    fn compute(players: &PlayerRepo<G>, teams: &TeamRepo<G>) -> Vec<LiveboardDto> {
        // Note: Binary heap is a max heap.
        let mut liveboard = BinaryHeap::new();

        liveboard.extend(players.iter_borrow().filter_map(|player| {
            if !player.is_alive() {
                return None;
            }

            if !G::LIVEBOARD_BOTS && player.is_bot() {
                return None;
            }

            let team = player.team_id().and_then(|t| teams.get(t));

            debug_assert_eq!(player.team_id().is_some(), team.is_some());

            Some(LiveboardDto {
                team_captain: team
                    .map(|t| t.is_captain(player.player_id))
                    .unwrap_or(false),
                team_id: player.team_id(),
                player_id: player.player_id,
                score: player.score,
            })
        }));

        liveboard
            .into_iter_sorted()
            .take(G::LEADERBOARD_SIZE)
            .collect()
    }

    /// Gets the "current" liveboard without recalculation (or diffing).
    pub fn get(&self) -> &Arc<[LiveboardDto]> {
        &self.previous
    }

    /// Gets initializer for new client.
    pub fn initializer(&self) -> LiveboardUpdate {
        LiveboardUpdate::Updated {
            added: Arc::clone(&self.previous),
            removed: Vec::new().into(),
        }
    }

    /// Recalculates liveboard and generates a diff.
    pub fn delta(
        &mut self,
        players: &PlayerRepo<G>,
        teams: &TeamRepo<G>,
    ) -> Option<(Arc<[LiveboardDto]>, Arc<[PlayerId]>)> {
        if self.update_rate_limiter.should_limit_rate() {
            return None;
        }

        let current_liveboard = Self::compute(players, teams);

        if let Some((added, removed)) =
            diff_small_n(&self.previous, &current_liveboard, |dto| dto.player_id)
        {
            self.previous = current_liveboard.into();
            Some((added.into(), removed.into()))
        } else {
            None
        }
    }
}
