//! Tier 3: the relief of whole worlds at 60 km cells, measured against
//! the published statistics of Earth, Mars and Venus. The planetary values
//! below are test inputs, not engine constants.

use science::constants::{EARTH_MEAN_RADIUS, MARS_SURFACE_GRAVITY, STANDARD_GRAVITY};
use science::relief::crust::{MARS_CRUST, floating_height};
use science::units::{EarthRadius, Fraction, Metre, MetrePerSecondSquared};
use timeline::fixtures::{earth_like_fixture, leaf};
use timeline::{PlanetState, Seed, TectonicRegime, Trace};
use world::Values;
use world::grid::Grid;
use world::relief::generate;
use world::stats::{area_mean, area_quantile};

const SEEDS: [u64; 3] = [1, 2, 3];
const CELL: Metre = Metre(60e3);

fn body(
    trace: &mut Trace,
    regime: TectonicRegime,
    radius: Metre,
    gravity: f64,
    ocean_share: Option<f64>,
) -> PlanetState {
    let mut state = earth_like_fixture(trace, 4.57);
    state.bulk.radius = leaf(trace, "bulk.radius", EarthRadius::from(radius));
    state.bulk.surface_gravity = leaf(trace, "bulk.gravity", MetrePerSecondSquared(gravity));
    let surface = state.surface.as_mut().unwrap();
    surface.regime = leaf(trace, "surface.regime", regime);
    match ocean_share {
        Some(share) => {
            let liquid = surface.liquid.as_mut().unwrap();
            liquid.ocean_share = leaf(trace, "liquid.share", Fraction::new(share).unwrap());
        }
        None => surface.liquid = None,
    }
    state
}

/// Earth: mean radius 6371.0 km, standard gravity 9.80665 m/s^2 (NASA
/// Planetary Fact Sheet, <https://nssdc.gsfc.nasa.gov/planetary/factsheet/>;
/// the constants of the science crate). Plates, 71 % ocean.
fn earth(trace: &mut Trace) -> PlanetState {
    body(
        trace,
        TectonicRegime::Plates,
        EARTH_MEAN_RADIUS,
        STANDARD_GRAVITY.0,
        Some(0.71),
    )
}

/// Mars: volumetric mean radius 3389.5 km, mean surface gravity
/// [`MARS_SURFACE_GRAVITY`] (NASA Mars Fact Sheet). A stagnant lid.
fn mars(trace: &mut Trace, ocean_share: Option<f64>) -> PlanetState {
    body(
        trace,
        TectonicRegime::Stagnant,
        Metre(3389.5e3),
        MARS_SURFACE_GRAVITY.0,
        ocean_share,
    )
}

/// Venus: volumetric mean radius 6051.8 km, equatorial gravity 8.87 m/s^2
/// (NASA Planetary Fact Sheet, Venus page); no liquid, episodic
/// resurfacing.
fn venus(trace: &mut Trace) -> PlanetState {
    body(trace, TectonicRegime::Episodic, Metre(6051.8e3), 8.87, None)
}

fn grid_of(state: &PlanetState) -> Grid {
    Grid::for_radius(Metre::from(*state.bulk.radius.value()), CELL)
}

fn heights(state: &PlanetState, seed: u64) -> (Grid, Vec<f32>) {
    let grid = grid_of(state);
    let field = generate(state, Seed::new(seed), &grid).unwrap();
    let Values::F32(h) = field.values else {
        panic!("heights are f32")
    };
    (grid, h)
}

/// Share of the area of the cells satisfying `inside` among those
/// satisfying `within`.
fn share(
    grid: &Grid,
    h: &[f32],
    within: impl Fn(f32) -> bool,
    inside: impl Fn(f32) -> bool,
) -> f64 {
    let (mut part, mut whole) = (0.0, 0.0);
    for (k, &v) in h.iter().enumerate() {
        if within(v) {
            let a = grid.area(k);
            whole += a;
            if inside(v) {
                part += a;
            }
        }
    }
    part / whole
}

fn extremes(h: &[f32]) -> (f32, f32) {
    h.iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)))
}

fn report(label: &str, seed: u64, line: &str) {
    eprintln!("{label} seed {seed}: {line}");
}

