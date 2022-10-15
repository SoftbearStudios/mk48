// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::name_to_ident;
use crate::ply::parser::Ply;
use itertools::Itertools;
use litrs::StringLit;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::fs;
use std::path::Path;
use std::str::FromStr;

mod parser;
mod serializer;

pub fn include_ply(item: TokenStream) -> TokenStream {
    let input = item.into_iter().collect::<Vec<_>>();
    if input.len() != 1 {
        let msg = format!("expected exactly one input token, got {}", input.len());
        return quote! { compile_error!(#msg) }.into();
    }

    let ply_path_lit = match StringLit::try_from(&input[0]) {
        Err(e) => return e.to_compile_error(),
        Ok(lit) => lit,
    };
    let ply_path_string = ply_path_lit.value();

    let mut ply_path = proc_macro::Span::call_site().source_file().path();
    ply_path.pop();
    ply_path.push(Path::new(ply_path_string));

    proc_macro::tracked_path::path(ply_path.to_string_lossy());
    let load_err = format!("unable to load ply {}", ply_path.display());
    let parse_err = format!("unable to parse ply {}", ply_path.display());
    let ply_src = fs::read_to_string(ply_path).expect(&load_err);
    let ply = Ply::from_str(&ply_src).expect(&parse_err);
    ply.to_tokens().into()
}

pub fn include_plys(item: TokenStream, define: bool) -> TokenStream {
    let input = TokenStream2::from(item)
        .into_iter()
        .filter(|item| {
            if let proc_macro2::TokenTree::Punct(_) = item {
                false
            } else {
                true
            }
        })
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

    if !define {
        assert!(
            extra_traits.is_empty(),
            "only include_plys_define can specify extra traits"
        );
    }

    let ply_path_lit = match StringLit::try_from(&input[0]) {
        Err(e) => return e.to_compile_error(),
        Ok(lit) => lit,
    };
    let ply_path_string = ply_path_lit.value();

    let mut ply_path = proc_macro::Span::call_site().source_file().path();
    ply_path.pop();
    ply_path.push(Path::new(ply_path_string));

    proc_macro::tracked_path::path(ply_path.to_string_lossy());

    let mut variants = Vec::new();
    let mut includes = Vec::<proc_macro2::TokenStream>::new();

    for entry in fs::read_dir(ply_path)
        .expect("couldn't read model dir")
        .map(|r| r.expect("couldn't read model dir entry"))
        .sorted_by_key(|d| d.path())
    {
        let tmp = entry.file_name();
        let file_name = tmp.to_string_lossy();
        if file_name.ends_with(".ply") {
            let raw_name = file_name.split(".").next().unwrap();
            assert!(!raw_name.contains('.'));
            let name = name_to_ident(raw_name.to_string());
            if !define {
                let tmp = entry.path();
                let ply_path = tmp.to_string_lossy();
                let load_err = format!("unable to load ply {}", ply_path);
                let parse_err = format!("unable to parse ply {}", ply_path);
                let ply_src = fs::read_to_string(ply_path.to_string()).expect(&load_err);
                let ply = Ply::from_str(&ply_src).expect(&parse_err);
                let ply = ply.to_tokens();
                includes.push(
                    quote! {
                        Self::#name => #ply
                    }
                    .into(),
                );
            } else {
                variants.push(name);
            }
        }
    }

    if define {
        quote! {
            #[derive(Ord, PartialOrd, Hash, Copy, Clone, PartialEq, Eq, #(#extra_traits),*)]
            #[repr(u8)]
            pub enum ModelId {
                #(#variants),*
            }
        }
    } else {
        quote! {
            pub trait IntoModel {
                fn model(self) -> renderer3d::Model;
            }

            impl IntoModel for ModelId {
                fn model(self) -> renderer3d::Model {
                    match self {
                        #(#includes),*
                    }
                }
            }
        }
    }
    .into()
}
