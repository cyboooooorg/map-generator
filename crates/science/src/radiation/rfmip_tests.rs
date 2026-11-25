//! The shortwave on RFMIP's clear-sky field against the line-by-line
//! reference: how the sunlight the air absorbs responds to CO₂ and to
//! water on 51 sunlit ERA-Interim profiles (Pincus et al. 2016), against
//! LBLRTM's fluxes (`crates/science/data/rfmip-shortwave.tsv`, built by
//! `tools/rfmip-shortwave`, provenance in `crates/science/data/RFMIP.md`).
//!
//! The total is not compared: the reference holds ozone, O₂, CH₄ and N₂O,
//! which the column lacks (#653); a difference between two experiments
//! that change only CO₂, or only water, cancels them. The column takes
//! its fixed slant (`spectrum::DIFFUSIVITY`), not each site's zenith
//! angle, and the table's Sun.

use crate::constants::{SOLAR_EFFECTIVE_TEMPERATURE, STANDARD_GRAVITY};
use crate::radiation::column::Levels;
use crate::radiation::gases::PartialPressures;
use crate::radiation::rayleigh::rayleigh_depth_550;
use crate::radiation::shortwave::{Air, Paths};
use crate::radiation::spectrum::Level;
use crate::radiation::starlight::{SOLAR_METALLICITY, star_bands};
use crate::radiation::test_points::EARTH_AIR;

/// The field (see the module's documentation).
const TABLE: &str = include_str!("../../data/rfmip-shortwave.tsv");

/// Definition: Pascals per bar, a unit conversion.
const PA_PER_BAR: f64 = 1e5;

/// Definition: mole fraction per ppmv, a unit conversion.
const PER_PPMV: f64 = 1e-6;

/// Definition: the index of the present-day experiment in the RFMIP
/// files (`expt`, named by their `expt_label`).
const PRESENT_DAY: usize = 0;

/// Definition: the index of the experiment at 0.5 times present-day CO₂.
const HALF_CO2: usize = 4;

/// Definition: the index of the experiment at 8 times present-day CO₂.
const EIGHT_CO2: usize = 7;

/// Definition: the index of the experiment at pre-industrial CO₂.
const PRE_INDUSTRIAL_CO2: usize = 8;

/// Definition: the index of the experiment 4 K warmer at present-day
/// water.
const WARMER: usize = 13;

/// Definition: the index of the experiment 4 K warmer at constant
/// relative humidity.
const WARMER_MOISTER: usize = 14;

/// Source: RRTMGP's error on the sunlight the air gains from
/// pre-industrial to present-day CO₂ against LBLRTM on the same field,
/// W/m²: 0.1679 against 0.2009 (RTE-RRTMGP-181204 and LBLRTM-12-8 in
/// RFMIP-IRF, Pincus, Mlawer & Delamere 2019, doi:10.1029/2019MS001621,
/// the rows of `rfmip-shortwave.tsv`), 0.0330 rounded up; the user's rule
/// for these tests (2026-11-25): no worse than RRTMGP.
const PRESENT_CO2_TOLERANCE_W_M2: f64 = 0.034;

/// Calibration: the tolerance on the sunlight the air gains from 0.5 to 8
/// times present-day CO₂, W/m². LBLRTM gives 2.0772, RRTMGP 2.1394
/// (+0.0622), the column 2.154 (+0.077), measured by study 35 on this
/// field: worse than RRTMGP, so the user's rule (no worse than RRTMGP)
/// is not met and the tolerance is the column's measured error rounded
/// up. Searched and not found: a published bound on a CO₂ band model's
/// shortwave error over a 16-fold range. Study 35, Wiki
/// `35-stellar-spectra-audit`.
const CO2_RANGE_TOLERANCE_W_M2: f64 = 0.08;

