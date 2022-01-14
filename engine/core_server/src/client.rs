// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::chat::log_chat;
use crate::core::*;
use crate::repo::*;
use crate::session::Session;
use crate::user_agent::parse_user_agent;
use actix::prelude::*;
use core_protocol::id::*;
use core_protocol::rpc::{ClientRequest, ClientUpdate};
use log::{error, info, trace, warn};
use rustrict::BlockReason;
use serde::{Deserialize, Serialize};
use server_util::observer::*;
use server_util::user_agent::UserAgent;
use std::collections::hash_map::Entry;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

const CLIENT_TIMER_MILLIS: u64 = 250;
const LEADERBOARD_TIMER_SECS: u64 = 1;
const TEAM_TIMER_SECS: u64 = 15;

#[derive(Serialize, Deserialize)]
pub struct ClientState {
    pub arena_id: Option<ArenaId>,
    pub newbie: bool,
    pub ip_addr: Option<IpAddr>,
    pub session_id: Option<SessionId>,
    pub user_agent_id: Option<UserAgentId>,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "Result<ClientUpdate, &'static str>")]
pub struct ParametrizedClientRequest {
    pub params: ClientState,
    pub request: ClientRequest,
}

fn log_err<O, E: std::fmt::Display>(res: Result<O, E>) {
    if let Err(e) = res {
        warn!("Error sending {}", e);
    }
}

