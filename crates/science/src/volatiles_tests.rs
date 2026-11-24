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

/// Definition: relative tolerance of the ammonia frost at its triple
/// point, 1e-6.
const AMMONIA_TRIPLE_TOLERANCE: f64 = 1e-6;
/// Source: triple-point pressure of ammonia, 0.06060 bar (Fonseca and Lobo
/// 1989, via the NIST Chemistry WebBook phase-change page).
const AMMONIA_TRIPLE_BAR: f64 = 0.060_60;
/// Source: triple-point temperature of ammonia, 195.49 K (Gao et al. 2023,
/// doi:10.1063/5.0128269).
const AMMONIA_TRIPLE_K: f64 = 195.49;
/// Source: ammonia frost points (T in K, p in bar, relative tolerance):
/// 1.27e-3 bar at 162.39 K (Karwat 1924, via Fray and Schmitt 2009, quoted
/// by Blakley et al. 2024 section 1) within 5 %, and 8.26e-3 bar at 177 K
/// (Fray and Schmitt 2009 fit NH3-1, via Lisse et al. 2021 supplement
/// Table 2) within 2 %.
const AMMONIA_FROST_POINTS: [(f64, f64, f64); 2] =
    [(162.39, 1.27e-3, 5e-2), (177.0, 8.26e-3, 2e-2)];
/// Definition: relative tolerance of two reads of the same curve, 1e-12
/// (the bar round trip changes the last bit).
const SAME_CURVE_TOLERANCE: f64 = 1e-12;
/// Definition: temperatures of the CO2 comparison, 150 K and 200 K, both
/// under the triple point.
const CO2_FROST_CHECKS_K: [f64; 2] = [150.0, 200.0];
/// Definition: coldest temperature of the cap check, 10 K.
const FROST_CAP_FLOOR_K: f64 = 10.0;
/// Definition: step of the cap check, 0.5 K.
const FROST_CAP_STEP_K: f64 = 0.5;
/// Definition: relative gap under the triple point where the frost is read,
/// 1e-9.
const JUST_UNDER: f64 = 1e-9;

#[test]
fn ammonia_frost_meets_its_triple_point() {
    let p = ammonia_frost_pressure(Kelvin(AMMONIA_TRIPLE_K * (1.0 - JUST_UNDER))).unwrap();
    assert!(close(
        p.0 / PASCALS_PER_BAR,
        AMMONIA_TRIPLE_BAR,
        AMMONIA_TRIPLE_TOLERANCE
    ));
}

#[test]
fn ammonia_frost_follows_its_source() {
    for (t, bar, tolerance) in AMMONIA_FROST_POINTS {
        let p = ammonia_frost_pressure(Kelvin(t)).unwrap();
        assert!(
            close(p.0 / PASCALS_PER_BAR, bar, tolerance),
            "{t} K: {}",
            p.0
        );
    }
}

#[test]
fn co2_frost_is_the_co2_curve_below_its_triple_point() {
    for t in CO2_FROST_CHECKS_K {
        let frost = co2_frost_pressure(Kelvin(t)).unwrap().0;
        assert!(close(
            frost,
            co2_saturation_pressure(Kelvin(t)).0,
            SAME_CURVE_TOLERANCE
        ));
    }
}

