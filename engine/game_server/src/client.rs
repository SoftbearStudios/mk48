// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::chat::{ChatRepo, ClientChatData};
use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::invitation::{ClientInvitationData, InvitationRepo};
use crate::leaderboard::LeaderboardRepo;
use crate::liveboard::LiveboardRepo;
use crate::metric::{ClientMetricData, MetricRepo};
use crate::player::{PlayerData, PlayerRepo, PlayerTuple};
use crate::system::SystemRepo;
use crate::team::{ClientTeamData, TeamRepo};
use actix::WrapStream;
use actix::{
    fut, ActorFutureExt, ActorStreamExt, Context as ActorContext, ContextFutureSpawner, Handler,
    Message, ResponseActFuture, WrapFuture,
};
use atomic_refcell::AtomicRefCell;
use core_protocol::dto::{InvitationDto, ServerDto};
use core_protocol::get_unix_time_now;
use core_protocol::id::{
    ArenaId, CohortId, InvitationId, PlayerId, ServerId, SessionId, UserAgentId,
};
use core_protocol::name::{PlayerAlias, Referrer};
use core_protocol::rpc::{
    AdType, ClientRequest, ClientUpdate, LeaderboardUpdate, LiveboardUpdate, PlayerUpdate, Request,
    SystemUpdate, TeamUpdate, Update,
};
use futures::stream::FuturesUnordered;
use log::{error, info, warn};
use maybe_parallel_iterator::IntoMaybeParallelRefIterator;
use rust_embed::RustEmbed;
use server_util::database_schema::SessionItem;
use server_util::generate_id::{generate_id, generate_id_64};
use server_util::ip_rate_limiter::IpRateLimiter;
use server_util::observer::{ObserverMessage, ObserverUpdate};
use server_util::rate_limiter::{RateLimiter, RateLimiterProps};
use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::marker::PhantomData;
use std::net::IpAddr;
use std::num::NonZeroU64;
use std::ops::Deref;
use std::str;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

/// Directed to a websocket future corresponding to a client.
pub type ClientAddr<G> =
    UnboundedSender<ObserverUpdate<Update<<G as GameArenaService>::GameUpdate>>>;

/// Keeps track of clients a.k.a. real players a.k.a. websockets.
pub struct ClientRepo<G: GameArenaService> {
    authenticate_rate_limiter: IpRateLimiter,
    prune_rate_limiter: RateLimiter,
    database_rate_limiter: RateLimiter,
    pending_session_write: Vec<SessionItem>,
    pub(crate) snippets: HashMap<(Option<CohortId>, Option<Referrer>), Arc<str>>,
    /// Where to log traces to.
    trace_log: Option<Arc<str>>,
    _spooky: PhantomData<G>,
}

#[derive(RustEmbed)]
#[folder = "../js/public/referrer/"]
struct ReferrerSnippet;

impl<G: GameArenaService> ClientRepo<G> {
    pub fn new(trace_log: Option<String>, authenticate: RateLimiterProps) -> Self {
        Self {
            authenticate_rate_limiter: authenticate.into(),
            prune_rate_limiter: RateLimiter::new(Duration::from_secs(1), 0),
            database_rate_limiter: RateLimiter::new(Duration::from_secs(30), 0),
            pending_session_write: Vec::new(),
            snippets: Self::load_default_snippets(),
            trace_log: trace_log.map(Into::into),
            _spooky: PhantomData,
        }
    }

    fn load_default_snippets() -> HashMap<(Option<CohortId>, Option<Referrer>), Arc<str>> {
        let mut hash_map = HashMap::new();
        for key in ReferrerSnippet::iter() {
            let value = ReferrerSnippet::get(&key).map(|f| f.data);
            if let Some(value) = value {
                // key is like "default.js" or "1.foo.js" where "foo" is a referrer (referrer cannot contain ".").
                let segs: Vec<&str> = key.split(".").collect();
                let n = segs.len();
                if n < 2 || n > 3 || segs[n - 1].to_lowercase() != "js" {
                    error!("invalid snippet key: {:?}", key);
                    continue;
                }
                let cohort_id = if n == 3 {
                    segs[0].parse().ok().map(CohortId)
                } else {
                    None
                };
                let referrer = if segs[n - 2].to_lowercase() == "default" {
                    None
                } else {
                    Referrer::new(segs[n - 2])
                };

                match str::from_utf8(&value) {
                    Ok(js_src) => {
                        log::info!(
                            "referrer snippet for cohort {:?}, referrer {:?} is {}",
                            cohort_id,
                            referrer,
                            js_src
                        );
                        hash_map.insert((cohort_id, referrer), js_src.to_string().into());
                    }
                    Err(e) => {
                        error!("invalid UTF-8 in referrer JS file: {:?}", e);
                    }
                }
            }
        }
        hash_map
    }