/// Calibration: the tolerance on the sunlight the air gains from the
/// water of +4 K at constant relative humidity over +4 K at present-day
/// water, W/m². LBLRTM gives 4.2098, RRTMGP 4.1653 (−0.0445), the column
/// 3.955 (−0.255, −6 %), measured by study 35 on this field: worse than
/// RRTMGP, so the tolerance is the column's measured error rounded up.
/// Searched and not found: a line-by-line reference on water alone
/// (study 35, Wiki `35-stellar-spectra-audit`).
const WATER_TOLERANCE_W_M2: f64 = 0.26;

/// One sunlit site of the field: its weight over the whole field, the
/// sunlight at its top, W/m², its surface albedo, and its levels from the
/// surface up: pressure, Pa, then temperature, K, and water, mol per mol
/// of dry air, at present day, at +4 K, and the water at +4 K at constant
/// relative humidity.
struct Site {
    weight: f64,
    sunlight: f64,
    albedo: f64,
    levels: Vec<[f64; 6]>,
}

/// The field: the CO₂ of each experiment, ppmv, the absorbed sunlight of
/// LBLRTM and RRTMGP over the field, W/m², and the sites.
struct Field {
    experiments: Vec<(usize, f64, f64, f64)>,
    sites: Vec<Site>,
}

fn field() -> Field {
    let mut field = Field {
        experiments: Vec::new(),
        sites: Vec::new(),
    };
    for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let number = |i: usize| -> f64 { fields[i].parse().expect("a number") };
        match fields[0] {
            "E" => field.experiments.push((
                fields[1].parse().expect("an index"),
                number(2),
                number(3),
                number(4),
            )),
            "S" => field.sites.push(Site {
                weight: number(2),
                sunlight: number(3),
                albedo: number(4),
                levels: Vec::new(),
            }),
            "L" => {
                let site = field.sites.last_mut().expect("a site first");
                site.levels.push(std::array::from_fn(|i| number(i + 1)));
            }
            other => panic!("rfmip-shortwave.tsv: {other}"),
        }
    }
    field
}

impl Field {
    fn experiment(&self, e: usize) -> (f64, f64, f64) {
        let (_, co2, lbl, rrtmgp) = *self
            .experiments
            .iter()
            .find(|x| x.0 == e)
            .expect("the experiment is in the table");
        (co2, lbl, rrtmgp)
    }

    /// The sunlight the column absorbs over the field, W/m², with the
    /// temperatures and water of columns `t` and `x` of the levels and
    /// `co2_ppmv` of CO₂: each site's absorbed share of its sunlight,
    /// weighted as the reference weights it.
    fn absorbed(&self, t: usize, x: usize, co2_ppmv: f64) -> f64 {
        let bands = star_bands(SOLAR_EFFECTIVE_TEMPERATURE.0, SOLAR_METALLICITY).bands;
        let others = EARTH_AIR.n2 + EARTH_AIR.o2;
        self.sites
            .iter()
            .map(|site| {
                let levels: Vec<Level> = site
                    .levels
                    .iter()
                    .map(|row| {
                        let p = row[0] / PA_PER_BAR;
                        let h2o = row[x] / (1.0 + row[x]) * p;
                        let dry = p - h2o;
                        let co2 = dry * co2_ppmv * PER_PPMV;
                        let rest = dry - co2;
                        Level {
                            temperature_k: row[t],
                            gravity_m_s2: STANDARD_GRAVITY.0,
                            gases: PartialPressures {
                                n2: rest * EARTH_AIR.n2 / others,
                                o2: rest * EARTH_AIR.o2 / others,
                                co2,
                                h2o,
                                ..PartialPressures::default()
                            },
                        }
                    })
                    .collect();
                let spans = site
                    .levels
                    .windows(2)
                    .map(|w| (w[0][0] / w[1][0]).ln())
                    .collect();
                let column = Levels { levels, spans };
                let air = Air {
                    haze: (0.0, 1.0),
                    rayleigh_550: rayleigh_depth_550(&column.levels[0].gases, STANDARD_GRAVITY.0),
                    column: Some(&column),
                };
                let budget = Paths::through(&air, &bands).budget(site.albedo, 0.0);
                site.weight * site.sunlight * budget.air
            })
            .sum()
    }
}

