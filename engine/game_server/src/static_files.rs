// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use axum::body::{boxed, Empty, Full};
use axum::handler::Handler;
use axum::http::header::{ACCEPT, ACCEPT_ENCODING, IF_NONE_MATCH};
use axum::http::{header, HeaderValue, Request, StatusCode};
use axum::response::Response;
use hyper::Body;
use minicdn::{Base64Bytes, MiniCdn};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::future::ready;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct StaticFilesHandler {
    pub cdn: Arc<RwLock<MiniCdn>>,
    pub prefix: &'static str,
    pub browser_router: bool,
}

impl<S: Send + Sync + 'static> Handler<((),), S> for StaticFilesHandler {
    type Future = std::future::Ready<Response>;

    fn call(self, req: Request<Body>, _: S) -> Self::Future {
        // Path, minus preceding slash, prefix, and trailing index.html.
        let path = req
            .uri()
            .path()
            .trim_start_matches(self.prefix)
            .trim_start_matches('/')
            .trim_end_matches("index.html");

        let true_path = if self.browser_router && !path.contains('.') {
            // Browser routers require that all routes return the root index.html file.
            Cow::Borrowed("index.html")
        } else if path.is_empty() || path.ends_with('/') {
            // Undo removing index.html so we can lookup via rust_embed.
            Cow::Owned(format!("{}index.html", path))
        } else {
            Cow::Borrowed(path)
        };

        let files = self.cdn.read().unwrap();
        let file = match files.get(&true_path) {
            Some(file) => file,
            None => {
                return ready(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(boxed(Full::from("404 Not Found")))
                        .unwrap(),
                )
            }
        };

        let if_none_match = req.headers().get(IF_NONE_MATCH);

        let (accepting_brotli, accepting_gzip) = req
            .headers()
            .get(ACCEPT_ENCODING)
            .and_then(|h| h.to_str().ok())
            .map(|s| (s.contains("br"), s.contains("gzip")))
            .unwrap_or((false, false));

        let accepting_webp = req
            .headers()
            .get(ACCEPT)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.contains("image/webp"))
            .unwrap_or(false);

        ready(
            if if_none_match
                .map(|inm| {
                    let s: &str = file.etag.as_ref();
                    inm == s
                })
                .unwrap_or(false)
            {
                Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body(boxed(Empty::new()))
                    .unwrap()
            } else if let Some(contents_webp) =
                file.contents_webp.as_ref().filter(|_| accepting_webp)
            {
                Response::builder()
                    .header(header::ETAG, unsafe {
                        HeaderValue::from_maybe_shared_unchecked(file.etag.as_bytes().clone())
                    })
                    .header(header::CONTENT_TYPE, "image/webp")
                    .body(boxed(Full::from(<Base64Bytes as Into<
                        axum::body::Bytes,
                    >>::into(
                        contents_webp.clone()
                    ))))
                    .unwrap()
            } else if let Some(contents_brotli) =
                file.contents_brotli.as_ref().filter(|_| accepting_brotli)
            {
                Response::builder()
                    .header(header::ETAG, unsafe {
                        HeaderValue::from_maybe_shared_unchecked(file.etag.as_bytes().clone())
                    })
                    .header(header::CONTENT_ENCODING, "br")
                    .header(header::CONTENT_TYPE, unsafe {
                        HeaderValue::from_maybe_shared_unchecked(file.mime.as_bytes().clone())
                    })
                    .body(boxed(Full::from(<Base64Bytes as Into<
                        axum::body::Bytes,
                    >>::into(
                        contents_brotli.clone()
                    ))))
                    .unwrap()
            } else if let Some(contents_gzip) =
                file.contents_gzip.as_ref().filter(|_| accepting_gzip)
            {
                Response::builder()
                    .header(header::ETAG, unsafe {
                        HeaderValue::from_maybe_shared_unchecked(file.etag.as_bytes().clone())
                    })
                    .header(header::CONTENT_ENCODING, "gzip")
                    .header(header::CONTENT_TYPE, unsafe {
                        HeaderValue::from_maybe_shared_unchecked(file.mime.as_bytes().clone())
                    })
                    .body(boxed(Full::from(<Base64Bytes as Into<
                        axum::body::Bytes,
                    >>::into(
                        contents_gzip.clone()
                    ))))
                    .unwrap()
            } else {
                Response::builder()
                    .header(header::ETAG, unsafe {
                        HeaderValue::from_maybe_shared_unchecked(file.etag.as_bytes().clone())
                    })
                    .header(header::CONTENT_TYPE, unsafe {
                        HeaderValue::from_maybe_shared_unchecked(file.mime.as_bytes().clone())
                    })
                    .body(boxed(Full::from(<Base64Bytes as Into<
                        axum::body::Bytes,
                    >>::into(
                        file.contents.clone()
                    ))))
                    .unwrap()
            },
        )
    }
}

/// Returns the size in bytes of all client files, followed by a collective hash of them.
pub fn static_size_and_hash(cdn: &MiniCdn) -> (usize, u64) {
    let mut size = 0;
    let mut hash = 0u64;

    cdn.for_each(|path, file| {
        size += file.contents.len();
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        file.etag.hash(&mut hasher);
        //println!("{:?} -> {}", path, hasher.finish());
        // Order-independent.
        hash ^= hasher.finish();
    });

    (size, hash)
}
