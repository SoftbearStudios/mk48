// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use common::entity::{EntityId, EntityType};
use common::transform::Transform;
use glam::Vec2;
use std::cmp::Ordering;

pub struct SortableSprite<'a> {
    pub alpha: f32,
    pub altitude: f32,
    pub dimensions: Vec2,
    pub entity_id: Option<EntityId>,
    pub frame: Option<usize>,
    pub sprite: &'a str,
    pub transform: Transform,
}

impl<'a> SortableSprite<'a> {
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

    pub fn new_animation(animation: &Animation) -> Self {
        Self {
            alpha: 1.0,
            altitude: animation.altitude,
            dimensions: Vec2::splat(animation.scale),
            entity_id: None,
            frame: Some(animation.frame),
            sprite: animation.name,
            transform: Transform::from_position(animation.position),
        }
    }

    pub fn entity_height(entity_type: EntityType) -> f32 {
        entity_type.data().length * 0.0001
    }
}

impl<'a> PartialEq for SortableSprite<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.altitude == other.altitude && self.entity_id == other.entity_id
    }
}

impl<'a> PartialOrd for SortableSprite<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.altitude
                .partial_cmp(&other.altitude)?
                .then_with(|| self.entity_id.cmp(&other.entity_id)),
        )
    }
}