    /// Updates sessions to database (internally rate-limited).
    ///
    /// Note: Sessions also get updated to database when they are being dropped.
    pub(crate) fn update_to_database(
        infrastructure: &mut Infrastructure<G>,
        ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        if infrastructure
            .context_service
            .context
            .clients
            .database_rate_limiter
            .should_limit_rate()
        {
            return;
        }

        // Mocker server id if read only, so we can still proceed.
        #[cfg(debug_assertions)]
        let server_id = infrastructure
            .server_id
            .unwrap_or(ServerId::new(200).unwrap());
        #[cfg(not(debug_assertions))]
        let server_id = crate::unwrap_or_return!(infrastructure.server_id);
        let arena_id = infrastructure.context_service.context.arena_id;

        let queue = FuturesUnordered::new();

        // Backlog from leaving sessions.
        for pending in infrastructure
            .context_service
            .context
            .clients
            .pending_session_write
            .drain(..)
        {
            queue.push(infrastructure.database.put_session(pending));
        }

        for mut player in infrastructure
            .context_service
            .context
            .players
            .iter_borrow_mut()
        {
            let player_id = player.player_id;
            if let Some(client) = player.client_mut() {
                if let Some(session_item) =
                    Self::db_session_item(server_id, arena_id, player_id, client)
                {
                    queue.push(infrastructure.database.put_session(session_item))
                }
            }
        }

        queue
            .into_actor(infrastructure)
            .map(|result, _, _| {
                if let Err(e) = result {
                    error!("error putting session: {:?}", e);
                }
            })
            .finish()
            .spawn(ctx);
    }

    /// If the session is dirty with respect to the database, creates a session item to overwrite
    /// the database version.
    fn db_session_item(
        server_id: ServerId,
        arena_id: ArenaId,
        player_id: PlayerId,
        client: &mut PlayerClientData<G>,
    ) -> Option<SessionItem> {
        let session_item = SessionItem {
            alias: client.alias,
            arena_id,
            cohort_id: client.metrics.cohort_id,
            date_created: client.metrics.date_created,
            date_previous: client.metrics.date_previous,
            date_renewed: client.metrics.date_renewed,
            date_terminated: None,
            game_id: G::GAME_ID,
            player_id,
            plays: client.metrics.plays + client.metrics.previous_plays,
            moderator: client.moderator,
            previous_id: client.metrics.session_id_previous,
            referrer: client.metrics.referrer,
            user_agent_id: client.metrics.user_agent_id,
            server_id,
            session_id: client.session_id,
        };

        if client.session_item.as_ref() != Some(&session_item) {
            client.session_item = Some(session_item.clone());
            Some(session_item)
        } else {
            None
        }
    }

    /// Client websocket (re)connected.
    pub(crate) fn register(
        &mut self,
        player_id: PlayerId,
        register_observer: ClientAddr<G>,
        players: &mut PlayerRepo<G>,
        teams: &mut TeamRepo<G>,
        chat: &ChatRepo<G>,
        leaderboards: &LeaderboardRepo<G>,
        liveboard: &LiveboardRepo<G>,
        metrics: &mut MetricRepo<G>,
        system: Option<&SystemRepo<G>>,
        arena_id: ArenaId,
        server_id: Option<ServerId>,
        game: &mut G,
    ) {
        let player_tuple = match players.get(player_id) {
            Some(player_tuple) => player_tuple,
            None => {
                debug_assert!(false, "client gone in register");
                return;
            }
        };

        let mut player = player_tuple.borrow_player_mut();

        let client = match player.client_mut() {
            Some(client) => client,
            None => {
                debug_assert!(false, "register wasn't a client");
                return;
            }
        };

        // Welcome the client in.
        let _ = register_observer.send(ObserverUpdate::Send {
            message: Update::Client(ClientUpdate::SessionCreated {
                arena_id,
                cohort_id: client.metrics.cohort_id,
                server_id,
                session_id: client.session_id,
                player_id,
            }),
        });

        // Don't assume client remembered anything, although it may/should have.
        *client.data.borrow_mut() = G::ClientData::default();
        client.chat.forget_state();
        client.team.forget_state();

        // If there is a JS snippet for the cohort and referrer, send it to client for eval.
        let snippet = client
            .metrics
            .referrer
            .and_then(|referrer| {
                self.snippets
                    .get(&(Some(client.metrics.cohort_id), Some(referrer)))
            })
            .or_else(|| {
                client
                    .metrics
                    .referrer
                    .and_then(|referrer| self.snippets.get(&(None, Some(referrer))))
            })
            .or_else(|| self.snippets.get(&(Some(client.metrics.cohort_id), None)))
            .or_else(|| self.snippets.get(&(None, None)));
        if let Some(snippet) = snippet {
            let _ = register_observer.send(ObserverUpdate::Send {
                message: Update::Client(ClientUpdate::EvalSnippet(snippet.clone())),
            });
        }

        // Change status to connected.
        let new_status = ClientStatus::Connected {
            observer: register_observer.clone(),
        };
        let old_status = std::mem::replace(&mut client.status, new_status);

        match old_status {
            ClientStatus::Connected { observer } => {
                // If it still exists, old client is now retired.
                let _ = observer.send(ObserverUpdate::Close);
                drop(player);
            }
            ClientStatus::Limbo { .. } => {
                info!("player {:?} restored from limbo", player_id);
                drop(player);
            }
            ClientStatus::Pending { .. } => {
                metrics.start_visit(client);

                drop(player);

                // We previously left the game, so now we have to rejoin.
                game.player_joined(player_tuple, &*players);
            }
            ClientStatus::LeavingLimbo { .. } => {
                drop(player);

                // We previously left the game, so now we have to rejoin.
                game.player_joined(player_tuple, &*players);
            }
        }

        // Send initial data.
        for initializer in leaderboards.initializers() {
            let _ = register_observer.send(ObserverUpdate::Send {
                message: Update::Leaderboard(initializer),
            });
        }

        let _ = register_observer.send(ObserverUpdate::Send {
            message: Update::Liveboard(liveboard.initializer()),
        });

        let chat_success = chat.initialize_client(player_id, players);
        debug_assert!(chat_success);

        let _ = register_observer.send(ObserverUpdate::Send {
            message: Update::Player(players.initializer()),
        });

        if let Some(initializer) = teams.initializer() {
            let _ = register_observer.send(ObserverUpdate::Send {
                message: Update::Team(initializer),
            });
        }

        if let Some(system) = system {
            if let Some(initializer) = system.initializer() {
                let _ = register_observer.send(ObserverUpdate::Send {
                    message: Update::System(initializer),
                });
            }
        }
    }

