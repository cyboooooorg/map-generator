//! Climate model: temperature from annual insolation and altitude, moisture
//! from winds carrying ocean evaporation over land.
//!
//! Temperatures are annual means in degrees Celsius.
use crate::geo::Grid;
use rayon::prelude::*;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// Earth's axial tilt in degrees, the calibration point of the temperature scale.
pub const EARTH_AXIAL_TILT_DEG: f32 = 23.44;

/// Range temperatures are kept in, °C: the bounds of the exported layer,
/// wide enough for any planet (Venus is about 460 °C).
pub const TEMPERATURE_RANGE: (f32, f32) = (-200.0, 600.0);

/// Mean slope of [`celsius`] over Earth's range of insolation, °C per unit:
/// converts climate terms once expressed on the former `[0, 1]` scale.
pub const DEGREES_PER_UNIT: f32 = 45.0;

/// Cooling of the air with height, °C per km: the environmental lapse rate
/// of the standard atmosphere, whose troposphere follows
/// `T = 15.04 − 0.00649 h` (°C, h in m; NASA Glenn Research Center, Earth
/// atmosphere model, after the U.S. Standard Atmosphere 1976).
pub const LAPSE_RATE_K_PER_KM: f32 = 6.5;
/// Exponent of the pressure in [`lapse_rate`]: the power law through
/// Earth's 6.5 °C per km at 1 bar and Mars's 0.998 °C per km at 0.007 bar
/// (NASA Glenn Research Center, Mars atmosphere model: `T = −31 −
/// 0.000998 h` from the ground to 7 km, `p = 0.699 exp(−0.00009 h)` kPa).
/// The two points are measured; the power law between them is a
/// calibration, no source.
const LAPSE_PRESSURE_EXPONENT: f32 = 0.378;

/// The cooling per elevation unit worlds stored before format version 11
/// were made with (0.54 °C per km): their detail tiles and epochs keep it,
/// so that they match their stored temperatures.
pub const LEGACY_LAPSE_RATE: f32 = 13.5;

/// Cooling of the land with height on world `params`, °C per unit of
/// elevation (elevation in `[-1, 1]`, [`crate::craters::ELEVATION_KM`] km
/// per unit). Earth-like air of 1 bar or more cools at
/// [`LAPSE_RATE_K_PER_KM`] (162.5 °C per unit). Thinner air couples the
/// ground less to the air column: the ground's temperature is set by its
/// own radiation balance and falls less with height, Mars's air by 0.998
/// °C per km near the ground. The rate follows `(p / 1 bar)^0.378` between
/// the two (see [`LAPSE_PRESSURE_EXPONENT`]), and tends smoothly to none as
/// the air vanishes: an airless body's ground takes the temperature its
/// light gives it at any height.
pub fn lapse_rate(params: &crate::world::WorldParams) -> f32 {
    let coupling = params
        .pressure_bar
        .max(0.0)
        .powf(LAPSE_PRESSURE_EXPONENT)
        .min(1.0);
    LAPSE_RATE_K_PER_KM * crate::craters::ELEVATION_KM * coupling
}

/// Range of insolation levels [`row_temperatures`] keeps, in units of
/// Earth's equator-to-pole difference: from no light at all (−0.71) to
/// 4 times the mean under the star of a locked planet (4.87), the curve of
/// [`sea_level_celsius`] would leave Earth's range of climates far behind.
const INSOLATION_RANGE: (f32, f32) = (-0.5, 1.5);

/// Annual mean sea-level temperature, in °C, for an insolation level `t`
/// (0 at Earth's poles, 1 at its equator, from [`annual_insolation`]).
///
/// Fitted to Earth's annual zonal mean: −18 °C at the poles, 12 °C at 42°
/// and 27 °C at the equator (calibration on Earth, no source for the
/// curve). Temperature is not linear in insolation (the tropics are
/// flatter, heat being carried polewards), so a straight line would put
/// mid-latitudes too cold. Its ends: −49 °C with no light, 41 °C at 1.5.
pub fn sea_level_celsius(t: f32) -> f32 {
    -18.0 + 56.28 * t - 11.28 * t * t
}

/// The former curve of [`sea_level_celsius`], fitted to the two-term
/// insolation series this module used before (`1 − 5/8 P2(cos ε) P2(sin
/// φ)`): it converts temperatures stored on the former `[0, 1]` scale
/// (worlds of format version 3 and older, and their presets), and must not
/// change.
pub fn celsius(t: f32) -> f32 {
    -18.0 + 66.2 * t - 21.2 * t * t
}

/// Steps of the orbit [`annual_insolation`] averages over (midpoint rule):
/// within 0.1 % of the exact mean at every latitude and tilt.
const ORBIT_STEPS: usize = 360;

/// Daily-mean insolation at the top of the atmosphere at `latitude`
/// (radians) when the star stands over latitude `declination` (radians),
/// relative to the global annual mean `S0/4` of a circular orbit
/// (Nadeau & McGehee 2021, eq. 45–47, after Sellers 1965,
/// doi:10.1016/j.jmaa.2021.125129):
///
/// `Q = (4/π) (h0 sin φ sin δ + cos φ cos δ sin h0)`, `cos h0 = −tan φ tan δ`,
///
/// `h0` the half-length of the day as an angle (0 in polar night, π in
/// polar day). The North Pole at the June solstice of Earth gets 1.59
/// (541 W/m²), the equator 1.17 (397 W/m²).
pub fn daily_insolation(latitude: f64, declination: f64) -> f64 {
    let x = -latitude.tan() * declination.tan();
    let h0 = if x >= 1.0 {
        0.0
    } else if x <= -1.0 {
        PI
    } else {
        x.acos()
    };
    let q = 4.0 / PI
        * (h0 * latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * h0.sin());
    q.max(0.0)
}

/// Declination of the star (radians) at an angle `longitude` (radians)
/// along the orbit from the spring equinox, for an axial tilt in degrees:
/// `sin δ = sin ε sin λ` (Nadeau & McGehee 2021).
pub fn declination(axial_tilt_deg: f32, longitude: f64) -> f64 {
    ((axial_tilt_deg as f64).to_radians().sin() * longitude.sin()).asin()
}

/// Annual-mean insolation at `latitude` (radians) for an axial tilt in
/// `[0, 90]` degrees, relative to the global mean: the exact mean of
/// [`daily_insolation`] over a circular orbit (an eccentric one multiplies
/// every latitude by the same `1/sqrt(1 − e²)`, see
/// [`crate::star::mean_flux_factor`]). With no tilt the poles get nothing;
/// at Earth's tilt the equator gets 1.22 (416 W/m²) and the poles 0.51
/// (172 W/m²); from 53.9° the poles get more than the equator (Nadeau &
/// McGehee 2017, doi:10.1016/j.icarus.2017.01.040, show 54° as the tilt
/// "where polar and equatorial regions receive about the same yearly
/// insolation"). The global mean stays 1 whatever the tilt.
pub fn annual_insolation(latitude: f64, axial_tilt_deg: f32) -> f64 {
    (0..ORBIT_STEPS)
        .map(|k| {
            let longitude = (k as f64 + 0.5) / ORBIT_STEPS as f64 * TAU;
            daily_insolation(latitude, declination(axial_tilt_deg, longitude))
        })
        .sum::<f64>()
        / ORBIT_STEPS as f64
}

/// Insolation level of the temperature scale for an insolation relative
/// to the global mean: 0 at Earth's poles, 1 at its equator.
fn insolation_level(s: f64) -> f32 {
    let (pole, equator) = earth_insolation();
    ((s - pole) / (equator - pole)) as f32
}

/// Annual-mean insolation of Earth's poles and equator (see
/// [`annual_insolation`]), computed once.
fn earth_insolation() -> (f64, f64) {
    static EARTH: std::sync::OnceLock<(f64, f64)> = std::sync::OnceLock::new();
    *EARTH.get_or_init(|| {
        (
            annual_insolation(FRAC_PI_2, EARTH_AXIAL_TILT_DEG),
            annual_insolation(0.0, EARTH_AXIAL_TILT_DEG),
        )
    })
}

