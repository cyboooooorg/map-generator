//! Climate model: temperature from annual insolation and altitude.
use crate::geo::Grid;

/// Earth's axial tilt in degrees, the calibration point of the temperature scale.
pub const EARTH_AXIAL_TILT_DEG: f32 = 23.44;

/// Cooling per unit of elevation above the waterline (elevation in `[-1, 1]`).
const LAPSE_RATE: f32 = 0.3;

/// Second Legendre polynomial, P2(x) = (3x² − 1) / 2.
fn p2(x: f64) -> f64 {
    (3.0 * x * x - 1.0) / 2.0
}

/// Annual-mean insolation at `latitude` (radians) for an axial tilt (degrees),
/// relative to the global mean.
///
/// Uses the classic two-term approximation `S(x) = 1 + s2·P2(x)` with
/// `x = sin(latitude)` and `s2 = −5/8 · P2(cos(tilt))` (North, 1975). Low tilt
/// makes poles much colder than the equator; above ~54° the poles receive more
/// sunlight over a year than the equator.
fn insolation(latitude: f64, axial_tilt_deg: f32) -> f64 {
    let s2 = -5.0 / 8.0 * p2((axial_tilt_deg as f64).to_radians().cos());
    1.0 + s2 * p2(latitude.sin())
}

/// Sea-level temperature of each row in `[0, 1]`, calibrated so that an
/// Earth-like tilt gives 0 at the poles and 1 at the equator. Other tilts can
/// leave that range and are clamped.
pub fn row_temperatures(grid: &Grid, axial_tilt_deg: f32) -> Vec<f32> {
    let pole = insolation(std::f64::consts::FRAC_PI_2, EARTH_AXIAL_TILT_DEG);
    let equator = insolation(0.0, EARTH_AXIAL_TILT_DEG);
    (0..grid.height)
        .map(|y| {
            let s = insolation(grid.latitude(y), axial_tilt_deg);
            (((s - pole) / (equator - pole)) as f32).clamp(0.0, 1.0)
        })
        .collect()
}

/// Temperature of a pixel: its row temperature, cooled on land with altitude.
/// The sea surface follows latitude only, whatever the depth underneath.
///
/// `height_above_water` is the elevation above the waterline (negative under water).
pub fn temperature(row_temperature: f32, height_above_water: f32) -> f32 {
    row_temperature - height_above_water.max(0.0) * LAPSE_RATE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid {
            width: 8,
            height: 90,
            circumference_km: 40_000.0,
        }
    }

    #[test]
    fn earth_tilt_spans_the_full_range() {
        let t = row_temperatures(&grid(), EARTH_AXIAL_TILT_DEG);
        assert!(t[0] < 0.02, "pole {}", t[0]);
        assert!(t[44] > 0.98, "equator {}", t[44]);
        // Symmetric hemispheres.
        assert!((t[10] - t[79]).abs() < 1e-5);
    }

    #[test]
    fn high_tilt_warms_the_poles_above_the_equator() {
        let t = row_temperatures(&grid(), 80.0);
        assert!(t[0] > t[44]);
    }

    #[test]
    fn only_land_cools_with_altitude() {
        assert_eq!(temperature(0.8, -0.5), 0.8);
        assert!(temperature(0.8, 0.5) < 0.8);
    }
}
