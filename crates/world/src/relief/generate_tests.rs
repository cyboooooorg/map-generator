use super::*;
use crate::fields::{Storage, Values};
use science::units::Fraction;
use timeline::fixtures::earth_like_fixture;
use timeline::{Seed, Trace};

fn below(grid: &Grid, h: &[f32]) -> f64 {
    let total: f64 = (0..h.len()).map(|k| grid.area(k)).sum();
    let wet: f64 = (0..h.len())
        .filter(|&k| h[k] < 0.0)
        .map(|k| grid.area(k))
        .sum();
    wet / total
}

fn ramp(grid: &Grid) -> Vec<f32> {
    (0..grid.cell_count())
        .map(|k| (k as f32 * 37.0) % 1000.0)
        .collect()
}

#[test]
fn sea_level_leaves_the_ocean_share_under_water() {
    let grid = Grid::new(16);
    let h = ramp(&grid);
    let level = sea_level(&grid, &h, Some(Fraction::new(0.71).unwrap()));
    let shifted: Vec<f32> = h.iter().map(|v| v - level).collect();
    let cell = (0..grid.cell_count())
        .map(|k| grid.area(k))
        .fold(0.0, f64::max);
    let total: f64 = (0..grid.cell_count()).map(|k| grid.area(k)).sum();
    // Ties in the ramp put a few equal cells on the same side.
    let ties = h.iter().filter(|&&v| v == level).count() as f64;
    let tolerance = (1.0 + ties) * cell / total;
    assert!(
        (below(&grid, &shifted) - 0.71).abs() <= tolerance,
        "{}",
        below(&grid, &shifted)
    );
}

#[test]
fn a_dry_world_is_measured_from_its_mean() {
    let grid = Grid::new(8);
    let h = ramp(&grid);
    assert_eq!(sea_level(&grid, &h, None), area_mean(&grid, &h));
    assert_eq!(
        sea_level(&grid, &h, Some(Fraction::new(0.0).unwrap())),
        area_mean(&grid, &h)
    );
}

#[test]
fn a_full_ocean_covers_every_cell() {
    let grid = Grid::new(8);
    let h = ramp(&grid);
    let level = sea_level(&grid, &h, Some(Fraction::new(1.0).unwrap()));
    assert!(h.iter().all(|&v| v - level <= 0.0));
}

#[test]
fn a_planet_without_surface_has_no_relief() {
    let mut trace = Trace::new();
    let mut state = earth_like_fixture(&mut trace, 4.57);
    state.surface = None;
    assert_eq!(
        generate(&state, Seed::new(1), &Grid::new(4)),
        Err(ReliefError::NoSurface)
    );
}

#[test]
fn the_smallest_grid_still_has_relief() {
    let mut trace = Trace::new();
    let state = earth_like_fixture(&mut trace, 4.57);
    let field = generate(&state, Seed::new(1), &Grid::new(1)).unwrap();
    let Values::F32(v) = &field.values else {
        panic!("heights are f32")
    };
    assert_eq!(v.len(), 6);
    assert!(v.iter().all(|x| x.is_finite()));
    assert!(!field.categorical);
    assert!(matches!(field.storage, Storage::U16 { .. }));
}

#[test]
fn the_ocean_share_of_the_state_is_met_on_a_grid() {
    let mut trace = Trace::new();
    let state = earth_like_fixture(&mut trace, 4.57);
    let grid = Grid::new(8);
    let field = generate(&state, Seed::new(2), &grid).unwrap();
    let Values::F32(v) = &field.values else {
        panic!("heights are f32")
    };
    let share = *state
        .surface
        .as_ref()
        .unwrap()
        .liquid
        .as_ref()
        .unwrap()
        .ocean_share
        .value();
    let total: f64 = (0..v.len()).map(|k| grid.area(k)).sum();
    let cell = (0..v.len()).map(|k| grid.area(k)).fold(0.0, f64::max);
    assert!((below(&grid, v) - share.get()).abs() <= 2.0 * cell / total);
}
