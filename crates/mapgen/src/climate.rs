//! Climate model: temperature from annual insolation and altitude, moisture
//! from winds carrying ocean evaporation over land.
//!
//! Temperatures are annual means in degrees Celsius.
use crate::geo::Grid;
use rayon::prelude::*;

/// Earth's axial tilt in degrees, the calibration point of the temperature scale.
pub const EARTH_AXIAL_TILT_DEG: f32 = 23.44;

/// Range temperatures are kept in, °C: the bounds of the exported layer,
/// wide enough for any planet (Venus is about 460 °C).
pub const TEMPERATURE_RANGE: (f32, f32) = (-200.0, 600.0);

/// Mean slope of [`celsius`] over Earth's range of insolation, °C per unit:
/// converts climate terms once expressed on the former `[0, 1]` scale.
pub const DEGREES_PER_UNIT: f32 = 45.0;

/// Cooling per unit of elevation above the waterline (elevation in `[-1, 1]`),
/// in °C.
const LAPSE_RATE: f32 = 0.3 * DEGREES_PER_UNIT;

/// Range of insolation levels [`row_temperatures`] keeps, in units of
/// Earth's equator-to-pole difference: beyond it the fitted curve would
/// turn back.
const INSOLATION_RANGE: (f32, f32) = (-0.5, 1.5);

/// Annual mean sea-level temperature, in °C, for an insolation level `t`
/// (0 at Earth's poles, 1 at its equator).
///
/// Fitted to Earth's annual zonal mean: −18 °C at the poles, 12 °C at 42°
/// and 27 °C at the equator. Temperature is not linear in insolation (the
/// tropics are flatter, heat being carried polewards), so a straight line
/// would put mid-latitudes too cold.
pub fn celsius(t: f32) -> f32 {
    -18.0 + 66.2 * t - 21.2 * t * t
}

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

/// Sea-level temperature of each row, in °C (see [`celsius`]): Earth's tilt
/// gives −18 °C at the poles and 27 °C at the equator. A lower tilt makes
/// the poles colder, a higher one warmer, within [`INSOLATION_RANGE`].
pub fn row_temperatures(grid: &Grid, axial_tilt_deg: f32) -> Vec<f32> {
    let pole = insolation(std::f64::consts::FRAC_PI_2, EARTH_AXIAL_TILT_DEG);
    let equator = insolation(0.0, EARTH_AXIAL_TILT_DEG);
    (0..grid.height)
        .map(|y| {
            let s = insolation(grid.latitude(y), axial_tilt_deg);
            let t = ((s - pole) / (equator - pole)) as f32;
            celsius(t.clamp(INSOLATION_RANGE.0, INSOLATION_RANGE.1))
        })
        .collect()
}

/// Weight of the large-scale temperature anomaly noise on land, °C: about
/// 3.6 °C of standard deviation, so climate zones move by up to ~15° of
/// latitude between continents.
const ANOMALY_LAND: f32 = 0.40 * DEGREES_PER_UNIT;
/// Weight on the sea, where water inertia damps the anomalies.
const ANOMALY_SEA: f32 = 0.20 * DEGREES_PER_UNIT;

/// Temperature anomaly for a noise sample in `[-1, 1]`, standing for ocean
/// currents, monsoons and continentality without modelling them. Zero-mean,
/// so the overall share of cold and hot climates does not change.
pub fn temperature_anomaly(noise: f32, land: bool) -> f32 {
    noise * if land { ANOMALY_LAND } else { ANOMALY_SEA }
}

/// A climate field that regional variability applies to.
#[derive(Clone, Copy, Debug)]
pub enum Field {
    Temperature,
    Moisture,
}

/// Regional climate variability for a noise sample in `[-1, 1]`: local
/// differences of soil, aspect and circulation (about 2 °C and 0.09 of
/// moisture of standard deviation). It makes biome borders irregular and
/// interleaved instead of following isotherms and isohyets.
pub fn regional_variability(noise: f32, field: Field) -> f32 {
    noise
        * match field {
            Field::Temperature => 0.25 * DEGREES_PER_UNIT,
            Field::Moisture => 0.50,
        }
}

/// Temperature of a pixel: its row temperature, cooled on land with altitude.
/// The sea surface follows latitude only, whatever the depth underneath.
///
/// `height_above_water` is the elevation above the waterline (negative under water).
pub fn temperature(row_temperature: f32, height_above_water: f32) -> f32 {
    row_temperature - height_above_water.max(0.0) * LAPSE_RATE
}

// ── Moisture ──────────────────────────────────────────────────────────────────

