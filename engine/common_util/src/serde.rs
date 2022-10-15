// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

pub fn is_default<T: Default + PartialEq>(x: &T) -> bool {
    x == &T::default()
}