    /// Client websocket disconnected.
    pub(crate) fn unregister(
        &mut self,
        player_id: PlayerId,
        unregister_observer: ClientAddr<G>,
        players: &PlayerRepo<G>,
    ) {
        // There is a possible race condition to handle:
        //  1. Client A registers
        //  3. Client B registers with the same session and player so evicts client A from limbo
        //  2. Client A unregisters and is placed in limbo

        let mut player = match players.borrow_player_mut(player_id) {
            Some(player) => player,
            None => return,
        };

        let client = match player.client_mut() {
            Some(client) => client,
            None => return,
        };

        match &client.status {
            ClientStatus::Connected { observer } => {
                if observer.same_channel(&unregister_observer) {
                    client.status = ClientStatus::Limbo {
                        expiry: Instant::now() + G::LIMBO,
                    };
                    info!("player {:?} is in limbo", player_id);
                }
            }
            _ => {}
        }
    }

    /// Update all clients with game state.
    pub(crate) fn update(
        &mut self,
        game: &G,
        players: &mut PlayerRepo<G>,
        teams: &mut TeamRepo<G>,
        liveboard: &mut LiveboardRepo<G>,
        leaderboard: &LeaderboardRepo<G>,
        server_delta: Option<(Arc<[ServerDto]>, Arc<[ServerId]>)>,
    ) {
        let player_update = players.delta(&*teams);
        let team_update = teams.delta(&*players);
        let immut_players = &*players;
        let player_chat_team_updates: HashMap<PlayerId, _> = players
            .iter_player_ids()
            .filter(|&id| {
                !id.is_bot()
                    && immut_players
                        .borrow_player(id)
                        .unwrap()
                        .client()
                        .map(|c| matches!(c.status, ClientStatus::Connected { .. }))
                        .unwrap_or(false)
            })
            .map(|player_id| {
                (
                    player_id,
                    (
                        ChatRepo::<G>::player_delta(player_id, immut_players),
                        teams.player_delta(player_id, immut_players).unwrap(),
                    ),
                )
            })
            .collect();
        let liveboard_update = liveboard.delta(&*players, &*teams);
        let leaderboard_update: Vec<_> = leaderboard.deltas_nondestructive().collect();

        let players = &*players;
        players.players.maybe_par_iter().for_each(
            move |(player_id, player_tuple): (&PlayerId, &Arc<PlayerTuple<G>>)| {
                let player = player_tuple.borrow_player();

                let client_data = match player.client() {
                    Some(client) => client,
                    None => return,
                };

                // In limbo or will be soon (not connected, cannot send an update).
                if let ClientStatus::Connected { observer } = &client_data.status {
                    if let Some(update) = game.get_game_update(
                        player_tuple,
                        &mut *client_data.data.borrow_mut(),
                        players,
                    ) {
                        let _ = observer.send(ObserverUpdate::Send {
                            message: Update::Game(update),
                        });
                    }

                    if let Some((added, removed, real_players)) = player_update.as_ref() {
                        let _ = observer.send(ObserverUpdate::Send {
                            message: Update::Player(PlayerUpdate::Updated {
                                added: Arc::clone(added),
                                removed: Arc::clone(removed),
                                real_players: *real_players,
                            }),
                        });
                    }

                    if let Some((added, removed)) = team_update.as_ref() {
                        if !added.is_empty() {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::Team(TeamUpdate::AddedOrUpdated(Arc::clone(
                                    added,
                                ))),
                            });
                        }
                        if !removed.is_empty() {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::Team(TeamUpdate::Removed(Arc::clone(removed))),
                            });
                        }
                    }

