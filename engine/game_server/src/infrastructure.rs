// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::BotZoo;
use crate::context::Context;
use crate::context::PlayerData;
use crate::context::{ClientAddr, ClientData, CoreStatus, PlayerTuple};
use crate::game_service::GameArenaService;
use crate::protocol::Authenticate;
use actix::AsyncContext;
use actix::{
    Actor, ActorFutureExt, Addr, Context as ActorContext, ContextFutureSpawner, Handler,
    ResponseActFuture, WrapFuture,
};
use common_util::ticks::Ticks;
use core_protocol::dto::InvitationDto;
use core_protocol::id::{PlayerId, RegionId, ServerId, SessionId};
use core_protocol::rpc::{ServerRequest, ServerUpdate};
use core_server::core::Core;
use core_server::server::{ParametrizedServerRequest, ServerState};
use log::trace;
use log::{debug, error, info, warn};
use rayon::prelude::*;
use server_util::observer::{ObserverMessage, ObserverUpdate};
use server_util::ups_monitor::UpsMonitor;
use std::collections::HashMap;
use std::process;
use std::time::Instant;

pub struct Infrastructure<G: GameArenaService> {
    context: Context<G>,
    service: G,
    core: Addr<Core>,
    server_id: Option<ServerId>,
    ups_monitor: UpsMonitor,
}

impl<G: GameArenaService> Actor for Infrastructure<G> {
    type Context = ActorContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Server started");

        // TODO: Investigate whether this only affects performance or can affect correctness.
        ctx.set_mailbox_capacity(50);

        let _ = self
            .core
            .send(
                ObserverMessage::<ServerRequest, ServerUpdate, _>::Register {
                    observer: ctx.address().recipient(),
                    payload: (None, None),
                },
            )
            .into_actor(self)
            .then(move |res, self2, ctx| {
                debug!("register resulted in {:?}", res);
                self2
                    .core
                    .send(ObserverMessage::<ServerRequest, ServerUpdate, _>::Request {
                        observer: ctx.address().recipient(),
                        request: ServerRequest::StartArena {
                            game_id: G::GAME_ID,
                            region: RegionId::Usa,
                            rules: Some(self2.service.get_rules()),
                            saved_arena_id: None,
                            server_id: self2.server_id,
                        },
                    })
                    .into_actor(self2)
            })
            .then(move |res, _self3, _ctx| {
                debug!("start arena resulted in {:?}", res);
                actix::fut::ready(())
            })
            .wait(ctx);

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
    pub fn new(
        service: G,
        server_id: Option<ServerId>,
        min_players: usize,
        core: Addr<Core>,
    ) -> Self {
        Self {
            core,
            server_id,
            context: Context {
                arena_id: None,
                counter: Ticks::ZERO,
                clients: HashMap::new(),
                bots: BotZoo::new(min_players, if min_players == 0 { 0 } else { 80 }),
            },
            ups_monitor: UpsMonitor::new(),
            service,
        }
    }

    pub fn update(&mut self, ctx: &mut <Infrastructure<G> as Actor>::Context) {
        self.context.counter = self.context.counter.wrapping_add(Ticks::ONE);

        self.context
            .bots
            .update_count(self.context.clients.len(), &mut self.service);

        self.service.update(Ticks::ONE, self.context.counter);

        let core = &self.core;
        let addr = ctx.address();
        let counter = self.context.counter;

        {
            let service = &self.service;

            self.context.clients.par_iter_mut().for_each(
                |(client, client_data): (&ClientAddr<G>, &mut ClientData<G>)| {
                    if client.connected() {
                        // In limbo or will be soon (not connected, cannot send an update).
                        if let Some(update) = service.get_client_update(
                            counter,
                            &client_data.player_tuple,
                            &mut client_data.data,
                        ) {
                            if let Err(e) = client.do_send(ObserverUpdate::Send { message: update })
                            {
                                warn!("Error sending update to client: {}", e); // TODO: drop_session() !
                            }
                        }
                    }

                    let core_status = service.get_core_status(&client_data.player_tuple);
                    Self::update_core_status(
                        core,
                        &addr,
                        client_data.session_id,
                        &mut client_data.last_status,
                        core_status,
                    );
                },
            );
        }

        self.context.bots.update(counter, &mut self.service);

        self.service.post_update();

        self.flush_limbo(ctx);

        if let Some(ups) = self.ups_monitor.update() {
            self.core.do_send(ObserverMessage::Request {
                observer: ctx.address().recipient(),
                request: ServerRequest::TallyUps { ups },
            })
        }
    }

