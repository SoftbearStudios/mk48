// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::{Data, DataEnum, DeriveInput, Ident, Lit, Meta, MetaList, NestedMeta, Variant};

pub struct SmolRoutable {
    enum_name: Ident,
    variants: Vec<(Ident, String, HashMap<String, Ident>)>,
    not_found_route: Option<Ident>,
}

impl Parse for SmolRoutable {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput { ident, data, .. } = input.parse()?;
        let enum_name = ident;

        match data {
            Data::Enum(DataEnum { variants, .. }) => {
                let mut not_found_route = None;

                let variants = variants
                    .into_iter()
                    .map(
                        |Variant {
                             ident,
                             attrs,
                             fields,
                             ..
                         }| {
                            // TODO replace panics with helpful errors.

                            let variant = ident;
                            let mut route = None;
                            for attribute in attrs {
                                let meta = attribute.parse_meta().expect("expected meta");
                                if let Meta::List(MetaList { path, nested, .. }) = &meta {
                                    if path.is_ident("at") {
                                        assert_eq!(nested.len(), 1);
                                        for nested in nested {
                                            match nested {
                                                NestedMeta::Lit(Lit::Str(s)) => {
                                                    route = Some(s.value());
                                                }
                                                _ => panic!("unexpected nested meta"),
                                            }
                                        }
                                    }
                                } else if let Meta::Path(path) = &meta {
                                    if path.is_ident("not_found") {
                                        assert!(
                                            not_found_route.is_none(),
                                            "multiple not found routes"
                                        );
                                        not_found_route = Some(variant.clone());
                                    }
                                } else {
                                    panic!("expected meta list");
                                }
                            }

                            let fields: HashMap<_, _> = fields
                                .into_iter()
                                .map(|field| {
                                    let ident = field.ident.unwrap();
                                    (ident.to_string(), ident)
                                })
                                .collect();

                            let mut route =
                                route.expect("no route specified with #[at(\"route\")]");
                            if !route.ends_with('/') {
                                route.push('/')
                            }
                            (variant, route, fields)
                        },
                    )
                    .collect();

                Ok(Self {
                    enum_name,
                    variants,
                    not_found_route,
                })
            }
            Data::Struct(s) => Err(syn::Error::new(
                s.struct_token.span,
                "expected enum, found struct",
            )),
            Data::Union(u) => Err(syn::Error::new(
                u.union_token.span,
                "expected enum, found union",
            )),
        }
    }
}

#[derive(Clone)]
enum RouteSegment {
    Field(Ident),
    Text(String),
}

struct Route {
    variant: Ident,
    segments: Vec<RouteSegment>,
}

