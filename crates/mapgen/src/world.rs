use crate::biome::BiomeInput;
use crate::geo::Grid;
use crate::look::Look;
use crate::noise::EARTH_CIRCUMFERENCE_KM;
use crate::planet::PlanetModel;
use crate::rivers::RiverLine;

// Re-export so existing `use crate::world::*;` in other modules keeps working.
pub use crate::biome::{Biome, biome_color, biome_name, surface_color};

/// Everything needed to generate a world. The same parameters always produce
/// the same world. Stored with each body of a system (see
/// [`crate::system`]), so that it is generated later as it was drawn.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct WorldParams {
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    /// The class key (`PlanetClass::key`), or `rocky-moon` / `icy-moon`.
    pub class: String,
    /// The archetype asked for, for display only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub archetype: Option<String>,
    /// The planet's model; `None` for a moon (part 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub model: Option<PlanetModel>,
    /// How the world is drawn until #424 (see [`crate::look`]): climate
    /// shift, biome rules and remapping, sky, geology and giant look.
    pub look: Look,
    /// Elevation bias applied before biome selection.
    /// 0.0 = default. Positive → more ocean, negative → more land. Range [-1, 1].
    pub sea_level: f32,
    /// Fraction of mountain chains that become volcanic.
    /// 0.0 = no volcanoes, 1.0 = most mountain chains are volcanic.
    pub volcanic_intensity: f32,
    /// Equatorial circumference of the planet in kilometres. Earth ≈ 40 075 km.
    pub circumference_km: f32,
    /// Tilt of the rotation axis in degrees, in `[0, 90]`. Earth ≈ 23.44°.
    /// Sets the latitude temperature profile and the tropics / polar circles.
    pub axial_tilt_deg: f32,
    /// Hydraulic erosion strength: droplets per erosion-grid pixel. 0 disables it.
    pub erosion: f32,
    /// Surface pressure in bar (0 = airless).
    pub pressure_bar: f32,
    /// Age of the oldest surviving surface, in Ga: how long impacts have
    /// cratered it.
    pub surface_age_ga: f32,
    /// Spectral type of the host star, `class × 10 + subtype` (see
    /// `crate::star`; the Sun, G2V, is 42).
    pub star: f32,
    /// Semi-major axis of the planet's orbit, AU.
    pub orbit_au: f32,
    /// Eccentricity of the orbit (0 circular).
    pub eccentricity: f32,
    /// Epoch of the plates, Ma from the present (negative in the past): on
    /// worlds with plate tectonics, the continents as the plates have
    /// carried them by then (see `crate::plates`). 0 is the present.
    pub drift_ma: f32,
    /// Whether the world has the moons its seed draws (see `crate::moons`).
    pub moons: crate::moons::MoonsMode,
    /// The three semi-axes of an irregular body (a moon too small to be
    /// round) as shares of its mean radius, the first towards longitude
    /// 180°, the last along the axis of rotation; `None` for a round one.
    pub shape: Option<[f32; 3]>,
    /// Metallicity of the star, `[Fe/H]` in dex (0 the Sun's): sets how
    /// likely giant planets are.
    pub metallicity: f32,
    /// Whether the world has the giant planets its seed draws (see
    /// `crate::giants`).
    pub giants: crate::giants::GiantsMode,
    /// Mass of a giant planet, Earth masses: `Some` for a giant (its look
    /// has a `[giant]` table, see `crate::giant_world`), `None` otherwise.
    pub giant_mass_earth: Option<f32>,
    /// Whether the world is the moon of a giant planet (see
    /// `crate::exomoon`): as its seed draws, always, or never.
    pub host: crate::exomoon::HostMode,
    /// A giant's rings as its planetary system drew them (see
    /// `crate::system`), which its look takes instead of drawing its own;
    /// absent for every other world, whose rings (if a giant) come from its
    /// seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub given_rings: Option<crate::giant_world::GivenRings>,
    /// The crater rate relative to the Solar System's, as its planetary
    /// system sets it from its giants and belts (see `crate::belts`);
    /// absent for every other world, whose rate comes from the giants its
    /// seed draws.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub impact_factor: Option<f32>,
    /// Whether the star's tides locked the world, as it was generated: set
    /// when a stored world is reloaded, so that its tiles and epochs keep
    /// the lock its base map was made with (see [`crate::spin::star_info`]).
    /// Stored together with [`Self::resonance_to_star`]: set one without the
    /// other and a free world takes the resonance its seed draws. Absent on
    /// a world about to be generated, whose lock follows the draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locked_to_star: Option<bool>,
    /// The turns per orbit of the spin-orbit resonance the star's tides
    /// held the world in, as it was generated (0 for none), set with
    /// [`Self::locked_to_star`] when a stored world is reloaded: the
    /// stored one is what its day and climate were made with. Absent on a
    /// world about to be generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub resonance_to_star: Option<f32>,
    /// Surface gravity relative to Earth's, when the body's own is known:
    /// a moon's, from the density its system drew for it (the gravity of
    /// its card), or a reloaded world's, as it was generated. Absent for
    /// every other world, whose gravity follows from its size (see
    /// [`WorldParams::gravity_modifier`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub gravity: Option<f32>,
    /// Height of an equatorial ridge of ring material swept up by a small
    /// ring moon (Pan, Atlas, Daphnis), as a share of its radius; it adds to
    /// the `shape` layer. Absent for every other body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ridge: Option<f32>,
    /// Longitude of the orbit's perihelion, degrees from the spring
    /// equinox in `[0, 360)`: where in the seasons the planet is closest to
    /// its star (Earth ≈ 283°, a few days after the December solstice; Mars
    /// ≈ 251°). With an eccentric orbit it makes the seasons uneven (see
    /// [`crate::climate::EccentricOrbit`]). Always drawn for a planet of a
    /// request; absent only on a moon world outside a system and on the
    /// host of an exomoon, whose seasonal swing then ignores the distance to
    /// the star.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub perihelion_longitude_deg: Option<f32>,
    /// Age of the star, Gyr, at most its main-sequence life: drawn per
    /// system or per lone world ([`crate::star::draw_age_gyr`]); it sets
    /// how far the star's tides have worked on the planet (lock, resonance,
    /// circular orbit) and how far a giant has cooled. Files without one
    /// read as [`crate::star::DEFAULT_AGE_GYR`].
    #[serde(default = "crate::star::default_age_gyr")]
    pub age_gyr: f32,
}