/// Sea-level temperature at `latitude` (radians), °C, before the mean is
/// set (see [`row_temperatures`]).
fn latitude_temperature(latitude: f64, axial_tilt_deg: f32) -> f32 {
    let t = insolation_level(annual_insolation(latitude, axial_tilt_deg));
    sea_level_celsius(t.clamp(INSOLATION_RANGE.0, INSOLATION_RANGE.1))
}

/// Bands of latitude [`mean_latitude_temperature`] integrates over.
const MEAN_BANDS: usize = 180;

/// Area-weighted global mean of [`latitude_temperature`], °C, on a fixed
/// set of bands so that it does not depend on the map's size.
fn mean_latitude_temperature(axial_tilt_deg: f32) -> f32 {
    let (mut sum, mut weight) = (0.0f64, 0.0f64);
    for k in 0..MEAN_BANDS {
        let latitude = ((k as f64 + 0.5) / MEAN_BANDS as f64 - 0.5) * PI;
        let w = latitude.cos();
        sum += f64::from(latitude_temperature(latitude, axial_tilt_deg)) * w;
        weight += w;
    }
    (sum / weight) as f32
}

/// Sea-level temperature of each row, in °C (see [`sea_level_celsius`]): Earth's tilt
/// gives −18 °C at the poles and 27 °C at the equator. A lower tilt makes
/// the poles colder, a higher one warmer, within [`INSOLATION_RANGE`].
///
/// The tilt moves the light between latitudes without changing its global
/// mean, so the rows are then shifted to keep the global mean temperature
/// of Earth's tilt (14.5 °C): in an energy balance model whose outgoing
/// radiation is linear in temperature, `A + B T`, the mean temperature
/// depends only on the mean absorbed light (Budyko 1969,
/// doi:10.3402/tellusa.v21i5.10109; North et al. 1981,
/// doi:10.1029/RG019i001p00091). Without the shift the curvature of
/// [`sea_level_celsius`] would move it with the tilt (by about 3 °C
/// between no tilt and 55° with the former curve).
pub fn row_temperatures(grid: &Grid, axial_tilt_deg: f32) -> Vec<f32> {
    let shift =
        mean_latitude_temperature(EARTH_AXIAL_TILT_DEG) - mean_latitude_temperature(axial_tilt_deg);
    (0..grid.height)
        .map(|y| latitude_temperature(grid.latitude(y), axial_tilt_deg) + shift)
        .collect()
}

/// Share of the seasonal swing of sunlight that land follows, against the
/// temperature its annual light gives it: a body of heat capacity `C`
/// under a yearly swing follows `1 / sqrt(1 + (ω C / λ)²)` of its
/// equilibrium response (ω the year's angular frequency, λ the climate's
/// restoring rate, North et al. 1981, doi:10.1029/RG019i001p00091).
/// Calibration, no source for the value: land's small heat capacity
/// (its air and a few metres of ground) lets it follow almost half, chosen
/// so that continental interiors swing by about ±10 °C at 30° and ±20 °C
/// at 60°, the order of Phoenix's and Fairbanks's ranges (not checked
/// against station normals). Coasts swing less: see [`Seasons::at`].
pub const LAND_SEASONS: f32 = 0.45;
/// The same for the sea, whose mixed layer (tens of metres of water)
/// holds the heat of a whole season: a half-range of about 4 °C at 60°.
/// Calibration, no source for the value; with λ = 5.4 W/m²/K (the slope
/// of [`sea_level_celsius`] at Earth's light), a mixed layer 50 m deep
/// would give 0.13.
pub const SEA_SEASONS: f32 = 0.10;
/// Distance from the coast over which land goes ~63 % of the way from the
/// sea's seasonal range to the interior's, km: the sea's mild air reaches
/// a few hundred km inland. Calibration, no source.
pub const CONTINENTALITY_KM: f32 = 400.0;

/// Swing of the daily-mean light at `latitude` over a year, as half the
/// difference between its sunniest and darkest days, relative to the
/// global mean (see [`daily_insolation`]).
fn insolation_swing(latitude: f64, axial_tilt_deg: f32) -> f64 {
    let (mut most, mut least) = (f64::MIN, f64::MAX);
    for k in 0..ORBIT_STEPS {
        let longitude = (k as f64 + 0.5) / ORBIT_STEPS as f64 * TAU;
        let q = daily_insolation(latitude, declination(axial_tilt_deg, longitude));
        most = most.max(q);
        least = least.min(q);
    }
    (most - least) / 2.0
}

/// The seasonal cycle of a planet's climate: for each row, half the
/// difference between its summer and its winter temperature, °C, on land
/// and on the sea. Summer is the annual mean plus it, winter the mean
/// minus it. The climate's layers keep the annual mean; the biomes read
/// the seasons where they decide the outcome (ice needs a summer below
/// freezing, see [`crate::biome::BiomeRules`]).
///
/// The swing of light is turned into degrees with the mean slope of
/// [`sea_level_celsius`], damped by thermal inertia ([`LAND_SEASONS`],
/// [`SEA_SEASONS`]) and by dense air (like the rows, see
/// [`crate::atmosphere::heat_contrast`]), then moved by the star like the
/// rows. At Earth's tilt land swings by about 11 °C at 30° and 20 °C at
/// 60°; at 90° a land pole swings by 57 °C, from a scorching summer to a
/// winter far below freezing. A planet locked to its star has no seasons.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seasons {
    land: Vec<f32>,
    sea: Vec<f32>,
}

impl Seasons {
    /// No seasonal cycle: summer and winter are the annual mean. Worlds
    /// stored before the seasons (format version 10 and older) read their
    /// biomes this way, as they were made.
    pub fn none(height: u32) -> Self {
        Self {
            land: vec![0.0; height as usize],
            sea: vec![0.0; height as usize],
        }
    }

    /// The seasons of the rows of `grid` for an axial tilt (degrees) and a
    /// surface pressure (bar); `star` moves rows of sea-level temperature
    /// as [`crate::star::star_temperatures`] does.
    pub fn new(
        grid: &Grid,
        axial_tilt_deg: f32,
        pressure_bar: f32,
        star: impl Fn(&mut [f32]),
    ) -> Self {
        let mut rows = row_temperatures(grid, axial_tilt_deg);
        crate::atmosphere::transport_heat(&mut rows, grid, pressure_bar);
        let (pole, equator) = earth_insolation();
        let contrast = crate::atmosphere::heat_contrast(pressure_bar);
        let swing: Vec<f32> = (0..grid.height)
            .map(|y| {
                let s = insolation_swing(grid.latitude(y), axial_tilt_deg) / (equator - pole);
                s as f32 * DEGREES_PER_UNIT * contrast
            })
            .collect();
        let half_range = |damping: f32| {
            let mut summer: Vec<f32> = rows
                .iter()
                .zip(&swing)
                .map(|(t, s)| t + s * damping)
                .collect();
            let mut winter: Vec<f32> = rows
                .iter()
                .zip(&swing)
                .map(|(t, s)| t - s * damping)
                .collect();
            star(&mut summer);
            star(&mut winter);
            summer
                .iter()
                .zip(&winter)
                .map(|(s, w)| ((s - w) / 2.0).max(0.0))
                .collect()
        };
        Self {
            land: half_range(LAND_SEASONS),
            sea: half_range(SEA_SEASONS),
        }
    }

    /// Half the summer-to-winter difference on row `y`, °C, on the sea
    /// or deep inland.
    pub fn half_range(&self, y: u32, land: bool) -> f32 {
        let rows = if land { &self.land } else { &self.sea };
        rows.get(y as usize).copied().unwrap_or(0.0)
    }

    /// Half the summer-to-winter difference of a pixel on row `y`, °C: the
    /// sea's on water, rising inland from it to the interior's with the
    /// distance to the coast (`coast_km`), over [`CONTINENTALITY_KM`]. A
    /// coast swings about half as much as an interior at the same
    /// latitude (Bergen against Yakutsk).
    pub fn at(&self, y: u32, land: bool, coast_km: f32) -> f32 {
        let sea = self.half_range(y, false);
        if !land {
            return sea;
        }
        let inland = 1.0 - (-coast_km.max(0.0) / CONTINENTALITY_KM).exp();
        sea + (self.half_range(y, true) - sea) * inland
    }

