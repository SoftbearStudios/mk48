use crate::entity::EntityType;
use crate::ticks::Ticks;
use common_util::angle::Angle;
use glam::Vec2;

#[derive(Clone, Debug)]
pub struct Armament {
    pub entity_type: EntityType,
    pub hidden: bool,
    pub external: bool,
    pub vertical: bool,
    pub position_forward: f32,
    pub position_side: f32,
    pub angle: Angle,
    pub turret: Option<usize>,
}

impl Armament {
    pub fn reload(&self) -> Ticks {
        self.entity_type.data().reload
    }

    pub fn position(&self) -> Vec2 {
        Vec2::new(self.position_forward, self.position_side)
    }

    /// is_similar_to reports if two armaments are similar enough to reload
    /// together (presumably will be grouped in GUI).
    pub fn is_similar_to(&self, other: &Self) -> bool {
        self.entity_type == other.entity_type && self.turret == other.turret
    }
}