                    if let Some((chat_update, (members, joiners, joins))) =
                        player_chat_team_updates.get(&player_id)
                    {
                        if let Some(chat_update) = chat_update {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::Chat(chat_update.clone()),
                            });
                        }

                        // TODO: We could get members on a per team basis.
                        if let Some(members) = members {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::Team(TeamUpdate::Members(
                                    members.deref().clone().into(),
                                )),
                            });
                        }

                        if let Some(joiners) = joiners {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::Team(TeamUpdate::Joiners(
                                    joiners.deref().clone().into(),
                                )),
                            });
                        }

                        if let Some(joins) = joins {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::Team(TeamUpdate::Joins(
                                    joins.iter().cloned().collect(),
                                )),
                            });
                        }
                    } else {
                        debug_assert!(
                            false,
                            "not possible, all connected clients should have an entry"
                        );
                    }

                    for &(period_id, leaderboard) in &leaderboard_update {
                        let _ = observer.send(ObserverUpdate::Send {
                            message: Update::Leaderboard(LeaderboardUpdate::Updated(
                                period_id,
                                Arc::clone(&leaderboard),
                            )),
                        });
                    }

                    if let Some((added, removed)) = liveboard_update.as_ref() {
                        let _ = observer.send(ObserverUpdate::Send {
                            message: Update::Liveboard(LiveboardUpdate::Updated {
                                added: Arc::clone(added),
                                removed: Arc::clone(removed),
                            }),
                        });
                    }

                    if let Some((added, removed)) = server_delta.as_ref() {
                        if !added.is_empty() {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::System(SystemUpdate::Added(Arc::clone(added))),
                            });
                        }
                        if !removed.is_empty() {
                            let _ = observer.send(ObserverUpdate::Send {
                                message: Update::System(SystemUpdate::Removed(Arc::clone(removed))),
                            });
                        }
                    }
                }
            },
        );
    }

    /// Cleans up old clients. Rate limited internally.
    pub(crate) fn prune(
        &mut self,
        service: &mut G,
        players: &mut PlayerRepo<G>,
        teams: &mut TeamRepo<G>,
        invitations: &mut InvitationRepo<G>,
        metrics: &mut MetricRepo<G>,
        server_id: Option<ServerId>,
        arena_id: ArenaId,
    ) {
        let now = Instant::now();

        if self.prune_rate_limiter.should_limit_rate_with_now(now) {
            return;
        }

        let immut_players = &*players;
        let to_forget: Vec<PlayerId> = immut_players
            .players
            .iter()
            .filter(|&(&player_id, player_tuple)| {
                let mut player = player_tuple.borrow_player_mut();
                let was_alive = player.was_alive;
                if let Some(client_data) = player.client_mut() {
                    match &client_data.status {
                        ClientStatus::Connected { .. } => {
                            // Wait for transition to limbo via unregister, which is the "proper" channel.
                            false
                        }
                        ClientStatus::Limbo { expiry } => {
                            if &now >= expiry {
                                client_data.status = ClientStatus::LeavingLimbo { since: now };
                                drop(player);
                                service.player_left(player_tuple, immut_players);
                            }
                            false
                        }
                        ClientStatus::LeavingLimbo { since } => {
                            if was_alive {
                                debug_assert!(since.elapsed() < Duration::from_secs(1));
                                false
                            } else {
                                metrics.stop_visit(&mut *player);
                                // Unfortunately, the above makes finishing touches to metrics, but
                                // borrows the entire player. Must therefore re-borrow client.
                                let client_data = player.client_mut().unwrap();
                                if let Some(server_id) = server_id {
                                    if let Some(session_item) = Self::db_session_item(
                                        server_id,
                                        arena_id,
                                        player_id,
                                        client_data,
                                    ) {
                                        self.pending_session_write.push(session_item);
                                    }
                                }
                                info!("player_id {:?} expired from limbo", player_id);
                                true
                            }
                        }
                        ClientStatus::Pending { expiry } => {
                            // Not actually in game, so no cleanup required.
                            &now > expiry
                        }
                    }
                } else {
                    false
                }
            })
            .map(|(&player_id, _)| player_id)
            .collect();

        for player_id in to_forget {
            players.forget(player_id, teams, invitations);
        }
    }

    /// Handles [`G::Command`]'s.
    fn handle_game_command(
        player_id: PlayerId,
        command: G::GameRequest,
        service: &mut G,
        players: &PlayerRepo<G>,
    ) -> Result<Option<G::GameUpdate>, &'static str> {
        if let Some(player_data) = players.get(player_id) {
            // Game updates for all players are usually processed at once, but we also allow
            // one-off responses.
            Ok(service.player_command(command, player_data, players))
        } else {
            Err("nonexistent observer")
        }
    }

    /// Request a different alias (may not be done while alive).
    fn set_alias(
        player_id: PlayerId,
        alias: PlayerAlias,
        players: &PlayerRepo<G>,
    ) -> Result<ClientUpdate, &'static str> {
        let mut player = players
            .borrow_player_mut(player_id)
            .ok_or("player doesn't exist")?;

        if player
            .alive_duration()
            .map(|d| d > Duration::from_secs(1))
            .unwrap_or(false)
        {
            return Err("cannot change alias while alive");
        }

        let client = player.client_mut().ok_or("only clients can set alias")?;
        let censored_alias = PlayerAlias::new_sanitized(alias.as_str());
        client.alias = censored_alias;
        Ok(ClientUpdate::AliasSet(censored_alias))
    }

    /// Record client frames per second (FPS) for statistical purposes.
    fn tally_ad(
        player_id: PlayerId,
        ad_type: AdType,
        players: &PlayerRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) -> Result<ClientUpdate, &'static str> {
        let mut player = players
            .borrow_player_mut(player_id)
            .ok_or("player doesn't exist")?;
        let client = player.client_mut().ok_or("only clients can tally ads")?;
        metrics.mutate_with(
            |metrics| {
                let metric = match ad_type {
                    AdType::Banner => &mut metrics.banner_ads,
                    AdType::Rewarded => &mut metrics.rewarded_ads,
                    AdType::Video => &mut metrics.video_ads,
                };
                metric.increment();
            },
            &mut client.metrics,
        );
        Ok(ClientUpdate::AdTallied)
    }

    /// Record client frames per second (FPS) for statistical purposes.
    fn tally_fps(
        player_id: PlayerId,
        fps: f32,
        players: &PlayerRepo<G>,
    ) -> Result<ClientUpdate, &'static str> {
        let mut player = players
            .borrow_player_mut(player_id)
            .ok_or("player doesn't exist")?;
        let client = player.client_mut().ok_or("only clients can tally fps")?;

        client.metrics.fps = sanitize_tps(fps);
        if client.metrics.fps.is_some() {
            Ok(ClientUpdate::FpsTallied)
        } else {
            Err("invalid fps")
        }
    }

    /// Record a client-side error message for investigation.
    fn trace(
        &self,
        player_id: PlayerId,
        message: String,
        players: &PlayerRepo<G>,
    ) -> Result<ClientUpdate, &'static str> {
        let mut player = players
            .borrow_player_mut(player_id)
            .ok_or("player doesn't exist")?;
        let client = player.client_mut().ok_or("only clients can trace")?;

        #[cfg(debug_assertions)]
        let trace_limit = None;
        #[cfg(not(debug_assertions))]
        let trace_limit = Some(25);

        if message.len() > 4096 {
            Err("trace too long")
        } else if trace_limit
            .map(|limit| client.traces < limit)
            .unwrap_or(true)
        {
            if let Some(trace_log) = self.trace_log.as_ref() {
                let trace_log = Arc::clone(trace_log);
                let mut line = Vec::with_capacity(256);
                let mut writer = csv::Writer::from_writer(&mut line);
                if let Err(e) = writer.write_record(&[
                    get_unix_time_now().to_string().as_str(),
                    &format!("{:?}", G::GAME_ID),
                    &client.ip_address.to_string(),
                    &client
                        .metrics
                        .region_id
                        .map(|r| Cow::Owned(format!("{:?}", r)))
                        .unwrap_or(Cow::Borrowed("?")),
                    client
                        .metrics
                        .referrer
                        .as_ref()
                        .map(|r| r.as_str())
                        .unwrap_or("?"),
                    &client
                        .metrics
                        .user_agent_id
                        .map(|ua| Cow::Owned(format!("{:?}", ua)))
                        .unwrap_or(Cow::Borrowed("?")),
                    &message,
                ]) {
                    error!("error composing trace line: {:?}", e);
                } else {
                    drop(writer);
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&*trace_log)
                            .and_then(move |mut file| file.write_all(&line))
                        {
                            error!("error logging trace: {:?}", e);
                        }
                    });
                }
            } else {
                info!("client_trace: {}", message);
            }
            client.traces += 1;
            Ok(ClientUpdate::Traced)
        } else {
            Err("too many traces")
        }
    }

    /// Handles an arbitrary [`ClientRequest`].
    fn handle_client_request(
        &mut self,
        player_id: PlayerId,
        request: ClientRequest,
        players: &PlayerRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) -> Result<ClientUpdate, &'static str> {
        match request {
            ClientRequest::SetAlias(alias) => Self::set_alias(player_id, alias, players),
            ClientRequest::TallyAd(ad_type) => Self::tally_ad(player_id, ad_type, players, metrics),
            ClientRequest::TallyFps(fps) => Self::tally_fps(player_id, fps, players),
            ClientRequest::Trace { message } => self.trace(player_id, message, players),
        }
    }

    /// Handles request made by real player.
    fn handle_observer_request(
        &mut self,
        player_id: PlayerId,
        request: Request<G::GameRequest>,
        service: &mut G,
        arena_id: ArenaId,
        server_id: Option<ServerId>,
        players: &mut PlayerRepo<G>,
        teams: &mut TeamRepo<G>,
        chat: &mut ChatRepo<G>,
        invitations: &mut InvitationRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) -> Result<Option<Update<G::GameUpdate>>, &'static str> {
        match request {
            // Goes first (fast path).
            Request::Game(command) => {
                Self::handle_game_command(player_id, command, service, &*players)
                    .map(|u| u.map(Update::Game))
            }
            Request::Client(request) => self
                .handle_client_request(player_id, request, &*players, metrics)
                .map(|u| Some(Update::Client(u))),
            Request::Chat(request) => chat
                .handle_chat_request(player_id, request, service, players, teams, metrics)
                .map(|u| Some(Update::Chat(u))),
            Request::Invitation(request) => invitations
                .handle_invitation_request(player_id, request, arena_id, server_id, players)
                .map(|u| Some(Update::Invitation(u))),
            Request::Player(request) => players
                .handle_player_request(player_id, request, metrics)
                .map(|u| Some(Update::Player(u))),
            Request::Team(request) => teams
                .handle_team_request(player_id, request, players)
                .map(|u| Some(Update::Team(u))),
        }
    }

    /// Record network round-trip-time measured by websocket for statistical purposes.
    fn handle_observer_rtt(&mut self, player_id: PlayerId, rtt: u16, players: &PlayerRepo<G>) {
        let mut player = match players.borrow_player_mut(player_id) {
            Some(player) => player,
            None => return,
        };

        let client = match player.client_mut() {
            Some(client) => client,
            None => {
                debug_assert!(false);
                return;
            }
        };

        client.metrics.rtt = Some(rtt);
    }
}

