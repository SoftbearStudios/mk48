// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::altitude::Altitude;
use crate::angle::Angle;
use crate::entity::*;
use crate::guidance::Guidance;
use crate::ticks::Ticks;
use crate::transform::Transform;
use core_protocol::id::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub trait ContactTrait {
    fn altitude(&self) -> Altitude;

    fn damage(&self) -> Ticks;

    fn entity_type(&self) -> Option<EntityType>;

    fn guidance(&self) -> &Guidance;

    fn id(&self) -> EntityId;

    fn player_id(&self) -> Option<PlayerId>;

    fn reloads(&self) -> &[Ticks];

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contact {
    altitude: Altitude,
    damage: Ticks,
    entity_type: Option<EntityType>,
    guidance: Guidance,
    id: EntityId,
    player_id: Option<PlayerId>,
    reloads: Option<Arc<[Ticks]>>,
    transform: Transform,
    turrets: Option<Arc<[Angle]>>,
}

impl Contact {
    pub fn new(
        altitude: Altitude,
        damage: Ticks,
        entity_type: Option<EntityType>,
        guidance: Guidance,
        id: EntityId,
        player_id: Option<PlayerId>,
        reloads: Option<Arc<[Ticks]>>,
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
            reloads,
            transform,
            turrets,
        }
    }

    // TODO handle predictive physics in common.
    #[inline]
    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    /// Interpolates or snaps one contact's fields to another, assuming they share the same id.
    /// Optionally affects guidance, because that is more of an input, and is not subject to physics.
    pub fn interpolate_towards(
        &mut self,
        model: &Self,
        interpolate_guidance: bool,
        delta_seconds: f32,
    ) {
        let lerp = delta_seconds.clamp(0.0, 1.0);

        assert_eq!(self.id, model.id);
        self.altitude = self.altitude.lerp(model.altitude, lerp);
        self.damage = model.damage;
        self.entity_type = model.entity_type;
        self.player_id = model.player_id;
        self.reloads = model.reloads.clone();
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
        self.turrets = model.turrets.clone();
    }

    /// Applies a control message to a contact (can be used to predict its outcome).
    pub fn predict_guidance(&mut self, guidance: &Guidance) {
        self.guidance = *guidance;
    }
}

pub static ANGLE_ARRAY_ZERO: [Angle; 0] = [Angle::ZERO; 0];
pub static TICKS_ARRAY_ZERO: [Ticks; 0] = [Ticks::ZERO; 0];

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
    fn reloads(&self) -> &[Ticks] {
        self.reloads
            .as_ref()
            .map_or(&TICKS_ARRAY_ZERO, |a| a.as_ref())
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
