//! `thumb.png`: a small copy of `world.png` for lists and previews.
//!
//! A full-size `world.png` decodes to 3 bytes per pixel in a browser (and
//! often 4): 100–130 MB at 8192 × 4096, for a picture shown 64 pixels wide.
//! The thumbnail is [`THUMB_WIDTH`] pixels wide (narrower maps keep their
//! width) and keeps the map's aspect ratio.
//!
//! [`Downscaler`] is an area average (box filter) in linear light, fed one
//! source row at a time: the export feeds it the rows of the image it is
//! about to write as `world.png`, and the server feeds it the rows it
//! decodes from the `world.png` of a world made before thumbnails, so both
//! give the same thumbnail without holding a second full-size image.
use super::colour::{linear_to_srgb, srgb_to_linear};
use image::RgbImage;
use std::sync::LazyLock;

/// File name of the thumbnail, next to `world.png`.
pub const THUMB_FILE: &str = "thumb.png";
/// Width of the thumbnail, in pixels.
pub const THUMB_WIDTH: u32 = 512;

/// Size of the thumbnail of a `width` × `height` map: [`THUMB_WIDTH`] wide
/// (or `width` when narrower), with the same aspect ratio, at least one
/// pixel high.
pub fn thumbnail_size(width: u32, height: u32) -> (u32, u32) {
    let w = width.min(THUMB_WIDTH);
    let h = (height as u64 * w as u64).div_ceil(width.max(1) as u64);
    (w, (h as u32).clamp(1, height.max(1)))
}

/// Linear value of each 8-bit sRGB level.
static LINEAR: LazyLock<[f32; 256]> =
    LazyLock::new(|| std::array::from_fn(|v| srgb_to_linear(v as f32)));

/// Shares of a source pixel `i` (covering `[i, i + 1)`) in the output cells
/// of size `scale`: the first cell and its weight, and the weight of the
/// next one when the pixel straddles a cell boundary (`scale` ≥ 1, so a
/// pixel overlaps two cells at most).
fn shares(i: u32, scale: f64, cells: u32) -> (usize, f32, f32) {
    let first = ((i as f64 / scale) as u32).min(cells - 1);
    let boundary = (first + 1) as f64 * scale;
    let end = (i + 1) as f64;
    if end <= boundary || first + 1 >= cells {
        (first as usize, 1.0, 0.0)
    } else {
        let next = (end - boundary) as f32;
        (first as usize, 1.0 - next, next)
    }
}

/// Adds `colour` × `weight` to `sum`.
fn add(sum: &mut [f32; 3], colour: [f32; 3], weight: f32) {
    for (s, c) in sum.iter_mut().zip(colour) {
        *s += c * weight;
    }
}

/// Area-average downscaling of an RGB image fed row by row (see the
/// module documentation). Holds the output in linear light, a few MB at
/// most, and one row of the input at a time.
pub struct Downscaler {
    width: u32,
    height: u32,
    out_width: u32,
    out_height: u32,
    /// Output column (and weights) of each input column.
    columns: Vec<(usize, f32, f32)>,
    /// Weighted sums of linear colour, per output pixel.
    sums: Vec<[f32; 3]>,
    /// One input row reduced to the output columns.
    row: Vec<[f32; 3]>,
    next_row: u32,
}

impl Downscaler {
    /// A downscaler of a `width` × `height` image to its thumbnail size
    /// ([`thumbnail_size`]).
    pub fn new(width: u32, height: u32) -> Self {
        let (out_width, out_height) = thumbnail_size(width, height);
        let scale = width as f64 / out_width as f64;
        Self {
            width,
            height,
            out_width,
            out_height,
            columns: (0..width).map(|x| shares(x, scale, out_width)).collect(),
            sums: vec![[0.0; 3]; out_width as usize * out_height as usize],
            row: vec![[0.0; 3]; out_width as usize],
            next_row: 0,
        }
    }