/// Ocean distance over which air picks up ~63% of the humidity it can hold.
const EVAPORATION_KM: f32 = 1_500.0;
/// Land distance over which air rains out ~63% of its humidity.
const RAIN_KM: f32 = 2_000.0;
/// Land distance over which air drifts ~63% of the way towards the background
/// humidity brought by north-south circulation.
const MERIDIONAL_KM: f32 = 3_000.0;
/// Half-width, in degrees of latitude, of the band whose ocean feeds the
/// background humidity of a row.
const MERIDIONAL_REACH_DEG: f64 = 20.0;

/// Share of the rain over land that evaporates again (soil and plants), so
/// continental interiors are not starved of moisture.
const RECYCLING: f32 = 0.6;
/// Extra fraction of humidity rained out per unit of elevation climbed.
const OROGRAPHIC_RAIN: f32 = 4.0;
/// Precipitation rate (humidity per 1 000 km) that maps to neutral moisture.
const NEUTRAL_RAIN: f32 = 0.08;
/// Radius of the smoothing applied to precipitation, in km. Rain depends on
/// slopes pixel by pixel; smoothing keeps biome borders from turning to speckle.
const RAIN_SMOOTHING_KM: f32 = 100.0;
/// Weights of the precipitation, circulation-band and noise terms.
const RAIN_WEIGHT: f32 = 0.75;
const BAND_WEIGHT: f32 = 0.25;
const NOISE_WEIGHT: f32 = 0.50;

/// Zonal wind at `latitude` in `[-1, 1]`: positive blows east. Easterly trade
/// winds near the equator, westerlies at mid-latitudes, polar easterlies.
pub fn zonal_wind(latitude: f64) -> f32 {
    -(4.0 * latitude).cos() as f32
}

/// Large-scale circulation bias in `[-1, 1]`: rising wet air at the equator
/// and around 60°, sinking dry air around 30° (desert belts) and at the poles.
pub(crate) fn circulation_band(latitude: f64) -> f32 {
    (6.0 * latitude).cos() as f32
}

/// Humidity that warm air can hold, relative to the warmest air, from a
/// temperature in °C: rises from polar (−18 °C) to equatorial (27 °C) air.
fn holding_capacity(temperature: f32) -> f32 {
    0.2 + 0.8 * ((temperature + 18.0) / DEGREES_PER_UNIT).clamp(0.0, 1.0)
}

/// Inputs of the moisture model, all row-major on the same grid.
pub struct MoistureInput<'a> {
    pub grid: Grid,
    /// Elevation relative to the waterline (negative under water).
    pub height_above_water: &'a [f32],
    /// Temperature in °C, before any planet-type offset.
    pub temperature: &'a [f32],
    /// Moisture noise in `[-1, 1]`, for local variation.
    pub noise: &'a [f32],
}

/// Rain rate along one row when the wind blows in `direction` (+1 east, −1 west).
///
/// Walks the row twice around the planet so the humidity entering the first
/// pixel is the steady state left by the previous lap.
///
/// `background` is the humidity, relative to capacity, that north-south
/// circulation brings to this latitude: land air drifts towards it, so rows
/// with no ocean of their own still get some rain.
fn rain_along_row(input: &MoistureInput, y: u32, direction: i64, background: f32) -> Vec<f32> {
    let w = input.grid.width as i64;
    let row = (y * input.grid.width) as usize;
    let step_km = input.grid.dx_km(y);
    let evaporation = 1.0 - (-step_km / EVAPORATION_KM).exp();
    let supply = 1.0 - (-step_km / MERIDIONAL_KM).exp();
    let drizzle = 1.0 - (-step_km / RAIN_KM).exp();
    let mut rain = vec![0.0f32; w as usize];
    let mut humidity = 0.0f32;

    for k in 0..2 * w {
        let x = (k * direction).rem_euclid(w) as usize;
        let next = (x as i64 + direction).rem_euclid(w) as usize;
        let here = input.height_above_water[row + x];
        let capacity = holding_capacity(input.temperature[row + x]);
        let fallen = if here < 0.0 {
            // Over the sea: evaporation tops the air up towards its capacity,
            // and some of it rains back.
            humidity += (capacity - humidity).max(0.0) * evaporation;
            humidity * drizzle
        } else {
            // Over land: steady drizzle, heavy rain where the wind climbs, and
            // forced condensation where the air cools below what it holds.
            humidity += (background * capacity - humidity).max(0.0) * supply;
            let climb = (input.height_above_water[row + next] - here).max(0.0);
            let fraction = (drizzle + OROGRAPHIC_RAIN * climb).min(1.0);
            let excess = (humidity - capacity).max(0.0);
            (humidity - excess) * fraction + excess
        };
        humidity -= if here < 0.0 {
            fallen
        } else {
            fallen * (1.0 - RECYCLING)
        };
        if k >= w {
            rain[x] = fallen / step_km * 1_000.0;
        }
    }
    rain
}

