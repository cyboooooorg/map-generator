//! The infrared column on RFMIP's 100 clear-sky profiles
//! (`crates/science/data/rfmip-longwave.tsv`, built by
//! `tools/rfmip-shortwave`): methane's and N₂O's steps from
//! pre-industrial to present day against the line-by-line codes, with
//! the tolerance of study 36 (`docs/science/longwave-absorbers.md`,
//! "Decisions"). Every column holds RFMIP's N₂O, as the line-by-line
//! codes' air does: N₂O's band overlaps methane's.

use super::*;
use crate::constants::{PASCALS_PER_BAR, STANDARD_GRAVITY};
use crate::radiation::test_points::dobson_column;

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
pub(super) const LINE_BY_LINE_W_M2: f64 = 0.613;

/// Source: the same step by RRTMGP, the k-distribution of the same
/// group on the same profiles, 0.563 W/m² (Pincus et al. 2020's fluxes):
/// the tolerance study 36 decided (`longwave-absorbers.md`,
/// "Decisions"), no worse than RRTMGP against the line-by-line codes,
/// `|0.563 / 0.613 − 1|`, 8.2 %.
pub(super) const RRTMGP_W_M2: f64 = 0.563;

/// Source: N₂O's step from pre-industrial to present day on the same
/// profiles, "PI N2O" less "PD": 0.204 W/m² as the mean of the five
/// line-by-line codes, s.d. 0.001 (Pincus et al. 2020's published
/// fluxes, `docs/science/longwave-absorbers.md`).
const N2O_LINE_BY_LINE_W_M2: f64 = 0.204;

/// Source: the same N₂O step by RRTMGP, 0.180 W/m² (Pincus et al.
/// 2020's fluxes): study 36's tolerance, `|0.180 / 0.204 − 1|`, 11.8 %.
const N2O_RRTMGP_W_M2: f64 = 0.180;

/// Source: the clear-sky outgoing infrared of RFMIP's 100 profiles,
/// weighted to the global mean, W/m², by the five line-by-line codes of
/// Pincus et al. 2020 (*J. Geophys. Res. Atmos.* 125, e2020JD033483,
/// doi:10.1029/2020JD033483; their published fluxes, `lw-lbl-models.nc`
/// of `RobertPincus/rfmip-benchmark-paper-figures` at `76f2bff4`, the top
/// level's `rlu`): ARTS 2.3, GRTCODE, RFM-DISORT, LBLRTM 12.8 and 4A/OP
/// 1.5, each `p1-f1` but RFM-DISORT's `p1-f2` (its only one). Each pair
/// is present day ("PD", experiment 0) and "PI HCs" (experiment 12),
/// present day with the halocarbons at their pre-industrial amounts: the
/// column has no halocarbons, so the test reads the second. LBLRTM's
/// equal its ESGF files' (263.2287 and 263.7138,
/// `crates/science/data/RFMIP.md`).
const LINE_BY_LINE_FIELD_W_M2: [[f64; 2]; 5] = [
    [261.8933, 262.4495],
    [262.9521, 263.5135],
    [263.2488, 263.7986],
    [263.2287, 263.7139],
    [263.1728, 263.6705],
];

/// Source: the halocarbons of RFMIP's present day and "PI HCs", ppt, as
/// the equivalents the input file gives (`cfc11eq_GM`, `cfc12eq_GM`,
/// `hfc134aeq_GM`, experiments 0 and 12; Pincus et al. 2016, *Geosci.
/// Model Dev.* 9, 3447, the input file of `crates/science/data/RFMIP.md`).
/// "PI HCs" keeps their pre-industrial amounts (methyl chloride and
/// bromide, CF₄), which the column lacks too.
const HALOCARBON_EQUIVALENTS_PPT: [[f64; 2]; 3] = [
    [809.18646, 32.1102],
    [1049.5128, 16.512882],
    [257.0642, 19.15331],
];

/// Calibration: how far the light [`Ground::sent`] takes the air to send
/// down to RFMIP's grey grounds (an air at the ground's temperature)
/// exceeds what the air sends down, in the outgoing infrared of the
/// field: 0.0968 W/m², measured by
/// `the_grounds_reflection_bound_is_within_its_calibration` (the
/// double-precision sum from the ground up), rounded up. A term of the
/// field test's tolerance (`docs/science/radiative-transfer.md`, "The
/// radiation on RFMIP's field").
const REFLECTION_BOUND_W_M2: f64 = 0.1;

/// Definition: a mole fraction per ppm.
const PER_PPM: f64 = 1e-6;
/// Definition: a mole fraction per ppb.
const PER_PPB: f64 = 1e-9;

/// Study 36's tolerance of a k-distribution against the line-by-line
/// codes on a step, relative: RRTMGP's error on it,
/// `|rrtmgp / line_by_line − 1|` (`docs/science/longwave-absorbers.md`,
/// "Decisions").
pub(super) fn rrtmgp_error(rrtmgp: f64, line_by_line: f64) -> f64 {
    (rrtmgp / line_by_line - 1.0).abs()
}

/// One experiment's well-mixed gases, as mole fractions.
#[derive(Clone, Copy, Debug)]
struct Gases {
    expt: usize,
    co2: f64,
    ch4: f64,
    o2: f64,
    n2o: f64,
}

