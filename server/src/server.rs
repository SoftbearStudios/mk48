// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::*;
use crate::player::*;
use crate::protocol::*;
use crate::world::World;
use actix::prelude::*;
use common::entity::EntityId;
use common::protocol::{Command, Update};
use common::terrain::ChunkSet;
use common::ticks::Ticks;
use core::core::Core;
use core::server::{ParametrizedServerRequest, ServerState};
use core_protocol::dto::{InvitationDto, RulesDto};
use core_protocol::id::*;
use core_protocol::name::Location;
use core_protocol::rpc::{ServerRequest, ServerUpdate};
use log::{debug, error, info, trace, warn};
use rayon::prelude::*;
use servutil::benchmark;
use servutil::benchmark::Timer;
use servutil::benchmark_scope;
use servutil::observer::{ObserverMessage, ObserverUpdate};
use std::collections::HashMap;
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A game server.
pub struct Server {
    core: Addr<Core>,
    arena_id: Option<ArenaId>,
    /// Real players with maybe disconnected sockets.
    pub clients: HashMap<Client, ClientData>,
    /// Bot players.
    pub bots: Vec<(Bot, SharedData)>,
    pub world: World,
    server_id: Option<ServerId>,
    counter: Ticks,
    min_players: usize,
}

/// The status of an player from the perspective of the core.
#[derive(Copy, Clone, Debug)]
pub struct CoreStatus {
    location: Location,
    score: u32,
}

impl Eq for CoreStatus {}
impl PartialEq for CoreStatus {
    fn eq(&self, other: &Self) -> bool {
        const THRESHOLD: f32 = 100.0;
        self.location.distance_squared(other.location) <= THRESHOLD.powi(2)
            && self.score == other.score
    }
}

/// Data shared by both real players and bots.
pub struct SharedData {
    pub player: Arc<PlayerTuple>,
    pub session_id: SessionId,
    /// If invited by a player, will store their id. Taken on the next spawn.
    pub invitation: Option<InvitationDto>,
    pub last_status: Option<CoreStatus>, // None == not playing.
}

/// Stores a player, and metadata related to it. Data stored here may only be accessed when processing,
/// this client (i.e. not when processing other entities). Bots don't use this.
pub struct ClientData {
    pub data: SharedData,
    pub chunk_loading_cooldown: Ticks,
    pub loaded_chunks: ChunkSet,
    /// Map of `EntityId` to `Ticks` until next send, considering keepalive.
    pub loaded_entities: HashMap<EntityId, Ticks>,
    pub limbo_expiry: Option<Instant>,
}

impl Server {
    /// How long a player can remain in limbo after they lose connection.
    const LIMBO: Duration = Duration::from_secs(6);

    /// new returns a game server with the specified parameters.
    pub fn new(server_id: Option<ServerId>, min_players: usize, core: Addr<Core>) -> Self {
        Self {
            core,
            world: World::new(World::target_radius(min_players, World::BOAT_DENSITY)),
            clients: HashMap::new(),
            bots: Vec::new(),
            counter: Ticks::ZERO,
            server_id,
            min_players,
            arena_id: None,
        }
    }

    /// tick runs one server tick.
    fn tick(&mut self, ctx: &mut <Self as Actor>::Context) {
        benchmark_scope!("tick");

        self.counter = self.counter.wrapping_add(Ticks::ONE);

        self.world.update(Ticks::ONE);

        // Pre-borrow one field to avoid borrowing entire self later.
        let core = self.core.to_owned();
        let addr = ctx.address();
        let world = &self.world;

        {
            benchmark_scope!("clients");

            self.clients
                .par_iter_mut()
                .for_each(|(client, client_data)| {
                    client_data.chunk_loading_cooldown = client_data
                        .chunk_loading_cooldown
                        .saturating_sub(Ticks::ONE);

                    if !client.connected() {
                        // In limbo or will be soon (not connected, cannot send an update).
                        return;
                    }

                    let update = world.get_player_complete(&client_data.data.player);
                    if let Err(e) = client.do_send(ObserverUpdate::Send {
                        message: update.into_update(
                            &mut client_data.loaded_entities,
                            &mut client_data.loaded_chunks,
                            &mut client_data.chunk_loading_cooldown,
                        ),
                    }) {
                        warn!("Error sending update to client: {}", e); // TODO: drop_session() !
                    }
                    Self::update_core_status(&core, &addr, world, &mut client_data.data);
                });
        }

        {
            benchmark_scope!("bots");

            let mut bot_actions = Vec::new();
            self.bots
                .par_iter_mut()
                .enumerate()
                .with_min_len(16)
                .map(|(i, (bot, shared_data))| {
                    let update = world.get_player_complete(&shared_data.player);
                    let bot_action = bot.update(update);
                    Self::update_core_status(&core, &addr, world, shared_data);
                    (i, bot_action)
                })
                .collect_into_vec(&mut bot_actions);

            for (i, (commands, quit)) in bot_actions.into_iter().rev() {
                if quit {
                    let shared_data = &self.bots[i].1;

                    self.world.remove_if(|e| {
                        e.player
                            .as_ref()
                            .map_or(false, |p| *p == shared_data.player)
                    });

                    self.bots.swap_remove(i);
                } else {
                    for c in commands {
                        let _ = c
                            .as_command()
                            .apply(&mut self.world, &mut self.bots[i].1, true);
                    }
                }
            }
        }

        if self.counter % Ticks::from_secs(5.0) == Ticks::ZERO {
            info!(
                "[{} entities]: {:?}",
                self.world.arena.total(),
                benchmark::borrow_all()
            );
        }

        // Generated any chunks queued for generation.
        self.world.terrain.reset_updated();

        // Regenerate modified chunks after a while.
        self.world.terrain.regenerate_if_applicable();

        self.flush_limbo(ctx);
    }

