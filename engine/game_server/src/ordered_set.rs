// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;

/// A set that maintains order of insertion. Operations should be regarded as O(n).
/// Intended to be used with small, [`Copy`] datatypes, so references are not used.
#[derive(Clone, PartialEq)]
pub struct OrderedSet<T> {
    contents: Vec<T>,
}

impl<T: Eq + Copy> OrderedSet<T> {
    /// Creates an empty set.
    pub const fn new() -> Self {
        Self {
            contents: Vec::new(),
        }
    }

    /// Creates a set of size one.
    pub fn new_with_one(item: T) -> Self {
        Self {
            contents: vec![item],
        }
    }

    /// Returns true iff item is in the set.
    pub fn contains(&self, item: T) -> bool {
        self.contents.iter().any(|&i| i == item)
    }

    /// Returns true if inserted, false if already contained.
    pub fn insert_back(&mut self, item: T) -> bool {
        if self.contains(item) {
            false
        } else {
            self.contents.push(item);
            true
        }
    }

    /// Returns true if was removed, false if didn't contain.
    pub fn remove(&mut self, item: T) -> bool {
        if let Some(index) = self.position(item) {
            self.contents.remove(index);
            true
        } else {
            false
        }
    }

    /// Returns the first element inserted (or swapped), if any.
    pub fn peek_front(&self) -> Option<T> {
        self.contents.get(0).copied()
    }

    /// Returns true if the item existed and was swapped to front, or was already front,
    /// otherwise returns false.
    pub fn swap_to_front(&mut self, item: T) -> bool {
        if let Some(idx) = self.position(item) {
            if idx > 0 {
                self.contents.swap(0, idx);
            }
            true
        } else {
            false
        }
    }

    /// Returns position of item if it exists, otherwise None.
    pub fn position(&self, item: T) -> Option<usize> {
        self.contents.iter().position(|&i| i == item)
    }

    /// Iterates the set.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        self.contents.iter().copied()
    }

    /// Gets the number of elements contained.
    pub fn len(&self) -> usize {
        self.contents.len()
    }

    /// Returns true iff length is zero.
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    /// Empties the set.
    pub fn clear(&mut self) {
        self.contents.clear();
    }

    /// Returns the backing vec.
    pub fn into_inner(self) -> Vec<T> {
        self.contents
    }
}

impl<T> Default for OrderedSet<T> {
    fn default() -> Self {
        Self {
            contents: Vec::default(),
        }
    }
}

impl<T: Debug> Debug for OrderedSet<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.contents)
    }
}

impl<T> Deref for OrderedSet<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.contents
    }
}

impl<T: Serialize + Eq + Copy> Serialize for OrderedSet<T> {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        self.contents.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de> + Eq + Copy> Deserialize<'de> for OrderedSet<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<T>::deserialize(deserializer).map(|contents| Self { contents })
    }
}
