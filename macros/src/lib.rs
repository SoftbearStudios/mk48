// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(proc_macro_span)]
#![feature(let_else)]

mod entity_type;

use proc_macro::TokenStream;

#[proc_macro_derive(
    EntityTypeData,
    attributes(info, entity, size, offset, props, sensors, armament, turret, exhaust)
)]
pub fn entity_type_data(input: TokenStream) -> TokenStream {
    crate::entity_type::derive_entity_type(input)
}
