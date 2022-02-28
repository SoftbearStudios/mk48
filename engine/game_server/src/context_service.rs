// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::context::Context;
use crate::game_service::GameArenaService;
use crate::invitation::InvitationRepo;
use crate::leaderboard::LeaderboardRepo;
use crate::metric::MetricRepo;
use common_util::ticks::Ticks;
use core_protocol::dto::ServerDto;
use core_protocol::id::{ArenaId, ServerId};
use std::sync::Arc;

/// Contains a [`GameArenaService`] and the corresponding [`Context`].
pub struct ContextService<G: GameArenaService> {
    pub context: Context<G>,
    pub service: G,
}

impl<G: GameArenaService> ContextService<G> {
    pub fn new(
        arena_id: ArenaId,
        min_players: usize,
        chat_log: Option<String>,
        trace_log: Option<String>,
    ) -> Self {
        Self {
            context: Context::new(arena_id, min_players, chat_log, trace_log),
            service: G::new(min_players),
        }
    }

    pub(crate) fn update(
        &mut self,
        leaderboard: &mut LeaderboardRepo<G>,
        invitations: &mut InvitationRepo<G>,
        metrics: &mut MetricRepo<G>,
        server_delta: Option<(Arc<[ServerDto]>, Arc<[ServerId]>)>,
    ) {
        // Keep track of time.
        self.context.count_tick();

        // Spawn/de-spawn clients and bots.
        self.context.clients.prune(
            &mut self.service,
            &mut self.context.players,
            &mut self.context.teams,
            invitations,
            metrics,
        );
        self.context
            .bots
            .update_count(&mut self.service, &mut self.context.players);

        // Update game logic.
        self.service.update(Ticks::ONE, self.context.counter);
        self.context
            .players
            .update_is_alive(&self.service, &mut self.context.teams, metrics);

        // Update clients and bots.
        self.context.clients.update(
            &self.service,
            &mut self.context.players,
            &mut self.context.teams,
            &mut self.context.liveboard,
            leaderboard,
            server_delta,
            self.context.counter,
        );
        self.context
            .bots
            .update(self.context.counter, &mut self.service);

        leaderboard.process(&self.context.liveboard, &self.context.players);

        // Post-update game logic.
        self.service.post_update();
    }
}
