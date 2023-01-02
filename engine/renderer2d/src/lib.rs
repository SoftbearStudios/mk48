// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(array_zip)]
#![feature(label_break_value)]
#![feature(mixed_integer_ops)]
#![feature(build_hasher_simple_hash_one)]
#![feature(hash_raw_entry)]
#![warn(missing_docs)]
#![crate_name = "renderer2d"]

//! # Renderer2D
//!
//! [`renderer2d`][`crate`] is an add-on to [`renderer`] that provides a [`Camera2d`] and some 2D specific
//! [`Layer`][`renderer::Layer`]s.

mod background;
mod camera_2d;
mod graphic;
mod mask;
mod particle;
mod text;

pub use background::*;
pub use camera_2d::*;
pub use graphic::*;
pub use mask::*;
pub use particle::*;
pub use text::*;
