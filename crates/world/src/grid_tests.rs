use super::*;
use science::units::{EarthRadius, Metre};

#[test]
fn cells_cover_the_sphere_evenly() {
    let g = Grid::new(32);
    let angle = |a: [f64; 3], b: [f64; 3]| dot(a, b).clamp(-1.0, 1.0).acos();
    let (mut short, mut long) = (f64::MAX, 0.0f64);
    for k in 0..g.cell_count() {
        let p = g.point(k);
        // Each centre lies in its own cell.
        assert_eq!(g.index_of(p), k);
        for j in g.neighbours(k) {
            assert_ne!(j, k);
            let step = angle(p, g.point(j));
            short = short.min(step);
            long = long.max(step);
        }
    }
    // Neighbours (diagonal ones included, √2 farther) are about a cell
    // apart everywhere, face edges and corners included.
    let cell = std::f64::consts::FRAC_PI_2 / 32.0;
    assert!(
        short > 0.6 * cell && long < 1.9 * cell,
        "{short} {long} {cell}"
    );
}

#[test]
fn neighbours_across_an_edge_are_mutual() {
    let g = Grid::new(16);
    for k in 0..g.cell_count() {
        for j in g.neighbours(k) {
            assert!(g.neighbours(j).any(|i| i == k), "{k} {j}");
        }
    }
}

#[test]
fn earth_and_mars_at_15_km_match_the_design() {
    // n = round(π/2 × R / cell): R in metres from the IAU nominal
    // equatorial Earth radius; Mars mean radius 3389.5 km (Seidelmann
    // et al. 2007, IAU WGCCRE).
    let earth = Grid::for_radius(Metre::from(EarthRadius(1.0)), DEFAULT_CELL);
    assert_eq!(earth.n(), 668);
    assert_eq!(earth.cell_count(), 2_677_344);
    let mars = Grid::for_radius(Metre(3_389_500.0), DEFAULT_CELL);
    assert_eq!(mars.n(), 355);
}

#[test]
fn the_cap_holds_the_cell_count_under_eight_million() {
    let big = Grid::for_radius(Metre::from(EarthRadius(3.0)), DEFAULT_CELL);
    assert!(big.cell_count() <= MAX_CELLS);
    assert_eq!(big.n(), 1154); // floor(sqrt(8e6 / 6))
}

#[test]
fn a_tiny_body_keeps_one_cell_per_side() {
    assert_eq!(Grid::for_radius(Metre(1_000.0), DEFAULT_CELL).n(), 1);
}

#[test]
fn areas_sum_to_the_whole_sphere() {
    for n in [1, 2, 7, 64] {
        let g = Grid::new(n);
        let total: f64 = (0..g.cell_count()).map(|k| g.area(k)).sum();
        assert!((total - 4.0 * std::f64::consts::PI).abs() < 1e-9, "n={n}");
    }
}

#[test]
fn corner_cells_have_eight_distinct_neighbours() {
    let g = Grid::new(8);
    for face in 0..6u32 {
        for (x, y) in [(0, 0), (7, 0), (0, 7), (7, 7)] {
            let k = ((face * 8 + y) * 8 + x) as usize;
            let mut ns: Vec<usize> = g.neighbours(k).collect();
            ns.sort_unstable();
            ns.dedup();
            assert!(
                ns.len() >= 7 && !ns.contains(&k),
                "face {face} ({x},{y}): {ns:?}"
            );
        }
    }
}

#[test]
fn area_ratio_stays_within_the_equiangular_spread() {
    // Equi-angular cube map: cell areas vary by about 30 % (design).
    let g = Grid::new(64);
    let (min, max) = (0..g.cell_count())
        .map(|k| g.area(k))
        .fold((f64::MAX, 0.0_f64), |(a, b), v| (a.min(v), b.max(v)));
    assert!(max / min < 1.45, "{}", max / min);
}
