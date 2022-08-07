#![feature(proc_macro_span)]
extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;

use convert_case::{Case, Casing};
use litrs::StringLit;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use sprite_sheet::AudioSpriteSheet;
use std::fs;
use std::path::Path;
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

#[proc_macro]
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

    sorted.sort_by_key(|(name, _)| name.clone());

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
                .into()
            } else {
                quote! {
                    None
                }
                .into()
            };
            let duration = sprite.duration;

            sprites.push(
                quote! {
                    Self::#variant => sprite_sheet::AudioSprite{
                        start: #start,
                        loop_start: #loop_start,
                        duration: #duration
                    }
                }
                .into(),
            );

            quote! {
                #variant
            }
            .into()
        })
        .collect();

    quote! {
        #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
        pub enum Audio {
            #(#variants,)*
        }

        impl client_util::audio::Audio for Audio {
            fn path() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(#audio_string)
            }

            fn sprite(self) -> sprite_sheet::AudioSprite {
                match self {
                    #(#sprites,)*
                }
            }
        }
    }
    .into()
}

fn name_to_ident(name: String) -> proc_macro2::Ident {
    let upper_camel = name.to_case(convert_case::Case::UpperCamel);
    proc_macro2::Ident::new(&upper_camel, Span::call_site())
}

#[proc_macro_derive(Layer)]
pub fn derive_layer(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    if let Data::Struct(DataStruct { fields, .. }) = data {
        if let Fields::Named(FieldsNamed { named, .. }) = fields {
            let mut pre_prepares = Vec::with_capacity(named.len());
            let mut pre_renders = Vec::with_capacity(named.len());
            let mut renders = Vec::with_capacity(named.len());

            for Field { ident, .. } in named {
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

            let output = quote! {
                impl Layer for #ident {
                    fn pre_prepare(&mut self, renderer: &Renderer) {
                        #(#pre_prepares)*
                    }

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
            let bind_attribs: Vec<_> = named
                .into_iter()
                .map(|Field { ty, .. }| {
                    quote! {
                        #ty::bind_attribs(attribs);
                    }
                })
                .collect();

            let c = if std::env::var("CARGO_PKG_NAME").unwrap() == "client_util" {
                quote!(crate)
            } else {
                quote!(client_util)
            };

            let output = quote! {
                impl Vertex for #ident {
                    fn bind_attribs(attribs: &mut #c::renderer::attribs::Attribs) {
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

                let mut storage = quote! { local };
                let mut arbitrary = true;
                let mut unquote = false;
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
                                    }
                                }
                                NestedMeta::Meta(Meta::Path(path)) => {
                                    if path.is_ident("finite") {
                                        validations.push(quote! {
                                            if !value.is_finite() {
                                                return None;
                                            }
                                        });
                                    } else if path.is_ident("no_serde_wasm_bindgen") {
                                        arbitrary = false;
                                    } else if path.is_ident("volatile") {
                                        storage = quote! { session };
                                    } else if path.is_ident("unquote") {
                                        unquote = true;
                                    } else {
                                        panic!("Unexpected path");
                                    }
                                }
                                _ => panic!("Expected nested name-value pair"),
                            }
                        }
                    } else {
                        panic!("Expected a list");
                    }
                }

                let loader = quote! {
                    #ident: browser_storages.#storage.get(#ident_string, #unquote).and_then(Self::#validator_name).unwrap_or(default.#ident),
                };
                let getter = quote! {
                    pub fn #getter_name(&self) -> #ty {
                        self.#ident
                    }
                };
                let setter = quote! {
                    pub fn #setter_name(&mut self, value: #ty, browser_storages: &mut BrowserStorages) {
                        if let Some(valid) = Self::#validator_name(value) {
                            self.#ident = valid;
                            let _ = browser_storages.#storage.set(#ident_string, Some(valid), #unquote);
                        }
                    }
                };
                let validator = quote! {
                    fn #validator_name(value: #ty) -> Option<#ty> {
                        #(#validations)*
                        Some(value)
                    }
                };

                loaders.push(loader);
                getters.push(getter);
                setters.push(setter);
                validators.push(validator);
                if arbitrary {
                    arbitrary_getters.push(quote! {
                        #ident_string => serde_wasm_bindgen::to_value(&self.#getter_name()).unwrap_or(wasm_bindgen::JsValue::NULL),
                    });
                    arbitrary_setters.push(quote! {
                        #ident_string => {
                            if let Ok(value) = serde_wasm_bindgen::from_value(value) {
                                self.#setter_name(value, browser_storages);
                            }
                        }
                    });
                }
            }

            let output = quote! {
                impl Settings for #ident {
                    fn load(browser_storages: &BrowserStorages, default: Self) -> Self {
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

                    fn set(&mut self, key: &str, value: wasm_bindgen::JsValue, browser_storages: &mut BrowserStorages) {
                        match key {
                            #(#arbitrary_setters),*
                            _ => ()
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
