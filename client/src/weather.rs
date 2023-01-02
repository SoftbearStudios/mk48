use glam::{vec2, vec3, Vec2, Vec3};
use std::f32::consts::TAU;

#[derive(Copy, Clone, PartialEq)]
pub struct Weather {
    // Points towards the sun.
    /// X/Y are aligned to world space X/Y and +Z is towards the camera.
    pub sun: Vec3,
    /// Points in the direction of the wind.
    pub wind: Vec2,
    // TODO make wind change wave height
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            sun: vec3(0.5, 0.5, 0.8).normalize(),
            wind: vec2(7.0, 1.5),
        }
    }
}

impl Weather {
    // Test [`Weather`] with unrealistic conditions.
    const TEST: bool = false;

    pub fn new(time: f32) -> Self {
        if Self::TEST {
            // Make sun sin fast for testing.
            let (x, y) = (time * (3.0 / TAU)).sin_cos();
            let sun = (vec2(x, y) * 0.5).extend(0.7).normalize();

            // Increase wind for testing.
            let wind = Self::default().wind * 1.0;
            Self { sun, wind }
        } else {
            Self::default()
        }
    }

    // Since the camera is orthographic, if the sun was pointing straight down the water would be
    // way too bright so we give it a different sun direction.
    pub fn water_sun(&self) -> Vec3 {
        // Creates a vector with sun's xy direction but with z always the same.
        let z = 0.6964f32;
        let mut dir = self.sun.truncate();
        if dir == Vec2::ZERO {
            dir = Vec2::ONE;
        }
        dir = dir.normalize();

        let normal = (dir * (1.0 - z.powi(2)).sqrt()).extend(z);
        debug_assert!(normal.is_normalized());
        normal
    }
}
