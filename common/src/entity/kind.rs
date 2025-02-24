use crate::ticks::Ticks;
use std::ops::RangeInclusive;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum EntityKind {
    Aircraft,
    Boat,
    Collectible,
    Decoy,
    Obstacle,
    Turret,
    Weapon,
}

impl EntityKind {
    /// Largest possible `Self::keep_alive()` return value.
    pub const MAX_KEEP_ALIVE: Ticks = Ticks::from_repr(10);

    /// After how many ticks of not hearing about an entity should we assume it is gone/no longer
    /// visible. This allows the server to optimize bandwidth usage but transmitting certain entities
    /// less frequently.
    ///
    /// The higher end of the range is used (for efficiency) except if the velocity is above
    /// a certain threshold.
    ///
    /// To guarantee some updates are sent, make sure the (start + 1) divides (end + 1).
    pub const fn keep_alive(self) -> RangeInclusive<Ticks> {
        match self {
            Self::Boat | Self::Decoy | Self::Weapon | Self::Aircraft | Self::Turret => {
                Ticks::from_repr(0)..=Ticks::from_repr(0)
            }
            Self::Collectible => Ticks::from_repr(2)..=Ticks::from_repr(5),
            Self::Obstacle => Self::MAX_KEEP_ALIVE..=Self::MAX_KEEP_ALIVE,
        }
    }
}
