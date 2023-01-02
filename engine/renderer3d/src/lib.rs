// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(slice_as_chunks)]
#![feature(let_else)]
#![feature(is_some_with)]
#![feature(array_chunks)]
#![warn(missing_docs)]
#![crate_name = "renderer3d"]

//! # Renderer3D
//!
//! [`renderer3d`][`crate`] is an add-on to [`renderer`] that provides a [`Camera3d`], and in the future, some
//! 3D specific [`Layer`][`renderer::Layer`]s.

extern crate core;

#[cfg(feature = "shadow")]
mod shadow;

mod aabb;
mod camera_3d;
mod crosshair;
mod free_camera;
mod model;
mod skybox;
mod svg;

// Re-export to provide a simpler api.
#[cfg(feature = "shadow")]
pub use shadow::*;

pub use aabb::*;
pub use camera_3d::*;
pub use crosshair::*;
pub use free_camera::*;
pub use model::*;
pub use skybox::*;
