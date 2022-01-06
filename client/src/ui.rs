use client_util::apply::Apply;
use common::altitude::Altitude;
use common::angle::Angle;
use common::entity::{EntityKind, EntitySubKind, EntityType};
use common::ticks::Ticks;
use common::velocity::Velocity;
use core_protocol::id::*;
use core_protocol::name::*;
use glam::Vec2;
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
        alias: PlayerAlias,
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