/// Don't let bad values sneak in.
fn sanitize_tps(tps: f32) -> Option<f32> {
    tps.is_finite().then_some(tps.clamp(0.0, 144.0))
}

/// Data stored per client (a.k.a websocket a.k.a. real player).
#[derive(Debug)]
pub struct PlayerClientData<G: GameArenaService> {
    /// Authentication.
    pub(crate) session_id: SessionId,
    /// Alias chosen by player.
    pub(crate) alias: PlayerAlias,
    /// Connection state.
    pub(crate) status: ClientStatus<G>,
    /// Discord user id.
    pub(crate) discord_id: Option<NonZeroU64>,
    /// Ip address.
    pub(crate) ip_address: IpAddr,
    /// Is moderator for in-game chat?
    pub moderator: bool,
    /// Previous database item.
    pub(crate) session_item: Option<SessionItem>,
    /// Metrics-related information associated with each client.
    pub(crate) metrics: ClientMetricData<G>,
    /// Invitation-related information associated with each client.
    pub(crate) invitation: ClientInvitationData,
    /// Chat-related information associated with each client.
    pub(crate) chat: ClientChatData,
    /// Team-related information associated with each client.
    pub(crate) team: ClientTeamData,
    /// Players this client has reported.
    pub(crate) reported: HashSet<PlayerId>,
    /// Number of times sent error trace (in order to limit abuse).
    pub(crate) traces: u8,
    /// Game specific client data. Manually serialized
    pub(crate) data: AtomicRefCell<G::ClientData>,
}

