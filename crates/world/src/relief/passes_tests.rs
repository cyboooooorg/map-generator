use super::*;
use crate::grid::Grid;
use crate::relief::ReliefError;
use crate::stats::area_quantile;
use science::constants::{EARTH_MEAN_RADIUS, MARS_SURFACE_GRAVITY, STANDARD_GRAVITY};
use science::relief::crust::{CONTINENTAL_SLOPE, crust, floating_height};
use science::relief::highlands::{DRY_PLAINS, MOUNTAIN_RELIEF};
use science::relief::spectrum::{
    Analogue, Province, REFERENCE_WAVELENGTH, amplitude, analogue_relief_scale, hurst, relief_scale,
};
use science::relief::volcanoes::{CONE_ASPECT, CONE_HEIGHT_MAX, EARTH_HEAT_FLUX};
use science::units::{Fraction, WattPerSquareMetre};
use science::units::{Metre, MetrePerSecondSquared};
use timeline::Seed;
use timeline::fixtures::{earth_like_fixture, leaf};
use timeline::{TectonicRegime, Trace};

fn earth() -> Forcing {
    Forcing {
        radius: EARTH_MEAN_RADIUS,
        gravity: STANDARD_GRAVITY,
        analogue: Analogue::Earth,
        plates: true,
        heat_flux: EARTH_HEAT_FLUX,
        ocean_share: Some(Fraction::new(0.71).unwrap()),
    }
}

