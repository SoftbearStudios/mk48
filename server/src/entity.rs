// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::collision::{radius_collision, sat_collision};
use crate::entities::*;
use crate::entity_extension::EntityExtension;
use crate::player::*;
use atomic_refcell::{AtomicRef, AtomicRefMut};
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::*;
use common::guidance::Guidance;
use common::terrain::*;
use common::ticks::{Ticks, TicksRepr};
use common::transform::{DimensionTransform, Transform};
use glam::Vec2;
use std::ptr;
use std::sync::Arc;
use std::time::Instant;

/// A game object, such as a boat, weapon, or obstacle.
/// Lots of effort is made to keep this only 32 bytes, to improve cache locality.
#[derive(Debug)]
pub struct Entity {
    pub transform: Transform,
    pub guidance: Guidance,
    /// Cannot change without calling change_entity_type.
    pub entity_type: EntityType,
    pub altitude: Altitude,
    /// Unique id, useful for communicating contacts between client and server.
    pub id: EntityId,
    /// All boats, aircraft, decoys, weapons, and paid coins have `Some`, everything else has `None`.
    pub player: Option<Arc<PlayerTuple>>,
    /// When it represents damage, it is less than or equal to self.data().max_health(). Otherwise,
    /// it represents lifetime (for entities with finite lifespan).
    pub ticks: Ticks,
}

/// unset_entity_id returns a nonexistent id that will be overwritten by world.add
pub fn unset_entity_id() -> EntityId {
    EntityId::new(u32::MAX).unwrap()
}

impl Entity {
    /// Allocates a new entity with some blank fields that should probably be populated, e.g. transform.
    pub fn new(entity_type: EntityType, player: Option<Arc<PlayerTuple>>) -> Self {
        Self {
            transform: Transform::default(),
            guidance: Guidance::new(),
            entity_type,
            id: unset_entity_id(),
            altitude: Altitude::ZERO,
            player,
            ticks: Ticks::ZERO,
        }
    }

    /// extension gets the extension of the entity.
    /// It is safe because it requires a shared reference
    /// to the entity which is the sole owner of the extension.
    pub fn extension(&self) -> &EntityExtension {
        assert!(self.is_boat());
        unsafe { self.player.as_ref().unwrap().unsafe_extension() }
    }

    /// extension_mut gets the extension of the entity.
    /// It is safe because it requires a mutable reference
    /// to the entity which is the sole owner of the extension.
    pub fn extension_mut(&mut self) -> &mut EntityExtension {
        assert!(self.is_boat());
        unsafe { self.player.as_ref().unwrap().unsafe_extension_mut() }
    }

    /// change_entity_type is the only valid way to change an entity's type.
    pub fn change_entity_type(&mut self, entity_type: EntityType, arena: &mut Arena) {
        let old_data = self.data();
        debug_assert_eq!(old_data.kind, entity_type.data().kind);

        // Update entity type counts.
        arena.change_type(self.entity_type, entity_type);

        self.entity_type = entity_type;

        if entity_type.data().kind != EntityKind::Boat {
            // Not a boat, no additional consideration needed.
            return;
        }

        let new_data = self.data();

        // Regen half of damage (as a fraction). Get original damage fraction before changing
        // entity type. Never result in boat being dead.
        let damage_fraction = self.ticks.to_secs() / old_data.max_health().to_secs();
        let new_damage_fraction = damage_fraction * 0.5;
        let max_health = new_data.max_health();
        self.ticks = (max_health * new_damage_fraction).min(max_health - Ticks::ONE);

        // Changing entity type cannot leave an entity with more damage than health. Other code
        // may crash if this invalid state is propagated. Also, it would be nice if boat isn't
        // immediately dead.
        debug_assert!(self.ticks < max_health);

        let extension = self.extension_mut();

        // Save some settings from the old extension.
        let old_active = extension.active;
        let old_altitude_target = extension.altitude_target;

        // Keep armament (lack of) reloads. Use usize ot avoid overflow.
        // Start by counting the total ticks left to reload (for non-limited armaments).
        let mut total_reload = 0;
        for (i, reload) in extension.reloads.iter().enumerate() {
            if !old_data.armaments[i].entity_type.data().limited {
                total_reload += reload.0 as usize;
            }
        }

        // Change the extension to correspond with the new type.
        *extension = EntityExtension::new(entity_type);

        // Restore some settings from the old extension.
        extension.set_active(old_active);
        extension.altitude_target = old_altitude_target;

        // Finish (un)reloading.
        for (i, reload) in extension.reloads_mut().iter_mut().enumerate() {
            let armament = &new_data.armaments[i];
            if !armament.entity_type.data().limited {
                let to_consume = (armament.reload().0 as usize).min(total_reload);
                *reload = Ticks(to_consume as TicksRepr);
                total_reload -= to_consume;
                if total_reload == 0 {
                    break;
                }
            }
        }

        // extension dropped, can call methods that get it themselves now:
        self.update_turret_aim(10.0);
    }