/// Background humidity of each row: the mean of the area-weighted ocean
/// fraction of the rows within [`MERIDIONAL_REACH_DEG`] of latitude and of the
/// whole planet.
fn background_humidity(input: &MoistureInput) -> Vec<f32> {
    let (w, h) = (input.grid.width as usize, input.grid.height as usize);
    let ocean: Vec<f32> = (0..h)
        .map(|y| {
            let row = &input.height_above_water[y * w..(y + 1) * w];
            row.iter().filter(|&&z| z < 0.0).count() as f32 / w as f32
        })
        .collect();
    let reach = (MERIDIONAL_REACH_DEG / 180.0 * h as f64).round() as i64;
    let area = |y: usize| input.grid.latitude(y as u32).cos() as f32;
    let global = (0..h).map(|y| ocean[y] * area(y)).sum::<f32>() / (0..h).map(area).sum::<f32>();
    (0..h as i64)
        .map(|y| {
            let (mut sum, mut weight) = (0.0f32, 0.0f32);
            for ny in (y - reach).max(0)..=(y + reach).min(h as i64 - 1) {
                let area = input.grid.latitude(ny as u32).cos() as f32;
                sum += ocean[ny as usize] * area;
                weight += area;
            }
            let local = if weight > 0.0 { sum / weight } else { 0.0 };
            // Half from nearby latitudes, half from the planet's whole ocean.
            (local + global) / 2.0
        })
        .collect()
}

/// Moisture of every pixel in `[-1, 1]`, and the precipitation term alone
/// (for the diagnostic maps).
pub fn moisture(input: &MoistureInput) -> (Vec<f32>, Vec<f32>) {
    let grid = input.grid;
    let w = grid.width as usize;
    let background = background_humidity(input);

    // Rain rate of every pixel, blending both wind directions by how much the
    // wind blows each way, so the field stays continuous across wind belts.
    // Scoped so the unsmoothed rate is freed once smoothed.
    let mut rain = {
        let rate: Vec<f32> = (0..grid.height)
            .into_par_iter()
            .flat_map_iter(|y| {
                let east_share = (1.0 + zonal_wind(grid.latitude(y))) / 2.0;
                let b = background[y as usize];
                let east = rain_along_row(input, y, 1, b);
                let west = rain_along_row(input, y, -1, b);
                (0..w).map(move |x| east_share * east[x] + (1.0 - east_share) * west[x])
            })
            .collect();
        smooth(&grid, &rate, RAIN_SMOOTHING_KM)
    };
    // The smoothed rate turned into the precipitation term, in place.
    rain.par_iter_mut()
        .for_each(|r| *r = 1.0 - 2.0 * (-*r / NEUTRAL_RAIN).exp2());
    let moisture = (0..grid.pixel_count())
        .into_par_iter()
        .map(|i| {
            let band = circulation_band(grid.latitude((i / w) as u32));
            let m = RAIN_WEIGHT * rain[i] + BAND_WEIGHT * band + NOISE_WEIGHT * input.noise[i];
            m.clamp(-1.0, 1.0)
        })
        .collect();
    (moisture, rain)
}

