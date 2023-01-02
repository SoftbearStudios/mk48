// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::name_to_ident;
use itertools::Itertools;
use litrs::StringLit;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::fs;
use std::path::Path;

const EXTENSION: &str = ".png";

fn get_name(file_name: &str) -> Option<&str> {
    let trimmed = file_name.trim_end_matches(EXTENSION);
    (trimmed != file_name).then_some(trimmed)
}

fn is_normal_map(name: &str) -> bool {
    name.ends_with("_nrm")
}

pub fn include_textures(item: TokenStream) -> TokenStream {
    let input = TokenStream2::from(item)
        .into_iter()
        .filter(|item| !matches!(item, proc_macro2::TokenTree::Punct(_)))
        .collect::<Vec<_>>();

    if input.is_empty() {
        let msg = "expected at least one input";
        return quote! { compile_error!(#msg) }.into();
    }

    let extra_traits: Vec<proc_macro2::Ident> = input[1..]
        .iter()
        .map(|name| {
            // TODO better error handling.
            if let proc_macro2::TokenTree::Ident(ident) = name {
                ident.clone()
            } else {
                panic!("expected idents after literal string {:?}", name)
            }
        })
        .collect();

    let texture_path_lit = match StringLit::try_from(&input[0]) {
        Err(e) => return e.to_compile_error(),
        Ok(lit) => lit,
    };
    let texture_path_string = texture_path_lit.value();

    let mut texture_path = proc_macro::Span::call_site().source_file().path();
    texture_path.pop();
    texture_path.push(Path::new(texture_path_string));

    let texture_path_string = texture_path.to_string_lossy().into_owned();
    proc_macro::tracked_path::path(texture_path_string.as_str());

    let mut variants = Vec::new();
    let mut names = Vec::<proc_macro2::TokenStream>::new();

    for entry in fs::read_dir(texture_path)
        .expect("couldn't read texture dir")
        .map(|r| r.expect("couldn't read texture dir entry"))
        .sorted_by_key(|d| d.path())
    {
        let tmp = entry.file_name();
        let file_name = tmp.to_string_lossy();
        if let Some(name) = get_name(&file_name) {
            if is_normal_map(name) {
                continue;
            }

            assert!(
                !name.contains('.'),
                "invalid file name {file_name} -> {name}"
            );

            let ident = name_to_ident(name.to_string());
            let tmp = entry.path();
            let tmp = tmp.to_string_lossy();
            let texture_name = tmp
                .strip_prefix(texture_path_string.as_str())
                .unwrap()
                .strip_suffix(EXTENSION)
                .unwrap();

            // TODO why is this required.
            let texture_name = texture_name.strip_prefix('/').unwrap_or(texture_name);

            names.push(quote! {
                Self::#ident => #texture_name
            });
            variants.push(ident);
        }
    }

    quote! {
        #[derive(Ord, PartialOrd, Hash, Copy, Clone, PartialEq, Eq, #(#extra_traits),*)]
        #[repr(u8)]
        pub enum TextureId {
            #(#variants),*
        }

        impl TextureId {
            pub fn name(self) -> &'static str {
                match self {
                    #(#names),*
                }
            }

            // TODO placeholder
        }
    }
    .into()
}