    /// Adds a reference from player to self.
    pub fn create_index(&mut self, i: EntityIndex) {
        if let Some(ref player) = self.player {
            let mut player = player.borrow_mut();
            assert!(!player.status.is_alive());
            player.status = Status::Alive {
                entity_index: i,
                aim_target: None,
                time: Instant::now(),
            };
        } else {
            return;
        }
        *self.extension_mut() = EntityExtension::new(self.entity_type);
    }

    /// Adjusts player's pointer to self, if applicable.
    pub fn set_index(&mut self, i: EntityIndex) {
        if let Some(ref player) = self.player {
            player.borrow_mut().status.set_entity_index(i);
        }
    }

    /// Set's player to dead, removing reference to self, if applicable.
    pub fn delete_index(&mut self, reason: DeathReason) {
        if let Some(ref player) = self.player {
            player.borrow_mut().status = Status::Dead {
                reason,
                position: self.transform.position,
                time: Instant::now(),
                visual_range: self.data().sensors.visual.range,
            }
        }
    }

    /// Borrows player immutably.
    /// Must be manually serialized to avoid contention.
    /// Panics if entity doesn't have player.
    pub fn borrow_player(&self) -> AtomicRef<Player> {
        Self::borrow_player_inner(&self.player)
    }

    fn borrow_player_inner(player: &Option<Arc<PlayerTuple>>) -> AtomicRef<Player> {
        player
            .as_ref()
            .expect("only call on entities that are guaranteed to have players")
            .borrow()
    }

    /// Borrows player mutably.
    /// Must be manually serialized to avoid contention.
    /// Panics if entity doesn't have player.
    pub fn borrow_player_mut(&mut self) -> AtomicRefMut<Player> {
        Self::borrow_player_mut_inner(&mut self.player)
    }

    pub fn borrow_player_mut_inner(player: &mut Option<Arc<PlayerTuple>>) -> AtomicRefMut<Player> {
        player
            .as_ref()
            .expect("only call on entities that are guaranteed to have players")
            .borrow_mut()
    }