/// Box blur of `radius_km`, separable: east-west (wrapping, radius in pixels
/// growing towards the poles) then north-south.
fn smooth(grid: &Grid, values: &[f32], radius_km: f32) -> Vec<f32> {
    let (w, h) = (grid.width as usize, grid.height as usize);
    let horizontal: Vec<f32> = (0..h)
        .into_par_iter()
        .flat_map_iter(|y| {
            // At most (w - 1) / 2: a window wider than the row would count
            // pixels twice.
            let r = ((radius_km / grid.dx_km(y as u32)).round() as usize).min((w - 1) / 2);
            let row = &values[y * w..(y + 1) * w];
            let n = (2 * r + 1) as f32;
            let mut sum: f32 = (0..=2 * r).map(|k| row[(k + w - r) % w]).sum();
            (0..w)
                .map(move |x| {
                    let mean = sum / n;
                    sum += row[(x + r + 1) % w] - row[(x + w - r) % w];
                    mean
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let r = (radius_km / grid.dy_km()).round() as i64;
    (0..h)
        .into_par_iter()
        .flat_map_iter(|y| {
            let (lo, hi) = (
                (y as i64 - r).max(0) as usize,
                (y as i64 + r).min(h as i64 - 1) as usize,
            );
            let horizontal = &horizontal;
            (0..w).map(move |x| {
                (lo..=hi).map(|ny| horizontal[ny * w + x]).sum::<f32>() / (hi - lo + 1) as f32
            })
        })
        .collect()
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
    fn earth_tilt_matches_earth_temperatures() {
        let t = row_temperatures(&grid(), EARTH_AXIAL_TILT_DEG);
        assert!((t[0] + 18.0).abs() < 0.5, "pole {}", t[0]);
        assert!((t[44] - 27.0).abs() < 0.5, "equator {}", t[44]);
        // 43° south (rows are 2° tall): 12 °C at 42°.
        assert!((t[66] - 11.5).abs() < 1.0, "43° {}", t[66]);
        // Symmetric hemispheres.
        assert!((t[10] - t[79]).abs() < 1e-5);
    }

    #[test]
    fn high_tilt_warms_the_poles_above_the_equator() {
        let t = row_temperatures(&grid(), 80.0);
        assert!(t[0] > t[44]);
    }

    /// A row of ocean on the west half and flat land on the east half.
    fn half_ocean(grid: &Grid) -> Vec<f32> {
        (0..grid.pixel_count())
            .map(|i| {
                if i % grid.width as usize >= grid.width as usize / 2 {
                    0.1
                } else {
                    -0.5
                }
            })
            .collect()
    }

    #[test]
    fn rain_decreases_inland_downwind() {
        let grid = Grid {
            width: 400,
            height: 4,
            circumference_km: 40_000.0,
        };
        let height = half_ocean(&grid);
        let temperature = vec![0.8; grid.pixel_count()];
        let noise = vec![0.0; grid.pixel_count()];
        let input = MoistureInput {
            grid,
            height_above_water: &height,
            temperature: &temperature,
            noise: &noise,
        };
        // Wind blowing east: land starts at x = 200, coast first.
        let rain = rain_along_row(&input, 1, 1, 0.0);
        assert!(rain[201] > rain[260] && rain[260] > rain[390]);
    }

    #[test]
    fn mountains_cast_a_rain_shadow() {
        let grid = Grid {
            width: 400,
            height: 4,
            circumference_km: 40_000.0,
        };
        let mut height = half_ocean(&grid);
        // A ridge at x = 220..230 on every row.
        for y in 0..4 {
            for x in 220..230 {
                height[y * 400 + x] = 0.8;
            }
        }
        let flat = half_ocean(&grid);
        let temperature = vec![0.8; grid.pixel_count()];
        let noise = vec![0.0; grid.pixel_count()];
        let with_ridge = MoistureInput {
            grid,
            height_above_water: &height,
            temperature: &temperature,
            noise: &noise,
        };
        let without = MoistureInput {
            height_above_water: &flat,
            ..with_ridge
        };
        let shadow = rain_along_row(&with_ridge, 1, 1, 0.0)[250];
        let open = rain_along_row(&without, 1, 1, 0.0)[250];
        assert!(shadow < open * 0.75, "shadow {shadow} vs open {open}");
    }

    #[test]
    fn smoothing_keeps_constant_fields_and_wraps() {
        let grid = Grid {
            width: 40,
            height: 20,
            circumference_km: 40_000.0,
        };
        let flat = vec![0.5; grid.pixel_count()];
        assert!(
            smooth(&grid, &flat, 3_000.0)
                .iter()
                .all(|v| (v - 0.5).abs() < 1e-5)
        );
        let mut spike = vec![0.0; grid.pixel_count()];
        spike[grid.index(0, 10)] = 1.0;
        let s = smooth(&grid, &spike, 2_000.0);
        // The spike spreads to its western neighbour across the seam.
        assert!(s[grid.index(39, 10)] > 0.0);
    }

    #[test]
    fn anomalies_are_weaker_on_the_sea() {
        assert_eq!(temperature_anomaly(0.0, true), 0.0);
        assert!(temperature_anomaly(0.5, true) > temperature_anomaly(0.5, false));
        assert!(temperature_anomaly(-0.5, true) < 0.0);
    }

    #[test]
    fn only_land_cools_with_altitude() {
        assert_eq!(temperature(20.0, -0.5), 20.0);
        assert!(temperature(20.0, 0.5) < 20.0);
    }
}
