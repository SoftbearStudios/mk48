// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::admin::AdminRepo;
use crate::client::ClientRepo;
use crate::context_service::ContextService;
use crate::discord::{DiscordBotRepo, DiscordOauth2Repo};
use crate::game_service::GameArenaService;
use crate::invitation::InvitationRepo;
use crate::leaderboard::LeaderboardRepo;
use crate::metric::MetricRepo;
use crate::status::StatusRepo;
use crate::system::SystemRepo;
use actix::AsyncContext;
use actix::{Actor, Context as ActorContext};
use core_protocol::id::{ArenaId, RegionId, ServerId};
use log::{error, info};
use minicdn::MiniCdn;
use server_util::database::Database;
use server_util::rate_limiter::RateLimiterProps;
use std::num::NonZeroU32;
use std::process;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// An entire game server.
pub struct Infrastructure<G: GameArenaService> {
    /// What server/region does this infrastructure represent?
    pub(crate) server_id: Option<ServerId>,
    pub(crate) region_id: Option<RegionId>,

    /// API.
    pub(crate) database: &'static Database,
    pub(crate) system: Option<SystemRepo<G>>,
    pub(crate) discord_bot: Option<&'static DiscordBotRepo>,
    pub(crate) discord_oauth2: Option<&'static DiscordOauth2Repo>,

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

    /// Drop missed updates.
    last_update: Instant,
}

impl<G: GameArenaService> Actor for Infrastructure<G> {
    type Context = ActorContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("infrastructure started");

        // TODO: Investigate whether this only affects performance or can affect correctness.
        ctx.set_mailbox_capacity(50);

        ctx.run_interval(Duration::from_secs_f32(G::TICK_PERIOD_SECS), Self::update);
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
        discord_bot: Option<DiscordBotRepo>,
        discord_oauth2: Option<&'static DiscordOauth2Repo>,
        client_hash: u64,
        region_id: Option<RegionId>,
        database_read_only: bool,
        min_bots: Option<usize>,
        max_bots: Option<usize>,
        bot_percent: Option<usize>,
        chat_log: Option<String>,
        trace_log: Option<String>,
        game_client: Arc<RwLock<MiniCdn>>,
        allow_web_socket_json: &'static AtomicBool,
        admin_config_file: Option<String>,
        client_authenticate: RateLimiterProps,
    ) -> Self {
        // TODO: If multiple arenas, generate randomly.
        let arena_id = ArenaId(
            NonZeroU32::new(server_id.map(|s| s.0.get()).unwrap_or(0) as u32 + 2000).unwrap(),
        );

        Self {
            server_id,
            region_id,
            /// Leak the boxes, because static lifetime facilitates async code. This will probably
            /// only ever happen once, and it will last for the lifetime of the program.
            database: Box::leak(Box::new(Database::new(database_read_only).await)),
            system,
            discord_bot: discord_bot.map(|b| &*Box::leak(Box::new(b))),
            discord_oauth2,
            admin: AdminRepo::new(game_client, admin_config_file, allow_web_socket_json),
            context_service: ContextService::new(
                arena_id,
                min_bots,
                max_bots,
                bot_percent,
                chat_log,
                trace_log,
                client_authenticate,
            ),
            invitations: InvitationRepo::new(),
            leaderboard: LeaderboardRepo::new(),
            metrics: MetricRepo::new(),
            status: StatusRepo::new(client_hash),
            last_update: Instant::now(),
        }
    }

    /// Call once every tick.
    pub fn update(&mut self, ctx: &mut <Infrastructure<G> as Actor>::Context) {
        let now = Instant::now();
        if now.duration_since(self.last_update) < Duration::from_secs_f32(G::TICK_PERIOD_SECS * 0.5)
        {
            // Less than half a tick elapsed. Drop this update on the floor, to avoid jerking.
            return;
        }
        self.last_update = now;

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
        self.status.health.record_tick(G::TICK_PERIOD_SECS);

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
