// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::database::{Database, MetricsItem, Score, ScoreType, SessionItem};
use crate::repo::*;
use crate::session::Session;
use actix::prelude::*;
use actix::Recipient;
use core_protocol::dto::LeaderboardDto;
use core_protocol::id::PeriodId;
use core_protocol::id::*;
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{
    ClientRequest, ClientUpdate, MetricRequest, MetricUpdate, ServerRequest, ServerUpdate,
};
use core_protocol::*;
use futures::stream::futures_unordered::FuturesUnordered;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use servutil::observer::*;
use std::collections::hash_map::{Entry, HashMap};
use std::fs::OpenOptions;
use std::lazy::OnceCell;
use std::sync::Arc;
use std::time::Duration;
use std::process;

const CLIENT_TIMER_MILLIS: u64 = 250;
const DATABASE_TIMER_SECS: u64 = 60;
const LEADERBOARD_TIMER_SECS: u64 = 1;
const SERVER_TIMER_MILLIS: u64 = 250;
const TEAM_TIMER_SECS: u64 = 15;
const METRICS_MILLIS: u64 = 60 * 60 * 1000;

/// Putting this in an actor is very tricky. It won't have a static lifetime, and that makes
/// it hard to use async functions.
static mut DATABASE: OnceCell<Database> = OnceCell::new();

#[derive(Serialize, Deserialize)]
pub struct ClientState {
    arena_id: Option<ArenaId>,
    newbie: bool,
    session_id: Option<SessionId>,
}

#[derive(Serialize, Deserialize)]
pub struct ServerState {
    pub arena_id: Option<ArenaId>,
}

pub struct Core {
    /// Optional file to append chat logs as CSV.
    chat_log: Option<String>,
    /// Inhibits writing to the database.
    database_read_only: bool,
    clients: HashMap<Recipient<ObserverUpdate<ClientUpdate>>, ClientState>,
    repo: Repo,
    servers: HashMap<Recipient<ObserverUpdate<ServerUpdate>>, ServerState>,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "Result<ClientUpdate, &'static str>")]
pub struct ParametrizedClientRequest {
    pub params: ClientState,
    pub request: ClientRequest,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "Result<MetricUpdate, &'static str>")]
pub struct ParameterizedMetricRequest {
    pub request: MetricRequest,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "Result<ServerUpdate, &'static str>")]
pub struct ParametrizedServerRequest {
    pub params: ServerState,
    pub request: ServerRequest,
}

impl Core {
    /// Creates a new core, with various options. Unsafe to call more than once.
    pub async fn new(chat_log: Option<String>, database_read_only: bool) -> Self {
        // SAFETY: Only happens once.
        unsafe {
            let _ = DATABASE.set(Database::new().await);
        }

        Self {
            chat_log,
            database_read_only,
            clients: HashMap::new(),
            repo: Repo::new(),
            servers: HashMap::new(),
        }
    }