    /// Each row's summer temperature on land, from its annual-mean rows.
    pub fn land_summer(&self, rows: &[f32]) -> Vec<f32> {
        rows.iter()
            .enumerate()
            .map(|(y, t)| t + self.half_range(y as u32, true))
            .collect()
    }
}

/// Steps of [`LockedClimate`]'s profile over the angle from the substellar
/// point, 0 to 180°.
const LOCKED_STEPS: usize = 720;

/// The climate of a tidally locked planet: one face always towards its
/// star, so temperature follows the angle θ from the substellar point (at
/// the map's centre, longitude 180°, on the equator) instead of the
/// latitude. The light is `4 cos θ` times the mean on the day side, none on
/// the night side, turned into °C on Earth's scale and curve like the rows
/// of [`row_temperatures`] (from 41 °C under the star to −49 °C on the
/// night side, then evened out by the atmosphere and moved by the star,
/// see [`LockedClimate::profile_mut`]).
#[derive(Clone, Debug)]
pub struct LockedClimate {
    /// Sea-level temperature, °C, by step of θ.
    profile: Vec<f32>,
}

impl Default for LockedClimate {
    fn default() -> Self {
        Self::new()
    }
}

impl LockedClimate {
    pub fn new() -> Self {
        let profile = (0..LOCKED_STEPS)
            .map(|k| {
                let theta = Self::angle(k);
                let t = insolation_level(4.0 * theta.cos().max(0.0));
                sea_level_celsius(t.clamp(INSOLATION_RANGE.0, INSOLATION_RANGE.1))
            })
            .collect();
        Self { profile }
    }

    fn angle(step: usize) -> f64 {
        (step as f64 + 0.5) / LOCKED_STEPS as f64 * std::f64::consts::PI
    }

    /// Pulls the profile towards its area-weighted mean (`sin θ`) by the
    /// atmosphere's heat transport, like [`crate::atmosphere::transport_heat`].
    pub fn transport_heat(&mut self, pressure_bar: f32) {
        let contrast = crate::atmosphere::heat_contrast(pressure_bar);
        if contrast >= 1.0 {
            return;
        }
        let weights: Vec<f32> = (0..LOCKED_STEPS)
            .map(|k| Self::angle(k).sin() as f32)
            .collect();
        let mean = self
            .profile
            .iter()
            .zip(&weights)
            .map(|(t, w)| t * w)
            .sum::<f32>()
            / weights.iter().sum::<f32>();
        for t in &mut self.profile {
            *t = mean + (*t - mean) * contrast;
        }
    }

    /// The profile, to apply the star to it.
    pub fn profile_mut(&mut self) -> &mut [f32] {
        &mut self.profile
    }

    /// Angle from the substellar point of a pixel's centre, radians.
    pub fn angle_at(grid: &Grid, x: u32, y: u32) -> f64 {
        let lat = grid.latitude(y);
        let lon = (x as f64 + 0.5) / grid.width as f64 * std::f64::consts::TAU;
        (lat.cos() * (lon - std::f64::consts::PI).cos())
            .clamp(-1.0, 1.0)
            .acos()
    }

    /// Sea-level temperature at an angle from the substellar point, °C.
    pub fn at_angle(&self, theta: f64) -> f32 {
        let k = (theta / std::f64::consts::PI * LOCKED_STEPS as f64) as usize;
        self.profile[k.min(LOCKED_STEPS - 1)]
    }

    /// Warmest sea-level temperature of each row (its point nearest the
    /// star), for the per-row uses of the rows (where erosion thaws).
    pub fn rows(&self, grid: &Grid) -> Vec<f32> {
        (0..grid.height)
            .map(|y| self.at_angle(grid.latitude(y).abs()))
            .collect()
    }
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

/// Temperature of a pixel: its row temperature, cooled on land with altitude
/// by `lapse_rate` (°C per elevation unit, see [`lapse_rate`]). The sea
/// surface follows latitude only, whatever the depth underneath.
///
/// `height_above_water` is the elevation above the waterline (negative under water).
pub fn temperature(row_temperature: f32, height_above_water: f32, lapse_rate: f32) -> f32 {
    row_temperature - height_above_water.max(0.0) * lapse_rate
}

/// Earth's lapse rate per elevation unit, for the tests.
#[cfg(test)]
const EARTH_LAPSE: f32 = LAPSE_RATE_K_PER_KM * crate::craters::ELEVATION_KM;

// ── Moisture ──────────────────────────────────────────────────────────────────

/// Smallest and largest stretch of the moisture lengths with the planet's
/// circumference (Earth's is 1); beyond them the planet is not Earth-like
/// enough for the scaling to mean anything. Calibration, no source.
const MIN_STRETCH: f32 = 0.5;
const MAX_STRETCH: f32 = 2.0;

/// Ocean distance over which air picks up ~63% of the humidity it can hold.
/// With [`RAIN_KM`] it sets the humidity of marine air, where evaporation
/// balances the drizzle: `RAIN_KM / (RAIN_KM + EVAPORATION_KM)`, 80 % of
/// what the air can hold: turbulent mixing keeps the relative humidity of
/// the lower planetary boundary layer at about 80 % on average (Held &
/// Soden 2000, doi:10.1146/annurev.energy.25.1.441, p. 459, "The
/// Planetary Boundary Layer"; read in the paper, which gives it for the
/// boundary layer in general, and models ocean evaporation from it).
const EVAPORATION_KM: f32 = 500.0;
/// Land distance over which air rains out ~63% of its humidity.
const RAIN_KM: f32 = 2_000.0;
/// Land distance over which air drifts ~63% of the way towards the background
/// humidity brought by north-south circulation.
const MERIDIONAL_KM: f32 = 3_000.0;
/// Half-width, in degrees of latitude, of the band whose ocean feeds the
/// background humidity of a row.
const MERIDIONAL_REACH_DEG: f64 = 20.0;

/// Share of the ordinary rain over land that evaporates again (soil and
/// plants), so continental interiors are not starved of moisture: land
/// evapotranspiration is 65 500 km³ a year for 111 000 km³ of rain, 0.59
/// (Oki & Kanae 2006, doi:10.1126/science.1128845). The rain wrung out by
/// a climb does not come back: evaporation is limited by the energy the
/// ground receives, so the heavy rain of windward slopes runs off, and the
/// air crosses the crest drier.
const RECYCLING: f32 = 0.6;
/// Precipitation index (rain per 1 000 km relative to what the air holds
/// where it falls) that maps to neutral moisture.
const NEUTRAL_RAIN: f32 = 0.08;
/// Radius of the smoothing applied to precipitation, in km (at least a
/// pixel's width rounds to one pixel). Rain depends on slopes pixel by
/// pixel; smoothing keeps biome borders from turning to speckle, short
/// enough to keep the wet windward side of a range 50 km wide.
const RAIN_SMOOTHING_KM: f32 = 25.0;
/// Weight of the moisture noise, in index units (see [`moisture_index`]): a
/// factor of about 2.8 in rain at one unit. Calibration, no source.
const NOISE_WEIGHT: f32 = 1.0;

/// Annual precipitation per unit of the smoothed rain rate (humidity per
/// 1 000 km), mm. Calibration to the mean over Earth's land, 2.25 mm a day
/// or 821 mm a year (GPCP version 2.3, 1979-2013: Adler et al. 2018,
/// doi:10.3390/atmos9040138, Table 1), of the rain of the stored moisture
/// layer (noise and regional variability included, so within 125 to 8 000
/// mm), over `earth-like` seeds 1-4 and 11-16. The unclamped mean of the
/// model is higher: its unbounded windward maxima are what GPCP's 2.5°
/// cells and gauges never see.
const MM_PER_RATE: f32 = 2_120.0;
/// Octaves of rain (doublings) from 1 000 mm to a moisture of 1: the
/// index runs from 125 mm a year (-1) to 8 000 mm (+1).
const RAIN_OCTAVES: f32 = 3.0;
/// Rain at moisture 0, mm a year.
const NEUTRAL_RAIN_MM: f32 = 1_000.0;
/// Exponent of the cold damping of rain: rain is divided by the air's
/// holding capacity to this power, so cold air rains less than the warm
/// air that moisture comes from, but less than its capacity alone would
/// say (the eddies of the westerlies keep carrying moisture polewards).
/// Calibration to GPCP's zonal means (Adler et al. 2018, same table and
/// version, 2.5° zonal means from NOAA PSL), no source for the value.
const COLD_RAIN_EXPONENT: f32 = 0.8;
/// Holding capacity below which the cold damping stops growing (about
/// -8 °C), so polar land is dry but not absurdly wet. Calibration, no
/// source.
const COLD_CAPACITY_FLOOR: f32 = 0.1;
/// Octaves by which the circulation raises or lowers the rain: the
/// equator and the polar front gain, the Hadley edge and the poles lose.
/// Calibration to GPCP's zonal means, no source.
const CIRCULATION_OCTAVES: f32 = 0.4;
/// Extra octaves of the ITCZ, a belt `ITCZ_WIDTH_DEG` wide (in Earth
/// degrees of latitude, as a Gaussian's scale) at the equator. Calibration
/// to GPCP's zonal means, no source.
const ITCZ_OCTAVES: f32 = 0.6;
const ITCZ_WIDTH_DEG: f32 = 10.0;
/// Share of the background humidity that reaches the land under the
/// sinking air of the Hadley edge (1 under rising air): the dry air of the
/// subtropical highs brings land no moisture. Calibration, no source.
const SUBSIDENCE_HUMIDITY: f32 = 0.5;

/// Warming of air, K, per unit of humidity it rains out (humidity being
/// relative to saturated air at 27 °C and 1013 hPa, 21.9 g of vapour per
/// kg): the latent heat it releases, `L/cp` = 2491 K per kg/kg.
const LATENT_K_PER_UNIT: f32 = 54.5;
/// Distance over which the warmth of a foehn fades by ~63 % as the air
/// mixes and radiates, km. Calibration, no source.
const FOEHN_KM: f32 = 200.0;

/// Edge of the Hadley cells on Earth, degrees of latitude: the trade winds
/// blow "between about 30 degrees north and 30 degrees south" (NOAA National
/// Ocean Service), the westerlies between it and about 60° (NOAA
/// JetStream).
pub const EARTH_HADLEY_EDGE_DEG: f64 = 30.0;
/// Range the Hadley edge is kept in, degrees: below it a fast rotator
/// would grow more jets than three belts can draw, above it the cells
/// reach the poles. Calibration, no source.
const HADLEY_EDGE_RANGE_DEG: (f64, f64) = (15.0, 70.0);
/// Strength of the north-south surface wind against the east-west one at
/// the middle of each belt: the trades blow from the north-east and the
/// westerlies from the south-west (NOAA JetStream), the meridional part
/// being the weaker. Calibration, no source.
const MERIDIONAL_WIND: f32 = 0.5;
/// Weight the zonal runs keep where the wind is calm, against the
/// meridional ones (see [`moisture`]): the meridional share is
/// `|v| / (|u| + CALM_ZONAL + |v|)`, 0.29 at the middle of a belt and
/// fading to 0 at its edges. Calibration, no source.
const CALM_ZONAL: f32 = 0.25;

/// The wind belts of a planet: easterly trade winds blowing towards the
/// equator up to the edge of the Hadley cells, westerlies blowing towards
/// the poles up to the polar front, then polar easterlies blowing away from
/// the poles. Each belt is Earth's (edges at 30° and 60°) stretched between
/// the planet's own edges.
///
/// The Hadley edge scales as `1 / (Ω a)` with the rotation rate Ω and the
/// radius a: Held & Hou (1980, doi:10.1175/1520-0469(1980)037<0515:NASCIA>2.0.CO;2)
/// find `φ_H = (5 g H Δ / (3 Ω² a²))^(1/2)`, H the depth of the
/// circulation. Kaspi & Showman (2015, doi:10.1088/0004-637X/804/1/60)
/// find nearly global cells on slow rotators and narrower ones on fast
/// rotators, with `Ω⁻¹` as a reference that eddies complicate. Gravity is
/// left out: Held & Hou give a `g^(1/2)` widening, Kaspi & Showman find
/// less. The polar front stays halfway between the Hadley edge and the
/// pole. The atmosphere's mass, which narrows the cells in their models, is
/// not taken into account (see `docs/backlog.md`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Belts {
    /// Edge of the Hadley cells, degrees.
    hadley_deg: f64,
}

