// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(array_chunks)]
#![feature(iter_intersperse)]
#![feature(iterator_try_collect)]
#![feature(let_else)]
#![feature(proc_macro_span)]
#![feature(track_path)]

pub(crate) mod audio;
mod emoji;
pub(crate) mod layer;
mod ply;
pub(crate) mod settings;
pub(crate) mod smol_routable;
pub(crate) mod texture;
pub(crate) mod vertex;

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;

use convert_case::Casing;
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{parse_macro_input, Expr, Lit};

#[proc_macro]
pub fn include_audio(item: TokenStream) -> TokenStream {
    crate::audio::include_audio(item)
}

#[proc_macro]
pub fn include_emoji(item: TokenStream) -> TokenStream {
    crate::emoji::include_emoji(item)
}

#[proc_macro]
pub fn include_ply(item: TokenStream) -> TokenStream {
    crate::ply::include_ply(item)
}

#[proc_macro]
pub fn include_plys_into_model(item: TokenStream) -> TokenStream {
    crate::ply::include_plys(item, false, true)
}

#[proc_macro]
pub fn include_plys_into_triangles(item: TokenStream) -> TokenStream {
    crate::ply::include_plys(item, false, false)
}

#[proc_macro]
pub fn include_plys_define(item: TokenStream) -> TokenStream {
    crate::ply::include_plys(item, true, false)
}

#[proc_macro]
pub fn include_textures(item: TokenStream) -> TokenStream {
    crate::texture::include_textures(item)
}

#[proc_macro_derive(Layer, attributes(alpha, depth, layer, render, stencil))]
pub fn derive_layer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as crate::layer::LayerInput);
    crate::layer::derive_layer(input)
}

#[proc_macro_derive(Settings, attributes(setting))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    crate::settings::derive_settings(input)
}

#[proc_macro_derive(Vertex)]
pub fn derive_vertex(input: TokenStream) -> TokenStream {
    crate::vertex::derive_vertex(input)
}

#[proc_macro_derive(SmolRoutable, attributes(at, not_found))]
pub fn smol_routable_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as crate::smol_routable::SmolRoutable);
    crate::smol_routable::derive_smol_routable(input)
}

fn str_lit_to_expr(lit: Lit) -> Expr {
    if let Lit::Str(s) = lit {
        let string = s.value();
        let str = string.as_str();
        syn::parse_str::<Expr>(str).expect(str)
    } else {
        panic!("expected string literal")
    }
}

fn name_to_ident(name: String) -> proc_macro2::Ident {
    let upper_camel = name.to_case(convert_case::Case::UpperCamel);
    proc_macro2::Ident::new(&upper_camel, Span::call_site())
}
