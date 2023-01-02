// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use proc_macro::TokenStream;
use quote::quote;

pub fn include_emoji(item: TokenStream) -> TokenStream {
    assert!(
        item.into_iter().next().is_none(),
        "emoji!() expected no arguments"
    );

    let (find, replace): (Vec<_>, Vec<_>) = emojis::iter()
        .flat_map(|emoji| {
            emoji
                .shortcodes()
                .map(move |alias| (format!(":{alias}:"), emoji.as_str().to_owned()))
        })
        .unzip();

    quote! {
        const EMOJI_FIND: &'static [&'static str] = &[#(#find),*];
        const EMOJI_REPLACE: &'static [&'static str] = &[#(#replace),*];
    }
    .into()
}
