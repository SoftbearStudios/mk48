// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::altitude::Altitude;
use crate::angle::Angle;
use crate::contact::Contact;
use crate::death_reason::DeathReason;
use crate::entity::*;
use crate::guidance::Guidance;
use crate::terrain::ChunkId;
use core_protocol::id::*;
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Server to client update.
#[cfg_attr(feature = "server", derive(actix::Message))]
#[cfg_attr(feature = "server", rtype(result = "()"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct Update {
    /// All currently visible contacts.
    pub contacts: Vec<Contact>,
    /// Why the player died, if they died, otherwise None.
    pub death_reason: Option<DeathReason>,
    /// Player's id.
    pub player_id: PlayerId,
    /// Player's current score.
    pub score: u32,
    /// Current world border radius.
    pub world_radius: f32,
    pub terrain: Box<TerrainUpdate>,
}

/// Updates for terrain chunks.
pub type TerrainUpdate = [SerializedChunk];

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedChunk(pub ChunkId, #[serde(with = "serde_bytes")] pub Box<[u8]>);

/// Client to server commands.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "server", derive(actix::Message))]
#[cfg_attr(feature = "server", rtype(result = "()"))]
pub enum Command {
    Control(Control),
    Fire(Fire),
    Pay(Pay),
    Spawn(Spawn),
    Upgrade(Upgrade),
}

/// Generic command to control one's ship.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Control {
    /// Steering commands.
    pub guidance: Option<Guidance>,
    /// Unimplemented.
    pub angular_velocity_target: Option<Angle>,
    /// Altitude target (useful for submarines).
    pub altitude_target: Option<Altitude>,
    /// Turret/aircraft target.
    pub aim_target: Option<Vec2>,
    /// Active sensors.
    pub active: bool,
}

/// Fire/use a single weapon.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Fire {
    /// The index of the weapon to fire/use, relative to `EntityData.armaments`.
    pub index: u8,
    /// The target of the weapon (useful for depositors).
    pub position_target: Vec2,
}

/// Pay one coin.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Pay {
    /// Where to spawn the coin. Must be within outer radius.
    pub position: Vec2,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Spawn {
    /// What to spawn as. Must be a level 1 boat.
    pub entity_type: EntityType,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Upgrade {
    /// What to upgrade to. Must be affordable.
    pub entity_type: EntityType,
}
