// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident, Meta, MetaList,
    NestedMeta,
};

pub struct LayerInput {
    ident: Ident,
    named: FieldsNamed,
    bound: Option<Ident>,
}

impl Parse for LayerInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput {
            attrs, ident, data, ..
        } = input.parse()?;
        let attr = attrs
            .iter()
            .filter(|a| a.path.segments.len() == 1 && a.path.segments[0].ident == "layer")
            .nth(0);

        match data {
            Data::Struct(DataStruct {
                struct_token,
                fields,
                ..
            }) => {
                if let Fields::Named(named) = fields {
                    let bound = if let Some(attr) = attr {
                        let meta = attr.parse_meta().expect("expected meta");
                        let mut bound = None;

                        if let Meta::List(MetaList { nested, .. }) = &meta {
                            assert_eq!(nested.len(), 1);
                            for nested in nested {
                                match nested {
                                    NestedMeta::Meta(Meta::Path(path)) => {
                                        bound = Some(path.get_ident().unwrap().clone());
                                    }
                                    _ => panic!("unexpected nested meta"),
                                }
                            }
                        } else {
                            panic!("expected meta list");
                        }
                        Some(bound.unwrap())
                    } else {
                        None
                    };

                    Ok(Self {
                        ident,
                        named,
                        bound,
                    })
                } else {
                    Err(syn::Error::new(struct_token.span, "expected named fields"))
                }
            }
            Data::Enum(s) => Err(syn::Error::new(
                s.enum_token.span,
                "expected struct, found enum",
            )),
            Data::Union(u) => Err(syn::Error::new(
                u.union_token.span,
                "expected struct, found union",
            )),
        }
    }
}

pub(crate) fn derive_layer(input: LayerInput) -> TokenStream {
    let LayerInput {
        ident,
        named: FieldsNamed { named, .. },
        bound,
    } = input;

    let mut pre_prepares = Vec::with_capacity(named.len());
    let mut pre_renders = Vec::with_capacity(named.len());
    let mut renders = Vec::with_capacity(named.len());

    fn is_field_attr(a: &Attribute) -> bool {
        a.path.segments.len() == 1 && a.path.segments[0].ident == "layer"
    }

    let marked_fields = named
        .iter()
        .any(|field| field.attrs.iter().any(is_field_attr));

    for Field { attrs, ident, .. } in named {
        if marked_fields {
            let count = attrs.iter().filter(|a| is_field_attr(a)).count();
            assert!(count <= 1);

            // If any fields are marked, skip marked fields.
            if count == 0 {
                continue;
            }
        }

        pre_prepares.push(quote! {
            self.#ident.pre_prepare(renderer);
        });
        pre_renders.push(quote! {
            self.#ident.pre_render(renderer);
        });
        renders.push(quote! {
            self.#ident.render(renderer);
        });
    }

    let (impl_decl, bound) = bound.map_or_else(
        || (quote! { impl<C> Layer<C> }, quote! { C }),
        |bound| (quote! { impl Layer<#bound> }, quote! { #bound }),
    );

    let c = if std::env::var("CARGO_PKG_NAME").unwrap() == "renderer" {
        quote!(crate)
    } else {
        quote!(renderer)
    };

    let output = quote! {
        #impl_decl for #ident {
            fn pre_prepare(&mut self, renderer: &#c::Renderer<#bound>) {
                #(#pre_prepares)*
            }

            fn pre_render(&mut self, renderer: &#c::Renderer<#bound>) {
                #(#pre_renders)*
            }

            fn render(&mut self, renderer: &#c::Renderer<#bound>) {
                #(#renders)*
            }
        }
    };
    output.into()
}
