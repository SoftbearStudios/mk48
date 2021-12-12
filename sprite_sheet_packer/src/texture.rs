// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::shorten_name;
use common::entity::EntityType;
use crunch::{pack, Item, Rect, Rotation};
use image::imageops::{replace, resize, FilterType};
use image::{codecs::png, io::Reader, ColorType, GenericImageView, ImageEncoder, RgbaImage};
use oxipng::{optimize_from_memory, Headers, Options};
use rayon::prelude::*;
use sprite_sheet::{Sprite, SpriteSheet};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::sync::Mutex;

#[derive(Eq, PartialEq)]
pub(crate) struct EntityPackParams {
    /// If zero, will skip.
    pub(crate) width: u32,
}

const WEBP_QUALITY: f32 = 90.0;

pub(crate) fn pack_sprite_sheet<E: Fn(EntityType) -> EntityPackParams + Sync>(
    entity_params: E,
    sprites_and_animations: bool,
    padding: u32,
    power_of_two: bool,
    uv_spritesheet: bool,
    optimize: bool,
    output_texture: &str,
    output_data: &str,
) {
    let mut processed_images: HashMap<_, _> = EntityType::iter()
        .collect::<Vec<_>>()
        .par_iter()
        .filter_map(|t| {
            let entity_type = *t;
            let params = entity_params(entity_type);

            if params.width == 0 {
                return None;
            }

            println!(
                "Processing entity type {:?} with width {}...",
                entity_type, params.width
            );

            let image_name = entity_type.to_string();
            let mut alt_image_name = image_name.as_str();

            // Some shell models use the same sprite.
            if alt_image_name.contains("mmR") {
                alt_image_name = "mark8";
            }

            let image_path = format!("../assets/models/rendered/{}.png", &image_name);

            let alt_image_path = format!("../assets/models/rendered/{}.png", alt_image_name);

            println!("Loading {}...", image_path);

            let image = Reader::open(&image_path)
                .unwrap_or_else(|_| {
                    println!("Using alt sprite {}...", alt_image_path);
                    Reader::open(&alt_image_path).unwrap()
                })
                .decode()
                .unwrap();

            let width = image.width();
            let height = image.height();
            let aspect = width as f32 / height as f32;
            if params.width > width {
                println!(
                    "Warning upscaling {} from {} to {}",
                    &image_name, width, params.width
                )
            }

            let resized = resize(
                &image,
                params.width,
                (params.width as f32 / aspect) as u32,
                FilterType::Lanczos3,
            );

            Some((image_name, resized))
        })
        .collect();

    let animations = if sprites_and_animations {
        let images_mutex = Mutex::new(&mut processed_images);
        let animations: HashMap<_, _> = fs::read_dir("../assets/sprites/")
            .unwrap()
            .filter_map(|f| f.ok())
            .filter_map(|a| a.metadata().ok().map(|m| (a, m)))
            .collect::<Vec<_>>()
            .par_iter()
            .filter_map(|(sprite_or_animation, meta)| {
                if meta.is_file() {
                    // Sprite
                    let name_os = sprite_or_animation.file_name();
                    let name = name_os.to_str().unwrap();

                    if !name.ends_with(".png") {
                        println!("Ignoring {}.", name);
                    } else {
                        let image = Reader::open(&format!("../assets/sprites/{}", name))
                            .unwrap()
                            .decode()
                            .unwrap();

                        let short_name = shorten_name(name);

                        println!("Including {}...", &short_name);

                        let image = image.into_rgba8();
                        images_mutex.lock().unwrap().insert(short_name, image);
                    }
                    None
                } else if meta.is_dir() {
                    // Animation
                    let dir_os = sprite_or_animation.file_name();
                    let dir = dir_os.to_str().unwrap();

                    println!("Including animation {}", dir);

                    let frame_set: BTreeSet<_> =
                        fs::read_dir(&format!("../assets/sprites/{}", dir))
                            .unwrap()
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>()
                            .par_iter()
                            .map(|entry_2| {
                                let name_os = entry_2.file_name();
                                let name = name_os.to_str().unwrap();

                                let image =
                                    Reader::open(&format!("../assets/sprites/{}/{}", dir, name))
                                        .unwrap()
                                        .decode()
                                        .unwrap();

                                let short_name = shorten_name(name);

                                let image = image.into_rgba8();
                                images_mutex
                                    .lock()
                                    .unwrap()
                                    .insert(short_name.to_owned(), image);
                                short_name
                            })
                            .collect();
                    Some((dir.to_string(), frame_set))
                } else {
                    None
                }
            })
            .collect();

        // Renumber animation frames to be consecutive.
        for (animation, frames) in animations.iter() {
            for (i, frame) in frames.iter().enumerate() {
                let old_name = frame;
                let new_name = format!("{}{}", animation, i);
                let image = processed_images.remove(old_name).unwrap();
                processed_images.insert(new_name, image);
            }
        }
        animations
    } else {
        HashMap::new()
    };

    // Sort images by size for better packing results.
    let sorted: BTreeMap<_, _> = processed_images.into_iter().collect();

    // Packing conserves overall area. Don't even try sizes that wouldn't fit all the images.
    let total_area: u32 = sorted.iter().map(|(_, i)| i.width() * i.height()).sum();

    let size_step = 1;

    let min_size = if power_of_two {
        (total_area as f32).sqrt().log2() as u32
    } else {
        (total_area as f32).sqrt() as u32 / size_step
    };

    for size in (min_size..=if power_of_two { 12 } else { 64 * 32 })
        .into_iter()
        .map(|power| {
            if power_of_two {
                2u32.pow(power as u32)
            } else {
                power * size_step
            }
        })
    {
        println!("Trying {}px...", size);

        let c_size = (size + padding) as usize;
        let container = Rect::of_size(c_size, c_size);
        let items: Vec<_> = sorted
            .iter()
            .map(|(key, image)| {
                let w = image.width() + padding;
                let h = image.height() + padding;
                Item::new(
                    key.to_owned(),
                    w as usize,
                    h as usize,
                    Rotation::None, /*Allowed*/
                )
            })
            .collect();

        let packed_rects = match pack(container, items) {
            Ok(all_packed) => {
                println!("All packed!");
                all_packed
            }
            Err(some_packed) => {
                println!(
                    "Only packed {}/{}.",
                    some_packed.into_iter().count(),
                    sorted.len()
                );
                continue;
            }
        };

        let mut packed = RgbaImage::new(size, size);
        let mut data = SpriteSheet {
            width: size,
            height: size,
            sprites: HashMap::new(),
            animations: animations
                .iter()
                .map(|(animation, frames)| (animation.to_owned(), Vec::with_capacity(frames.len())))
                .collect(),
        };

        for (rect, key) in packed_rects {
            // Blit image.
            let image = &sorted[&key];
            // Don't add padding / 2 because of c_size.
            let x = rect.x as u32;
            let y = rect.y as u32;

            replace(&mut packed, image, x, y);

            let width = rect.w as u32 - padding;
            let height = rect.h as u32 - padding;
            data.sprites.insert(
                key.to_string(),
                Sprite {
                    x,
                    y,
                    width,
                    height,
                },
            );
        }

        // Move animation frames into animations.
        for (animation, frames) in data.animations.iter_mut() {
            for i in 0..frames.capacity() {
                let frame = data.sprites.remove(&format!("{}{}", animation, i)).unwrap();
                frames.push(frame);
            }
        }

        println!("Creating png...");

        let mut buf = Vec::new();

        png::PngEncoder::new(&mut buf)
            .write_image(packed.as_raw(), size, size, ColorType::Rgba8)
            .unwrap();

        let optimized = if optimize {
            optimize_from_memory(
                &buf,
                &Options {
                    bit_depth_reduction: true,
                    color_type_reduction: true,
                    palette_reduction: true,
                    grayscale_reduction: true,
                    strip: Headers::Safe,
                    ..Options::default()
                },
            )
            .unwrap()
        } else {
            buf
        };

        let png_texture_path = format!("{}.png", output_texture);
        println!("Writing {}...", png_texture_path);
        fs::write(&png_texture_path, optimized).unwrap();

        let webp_image = webp::Encoder::from_rgba(packed.as_raw(), size, size).encode(WEBP_QUALITY);

        let webp_texture_path = format!("{}.webp", output_texture);
        println!("Writing {}...", webp_texture_path);
        fs::write(&webp_texture_path, &*webp_image).unwrap();

        let json = if uv_spritesheet {
            serde_json::to_string(&data.to_uv_spritesheet())
        } else {
            serde_json::to_string(&data)
        }
        .unwrap();

        let data_path = format!("{}.json", output_data);
        println!("Writing {}...", data_path);
        fs::write(&data_path, json).unwrap();

        break;
    }
}

pub fn webpify(path: &str) {
    let image = Reader::open(&path).unwrap().decode().unwrap();

    let webp_image = webp::Encoder::from_rgba(image.as_bytes(), image.width(), image.height())
        .encode(WEBP_QUALITY);

    let webp_texture_path = path.replace(".png", ".webp");
    println!("Writing {}...", webp_texture_path);
    fs::write(&webp_texture_path, &*webp_image).unwrap();
}
