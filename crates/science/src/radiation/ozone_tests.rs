//! Tests of ozone's column and cross-sections against their sources:
//! Serdyuchenko et al. 2014's measured peaks, Rugheimer et al. 2013's and
//! Segura et al. 2005's columns.

use super::*;
use crate::radiation::starlight::band_edge_m;

/// Source: the Hartley band's peak at 223 K, 1.165e-17 cm² at 255.3 nm
/// (Serdyuchenko et al. 2014, data file `serdyuchenkogorshelev5digits.dat`,
/// as `docs/science/ozone.md`, "What ozone absorbs", reads it).
const HARTLEY_PEAK: (f64, f64) = (255.3e-9, 1.165e-17);

/// Source: the Chappuis band's peak at 223 K, 5.16e-21 cm² at 602.5 nm
/// (same file and page).
const CHAPPUIS_PEAK: (f64, f64) = (602.5e-9, 5.16e-21);

/// Source: the Hartley band's peak at 233 K, 1.165e-17 cm² at 255.45 nm,
/// and the Chappuis band's, 5.153e-21 cm² at 602.4 nm (same file and
/// page).
const PEAKS_233_K: [(f64, f64); 2] = [(255.45e-9, 1.165e-17), (602.4e-9, 5.153e-21)];

/// Source: how far the Hartley maximum and the Chappuis band move from 193
/// to 293 K, under 1 % (Serdyuchenko et al. 2014, p. 626: 0.5 to 1 % for
/// the Hartley maximum, under 1 % for the Chappuis band).
const PEAK_SHIFT_193_293_K: f64 = 0.01;

/// Definition: how far a band's highest g-point may lie from the peak the
/// band holds, relative: the source's rounding to four digits, 1e-3.
const PEAK_TOLERANCE: f64 = 1e-3;

/// Source: Rugheimer et al. 2013, Table 2: the column, cm⁻², of an
/// Earth-like planet at present O₂ around each grid star (effective
/// temperature, K) and around the Sun (5.3e18).
const RUGHEIMER_TABLE_2: [(f64, f64); 12] = [
    (7000.0, 1.2e19),
    (6750.0, 1.1e19),
    (6500.0, 9.6e18),
    (6250.0, 8.3e18),
    (6000.0, 7.3e18),
    (5750.0, 5.1e18),
    (5500.0, 3.2e18),
    (5250.0, 4.1e18),
    (5000.0, 3.3e18),
    (4750.0, 2.6e18),
    (4500.0, 2.6e18),
    (4250.0, 3.5e18),
];

/// Source: Rugheimer et al. 2013, Table 2, the Sun's column, cm⁻².
const RUGHEIMER_SUN: f64 = 5.3e18;

/// Source: Segura et al. 2005, Table 2: the column, cm⁻², around the
/// quiescent model M dwarfs (effective temperature, K) and the Sun
/// (8.4e18).
const SEGURA_TABLE_2: [(f64, f64); 3] = [(3100.0, 1.2e18), (3400.0, 2.4e18), (3650.0, 3.2e18)];

/// Source: Segura et al. 2005, Table 2, the Sun's column, cm⁻².
const SEGURA_SUN: f64 = 8.4e18;

/// Earth's O₂ (`balance::EARTH_DRY_AIR`, 0.21 bar).
const EARTH_O2: Bar = Bar(crate::balance::EARTH_DRY_AIR.o2);

/// Definition: the tolerance of an identity.
const EXACT: f64 = 1e-9;

/// Source: one Dobson unit, 2.687e16 molecules per cm² (Cooke et al. 2022,
/// §3.1).
const COOKE_DOBSON_UNIT: f64 = 2.687e16;

/// The band (bluest first) that holds `lambda_m`.
fn band_of(lambda_m: f64) -> usize {
    (0..BANDS)
        .find(|&b| band_edge_m(b) <= lambda_m && lambda_m < band_edge_m(b + 1))
        .expect("a wavelength inside the bands")
}

