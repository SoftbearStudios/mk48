// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use bytemuck::Pod;
use std::fmt::Debug;

/// Types that can be used as `indices` in [`MeshBuilder`][`crate::buffer::MeshBuilder`]
/// and [`TriangleBuffer`][`crate::buffer::TriangleBuffer`].
pub trait Index: TryFrom<usize> + Pod + Debug + Default {
    #[doc(hidden)]
    const GL_ENUM: u32;

    /// Converts a usize to the [`Index`].
    ///
    /// # Panics
    ///
    /// When `n` overflows the type of [`Index`] it panics in debug mode. `n` is truncated in
    /// release mode.
    fn from_usize(n: usize) -> Self;
}

macro_rules! impl_index {
    ($typ:ty, $name:literal, $gl_enum:expr) => {
        impl Index for $typ {
            const GL_ENUM: u32 = $gl_enum;
            #[inline]
            fn from_usize(n: usize) -> Self {
                #[cfg(debug_assertions)]
                <$typ>::try_from(n).expect(concat!("index overflowed ", $name));
                n as Self
            }
        }
    };
}

impl_index!(u8, "u8", Gl::UNSIGNED_BYTE);
impl_index!(u16, "u16", Gl::UNSIGNED_SHORT);
impl_index!(u32, "u32", Gl::UNSIGNED_INT);
