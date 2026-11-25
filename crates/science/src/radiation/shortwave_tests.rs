//! Tests of the shortwave, ported from `legacy-final`'s
//! `planet/shortwave.rs` with their tolerances, and the check of the
//! light's paths over a changing ground.

use super::*;
use crate::constants::{SOLAR_EFFECTIVE_TEMPERATURE, STANDARD_GRAVITY};
use crate::radiation::column::{Profile, levels};
use crate::radiation::gases::PartialPressures;
use crate::radiation::rayleigh::rayleigh_depth_550;
use crate::radiation::starlight::{SOLAR_METALLICITY, star_bands};
use crate::radiation::test_points::{EARTH_AIR, EARTH_GROUND_ALBEDO, EARTH_SURFACE_K};
use crate::surface::RELATIVE_HUMIDITY;
use crate::water::vapour_saturation_bar;

/// Definition: no Rayleigh scattering, so that the paths' check reads the
/// gases' absorption and the ground alone.
const NO_RAYLEIGH: f64 = 0.0;

#[test]
fn the_table_is_on_the_albedo_bands() {
    let table = &*SHORTWAVE;
    assert_eq!(table.bins, 147);
    assert!((table.edges_cm[0] - 2000.0).abs() < 1e-6);
    assert!(table.edges_cm[table.bins] > 14_300.0);
    assert!((table.weights.iter().sum::<f64>() - 1.0).abs() < 1e-12);
}

/// The limits of the hemispheric-mean two-stream (the reference page,
/// "The shortwave half", "In words"): a pure absorber transmits
/// `e^{-D tau}` and reflects nothing; a pure scatterer (`omega` = 1,
/// `lambda` = 0 in Meador & Weaver 1980, eqs. 25 and 26) reflects
/// `(D/2) tau / (1 + (D/2) tau)` and loses nothing. With both, some of
/// the light is lost, the solution is continuous as the absorption
/// vanishes, and a thick layer neither overflows nor transmits.
#[test]
fn a_layer_absorbs_or_scatters_as_the_two_stream_limits_say() {
    // Pure absorption: e^{-D tau}, nothing reflected.
    let (r, t) = layer(0.7, 0.0);
    assert!(r.abs() < 1e-15, "{r}");
    assert!((t - (-DIFFUSIVITY * 0.7).exp()).abs() < 1e-12, "{t}");
    // Pure scattering: (D/2) tau / (1 + (D/2) tau), and nothing lost.
    for tau in [1e-6, 0.1, 3.0, 300.0] {
        let (r, t) = layer(0.0, tau);
        let a = 0.5 * DIFFUSIVITY * tau;
        assert!((r - a / (1.0 + a)).abs() < 1e-12, "{tau}: {r}");
        assert!((r + t - 1.0).abs() < 1e-12, "{tau}: {r} {t}");
    }
    // Both: some of each lost, continuous as the absorption vanishes,
    // and a thick layer neither overflows nor transmits.
    let (r, t) = layer(0.05, 1.0);
    assert!(r + t < 1.0 && r > 0.0 && t > 0.0, "{r} {t}");
    let (r0, t0) = layer(1e-13, 1.0);
    let (r1, t1) = layer(0.0, 1.0);
    assert!((r0 - r1).abs() < 1e-9 && (t0 - t1).abs() < 1e-9);
    let (r, t) = layer(1e3, 1e3);
    assert!(r.is_finite() && t < 1e-300 && r < 1.0, "{r} {t}");
}

/// [`layer_fast`] is [`layer`] in single precision, within 1e-6 of it
/// over the depths the shortwave paths meet.
#[test]
fn the_fast_layer_is_the_layer() {
    for a in [0.0, 1e-7, 1e-4, 3e-3, 0.02, 0.5, 4.0, 80.0] {
        for sc in [0.0, 1e-6, 1e-3, 0.05, 1.0, 20.0] {
            let (r, t) = layer(a, sc);
            let (rf, tf) = layer_fast(a as f32, sc as f32);
            assert!((f64::from(rf) - r).abs() < 1e-6, "{a} {sc}: {rf} {r}");
            assert!((f64::from(tf) - t).abs() < 1e-6, "{a} {sc}: {tf} {t}");
        }
    }
}

