// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![warn(missing_docs)]
#![crate_name = "renderer3d"]

//! # Renderer3D
//!
//! [`renderer3d`][`crate`] is an add-on to [`renderer`] that provides a [`Camera3d`], and in the future, some
//! 3D specific [`Layer`][`renderer::Layer`]s.

use renderer::Renderer;

mod camera_3d;
mod model;

// Re-export to provide a simpler api.
pub use camera_3d::*;
pub use model::*;

/// An alias for [`Renderer<Camera3d>`].
pub type Renderer3d = Renderer<Camera3d>;
