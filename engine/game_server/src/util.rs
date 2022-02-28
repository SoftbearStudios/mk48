// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

/// Yields an option's contents if [`Some`], otherwise returns from the function.
#[macro_export]
macro_rules! unwrap_or_return {
    ($opt: expr) => {{
        match $opt {
            Some(some) => some,
            None => return,
        }
    }};
}

/// Returns items added and ids removed, if there was a difference.
///
/// Time complexity isn't great. Use [`diff_large_n`] instead, if time complexity matters.
pub fn diff_small_n<T: Clone + PartialEq, ID: PartialEq>(
    old: &Arc<[T]>,
    new: &Vec<T>,
    get_id: impl Fn(&T) -> ID,
) -> Option<(Vec<T>, Vec<ID>)> {
    let mut added = Vec::<T>::new();
    let mut removed = Vec::<ID>::new();

    for old_item in old.iter() {
        if let Some(new_item) = new.iter().find(|i| get_id(i) == get_id(old_item)) {
            // Add changed items.
            if new_item != old_item {
                added.push(new_item.clone());
            }
        } else {
            // Remove missing items.
            removed.push(get_id(old_item));
        }
    }

    // Add new items.
    for new_item in new.iter() {
        if !old.iter().any(|i| get_id(i) == get_id(new_item)) {
            added.push(new_item.clone());
        }
    }

    if added.is_empty() && removed.is_empty() {
        None
    } else {
        Some((added, removed))
    }
}

/// Better time complexity than [`diff_small_n`], at the cost of more allocations.
pub fn diff_large_n<T: Clone + PartialEq, ID: Eq + Hash>(
    old: &Arc<[T]>,
    new: &Vec<T>,
    get_id: impl Fn(&T) -> ID,
) -> Option<(Vec<T>, Vec<ID>)> {
    let mut added = Vec::<T>::new();
    let mut removed = Vec::<ID>::new();

    // Faster access via a hash map, at the cost of extra allocations.
    let old_ids: HashSet<ID> = old.iter().map(|v| get_id(v)).collect();
    let new_map: HashMap<ID, T> = new.iter().map(|v| (get_id(v), v.clone())).collect();

    // We assume that get_id is fast relative to hashmap iteration overhead.
    for old_item in old.iter() {
        if let Some(new_item) = new_map.get(&get_id(old_item)) {
            // Add changed items.
            if new_item != old_item {
                added.push(new_item.clone());
            }
        } else {
            // Remove missing items.
            removed.push(get_id(old_item));
        }
    }

    // Add new items.
    for new_item in new.iter() {
        if !old_ids.contains(&get_id(new_item)) {
            added.push(new_item.clone());
        }
    }

    if added.is_empty() && removed.is_empty() {
        None
    } else {
        Some((added, removed))
    }
}

#[cfg(test)]
mod test {
    use crate::util::{diff_large_n, diff_small_n};
    use rand::{thread_rng, Rng};
    use std::collections::HashSet;
    use std::sync::Arc;

    #[test]
    fn diff_test() {
        let old: Arc<[i8]> = vec![1, 2, 4, 3, 7].into();
        let new: Vec<i8> = vec![2, 4, 5, 6, -7];
        let diff1 = diff_small_n(&old, &new, |i| i.abs());
        assert_eq!(diff1, Some((vec![-7, 5, 6], vec![1, 3])));

        let diff2 = diff_large_n(&old, &new, |i| i.abs());
        assert_eq!(diff1, diff2);
    }

    #[test]
    fn fuzz() {
        for _ in 0..500 {
            let mut old: Vec<usize> = Vec::new();
            for _ in 0..thread_rng().gen_range(2..50) {
                let n = thread_rng().gen_range(0..100);
                if !old.contains(&n) {
                    old.push(n);
                }
            }
            let mut new = old.clone();
            let diff_count = thread_rng().gen_range(0..10);
            //println!("old: {:?}", old);
            for j in 0..diff_count {
                if thread_rng().gen_bool(0.5) {
                    //println!("+ {}", (j + 1) * 100);
                    new.push((j + 1) * 100);
                } else if new.len() > 1 {
                    //println!("-");
                    new.remove(thread_rng().gen_range(0..new.len() - 1));
                }
            }
            //println!("new: {:?}", new);

            assert_eq!(old.len(), old.iter().collect::<HashSet<_>>().len());
            assert_eq!(new.len(), new.iter().collect::<HashSet<_>>().len());

            let old_arc: Arc<[_]> = old.into();
            let diff1 = diff_small_n(&old_arc, &new, |i| i * 10);
            let diff2 = diff_large_n(&old_arc, &new, |i| i * 10);
            if diff_count == 0 {
                assert!(diff1.is_none());
            } else {
                let diffs = diff1.as_ref().unwrap();
                assert!(diffs.0.len() + diffs.1.len() <= diff_count);
            }
            assert_eq!(diff1, diff2);
        }
    }
}
