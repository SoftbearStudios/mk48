// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::admin::AdminRepo;
use crate::client::ClientRepo;
use crate::context_service::ContextService;
use crate::game_service::GameArenaService;
use crate::invitation::InvitationRepo;
use crate::leaderboard::LeaderboardRepo;
use crate::metric::MetricRepo;
use crate::status::StatusRepo;
use crate::system::SystemRepo;
use actix::AsyncContext;
use actix::{Actor, Context as ActorContext};
use common_util::ticks::Ticks;
use core_protocol::id::{ArenaId, RegionId, ServerId};
use log::{error, info};
use server_util::benchmark::{benchmark_scope, Timer};
use server_util::database::Database;
use server_util::rate_limiter::RateLimiterProps;
use std::num::NonZeroU32;
use std::process;

/// An entire game server.
pub struct Infrastructure<G: GameArenaService> {
    /// What server/region does this infrastructure represent?
    pub(crate) server_id: Option<ServerId>,
    pub(crate) region_id: Option<RegionId>,

    /// API.
    pub(crate) database: &'static Database,
    pub(crate) system: Option<SystemRepo<G>>,

    /// Game specific stuff. In the future, there could be multiple of these.
    pub(crate) context_service: ContextService<G>,

    /// Shared invitations.
    pub(crate) invitations: InvitationRepo<G>,
    /// Shared admin interface.
    pub(crate) admin: AdminRepo<G>,
    /// Shared leaderboard.
    pub(crate) leaderboard: LeaderboardRepo<G>,
    /// Shared metrics.
    pub(crate) metrics: MetricRepo<G>,

    /// Monitoring.
    pub(crate) status: StatusRepo,
}

impl<G: GameArenaService> Actor for Infrastructure<G> {
    type Context = ActorContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("infrastructure started");

        // TODO: Investigate whether this only affects performance or can affect correctness.
        ctx.set_mailbox_capacity(50);

        ctx.run_interval(Ticks::ONE.to_duration(), Self::update);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        error!("infrastructure stopped");

        // A process without this actor running should be restarted immediately.
        process::exit(1);
    }
}

impl<G: GameArenaService> Infrastructure<G> {
    /// new returns a game server with the specified parameters.
    pub async fn new(
        server_id: Option<ServerId>,
        system: Option<SystemRepo<G>>,
        client_hash: u64,
        region_id: Option<RegionId>,
        database_read_only: bool,
        min_players: usize,
        chat_log: Option<String>,
        trace_log: Option<String>,
        client_authenticate: RateLimiterProps,
    ) -> Self {
        // TODO: If multiple arenas, generate randomly.
        let arena_id = ArenaId(
            NonZeroU32::new(server_id.map(|s| s.0.get()).unwrap_or(0) as u32 + 2000).unwrap(),
        );

        Self {
            server_id,
            region_id,
            /// Leak the box, because static lifetime facilitates async code. This will probably
            /// only ever happen once, and it will last for the lifetime of the program.
            database: Box::leak(Box::new(Database::new(database_read_only).await)),
            system,
            admin: AdminRepo::new(),
            context_service: ContextService::new(
                arena_id,
                min_players,
                chat_log,
                trace_log,
                client_authenticate,
            ),
            invitations: InvitationRepo::new(),
            leaderboard: LeaderboardRepo::new(),
            metrics: MetricRepo::new(),
            status: StatusRepo::new(client_hash),
        }
    }

    /// Call once every tick.
    pub fn update(&mut self, ctx: &mut <Infrastructure<G> as Actor>::Context) {
        benchmark_scope!("infrastructure_update");

        let status = &self.status;
        let server_delta = self.system.as_mut().and_then(|system| system.delta(status));
        self.context_service.update(
            &mut self.leaderboard,
            &mut self.invitations,
            &mut self.metrics,
            self.server_id,
            server_delta,
        );
        self.leaderboard.clear_deltas();
        self.status.health.update_ups();

        // These are all rate-limited internally.
        LeaderboardRepo::update_to_database(self, ctx);
        LeaderboardRepo::update_from_database(self, ctx);
        MetricRepo::update_to_database(self, ctx);
        ClientRepo::update_to_database(self, ctx);
        SystemRepo::update(self, ctx);
    }

    /// Returns a static reference to the database singleton.
    pub fn database(&self) -> &'static Database {
        self.database
    }
}