/// Mass–radius exponent of rocky planets, `M ∝ R^3.7` (Zeng, Sasselov &
/// Jacobsen 2016, doi:10.3847/0004-637X/819/2/127: `R/R⊕ = (1.07 − 0.21
/// CMF) (M/M⊕)^(1/3.7)`, fitted to Earth's interior model PREM for 1 to
/// 8 Earth masses). At Earth's core mass fraction (0.325) the coefficient
/// is 1.002: it is taken as 1, so that a world of Earth's size has Earth's
/// mass and gravity. Below 1 Earth mass the fit is extrapolated, down to
/// [`UNCOMPRESSED_ROCK_DENSITY`].
const ROCKY_MASS_RADIUS_EXPONENT: f32 = 3.7;
/// Bulk density of a rocky body too small to compress its own rock,
/// relative to Earth's: the Moon's 3.344 g/cm³ over Earth's 5.513 (NASA
/// planetary fact sheets). At small masses the density of a planet tends
/// to that of its uncompressed material (Seager et al. 2007,
/// doi:10.1086/521346: "for Ms ≪ 1, the EOS reduces to ρs = 1"). The
/// compressed density of [`ROCKY_MASS_RADIUS_EXPONENT`] falls to it at
/// 0.48 Earth radii.
pub(crate) const UNCOMPRESSED_ROCK_DENSITY: f32 = 3.344 / 5.513;

