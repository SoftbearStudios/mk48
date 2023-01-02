// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(exit_status_error)]
#![feature(array_zip)]

mod audio;

use common::entity::{EntityData, EntityKind, EntitySubKind, EntityType};
use glam::Vec3;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use sprite_sheet_util::{
    pack_audio_sprite_sheet, pack_monochrome, pack_sprite_sheet, Animation, Image, Output,
    PackInput,
};
use std::borrow::Cow;
use std::fs;
use std::sync::Mutex;

fn main() {
    pack_monochrome(
        512,
        512,
        ["sand", "grass", "snow"]
            .into_iter()
            .map(|name| PackInput::File(Cow::Owned(format!("../assets/textures/{name}.png"))))
            .chain(std::iter::once(PackInput::Image(
                image::open("../assets/textures/waves.png")
                    .unwrap()
                    .into_luma8(),
            ))),
        "../client/textures.png",
    );

    // Allows skipping audio if you don't have ffmpeg with `cargo run --no-default-features`.
    if cfg!(feature = "audio") {
        pack_audio_sprite_sheet(
            audio::sounds(),
            1,
            44100,
            "../assets/sounds",
            "../client/sprites_audio",
            "../client/src/sprites_audio",
            "../assets/sounds/README",
        );
    }

    // Sprites that aren't entities such as animations and missing contact icon.
    let non_entity_sprites = Mutex::new(Vec::<Image>::new());
    let animations = Mutex::new(Vec::<Animation>::new());

    fs::read_dir("../assets/sprites/")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|a| a.metadata().ok().map(|m| (a, m)))
        .par_bridge()
        .for_each(|(sprite_or_animation, meta)| {
            if meta.is_file() {
                // Sprite
                let name_os = sprite_or_animation.file_name();
                let name = name_os.to_str().unwrap();

                if let Some(name) = name.strip_suffix(".png").map(str::to_owned) {
                    println!("Including sprite {}", &name);
                    non_entity_sprites.lock().unwrap().push(Image {
                        file: format!("../assets/sprites/{}.png", name),
                        name,
                        width: 0,
                    });
                } else {
                    println!("Ignoring {}.", name);
                }
            } else if meta.is_dir() {
                // Animation
                let dir_os = sprite_or_animation.file_name();
                let dir = dir_os.to_str().unwrap();
                println!("Including animation {}", dir);
                animations.lock().unwrap().push(Animation {
                    name: dir.to_string(),
                    dir: format!("../assets/sprites/{}", dir),
                })
            }
        });

    let non_entity_sprites = non_entity_sprites.into_inner().unwrap();
    let animations = animations.into_inner().unwrap();

    let optimize = true;
    pack_sprite_sheet(
        EntityType::iter()
            .map(|entity_type| {
                let data: &EntityData = entity_type.data();
                let width = match data.kind {
                    EntityKind::Weapon | EntityKind::Aircraft | EntityKind::Decoy => {
                        let mut scale: f32 = 0.0;
                        for owner_type in EntityType::iter() {
                            let owner: &EntityData = owner_type.data();
                            if !matches!(owner.kind, EntityKind::Boat | EntityKind::Aircraft) {
                                // Turrets/rocket torpedoes don't render their armaments so they
                                // don't influence the resolution of the armament.
                                if owner.kind == EntityKind::Turret
                                    || owner.sub_kind == EntitySubKind::RocketTorpedo
                                {
                                    continue;
                                }
                                assert!(owner.armaments.is_empty(), "{owner_type:?} has armaments");
                            }

                            if owner.armaments.iter().any(|a| a.entity_type == entity_type) {
                                scale = scale.max(armament_pixels(data, owner));
                            }
                        }
                        if scale == 0.0 {
                            panic!("{:?} is not used", entity_type);
                        }
                        scale
                    }
                    EntityKind::Turret => {
                        let mut scale: f32 = 0.0;
                        for owner_type in EntityType::iter() {
                            let owner: &EntityData = owner_type.data();
                            if owner.kind != EntityKind::Boat {
                                assert!(owner.turrets.is_empty(), "{owner_type:?} has turrets");
                            }

                            if owner
                                .turrets
                                .iter()
                                .any(|t| t.entity_type == Some(entity_type))
                            {
                                scale = scale.max(armament_pixels(data, owner));
                            }
                        }
                        if scale == 0.0 {
                            panic!("{:?} is not used", entity_type);
                        }
                        scale
                    }
                    EntityKind::Obstacle => boat_pixels(data) * 0.85,
                    _ => boat_pixels(data),
                }
                // TODO clamp to 2048 once Montana is rendered above 1024.
                .clamp(4.0, 1024.0) as u32;

                entity_sprite_params(entity_type, width)
            })
            .chain(non_entity_sprites)
            .collect(),
        animations,
        4,
        true,
        true,
        optimize,
        &[
            Output {
                path: "../client/sprites_webgl",
                ..Default::default()
            },
            Output {
                path: "../client/sprites_normal_webgl",
                map_file: |color| {
                    if color.contains("contact") {
                        "does_not_exist".to_owned()
                    } else {
                        color.replace("color", "normal")
                    }
                },
                pre_process: Some(|pixel| {
                    fn valid(pixel: [u8; 4]) -> bool {
                        !pixel[0..3].iter().all(|&v| v >= 127 - 3 && v <= 127 + 3)
                    }

                    if !valid(pixel) || pixel[3] == 0 {
                        [127, 127, 127, pixel[3].max(1)] // Png encoder requires alpha of 1 to not drop color contrary to spec.
                    } else {
                        pixel
                    }
                }),
                post_process: Some(|pixel| {
                    fn normalize(pixel: [u8; 4]) -> [u8; 4] {
                        let input = [pixel[0], pixel[1], pixel[2]];
                        let normal = (Vec3::from(input.map(|v| v as f32)) * (2.0 / 254.0) - 1.0)
                            .normalize_or_zero();
                        let output = ((normal + 1.0) * (254.0 / 2.0)).to_array().map(|v| v as u8);
                        [output[0], output[1], output[2], pixel[3]]
                    }

                    normalize(pixel)
                }),
                if_missing: Some([127, 127, 255, 1]),
                padding: [127, 127, 127, 1],
            },
        ],
        "../client/src/sprites_webgl",
    );

    pack_sprite_sheet(
        EntityType::iter()
            .filter_map(|entity_type| {
                let data: &'static EntityData = entity_type.data();
                let aspect = data.length / data.width;
                let width = match data.kind {
                    EntityKind::Boat => 160,
                    EntityKind::Weapon | EntityKind::Decoy | EntityKind::Aircraft => {
                        120.min((40.0 * aspect) as u32)
                    }
                    _ => 0,
                };
                (width != 0).then(|| entity_sprite_params(entity_type, width))
            })
            .collect(),
        vec![],
        2,
        false,
        false,
        optimize,
        &[Output {
            path: "../client/sprites_css",
            ..Default::default()
        }],
        "../client/src/ui/sprites_css",
    );
}

/// Returns the number of pixels (width of sprite) that an `armament` should be in the sprite sheet
/// if it was rendered on an `owner`.
fn armament_pixels(armament: &EntityData, owner: &EntityData) -> f32 {
    boat_pixels(owner) * armament.length / owner.length
}

/// Returns the number of pixels (width of sprite) that a boat should be in the sprite sheet.
/// Smaller boats have increased resolution because they are viewed at higher zoom.
fn boat_pixels(boat: &EntityData) -> f32 {
    let meters = boat.length;
    fn f(x: f32) -> f32 {
        62.0 * x.sqrt()
    }
    f(meters).min(meters * f(18.0) / 18.0)
}

/// Creates [`Image`] from an `entity_type` and a `width` while mapping all shells to mark8.
fn entity_sprite_params(entity_type: EntityType, width: u32) -> Image {
    // Other shells besides mark8 don't have their own sprites so they copy mark8.
    let mut file_name = entity_type.as_str();
    if file_name.contains("MmR") {
        file_name = "Mark8";
    }

    Image {
        name: entity_type.as_str().to_owned(),
        file: format!("../assets/models/rendered/{file_name}/color0001.png"),
        width,
    }
}
