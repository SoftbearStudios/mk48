use crate::entity::EntitySubKind;
use crate::entity::EntityType;
use glam::{vec2, Vec2};

/// For testing larger world sizes.
pub const SIZE: usize = 1;

/// Everything with a y coordinate above this is in the arctic biome.
pub const ARCTIC: f32 = 1250.0;

/// Everything with a y coordinate below this is in the tropics biome.
pub const TROPICS: f32 = -2250.0;

// TODO: Would it make more sense to represent areas as [`Range<f32>`]?

/// Returns if an entity is within it's spawnable area such as ocean for dredger or arctic for icebreaker.
pub fn outside_strict_area(entity_type: EntityType, position: Vec2) -> bool {
    distance_to_strict_area_border(entity_type, position)
        .map(|d| d < 0.0)
        .unwrap_or(false)
}

/// Returns a clamped y position of an entity to it's reachable area's border.
pub fn clamp_y_to_strict_area_border(entity_type: EntityType, y: f32) -> f32 {
    strict_area_border(entity_type)
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

/// Returns the distance to the entity's reachable area's border such as distance to arctic for dredger.
/// If it's negative the entity is behind the border and should be moved.
pub fn distance_to_strict_area_border(entity_type: EntityType, position: Vec2) -> Option<f32> {
    strict_area_border(entity_type).map(|(height, above)| {
        if above {
            height - position.y
        } else {
            position.y - height
        }
    })
}

/// Returns an option containing the y position of the area's border and if it's above or below.
pub fn strict_area_border(entity_type: EntityType) -> Option<(f32, bool)> {
    Some(match entity_type.data().sub_kind {
        EntitySubKind::Dredger => (ARCTIC, true),
        EntitySubKind::Icebreaker => (ARCTIC, false),
        _ => return None,
    })
}

/// Returns the distance to the entity's spawnable area's border such as distance to arctic for dredger.
/// If it's negative the entity is behind the border and should be moved.
pub fn distance_to_soft_area_border(entity_type: EntityType, position: Vec2) -> f32 {
    let (height, above) = soft_area_border(entity_type);
    if above {
        height - position.y
    } else {
        position.y - height
    }
}

/// Returns an option containing the y position of the area's border and if it's above or below.
pub fn soft_area_border(entity_type: EntityType) -> (f32, bool) {
    match entity_type.data().sub_kind {
        EntitySubKind::Icebreaker => (ARCTIC, false),
        _ => (ARCTIC, true),
    }
}

/// Returns an option containing the normal of the area's border.
/// The normal points from the border to where it ends.
pub fn strict_area_border_normal(entity_type: EntityType) -> Option<Vec2> {
    strict_area_border(entity_type).map(|(_, above)| {
        if above {
            vec2(0.0, -1.0)
        } else {
            vec2(0.0, 1.0)
        }
    })
}
