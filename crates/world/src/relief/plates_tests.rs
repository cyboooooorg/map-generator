use super::*;
use crate::stats::distance_to;
use science::constants::{EARTH_MEAN_RADIUS, STANDARD_GRAVITY};
use science::relief::plates::{
    Boundary, PLATE_COUNT, PLATE_RMS_SPEED, RELIEF_LEVELS, RELIEF_SPEED, TRENCH_DEPTH,
    TRENCH_WIDTH, boundary_height, orogen_elevation, orogeny_weight, relief_strength,
};

/// A body of 306 km radius: on `Grid::new(GRID)` its cells are about
/// 15 km, a quarter of a trench's width.
const SMALL_RADIUS: Metre = Metre(306e3);

/// Cells per face side of the profile tests.
const GRID: u32 = 32;

/// The body of the ranges test, 1500 km in radius (cells about 75 km):
/// large enough for a collision's flank not to cover the whole sphere, so
/// mid-ocean ridges can lie clear of it, as on Earth.
const RANGES_RADIUS: Metre = Metre(1500e3);

/// Continents in one hemisphere (x > 0), ocean in the other, as the
/// legacy tests.
fn hemisphere(grid: &Grid) -> Vec<bool> {
    (0..grid.cell_count())
        .map(|k| grid.point(k)[0] > 0.0)
        .collect()
}

fn draw(grid: &Grid, seed: u64) -> PlateMap {
    PlateMap::draw(grid, Seed::new(seed), &hemisphere(grid))
}

#[test]
fn plates_cover_the_sphere_with_varied_sizes() {
    // legacy-final:crates/mapgen/src/plates.rs:1186.
    let grid = Grid::new(24);
    let map = draw(&grid, 3);
    let count = map.count();
    assert!(
        (PLATE_COUNT.0 as usize..=PLATE_COUNT.1 as usize).contains(&count),
        "{count}"
    );
    assert_eq!(map.plate().len(), grid.cell_count());
    assert_eq!(map.motion().len(), count);
    let mut area = vec![0.0f64; count];
    for (k, &plate) in map.plate().iter().enumerate() {
        area[usize::from(plate)] += grid.area(k);
    }
    let total: f64 = area.iter().sum();
    let shares: Vec<f64> = area.iter().map(|a| a / total).collect();
    // Most plates hold some ground, and they are not all alike.
    assert!(
        shares.iter().filter(|&&s| s > 0.005).count() >= count - 2,
        "{shares:?}"
    );
    let (small, large) = shares
        .iter()
        .fold((1.0f64, 0.0f64), |(a, b), &s| (a.min(s), b.max(s)));
    assert!(large > 3.0 * small.max(0.005), "{shares:?}");
}

#[test]
fn plates_move_at_their_rms_speeds() {
    // Zahirovic et al. 2015: each plate's RMS surface speed within
    // PLATE_RMS_SPEED, slab pull speeding up some of them.
    let grid = Grid::new(24);
    let map = draw(&grid, 5);
    let mut sum = vec![(0.0f64, 0.0f64); map.count()];
    for (k, &plate) in map.plate().iter().enumerate() {
        let v = map.velocity(usize::from(plate), grid.point(k));
        let s = &mut sum[usize::from(plate)];
        s.0 += grid.area(k) * v.iter().map(|c| c * c).sum::<f64>();
        s.1 += grid.area(k);
    }
    let (lo, hi) = PLATE_RMS_SPEED;
    let speeds: Vec<f64> = sum
        .iter()
        .filter(|s| s.1 > 0.0)
        .map(|s| (s.0 / s.1).sqrt())
        .collect();
    for &v in &speeds {
        assert!(v >= lo.0 * (1.0 - 1e-9) && v <= hi.0 * (1.0 + 1e-9), "{v}");
    }
    let middle = (lo.0 + hi.0) / 2.0;
    assert!(speeds.iter().any(|&v| v > middle), "{speeds:?}");
    assert!(speeds.iter().any(|&v| v < middle), "{speeds:?}");
}

#[test]
fn plate_map_is_deterministic() {
    let grid = Grid::new(24);
    let map = draw(&grid, 3);
    assert_eq!(map, draw(&grid, 3));
    assert_ne!(map.plate(), draw(&grid, 4).plate());
    let radius = EARTH_MEAN_RADIUS;
    assert_eq!(
        map.relief(&grid, radius, STANDARD_GRAVITY),
        draw(&grid, 3).relief(&grid, radius, STANDARD_GRAVITY)
    );
}

