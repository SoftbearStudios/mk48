// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::body::{boxed, Full};
use axum::http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use std::str::FromStr;

pub fn limit_content_length(headers: &HeaderMap, limit: usize) -> Result<(), Response> {
    if headers
        .get(TRANSFER_ENCODING)
        .map(|hv| hv == "chunked")
        .unwrap_or(false)
    {
        return Err(Response::builder()
            .status(StatusCode::LENGTH_REQUIRED)
            .body(boxed(Full::from(
                "Content-Length required, chunked Transfer-Encoding not supported",
            )))
            .unwrap());
    }
    if headers
        .get(CONTENT_LENGTH)
        .and_then(|hv| hv.to_str().ok())
        .and_then(|s| usize::from_str(s).ok())
        .map(|u| u > limit)
        .unwrap_or(false)
    {
        return Err(Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .body(boxed(Full::from("Payload too large")))
            .unwrap());
    }

    // Either no Content-Length (no body) or acceptable Content-Length.
    Ok(())
}