    /// Updates the core with status changes (alive<->dead, score changes, and location changes).
    fn update_core_status(
        core: &Addr<Core>,
        addr: &Addr<Self>,
        session_id: SessionId,
        last_status: &mut Option<CoreStatus>,
        new_status: Option<CoreStatus>,
    ) {
        if &new_status == last_status {
            return;
        }

        core.do_send(ObserverMessage::<ServerRequest, ServerUpdate, _>::Request {
            observer: addr.to_owned().recipient(),
            request: match new_status {
                Some(_) if last_status.is_none() => ServerRequest::StartPlay { session_id },
                Some(status) => ServerRequest::SetStatus {
                    session_id,
                    location: Some(status.location),
                    score: Some(status.score),
                },
                None => ServerRequest::StopPlay { session_id },
            },
        });

        *last_status = new_status;
    }

    fn flush_limbo(&mut self, ctx: &mut <Self as Actor>::Context) {
        let now = Instant::now();
        for (_, mut client_data) in self.context.clients.drain_filter(|client, client_data| {
            // Note: Do not compare Some(now) > client_data.limbo_expiry, as this will return true
            // if limbo_expiry is None.
            !client.connected()
                && client_data
                    .limbo_expiry
                    .map(|exp| now >= exp)
                    .unwrap_or(false)
        }) {
            self.service.player_left(&client_data.player_tuple);

            Self::update_core_status(
                &self.core,
                &ctx.address(),
                client_data.session_id,
                &mut client_data.last_status,
                None, // TODO: Ideally get from service??
            );

            // The above should have stopped any active play, reducing the last_status to None.
            assert_eq!(client_data.last_status, None);

            // Postpone core updates, so as to not terminate the session until expired from limbo.
            self.core
                .do_send(ObserverMessage::<ServerRequest, ServerUpdate, _>::Request {
                    observer: ctx.address().recipient(),
                    request: ServerRequest::DropSession {
                        session_id: client_data.session_id,
                    },
                });

            info!("session {:?} expired from limbo", client_data.session_id);
        }
    }
}

impl<G: GameArenaService> Handler<Authenticate> for Infrastructure<G> {
    type Result = ResponseActFuture<Self, Option<(PlayerId, Option<InvitationDto>)>>;

    fn handle(&mut self, msg: Authenticate, _ctx: &mut ActorContext<Self>) -> Self::Result {
        Box::pin(
            self.core
                .send(ParametrizedServerRequest {
                    params: ServerState {
                        arena_id: self.context.arena_id, // By now, this is definitely Some.
                    },
                    request: ServerRequest::ValidateSession {
                        session_id: msg.session_id,
                    },
                })
                .into_actor(self) // converts future to ActorFuture
                .map(|res, _act, _ctx| {
                    match res {
                        Ok(res) => match res {
                            Ok(update) => match update {
                                ServerUpdate::SessionValid {
                                    player_id,
                                    invitation,
                                    ..
                                } => Some((player_id, invitation)),
                                _ => panic!("incorrect response type"),
                            },
                            Err(_) => None,
                        },
                        Err(e) => {
                            error!("authenticate actix: {}", e);
                            None
                        } // actix error
                    }
                }),
        )
    }
}