/// Largest surface gravity of a rocky world, Earth's = 1: the bound the
/// app kept before the mass–radius relation, reached at 2.6 Earth radii
/// (about 100 000 km round), far beyond the relation's fit (8 Earth
/// masses, 1.75 radii). Larger "rocky" worlds keep it, their mass growing
/// as their radius squared. Calibration, no source.
const MAX_ROCKY_GRAVITY: f32 = 5.0;

/// Surface gravity of a rocky body of `radius` Earth radii, relative to
/// Earth's: from its mass, `M = R^3.7` (Zeng et al. 2016), and never less
/// dense than uncompressed rock (the Moon's density), up to
/// [`MAX_ROCKY_GRAVITY`]. Mars's size gives 0.34 (Mars: 0.38, NASA), the
/// Moon's 0.165 (0.165), Venus's 0.92 (0.90). Mercury, with its large iron
/// core, has 0.38 where this gives 0.23. Zeng et al. fitted the relation on
/// 1 to 8 Earth masses; the app extends it below 1 Earth mass (Mars, the
/// Moon), beyond the fit.
pub fn rocky_gravity(radius: f32) -> f32 {
    let radius = radius.max(0.0);
    let density = radius
        .powf(ROCKY_MASS_RADIUS_EXPONENT - 3.0)
        .max(UNCOMPRESSED_ROCK_DENSITY);
    (density * radius).min(MAX_ROCKY_GRAVITY)
}

/// Circumference, km, of the rocky body of `mass_earth` Earth masses: the
/// inverse of `M = g(R) R²` with [`rocky_gravity`], so that a world of this
/// size weighs ([`WorldParams::mass_earth`]) the mass asked.
pub fn rocky_circumference_km(mass_earth: f64) -> f32 {
    let mass = |radius: f64| f64::from(rocky_gravity(radius as f32)) * radius * radius;
    // The mass grows with the radius, past 2.6 Earth radii as R² once the
    // gravity is capped: bisect between 0 and 100 Earth radii.
    let (mut low, mut high) = (0.0_f64, 100.0_f64);
    for _ in 0..60 {
        let mid = 0.5 * (low + high);
        if mass(mid) < mass_earth {
            low = mid;
        } else {
            high = mid;
        }
    }
    (0.5 * (low + high) * f64::from(EARTH_CIRCUMFERENCE_KM)) as f32
}

impl WorldParams {
    /// The elevation below which a pixel is open water: the sea level plus
    /// the look's waterline, or below any elevation on a world without
    /// surface liquid ([`crate::biome::BiomeRules::dry`]).
    pub fn open_waterline(&self) -> f32 {
        self.sea_level + self.look.rules.open_water()
    }

    /// Surface gravity relative to Earth's (1.0 = Earth's).
    ///
    /// The body's own when known ([`Self::gravity`]: a moon's, from its
    /// density); a giant's from its mass and radius; a rocky world's from
    /// its size, by the mass–radius relation of rocky planets
    /// ([`rocky_gravity`]).
    pub fn gravity_modifier(&self) -> f32 {
        if let Some(gravity) = self.gravity {
            return gravity;
        }
        let radius = self.circumference_km / EARTH_CIRCUMFERENCE_KM;
        // A giant: its own mass over its radius squared.
        if let Some(mass) = self.giant_mass_earth {
            return mass / (radius * radius);
        }
        // An icy moon asked for on its own: the density of the icy moons
        // a system draws, 1.9 g/cm³ against Earth's 5.513.
        if self.class == "icy-moon" {
            return 1.9 / 5.513 * radius;
        }
        rocky_gravity(radius)
    }

    /// Mass of a rocky world, Earth masses: its gravity times its radius
    /// squared, as [`Self::gravity_modifier`] has it.
    pub fn mass_earth(&self) -> f32 {
        let radius = self.circumference_km / EARTH_CIRCUMFERENCE_KM;
        self.gravity_modifier() * radius * radius
    }

