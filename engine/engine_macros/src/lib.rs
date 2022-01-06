extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DataStruct, DeriveInput, Expr, Field, Fields, FieldsNamed, Lit, Meta,
    MetaList, NestedMeta,
};

fn str_lit_to_expr(lit: Lit) -> Expr {
    if let Lit::Str(s) = lit {
        let string = s.value();
        let str = string.as_str();
        let ret = syn::parse_str::<Expr>(str).expect(str);
        //println!("{}", matches!(ret, Expr::Range(_)));
        ret
    } else {
        panic!("expected string literal")
    }
}

#[proc_macro_derive(Layer)]
pub fn derive_layer(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    if let Data::Struct(DataStruct { fields, .. }) = data {
        if let Fields::Named(FieldsNamed { named, .. }) = fields {
            let mut pre_renders = Vec::with_capacity(named.len());
            let mut renders = Vec::with_capacity(named.len());

            for Field { ident, .. } in named {
                pre_renders.push(quote! {
                    self.#ident.pre_render(renderer);
                });
                renders.push(quote! {
                    self.#ident.render(renderer);
                });
            }

            let output = quote! {
                impl Layer for #ident {
                    fn pre_render(&mut self, renderer: &Renderer) {
                        #(#pre_renders)*
                    }

                    fn render(&mut self, renderer: &Renderer) {
                        #(#renders)*
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

#[proc_macro_derive(Vertex)]
pub fn derive_vertex(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    if let Data::Struct(DataStruct { fields, .. }) = data {
        if let Fields::Named(FieldsNamed { named, .. }) = fields {
            let mut bind_attribs = Vec::with_capacity(named.len());

            for Field { ty, .. } in named {
                bind_attribs.push(quote! {
                    #ty::bind_attrib(attribs);
                });
            }

            let output = quote! {
                impl Vertex for #ident {
                    fn bind_attribs(attribs: &mut crate::renderer::attribute::Attribs<Self>) {
                        use crate::renderer::attribute::Attribute;
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

#[proc_macro_derive(Settings, attributes(setting))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    if let Data::Struct(DataStruct { fields, .. }) = data {
        if let Fields::Named(FieldsNamed { named, .. }) = fields {
            let mut defaults = Vec::with_capacity(named.len());
            let mut loaders = Vec::with_capacity(named.len());
            let mut getters = Vec::with_capacity(named.len());
            let mut setters = Vec::with_capacity(named.len());
            let mut validators = Vec::with_capacity(named.len());
            let mut arbitrary_getters = Vec::with_capacity(named.len());
            let mut arbitrary_setters = Vec::with_capacity(named.len());

            for Field {
                ident, ty, attrs, ..
            } in named
            {
                let ident = ident.expect("uh oh");
                let ident_string = ident.to_string().to_case(Case::Camel);
                let getter_name = format_ident!("get_{}", ident);
                let setter_name = format_ident!("set_{}", ident);
                let validator_name = format_ident!("validate_{}", ident);
                let mut default = quote! {
                    #ident: Default::default(),
                };
                let mut loader = quote! {
                    #ident: local_storage.get(#ident_string).and_then(Self::#validator_name).unwrap_or_default(),
                };
                let getter = quote! {
                    pub fn #getter_name(&self) -> #ty {
                        self.#ident
                    }
                };
                let setter = quote! {
                    pub fn #setter_name(&mut self, value: #ty, local_storage: &mut LocalStorage) {
                        if let Some(valid) = Self::#validator_name(value) {
                            self.#ident = valid;
                            let _ = local_storage.set(#ident_string, Some(valid));
                        }
                    }
                };

                let mut validations = Vec::new();

                for attribute in attrs.into_iter().filter(|a| a.path.is_ident("setting")) {
                    let meta = attribute.parse_meta().expect("couldn't parse as meta");
                    if let Meta::List(MetaList { nested, .. }) = meta {
                        for meta in nested {
                            match meta {
                                NestedMeta::Meta(Meta::NameValue(meta)) => {
                                    if meta.path.is_ident("default") {
                                        let default_value = str_lit_to_expr(meta.lit);
                                        default = quote! {
                                            #ident: #default_value,
                                        };
                                        loader = quote! {
                                            #ident: local_storage.get(#ident_string).and_then(Self::#validator_name).unwrap_or(#default_value),
                                        };
                                    } else if meta.path.is_ident("range") {
                                        let valid_range = str_lit_to_expr(meta.lit);
                                        validations.push(quote! {
                                            let valid = #valid_range;
                                            let value = value.clamp(valid.start, valid.end);
                                        });
                                    }
                                }
                                NestedMeta::Lit(Lit::Str(s)) => {
                                    let val = s.value();
                                    match val.as_str() {
                                        "finite" => {
                                            validations.push(quote! {
                                                if !value.is_finite() {
                                                    return None;
                                                }
                                            });
                                        }
                                        _ => panic!("Unexpected {}", val),
                                    }
                                }
                                _ => panic!("Expected nested name-value pair"),
                            }
                        }
                    } else {
                        panic!("Expected a list");
                    }
                }

                let validator = quote! {
                    fn #validator_name(value: #ty) -> Option<#ty> {
                        #(#validations)*
                        Some(value)
                    }
                };

                defaults.push(default);
                loaders.push(loader);
                getters.push(getter);
                setters.push(setter);
                validators.push(validator);
                arbitrary_getters.push(quote! {
                    #ident_string => serde_wasm_bindgen::to_value(&self.#getter_name()).unwrap_or(wasm_bindgen::JsValue::NULL),
                });
                arbitrary_setters.push(quote! {
                    #ident_string => {
                        if let Ok(value) = serde_wasm_bindgen::from_value(value) {
                            self.#setter_name(value, local_storage);
                        }
                    }
                })
            }

            let output = quote! {
                impl Default for #ident {
                    fn default() -> Self {
                        Self {
                            #(#defaults)*
                        }
                    }
                }

                impl Settings for #ident {
                    fn load(local_storage: &LocalStorage) -> Self {
                        Self {
                            #(#loaders)*
                        }
                    }

                    fn get(&self, key: &str) -> wasm_bindgen::JsValue {
                        match key {
                            #(#arbitrary_getters)*
                            _ => wasm_bindgen::JsValue::NULL
                        }
                    }

                    fn set(&mut self, key: &str, value: wasm_bindgen::JsValue, local_storage: &mut LocalStorage) {
                        match key {
                            #(#arbitrary_setters),*
                            _ => {
                                #[cfg(debug_assertions)]
                                panic!("unrecognized setting {}", key);
                            }
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
