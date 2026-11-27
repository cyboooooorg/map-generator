//! Tests of the spectrum, ported from `legacy-final`'s
//! `planet/spectrum.rs` with their tolerances.

use std::sync::LazyLock;

use super::reference::*;
use super::*;
use crate::constants::{STANDARD_GRAVITY, STEFAN_BOLTZMANN};
use crate::radiation::gases::H2O_MOLAR_MASS;
use crate::radiation::ktable::{CO2_SET, Table, WATER_FOREIGN_SET, WATER_SELF_SET};
use crate::radiation::test_points::{self, EARTH_SURFACE_K, PER_MILLION, PREINDUSTRIAL_PPM};
use crate::surface::RELATIVE_HUMIDITY;
use crate::water::vapour_saturation_bar;

/// The table on bins half as wide (10 cm⁻¹) over 0-2500 cm⁻¹, at
/// 200-300 K only, built by the same tool for
/// `halving_bins_keeps_the_forcing` (see `OPACITY.md`).
static HALVED: LazyLock<Table> = LazyLock::new(|| {
    Table::parse(include_bytes!("../../data/opacity-halved.bin")).expect("a table")
});

/// Definition: Earth's surface gravity of the test columns, the standard
/// acceleration of gravity, m/s².
const EARTH_GRAVITY: f64 = STANDARD_GRAVITY.0;

/// Definition: the gas constant of dry air of the test profile, J/(kg K):
/// the molar gas constant over 28.964 g/mol, the mean molar mass of dry
/// air of the U.S. Standard Atmosphere 1976 (NASA-TM-X-74335; the
/// arithmetic checked by the radiative transfer audit, Wiki study 19, the
/// document itself not read).
const DRY_AIR_R: f64 = 287.05;

/// Definition: the tropospheric lapse rate of the test profile, K/m, the
/// U.S. Standard Atmosphere 1976's 6.5 K/km (audit, Wiki study 19).
const LAPSE_K_PER_M: f64 = 6.5e-3;

/// Definition: the test profile's levels per decade of pressure.
const LEVELS_PER_DECADE: usize = 20;

/// Definition: the test profile's decades of pressure, from 1 bar up to
/// 1e-6 bar.
const DECADES: usize = 6;

/// Definition: the tropopause temperature of the test profile, K: the
/// U.S. Standard Atmosphere 1976's isothermal layer from 11 km, 216.65 K
/// as the audit (Wiki study 19) quotes it, rounded; the document itself
/// was not read (the NASA scan has no text layer).
const TROPOPAUSE_K: f64 = 217.0;

/// Definition: O₂'s mole fraction in the test columns' dry air, rounded.
const O2_SHARE: f64 = 0.21;

/// Calibration: a plausibility band for the forcing of a CO₂ doubling,
/// W/m², on the legacy test profile, not a like-for-like anchor. 3.7 W/m²
/// is the IPCC TAR's best estimate, `5.35 ln 2` (Myhre, Highwood, Shine &
/// Stordal 1998, Geophys. Res. Lett. 25, 2715, doi:10.1029/98GL01908): a
/// global-mean, all-sky, stratosphere-adjusted forcing at the tropopause,
/// where the test takes an instantaneous clear-sky change of the outgoing
/// radiation of one column (3.41 W/m² for 280 to 560 ppm).
///
/// Known gap (issue #643): the like-for-like value, the instantaneous
/// clear-sky forcing at the top of the model for 287 to 574 ppm on the
/// AFGL mid-latitude summer profile (Anderson et al. 1986,
/// AFGL-TR-86-0110), is 2.80 W/m² (standard deviation 0.06) in RTMIP's
/// line-by-line codes (Collins et al. 2006, J. Geophys. Res. 111, D14317,
/// doi:10.1029/2005JD006713, Table 8, case 2b-1a, "TOM ⟨F_lbl⟩"). On that
/// profile, with RTMIP's 294 K surface, the column gives 3.01 W/m²,
/// outside both Collins' 2.80 ± 0.12 (twice the line-by-line spread) and
/// this band. The column departs from RTMIP's: it holds no ozone (RTMIP's
/// holds the profile's O₃, §2.1, paragraph 15), N₂ stands in for the O₂
/// RTMIP removes as CO₂ grows, and it sums 0-5000 cm⁻¹ where the
/// line-by-line codes take 4-100 µm. Untested candidate causes: the
/// missing ozone, the error of the k-distribution against line-by-line,
/// and the table's CO₂ line shape or continuum.
const DOUBLING_FORCING: f64 = 3.7;

