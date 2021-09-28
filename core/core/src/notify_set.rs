// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;
use std::hash::Hash;

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

    /// Mark an item as added.
    pub fn added(&mut self, key: T) {
        if self.remove.contains(&key) {
            self.remove.remove(&key);
        } else {
            self.add.insert(key);
        }
    }

    /// Mark an item as removed.
    pub fn removed(&mut self, key: T) {
        if self.add.contains(&key) {
            self.add.remove(&key);
        } else {
            self.remove.insert(key);
        }
    }
}
