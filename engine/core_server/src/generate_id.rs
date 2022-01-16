// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::get_unix_time_now;
use core_protocol::id::PlayerId;
use rand::Rng;
use std::num::{NonZeroU32, NonZeroU64};

/// Gets value that increments by 1 every 24 hours.
fn get_unix_day() -> u64 {
    let unix_millis = get_unix_time_now();
    (unix_millis / (24 * 60 * 60 * 1000)) as u64
}

/// Generates a random 32 bit id that is greater than 4 million.  To check if
/// unique, only need to check against items created in the last 24 hours (this
/// assumes items must not be able to live more than 2.8 years).
pub fn generate_id() -> NonZeroU32 {
    generate_id_with_day(get_unix_day() as u32)
}

/// Generates a random 64 bit id.
/// See `generate_id` for more info.
pub fn generate_id_64() -> NonZeroU64 {
    generate_id_64_with_day(get_unix_day())
}

#[doc(hidden)]
pub(crate) fn generate_id_with_day(day: u32) -> NonZeroU32 {
    // Generally, the most significant 10 bits are different each day
    // (although they are re-used every 2.8 years).
    let mut most_sig_10_bits = day.wrapping_shl(PlayerId::RANDOM_BITS);
    if most_sig_10_bits == 0 {
        // Guarantee that ID will be at least 2^RANDOM_BITS aka 4 million.
        most_sig_10_bits = 1 << PlayerId::RANDOM_BITS;
    }

    // The least significant 22 bits are random.
    let mut r: u32 = rand::thread_rng().gen();
    if r == 0 {
        // Preserve non-zero guarantee.
        r = 1;
    }
    let least_sig_22_bits = r & PlayerId::RANDOM_MASK;

    debug_assert!(least_sig_22_bits != 0);
    debug_assert!(most_sig_10_bits & least_sig_22_bits == 0);

    NonZeroU32::new(most_sig_10_bits | least_sig_22_bits).unwrap()
}

#[doc(hidden)]
pub(crate) fn generate_id_64_with_day(day: u64) -> NonZeroU64 {
    let most_sig_10_bits = day.wrapping_shl(64 - PlayerId::DAY_BITS);

    let mut r: u64 = rand::thread_rng().gen();
    if r == 0 {
        // Preserve non-zero guarantee.
        r = 1;
    }

    let least_sig_54_bits = r & ((1 << (64 - PlayerId::DAY_BITS)) - 1);

    debug_assert!(most_sig_10_bits & least_sig_54_bits == 0);

    // Top 10 bits are from day, bottom are random.
    NonZeroU64::new(most_sig_10_bits | least_sig_54_bits).unwrap()
}

#[cfg(test)]
mod test {
    use crate::generate_id::{generate_id_64_with_day, generate_id_with_day, get_unix_day};
    use core_protocol::id::{PlayerId, SessionId};
    use std::num::NonZeroU32;

    #[test]
    fn get_day() {
        println!("day: {}", get_unix_day());
    }

    #[test]
    fn test_player_id() {
        for i in 0..2u32.pow(PlayerId::DAY_BITS + 3) {
            let player_id = PlayerId(generate_id_with_day(i));
            assert!(!player_id.is_bot());
        }
    }

    #[test]
    fn test_bot_id() {
        println!("DAY_BITS={}", PlayerId::DAY_BITS);
        println!("RANDOM_BITS={}", PlayerId::RANDOM_BITS);
        println!("DAY_MASK={:b}", PlayerId::DAY_MASK);
        println!("RANDOM_MASK=       {:b}", PlayerId::RANDOM_MASK);

        let mut i = 2;
        loop {
            let bot_id = PlayerId(NonZeroU32::new(i).unwrap());
            assert!(bot_id.is_bot(), "{:b}", i);

            i *= 2;

            if i > (1 << (PlayerId::RANDOM_BITS - 1)) {
                break;
            }
        }

        let too_low = 1;
        let bot_id = PlayerId(NonZeroU32::new(too_low).unwrap());
        assert!(!bot_id.is_bot());

        let too_high = 2 << PlayerId::RANDOM_BITS;
        let bot_id = PlayerId(NonZeroU32::new(too_high).unwrap());
        assert!(!bot_id.is_bot());
    }

    #[test]
    fn test_64_bit_id() {
        for i in 0..2u64.pow(PlayerId::DAY_BITS + 3) {
            let _ = SessionId(generate_id_64_with_day(i));
        }
    }
}
