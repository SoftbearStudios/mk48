// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crunch::{pack, Item, Rect, Rotation};
use glam::UVec2;
use image::imageops::{replace, resize, FilterType};
use image::{codecs::png, io::Reader, ColorType, ImageEncoder, Rgba, RgbaImage};
use oxipng::{optimize_from_memory, Headers, Options};
use rayon::prelude::*;
use sprite_sheet::{Sprite, SpriteSheet};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::ErrorKind;
use std::sync::Mutex;

/// A single image file to add to pass to `pack_sprite_sheet`.
#[derive(Eq, PartialEq)]
pub struct Image {
    /// Name of the image such as "g5".
    pub name: String,
    /// Path to the image such as "../assets/models/rendered/g5.png".
    pub file: String,
    /// Width to resize the image to while maintaining its aspect ratio. If zero, the source image's
    /// size will be used.
    pub width: u32,
}

/// An sequence of images in a directory to pass to `pack_sprite_sheet`.
#[derive(Eq, PartialEq)]
pub struct Animation {
    /// Name of the animation such as "explosion".
    pub name: String,
    /// Directory that contains the animation such as "../assets/sprites/explosion". Must not have
    /// a trailing "/".
    pub dir: String,
}

/// An output image from `pack_sprite_sheet` containing colors, normals, etc.
pub struct Output<'a> {
    /// The file to output the sprite sheet to.
    pub path: &'a str,
    /// Maps `file`s of [`Image`]s e.g. "sprite.png" -> "sprite/normal.png".
    pub map_file: fn(&str) -> String,
    /// The color to use if a sprite is missing.
    pub if_missing: Option<[u8; 4]>,
    /// The color to use between sprites.
    pub padding: [u8; 4],
    /// Function to run on each pixel of the input image (and again on resized).
    pub pre_process: Option<fn([u8; 4]) -> [u8; 4]>,
    /// Function to run on each pixel of the output image.
    pub post_process: Option<fn([u8; 4]) -> [u8; 4]>,
}

impl<'a> Default for Output<'a> {
    fn default() -> Self {
        Self {
            path: "output/sprites",
            map_file: str::to_owned,
            if_missing: None,
            padding: [0, 0, 0, 4],
            pre_process: None,
            post_process: None,
        }
    }
}

struct Images {
    images: Vec<Option<RgbaImage>>,
    width: u32,
    height: u32,
}

impl Images {
    fn new(images: Vec<Option<RgbaImage>>, required: usize) -> Self {
        let dimensions = images[required]
            .as_ref()
            .expect("missing required image")
            .dimensions();
        assert!(
            images
                .iter()
                .filter_map(Option::as_ref)
                .all(|i| i.dimensions() == dimensions),
            "image size mismatch"
        );
        Self {
            images,
            width: dimensions.0,
            height: dimensions.1,
        }
    }

    fn get(&self, i: usize) -> Option<&RgbaImage> {
        self.images.get(i).and_then(|i| i.as_ref())
    }
}