#[test]
fn the_frost_cap_holds_from_10_k_to_the_triple_point() {
    type Frost = fn(Kelvin) -> Option<Pascal>;
    let gases: [(Frost, f64); 2] = [
        (co2_frost_pressure, CO2_TRIPLE_TEMPERATURE),
        (ammonia_frost_pressure, AMMONIA_TRIPLE_TEMPERATURE),
    ];
    for (frost, triple) in gases {
        let mut previous = 0.0;
        let mut t = FROST_CAP_FLOOR_K;
        while t < triple * (1.0 - JUST_UNDER) {
            let p = frost(Kelvin(t)).unwrap().0;
            assert!(p.is_finite() && p > previous, "{t} K: {p} after {previous}");
            previous = p;
            t += FROST_CAP_STEP_K;
        }
        let top = frost(Kelvin(triple * (1.0 - JUST_UNDER))).unwrap().0;
        assert!(top.is_finite() && top > previous);
        assert!(frost(Kelvin(triple)).is_none());
        assert!(frost(Kelvin(triple + FROST_CAP_STEP_K)).is_none());
    }
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

/// Source: triple-point pressure of nitrogen, 0.1253 bar (Angus, de Reuck
/// et al. 1979, via the NIST Chemistry WebBook; study 24).
const NITROGEN_TRIPLE_PA: f64 = 12_530.0;
/// Source: the beta-N2 fit `N2-2` at the triple point, 0.1215 bar, and at
/// 37.2 K, 1.36e-5 bar (Fray and Schmitt 2009 through Lisse et al. 2021,
/// Table 2; study 24, finding 1).
const NITROGEN_BETA_READS: (f64, f64) = (0.1215, 1.36e-5);
/// Source: pressure of Triton's N2 frost, 1.4e-5 bar at 37.2 K (Tyler et
/// al. 1989, as quoted by study 24).
const TRITON_FROST_BAR: f64 = 1.4e-5;
/// Definition: temperature of Triton's frost, 37.2 K.
const TRITON_FROST_K: f64 = 37.2;
/// Definition: absolute tolerance on Triton's pressure, 0.05e-5 bar.
const TRITON_TOLERANCE_BAR: f64 = 0.05e-5;
/// Definition: relative tolerance of the beta fit scaled to the triple
/// point, 1 %.
const BETA_SCALED_TOLERANCE: f64 = 1e-2;
/// Definition: coldest temperature, 2 K, and step, 0.1 K, of the
/// monotony check of the nitrogen frost.
const NITROGEN_SWEEP_FLOOR_K: f64 = 2.0;
/// Definition: warmest temperature of the sweep, 63.1 K, under the triple
/// point.
const NITROGEN_SWEEP_CEILING_K: f64 = 63.1;
/// Definition: see [`NITROGEN_SWEEP_FLOOR_K`].
const NITROGEN_SWEEP_STEP_K: f64 = 0.1;
/// Definition: alpha-beta transition of nitrogen, 35.61 K (Scott 1976, as
/// quoted by Tegler et al. 2019).
const NITROGEN_TRANSITION_K: f64 = 35.61;
/// Source: normal boiling point of nitrogen, 77.355 K, at 101 325 Pa
/// (NIST Chemistry WebBook).
const NITROGEN_NORMAL_BOILING: (f64, f64) = (77.355, 101_325.0);
/// Definition: tolerance of the Antoine fit at the normal boiling point,
/// 5 % (the fit is 4.5 % low there).
const ANTOINE_TOLERANCE: f64 = 5e-2;
/// Source: validity of the Antoine fit of nitrogen, 63.14 to 126 K (Edejer
/// and Thodos 1967, via the NIST Chemistry WebBook).
const ANTOINE_RANGE_K: (f64, f64) = (63.14, 126.0);

#[test]
fn nitrogen_frost_follows_fray_and_schmitt() {
    let triple = nitrogen_frost_pressure(Kelvin(63.14 * (1.0 - JUST_UNDER))).unwrap();
    assert!(close(triple.0, NITROGEN_TRIPLE_PA, 1e-6));
    assert!(nitrogen_frost_pressure(Kelvin(63.14)).is_none());

    let scale = NITROGEN_TRIPLE_PA / PASCALS_PER_BAR / NITROGEN_BETA_READS.0;
    let p = nitrogen_frost_pressure(Kelvin(TRITON_FROST_K)).unwrap().0 / PASCALS_PER_BAR;
    assert!(
        close(p, NITROGEN_BETA_READS.1 * scale, BETA_SCALED_TOLERANCE),
        "{p}"
    );
    assert!((p - TRITON_FROST_BAR).abs() <= TRITON_TOLERANCE_BAR, "{p}");

    let below = nitrogen_frost_pressure(Kelvin(NITROGEN_TRANSITION_K - 1e-9)).unwrap();
    let at = nitrogen_frost_pressure(Kelvin(NITROGEN_TRANSITION_K)).unwrap();
    assert!(close(below.0, at.0, 1e-6));

    let mut previous = f64::INFINITY;
    let mut t = NITROGEN_SWEEP_CEILING_K;
    while t >= NITROGEN_SWEEP_FLOOR_K - 1e-9 {
        let p = nitrogen_frost_pressure(Kelvin(t)).unwrap().0;
        assert!(p > 0.0 && p < previous, "{t} K: {p}");
        previous = p;
        t -= NITROGEN_SWEEP_STEP_K;
    }
}

#[test]
fn nitrogen_vapour_follows_its_antoine_fit() {
    let (t, p) = NITROGEN_NORMAL_BOILING;
    let fit = nitrogen_vapour_pressure(Kelvin(t)).0;
    assert!(close(fit, p, ANTOINE_TOLERANCE), "{fit} Pa at {t} K");
    let mut previous = 0.0;
    let mut t = ANTOINE_RANGE_K.0;
    while t <= ANTOINE_RANGE_K.1 {
        let p = nitrogen_vapour_pressure(Kelvin(t)).0;
        assert!(p > previous, "{t} K");
        previous = p;
        t += 0.5;
    }
}

/// Definition: coldest temperature, 10 K, of the sulphur dioxide sweep.
const SO2_SWEEP_FLOOR_K: f64 = 10.0;
/// Definition: warmest temperature of the sweep, 197 K, under the triple
/// point.
const SO2_SWEEP_CEILING_K: f64 = 197.0;
/// Source: Wagman 1979, `P = 1.516e8 exp(-4510 K / T)` bar (as quoted by
/// Giles et al. 2024, eq. 1; study 24, finding 2): prefactor.
const WAGMAN_BAR: f64 = 1.516e8;
/// Source: the exponent of the same law, 4510 K.
const WAGMAN_EXPONENT_K: f64 = 4510.0;
/// Definition: temperature of the Io check, 115 K (study 24, finding 2).
const IO_FROST_K: f64 = 115.0;
/// Definition: factor between the anchored curve and Wagman's own at
/// 115 K, 1.2 (study 24: 1.3e-9 against 1.4e-9 bar).
const WAGMAN_FACTOR: f64 = 1.2;

#[test]
fn sulphur_dioxide_frost_holds_from_io_to_its_triple_point() {
    let triple = SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE;
    let p = sulphur_dioxide_frost_pressure(Kelvin(triple.0 - 1e-9)).unwrap();
    assert!(close(p.0, SULPHUR_DIOXIDE_TRIPLE_PRESSURE.0, 1e-6));
    assert!(sulphur_dioxide_frost_pressure(triple).is_none());

    let mut previous = 0.0;
    let mut t = SO2_SWEEP_FLOOR_K;
    while t <= SO2_SWEEP_CEILING_K {
        let p = sulphur_dioxide_frost_pressure(Kelvin(t)).unwrap().0;
        assert!(p.is_finite() && p > previous, "{t} K: {p}");
        previous = p;
        t += 1.0;
    }

    let ours = sulphur_dioxide_frost_pressure(Kelvin(IO_FROST_K))
        .unwrap()
        .0
        / PASCALS_PER_BAR;
    let wagman = WAGMAN_BAR * (-WAGMAN_EXPONENT_K / IO_FROST_K).exp();
    let ratio = ours / wagman;
    assert!(
        ratio < WAGMAN_FACTOR && ratio > 1.0 / WAGMAN_FACTOR,
        "{ratio}"
    );
}

/// Source: normal boiling point of ethane, 184.6 K (NIST Chemistry
/// WebBook).
const ETHANE_NORMAL_BOILING_K: f64 = 184.6;
/// Source: critical temperature of ethane, 305.3 K (NIST Chemistry
/// WebBook).
const ETHANE_CRITICAL_K: f64 = 305.3;

#[test]
fn boiling_caps_at_the_critical_point() {
    let normal = ethane_boiling_temperature(Pascal(101_325.0)).0;
    assert!((normal - ETHANE_NORMAL_BOILING_K).abs() < 0.01, "{normal}");
    let mpa = ethane_boiling_temperature(Pascal(1e6)).0;
    assert!(
        mpa > ETHANE_NORMAL_BOILING_K && mpa <= ETHANE_CRITICAL_K,
        "{mpa}"
    );
    assert_eq!(ethane_boiling_temperature(Pascal(1e9)).0, ETHANE_CRITICAL_K);
    assert!(ethane_boiling_temperature(Pascal(1e5)).0 < mpa);
    assert!(mpa < ethane_boiling_temperature(Pascal(1e7)).0);
}