fn median(mut v: Vec<f32>) -> f32 {
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

#[test]
fn continental_share_lies_above_the_crust_line() {
    let grid = Grid::new(24);
    let (_, mask) = continents(&grid, Seed::new(1), &earth());
    let total: f64 = (0..grid.cell_count()).map(|k| grid.area(k)).sum();
    let land: f64 = (0..grid.cell_count())
        .filter(|&k| mask[k])
        .map(|k| grid.area(k))
        .sum();
    let cell = (0..grid.cell_count())
        .map(|k| grid.area(k))
        .fold(0.0, f64::max);
    // Earth: continental crust 0.41 (Cogley 1984).
    let want = 0.41;
    assert!(
        (land / total - want).abs() <= cell / total,
        "{}",
        land / total
    );
}

#[test]
fn the_continent_floor_step_is_airys() {
    let grid = Grid::new(24);
    let forcing = earth();
    let seed = Seed::new(2);
    let (h, mask) = continents(&grid, seed, &forcing);
    let (noise, _) = continent_noise(&grid, seed.derive("relief.continents"), &forcing);
    let pick = |a: &[f32], on: bool| -> Vec<f32> {
        (0..a.len())
            .filter(|&k| mask[k] == on)
            .map(|k| a[k])
            .collect()
    };
    let step = |a: &[f32]| f64::from(median(pick(a, true)) - median(pick(a, false)));
    // The noise is centred within each province, so the step between them
    // is Airy's alone (the medians differ from the means by a little).
    let platform = step(&h) - step(&noise);
    // Airy: 34 km x (1 - 2830/3380) = 5.53 km (PREM mantle).
    let earth_crust = crust(Analogue::Earth);
    let airy = floating_height(&earth_crust, earth_crust.continental_thickness);
    assert!((airy.0 - 5530.0).abs() < 10.0);
    assert!((platform - airy.0).abs() < 1000.0, "{platform}");
}

#[test]
fn the_continental_edge_keeps_its_slope() {
    let grid = Grid::new(24);
    let forcing = earth();
    let seed = Seed::new(3);
    let (h, mask) = continents(&grid, seed, &forcing);
    let (noise, _) = continent_noise(&grid, seed.derive("relief.continents"), &forcing);
    let radius = forcing.radius.0;
    let slope = |a: &[f32], k: usize, j: usize| {
        let (p, q) = (grid.point(k), grid.point(j));
        let d = radius
            * (p[0] * q[0] + p[1] * q[1] + p[2] * q[2])
                .clamp(-1.0, 1.0)
                .acos();
        f64::from(a[k] - a[j]).abs() / d
    };
    let mut noise_gradient = 0.0_f64;
    let mut edge_slope = 0.0_f64;
    for k in 0..grid.cell_count() {
        for j in grid.neighbours(k) {
            noise_gradient = noise_gradient.max(slope(&noise, k, j));
            if mask[k] != mask[j] {
                edge_slope = edge_slope.max(slope(&h, k, j));
            }
        }
    }
    let limit = CONTINENTAL_SLOPE.0.tan() + noise_gradient;
    assert!(edge_slope > 0.0, "an edge exists");
    assert!(edge_slope <= limit, "{edge_slope} > {limit}");
}

#[test]
fn volcanoes_keep_their_aspect() {
    let grid = Grid::new(24);
    let forcing = earth();
    let arcs = vec![0.5_f32; grid.cell_count()];
    let cones = cones(&grid, Seed::new(4), &forcing, &arcs);
    assert!(cones.len() > 50, "{}", cones.len());
    for c in &cones {
        // Aspect: height over basal width, 2 radii (Grosse & Kervyn 2018).
        let aspect = c.cone.height.0 / (2.0 * c.cone.radius.0);
        assert!(
            (CONE_ASPECT.0 - 1e-9..=CONE_ASPECT.1 + 1e-9).contains(&aspect),
            "{aspect}"
        );
        assert!(c.cone.height.0 <= CONE_HEIGHT_MAX.0 + 1e-6);
    }
    let h = volcanoes(&grid, Seed::new(4), &forcing, &arcs);
    assert!(h.iter().all(|&v| v >= 0.0));
    assert!(h.iter().any(|&v| v > 0.0));
    assert!(f64::from(h.iter().copied().fold(0.0, f32::max)) <= CONE_HEIGHT_MAX.0);
}

#[test]
fn a_cold_planet_has_no_isolated_cones() {
    let grid = Grid::new(24);
    let forcing = Forcing {
        heat_flux: WattPerSquareMetre(0.0),
        ..earth()
    };
    let none = vec![0.0_f32; grid.cell_count()];
    assert!(cones(&grid, Seed::new(5), &forcing, &none).is_empty());
    assert!(
        volcanoes(&grid, Seed::new(5), &forcing, &none)
            .iter()
            .all(|&v| v == 0.0)
    );
}

#[test]
fn forcing_reads_the_regime_not_the_body() {
    let mut trace = Trace::new();
    let mut state = earth_like_fixture(&mut trace, 4.57);
    let f = Forcing::read(&state).unwrap();
    assert!(f.plates);
    assert_eq!(f.analogue, Analogue::Earth);
    assert_eq!(f.ocean_share.map(Fraction::get), Some(0.71));
    // 3.01e13 W through the surface of one Earth radius.
    assert!((f.heat_flux.0 - 0.0588).abs() < 0.001, "{}", f.heat_flux.0);
    state.surface.as_mut().unwrap().regime =
        leaf(&mut trace, "surface.regime", TectonicRegime::Stagnant);
    state.surface.as_mut().unwrap().liquid = None;
    let f = Forcing::read(&state).unwrap();
    assert!(!f.plates);
    assert_eq!(f.analogue, Analogue::Mars);
    assert_eq!(f.ocean_share, None);
    state.surface = None;
    assert_eq!(Forcing::read(&state), Err(ReliefError::NoSurface));
    assert_eq!(ReliefError::NoSurface.to_string(), "relief.no-surface");
}

#[test]
fn a_dry_world_rolls_on_its_plains_and_the_highlands_gate_its_mountains() {
    let grid = Grid::new(24);
    let forcing = Forcing {
        plates: false,
        analogue: Analogue::Mars,
        ocean_share: None,
        ..earth()
    };
    let (h, mask) = continents(&grid, Seed::new(6), &forcing);
    assert!(mask.iter().any(|&m| m) && mask.iter().any(|&m| !m));
    // No platform step on a dry world: heights are the plains' noise only.
    assert!(f64::from(median(h.clone())).abs() < 1500.0);
    let weight = highlands(&grid, Seed::new(6));
    assert!(weight.iter().all(|w| (0.0..=1.0).contains(w)));
    let share = area_quantile(&grid, &weight, 0.5);
    assert_eq!(share, 0.0, "highlands are the minority");
    let m = mountains(&grid, Seed::new(6), &forcing, &weight);
    assert!(m.iter().all(|&v| v >= 0.0));
    assert!(m.iter().zip(&weight).all(|(&v, &w)| w > 0.0 || v == 0.0));
}

#[test]
fn arc_cones_follow_their_density_per_length_of_arc() {
    use science::relief::plates::{ARC_SPEED, Boundary, arc_weight};
    use science::relief::volcanoes::ARC_CONE_DENSITY;
    use science::units::Metre;
    // A small body, so that the cells (39 km) resolve the arc band.
    let grid = Grid::new(24);
    let forcing = Forcing {
        radius: Metre(600e3),
        heat_flux: WattPerSquareMetre(0.0),
        ..earth()
    };
    // An arc along the equator: its weight from the distance to it.
    let equator: Vec<bool> = (0..grid.cell_count())
        .map(|k| grid.point(k)[2].abs() < 0.03)
        .collect();
    let distance = crate::stats::distance_to(&grid, forcing.radius, &equator, None);
    // The arc stands `arc_distance` from the boundary, on its north side.
    let arcs: Vec<f32> = (0..grid.cell_count())
        .map(|k| {
            if grid.point(k)[2] <= 0.0 {
                return 0.0;
            }
            let d = Metre(f64::from(distance[k]));
            arc_weight(Boundary::IslandArc, d, ARC_SPEED).get() as f32
        })
        .collect();
    let seeds = 0..5;
    let count: usize = seeds
        .clone()
        .map(|s| cones(&grid, Seed::new(s), &forcing, &arcs).len())
        .sum();
    let length = 2.0 * std::f64::consts::PI * forcing.radius.0 * seeds.count() as f64;
    let density = count as f64 / length;
    let ratio = density / ARC_CONE_DENSITY;
    assert!((0.5..=2.0).contains(&ratio), "{ratio}");
}

/// The unit fBm the continents' noise of `forcing` is drawn from, per cell:
/// the octaves of [`continent_noise`], rebuilt.
fn unit_noise(grid: &Grid, seed: Seed, forcing: &Forcing) -> (Octaves, Vec<f64>) {
    let spectrum = Spectrum {
        hurst: hurst(forcing.analogue),
        octaves: octaves(forcing.radius, Metre(cell_length(grid, forcing.radius))),
        base_frequency: BASE_FREQUENCY,
    };
    let octs = Octaves::new(seed, &spectrum);
    let unit = (0..grid.cell_count())
        .map(|k| f64::from(octs.fbm(grid.point(k)) as f32))
        .collect();
    (octs, unit)
}

/// The factor taking `unit` to `noise` over the cells of `mask` that are
/// `on`, once the province's mean is removed: a least-squares slope.
fn province_scale(unit: &[f64], noise: &[f32], mask: &[bool], on: bool) -> f64 {
    let ks: Vec<usize> = (0..unit.len()).filter(|&k| mask[k] == on).collect();
    let mean = ks.iter().map(|&k| unit[k]).sum::<f64>() / ks.len() as f64;
    let (xy, xx) = ks.iter().fold((0.0, 0.0), |(xy, xx), &k| {
        let x = unit[k] - mean;
        (xy + x * f64::from(noise[k]), xx + x * x)
    });
    xy / xx
}

#[test]
fn the_noise_is_centred_within_each_province() {
    // The step between the crusts is Airy's alone: the noise has no mean
    // on either province (Airy 1855; diagnosis of the relief amplitude).
    let grid = Grid::new(24);
    for seed in 1..4 {
        let (noise, mask) = continent_noise(&grid, Seed::new(seed), &earth());
        for on in [true, false] {
            let mean = province_mean(&grid, &noise, &mask, on);
            assert!(mean.abs() < 1.0, "seed {seed}, thick {on}: {mean} m");
        }
    }
}

#[test]
fn the_noise_is_scaled_on_its_haar_fluctuation_per_province() {
    // A small body, so that 40 km cells resolve the 100 km lag.
    let grid = Grid::new(24);
    let forcing = Forcing {
        radius: Metre(600e3),
        ..earth()
    };
    let seed = Seed::new(8);
    let (octs, unit) = unit_noise(&grid, seed, &forcing);
    // The study's statistic, measured here independently: the mean of
    // |M2 - M1| over windows one lag long running north, each half
    // averaged over 64 samples.
    let lag = REFERENCE_WAVELENGTH.0;
    let half_mean = |p: [f64; 3], from: f64| {
        (0..64)
            .map(|i| {
                let along = from + (f64::from(i) + 0.5) * lag / 2.0 / 64.0;
                octs.fbm(displaced(p, along / forcing.radius.0))
            })
            .sum::<f64>()
            / 64.0
    };
    let windows: Vec<usize> = (0..grid.cell_count()).step_by(3).collect();
    let haar = windows
        .iter()
        .map(|&k| {
            let p = grid.point(k);
            (half_mean(p, lag / 2.0) - half_mean(p, 0.0)).abs()
        })
        .sum::<f64>()
        / windows.len() as f64;
    let measured = haar_fluctuation(&grid, &octs, &forcing, hurst(forcing.analogue));
    assert!(
        (measured / haar - 1.0).abs() < 0.1,
        "{measured} against {haar}"
    );
    // Each province gets its own measured amplitude (ETOPO 2022, land
    // 131 m and sea floor 246 m at 100 km).
    let (noise, mask) = continent_noise(&grid, seed, &forcing);
    for (on, province) in [(true, Province::Thick), (false, Province::Thin)] {
        let scale = province_scale(&unit, &noise, &mask, on);
        let want = amplitude(Analogue::Earth, province).0 / haar;
        assert!(
            (scale / want - 1.0).abs() < 0.1,
            "{province:?}: {scale} against {want}"
        );
    }
}

#[test]
fn orogens_lift_only_the_continental_crust() {
    // A collision's elevation is a continental plateau's (Tibet, Royden et
    // al. 2008): an oceanic cell under a range's weight keeps its floor.
    let grid = Grid::new(4);
    let n = grid.cell_count();
    let continental: Vec<bool> = (0..n).map(|k| grid.point(k)[0] > 0.0).collect();
    let mut h: Vec<f32> = continental
        .iter()
        .map(|&c| if c { 500.0 } else { -4000.0 })
        .collect();
    let before = h.clone();
    raise_orogens(
        &grid,
        &mut h,
        &vec![1.0; n],
        &vec![5000.0; n],
        &continental,
        None,
    );
    for k in 0..n {
        if continental[k] {
            assert!(h[k] > before[k], "{k}: continental cell not lifted");
        } else {
            assert_eq!(h[k], before[k], "{k}: oceanic cell lifted");
        }
    }
}

#[test]
fn a_height_measured_on_mars_counts_gravity_once() {
    // Mars's amplitude was measured at its own gravity (NASA Mars Fact
    // Sheet, mean surface gravity): there it is taken as is, at Earth's
    // gravity divided by the relief scale of Mars, 2.49.
    let grid = Grid::new(24);
    let mars = |g: f64| Forcing {
        gravity: MetrePerSecondSquared(g),
        analogue: Analogue::Mars,
        plates: false,
        ..earth()
    };
    let seed = Seed::new(9);
    let (own, _) = continent_noise(&grid, seed, &mars(MARS_SURFACE_GRAVITY.0));
    let (earthly, _) = continent_noise(&grid, seed, &mars(STANDARD_GRAVITY.0));
    let want = analogue_relief_scale(STANDARD_GRAVITY, Analogue::Mars);
    assert!((want - 1.0 / 2.49).abs() < 1e-12);
    for (a, b) in own.iter().zip(&earthly).filter(|(a, _)| a.abs() > 1.0) {
        assert!((f64::from(b / a) - want).abs() < 1e-4, "{a} {b}");
    }
}

#[test]
fn dry_plains_are_measured_in_the_legacy_noise_unit() {
    // The legacy's 5-octave noise of gain 1/2, divided by 1.9375: its
    // deviation per unit is 0.314 sqrt(1 + 1/4 + ... + 1/256) / 1.9375.
    let legacy = 0.314 * (1.0f64 + 0.25 + 0.0625 + 0.015625 + 0.00390625).sqrt() / 1.9375;
    assert!((legacy_unit(5, 0.5) - legacy).abs() < 1e-12);
    assert!((legacy - 0.187).abs() < 1e-3, "{legacy}");
    let grid = Grid::new(24);
    let forcing = Forcing {
        gravity: MetrePerSecondSquared(8.87),
        analogue: Analogue::Mars,
        plates: false,
        ocean_share: None,
        ..earth()
    };
    let seed = Seed::new(10);
    let (_, unit) = unit_noise(&grid, seed, &forcing);
    let (noise, _) = continent_noise(&grid, seed, &forcing);
    let want = DRY_PLAINS.0 * legacy * relief_scale(forcing.gravity);
    for (u, n) in unit.iter().zip(&noise).filter(|(u, _)| u.abs() > 0.1) {
        assert!((f64::from(*n) / u / want - 1.0).abs() < 1e-4, "{n} {u}");
    }
}

#[test]
fn the_highest_crest_stands_at_the_mountain_relief() {
    // MOUNTAIN_RELIEF is the highest summit above its plateau (Everest less
    // Tibet): the ridged term's maximum is 1.
    let grid = Grid::new(24);
    let forcing = earth();
    let full = vec![1.0_f32; grid.cell_count()];
    let m = mountains(&grid, Seed::new(11), &forcing, &full);
    let top = f64::from(m.iter().copied().fold(0.0, f32::max));
    let want = MOUNTAIN_RELIEF.0 * relief_scale(forcing.gravity);
    assert!((top - want).abs() < 1.0, "{top} against {want}");
    let half = vec![0.5_f32; grid.cell_count()];
    let m = mountains(&grid, Seed::new(11), &forcing, &half);
    let top = f64::from(m.iter().copied().fold(0.0, f32::max));
    assert!((top - want / 2.0).abs() < 1.0, "{top}");
}

#[test]
fn orogens_raise_the_column_to_their_elevation_not_above_it() {
    // Tibet's 5 km is an elevation above sea level (Royden et al. 2008).
    let grid = Grid::new(4);
    let n = grid.cell_count();
    // Half the area at -4 km, half at +1 km: a dry world's mean is the
    // sea, so the halves stand at -2.5 and +2.5 km.
    let mut h: Vec<f32> = (0..n)
        .map(|k| {
            if grid.point(k)[0] > 0.0 {
                1000.0
            } else {
                -4000.0
            }
        })
        .collect();
    let sea = sea_level(&grid, &h, None);
    let before = h.clone();
    let weight: Vec<f32> = (0..n).map(|k| [1.0, 0.5, 0.0][k % 3]).collect();
    let elevation = vec![5000.0_f32; n];
    raise_orogens(&grid, &mut h, &weight, &elevation, &vec![true; n], None);
    for k in 0..n {
        let local = before[k] - sea;
        let want = before[k] + weight[k] * (5000.0 - local).max(0.0);
        assert!((h[k] - want).abs() < 1e-2, "{k}: {} against {want}", h[k]);
        if weight[k] == 1.0 {
            assert!((h[k] - sea - 5000.0).abs() < 1e-2);
        }
    }
    // A column already above the elevation is left alone.
    let mut high = vec![9000.0_f32; n];
    high[0] = -9000.0;
    let kept = high.clone();
    raise_orogens(
        &grid,
        &mut high,
        &vec![1.0; n],
        &vec![1000.0; n],
        &vec![true; n],
        None,
    );
    let sea = sea_level(&grid, &kept, None);
    for k in 1..n {
        if kept[k] - sea >= 1000.0 {
            assert_eq!(high[k], kept[k]);
        }
    }
}