    /// Get's the entity's data, corresponding to it's current entity type.
    pub fn data(&self) -> &'static EntityData {
        self.entity_type.data()
    }

    /// Returns true if and only if the entity is of kind boat.
    pub fn is_boat(&self) -> bool {
        self.entity_type.data().kind == EntityKind::Boat
    }

    /// Determines if two entities would collide if delta_seconds elapsed.
    pub fn collides_with(&self, other: &Self, delta_seconds: f32) -> bool {
        let data = self.data();
        let other_data = other.data();

        if data.sub_kind == EntitySubKind::Sam || other_data.sub_kind == EntitySubKind::Sam {
            // SAMs collide if within radius, simulating their blast-fragmentation warheads.
            radius_collision(
                self.transform,
                data.radius,
                other.transform,
                other_data.radius,
                delta_seconds,
            )
        } else {
            sat_collision(
                self.transform,
                data.dimensions(),
                data.radius,
                other.transform,
                other_data.dimensions(),
                other_data.radius,
                delta_seconds,
            )
        }
    }

    /// Combines transform and dimensions.
    pub fn dimension_transform(&self) -> DimensionTransform {
        DimensionTransform {
            transform: self.transform,
            dimensions: self.data().dimensions(),
        }
    }

    /// Determines whether an entity collides with the terrain (underwater terrain ignored to avoid
    /// damaging submarines when going from deep to shallow; submarines must be forced to rise elsewhere).
    pub fn collides_with_terrain(&self, t: &Terrain, delta_seconds: f32) -> bool {
        // If submerged, colliding with terrain should be relatively temporary (boats should simply
        // rise up rather than taking damage) so it is ignored.
        t.collides_with(
            self.dimension_transform(),
            self.altitude.max(Altitude::ZERO),
            delta_seconds,
        )
    }

    /// Updates the aim of all turrets, assuming delta_seconds have elapsed.
    pub fn update_turret_aim(&mut self, delta_seconds: f32) {
        let aim_target = if let Status::Alive { aim_target, .. } = &self.borrow_player().status {
            *aim_target
        } else {
            panic!("boat's player was not alive in update_turret_aim()");
        };

        self.data().update_turret_aim(
            self.transform,
            self.extension_mut().turrets_mut(),
            aim_target,
            delta_seconds,
        );
    }

    /// Marks a particular armament as consumed.
    pub fn consume_armament(&mut self, index: usize) {
        //entity.clear_spawn_protection();

        let a = &self.data().armaments[index];

        // Limited armaments start their timer when they die.
        let reload = if a.entity_type.data().limited {
            Ticks::MAX
        } else {
            a.reload()

            /*
            // Submerged submarines reload slower
            if entity.Owner.ext.altitude() < 0 {
                reload *= 2
            }
             */
        };

        self.extension_mut().reloads_mut()[index] = reload;
    }

    /// Repairs by a certain amount, up to maximum health.
    pub fn repair(&mut self, amount: Ticks) {
        self.ticks = self.ticks.saturating_sub(amount);
    }

    /// Reloads arbitrary armaments/groups by a certain amount.
    pub fn reload(&mut self, amount: Ticks) {
        let armaments = &self.data().armaments;
        let extension = self.extension_mut();
        let reloads = extension.reloads_mut();
        if reloads.is_empty() {
            return;
        }
        let mut current = &armaments[0];
        let mut start = 0;

        for (end, next) in armaments.iter().enumerate() {
            if next.is_similar_to(current) {
                continue;
            }
            Self::reload_range(reloads, amount, start, end);
            current = next;
            start = end;
        }

        // Final iteration
        Self::reload_range(reloads, amount, start, armaments.len());
    }

    fn reload_range(reloads: &mut [Ticks], mut amount: Ticks, start: usize, end: usize) {
        while amount != Ticks::ZERO {
            let mut i = None;

            // Find the armament with the lowest consumption (to prioritize full reloads).
            // Limited are ticks max and won't be counted.
            let mut consumption = Ticks::MAX;
            for (j, c) in reloads[start..end].iter_mut().enumerate() {
                if *c != Ticks::ZERO && *c < consumption {
                    i = Some(j + start);
                    consumption = *c;
                }
            }

            if let Some(i) = i {
                debug_assert_eq!(reloads[i], consumption);

                let consume = consumption.min(amount);
                consumption -= consume;
                amount -= consume;

                reloads[i] = consumption;
            } else {
                // No armament has yet to be fully replenished, so this range is done.
                break;
            }
        }
    }

    /// Damage damages an entity and returns if it died.
    pub fn damage(&mut self, amount: Ticks) -> bool {
        let data = self.data();

        // Ticks is lifespan for non-boats.
        if data.kind != EntityKind::Boat {
            panic!("probably shouldn't be calling damage on non-boats");
            //return amount != 0
        }

        self.ticks = self.ticks.saturating_add(amount).min(data.max_health());
        self.ticks == data.max_health()
    }

    /// Returns whether a given amount of damage would kill an entity.
    #[allow(dead_code)]
    pub fn would_kill(&self, damage: Ticks) -> bool {
        self.ticks.saturating_add(damage) >= self.data().max_health()
    }

    /// Apply damage to ultimately kill an entity in kill_time, assuming delta ticks elapsed. Returns true if now dead.
    pub fn kill_in(&mut self, delta: Ticks, kill_time: Ticks) -> bool {
        self.damage(delta * (self.data().max_health() / kill_time).max(Ticks::ONE))
    }

    /// Returns true if the entity obeys special altitude mechanics (overlaps a wider altitude range),
    /// which is useful for unguided weapons that, were they not able to hit certain targets, would be
    /// underpowered.
    fn special_altitude_overlap(&self) -> bool {
        let data = self.data();
        data.sub_kind == EntitySubKind::Torpedo && !data.sensors.any()
    }

    /// Returns true if two entities are overlapping, only taking into account their altitudes.
    pub fn altitude_overlapping(&self, other: &Self) -> bool {
        if (self.altitude > Altitude::ZERO && other.altitude < Altitude::ZERO)
            || (self.altitude < Altitude::ZERO && other.altitude > Altitude::ZERO)
        {
            // Entities above water should never collide with entities below water.
            return false;
        }
        self.altitude.difference(other.altitude)
            <= if self.special_altitude_overlap() || other.special_altitude_overlap() {
                Altitude::SPECIAL_OVERLAP_MARGIN
            } else {
                Altitude::OVERLAP_MARGIN
            }
    }

    /// Returns amount altitude changed by.
    /// The speed parameter, which must be an integer, can be used to make one change of altitude have
    /// higher authority than another.
    pub fn apply_altitude_target(
        &mut self,
        terrain: &Terrain,
        target: Option<Altitude>,
        speed: f32,
        delta: Ticks,
    ) -> Altitude {
        let data = self.data();

        // This gives a decently wide collision spectrum (that includes Altitude::ZERO).
        let unguided_weapon_altitude: Altitude = if self.special_altitude_overlap() {
            Altitude::SPECIAL_OVERLAP_MARGIN
        } else {
            Altitude::OVERLAP_MARGIN
        };

        // max and min target altitudes.
        let max_altitude = match data.kind {
            EntityKind::Boat => Altitude::ZERO,
            EntityKind::Aircraft => Altitude::MAX,
            EntityKind::Weapon => match data.sub_kind {
                EntitySubKind::Missile
                | EntitySubKind::Sam
                | EntitySubKind::Rocket
                | EntitySubKind::Shell => Altitude::MAX,
                _ => Altitude::ZERO,
            },
            EntityKind::Decoy => match data.sub_kind {
                EntitySubKind::Sonar => Altitude::MIN,
                _ => Altitude::ZERO,
            },
            _ => Altitude::ZERO,
        };

        let min_altitude = (terrain
            .sample(self.transform.position)
            .unwrap_or(Altitude::MIN)
            .max(if data.sub_kind == EntitySubKind::Submarine {
                -data.depth
            } else {
                Altitude::MIN
            })
            + data.draft.max(Altitude::UNIT))
        .min(max_altitude);

        let target_altitude = match data.kind {
            EntityKind::Boat => match data.sub_kind {
                EntitySubKind::Submarine => target.unwrap_or(Altitude::ZERO),
                _ => Altitude::ZERO,
            },
            EntityKind::Weapon => match data.sub_kind {
                EntitySubKind::Torpedo => target.unwrap_or(-unguided_weapon_altitude),
                EntitySubKind::DepthCharge => Altitude::MIN, // Sink to bottom.
                EntitySubKind::Mine => -unguided_weapon_altitude,
                EntitySubKind::Missile => unguided_weapon_altitude,
                EntitySubKind::Rocket => unguided_weapon_altitude,
                EntitySubKind::Sam => target.unwrap_or(unguided_weapon_altitude),
                EntitySubKind::Shell => unguided_weapon_altitude,
                _ => {
                    debug_assert!(false, "{:?}", data.sub_kind);
                    Altitude::ZERO
                }
            },
            EntityKind::Decoy => match data.sub_kind {
                EntitySubKind::Sonar => -unguided_weapon_altitude,
                _ => {
                    debug_assert!(false, "{:?}", data.sub_kind);
                    Altitude::ZERO
                }
            },
            EntityKind::Aircraft => unguided_weapon_altitude * 1.5,
            EntityKind::Collectible => Altitude::ZERO,
            _ => {
                debug_assert!(false, "{:?}", data.kind);
                Altitude::ZERO
            }
        }
        .clamp(min_altitude, max_altitude);

        let altitude_change =
            (target_altitude - self.altitude).clamp_magnitude(Altitude::UNIT * speed * delta);
        self.altitude += altitude_change;
        altitude_change
    }

    /// Returns true if and only if two entities are friendly i.e. same player or same team.
    pub fn is_friendly(&self, other: &Self) -> bool {
        self.is_friendly_to_player(other.player.as_deref())
    }

    pub fn is_friendly_to_player(&self, other_player: Option<&PlayerTuple>) -> bool {
        if self.player.is_none() || other_player.is_none() {
            return false;
        }
        let player = self.player.as_ref().unwrap();
        let other_player = other_player.unwrap();

        if &**player == other_player {
            return true;
        }

        let player = player.borrow();
        let other_player = other_player.borrow();

        if player.team_id.is_none() || other_player.team_id.is_none() {
            return false;
        }

        player.team_id == other_player.team_id
    }

    /// Returns true if and only two entities have some, identical players.
    pub fn has_same_player(&self, other: &Self) -> bool {
        if self.player.is_none() || other.player.is_none() {
            return false;
        }
        self.player.as_ref().unwrap() == other.player.as_ref().unwrap()
    }

    /// can_land_on returns true if and only if self, an aircraft, can land on boat.
    pub fn can_land_on(&self, boat: &Self) -> bool {
        let data = self.data();
        let boat_data = boat.data();
        let boat_extension = boat.extension();
        for (i, armament) in boat_data.armaments.iter().enumerate() {
            if armament.entity_type != self.entity_type {
                // Irrelevant armament.
                continue;
            }

            // Minimum ticks to be considered eligible for receiving an aircraft.
            let minimum = if data.limited {
                // Limit armaments are *only* considered deployed if consumption = Ticks::MAX
                Ticks::MAX
            } else {
                // Unlimited armaments already reload naturally, but allow them to land anyway, even
                // if they are one tick away from reloading.
                Ticks::ONE
            };

            if boat_extension.reloads[i] < minimum {
                // Armament is not in need of landing an aircraft.
                continue;
            }

            let transform =
                boat.transform + boat.data().armament_transform(&boat_extension.turrets, i);
            if self.transform.position.distance_squared(transform.position) < data.radius.powi(2) {
                if data.sub_kind == EntitySubKind::Heli {
                    // Helicopters can land at any angle.
                    return true;
                } else if (self.transform.direction - boat.transform.direction).abs() < Angle::PI_2
                {
                    // Planes must be within angle parameters.
                    return true;
                }
            }
        }
        return false;
    }

    /// Returns whether an entity is in close proximity to a boat. This is the same as whether a mine
    /// should attract, or whether a weapon should be visible regardless of sensor conditions.
    /// This assumes there is sufficient overlap in altitude.
    pub fn is_in_close_proximity_to(&self, boat: &Self) -> bool {
        let boat_data = boat.data();

        debug_assert_eq!(boat_data.kind, EntityKind::Boat);

        const DISTANCE: f32 = 60.0;
        let normal = boat.transform.direction.to_vec();
        let tangent = Vec2::new(-normal.y, normal.x);
        let normal_distance =
            (normal.dot(boat.transform.position) - normal.dot(self.transform.position)).abs();
        let tangent_distance =
            (tangent.dot(boat.transform.position) - tangent.dot(self.transform.position)).abs();
        normal_distance < DISTANCE + boat_data.length * 0.5
            && tangent_distance < DISTANCE + boat_data.width * 0.5
    }

    // hash returns a float in range [0, 1) based on the entity's id.
    pub fn hash(&self) -> f32 {
        let hash_size = 64;
        (self.id.get() & (hash_size - 1)) as f32 * (1.0 / hash_size as f32)
    }
}

impl PartialEq for Entity {
    // Entity equality is strictly referential.
    fn eq(&self, other: &Self) -> bool {
        let referential = ptr::eq(self, other);
        // make sure two separate entities don't share the same ID.
        debug_assert!(referential == (self.id == other.id));
        referential
    }
}

impl Eq for Entity {}

#[cfg(test)]
mod tests {
    use crate::entity::Entity;
    use common::entity::{EntityId, EntityType};
    use std::mem;

    #[test]
    fn size_of() {
        println!("Entity is {} bytes", mem::size_of::<Entity>());
    }

    #[test]
    fn collides_with() {
        unsafe {
            EntityType::init();
        }
        assert!(Entity::new(EntityType::Zubr, None)
            .collides_with(&Entity::new(EntityType::Crate, None), 0.0));
    }

    #[test]
    fn eq() {
        unsafe {
            EntityType::init();
        }
        let mut e1 = Entity::new(EntityType::Zubr, None);
        let e2 = Entity::new(EntityType::Zubr, None);
        e1.id = EntityId::new(5).unwrap(); // make sure IDs are different.
        assert_eq!(e1, e1);
        assert_eq!(e2, e2);
        assert_ne!(e1, e2)
    }
}
