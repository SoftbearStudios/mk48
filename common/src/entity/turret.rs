use crate::entity::EntityType;
use common_util::angle::Angle;
use glam::Vec2;

#[derive(Clone, Debug)]
pub struct Turret {
    pub entity_type: Option<EntityType>,
    pub position_forward: f32,
    pub position_side: f32,
    pub angle: Angle,
    pub speed: Angle,
    pub azimuth_fl: Angle,
    pub azimuth_fr: Angle,
    pub azimuth_bl: Angle,
    pub azimuth_br: Angle,
}

impl Turret {
    pub fn position(&self) -> Vec2 {
        Vec2::new(self.position_forward, self.position_side)
    }

    /// within_azimuth returns whether the given boat-relative angle is within the azimuth (horizontal
    /// angle) limits, if any.
    pub fn within_azimuth(&self, curr: Angle) -> bool {
        /*
        Angles are counterclockwise.
        Each turret.azimuth_** angle is a restriction starting in the respective quadrant.
        ------------BL-----------FL-BR--------\
        |           ---- o=== ----             \
        |           BR    ^      FR BL          |  <-- boat
        |               turret       ^-flipped /
        --------------------------------------/
         */

        // The angle as it relates to the front azimuth limits.
        let azimuth_f = curr - self.angle;
        if -self.azimuth_fr < azimuth_f && azimuth_f < self.azimuth_fl {
            false
        } else {
            // The angle as it relates to the back azimuth limits.
            let azimuth_b = Angle::PI + curr - self.angle;
            !(-self.azimuth_bl < azimuth_b && azimuth_b < self.azimuth_br)
        }
    }
}
