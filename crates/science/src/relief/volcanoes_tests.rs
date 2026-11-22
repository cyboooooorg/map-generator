use super::*;
use crate::constants::EARTH_MEAN_RADIUS;
use crate::units::{Fraction, Watt, WattPerSquareMetre};

/// Source: Earth's heat loss, 44.2 TW (Pollack, Hurter & Johnson 1993,
/// doi:10.1029/93RG01249, abstract).
const EARTH_HEAT_LOSS: Watt = Watt(44.2e12);

fn intensity(flux: f64) -> f64 {
    volcanic_intensity(WattPerSquareMetre(flux)).get()
}

#[test]
fn earths_heat_flux_is_its_mean_flux() {
    // 44.2 TW over Earth's surface: 0.087 W/m² (Pollack et al. 1993).
    let flux = heat_flux(EARTH_HEAT_LOSS, EARTH_MEAN_RADIUS);
    assert!((flux.0 / EARTH_HEAT_FLUX.0 - 1.0).abs() < 0.01, "{flux:?}");
}

#[test]
fn volcanism_meets_its_anchors() {
    // Earth (Pollack et al. 1993) at its calibrated value, a heat pipe
    // (Moore & Webb 2013) at 1.
    assert!((intensity(EARTH_HEAT_FLUX.0) - EARTH_VOLCANIC_INTENSITY).abs() < 1e-12);
    assert!((intensity(HEAT_PIPE_FLUX.0) - 1.0).abs() < 1e-12);
    assert_eq!(intensity(10.0 * HEAT_PIPE_FLUX.0), 1.0);
    assert_eq!(intensity(0.0), 0.0);
    assert_eq!(intensity(-1.0), 0.0);
    // Monotonic between them.
    let mut last = 0.0;
    for i in 1..=300 {
        let v = intensity(i as f64 * 0.01);
        assert!(v >= last, "{i}: {v} < {last}");
        last = v;
    }
}

#[test]
fn cones_keep_their_published_shape() {
    // Grosse & Kervyn 2018: H/WB 0.01-0.1, volumes 0.1-1000 km³.
    assert_eq!(CONE_ASPECT, (0.01, 0.1));
    assert_eq!(CONE_VOLUME, (0.1e9, 1000e9));
    // Plescia 2004's 22 km on Mars, at standard gravity: 8.4 km.
    assert!(
        (CONE_HEIGHT_MAX.0 - 8.4e3).abs() < 50.0,
        "{CONE_HEIGHT_MAX:?}"
    );
    let draws = [0.0, 0.25, 0.5, 0.75, 1.0].map(|x| Fraction::new(x).unwrap());
    for volume in draws {
        for aspect in draws {
            let cone = cone_size(volume, aspect);
            let ratio = cone.height.0 / (2.0 * cone.radius.0);
            assert!(
                ratio >= CONE_ASPECT.0 - 1e-12 && ratio <= CONE_ASPECT.1 + 1e-12,
                "{cone:?}"
            );
            assert!(cone.height <= CONE_HEIGHT_MAX, "{cone:?}");
            let v = std::f64::consts::PI * cone.radius.0.powi(2) * cone.height.0 / 3.0;
            assert!(v >= CONE_VOLUME.0 * (1.0 - 1e-9), "{cone:?}");
            assert!(v <= CONE_VOLUME.1 * (1.0 + 1e-9), "{cone:?}");
        }
    }
    // The draw spans the range: the smallest and largest volumes.
    let low = cone_size(draws[0], draws[2]);
    let high = cone_size(draws[4], draws[2]);
    assert!(high.radius > low.radius && high.height > low.height);
}
