// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use proc_macro::TokenStream;
use quote::quote;
use std::array;
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    parse_quote, Attribute, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed,
    GenericParam, Generics, Ident, Type,
};

pub struct LayerInput {
    ident: Ident,
    generics: Generics,
    named: FieldsNamed,
    any_marked: bool,
    any_renders_marked: HashSet<Type>,
    alpha: bool,
    depth: bool,
    renders: Option<Option<Vec<Type>>>,
    stencil: bool,
}

impl Parse for LayerInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput {
            attrs,
            ident,
            data,
            generics,
            ..
        } = input.parse()?;

        let alpha = find_attr(&attrs, "alpha")?.is_some();
        let depth = find_attr(&attrs, "depth")?.is_some();
        let render_attrs = find_attrs(&attrs, "render");
        let stencil = find_attr(&attrs, "stencil")?.is_some();

        if let Some(attr) = find_attr(&attrs, "layer")? {
            return Err(syn::Error::new(
                attr.span(),
                "invalid attribute layer, use #[render]",
            ));
        }

        match data {
            Data::Struct(DataStruct {
                struct_token,
                fields,
                ..
            }) => {
                if let Fields::Named(named) = fields {
                    let renders: Vec<Option<Type>> =
                        render_attrs.map(|a| a.parse_args().ok()).collect();
                    let renders: Option<Option<Vec<Type>>> = (!renders.is_empty())
                        .then(|| {
                            if renders.iter().all(|b| b.is_none()) {
                                Ok(None)
                            } else if renders.iter().any(|b| b.is_none()) {
                                Err(syn::Error::new(
                                    ident.span(),
                                    "can either have #[render], or multiple #[render(Params)]",
                                ))
                            } else {
                                Ok(Some(renders.into_iter().map(|b| b.unwrap()).collect()))
                            }
                        })
                        .transpose()?;

                    let any_marked = named
                        .named
                        .iter()
                        .any(|field| field.attrs.iter().any(is_field_attr));

                    let any_renders_marked: HashSet<_> = renders
                        .iter()
                        .flatten()
                        .flatten()
                        .filter_map(|bound| {
                            named
                                .named
                                .iter()
                                .filter_map(|field| {
                                    let res = has_render_attr(&field.attrs, bound);
                                    if let Ok(false) = res {
                                        None
                                    } else {
                                        Some(res.map(|_| ()))
                                    }
                                })
                                .next()
                                .map(|res| res.map(|_| bound.clone()))
                        })
                        .try_collect()?;

                    Ok(Self {
                        ident,
                        generics,
                        named,
                        any_marked,
                        any_renders_marked,
                        alpha,
                        depth,
                        renders,
                        stencil,
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
        generics,
        named: FieldsNamed { named, .. },
        any_marked,
        any_renders_marked,
        alpha,
        depth,
        renders,
        stencil,
    } = input;

    let [mut alphas, mut depths, mut stencils, mut pre_prepares, mut pre_renders] =
        array::from_fn(|_| Vec::with_capacity(named.len()));

    let mut render_inners: HashMap<Option<Type>, _> = HashMap::new();

    for Field {
        attrs, ident, ty, ..
    } in named
    {
        // If any fields are marked, they aren't all assumed to be layers.
        if any_marked {
            let count = attrs.iter().filter(|a| is_field_attr(a)).count();
            assert!(count <= 1);

            if count == 0 {
                continue;
            }
        }

        alphas.push(quote! {
            <#ty as Layer>::ALPHA
        });
        depths.push(quote! {
            <#ty as Layer>::DEPTH
        });
        stencils.push(quote! {
            <#ty as Layer>::STENCIL
        });
        pre_prepares.push(quote! {
            self.#ident.pre_prepare(renderer);
        });
        pre_renders.push(quote! {
            self.#ident.pre_render(renderer);
        });

        if let Some(renders) = &renders {
            if let Some(renders) = renders {
                for bound in renders {
                    if !any_renders_marked.contains(&bound)
                        || has_render_attr(&attrs, &bound).unwrap()
                    {
                        render_inners
                            .entry(Some(bound.clone()))
                            .or_insert(vec![])
                            .push(quote! {
                                self.#ident.render(renderer, params.borrow());
                            });
                    }
                }
            } else {
                render_inners.entry(None).or_insert(vec![]).push(quote! {
                    self.#ident.render(renderer, params.borrow());
                })
            }
        }
    }

    let [alphas, depths, stencils] =
        [(alphas, alpha), (depths, depth), (stencils, stencil)].map(|(i, required)| {
            if required {
                quote! { true }
            } else {
                let any = i.into_iter().intersperse(quote! { || });
                quote! {
                    #(#any)*
                }
            }
        });

    let c = if std::env::var("CARGO_PKG_NAME").unwrap() == "renderer" {
        quote!(crate)
    } else {
        quote!(renderer)
    };

    let render_impls = renders
        .map(|renders| {
            let generic_render = renders.is_none();
            renders
                .into_iter()
                .flatten()
                .map(|bound| {
                    (
                        quote! { impl #c::RenderLayer<#bound> },
                        quote! { #bound },
                        Some(bound),
                    )
                })
                .chain(
                    generic_render
                        .then(|| (quote! { impl<P> #c::RenderLayer<P> }, quote! { P }, None)),
                )
                .map(|(impl_decl, bound, typ)| {
                    let render_inner = render_inners.remove(&typ).unwrap_or_else(|| {
                        panic!("no renders for {bound}");
                    });
                    quote! {
                        #impl_decl for #ident {
                            fn render(&mut self, renderer: &#c::Renderer, params: #bound) {
                                use std::borrow::Borrow;
                                #(#render_inner)*
                            }
                        }
                    }
                })
        })
        .into_iter()
        .flatten();

    let generics = add_trait_bounds(generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let output = quote! {
        impl #impl_generics Layer for #ident #ty_generics #where_clause {
            const ALPHA: bool = #alphas;
            const DEPTH: bool = #depths;
            const STENCIL: bool = #stencils;
            fn pre_prepare(&mut self, renderer: &#c::Renderer) {
                #(#pre_prepares)*
            }
            fn pre_render(&mut self, renderer: &#c::Renderer) {
                #(#pre_renders)*
            }
        }

        #(#render_impls)*
    };
    output.into()
}

// Add a bound `T: Layer` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Layer));
        }
    }
    generics
}

fn is_field_attr(a: &Attribute) -> bool {
    a.path.segments.len() == 1 && a.path.segments[0].ident == "layer"
}

fn has_render_attr(attrs: &[Attribute], bound: &Type) -> Result<bool, syn::Error> {
    let mut i = attrs.iter().filter_map(|a| {
        if a.path.segments.len() == 1 && a.path.segments[0].ident == "render" {
            let res = a
                .parse_args()
                .map_err(|_| {
                    syn::Error::new(a.span(), "invalid params type, use #[render(Params)]")
                })
                .map(|b: Type| (&b == bound).then_some(a));
            if let Ok(None) = res {
                None
            } else {
                Some(res.map(|a| a.unwrap()))
            }
        } else {
            None
        }
    });
    let ret = i.next().transpose()?;
    if let Some(a) = i.next().transpose()? {
        Err(syn::Error::new(a.span(), "duplicate attribute"))
    } else {
        Ok(ret.is_some())
    }
}

fn find_attr<'a>(attrs: &'a [Attribute], name: &'a str) -> Result<Option<&Attribute>, syn::Error> {
    let mut i = find_attrs(attrs, name);
    let attr = i.next();
    if let Some(attr) = i.next() {
        Err(syn::Error::new(attr.span(), "duplicate attribute"))
    } else {
        Ok(attr)
    }
}

fn find_attrs<'a>(attrs: &'a [Attribute], name: &'a str) -> impl Iterator<Item = &'a Attribute> {
    attrs
        .iter()
        .filter(move |a| a.path.segments.len() == 1 && a.path.segments[0].ident == name)
}