    /// Returns a static reference to the database singleton.
    fn database() -> &'static Database {
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

impl Handler<ObserverMessage<ClientRequest, ClientUpdate>> for Core {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        msg: ObserverMessage<ClientRequest, ClientUpdate>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ObserverMessage::Request { observer, request } => match request {
                // Handle asynchronous requests (i.e. those that may access database).
                ClientRequest::CreateInvitation => {
                    /* TODO:
                        if client.arena_id.is_some() && client.session_id.is_some() {
                            let Some((invitationId, invitation) = act.repo.create_invitation(arena_id, session_id);
                            // Note: Invitation contains (arena_id, team_id, server_addr) - see arena.rs
                            // Use async call to put it into DB with invitation_id as hash key.
                        }
                    */
                }
                ClientRequest::CreateSession {
                    alias,
                    game_id,
                    invitation_id: _,
                    language_pref,
                    referer,
                    region_pref,
                    saved_session_tuple,
                } => {
                    // TODO: if invitation_id is not in cache then load it from DB and put it into cache.
                    let found = self.repo.is_session_in_cache(saved_session_tuple);
                    info!("create session 1: found={}", found);
                    return Box::pin(
                        async move {
                            if found {
                                // No need to load from database because session is in memory.
                                Result::Ok(None)
                            } else if let Some((arena_id, session_id)) = saved_session_tuple {
                                info!("create session 2: reading from DB...");
                                Self::database().get_session(arena_id, session_id).await
                            } else {
                                // Cannot load from database because (arena_id, session_id) is unavailable.
                                Result::Ok(None)
                            }
                        }
                        .into_actor(self)
                        .map(move |db_result, act, _ctx| {
                            // Client may have been deleted during the async section, check again.
                            if !act.clients.contains_key(&observer) {
                                warn!("create session 3: observer lost");
                                return;
                            }
                            let client = act.clients.get_mut(&observer).unwrap();

                            if let Some(loaded) = db_result.ok() {
                                if let Some(session_item) = loaded {
                                    info!("create session 4: loaded from DB {:?}", session_item);

                                    let bot = false;
                                    let mut session = Session::new(
                                        session_item.alias.clone(),
                                        session_item.arena_id,
                                        bot,
                                        session_item.game_id,
                                        session_item.language,
                                        session_item.player_id,
                                        session_item.previous_id,
                                        session_item.referer,
                                        session_item.region_id,
                                        session_item.server_addr,
                                    );
                                    session.date_created = session_item.date_created;
                                    session.date_renewed = session_item.date_renewed;
                                    session.date_terminated = session_item.date_terminated;

                                    let _ = act.repo.put_session(
                                        session_item.arena_id,
                                        session_item.session_id,
                                        session,
                                    );
                                }
                            }

                            if let Some((arena_id, language, region, session_id, server_addr)) =
                                act.repo.create_session(
                                    alias,
                                    game_id,
                                    language_pref,
                                    referer,
                                    region_pref,
                                    saved_session_tuple,
                                )
                            {
                                info!("session created!");

                                if client.arena_id != None
                                    && client.session_id != None
                                    && (client.arena_id.unwrap() != arena_id
                                        || client.session_id.unwrap() != session_id)
                                {
                                    info!("terminating old session");

                                    act.repo.terminate_session(
                                        client.arena_id.unwrap(),
                                        client.session_id.unwrap(),
                                    );
                                }

                                client.arena_id = Some(arena_id);
                                client.session_id = Some(session_id);
                                let success = ClientUpdate::SessionCreated {
                                    arena_id,
                                    language,
                                    region,
                                    server_addr,
                                    session_id,
                                };
                                info!("notifying client about session");
                                match observer.do_send(ObserverUpdate::Send { message: success }) {
                                    Err(e) => {
                                        warn!("Error sending {}", e)
                                    }
                                    _ => {}
                                }
                            }
                        }),
                    );
                }
                // Handle synchronous requests.
                _ => {
                    if let Some(client) = self.clients.get_mut(&observer) {
                        let result =
                            self.repo
                                .handle_client_sync(client, request, self.chat_log.as_deref());
                        if let Ok(success) = result {
                            match observer.do_send(ObserverUpdate::Send { message: success }) {
                                Err(e) => {
                                    warn!("Error sending {}", e)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            },
            ObserverMessage::Register { observer, .. } => {
                if let Entry::Vacant(e) = self.clients.entry(observer) {
                    e.insert(ClientState {
                        arena_id: None,
                        newbie: true,
                        session_id: None,
                    });
                }
            }
            ObserverMessage::Unregister { observer } => {
                self.clients.remove(&observer);
            }
        }

        // Do absolutely nothing, but do it asynchronously so the type system is happy.
        Box::pin(fut::ready(()))
    }
}

impl Handler<ObserverMessage<MetricRequest, MetricUpdate>> for Core {
    type Result = ();
    fn handle(
        &mut self,
        msg: ObserverMessage<MetricRequest, MetricUpdate>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ObserverMessage::Request { observer, request } => {
                let result = self.repo.handle_metric(request);
                if let Ok(success) = result {
                    match observer.do_send(ObserverUpdate::Send { message: success }) {
                        Err(e) => {
                            warn!("Error sending {}", e)
                        }
                        _ => {}
                    }
                }
            }
            ObserverMessage::Register { .. } => {
                todo!();
            }
            ObserverMessage::Unregister { .. } => {
                todo!();
            }
        }
    }
}

impl Handler<ObserverMessage<ServerRequest, ServerUpdate>> for Core {
    type Result = ();
    fn handle(
        &mut self,
        msg: ObserverMessage<ServerRequest, ServerUpdate>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ObserverMessage::Request { observer, request } => {
                if let Some(server) = self.servers.get_mut(&observer) {
                    let result = self.repo.handle_server(server, request);
                    if let Ok(success) = result {
                        match observer.do_send(ObserverUpdate::Send { message: success }) {
                            Err(e) => {
                                warn!("Error sending {}", e)
                            }
                            _ => {}
                        }
                    }
                }
            }
            ObserverMessage::Register { observer, .. } => {
                if let Entry::Vacant(e) = self.servers.entry(observer) {
                    e.insert(ServerState { arena_id: None });
                }
            }
            ObserverMessage::Unregister { observer } => {
                self.servers.remove(&observer);
            }
        }
    }
}

impl Handler<ParametrizedClientRequest> for Core {
    type Result = Result<ClientUpdate, &'static str>;

