//! Colour-space helpers shared by the renderers.
//!
//! Colours are stored and written as sRGB in `[0, 255]`, but light adds up
//! in linear space: every mix (ecotones, liquids, river edges, variation)
//! converts to linear light, mixes, then converts back once.

/// sRGB channel in `[0, 255]` to linear light in `[0, 1]`.
pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    let c = (c / 255.0).clamp(0.0, 1.0);
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light in `[0, 1]` to an sRGB channel in `[0, 255]`.
pub(crate) fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    s * 255.0
}

/// An sRGB colour in `[0, 255]` to linear light.
pub(crate) fn to_linear(c: [f32; 3]) -> [f32; 3] {
    c.map(srgb_to_linear)
}

/// A colour in linear light to sRGB in `[0, 255]`.
pub(crate) fn to_srgb(c: [f32; 3]) -> [f32; 3] {
    c.map(linear_to_srgb)
}

/// An 8-bit sRGB colour to linear light.
pub(crate) fn linear_of(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| srgb_to_linear(v as f32))
}

/// `base` moved towards `target` by `share`, all in linear light.
pub(crate) fn mix(base: [f32; 3], target: [f32; 3], share: f32) -> [f32; 3] {
    std::array::from_fn(|k| base[k] + (target[k] - base[k]) * share)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_round_trips_and_blends_brighter_than_srgb() {
        for c in [0.0, 1.0, 10.0, 128.0, 254.0, 255.0] {
            assert!((linear_to_srgb(srgb_to_linear(c)) - c).abs() < 0.01, "{c}");
        }
        // Half black, half white is about 188 in sRGB, not 128.
        let mid = linear_to_srgb((srgb_to_linear(0.0) + srgb_to_linear(255.0)) / 2.0);
        assert!((mid - 188.0).abs() < 1.0, "{mid}");
    }
}
