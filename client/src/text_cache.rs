// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::texture::Texture;
use std::collections::HashMap;
use web_sys::WebGlRenderingContext as Gl;

/// Caches textures from input strings, discarding them if they go unused for a while.
#[derive(Default)]
pub struct TextCache {
    textures: HashMap<String, (Texture, u8)>,
}

impl TextCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&mut self, gl: &Gl, text: &str) -> &Texture {
        let texture = self
            .textures
            .raw_entry_mut()
            .from_key(text)
            .or_insert_with(|| {
                //crate::console_log!("Alloc: {}", text);
                (text.to_owned(), (Texture::from_str(gl, &text), 0))
            })
            .1;

        texture.1 = 0;
        &texture.0
    }

    pub fn tick(&mut self) {
        self.textures.drain_filter(|_text, texture| {
            if let Some(next) = texture.1.checked_add(1) {
                texture.1 = next;
                false
            } else {
                //crate::console_log!("Free: {}", text);
                true
            }
        });
    }
}
