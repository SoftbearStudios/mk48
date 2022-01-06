// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::Entity;
use common::altitude::Altitude;
use common::angle::Angle;
use common::contact::{Contact, ContactTrait, ANGLE_ARRAY_ZERO, TICKS_ARRAY_ZERO};
use common::entity::EntityId;
use common::entity::EntityType;
use common::guidance::Guidance;
use common::ticks::Ticks;
use common::transform::Transform;
use core_protocol::id::PlayerId;
use std::sync::Arc;

/// A contact that references world data to avoid additional allocation.
pub struct ContactRef<'a> {
    entity: &'a Entity,
    visible: bool,
    known: bool,
    has_type: bool,
}

impl<'a> ContactRef<'a> {
    /// Creates a new `ContactRef`, referencing an entity, and having certain visibility parameters.
    pub fn new(entity: &'a Entity, visible: bool, known: bool, has_type: bool) -> Self {
        Self {
            entity,
            visible,
            known,
            has_type,
        }
    }

    /// Converts into a non-ref `Contact`.
    pub fn into_contact(self) -> Contact {
        Contact::new(
            self.altitude(),
            self.damage(),
            self.entity_type(),
            *self.guidance(),
            self.id(),
            self.player_id(),
            self.reloads_arc().cloned(),
            *self.transform(),
            self.turrets_arc().cloned(),
        )
    }

    fn reloads_arc(&self) -> Option<&Arc<[Ticks]>> {
        if self.reloads_known() {
            Some(&self.entity.extension().reloads)
        } else {
            None
        }
    }

    fn turrets_arc(&self) -> Option<&Arc<[Angle]>> {
        if self.turrets_known() {
            Some(&self.entity.extension().turrets)
        } else {
            None
        }
    }
}

impl<'a> ContactTrait for ContactRef<'a> {
    #[inline]
    fn altitude(&self) -> Altitude {
        self.entity.altitude
    }

    #[inline]
    fn damage(&self) -> Ticks {
        // Don't send lifespan to client.
        if self.is_boat() {
            self.entity.ticks
        } else {
            Ticks::ZERO
        }
    }

    #[inline]
    fn entity_type(&self) -> Option<EntityType> {
        if self.has_type {
            Some(self.entity.entity_type)
        } else {
            None
        }
    }

    #[inline]
    fn guidance(&self) -> &Guidance {
        &self.entity.guidance
    }

    #[inline]
    fn id(&self) -> EntityId {
        self.entity.id
    }

    #[inline]
    fn player_id(&self) -> Option<PlayerId> {
        self.entity.player.as_ref().map(|p| p.borrow().player_id)
    }

    #[inline]
    fn reloads(&self) -> &[Ticks] {
        self.reloads_arc().map_or(&TICKS_ARRAY_ZERO, |a| a.as_ref())
    }

    #[inline]
    fn reloads_known(&self) -> bool {
        self.has_type && self.entity.is_boat() && (self.visible || self.known)
    }

    #[inline]
    fn transform(&self) -> &Transform {
        &self.entity.transform
    }

    #[inline]
    fn turrets(&self) -> &[Angle] {
        self.turrets_arc().map_or(&ANGLE_ARRAY_ZERO, |a| a.as_ref())
    }

    #[inline]
    fn turrets_known(&self) -> bool {
        self.has_type && self.entity.is_boat()
    }
}