/// The boundary cells of `map` with their kind and strength.
fn classified(grid: &Grid, map: &PlateMap) -> Vec<(Edge, Boundary, f64)> {
    edges(grid, &map.plate, &map.continental)
        .into_iter()
        .map(|e| {
            let v = MetrePerSecond(e.convergence(&map.motion));
            let kind = e.kind(v.0);
            let strength = relief_strength(kind, v);
            (e, kind, strength)
        })
        .collect()
}

/// Strong enough to reach the top level.
fn strong(strength: f64) -> bool {
    (strength * f64::from(RELIEF_LEVELS)).round() as u32 == RELIEF_LEVELS
}

#[test]
fn boundaries_raise_ranges_and_arcs_and_sink_trenches() {
    // legacy-final:crates/mapgen/src/plates.rs:1333, on what the world pass
    // owns (the profiles' shapes are science's tests): trenches on the
    // sinking side, ranges over collisions, arcs on the overriding side,
    // no mountain ridges on mid-ocean ridges.
    let grid = Grid::new(GRID);
    let radius = RANGES_RADIUS;
    let map = draw(&grid, 5);
    let relief = map.relief(&grid, radius, STANDARD_GRAVITY);
    // Each kind's relief apart (they add up): a trench next to a
    // collision range is still a trench.
    let of = |kinds: &[Boundary]| whole(&map.relief_of(&grid, radius, STANDARD_GRAVITY, kinds));
    let subductions = of(&[Boundary::Andean, Boundary::IslandArc]);
    let collisions = of(&[Boundary::Collision]);
    let cells = classified(&grid, &map);
    let mean = |h: &[f32], ks: &[usize]| {
        ks.iter().map(|&k| f64::from(h[k])).sum::<f64>() / ks.len() as f64
    };
    let subduction = |kind: Boundary| kind == Boundary::Andean || kind == Boundary::IslandArc;
    let (mut trenches, mut ranges) = (0, 0);
    for plate in 0..map.count() as u16 {
        let own: Vec<usize> = (0..grid.cell_count())
            .filter(|&k| map.plate[k] == plate)
            .collect();
        // Trenches: sinking cells lower than the plate 2-3 trench widths
        // out by half a trench.
        let sinking: Vec<usize> = cells
            .iter()
            .filter(|(e, kind, s)| e.plate == plate && subduction(*kind) && e.sinks() && strong(*s))
            .map(|(e, _, _)| e.cell)
            .collect();
        if !sinking.is_empty() {
            let mut mask = vec![false; grid.cell_count()];
            sinking.iter().for_each(|&k| mask[k] = true);
            let d = distance_to(&grid, radius, &mask, Some(TRENCH_WIDTH * 3.0));
            let ring: Vec<usize> = own
                .iter()
                .copied()
                .filter(|&k| f64::from(d[k]) >= 2.0 * TRENCH_WIDTH.0 && d[k].is_finite())
                .collect();
            if !ring.is_empty() {
                let depth = mean(&subductions, &ring) - mean(&subductions, &sinking);
                assert!(
                    depth > TRENCH_DEPTH.0 / 2.0 * relief_scale(STANDARD_GRAVITY),
                    "plate {plate}: trench {depth} m"
                );
                trenches += 1;
            }
        }
        // Ranges: collision cells above their plate's mean.
        let collision: Vec<usize> = cells
            .iter()
            .filter(|(e, kind, s)| e.plate == plate && *kind == Boundary::Collision && strong(*s))
            .map(|(e, _, _)| e.cell)
            .collect();
        if !collision.is_empty() {
            assert!(
                mean(&collisions, &collision) > mean(&collisions, &own),
                "plate {plate}: no range"
            );
            ranges += 1;
        }
    }
    assert!(trenches > 0 && ranges > 0, "{trenches} {ranges}");
    // Arcs: on the overriding side only, a continent over an Andean margin
    // or a plate overriding an island arc. Across the trench the arc's
    // bump keeps a tail under exp(-(arc distance / sigma)²), 4e-4.
    let overrides: Vec<bool> = (0..map.count() as u16)
        .map(|p| {
            cells
                .iter()
                .any(|(e, kind, _)| e.plate == p && *kind == Boundary::IslandArc && !e.sinks())
        })
        .collect();
    let mut arcs = 0;
    for k in 0..grid.cell_count() {
        if relief.arc[k] > 1e-3 {
            assert!(
                map.continental[k] || overrides[usize::from(map.plate[k])],
                "arc {} on the sinking side at {k}",
                relief.arc[k]
            );
            arcs += 1;
        }
    }
    assert!(arcs > 0);
    // Ridges: no mountain ridges on the mid-ocean ridges, but where a
    // ridge meets a collision range: the median ridge cell stays under 0.05.
    let mut ridges: Vec<f32> = cells
        .iter()
        .filter(|(_, kind, s)| *kind == Boundary::Ridge && strong(*s))
        .map(|(e, _, _)| relief.orogeny[e.cell])
        .collect();
    assert!(!ridges.is_empty());
    ridges.sort_by(f32::total_cmp);
    let median = ridges[ridges.len() / 2];
    assert!(median < 0.05, "orogeny {median} on the median ridge cell");
}