/// Calibration: the width of the plausibility band of
/// [`DOUBLING_FORCING`], W/m², about 16 %, the legacy author's (Myhre et
/// al. 1998 state an uncertainty of about 10 %).
const DOUBLING_FORCING_TOLERANCE: f64 = 0.6;

/// Definition: the CO₂ amounts, ppm, over which the forcing per doubling
/// is compared, from a fourth to more than thrice the pre-industrial.
const LOGARITHMIC_PPM: [f64; 3] = [70.0, 280.0, 1000.0];

/// Calibration: how far each forcing per doubling may stray from their
/// mean, a share. The logarithmic law is Romps, Seeley & Edman 2022
/// (J. Climate 35, 4027, doi:10.1175/JCLI-D-21-0275.1); the 15 % band is
/// the legacy author's, not from the paper, which notes departures from
/// the logarithm at very low and very high CO₂.
const LOGARITHMIC_SHARE: f64 = 0.15;

/// Calibration: how far halving the bins may move the doubling forcing,
/// W/m², a convergence tolerance (about 3 % of the forcing); the legacy
/// measured 0.006.
const HALVING_TOLERANCE: f64 = 0.1;

/// Definition: relative tolerance of two sums of the same terms in a
/// different order, in `f64`.
const SAME_SUM: f64 = 1e-12;

/// Definition: relative tolerance of the bins plus the tail against
/// `σT⁴`, the series' truncation.
const SIGMA_T4_TOLERANCE: f64 = 1e-9;

/// Definition: relative tolerance of a trace of vapour against the
/// air-broadened set alone (a share of 1e-6 of self-broadening).
const TRACE_TOLERANCE: f64 = 1e-5;

/// Definition: relative tolerance of pure steam against the
/// self-broadened set.
const STEAM_TOLERANCE: f64 = 1e-9;

/// Definition: relative tolerance of a factor taken from logarithms.
const SCALE_TOLERANCE: f64 = 1e-6;

/// Definition: Venus's gravity of the test cases, m/s², the shared
/// sourced value.
const VENUS_GRAVITY: f64 = test_points::VENUS_GRAVITY.0;

/// Definition: Titan's gravity of the test cases, m/s², the shared
/// sourced value.
const TITAN_GRAVITY: f64 = test_points::TITAN_GRAVITY.0;

/// Definition: a Venus-like surface level of the test cases, near the
/// NASA Venus Fact Sheet's surface (737 K, 92 bar, 96.5 % CO₂, 3.5 % N₂,
/// 150 ppm SO₂) with 30 ppm of water: temperature (K), N₂, CO₂, H₂O and
/// SO₂ (bar).
const VENUS_LEVEL: [f64; 5] = [735.0, 3.2, 88.8, 3e-3, 1.4e-2];

/// Definition: a Titan-like surface level of the test cases, near the
/// Huygens surface (93.65 K, 1467 hPa, Fulchignoni et al. 2005, Nature
/// 438, 785, doi:10.1038/nature04314): temperature (K), N₂ and CH₄
/// (bar).
const TITAN_LEVEL: [f64; 3] = [93.7, 1.45, 0.07];

/// Definition: a steam level of the test cases, the hottest column the
/// legacy climate solved: temperature (K), H₂O, N₂ and CO₂ (bar).
const STEAM_LEVEL: [f64; 4] = [640.0, 70.0, 0.78, 4e-4];

/// Definition: an Earth surface level of the test cases: temperature
/// (K), vapour (bar) and CO₂ (ppm).
const EARTH_LEVEL: [f64; 3] = [288.0, 0.013, 400.0];

/// Definition: a bin of water's rotation band, 200-220 cm⁻¹.
const ROTATION_BIN: usize = 10;

/// Definition: a bin of CO₂'s 15 µm band, 660-680 cm⁻¹.
const CO2_BAND_BIN: usize = 33;