    /// The pixel grid with its physical dimensions.
    pub fn grid(&self) -> Grid {
        Grid {
            width: self.width,
            height: self.height,
            circumference_km: self.circumference_km,
        }
    }

    /// Number of pixels in each layer.
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
}

/// A generated world: its parameters plus one value per pixel for each layer.
///
/// Every layer is stored row-major (`index = y * width + x`), with row 0 at
/// the north pole and column 0 at longitude 0.
pub struct World {
    pub params: WorldParams,
    pub width: u32,
    pub height: u32,
    /// Terrain height in `[-1, 1]` after erosion, before the sea-level shift.
    /// Biome selection uses `elevation - sea_level`.
    pub elevation: Vec<f32>,
    /// The elevation without the craters younger than the wet era (those
    /// drawn after the erosion, see [`crate::craters`]): the surface the
    /// time slider draws them on. `None` on a world reloaded from
    /// `raw_data`, which only keeps it on disk.
    pub elevation_base: Option<Vec<f32>>,
    /// Moisture in `[-1, 1]` (`> 0` is wet), planet offset included.
    pub moisture: Vec<f32>,
    /// Annual mean temperature in °C, planet offset and greenhouse included.
    pub temperature: Vec<f32>,
    /// The seasonal cycle by row (see [`crate::climate::Seasons`]),
    /// computed from the parameters.
    pub seasons: crate::climate::Seasons,
    /// Cooling of the land per elevation unit the temperatures were made
    /// with (see [`crate::climate::lapse_rate`]).
    pub lapse_rate: f32,
    /// Wet-equivalent drainage area in km² flowing through each pixel
    /// (0 on the sea): rivers are pixels above a threshold.
    pub flow_km2: Vec<f32>,
    /// Volcanic zone in `[0, 1]` as biome selection sees it (planet shift included).
    pub volcanic_zone: Vec<f32>,
    /// Distance to the nearest open water in km, capped (0 on water).
    pub coast_distance_km: Vec<f32>,
    /// Floodplain weight in `[0, 1]`: 1 on the flat valley floor of a river,
    /// 0 away from rivers (see [`crate::carve`]).
    pub floodplain: Vec<f32>,
    /// Local shift of the highland line used by biome selection.
    pub highland_offset: Vec<f32>,
    pub biomes: Vec<Biome>,
    /// Smoothed, meandering river polylines (see `rivers`).
    pub rivers: Vec<RiverLine>,
    /// Brightness shift of the ground in `[-1, 1]` (0: none), from fresh
    /// impact craters, their ejecta and rays (see
    /// [`crate::craters::CraterField::albedo`]); empty for a world reloaded
    /// from a format older than 8.
    pub albedo: Vec<f32>,
    /// Cloud cover in `[0, 1]`, empty for a world without clouds (or
    /// reloaded from `raw_data`, where the viewer reads it).
    pub clouds: Vec<f32>,
    /// The rendered colours, sRGB: a giant world's cloud bands (see
    /// `crate::giant_world`); empty for a rocky world, whose render comes
    /// from its biomes and relief.
    pub surface: Vec<[u8; 3]>,
}

impl World {
    /// The biome selection input of pixel `i`, without the lake and river
    /// flags (not stored): for lake and river pixels it selects the biome
    /// the pixel would have without them.
    pub fn biome_input(&self, i: usize) -> BiomeInput {
        let elevation = (self.elevation[i] - self.params.sea_level).clamp(-1.0, 1.0);
        let land = elevation >= self.params.look.rules.open_water();
        BiomeInput {
            elevation,
            moisture: self.moisture[i],
            temperature: self.temperature[i],
            seasonality: self.seasons.at(
                (i / self.width as usize) as u32,
                land,
                self.coast_distance_km[i],
            ),
            volcanic_zone: self.volcanic_zone[i],
            coast_distance_km: self.coast_distance_km[i],
            lake: false,
            river: false,
            highland_offset: self.highland_offset[i],
        }
    }

    /// Index of pixel `(x, y)` in the layer vectors.
    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }
}
