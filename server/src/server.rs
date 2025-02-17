// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::*;
use crate::entity_extension::EntityExtension;
use crate::player::*;
use crate::protocol::*;
use crate::team::TeamRepo;
use crate::terrain_pool::improve_terrain_pool;
use crate::world::World;
use common::entity::EntityData;
use common::entity::EntityType;
use common::protocol::TeamDto;
use common::protocol::TeamUpdate;
use common::protocol::{Command, Update};
use common::terrain::ChunkSet;
use common::ticks::Ticks;
use common::util::level_to_score;
use common::MK48_CONSTANTS;
use kodiak_server::log::{error, info};
use kodiak_server::rand::{thread_rng, Rng};
use kodiak_server::{
    map_ranges, ArenaContext, ArenaService, GameConstants, Player, PlayerAlias, PlayerId, Score,
    TeamId, TeamName,
};
use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

/// A game server.
pub struct Server {
    pub world: World,
    pub counter: Ticks,
    pub player: PlayerTupleRepo,
    pub team: TeamRepo<Self>,
    /// update -> get_client_update.
    team_update: Option<(Arc<[TeamDto]>, Arc<[TeamId]>)>,
    free_points: u32,
}

/// Stores a player, and metadata related to it. Data stored here may only be accessed when processing,
/// this client (i.e. not when processing other entities). Bots don't use this.
#[derive(Default, Debug)]
pub struct ClientData {
    pub loaded_chunks: ChunkSet,
    pub team_initialized: bool,
}

#[derive(Default)]
pub struct PlayerExtension(pub UnsafeCell<EntityExtension>);

/// This is sound because access is limited to when the entity is in scope.
unsafe impl Send for PlayerExtension {}
unsafe impl Sync for PlayerExtension {}

impl ArenaService for Server {
    const GAME_CONSTANTS: &'static GameConstants = MK48_CONSTANTS;
    const TICK_PERIOD_SECS: f32 = Ticks::PERIOD_SECS;

    /// How long a player can remain in limbo after they lose connection.
    const LIMBO: Duration = Duration::from_secs(6);

    type Bot = Bot;
    type ClientData = ClientData;
    type GameUpdate = Update;
    type GameRequest = Command;

    /// new returns a game server with the specified parameters.
    fn new(context: &mut ArenaContext<Self>) -> Self {
        Self {
            world: World::new(World::target_radius(
                context.min_players() as f32 * EntityType::FairmileD.data().visual_area(),
            )),
            counter: Ticks::ZERO,
            player: PlayerTupleRepo::default(),
            team: TeamRepo::default(),
            team_update: None,
            free_points: if context.topology.local_arena_id.realm_id.is_temporary()
                || context.topology.local_arena_id.realm_id.is_named()
            {
                level_to_score(3)
            } else if cfg!(debug_assertions) {
                level_to_score(EntityData::MAX_BOAT_LEVEL)
            } else {
                0
            },
        }
    }

    fn player_joined(&mut self, player_id: PlayerId, engine_player: &mut Player<Self>) {
        if !self.player.contains(player_id) {
            self.player.insert(
                player_id,
                Arc::new(PlayerTuple::new(TempPlayer::new(
                    player_id,
                    engine_player.rank(),
                ))),
            )
        }
        let mut player = self.player.borrow_player_mut(player_id).unwrap();
        if !player.is_bot() {
            player.score = self.free_points;
        } else if cfg!(debug_assertions) {
            player.score = thread_rng().gen_range(0..=self.free_points);
        };
        player.flags.left_game = false;
    }

    fn player_command(
        &mut self,
        update: Self::GameRequest,
        player_id: PlayerId,
        engine_player: &mut Player<Self>,
    ) -> Option<Update> {
        let player_tuple = self.player.get(player_id).unwrap();
        if let Err(e) = update.as_command().apply(
            &mut self.world,
            &player_tuple,
            &self.player,
            &mut self.team,
            engine_player.invitation_accepted().cloned(),
            engine_player.rank(),
        ) {
            info!("Command resulted in {}", e);
        }
        None
    }

    fn player_quit(&mut self, player_id: PlayerId, _player: &mut Player<Self>) {
        let mut player = self.player.borrow_player_mut(player_id).unwrap();

        // If not dead, killing entity will be sufficient.
        if player.status.is_dead() {
            player.status = Status::Spawning;
        }
        // Clear player's score.
        player.score = 0;
        // Delete all player's entities (efficiently, in the next update cycle).
        player.flags.left_game = true;
    }

    fn player_left(&mut self, player_id: PlayerId, _player: &mut Player<Self>) {
        self.player.forget(player_id, &mut self.team);
    }

    fn get_game_update(
        &self,
        player_id: PlayerId,
        player: &mut Player<Self>,
    ) -> Option<Self::GameUpdate> {
        let player_tuple = self.player.get(player_id).unwrap();
        let client = player.client_mut().unwrap();
        let client = client.data_mut()?;
        let player_team_update = self.team.player_delta(player_id, &self.player).unwrap();
        let team_update = {
            let mut ret = Vec::new();
            if !client.team_initialized {
                player_tuple.borrow_player_mut().team.forget_state();
                if let Some(initializer) = self.team.initializer() {
                    ret.push(initializer);
                }
                client.team_initialized = true;
            }
            let (members, joiners, joins) = &player_team_update;
            // TODO: We could get members on a per team basis.
            if let Some(members) = members {
                ret.push(TeamUpdate::Members(members.deref().clone().into()));
            }

            if let Some(joiners) = joiners {
                ret.push(TeamUpdate::Joiners(joiners.deref().clone().into()));
            }

            if let Some(joins) = joins {
                ret.push(TeamUpdate::Joins(joins.iter().cloned().collect()));
            }

            if let Some((added, removed)) = self.team_update.as_ref() {
                if !added.is_empty() {
                    ret.push(TeamUpdate::AddedOrUpdated(Arc::clone(added)))
                }
                if !removed.is_empty() {
                    ret.push(TeamUpdate::Removed(Arc::clone(removed)))
                }
            }
            ret
        };
        Some(self.world.get_player_complete(player_tuple).into_update(
            self.counter,
            team_update,
            &mut client.loaded_chunks,
        ))
    }