/// Definition: the temperature, K, of the broadening tests, on the
/// table's grid.
const BROADENING_K: f64 = 300.0;

/// Definition: a hot temperature, K, on the table's grid, for the
/// pressures past it.
const HOT_K: f64 = 700.0;

/// Definition: the top pressure of the table's grid, bar, and twice it.
const GRID_TOP_BAR: f64 = 100.0;

/// Definition: a trace of vapour, bar.
const TRACE_BAR: f64 = 1e-6;

/// Definition: temperatures, K, of the black-body bands test, from a
/// frozen body to past the bins' reach.
const PLANCK_BANDS_K: [f64; 6] = [40.0, 100.0, 288.0, 640.0, 1800.0, 3000.0];

/// Definition: a floor for relative comparisons near zero.
const TINY: f64 = 1e-300;

/// One level of a fixed profile: pressure (bar), temperature (K) and
/// water vapour (bar).
#[derive(Clone, Copy)]
struct Point {
    pressure_bar: f64,
    temperature_k: f64,
    vapour_bar: f64,
}

/// A US-standard-like Earth column: `surface_k` at 1 bar, 6.5 K/km up
/// to a tropopause at `tropopause_k`, isothermal above, with
/// `humidity` relative humidity in the troposphere and the
/// tropopause's mixing ratio above it; 20 levels per decade of
/// pressure up to 1e-6 bar. The saturation is IAPWS's
/// ([`vapour_saturation_bar`]).
fn earth_profile(surface_k: f64, tropopause_k: f64, humidity: f64) -> Vec<Point> {
    let surface_bar = 1.0;
    let exponent = DRY_AIR_R * LAPSE_K_PER_M / EARTH_GRAVITY;
    let tropopause_bar = surface_bar * (tropopause_k / surface_k).powf(1.0 / exponent);
    let trapped = humidity * vapour_saturation_bar(tropopause_k) / tropopause_bar;
    let levels = DECADES * LEVELS_PER_DECADE;
    (0..=levels)
        .map(|i| {
            let p = surface_bar * 10f64.powf(-(i as f64) / LEVELS_PER_DECADE as f64);
            let t = (surface_k * (p / surface_bar).powf(exponent)).max(tropopause_k);
            let e = if p < tropopause_bar {
                trapped * p
            } else {
                humidity * vapour_saturation_bar(t)
            };
            Point {
                pressure_bar: p,
                temperature_k: t,
                vapour_bar: e,
            }
        })
        .collect()
}

/// Earth's dry air with `co2_ppm` of CO₂, as mole fractions.
fn earth_air(co2_ppm: f64) -> PartialPressures {
    let co2 = co2_ppm * PER_MILLION;
    PartialPressures {
        n2: 1.0 - O2_SHARE - co2,
        co2,
        o2: O2_SHARE,
        ..PartialPressures::default()
    }
}

/// A path through the air: the optical depth of each g-point of each
/// gas, and the smooth part's, summed layer by layer.
fn add_layer(path: &mut BinAbsorption, a: &BinAbsorption, b: &BinAbsorption, span: f64) {
    for ((path, a), b) in path.lines.iter_mut().zip(&a.lines).zip(&b.lines) {
        for g in 0..G {
            path[g] += log_mean(a[g], b[g]) * span;
        }
    }
    path.smooth += log_mean(a.smooth, b.smooth) * span;
}