    /// Adds the next input row: `width` RGB pixels, 3 bytes each.
    pub fn push_row(&mut self, rgb: &[u8]) {
        assert_eq!(rgb.len(), self.width as usize * 3, "one row of RGB pixels");
        assert!(self.next_row < self.height, "more rows than the image has");
        let linear = &*LINEAR;
        self.row.fill([0.0; 3]);
        for (pixel, &(x, a, b)) in rgb.as_chunks::<3>().0.iter().zip(&self.columns) {
            let c = pixel.map(|v| linear[v as usize]);
            add(&mut self.row[x], c, a);
            if b > 0.0 {
                add(&mut self.row[x + 1], c, b);
            }
        }
        let scale = self.height as f64 / self.out_height as f64;
        let (y, a, b) = shares(self.next_row, scale, self.out_height);
        let w = self.out_width as usize;
        for (weight, y) in [(a, y), (b, y + 1)] {
            if weight == 0.0 {
                continue;
            }
            for (sum, value) in self.sums[y * w..(y + 1) * w].iter_mut().zip(&self.row) {
                add(sum, *value, weight);
            }
        }
        self.next_row += 1;
    }

    /// The thumbnail, once every row was pushed.
    pub fn finish(self) -> RgbImage {
        assert_eq!(self.next_row, self.height, "every row of the image");
        let area = (self.width as f64 / self.out_width as f64)
            * (self.height as f64 / self.out_height as f64);
        let pixels: Vec<u8> = self
            .sums
            .iter()
            .flat_map(|sum| {
                sum.map(|s| {
                    linear_to_srgb((s as f64 / area) as f32)
                        .round()
                        .clamp(0.0, 255.0) as u8
                })
            })
            .collect();
        RgbImage::from_raw(self.out_width, self.out_height, pixels).expect("one pixel per cell")
    }
}

/// The thumbnail of `image` (see [`Downscaler`]).
pub fn thumbnail(image: &RgbImage) -> RgbImage {
    let mut scaler = Downscaler::new(image.width(), image.height());
    for row in image.as_raw().chunks_exact(image.width() as usize * 3) {
        scaler.push_row(row);
    }
    scaler.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn keeps_the_aspect_ratio_and_small_maps() {
        assert_eq!(thumbnail_size(8192, 4096), (512, 256));
        assert_eq!(thumbnail_size(1920, 1080), (512, 288));
        assert_eq!(thumbnail_size(1000, 300), (512, 154));
        assert_eq!(thumbnail_size(300, 150), (300, 150));
        assert_eq!(thumbnail_size(4096, 1), (512, 1));
    }

    #[test]
    fn averages_areas_in_linear_light() {
        // Uniform colour stays itself.
        let flat = RgbImage::from_pixel(1024, 512, Rgb([40, 120, 200]));
        let thumb = thumbnail(&flat);
        assert_eq!(thumb.dimensions(), (512, 256));
        assert!(thumb.pixels().all(|p| p.0 == [40, 120, 200]));

        // A black and white checkerboard averages to mid-grey in linear
        // light (188 in sRGB), not to 128.
        let checker = RgbImage::from_fn(1024, 512, |x, y| {
            if (x + y) % 2 == 0 {
                Rgb([255; 3])
            } else {
                Rgb([0; 3])
            }
        });
        let thumb = thumbnail(&checker);
        assert!(
            thumb
                .pixels()
                .all(|p| p.0.iter().all(|&c| c.abs_diff(188) <= 1))
        );

        // Non-integer ratios: a left half white, right half black splits at
        // the middle column.
        let halves =
            RgbImage::from_fn(
                1000,
                300,
                |x, _| {
                    if x < 500 { Rgb([255; 3]) } else { Rgb([0; 3]) }
                },
            );
        let thumb = thumbnail(&halves);
        assert_eq!(thumb.get_pixel(0, 0).0, [255; 3]);
        assert_eq!(thumb.get_pixel(255, 70).0, [255; 3]);
        assert_eq!(thumb.get_pixel(256, 70).0, [0; 3]);
        assert_eq!(thumb.get_pixel(511, 153).0, [0; 3]);
    }

    #[test]
    fn a_small_map_is_its_own_thumbnail() {
        let image = RgbImage::from_fn(200, 100, |x, y| Rgb([x as u8, y as u8, (x ^ y) as u8]));
        assert_eq!(thumbnail(&image), image);
    }
}
