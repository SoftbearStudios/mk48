// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed,
    GenericParam, Generics,
};

pub(crate) fn derive_vertex(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = parse_macro_input!(input);
    if let Data::Struct(DataStruct { fields, .. }) = data {
        if let Fields::Named(FieldsNamed { named, .. }) = fields {
            let bind_attribs: Vec<_> = named
                .into_iter()
                .map(|Field { ty, .. }| {
                    quote! {
                        #ty::bind_attribs(attribs);
                    }
                })
                .collect();

            let c = if std::env::var("CARGO_PKG_NAME").unwrap() == "renderer" {
                quote!(crate)
            } else {
                quote!(renderer)
            };

            let generics = add_trait_bounds(generics);
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            let output = quote! {
                impl #impl_generics #c::Vertex for #ident #ty_generics #where_clause {
                    fn bind_attribs(attribs: &mut #c::Attribs) {
                        #(#bind_attribs)*
                    }
                }
            };
            output.into()
        } else {
            panic!("Must have named fields.");
        }
    } else {
        panic!("Must be struct");
    }
}

// Add a bound `T: Vertex` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Vertex));
        }
    }
    generics
}