    fn is_alive(&self, player_id: PlayerId) -> bool {
        let player = self.player.borrow_player(player_id).unwrap();
        !player.flags.left_game && player.status.is_alive()
    }

    fn get_team_id(&self, player_id: PlayerId) -> Option<TeamId> {
        self.player.borrow_player(player_id).unwrap().team_id()
    }

    fn get_team_name(&self, player_id: PlayerId) -> Option<TeamName> {
        self.player
            .borrow_player(player_id)
            .unwrap()
            .team_id()
            .map(|team_id| self.team.get(team_id).unwrap().name)
    }

    fn get_team_members(&self, player_id: PlayerId) -> Option<Vec<PlayerId>> {
        self.player
            .borrow_player(player_id)
            .unwrap()
            .team_id()
            .map(|team_id| self.team.get(team_id).unwrap().members.clone().into_inner())
    }

    fn get_alias(&self, player_id: PlayerId) -> PlayerAlias {
        self.player.borrow_player(player_id).unwrap().alias
    }

    fn override_alias(&mut self, player_id: PlayerId, alias: PlayerAlias) {
        self.player.borrow_player_mut(player_id).unwrap().alias = alias;
    }

    fn get_score(&self, player_id: PlayerId) -> Score {
        let status = self.player.borrow_player(player_id).unwrap();
        if status.is_alive() {
            Score::Some(status.score)
        } else {
            Score::None
        }
    }

    /// update runs server ticks.
    fn tick(&mut self, context: &mut ArenaContext<Self>) {
        if context.topology.local_arena_id.realm_id.is_public_default() {
            improve_terrain_pool();
        }

        self.counter = self.counter.next();

        self.world.update(Ticks::ONE, &mut |killer, dead| {
            context.tally_victory(killer, dead)
        });

        // Needs to be called before clients receive updates, but after World::update.
        self.world.terrain.pre_update();

        if self.counter.every(Ticks::from_whole_secs(60)) {
            use std::collections::{BTreeMap, HashMap};
            use std::fs::OpenOptions;
            use std::io::{Read, Seek, Write};

            let mut count_score = HashMap::<EntityType, (usize, f32)>::new();

            for player in self.player.iter_borrow() {
                if let Status::Alive { entity_index, .. } = player.status {
                    let entity = &self.world.entities[entity_index];
                    debug_assert!(entity.is_boat());

                    let (current_count, current_score) =
                        count_score.entry(entity.entity_type).or_default();
                    *current_count += 1;

                    let level = entity.data().level;
                    let level_score = level_to_score(level);
                    let next_level_score = level_to_score(level + 1);
                    let progress = map_ranges(
                        player.score as f32,
                        level_score as f32..next_level_score as f32,
                        0.0..1.0,
                        false,
                    );
                    if progress.is_finite() {
                        *current_score += progress;
                    }
                }
            }

            tokio::task::spawn_blocking(move || {
                if let Err(e) = OpenOptions::new()
                    .create(true)
                    .read(true)
                    .write(true)
                    .open(&*"playtime.json")
                    .and_then(move |mut file| {
                        let mut buf = Vec::new();
                        file.read_to_end(&mut buf)?;
                        let mut old = if let Ok(old) =
                            serde_json::from_slice::<BTreeMap<Cow<'static, str>, (u64, f32)>>(&buf)
                        {
                            old
                        } else {
                            error!("error loading old playtime.");
                            BTreeMap::new()
                        };

                        for (entity_type, (new_count, new_score)) in count_score {
                            if new_count > 0 {
                                let string: &'static str = entity_type.into();
                                let (old_count, old_score) =
                                    old.entry(Cow::Borrowed(string)).or_default();
                                *old_count = old_count.saturating_add(new_count as u64);
                                *old_score += new_score;
                            }
                        }

                        file.set_len(0)?;
                        file.rewind()?;

                        let serialized = serde_json::to_vec(&old).unwrap_or_default();
                        file.write_all(&serialized)
                    })
                {
                    error!("error logging playtime: {:?}", e);
                }
            });
        }

        self.team_update = self.team.delta(&self.player);
    }

    fn post_update(&mut self, context: &mut ArenaContext<Self>) {
        // Needs to be after clients receive updates.
        self.world.terrain.post_update();
        self.team_update = None;
        for mut player in self.player.iter_borrow_mut() {
            if player.team.team_id() != player.team.previous_team_id {
                if player.team.previous_team_id.is_some() {
                    player.flags.left_populated_team = true;
                }
                player.team.previous_team_id = player.team.team_id();
            }
        }
        self.player.real_players_live = context.players.real_players_live;
    }

    fn entities(&self) -> usize {
        self.world.arena.count_all()
    }

    fn world_size(&self) -> f32 {
        self.world.radius
    }
}
