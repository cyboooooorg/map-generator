use super::*;
use crate::grid::Grid;
use crate::relief::ReliefError;
use crate::stats::area_quantile;
use science::constants::{EARTH_MEAN_RADIUS, STANDARD_GRAVITY};
use science::relief::crust::{CONTINENTAL_SLOPE, crust, floating_height};
use science::relief::spectrum::Analogue;
use science::relief::volcanoes::{CONE_ASPECT, CONE_HEIGHT_MAX, EARTH_HEAT_FLUX};
use science::units::{Fraction, WattPerSquareMetre};
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
    let noise = continent_noise(&grid, seed.derive("relief.continents"), &forcing);
    let pick = |a: &[f32], on: bool| -> Vec<f32> {
        (0..a.len())
            .filter(|&k| mask[k] == on)
            .map(|k| a[k])
            .collect()
    };
    let step = |a: &[f32]| f64::from(median(pick(a, true)) - median(pick(a, false)));
    // The continents are the high parts of the noise, so the noise alone
    // already stands higher on them: what is Airy's is the step beyond it.
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
    let noise = continent_noise(&grid, seed.derive("relief.continents"), &forcing);
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
