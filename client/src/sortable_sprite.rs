// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use common::altitude::Altitude;
use common::entity::{EntityId, EntityKind, EntitySubKind, EntityType};
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
    pub height: f32,
    pub shadow_height: f32,
    pub sprite: &'static str,
    pub transform: Transform,
}

impl SortableSprite {
    /// Creates from an entity (contact).
    pub fn new_entity(
        entity_id: EntityId,
        entity_type: EntityType,
        transform: Transform,
        altitude: f32,
        alpha: f32,
    ) -> Self {
        let height = Self::deck_height(entity_type);
        let altitude = height + altitude;

        Self {
            alpha,
            altitude,
            dimensions: entity_type.data().dimensions(),
            entity_id: Some(entity_id),
            frame: None,
            height,
            shadow_height: altitude,
            sprite: entity_type.as_str(),
            transform,
        }
    }

    /// Creates from the child of an entity, i.e. a turret.
    pub fn new_child_entity(
        entity_id: EntityId,
        parent_type: EntityType,
        entity_type: EntityType,
        transform: Transform,
        altitude: f32,
        alpha: f32,
    ) -> Self {
        let parent_height = Self::deck_height(parent_type);
        let height = Self::deck_height(entity_type);
        let shadow_height = altitude + height;
        let altitude = altitude + parent_height + height;

        Self {
            alpha,
            altitude,
            dimensions: entity_type.data().dimensions(),
            entity_id: Some(entity_id),
            frame: None,
            height,
            shadow_height,
            sprite: entity_type.as_str(),
            transform,
        }
    }

    /// Creates from an animation frame.
    pub fn new_animation(animation: &Animation, time_seconds: f32) -> Self {
        Self {
            alpha: 1.0,
            altitude: animation.altitude,
            dimensions: Vec2::splat(animation.scale),
            entity_id: None,
            frame: Some(animation.frame(time_seconds)),
            height: 0.0,
            shadow_height: 0.0, // Animations don't have height so they don't have shadows.
            sprite: animation.name,
            transform: Transform::from_position(animation.position),
        }
    }

    fn deck_height(entity_type: EntityType) -> f32 {
        let data = entity_type.data();
        if data.sub_kind == EntitySubKind::Submarine {
            // Submarines surface halfway in water so no shadows.
            return 0.0;
        }

        // TODO more accurate height for all ships.
        if data.mast != Altitude::ZERO {
            let mast = data.mast.to_meters();
            mast * 0.25
        } else {
            let mut height = data.width.min(data.length) * 0.33;
            if data.kind == EntityKind::Aircraft {
                // Aircraft have wings/blades that are wide compared to the height of the aircraft.
                height *= 0.66;
            } else if data.kind == EntityKind::Obstacle && data.sub_kind != EntitySubKind::Tree {
                // Oil rigs aren't that tall.
                height *= 0.66;
            }
            height
        }
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
                .partial_cmp(&other.altitude)
                .unwrap()
                .then_with(|| self.entity_id.cmp(&other.entity_id)),
        )
    }
}