impl Belts {
    /// Earth's belts.
    pub const EARTH: Belts = Belts {
        hadley_deg: EARTH_HADLEY_EDGE_DEG,
    };

    /// The belts of a planet whose day lasts `day_hours` (sidereal, Earth:
    /// 24) and whose equator is `circumference_km` round.
    pub fn new(day_hours: f64, circumference_km: f64) -> Self {
        let scale = day_hours / 24.0 * crate::noise::EARTH_CIRCUMFERENCE_KM as f64
            / circumference_km.max(1.0);
        let edge = EARTH_HADLEY_EDGE_DEG * scale;
        Self {
            hadley_deg: if edge.is_finite() {
                edge.clamp(HADLEY_EDGE_RANGE_DEG.0, HADLEY_EDGE_RANGE_DEG.1)
            } else {
                HADLEY_EDGE_RANGE_DEG.1
            },
        }
    }

    /// Edge of the Hadley cells, degrees.
    pub fn hadley_edge_deg(&self) -> f64 {
        self.hadley_deg
    }

    /// Latitude of the polar front, degrees.
    pub fn polar_front_deg(&self) -> f64 {
        self.hadley_deg + (90.0 - self.hadley_deg) / 2.0
    }

    /// The latitude on Earth at the same place in its belt, degrees,
    /// unsigned: 0 to 30 in the Hadley cell, 30 to 60 in the westerlies,
    /// 60 to 90 in the polar cell.
    fn earth_latitude(&self, latitude: f64) -> f64 {
        let b = latitude.to_degrees().abs().min(90.0);
        let (h, p) = (self.hadley_deg, self.polar_front_deg());
        if b < h {
            30.0 * b / h
        } else if b < p {
            30.0 + 30.0 * (b - h) / (p - h)
        } else {
            60.0 + 30.0 * (b - p) / (90.0 - p)
        }
    }

    /// Zonal wind at `latitude` (radians) in `[-1, 1]`: positive blows
    /// east. Easterly at the equator and the poles, westerly between the
    /// belt edges, calm at the edges.
    pub fn zonal_wind(&self, latitude: f64) -> f32 {
        let b = self.earth_latitude(latitude);
        (if b < 30.0 {
            -(3.0 * b).to_radians().cos()
        } else if b < 60.0 {
            (6.0 * (b - 30.0)).to_radians().sin()
        } else {
            -(3.0 * (b - 60.0)).to_radians().sin()
        }) as f32
    }

    /// Meridional wind at `latitude` (radians) in `[-1, 1]`: positive blows
    /// towards the pole of its hemisphere. The trades and the polar
    /// easterlies blow towards the equator, the westerlies towards the
    /// poles; calm at the equator, the belt edges and the poles, where air
    /// rises or sinks.
    pub fn meridional_wind(&self, latitude: f64) -> f32 {
        let b = self.earth_latitude(latitude);
        let shape = if b < 30.0 {
            -(6.0 * b).to_radians().sin()
        } else if b < 60.0 {
            (6.0 * (b - 30.0)).to_radians().sin()
        } else {
            -(6.0 * (b - 60.0)).to_radians().sin()
        };
        MERIDIONAL_WIND * shape as f32
    }

    /// Octaves (doublings) by which the large-scale circulation raises or
    /// lowers the rain at `latitude` (radians): [`band`](Self::band) times
    /// [`CIRCULATION_OCTAVES`], plus the narrow rain belt of the ITCZ.
    pub fn rain_octaves(&self, latitude: f64) -> f32 {
        let e = self.earth_latitude(latitude) as f32;
        let itcz = (-(e / ITCZ_WIDTH_DEG).powi(2)).exp();
        CIRCULATION_OCTAVES * self.band(latitude) + ITCZ_OCTAVES * itcz
    }

    /// Large-scale circulation bias in `[-1, 1]`: rising wet air at the
    /// equator and at the polar front, sinking dry air at the Hadley edge
    /// (desert belts) and at the poles.
    pub fn band(&self, latitude: f64) -> f32 {
        (6.0 * self.earth_latitude(latitude)).to_radians().cos() as f32
    }
}