/// Packs `images` and `animations` into a
/// [`SpriteSheet`]/[`UvSpriteSheet`][`sprite_sheet::UvSpriteSheet`]. At least one of the `outputs`
/// must not have an `if_missing` color. TODO support multiple outputs on animations.
pub fn pack_sprite_sheet(
    images: Vec<Image>,
    animations: Vec<Animation>,
    padding: u32,
    power_of_two: bool,
    uv_spritesheet: bool,
    optimize: bool,
    outputs: &[Output<'_>],
    output_data: &str,
) {
    let Some((required, _)) = outputs.iter().enumerate().find(|(_, t)| t.if_missing.is_none()) else {
        panic!("at least one of the outputs must not have an if_missing color");
    };

    let mut processed_images: HashMap<String, Images> = images
        .into_par_iter()
        .map(|params| {
            println!("Loading {}", params.file);

            let images = outputs
                .iter()
                .map(|output| {
                    let path = (output.map_file)(&params.file);
                    let image = Reader::open(&path)
                        .inspect_err(|e| {
                            assert!(
                                matches!(e.kind(), ErrorKind::NotADirectory | ErrorKind::NotFound),
                                "error for {path} wasn't not found: {e}"
                            )
                        })
                        .ok()
                        .map(|image| {
                            image
                                .decode()
                                .unwrap_or_else(|_| panic!("failed to decode {path}"))
                        });

                    if image.is_none() && output.if_missing.is_none() {
                        panic!("missing required output {path}");
                    }

                    image.map(|image| {
                        let mut image = image.to_rgba8();
                        if let Some(f) = output.pre_process {
                            image.pixels_mut().for_each(|pixel| {
                                pixel.0 = (f)(pixel.0);
                            });
                        }

                        if params.width == 0 {
                            image
                        } else {
                            let width = image.width();
                            let height = image.height();
                            let aspect = width as f32 / height as f32;
                            if params.width > width {
                                println!(
                                    "Upscaling {} from {} to {}",
                                    params.name, width, params.width
                                )
                            }

                            let mut image = resize(
                                &image,
                                params.width,
                                (params.width as f32 / aspect) as u32,
                                FilterType::Lanczos3,
                            );
                            if let Some(f) = output.pre_process {
                                image.pixels_mut().for_each(|pixel| {
                                    pixel.0 = (f)(pixel.0);
                                });
                            }
                            image
                        }
                    })
                })
                .collect::<Vec<_>>();

            (params.name, Images::new(images, required))
        })
        .collect();

    let images_mutex = Mutex::new(&mut processed_images);
    let animations: HashMap<_, _> = animations
        .into_par_iter()
        .map(|animation: Animation| {
            println!("Loading {}/*", animation.dir);

            let frame_set: BTreeSet<_> = fs::read_dir(&animation.dir)
                .unwrap()
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
                .par_iter()
                .map(|entry_2| {
                    let name_os = entry_2.file_name();
                    let name = name_os.to_str().unwrap();

                    let image = Reader::open(&format!("{}/{}", animation.dir, name))
                        .unwrap()
                        .decode()
                        .expect(name);

                    let short_name = shorten_name(name);

                    let image = image.into_rgba8();
                    images_mutex.lock().unwrap().insert(
                        short_name.to_owned(),
                        Images::new(vec![Some(image)], required),
                    );
                    short_name.to_owned()
                })
                .collect();
            (animation.name, frame_set)
        })
        .collect();

    // Renumber animation frames to be consecutive.
    for (animation, frames) in animations.iter() {
        for (i, frame) in frames.iter().enumerate() {
            let old_name = frame;
            let new_name = format!("{}{}", animation, i);
            let images = processed_images.remove(old_name).unwrap();
            processed_images.insert(new_name, images);
        }
    }

    // Sort images by size for better packing results.
    let sorted: BTreeMap<_, _> = processed_images.into_iter().collect();

    // Packing conserves overall area. Don't even try sizes that wouldn't fit all the images.
    let total_area: u32 = sorted.iter().map(|(_, v)| v.width * v.height).sum();
    assert!(total_area > 0, "empty sprite sheet");

    const MAX_SIZE: u32 = 4096;
    let sizes: Box<dyn Iterator<Item = UVec2>> = if power_of_two {
        // Dividing log by 2 is equivilant to integer sqrt beforehand.
        // Subtract 1 and add 1 to make it round up.
        let min_pow2 = ((total_area - 1).ilog2() + 1) / 2;

        Box::new(
            (min_pow2..=MAX_SIZE.ilog2())
                .into_iter()
                .flat_map(move |power| {
                    // Try 2x1 scale first as it could half the result's size.
                    [1u32, 0].into_iter().filter_map(move |d| {
                        let dim = UVec2::new(2u32.pow(power), 2u32.pow(power.saturating_sub(d)));
                        (dim.x * dim.y >= (1 << (min_pow2 * 2))).then_some(dim)
                    })
                }),
        )
    } else {
        // TODO could binary search instead of linear.
        let size_step = 1;
        let min_size = (total_area as f32).sqrt() as u32 / size_step;
        Box::new(
            (min_size..=(MAX_SIZE))
                .into_iter()
                .map(move |power| UVec2::splat(power * size_step)),
        )
    };

    let items: Vec<_> = sorted
        .iter()
        .map(|(key, images)| {
            let w = (images.width + padding) as usize;
            let h = (images.height + padding) as usize;
            Item::new(key.to_owned(), w, h, Rotation::None /*Allowed*/)
        })
        .collect();

    for size in sizes {
        println!("Trying {}px...", size);

        let padded_size = size + padding;
        let container = Rect::of_size(padded_size.x as usize, padded_size.y as usize);

        let packed_rects = match pack(container, items.clone()) {
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

        let mut sprites = packed_rects
            .into_iter()
            .map(|(rect, key)| {
                // Don't add padding / 2 because of padded_size.
                let x = rect.x as u32;
                let y = rect.y as u32;

                let width = rect.w as u32 - padding;
                let height = rect.h as u32 - padding;
                (
                    key.to_string(),
                    Sprite {
                        x,
                        y,
                        width,
                        height,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        // Move animation frames into animations.
        let animations = animations
            .iter()
            .map(|(animation, frames)| {
                (
                    animation.to_owned(),
                    (0..frames.len())
                        .map(|i| sprites.remove(&format!("{}{}", animation, i)).unwrap())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();

        let data = SpriteSheet {
            width: size.x,
            height: size.y,
            sprites,
            animations,
        };

        outputs.par_iter().enumerate().for_each(|(i, output)| {
            let mut packed = RgbaImage::from_pixel(size.x, size.y, Rgba(output.padding));

            for (key, sprite) in data.sprites.iter().map(|(k, v)| (k.to_owned(), v)).chain(
                data.animations.iter().flat_map(|(animation, frames)| {
                    frames
                        .iter()
                        .enumerate()
                        .map(move |(i, sprite)| (format!("{}{}", animation, i), sprite))
                }),
            ) {
                // Blit image.
                if let Some(image) = sorted[&*key].get(i) {
                    replace(&mut packed, image, sprite.x, sprite.y);
                } else if let Some(color) = output.if_missing {
                    for y in sprite.y..sprite.y + sprite.height {
                        for x in sprite.x..sprite.x + sprite.width {
                            packed.put_pixel(x, y, Rgba(color));
                        }
                    }
                }
            }

            if let Some(f) = output.post_process {
                packed.pixels_mut().for_each(|pixel| {
                    pixel.0 = (f)(pixel.0);
                });
            }

            println!("Encoding png...");

            let mut unoptimized = Vec::new();
            png::PngEncoder::new(&mut unoptimized)
                .write_image(packed.as_raw(), size.x, size.y, ColorType::Rgba8)
                .unwrap();

            let optimized = if optimize {
                optimize_from_memory(
                    &unoptimized,
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
                unoptimized
            };

            let png_output_path = format!("{}.png", output.path);
            println!("Writing {}", png_output_path);
            fs::write(&png_output_path, optimized).unwrap();
        });

        let json = if uv_spritesheet {
            serde_json::to_string(&data.to_uv_spritesheet())
        } else {
            serde_json::to_string(&data)
        }
        .unwrap();

        let data_path = format!("{}.json", output_data);
        println!("Writing {}", data_path);
        fs::write(&data_path, json).unwrap();

        return;
    }
    println!("Failed took more than {0}x{0}!", MAX_SIZE)
}

fn shorten_name(name: &str) -> &str {
    let idx = name.rfind('.').unwrap();
    &name[..idx]
}
