//! Tests of the star's bands: the table of spectra, its interpolation,
//! and the Sun it gives against the measured one.

use super::*;
use crate::constants::SECOND_RADIATION_CONSTANT_CM_K;

/// Source: IAU 2015 Resolution B3, nominal solar effective temperature
/// (the value of `SOLAR_EFFECTIVE_TEMPERATURE`).
const SUN_K: f64 = 5772.0;

/// Definition: test temperature of an M dwarf, K.
const M_DWARF_K: f64 = 3300.0;

/// Definition: test temperature of a hot star, K.
const HOT_K: f64 = 20_000.0;

/// Definition: wavelength past which light counts as infrared here, m.
const INFRARED_M: f64 = 0.7e-6;

/// Definition: the temperatures the table covers, K: PHOENIX's coolest
/// model and ATLAS9's hottest row kept (see
/// `crates/science/data/SPECTRA.md`).
const TABLE_K: (f64, f64) = (2300.0, 27_000.0);

/// Definition: the metallicities the table covers, dex: the stellar
/// tables' (see `crates/science/data/SPECTRA.md`).
const TABLE_DEX: (f64, f64) = (-1.0, 0.5);

/// Definition: the cut of the near ultraviolet and of its shorter part,
/// m: the shares below 400 and 300 nm of issue #655 (a band counts when
/// its centre lies below the cut).
const CUTS_M: [f64; 2] = [0.4e-6, 0.3e-6];

/// Calibration: the share of the black body's excess over the measured Sun
/// below 400 nm and below 300 nm that the table's Sun must remove, 0.80:
/// what the PHOENIX Sun removes, measured by study 35 ((12.31 − 8.92) /
/// (12.31 − 8.19) = 0.82 and (2.99 − 1.37) / (2.99 − 1.03) = 0.83),
/// rounded down; decided by the user on 2026-11-25. No source gives a
/// model library's error against the measured Sun in this range.
const GAP_CLOSED: f64 = 0.80;

/// Definition: the tolerance of each row's sum, 1e-4: the table writes
/// five significant digits, so each share is off by at most 5e-6 of
/// itself, and the 240 shares of a row sum within 5e-6 of 1 (measured:
/// 4.8e-6 at most).
const SUM_TOLERANCE: f64 = 1e-4;

/// Definition: metres per centimetre, a unit conversion.
const M_PER_CM: f64 = 1e-2;

/// The share of `bands` whose centre lies below `cut_m`.
fn share_below(bands: &[(f64, f64)], cut_m: f64) -> f64 {
    bands
        .iter()
        .enumerate()
        .filter(|(i, _)| (band_edge_m(*i) * band_edge_m(i + 1)).sqrt() < cut_m)
        .map(|(_, b)| b.1)
        .sum()
}

/// A black body at `star_k` on the bands, `B_lambda` at each band's centre
/// times its width (the bands the legacy and the first port used).
fn black_body(star_k: f64) -> Vec<(f64, f64)> {
    let c2 = SECOND_RADIATION_CONSTANT_CM_K * M_PER_CM;
    let raw: Vec<f64> = (0..BANDS)
        .map(|i| {
            let lambda = (band_edge_m(i) * band_edge_m(i + 1)).sqrt();
            lambda.powi(-4) / ((c2 / (lambda * star_k)).exp() - 1.0)
        })
        .collect();
    let total: f64 = raw.iter().sum();
    raw.iter().map(|w| (0.0, w / total)).collect()
}

#[test]
fn the_table_holds_every_metallicity_at_every_temperature() {
    let s = &*SPECTRA;
    assert_eq!(s.metallicities, vec![-1.0, -0.5, 0.0, 0.5]);
    assert_eq!(s.temperatures.first(), Some(&TABLE_K.0));
    assert_eq!(s.temperatures.last(), Some(&TABLE_K.1));
    assert!(s.temperatures.windows(2).all(|w| w[0] < w[1]));
    for (m, row) in s.metallicities.iter().zip(&s.shares) {
        assert_eq!(row.len(), s.temperatures.len());
        for (t, shares) in s.temperatures.iter().zip(row) {
            let sum: f64 = shares.iter().sum();
            assert!((sum - 1.0).abs() < SUM_TOLERANCE, "{m} dex, {t} K: {sum}");
            assert!(shares.iter().all(|x| x.is_finite() && *x >= 0.0));
        }
    }
}