/// Saturation vapour pressure over water at `temperature` (°C), hPa:
/// `6.112 exp(17.67 T / (T + 243.5))` (Bolton 1980,
/// doi:10.1175/1520-0493(1980)108<1046:TCOEPT>2.0.CO;2, as Roe 2005 gives
/// it). It halves for about every 10 °C of cooling.
pub fn saturation_vapour_hpa(temperature: f32) -> f32 {
    6.112 * (17.67 * temperature / (temperature + 243.5)).exp()
}

/// Humidity that air can hold at `temperature` (°C), relative to saturated
/// air at 27 °C (the warm equator), from [`saturation_vapour_hpa`]: 0.66 at
/// 20 °C, 0.17 at 0 °C, 0.04 at −18 °C. Rising air cools at the lapse
/// rate and its capacity falls with it, so what it can no longer hold
/// condenses and rains out: the windward rain of a range.
fn holding_capacity(temperature: f32) -> f32 {
    // Below −80 °C the formula's pole at −243.5 °C is far, but the air
    // holds next to nothing: keep the capacity positive.
    saturation_vapour_hpa(temperature.max(-80.0)) / saturation_vapour_hpa(27.0)
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
    /// The planet's wind belts.
    pub belts: Belts,
}

/// What the moisture model gives each pixel.
pub struct Moisture {
    /// Moisture in `[-1, 1]`.
    pub moisture: Vec<f32>,
    /// The precipitation term alone, in `[-1, 1]` (for the clouds and the
    /// diagnostic maps).
    pub precipitation: Vec<f32>,
    /// Annual precipitation, mm, before the moisture noise.
    pub annual_mm: Vec<f32>,
    /// Warming of the air that rained out on a climb and comes down dry
    /// (foehn), °C, on land.
    pub foehn: Vec<f32>,
}

/// Rain rate (humidity per 1 000 km) and foehn warming (°C) of each pixel
/// of `path`, the pixel indices the wind crosses in order, `step_km`
/// apart. The path is walked `laps` times (twice round a row, so the
/// humidity entering the first pixel is the steady state left by the
/// previous lap); only the last lap is kept. The air sets out with the
/// background humidity of the first pixel, as the surface air of the
/// subsiding branch of a cell has taken up.
///
/// `background(k)` is the humidity, relative to capacity, that north-south
/// circulation brings to the `k`-th pixel: land air drifts towards it, so
/// rows with no ocean of their own still get some rain.
fn rain_along(
    input: &MoistureInput,
    path: &[usize],
    laps: usize,
    step_km: f32,
    background: impl Fn(usize) -> f32,
) -> (Vec<f32>, Vec<f32>) {
    let n = path.len();
    // The distances of the moisture model are Earth's: on a larger planet
    // (continents and oceans grow with it) they stretch with it, so
    // interiors are not drier for being farther from the sea. Rain rates
    // stay per Earth-equivalent 1 000 km.
    let stretch = (input.grid.circumference_km / crate::noise::EARTH_CIRCUMFERENCE_KM)
        .clamp(MIN_STRETCH, MAX_STRETCH);
    let reach_km = step_km / stretch;
    let evaporation = 1.0 - (-reach_km / EVAPORATION_KM).exp();
    let supply = 1.0 - (-reach_km / MERIDIONAL_KM).exp();
    let drizzle = 1.0 - (-reach_km / RAIN_KM).exp();
    let fade = (-step_km / FOEHN_KM).exp();
    let (mut rain, mut warm) = (vec![0.0f32; n], vec![0.0f32; n]);
    let mut humidity = match path.first() {
        Some(&i) => background(0) * holding_capacity(input.temperature[i]),
        None => 0.0,
    };
    let mut foehn = 0.0f32;
    for lap in 0..laps {
        for k in 0..n {
            let i = path[k];
            let here = input.height_above_water[i];
            // The air climbed to get here.
            let behind = if k > 0 || laps > 1 {
                input.height_above_water[path[(k + n - 1) % n]]
            } else {
                here
            };
            let climbing = here >= 0.0 && here > behind;
            foehn *= fade;
            // Air that rained out on a climb comes down warmer than it
            // went up: it climbed at the moist rate, it sinks at the dry
            // one (9.8 °C per km, NWS glossary). Its warmth shows once it
            // stops climbing.
            // Over land only: the sea takes back its own temperature.
            let lift = if climbing || here < 0.0 { 0.0 } else { foehn };
            let capacity = holding_capacity(input.temperature[i] + lift);
            let fallen = if here < 0.0 {
                // Over the sea: evaporation tops the air up towards its
                // capacity, and some of it rains back.
                humidity += (capacity - humidity).max(0.0) * evaporation;
                let fallen = humidity * drizzle;
                humidity -= fallen;
                fallen
            } else {
                // Over land: a steady drizzle, of which RECYCLING
                // evaporates again, and the condensation of whatever the
                // air can no longer hold where it has cooled (climbing,
                // or moving towards colder latitudes), which runs off.
                // Wrung out by a climb, its latent heat warms the air.
                humidity += (background(k) * capacity - humidity).max(0.0) * supply;
                let excess = (humidity - capacity).max(0.0);
                let ordinary = (humidity - excess) * drizzle;
                humidity -= ordinary * (1.0 - RECYCLING) + excess;
                if climbing {
                    foehn += LATENT_K_PER_UNIT * excess;
                }
                ordinary + excess
            };
            if lap + 1 == laps {
                rain[k] = fallen / reach_km * 1_000.0;
                warm[k] = if here >= 0.0 { lift } else { 0.0 };
            }
        }
    }
    (rain, warm)
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
            let lift = (1.0 + input.belts.band(input.grid.latitude(y as u32))) / 2.0;
            (local + global) / 2.0 * (SUBSIDENCE_HUMIDITY + (1.0 - SUBSIDENCE_HUMIDITY) * lift)
        })
        .collect()
}

/// The runs of rows the meridional wind crosses, each in the direction it
/// blows: from where air sinks and spreads (the poles, the Hadley edges)
/// to where it meets and rises (the equator, the polar fronts). Row 0 is
/// the north pole.
fn meridional_runs(grid: &Grid, belts: &Belts) -> Vec<Vec<u32>> {
    let h = grid.height;
    // Rows by hemisphere, from the equator outwards.
    let north: Vec<u32> = (0..h / 2).rev().collect();
    let south: Vec<u32> = (h.div_ceil(2)..h).collect();
    let mut runs = Vec::new();
    for rows in [north, south] {
        let mut run: Vec<u32> = Vec::new();
        let mut poleward: Option<bool> = None;
        for &y in &rows {
            let v = belts.meridional_wind(grid.latitude(y));
            let dir = v > 0.0;
            if poleward.is_some_and(|p| p != dir) && !run.is_empty() {
                runs.push(std::mem::take(&mut run));
            }
            poleward = Some(dir);
            run.push(y);
        }
        if !run.is_empty() {
            runs.push(run);
        }
    }
    // Each run in the wind's direction: outwards when poleward, else
    // towards the equator.
    runs.into_iter()
        .map(|mut run| {
            let v = belts.meridional_wind(grid.latitude(run[run.len() / 2]));
            if v <= 0.0 {
                run.reverse();
            }
            run
        })
        .collect()
}

