// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use common::transform::Transform;
use glam::Vec2;

/// radius_collision performs a simple radius check. This is faster but less accurate than SAT.
pub fn radius_collision(
    transform: Transform,
    radius: f32,
    other_transform: Transform,
    other_radius: f32,
    delta_seconds: f32,
) -> bool {
    let sweep = transform.velocity.to_mps() * delta_seconds;
    let other_sweep = other_transform.velocity.to_mps() * delta_seconds;

    let d2 = transform
        .position
        .distance_squared(other_transform.position);
    let r2 = (radius + other_radius + sweep + other_sweep).powi(2);

    d2 <= r2
}

/// sat_collision performs continuous rectangle-based separating axis theorem collision.
pub fn sat_collision(
    mut transform: Transform,
    mut dimensions: Vec2,
    radius: f32,
    mut other_transform: Transform,
    mut other_dimensions: Vec2,
    other_radius: f32,
    delta_seconds: f32,
) -> bool {
    let sweep = transform.velocity.to_mps() * delta_seconds;
    let other_sweep = other_transform.velocity.to_mps() * delta_seconds;

    let d2 = transform
        .position
        .distance_squared(other_transform.position);
    let r2 = (radius + other_radius + sweep + other_sweep).powi(2);
    if d2 > r2 {
        return false;
    }

    let axis_normal = transform.direction.to_vec();
    let other_axis_normal = other_transform.direction.to_vec();

    transform.position += axis_normal * (sweep * 0.5);
    other_transform.position += other_axis_normal * (other_sweep * 0.5);

    dimensions.x += sweep;
    other_dimensions.x += other_sweep;

    // Make math easier later on
    other_dimensions *= 0.5;
    dimensions *= 0.5;

    sat_collision_half(
        transform.position,
        other_transform.position,
        axis_normal,
        other_axis_normal,
        dimensions,
        other_dimensions,
    ) && sat_collision_half(
        other_transform.position,
        transform.position,
        other_axis_normal,
        axis_normal,
        other_dimensions,
        dimensions,
    )
}

/// sat_collision_half performs half an SAT test (checks angles of one of two rectangles).
fn sat_collision_half(
    position: Vec2,
    other_position: Vec2,
    mut axis_normal: Vec2,
    other_axis_normal: Vec2,
    dimensions: Vec2,
    other_dimensions: Vec2,
) -> bool {
    let other_axis_tangent = other_axis_normal.perp();

    let other_ps: [Vec2; 4] = [
        other_position
            + other_axis_normal * other_dimensions.x
            + other_axis_tangent * other_dimensions.y,
        other_position + other_axis_normal * other_dimensions.x
            - other_axis_tangent * other_dimensions.y,
        other_position
            - other_axis_normal * other_dimensions.x
            - other_axis_tangent * other_dimensions.y,
        other_position - other_axis_normal * other_dimensions.x
            + other_axis_tangent * other_dimensions.y,
    ];

    for f in 0..4 {
        let dimension = if f % 2 == 0 {
            dimensions.x
        } else {
            dimensions.y
        };

        let dot = position.dot(axis_normal);

        // Dimension is always positive, so min < max.
        let min = dot - dimension;
        let max = dot + dimension;

        let mut less = true;
        let mut greater = true;

        for other_p in other_ps {
            let d = other_p.dot(axis_normal);
            less &= d < min;
            greater &= d > max;
        }

        if less || greater {
            return false;
        }

        // Start over with next axis.
        axis_normal = axis_normal.perp();
    }

    true
}