#[test]
fn the_bands_carry_all_the_light() {
    for k in [TABLE_K.0, M_DWARF_K, SUN_K, HOT_K, TABLE_K.1] {
        for dex in [TABLE_DEX.0, -0.25, SOLAR_METALLICITY, TABLE_DEX.1] {
            let s = star_bands(k, dex);
            assert_eq!(s.bands.len(), BANDS);
            let sum: f64 = s.bands.iter().map(|b| b.1).sum();
            assert!((sum - 1.0).abs() < 1e-12, "{k} K, {dex}: {sum}");
            assert!(
                s.bands
                    .iter()
                    .all(|b| b.0.is_finite() && b.1.is_finite() && b.0 >= 0.0 && b.1 >= 0.0)
            );
        }
    }
}

#[test]
fn a_cold_or_unknown_star_is_the_sun() {
    let sun = star_bands(SUN_K, SOLAR_METALLICITY);
    assert_eq!(sun.star_k, SUN_K);
    for k in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(star_bands(k, SOLAR_METALLICITY), sun, "{k}");
    }
    for dex in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(star_bands(SUN_K, dex), sun, "{dex}");
    }
}

#[test]
fn a_star_outside_the_table_takes_its_edge() {
    let bands = |k: f64, dex: f64| star_bands(k, dex).bands;
    assert_eq!(bands(1000.0, 0.0), bands(TABLE_K.0, 0.0));
    assert_eq!(bands(60_000.0, 0.0), bands(TABLE_K.1, 0.0));
    assert_eq!(bands(SUN_K, -3.0), bands(SUN_K, TABLE_DEX.0));
    assert_eq!(bands(SUN_K, 2.0), bands(SUN_K, TABLE_DEX.1));
}

#[test]
fn a_grid_point_is_its_row() {
    let s = &*SPECTRA;
    for (mi, m) in s.metallicities.iter().enumerate() {
        for (ti, t) in s.temperatures.iter().enumerate() {
            let row = &s.shares[mi][ti];
            let total: f64 = row.iter().sum();
            let bands = star_bands(*t, *m).bands;
            for (b, x) in bands.iter().zip(row) {
                assert!((b.1 - x / total).abs() < 1e-15, "{m} dex, {t} K");
            }
        }
    }
}

#[test]
fn a_red_star_puts_its_light_in_the_infrared() {
    let share =
        |k: f64| -> f64 { 1.0 - share_below(&star_bands(k, SOLAR_METALLICITY).bands, INFRARED_M) };
    assert!(share(M_DWARF_K) > share(SUN_K));
    assert!(share(SUN_K) > share(HOT_K));
}

#[test]
#[should_panic(expected = "the table's bands are not the shortwave's")]
fn a_table_on_other_bands_is_refused() {
    check_layout(&["bands", "240", "2e-7", "4e-6"], 0);
}

#[test]
fn the_suns_ultraviolet_closes_the_black_bodys_gap() {
    // The table's Sun against the measured Sun and the black body of its
    // temperature, the light below 400 and 300 nm (study 35).
    let sun = star_bands(SUN_K, SOLAR_METALLICITY).bands;
    let body = black_body(SUN_K);
    let tsis = measured_sun();
    for cut in CUTS_M {
        let (model, black) = (share_below(&sun, cut), share_below(&body, cut));
        let measured = share_below(&tsis, cut);
        let closed = (black - model) / (black - measured);
        println!(
            "below {:.0} nm: black body {black:.4}, table {model:.4}, TSIS-1 {measured:.4}: \
             {closed:.3} of the gap",
            cut * 1e9
        );
        assert!(closed >= GAP_CLOSED, "{cut}: {closed}");
    }
}