#[test]
#[ignore = "heavy: world at 60 km"]
fn earth_like_has_earths_hypsometry() {
    // ETOPO 2022 (NOAA NCEI, doi:10.25921/fd45-gt74), as the legacy test
    // (generation.rs:3202). Asserted: the land share is one minus the
    // ocean share, and over 70 % of the sea floor is deeper than 1 km.
    // The other published shares are printed, not asserted, because W2
    // lacks the process that sets them (docs/science/mountains-and-gravity.md,
    // "Where it departs"):
    // - land under 500 m (45-75 %), above 2 km (4-15 %) and above 4 km
    //   (0.5-4 %): no erosion and sedimentation, which pile the lowlands up
    //   near sea level and wear the high land down (follow-up issue: erosion
    //   and sedimentation);
    // - sea floor deeper than 6 km (under 3 %): no sea-floor age-depth law
    //   for the deep floor (follow-up issue: sea-floor age-depth).
    let mut failures = Vec::new();
    for seed in SEEDS {
        let mut trace = Trace::new();
        let (grid, h) = heights(&earth(&mut trace), seed);
        let land = |v: f32| v > 0.0;
        let sea = |v: f32| v <= 0.0;
        let total = share(&grid, &h, |_| true, land);
        let low = share(&grid, &h, land, |v| v < 500.0);
        let high = share(&grid, &h, land, |v| v > 2000.0);
        let top = share(&grid, &h, land, |v| v > 4000.0);
        let trench = share(&grid, &h, sea, |v| v < -6000.0);
        let deep = share(&grid, &h, sea, |v| v < -1000.0);
        let (min, max) = extremes(&h);
        report(
            "earth",
            seed,
            &format!(
                "land {total:.3}, <500m {low:.3}, >2km {high:.3}, >4km {top:.3}, \
                 sea >6km {trench:.4}, sea >1km {deep:.3}, min {min:.0}, max {max:.0}"
            ),
        );
        let mut check = |name: &str, ok: bool| {
            if !ok {
                failures.push(format!("seed {seed}: {name}"));
            }
        };
        check("land share is 1 - ocean share", (total - 0.29).abs() < 0.01);
        check("sea floor deeper than 1 km over 0.70", deep > 0.70);
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
#[ignore = "heavy: world at 60 km"]
fn mars_like_keeps_its_highland_share() {
    // Printed, not asserted. Highland share 0.58 (Mars's southern
    // highlands, the science crate's continental share); the legacy test
    // accepts 0.55-0.61 (generation.rs:3436). W2 lacks the dichotomy: the
    // highlands are cut from the noise's area quantile, not placed as a
    // hemispheric step (follow-up issue: the Mars dichotomy census).
    // Definition: the world is wet (the crust step only exists with liquid,
    // as in the legacy test). The two crust levels are the area-weighted
    // medians of the lowland and highland populations (quantiles
    // 0.5 x (1 - 0.58) and 1 - 0.5 x 0.58 of the heights); the highland
    // share is the area above the midpoint between them.
    for seed in SEEDS {
        let mut trace = Trace::new();
        let (grid, h) = heights(&mars(&mut trace, Some(0.3)), seed);
        let low = area_quantile(&grid, &h, 0.5 * (1.0 - 0.58));
        let high = area_quantile(&grid, &h, 1.0 - 0.5 * 0.58);
        let line = 0.5 * (low + high);
        let highland = share(&grid, &h, |_| true, |v| v > line);
        let step = floating_height(&MARS_CRUST, MARS_CRUST.continental_thickness).0;
        report(
            "mars",
            seed,
            &format!("highland {highland:.3} (levels {low:.0} / {high:.0} m, step {step:.0} m)"),
        );
    }
}

#[test]
#[ignore = "heavy: world at 60 km"]
fn venus_like_is_unimodal() {
    // Ford & Pettengill 1992 (JGR 97 E8, doi:10.1029/92JE01165): 80 % of
    // the surface of Venus lies within 1 km of the mean radius.
    let mut failures = Vec::new();
    for seed in SEEDS {
        let mut trace = Trace::new();
        let (grid, h) = heights(&venus(&mut trace), seed);
        let mean = area_mean(&grid, &h);
        let near = share(&grid, &h, |_| true, |v| (v - mean).abs() < 1000.0);
        let (min, max) = extremes(&h);
        report(
            "venus",
            seed,
            &format!("within 1 km {near:.3}, min {min:.0}, max {max:.0}"),
        );
        if near < 0.8 {
            failures.push(format!("seed {seed}: within 1 km {near}"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

/// Mean Haar fluctuation, in metres, of the north-south profiles of `h`
/// at each lag (in samples): the study's method
/// (`scripts/etopo_haar_split.py`), the absolute difference of the means
/// of the two halves of every window, averaged over windows and
/// meridians.
fn haar(grid: &Grid, h: &[f32], step: f64, lags: &[usize]) -> Vec<f64> {
    let samples = (std::f64::consts::PI / step).floor() as usize;
    let meridians = 180;
    let mut sum = vec![0.0; lags.len()];
    let mut count = vec![0usize; lags.len()];
    for m in 0..meridians {
        let lon = std::f64::consts::TAU * f64::from(m) / f64::from(meridians);
        let mut cumulative = vec![0.0f64];
        for s in 0..samples {
            let lat = -std::f64::consts::FRAC_PI_2 + (s as f64 + 0.5) * step;
            let p = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
            let v = f64::from(h[grid.index_of(p)]);
            cumulative.push(cumulative[s] + v);
        }
        for (i, &lag) in lags.iter().enumerate() {
            let half = lag / 2;
            for a in 0..=samples - lag {
                let first = cumulative[a + half] - cumulative[a];
                let second = cumulative[a + lag] - cumulative[a + half];
                sum[i] += ((second - first) / half as f64).abs();
                count[i] += 1;
            }
        }
    }
    sum.iter().zip(&count).map(|(s, &c)| s / c as f64).collect()
}

/// The Hurst exponent of the field of `state`, and the fluctuations it
/// comes from.
fn hurst(state: &PlanetState, seed: u64) -> (f64, Vec<f64>) {
    let radius = Metre::from(*state.bulk.radius.value());
    let (grid, h) = heights(state, seed);
    // Lags of 4 to 32 samples, 240 to 1920 km: above the 4 cells under
    // which a 60 km grid does not resolve its own structure, below the
    // scales where the finite sphere bends the profiles.
    let step = CELL.0 / radius.0;
    let lags = [4usize, 8, 16, 32];
    let f = haar(&grid, &h, step, &lags);
    // Least-squares slope of log fluctuation against log lag.
    let x: Vec<f64> = lags.iter().map(|&l| (l as f64).ln()).collect();
    let y: Vec<f64> = f.iter().map(|v| v.ln()).collect();
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let slope = x
        .iter()
        .zip(&y)
        .map(|(a, b)| (a - mx) * (b - my))
        .sum::<f64>()
        / x.iter().map(|a| (a - mx).powi(2)).sum::<f64>();
    (slope, f)
}

#[test]
#[ignore = "heavy: world at 60 km"]
fn relief_has_its_analogues_hurst_exponent() {
    // Landais, Schmidt & Lovejoy 2019 (Icarus 319,
    // doi:10.1016/j.icarus.2018.07.005), Table 2: H above about 10 km is
    // 0.479 on Earth and 0.53 on Mars. Earth's is asserted; Mars's is
    // printed only: W2 lacks the dichotomy census (follow-up issue: the
    // Mars dichotomy census), which sets the large-scale Mars fluctuation.
    let mut failures = Vec::new();
    for seed in SEEDS {
        let mut trace = Trace::new();
        let (e, ef) = hurst(&earth(&mut trace), seed);
        let (m, mf) = hurst(&mars(&mut trace, None), seed);
        report("earth H", seed, &format!("{e:.3}, Haar {ef:.0?} m"));
        report("mars H", seed, &format!("{m:.3}, Haar {mf:.0?} m"));
        if (e - 0.479).abs() > 0.08 {
            failures.push(format!("earth seed {seed}: H {e}"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
#[ignore = "heavy: world at 60 km"]
fn relief_never_saturates() {
    // Earth-like at 3 g and Mars-like at 0.2 g (multiples of the standard
    // gravity): every height stays within the +-25 km of the storage range.
    let mut failures = Vec::new();
    for seed in SEEDS {
        let mut trace = Trace::new();
        let heavy = body(
            &mut trace,
            TectonicRegime::Plates,
            EARTH_MEAN_RADIUS,
            3.0 * STANDARD_GRAVITY.0,
            Some(0.71),
        );
        let light = body(
            &mut trace,
            TectonicRegime::Stagnant,
            Metre(3389.5e3),
            0.2 * STANDARD_GRAVITY.0,
            None,
        );
        for (label, state) in [("earth 3 g", heavy), ("mars 0.2 g", light)] {
            let (_, h) = heights(&state, seed);
            let (min, max) = extremes(&h);
            report(label, seed, &format!("min {min:.0}, max {max:.0}"));
            if min < -25_000.0 || max > 25_000.0 {
                failures.push(format!("{label} seed {seed}: {min} .. {max}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
#[ignore = "heavy: world at 60 km"]
fn same_seed_same_relief() {
    let mut trace = Trace::new();
    let state = earth(&mut trace);
    let grid = grid_of(&state);
    let a = generate(&state, Seed::new(1), &grid).unwrap();
    let b = generate(&state, Seed::new(1), &grid).unwrap();
    let c = generate(&state, Seed::new(2), &grid).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}