#[test]
fn the_bands_keep_serdyuchenkos_peaks() {
    let table = &*CROSS_SECTIONS;
    assert_eq!(table.temperatures, [223.0, 233.0]);
    assert!((table.weights.iter().sum::<f64>() - 1.0).abs() < EXACT);
    let check = |t_k: f64, (lambda_m, peak): (f64, f64)| {
        let band = &table.at(t_k)[band_of(lambda_m)];
        let top = band[G - 1];
        println!("{t_k} K, {lambda_m:e} m: {top:e} cm² against {peak:e}");
        assert!((top / peak - 1.0).abs() <= PEAK_TOLERANCE, "{band:?}");
        // The g-points are sorted, and none passes the peak.
        assert!(band.windows(2).all(|w| w[0] <= w[1]), "{band:?}");
    };
    check(OZONE_LAYER_K, HARTLEY_PEAK);
    check(OZONE_LAYER_K, CHAPPUIS_PEAK);
    for peak in PEAKS_233_K {
        check(233.0, peak);
    }
    // From 223 to 233 K the peak bands move by less than they do from 193
    // to 293 K.
    for (lambda_m, _) in [HARTLEY_PEAK, CHAPPUIS_PEAK] {
        let b = band_of(lambda_m);
        let (cold, warm) = (table.at(OZONE_LAYER_K)[b], table.at(233.0)[b]);
        for (c, w) in cold.iter().zip(&warm) {
            assert!(
                (w / c - 1.0).abs() < PEAK_SHIFT_193_293_K,
                "{cold:?} {warm:?}"
            );
        }
    }
    // No band holds more than the Hartley peak, and nothing past 1.1 µm.
    let most = table
        .at(OZONE_LAYER_K)
        .iter()
        .flatten()
        .copied()
        .fold(0.0, f64::max);
    assert!(most <= HARTLEY_PEAK.1 * (1.0 + PEAK_TOLERANCE), "{most}");
    assert!(
        table.at(OZONE_LAYER_K)[band_of(1.2e-6)..]
            .iter()
            .flatten()
            .all(|s| *s == 0.0)
    );
}

#[test]
fn a_dobson_unit_is_cookes() {
    let per_du = molecules_per_cm2(DobsonUnit(1.0));
    assert!((per_du / COOKE_DOBSON_UNIT - 1.0).abs() < 1e-3, "{per_du}");
}

#[test]
fn the_column_follows_rugheimers_and_seguras_ratios() {
    let column = |t: f64| ozone_column(EARTH_O2, Kelvin(t)).0;
    // The Sun gives Earth's 300 DU.
    assert!((column(SOLAR_EFFECTIVE_TEMPERATURE.0) - 300.0).abs() < EXACT);
    for (t, c) in RUGHEIMER_TABLE_2 {
        let expected = 300.0 * c / RUGHEIMER_SUN;
        assert!((column(t) - expected).abs() < EXACT, "{t} K: {}", column(t));
    }
    for (t, c) in SEGURA_TABLE_2 {
        let expected = 300.0 * c / SEGURA_SUN;
        assert!((column(t) - expected).abs() < EXACT, "{t} K: {}", column(t));
    }
    // Held past the ends of the grid.
    assert_eq!(column(2300.0), column(3100.0));
    assert_eq!(column(27_000.0), column(7000.0));
    // Linear between two grid points.
    let mid = 0.5 * (column(6000.0) + column(6250.0));
    assert!((column(6125.0) - mid).abs() < EXACT);
}

#[test]
fn an_air_without_oxygen_holds_no_ozone() {
    for o2 in [0.0, -1e-12, f64::NAN] {
        assert_eq!(ozone_column(Bar(o2), SOLAR_EFFECTIVE_TEMPERATURE).0, 0.0);
    }
    // Any O₂ holds a modern air's column.
    assert_eq!(
        ozone_column(Bar(1e-6), SOLAR_EFFECTIVE_TEMPERATURE),
        EARTH_OZONE_COLUMN
    );
    // A star of no temperature is the Sun.
    for t in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(ozone_column(EARTH_O2, Kelvin(t)), EARTH_OZONE_COLUMN);
    }
}
