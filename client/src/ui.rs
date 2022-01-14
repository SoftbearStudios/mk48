// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use client_util::apply::Apply;
use client_util::context::{Context, CoreState};
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::{EntityKind, EntitySubKind, EntityType};
use common::ticks::Ticks;
use common::velocity::Velocity;
use core_protocol::id::{InvitationId, PeriodId, PlayerId, TeamId};
use core_protocol::name::{PlayerAlias, TeamName};
use glam::Vec2;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
    pub player_count: u32,
    pub fps: f32,
    pub status: UiStatus,
    pub chats: Vec<ChatModel>,
    pub liveboard: Vec<LeaderboardItemModel>,
    pub leaderboards: HashMap<PeriodId, Vec<LeaderboardItemModel>>,
    pub team_captain: bool,
    pub team_members: Vec<TeamPlayerModel>,
    pub team_join_requests: Vec<TeamPlayerModel>,
    pub teams: Vec<TeamModel>,
}

/// Mutually exclusive statuses.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiStatus {
    #[serde(rename_all = "camelCase")]
    Alive {
        #[serde(rename = "type")]
        entity_type: EntityType,
        velocity: Velocity,
        direction: Angle,
        position: Vec2Model,
        altitude: Altitude,
        #[serde(skip_serializing_if = "Option::is_none")]
        armament_consumption: Option<Arc<[Ticks]>>,
    },
    #[serde(rename_all = "camelCase")]
    Spawning {
        connection_lost: bool,
        death_reason: Option<DeathReasonModel>,
    },
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
    pub fn from_death_reason(
        reason: &DeathReason,
        core_state: &CoreState,
    ) -> Result<Self, &'static str> {
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
            DeathReason::Boat(player_id) => DeathReasonModel {
                death_type: "collision",
                player: Some(
                    core_state
                        .players
                        .get(player_id)
                        .map(|p| p.alias)
                        .unwrap_or_else(|| PlayerAlias::new("???")),
                ),
                entity: None,
            },
            DeathReason::Entity(entity_type) => DeathReasonModel {
                death_type: "collision",
                player: None,
                entity: Some(*entity_type),
            },
            DeathReason::Ram(player_id) => DeathReasonModel {
                death_type: "ramming",
                player: Some(
                    core_state
                        .players
                        .get(player_id)
                        .map(|p| p.alias)
                        .unwrap_or_else(|| PlayerAlias::new("???")),
                ),
                entity: None,
            },
            DeathReason::Weapon(player_id, entity_type) => DeathReasonModel {
                death_type: "sinking",
                player: Some(
                    core_state
                        .players
                        .get(player_id)
                        .map(|p| p.alias)
                        .unwrap_or_else(|| PlayerAlias::new("???")),
                ),
                entity: Some(*entity_type),
            },
            _ => return Err("invalid death reason for boat"),
        })
    }
}

// For serializing a vec2 as {"x": ..., "y": ...} instead of [..., ...]
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
        let core_state = context.core();
        let props = UiProps {
            player_id: core_state.player_id,
            team_name: core_state.team().map(|t| t.team_name),
            invitation_id: core_state.created_invitation_id,
            score: context.game().score,
            player_count: core_state.player_count,
            fps: self.fps_counter.last_sample().unwrap_or(0.0),
            status,
            chats: core_state
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
                .core()
                .liveboard
                .iter()
                .filter_map(|item| {
                    let player = core_state.players.get(&item.player_id);
                    if let Some(player) = player {
                        let team_name = player
                            .team_id
                            .and_then(|team_id| core_state.teams.get(&team_id))
                            .map(|team| team.team_name);
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
                .core()
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
            team_members: if let Some(team_id) = core_state.team_id() {
                core_state
                    .players
                    .values()
                    .filter(|p| p.team_id == Some(team_id))
                    .map(|p| TeamPlayerModel {
                        player_id: p.player_id,
                        name: p.alias,
                        captain: p.team_captain,
                    })
                    .sorted_by(|a, b| b.captain.cmp(&a.captain).then(a.name.cmp(&b.name)))
                    .collect()
            } else {
                vec![]
            },
            team_captain: core_state.team_id().is_some()
                && core_state.player().map(|p| p.team_captain).unwrap_or(false),
            team_join_requests: context
                .core()
                .joiners
                .iter()
                .filter_map(|id| {
                    context
                        .core()
                        .players
                        .get(id)
                        .map(|player| TeamPlayerModel {
                            player_id: player.player_id,
                            name: player.alias,
                            captain: false,
                        })
                })
                .collect(),
            teams: context
                .core()
                .teams
                .iter()
                .sorted_by(|&(a, _), &(b, _)| {
                    team_proximity
                        .get(a)
                        .unwrap_or(&f32::INFINITY)
                        .partial_cmp(team_proximity.get(b).unwrap_or(&f32::INFINITY))
                        .unwrap()
                })
                .map(|(team_id, team)| TeamModel {
                    team_id: *team_id,
                    name: team.team_name,
                    joining: core_state.joins.contains(team_id),
                })
                .take(5)
                .collect(),
        };

        context.set_ui_props(props);
    }
}
