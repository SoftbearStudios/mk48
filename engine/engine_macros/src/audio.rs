// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::name_to_ident;
use litrs::StringLit;
use proc_macro::TokenStream;
use quote::quote;
use sprite_sheet::AudioSpriteSheet;
use std::fs;
use std::path::Path;

pub fn include_audio(item: TokenStream) -> TokenStream {
    let input = item.into_iter().collect::<Vec<_>>();
    if input.len() != 2 {
        let msg = format!("expected exactly two input tokens, got {}", input.len());
        return quote! { compile_error!(#msg) }.into();
    }

    let audio_string_lit = match StringLit::try_from(&input[0]) {
        // Error if the token is not a string literal
        Err(e) => return e.to_compile_error(),
        Ok(lit) => lit,
    };
    let audio_string = audio_string_lit.value();

    let json_string_lit = match StringLit::try_from(&input[1]) {
        // Error if the token is not a string literal
        Err(e) => return e.to_compile_error(),
        Ok(lit) => lit,
    };
    let json_string = json_string_lit.value();

    let mut json_path = proc_macro::Span::call_site().source_file().path();
    json_path.pop();
    json_path.push(Path::new(json_string));

    let json = fs::read_to_string(json_path).expect("unable to load audio json");

    let sprite_sheet: AudioSpriteSheet =
        serde_json::from_str(&json).expect("unable to parse audio json");

    let mut sorted: Vec<_> = sprite_sheet.sprites.into_iter().collect();

    // Duplicates names will be weeded out when code is generated.
    sorted.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    let mut sprites: Vec<proc_macro2::TokenStream> = Vec::new();
    let variants: Vec<proc_macro2::TokenStream> = sorted
        .into_iter()
        .map(|(name, sprite)| {
            let variant = name_to_ident(name);
            let start = sprite.start;
            let loop_start: proc_macro2::TokenStream = if let Some(loop_start) = sprite.loop_start {
                quote! {
                    Some(#loop_start)
                }
            } else {
                quote! {
                    None
                }
            };
            let duration = sprite.duration;

            sprites.push(quote! {
                sprite_sheet::AudioSprite {
                    start: #start,
                    loop_start: #loop_start,
                    duration: #duration
                }
            });

            quote! {
                #variant
            }
        })
        .collect();

    quote! {
        #[derive(Copy, Clone, Debug, PartialEq)]
        pub enum Audio {
            #(#variants,)*
        }

        impl client_util::audio::Audio for Audio {
            fn index(self) -> usize {
                self as usize
            }

            fn path() -> &'static str {
                #audio_string
            }

            fn sprites() -> &'static [sprite_sheet::AudioSprite] {
                static SPRITES: &'static [sprite_sheet::AudioSprite] = &[
                    #(#sprites,)*
                ];
                SPRITES
            }
        }
    }
    .into()
}
