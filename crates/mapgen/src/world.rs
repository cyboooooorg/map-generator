use crate::biome::BiomeInput;
use crate::geo::Grid;
use crate::noise::EARTH_CIRCUMFERENCE_KM;
use crate::preset::Preset;
use crate::rivers::RiverLine;

// Re-export so existing `use crate::world::*;` in other modules keeps working.
pub use crate::biome::{Biome, biome_color, biome_name, surface_color};

/// Everything needed to generate a world. The same parameters always produce
/// the same world.
#[derive(Clone, Debug)]
pub struct WorldParams {
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    /// Planet preset: climate shift, biome rules and biome remapping.
    pub planet: Preset,
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
}

impl WorldParams {
    /// Approximate surface gravity relative to Earth (1.0 = Earth gravity).
    ///
    /// Assumes constant density, so gravity scales linearly with radius (and
    /// thus circumference). Clamped to a plausible range for rocky planets.
    pub fn gravity_modifier(&self) -> f32 {
        (self.circumference_km / EARTH_CIRCUMFERENCE_KM).clamp(0.1, 5.0)
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
    /// Cloud cover in `[0, 1]`, empty for a world without clouds (or
    /// reloaded from `raw_data`, where the viewer reads it).
    pub clouds: Vec<f32>,
}

impl World {
    /// The biome selection input of pixel `i`, without the lake and river
    /// flags (not stored): for lake and river pixels it selects the biome
    /// the pixel would have without them.
    pub fn biome_input(&self, i: usize) -> BiomeInput {
        BiomeInput {
            elevation: (self.elevation[i] - self.params.sea_level).clamp(-1.0, 1.0),
            moisture: self.moisture[i],
            temperature: self.temperature[i],
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