/// The relief of the boundaries on a surface at sea level: what they add
/// and the lift of their orogens, the orogeny weight times the elevation.
fn whole(relief: &PlateRelief) -> Vec<f32> {
    (0..relief.height.len())
        .map(|k| relief.height[k] + relief.orogeny[k] * relief.elevation[k])
        .collect()
}

/// The kinds of boundary that build relief.
const RELIEF_KINDS: [Boundary; 5] = [
    Boundary::Collision,
    Boundary::Andean,
    Boundary::IslandArc,
    Boundary::Rift,
    Boundary::Ridge,
];

/// For `kind`, at Earth's gravity: the peak of its profiles and the most
/// any of them changes over `step` metres.
fn profile(kind: Boundary, step: f64) -> (f64, f64) {
    let (mut peak, mut slope) = (0.0f64, 0.0f64);
    for i in -20..=20 {
        let v = MetrePerSecond(RELIEF_SPEED.1.0 * f64::from(i) / 10.0);
        for j in -400..400 {
            let d = f64::from(j) * step / 4.0;
            let at = |d: f64| {
                boundary_height(kind, Metre(d), v).0
                    + orogeny_weight(kind, Metre(d), v).get() * orogen_elevation(kind).0
            };
            let (a, b) = (at(d), at(d + step));
            peak = peak.max(a.abs());
            slope = slope.max((a - b).abs());
        }
    }
    (peak, slope)
}

/// The step bound of the relief of `kind` on `grid` (radius
/// [`SMALL_RADIUS`]): one layer of the layer cake (the kind's peak over
/// RELIEF_LEVELS) plus the steepest change of its profile over the
/// longest edge of the grid. The relief of all kinds is their sum, so
/// each kind is held to its own bound.
fn step_bound(grid: &Grid, kind: Boundary) -> f64 {
    let edge = (0..grid.cell_count())
        .flat_map(|k| grid.neighbours(k).map(move |j| (k, j)))
        .map(|(k, j)| {
            let cos: f64 = grid
                .point(k)
                .iter()
                .zip(grid.point(j))
                .map(|(x, y)| x * y)
                .sum();
            SMALL_RADIUS.0 * cos.clamp(-1.0, 1.0).acos()
        })
        .fold(0.0f64, f64::max);
    let (peak, slope) = profile(kind, edge);
    peak / f64::from(RELIEF_LEVELS) + slope
}

/// The neighbour pairs of `map` whose relief of one kind differs by more
/// than that kind's bound, and whether each crosses a plate boundary or a
/// coastline.
fn steps(grid: &Grid, map: &PlateMap) -> Vec<(Boundary, bool)> {
    let mut found = Vec::new();
    for kind in RELIEF_KINDS {
        let bound = step_bound(grid, kind);
        let relief = whole(&map.relief_of(grid, SMALL_RADIUS, STANDARD_GRAVITY, &[kind]));
        for k in 0..grid.cell_count() {
            for j in grid.neighbours(k) {
                if f64::from((relief[k] - relief[j]).abs()) > bound {
                    let crosses =
                        map.plate[k] != map.plate[j] || map.continental[k] != map.continental[j];
                    found.push((kind, crosses));
                }
            }
        }
    }
    found
}

#[test]
fn boundary_relief_does_not_step_from_cell_to_cell() {
    // legacy-final:crates/mapgen/src/plates.rs:1243: where the nearest
    // boundary changes, the relief does not jump by a whole profile. Pairs
    // across a plate boundary or a coastline are left out: margins and
    // coasts are real steps (see the next test).
    let grid = Grid::new(GRID);
    let inner: Vec<Boundary> = steps(&grid, &draw(&grid, 3))
        .into_iter()
        .filter(|(_, crosses)| !crosses)
        .map(|(kind, _)| kind)
        .collect();
    assert!(inner.is_empty(), "steps inside plates: {inner:?}");
}

#[test]
fn margins_step_only_where_plates_or_crust_change() {
    // Every pair of neighbours whose relief differs by more than the bound
    // crosses a plate boundary or a coastline.
    let grid = Grid::new(GRID);
    let found = steps(&grid, &draw(&grid, 7));
    assert!(found.iter().all(|(_, crosses)| *crosses), "{found:?}");
}
