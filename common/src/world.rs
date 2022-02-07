use crate::entity::EntitySubKind;
use crate::entity::EntityType;
use glam::{vec2, Vec2};

/// For testing larger world sizes.
pub const SIZE: usize = 1;

/// Everything with a y coordinate above this is in the arctic biome.
pub const ARCTIC: f32 = 1250.0;

/// Returns if an entity is within it's area such as ocean for dredger or arctic for icebreaker.
pub fn outside_area(entity_type: EntityType, position: Vec2) -> bool {
    distance_to_area_border(entity_type, position)
        .map(|d| d < 0.0)
        .unwrap_or(false)
}

/// Returns a clamped y position of an entity to it's area's border.
pub fn clamp_y_to_area_border(entity_type: EntityType, y: f32) -> f32 {
    area_border(entity_type)
        .map(
            |(height, above)| {
                if above {
                    y.min(height)
                } else {
                    y.max(height)
                }
            },
        )
        .unwrap_or(y)
}

/// Returns a clamped y position of an entity to it's area's border or ocean.
/// Clamps distance away from the border.
pub fn clamp_y_to_default_area_border(entity_type: EntityType, y: f32, distance: f32) -> f32 {
    area_border(entity_type)
        .map(|(height, above)| {
            if above {
                y.min(height - distance)
            } else {
                y.max(height + distance)
            }
        })
        .unwrap_or(y.min(ARCTIC - distance))
}

/// Returns the distance to the entity's area's border such as distance to arctic for dredger.
/// If it's negative the entity is behind the border and should be moved.
pub fn distance_to_area_border(entity_type: EntityType, position: Vec2) -> Option<f32> {
    area_border(entity_type).map(|(height, above)| {
        if above {
            height - position.y
        } else {
            position.y - height
        }
    })
}

/// Returns an option containing the y position of the area's border and if it's above or below.
pub fn area_border(entity_type: EntityType) -> Option<(f32, bool)> {
    Some(match entity_type.data().sub_kind {
        EntitySubKind::Dredger => (ARCTIC, true),
        EntitySubKind::Icebreaker => (ARCTIC, false),
        _ => return None,
    })
}

/// Returns an option containing the normal of the area's border.
/// The normal points from the border to where it ends.
pub fn area_border_normal(entity_type: EntityType) -> Option<Vec2> {
    area_border(entity_type).map(|(_, above)| {
        if above {
            vec2(0.0, -1.0)
        } else {
            vec2(0.0, 1.0)
        }
    })
}