    /// Permanently removes clients that have expired from limbo.
    fn flush_limbo(&mut self, ctx: &mut <Self as Actor>::Context) {
        let now = Instant::now();
        for (_, mut client_data) in self.clients.drain_filter(|client, client_data| {
            !client.connected() && Some(now) > client_data.limbo_expiry
        }) {
            self.world.remove_if(|e| {
                e.player
                    .as_ref()
                    .map_or(false, |p| *p == client_data.data.player)
            });

            Self::update_core_status(
                &self.core,
                &ctx.address(),
                &self.world,
                &mut client_data.data,
            );

            // The above should have stopped any active play, reducing the last_status to None.
            assert_eq!(client_data.data.last_status, None);

            // Postpone core updates, so as to not terminate the session until expired from limbo.
            self.core
                .do_send(ObserverMessage::<ServerRequest, ServerUpdate, _>::Request {
                    observer: ctx.address().recipient(),
                    request: ServerRequest::DropSession {
                        session_id: client_data.data.session_id,
                    },
                });

            info!(
                "session {:?} expired from limbo",
                client_data.data.session_id
            );
        }
    }

    /// Updates the core with status changes (alive<->dead, score changes, and location changes).
    fn update_core_status(
        core: &Addr<Core>,
        addr: &Addr<Server>,
        world: &World,
        data: &mut SharedData,
    ) {
        let player = data.player.borrow();
        let player_entity = match &player.status {
            Status::Alive { entity_index, .. } => {
                let entity = &world.entities[*entity_index];
                Some(entity)
            }
            _ => None,
        };

        let new_status = player_entity.map(|e| CoreStatus {
            location: e.transform.position.extend(0.0),
            score: e.borrow_player().score,
        });

        if new_status == data.last_status {
            return;
        }

        core.do_send(ObserverMessage::<ServerRequest, ServerUpdate, _>::Request {
            observer: addr.to_owned().recipient(),
            request: match new_status {
                Some(_) if data.last_status.is_none() => ServerRequest::StartPlay {
                    session_id: data.session_id,
                },
                Some(status) => ServerRequest::SetStatus {
                    session_id: data.session_id,
                    location: Some(status.location),
                    score: Some(status.score),
                },
                None => ServerRequest::StopPlay {
                    session_id: data.session_id,
                },
            },
        });

        data.last_status = new_status;
    }
}

impl Actor for Server {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Server started");

        // TODO: Investigate whether this only affects performance or can affect correctness.
        ctx.set_mailbox_capacity(self.min_players + 50);

        let _ = self
            .core
            .send(
                ObserverMessage::<ServerRequest, ServerUpdate, _>::Register {
                    observer: ctx.address().recipient(),
                    payload: None,
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
                            game_id: GameId::Mk48,
                            region: RegionId::Usa,
                            rules: Some(RulesDto {
                                bot_min: self2.min_players as u32,
                                bot_percent: 50,
                                default_score: Some(0),
                                show_bots_on_liveboard: false,
                                team_size_max: 6,
                            }),
                            saved_arena_id: None,
                            server_id: self2.server_id,
                        },
                    })
                    .into_actor(self2)
            })
            .then(move |res, _self3, _ctx| {
                debug!("start arena resulted in {:?}", res);
                fut::ready(())
            })
            .wait(ctx);

        ctx.run_interval(Ticks::ONE.to_duration(), Self::tick);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        error!("server stopped");

        // A process without this actor running should be restarted immediately.
        process::exit(1);
    }
}

