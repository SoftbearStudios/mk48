// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{uvec2, vec2, UVec2, Vec2};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};

/// SpriteSheet stores pixel coordinates of its sprites.
#[derive(Serialize, Deserialize)]
pub struct SpriteSheet {
    pub width: u32,
    pub height: u32,
    /// Sprites are addressed by their name.
    #[serde(serialize_with = "ordered_map")]
    pub sprites: HashMap<String, Sprite>,
    /// Each animation has a name and multiple sprites to cycle through.
    #[serde(serialize_with = "ordered_map")]
    pub animations: HashMap<String, Vec<Sprite>>,
}

/// Sprite stores pixel coordinates.
#[derive(Serialize, Deserialize)]
pub struct Sprite {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// UvSpriteSheet stores the precise texture coordinates of its sprites.
#[derive(Serialize, Deserialize)]
pub struct UvSpriteSheet {
    /// Sprites are addressed by their name.
    #[serde(serialize_with = "ordered_map")]
    pub sprites: HashMap<String, UvSprite>,
    /// Each animation has a name and multiple sprites to cycle through.
    #[serde(serialize_with = "ordered_map")]
    pub animations: HashMap<String, Vec<UvSprite>>,
}

/// UvSprite stores precise texture coordinates.
#[derive(Serialize, Deserialize)]
pub struct UvSprite {
    /// Texture coordinates into [`UvSpriteSheet`] in counter-clockwise order starting at bottom
    /// left.
    ///
    /// ```x
    /// D - C
    /// | / |
    /// A - B
    /// ```
    pub uvs: [Vec2; 4],
    /// Aspect ratio aka width / height. Could be calculated from uvs.
    pub aspect: f32,
}

#[derive(Serialize, Deserialize)]
pub struct AudioSpriteSheet {
    /// AudioSprites are addressed by their name, and may have multiple variations.
    #[serde(serialize_with = "ordered_map")]
    pub sprites: HashMap<String, AudioSprite>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioSprite {
    pub start: f32,
    pub loop_start: Option<f32>,
    pub duration: f32,
}

impl SpriteSheet {
    /// Dimensions of [`SpriteSheet`] equivilant to `uvec2(width, height)`.
    pub fn dimensions(&self) -> UVec2 {
        uvec2(self.width, self.height)
    }

    /// Converts a [`SpriteSheet`] into a [`UvSpriteSheet`] which is useful for rendering.
    pub fn to_uv_spritesheet(&self) -> UvSpriteSheet {
        UvSpriteSheet {
            sprites: self
                .sprites
                .iter()
                .map(|(name, sprite)| (name.clone(), sprite.uvs(self.dimensions())))
                .collect(),
            animations: self
                .animations
                .iter()
                .map(|(animation, frames)| {
                    (
                        animation.clone(),
                        frames
                            .iter()
                            .map(|frame| frame.uvs(self.dimensions()))
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

impl Sprite {
    /// Position of [`Sprite`] equivilant to `uvec2(x, y)`.
    pub fn position(&self) -> UVec2 {
        uvec2(self.x, self.y)
    }

    /// Dimensions of [`Sprite`] equivilant to `uvec2(width, height)`.
    pub fn dimensions(&self) -> UVec2 {
        uvec2(self.width, self.height)
    }

    /// Converts a [`Sprite`] into a [`UvSprite`]. Requires the dimensions of the [`SpriteSheet`].
    fn uvs(&self, sheet_dims: UVec2) -> UvSprite {
        let pos = self.position().as_vec2();
        let dim = self.dimensions().as_vec2();

        /*
        D  C

        A  B
         */
        let uvs = [
            pos + vec2(0.0, dim.y),
            pos + dim,
            pos + vec2(dim.x, 0.0),
            pos,
        ]
        .map(|v| v / sheet_dims.as_vec2());

        let aspect = dim.x / dim.y;
        UvSprite { uvs, aspect }
    }
}

fn ordered_map<S, K, V>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Serialize + Eq + PartialEq + Ord + PartialOrd,
    V: Serialize,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}
