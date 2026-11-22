use super::*;
use crate::units::{CentimetrePerYear, Degree, Gyr, Metre, MetrePerSecond, Radian};

fn speed(cm_per_year: f64) -> MetrePerSecond {
    MetrePerSecond::from(CentimetrePerYear(cm_per_year))
}

/// Heights of a profile from `from` to `to` km, every 0.5 km.
fn profile(kind: Boundary, from: f64, to: f64, convergence: MetrePerSecond) -> Vec<(f64, f64)> {
    let steps = ((to - from) * 2.0) as usize;
    (0..=steps)
        .map(|i| {
            let d = from + i as f64 * 0.5;
            (d, boundary_height(kind, Metre(d * 1e3), convergence).0)
        })
        .collect()
}

/// Lowest point of a profile and the width of its trough at half that
/// depth, both in metres.
fn trough(points: &[(f64, f64)]) -> (f64, f64) {
    let (at, low) = points
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.1.total_cmp(&b.1.1))
        .map(|(i, p)| (i, p.1))
        .unwrap();
    let half = low / 2.0;
    let left = (0..at).rev().find(|&i| points[i].1 > half).unwrap();
    let right = (at..points.len()).find(|&i| points[i].1 > half).unwrap();
    (low, (points[right].0 - points[left].0) * 1e3)
}

