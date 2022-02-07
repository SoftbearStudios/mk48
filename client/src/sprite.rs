// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use common::entity::{EntityId, EntityType};
use common::transform::Transform;
use glam::Vec2;
use std::cmp::Ordering;

/// Rendering information of a sprite that may be sorted relative to other sprites.
#[derive(Copy, Clone)]
pub struct SortableSprite {
    pub alpha: f32,
    pub altitude: f32,
    pub dimensions: Vec2,
    pub entity_id: Option<EntityId>,
    pub frame: Option<usize>,
    pub sprite: &'static str,
    pub transform: Transform,
}

impl SortableSprite {
    /// Creates from an entity (contact).
    pub fn new_entity(
        entity_id: EntityId,
        entity_type: EntityType,
        transform: Transform,
        mut altitude: f32,
        alpha: f32,
    ) -> Self {
        altitude += Self::entity_height(entity_type);
        Self {
            sprite: entity_type.as_str(),
            frame: None,
            dimensions: entity_type.data().dimensions(),
            transform,
            altitude,
            alpha,
            entity_id: Some(entity_id),
        }
    }

    /// Creates from the child of an entity, i.e. a turret.
    pub fn new_child_entity(
        entity_id: EntityId,
        parent_type: EntityType,
        entity_type: EntityType,
        transform: Transform,
        mut altitude: f32,
        alpha: f32,
    ) -> Self {
        altitude += Self::entity_height(parent_type);
        Self::new_entity(entity_id, entity_type, transform, altitude, alpha)
    }

    /// Creates from an animation frame.
    pub fn new_animation(animation: &Animation, time_seconds: f32) -> Self {
        Self {
            alpha: 1.0,
            altitude: animation.altitude,
            dimensions: Vec2::splat(animation.scale),
            entity_id: None,
            frame: Some(animation.frame(time_seconds)),
            sprite: animation.name,
            transform: Transform::from_position(animation.position),
        }
    }

    /// Depth contribution of entity type, for sorting.
    fn entity_height(entity_type: EntityType) -> f32 {
        entity_type.data().length * 0.0001
    }
}

impl PartialEq for SortableSprite {
    fn eq(&self, other: &Self) -> bool {
        self.altitude == other.altitude && self.entity_id == other.entity_id
    }
}

impl PartialOrd for SortableSprite {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.altitude
                .partial_cmp(&other.altitude)?
                .then_with(|| self.entity_id.cmp(&other.entity_id)),
        )
    }
}
