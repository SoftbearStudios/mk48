// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::Entity;
use common::entity::EntityId;
use common::entity::*;
use idalloc::Slab;
use ringbuffer::{ConstGenericRingBuffer, RingBufferExt, RingBufferRead, RingBufferWrite};

/// Arena manages entity ids and counts of each entity type. it takes care of delaying the reuse of
/// EntityIds that are remembered by clients.
pub struct Arena {
    /// Available EntityIDs.
    slab: Slab<u32>,
    /// Must delay recycling EntityId's for at least MAX_KEEP_ALIVE, so clients don't interpolate
    /// between two different entities. Every tick, the front of the buffer is popped and recycled,
    /// and a new slot is pushed to the back.
    delay_recycle: ConstGenericRingBuffer<
        Vec<EntityId>,
        { EntityKind::MAX_KEEP_ALIVE.0.next_power_of_two() as usize },
    >,
    /// Count of each EntityType.
    counts: Vec<u32>,
}

impl Arena {
    pub fn new() -> Self {
        let mut delay_recycle = ConstGenericRingBuffer::new();

        // These vectors should be rarely resized, if every.
        delay_recycle.fill_with(|| Vec::with_capacity(16));

        Self {
            slab: Slab::new(),
            delay_recycle,
            counts: vec![0; EntityType::iter().count()],
        }
    }

    /// count returns the number of entities with a certain type.
    pub fn count(&self, entity_type: EntityType) -> usize {
        self.counts[entity_type as u8 as usize] as usize
    }

    /// count_predicate returns the number of entities that satisfy a predicate.
    pub fn count_predicate<P>(&self, predicate: P) -> usize
    where
        P: Fn(EntityType) -> bool,
    {
        EntityType::iter()
            .filter(|t| predicate(*t))
            .map(|t| self.count(t))
            .sum()
    }

    /// count_kind returns the number of entities with a certain kind.
    #[allow(dead_code)]
    pub fn count_kind(&self, kind: EntityKind) -> usize {
        self.count_predicate(|t| t.data().kind == kind)
    }

    /// count_sub_kind returns the number of entities with a certain sub kind.
    #[allow(dead_code)]
    pub fn count_sub_kind(&self, sub_kind: EntitySubKind) -> usize {
        self.count_predicate(|t| t.data().sub_kind == sub_kind)
    }

    /// total returns the total number of entities.
    #[allow(dead_code)]
    pub fn total(&self) -> usize {
        self.counts.iter().sum::<u32>() as usize
    }

    fn increment_count(&mut self, entity_type: EntityType) {
        let count = &mut self.counts[entity_type as u8 as usize];
        *count = count.checked_add(1).unwrap();
    }

    fn decrement_count(&mut self, entity_type: EntityType) {
        let count = &mut self.counts[entity_type as u8 as usize];
        *count = count.checked_sub(1).unwrap();
    }

    /// Generate a new ID for an entity of a certain type.
    pub fn new_id(&mut self, entity_type: EntityType) -> EntityId {
        self.increment_count(entity_type);
        EntityId::new(self.slab.next() + 1).unwrap() // +1 so not zero
    }

    /// Call when an entity changes type.
    pub fn change_type(&mut self, from: EntityType, to: EntityType) {
        self.decrement_count(from);
        self.increment_count(to);
    }

    /// Call when an entity goes away.
    pub fn drop_entity(&mut self, entity: Entity) {
        self.decrement_count(entity.entity_type);

        // Enqueue for recycling later.
        self.delay_recycle.back_mut().unwrap().push(entity.id);
    }

    /// Call once per send to client.
    pub fn recycle(&mut self) {
        let mut front = self.delay_recycle.dequeue().unwrap();
        for recycled in front.drain(..) {
            // Recycle the EntityID.
            self.slab.free(recycled.get() - 1); // -1 so not zero
        }
        // Recycle the vector.
        self.delay_recycle.push(front);
    }
}
