// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(exit_status_error)]
#![feature(int_log)]
#![feature(result_option_inspect)]
#![feature(io_error_more)]
#![feature(let_else)]
#![feature(array_zip)]
#![feature(mixed_integer_ops)]
#![warn(missing_docs)]
#![crate_name = "sprite_sheet_util"]

//! # Sprite Sheet Util
//!
//! [`sprite_sheet_util`][`crate`] facilitates the creation of image and audio
//! [`sprite_sheet`]s.

mod audio;
mod compress;
mod sprite;

// Re-export to provide a simpler api.
pub use audio::*;
pub use compress::*;
pub use sprite::*;
