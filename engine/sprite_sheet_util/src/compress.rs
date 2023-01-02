use image::{GrayImage, Luma, Rgb, RgbImage, Rgba, RgbaImage};
use std::array;
use std::borrow::Cow;

/// Input to [pack_monochrome].
pub enum PackInput {
    /// Read RGB image from file.
    File(Cow<'static, str>),
    /// Raw gray image.
    Image(GrayImage),
    /// Takes x, y to produce raw gray image.
    Fn(Box<dyn Fn(u32, u32) -> u8>),
}

/// Packs up to four images into an RGBA texture. Must all have the same size. Prints color info
/// for files to stdout and panics on errors.
pub fn pack_monochrome(
    width: u32,
    height: u32,
    images: impl IntoIterator<Item = PackInput>,
    output_path: &str,
) {
    let channels = images
        .into_iter()
        .enumerate()
        .map(|(i, image)| match image {
            PackInput::File(path) => {
                let path = path.as_ref();
                let img = image::io::Reader::open(path)
                    .expect(path)
                    .with_guessed_format()
                    .expect(path)
                    .decode()
                    .expect(path);
                let rgb = img.into_rgb8();
                assert_eq!(rgb.width(), width);
                assert_eq!(rgb.height(), height);
                let mono = Monochrome::new(&rgb, i < 3);
                println!(
                    "#{i} vec3({}, {}, {}) * x + vec3({}, {}, {})",
                    mono.factor[0],
                    mono.factor[1],
                    mono.factor[2],
                    mono.term[0],
                    mono.term[1],
                    mono.term[2]
                );
                mono.image
            }
            PackInput::Image(gray) => {
                assert_eq!(gray.width(), width);
                assert_eq!(gray.height(), height);
                gray
            }
            PackInput::Fn(func) => GrayImage::from_fn(width, height, |x, y| Luma([func(x, y)])),
        })
        .collect::<Vec<_>>();

    assert!(!channels.is_empty() && channels.len() <= 4);

    /*
    for (i, channel) in channels.iter().enumerate() {
        channel.save(format!("/tmp/mono{i}.png")).unwrap();
    }
     */

    let rgba = RgbaImage::from_fn(width, height, |x, y| {
        Rgba(array::from_fn(|component| {
            channels
                .get(component)
                .map(|c| c.get_pixel(x, y).0[0])
                .unwrap_or(if component == 3 { 255 } else { 0 })
        }))
    });

    rgba.save(output_path).expect(output_path);
}

/// A monochrome image.
pub struct Monochrome {
    /// The inputs to the function described by term and factor.
    pub image: GrayImage,
    /// The y-intercept color values.
    pub term: [f32; 3],
    /// The slope color values.
    pub factor: [f32; 3],
}

impl Monochrome {
    /// Encodes an RGB image as a monochrome image.
    pub fn new(image: &RgbImage, srgb_output: bool) -> Self {
        fn to_float(p: [u8; 3]) -> [f32; 3] {
            srgb::gamma::linear_from_u8(p)
        }

        fn get_luma(p: [f32; 3]) -> f32 {
            p.into_iter()
                .zip([0.2126f32, 0.7152f32, 0.0722f32])
                .map(|(c, l)| c * l)
                .sum::<f32>()
        }

        let regressions: [(f32, f32); 3] = array::from_fn(|c| {
            let data = image
                .pixels()
                .map(|p| {
                    let floats = to_float(p.0);
                    (get_luma(floats), floats[c])
                })
                .collect::<Vec<_>>();

            linreg::linear_regression_of(&data).unwrap()
        });

        Self {
            image: GrayImage::from_fn(image.width(), image.height(), |x, y| {
                let linear = get_luma(to_float(image.get_pixel(x, y).0));
                Luma([if srgb_output {
                    srgb::gamma::compress_u8(linear)
                } else {
                    (linear * 255.0) as u8
                }])
            }),
            term: regressions.map(|(_, t)| t as f32),
            factor: regressions.map(|(f, _)| f as f32),
        }
    }

    /// Decodes a monochrome image to an RGB image
    pub fn decode(&self) -> RgbImage {
        RgbImage::from_fn(self.image.width(), self.image.height(), |x, y| {
            let luma = srgb::gamma::expand_u8(self.image.get_pixel(x, y).0[0]);
            Rgb(self
                .term
                .zip(self.factor)
                .map(|(term, factor)| srgb::gamma::compress_u8(term + factor * luma)))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::compress::Monochrome;
    use std::io::Cursor;

    #[test]
    fn monochrome() {
        let img = image::io::Reader::new(Cursor::new(include_bytes!("test_image.png")))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        let rgb = img.into_rgb8();
        let mono = Monochrome::new(&rgb);
        println!("term={:?} factor={:?}", mono.term, mono.factor);
        let decoded = mono.decode();
        decoded.save("test_image_monochrome.png").unwrap();
    }
}
