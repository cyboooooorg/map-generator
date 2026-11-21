//! A raster as an 8-bit RGB PNG.

use std::path::Path;

use world::Raster;

use crate::store::write_atomic;

/// A colour ramp: stops sorted by value, linear between them, clamped at
/// the ends.
#[derive(Clone, Debug)]
pub struct Ramp {
    stops: Vec<(f32, [u8; 3])>,
}

impl Ramp {
    /// A ramp through `stops`.
    ///
    /// # Panics
    ///
    /// Panics unless `stops` is non-empty and sorted by value.
    pub fn new(stops: Vec<(f32, [u8; 3])>) -> Ramp {
        assert!(!stops.is_empty(), "a ramp needs at least one stop");
        assert!(
            stops.windows(2).all(|pair| pair[0].0 <= pair[1].0),
            "the stops of a ramp must be sorted by value"
        );
        Ramp { stops }
    }

    /// Black at `min`, white at `max`.
    pub fn grey(min: f32, max: f32) -> Ramp {
        Ramp::new(vec![(min, [0, 0, 0]), (max, [255, 255, 255])])
    }

    /// The colour of `v`; NaN is black.
    pub fn colour(&self, v: f32) -> [u8; 3] {
        if v.is_nan() {
            return [0, 0, 0];
        }
        let (first_value, first_colour) = self.stops[0];
        if v <= first_value {
            return first_colour;
        }
        for pair in self.stops.windows(2) {
            let ((v0, c0), (v1, c1)) = (pair[0], pair[1]);
            if v <= v1 {
                let t = (v - v0) / (v1 - v0);
                return std::array::from_fn(|i| {
                    let a = f32::from(c0[i]);
                    let b = f32::from(c1[i]);
                    (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
                });
            }
        }
        self.stops[self.stops.len() - 1].1
    }
}

/// The raster as a PNG file (8-bit RGB), coloured through `ramp`.
///
/// # Panics
///
/// When the raster holds other than `width * height` values, or when its
/// size is zero or beyond what PNG can hold: neither comes out of
/// `world::project`.
pub fn encode_png(raster: &Raster, ramp: &Ramp) -> Vec<u8> {
    let data: Vec<u8> = raster.values.iter().flat_map(|&v| ramp.colour(v)).collect();
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, raster.width, raster.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .expect("a header into memory cannot fail");
    writer
        .write_image_data(&data)
        .expect("the raster holds width * height values");
    writer.finish().expect("a PNG into memory cannot fail");
    out
}

/// Writes the PNG of `raster` to `path`, staged then renamed.
pub fn write_png(path: &Path, raster: &Raster, ramp: &Ramp) -> std::io::Result<()> {
    write_atomic(path, &encode_png(raster, ramp))
}