    fn handle(
        &mut self,
        mut msg: ParametrizedClientRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.repo
            .handle_client_sync(&mut msg.params, msg.request, self.chat_log.as_deref())
    }
}

impl Handler<ParametrizedServerRequest> for Core {
    type Result = Result<ServerUpdate, &'static str>;

    fn handle(
        &mut self,
        mut msg: ParametrizedServerRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.repo.handle_server(&mut msg.params, msg.request)
    }
}

impl Handler<ParameterizedMetricRequest> for Core {
    type Result = Result<MetricUpdate, &'static str>;

    fn handle(
        &mut self,
        msg: ParameterizedMetricRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.repo.handle_metric(msg.request)
    }
}

impl Core {
    fn start_timers(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_millis(CLIENT_TIMER_MILLIS), |act, _ctx| {
            let mut found = 0;
            let mut sent = 0;

            // Initialize new clients.
            let mut any_newbies = false;
            for (_, client) in act.clients.iter() {
                if client.newbie {
                    any_newbies = true;
                    break;
                }
            }
            if any_newbies {
                // An optimization, if multiple clients join at once, would be to get inits only once per arena.
                let regions = act.repo.get_regions();
                for (addr, client) in act.clients.iter_mut() {
                    if !client.newbie || client.arena_id == None {
                        continue;
                    }
                    client.newbie = false;

                    sent += 1;
                    match addr.do_send(ObserverUpdate::Send {
                        message: ClientUpdate::RegionsUpdated {
                            added: Arc::clone(&regions),
                            removed: Arc::new([]),
                        },
                    }) {
                        Err(e) => {
                            warn!("Error sending {}", e)
                        }
                        _ => {}
                    }

                    if let Some((
                        leaderboard_initializer,
                        liveboard_initializer,
                        message_initializer,
                        player_initializer,
                        team_initializer,
                    )) = act.repo.get_initializers(client.arena_id.unwrap())
                    {
                        for (leaderboard, period) in leaderboard_initializer {
                            sent += 1;
                            match addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::LeaderboardUpdated {
                                    leaderboard,
                                    period,
                                },
                            }) {
                                Err(e) => {
                                    warn!("Error sending {}", e)
                                }
                                _ => {}
                            }
                        }

                        sent += 1;
                        match addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::LiveboardUpdated {
                                liveboard: liveboard_initializer.clone(),
                            },
                        }) {
                            Err(e) => {
                                warn!("Error sending {}", e)
                            }
                            _ => {}
                        }

                        sent += 1;
                        match addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::MessagesUpdated {
                                added: Arc::clone(&message_initializer),
                            },
                        }) {
                            Err(e) => {
                                warn!("Error sending {}", e)
                            }
                            _ => {}
                        }