impl<G: GameArenaService>
    Handler<
        ObserverMessage<G::Command, G::ClientUpdate, (SessionId, PlayerId, Option<InvitationDto>)>,
    > for Infrastructure<G>
{
    type Result = ();

    fn handle(
        &mut self,
        msg: ObserverMessage<
            G::Command,
            G::ClientUpdate,
            (SessionId, PlayerId, Option<InvitationDto>),
        >,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        match msg {
            ObserverMessage::Register { observer, payload } => {
                // Search for player in limbo. TODO: N^2
                let limbo_client = self.context.clients.iter().find_map(
                    |(client, client_data): (&ClientAddr<G>, &ClientData<G>)| {
                        if client_data.session_id == payload.0
                            && client_data.player_tuple.player.borrow_mut().player_id == payload.1
                        {
                            Some(client.clone())
                        } else {
                            None
                        }
                    },
                );
                if let Some(limbo_client) = limbo_client {
                    // If it still exists, old client is now retired.
                    let _ = limbo_client.do_send(ObserverUpdate::Close);

                    // Restore player from limbo.
                    let mut client_data = self.context.clients.remove(&limbo_client).unwrap();
                    info!("session {:?} restored from limbo", payload.1);

                    client_data.limbo_expiry = None;

                    // Don't assume client remembered chunks, although it should have.
                    client_data.data = G::ClientData::default();

                    self.context.clients.insert(observer, client_data);
                } else {
                    // Create a new player.
                    let client_data = ClientData::new(
                        payload.0,
                        PlayerTuple::new(PlayerData::new(payload.1, payload.2)),
                    );

                    self.service.player_joined(&client_data.player_tuple);

                    self.context.clients.insert(observer, client_data);
                }
            }
            ObserverMessage::Unregister { observer } => {
                // The only legitimate reason for None would be a race condition in which
                //  1. Client A registers
                //  3. Client B registers with the same session and player so evicts client A from limbo
                //  2. Client A unregisters and is placed in limbo
                if let Some(client_data) = self.context.clients.get_mut(&observer) {
                    client_data.limbo_expiry = Some(Instant::now() + G::LIMBO);
                    info!("session {:?} is in limbo", client_data.session_id);
                }
            }
            ObserverMessage::Request { request, observer } => {
                // The only legitimate reason for None is explained above.
                if let Some(client_data) = self.context.clients.get_mut(&observer) {
                    self.service
                        .player_command(request, &client_data.player_tuple);
                }
            }
            _ => {}
        }
    }
}

impl<G: GameArenaService> Handler<ObserverUpdate<ServerUpdate>> for Infrastructure<G> {
    type Result = ();

    fn handle(
        &mut self,
        update: ObserverUpdate<ServerUpdate>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trace!("Game server received server update: {:?}", update);
        if let ObserverUpdate::Send { message } = update {
            match message {
                ServerUpdate::ArenaStarted { arena_id } => {
                    self.context.arena_id = Some(arena_id);
                }
                ServerUpdate::ArmageddonStarted { .. } => {}
                ServerUpdate::ArenaStopped => {}
                ServerUpdate::PlayStarted { .. } => {}
                ServerUpdate::PlayStopped => {}
                ServerUpdate::SessionDropped => {}
                ServerUpdate::SessionValid { .. } => {}
                ServerUpdate::StatusSet => {}
                ServerUpdate::MembersChanged { changes } => {
                    for change in changes.iter() {
                        for (_, client_data) in self.context.clients.iter_mut() {
                            let mut player = client_data.player_tuple.player.borrow_mut();
                            if player.player_id == change.player_id {
                                if change.team_id != player.team_id {
                                    let old_team = player.team_id;
                                    player.team_id = change.team_id;
                                    drop(player);
                                    self.service
                                        .player_changed_team(&client_data.player_tuple, old_team)
                                }
                                break;
                            }
                        }
                    }
                    /*
                    for change in changes.iter() {
                        if let Some(player) = self
                            .context
                            .players
                            .get_mut(&change.player_id)
                            .as_deref_mut()
                        {
                            player.team_id = change.team_id;
                        }
                    }
                     */
                }
            }
        }
    }
}
