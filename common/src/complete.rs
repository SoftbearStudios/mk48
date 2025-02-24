// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::contact::*;
use crate::death_reason::DeathReason;
use crate::protocol::*;
use crate::terrain::Terrain;
use std::mem;

pub trait CompleteTrait<'a> {
    type Contact: ContactTrait;
    type Iterator: Iterator<Item = Self::Contact>;

    /// contacts can only be called once.
    /// The first item is guaranteed to be the player's boat if it exists.
    fn contacts(&mut self) -> Self::Iterator;

    /// collect_contacts can only be called once instead of contacts.
    /// it may be faster than self.contacts.collect().
    fn collect_contacts(&mut self) -> Vec<Self::Contact>;

    fn death_reason(&self) -> Option<&DeathReason>;

    fn score(&self) -> u32;

    fn world_radius(&self) -> f32;

    fn terrain(&self) -> &Terrain;
}

pub struct Complete<'a> {
    update: Update,
    terrain: &'a Terrain,
}

impl<'a> Complete<'a> {
    pub fn from_update(update: Update, terrain: &'a mut Terrain) -> Self {
        terrain.apply_update(&update.terrain);
        Self { update, terrain }
    }
}

impl<'a> CompleteTrait<'a> for Complete<'a> {
    type Contact = Contact;
    type Iterator = std::vec::IntoIter<Contact>;

    fn contacts(&mut self) -> Self::Iterator {
        mem::take(&mut self.update.contacts).into_iter()
    }

    fn collect_contacts(&mut self) -> Vec<Self::Contact> {
        mem::take(&mut self.update.contacts)
    }

    fn death_reason(&self) -> Option<&DeathReason> {
        self.update.death_reason.as_ref()
    }

    #[inline]
    fn score(&self) -> u32 {
        self.update.score
    }

    #[inline]
    fn world_radius(&self) -> f32 {
        self.update.world_radius
    }

    #[inline]
    fn terrain(&self) -> &Terrain {
        self.terrain
    }
}
