// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::Vec2;
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
    pub uvs: [Vec2; 4],
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
    pub fn to_uv_spritesheet(&self) -> UvSpriteSheet {
        UvSpriteSheet {
            sprites: self
                .sprites
                .iter()
                .map(|(name, sprite)| (name.clone(), self.to_uv_sprite(sprite)))
                .collect(),
            animations: self
                .animations
                .iter()
                .map(|(animation, frames)| {
                    (
                        animation.clone(),
                        frames
                            .iter()
                            .map(|frame| self.to_uv_sprite(frame))
                            .collect(),
                    )
                })
                .collect(),
        }
    }

    fn to_uv_sprite(&self, sprite: &Sprite) -> UvSprite {
        /*
        A  B

        C  D
         */

        let inv_width = 1.0 / self.width as f32;
        let inv_height = 1.0 / self.height as f32;

        let uvs = [
            Vec2::new(sprite.x as f32 * inv_width, sprite.y as f32 * inv_height),
            Vec2::new(
                (sprite.x + sprite.width) as f32 * inv_width,
                sprite.y as f32 * inv_height,
            ),
            Vec2::new(
                sprite.x as f32 * inv_width,
                (sprite.y + sprite.height) as f32 * inv_height,
            ),
            Vec2::new(
                (sprite.x + sprite.width) as f32 * inv_width,
                (sprite.y + sprite.height) as f32 * inv_height,
            ),
        ];

        let aspect = sprite.height as f32 / sprite.width as f32;

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
