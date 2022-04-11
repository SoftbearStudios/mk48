// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use axum::body::{boxed, Empty, Full};
use axum::headers::HeaderMap;
use axum::http::header::{ACCEPT, ACCEPT_ENCODING, IF_NONE_MATCH};
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use flate2::write::GzEncoder;
use flate2::Compression;
use image::ImageFormat;
use rust_embed::{EmbeddedFile, RustEmbed};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::convert::TryInto;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::Cursor;
use std::io::Write;
use std::sync::RwLock;

// We use a wildcard matcher ("/dist/*file") to match against everything
// within our defined assets directory. This is the directory on our Asset
// struct below, where folder = "examples/public/".
pub async fn static_handler<E: RustEmbed>(uri: Uri, headers: HeaderMap) -> impl IntoResponse {
    lazy_static::lazy_static! {
        static ref CACHED: RwLock<HashMap<String, CachedFile>> = RwLock::new(HashMap::new());
    }

    // Path, minus preceding slash, and minus trailing index.html.
    let path = uri
        .path()
        .trim_start_matches('/')
        .trim_end_matches("index.html");

    let if_none_match = headers.get(IF_NONE_MATCH);

    let accepting_gzip = headers
        .get(ACCEPT_ENCODING)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.contains("gzip"))
        .unwrap_or(false);

    let accepting_webp = headers
        .get(ACCEPT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.contains("image/webp"))
        .unwrap_or(false);

    let cached_map: &'static RwLock<HashMap<String, CachedFile>> = &*CACHED;

    #[cfg(not(debug_assertions))]
    let cached = lookup_cached(
        path,
        if_none_match,
        accepting_gzip,
        accepting_webp,
        cached_map,
    );

    // Load from disk every time, in case it changed.
    #[cfg(debug_assertions)]
    let cached: Option<Response> = None;

    if let Some(cached_response) = cached {
        cached_response
    } else {
        let true_path = if path.is_empty() || path.ends_with('/') {
            // Undo removing index.html so we can lookup via rust_embed.
            format!("{}index.html", path)
        } else {
            String::from(path)
        };

        /*
        println!(
            "true={}, files={:?}",
            true_path,
            E::iter().map(|s| s.to_owned()).collect::<Vec<_>>()
        );
         */

        // Populate cache.
        match E::get(&true_path) {
            Some(embedded) => {
                let cached_file =
                    tokio::task::spawn_blocking(move || CachedFile::new(&true_path, embedded))
                        .await
                        .unwrap();

                cached_map
                    .write()
                    .unwrap()
                    .insert(path.to_owned(), cached_file);

                lookup_cached(
                    path,
                    if_none_match,
                    accepting_gzip,
                    accepting_webp,
                    cached_map,
                )
                .unwrap()
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(boxed(Full::from("404 Not Found")))
                .unwrap(),
        }
    }
}

fn lookup_cached(
    path: &str,
    if_none_match: Option<&HeaderValue>,
    accepting_gzip: bool,
    accepting_webp: bool,
    cached_map: &'static RwLock<HashMap<String, CachedFile>>,
) -> Option<Response> {
    let cached_map_read = cached_map.read().unwrap();
    let cached = cached_map_read.get(path)?;

    if if_none_match
        .map(|inm| inm == &cached.etag)
        .unwrap_or(false)
    {
        return Some(
            Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(boxed(Empty::new()))
                .unwrap(),
        );
    }

    if let Some((compressed, is_webp)) = &cached.compressed {
        if *is_webp {
            if accepting_webp {
                return Some(
                    Response::builder()
                        .header(header::ETAG, cached.etag.clone())
                        .header(header::CONTENT_TYPE, "image/webp")
                        .body(boxed(Full::from(compressed.clone())))
                        .unwrap(),
                );
            }
        } else if accepting_gzip {
            return Some(
                Response::builder()
                    .header(header::ETAG, cached.etag.clone())
                    .header(header::CONTENT_ENCODING, "gzip")
                    .header(header::CONTENT_TYPE, cached.mime.clone())
                    .body(boxed(Full::from(compressed.clone())))
                    .unwrap(),
            );
        }
    }

    Some(
        Response::builder()
            .header(header::ETAG, cached.etag.clone())
            .header(header::CONTENT_TYPE, cached.mime.clone())
            .body(boxed(Full::from(cached.raw.clone())))
            .unwrap(),
    )
}

struct CachedFile {
    mime: HeaderValue,
    etag: HeaderValue,
    raw: Cow<'static, [u8]>,
    /// bool is false if gzip, true if webp.
    compressed: Option<(Bytes, bool)>,
}

impl CachedFile {
    pub fn new(true_path: &str, embedded: EmbeddedFile) -> Self {
        let mime = mime_guess::from_path(&true_path).first_or_octet_stream();
        let etag = hex::encode(embedded.metadata.sha256_hash());

        let compressed = match mime.essence_str() {
            #[cfg(not(debug_assertions))]
            "image/png" | "image/jpeg" => {
                let cursor = Cursor::new(embedded.data.as_ref());
                let mut reader = image::io::Reader::new(cursor);
                reader.set_format(match mime.essence_str() {
                    "image/png" => ImageFormat::Png,
                    "image/jpeg" => ImageFormat::Jpeg,
                    _ => unreachable!(),
                });
                match reader.decode() {
                    Ok(image) => {
                        let webp_image = webp::Encoder::from_rgba(
                            image.as_bytes(),
                            image.width(),
                            image.height(),
                        )
                        .encode(90.0);

                        Some((Bytes::copy_from_slice(webp_image.as_ref()), true))
                    }
                    Err(e) => {
                        println!("failed to decode {} due to {}", true_path, e);
                        None
                    }
                }
            }
            #[cfg(not(debug_assertions))]
            _ if embedded.data.as_ref().len() > 1000 => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
                encoder.write_all(embedded.data.as_ref()).unwrap();
                let vec = encoder.finish().unwrap();
                if vec.len() < embedded.data.as_ref().len() {
                    Some((Bytes::from(vec), false))
                } else {
                    // Compression bought us nothing.
                    None
                }
            }
            _ => None,
        };

        Self {
            mime: mime.as_ref().try_into().unwrap(),
            etag: etag.try_into().unwrap(),
            raw: embedded.data.into(),
            compressed,
        }
    }
}

/// Returns the size in bytes of all client files, followed by a collective hash of them.
pub fn static_size_and_hash<E: RustEmbed>() -> (usize, u64) {
    let mut size = 0;
    let mut hash = 0u64;

    for path in E::iter() {
        let file = E::get(path.as_ref()).unwrap();
        size += match &file.data {
            Cow::Owned(owned) => owned.len(),
            &Cow::Borrowed(borrowed) => borrowed.len(),
        };
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        file.metadata.sha256_hash().hash(&mut hasher);
        // println!("{:?} -> {}", path, hasher.finish());
        // Order-independent.
        hash ^= hasher.finish();
    }

    (size, hash)
}