/// Outgoing longwave radiation, W/m², through the `bins` of `table`,
/// of a fixed `profile` (from the surface up, uniform steps in
/// `ln p`) whose dry air has the mole fractions `gases`: per bin, the
/// two-stream emission with [`DIFFUSIVITY`], the layer-mean Planck
/// flux and the logarithmic mean of the opacities at a layer's two
/// levels, CO₂ and water overlapping at random, as the column sums
/// it. Above the top the air is taken isothermal and its depth is
/// half the top's rate per `ln p`, exact for absorbers that grow as
/// `p²` (broadened lines, the continuum, the pairs). The surface is a
/// black body at the first level's temperature.
fn olr_bins(
    table: &Table,
    bins: std::ops::Range<usize>,
    profile: &[Point],
    gases: &PartialPressures,
) -> f64 {
    let dry = gases.total();
    let level = |point: &Point| {
        let share = (point.pressure_bar - point.vapour_bar).max(0.0) / dry;
        let mut g = PartialPressures {
            n2: gases.n2 * share,
            co2: gases.co2 * share,
            h2o: 0.0,
            ch4: gases.ch4 * share,
            nh3: gases.nh3 * share,
            so2: gases.so2 * share,
            o2: gases.o2 * share,
            ar: gases.ar * share,
        };
        g.h2o = point.vapour_bar;
        Level {
            temperature_k: point.temperature_k,
            gases: g,
            ozone_bar: 0.0,
            n2o_bar: 0.0,
            gravity_m_s2: EARTH_GRAVITY,
        }
    };
    let levels: Vec<Level> = profile.iter().map(level).collect();
    let n = levels.len() - 1;
    let width = table.bin_width_cm;
    let mut total = 0.0;
    for b in bins {
        let (lo, hi) = (b as f64 * width, (b + 1) as f64 * width);
        let rates: Vec<BinAbsorption> = levels.iter().map(|l| table.bin_absorption(b, l)).collect();
        let planck: Vec<f64> = levels
            .iter()
            .map(|l| planck_between(lo, hi, l.temperature_k))
            .collect();
        let mut path = BinAbsorption::default();
        add_layer(&mut path, &rates[n], &rates[n], 0.5);
        let mut seen = path.transmission(1.0);
        let mut out = planck[n] * (1.0 - seen);
        for i in (0..n).rev() {
            let span = (profile[i].pressure_bar / profile[i + 1].pressure_bar).ln();
            add_layer(&mut path, &rates[i], &rates[i + 1], span);
            let below = path.transmission(1.0);
            out += 0.5 * (planck[i] + planck[i + 1]) * (seen - below);
            seen = below;
        }
        total += out + planck[0] * seen;
    }
    total
}

/// [`olr_bins`] on the model's bins.
fn olr_fixed_profile(profile: &[Point], gases: &PartialPressures) -> f64 {
    olr_bins(&OPACITY, 0..BINS, profile, gases)
}

fn earth_column() -> Vec<Point> {
    earth_profile(EARTH_SURFACE_K, TROPOPAUSE_K, RELATIVE_HUMIDITY)
}

fn doubling_forcing(co2_ppm: f64) -> f64 {
    let profile = earth_column();
    olr_fixed_profile(&profile, &earth_air(co2_ppm))
        - olr_fixed_profile(&profile, &earth_air(2.0 * co2_ppm))
}

fn venus_level() -> Level {
    let [t, n2, co2, h2o, so2] = VENUS_LEVEL;
    Level {
        temperature_k: t,
        gases: PartialPressures {
            n2,
            co2,
            h2o,
            so2,
            ..PartialPressures::default()
        },
        ozone_bar: 0.0,
        n2o_bar: 0.0,
        gravity_m_s2: VENUS_GRAVITY,
    }
}

#[test]
fn the_table_is_on_the_models_bins() {
    // The table's own layout is `ktable`'s test; here, that it is the
    // spectrum's.
    let table = &*OPACITY;
    assert_eq!((table.bins, table.bin_width_cm), (BINS, BIN_WIDTH_CM));
    assert!(table.ln_k.iter().all(|k| k.is_finite()));
}

#[test]
fn water_mixes_its_two_broadenings_by_its_share() {
    // At 1 bar and 300 K in the rotation band (bin 10, 200-220 cm⁻¹):
    // a trace of vapour takes the air-broadened set, pure steam the
    // self-broadened one, which is wider and so stronger between the
    // lines.
    let level = |h2o: f64, n2: f64| Level {
        temperature_k: BROADENING_K,
        gases: PartialPressures {
            h2o,
            n2,
            ..PartialPressures::default()
        },
        ozone_bar: 0.0,
        n2o_bar: 0.0,
        gravity_m_s2: EARTH_GRAVITY,
    };
    let per_kg = |l: &Level| {
        let a = absorption_per_ln_p(ROTATION_BIN, l);
        let mass = l.mass_per_ln_p(l.gases.h2o, H2O_MOLAR_MASS);
        a.lines[LineGas::Water as usize].map(|k| k / mass)
    };
    let trace = per_kg(&level(TRACE_BAR, 1.0));
    let steam = per_kg(&level(1.0, 0.0));
    let table = &*OPACITY;
    let mut foreign = [0.0; G];
    let mut own = [0.0; G];
    table.coefficients(
        WATER_FOREIGN_SET,
        1.0,
        BROADENING_K,
        ROTATION_BIN,
        1.0,
        &mut foreign,
    );
    table.coefficients(
        WATER_SELF_SET,
        1.0,
        BROADENING_K,
        ROTATION_BIN,
        1.0,
        &mut own,
    );
    for g in 0..G {
        assert!(
            (trace[g] / foreign[g] - 1.0).abs() < TRACE_TOLERANCE,
            "g {g}"
        );
        assert!((steam[g] / own[g] - 1.0).abs() < STEAM_TOLERANCE, "g {g}");
    }
    assert!(own[0] > foreign[0], "{own:?} vs {foreign:?}");
}