#[derive(Debug)]
pub(crate) enum ClientStatus<G: GameArenaService> {
    /// Pending: Initial state. Visit not started yet. Can be forgotten after expiry.
    Pending { expiry: Instant },
    /// Connected and in game. Transitions to limbo if the connection is lost.
    Connected { observer: ClientAddr<G> },
    /// Disconnected but still in game (and visit still in progress).
    /// - Transitions to connected if a new connection is established.
    /// - Transitions to leaving limbo after expiry.
    Limbo { expiry: Instant },
    /// Disconnected and not in game (but visit still in progress).
    /// - Transitions to connected if a new connection is established.
    /// - Transitions to stale after finished leaving game.
    LeavingLimbo { since: Instant },
}

impl<G: GameArenaService> PlayerClientData<G> {
    pub(crate) fn new(
        session_id: SessionId,
        metrics: ClientMetricData<G>,
        invitation: Option<InvitationDto>,
        discord_id: Option<NonZeroU64>,
        ip: IpAddr,
        moderator: bool,
    ) -> Self {
        Self {
            session_id,
            alias: G::default_alias(),
            status: ClientStatus::Pending {
                expiry: Instant::now() + Duration::from_secs(10),
            },
            discord_id,
            ip_address: ip,
            moderator,
            session_item: None,
            metrics,
            invitation: ClientInvitationData::new(invitation),
            chat: ClientChatData::default(),
            team: ClientTeamData::default(),
            reported: Default::default(),
            traces: 0,
            data: AtomicRefCell::new(G::ClientData::default()),
        }
    }

