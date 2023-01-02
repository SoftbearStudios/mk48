// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::str_lit_to_expr;
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Lit, Meta,
    MetaList, NestedMeta,
};

pub(crate) fn derive_settings(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    if let Data::Struct(DataStruct { fields, .. }) = data {
        if let Fields::Named(FieldsNamed { named, .. }) = fields {
            let mut loaders = Vec::with_capacity(named.len());
            let mut getters = Vec::with_capacity(named.len());
            let mut setters = Vec::with_capacity(named.len());
            let mut validators = Vec::with_capacity(named.len());

            for Field {
                ident, ty, attrs, ..
            } in named
            {
                let ident = ident.expect("uh oh");
                let mut ident_string = ident.to_string().to_case(Case::Camel);
                let getter_name = format_ident!("get_{}", ident);
                let setter_name = format_ident!("set_{}", ident);
                let validator_name = format_ident!("validate_{}", ident);

                let mut storage = quote! { local };
                let mut optional = false;
                let mut validations = Vec::new();

                for attribute in attrs.into_iter().filter(|a| a.path.is_ident("setting")) {
                    let meta = attribute.parse_meta().expect("couldn't parse as meta");
                    if let Meta::List(MetaList { nested, .. }) = meta {
                        for meta in nested {
                            match meta {
                                NestedMeta::Meta(Meta::NameValue(meta)) => {
                                    if meta.path.is_ident("range") {
                                        let valid_range = str_lit_to_expr(meta.lit);
                                        validations.push(quote! {
                                            let valid = #valid_range;
                                            let value = value.clamp(valid.start, valid.end);
                                        });
                                    } else if meta.path.is_ident("rename") {
                                        ident_string = if let Lit::Str(s) = meta.lit {
                                            s.value()
                                        } else {
                                            panic!("must rename to string");
                                        };
                                    }
                                }
                                NestedMeta::Meta(Meta::Path(path)) => {
                                    if path.is_ident("finite") {
                                        validations.push(quote! {
                                            if !value.is_finite() {
                                                return None;
                                            }
                                        });
                                    } else if path.is_ident("optional") {
                                        optional = true;
                                    } else if path.is_ident("volatile") {
                                        storage = quote! { session };
                                    } else if path.is_ident("no_store") {
                                        storage = quote! { no_op };
                                    } else {
                                        panic!("Unexpected path: {}", path.get_ident().unwrap());
                                    }
                                }
                                _ => panic!("Expected nested name-value pair"),
                            }
                        }
                    } else {
                        panic!("Expected a list");
                    }
                }

                assert!(
                    !optional || validations.is_empty(),
                    "cant be optional and have validations"
                );
                let loader = if optional {
                    quote! {
                        #ident: {
                            debug_assert_eq!(default.#ident, None, "optional defaults must be None");
                            browser_storages.#storage.get(#ident_string).or(default.#ident)
                        },
                    }
                } else {
                    quote! {
                        #ident: browser_storages.#storage.get(#ident_string).and_then(Self::#validator_name).unwrap_or(default.#ident),
                    }
                };
                let getter = quote! {
                    pub fn #getter_name(&self) -> #ty {
                        self.#ident.clone()
                    }
                };

                let setter = if optional {
                    quote! {
                        pub fn #setter_name(&mut self, value: #ty, browser_storages: &mut BrowserStorages) {
                            self.#ident = value.clone();
                            let _ = browser_storages.#storage.set(#ident_string, value);
                        }
                    }
                } else {
                    quote! {
                        pub fn #setter_name(&mut self, value: #ty, browser_storages: &mut BrowserStorages) {
                            if let Some(valid) = Self::#validator_name(value) {
                                self.#ident = valid.clone();
                                let _ = browser_storages.#storage.set(#ident_string, Some(valid));
                            }
                        }
                    }
                };

                if !optional {
                    let validator = quote! {
                        fn #validator_name(value: #ty) -> Option<#ty> {
                            #(#validations)*
                            Some(value)
                        }
                    };

                    validators.push(validator);
                }

                loaders.push(loader);
                getters.push(getter);
                setters.push(setter);
            }

            let output = quote! {
                impl Settings for #ident {
                    fn load(browser_storages: &BrowserStorages, default: Self) -> Self {
                        Self {
                            #(#loaders)*
                        }
                    }
                }

                impl #ident {
                     #(#getters)*
                     #(#setters)*
                     #(#validators)*
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
