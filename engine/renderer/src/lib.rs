#![feature(associated_type_defaults)]
#![feature(cell_update)]
// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later
#![warn(missing_docs)]
#![crate_name = "renderer"]

//! # Renderer
//!
//! [`renderer`][`crate`] is an abstraction over
//! [WebGL](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.WebGlRenderingContext.html)/
//! [WebGL2](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.WebGl2RenderingContext.html)
//! that can be used in 2D and 3D applications.

// Gl primitives should not escape this crate.
#[macro_use]
mod gl;

#[cfg(feature = "srgb")]
mod srgb_layer;

mod attribs;
mod buffer;
mod deque;
mod framebuffer;
mod index;
mod instance;
mod renderer;
mod rgb;
mod shader;
mod texture;
mod toggle;
mod vertex;

// Required to be public so derive Vertex works.
#[doc(hidden)]
pub use attribs::*;

// Re-export to provide a simpler api.
pub use buffer::*;
pub use deque::*;
pub use framebuffer::*;
pub use index::*;
pub use instance::*;
pub use renderer::*;
pub use rgb::*;
pub use shader::*;
pub use texture::*;
pub use toggle::*;
pub use vertex::*;
