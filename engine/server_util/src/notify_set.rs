// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::hash::Hash;

#[derive(Debug)]
pub struct NotifySet<T> {
    /// Items being added.
    pub add: HashSet<T>,
    /// Items being removed.
    pub remove: HashSet<T>,
}

impl<T: Eq + Hash> NotifySet<T> {
    pub fn new() -> Self {
        Self {
            add: HashSet::new(),
            remove: HashSet::new(),
        }
    }

    /// Mark an item as added (or changed), and thus requiring notification.
    pub fn added(&mut self, key: T) {
        self.remove.remove(&key);
        self.add.insert(key);
    }

    /// Mark an item as removed.
    pub fn removed(&mut self, key: T) {
        self.add.remove(&key);
        self.remove.insert(key);
    }
}

impl<T: Eq + Hash> Default for NotifySet<T> {
    fn default() -> Self {
        Self::new()
    }
}
