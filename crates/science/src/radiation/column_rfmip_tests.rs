//! The infrared column on RFMIP's 100 clear-sky profiles
//! (`crates/science/data/rfmip-longwave.tsv`, built by
//! `tools/rfmip-shortwave`): methane's and N₂O's steps from
//! pre-industrial to present day against the line-by-line codes, with
//! the tolerance of study 36 (`docs/science/longwave-absorbers.md`,
//! "Decisions"). Every column holds RFMIP's N₂O, as the line-by-line
//! codes' air does: N₂O's band overlaps methane's.

use super::*;
use crate::constants::{AVOGADRO, PASCALS_PER_BAR, STANDARD_GRAVITY};
use crate::radiation::ozone::molecules_per_cm2;

/// The longwave field: RFMIP's 100 sites at present day, with the
/// well-mixed gases of each experiment it holds.
const FIELD: &str = include_str!("../../data/rfmip-longwave.tsv");

/// Definition: the index of RFMIP's present-day experiment (`expt_label`
/// "Present day (PD)").
const PRESENT_DAY: usize = 0;

/// Definition: the index of RFMIP's pre-industrial methane experiment
/// (`expt_label` "PI CH4", 808.249 ppb against 1831.471).
const PI_METHANE: usize = 9;

/// Definition: the index of RFMIP's pre-industrial N₂O experiment
/// (`expt_label` "PI N2O", 273.021 ppb against 326.988).
const PI_N2O: usize = 10;

/// Source: methane's step from pre-industrial to present day on RFMIP's
/// 100 profiles, the clear-sky outgoing infrared of "PI CH4" less that
/// of "PD", weighted to the global mean: 0.613 W/m² as the mean of five
/// line-by-line codes, s.d. 0.003 (Pincus et al. 2020, *J. Geophys.
/// Res. Atmos.* 125, e2020JD033483, doi:10.1029/2020JD033483, their
/// published fluxes, `docs/science/longwave-absorbers.md`).
const LINE_BY_LINE_W_M2: f64 = 0.613;

/// Source: the same step by RRTMGP, the k-distribution of the same
/// group on the same profiles, 0.563 W/m² (Pincus et al. 2020's fluxes):
/// the tolerance study 36 decided (`longwave-absorbers.md`,
/// "Decisions"), no worse than RRTMGP against the line-by-line codes,
/// `|0.563 / 0.613 − 1|`, 8.2 %.
const RRTMGP_W_M2: f64 = 0.563;

/// Source: N₂O's step from pre-industrial to present day on the same
/// profiles, "PI N2O" less "PD": 0.204 W/m² as the mean of the five
/// line-by-line codes, s.d. 0.001 (Pincus et al. 2020's published
/// fluxes, `docs/science/longwave-absorbers.md`).
const N2O_LINE_BY_LINE_W_M2: f64 = 0.204;

/// Source: the same N₂O step by RRTMGP, 0.180 W/m² (Pincus et al.
/// 2020's fluxes): study 36's tolerance, `|0.180 / 0.204 − 1|`, 11.8 %.
const N2O_RRTMGP_W_M2: f64 = 0.180;

/// Source: the mean molar mass of the U.S. Standard Atmosphere 1976's
/// dry air, 28.964 g/mol (NASA-TM-X-74335), kg/mol: the air whose
/// weight carries a site's ozone.
const DRY_AIR_MOLAR_MASS: f64 = 28.964e-3;

/// Definition: a mole fraction per ppm.
const PER_PPM: f64 = 1e-6;
/// Definition: a mole fraction per ppb.
const PER_PPB: f64 = 1e-9;

/// One experiment's well-mixed gases, as mole fractions.
#[derive(Clone, Copy, Debug)]
struct Gases {
    expt: usize,
    co2: f64,
    ch4: f64,
    o2: f64,
    n2o: f64,
}

/// One site: its weight (over the 100 sites), surface temperature and
/// levels from the surface up: pressure (Pa), temperature (K), water and
/// ozone mole fractions.
struct Site {
    weight: f64,
    surface_k: f64,
    levels: Vec<[f64; 4]>,
}

fn field() -> (Vec<Gases>, Vec<Site>) {
    let (mut gases, mut sites) = (Vec::new(), Vec::<Site>::new());
    for line in FIELD.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<&str> = line.split('\t').collect();
        let f = |i: usize| v[i].parse::<f64>().expect("a number");
        match v[0] {
            "X" => gases.push(Gases {
                expt: v[1].parse().expect("an index"),
                co2: f(2) * PER_PPM,
                ch4: f(3) * PER_PPB,
                o2: f(5),
                n2o: f(6) * PER_PPB,
            }),
            "S" => sites.push(Site {
                weight: f(2),
                surface_k: f(3),
                levels: Vec::new(),
            }),
            "L" => sites
                .last_mut()
                .expect("a site")
                .levels
                .push([f(1), f(2), f(3), f(4)]),
            other => panic!("unknown row {other}"),
        }
    }
    (gases, sites)
}

