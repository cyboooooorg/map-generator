use super::*;
use crate::{Field, Grid, Storage, Values, World};
use std::f64::consts::PI;
use timeline::{Code, Designation, EngineVersion, SystemNumber};

fn id() -> FieldId {
    FieldId(Code::of("test.field"))
}

fn world_with(n: u32, values: Values, categorical: bool) -> World {
    let body = Designation::star(SystemNumber::new(12_345).unwrap());
    let mut w = World::new(Grid::new(n), body, 0, EngineVersion(1), EngineVersion(1));
    w.insert(
        id(),
        Field {
            values,
            categorical,
            storage: Storage::Exact,
        },
    )
    .unwrap();
    w
}

fn f32_world(n: u32, f: impl Fn(usize, [f64; 3]) -> f32) -> World {
    let g = Grid::new(n);
    let v = (0..g.cell_count()).map(|k| f(k, g.point(k))).collect();
    world_with(n, Values::F32(v), false)
}

#[test]
fn a_constant_field_stays_constant_at_every_width() {
    let w = f32_world(16, |_, _| 3.5);
    for width in [8, 64, 512] {
        let r = project(&w, &id(), width).unwrap();
        assert_eq!((r.width, r.height), (width, width / 2));
        assert_eq!(r.values.len(), (width * (width / 2)) as usize);
        assert!(r.values.iter().all(|v| (v - 3.5).abs() < 1e-6));
    }
}

#[test]
fn a_latitude_field_projects_to_its_latitude() {
    let w = f32_world(64, |_, p| p[2].asin().to_degrees() as f32);
    let r = project(&w, &id(), 64).unwrap();
    // The latitude of a pixel is the mean over its band, and a cell is
    // up to 90/64 degrees tall at n = 64: a discretisation bound of the
    // test grid, not a physical tolerance.
    let tol = 90.0 / 64.0;
    for j in 0..r.height {
        let lat = 90.0 - 180.0 * (f64::from(j) + 0.5) / f64::from(r.height);
        for i in 0..r.width {
            let v = f64::from(r.values[(j * r.width + i) as usize]);
            assert!((v - lat).abs() <= tol, "row {j}: {v} vs {lat}");
        }
    }
}

#[test]
fn areas_agree_between_grid_and_projection() {
    // The cap z > 0.5 has area 2π(1 − 0.5) = π, a quarter of the sphere's
    // 4π, so the area-weighted mean is 1/4. The 0.01 tolerances are
    // discretisation bounds of the test grid and of the sampling lattice.
    let w = f32_world(64, |_, p| if p[2] > 0.5 { 1.0 } else { 0.0 });
    let g = w.grid();
    let f = w.field(&id()).unwrap();
    let grid_mean: f64 = (0..g.cell_count())
        .map(|k| g.area(k) * f64::from(f.values.get_f32(k)))
        .sum::<f64>()
        / (4.0 * PI);
    assert!((grid_mean - 0.25).abs() < 0.01, "grid {grid_mean}");
    for width in [128, 1024] {
        let r = project(&w, &id(), width).unwrap();
        let (mut sum, mut weight) = (0.0, 0.0);
        for j in 0..r.height {
            let lat = PI / 2.0 - PI * (f64::from(j) + 0.5) / f64::from(r.height);
            for i in 0..r.width {
                sum += lat.cos() * f64::from(r.values[(j * r.width + i) as usize]);
                weight += lat.cos();
            }
        }
        let mean = sum / weight;
        assert!((mean - grid_mean).abs() < 0.01, "width {width}: {mean}");
    }
}

#[test]
fn polar_rows_average_every_cell_they_cover() {
    let w = f32_world(32, |k, _| (k % 2) as f32);
    let r = project(&w, &id(), 16).unwrap();
    assert!(
        r.values[..16].iter().all(|&v| (0.2..=0.8).contains(&v)),
        "{:?}",
        &r.values[..16]
    );
}