/// One site: its weight (over the 100 sites), its ground's temperature
/// and infrared emissivity, and its levels from the surface up: pressure
/// (Pa), temperature (K), water and ozone mole fractions.
struct Site {
    weight: f64,
    surface_k: f64,
    emissivity: f64,
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
                emissivity: f(4),
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

/// The ozone column of a site, Dobson units ([`dobson_column`]): its
/// mixing ratio linear in `ln p` between its levels, as [`at`] reads its
/// temperature and water, held at its top above it.
fn ozone_column(site: &Site) -> DobsonUnit {
    DobsonUnit(dobson_column(
        |p| at(&site.levels, p * PASCALS_PER_BAR, 3) * p,
        site.levels[0][0] / PASCALS_PER_BAR,
        STANDARD_GRAVITY.0,
    ))
}

/// The clear-sky outgoing infrared of `site` under `gases`, W/m², on the
/// climate's levels per decade from its surface up to its top level:
/// temperature and water linear in `ln p` between its levels, the
/// ground grey at its surface temperature and emissivity ([`Ground`]),
/// its ozone column laid out as the app lays it ([`OzoneProfile`]). Water is a mole fraction
/// of the dry air (RFMIP's convention); N₂ is the rest of the dry air,
/// argon in its place.
fn outgoing(site: &Site, gases: Gases, water_scale: f64) -> f64 {
    with_site_column(site, gases, water_scale, |column| {
        radiate(column, &mut Scratch::new()).iter().sum()
    })
}

/// `use_column` of the column [`outgoing`] radiates.
fn with_site_column<R>(
    site: &Site,
    gases: Gases,
    water_scale: f64,
    use_column: impl FnOnce(&Column) -> R,
) -> R {
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
        temperature.push(at(&site.levels, p_pa, 1));
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
        ground: None,
    };
    let with_ozone = column.with_ozone(ozone_column(site));
    let with_n2o = with_ozone.with_n2o(gases.n2o);
    let column = with_n2o.with_ground(Ground {
        temperature_k: site.surface_k,
        emissivity: site.emissivity,
    });
    use_column(&column)
}

/// The weighted mean over the sites of `f`.
fn mean(sites: &[Site], f: impl Fn(&Site) -> f64) -> f64 {
    let total: f64 = sites.iter().map(|s| s.weight).sum();
    sites.iter().map(|s| s.weight * f(s)).sum::<f64>() / total
}

/// The mean and the sample standard deviation of `xs`.
fn mean_and_deviation(xs: &[f64]) -> (f64, f64) {
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let variance = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, variance.sqrt())
}

#[test]
#[ignore = "heavy: 100 columns, about 25 s in a debug build (54 s on CI); moved to tier 3 by the user on 2026-11-27 to keep tier 1 in its budget"]
fn the_columns_clear_sky_is_the_line_by_line_codes_on_rfmips_field() {
    // Study 40 and the user's decision of 2026-11-27 (issue #704): the
    // column's radiation on RFMIP's 100 present-day profiles, each at its
    // own temperatures and over its own grey ground, against the five
    // line-by-line codes on the same air without its halocarbons ("PI
    // HCs"). The single column of the climate is darker than CERES: its
    // profile, not its radiation (track T11, issue #757).
    //
    // The tolerance is what the inputs leave unknown, each measured on
    // the field (a calibration, `docs/science/radiative-transfer.md`):
    // the codes' spread, the ground's reflection bound, and the
    // pre-industrial halocarbons "PI HCs" keeps, which the column lacks
    // (their removal is linear in their amounts, so they take at most the
    // largest share of the three equivalents of what removing the rest
    // takes).
    let (gases, sites) = field();
    let present = *gases
        .iter()
        .find(|g| g.expt == PRESENT_DAY)
        .expect("present day");
    let olr = mean(&sites, |s| outgoing(s, present, 1.0));
    let without: Vec<f64> = LINE_BY_LINE_FIELD_W_M2
        .iter()
        .map(|[_, hcs]| hcs)
        .copied()
        .collect();
    let (expected, spread) = mean_and_deviation(&without);
    let removal = LINE_BY_LINE_FIELD_W_M2
        .iter()
        .map(|[pd, hcs]| hcs - pd)
        .sum::<f64>()
        / LINE_BY_LINE_FIELD_W_M2.len() as f64;
    let share = HALOCARBON_EQUIVALENTS_PPT
        .iter()
        .map(|[pd, pi]| pi / (pd - pi))
        .fold(0.0, f64::max);
    let tolerance = spread + REFLECTION_BOUND_W_M2 + share * removal;
    println!(
        "RFMIP's field: the column {olr:.3} W/m², the codes {expected:.3} (s.d. {spread:.3}) \
         without the halocarbons ({removal:.3} of them, {:.3} left), within {tolerance:.3}",
        share * removal
    );
    assert!(
        (olr - expected).abs() <= tolerance,
        "{olr} against {expected} within {tolerance}"
    );
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
        let tolerance = rrtmgp_error(rrtmgp, line_by_line);
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

#[test]
#[ignore = "heavy: 100 columns in double precision, from the ground up"]
fn the_grounds_reflection_bound_is_within_its_calibration() {
    let (gases, sites) = field();
    let present = *gases
        .iter()
        .find(|g| g.expt == PRESENT_DAY)
        .expect("present day");
    let excess = |site: &Site| {
        with_site_column(site, present, 1.0, |column| {
            let (down, total) = ground_downward(column);
            let mut planck = [0.0; BANDS];
            planck_spectrum(site.surface_k, &mut planck);
            let grey = 1.0 - site.emissivity;
            (0..BANDS)
                .map(|k| grey * total[k] * ((1.0 - total[k]) * planck[k] - down[k]))
                .sum::<f64>()
        })
    };
    let bound = mean(&sites, excess);
    println!("RFMIP's field: the ground's reflection bound over the air's {bound:.4} W/m²");
    assert!(
        (0.0..=REFLECTION_BOUND_W_M2).contains(&bound),
        "{bound} against {REFLECTION_BOUND_W_M2}"
    );
}
