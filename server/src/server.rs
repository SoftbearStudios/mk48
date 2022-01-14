// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::*;
use crate::complete_ref::CompleteRef;
use crate::contact_ref::ContactRef;
use crate::entity_extension::EntityExtension;
use crate::player::*;
use crate::protocol::*;
use crate::world::World;
use crate::world_mutation::Mutation;
use common::entity::EntityType;
use common::protocol::{Command, Update};
use common::terrain::ChunkSet;
use common::ticks::Ticks;
use core_protocol::id::*;
use game_server::context::{CoreStatus, PlayerTuple};
use game_server::game_service::GameArenaService;
use log::{info, warn};
use server_util::benchmark;
use server_util::benchmark::Timer;
use server_util::benchmark_scope;
use std::cell::UnsafeCell;
use std::sync::Arc;
use std::time::Duration;

/// A game server.
pub struct Server {
    pub world: World,
}

/// Stores a player, and metadata related to it. Data stored here may only be accessed when processing,
/// this client (i.e. not when processing other entities). Bots don't use this.
#[derive(Default)]
pub struct ClientData {
    pub loaded_chunks: ChunkSet,
}

#[derive(Default)]
pub struct PlayerExtension(pub UnsafeCell<EntityExtension>);

/// This is sound because access is limited to when the entity is in scope.
unsafe impl Send for PlayerExtension {}
unsafe impl Sync for PlayerExtension {}

impl GameArenaService for Server {
    const GAME_ID: GameId = GameId::Mk48;

    /// How long a player can remain in limbo after they lose connection.
    const LIMBO: Duration = Duration::from_secs(6);

    type Bot = Bot;
    type ClientData = ClientData;
    type ClientUpdate = Update;
    type Command = Command;
    type PlayerData = Player;
    type PlayerExtension = PlayerExtension;
    type BotUpdate<'a> = CompleteRef<'a, impl Iterator<Item = ContactRef<'a>>>;

    /// new returns a game server with the specified parameters.
    fn new(min_players: usize) -> Self {
        Self {
            world: World::new(World::target_radius(
                min_players as f32 * EntityType::FairmileD.data().visual_area(),
            )),
        }
    }

    fn player_command(&mut self, update: Self::Command, player: &Arc<PlayerTuple<Self>>) {
        if let Err(e) = update.as_command().apply(&mut self.world, player) {
            warn!("Command resulted in {}", e);
        }
    }

    fn player_changed_team(
        &mut self,
        player_tuple: &Arc<PlayerTuple<Self>>,
        old_team: Option<TeamId>,
    ) {
        if old_team.is_some() {
            player_tuple
                .borrow_player_mut()
                .data
                .flags
                .left_populated_team = true;
        }
    }

    fn player_left_game(&mut self, player_tuple: &Arc<PlayerTuple<Self>>) {
        let borrow = player_tuple.borrow_player();
        if let Status::Alive { entity_index, .. } = borrow.data.status {
            drop(borrow);
            Mutation::boat_died(&mut self.world, entity_index, true);
        } else {
            drop(borrow);
        }

        // Delete all player's entities (efficiently, in the next update cycle).
        player_tuple.borrow_player_mut().data.flags.left_game = true;
    }

    fn get_client_update(
        &self,
        counter: Ticks,
        player: &Arc<PlayerTuple<Self>>,
        client_data: &mut Self::ClientData,
    ) -> Option<Self::ClientUpdate> {
        Some(
            self.world
                .get_player_complete(player)
                .into_update(counter, &mut client_data.loaded_chunks),
        )
    }

    fn get_bot_update<'a>(
        &'a self,
        _counter: Ticks,
        player: &'a Arc<PlayerTuple<Self>>,
    ) -> Self::BotUpdate<'a> {
        self.world.get_player_complete(player)
    }

    fn get_core_status(&self, player_tuple: &Arc<PlayerTuple<Self>>) -> Option<CoreStatus> {
        let player = player_tuple.borrow_player();

        // Don't get player if it just left the game.
        let player_entity = (!player.data.flags.left_game)
            .then(|| ())
            .and_then(|_| match &player.data.status {
                Status::Alive { entity_index, .. } => {
                    let entity = &self.world.entities[*entity_index];
                    Some(entity)
                }
                _ => None,
            });

        player_entity.map(|e| CoreStatus {
            location: e.transform.position.extend(0.0),
            score: e.borrow_player().data.score,
        })
    }

    /// tick runs one server tick.
    fn update(&mut self, ticks: Ticks, counter: Ticks) {
        benchmark_scope!("tick");

        self.world.update(ticks);

        // Needs to be called before clients receive updates, but after World::update.
        self.world.terrain.pre_update();

        if counter % Ticks::from_secs(5.0) == Ticks::ZERO {
            info!(
                "[{} entities]: {:?}",
                self.world.arena.total(),
                benchmark::borrow_all()
            );

            benchmark::reset_all();
        }

        //self.flush_limbo(ctx);
    }

    fn post_update(&mut self) {
        // Needs to be after clients receive updates.
        self.world.terrain.post_update();
    }
}
