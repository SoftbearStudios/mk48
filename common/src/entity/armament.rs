use crate::entity::EntityType;
use crate::ticks::Ticks;
use kodiak_common::glam::Vec2;
use kodiak_common::Angle;

#[derive(Clone, Debug)]
pub struct Armament {
    pub entity_type: EntityType,
    pub reload_override: Option<Ticks>,
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
        self.reload_override
            .unwrap_or_else(|| self.entity_type.data().reload)
    }

    pub fn position(&self) -> Vec2 {
        Vec2::new(self.position_forward, self.position_side)
    }

    /// is_similar_to reports if two armaments are similar enough to reload
    /// together (presumably will be grouped in GUI).
    pub fn is_similar_to(&self, other: &Self) -> bool {
        let ret = self.entity_type == other.entity_type && self.turret == other.turret;
        debug_assert!(!ret || self.reload_override == other.reload_override);
        ret
    }
}