/// `y` at `ln p` linearly between a site's levels (surface first, `p`
/// falling), held at the ends.
fn at(levels: &[[f64; 4]], p_pa: f64, field: usize) -> f64 {
    let x = p_pa.ln();
    if x >= levels[0][0].ln() {
        return levels[0][field];
    }
    for w in levels.windows(2) {
        let (a, b) = (w[0][0].ln(), w[1][0].ln());
        if x <= a && x >= b {
            return w[0][field] + (x - a) / (b - a) * (w[1][field] - w[0][field]);
        }
    }
    levels[levels.len() - 1][field]
}

/// The ozone column of a site, Dobson units: `∫ x dp N_A / (M̄ g)` over
/// its levels (trapezoids), in a dry air of [`DRY_AIR_MOLAR_MASS`] on
/// the standard gravity.
fn ozone_column(site: &Site) -> DobsonUnit {
    let molar = DRY_AIR_MOLAR_MASS;
    let per_m2: f64 = site
        .levels
        .windows(2)
        .map(|w| 0.5 * (w[0][3] + w[1][3]) * (w[0][0] - w[1][0]))
        .sum::<f64>()
        * AVOGADRO
        / (molar * STANDARD_GRAVITY.0);
    DobsonUnit(per_m2 / (molecules_per_cm2(DobsonUnit(1.0)) * 1e4))
}

/// The clear-sky outgoing infrared of `site` under `gases`, W/m², on the
/// climate's levels per decade from its surface up to its top level:
/// temperature and water linear in `ln p` between its levels, the
/// ground a black body at its surface temperature, its ozone column laid
/// out as the app lays it ([`OzoneProfile`]). Water is a mole fraction
/// of the dry air (RFMIP's convention); N₂ is the rest of the dry air,
/// argon in its place.
fn outgoing(site: &Site, gases: Gases, water_scale: f64) -> f64 {
    let (surface, top) = (site.levels[0][0], site.levels[site.levels.len() - 1][0]);
    let span = (surface / top).ln();
    let step = std::f64::consts::LN_10 / LEVELS_PER_DECADE;
    let n = (span / step).ceil() as usize;
    let last = span - (n - 1) as f64 * step;
    let mut temperature = Vec::with_capacity(n + 1);
    let mut levels = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let p_pa = if i == n {
            top
        } else {
            surface * (-(i as f64) * step).exp()
        };
        let p = p_pa / PASCALS_PER_BAR;
        let x = at(&site.levels, p_pa, 2) * water_scale;
        let h2o = x / (1.0 + x) * p;
        let dry = p - h2o;
        let (co2, ch4, o2) = (gases.co2 * dry, gases.ch4 * dry, gases.o2 * dry);
        temperature.push(if i == 0 {
            site.surface_k
        } else {
            at(&site.levels, p_pa, 1)
        });
        levels.push(PartialPressures {
            n2: dry - co2 - ch4 - o2,
            co2,
            h2o,
            ch4,
            o2,
            ..PartialPressures::default()
        });
    }
    let column = Column {
        temperature: &temperature,
        levels: &levels,
        step,
        last,
        gravity_m_s2: STANDARD_GRAVITY.0,
        has_lines: [gases.co2 > 0.0, true, gases.ch4 > 0.0, false, false],
        ozone: OzoneProfile::NONE,
        n2o: 0.0,
        cloud: None,
    };
    let with_ozone = column.with_ozone(ozone_column(site));
    let column = with_ozone.with_n2o(gases.n2o);
    radiate(&column, &mut Scratch::new()).iter().sum()
}

/// The weighted mean over the sites of `f`.
fn mean(sites: &[Site], f: impl Fn(&Site) -> f64) -> f64 {
    let total: f64 = sites.iter().map(|s| s.weight).sum();
    sites.iter().map(|s| s.weight * f(s)).sum::<f64>() / total
}

#[test]
#[ignore = "heavy: 300 columns, about 90 s in a debug build"]
fn methanes_and_n2os_rfmip_steps_are_within_rrtmgps_error() {
    let (gases, sites) = field();
    assert_eq!(sites.len(), 100);
    let of = |expt| {
        *gases
            .iter()
            .find(|g| g.expt == expt)
            .expect("an experiment")
    };
    let olr_pd = mean(&sites, |s| outgoing(s, of(PRESENT_DAY), 1.0));
    let ozone = mean(&sites, |s| ozone_column(s).0);
    println!("RFMIP: mean ozone {ozone:.1} DU; clear-sky OLR PD {olr_pd:.3} W/m²");
    let mut failed = Vec::new();
    for (name, expt, line_by_line, rrtmgp) in [
        ("CH4", PI_METHANE, LINE_BY_LINE_W_M2, RRTMGP_W_M2),
        ("N2O", PI_N2O, N2O_LINE_BY_LINE_W_M2, N2O_RRTMGP_W_M2),
    ] {
        let olr_pi = mean(&sites, |s| outgoing(s, of(expt), 1.0));
        let step = olr_pi - olr_pd;
        let tolerance = (rrtmgp / line_by_line - 1.0).abs();
        println!(
            "RFMIP: PI {name} {olr_pi:.3} W/m²; step {step:.4} against {line_by_line} ± {:.4}",
            tolerance * line_by_line
        );
        if (step / line_by_line - 1.0).abs() > tolerance {
            failed.push(format!(
                "{name}: {step} W/m² against {line_by_line} within {tolerance}"
            ));
        }
    }
    assert!(failed.is_empty(), "{failed:?}");
}