/// Moisture of every pixel in `[-1, 1]`, the precipitation term alone, and
/// the foehn warming of the lee of ranges.
///
/// The wind carries the humidity along each row (blending the east and
/// west runs by the zonal wind, so the field stays continuous across the
/// belts) and along each column (the meridional wind, from where air
/// sinks to where it rises), the two weighed by the strength of each
/// component.
pub fn moisture(input: &MoistureInput) -> Moisture {
    let grid = input.grid;
    let (w, h) = (grid.width as usize, grid.height as usize);
    let background = background_humidity(input);
    let belts = input.belts;

    // Zonal runs: rain and warmth of every pixel.
    let zonal: Vec<(f32, f32)> = (0..grid.height)
        .into_par_iter()
        .flat_map_iter(|y| {
            let u = belts.zonal_wind(grid.latitude(y));
            let east_share = (1.0 + u) / 2.0;
            let b = background[y as usize];
            let row = y as usize * w;
            let path_east: Vec<usize> = (0..w).map(|x| row + x).collect();
            let path_west: Vec<usize> = (0..w).rev().map(|x| row + x).collect();
            let step = grid.dx_km(y);
            let (east, east_warm) = rain_along(input, &path_east, 2, step, |_| b);
            let (west, west_warm) = rain_along(input, &path_west, 2, step, |_| b);
            (0..w).map(move |x| {
                let wx = w - 1 - x;
                (
                    east_share * east[x] + (1.0 - east_share) * west[wx],
                    east_share * east_warm[x] + (1.0 - east_share) * west_warm[wx],
                )
            })
        })
        .collect();
    // Meridional runs, column by column.
    let runs = meridional_runs(&grid, &belts);
    let columns: Vec<Vec<(usize, f32, f32)>> = (0..w)
        .into_par_iter()
        .map(|x| {
            let mut out = Vec::with_capacity(h);
            for run in &runs {
                let path: Vec<usize> = run.iter().map(|&y| y as usize * w + x).collect();
                let (rain, warm) = rain_along(input, &path, 1, grid.dy_km(), |k| {
                    background[run[k] as usize]
                });
                out.extend(
                    path.iter()
                        .zip(rain)
                        .zip(warm)
                        .map(|((&i, r), t)| (i, r, t)),
                );
            }
            out
        })
        .collect();
    let mut meridional = vec![(0.0f32, 0.0f32); w * h];
    for (i, r, t) in columns.into_iter().flatten() {
        meridional[i] = (r, t);
    }
    // Weighed by the strength of each wind component; the zonal one alone
    // at the belt edges, where both are calm and where the meridional runs
    // of two cells meet (a floor of CALM_ZONAL on the zonal weight keeps
    // the field continuous there).
    let (rate, warmth): (Vec<f32>, Vec<f32>) = zonal
        .into_par_iter()
        .zip(meridional)
        .enumerate()
        .map(|(i, ((zr, zt), (mr, mt)))| {
            let latitude = grid.latitude((i / w) as u32);
            let wu = belts.zonal_wind(latitude).abs() + CALM_ZONAL;
            let wv = belts.meridional_wind(latitude).abs();
            let share = wv / (wu + wv);
            (zr + (mr - zr) * share, zt + (mt - zt) * share)
        })
        .unzip();
    // Scoped so the unsmoothed fields are freed once smoothed.
    let mut rain = smooth(&grid, &rate, RAIN_SMOOTHING_KM);
    drop(rate);
    let foehn = smooth(&grid, &warmth, RAIN_SMOOTHING_KM);
    drop(warmth);
    // Rain in mm a year: the smoothed rate, scaled to GPCP's land mean,
    // damped where the air is cold and modulated by the circulation (rising
    // air at the equator and the polar front, sinking air at the Hadley
    // edge), then the log-scale moisture index of the biomes.
    let row_octaves: Vec<f32> = (0..grid.height)
        .map(|y| belts.rain_octaves(grid.latitude(y)))
        .collect();
    let annual: Vec<f32> = (0..grid.pixel_count())
        .into_par_iter()
        .map(|i| {
            let octaves = row_octaves[i / w];
            annual_rain_mm(rain[i], input.temperature[i], octaves)
        })
        .collect();
    // The rate, relative to what the air holds where it falls, turned into
    // the precipitation term, in place: a ratio of rain to the evaporation
    // the warmth allows, potential evaporation taken as proportional to the
    // saturation vapour pressure. Calibration, no source: e_sat grows by
    // about 7 % per °C, faster than evaporation formulas, so cold land
    // comes out wetter than it is. The clouds read it.
    rain.par_iter_mut()
        .zip(input.temperature)
        .for_each(|(r, &t)| {
            let index = *r / holding_capacity(t);
            *r = 1.0 - 2.0 * (-index / NEUTRAL_RAIN).exp2();
        });
    let moisture = (0..grid.pixel_count())
        .into_par_iter()
        .map(|i| moisture_index(annual[i]) + NOISE_WEIGHT * input.noise[i])
        .map(|m| m.clamp(-1.0, 1.0))
        .collect();
    let foehn = foehn
        .into_par_iter()
        .zip(input.height_above_water)
        .map(|(f, &z)| if z >= 0.0 { f } else { 0.0 })
        .collect();
    Moisture {
        moisture,
        precipitation: rain,
        annual_mm: annual,
        foehn,
    }
}

/// Annual rain, mm, from the smoothed rain `rate`, the air `temperature`
/// (°C) and the circulation `band` in `[-1, 1]`.
fn annual_rain_mm(rate: f32, temperature: f32, octaves: f32) -> f32 {
    let cold = holding_capacity(temperature)
        .max(COLD_CAPACITY_FLOOR)
        .powf(-COLD_RAIN_EXPONENT);
    (MM_PER_RATE * rate * cold * octaves.exp2()).max(1.0)
}

/// Moisture index in `[-1, 1]` of an annual rain in mm: its base-2
/// logarithm, so each doubling is a third of a unit.
pub fn moisture_index(rain_mm: f32) -> f32 {
    (rain_mm / NEUTRAL_RAIN_MM).log2() / RAIN_OCTAVES
}

/// Annual rain, mm, of a moisture index (the inverse of [`moisture_index`]).
pub fn rain_mm(moisture: f32) -> f32 {
    NEUTRAL_RAIN_MM * (moisture * RAIN_OCTAVES).exp2()
}

