// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::num::NonZeroU32;

mod _type;
mod armament;
mod data;
mod exhaust;
mod kind;
mod sensor;
mod sub_kind;
mod turret;

pub type EntityId = NonZeroU32;
pub use _type::EntityType;
pub use armament::Armament;
pub use data::EntityData;
pub use exhaust::Exhaust;
pub use kind::EntityKind;
pub use sensor::{Sensor, Sensors};
pub use sub_kind::EntitySubKind;
pub use turret::Turret;

#[cfg(test)]
mod tests {
    use crate::entity::{EntityKind, EntityType};

    #[test]
    fn weapon_sensors() {
        let mut ranges: Vec<(EntityType, f32)> = EntityType::iter()
            .map(|typ| (typ, typ.data().sensors.max_range()))
            .collect();
        ranges.sort_by_key(|(_, f)| *f as usize);
        for (typ, range) in ranges {
            if typ.data().kind == EntityKind::Boat {
                continue;
            }
            println!("{:?} sensor range is {}", typ, range);
        }
    }
}