                        sent += 1;
                        match addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::PlayersUpdated {
                                added: player_initializer.clone(),
                                removed: Arc::new([]),
                            },
                        }) {
                            Err(e) => {
                                warn!("Error sending {}", e)
                            }
                            _ => {}
                        }

                        sent += 1;
                        match addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::TeamsUpdated {
                                added: team_initializer.clone(),
                                removed: Arc::new([]),
                            },
                        }) {
                            Err(e) => {
                                warn!("Error sending {}", e)
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Notify existing clients of any changes.
            if let Some((players_added_or_removed, teams_added_or_removed)) =
                act.repo.read_broadcasts()
            {
                for (arena_id, (added, removed)) in players_added_or_removed.iter() {
                    found += 1;
                    for (addr, client) in act.clients.iter_mut() {
                        if let Some(client_arena_id) = client.arena_id {
                            if client_arena_id == *arena_id {
                                sent += 1;
                                match addr.do_send(ObserverUpdate::Send {
                                    message: ClientUpdate::PlayersUpdated {
                                        added: Arc::clone(added),
                                        removed: Arc::clone(removed),
                                    },
                                }) {
                                    Err(e) => {
                                        warn!("Error sending {}", e)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                for (arena_id, (added, removed)) in teams_added_or_removed.iter() {
                    found += 1;
                    for (addr, client) in act.clients.iter_mut() {
                        if let Some(client_arena_id) = client.arena_id {
                            if client_arena_id == *arena_id {
                                sent += 1;
                                match addr.do_send(ObserverUpdate::Send {
                                    message: ClientUpdate::TeamsUpdated {
                                        added: Arc::clone(added),
                                        removed: Arc::clone(removed),
                                    },
                                }) {
                                    Err(e) => {
                                        warn!("Error sending {}", e)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            // Notify servers of any new bots.
            // Do this regardless of whether players change, simply because bots are required
            // even if no players have joined yet (otherwise bots will all join the moment the
            // first player does, which has negative side effects).
            for (addr, server) in act.servers.iter() {
                if server.arena_id == None {
                    continue;
                }
                if let Some(bots) = act.repo.read_available_bots(server.arena_id.unwrap()) {
                    for (player_id, session_id) in bots.iter() {
                        match addr.do_send(ObserverUpdate::Send {
                            message: ServerUpdate::BotReady {
                                player_id: *player_id,
                                session_id: *session_id,
                            },
                        }) {
                            Err(e) => {
                                warn!("Error sending {}", e)
                            }
                            _ => {}
                        }
                    }
                }
            }

            for (addr, client) in act.clients.iter_mut() {
                if let Some(arena_id) = client.arena_id {
                    if let Some(session_id) = client.session_id {
                        let (joiners_added_or_removed, joins_added_or_removed, messages_added) =
                            act.repo.read_whispers(arena_id, session_id);

                        let (added, removed) = joiners_added_or_removed;
                        if added.len() + removed.len() > 0 {
                            match addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::JoinersUpdated {
                                    added: Arc::clone(&added),
                                    removed: Arc::clone(&removed),
                                },
                            }) {
                                Err(e) => {
                                    warn!("Error sending {}", e)
                                }
                                _ => {}
                            }
                        }

                        let (added, removed) = joins_added_or_removed;
                        if added.len() + removed.len() > 0 {
                            match addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::JoinsUpdated {
                                    added: Arc::clone(&added),
                                    removed: Arc::clone(&removed),
                                },
                            }) {
                                Err(e) => {
                                    warn!("Error sending {}", e)
                                }
                                _ => {}
                            }
                        }

                        if messages_added.len() > 0 {
                            match addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::MessagesUpdated {
                                    added: Arc::clone(&messages_added),
                                },
                            }) {
                                Err(e) => {
                                    warn!("Error sending {}", e)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            if found != 0 && sent != 0 && found == sent {
                trace!("{} change(s) sent", sent);
            } else if found == 0 {
                trace!("no changes found");
            } else if sent == 0 {
                trace!("{} change(s) not sent", found);
            } else {
                trace!("{} change(s) found, {} change(s) sent", found, sent);
            }
        }); // ctx.run_interval

        ctx.run_interval(Duration::from_secs(DATABASE_TIMER_SECS), |act, ctx| {
            // Update leaderboard with database.
            {
                let stream: FuturesUnordered<_> = act
                    .repo
                    .get_liveboards(false)
                    .into_iter()
                    .map(|(arena_id, game_id, leaderboard)| {
                        let mut player_scores: Vec<Score> = leaderboard
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

                        if act.database_read_only {
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
                    .iter_recently_modified_sessions(DATABASE_TIMER_SECS * 1000)
                {
                    stream.push(Core::database().put_session(SessionItem {
                        alias: session.alias.clone(),
                        arena_id,
                        date_created: session.date_created,
                        date_renewed: session.date_renewed,
                        date_terminated: session.date_terminated,
                        game_id: session.game_id,
                        language: session.language,
                        player_id: session.player_id,
                        previous_id: session.previous_id,
                        region_id: session.region_id,
                        referer: session.referer,
                        server_addr: session.server_addr,
                        session_id,
                    }));
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

        ctx.run_interval(Duration::from_millis(METRICS_MILLIS), |act, ctx| {
            if act.database_read_only {
                warn!("Would have written to metrics database, but was inhibited");
            } else {
                let stream = FuturesUnordered::new();

                for (game_id, metrics) in act.repo.get_metrics(METRICS_MILLIS) {
                    stream.push(Core::database().update_metrics(MetricsItem {
                        game_id,
                        timestamp: (get_unix_time_now() / METRICS_MILLIS) * METRICS_MILLIS,
                        metrics,
                    }))
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

        ctx.run_interval(Duration::from_secs(LEADERBOARD_TIMER_SECS), |act, _ctx| {
            if let Some(changed_leaderboards) = act.repo.read_leaderboards() {
                for (arena_id, leaderboard, period) in changed_leaderboards.iter() {
                    for (addr, client) in act.clients.iter_mut() {
                        if client.newbie {
                            continue; // Will be initialized elsewhere.
                        }
                        if let Some(client_arena_id) = client.arena_id {
                            if client_arena_id == *arena_id {
                                match addr.do_send(ObserverUpdate::Send {
                                    message: ClientUpdate::LeaderboardUpdated {
                                        leaderboard: leaderboard.clone(),
                                        period: *period,
                                    },
                                }) {
                                    Err(e) => {
                                        warn!("Error sending {}", e)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            if let Some(changed_liveboards) = act.repo.read_liveboards() {
                for (arena_id, liveboard) in changed_liveboards.iter() {
                    for (addr, client) in act.clients.iter_mut() {
                        if client.newbie {
                            continue; // Will be initialized elsewhere.
                        }
                        if let Some(client_arena_id) = client.arena_id {
                            if client_arena_id == *arena_id {
                                match addr.do_send(ObserverUpdate::Send {
                                    message: ClientUpdate::LiveboardUpdated {
                                        liveboard: liveboard.clone(),
                                    },
                                }) {
                                    Err(e) => {
                                        warn!("Error sending {}", e)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }); // ctx.run_interval LEADERBOARD

        ctx.run_interval(Duration::from_millis(SERVER_TIMER_MILLIS), |act, _ctx| {
            // Notify existing servers of any changes.
            if let Some(server_updates) = act.repo.read_server_updates() {
                for (arena_id, team_assignments) in server_updates.iter() {
                    for (addr, server) in act.servers.iter_mut() {
                        if let Some(server_arena_id) = server.arena_id {
                            if server_arena_id == *arena_id {
                                match addr.do_send(ObserverUpdate::Send {
                                    message: ServerUpdate::MembersChanged {
                                        changes: Arc::clone(team_assignments), // TODO: only used once; should use Box.
                                    },
                                }) {
                                    Err(e) => {
                                        warn!("Error sending {}", e)
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }); // ctx.run_interval

        ctx.run_interval(Duration::from_secs(TEAM_TIMER_SECS), |act, _ctx| {
            act.repo.prune_sessions();
            act.repo.prune_teams();
        }); // ctx.run_interval TEAM
    }
}

impl Repo {
    fn handle_client_sync(
        &mut self,
        client: &mut ClientState,
        request: ClientRequest,
        chat_log: Option<&str>,
    ) -> Result<ClientUpdate, &'static str> {
        let mut result = Err("client request failed");
        match request {
            ClientRequest::AcceptPlayer { player_id } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.accept_player(arena_id, session_id, player_id) {
                        result = Ok(ClientUpdate::PlayerAccepted { player_id });
                    }
                }
            }
            ClientRequest::AssignCaptain { player_id } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.assign_captain(arena_id, session_id, player_id) {
                        result = Ok(ClientUpdate::CaptainAssigned { player_id });
                    }
                }
            }
            ClientRequest::CreateTeam { team_name } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if let Some(team_id) = self.create_team(arena_id, session_id, team_name) {
                        result = Ok(ClientUpdate::TeamCreated { team_id });
                    }
                }
            }
            ClientRequest::KickPlayer { player_id } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.kick_player(arena_id, session_id, player_id) {
                        result = Ok(ClientUpdate::PlayerKicked { player_id });
                    }
                }
            }
            ClientRequest::MuteSender { enable, player_id } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.mute_sender(arena_id, session_id, enable, player_id) {
                        result = Ok(ClientUpdate::SenderMuted { enable, player_id });
                    }
                }
            }
            ClientRequest::QuitTeam => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.quit_team(arena_id, session_id) {
                        result = Ok(ClientUpdate::TeamQuit);
                    }
                }
            }
            ClientRequest::RequestJoin { team_id } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.request_join(arena_id, session_id, team_id) {
                        result = Ok(ClientUpdate::JoinRequested { team_id });
                    }
                }
            }
            ClientRequest::RejectPlayer { player_id } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.reject_player(arena_id, session_id, player_id) {
                        result = Ok(ClientUpdate::PlayerRejected { player_id });
                    }
                }
            }
            ClientRequest::SendChat { message, whisper } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    let player_id = self.send_chat(arena_id, session_id, message.clone(), whisper);
                    if let Some(chat_log) = chat_log {
                        if let Some(arena) = self.arenas.get(&arena_id) {
                            if let Some(session) = arena.sessions.get(&session_id) {
                                if let Ok(file) =
                                    OpenOptions::new().create(true).append(true).open(chat_log)
                                {
                                    // player_id being Some means the message went through.
                                    let mut wtr = csv::Writer::from_writer(file);
                                    let _ = wtr.write_record(&[
                                        &format!("{}", get_unix_time_now()),
                                        &format!("{:?}", arena.game_id),
                                        &format!("{}", player_id.is_some()),
                                        &session.alias.0.to_string(),
                                        &message,
                                    ]);
                                }
                            }
                        }
                    }
                    result = Ok(ClientUpdate::ChatSent { player_id });
                }
            }
            ClientRequest::Trace { message } => {
                debug!("{}", message);
                result = Ok(ClientUpdate::Traced)
            }
            _ => result = Err("cannot process request synchronously"),
        }

        result
    }

    fn handle_metric(&mut self, request: MetricRequest) -> Result<MetricUpdate, &'static str> {
        let result;
        match request {
            MetricRequest::RequestMetrics => {
                let metrics = self.get_metrics(24 * 60 * 60 * 1000);
                result = Ok(MetricUpdate::MetricsRequested {
                    metrics: metrics
                        .iter()
                        .map(|(game_id, metrics)| (*game_id, metrics.summarize()))
                        .collect(),
                })
            }
        }

        result
    }

    fn handle_server(
        &mut self,
        server: &mut ServerState,
        request: ServerRequest,
    ) -> Result<ServerUpdate, &'static str> {
        let mut result = Err("server request failed");
        match request {
            ServerRequest::BotRequest {
                session_id,
                request,
            } => {
                // TODO: validate that session_id is actually a bot!
                let mut client = ClientState {
                    arena_id: server.arena_id.clone(),
                    newbie: false,
                    session_id: Some(session_id),
                };
                let _ = self.handle_client_sync(&mut client, request, None);
            }
            ServerRequest::DropSession { session_id } => {
                if let Some(arena_id) = server.arena_id {
                    self.drop_session(arena_id, session_id);
                    result = Ok(ServerUpdate::SessionDropped);
                }
            }
            ServerRequest::SetStatus {
                session_id,
                location,
                score,
            } => {
                if let Some(arena_id) = server.arena_id {
                    self.set_status(arena_id, session_id, location, score);
                    result = Ok(ServerUpdate::StatusSet);
                }
            }
            ServerRequest::StartArena {
                game_id,
                region,
                rules,
                saved_arena_id,
                server_addr,
            } => {
                server.arena_id =
                    Some(self.start_arena(game_id, region, rules, saved_arena_id, server_addr));
                if server.arena_id != None {
                    result = Ok(ServerUpdate::ArenaStarted {
                        arena_id: server.arena_id.unwrap(),
                    });
                }
            }
            ServerRequest::StartPlay { session_id } => {
                if let Some(arena_id) = server.arena_id {
                    if let Some(player_id) = self.start_play(arena_id, session_id) {
                        result = Ok(ServerUpdate::PlayStarted { player_id });
                    }
                }
            }
            ServerRequest::StopArena => {
                if let Some(arena_id) = server.arena_id {
                    self.stop_arena(arena_id);
                    server.arena_id = None;
                    result = Ok(ServerUpdate::ArenaStopped);
                }
            }
            ServerRequest::StopPlay { session_id } => {
                if let Some(arena_id) = server.arena_id {
                    self.stop_play(arena_id, session_id);
                    result = Ok(ServerUpdate::PlayStopped);
                }
            }
            ServerRequest::ValidateSession { session_id } => {
                if let Some(arena_id) = server.arena_id {
                    if let Some((elapsed, player_id, score)) =
                        self.validate_session(arena_id, session_id)
                    {
                        result = Ok(ServerUpdate::SessionValid {
                            elapsed,
                            player_id,
                            score,
                        });
                    }
                }
            }
        }

        result
    }
}
