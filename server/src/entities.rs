// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::*;
use common::death_reason::DeathReason;
use glam::Vec2;
use maybe_parallel_iterator::{
    IntoMaybeParallelIterator, IntoMaybeParallelRefIterator, IntoMaybeParallelRefMutIterator,
};
use std::convert::{TryFrom, TryInto};
use std::ops::{Index, IndexMut, RangeInclusive};

const SIZE: usize = 32 * common::world::SIZE;
const SCALE: f32 = 800.0;

/// An efficient collection of entities.
pub struct Entities {
    sectors: [Sector; SIZE * SIZE],
}

/// A single square sector, storing the entities within it.
pub struct Sector {
    entities: Vec<Entity>,
}

impl Sector {
    /// new allocates an empty sector.
    const fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// shrink may reduce the allocation size of a sector if its entity count dropped sufficiently.
    fn shrink(&mut self) {
        if self.entities.capacity() > self.entities.len() * 3 {
            let new_size = (self.entities.len() * 3 / 2).next_power_of_two().max(4);
            if new_size < self.entities.capacity() {
                self.entities.shrink_to(new_size);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct SectorId(u8, u8);

impl SectorId {
    /// Forms an id from the corresponding sector index scalar.
    fn from_sector_index(index: usize) -> Self {
        Self((index / SIZE) as u8, (index % SIZE) as u8)
    }

    /// Returns the corresponding sector index scalar.
    fn as_sector_index(&self) -> usize {
        let result = self.0 as usize * SIZE + self.1 as usize;
        debug_assert_eq!(*self, Self::from_sector_index(result));
        result
    }

    /// Gets center of sector with id.
    fn center(&self) -> Vec2 {
        let mut pos = Vec2::new(self.0 as f32, self.1 as f32);
        pos *= SCALE;
        pos += SIZE as f32 * SCALE / -2.0 + SCALE / 2.0;
        debug_assert_eq!(*self, Self::try_from(pos).unwrap());
        pos
    }

    /// Returns true if any part of self is within radius of position.
    fn in_radius(&self, position: Vec2, radius: f32) -> bool {
        const HALF: f32 = SCALE / 2.0;
        let abs_diff = (self.center() - position).abs();
        if abs_diff.x > HALF + radius || abs_diff.y > HALF + radius {
            false
        } else if abs_diff.x <= HALF || abs_diff.y <= HALF {
            true
        } else {
            (abs_diff - HALF).max(Vec2::ZERO).length_squared() < radius.powi(2)
        }
    }

    /// Iterates all `SectorId`s in a rectangle defined by corners start and end.
    fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        // Range inclusive is slow so add 1.
        (start.0..end.0 + 1).flat_map(move |x| (start.1..end.1 + 1).map(move |y| Self(x, y)))
    }

    /// Iterates all `SectorId`s in a circle.
    fn iter_radius(center: Vec2, radius: f32) -> impl Iterator<Item = Self> {
        let start = Self::saturating_from(center - radius);
        let end = Self::saturating_from(center + radius);
        Self::iter(start, end).filter(move |id| id.in_radius(center, radius))
    }

    /// Returns the `SectorId` containing pos, with pos being clamped to the dimensions of the data
    /// structure.
    fn saturating_from(mut pos: Vec2) -> Self {
        pos *= 1.0 / (SCALE);
        pos += SIZE as f32 / 2.0;
        let x = (pos.x as i32).clamp(0, (SIZE - 1) as i32) as u8;
        let y = (pos.y as i32).clamp(0, (SIZE - 1) as i32) as u8;
        Self(x, y)
    }
}

impl TryFrom<Vec2> for SectorId {
    type Error = &'static str;

    fn try_from(mut pos: Vec2) -> Result<Self, Self::Error> {
        pos *= 1.0 / (SCALE);
        pos += SIZE as f32 / 2.0;
        let (x, y) = (pos.x as i32, pos.y as i32);
        const RANGE: RangeInclusive<i32> = 0..=((SIZE - 1) as i32);
        if RANGE.contains(&x) && RANGE.contains(&y) {
            Ok(Self(x as u8, y as u8))
        } else {
            Err("out of world")
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct EntityIndex(SectorId, u16);

impl EntityIndex {
    pub fn changed(&self, e: &Entity) -> bool {
        self.0 != e.transform.position.try_into().unwrap()
    }
}

impl Entities {
    pub fn new() -> Self {
        const INIT: Sector = Sector::new();
        Self {
            sectors: [INIT; SIZE * SIZE],
        }
    }

    /// Returns the maximum possible world radius to avoid going out of bounds of this collection.
    pub fn max_world_radius() -> f32 {
        // TODO: Shouldn't need "- 1", but crashes otherwise.
        ((SIZE - 1) / 2) as f32 * SCALE
    }

    fn get_sector(&self, sector_id: SectorId) -> &Sector {
        &self.sectors[sector_id.as_sector_index()]
    }

    fn mut_sector(&mut self, sector_id: SectorId) -> &mut Sector {
        &mut self.sectors[sector_id.as_sector_index()]
    }

    pub fn add_internal(&mut self, mut entity: Entity) {
        assert_ne!(entity.id, unset_entity_id());
        let sector_id = entity.transform.position.try_into().unwrap();
        let sector = self.mut_sector(sector_id);
        if entity.is_boat() {
            entity.create_index(EntityIndex(sector_id, sector.entities.len() as u16));
        }
        sector.entities.push(entity);
    }

    /// When an entity moves, it may reside in a different sector. This function commits that
    /// change to the `Entities` state.
    pub fn move_sector(&mut self, index: EntityIndex) {
        let sector_id = index.0;
        let i = index.1 as usize;
        let sector = self.mut_sector(sector_id);

        let last = sector.entities.len() - 1;
        if i != last && sector.entities[last].is_boat() {
            sector.entities[last].set_index(index)
        }

        let mut entity = sector.entities.swap_remove(i as usize);
        sector.shrink();

        let new_sector_id = entity.transform.position.try_into().unwrap();
        let new_sector = self.mut_sector(new_sector_id);

        if entity.is_boat() {
            entity.set_index(EntityIndex(new_sector_id, new_sector.entities.len() as u16))
        }
        new_sector.entities.push(entity);
    }

    /// Don't use directly. Wrapped by world's remove.
    pub fn remove_internal(&mut self, index: EntityIndex, death_reason: DeathReason) -> Entity {
        let sector_id = index.0;
        let i = index.1 as usize;
        let sector = self.mut_sector(sector_id);

        let last = sector.entities.len() - 1;
        if i != last && sector.entities[last].is_boat() {
            sector.entities[last].set_index(index)
        }

        let mut entity = sector.entities.swap_remove(i as usize);
        sector.shrink();

        if entity.is_boat() {
            entity.delete_index(death_reason);
        }
        entity
    }

    /// Iterates all entities in parallel.
    pub fn par_iter(&self) -> impl IntoMaybeParallelIterator<Item = (EntityIndex, &Entity)> {
        self.sectors
            .maybe_par_iter()
            .enumerate()
            .flat_map(|(sector_index, sector)| {
                let sector_id = SectorId::from_sector_index(sector_index);

                sector
                    .entities
                    .maybe_par_iter()
                    .with_min_sequential(256)
                    .enumerate()
                    .map(move |(index, entity)| {
                        let entity_index = EntityIndex(sector_id, index as u16);
                        (entity_index, entity)
                    })
            })
    }

    /// Mutably iterates all entities in parallel.
    pub fn par_iter_mut(
        &mut self,
    ) -> impl IntoMaybeParallelIterator<Item = (EntityIndex, &mut Entity)> {
        self.sectors
            .maybe_par_iter_mut()
            .enumerate()
            .flat_map(|(sector_index, sector)| {
                let sector_id = SectorId::from_sector_index(sector_index);

                sector
                    .entities
                    .maybe_par_iter_mut()
                    .with_min_sequential(256)
                    .enumerate()
                    .map(move |(index, entity)| {
                        let entity_index = EntityIndex(sector_id, index as u16);
                        (entity_index, entity)
                    })
            })
    }

    /// Iterates all entities in a given radius around center.
    pub fn iter_radius(
        &self,
        center: Vec2,
        radius: f32,
    ) -> impl Iterator<Item = (EntityIndex, &Entity)> {
        let r2 = radius * radius;
        SectorId::iter_radius(center, radius).flat_map(move |sector_id| {
            self.get_sector(sector_id)
                .entities
                .iter()
                .enumerate()
                .filter(move |(_, e)| e.transform.position.distance_squared(center) <= r2)
                .map(move |(index, entity)| (EntityIndex(sector_id, index as u16), entity))
        })
    }
}

impl Index<EntityIndex> for Entities {
    type Output = Entity;

    fn index(&self, i: EntityIndex) -> &Self::Output {
        &self.get_sector(i.0).entities[i.1 as usize]
    }
}

impl IndexMut<EntityIndex> for Entities {
    fn index_mut(&mut self, i: EntityIndex) -> &mut Self::Output {
        &mut self.mut_sector(i.0).entities[i.1 as usize]
    }
}