impl Handler<ObserverMessage<ClientRequest, ClientUpdate, (Option<IpAddr>, Option<UserAgent>)>>
    for Core
{
    type Result = ResponseActFuture<Self, ()>;

    fn handle(
        &mut self,
        msg: ObserverMessage<ClientRequest, ClientUpdate, (Option<IpAddr>, Option<UserAgent>)>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ObserverMessage::Request { observer, request } => match request {
                // Handle asynchronous requests (i.e. those that may access database).
                ClientRequest::CreateInvitation => {
                    if let Some(client) = self.clients.get_mut(&observer) {
                        if let Some(arena_id) = client.arena_id {
                            if let Some(session_id) = client.session_id {
                                if let Some(invitation_id) =
                                    self.repo.create_invitation(arena_id, session_id)
                                {
                                    let message = ClientUpdate::InvitationCreated { invitation_id };
                                    log_err(observer.do_send(ObserverUpdate::Send { message }));
                                }
                            }
                        }
                    }
                }
                ClientRequest::CreateSession {
                    game_id,
                    invitation_id,
                    referrer,
                    saved_session_tuple,
                } => {
                    if let Some(data) = self.clients.get(&observer) {
                        if let Some(ip_addr) = data.ip_addr {
                            if self.session_rate_limiter.limit_rate(ip_addr) {
                                // Should only log IP of malicious actors.
                                warn!("IP {} was rate limited in create_session", ip_addr);
                                return Box::pin(fut::ready(()));
                            }
                        } else {
                            error!("client missing ip address in create_session");
                        }
                    } else {
                        error!("client not found in create_session");
                    }

                    info!(
                        "session rate limiter is tracking {} ip(s)",
                        self.session_rate_limiter.len()
                    );

                    // TODO: if invitation_id is not in cache then load it from DB and call self.repo.put_invitation(invitatation_id, invitation)
                    let found = self.repo.is_session_in_cache(saved_session_tuple);
                    info!("session cache hit={}", found);
                    return Box::pin(
                        async move {
                            if found {
                                // No need to load from database because session is in memory.
                                Result::Ok(None)
                            } else if let Some((arena_id, session_id)) = saved_session_tuple {
                                info!("reading session from DB");
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

                            if let Ok(Some(session_item)) = db_result {
                                info!("populating cache with session from DB {:?}", session_item);

                                let bot = false;
                                let mut session = Session::new(
                                    session_item.alias,
                                    session_item.arena_id,
                                    bot,
                                    session_item.date_previous,
                                    session_item.game_id,
                                    session_item.player_id,
                                    session_item.previous_id,
                                    session_item.referrer,
                                    Some(session_item.server_id),
                                    session_item.user_agent_id,
                                );
                                session.date_created = session_item.date_created;
                                session.date_renewed = session_item.date_renewed;
                                session.date_terminated = session_item.date_terminated;
                                session.previous_plays = session_item.plays;

                                act.repo.put_session(
                                    session_item.arena_id,
                                    session_item.session_id,
                                    session,
                                );
                            }

                            if let Some((arena_id, session_id, player_id, server_id)) =
                                act.repo.create_session(
                                    game_id,
                                    invitation_id,
                                    referrer,
                                    saved_session_tuple,
                                    client.user_agent_id,
                                )
                            {
                                info!("session was created!");

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
                                    server_id,
                                    session_id,
                                    player_id,
                                };
                                info!("notifying client about session");
                                log_err(
                                    observer.do_send(ObserverUpdate::Send { message: success }),
                                );
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
                            log_err(observer.do_send(ObserverUpdate::Send { message: success }));
                        }
                    }
                }
            },
            ObserverMessage::Register { observer, payload } => {
                if let Entry::Vacant(e) = self.clients.entry(observer) {
                    e.insert(ClientState {
                        arena_id: None,
                        newbie: true,
                        ip_addr: payload.0,
                        user_agent_id: parse_user_agent(payload.1),
                        session_id: None,
                    });
                }
            }
            ObserverMessage::Unregister { observer } => {
                self.clients.remove(&observer);
            }
        } // match msg

        // Do absolutely nothing, but do it asynchronously so the type system is happy.
        Box::pin(fut::ready(()))
    } // fn handle
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

impl Core {
    pub fn start_client_timers(&self, ctx: &mut <Self as Actor>::Context) {
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
                    log_err(addr.do_send(ObserverUpdate::Send {
                        message: ClientUpdate::RegionsUpdated {
                            added: Arc::clone(&regions),
                            removed: Arc::new([]),
                        },
                    }));

                    if let Some((
                        leaderboard_initializer,
                        liveboard_initializer,
                        message_initializer,
                        (player_count, player_initializer),
                        team_initializer,
                    )) = act.repo.get_initializers(client.arena_id.unwrap())
                    {
                        for (index, leaderboard) in
                            std::array::IntoIter::new(leaderboard_initializer).enumerate()
                        {
                            sent += 1;
                            log_err(addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::LeaderboardUpdated {
                                    leaderboard,
                                    period: index.into(),
                                },
                            }));
                        }

                        sent += 1;
                        log_err(addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::LiveboardUpdated {
                                added: liveboard_initializer.clone(),
                                removed: vec![].into(),
                            },
                        }));

                        sent += 1;
                        log_err(addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::MessagesUpdated {
                                added: Arc::clone(&message_initializer),
                            },
                        }));

                        sent += 1;
                        log_err(addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::PlayersUpdated {
                                count: player_count,
                                added: player_initializer.clone(),
                                removed: Arc::new([]),
                            },
                        }));

                        sent += 1;
                        log_err(addr.do_send(ObserverUpdate::Send {
                            message: ClientUpdate::TeamsUpdated {
                                added: team_initializer.clone(),
                                removed: Arc::new([]),
                            },
                        }));
                    }
                }
            }

            // Notify existing clients of any changes.
            if let Some((players_counted_added_or_removed, teams_added_or_removed)) =
                act.repo.read_broadcasts()
            {
                for (arena_id, (player_count, added, removed)) in
                    players_counted_added_or_removed.iter()
                {
                    found += 1;
                    for (addr, client) in act.clients.iter_mut() {
                        if let Some(client_arena_id) = client.arena_id {
                            if client_arena_id == *arena_id {
                                sent += 1;
                                log_err(addr.do_send(ObserverUpdate::Send {
                                    message: ClientUpdate::PlayersUpdated {
                                        count: *player_count,
                                        added: Arc::clone(added),
                                        removed: Arc::clone(removed),
                                    },
                                }));
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
                                log_err(addr.do_send(ObserverUpdate::Send {
                                    message: ClientUpdate::TeamsUpdated {
                                        added: Arc::clone(added),
                                        removed: Arc::clone(removed),
                                    },
                                }));
                            }
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
                            log_err(addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::JoinersUpdated {
                                    added: Arc::clone(&added),
                                    removed: Arc::clone(&removed),
                                },
                            }));
                        }

                        let (added, removed) = joins_added_or_removed;
                        if added.len() + removed.len() > 0 {
                            log_err(addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::JoinsUpdated {
                                    added: Arc::clone(&added),
                                    removed: Arc::clone(&removed),
                                },
                            }));
                        }

                        if messages_added.len() > 0 {
                            log_err(addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::MessagesUpdated {
                                    added: Arc::clone(&messages_added),
                                },
                            }));
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

        ctx.run_interval(Duration::from_secs(LEADERBOARD_TIMER_SECS), |act, _ctx| {
            for (arena_id, leaderboard, period) in act.repo.read_leaderboards() {
                for (addr, client) in act.clients.iter_mut() {
                    if client.newbie {
                        continue; // Will be initialized elsewhere.
                    }
                    if let Some(client_arena_id) = client.arena_id {
                        if client_arena_id == arena_id {
                            log_err(addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::LeaderboardUpdated {
                                    leaderboard: leaderboard.clone(),
                                    period,
                                },
                            }));
                        }
                    }
                }
            }

            for (arena_id, added, removed) in act.repo.read_liveboards() {
                for (addr, client) in act.clients.iter_mut() {
                    if client.newbie {
                        continue; // Will be initialized elsewhere.
                    }
                    if let Some(client_arena_id) = client.arena_id {
                        if client_arena_id == arena_id {
                            log_err(addr.do_send(ObserverUpdate::Send {
                                message: ClientUpdate::LiveboardUpdated {
                                    added: added.clone(),
                                    removed: removed.clone(),
                                },
                            }));
                        }
                    }
                }
            }
        }); // ctx.run_interval LEADERBOARD

        ctx.run_interval(Duration::from_secs(TEAM_TIMER_SECS), |act, _ctx| {
            act.repo.prune_arenas();
            act.repo.prune_sessions();
            act.repo.prune_teams();
            act.repo.prune_invitations();
        }); // ctx.run_interval TEAM
    }
}

