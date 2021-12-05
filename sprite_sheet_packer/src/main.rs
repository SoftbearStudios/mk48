// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(exit_status_error)]

mod audio;
mod texture;

use crate::audio::pack_audio_sprite_sheet;
use crate::texture::{pack_sprite_sheet, webpify, EntityPackParams};
use common::entity::{EntityData, EntityKind, EntityType};
use common::util::map_ranges;

fn main() {
    unsafe { EntityType::init() };

    webpify("../js/public/sand.png");
    webpify("../js/public/grass.png");

    pack_audio_sprite_sheet(
        1,
        44100,
        "../js/public/sprites_audio",
        "../client/src/sprites_audio",
        "../assets/sounds/README",
    );

    //return;

    // NOTE: Pre-multiplication is not compatible with WebP, so avoid doing it here.

    let optimize = true;
    pack_sprite_sheet(
        |entity_type| {
            let data: &'static EntityData = entity_type.data();
            let min_width = if data.kind == EntityKind::Boat {
                200
            } else {
                48
            } as f32;
            EntityPackParams {
                width: map_ranges(
                    entity_type.data().length,
                    0f32..200f32,
                    min_width..1024f32,
                    true,
                ) as u32,
            }
        },
        true,
        4,
        true,
        true,
        optimize,
        "../js/public/sprites_webgl",
        "../client/src/sprites_webgl",
    );

    pack_sprite_sheet(
        |entity_type| {
            let data: &'static EntityData = entity_type.data();
            let aspect = data.length / data.width;
            match data.kind {
                EntityKind::Boat => EntityPackParams { width: 160 },
                EntityKind::Weapon | EntityKind::Decoy | EntityKind::Aircraft => EntityPackParams {
                    width: 120.min((40.0 * aspect) as u32),
                },
                _ => EntityPackParams { width: 0 },
            }
        },
        false,
        2,
        false,
        false,
        optimize,
        "../js/public/sprites_css",
        "../js/src/data/sprites_css",
    );
}

fn shorten_name(name: &str) -> String {
    let string = name.to_string();
    let idx = string.rfind('.').unwrap();
    String::from(&string[..idx])
}
