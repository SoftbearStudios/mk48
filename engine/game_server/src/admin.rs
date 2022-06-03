// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::client::ClientRepo;
use crate::context::Context;
use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::metric::{MetricBundle, MetricRepo};
use crate::player::PlayerRepo;
use crate::system::{ServerStatus, SystemRepo};
use actix::{fut, ActorFutureExt, Handler, Message, ResponseActFuture, WrapFuture};
use core_protocol::dto::{
    AdminPlayerDto, AdminServerDto, MessageDto, MetricFilterDto, MetricsDataPointDto,
};
use core_protocol::id::{PlayerId, ServerId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{AdminRequest, AdminUpdate};
use core_protocol::{get_unix_time_now, UnixTime};
use log::warn;
use serde::Deserialize;
use server_util::database_schema::Metrics;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Responsible for the admin interface.
pub struct AdminRepo<G: GameArenaService> {
    allow_web_socket_json: &'static AtomicBool,
    pub(crate) redirect_server_id_preference: Option<ServerId>,
    /// Route players to other available servers.
    pub(crate) distribute_load: bool,
    #[cfg(unix)]
    profile: Option<pprof::ProfilerGuard<'static>>,
    _spooky: PhantomData<G>,
}

/// An authenticated question to the [`AdminRepo`].
#[derive(Message, Deserialize)]
#[rtype(result = "Result<AdminUpdate, &'static str>")]
pub struct ParameterizedAdminRequest {
    pub auth: String,
    pub request: AdminRequest,
}

impl ParameterizedAdminRequest {
    fn is_authentic(&self) -> bool {
        const AUTH: &'static [u8] = include_bytes!("auth.txt");

        // Avoid timing side channel attack that could be used to get the auth.
        constant_time_eq::constant_time_eq(self.auth.as_bytes(), AUTH)
    }
}

impl<G: GameArenaService> AdminRepo<G> {
    pub fn new(allow_web_socket_json: &'static AtomicBool) -> Self {
        Self {
            allow_web_socket_json,
            redirect_server_id_preference: None,
            distribute_load: false,
            #[cfg(unix)]
            profile: None,
            _spooky: PhantomData,
        }
    }

    /// Get list of games hosted by the server.
    fn request_games(&self) -> Result<AdminUpdate, &'static str> {
        // We only support one game type per server.
        Ok(AdminUpdate::GamesRequested(
            vec![(G::GAME_ID, 1.0)].into_boxed_slice(),
        ))
    }

    /// Get admin view of real players in the game.
    fn request_players(&self, players: &PlayerRepo<G>) -> Result<AdminUpdate, &'static str> {
        Ok(AdminUpdate::PlayersRequested(
            players
                .iter_borrow()
                .filter_map(|player| {
                    if let Some(client) = player.client().filter(|_| !player.is_out_of_game()) {
                        Some(AdminPlayerDto {
                            alias: client.alias,
                            player_id: player.player_id,
                            team_id: player.team_id(),
                            region_id: client.metrics.region_id,
                            score: player.score,
                            plays: client.metrics.plays,
                            fps: client.metrics.fps,
                            rtt: client.metrics.rtt,
                            messages: client.chat.context.total(),
                            inappropriate_messages: client.chat.context.total_inappropriate(),
                            abuse_reports: client.chat.context.reports(),
                            mute: seconds_ceil(client.chat.context.muted_for()),
                            restriction: seconds_ceil(client.chat.context.restricted_for()),
                        })
                    } else {
                        None
                    }
                })
                .collect(),
        ))
    }

    /// (Temporarily) overrides the alias of a given real player.
    fn override_player_alias(
        &self,
        player_id: PlayerId,
        alias: PlayerAlias,
        players: &PlayerRepo<G>,
    ) -> Result<AdminUpdate, &'static str> {
        let mut player = players
            .borrow_player_mut(player_id)
            .ok_or("nonexistent player")?;
        let client = player.client_mut().ok_or("not a real player")?;
        // We still censor, in case of unauthorized admin access.
        let censored = PlayerAlias::new_sanitized(alias.as_str());
        client.alias = censored;
        Ok(AdminUpdate::PlayerAliasOverridden(censored))
    }

    /// Mutes a given real player for a configurable amount of minutes (0 means disable mute).
    fn mute_player(
        &self,
        player_id: PlayerId,
        minutes: usize,
        players: &PlayerRepo<G>,
    ) -> Result<AdminUpdate, &'static str> {
        let mut player = players
            .borrow_player_mut(player_id)
            .ok_or("nonexistent player")?;
        let client = player.client_mut().ok_or("not a real player")?;
        client
            .chat
            .context
            .mute_for(Duration::from_secs(minutes as u64 * 60));
        Ok(AdminUpdate::PlayerMuted(seconds_ceil(
            client.chat.context.muted_for(),
        )))
    }

    /// Restrict a given real player's chat to safe phrases for a configurable amount of minutes
    /// (0 means disable restriction).
    fn restrict_player(
        &self,
        player_id: PlayerId,
        minutes: usize,
        players: &PlayerRepo<G>,
    ) -> Result<AdminUpdate, &'static str> {
        let mut player = players
            .borrow_player_mut(player_id)
            .ok_or("nonexistent player")?;
        let client = player.client_mut().ok_or("not a real player")?;
        client
            .chat
            .context
            .restrict_for(Duration::from_secs(minutes as u64 * 60));
        Ok(AdminUpdate::PlayerRestricted(seconds_ceil(
            client.chat.context.restricted_for(),
        )))
    }

    /// Get list of all known servers for the game, including incompatible/unreachable/etc. servers.
    fn request_servers(system: &Option<SystemRepo<G>>) -> Result<AdminUpdate, &'static str> {
        let system = system.as_ref().ok_or("system not configured")?;

        let mut servers = system
            .servers
            .iter()
            .map(|(&server_id, server)| {
                let (redirect_server_id, client_hash, player_count) = match &server.status {
                    &ServerStatus::Healthy {
                        redirect_server_id,
                        client_hash,
                        player_count,
                        ..
                    }
                    | &ServerStatus::Unhealthy {
                        redirect_server_id,
                        client_hash,
                        player_count,
                        ..
                    } => (redirect_server_id, client_hash, player_count),
                    _ => (None, None, None),
                };

                AdminServerDto {
                    server_id,
                    redirect_server_id,
                    region_id: server.region_id,
                    ip: server.ip,
                    home: server.home,
                    rtt: server.rtt.as_millis().min(u16::MAX as u128) as u16,
                    reachable: !matches!(&server.status, ServerStatus::Unreachable { .. }),
                    healthy: matches!(server.status, ServerStatus::Healthy { .. }),
                    client_hash,
                    player_count,
                }
            })
            .collect::<Vec<_>>();

        servers.sort_unstable();

        Ok(AdminUpdate::ServersRequested(servers.into()))
    }

    /// Request summary of metrics for the current calendar calendar hour.
    fn request_summary(
        infrastructure: &mut Infrastructure<G>,
        filter: Option<MetricFilterDto>,
    ) -> Result<AdminUpdate, &'static str> {
        let current = MetricRepo::get_metrics(infrastructure, filter);

        // One hour.
        // MetricRepo::get_metrics(infrastructure, filter).summarize(),
        let mut summary = infrastructure
            .metrics
            .history
            .oldest_ordered()
            .map(|bundle: &MetricBundle| bundle.metric(filter))
            .chain(std::iter::once(current.clone()))
            .sum::<Metrics>()
            .summarize();

        // TODO: Make special [`DiscreteMetric`] that handles data that is not necessarily unique.
        summary.arenas_cached.total = current.arenas_cached.total;
        summary.invitations_cached.total = current.invitations_cached.total;
        summary.players_cached.total = current.players_cached.total;
        summary.sessions_cached.total = current.sessions_cached.total;

        Ok(AdminUpdate::SummaryRequested(summary))
    }

    /// Request metric data points for the last 24 calendar hours (excluding the current hour, in
    /// which metrics are incomplete).
    fn request_day(
        metrics: &MetricRepo<G>,
        filter: Option<MetricFilterDto>,
    ) -> Result<AdminUpdate, &'static str> {
        Ok(AdminUpdate::DayRequested(
            metrics
                .history
                .oldest_ordered()
                .map(|bundle| (bundle.start, bundle.data_point(filter)))
                .collect(),
        ))
    }

    /// Request a list of referrers, sorted by percentage, and truncated to a reasonable limit.
    fn request_referrers(&self, players: &PlayerRepo<G>) -> Result<AdminUpdate, &'static str> {
        let mut referrers =
            ClientRepo::filter_map_reduce(players, |client_data| client_data.metrics.referrer);
        referrers.truncate(20);
        Ok(AdminUpdate::ReferrersRequested(
            referrers.into_boxed_slice(),
        ))
    }

    /// Request a list of user agents, sorted by percentage.
    fn request_user_agents(&self, players: &PlayerRepo<G>) -> Result<AdminUpdate, &'static str> {
        let user_agents =
            ClientRepo::filter_map_reduce(players, |client_data| client_data.metrics.user_agent_id);
        Ok(AdminUpdate::UserAgentsRequested(
            user_agents.into_boxed_slice(),
        ))
    }

    /// Send a chat to all players on the server, or a specific player (in which case, will send a
    /// whisper message).
    fn send_chat(
        &self,
        player_id: Option<PlayerId>,
        alias: PlayerAlias,
        message: String,
        context: &mut Context<G>,
    ) -> Result<AdminUpdate, &'static str> {
        context.chat.log_chat(alias, &message, false, "ok");

        let message = MessageDto {
            alias,
            date_sent: get_unix_time_now(),
            player_id: None,
            team_captain: false,
            team_name: None,
            text: message,
            whisper: player_id.is_some(),
        };

        if let Some(player_id) = player_id {
            let mut player = context
                .players
                .borrow_player_mut(player_id)
                .ok_or("nonexistent player")?;
            let client = player.client_mut().ok_or("not a real player")?;
            client.chat.receive(&Arc::new(message));
        } else {
            context
                .chat
                .broadcast_message(Arc::new(message), &mut context.players);
        }

        Ok(AdminUpdate::ChatSent)
    }

    /// Responds with the current status of web socket json.
    fn request_allow_web_socket_json(&self) -> Result<AdminUpdate, &'static str> {
        Ok(AdminUpdate::AllowWebSocketJsonRequested(
            self.allow_web_socket_json.load(Ordering::Relaxed),
        ))
    }

    /// Changes the web socket json setting.
    fn set_allow_web_socket_json(
        &mut self,
        allow_web_socket_json: bool,
    ) -> Result<AdminUpdate, &'static str> {
        self.allow_web_socket_json
            .store(allow_web_socket_json, Ordering::Relaxed);
        Ok(AdminUpdate::AllowWebSocketJsonSet(allow_web_socket_json))
    }

    /// Responds with the current status of load distribution.
    fn request_distribute_load(&self) -> Result<AdminUpdate, &'static str> {
        Ok(AdminUpdate::DistributeLoadRequested(self.distribute_load))
    }

    /// Changes the load distribution setting.
    fn set_distribute_load(&mut self, distribute_load: bool) -> Result<AdminUpdate, &'static str> {
        self.distribute_load = distribute_load;
        Ok(AdminUpdate::DistributeLoadSet(distribute_load))
    }

    /// Requests the currently-set server to redirect to.
    fn request_redirect(&self) -> Result<AdminUpdate, &'static str> {
        Ok(AdminUpdate::RedirectRequested(
            self.redirect_server_id_preference,
        ))
    }

    /// Changes the server to redirect to. Must not redirect to self.
    ///
    /// Only has an effect if the [`SystemRepo`] is configured.
    fn set_redirect(
        &mut self,
        redirect: Option<ServerId>,
        server_id: Option<ServerId>,
        system: Option<&mut SystemRepo<G>>,
    ) -> Result<AdminUpdate, &'static str> {
        if let Some(server_id) = server_id {
            if redirect == Some(server_id) {
                return Err("cannot redirect to self");
            }
        }
        self.redirect_server_id_preference = redirect;

        if let Some(system) = system {
            system.set_redirect(self.redirect_server_id_preference);
        } else {
            warn!("no system configured, cannot actually set redirect.");
        }

        Ok(AdminUpdate::RedirectSet(redirect))
    }

    #[cfg(unix)]
    fn start_profile(&mut self) -> Result<(), &'static str> {
        if self.profile.is_some() {
            Err("profile already started")
        } else {
            self.profile =
                Some(pprof::ProfilerGuard::new(1000).map_err(|_| "failed to start profile")?);
            Ok(())
        }
    }

    #[cfg(unix)]
    fn finish_profile(&mut self) -> Result<AdminUpdate, &'static str> {
        if let Some(profile) = self.profile.as_mut() {
            if let Ok(report) = profile.report().build() {
                self.profile = None;

                let mut buf = Vec::new();
                report
                    .flamegraph(&mut buf)
                    .map_err(|_| "error writing profiler flamegraph")?;

                Ok(AdminUpdate::ProfileRequested(
                    String::from_utf8(buf).map_err(|_| "profile contained invalid utf8")?,
                ))
            } else {
                Err("error building profile report")
            }
        } else {
            Err("profile not started or was interrupted")
        }
    }
}

