// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::altitude::Altitude;
use crate::angle::Angle;
use crate::ticks::Ticks;
use crate::transform::Transform;
use crate::util::level_to_score;
use crate::velocity::Velocity;
use enum_iterator::IntoEnumIterator;
use glam::Vec2;
use macros::entity_type;
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::{Mul, Range, RangeInclusive};

pub type EntityId = NonZeroU32;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    Aircraft,
    Boat,
    Collectible,
    Decoy,
    Obstacle,
    Turret,
    Weapon,
}

impl EntityKind {
    /// Largest possible `Self::keep_alive()` return value.
    pub const MAX_KEEP_ALIVE: Ticks = Ticks(10);

    /// After how many ticks of not hearing about an entity should we assume it is gone/no longer
    /// visible. This allows the server to optimize bandwidth usage but transmitting certain entities
    /// less frequently.
    ///
    /// The higher end of the range is used (for efficiency) except if the velocity is above
    /// a certain threshold.
    ///
    /// To guarantee some updates are sent, make sure the (start + 1) divides (end + 1).
    pub const fn keep_alive(self) -> RangeInclusive<Ticks> {
        match self {
            Self::Boat | Self::Decoy | Self::Weapon | Self::Aircraft | Self::Turret => {
                Ticks(0)..=Ticks(0)
            }
            Self::Collectible => Ticks(2)..=Ticks(5),
            Self::Obstacle => Self::MAX_KEEP_ALIVE..=Self::MAX_KEEP_ALIVE,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntitySubKind {
    Battleship,
    Carrier,
    Corvette,
    Cruiser,
    Depositor,
    DepthCharge,
    Destroyer,
    Dreadnought,
    Dredger,
    Heli,
    Hovercraft,
    Gun,
    Lcs,
    Mine,
    Minelayer,
    Missile,
    Mtb,
    Pirate,
    Plane,
    Ram,
    Rocket,
    Sam,
    Score,
    Shell,
    Sonar,
    Structure,
    Submarine,
    Tanker,
    Torpedo,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityData {
    pub kind: EntityKind,
    #[serde(rename = "subkind")]
    pub sub_kind: EntitySubKind,
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub limited: bool,
    #[serde(default)]
    pub npc: bool,
    #[serde(default)]
    pub lifespan: Ticks,
    #[serde(default)]
    pub reload: Ticks,
    #[serde(default)]
    pub speed: Velocity, // Guaranteed to get the attention of any physics professor.
    pub length: f32,
    pub width: f32,
    #[serde(default)]
    pub draft: Altitude, // Type is a bit cheeky but kind of makes sense.
    #[serde(default)]
    pub depth: Altitude,
    #[serde(skip)]
    pub radius: f32,
    #[serde(skip)]
    pub inv_size: f32,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub anti_aircraft: f32,
    #[serde(default)]
    pub stealth: f32,
    #[serde(default)]
    pub sensors: Sensors,
    #[serde(default)]
    pub armaments: Vec<Armament>,
    #[serde(default)]
    pub turrets: Vec<Turret>,
    #[serde(default)]
    pub exhausts: Vec<Exhaust>,
    pub label: String,
    #[serde(default)]
    pub position_forward: f32,
    #[serde(default)]
    pub position_side: f32,
}

impl EntityData {
    /// Missiles, rockets, Sams, etc. that are rising from a submerged submarine don't move
    /// horizontally (very fast) until they reach the surface.
    pub const SURFACING_PROJECTILE_SPEED_LIMIT: f32 = 0.5;

    /// radii range of throttle (0-100%) and limit of collecting things.
    pub fn radii(&self) -> Range<f32> {
        self.length * 0.55..self.length
    }

    /// dimensions returns a Vec2 with the x component equal to the length and the y component equal to the width.
    pub fn dimensions(&self) -> Vec2 {
        Vec2::new(self.length, self.width)
    }

    /// offset returns an offset to use while rendering.
    pub fn offset(&self) -> Vec2 {
        Vec2::new(self.position_forward, self.position_side)
    }

    /// max_health returns the the minimum damage to kill a boat, panicking if the corresponding
    /// entity does not have health.
    pub fn max_health(&self) -> Ticks {
        if self.kind == EntityKind::Boat {
            return Ticks::from_damage(self.damage);
        }
        unreachable!("only boats have health");
    }

    /// armament_transform returns the entity-relative transform of a given armament.
    pub fn armament_transform(&self, turret_angles: &[Angle], index: usize) -> Transform {
        let armament = &self.armaments[index];
        let mut transform = Transform {
            position: armament.position(),
            direction: armament.angle,
            velocity: Velocity::ZERO,
        };

        let weapon_data = armament.entity_type.data();

        // Shells start with all their velocity.
        if weapon_data.sub_kind == EntitySubKind::Shell {
            transform.velocity = weapon_data.speed
        } else if !armament.vertical {
            // Minimal launch velocity (except if vertical, in which case only initial velocity is up).
            transform.velocity = Velocity::from_mps(1.0);
        }

        if let Some(turret_index) = armament.turret {
            let turret = &self.turrets[turret_index];
            transform = Transform {
                position: turret.position(),
                direction: turret_angles[turret_index],
                velocity: Velocity::ZERO,
            } + transform;
        }
        transform
    }

    /// update_turret_aim brings turret_angles delta_seconds closer to position_target.
    pub fn update_turret_aim(
        &self,
        boat_transform: Transform,
        turret_angles: &mut [Angle],
        position_target: Option<Vec2>,
        delta_seconds: f32,
    ) {
        for (i, a) in turret_angles.iter_mut().enumerate() {
            let turret = &self.turrets[i];
            let amount = Angle::from_radians(
                (delta_seconds * turret.speed.to_radians()).clamp(0.0, std::f32::consts::PI),
            );
            let mut direction_target = turret.angle;
            if let Some(target) = position_target {
                let turret_global_transform = boat_transform
                    + Transform {
                        position: turret.position(),
                        direction: *a,
                        velocity: Velocity::ZERO,
                    };
                let global_direction = Angle::from(target - turret_global_transform.position);
                direction_target = global_direction - boat_transform.direction;
            }
            let delta_angle = (direction_target - *a).clamp_magnitude(amount);

            // Allow turning through, but not stopping in, restricted angles
            if delta_angle != Angle::ZERO
                && (turret.within_azimuth(*a + delta_angle)
                    || turret.within_azimuth(direction_target))
            {
                *a += delta_angle
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Armament {
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub external: bool,
    #[serde(default)]
    pub vertical: bool,
    #[serde(default)]
    pub position_forward: f32,
    #[serde(default)]
    pub position_side: f32,
    #[serde(default)]
    pub angle: Angle,
    #[serde(default)]
    pub turret: Option<usize>,
}

impl Armament {
    pub fn reload(&self) -> Ticks {
        self.entity_type.data().reload
    }

    pub fn position(&self) -> Vec2 {
        Vec2::new(self.position_forward, self.position_side)
    }

    /// is_similar_to reports if two armaments are similar enough to reload
    /// together (presumably will be grouped in GUI).
    pub fn is_similar_to(&self, other: &Self) -> bool {
        self.entity_type == other.entity_type && self.turret == other.turret
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exhaust {
    #[serde(default)]
    pub position_forward: f32,
    #[serde(default)]
    pub position_side: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Sensors {
    #[serde(default)]
    pub visual: Sensor,
    #[serde(default)]
    pub radar: Sensor,
    #[serde(default)]
    pub sonar: Sensor,
}

impl Sensors {
    /// any returns if any of the sensors have a non-zero range.
    pub fn any(&self) -> bool {
        self.visual.range != 0.0 || self.radar.range != 0.0 || self.sonar.range != 0.0
    }

    /// max_range returns the maximum range of all sensors.
    pub fn max_range(&self) -> f32 {
        self.visual
            .range
            .max(self.radar.range.max(self.sonar.range))
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Sensor {
    pub range: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turret {
    #[serde(rename = "type")]
    #[serde(default)]
    pub entity_type: Option<EntityType>,
    #[serde(default)]
    pub position_forward: f32,
    #[serde(default)]
    pub position_side: f32,
    #[serde(default)]
    pub angle: Angle,
    pub speed: Angle,
    #[serde(default, rename = "azimuthFL")]
    pub azimuth_fl: Angle,
    #[serde(default, rename = "azimuthFR")]
    pub azimuth_fr: Angle,
    #[serde(default, rename = "azimuthBL")]
    pub azimuth_bl: Angle,
    #[serde(default, rename = "azimuthBR")]
    pub azimuth_br: Angle,
}

impl Turret {
    pub fn position(&self) -> Vec2 {
        Vec2::new(self.position_forward, self.position_side)
    }

    /// within_azimuth returns whether the given boat-relative angle is within the azimuth (horizontal
    /// angle) limits, if any.
    pub fn within_azimuth(&self, curr: Angle) -> bool {
        /*
        Angles are counterclockwise.
        Each turret.azimuth_** angle is a restriction starting in the respective quadrant.
        ------------BL-----------FL-BR--------\
        |           ---- o=== ----             \
        |           BR    ^      FR BL          |  <-- boat
        |               turret       ^-flipped /
        --------------------------------------/
         */

        // The angle as it relates to the front azimuth limits.
        let azimuth_f = curr - self.angle;
        if -self.azimuth_fr < azimuth_f && azimuth_f < self.azimuth_fl {
            false
        } else {
            // The angle as it relates to the back azimuth limits.
            let azimuth_b = Angle::PI + curr - self.angle;
            !(-self.azimuth_bl < azimuth_b && azimuth_b < self.azimuth_br)
        }
    }
}

entity_type!("../../js/src/data/entities.json");

static mut ENTITY_DATA: Vec<EntityData> = Vec::new();

impl EntityType {
    /// Data returns the data associated with the entity type.
    /// This is only safe to call after Self::init() is called.
    #[inline]
    pub fn data(self) -> &'static EntityData {
        // SAFETY: Safe if called after Self::init() is called once.
        unsafe { ENTITY_DATA.get_unchecked(self as usize) }
    }

    /// reduced lifespan returns a lifespan to start an entity's life at, so as to make it expire
    /// in desired_lifespan ticks
    #[allow(dead_code)]
    pub fn reduced_lifespan(self, desired_lifespan: Ticks) -> Ticks {
        let data = self.data();
        if data.lifespan > desired_lifespan {
            return data.lifespan - desired_lifespan;
        }
        data.lifespan
    }

    /// can_spawn_as returns whether it is possible to spawn as the entity type, which may depend
    /// on whether you are a bot.
    pub fn can_spawn_as(self, bot: bool) -> bool {
        let data = self.data();
        data.kind == EntityKind::Boat && data.level == 1 && (bot || !data.npc)
    }

    /// can_upgrade_to returns whether it is possible to upgrade to the entity type, which may depend
    /// on your score and whether you are a bot.
    pub fn can_upgrade_to(self, upgrade: Self, score: u32, bot: bool) -> bool {
        let data = self.data();
        let upgrade_data = upgrade.data();
        upgrade_data.level > data.level
            && upgrade_data.kind == data.kind
            && score >= level_to_score(upgrade_data.level)
            && (bot || !upgrade_data.npc)
    }

    /// iter returns an iterator that visits all possible entity types and allows a random choice to
    /// be made.
    pub fn iter() -> impl Iterator<Item = Self> + IteratorRandom {
        Self::into_enum_iter()
    }

    /// spawn_options returns an iterator that visits all spawnable entity types and allows a random
    /// choice to be made.
    pub fn spawn_options(bot: bool) -> impl Iterator<Item = Self> + IteratorRandom {
        Self::iter().filter(move |t| t.can_spawn_as(bot))
    }

    /// upgrade_options returns an iterator that visits all entity types that may be upgraded to
    /// and allows a random choice to be made.
    #[inline]
    pub fn upgrade_options(
        self,
        score: u32,
        bot: bool,
    ) -> impl Iterator<Item = Self> + IteratorRandom {
        // Don't iterate if not enough score for next level.
        if score >= level_to_score(self.data().level + 1) {
            Some(Self::iter().filter(move |t| self.can_upgrade_to(*t, score, bot)))
        } else {
            None
        }
        .into_iter()
        .flatten()
    }

    /// init initializes EntityData.
    /// # Safety
    /// To be called ONLY ONCE, near the beginning of main, before self.data() is called.
    pub unsafe fn init() {
        let map: HashMap<EntityType, EntityData> =
            serde_json::from_str(include_str!("../../js/src/data/entities.json"))
                .expect("could not parse entity json");
        let mut vector = Vec::with_capacity(map.len());

        let mut sorted: Vec<(EntityType, EntityData)> = map.into_iter().collect();
        sorted.sort_by_key(|(k, _)| *k as u8);

        vector.extend(sorted.into_iter().map(|(_, mut v)| {
            v.radius = Vec2::new(v.width, v.length).mul(0.5).length();
            v.inv_size = 1.0 / (v.radius * (1.0 / 30.0) * (1.0 - v.stealth).powi(2)).min(1.0);
            v
        }));

        ENTITY_DATA = vector;
    }
}