pub(crate) fn derive_smol_routable(input: SmolRoutable) -> TokenStream {
    let mut to_paths = vec![];
    let mut string_routes = vec![];
    let mut routes = vec![];

    for (variant, route_string, mut fields) in input.variants {
        string_routes.push(route_string.clone());

        let mut segments: Vec<&str> = route_string.split('/').collect();
        assert!(segments.len() >= 2, "route must contain a /");

        let removed = segments.remove(0);
        assert!(removed.is_empty(), "route must begin with a /");

        // Remove trailing "/".
        if segments.last().unwrap().is_empty() {
            segments.pop();
        }

        let mut any_fields = false;
        let route: Vec<_> = segments
            .into_iter()
            .map(|text| {
                let field = text.strip_prefix(':');
                if let Some(field_name) = field {
                    let field = fields.remove(field_name).unwrap_or_else(|| {
                        panic!(
                            "field {} does not exist or was already used in route",
                            field_name
                        );
                    });
                    any_fields = true;
                    RouteSegment::Field(field)
                } else {
                    // assert!(!any_fields, "cannot do text after fields: {text}");
                    RouteSegment::Text(text.to_string())
                }
            })
            .collect();

        assert!(fields.is_empty(), "unused fields {:?}", fields);
        let _ = fields; // Make sure we don't use empty fields again.

        routes.push(Route {
            variant: variant.clone(),
            segments: route.clone(),
        });

        to_paths.push(if any_fields {
            use itertools::Itertools;

            let mut fields = vec![];
            let path = format!(
                "/{}/",
                route
                    .iter()
                    .map(|s| {
                        match s {
                            RouteSegment::Field(field) => {
                                fields.push(field);
                                "{}"
                            }
                            RouteSegment::Text(text) => text,
                        }
                    })
                    .join("/")
            );

            quote! {
                Self::#variant { #(#fields,)* } => format!(#path, #(#fields,)*),
            }
        } else {
            use itertools::Itertools;

            let path = if route.is_empty() {
                "".into()
            } else {
                format!(
                    "/{}/",
                    route
                        .iter()
                        .map(|s| {
                            if let RouteSegment::Text(text) = s {
                                text
                            } else {
                                unreachable!()
                            }
                        })
                        .join("/")
                )
            };

            quote! {
                Self::#variant => #path.into(),
            }
        });
    }

    let not_found_route = if let Some(r) = input.not_found_route {
        quote! {
            Some(Self::#r)
        }
    } else {
        quote! {
            None
        }
    };

    let mut routes_by_length = vec![];
    for route in routes {
        let n = route.segments.len();
        if n >= routes_by_length.len() {
            routes_by_length.resize_with(n + 1, Vec::new);
        }
        routes_by_length[n].push(route);
    }

    let mut next = quote! {
        None
    };
    for (i, routes) in routes_by_length.into_iter().enumerate().rev() {
        let span = Span::call_site();
        let si = Ident::new(&format!("s{i}"), span);

        let match_routes: Vec<_> = routes
            .iter()
            .map(|route| {
                if route.segments.is_empty() {
                    return quote! { true };
                }
                route
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(j, segment)| {
                        let sj = Ident::new(&format!("s{j}"), span);
                        match segment {
                            RouteSegment::Field(field) => {
                                quote! {
                                    let Ok(#field) = FromStr::from_str(#sj)
                                }
                            }
                            RouteSegment::Text(text) => {
                                quote! {
                                    #sj == #text
                                }
                            }
                        }
                    })
                    .intersperse(quote! { && })
                    .collect()
            })
            .collect();

        let create_routes: Vec<_> = routes
            .iter()
            .map(|route| {
                let variant = &route.variant;

                let initializers: Vec<_> = route
                    .segments
                    .iter()
                    .filter_map(|segment| match segment {
                        RouteSegment::Field(field) => Some(quote! { #field }),
                        RouteSegment::Text(_) => None,
                    })
                    .intersperse(quote! { , })
                    .collect();

                if initializers.is_empty() {
                    quote! { Self::#variant }
                } else {
                    quote! { Self::#variant { #(#initializers)* }}
                }
            })
            .collect();

        let current: Vec<_> = match_routes
            .iter()
            .zip(create_routes)
            .map(|(match_, create)| {
                quote! {
                    if #match_ {
                        Some(#create)
                    }
                }
            })
            .chain(std::iter::once(quote! {
                {
                    None
                }
            }))
            .intersperse(quote! { else })
            .collect();

        next = quote! {
            if let Some(#si) = s.next() {
                #next
            } else {
                #(#current)*
            }
        }
    }

    let enum_name = input.enum_name;
    let output = quote! {
        impl Routable for #enum_name {
            fn from_path(_path: &str, _params: &std::collections::HashMap<&str, &str>) -> Option<Self> {
                unimplemented!();
            }

            fn to_path(&self) -> String {
                match self {
                    #(#to_paths)*
                }
            }

            fn routes() -> Vec<&'static str> {
                vec![#(#string_routes,)*]
            }

            fn not_found_route() -> Option<Self> {
                #not_found_route
            }

            fn recognize(path: &str) -> Option<Self> {
                use std::str::FromStr;
                let mut s = path.strip_suffix('/').unwrap_or(path).split('/');
                let _ = s.next();
                let ret = #next;
                ret.or_else(Self::not_found_route)
            }
        }
    };
    output.into()
}