/// Köppen's dryness threshold `P_th`, mm: `2 |T|` plus 0, 14 or 28 for rain
/// mostly in the winter half-year, spread, or mostly in the summer one
/// (at least 2/3 of the year's rain), with `T` the annual mean (Kottek et
/// al. 2006, doi:10.1127/0941-2948/2006/0130, eq. 2.1). The model has no
/// monthly rain, so the share of the summer half-year is the one the
/// model's own rain gives it: rain goes as the air's holding capacity
/// (humidity) to the power `1 - COLD_RAIN_EXPONENT` (the cold damping of
/// [`annual_rain_mm`]), at the half-years' mean temperatures (the annual
/// mean plus or minus `2/π` of the half-range of the temperature, the mean
/// of a sine over its half-period). Rain peaking in winter (the
/// Mediterranean) and monsoons are not modelled: calibration, no source.
pub fn dryness_threshold_mm(annual: f32, summer: f32, winter: f32) -> f32 {
    let half_year = |t: f32| annual + (t - annual) * 2.0 / std::f32::consts::PI;
    let rain = |t: f32| holding_capacity(half_year(t)).powf(1.0 - COLD_RAIN_EXPONENT);
    let (warm, cold) = (rain(summer), rain(winter));
    let share = warm / (warm + cold);
    let season = if share >= 2.0 / 3.0 { 28.0 } else { 14.0 };
    (2.0 * annual.abs() + season).max(1.0)
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
    fn a_locked_planet_is_hot_under_its_star_and_cold_behind() {
        let locked = LockedClimate::new();
        let grid = Grid {
            width: 360,
            height: 180,
            circumference_km: 40_000.0,
        };
        let substellar = locked.at_angle(LockedClimate::angle_at(&grid, 180, 90));
        let antistellar = locked.at_angle(LockedClimate::angle_at(&grid, 0, 90));
        assert!((substellar - 41.0).abs() < 0.5, "{substellar}");
        assert!((antistellar - -49.0).abs() < 0.5, "{antistellar}");
        // A dense atmosphere carries heat to the night side.
        let mut thick = locked.clone();
        thick.transport_heat(30.0);
        assert!(thick.at_angle(3.0) > antistellar + 20.0);
        assert_eq!(locked.rows(&grid).len(), 180);
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

    /// W/m² for an insolation relative to the mean, with the Sun's 1361
    /// W/m² (Nadeau & McGehee's tables use the same).
    fn watts(s: f64) -> f64 {
        s * 1361.0 / 4.0
    }

    #[test]
    fn annual_insolation_is_the_exact_mean_of_the_daily_one() {
        let at = |latitude_deg: f64, tilt: f32| {
            watts(annual_insolation(latitude_deg.to_radians(), tilt))
        };
        // The values of the axial tilt article (from Nadeau & McGehee 2021).
        for (tilt, equator, pole) in [
            (0.0, 433.2, 0.0),
            (23.44, 415.5, 172.3),
            (45.0, 372.5, 306.3),
            (90.0, 275.8, 433.2),
        ] {
            assert!(
                (at(0.0, tilt) - equator).abs() < 1.0,
                "{tilt}: {}",
                at(0.0, tilt)
            );
            assert!(
                (at(90.0, tilt) - pole).abs() < 1.0,
                "{tilt}: {}",
                at(90.0, tilt)
            );
        }
        // The poles overtake the equator at 53.9°, not at 54.7° as the
        // two-term series had it.
        assert!(at(90.0, 53.8) < at(0.0, 53.8));
        assert!(at(90.0, 54.0) > at(0.0, 54.0));
        // The tilt moves the light, not its global mean.
        for tilt in [0.0, 23.44, 60.0, 90.0] {
            let (mut sum, mut weight) = (0.0, 0.0);
            for k in 0..360 {
                let latitude = ((k as f64 + 0.5) / 360.0 - 0.5) * PI;
                sum += annual_insolation(latitude, tilt) * latitude.cos();
                weight += latitude.cos();
            }
            assert!(
                (sum / weight - 1.0).abs() < 2e-3,
                "{tilt}: {}",
                sum / weight
            );
        }
        // The pole at the June solstice: lit around the clock.
        let solstice = daily_insolation(FRAC_PI_2, declination(23.44, FRAC_PI_2));
        assert!((watts(solstice) - 541.0).abs() < 1.0);
    }

    #[test]
    fn the_global_mean_temperature_does_not_follow_the_tilt() {
        let grid = Grid {
            width: 4,
            height: 180,
            circumference_km: 40_000.0,
        };
        let mean = |tilt: f32| {
            let rows = row_temperatures(&grid, tilt);
            let w: Vec<f32> = (0..180).map(|y| grid.latitude(y).cos() as f32).collect();
            rows.iter().zip(&w).map(|(t, w)| t * w).sum::<f32>() / w.iter().sum::<f32>()
        };
        let earth = mean(EARTH_AXIAL_TILT_DEG);
        assert!((earth - 14.5).abs() < 0.3, "{earth}");
        for tilt in [0.0, 40.0, 54.7, 90.0] {
            assert!((mean(tilt) - earth).abs() < 0.05, "{tilt}: {}", mean(tilt));
        }
        // With no tilt the poles get no light: as cold as the curve goes.
        let flat = row_temperatures(&grid, 0.0);
        assert!(flat[0] < -45.0, "{}", flat[0]);
    }

    #[test]
    fn land_swings_with_the_seasons_far_more_than_the_sea() {
        let grid = Grid {
            width: 4,
            height: 180,
            circumference_km: 40_000.0,
        };
        // Row 29 is centred on 60.5° N, row 59 on 30.5° N.
        let earth = Seasons::new(&grid, EARTH_AXIAL_TILT_DEG, 1.0, |_| {});
        let (land60, sea60) = (earth.half_range(29, true), earth.half_range(29, false));
        assert!((land60 - 19.6).abs() < 1.0, "{land60}");
        assert!((sea60 - 4.4).abs() < 0.5, "{sea60}");
        assert!((earth.half_range(59, true) - 11.2).abs() < 1.0);
        // The equator: the twice-yearly swing of its light only.
        assert!(earth.half_range(89, true) < 2.0);
        // Tilted by 90°, the pole swings from a scorching summer to a
        // winter far below freezing.
        let rows = row_temperatures(&grid, 90.0);
        let tipped = Seasons::new(&grid, 90.0, 1.0, |_| {});
        assert!(rows[0] - tipped.half_range(0, true) < -20.0);
        // Dense air damps the seasons; a dim star narrows them in degrees.
        let dense = Seasons::new(&grid, EARTH_AXIAL_TILT_DEG, 92.0, |_| {});
        assert!(dense.half_range(29, true) < 0.11 * land60);
        let dim = Seasons::new(&grid, EARTH_AXIAL_TILT_DEG, 1.0, |rows| {
            for t in rows {
                *t = (*t + 273.15) * 0.8 - 273.15;
            }
        });
        assert!((dim.half_range(29, true) - 0.8 * land60).abs() < 1e-3);
        assert_eq!(Seasons::none(180).half_range(29, true), 0.0);
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

    /// The pixels of row `y` in the order a wind blowing east crosses them.
    fn eastward(grid: &Grid, y: u32) -> Vec<usize> {
        (0..grid.width as usize)
            .map(|x| y as usize * grid.width as usize + x)
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
        let temperature = vec![15.0; grid.pixel_count()];
        let noise = vec![0.0; grid.pixel_count()];
        let input = MoistureInput {
            grid,
            height_above_water: &height,
            temperature: &temperature,
            noise: &noise,
            belts: Belts::EARTH,
        };
        // Wind blowing east: land starts at x = 200, coast first.
        let (rain, _) = rain_along(&input, &eastward(&grid, 1), 2, grid.dx_km(1), |_| 0.0);
        assert!(rain[201] > rain[260] && rain[260] > rain[390]);
    }

    /// A coast at x = 2048 of a 4096-pixel equator (10 km pixels, the
    /// server's default on an Earth) and, when `height_km` is not 0, a
    /// ridge `height_km` high and 50 km wide 760 km inland; the air
    /// cools with height at the lapse rate. Returns the rain rate along
    /// the row (smoothed as the moisture pass does), the foehn warming,
    /// and the ridge's pixel.
    fn ridge_row(height_km: f32) -> (Vec<f32>, Vec<f32>, usize) {
        let grid = Grid {
            width: 4096,
            height: 1,
            circumference_km: 40_000.0,
        };
        let step = grid.dx_km(0);
        let ridge = 2048 + (760.0 / step) as usize;
        let height: Vec<f32> = (0..4096)
            .map(|x| {
                if x < 2048 {
                    -0.3
                } else {
                    let d = (x as f32 - ridge as f32).abs() * step;
                    0.001 + (height_km / crate::craters::ELEVATION_KM) * (1.0 - d / 25.0).max(0.0)
                }
            })
            .collect();
        let temperature: Vec<f32> = height
            .iter()
            .map(|&z| self::temperature(20.0, z, EARTH_LAPSE))
            .collect();
        let noise = vec![0.0; 4096];
        let input = MoistureInput {
            grid,
            height_above_water: &height,
            temperature: &temperature,
            noise: &noise,
            belts: Belts::EARTH,
        };
        let (rain, warm) = rain_along(&input, &eastward(&grid, 0), 2, step, |_| 0.5);
        (smooth(&grid, &rain, RAIN_SMOOTHING_KM), warm, ridge)
    }

    #[test]
    fn a_range_wrings_the_rain_out_of_the_wind_and_leaves_a_dry_warm_lee() {
        // A range 3 km high and 50 km wide (Roe 2005: the Southern Alps get
        // 11–12 m of rain a year on their windward side, under 1 m in
        // their lee).
        let (rain, warm, ridge) = ridge_row(3.0);
        let (flat, _, _) = ridge_row(0.0);
        let windward = rain[ridge - 3..=ridge].iter().copied().fold(0.0, f32::max);
        let lee = rain[ridge + 8];
        assert!(
            windward > 5.0 * flat[ridge],
            "{windward} vs {}",
            flat[ridge]
        );
        assert!(windward > 10.0 * lee, "{windward} vs {lee}");
        // Roe (2005): the lee gets a third to a half of the rain of the flat
        // ground, and a fraction of the windward maximum.
        assert!(lee < 0.5 * flat[ridge + 8], "{lee} vs {}", flat[ridge + 8]);
        assert!(windward > 20.0 * lee, "{windward} vs {lee}");
        // In mm a year: Southern Alps 11-12 m windward, under 1 m in the lee.
        let (windward_mm, lee_mm) = (windward * MM_PER_RATE, lee * MM_PER_RATE);
        assert!(
            (6_000.0..16_000.0).contains(&windward_mm),
            "{windward_mm} mm"
        );
        assert!(lee_mm < 1_000.0, "{lee_mm} mm");
        // The air that rained out comes down warmer than it went up, and
        // the warmth fades downwind.
        assert_eq!(warm[ridge - 2], 0.0);
        assert!(warm[ridge + 8] > 2.0, "{}", warm[ridge + 8]);
        assert!(warm[ridge + 100] < 0.5 * warm[ridge + 8]);
    }

    #[test]
    fn the_moisture_lengths_stretch_with_the_planet() {
        // Half the row is sea, half flat land: at the same fraction of the
        // way across, a planet twice as large rains the same, not less.
        let rain_at = |circumference_km: f32| {
            let grid = Grid {
                width: 512,
                height: 1,
                circumference_km,
            };
            let height: Vec<f32> = (0..512)
                .map(|x| if x < 256 { -0.3 } else { 0.01 })
                .collect();
            let temperature = vec![20.0; 512];
            let noise = vec![0.0; 512];
            let input = MoistureInput {
                grid,
                height_above_water: &height,
                temperature: &temperature,
                noise: &noise,
                belts: Belts::EARTH,
            };
            let (rain, _) = rain_along(&input, &eastward(&grid, 0), 2, grid.dx_km(0), |_| 0.5);
            rain[256 + 128]
        };
        let (earth, twice) = (rain_at(40_000.0), rain_at(80_000.0));
        assert!((earth / twice - 1.0).abs() < 0.05, "{earth} vs {twice}");
    }

    #[test]
    fn the_moisture_index_is_the_log_of_the_annual_rain() {
        assert!(moisture_index(NEUTRAL_RAIN_MM).abs() < 1e-6);
        assert!((moisture_index(8_000.0) - 1.0).abs() < 1e-5);
        assert!((moisture_index(125.0) + 1.0).abs() < 1e-5);
        for m in [-0.7f32, -0.2, 0.0, 0.4, 0.9] {
            assert!((moisture_index(rain_mm(m)) - m).abs() < 1e-5);
        }
    }

    #[test]
    fn koppens_dryness_threshold_follows_the_temperature_and_the_rain_season() {
        // Kottek et al. 2006, eq. 2.1, at 20 °C: 2T plus 14 spread through
        // the year, plus 28 when two thirds of the rain falls in summer.
        assert!((dryness_threshold_mm(20.0, 20.0, 20.0) - 54.0).abs() < 1e-3);
        // Rain follows the capacity to the power 0.2, so only an extreme
        // swing puts two thirds of it in the warm half-year.
        assert!((dryness_threshold_mm(20.0, 35.0, 5.0) - 54.0).abs() < 1e-3);
        assert!((dryness_threshold_mm(20.0, 80.0, -40.0) - 68.0).abs() < 1e-3);
        // Absolute temperature, as the paper writes it.
        assert!((dryness_threshold_mm(-5.0, -5.0, -5.0) - 24.0).abs() < 1e-3);
    }

    #[test]
    fn an_east_west_range_shades_the_meridional_wind() {
        // Land from 37° to 50° N on an ocean planet; a ridge along 40° N in
        // the westerlies, whose air moves polewards from the sea.
        let grid = Grid {
            width: 64,
            height: 180,
            circumference_km: 40_000.0,
        };
        let row = |latitude: f64| (90.0 - latitude) as usize;
        let land = |ridge: bool| -> Vec<f32> {
            (0..grid.pixel_count())
                .map(|i| {
                    let y = i / 64;
                    if y > row(37.0) || y < row(50.0) {
                        -0.3
                    } else if ridge && (row(41.0)..=row(39.0)).contains(&y) {
                        0.12
                    } else {
                        0.001
                    }
                })
                .collect()
        };
        let noise = vec![0.0; grid.pixel_count()];
        let rain_north_of_ridge = |height: &[f32]| {
            let temperature: Vec<f32> = (0..grid.pixel_count())
                .map(|i| self::temperature(20.0 - 0.3 * (i / 64) as f32, height[i], EARTH_LAPSE))
                .collect();
            let m = moisture(&MoistureInput {
                grid,
                height_above_water: height,
                temperature: &temperature,
                noise: &noise,
                belts: Belts::EARTH,
            });
            (0..64)
                .map(|x| m.precipitation[row(44.0) * 64 + x])
                .sum::<f32>()
                / 64.0
        };
        let (with, without) = (
            rain_north_of_ridge(&land(true)),
            rain_north_of_ridge(&land(false)),
        );
        // Without the meridional wind the rows north of the ridge would
        // never see it: the same rain with or without it.
        assert!(with < without - 0.03, "{with} vs {without}");
    }

    #[test]
    fn belts_turn_at_30_and_60_degrees_on_earth_and_widen_on_slow_rotators() {
        let earth = Belts::EARTH;
        let at = |b: &Belts, deg: f64| {
            (
                b.zonal_wind(deg.to_radians()),
                b.meridional_wind(deg.to_radians()),
            )
        };
        assert!(at(&earth, 30.0).0.abs() < 1e-6 && at(&earth, 60.0).0.abs() < 1e-6);
        // Trades from the east, blowing towards the equator.
        let trades = at(&earth, 15.0);
        assert!(trades.0 < -0.5 && trades.1 < -0.4);
        // A range at 25° gets its rain from the east (it got it from the
        // west when the belts turned at 22.5°).
        assert!(at(&earth, 25.0).0 < 0.0);
        let westerlies = at(&earth, -45.0);
        assert!(westerlies.0 > 0.99 && westerlies.1 > 0.49);
        assert!(at(&earth, 75.0).0 < -0.5 && at(&earth, 75.0).1 < -0.4);
        assert!(earth.band(0.0) > 0.99 && earth.band(30f64.to_radians()) < -0.99);
        // Held & Hou: the edge scales as 1 / (rotation rate × radius).
        let earth_km = f64::from(crate::noise::EARTH_CIRCUMFERENCE_KM);
        assert!((Belts::new(24.0, earth_km).hadley_edge_deg() - 30.0).abs() < 1e-9);
        assert!((Belts::new(48.0, earth_km).hadley_edge_deg() - 60.0).abs() < 1e-9);
        assert!((Belts::new(24.0, 2.0 * earth_km).hadley_edge_deg() - 15.0).abs() < 1e-9);
        assert_eq!(Belts::new(24.0 * 365.0, earth_km).hadley_edge_deg(), 70.0);
        assert_eq!(Belts::new(4.0, earth_km).hadley_edge_deg(), 15.0);
        let wide = Belts::new(40.0, earth_km);
        assert!((wide.polar_front_deg() - 70.0).abs() < 1e-9);
        assert!(wide.zonal_wind(45f64.to_radians()) < 0.0);
    }

    #[test]
    fn warm_air_holds_more_water_by_clausius_clapeyron() {
        assert!((saturation_vapour_hpa(0.0) - 6.112).abs() < 1e-3);
        assert!((saturation_vapour_hpa(20.0) - 23.37).abs() < 0.05);
        assert!((holding_capacity(27.0) - 1.0).abs() < 1e-6);
        // About half for every 10 °C of cooling (Roe 2005).
        let ratio = holding_capacity(20.0) / holding_capacity(10.0);
        assert!((1.7..2.0).contains(&ratio), "{ratio}");
    }

    #[test]
    fn land_cools_at_the_standard_lapse_rate() {
        // 3 km above the waterline: 19.5 °C colder.
        let t = temperature(20.0, 3.0 / crate::craters::ELEVATION_KM, EARTH_LAPSE);
        assert!((t - 0.5).abs() < 1e-4, "{t}");
        // A lighter world cools more slowly; an airless one not at all.
        let p = crate::generation::tests::params(1, "terran");
        assert!((lapse_rate(&p) - EARTH_LAPSE).abs() < 1e-3);
        let at = |pressure_bar: f32| {
            lapse_rate(&crate::world::WorldParams {
                pressure_bar,
                ..p.clone()
            }) / crate::craters::ELEVATION_KM
        };
        // Venus's dense air cools like Earth's, Mars's thin air by about
        // 1 °C per km (NASA Glenn), and the rate fades out smoothly with
        // the air.
        assert_eq!(at(92.0), LAPSE_RATE_K_PER_KM);
        assert!((at(0.007) - 0.998).abs() < 0.02, "{}", at(0.007));
        assert!(at(2e-4) > at(5e-5) && at(5e-5) > 0.0 && at(2e-4) < 0.3);
        assert_eq!(at(0.0), 0.0);
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
        assert_eq!(temperature(20.0, -0.5, EARTH_LAPSE), 20.0);
        assert!(temperature(20.0, 0.5, EARTH_LAPSE) < 20.0);
    }
}
