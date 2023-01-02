// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::EntityData;
use common_util::range::map_ranges;
use std::sync::Arc;

/// level_to_score converts a boat level to a score required to upgrade to it.
pub const fn level_to_score(level: u8) -> u32 {
    // For reference, https://www.desmos.com/calculator/8cwxdws7fp
    // Must match JS.
    ((level as u32).pow(2) + 2u32.pow(level.saturating_sub(3) as u32) - 2) * 10
}

pub fn score_to_level(score: u32) -> u8 {
    // Min level is 1 so don't iterate it because returns level - 1.
    for level in 2..=EntityData::MAX_BOAT_LEVEL {
        if score < level_to_score(level) {
            return level - 1;
        }
    }
    EntityData::MAX_BOAT_LEVEL
}

/// Diminishes score by n levels.
#[allow(dead_code)]
pub(crate) fn lose_n_levels(score: u32, n: u8) -> u32 {
    let level = score_to_level(score);

    // Lose 2 levels when you die (minimum level is 1 so sub 1 before and add 1 after).
    if let Some(respawn_level) = (level - 1).checked_sub(n).map(|l| l + 1) {
        // Could be the same if score was enough for max level.
        let current_floor = level_to_score(level);
        let current_ceil = level_to_score((level + 1).min(EntityData::MAX_BOAT_LEVEL));

        // Can't be the same.
        let respawn_floor = level_to_score(respawn_level);
        let respawn_ceil = level_to_score(respawn_level + 1);

        if current_floor == current_ceil {
            respawn_floor
        } else {
            map_ranges(
                score as f32,
                (current_floor as f32)..(current_ceil as f32),
                (respawn_floor as f32)..(respawn_ceil as f32),
                true,
            ) as u32
        }
    } else {
        // If level 1 or 2, respawn as level 1 with 0 points.
        0
    }
}

/// respawn_score returns how much score is kept when a boat owned by a real player dies.
pub fn respawn_score(score: u32) -> u32 {
    /*
    let levels_to_lose = match score_to_level(score) {
        1 => 0,
        2..=3 => 1,
        _ => 2,
    };
    lose_n_levels(score, levels_to_lose)
     */

    score.min(level_to_score(EntityData::MAX_BOAT_LEVEL)) * 10 / 25
}

/// respawn_score returns how much score a boat gets from a kill.
pub fn kill_score(score: u32, killer_score: u32) -> u32 {
    let raw = 10 + score / 4;
    let killer_score = killer_score.min(level_to_score(EntityData::MAX_BOAT_LEVEL));
    if killer_score / 16 >= score {
        0
    } else if killer_score / 4 >= score {
        raw / 2
    } else {
        raw
    }
}

/// respawn_score returns how much score a boat gets from a ramming kill.
pub fn ram_score(score: u32, killer_score: u32) -> u32 {
    kill_score(score, killer_score) / 2
}

/// natural_death_coins returns how many coins a boat should drop, assuming it died of natural causes.
pub fn natural_death_coins(score: u32) -> u32 {
    (score / 4 / 10).min(200)
}

/// returns a float in range [0, 1) based on n.
pub fn hash_u32_to_f32(n: u32) -> f32 {
    let hash_size = 64;
    (n & (hash_size - 1)) as f32 * (1.0 / hash_size as f32)
}

/// make_mut_slice derives a mutable slice from an Arc, cloning the Arc if necessary.
pub fn make_mut_slice<T: Clone>(arc: &mut Arc<[T]>) -> &mut [T] {
    let mut_ref = unsafe { &mut *(arc as *mut Arc<[T]>) };

    match Arc::get_mut(mut_ref) {
        Some(x) => x,
        None => {
            *arc = arc.iter().cloned().collect();
            Arc::get_mut(arc).unwrap()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::entity::{EntityData, EntityKind, EntityType};
    use crate::util::{
        kill_score, level_to_score, lose_n_levels, ram_score, respawn_score, score_to_level,
    };
    use rand::seq::IteratorRandom;
    use rand::{thread_rng, Rng};

    #[test]
    fn score_to_and_from_level() {
        for score in 0..=level_to_score(EntityData::MAX_BOAT_LEVEL) * 3 {
            assert_eq!(
                score_to_level(score),
                score_to_level(level_to_score(score_to_level(score))),
                "{}",
                score
            );
        }
    }

    #[test]
    fn test_lose_n_levels() {
        for i in 1..EntityData::MAX_BOAT_LEVEL {
            assert_eq!(level_to_score(i), lose_n_levels(level_to_score(i + 1), 1));
        }
    }

    #[test]
    fn test_respawn_score() {
        assert_eq!(respawn_score(0), 0);
        assert_eq!(respawn_score(5), 2);
        assert_eq!(
            respawn_score(100000),
            level_to_score(EntityData::MAX_BOAT_LEVEL) / 5 * 2
        );

        /*
        assert_eq!(respawn_score(5), 5);
        assert_eq!(respawn_score(level_to_score(1)), level_to_score(1));
        assert_eq!(respawn_score(level_to_score(2)), level_to_score(1));
        assert_eq!(respawn_score(level_to_score(3)), level_to_score(2));
        for i in 4..=EntityData::MAX_BOAT_LEVEL {
            assert_eq!(respawn_score(level_to_score(i)), level_to_score(i - 2));
        }
         */
    }

    #[test]
    fn non_conservation_of_score() {
        let mut total_before = 0u32;
        let mut total_after = 0u32;

        let mut rng = thread_rng();
        for i in 1..=100 {
            let initial_boats: Vec<(EntityType, u32)> = (0..=i.min(20))
                .map(|_| {
                    let level = rng.gen_range(1..=EntityData::MAX_BOAT_LEVEL);
                    let entity_type: EntityType = EntityType::iter()
                        .filter(|t| t.data().kind == EntityKind::Boat && t.data().level == level)
                        .choose(&mut rng)
                        .unwrap();
                    let score = level_to_score(level) * rng.gen_range(1..5);
                    (entity_type, score)
                })
                .collect();

            let mut boats = initial_boats.clone();

            for _time in 0..boats.len() {
                // Which boat loses the points.
                let died = rng.gen_range(0..boats.len());
                // Which boat scoops the points.
                let beneficiary = rng.gen_range(0..boats.len());

                let natural = died == beneficiary || rng.gen_bool(0.5);
                let mut winnings = boats[died]
                    .0
                    .loot(boats[died].1, natural)
                    .map(|t| match t {
                        EntityType::Coin => 10,
                        _ => 2,
                    })
                    .sum::<u32>();

                if !natural {
                    winnings += if rng.gen_bool(0.1) {
                        ram_score(boats[died].1, boats[beneficiary].1)
                    } else {
                        kill_score(boats[died].1, boats[beneficiary].1)
                    }
                }

                boats[died].1 = respawn_score(boats[died].1);
                boats[beneficiary].1 += winnings;
            }

            fn total_score(boats: &Vec<(EntityType, u32)>) -> u32 {
                boats.iter().map(|b| b.1).sum()
            }

            let before = total_score(&initial_boats);
            let after = total_score(&boats);

            total_before += before;
            total_after += after;

            /*
            assert!(
                after < before + boats.len() as u32 * 20,
                "in one iteration, score went from {} to {}",
                before,
                after
            )
             */
        }

        let percent = total_after * 100 / total_before;

        assert!(percent < 90);

        println!("An average of {}% score remains", percent);
    }
}
