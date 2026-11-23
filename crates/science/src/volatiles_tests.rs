use super::*;
use crate::constants::{EARTH_MASS, MOLAR_GAS_CONSTANT};
use crate::units::Kilogram;

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
/// Definition: relative tolerance where the CO2 curves meet their end
/// points, 0.01 %: the vapour ancillary and the sublimation equation are
/// separate fits that meet at 0.51795 MPa within 0.004 %, and the vapour
/// curve reaches the critical pressure.
const JOIN_TOLERANCE: f64 = 1e-4;

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
/// Definition: normal sublimation temperature of CO2 the test reads, 194.6855 K
/// at 1 atm, checked at 1 % of the pressure (about 0.1 K). No primary source
/// was reachable: the value is the one a secondary page gives as Span and
/// Wagner 1996's (doi:10.1063/1.555991, paywalled, not read), and NIST IR 8608,
/// which quotes 194.673 K for that correlation, could not be opened as text.
/// Their sublimation equation, which the code uses, gives 1.01325 bar there.
const CO2_SUBLIMATION_1_ATM_K: f64 = 194.6855;
/// Source: sublimation enthalpy of CO2, 26.1 kJ/mol at 207 K (Stephenson
/// and Malanowski 1987, as listed by the NIST Chemistry WebBook, phase
/// change data of CO2, read on 2026-11-22; other entries 25.2 to 27.2).
const CO2_SUBLIMATION_ENTHALPY_207_K: (f64, f64) = (207.0, 26_100.0);
/// Source: triple-point pressure of methane, 0.11696 bar (Setzmann and
/// Wagner 1991, doi:10.1063/1.555898), read in thermopack
/// (`fluids/Methane.json`, `sublimation_curve.reducing_pressure`, 11 696 Pa,
/// commit d68c794c).
const METHANE_TRIPLE_BAR: f64 = 0.116_96;
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
    // Colder, less; the curve meets the vapour curve at the triple point.
    let below = co2_saturation_bar(CO2_TRIPLE_TEMPERATURE.next_down());
    let triple = co2_saturation_bar(CO2_TRIPLE_TEMPERATURE);
    assert!(
        close(below, triple, JOIN_TOLERANCE),
        "{below} against {triple}"
    );
    assert!(below <= triple);
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
    // Below the triple point, the sublimation curve's slope gives the
    // measured sublimation enthalpy; past the critical point, zero.
    let (t, enthalpy) = CO2_SUBLIMATION_ENTHALPY_207_K;
    let latent = co2_latent_j_mol(t);
    assert!(
        close(latent, enthalpy, LATENT_TOLERANCE),
        "{latent} against {enthalpy}"
    );
    assert_eq!(co2_latent_j_mol(310.0), 0.0);
}

#[test]
fn co2_caps_at_its_triple_and_critical_points() {
    let triple = co2_saturation_bar(CO2_TRIPLE_TEMPERATURE);
    let sourced = CO2_TRIPLE_PRESSURE_PA / PASCALS_PER_BAR;
    assert!(close(triple, sourced, JOIN_TOLERANCE), "{triple}");
    let critical = co2_saturation_bar(CO2_CRITICAL_TEMPERATURE.next_down());
    let sourced = CO2_CRITICAL_PRESSURE_PA / PASCALS_PER_BAR;
    assert!(close(critical, sourced, JOIN_TOLERANCE), "{critical}");
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
    assert!(close(triple, METHANE_TRIPLE_BAR, 1e-3), "{triple}");
    assert!(methane_saturation_bar(60.0) < methane_saturation_bar(80.0));
    for t in [METHANE_CRITICAL_TEMPERATURE, 200.0, 1500.0] {
        assert_eq!(methane_saturation_bar(t), f64::INFINITY, "{t} K");
    }
    for t in [1.0, 0.0, -5.0, f64::NAN] {
        let p = methane_saturation_bar(t);
        assert!(p.is_finite() && p >= 0.0, "{t} K: {p}");
    }
}

/// Source: core mass fraction of the Earth, 32.5 ± 0.3 wt%, the primitive
/// mantle (the bulk silicate Earth) the other 67.5 wt% (Wang, Lineweaver
/// & Ireland 2018, Icarus 299, 460, doi:10.1016/j.icarus.2017.08.024,
/// arXiv:1708.08718, section 2.2 and Fig. 2).
const EARTH_SILICATE_SHARE: f64 = 0.675;
/// Source: sulphur in the primitive mantle, 235 ± 38 ppm by mass (Wang et
/// al. 2018, Table 1, concordance of three compilations), in kg per kg.
const WANG_SULPHUR: (f64, f64) = (235e-6, 38e-6);

#[test]
fn earths_silicate_sulphur_is_wangs() {
    let silicate = Kilogram(EARTH_MASS.0 * EARTH_SILICATE_SHARE);
    let core = Kilogram(EARTH_MASS.0 - silicate.0);
    let earth = inventory(EARTH_MASS, silicate, Kilogram(0.0));
    // The sulphur comes from the silicate alone, never from the core.
    assert_eq!(earth.s, Kilogram(ROCK_S * silicate.0));
    assert_eq!(
        inventory(core, Kilogram(0.0), Kilogram(0.0)).s,
        Kilogram(0.0)
    );
    let (centre, spread) = WANG_SULPHUR;
    let ppm = earth.s.0 / silicate.0;
    assert!((ppm - centre).abs() <= spread, "{ppm} kg/kg");
    assert_eq!(earth.co2, Kilogram(ROCK_CO2 * EARTH_MASS.0));
    assert_eq!(earth.n2, Kilogram(ROCK_N2 * EARTH_MASS.0));
}
