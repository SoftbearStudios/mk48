// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::altitude::Altitude;
use crate::angle::Angle;
use crate::entity::*;
use crate::guidance::Guidance;
use crate::ticks::Ticks;
use crate::transform::Transform;
use crate::util::make_mut_slice;
use bitvec::prelude::*;
use kodiak_common::bitcode::{self, *};
use kodiak_common::PlayerId;
use std::sync::Arc;

pub type ReloadsStorage = u32;

pub trait ContactTrait {
    fn altitude(&self) -> Altitude;

    fn damage(&self) -> Ticks;

    fn entity_type(&self) -> Option<EntityType>;

    fn guidance(&self) -> &Guidance;

    fn id(&self) -> EntityId;

    fn player_id(&self) -> Option<PlayerId>;

    fn reloads(&self) -> &BitSlice<ReloadsStorage>;

    /// Whether reloads() will return real data or all zeroes.
    fn reloads_known(&self) -> bool;

    fn transform(&self) -> &Transform;

    fn turrets(&self) -> &[Angle];

    /// Whether turrets() will return real data or all zeroes.
    fn turrets_known(&self) -> bool;

    #[inline]
    fn is_boat(&self) -> bool {
        self.entity_type()
            .map_or(false, |t| t.data().kind == EntityKind::Boat)
    }

    #[inline]
    fn data(&self) -> &'static EntityData {
        self.entity_type().unwrap().data()
    }
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct Contact {
    // TODO: hint velocity = 0
    transform: Transform,
    // TODO: hint = 0
    altitude: Altitude,
    guidance: Guidance,
    // TODO: hint = 0
    damage: Ticks,
    entity_type: Option<EntityType>,
    id: EntityId,
    player_id: Option<PlayerId>,
    // TODO: no option + gamma
    reloads: Option<ReloadsStorage>,
    // TODO: no option + len 0
    turrets: Option<Arc<[Angle]>>,
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            altitude: Altitude::default(),
            damage: Ticks::default(),
            entity_type: None,
            guidance: Guidance::default(),
            id: EntityId::new(u32::MAX).unwrap(),
            player_id: None,
            reloads: None,
            transform: Transform::default(),
            turrets: None,
        }
    }
}

impl Contact {
    /// Initializes all (private) fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        altitude: Altitude,
        damage: Ticks,
        entity_type: Option<EntityType>,
        guidance: Guidance,
        id: EntityId,
        player_id: Option<PlayerId>,
        reloads: Option<BitArray<ReloadsStorage>>,
        transform: Transform,
        turrets: Option<Arc<[Angle]>>,
    ) -> Self {
        Self {
            altitude,
            damage,
            entity_type,
            guidance,
            id,
            player_id,
            reloads: reloads.map(|a| a.into_inner()),
            transform,
            turrets,
        }
    }

    /// Simulate delta_seconds passing, by updating guidance and kinematics. This is an approximation
    /// of how the corresponding entity behaves on the server.
    pub fn simulate(&mut self, delta_seconds: f32) {
        if let Some(entity_type) = self.entity_type() {
            let guidance = *self.guidance();
            let max_speed = match entity_type.data().sub_kind {
                // Wait until risen to surface.
                EntitySubKind::Missile
                | EntitySubKind::Rocket
                | EntitySubKind::RocketTorpedo
                | EntitySubKind::Sam
                    if self.altitude().is_submerged() =>
                {
                    EntityData::SURFACING_PROJECTILE_SPEED_LIMIT
                }
                _ => f32::INFINITY,
            };

            self.transform_mut().apply_guidance(
                entity_type.data(),
                guidance,
                max_speed,
                delta_seconds,
            );
        }
        self.transform_mut().do_kinematics(delta_seconds);
    }

    /// Interpolates or snaps one contact's fields to another, assuming they share the same id.
    /// Optionally affects guidance, because that is more of an input, and is not subject to physics.
    pub fn interpolate_towards(
        &mut self,
        model: &Self,
        interpolate_guidance: bool,
        lerp: f32,
        delta_seconds: f32,
    ) {
        // Clamp to valid range once.
        let lerp = lerp.clamp(0.0, 1.0);

        assert_eq!(self.id, model.id);

        // Upgraded.
        let changed_type = self.entity_type != model.entity_type;
        self.entity_type = model.entity_type;

        self.altitude = self.altitude.lerp(model.altitude, lerp);
        self.damage = model.damage;
        self.player_id = model.player_id;
        self.reloads = model.reloads;
        if interpolate_guidance {
            self.guidance = model.guidance;
        }

        self.transform = Transform {
            position: self.transform.position.lerp(model.transform.position, lerp),
            direction: self
                .transform
                .direction
                .lerp(model.transform.direction, lerp),
            velocity: self.transform.velocity.lerp(model.transform.velocity, lerp),
        };

        if let Some((turrets, model_turrets)) = self
            .turrets
            .as_mut()
            .zip(model.turrets.as_ref())
            .filter(|_| !changed_type)
        {
            let turrets = make_mut_slice(turrets);
            let data: &'static EntityData = self.entity_type.unwrap().data();
            let turret_data = &*data.turrets;
            for ((v, m), t) in turrets
                .iter_mut()
                .zip(model_turrets.iter())
                .zip(turret_data)
            {
                let diff = *m - *v;

                // Don't let it get too far off.
                if diff.abs() > t.speed * Ticks::from_repr(2).to_secs() {
                    *v = *m;
                } else {
                    *v += diff.clamp_magnitude(t.speed * delta_seconds);
                }
            }
        } else {
            self.turrets = model.turrets.clone()
        }
    }

    /// Applies a control message to a contact (can be used to predict its outcome).
    pub fn predict_guidance(&mut self, guidance: &Guidance) {
        self.guidance = *guidance;
    }

    // TODO handle predictive physics in common.
    #[inline]
    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }
}

pub static ANGLE_ARRAY_ZERO: [Angle; 0] = [Angle::ZERO; 0];
pub static RELOADS_ARRAY_ZERO: BitArray<ReloadsStorage> = BitArray::ZERO;

impl ContactTrait for Contact {
    fn altitude(&self) -> Altitude {
        self.altitude
    }

    #[inline]
    fn damage(&self) -> Ticks {
        self.damage
    }

    #[inline]
    fn entity_type(&self) -> Option<EntityType> {
        self.entity_type
    }

    #[inline]
    fn guidance(&self) -> &Guidance {
        &self.guidance
    }

    #[inline]
    fn id(&self) -> EntityId {
        self.id
    }

    #[inline]
    fn player_id(&self) -> Option<PlayerId> {
        self.player_id
    }

    #[inline]
    fn reloads(&self) -> &BitSlice<ReloadsStorage> {
        self.reloads.as_ref().map_or(&RELOADS_ARRAY_ZERO, |a| {
            &BitSlice::from_element(a)[0..self.entity_type.unwrap().data().armaments.len()]
        })
    }

    #[inline]
    fn reloads_known(&self) -> bool {
        self.reloads.is_some()
    }

    #[inline]
    fn transform(&self) -> &Transform {
        &self.transform
    }

    #[inline]
    fn turrets(&self) -> &[Angle] {
        self.turrets
            .as_ref()
            .map_or(&ANGLE_ARRAY_ZERO, |a| a.as_ref())
    }

    #[inline]
    fn turrets_known(&self) -> bool {
        self.turrets.is_some()
    }
}
