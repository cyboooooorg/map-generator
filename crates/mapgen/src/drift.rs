//! Low-resolution previews of a world at another epoch of its plates, for
//! the drift slider (backlog item 9, step 3).
//!
//! A preview runs the whole pipeline (relief, climate, rivers, biomes) on a
//! map a few hundred pixels wide, with the world's own parameters but the
//! epoch asked for, and renders it like the base map. It costs a fraction
//! of a second, where the world at full size takes seconds to minutes; the
//! viewer asks for one per stop of its slider and, once an epoch is chosen,
//! generates the full world with the same epoch (`drift_ma` in the request).
//!
//! Previews are as deterministic as worlds: the seed and the epoch give
//! the same pixels. They are not the downscaled full world: the pipeline
//! runs at the preview's own resolution, so rivers and small coasts differ.
use crate::export::{MapLayers, Style, render_pixels};
use crate::generation::generate;
use crate::geology::Regime;
use crate::plates::DRIFT_LIMIT_MA;
use crate::world::WorldParams;

/// The widths a preview can have: a coarse one while the slider moves and a
/// finer one when it rests. A world narrower than the width asked for
/// keeps its own.
pub const PREVIEW_WIDTHS: [u32; 2] = [256, 512];

/// Why a world has no drift preview.
#[derive(Debug, PartialEq, Eq)]
pub enum PreviewError {
    /// The world has no plates: giants, and presets or worlds without plate
    /// tectonics (see [`crate::geology::regime_of`]).
    NoPlates,
    /// The epoch is outside `[-DRIFT_LIMIT_MA, DRIFT_LIMIT_MA]`.
    OutOfRange,
    /// The width is not one of [`PREVIEW_WIDTHS`].
    BadWidth,
}

impl std::fmt::Display for PreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoPlates => "this world has no plate tectonics: its plates do not drift",
            Self::OutOfRange => "the epoch is outside the drift's range",
            Self::BadWidth => "width must be 256 or 512",
        })
    }
}

/// Whether the plates of the world `p` can drift: a rocky world with plate
/// tectonics.
pub fn has_plates(p: &WorldParams) -> bool {
    p.giant_mass_earth.is_none() && p.planet.geology.regime == Regime::Plates
}

/// A rendered preview: `width × height` rows of RGB.
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 3]>,
}

/// The parameters of the preview of `params` at `drift_ma` (Ma from the
/// present, negative in the past), `width` pixels wide (one of
/// [`PREVIEW_WIDTHS`], capped at the world's own width), at the aspect
/// ratio of the world.
pub fn preview_params(
    params: &WorldParams,
    drift_ma: f32,
    width: u32,
) -> Result<WorldParams, PreviewError> {
    if !PREVIEW_WIDTHS.contains(&width) {
        return Err(PreviewError::BadWidth);
    }
    if !drift_ma.is_finite() || drift_ma.abs() > DRIFT_LIMIT_MA as f32 {
        return Err(PreviewError::OutOfRange);
    }
    if !has_plates(params) {
        return Err(PreviewError::NoPlates);
    }
    let mut p = params.clone();
    p.width = width.min(params.width).max(2);
    let source = u64::from(params.width.max(1));
    p.height =
        ((u64::from(params.height) * u64::from(p.width) + source / 2) / source).max(1) as u32;
    p.drift_ma = drift_ma;
    Ok(p)
}

/// Generates and renders the preview of `params` at `drift_ma`.
pub fn preview(params: &WorldParams, drift_ma: f32, width: u32) -> Result<Preview, PreviewError> {
    let p = preview_params(params, drift_ma, width)?;
    let world = generate(&p).world;
    let pixels = render_pixels(&world, &MapLayers::new(&world), Style::Detailed);
    Ok(Preview {
        width: p.width,
        height: p.height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moons::MoonsMode;
    use crate::request::WorldRequest;

    fn params(planet: &str) -> WorldParams {
        WorldRequest {
            planet: Some(planet.into()),
            seed: Some(11),
            sea_level: Some(0.3),
            width: Some(1024),
            height: Some(512),
            moons: Some(MoonsMode::None),
            ..Default::default()
        }
        .resolve(&mut rand::rng())
        .unwrap()
        .params
    }

    #[test]
    fn sizes_follow_the_world_and_the_widths_offered() {
        let p = params("terran");
        let q = preview_params(&p, -100.0, 256).unwrap();
        assert_eq!((q.width, q.height, q.drift_ma), (256, 128, -100.0));
        // A world narrower than the preview keeps its width.
        let mut small = p.clone();
        small.width = 200;
        small.height = 100;
        assert_eq!(preview_params(&small, 0.0, 512).unwrap().width, 200);
        assert_eq!(
            preview_params(&p, 0.0, 300).err(),
            Some(PreviewError::BadWidth)
        );
        assert_eq!(
            preview_params(&p, 301.0, 256).err(),
            Some(PreviewError::OutOfRange)
        );
        assert_eq!(
            preview_params(&p, f32::NAN, 256).err(),
            Some(PreviewError::OutOfRange)
        );
    }

    #[test]
    fn worlds_without_plates_have_no_preview() {
        for planet in ["barren", "volcanic"] {
            let p = params(planet);
            assert!(!has_plates(&p), "{planet}");
            assert_eq!(
                preview_params(&p, -50.0, 256).err(),
                Some(PreviewError::NoPlates)
            );
        }
    }

    #[test]
    fn a_preview_is_deterministic_and_moves_with_the_epoch() {
        let p = params("terran");
        assert!(has_plates(&p));
        let a = preview(&p, -150.0, 256).unwrap();
        let b = preview(&p, -150.0, 256).unwrap();
        assert_eq!(a.pixels, b.pixels);
        assert_eq!(a.pixels.len(), (a.width * a.height) as usize);
        let present = preview(&p, 0.0, 256).unwrap();
        assert_ne!(a.pixels, present.pixels);
        // The present preview is the world at the preview's size, drift 0.
        let q = preview_params(&p, 0.0, 256).unwrap();
        let world = generate(&q).world;
        let direct = render_pixels(&world, &MapLayers::new(&world), Style::Detailed);
        assert_eq!(present.pixels, direct);
    }
}