    /// Requires mutable self, but as a result, guaranteed not to panic.
    pub fn data(&mut self) -> &G::ClientData {
        &*self.data.get_mut()
    }

    /// Infallible way of getting mutable client data.
    pub fn data_mut(&mut self) -> &mut G::ClientData {
        self.data.get_mut()
    }
}

/// Handle client messages.
impl<G: GameArenaService> Handler<ObserverMessage<Request<G::GameRequest>, Update<G::GameUpdate>>>
    for Infrastructure<G>
{
    type Result = ();

    fn handle(
        &mut self,
        msg: ObserverMessage<Request<G::GameRequest>, Update<G::GameUpdate>>,
        _ctx: &mut Self::Context,
    ) {
        match msg {
            ObserverMessage::Register {
                player_id,
                observer,
                ..
            } => self.context_service.context.clients.register(
                player_id,
                observer,
                &mut self.context_service.context.players,
                &mut self.context_service.context.teams,
                &self.context_service.context.chat,
                &self.leaderboard,
                &self.context_service.context.liveboard,
                &mut self.metrics,
                self.system.as_ref(),
                self.context_service.context.arena_id,
                self.server_id,
                &mut self.context_service.service,
            ),
            ObserverMessage::Unregister {
                player_id,
                observer,
            } => self.context_service.context.clients.unregister(
                player_id,
                observer,
                &self.context_service.context.players,
            ),
            ObserverMessage::Request { player_id, request } => {
                let context = &mut self.context_service.context;
                let service = &mut self.context_service.service;
                match context.clients.handle_observer_request(
                    player_id,
                    request,
                    service,
                    context.arena_id,
                    self.server_id,
                    &mut context.players,
                    &mut context.teams,
                    &mut context.chat,
                    &mut self.invitations,
                    &mut self.metrics,
                ) {
                    Ok(Some(message)) => {
                        let player = match context.players.borrow_player_mut(player_id) {
                            Some(player) => player,
                            None => {
                                debug_assert!(false);
                                return;
                            }
                        };

                        let client = match player.client() {
                            Some(client) => client,
                            None => {
                                debug_assert!(false);
                                return;
                            }
                        };

                        if let ClientStatus::Connected { observer } = &client.status {
                            let _ = observer.send(ObserverUpdate::Send { message });
                        } else {
                            debug_assert!(false, "impossible due to synchronous nature of code");
                        }
                    }
                    Ok(None) => {}
                    Err(s) => {
                        warn!("observer request resulted in {}", s);
                    }
                }
            }
            ObserverMessage::RoundTripTime { player_id, rtt } => self
                .context_service
                .context
                .clients
                .handle_observer_rtt(player_id, rtt, &self.context_service.context.players),
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<PlayerId, &'static str>")]
pub struct Authenticate {
    /// Client ip address.
    pub ip_address: IpAddr,
    /// User agent.
    pub user_agent_id: Option<UserAgentId>,
    /// Referrer.
    pub referrer: Option<Referrer>,
    /// Last valid credentials.
    pub arena_id_session_id: Option<(ArenaId, SessionId)>,
    /// Invitation?
    pub invitation_id: Option<InvitationId>,
    /// Oauth2 code.
    pub oauth2_code: Option<Oauth2Code>,
}

pub enum Oauth2Code {
    Discord(String),
}

impl<G: GameArenaService> Handler<Authenticate> for Infrastructure<G> {
    type Result = ResponseActFuture<Self, Result<PlayerId, &'static str>>;

    fn handle(&mut self, mut msg: Authenticate, _ctx: &mut ActorContext<Self>) -> Self::Result {
        let arena_id = self.context_service.context.arena_id;
        let clients = &mut self.context_service.context.clients;
        let players = &self.context_service.context.players;

        if clients
            .authenticate_rate_limiter
            .should_limit_rate(msg.ip_address)
        {
            // Should only log IP of malicious actors.
            warn!("IP {:?} was rate limited", msg.ip_address);
            return Box::pin(fut::ready(Err("rate limit exceeded")));
        }

        // TODO: O(n) on players.
        let cached_session_id_player_id = msg
            .arena_id_session_id
            .filter(|&(msg_arena_id, _)| arena_id == msg_arena_id)
            .and_then(|(_, msg_session_id)| {
                players
                    .iter_borrow()
                    .find(|p| {
                        p.client()
                            .map(|c| c.session_id == msg_session_id)
                            .unwrap_or(false)
                    })
                    .map(|p| (msg_session_id, p.player_id))
            });

        let arena_id_session_id = msg.arena_id_session_id;
        let oauth2_code = std::mem::take(&mut msg.oauth2_code);
        let database = self.database();
        let discord_bot = self.discord_bot;
        let discord_oauth2 = self.discord_oauth2;

        Box::pin(
            async move {
                let discord_id = if let Some((Oauth2Code::Discord(code), discord_oauth2)) =
                    oauth2_code.zip(discord_oauth2)
                {
                    match discord_oauth2.authenticate(code).await {
                        Ok(id) => Some(id),
                        Err(e) => {
                            warn!("{}", e);
                            None
                        }
                    }
                } else {
                    None
                };

                let is_moderator =
                    if let Some((discord_id, discord_bot)) = discord_id.zip(discord_bot) {
                        match discord_bot.is_moderator(discord_id).await {
                            Ok(is_moderator) => is_moderator,
                            Err(e) => {
                                warn!("{}", e);
                                false
                            }
                        }
                    } else {
                        false
                    };

                let session_item = if cached_session_id_player_id.is_some() {
                    // No need to load from database because session is in memory.
                    Result::Ok(None)
                } else if let Some((arena_id, session_id)) = arena_id_session_id {
                    database.get_session(arena_id, session_id).await
                } else {
                    // Cannot load from database because (arena_id, session_id) is unavailable.
                    Result::Ok(None)
                };

                (discord_id, is_moderator, session_item)
            }
            .into_actor(self)
            .map(
                move |(discord_id, mut is_moderator, db_result), act, _ctx| {
                    let invitation = msg
                        .invitation_id
                        .and_then(|id| act.invitations.get(id).cloned());
                    let invitation_dto = invitation.map(|i| InvitationDto {
                        player_id: i.player_id,
                    });

                    let mut client_metric_data = ClientMetricData::from(&msg);

                    let restore_session_id_player_id = if let Ok(Some(session_item)) = db_result {
                        client_metric_data.supplement(&session_item);
                        // Restore moderator status.
                        is_moderator |= session_item.moderator;
                        (session_item.arena_id == arena_id)
                            .then_some((session_item.session_id, session_item.player_id))
                    } else {
                        None
                    };

                    let (session_id, player_id) = if let Some(existing) =
                        cached_session_id_player_id.or(restore_session_id_player_id)
                    {
                        existing
                    } else {
                        let mut session_ids = HashSet::with_capacity(
                            act.context_service.context.players.real_players,
                        );

                        // TODO: O(n) on players.
                        for player in act.context_service.context.players.iter_borrow() {
                            if let Some(client_data) = player.client() {
                                session_ids.insert(client_data.session_id);
                            }
                        }

                        let new_session_id = loop {
                            let session_id = SessionId(generate_id_64());
                            if !session_ids.contains(&session_id) {
                                break session_id;
                            }
                        };

                        let new_player_id = loop {
                            let player_id = PlayerId(generate_id());
                            if !act.context_service.context.players.contains(player_id) {
                                break player_id;
                            }
                        };

                        (new_session_id, new_player_id)
                    };

                    match act.context_service.context.players.players.entry(player_id) {
                        Entry::Occupied(mut occupied) => {
                            if let Some(client) =
                                occupied.get_mut().borrow_player_mut().client_mut()
                            {
                                client.metrics.date_renewed = get_unix_time_now();
                                // Update the referrer, such that the correct snippet may be served.
                                client.metrics.referrer = msg.referrer.or(client.metrics.referrer);
                                if let Some(discord_id) = discord_id {
                                    client.discord_id = Some(discord_id);
                                    client.moderator = is_moderator;
                                }
                            } else {
                                debug_assert!(
                                    false,
                                    "impossible to be a bot since session was valid"
                                );
                            }
                        }
                        Entry::Vacant(vacant) => {
                            let client = PlayerClientData::new(
                                session_id,
                                client_metric_data,
                                invitation_dto,
                                discord_id,
                                msg.ip_address,
                                is_moderator,
                            );
                            let pd = PlayerData::new(player_id, Some(Box::new(client)));
                            let pt = Arc::new(PlayerTuple::new(pd));
                            vacant.insert(pt);
                        }
                    }

                    Ok(player_id)
                },
            ),
        )
    }
}