#[test]
fn lines_broaden_with_pressure_past_the_grid() {
    // Over 100 bar the coefficients scale as p (Lorentz).
    let table = &*OPACITY;
    let mut at_100 = [0.0; G];
    let mut at_200 = [0.0; G];
    table.coefficients(CO2_SET, GRID_TOP_BAR, HOT_K, CO2_BAND_BIN, 1.0, &mut at_100);
    table.coefficients(
        CO2_SET,
        2.0 * GRID_TOP_BAR,
        HOT_K,
        CO2_BAND_BIN,
        1.0,
        &mut at_200,
    );
    for g in 0..G {
        assert!(
            (at_200[g] / at_100[g] - 2.0).abs() < SCALE_TOLERANCE,
            "g {g}"
        );
    }
}

/// A plausibility band, not a like-for-like check (see
/// [`DOUBLING_FORCING`]): the like-for-like RTMIP comparison fails, issue
/// #643, which decides this test's fate.
#[test]
fn doubling_co2_forcing_is_within_the_plausibility_band() {
    let forcing = doubling_forcing(PREINDUSTRIAL_PPM);
    assert!(
        (forcing - DOUBLING_FORCING).abs() <= DOUBLING_FORCING_TOLERANCE,
        "280 to 560 ppm forces {forcing:.3} W/m², against \
         {DOUBLING_FORCING} ± {DOUBLING_FORCING_TOLERANCE}"
    );
}

#[test]
fn co2_forcing_is_logarithmic() {
    let forcings = LOGARITHMIC_PPM.map(doubling_forcing);
    let mean = forcings.iter().sum::<f64>() / forcings.len() as f64;
    assert!(mean > 0.0, "no forcing: {forcings:?}");
    for f in forcings {
        assert!(
            (f / mean - 1.0).abs() < LOGARITHMIC_SHARE,
            "forcing per doubling {forcings:?} strays from {mean:.3}"
        );
    }
}

#[test]
fn halving_bins_keeps_the_forcing() {
    // The halved table covers 0-2500 cm⁻¹; past it both sums take the
    // model's bins, which then cancel in the comparison.
    let halved = &*HALVED;
    assert_eq!(halved.bin_width_cm, 0.5 * BIN_WIDTH_CM);
    let split = (halved.bins as f64 * halved.bin_width_cm / BIN_WIDTH_CM).round() as usize;
    let profile = earth_column();
    let fine_olr = |ppm: f64| {
        olr_bins(halved, 0..halved.bins, &profile, &earth_air(ppm))
            + olr_bins(&OPACITY, split..BINS, &profile, &earth_air(ppm))
    };
    let coarse = doubling_forcing(PREINDUSTRIAL_PPM);
    let fine = fine_olr(PREINDUSTRIAL_PPM) - fine_olr(2.0 * PREINDUSTRIAL_PPM);
    assert!(coarse > 0.0, "no forcing");
    assert!(
        (coarse - fine).abs() < HALVING_TOLERANCE,
        "halving the bins moves the forcing from {coarse:.4} to {fine:.4} W/m²"
    );
}

