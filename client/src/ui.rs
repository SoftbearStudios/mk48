// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use client_util::apply::Apply;
use client_util::context::Context;
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::{EntityKind, EntitySubKind, EntityType};
use common::velocity::Velocity;
use common::world::outside_area;
use core_protocol::id::{InvitationId, PeriodId, PlayerId, ServerId, TeamId};
use core_protocol::name::{PlayerAlias, TeamName};
use glam::{vec2, Vec2};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State of UI inputs.
pub struct UiState {
    pub active: bool,
    pub altitude_target: Altitude,
    pub armament: Option<(EntityKind, EntitySubKind)>,
    pub cinematic: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active: true,
            altitude_target: Altitude::ZERO,
            armament: None,
            cinematic: false,
        }
    }
}

#[derive(Deserialize)]
pub enum UiEvent {
    Spawn {
        alias: String,
        #[serde(rename = "entityType")]
        entity_type: EntityType,
    },
    Upgrade(EntityType),
    /// Sensors active.
    Active(bool),
    /// Normalized altitude target.
    AltitudeTarget(f32),
    Armament(EntityKind, EntitySubKind),
    Cinematic(bool),
    /// Go from respawning to spawning.
    OverrideRespawn,
}

impl Apply<UiEvent> for UiState {
    fn apply(&mut self, update: UiEvent) {
        match update {
            UiEvent::Active(active) => self.active = active,
            UiEvent::AltitudeTarget(altitude_target) => {
                self.altitude_target = Altitude::from_norm(altitude_target)
            }
            UiEvent::Armament(kind, sub_kind) => self.armament = Some((kind, sub_kind)),
            UiEvent::Cinematic(cinematic) => self.cinematic = cinematic,
            _ => {}
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiProps {
    pub player_id: Option<PlayerId>,
    pub team_name: Option<TeamName>,
    pub invitation_id: Option<InvitationId>,
    pub score: u32,
    pub player_count: usize,
    pub fps: f32,
    pub status: UiStatus,
    pub chats: Vec<ChatModel>,
    pub liveboard: Vec<LeaderboardItemModel>,
    pub leaderboards: HashMap<PeriodId, Vec<LeaderboardItemModel>>,
    pub team_captain: bool,
    pub team_full: bool,
    pub team_members: Vec<TeamPlayerModel>,
    pub team_join_requests: Vec<TeamPlayerModel>,
    pub teams: Vec<TeamModel>,
    pub restrictions: Vec<EntityType>, // Entity types that can't be used.
    /// Which server client is currently connected to.
    pub server_id: Option<ServerId>,
    /// All available (alive, compatible) servers.
    pub servers: Vec<ServerModel>,
}

/// Mutually exclusive statuses.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiStatus {
    #[serde(rename_all = "camelCase")]
    Offline,
    #[serde(rename_all = "camelCase")]
    Playing {
        #[serde(rename = "type")]
        entity_type: EntityType,
        velocity: Velocity,
        direction: Angle,
        position: Vec2Model,
        altitude: Altitude,
        #[serde(skip_serializing_if = "Option::is_none")]
        armament_consumption: Option<Box<[bool]>>,
    },
    #[serde(rename_all = "camelCase")]
    Respawning {
        death_reason: DeathReasonModel,
        respawn_level: u8,
    },
    #[serde(rename_all = "camelCase")]
    Spawning,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamPlayerModel {
    pub player_id: PlayerId,
    pub name: PlayerAlias,
    pub captain: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatModel {
    pub name: PlayerAlias,
    pub player_id: Option<PlayerId>,
    pub team: Option<TeamName>,
    pub whisper: bool,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamModel {
    pub team_id: TeamId,
    pub name: TeamName,
    pub joining: bool,
    /// See TeamDto.
    pub full: bool,
    /// See TeamDto.
    pub closed: bool,
}

#[derive(Serialize)]
pub struct LeaderboardItemModel {
    pub name: PlayerAlias,
    pub team: Option<TeamName>,
    pub score: u32,
}

#[derive(Serialize)]
pub struct DeathReasonModel {
    #[serde(rename = "type")]
    pub death_type: &'static str,
    pub player: Option<PlayerAlias>,
    pub entity: Option<EntityType>,
}

impl DeathReasonModel {
    pub fn from_death_reason(reason: &DeathReason) -> Result<Self, &'static str> {
        Ok(match reason {
            DeathReason::Border => DeathReasonModel {
                death_type: "border",
                player: None,
                entity: None,
            },
            DeathReason::Terrain => DeathReasonModel {
                death_type: "terrain",
                player: None,
                entity: None,
            },
            &DeathReason::Boat(alias) => DeathReasonModel {
                death_type: "collision",
                player: Some(alias),
                entity: None,
            },
            DeathReason::Entity(entity_type) => DeathReasonModel {
                death_type: "collision",
                player: None,
                entity: Some(*entity_type),
            },
            &DeathReason::Ram(alias) => DeathReasonModel {
                death_type: "ramming",
                player: Some(alias),
                entity: None,
            },
            &DeathReason::Weapon(alias, entity_type) => DeathReasonModel {
                death_type: "sinking",
                player: Some(alias),
                entity: Some(entity_type),
            },
            _ => return Err("invalid death reason for boat"),
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerModel {
    server_id: ServerId,
    region: &'static str,
    players: usize,
}

/// For serializing a vec2 as {"x": ..., "y": ...} instead of [..., ...]
#[derive(Serialize)]
pub struct Vec2Model {
    x: f32,
    y: f32,
}

impl From<Vec2> for Vec2Model {
    fn from(vec2: Vec2) -> Self {
        Self {
            x: vec2.x,
            y: vec2.y,
        }
    }
}

impl Mk48Game {
    pub(crate) fn update_ui_props(
        &self,
        context: &mut Context<Self>,
        status: UiStatus,
        team_proximity: &HashMap<TeamId, f32>,
    ) {
        let props = UiProps {
            player_id: context.state.core.player_id,
            team_name: context.state.core.team().map(|t| t.name),
            invitation_id: context.state.core.created_invitation_id,
            score: context.state.game.score,
            player_count: context.state.core.real_players as usize,
            fps: self.fps_counter.last_sample().unwrap_or(0.0),
            chats: context
                .state
                .core
                .messages
                .iter()
                .map(|message| ChatModel {
                    name: message.alias,
                    player_id: message.player_id,
                    team: message.team_name,
                    message: message.text.clone(),
                    whisper: message.whisper,
                })
                .collect(),
            liveboard: context
                .state
                .core
                .liveboard
                .iter()
                .filter_map(|item| {
                    let player = context.state.core.only_players().get(&item.player_id);
                    if let Some(player) = player {
                        let team_name = player
                            .team_id
                            .and_then(|team_id| context.state.core.teams.get(&team_id))
                            .map(|team| team.name);
                        Some(LeaderboardItemModel {
                            name: player.alias,
                            team: team_name,
                            score: item.score,
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            leaderboards: context
                .state
                .core
                .leaderboards
                .iter()
                .enumerate()
                .map(|(i, leaderboard)| {
                    let period: PeriodId = i.into();
                    (
                        period,
                        leaderboard
                            .iter()
                            .map(|item| LeaderboardItemModel {
                                name: item.alias,
                                team: None,
                                score: item.score,
                            })
                            .collect(),
                    )
                })
                .collect(),
            team_members: if context.state.core.team_id().is_some() {
                context
                    .state
                    .core
                    .members
                    .iter()
                    .filter_map(|&player_id| context.state.core.player_or_bot(player_id))
                    .map(|p| TeamPlayerModel {
                        player_id: p.player_id,
                        name: p.alias,
                        captain: p.team_captain,
                    })
                    .collect()
            } else {
                vec![]
            },
            team_captain: context.state.core.team_id().is_some()
                && context
                    .state
                    .core
                    .player()
                    .map(|p| p.team_captain)
                    .unwrap_or(false),
            team_full: context
                .state
                .core
                .team_id()
                .and_then(|team_id| context.state.core.teams.get(&team_id))
                .map(|team| team.full)
                .unwrap_or(false),
            team_join_requests: context
                .state
                .core
                .joiners
                .iter()
                .filter_map(|&id| {
                    context
                        .state
                        .core
                        .player_or_bot(id)
                        .map(|player| TeamPlayerModel {
                            player_id: player.player_id,
                            name: player.alias,
                            captain: false,
                        })
                })
                .collect(),
            teams: context
                .state
                .core
                .teams
                .iter()
                .sorted_by(|&(a, team_a), &(b, team_b)| {
                    team_a
                        .closed
                        .cmp(&team_b.closed)
                        .then(team_a.full.cmp(&team_b.full))
                        .then_with(|| {
                            team_proximity
                                .get(a)
                                .unwrap_or(&f32::INFINITY)
                                .partial_cmp(team_proximity.get(b).unwrap_or(&f32::INFINITY))
                                .unwrap()
                        })
                })
                .map(|(team_id, team)| TeamModel {
                    team_id: *team_id,
                    name: team.name,
                    joining: context.state.core.joins.contains(team_id),
                    full: team.full,
                    closed: team.closed,
                })
                .take(5)
                .collect(),
            restrictions: EntityType::iter()
                .filter(|&entity_type: &EntityType| {
                    if let UiStatus::Playing { position, .. } = &status {
                        outside_area(entity_type, vec2(position.x, position.y))
                    } else {
                        false
                    }
                })
                .collect(),
            server_id: context.common_settings.server_id,
            servers: context
                .state
                .core
                .servers
                .iter()
                .map(|(&server_id, server_dto)| ServerModel {
                    server_id,
                    region: server_dto.region_id.as_human_readable_str(),
                    players: server_dto.player_count as usize,
                })
                .sorted_by_key(|model| model.server_id)
                .collect(),
            status,
        };

        context.set_ui_props(props);
    }
}
