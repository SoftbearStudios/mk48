use crate::altitude::Altitude;
use crate::entity::{Armament, EntityKind, EntitySubKind, Exhaust, Sensors, Turret};
use crate::ticks;
use crate::ticks::Ticks;
use crate::transform::Transform;
use crate::velocity::Velocity;
use common_util::angle::Angle;
use common_util::range::map_ranges_fast;
use glam::Vec2;
use std::ops::Range;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct EntityData {
    pub kind: EntityKind,
    pub sub_kind: EntitySubKind,
    pub level: u8,
    pub limited: bool,
    pub npc: bool,
    pub lifespan: Ticks,
    pub reload: Ticks,
    pub speed: Velocity, // Guaranteed to get the attention of any physicist.
    pub length: f32,
    pub width: f32,
    pub draft: Altitude, // Type is a bit cheeky but kind of makes sense.
    pub mast: Altitude,
    pub depth: Altitude,
    pub radius: f32,
    pub inv_size: f32,
    pub damage: f32,
    pub anti_aircraft: f32,
    pub ram_damage: f32,
    pub torpedo_resistance: f32,
    pub stealth: f32,
    pub sensors: Sensors,
    pub armaments: &'static [Armament],
    pub turrets: &'static [Turret],
    pub exhausts: &'static [Exhaust],
    pub label: &'static str,
    pub link: Option<&'static str>,
    pub range: f32,
    pub position_forward: f32,
    pub position_side: f32,
}

impl EntityData {
    /// Missiles, rockets, SAMs, etc. that are rising from a submerged submarine don't move
    /// horizontally (very fast) until they reach the surface.
    pub const SURFACING_PROJECTILE_SPEED_LIMIT: f32 = 0.5;

    /// Constant used for checking whether a depth charge should explode.
    pub const DEPTH_CHARGE_PROXIMITY: f32 = 30.0;

    /// radii range of throttle (0-100%) and limit of collecting things.
    pub fn radii(&self) -> Range<f32> {
        self.length * 0.55..self.length
    }

    /// dimensions returns a Vec2 with the x component equal to the length and the y component equal to the width.
    pub fn dimensions(&self) -> Vec2 {
        Vec2::new(self.length, self.width)
    }

    /// offset returns an offset to use while rendering.
    pub fn offset(&self) -> Vec2 {
        Vec2::new(self.position_forward, self.position_side)
    }

    /// returns area, in square meters, of vision.
    pub fn visual_area(&self) -> f32 {
        self.sensors.visual.range.powi(2) * std::f32::consts::PI
    }

    /// The expected radius of a square view.
    pub fn camera_range(&self) -> f32 {
        // Reduce camera range to to fill more of screen with visual field.
        self.sensors.visual.range * 0.75
    }

    /// Range of anti aircraft guns (whereas `self.anti_aircraft` is their power).
    pub fn anti_aircraft_range(&self) -> f32 {
        self.radii().end
    }

    /// max_health returns the the minimum damage to kill a boat, panicking if the corresponding
    /// entity does not have health.
    pub fn max_health(&self) -> Ticks {
        if self.kind == EntityKind::Boat {
            return ticks::from_damage(self.damage);
        }
        unreachable!("only boats have health");
    }

    /// Returns multiplier for damage due to given sub kind.
    pub fn resistance_to_subkind(&self, sub_kind: EntitySubKind) -> f32 {
        1.0 - match sub_kind {
            EntitySubKind::Torpedo => self.torpedo_resistance,
            _ => 0.0,
        }
    }

    /// Returns minimum cavitation (making noisy bubbles) speed.
    pub fn cavitation_speed(&self, altitude: Altitude) -> Velocity {
        let lo = Velocity::from_knots(8.0);
        let hi = Velocity::from_knots(12.0);
        Velocity::from_mps(
            map_ranges_fast(
                altitude.to_norm(),
                0.0..-1.0,
                lo.to_mps()..hi.to_mps(),
                true,
                false, // to_norm can't return less than -1 (high).
            ) * (1.0 + self.stealth),
        )
    }

    /// armament_transform returns the entity-relative transform of a given armament.
    pub fn armament_transform(&self, turret_angles: &[Angle], index: usize) -> Transform {
        let armament = &self.armaments[index];
        let mut transform = Transform {
            position: armament.position(),
            direction: armament.angle,
            velocity: Velocity::ZERO,
        };

        let weapon_data = armament.entity_type.data();

        // Shells start with all their velocity.
        if weapon_data.sub_kind == EntitySubKind::Shell {
            transform.velocity = weapon_data.speed
        } else if weapon_data.sub_kind == EntitySubKind::Plane {
            // Planes must attain minimum airspeed.
            transform.velocity = weapon_data.speed * 0.5;
        } else if armament.turret.is_some() && weapon_data.sub_kind == EntitySubKind::Torpedo {
            // Compressed gas.
            transform.velocity = Velocity::from_mps(10.0);
        } else if !armament.vertical {
            // Minimal launch velocity (except if vertical, in which case only initial velocity is up).
            transform.velocity = Velocity::from_mps(1.0);
        }

        if let Some(turret_index) = armament.turret {
            let turret = &self.turrets[turret_index];
            transform = Transform {
                position: turret.position(),
                direction: turret_angles[turret_index],
                velocity: Velocity::ZERO,
            } + transform;
        }
        transform
    }

    /// update_turret_aim brings turret_angles delta_seconds closer to position_target.
    pub fn update_turret_aim(
        &self,
        boat_transform: Transform,
        turret_angles: &mut [Angle],
        position_target: Option<Vec2>,
        delta_seconds: f32,
    ) {
        for (i, a) in turret_angles.iter_mut().enumerate() {
            let turret = &self.turrets[i];
            let amount = Angle::from_radians(
                (delta_seconds * turret.speed.to_radians()).clamp(0.0, std::f32::consts::PI),
            );
            let mut direction_target = turret.angle;
            if let Some(target) = position_target {
                let turret_global_transform = boat_transform
                    + Transform {
                        position: turret.position(),
                        direction: *a,
                        velocity: Velocity::ZERO,
                    };
                let global_direction = Angle::from(target - turret_global_transform.position);
                direction_target = global_direction - boat_transform.direction;
            }
            let delta_angle = (direction_target - *a).clamp_magnitude(amount);

            // Allow turning through, but not stopping in, restricted angles
            if delta_angle != Angle::ZERO
                && (turret.within_azimuth(*a + delta_angle)
                    || turret.within_azimuth(direction_target))
            {
                *a += delta_angle
            }
        }
    }
}