#[test]
fn the_spectrum_of_a_level_is_its_bins() {
    let [earth_k, earth_h2o, earth_ppm] = EARTH_LEVEL;
    let [titan_k, titan_n2, titan_ch4] = TITAN_LEVEL;
    let [steam_k, steam_h2o, steam_n2, steam_co2] = STEAM_LEVEL;
    let mut venus = venus_level();
    venus.gases.so2 = 0.0;
    let levels = [
        venus_level(),
        Level {
            temperature_k: earth_k,
            gases: PartialPressures {
                h2o: earth_h2o,
                ..earth_air(earth_ppm)
            },
            ozone_bar: 0.0,
            n2o_bar: 0.0,
            gravity_m_s2: EARTH_GRAVITY,
        },
        venus,
        Level {
            temperature_k: titan_k,
            gases: PartialPressures {
                n2: titan_n2,
                ch4: titan_ch4,
                ..PartialPressures::default()
            },
            ozone_bar: 0.0,
            n2o_bar: 0.0,
            gravity_m_s2: TITAN_GRAVITY,
        },
        Level {
            temperature_k: steam_k,
            gases: PartialPressures {
                h2o: steam_h2o,
                n2: steam_n2,
                co2: steam_co2,
                ..PartialPressures::default()
            },
            ozone_bar: 0.0,
            n2o_bar: 0.0,
            gravity_m_s2: EARTH_GRAVITY,
        },
    ];
    let mut spectrum = Spectrum::default();
    let close = |a: f64, b: f64| (a - b).abs() <= SAME_SUM * b.abs().max(TINY);
    for level in &levels {
        absorption_spectrum(level, &mut spectrum);
        for i in 0..BINS {
            let one = absorption_per_ln_p(i, level);
            let all = one.lines.iter().flatten().chain([&one.smooth]);
            assert!(all.into_iter().all(|x| *x >= 0.0), "bin {i}: {one:?}");
            for (gas, (lines, one)) in LineGas::ALL
                .iter()
                .zip(spectrum.lines.iter().zip(&one.lines))
            {
                for g in 0..G {
                    assert!(close(lines[i][g], one[g]), "{gas:?} bin {i}");
                }
            }
            assert!(
                close(spectrum.smooth[i], one.smooth),
                "bin {i} at {} K: {} vs {}",
                level.temperature_k,
                spectrum.smooth[i],
                one.smooth
            );
        }
        // The tail holds no absorption: the column sums it on the
        // shortwave table's bins (`column::beyond`).
        for lines in &spectrum.lines {
            assert_eq!(lines[TAIL], [0.0; G]);
        }
        assert_eq!(spectrum.smooth[TAIL], 0.0);
    }
}

#[test]
fn the_planck_spectrum_is_the_bins_and_their_rest() {
    let mut planck = [0.0; BANDS];
    for t in PLANCK_BANDS_K {
        planck_spectrum(t, &mut planck);
        for (i, p) in planck[..BINS].iter().enumerate() {
            let one = planck_bin_w_m2(i, t);
            assert!(
                (p - one).abs() <= SAME_SUM * one.max(TINY),
                "bin {i}: {p} vs {one}"
            );
        }
        let black = STEFAN_BOLTZMANN * f64::powi(t, 4);
        let sum: f64 = planck.iter().sum();
        assert!(
            (sum / black - 1.0).abs() < SIGMA_T4_TOLERANCE,
            "{t} K: {sum} vs {black}"
        );
    }
}

#[test]
fn the_planck_flux_over_any_edges_is_their_integrals_and_the_rest() {
    // Uneven edges across the head and tail series' switch, as the
    // column's sum past the longwave table takes them: each interval is
    // its exact integral, the last slot the rest, and the whole `σT⁴`
    // from the first edge at 0.
    let edges = [0.0, 1000.0, 5000.0, 7311.0, 14_364.0, 50_000.0];
    let mut out = [0.0; 6];
    for t in PLANCK_BANDS_K {
        planck_over(t, &edges, &mut out);
        for (j, p) in out[..edges.len() - 1].iter().enumerate() {
            let one = planck_between(edges[j], edges[j + 1], t);
            assert!(
                (p - one).abs() <= SAME_SUM * one.max(TINY),
                "{t} K, interval {j}: {p} vs {one}"
            );
        }
        let black = STEFAN_BOLTZMANN * f64::powi(t, 4);
        let sum: f64 = out.iter().sum();
        assert!(
            (sum / black - 1.0).abs() < SIGMA_T4_TOLERANCE,
            "{t} K: {sum} vs {black}"
        );
    }
}