impl Repo {
    pub fn handle_client_sync(
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
            ClientRequest::CreateInvitation => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if let Some(invitation_id) = self.create_invitation(arena_id, session_id) {
                        result = Ok(ClientUpdate::InvitationCreated { invitation_id });
                    }
                }
            }
            ClientRequest::CreateTeam { team_name } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    match self.create_team(arena_id, session_id, team_name) {
                        Ok(team_id) => result = Ok(ClientUpdate::TeamCreated { team_id }),
                        Err(e) => warn!("CreateTeam: {}", e),
                    }
                }
            }
            ClientRequest::IdentifySession { alias } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.identify_session(arena_id, session_id, alias) {
                        result = Ok(ClientUpdate::SessionIdentified { alias });
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
            ClientRequest::ReportPlayer { player_id } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    if self.report_player(arena_id, session_id, player_id) {
                        result = Ok(ClientUpdate::PlayerReported { player_id });
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
                    if let Some(chat_result) =
                        self.send_chat(arena_id, session_id, message.clone(), whisper)
                    {
                        let result_str = match chat_result {
                            Ok(_) => "ok",
                            Err(BlockReason::Inappropriate(_)) => "inappropriate",
                            Err(BlockReason::Unsafe(_)) => "unsafe",
                            Err(BlockReason::Repetitious(_)) => "repetitious",
                            Err(BlockReason::Spam(_)) => "spam",
                            Err(BlockReason::Muted(_)) => "muted",
                            Err(BlockReason::Empty) => "empty",
                            _ => "???",
                        };

                        if let Some(chat_log) = chat_log {
                            if let Some(arena) = Arena::get(&mut self.arenas, arena_id) {
                                if let Some(session) = arena.sessions.get(&session_id) {
                                    log_chat(
                                        chat_log,
                                        Some(arena.game_id),
                                        whisper,
                                        // player_id being Some means the message went through.
                                        result_str,
                                        session.alias,
                                        &message,
                                    );
                                }
                            }
                        }
                        result = Ok(ClientUpdate::ChatSent {
                            player_id: chat_result.ok(),
                        });
                    }
                }
            }
            ClientRequest::SubmitSurvey { survey: _ } => {}
            ClientRequest::TallyFps { fps } => {
                if let Some((arena_id, session_id)) = client.arena_id.zip(client.session_id) {
                    self.tally_fps(arena_id, session_id, fps);
                }
            }
            ClientRequest::Trace { message } => {
                println!("client_trace: {}", message);
                result = Ok(ClientUpdate::Traced)
            }
            _ => result = Err("cannot process client request synchronously"),
        }

        result
    }
}