#[test]
fn categorical_fields_take_the_mode() {
    let g = Grid::new(32);
    let v = (0..g.cell_count())
        .map(|k| if g.point(k)[2] > 0.0 { 3u8 } else { 7u8 })
        .collect();
    let w = world_with(32, Values::U8(v), true);
    let r = project(&w, &id(), 64).unwrap();
    assert!(r.values.iter().all(|&v| v == 3.0 || v == 7.0));
    let h = r.height as usize;
    assert!(
        r.values[(h / 2 - 1) * 64..(h / 2) * 64]
            .iter()
            .all(|&v| v == 3.0)
    );
    assert!(
        r.values[(h / 2) * 64..(h / 2 + 1) * 64]
            .iter()
            .all(|&v| v == 7.0)
    );
}

#[test]
fn an_unknown_field_is_missing() {
    let w = f32_world(4, |_, _| 0.0);
    let other = FieldId(Code::of("test.other"));
    assert_eq!(
        project(&w, &other, 8).unwrap_err(),
        WorldError::Missing(other.0)
    );
}

#[test]
#[should_panic(expected = "projection width must be even and non-zero")]
fn an_odd_width_panics() {
    let w = f32_world(4, |_, _| 0.0);
    let _ = project(&w, &id(), 7);
}

/// Two export widths of one world agree on area-weighted statistics
/// (issue #381): the mean, and the area share above zero, of a smooth
/// relief-like field on `Grid::new(24)`, read from the grid cells and
/// from rasters of 96 and 384 pixels. Raster rows are weighted by their
/// band's area (`sin φ_top − sin φ_bottom`).
///
/// A pixel averages the cells it covers, so for a smooth field a raster
/// differs from the grid by the averaging error only, not by any
/// projection bias. The bounds are that error on this field: 1 % of the
/// field's range for the mean, 0.01 for the share (the share also moves
/// with the few pixels straddling the zero contour). Width against width
/// is held to the same bounds.
#[test]
fn two_widths_agree_on_area_statistics() {
    let h = |p: [f64; 3]| {
        let (lat, lon) = (p[2].asin(), p[1].atan2(p[0]));
        (3000.0 * (3.0 * lon).sin() * lat.cos() + 1500.0 * lat.sin() - 400.0) as f32
    };
    let w = f32_world(24, |_, p| h(p));
    let g = w.grid();
    let values: Vec<f32> = (0..g.cell_count()).map(|k| h(g.point(k))).collect();
    let range = f64::from(values.iter().copied().fold(f32::MIN, f32::max))
        - f64::from(values.iter().copied().fold(f32::MAX, f32::min));
    let grid_mean = f64::from(crate::stats::area_mean(g, &values));
    let total: f64 = (0..g.cell_count()).map(|k| g.area(k)).sum();
    let grid_share = (0..g.cell_count())
        .filter(|&k| values[k] > 0.0)
        .map(|k| g.area(k))
        .sum::<f64>()
        / total;

    let stats = |width: u32| {
        let r = project(&w, &id(), width).unwrap();
        let (mut sum, mut above, mut weight) = (0.0, 0.0, 0.0);
        for j in 0..r.height {
            let top = PI / 2.0 - PI * f64::from(j) / f64::from(r.height);
            let bottom = PI / 2.0 - PI * f64::from(j + 1) / f64::from(r.height);
            let row = top.sin() - bottom.sin();
            for i in 0..r.width {
                let v = f64::from(r.values[(j * r.width + i) as usize]);
                sum += row * v;
                if v > 0.0 {
                    above += row;
                }
                weight += row;
            }
        }
        (sum / weight, above / weight)
    };
    let (mean_a, share_a) = stats(96);
    let (mean_b, share_b) = stats(384);
    let mean_tol = 0.01 * range;
    let msg = format!(
        "grid ({grid_mean}, {grid_share}); 96 ({mean_a}, {share_a}); 384 ({mean_b}, {share_b}); \
         mean tol {mean_tol}"
    );
    assert!(
        (mean_a - grid_mean).abs() < mean_tol,
        "96 vs grid mean: {msg}"
    );
    assert!(
        (mean_b - grid_mean).abs() < mean_tol,
        "384 vs grid mean: {msg}"
    );
    assert!((mean_a - mean_b).abs() < mean_tol, "96 vs 384 mean: {msg}");
    assert!(
        (share_a - grid_share).abs() < 0.01,
        "96 vs grid share: {msg}"
    );
    assert!(
        (share_b - grid_share).abs() < 0.01,
        "384 vs grid share: {msg}"
    );
    assert!((share_a - share_b).abs() < 0.01, "96 vs 384 share: {msg}");
}
