// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::collision::{radius_collision, sat_collision};
use crate::entities::*;
use crate::entity_extension::EntityExtension;
use crate::player::*;
use crate::server::Server;
use atomic_refcell::{AtomicRef, AtomicRefMut};
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::*;
use common::guidance::Guidance;
use common::terrain::*;
use common::ticks::{Ticks, TicksRepr};
use common::transform::{DimensionTransform, Transform};
use common::util::hash_u32_to_f32;
use game_server::player::{PlayerData, PlayerTuple};
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
    pub player: Option<Arc<PlayerTuple<Server>>>,
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
    pub fn new(entity_type: EntityType, player: Option<Arc<PlayerTuple<Server>>>) -> Self {
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
        unsafe { &*self.player.as_ref().unwrap().extension.0.get() }
    }

    /// extension_mut gets the extension of the entity.
    /// It is safe because it requires a mutable reference
    /// to the entity which is the sole owner of the extension.
    pub fn extension_mut(&mut self) -> &mut EntityExtension {
        assert!(self.is_boat());
        unsafe { &mut *self.player.as_ref().unwrap().extension.0.get() }
    }

    /// change_entity_type is the only valid way to change an entity's type.
    pub fn change_entity_type(
        &mut self,
        entity_type: EntityType,
        arena: &mut Arena,
        boat_below_full_potential: bool,
    ) {
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

        // Regen a fraction of damage. Get original damage fraction before changing
        // entity type. Never result in boat being dead.
        let damage_fraction = self.ticks.to_secs() / old_data.max_health().to_secs();
        let new_damage_fraction = if boat_below_full_potential {
            // See https://github.com/SoftbearStudios/mk48/issues/168
            damage_fraction
        } else {
            damage_fraction * 0.75
        };
        let max_health = new_data.max_health();
        self.ticks = (max_health * new_damage_fraction).min(max_health - Ticks::ONE);

        // Changing entity type cannot leave an entity with more damage than health. Other code
        // may crash if this invalid state is propagated. Also, it would be nice if boat isn't
        // immediately dead.
        debug_assert!(self.ticks < max_health);

        let extension = self.extension_mut();

        // Keep time until armaments reload. Use u32 to avoid overflow.
        // Start by counting the total ticks left to reload (for non-limited armaments).
        let mut total_reload = 0;
        for (i, reload) in extension.reloads.iter().enumerate() {
            if !old_data.armaments[i].entity_type.data().limited {
                total_reload += reload.0 as u32;
            }
        }

        // Penalty for upgrading with fired weapons to nerf upgrading during fights (see #168).
        total_reload = total_reload * 3 / 2;

        // Change the extension to correspond with the new type.
        extension.change_entity_type(entity_type);

        // Finish by (un)reloading.
        for (i, reload) in extension.reloads_mut().iter_mut().enumerate() {
            let armament = &new_data.armaments[i];
            if !armament.entity_type.data().limited {
                let to_consume = (armament.reload().0 as u32).min(total_reload);
                *reload = Ticks::from_repr(to_consume as TicksRepr);
                total_reload -= to_consume;
                if total_reload == 0 {
                    break;
                }
            }
        }

        // Pre-aim turrets at aim target.
        self.update_turret_aim(10.0);
    }

    /// Adds a reference from player to self.
    /// Only call on boats.
    pub fn create_index(&mut self, i: EntityIndex) {
        debug_assert!(self.is_boat());

        let mut player = self.borrow_player_mut();

        // Set status to alive.
        assert!(!player.data.status.is_alive());
        player.data.status = Status::new_alive(i);

        // Clear flags when player's boat is spawned.
        player.data.flags = Flags::default();
        drop(player);

        // Change entity type (allocate turrets/reloads).
        let entity_type = self.entity_type;
        self.extension_mut().change_entity_type(entity_type);
    }

    /// Adjusts player's pointer to self, if applicable.
    /// Only call on boats.
    pub fn set_index(&mut self, i: EntityIndex) {
        debug_assert!(self.is_boat());

        self.borrow_player_mut().data.status.set_entity_index(i);
    }

    /// Set's player to dead, removing reference to self, if applicable.
    /// Only call on boats.
    pub fn delete_index(&mut self, reason: DeathReason) {
        debug_assert!(self.is_boat());
        let position = self.transform.position;
        let visual_range = self.data().sensors.visual.range;

        let mut player = self.borrow_player_mut();
        player.data.status = if player.data.flags.left_game {
            Status::Spawning
        } else {
            Status::Dead {
                reason,
                position,
                time: Instant::now(),
                visual_range,
            }
        }
    }

    /// Borrows player immutably.
    /// Must be manually serialized to avoid contention.
    /// Panics if entity doesn't have player.
    pub fn borrow_player(&self) -> AtomicRef<PlayerData<Server>> {
        Self::borrow_player_inner(&self.player)
    }

    fn borrow_player_inner(
        player: &Option<Arc<PlayerTuple<Server>>>,
    ) -> AtomicRef<PlayerData<Server>> {
        player
            .as_ref()
            .expect("only call on entities that are guaranteed to have players")
            .borrow_player()
    }

    /// Borrows player mutably.
    /// Must be manually serialized to avoid contention.
    /// Panics if entity doesn't have player.
    pub fn borrow_player_mut(&mut self) -> AtomicRefMut<PlayerData<Server>> {
        Self::borrow_player_mut_inner(&mut self.player)
    }

    pub fn borrow_player_mut_inner(
        player: &mut Option<Arc<PlayerTuple<Server>>>,
    ) -> AtomicRefMut<PlayerData<Server>> {
        player
            .as_ref()
            .expect("only call on entities that are guaranteed to have players")
            .borrow_player_mut()
    }

    /// Gets the entity's data, corresponding to it's current entity type.
    pub fn data(&self) -> &'static EntityData {
        self.entity_type.data()
    }

    /// Returns true if and only if the entity is of kind boat.
    pub fn is_boat(&self) -> bool {
        self.entity_type.data().kind == EntityKind::Boat
    }

    /// Returns if this entity is owned by a real player (not a bot, not ownerless).
    /// For printing debug info without being too verbose (including bots).
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub fn is_real_player(&self) -> bool {
        self.player
            .as_ref()
            .map(|p| !p.borrow_player().player_id.is_bot())
            .unwrap_or(false)
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
        } else if data.kind == EntityKind::Boat
            && other_data.sub_kind == EntitySubKind::DepthCharge
            && self.altitude.is_submerged()
        {
            other.is_in_proximity_to(self, EntityData::DEPTH_CHARGE_PROXIMITY)
        } else if data.sub_kind == EntitySubKind::DepthCharge
            && other_data.kind == EntityKind::Boat
            && other.altitude.is_submerged()
        {
            self.is_in_proximity_to(other, EntityData::DEPTH_CHARGE_PROXIMITY)
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

    /// Closest point on self's keel (a line segment from bow to stern) to position.
    /// Tolerance is what fraction of the length of the keep to consider.
    pub fn closest_point_on_keel_to(&self, position: Vec2, tolerance: f32) -> Vec2 {
        debug_assert!((0.0..=1.0).contains(&tolerance));
        self.transform
            .closest_point_on_keel_to(self.data().length * tolerance, position)
    }

    /// Determines whether an entity collides with the terrain (underwater terrain ignored to avoid
    /// damaging submarines when going from deep to shallow; submarines must be forced to rise elsewhere).
    ///
    /// Threshold is minimum terrain altitude to be considered colliding. `Altitude::ZERO` is a good
    /// default.
    ///
    /// Returns a `TerrainCollision` if one occurred.
    pub fn collides_with_terrain(
        &self,
        t: &Terrain,
        delta_seconds: f32,
    ) -> Option<TerrainCollision> {
        let arctic = self.transform.position.y >= common::world::ARCTIC;

        let threshold = if arctic && self.altitude.is_submerged() {
            // Below ice, so only collide with solid land.
            Altitude(2)
        } else {
            Altitude::ZERO
        }
        .max(self.altitude);

        // If submerged, colliding with terrain should be relatively temporary (boats should simply
        // rise up rather than taking damage) so it is ignored.
        t.collides_with(self.dimension_transform(), threshold, delta_seconds)
    }

    /// Updates the aim of all turrets, assuming delta_seconds have elapsed.
    pub fn update_turret_aim(&mut self, delta_seconds: f32) {
        let aim_target = if let Status::Alive { aim_target, .. } = &self.borrow_player().data.status
        {
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
        let a = &self.data().armaments[index];

        // Limited armaments start their timer when they die.
        let reload = if a.entity_type.data().limited {
            Ticks::MAX
        } else {
            a.reload()
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
        let reloads = self.extension_mut().reloads_mut();
        if reloads.is_empty() {
            return;
        }

        // Split reloads into ranges of similar armaments to reload in parallel.
        let mut current = &armaments[0];
        let mut start = 0;

        for (end, next) in armaments.iter().enumerate() {
            if next.is_similar_to(current) {
                continue;
            }
            Self::reload_range(&mut reloads[start..end], amount);
            current = next;
            start = end;
        }

        // Final iteration
        Self::reload_range(&mut reloads[start..], amount);
    }

    fn reload_range(reloads: &mut [Ticks], mut amount: Ticks) {
        while amount > Ticks::ZERO {
            // Find the armament with the lowest consumption (to prioritize full reloads).
            // Skip reloaded (Ticks::ZERO) and limited (Ticks::MAX) armaments.
            let reload = reloads
                .iter_mut()
                .filter(|&&mut r| !matches!(r, Ticks::ZERO | Ticks::MAX))
                .min_by_key(|&&mut r| r);

            if let Some(reload) = reload {
                let consumed = (*reload).min(amount);
                *reload -= consumed;
                amount -= consumed;
            } else {
                // No armament has yet to be fully replenished, so this range is done.
                break;
            }
        }
    }

    /// Damage damages an entity and returns if it died.
    pub fn damage(&mut self, amount: Ticks) -> bool {
        let data = self.data();

        // Ticks is lifespan, not damage, for non-boats.
        assert_eq!(data.kind, EntityKind::Boat);

        self.ticks = self.ticks.saturating_add(amount).min(data.max_health());
        self.ticks == data.max_health()
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
        if (self.altitude.is_airborne() && other.altitude.is_submerged())
            || (self.altitude.is_submerged() && other.altitude.is_airborne())
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
                | EntitySubKind::RocketTorpedo
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
            .map(|alt| {
                if alt < Altitude(2) && self.transform.position.y > common::world::ARCTIC {
                    // Under ice sheet
                    Altitude::MIN
                } else {
                    alt
                }
            })
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
                EntitySubKind::Shell
                | EntitySubKind::Rocket
                | EntitySubKind::RocketTorpedo
                | EntitySubKind::Missile => unguided_weapon_altitude,
                EntitySubKind::Sam => target.unwrap_or(unguided_weapon_altitude),
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

    pub fn is_friendly_to_player(&self, other_player: Option<&PlayerTuple<Server>>) -> bool {
        let (player, other_player) = match (self.player.as_ref(), other_player) {
            (Some(player), Some(other_player)) => (player, other_player),
            _ => return false,
        };

        // This is very hot code, and we can't afford atomic shenanigans.
        if ptr::eq(Arc::as_ptr(player), other_player as *const _) {
            return true;
        }

        let player = player.borrow_player();

        if player.team_id().is_none() {
            return false;
        }

        let other_player = other_player.borrow_player();

        player.team_id() == other_player.team_id()
    }

    /// Returns true if and only two entities have some, identical players.
    pub fn has_same_player(&self, other: &Self) -> bool {
        if self.player.is_none() || other.player.is_none() {
            return false;
        }
        self.player.as_ref().unwrap() == other.player.as_ref().unwrap()
    }

    /// Returns Some(pad_index) iff self, an aircraft, can land on boat.
    pub fn landing_pad(&self, boat: &Self) -> Option<usize> {
        let data = self.data();
        let boat_data = boat.data();
        let boat_extension = boat.extension();
        for (i, armament) in boat_data.armaments.iter().enumerate() {
            if armament.entity_type != self.entity_type || !data.limited {
                // Irrelevant armament.
                continue;
            }

            if boat_extension.reloads[i] < Ticks::MAX {
                // Armament is not in need of landing an aircraft.
                continue;
            }

            let transform =
                boat.transform + boat.data().armament_transform(&boat_extension.turrets, i);
            if self.transform.position.distance_squared(transform.position) < data.radius.powi(2) {
                // Helicopters can land at any angle, but planes must be withing angle parameters.
                if data.sub_kind == EntitySubKind::Heli
                    || (self.transform.direction - transform.direction).abs() < Angle::PI_2
                {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Constant used for checking whether, for example, a weapon becomes visible regardless of
    /// sensor ranges.
    pub const CLOSE_PROXIMITY: f32 = 60.0;

    /// Calculates proximity to a boat (which is defined as the minimum normal or tangential
    /// difference to the boats front or side).
    pub fn proximity_to(&self, boat: &Self) -> f32 {
        let boat_data = boat.data();
        debug_assert_eq!(boat_data.kind, EntityKind::Boat);

        let normal = boat.transform.direction.to_vec();
        let tangent = Vec2::new(-normal.y, normal.x);
        let normal_distance =
            (normal.dot(boat.transform.position) - normal.dot(self.transform.position)).abs();
        let tangent_distance =
            (tangent.dot(boat.transform.position) - tangent.dot(self.transform.position)).abs();
        (normal_distance - boat_data.length * 0.5)
            .max(tangent_distance - boat_data.width * 0.5)
            .max(0.0)
    }

    /// Returns whether an entity is in close proximity to a boat. This is the same as whether a mine
    /// should attract, or whether a weapon should be visible regardless of sensor conditions.
    /// This assumes there is sufficient overlap in altitude.
    pub fn is_in_proximity_to(&self, boat: &Self, distance: f32) -> bool {
        debug_assert!(distance >= 0.0);
        self.proximity_to(boat) <= distance
    }

    // hash returns a float in range [0, 1) based on the entity's id.
    pub fn hash(&self) -> f32 {
        hash_u32_to_f32(self.id.get())
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
    use glam::Vec2;
    use std::mem;

    #[test]
    fn size_of() {
        println!("Entity is {} bytes", mem::size_of::<Entity>());
    }

    #[test]
    fn collides_with() {
        assert!(Entity::new(EntityType::Zubr, None)
            .collides_with(&Entity::new(EntityType::Crate, None), 0.0));
    }

    #[test]
    fn closest_point_on_keep_to() {
        assert_eq!(
            Entity::new(EntityType::Zubr, None)
                .closest_point_on_keel_to(Vec2::new(-100.0, 0.0), 0.5),
            Vec2::new(-EntityType::Zubr.data().length * 0.25, 0.0)
        );
    }

    #[test]
    fn eq() {
        let mut e1 = Entity::new(EntityType::Zubr, None);
        let e2 = Entity::new(EntityType::Zubr, None);
        e1.id = EntityId::new(5).unwrap(); // make sure IDs are different.
        assert_eq!(e1, e1);
        assert_eq!(e2, e2);
        assert_ne!(e1, e2)
    }
}
