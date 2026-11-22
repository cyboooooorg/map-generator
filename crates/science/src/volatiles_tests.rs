use super::*;
use crate::constants::MOLAR_GAS_CONSTANT;

/// Definition: relative tolerance of the CO2 vapour pressure against the
/// NIST WebBook table, 0.1 %.
const CO2_TOLERANCE: f64 = 1e-3;
/// Definition: relative tolerance of the methane vapour pressure against
/// the NIST WebBook table, 0.5 %.
const METHANE_TOLERANCE: f64 = 5e-3;
/// Definition: relative tolerance of the CO2 latent heat and of its normal
/// sublimation check, 2 %.
const LATENT_TOLERANCE: f64 = 2e-2;
/// Definition: relative tolerance of the sublimation pressure at 1 atm,
/// 1 %, about 0.1 K of the sublimation temperature.
const SUBLIMATION_TOLERANCE: f64 = 1e-2;

/// Source: NIST Chemistry WebBook, fluid properties of carbon dioxide,
/// saturation table (Span and Wagner 1996, doi:10.1063/1.555991),
/// (T in K, p in bar), https://webbook.nist.gov/cgi/fluid.cgi?ID=C124389&Type=SatP
/// read on 2026-11-22.
const CO2_TABLE: [(f64, f64); 4] = [
    (220.0, 5.9913),
    (250.0, 17.850),
    (273.15, 34.851),
    (300.0, 67.131),
];
/// Source: NIST Chemistry WebBook, fluid properties of carbon dioxide,
/// saturation at 250 K: liquid enthalpy 6.5007 kJ/mol, vapour enthalpy
/// 19.234 kJ/mol, liquid volume 0.042075 l/mol, vapour volume
/// 0.94353 l/mol (Span and Wagner 1996), read on 2026-11-22.
const CO2_250_K: (f64, f64, f64, f64) = (6.5007e3, 19.234e3, 0.042_075e-3, 0.943_53e-3);
/// Source: normal sublimation temperature of CO2, 194.6855 K at 1 atm,
/// the value the literature gives (Wikipedia, "Carbon dioxide"; the NIST
/// Span-Wagner correlation gives 194.673 K, NIST IR 8608, read in a
/// search result only), 0.01 K apart.
const CO2_SUBLIMATION_1_ATM_K: f64 = 194.6855;
/// Source: NIST Chemistry WebBook, fluid properties of methane, saturation
/// table (Setzmann and Wagner 1991, doi:10.1063/1.555898), (T in K, p in
/// bar), read on 2026-11-22.
const METHANE_TABLE: [(f64, f64); 4] = [
    (100.0, 0.34376),
    (130.0, 3.6732),
    (150.0, 10.400),
    (180.0, 32.852),
];

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    ((a - b) / b).abs() <= tolerance
}

#[test]
fn co2_follows_span_wagner() {
    for (t, bar) in CO2_TABLE {
        let p = co2_saturation_pressure(Kelvin(t)).0 / PASCALS_PER_BAR;
        assert!(close(p, bar, CO2_TOLERANCE), "{t} K: {p} against {bar}");
    }
}

#[test]
fn co2_sublimes_at_one_atmosphere() {
    let p = co2_saturation_pressure(Kelvin(CO2_SUBLIMATION_1_ATM_K)).0;
    assert!(close(p, 101_325.0, SUBLIMATION_TOLERANCE), "{p} Pa");
    // Colder, less; the curve joins the vapour curve at the triple point.
    let below = co2_saturation_bar(CO2_TRIPLE_TEMPERATURE.next_down());
    let triple = co2_saturation_bar(CO2_TRIPLE_TEMPERATURE);
    assert!(close(below, triple, 1e-9), "{below} against {triple}");
    assert!(co2_saturation_bar(150.0) < co2_saturation_bar(190.0));
}

#[test]
fn co2_latent_heat_is_the_curves_slope() {
    // Clapeyron, dp/dT = L / (T dV), with the table's real enthalpy and
    // volume of vaporisation: the curve's slope must agree.
    let (hl, hv, vl, vv) = CO2_250_K;
    let clapeyron = (hv - hl) / (250.0 * (vv - vl));
    let slope = co2_saturation_pressure(Kelvin(250.0)).0 * co2_latent_j_mol(250.0)
        / (MOLAR_GAS_CONSTANT * 250.0 * 250.0);
    assert!(
        close(slope, clapeyron, LATENT_TOLERANCE),
        "{slope} against {clapeyron}"
    );
    // The latent heat is that of the sublimation curve below the triple
    // point and zero past the critical point.
    assert_eq!(co2_latent_j_mol(150.0), CO2_SUBLIMATION_ENTHALPY);
    assert_eq!(co2_latent_j_mol(310.0), 0.0);
}

#[test]
fn co2_caps_at_its_triple_and_critical_points() {
    let triple = co2_saturation_bar(CO2_TRIPLE_TEMPERATURE);
    assert!(close(triple, 5.1796, 1e-4), "{triple}");
    let critical = co2_saturation_bar(CO2_CRITICAL_TEMPERATURE.next_down());
    assert!(close(critical, 73.773, 1e-4), "{critical}");
    for t in [CO2_CRITICAL_TEMPERATURE, 305.0, 1500.0] {
        assert_eq!(co2_saturation_bar(t), f64::INFINITY, "{t} K");
        assert_eq!(co2_latent_j_mol(t), 0.0, "{t} K");
    }
    for t in [1.0, 0.0, -5.0, f64::NAN] {
        let p = co2_saturation_bar(t);
        assert!(p.is_finite() && p >= 0.0, "{t} K: {p}");
        assert!(co2_latent_j_mol(t).is_finite(), "{t} K");
    }
}

#[test]
fn methane_follows_nist() {
    for (t, bar) in METHANE_TABLE {
        let p = methane_saturation_pressure(Kelvin(t)).0 / PASCALS_PER_BAR;
        assert!(close(p, bar, METHANE_TOLERANCE), "{t} K: {p} against {bar}");
    }
}

#[test]
fn methane_caps_and_freezes() {
    let triple = methane_saturation_bar(METHANE_TRIPLE_TEMPERATURE);
    let below = methane_saturation_bar(METHANE_TRIPLE_TEMPERATURE.next_down());
    assert!(close(below, triple, 1e-9), "{below} against {triple}");
    assert!(close(triple, 0.116_96, 1e-3), "{triple}");
    assert!(methane_saturation_bar(60.0) < methane_saturation_bar(80.0));
    for t in [190.564, 200.0, 1500.0] {
        assert_eq!(methane_saturation_bar(t), f64::INFINITY, "{t} K");
    }
    for t in [1.0, 0.0, -5.0, f64::NAN] {
        let p = methane_saturation_bar(t);
        assert!(p.is_finite() && p >= 0.0, "{t} K: {p}");
    }
}