/// Distance of the highest point of `points` beyond the boundary, metres.
fn crest(points: &[(f64, f64)]) -> f64 {
    points
        .iter()
        .filter(|p| p.0 > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
        .0
        * 1e3
}

#[test]
fn trenches_sit_in_the_published_range() {
    // Zhang et al. 2018: trench relief 0.9-4.7 km, width 31-93 km.
    for kind in [Boundary::Andean, Boundary::IslandArc] {
        let (low, width) = trough(&profile(kind, -500.0, 1000.0, RELIEF_SPEED.1));
        assert!((-4.7e3..=-0.9e3).contains(&low), "{kind:?}: {low}");
        assert!((31e3..=93e3).contains(&width), "{kind:?}: {width}");
    }
    // No convergence, no trench.
    let still = boundary_height(Boundary::Andean, Metre(0.0), MetrePerSecond(0.0));
    assert_eq!(still, Metre(0.0));
}

#[test]
fn arcs_stand_above_the_slab_depth() {
    // Syracuse & Abers 2006: 105 km; Lallemand et al. 2005: 32°.
    assert_eq!(ARC_SLAB_DEPTH, Metre(105e3));
    assert!((SLAB_DIP.0 - Radian::from(Degree(32.0)).0).abs() < 1e-12);
    // 105 km / tan 32° = 168 km.
    let arc = arc_distance().0;
    assert!((arc - 168.0e3).abs() < 0.5e3, "{arc}");
    // The island arc's crest stands above it.
    let points = profile(Boundary::IslandArc, -500.0, 1000.0, RELIEF_SPEED.1);
    let top = crest(&points);
    assert!((top - arc).abs() <= ISLAND_ARC_SIGMA.0, "{top}");
    // Both arcs' volcanic weight peaks there, at full strength from
    // ARC_SPEED.
    for kind in [Boundary::Andean, Boundary::IslandArc] {
        let weight = |d: f64| arc_weight(kind, Metre(d), ARC_SPEED).get();
        assert!((weight(arc) - 1.0).abs() < 1e-12, "{kind:?}");
        assert!(weight(arc + ISLAND_ARC_SIGMA.0) < weight(arc));
        assert!(weight(arc - ISLAND_ARC_SIGMA.0) < weight(arc));
        assert_eq!(arc_weight(kind, Metre(arc), MetrePerSecond(0.0)).get(), 0.0);
    }
    for kind in [Boundary::Collision, Boundary::Rift, Boundary::Ridge] {
        assert_eq!(arc_weight(kind, Metre(arc), ARC_SPEED).get(), 0.0);
    }
}

#[test]
fn andean_margins_raise_the_altiplano() {
    // Allmendinger et al. 1997: 3.65 km over 350-400 km, behind the arc.
    let points = profile(Boundary::Andean, -500.0, 2000.0, RELIEF_SPEED.1);
    let top = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    assert!((top - 3.65e3).abs() < 50.0, "{top}");
    let high: Vec<f64> = points
        .iter()
        .filter(|p| p.1 > top / 2.0)
        .map(|p| p.0)
        .collect();
    let width = (high[high.len() - 1] - high[0]) * 1e3;
    assert!((350e3..=400e3).contains(&width), "{width}");
    assert!(high[0] * 1e3 > arc_distance().0 - 10e3, "{}", high[0]);
}

#[test]
fn rifts_are_narrower_than_a_hundred_kilometres() {
    // Hopper & Buck 1996: rifts under 100 km wide.
    let (low, width) = trough(&profile(Boundary::Rift, -500.0, 500.0, speed(-5.0)));
    assert!(low < 0.0, "{low}");
    assert!(width < 100e3, "{width}");
    // Shoulders stand on either side, within Buck's 3 km.
    let points = profile(Boundary::Rift, -500.0, 500.0, speed(-5.0));
    let top = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    assert!(top > 0.0 && top < 3e3, "{top}");
    // A closing rift does nothing.
    assert_eq!(
        boundary_height(Boundary::Rift, Metre(0.0), speed(5.0)),
        Metre(0.0)
    );
}

#[test]
fn collisions_reach_tibets_height() {
    // Royden et al. 2008: Tibet stands 5 km high.
    let points = profile(Boundary::Collision, -2000.0, 2000.0, RELIEF_SPEED.1);
    let top = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    assert!(top <= 5e3 && top > 4.99e3, "{top}");
    // Wider than the Altiplano, narrower than Tibet's 1000 km.
    let high: Vec<f64> = points
        .iter()
        .filter(|p| p.1 > top / 2.0)
        .map(|p| p.0)
        .collect();
    let width = (high[high.len() - 1] - high[0]) * 1e3;
    assert!((400e3..1000e3).contains(&width), "{width}");
    // Slower convergence, lower range; none below RELIEF_SPEED.
    let slow = boundary_height(Boundary::Collision, Metre(0.0), speed(2.0)).0;
    assert!(slow > 0.0 && slow < top, "{slow}");
    assert_eq!(
        boundary_height(Boundary::Collision, Metre(0.0), RELIEF_SPEED.0),
        Metre(0.0)
    );
}

/// Ported from `legacy-final:crates/mapgen/src/plates.rs:1212`.
#[test]
fn ridges_follow_the_age_depth_relation_of_the_ocean_floor() {
    // Parsons & Sclater: 2.5 km deep at the crest, 3.55 km at 9 Ma.
    assert!((ridge_depth(Gyr(0.0)).0 - 2.5e3).abs() < 1e-6);
    assert!((ridge_depth(Gyr(0.009)).0 - 3.55e3).abs() < 1e-6);
    // The crest stands above the 70 Ma floor at any spreading rate: a slow
    // ridge, as the Mid-Atlantic's 2.5 cm/yr, keeps its full crest.
    let crest_height = ridge_depth(RIDGE_FLOOR_AGE) - ridge_depth(Gyr(0.0));
    for v in [-2.5, -5.0] {
        let h = boundary_height(Boundary::Ridge, Metre(0.0), speed(v));
        assert!((h - crest_height).0.abs() < 1e-6, "{h:?}");
    }
    // 9 Ma old at 5 cm/yr (2.5 per plate): 225 km out.
    let h = boundary_height(Boundary::Ridge, Metre(225e3), speed(-5.0));
    assert!(
        (h.0 - (ridge_depth(RIDGE_FLOOR_AGE).0 - 3.55e3)).abs() < 1.0,
        "{h:?}"
    );
    // Symmetric about the crest.
    assert_eq!(
        boundary_height(Boundary::Ridge, Metre(-225e3), speed(-5.0)),
        h
    );
    // 70 Ma at 5 cm/yr, 1750 km out: the old floor.
    assert_eq!(
        boundary_height(Boundary::Ridge, Metre(1800e3), speed(-5.0)),
        Metre(0.0)
    );
    // Faster ridges are broader.
    let at = |v| boundary_height(Boundary::Ridge, Metre(200e3), speed(v)).0;
    assert!(at(-8.0) > at(-2.0));
    // Closing plates build no ridge.
    assert_eq!(
        boundary_height(Boundary::Ridge, Metre(0.0), speed(2.0)),
        Metre(0.0)
    );
}

#[test]
fn transforms_leave_the_relief_alone() {
    for d in [-100e3, 0.0, 100e3] {
        let h = boundary_height(Boundary::Transform, Metre(d), RELIEF_SPEED.1);
        assert_eq!(h, Metre(0.0));
    }
}

#[test]
fn orogens_weigh_the_mountain_ridges() {
    // Full over the collision's crest and the Altiplano, none over the sea.
    let full = orogeny_weight(Boundary::Collision, Metre(0.0), RELIEF_SPEED.1);
    assert!((full.get() - 1.0).abs() < 1e-12);
    let andes = orogeny_weight(Boundary::Andean, andean_centre(), RELIEF_SPEED.1);
    assert!((andes.get() - 1.0).abs() < 1e-12);
    for kind in [Boundary::IslandArc, Boundary::Ridge, Boundary::Rift] {
        assert_eq!(orogeny_weight(kind, Metre(0.0), RELIEF_SPEED.1).get(), 0.0);
    }
}

#[test]
fn plates_keep_their_cited_values() {
    // Morra et al. 2013, Bird 2003; Sornette & Pisarenko 2003;
    // Zahirovic et al. 2015.
    assert_eq!(PLATE_COUNT, (8, 14));
    assert_eq!(PLATE_SIZE_EXPONENT, 0.25);
    assert_eq!(SLAB_FULL_SHARE, 0.25);
    let cm = |v: MetrePerSecond| CentimetrePerYear::from(v).0;
    assert!((cm(PLATE_RMS_SPEED.0) - 2.6).abs() < 1e-12);
    assert!((cm(PLATE_RMS_SPEED.1) - 7.9).abs() < 1e-12);
    assert!((cm(RELIEF_SPEED.0) - 0.5).abs() < 1e-12);
    assert!((cm(RELIEF_SPEED.1) - 5.0).abs() < 1e-12);
    assert!((cm(ARC_SPEED) - 2.0).abs() < 1e-12);
}

#[test]
fn slab_pull_speeds_up_the_sinking_plates() {
    // Zahirovic et al. 2015: 2.6 cm/yr without a subducting edge, 7.9
    // from a quarter of the perimeter subducting.
    let cm = |share: f64| CentimetrePerYear::from(slab_pull_speed(share)).0;
    assert!((cm(0.0) - 2.6).abs() < 1e-12);
    assert!((cm(SLAB_FULL_SHARE) - 7.9).abs() < 1e-12);
    assert!((cm(1.0) - 7.9).abs() < 1e-12);
    let mut last = cm(0.0);
    for i in 1..=10 {
        let v = cm(f64::from(i) * SLAB_FULL_SHARE / 10.0);
        assert!(v > last, "{v} after {last}");
        last = v;
    }
}

#[test]
fn the_plate_draw_keeps_its_legacy_values() {
    // legacy-final:crates/mapgen/src/plates.rs:40-58.
    assert_eq!(PLATE_WEIGHT, Radian(0.35));
    assert_eq!(SUBPLATE_FREQUENCY, 5.0);
    assert_eq!(SUBPLATE_WARP, 0.08);
    assert_eq!(SUBPLATE_WARP_FREQUENCY, 3.0);
    assert_eq!(BOUNDARY_WINDOW, 3);
}

#[test]
fn relief_strength_ramps_over_the_relief_speeds() {
    // Closing kinds grow with convergence, parting kinds with spreading.
    for kind in [Boundary::Collision, Boundary::Andean, Boundary::IslandArc] {
        assert_eq!(relief_strength(kind, RELIEF_SPEED.0), 0.0);
        assert_eq!(relief_strength(kind, RELIEF_SPEED.1), 1.0);
        assert_eq!(relief_strength(kind, speed(-5.0)), 0.0);
    }
    for kind in [Boundary::Rift, Boundary::Ridge] {
        assert_eq!(relief_strength(kind, speed(-5.0)), 1.0);
        assert_eq!(relief_strength(kind, speed(5.0)), 0.0);
    }
    assert_eq!(relief_strength(Boundary::Transform, speed(5.0)), 0.0);
}

#[test]
fn relief_levels_invert_the_strength() {
    for kind in [Boundary::Collision, Boundary::Ridge] {
        for i in 1..RELIEF_LEVELS {
            let s = f64::from(i) / f64::from(RELIEF_LEVELS);
            let v = relief_level(kind, s);
            assert!((relief_strength(kind, v) - s).abs() < 1e-9, "{kind:?} {s}");
        }
        assert_eq!(relief_strength(kind, relief_level(kind, 1.0)), 1.0);
    }
    assert_eq!(RELIEF_LEVELS, 4);
}

#[test]
fn profiles_vanish_beyond_their_reach() {
    for kind in [
        Boundary::Collision,
        Boundary::Andean,
        Boundary::IslandArc,
        Boundary::Rift,
        Boundary::Ridge,
    ] {
        let v = relief_level(kind, 1.0);
        let reach = relief_reach(kind, v);
        let peak = (-2000..=2000)
            .map(|i| boundary_height(kind, Metre(f64::from(i) * 1e3), v).0.abs())
            .fold(0.0, f64::max);
        for d in [reach.0, -reach.0, 1.5 * reach.0] {
            let h = boundary_height(kind, Metre(d), v).0.abs();
            assert!(h < 1e-3 * peak, "{kind:?} at {d}: {h} of {peak}");
        }
    }
}

/// Definition: the step of the numerical integral, 1 km.
const STEP: f64 = 1e3;

#[test]
fn the_arc_band_width_is_the_integral_of_the_arc_weight() {
    // Rectangle rule over +-5 sigma, 1 km steps, at full speed.
    let step = STEP;
    let centre = arc_distance().0;
    let sum: f64 = (-300..=300)
        .map(|i| {
            let d = Metre(centre + f64::from(i) * step);
            arc_weight(Boundary::IslandArc, d, ARC_SPEED).get() * step
        })
        .sum();
    assert!((sum / arc_band_width().0 - 1.0).abs() < 1e-3, "{sum}");
}