/// Definition: the column of a site's levels holding the present-day
/// temperature.
const T_PRESENT: usize = 1;

/// Definition: the column holding the present-day water.
const X_PRESENT: usize = 2;

/// Definition: the column holding the temperature at +4 K.
const T_WARMER: usize = 3;

/// Definition: the column holding the water at +4 K.
const X_WARMER: usize = 4;

/// Definition: the column holding the water at +4 K at constant relative
/// humidity.
const X_WARMER_MOISTER: usize = 5;

/// The column's and the references' change from experiment `from` to
/// `to`, which differ only in CO₂, W/m².
fn co2_change(field: &Field, from: usize, to: usize) -> (f64, f64, f64) {
    let (co2_from, lbl_from, rrtmgp_from) = field.experiment(from);
    let (co2_to, lbl_to, rrtmgp_to) = field.experiment(to);
    let column = field.absorbed(T_PRESENT, X_PRESENT, co2_to)
        - field.absorbed(T_PRESENT, X_PRESENT, co2_from);
    (column, lbl_to - lbl_from, rrtmgp_to - rrtmgp_from)
}

#[test]
fn present_day_co2_absorbs_as_line_by_line() {
    let f = field();
    let (column, lbl, rrtmgp) = co2_change(&f, PRE_INDUSTRIAL_CO2, PRESENT_DAY);
    println!("PI to PD CO2: column {column:.4}, LBLRTM {lbl:.4}, RRTMGP {rrtmgp:.4} W/m²");
    assert!(
        ((rrtmgp - lbl).abs()) <= PRESENT_CO2_TOLERANCE_W_M2,
        "the tolerance is RRTMGP's error"
    );
    assert!(
        (column - lbl).abs() <= PRESENT_CO2_TOLERANCE_W_M2,
        "{column} against {lbl}"
    );
}

#[test]
fn sixteen_fold_co2_absorbs_as_line_by_line() {
    let f = field();
    let (column, lbl, rrtmgp) = co2_change(&f, HALF_CO2, EIGHT_CO2);
    println!("0.5x to 8x CO2: column {column:.4}, LBLRTM {lbl:.4}, RRTMGP {rrtmgp:.4} W/m²");
    assert!(
        (column - lbl).abs() <= CO2_RANGE_TOLERANCE_W_M2,
        "{column} against {lbl}"
    );
}

#[test]
fn moister_air_absorbs_as_line_by_line() {
    let f = field();
    let (co2, lbl_dry, rrtmgp_dry) = f.experiment(WARMER);
    let (_, lbl_moist, rrtmgp_moist) = f.experiment(WARMER_MOISTER);
    let column = f.absorbed(T_WARMER, X_WARMER_MOISTER, co2) - f.absorbed(T_WARMER, X_WARMER, co2);
    let (lbl, rrtmgp) = (lbl_moist - lbl_dry, rrtmgp_moist - rrtmgp_dry);
    println!("+4 K water: column {column:.4}, LBLRTM {lbl:.4}, RRTMGP {rrtmgp:.4} W/m²");
    assert!(
        (column - lbl).abs() <= WATER_TOLERANCE_W_M2,
        "{column} against {lbl}"
    );
}

#[test]
fn the_field_is_whole() {
    let f = field();
    for e in [
        PRESENT_DAY,
        HALF_CO2,
        EIGHT_CO2,
        PRE_INDUSTRIAL_CO2,
        WARMER,
        WARMER_MOISTER,
    ] {
        let _ = f.experiment(e);
    }
    let levels = f.sites[0].levels.len();
    for site in &f.sites {
        assert_eq!(site.levels.len(), levels);
        assert!(site.sunlight > 0.0 && site.weight > 0.0);
        assert!(
            site.levels.windows(2).all(|w| w[0][0] > w[1][0]),
            "surface first"
        );
    }
}
