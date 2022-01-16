// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::core::*;
use crate::repo::*;
use actix::prelude::*;
use core_protocol::id::*;
use core_protocol::rpc::{ServerRequest, ServerUpdate};
use log::warn;
use serde::{Deserialize, Serialize};
use server_util::observer::*;
use server_util::user_agent::UserAgent;
use std::collections::hash_map::Entry;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

const SERVER_TIMER_MILLIS: u64 = 250;

#[derive(Serialize, Deserialize)]
pub struct ServerState {
    pub arena_id: Option<ArenaId>,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "Result<ServerUpdate, &'static str>")]
pub struct ParametrizedServerRequest {
    pub params: ServerState,
    pub request: ServerRequest,
}

fn log_err<O, E: std::fmt::Display>(res: Result<O, E>) {
    if let Err(e) = res {
        warn!("Error sending {}", e);
    }
}

impl Handler<ObserverMessage<ServerRequest, ServerUpdate, (Option<IpAddr>, Option<UserAgent>)>>
    for Core
{
    type Result = ();
    fn handle(
        &mut self,
        msg: ObserverMessage<ServerRequest, ServerUpdate, (Option<IpAddr>, Option<UserAgent>)>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ObserverMessage::Request { observer, request } => {
                if let Some(server) = self.servers.get_mut(&observer) {
                    let result = self.repo.handle_server(server, request);
                    if let Ok(success) = result {
                        log_err(observer.do_send(ObserverUpdate::Send { message: success }))
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

impl Core {
    pub fn start_server_timers(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_millis(SERVER_TIMER_MILLIS), |act, _ctx| {
            // Notify existing servers of any changes.
            if let Some(server_updates) = act.repo.read_server_updates() {
                for (arena_id, team_assignments) in server_updates.iter() {
                    for (addr, server) in act.servers.iter_mut() {
                        if let Some(server_arena_id) = server.arena_id {
                            if server_arena_id == *arena_id {
                                log_err(addr.do_send(ObserverUpdate::Send {
                                    message: ServerUpdate::MembersChanged {
                                        changes: Arc::clone(team_assignments), // TODO: only used once; should use Box.
                                    },
                                }));
                            }
                        }
                    }
                }
            }

            // Notify servers of armageddon.
            if act.repo.read_armageddon() {
                for (addr, server) in act.servers.iter() {
                    if let Some(arena_id) = server.arena_id {
                        log_err(addr.do_send(ObserverUpdate::Send {
                            message: ServerUpdate::ArmageddonStarted { arena_id },
                        }))
                    }
                }
            }
        }); // ctx.run_interval
    }
}

impl Repo {
    fn handle_server(
        &mut self,
        server: &mut ServerState,
        request: ServerRequest,
    ) -> Result<ServerUpdate, &'static str> {
        let mut result = Err("server request failed");
        match request {
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
                server_id,
            } => {
                server.arena_id =
                    Some(self.start_arena(game_id, region, rules, saved_arena_id, server_id));
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
                    if let Some((elapsed, invitation, player_id, score)) =
                        self.validate_session(arena_id, session_id)
                    {
                        result = Ok(ServerUpdate::SessionValid {
                            elapsed,
                            player_id,
                            score,
                            invitation,
                        });
                    }
                }
            }
            ServerRequest::TallyUps { ups } => {
                if let Some(arena_id) = server.arena_id {
                    self.tally_ups(arena_id, ups);
                }
            }
        }

        result
    }
}