impl Handler<Authenticate> for Server {
    type Result = ResponseActFuture<Self, Option<(PlayerId, Option<InvitationDto>)>>;

    fn handle(&mut self, msg: Authenticate, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(
            self.core
                .send(ParametrizedServerRequest {
                    params: ServerState {
                        arena_id: self.arena_id, // By now, this is definitely Some.
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

impl Handler<ObserverMessage<Command, Update, (SessionId, PlayerId, Option<InvitationDto>)>>
    for Server
{
    type Result = ();

    fn handle(
        &mut self,
        msg: ObserverMessage<Command, Update, (SessionId, PlayerId, Option<InvitationDto>)>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ObserverMessage::Register { observer, payload } => {
                // Search for player in limbo.
                let limbo_client = self.clients.iter().find_map(|(client, client_data)| {
                    if client_data.data.session_id == payload.0
                        && client_data.data.player.borrow().player_id == payload.1
                    {
                        Some(client.clone())
                    } else {
                        None
                    }
                });
                if let Some(limbo_client) = limbo_client {
                    // If it still exists, old client is now retired.
                    let _ = limbo_client.do_send(ObserverUpdate::Close);

                    // Restore player from limbo.
                    let mut client_data = self.clients.remove(&limbo_client).unwrap();
                    info!("session {:?} restored from limbo", payload.1);

                    client_data.limbo_expiry = None;

                    // Don't assume client remembered chunks, although it should have.
                    client_data.loaded_chunks = ChunkSet::new();
                    client_data.chunk_loading_cooldown = Ticks::ZERO;

                    self.clients.insert(observer, client_data);
                } else {
                    // Create a new player.
                    self.clients.insert(
                        observer,
                        ClientData {
                            data: SharedData {
                                session_id: payload.0,
                                player: Arc::new(PlayerTuple::new(payload.1)),
                                last_status: None,
                                invitation: payload.2,
                            },
                            loaded_entities: HashMap::new(),
                            loaded_chunks: ChunkSet::new(),
                            chunk_loading_cooldown: Ticks::ZERO,
                            limbo_expiry: None,
                        },
                    );
                }
            }
            ObserverMessage::Unregister { observer } => {
                // The only legitimate reason for None would be a race condition in which
                //  1. Client A registers
                //  3. Client B registers with the same session and player so evicts client A from limbo
                //  2. Client A unregisters and is placed in limbo
                if let Some(client_data) = self.clients.get_mut(&observer) {
                    client_data.limbo_expiry = Some(Instant::now() + Self::LIMBO);
                    info!("session {:?} is in limbo", client_data.data.session_id);
                }
            }
            ObserverMessage::Request { request, observer } => {
                // The only legitimate reason for None is explained above.
                if let Some(client_data) = self.clients.get_mut(&observer) {
                    match request
                        .as_command()
                        .apply(&mut self.world, &mut client_data.data, false)
                    {
                        Ok(_) => {}
                        Err(e) => {
                            warn!("command error: {}", e);
                        }
                    }
                }
            }
        }
    }
}

impl Handler<ObserverUpdate<ServerUpdate>> for Server {
    type Result = ();

    fn handle(
        &mut self,
        update: ObserverUpdate<ServerUpdate>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trace!("Game server received server update: {:?}", update);
        match update {
            ObserverUpdate::Send { message } => match message {
                ServerUpdate::ArenaStarted { arena_id } => {
                    self.arena_id = Some(arena_id);
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
                        for (_, client_data) in self.clients.iter_mut() {
                            let mut player = client_data.data.player.borrow_mut();
                            if player.player_id == change.player_id {
                                player.team_id = change.team_id;
                            }
                        }
                    }
                }
                ServerUpdate::BotReady {
                    session_id,
                    player_id,
                } => {
                    if !self
                        .bots
                        .iter()
                        .any(|(_, data)| data.session_id == session_id)
                    {
                        self.bots.push((
                            Bot::new(),
                            SharedData {
                                player: Arc::new(PlayerTuple::new(player_id)),
                                session_id,
                                last_status: None,
                                invitation: None,
                            },
                        ))
                    }
                }
            },
            _ => {}
        }
    }
}
