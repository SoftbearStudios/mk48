// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::contact::Contact;
use crate::death_reason::DeathReason;
use crate::entity::*;
use crate::guidance::Guidance;
use crate::terrain::{ChunkId, SerializedChunk};
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
    /// Player's current score.
    pub score: u32,
    /// Current world border radius.
    pub world_radius: f32,
    pub terrain: Box<TerrainUpdate>,
}

/// Updates for terrain chunks.
pub type TerrainUpdate = [(ChunkId, SerializedChunk)];

/// Client to server commands.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "server", derive(actix::Message))]
#[cfg_attr(feature = "server", rtype(result = "()"))]
pub enum Command {
    Control(Control),
    Spawn(Spawn),
    Upgrade(Upgrade),
}

/// Generic command to control one's ship.
#[derive(Clone, Serialize, PartialEq, Deserialize, Debug)]
pub struct Control {
    /// Steering commands.
    pub guidance: Option<Guidance>,
    /// Submerge submarine.
    pub submerge: bool,
    /// Turret/aircraft/pay target.
    pub aim_target: Option<Vec2>,
    /// Active sensors.
    pub active: bool,
    /// Fire weapon a weapon.
    pub fire: Option<Fire>,
    /// Pay one coin.
    pub pay: Option<Pay>,
    /// Optional hints.
    pub hint: Option<Hint>,
}

/// Fire/use a single weapon.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Fire {
    /// The index of the weapon to fire/use, relative to `EntityData.armaments`.
    pub armament_index: u8,
}

/// Provide hints to optimize experience.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Hint {
    /// aspect ratio of screen (width / height).
    /// Allows the server to send the correct amount of terrain.
    pub aspect: f32,
}

impl Default for Hint {
    fn default() -> Self {
        Self { aspect: 1.0 }
    }
}

/// Pay one coin. TODO: Can't use Option<empty struct>, as serde_json serializes both [`None`] and
/// [`Some`] to `"null"`.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Pay;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Spawn {
    /// What to spawn as. Must be an affordable boat.
    pub entity_type: EntityType,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Upgrade {
    /// What to upgrade to. Must be an affordable boat of higher level.
    pub entity_type: EntityType,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::altitude::Altitude;
    use crate::contact::ReloadsStorage;
    use crate::entity::EntityId;
    use crate::guidance::Guidance;
    use crate::ticks::Ticks;
    use crate::transform::Transform;
    use crate::velocity::Velocity;
    use bincode::{DefaultOptions, Options};
    use bitvec::array::BitArray;
    use core_protocol::id::PlayerId;
    use glam::vec2;
    use rand::prelude::*;
    use std::num::NonZeroU32;

    #[test]
    fn serialize() {
        EntityType::from_str(EntityType::Barrel.as_str()).unwrap();

        let mut rng = thread_rng();
        for _ in 0..10000 {
            let entity_type: Option<EntityType> = rng
                .gen_bool(0.5)
                .then(|| EntityType::iter().choose(&mut rng).unwrap());
            let is_boat = entity_type.map_or(false, |t| t.data().kind == EntityKind::Boat);

            let c = Contact::new(
                Altitude::from_u8(rng.gen()),
                Ticks::from_secs(rng.gen::<f32>() * 10.0),
                entity_type,
                Guidance {
                    direction_target: rng.gen(),
                    velocity_target: Velocity::from_mps(rng.gen::<f32>() * 3.0),
                },
                EntityId::new(rng.gen_range(1..u32::MAX)).unwrap(),
                rng.gen_bool(0.5)
                    .then(|| PlayerId(NonZeroU32::new(rng.gen_range(1..u32::MAX)).unwrap())),
                (is_boat && rng.gen_bool(0.5)).then(|| {
                    let mut arr = BitArray::<ReloadsStorage>::ZERO;
                    for (_, mut r) in entity_type
                        .unwrap()
                        .data()
                        .armaments
                        .iter()
                        .zip(arr.iter_mut())
                    {
                        *r = rng.gen();
                    }
                    arr
                }),
                Transform {
                    position: vec2(
                        rng.gen::<f32>() * 1000.0 - 500.0,
                        rng.gen::<f32>() * 1000.0 - 500.0,
                    ),
                    velocity: Velocity::from_mps(rng.gen::<f32>() * 3.0),
                    direction: rng.gen(),
                },
                is_boat.then(|| {
                    entity_type
                        .unwrap()
                        .data()
                        .turrets
                        .iter()
                        .map(|_| rng.gen())
                        .collect()
                }),
            );

            let options = DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes();

            let bytes = options.serialize(&c).unwrap();

            match options.deserialize::<Contact>(&bytes) {
                Ok(contact) => {
                    assert_eq!(c, contact)
                }
                Err(err) => {
                    println!("len: {}, bytes: {:?}", bytes.len(), &bytes);
                    println!("contact: {:?}", &c);

                    let byte = bytes[0];
                    for i in 0u32..8 {
                        println!("byte {}: {}", i, byte & (1 << i) != 0)
                    }
                    panic!("{}", err);
                }
            }
        }
    }
}
