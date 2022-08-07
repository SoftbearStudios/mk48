// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::BotRepo;
use crate::context::Context;
use crate::game_service::GameArenaService;
use crate::invitation::InvitationRepo;
use crate::leaderboard::LeaderboardRepo;
use crate::metric::MetricRepo;
use core_protocol::dto::ServerDto;
use core_protocol::id::{ArenaId, ServerId};
use server_util::rate_limiter::RateLimiterProps;
use std::sync::Arc;

/// Contains a [`GameArenaService`] and the corresponding [`Context`].
pub struct ContextService<G: GameArenaService> {
    pub context: Context<G>,
    pub service: G,
}

impl<G: GameArenaService> ContextService<G> {
    pub fn new(
        arena_id: ArenaId,
        min_bots: Option<usize>,
        max_bots: Option<usize>,
        bot_percent: Option<usize>,
        chat_log: Option<String>,
        trace_log: Option<String>,
        client_authenticate: RateLimiterProps,
    ) -> Self {
        let bots = BotRepo::new_from_options(min_bots, max_bots, bot_percent);

        Self {
            service: G::new(bots.min_bots),
            context: Context::new(arena_id, bots, chat_log, trace_log, client_authenticate),
        }
    }

    pub(crate) fn update(
        &mut self,
        leaderboard: &mut LeaderboardRepo<G>,
        invitations: &mut InvitationRepo<G>,
        metrics: &mut MetricRepo<G>,
        server_id: Option<ServerId>,
        server_delta: Option<(Arc<[ServerDto]>, Arc<[ServerId]>)>,
    ) {
        // Spawn/de-spawn clients and bots.
        self.context.clients.prune(
            &mut self.service,
            &mut self.context.players,
            &mut self.context.teams,
            invitations,
            metrics,
            server_id,
            self.context.arena_id,
        );
        self.context
            .bots
            .update_count(&mut self.service, &mut self.context.players);

        // Update game logic.
        self.service.tick(&mut self.context);
        self.context.players.update_is_alive_and_team_id(
            &mut self.service,
            &mut self.context.teams,
            metrics,
        );

        // Update clients and bots.
        self.context.clients.update(
            &self.service,
            &mut self.context.players,
            &mut self.context.teams,
            &mut self.context.liveboard,
            leaderboard,
            server_delta,
        );
        self.context
            .bots
            .update(&self.service, &self.context.players);

        leaderboard.process(&self.context.liveboard, &self.context.players);

        // Post-update game logic.
        self.service.post_update(&mut self.context);

        // Bot commands/joining/leaving, postponed because no commands should be issued between
        // `GameService::tick` and `GameService::post_update`.
        self.context
            .bots
            .post_update(&mut self.service, &self.context.players);
    }
}