/// The level weighs its argon: Earth's dry air of the fact sheet, its
/// argon included, has the U.S. Standard Atmosphere's mean molar mass,
/// 28.964 g/mol ([`DRY_AIR_R`]'s), within 0.1 %; without its argon it is
/// 0.3 % lighter. Pure argon weighs its CIAAW 39.95 g/mol.
#[test]
fn a_level_weighs_its_argon() {
    /// Source: the mean molar mass of the U.S. Standard Atmosphere 1976's
    /// dry air, 28.964 g/mol (NASA-TM-X-74335, as [`DRY_AIR_R`] cites
    /// it), kg/mol.
    const DRY_AIR_MOLAR_MASS: f64 = 28.964e-3;
    let level = |gases| Level {
        temperature_k: EARTH_SURFACE_K,
        gases,
        ozone_bar: 0.0,
        n2o_bar: 0.0,
        gravity_m_s2: EARTH_GRAVITY,
    };
    // Source: dry air, NASA Earth fact sheet: 78.08 % N₂, 20.95 % O₂,
    // 9340 ppm Ar, 420 ppm CO₂, by volume.
    let air = PartialPressures {
        n2: 0.7808,
        o2: 0.2095,
        ar: 9340e-6,
        co2: 420e-6,
        ..PartialPressures::default()
    };
    let mass = level(air).molar_mass_kg();
    assert!((mass / DRY_AIR_MOLAR_MASS - 1.0).abs() < 1e-3, "{mass}");
    let without = level(PartialPressures { ar: 0.0, ..air }).molar_mass_kg();
    assert!(
        (without / DRY_AIR_MOLAR_MASS - 1.0).abs() > 2e-3,
        "{without}"
    );
    let argon = level(PartialPressures {
        ar: 1.0,
        ..PartialPressures::default()
    });
    assert!((argon.molar_mass_kg() - crate::radiation::gases::AR_MOLAR_MASS).abs() < 1e-15);
}

/// Source: points of HITRAN's collision-induced absorption files
/// (`OPACITY.md`, SHA-256 there): the pair, the file's block temperature
/// (K), a wavenumber (cm⁻¹) and the file's coefficient there
/// (cm⁵/molecule²), as the file's rows give them, with the column of
/// [`PAIRS_CIA_K`] nearest the block's temperature. `N2-N2_2021.cia`,
/// 70.0 K and 290.0 K at 110 cm⁻¹ (Chistikov et al. 2019);
/// `O2-O2_2024.cia`, 297.8 K at 1550.1 cm⁻¹; `O2-N2_2024.cia`, 293.0 K at
/// 1550.4882 cm⁻¹; `N2-CH4_2024.cia`, 90.0 K at 110 and 310 cm⁻¹
/// (Finenko et al. 2022); `CH4-CH4_2011.cia`, 400.0 K at 110 and 310 cm⁻¹
/// (Borysow & Frommhold 1987).
const PAIR_FILE_POINTS: [(&str, f64, f64, f64); 8] = [
    ("N2-N2", 70.0, 110.0, 4.4969e-45),
    ("N2-N2", 290.0, 110.0, 6.4148e-45),
    ("O2-O2", 300.0, 1550.1, 6.340e-45),
    ("O2-N2", 290.0, 1550.4882, 6.0856e-45),
    ("N2-CH4", 90.0, 110.0, 2.9326e-44),
    ("N2-CH4", 90.0, 310.0, 7.4544e-45),
    ("CH4-CH4", 400.0, 110.0, 4.193e-45),
    ("CH4-CH4", 400.0, 310.0, 1.388e-44),
];

/// Calibration: how far a 20 cm⁻¹ cell's mean may stray from the file's
/// value at a point inside it, a share, 5 %, chosen above the largest
/// stray measured on the points of [`PAIR_FILE_POINTS`]: N₂–N₂ at 70 K,
/// 110 cm⁻¹, +3.46 % (the band's curvature over the cell at its peak),
/// and O₂–O₂ at 300 K against the 297.8 K block, −3.50 % (the cell's mean
/// and a 2.2 K step in temperature); the others stray by 0.06 to 2.3 %.
/// Not a derivation: the band's change with temperature near its peak,
/// 2 to 2.5 % per K at 70 to 80 K (study 38's probes), would give a bound
/// per point, not one share for all.
const CELL_AGAINST_POINT: f64 = 0.05;