/// The two-stream solution of a homogeneous layer is the adding of
/// its halves (the adding method of the two-stream, Meador & Weaver
/// 1980 eqs. 25 and 26 applied to each half and stacked).
#[test]
fn stacking_two_halves_gives_the_whole_layer() {
    for (a, s) in [(0.0, 0.8), (0.3, 0.0), (0.2, 0.5), (2.0, 0.1)] {
        let (r, t) = layer(a, s);
        let (rh, th) = layer(0.5 * a, 0.5 * s);
        let stack = Stack::EMPTY.over(rh, th).over(rh, th);
        assert!((stack.down - r).abs() < 1e-12, "{a} {s}");
        assert!((stack.up - r).abs() < 1e-12, "{a} {s}");
        assert!((stack.through - t).abs() < 1e-12, "{a} {s}");
    }
}

/// [`piece_depth_fast`] is [`piece_depth`] in single precision, within
/// 1e-5 of it (the legacy tolerance), over whole and cut pieces and over
/// rates that grow, stay, start at zero or fall by nine decades.
#[test]
fn the_fast_depth_is_the_depth() {
    for (from, to) in [(0.0, 1.0), (0.0, 0.4), (0.4, 1.0)] {
        let piece = Piece {
            lower: 0,
            from,
            to,
            span: 0.7 * (to - from),
            mass: 0.0,
        };
        for (a, b) in [(1.0, 0.3), (2e-3, 2e-3), (0.0, 0.5), (5.0, 1e-9)] {
            let exact = piece_depth(&piece, a, b);
            let fast = piece_depth_fast(&piece, a as f32, b as f32);
            assert!(
                (fast - exact).abs() <= 1e-5 * exact + 1e-12,
                "{a} {b}: {fast} {exact}"
            );
        }
    }
}

/// The paths of an air do not depend on the ground or the clouds under
/// it: one [`Paths`] of Earth's air at 77 % humidity gives every budget.
/// Over a white ground the ground absorbs nothing, so the light is either
/// reflected or absorbed by the air (`albedo + air = 1`, an identity of
/// the adding method); the albedo grows with the ground's and with the
/// clouds' reflectance, and the air absorbs a share of the light, with
/// or without Rayleigh scattering.
#[test]
fn paths_do_not_depend_on_the_ground() {
    let air = PartialPressures {
        h2o: RELATIVE_HUMIDITY * vapour_saturation_bar(EARTH_SURFACE_K),
        ..EARTH_AIR
    };
    let column = levels(
        &air,
        EARTH_SURFACE_K,
        STANDARD_GRAVITY.0,
        Profile::SHORTWAVE,
    )
    .expect("Earth's air makes a column");
    let bands = star_bands(SOLAR_EFFECTIVE_TEMPERATURE.0, SOLAR_METALLICITY).bands;
    for rayleigh_550 in [NO_RAYLEIGH, rayleigh_depth_550(&air, STANDARD_GRAVITY.0)] {
        let paths = Paths::through(
            &Air {
                haze: (0.0, 1.0),
                rayleigh_550,
                column: Some(&column),
            },
            &bands,
        );
        for cloud in [0.0, 0.5] {
            let mut last = f64::NEG_INFINITY;
            for ground in [0.0, EARTH_GROUND_ALBEDO, 1.0] {
                let budget = paths.budget(ground, cloud);
                assert!(
                    budget.albedo > last,
                    "{rayleigh_550} {cloud} {ground}: {budget:?}"
                );
                assert!(
                    budget.air > 0.0 && budget.air < 1.0,
                    "{rayleigh_550} {cloud} {ground}: {budget:?}"
                );
                last = budget.albedo;
            }
            let white = paths.budget(1.0, cloud);
            assert!(
                (white.albedo + white.air - 1.0).abs() < 1e-12,
                "{rayleigh_550} {cloud}: {white:?}"
            );
        }
        for ground in [0.0, EARTH_GROUND_ALBEDO, 1.0] {
            let (clear, cloudy) = (paths.budget(ground, 0.0), paths.budget(ground, 0.5));
            assert!(
                cloudy.albedo > clear.albedo,
                "{rayleigh_550} {ground}: {clear:?} {cloudy:?}"
            );
        }
    }
}
