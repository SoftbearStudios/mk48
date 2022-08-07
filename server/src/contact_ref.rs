// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::Entity;
use bitvec::prelude::*;
use common::altitude::Altitude;
use common::angle::Angle;
use common::contact::{
    Contact, ContactTrait, ReloadsStorage, ANGLE_ARRAY_ZERO, RELOADS_ARRAY_ZERO,
};
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
    has_type: bool,
    reloads: Option<BitArray<ReloadsStorage>>,
}

impl<'a> ContactRef<'a> {
    /// Creates a new `ContactRef`, referencing an entity, and having certain visibility parameters.
    pub fn new(entity: &'a Entity, visible: bool, known: bool, has_type: bool) -> Self {
        let reloads = (has_type && entity.is_boat() && (visible || known)).then(|| {
            let reloads = &*entity.extension().reloads;
            let mut arr = BitArray::ZERO;
            assert!(
                reloads.len() <= ReloadsStorage::MAX.count_ones() as usize,
                "not enough bits in reloads storage"
            );
            for (mut b, t) in arr.as_mut_bitslice().into_iter().zip(reloads.iter()) {
                b.set(t == &Ticks::ZERO);
            }
            arr
        });

        Self {
            entity,
            has_type,
            reloads,
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
            self.reloads,
            *self.transform(),
            self.turrets_arc().cloned(),
        )
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
        self.entity
            .player
            .as_ref()
            .map(|p| p.borrow_player().player_id)
    }

    #[inline]
    fn reloads(&self) -> &BitSlice<ReloadsStorage> {
        self.reloads
            .as_ref()
            .map(|a| &a.as_bitslice()[0..self.entity.entity_type.data().armaments.len()])
            .unwrap_or_else(|| RELOADS_ARRAY_ZERO.as_bitslice())
    }

    #[inline]
    fn reloads_known(&self) -> bool {
        self.reloads.is_some()
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