impl<G: GameArenaService> Handler<ParameterizedAdminRequest> for Infrastructure<G> {
    type Result = ResponseActFuture<Self, Result<AdminUpdate, &'static str>>;

    fn handle(&mut self, msg: ParameterizedAdminRequest, _ctx: &mut Self::Context) -> Self::Result {
        if !msg.is_authentic() {
            return Box::pin(fut::ready(Err("invalid auth")));
        }

        let request = msg.request;
        let database = self.database();
        match request {
            // Handle asynchronous requests (i.e. those that access database).
            AdminRequest::RequestSeries {
                game_id,
                period_start,
                period_stop,
                resolution,
            } => Box::pin(
                async move {
                    database
                        .get_metrics_between(game_id, period_start, period_stop)
                        .await
                }
                .into_actor(self)
                .map(move |db_result, _act, _ctx| {
                    if let Ok(loaded) = db_result {
                        let series: Arc<[(UnixTime, MetricsDataPointDto)]> = loaded
                            .rchunks(resolution.map(|v| v.get() as usize).unwrap_or(1))
                            .map(|items| {
                                (
                                    items[0].timestamp,
                                    items
                                        .iter()
                                        .map(|i| i.metrics.clone())
                                        .sum::<Metrics>()
                                        .data_point(),
                                )
                            })
                            .collect();
                        let message = AdminUpdate::SeriesRequested(series);
                        Ok(message)
                    } else {
                        Err("failed to load")
                    }
                }),
            ),
            AdminRequest::RequestDay { filter } => {
                Box::pin(fut::ready(AdminRepo::request_day(&self.metrics, filter)))
            }
            AdminRequest::RequestGames => Box::pin(fut::ready(self.admin.request_games())),
            AdminRequest::RequestPlayers => Box::pin(fut::ready(
                self.admin
                    .request_players(&self.context_service.context.players),
            )),
            AdminRequest::OverridePlayerAlias { player_id, alias } => {
                Box::pin(fut::ready(self.admin.override_player_alias(
                    player_id,
                    alias,
                    &self.context_service.context.players,
                )))
            }
            AdminRequest::RestrictPlayer { player_id, minutes } => {
                Box::pin(fut::ready(self.admin.restrict_player(
                    player_id,
                    minutes,
                    &self.context_service.context.players,
                )))
            }
            AdminRequest::MutePlayer { player_id, minutes } => Box::pin(fut::ready(
                self.admin
                    .mute_player(player_id, minutes, &self.context_service.context.players),
            )),
            AdminRequest::RequestServers => {
                Box::pin(fut::ready(AdminRepo::request_servers(&self.system)))
            }
            AdminRequest::RequestSummary { filter } => {
                Box::pin(fut::ready(AdminRepo::request_summary(self, filter)))
            }
            AdminRequest::RequestReferrers => Box::pin(fut::ready(
                self.admin
                    .request_referrers(&self.context_service.context.players),
            )),
            AdminRequest::RequestUserAgents => Box::pin(fut::ready(
                self.admin
                    .request_user_agents(&self.context_service.context.players),
            )),
            AdminRequest::SendChat {
                player_id,
                alias,
                message,
            } => Box::pin(fut::ready(self.admin.send_chat(
                player_id,
                alias,
                message,
                &mut self.context_service.context,
            ))),
            AdminRequest::RequestAllowWebSocketJson => {
                Box::pin(fut::ready(self.admin.request_allow_web_socket_json()))
            }
            AdminRequest::SetAllowWebSocketJson(allow_web_socket_json) => Box::pin(fut::ready(
                self.admin.set_allow_web_socket_json(allow_web_socket_json),
            )),
            AdminRequest::RequestDistributeLoad => {
                Box::pin(fut::ready(self.admin.request_distribute_load()))
            }
            AdminRequest::SetDistributeLoad(distribute_load) => {
                Box::pin(fut::ready(self.admin.set_distribute_load(distribute_load)))
            }
            AdminRequest::RequestRedirect => Box::pin(fut::ready(self.admin.request_redirect())),
            AdminRequest::SetRedirect(server_id) => Box::pin(fut::ready(self.admin.set_redirect(
                server_id,
                self.server_id,
                self.system.as_mut(),
            ))),
            #[cfg(unix)]
            AdminRequest::RequestProfile => {
                if let Err(e) = self.admin.start_profile() {
                    Box::pin(fut::ready(Err(e)))
                } else {
                    Box::pin(
                        tokio::time::sleep(Duration::from_secs(10))
                            .into_actor(self)
                            .map(move |_res, act, _ctx| act.admin.finish_profile()),
                    )
                }
            }
        }
    }
}

/// Converts a duration to seconds, rounding up.
fn seconds_ceil(duration: Duration) -> usize {
    ((duration.as_secs() + 59) / 60) as usize
}
