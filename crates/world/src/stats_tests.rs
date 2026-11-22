use super::*;

#[test]
fn distances_follow_great_circles() {
    let grid = Grid::new(32);
    let mut sources = vec![false; grid.cell_count()];
    sources[0] = true;
    let d = distance_to(&grid, Metre(1.0), &sources, None);
    let p0 = grid.point(0);
    let anti = (0..grid.cell_count())
        .min_by(|&a, &b| {
            let da: f64 = grid.point(a).iter().zip(p0).map(|(x, y)| x * y).sum();
            let db: f64 = grid.point(b).iter().zip(p0).map(|(x, y)| x * y).sum();
            da.total_cmp(&db)
        })
        .unwrap();
    assert_eq!(d[0], 0.0);
    let pi = std::f64::consts::PI;
    assert!((f64::from(d[anti]) - pi).abs() < 0.02 * pi, "{}", d[anti]);
}

#[test]
fn area_quantile_splits_the_sphere() {
    let grid = Grid::new(32);
    let z: Vec<f32> = (0..grid.cell_count())
        .map(|k| grid.point(k)[2] as f32)
        .collect();
    let one_cell = 2.0 / 32.0;
    assert!(f64::from(area_quantile(&grid, &z, 0.5)).abs() < one_cell);
    assert!((f64::from(area_quantile(&grid, &z, 0.29)) - (2.0 * 0.29 - 1.0)).abs() < one_cell);
}

#[test]
fn area_quantile_handles_the_ends() {
    let grid = Grid::new(1);
    let v = [3.0, -1.0, 2.0, 5.0, 0.0, 4.0];
    assert_eq!(area_quantile(&grid, &v, 0.0), -1.0);
    assert_eq!(area_quantile(&grid, &v, 1.0), 5.0);
}

#[test]
fn area_mean_weights_by_area() {
    let grid = Grid::new(8);
    let v = vec![2.0f32; grid.cell_count()];
    assert!((area_mean(&grid, &v) - 2.0).abs() < 1e-6);
}

#[test]
fn distances_stop_at_their_reach() {
    let grid = Grid::new(32);
    let mut sources = vec![false; grid.cell_count()];
    sources[0] = true;
    let all = distance_to(&grid, Metre(1.0), &sources, None);
    let reach = Metre(1.0);
    let near = distance_to(&grid, Metre(1.0), &sources, Some(reach));
    for (a, b) in all.iter().zip(&near) {
        if f64::from(*a) <= reach.0 {
            assert_eq!(a, b);
        } else {
            assert_eq!(*b, f32::INFINITY);
        }
    }
    assert!(near.iter().any(|d| d.is_infinite()));
}