/// Definition: relative tolerance of the single-precision pairs against
/// the double-precision ones, a few roundings of `f32`.
const FAST_PAIRS_TOLERANCE: f64 = 1e-6;

/// The pair of [`PAIRS`] made of `a` and `b`.
fn pair(a: Partner, b: Partner) -> &'static Pair {
    PAIRS
        .iter()
        .find(|p| p.gases == (a, b))
        .expect("a pair of the list")
}

#[test]
fn the_pairs_are_hitrans_files() {
    use Partner::{Ch4, N2, O2};
    let named = |name: &str| match name {
        "N2-N2" => pair(N2, N2),
        "O2-O2" => pair(O2, O2),
        "O2-N2" => pair(O2, N2),
        "N2-CH4" => pair(N2, Ch4),
        "CH4-CH4" => pair(Ch4, Ch4),
        other => panic!("{other}"),
    };
    for (name, t, nu, file) in PAIR_FILE_POINTS {
        let p = named(name);
        let row = p.rows[(nu / CIA_CELL_CM).floor() as usize].expect("a cell");
        let k = p.coefficient(row, p.column(t));
        assert!(
            (k / file - 1.0).abs() < CELL_AGAINST_POINT,
            "{name} at {t} K, {nu} cm⁻¹: {k:e} against the file's {file:e}"
        );
    }
    // Each grows as the product of its two gases' densities: twice each
    // gas, the same mixture, four times the depth per `ln p`.
    let level = |n2: f64, ch4: f64| Level {
        temperature_k: 90.0,
        gases: PartialPressures {
            n2,
            ch4,
            ..PartialPressures::default()
        },
        ozone_bar: 0.0,
        n2o_bar: 0.0,
        gravity_m_s2: TITAN_GRAVITY,
    };
    let one = pair(N2, Ch4).depth(110.0, &level(1.0, 0.05));
    let more = pair(N2, Ch4).depth(110.0, &level(2.0, 0.1));
    assert!((more / one - 4.0).abs() < SCALE_TOLERANCE, "{more} {one}");
    assert_eq!(pair(N2, Ch4).depth(110.0, &level(1.0, 0.0)), 0.0);
}

#[test]
fn the_fast_pairs_are_the_reference() {
    // The fast path's smooth part against the reference's, on dry airs
    // whose smooth part is all pairs: Earth's N₂ and O₂ at 288 K, and
    // Titan's N₂ and methane at 93.7 K.
    let [titan_k, titan_n2, titan_ch4] = TITAN_LEVEL;
    let earth = Level {
        temperature_k: EARTH_SURFACE_K,
        gases: PartialPressures {
            co2: 0.0,
            ..earth_air(0.0)
        },
        ozone_bar: 0.0,
        n2o_bar: 0.0,
        gravity_m_s2: EARTH_GRAVITY,
    };
    let titan = Level {
        temperature_k: titan_k,
        gases: PartialPressures {
            n2: titan_n2,
            ch4: titan_ch4,
            ..PartialPressures::default()
        },
        ozone_bar: 0.0,
        n2o_bar: 0.0,
        gravity_m_s2: TITAN_GRAVITY,
    };
    let mut fast = FastLevel::default();
    let mut reference = Spectrum::default();
    for level in [earth, titan] {
        fast_level(&level, BINS, &mut fast);
        absorption_spectrum(&level, &mut reference);
        let mut absorbing = 0;
        for i in 0..BINS {
            let (f, r) = (f64::from(fast.smooth[i]), reference.smooth[i]);
            assert!(
                (f - r).abs() <= FAST_PAIRS_TOLERANCE * r,
                "bin {i} at {} K: {f} vs {r}",
                level.temperature_k
            );
            absorbing += usize::from(r > 0.0);
        }
        assert!(
            absorbing > 0,
            "no pair absorbs at {} K",
            level.temperature_k
        );
    }
}
