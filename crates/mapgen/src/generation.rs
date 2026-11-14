use crate::atmosphere::{AIRLESS_BAR, CloudKind, LIQUID_WATER_BAR, transport_heat};
use crate::biome::BiomeInput;
use crate::carve;
use crate::climate::{Belts, DayCycle, EccentricOrbit, LockedClimate, Moisture, Seasons};
use crate::climate::{
    Field, MoistureInput, TEMPERATURE_RANGE, moisture, regional_variability, row_temperatures,
    temperature, temperature_anomaly,
};
use crate::clouds::CloudInput;
use crate::craters::{CraterField, Era, RAY_MIN_PIXELS, base_min_diameter_km, darken_maria};
use crate::delta;
use crate::erosion::erode;
use crate::geo::Grid;
use crate::geology::Regime;
use crate::hydrology::hydrology;
use crate::noise::{
    EARTH_CIRCUMFERENCE_KM, cell_value, fbm, lattice_points, ridged, smoothstep, stream,
    stream_seed,
};
use crate::planet::regime::OCEAN_KG;
use crate::rivers;
use crate::star::{StarClimate, star_temperatures};
use crate::world::*;
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use serde::Serialize;
use std::f64::consts::PI;

/// The climate noise fields of [`Sampler::climate`]: smooth, sampled on a
/// coarser grid by the generation (see [`crate::coarse`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClimateField {
    TemperatureAnomaly,
    Moisture,
    VariabilityT,
    VariabilityM,
    HighlandJitter,
}

impl ClimateField {
    pub const COUNT: usize = 5;
    /// Every field, in the order [`Sampler::sample_with`] takes them.
    pub const ALL: [ClimateField; Self::COUNT] = [
        Self::TemperatureAnomaly,
        Self::Moisture,
        Self::VariabilityT,
        Self::VariabilityM,
        Self::HighlandJitter,
    ];
}

/// Every intermediate noise value computed for one pixel.
#[derive(Clone, Copy)]
pub struct Sample {
    pub warp_x: f32,
    pub warp_y: f32,
    pub continent: f32,
    pub mountain: f32,
    pub mountain_weight: f32,
    pub elevation: f32,
    pub biome_elevation: f32,
    pub moisture: f32,
    pub volcanic_raw: f32,
    pub volcanic_zone: f32,
    /// Large-scale temperature anomaly noise, roughly in `[-1, 1]`.
    pub temperature_anomaly: f32,
    /// Regional climate variability noise for temperature and moisture.
    pub variability_t: f32,
    pub variability_m: f32,
    /// Highland line jitter noise, roughly in `[-1, 1]`.
    pub highland_jitter: f32,
    /// Height added by plate boundaries (ranges, trenches, arcs, ridges,
    /// rifts), elevation units; 0 on worlds without plate tectonics.
    pub tectonic_uplift: f32,
    /// Height of the volcanic cones included in `elevation`: the generation
    /// adds it after the hydraulic erosion, which would wear the young
    /// cones down to their base.
    pub cone_relief: f32,
    /// Height of the thick crust's Airy step the highland band ignores,
    /// elevation units (see [`platform_highlands`]): 0 where the band is
    /// an altitude above the sea.
    pub platform: f32,
}

impl Sample {
    /// Local shift of the highland line used by biome selection: its
    /// jitter, less the thick crust's step ([`Self::platform`]), so that a
    /// plateau is not a mountain.
    pub fn highland_offset(&self) -> f32 {
        HIGHLAND_JITTER * self.highland_jitter - self.platform
    }
}

/// Weight of the highland line jitter (about 0.09 of standard deviation).
const HIGHLAND_JITTER: f32 = 0.15;

/// Lateral displacement of the continent outline, per octave of the coast
/// warp, in units of the octave's wavelength (on the unit sphere).
const COAST_WARP: f64 = 0.5;
/// Ratio of the displacement share from one coast-warp octave to the next:
/// below 1, so the finest octaves fray the outline less than the coarse ones
/// shape it.
const COAST_WARP_DECAY: f64 = 0.8;
/// Height of the "plate" step added to the continents: a piecewise-constant
/// cell noise that gives coasts straight stretches and angular corners.
const PLATE_STEP: f32 = 0.06;
/// Inverse width of the plate borders, in plate cells: about 1/20 of a cell
/// (100–150 km on an Earth-sized planet), a ramp rather than a cliff.
const PLATE_SHARPNESS: f64 = 20.0;
/// Share of the fine continent octaves kept on low ground inland, so plains
/// and deserts are smooth; coasts and mountains keep them all.
const PLAIN_ROUGHNESS: f32 = 0.3;
/// Height above the waterline over which inland ground turns from coastal to
/// plain roughness.
const COAST_BAND: (f32, f32) = (0.03, 0.2);
/// Height above (or depth below) the waterline over which the outline detail
/// fades out: wide enough for every point the displacement can bring to the
/// coast, narrow enough not to wrinkle the lowlands.
const OUTLINE_BAND: (f32, f32) = (0.02, 0.08);
/// Weight of the second and third ridged octaves of the mountains.
const MOUNTAIN_DETAIL: f32 = 0.35;

/// Octaves of the coast warp: from the finest continent octave down to about
/// two pixels of the map, so the outline is fractal at the map's resolution
/// without aliasing below it.
fn coast_octaves(noise_scale: f64, grid: &Grid) -> u32 {
    let radius_km = grid.circumference_km as f64 / (2.0 * PI);
    let pixel_km = grid.circumference_km as f64 / grid.width as f64;
    // The finest continent octave has frequency 0.8 × s × 16.
    let mut wavelength = radius_km / (0.8 * noise_scale * 16.0);
    let mut octaves = 0;
    while wavelength > 2.0 * pixel_km && octaves < 10 {
        wavelength /= 2.0;
        octaves += 1;
    }
    octaves
}

/// Moisture added on a floodplain (weight 1): rivers green their valleys.
const RIPARIAN_MOISTURE: f32 = 0.15;

/// Volcanic zone away from cones, as a share of the broad zone: below the
/// ash threshold (0.15), so the zone is ordinary land dotted with volcanoes,
/// each with its vent, lava flows and an ash halo.
const VOLCANIC_BACKGROUND: f32 = 0.12;
/// Cone lattice cell, km: at most one volcano per cell.
const CONE_CELL_KM: f64 = 900.0;
/// Share of the cells with a volcano.
const CONE_PRESENCE: f64 = 0.5;
/// Cone radius range, km. In a fully volcanic zone the vent biome reaches
/// about 1.2 radii, lava flows 1.8 and ash 2.6.
const CONE_KM: (f64, f64) = (50.0, 120.0);
/// Height of a cone above the terrain at its vent, in a fully volcanic zone.
const CONE_HEIGHT: f32 = 0.25;

/// On a dry world, depth below its waterline down to which the crust
/// counts as continental, elevation units (a wet world's crust counts from
/// its crust line, see [`CONTINENTAL_SHARE`]).
const CONTINENTAL_SHELF: f32 = 0.05;
/// Share of the noise relief the world `p` keeps, so that its span (from
/// the old floor to its highest mountains, see [`noise_span_km`]) stays
/// within
///
/// - [`SMALL_BODY_RELIEF`] of its radius: real bodies from Mars down to
///   Vesta carry relief of tens of km whatever their size, a larger share
///   of their radius the smaller they are, up to Vesta's 8 %;
/// - on an airless body with a stagnant lid (a moon, a dead world), the
///   relief impacts give it, [`IMPACT_RELIEF_KM`]: there neither plates
///   nor volcanism build ranges near the `1/g` ceiling. Ice holds
///   [`ICE_RELIEF`] of rock's.
/// - on an airless body whose lid overturns now and then (episodic, like
///   Venus's), the same: between overturns its lid is a stagnant one, and
///   without air nothing else shapes it. A heat-pipe body (Io) is left
///   out: its lid sinks under the lava piled on it and breaks into blocks
///   up to 17 km high (Schenk et al. 2001, doi:10.1029/2000JE001408), and
///   lava, not impacts, resurfaces it.
///
/// 1 for planets, whose relief comes from their plates, mountains and
/// volcanoes.
pub(crate) fn relief_share(p: &WorldParams) -> f32 {
    let radius_km = p.circumference_km / (2.0 * std::f32::consts::PI);
    let mut cap = SMALL_BODY_RELIEF * radius_km;
    let impact_shaped = matches!(p.look.geology.regime, Regime::Stagnant | Regime::Episodic);
    if impact_shaped && p.pressure_bar <= AIRLESS_BAR {
        let ice = if icy(p) { ICE_RELIEF } else { 1.0 };
        cap = cap.min(IMPACT_RELIEF_KM * ice);
    }
    (cap / noise_span_km(p)).min(1.0)
}

/// Span of the noise relief of the world `p`, km: from the bottom of the
/// abyss ([`ABYSS_KM`]) to the top of its floating crust ([`crust_of`])
/// and of the mountains at its gravity; on a dry world, which has no sea
/// floor, from the bottom of its rolling plains ([`DRY_PLAINS_KM`]).
fn noise_span_km(p: &WorldParams) -> f32 {
    let scale = relief_scale(p.gravity_modifier());
    let mountains = MOUNTAIN_BLEND * scale * crate::craters::ELEVATION_KM;
    if p.look.rules.dry {
        // No sea floor: the rolling plains instead.
        DRY_PLATFORM_KM + DRY_PLAINS_KM * DRY_PLAINS_SPAN * scale + mountains
    } else {
        ABYSS_KM - crate::plates::OLD_FLOOR_KM + crust_of(p).contrast_km() + mountains
    }
}

/// What a world's crust is made of, as its craters and relief feel it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CrustKind {
    /// Dry rock, or crust recycled by plates (Mercury, Venus, Earth).
    Rock,
    /// Rock rich in volatiles under a stagnant lid, weaker (Mars).
    WetRock,
    /// An ice shell (Ganymede, Europa).
    Ice,
}

/// Earth oceans of water a world without plates and without a runaway
/// keeps, from which its crust is rich in volatiles ([`CrustKind::WetRock`]).
/// Calibration, no source: Mars's polar deposits and ground ice, about
/// 0.004 oceans (Carr & Head 2015, as
/// [`crate::planet::classify::ICE_WORLD_MIN_OCEANS`] gives them; the
/// `mars-like` model keeps 0.006), lie above it, Mercury's none below it.
const WET_CRUST_MIN_OCEANS: f64 = 1e-3;

/// What the crust of the world `p` is made of, read from its model:
///
/// - an ice shell when it has at least
///   [`crate::planet::classify::ICE_WORLD_MIN_WATER`] of water and ices,
///   keeps at least [`crate::planet::classify::ICE_WORLD_MIN_OCEANS`] of
///   them at its surface and has no liquid water there (an ice world: an
///   ocean world's sea floor is rock, and a frozen Earth's ice sheet lies
///   on rock);
/// - wet rock without plates and without a runaway, keeping over
///   [`WET_CRUST_MIN_OCEANS`] of water (Mars);
/// - rock otherwise.
///
/// A model without a surface (an envelope hides it) is rock. Without a
/// model (a moon, part 4): ice when the bulk density is under 2.5 g/cm³
/// (rock and iron bodies are denser, the Moon 3.34; icy moons 1.8–1.9,
/// Ganymede and Callisto, NASA), rock otherwise.
pub(crate) fn crust_kind(p: &WorldParams) -> CrustKind {
    use crate::planet::classify::{ICE_WORLD_MIN_OCEANS, ICE_WORLD_MIN_WATER};
    use crate::planet::climate::SurfaceLiquid;
    let Some(model) = &p.model else {
        let radius = p.circumference_km / EARTH_CIRCUMFERENCE_KM;
        return if radius > 0.0 && 5.513 * p.gravity_modifier() / radius < 2.5 {
            CrustKind::Ice
        } else {
            CrustKind::Rock
        };
    };
    let Some(surface) = &model.surface else {
        return CrustKind::Rock;
    };
    if model.inputs.water_fraction >= ICE_WORLD_MIN_WATER
        && surface.water.oceans >= ICE_WORLD_MIN_OCEANS
        && surface.climate.liquid != SurfaceLiquid::Water
    {
        CrustKind::Ice
    } else if surface.regime != Regime::Plates
        && !surface.water.runaway
        && surface.water.oceans > WET_CRUST_MIN_OCEANS
    {
        CrustKind::WetRock
    } else {
        CrustKind::Rock
    }
}

/// Whether the crust of the world `p` is ice ([`crust_kind`]).
pub(crate) fn icy(p: &WorldParams) -> bool {
    crust_kind(p) == CrustKind::Ice
}

/// Largest relief of a small body, as a share of its radius: Vesta's
/// central mound in Rheasilvia, about 22 km high (Yingst et al. 2012, EGU
/// abstract, approximate) on a mean radius of about 262 km (NASA): 8 %.
const SMALL_BODY_RELIEF: f32 = 0.08;
/// Span of the noise relief of an airless body with a stagnant lid, km,
/// so that a Moon-sized rocky moon spans about the Moon's 13 km (NASA fact
/// sheet, as the mountains article gives it) with its craters.
/// Calibration on that one body.
const IMPACT_RELIEF_KM: f32 = 9.0;
/// Relief ice holds against rock: complex craters on Ganymede are about
/// 1.1 km deep at 26 km (Schenk 2002, doi:10.1038/417419a, read in the
/// letter's NTRS copy), where Pike's lunar law gives 2.8 km at nearly the
/// same gravity: 0.4. A calibration: taking that crater-depth ratio for
/// the share of all relief ice holds is an extrapolation. Icy bodies'
/// complex craters take it too (`craters::CraterField`).
pub(crate) const ICE_RELIEF: f32 = 0.4;

/// Height of the continents' noise above the crust line (see
/// [`CONTINENTAL_SHARE`]) from which a wet world's mountains rise (over
/// 0.4 more): ranges stand on thick crust. Calibration with
/// [`MOUNTAIN_BLEND`] on Earth's hypsometry.
const MOUNTAIN_ONSET: f32 = 0.28;
/// Height of the continents' noise above a dry world's waterline from
/// which its mountains rise (over 0.4 more): measured from the waterline,
/// as it was before the crust floated by its thickness, which dry worlds
/// do not (their plains stand on [`DRY_PLATFORM_KM`]). Calibration with
/// [`MOUNTAIN_BLEND`], made on Earth's hypsometry when every world's relief
/// was measured from its waterline, kept so that the dry worlds'
/// highlands (`venus-like`, see [`HIGHLANDS`]) do not move.
const DRY_MOUNTAIN_ONSET: f32 = 0.15;

/// Mountain blend at Earth's gravity: the weight of the ridged mountains
/// in the elevation. Calibration, no source: with the floating platform
/// (see [`floating`]), it puts about a tenth of an Earth-like world's land
/// above 2 km and 2 % above 4 km, as on Earth (ETOPO 2022).
const MOUNTAIN_BLEND: f32 = 0.45;
/// Largest relief scale ([`relief_scale`]): Olympus Mons's 21.9 km of
/// relief over its surroundings (Plescia 2004, doi:10.1029/2002JE002031)
/// is 2.5 times Everest's 8.85 km, the tallest mountain known against
/// Earth's tallest. Lighter bodies (the Moon, Io, Vesta) do not come near
/// their `1/g` ceiling, and higher ranges would fill the elevation range
/// (one unit is 25 km) with flat clipped summits.
const MAX_RELIEF_SCALE: f32 = 2.0;

/// How tall the relief built on a world of `gravity` (Earth's = 1) stands
/// against Earth's: `1/g`, the height rock can bear under its own weight
/// (`σ / ρ g`, Weisskopf 1975, doi:10.1126/science.187.4177.605), up to
/// [`MAX_RELIEF_SCALE`]. Mars's size (0.34 g) reaches the cap; twice
/// Earth's (3.2 g) keeps 31 %.
///
/// It scales what loads the crust: the mountains, the plate boundaries'
/// ranges, arcs, trenches and ridges, and the volcanoes. The continents and
/// the sea floor do not scale: they float on the mantle (Airy isostasy,
/// Airy 1855, doi:10.1098/rstl.1855.0003), and the height of a floating
/// crust, `t (1 − ρc/ρm)` for a crust `t` thick, does not depend on `g`.
pub(crate) fn relief_scale(gravity: f32) -> f32 {
    (1.0 / gravity.max(1e-3)).min(MAX_RELIEF_SCALE)
}

/// Share of a wet world's surface its thick crust covers (continental
/// crust; highlands without plates): Earth's continental crust covers 41 %
/// of its surface, 210.4 × 10⁶ km² with the submerged margins (Cogley
/// 1984, doi:10.1029/RG022i002p00101). The crust line, the level of the
/// continents' noise between thin crust below it and thick crust above
/// it, is the one that leaves this share above it ([`Sampler`]'s
/// `crust_line`): it does not follow the sea (the sea fills the relief
/// from its water, [`sea_level_for_water`]). A fixed noise level gave
/// Earth-like worlds 36 to 44 % of continental crust by seed, and their
/// seas, filled from the same water, then stood 0.5 km apart on their
/// continents: no calibration kept Earth's hypsometry on every seed.
const CONTINENTAL_SHARE: f64 = 0.41;
/// The grid the crust line is measured on: fixed, so that a world's crust
/// does not depend on its resolution.
const CRUST_GRID: (u32, u32) = (128, 64);
/// Elevation of the top of the oceanic crust's column, where the slope
/// meets the old floor ([`floating`] is measured from it): the datum of a
/// wet world's relief, which the sea fills. Calibration, no source: it
/// leaves Earth's ocean ([`ocean_depth_m`]) at a sea level near 0.26,
/// where it sat before the crust floated by its thickness (0.27–0.31),
/// with room in the range above and below.
///
/// The datum does not set the depths under the sea: the water fills the
/// relief from its volume, so the sea follows any move of the datum. With
/// Earth's ocean and 41 % of continental crust, the old floor lies about
/// 3.25 km under the sea on `earth-like` worlds (3.24–3.29 km on seeds 1–3,
/// 3.25 at 1024 × 512), not Parsons & Sclater's 4 km, and the abyss
/// reaches about 5.65 km, not 6.4: Earth's ocean, 3.7 km deep on average,
/// keeps its old floor at 4 km because ridges and margins hold much of it
/// above that depth, which the noise's basins do not. Putting the old
/// floor at 4 km here would flatten the abyss (a slower [`ABYSS_NOISE`]
/// of 1.0 gives 3.9–4.0 km, but 1 % of the sea deeper than 5 km against
/// 13–16 %).
const CRUST_DATUM: f32 = -0.02;

/// The crust of a world, as its relief floats on the mantle (Airy
/// isostasy, see [`floating`]): the thickness of its thick crust
/// (continents, or highlands without plates) and of its thin crust (ocean
/// floor, or lowlands), km, and the densities of crust and mantle, g/cm³.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Crust {
    pub continental_km: f32,
    pub oceanic_km: f32,
    pub crust_density: f32,
    pub mantle_density: f32,
}

impl Crust {
    /// Height of a column of crust `thickness_km` thick above a column of
    /// thin crust, km: `(t − t_o)(1 − ρc/ρm)` (Airy 1855,
    /// doi:10.1098/rstl.1855.0003). Gravity cancels out.
    pub fn height_km(&self, thickness_km: f32) -> f32 {
        (thickness_km - self.oceanic_km) * (1.0 - self.crust_density / self.mantle_density)
    }

    /// Height of the thick crust above the thin, km.
    pub fn contrast_km(&self) -> f32 {
        self.height_km(self.continental_km)
    }
}

/// Earth's crust, for worlds with plates.
///
/// - Continental crust 41.1 km thick, the global mean weighted by the area
///   of each crustal province (Christensen & Mooney 1995,
///   doi:10.1029/95JB00259); Cogley (1984, doi:10.1029/RG022i002p00101)
///   finds 36 km. Of the two, the thicker keeps Earth's hypsometry: a
///   thinner continent leaves less room in the basins below it.
/// - Oceanic crust 7.1 km, its igneous section away from fracture zones
///   and hot spots (White et al. 1992, doi:10.1029/92JB01749).
/// - Crust 2.83 g/cm³, the mean of Christensen & Mooney's (1995) model of
///   the continental crust; mantle 3.3 g/cm³, a typical value for the
///   uppermost mantle (assumed, as in the science page's example).
///
/// The continents then stand 4.8 km above the old ocean floor.
const EARTH_CRUST: Crust = Crust {
    continental_km: 41.1,
    oceanic_km: 7.1,
    crust_density: 2.83,
    mantle_density: 3.3,
};

/// Mars's crust, for worlds without plates: no continents, a thick
/// highland crust and a thin lowland one, the modal thicknesses of Mars's
/// southern and northern hemispheres, 58 and 32 km, with a mantle
/// 600 kg/m³ denser than the crust (Neumann et al. 2004,
/// doi:10.1029/2004JE002262). The crust's 2.9 g/cm³ (basalt) is assumed,
/// so the mantle 3.5. The highlands stand 4.5 km above the lowlands, the
/// order of Mars's dichotomy. Calibration on that one body: the thickness
/// of a stagnant lid's crust from its mass and heat is not modelled.
const MARS_CRUST: Crust = Crust {
    continental_km: 58.0,
    oceanic_km: 32.0,
    crust_density: 2.9,
    mantle_density: 3.5,
};

/// The crust of the world `p`: Earth's with plates ([`EARTH_CRUST`]),
/// Mars's dichotomy without ([`MARS_CRUST`]). Neither depends on gravity.
pub(crate) fn crust_of(p: &WorldParams) -> Crust {
    if p.look.geology.regime == Regime::Plates {
        EARTH_CRUST
    } else {
        MARS_CRUST
    }
}

/// Height of a dry world's platform above its waterline, km: where its
/// plains stand (see [`DRY_PLAINS_KM`]). The height of the continental
/// platform above the waterline before the crust floated by its
/// thickness; a dry world, which no sea ever shaped, keeps it.
/// Calibration on Earth's hypsometry at the time, no source.
const DRY_PLATFORM_KM: f32 = 0.5;

/// Rise of the platform inland, km, from its inner edge (the crust's
/// contrast less this) to the full continental thickness, and its width in
/// the continents' noise: `1 − 1/e` of its rise at [`PLATFORM_RISE`].
const PLATFORM_KM: f32 = 1.45;
const PLATFORM_RISE: f32 = 0.42;
/// Depth of the shelf's edge below the platform's inner edge, km, and the
/// shelf's width in the continents' noise.
const SHELF_KM: f32 = 0.3;
const SHELF_NOISE: f32 = 0.06;
/// Width of the continental slope in the noise, from the old floor (at the
/// crust line) up to the shelf's edge.
const SLOPE_NOISE: f32 = 0.03;
/// Depth of old ocean floor, km: from where Parsons & Sclater's floor is
/// 18 Ma old (4 km, `plates::OLD_FLOOR_KM`) to their old floor's level,
/// 6.4 km (Parsons & Sclater 1977, doi:10.1029/JB082i005p00803), reached
/// as the noise falls further by about [`ABYSS_NOISE`] below the crust
/// line.
const ABYSS_KM: f32 = 6.4;
const ABYSS_NOISE: f32 = 0.2;

/// Thickness of the crust `crust`, km, for the continents' noise `x`
/// units above (below) the crust line (see [`CONTINENTAL_SHARE`]):
/// oceanic below the line; above it,
/// thickening across the continental slope ([`SLOPE_NOISE`]) and the
/// shelf ([`SHELF_NOISE`]) to the platform's inner edge, then rising
/// inland by [`PLATFORM_KM`] of height towards the continental thickness.
pub(crate) fn crust_thickness_km(x: f32, crust: &Crust) -> f32 {
    let contrast = crust.contrast_km();
    let rise = (contrast - PLATFORM_KM).max(0.0);
    let height_km = if x <= 0.0 {
        0.0
    } else if x < SLOPE_NOISE {
        (rise - SHELF_KM) * smoothstep(0.0, 1.0, x / SLOPE_NOISE)
    } else if x < SLOPE_NOISE + SHELF_NOISE {
        rise - SHELF_KM + SHELF_KM * (x - SLOPE_NOISE) / SHELF_NOISE
    } else {
        let v = x - SLOPE_NOISE - SHELF_NOISE;
        rise + (contrast - rise) * (1.0 - (-v / PLATFORM_RISE).exp())
    };
    crust.oceanic_km + height_km / (1.0 - crust.crust_density / crust.mantle_density)
}

/// Height of the crust `crust` above the old ocean floor (the top of the
/// oceanic crust's column, [`CRUST_DATUM`]), elevation units, for the
/// continents' noise `x` units above (below) the crust line (see
/// [`CONTINENTAL_SHARE`]).
///
/// The crust floats on the mantle (Airy isostasy, Airy 1855,
/// doi:10.1098/rstl.1855.0003): a column `t` thick stands
/// `(t − t_o)(1 − ρc/ρm)` above a column of oceanic crust `t_o` thick
/// ([`Crust::height_km`]), whatever the gravity. The thickness follows the
/// noise ([`crust_thickness_km`]): continental crust stands as a platform
/// (Earth's 4.8 km above the old floor), oceanic crust as the floor of
/// the basins, with little in between. Below the crust line, the old floor
/// deepens by up to Parsons & Sclater's 6.4 km less 4 km ([`ABYSS_KM`],
/// `plates::OLD_FLOOR_KM`) as the noise falls (the sea floor's age is not
/// modelled, #317); only trenches go deeper. Under Earth's ocean, that is
/// from about 3.25 to 5.65 km under the sea (see [`CRUST_DATUM`]).
///
/// The sea does not enter: it fills this relief from its water
/// ([`sea_level_for_water`]), so a world with less water than Earth's
/// exposes its shelves, then its ocean floors.
///
/// The widths in the noise, the shelf and the platform's rise, and the
/// mountains' onset and weight ([`MOUNTAIN_ONSET`], [`MOUNTAIN_BLEND`])
/// are calibrations, no source, on
/// Earth's hypsometry (ETOPO 2022, NOAA NCEI, doi:10.25921/fd45-gt74; ice
/// surface): land median about 0.42 km, 56 % of land under 500 m, 10.8 %
/// above 2 km, 1.8 % above 4 km, 1 % of the sea floor deeper than 6 km.
pub(crate) fn floating(x: f32, crust: &Crust) -> f32 {
    let km = crate::craters::ELEVATION_KM;
    let old_floor = crate::plates::OLD_FLOOR_KM;
    let abyss_km = if x < 0.0 {
        (ABYSS_KM - old_floor) * (1.0 - (x / ABYSS_NOISE).exp())
    } else {
        0.0
    };
    (crust.height_km(crust_thickness_km(x, crust)) - abyss_km) / km
}

/// Level of the continents' noise at which the floating crust `crust`,
/// measured from the crust line `line`, meets the waterline `waterline`
/// (elevation): where the coast lies,
/// before the mountains, plates and outline move it. Found by bisection
/// ([`floating`] rises with the noise); beyond the relief's reach, the
/// nearest end of the searched span.
fn shore_noise(waterline: f32, crust: &Crust, line: f32) -> f32 {
    let (mut lo, mut hi) = (line - 2.0, line + 2.0);
    for _ in 0..32 {
        let mid = 0.5 * (lo + hi);
        if floating(mid - line, crust) + CRUST_DATUM < waterline {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Elevation beyond which the relief is compressed into the range
/// `[-1, 1]` rather than clipped, in units.
const RANGE_KNEE: f32 = 0.85;

/// An elevation brought within `[-1, 1]`: unchanged up to
/// [`RANGE_KNEE`], compressed smoothly beyond, so that the tall relief of
/// light worlds keeps its summits instead of flat clipped tops (on
/// Earth-sized worlds, only the highest volcanoes reach the knee).
fn within_range(e: f32) -> f32 {
    let over = e.abs() - RANGE_KNEE;
    if over <= 0.0 {
        return e;
    }
    let room = 1.0 - RANGE_KNEE;
    e.signum() * (RANGE_KNEE + room * (over / room).tanh())
}

/// Largest noise scale (relief features per Earth-sized ones), reached at
/// about 5 000 km around.
const MAX_NOISE_SCALE: f32 = 8.0;
/// Depth below the crust line of the crust formed where the plates part,
/// in continent-shape units; the crust itself is laid at the old floor
/// ([`CRUST_DATUM`], about 3.25 km under Earth's sea), the swell
/// of the ridge it formed at standing on it by its age (see
/// `plates::ridge_swell`). The share of the fine octaves it keeps.
const NEW_CRUST_DEPTH: f32 = crate::plates::OLD_FLOOR_KM / crate::craters::ELEVATION_KM;
const NEW_CRUST_ROUGHNESS: f32 = 0.5;
/// Share of the noise's mountain weight kept on worlds with plate
/// tectonics: the rest of their ranges follow the plate boundaries.
const NOISE_MOUNTAINS_WITH_PLATES: f32 = 0.5;
/// The highland provinces of a world without plates (stagnant, episodic
/// or heat-pipe lids), the only places its noise mountains rise: with no
/// plates to stack crust into ranges, such a world is plains with what the
/// generator places on them (craters, basins, maria, volcanoes, old sea
/// floors), and a few highland provinces, not a mountain fabric over the
/// whole globe (a dry world stands all above the mountains' onset). On
/// Venus, tesserae cover about 7-8 % of the surface (Ivanov & Head 2011,
/// doi:10.1016/j.pss.2011.07.008) and the highlands with them about a
/// tenth, against 70-80 % of plains; the hypsometry is unimodal, more
/// than 80 % of the surface within 1 km of the mean radius (Ford &
/// Pettengill 1992, doi:10.1029/92JE01085, from Magellan) and about 5 %
/// more than 2 km above it (Pettengill et al. 1980,
/// doi:10.1029/JA085iA13p08261), Maxwell Montes about 11 km.
///
/// A three-octave noise at a fixed frequency on the unit sphere, a few
/// provinces whatever the world's size, ramps the mountains in between
/// `edge.0` and `edge.1`. Calibration: over 40 seeds, the noise passes the
/// middle of the ramp (0.28) on about 9 % of the sphere (its 90th
/// percentile is 0.27, its 92nd 0.29), the top of the ramp on about 5 %,
/// its foot on about 14 %; a seed draws more or fewer. Inside, the
/// mountains keep their whole weight: the `venus-like` archetype spans
/// 16 km, 2.6 km below its median to 13 km above it (Venus: about 3 km
/// below its mean radius to Maxwell's 11 km above), `mars-like` 30 km
/// (Mars's dynamic range is about 30 km, Smith et al. 1999,
/// doi:10.1126/science.284.5419.1495). `mercury-like` spans 18 km, about
/// twice Mercury's 9.9 km (Becker et al. 2016, LPSC abstract 2959): its
/// craters, 7.7 km deep at most, and its highlands under the impact cap
/// ([`IMPACT_RELIEF_KM`], calibrated on the Moon).
struct Highlands {
    frequency: f64,
    octaves: u32,
    edge: (f32, f32),
}
const HIGHLANDS: Highlands = Highlands {
    frequency: 1.6,
    octaves: 3,
    edge: (0.23, 0.33),
};
/// Weight of volcanic arcs in the volcanic zone, on a world without
/// volcanism and on a fully volcanic one.
const ARC_VOLCANISM: (f32, f32) = (0.8, 1.0);
/// Volcanoes of an arc: one per cell of this size (km) at most, present in
/// this share of the cells (arcs on Earth have one every 50–100 km; at the
/// base map's resolution, about 20 km per pixel, they would merge).
const ARC_CONE_CELL_KM: f64 = 200.0;
const ARC_CONE_PRESENCE: f64 = 0.8;
/// Height of an arc volcano above the terrain at its vent, at full arc
/// weight (the cones of volcanic provinces: [`CONE_HEIGHT`]).
const ARC_CONE_HEIGHT: f32 = 0.4;
/// Arc weights over which arc volcanoes grow from nothing to their full
/// height: gradually, as a cut would draw a cliff of up to 10 km along
/// the arc's edge (where it follows the coarse plate grid).
const ARC_MIN: f32 = 0.02;
const ARC_FULL: f32 = 0.3;

/// The volcanoes at a point, from its cone profile `cone` (see
/// [`Sampler::cone`]), its arc volcanoes' `arc_cone` and the arc weight
/// `arc`: their share of the volcanic zone and their height, both
/// continuous (no step where arc volcanoes start, nor where they take
/// over from the cones of volcanic provinces).
fn volcanoes(cone: f32, arc_cone: f32, arc: f32) -> (f32, f32) {
    let arc_cone = arc_cone * smoothstep(ARC_MIN, ARC_FULL, arc);
    (
        cone.max(arc_cone),
        (CONE_HEIGHT * cone).max(ARC_CONE_HEIGHT * arc_cone),
    )
}

/// Share of the surface in volcanic provinces at a volcanic intensity of 1;
/// it is proportional to the intensity.
const VOLCANIC_PROVINCE_SHARE: f32 = 0.3;
/// Spread of the volcanic noise (a 3-octave fbm): roughly normal with this
/// standard deviation, measured on terran worlds (its 80, 95 and 99 %
/// quantiles about 0.18, 0.34 and 0.44).
const VOLCANIC_NOISE_SIGMA: f32 = 0.21;

/// The threshold of the volcanic noise above which the ground is a volcanic
/// province: its quantile for a share of the surface
/// [`VOLCANIC_PROVINCE_SHARE`] × `intensity` (12 % at 0.4, 30 % at 1, none
/// at 0). A threshold of `1 − intensity` gave no province at all up to an
/// intensity of about 0.5, the noise seldom exceeding 0.5, and half the
/// surface at 1.
fn volcanic_threshold(intensity: f32) -> f32 {
    let share = VOLCANIC_PROVINCE_SHARE * intensity.clamp(0.0, 1.0);
    if share <= 0.0 {
        return f32::INFINITY;
    }
    // Inverse of the normal distribution (Tukey's lambda approximation,
    // within about 1 %).
    let p = 1.0 - share;
    let z = 4.91 * (p.powf(0.14) - (1.0 - p).powf(0.14));
    VOLCANIC_NOISE_SIGMA * z
}

/// Strength of the carbon cycle of a wet world with a stagnant lid, against
/// a plate world's: Foley & Smye (2018, doi:10.1089/ast.2017.1695) find
/// that volcanic outgassing can sustain one there for about 1 to 5 Gyr,
/// though plates remain "the more favorable tectonic state for long-term
/// climate stability" (Foley 2018, doi:10.1098/rsta.2017.0409). The half
/// is a calibration, no source.
const STAGNANT_LID_REGULATION: f32 = 0.5;

/// How strongly a carbonate-silicate cycle steadies the climate of a world
/// of `regime` and surface pressure `pressure_bar` (see
/// [`StarClimate::regulation`]): fully with plates and liquid water, by
/// [`STAGNANT_LID_REGULATION`] with a stagnant lid and liquid water, not
/// at all otherwise (no rock weathers without water; the app gives none
/// to episodic and heat-pipe worlds, Venus and Io having no water).
pub fn carbon_cycle(regime: Regime, pressure_bar: f32) -> f32 {
    if pressure_bar < LIQUID_WATER_BAR {
        return 0.0;
    }
    match regime {
        Regime::Plates => 1.0,
        Regime::Stagnant => STAGNANT_LID_REGULATION,
        Regime::Episodic | Regime::HeatPipe => 0.0,
    }
}

/// Noise sources and derived constants for one world.
pub struct Sampler {
    grid: Grid,
    /// The plates and their boundaries, on worlds with plate tectonics.
    tectonics: Option<crate::plates::PlateMap>,
    /// Share of the noise's mountain weight kept on worlds with plates:
    /// their ranges follow the plate boundaries.
    noise_mountains: f32,
    /// Weight of volcanic arcs in the volcanic zone.
    arc_volcanism: f32,
    sea_level: f32,
    /// Absolute elevation of the sea surface; on a dry world, the old
    /// floor of the range, where the whole shape stands above it.
    waterline: f32,
    /// The crust the relief floats with (see [`floating`]).
    crust: Crust,
    /// Level of the continents' noise above which the crust is
    /// continental: on a wet world, the level that leaves
    /// [`CONTINENTAL_SHARE`] of its surface above it, whatever its sea,
    /// which its platform, mountains and inland plains are measured from;
    /// on a dry world, [`CONTINENTAL_SHELF`] below its waterline (its
    /// relief is measured from the waterline itself).
    crust_line: f32,
    /// Level of the continents' noise at the coast, where the floating
    /// crust meets the waterline: the coasts' outline is drawn around it.
    shore: f32,
    /// On a dry world (see [`crate::biome::BiomeRules::dry`]), its
    /// waterline, the datum its plains are laid from.
    dry_waterline: Option<f32>,
    /// On a world with its liquid kept near the poles (see
    /// [`WorldParams::liquid_poleward_deg`]): the sine of that latitude,
    /// and the elevation the ground stands at least at below it.
    dry_band: Option<(f64, f32)>,
    noise_scale: f64,
    coast_octaves: u32,
    coast_noise: Perlin,
    plate_seed: u32,
    cone_seed: u32,
    arc_cone_seed: u32,
    mountain_blend: f32,
    /// Height of the relief built on the crust against Earth's (see
    /// [`relief_scale`]).
    relief_scale: f32,
    volcanic_threshold: f32,
    elevation_noise: Perlin,
    moisture_noise: Perlin,
    continent_noise: Perlin,
    warp_noise_a: Perlin,
    warp_noise_b: Perlin,
    volcano_noise: Perlin,
    /// Continental-scale temperature anomalies (currents, monsoons, continentality).
    anomaly_noise: Perlin,
    /// Regional climate variability (soils, aspect, local circulation).
    variability_noise: Perlin,
    highland_noise: Perlin,
    /// The highland provinces of a world without plates, outside which
    /// the noise mountains vanish (see [`HIGHLANDS`]); `None` with plates.
    highlands: Option<Perlin>,
    /// Whether the highland band ignores the thick crust's Airy step
    /// (see [`platform_highlands`]).
    platform_highlands: bool,
}

/// Whether the highland band of the world `p` (the biomes mountain and
/// snow, [`crate::biome::BiomeRules::highland`]) ignores the Airy step of
/// its thick crust: the height its floating crust stands above the thin
/// crust's datum ([`CRUST_DATUM`], see [`floating`]) is taken off the
/// elevation the band reads, so a plateau counts as if its crust were
/// thin. On a world without plates whose crust floats (not a dry world,
/// whose plains stand on [`DRY_PLATFORM_KM`] from its waterline), with a
/// planet model.
///
/// Such a world's thick crust is a highland province of Mars's dichotomy
/// ([`MARS_CRUST`]): a plateau 4.5 km above its lowlands, kilometres
/// above a sea filled from little water, but flat. Mars's southern
/// highlands are cratered plains, not mountains. Measured from the sea,
/// the band turned the whole plateau into mountains (`mars-like`: 41 % of
/// its surface, against 9 % before the crust floated); without the step,
/// 11-16 % on seeds 1-3. A choice, no source: the band ignores the
/// buoyancy of the crust, not the relief built on it (mountains,
/// volcanoes, crater rims), whose summits can still be snow.
///
/// Worlds with plates keep the band above the sea, as do dry worlds and
/// small moons (no model).
fn platform_highlands(p: &WorldParams) -> bool {
    p.model.is_some() && p.look.geology.regime != Regime::Plates && !p.look.rules.dry
}

/// Height above a world's open water at which the ground of its dry
/// latitudes stands at least, in elevation units: just over the shore.
const DRY_BAND_FLOOR: f32 = 0.01;

/// The latitude, degrees, equatorward of which a world has no surface
/// liquid ([`WorldParams::liquid_poleward_deg`]), and the lowest elevation
/// its ground there stands at: the open water's level and a little more.
fn dry_band(p: &WorldParams) -> Option<(f32, f32)> {
    let lat = p.liquid_poleward_deg?;
    Some((lat, p.open_waterline() + DRY_BAND_FLOOR))
}

/// Keeps the dry latitudes of a world whose liquid gathers near the poles
/// ([`WorldParams::liquid_poleward_deg`]) above its open water, after a
/// pass (erosion, craters, valleys) that may have dug them under it, and
/// clears the lakes there. Does nothing on any other world.
fn keep_band_dry(
    params: &WorldParams,
    grid: &Grid,
    elevation: &mut [f32],
    lake: Option<&mut [bool]>,
) {
    let Some((lat, floor)) = dry_band(params) else {
        return;
    };
    let width = grid.width as usize;
    let dry = |i: usize| grid.latitude((i / width) as u32).to_degrees().abs() < f64::from(lat);
    elevation
        .par_iter_mut()
        .enumerate()
        .filter(|(i, _)| dry(*i))
        .for_each(|(_, e)| *e = e.max(floor));
    if let Some(lake) = lake {
        lake.par_iter_mut()
            .enumerate()
            .filter(|(i, _)| dry(*i))
            .for_each(|(_, l)| *l = false);
    }
}

impl Sampler {
    pub fn new(p: &WorldParams) -> Self {
        Self::with_frame(p, None)
    }

    /// Like [`Self::new`], the plates of a world at the present taken from
    /// `frame` when given (built for the same world, see
    /// [`crate::plates::PlateFrame`]): only their crust is found again.
    pub fn with_frame(p: &WorldParams, frame: Option<&crate::plates::PlateFrame>) -> Self {
        let seed = p.seed;
        // Scale noise frequencies by planet size: a larger circumference
        // stretches the unit-sphere coordinates, producing broader continents.
        // Earth (40 075 km) ≡ scale 1.0.
        // Capped: below about 5 000 km around (small moons), the relief
        // would turn into grain.
        let noise_scale =
            (EARTH_CIRCUMFERENCE_KM / p.circumference_km.max(1.0)).min(MAX_NOISE_SCALE) as f64;
        let grid = p.grid();
        let crust = crust_of(p);
        let (waterline, crust_line, shore) = if p.look.rules.dry {
            let waterline = crate::request::SEA_LEVEL.min + p.look.rules.waterline;
            (waterline, waterline - CONTINENTAL_SHELF, waterline)
        } else {
            // Set below, from the continents' noise.
            (p.sea_level + p.look.rules.waterline, 0.0, 0.0)
        };
        let mut sampler = Self {
            tectonics: None,
            noise_mountains: 1.0,
            arc_volcanism: ARC_VOLCANISM.0
                + (ARC_VOLCANISM.1 - ARC_VOLCANISM.0) * p.volcanic_intensity.clamp(0.0, 1.0),
            coast_octaves: coast_octaves(noise_scale, &grid),
            grid,
            sea_level: p.sea_level,
            waterline,
            crust,
            crust_line,
            shore,
            dry_waterline: p
                .look
                .rules
                .dry
                .then_some(p.sea_level + p.look.rules.waterline),
            dry_band: dry_band(p).map(|(lat, floor)| (f64::from(lat).to_radians().sin(), floor)),
            noise_scale,
            coast_noise: Perlin::new(stream_seed(seed, stream::COAST)),
            plate_seed: stream_seed(seed, stream::PLATES),
            cone_seed: stream_seed(seed, stream::CONES),
            arc_cone_seed: stream_seed(seed, stream::ARC_CONES),
            // Stronger gravity lowers the mountains, as 1/g (capped).
            mountain_blend: MOUNTAIN_BLEND * relief_scale(p.gravity_modifier()),
            relief_scale: relief_scale(p.gravity_modifier()),
            // The noise's quantile for the share of the surface the
            // intensity asks for (see `volcanic_threshold`).
            volcanic_threshold: volcanic_threshold(p.volcanic_intensity),
            // wrapping_add: a random seed close to u32::MAX must not overflow.
            elevation_noise: Perlin::new(stream_seed(seed, stream::ELEVATION)),
            moisture_noise: Perlin::new(stream_seed(seed, stream::MOISTURE)),
            continent_noise: Perlin::new(stream_seed(seed, stream::CONTINENT)),
            warp_noise_a: Perlin::new(stream_seed(seed, stream::WARP_A)),
            warp_noise_b: Perlin::new(stream_seed(seed, stream::WARP_B)),
            // Low-frequency noise that selects which mountain chains turn volcanic.
            volcano_noise: Perlin::new(stream_seed(seed, stream::VOLCANO)),
            anomaly_noise: Perlin::new(stream_seed(seed, stream::ANOMALY)),
            variability_noise: Perlin::new(stream_seed(seed, stream::VARIABILITY)),
            highland_noise: Perlin::new(stream_seed(seed, stream::HIGHLAND)),
            highlands: (p.look.geology.regime != crate::geology::Regime::Plates)
                .then(|| Perlin::new(stream_seed(seed, stream::HIGHLANDS))),
            platform_highlands: platform_highlands(p),
        };
        if !p.look.rules.dry {
            sampler.crust_line = sampler.crust_line();
            sampler.shore = shore_noise(sampler.waterline, &sampler.crust, sampler.crust_line);
        }
        if p.look.geology.regime == crate::geology::Regime::Plates {
            // The crust of each side of a boundary: continental above the
            // crust line, from the continents' shape alone.
            let continental = |q| sampler.continental(q);
            let map = match frame {
                Some(frame) if p.drift_ma == 0.0 => {
                    crate::plates::PlateMap::from_frame(frame, continental)
                }
                _ => crate::plates::PlateMap::at_epoch(p, f64::from(p.drift_ma), continental),
            };
            sampler.tectonics = Some(map);
            sampler.noise_mountains = NOISE_MOUNTAINS_WITH_PLATES;
        }
        sampler
    }

    /// Share of the noise mountains kept at a unit vector: 1 inside the
    /// highland provinces of a world without plates, 0 on its plains, 1
    /// everywhere on a world with plates.
    fn highland(&self, p: [f64; 3]) -> f32 {
        let Some(noise) = &self.highlands else {
            return 1.0;
        };
        let f = HIGHLANDS.frequency;
        let v = fbm(noise, p[0] * f, p[1] * f, p[2] * f, HIGHLANDS.octaves);
        smoothstep(HIGHLANDS.edge.0, HIGHLANDS.edge.1, v)
    }

    /// Whether the crust at a unit vector is continental: above the crust
    /// line, from the continents' shape alone (not from the sea).
    fn continental(&self, p: [f64; 3]) -> bool {
        let (coarse, fine) = self.continent_shape(p);
        coarse + fine > self.crust_line
    }

    /// The crust line of a wet world: the level of its continents' noise
    /// above which [`CONTINENTAL_SHARE`] of its surface lies, by area, on
    /// the fixed grid [`CRUST_GRID`] (whatever the world's resolution).
    fn crust_line(&self) -> f32 {
        let (width, height) = CRUST_GRID;
        let mut levels: Vec<(f32, f64)> = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            let latitude = PI / 2.0 - (f64::from(y) + 0.5) / f64::from(height) * PI;
            for x in 0..width {
                let longitude = (f64::from(x) + 0.5) / f64::from(width) * 2.0 * PI;
                let (coarse, fine) = self.continent_shape([
                    latitude.cos() * longitude.cos(),
                    latitude.sin(),
                    latitude.cos() * longitude.sin(),
                ]);
                levels.push((coarse + fine, latitude.cos()));
            }
        }
        levels.sort_by(|a, b| b.0.total_cmp(&a.0));
        let total: f64 = levels.iter().map(|l| l.1).sum();
        let mut above = 0.0;
        for (level, area) in &levels {
            above += area;
            if above >= CONTINENTAL_SHARE * total {
                return *level;
            }
        }
        levels.last().map_or(0.0, |l| l.0)
    }

    /// The continents' shape at a unit vector: its coarse octaves and its
    /// fine ones (see [`Self::sample`]).
    fn continent_shape(&self, p: [f64; 3]) -> (f32, f32) {
        let c = 0.8 * self.noise_scale;
        let norm = 1.0 + 0.5 + 0.25 + 0.125 + 0.0625;
        let octave = |k: u32| {
            let f = c * f64::from(1u32 << k);
            self.continent_noise.get([p[0] * f, p[1] * f, p[2] * f]) as f32 / (1u32 << k) as f32
        };
        (
            (octave(0) + octave(1)) / norm,
            (octave(2) + octave(3) + octave(4)) / norm,
        )
    }

    /// Volcanic cone profile at a point of the unit sphere: 1 at a cone's
    /// vent, falling with distance over a few cone radii, with lobed lava
    /// flows. Cones are one per [`CONE_CELL_KM`] cell at most, sized in km,
    /// so their size does not depend on the planet's.
    fn cone(&self, p: [f64; 3]) -> f32 {
        self.cone_on(p, self.cone_seed, CONE_CELL_KM, CONE_PRESENCE)
    }

    /// Like [`Self::cone`], on a lattice of its own: one point per
    /// `cell_km` cell, present with probability `presence`. Where cones
    /// overlap, the highest profile wins: taking the nearest cone's alone
    /// drew a cliff where a large cone met a small one, halfway between.
    fn cone_on(&self, p: [f64; 3], seed: u32, cell_km: f64, presence: f64) -> f32 {
        let radius_km = self.grid.circumference_km as f64 / (2.0 * PI);
        let f = radius_km / cell_km;
        let mut best = 0.0f64;
        for (d, h) in lattice_points(seed, p[0] * f, p[1] * f, p[2] * f, presence) {
            let size = CONE_KM.0 + (CONE_KM.1 - CONE_KM.0) * ((h >> 8) & 0xFF) as f64 / 255.0;
            // The lobes shorten a flow by 40 % at most: past 4 radii so
            // shortened, the profile is under 0.04 %.
            if 0.6 * d * cell_km / size > 4.0 {
                continue;
            }
            // Flows reach further in some directions than others.
            let g = radius_km / (2.0 * size);
            let lobe = self
                .volcano_noise
                .get([p[0] * g + 71.0, p[1] * g - 13.0, p[2] * g + 29.0]);
            let r = d * cell_km / size * (1.0 + 0.4 * lobe);
            best = best.max((-0.5 * r * r).exp());
        }
        best as f32
    }

    /// Samples every noise layer for the pixel at column `x`, row `y`.
    pub fn sample(&self, x: u32, y: u32) -> Sample {
        self.sample_with(x, y, |p| ClimateField::ALL.map(|f| self.climate(f, p)))
    }

    /// A climate field's noise, base frequency (per unit of the sphere's
    /// radius), offset and octaves: the one table [`Self::climate`] and
    /// [`Self::climate_frequency`] both read.
    fn climate_spec(&self, field: ClimateField) -> (&Perlin, f64, [f64; 3], u32) {
        let s = self.noise_scale;
        match field {
            // Features of ~3 000 km: whole zones shift between continents.
            ClimateField::TemperatureAnomaly => (&self.anomaly_noise, 1.2 * s, [0.0; 3], 3),
            ClimateField::Moisture => (&self.moisture_noise, 1.5 * s, [0.0; 3], 4),
            // Features from ~1 000 km down to ~60 km; two independent samples.
            ClimateField::VariabilityT => (&self.variability_noise, 6.0 * s, [0.0; 3], 5),
            ClimateField::VariabilityM => (&self.variability_noise, 6.0 * s, [17.3, -4.1, 9.7], 5),
            ClimateField::HighlandJitter => (&self.highland_noise, 4.0 * s, [0.0; 3], 4),
        }
    }

    /// One climate noise field at a point of the unit sphere (the map
    /// point: the climate does not move with the plates).
    pub fn climate(&self, field: ClimateField, [nx, ny, nz]: [f64; 3]) -> f32 {
        let (noise, f, offset, octaves) = self.climate_spec(field);
        fbm(
            noise,
            nx * f + offset[0],
            ny * f + offset[1],
            nz * f + offset[2],
            octaves,
        )
    }

    /// The highest frequency of a climate field's noise, in cycles per unit
    /// of the sphere's radius: its finest octave ([`fbm`] doubles the
    /// frequency at each one).
    pub fn climate_frequency(&self, field: ClimateField) -> f64 {
        let (_, f, _, octaves) = self.climate_spec(field);
        f * f64::from(1u32 << (octaves - 1))
    }

    /// Like [`Self::sample`], with the climate fields (in [`ClimateField::ALL`]
    /// order) given by `climate` at the pixel's map point.
    pub fn sample_with(
        &self,
        x: u32,
        y: u32,
        climate: impl FnOnce([f64; 3]) -> [f32; ClimateField::COUNT],
    ) -> Sample {
        let s = self.noise_scale;

        // Spherical mapping of the pixel centre: longitude 0..2π, latitude
        // +π/2 (row 0, north) to -π/2 (last row, south). Projects the flat map
        // onto a unit sphere → no seams, no mirror symmetry.
        let [nx, ny, nz] = self.grid.unit_vector_at(f64::from(x), f64::from(y));
        // The plates carry the crust: the relief of a point is that of its
        // crust where it lies today (the point itself at the present),
        // while the climate and the plate boundaries stay at the point.
        let map_point = [nx, ny, nz];
        let origin = self.tectonics.as_ref().map_or(
            crate::plates::Origin {
                point: map_point,
                new_crust: false,
            },
            |t| t.origin(map_point, |q| self.continental(q)),
        );
        let [nx, ny, nz] = origin.point;

        // Domain warping: twist coordinates before sampling for organic coastlines.
        let warp_x = self
            .warp_noise_a
            .get([nx * 2.0 * s, ny * 2.0 * s, nz * 2.0 * s]);
        let warp_y =
            self.warp_noise_b
                .get([nx * 2.0 * s + 5.2, ny * 2.0 * s + 1.3, nz * 2.0 * s + 3.7]);
        let wnx = nx + warp_x * 0.25;
        let wny = ny + warp_y * 0.25;

        // Continent shape: 5 octaves, split into the 2 coarse ones (continents,
        // basins) and the 3 fine ones (hills).
        let c = 0.8 * s;
        // Same normalisation as `fbm` with 5 octaves.
        let norm = 1.0 + 0.5 + 0.25 + 0.125 + 0.0625;
        let shape = |x: f64, y: f64, z: f64| {
            let octave = |k: u32| {
                let f = c * f64::from(1u32 << k);
                self.continent_noise.get([x * f, y * f, z * f]) as f32 / (1u32 << k) as f32
            };
            (
                (octave(0) + octave(1)) / norm,
                (octave(2) + octave(3) + octave(4)) / norm,
            )
        };
        let (coarse, fine) = shape(nx, ny, nz);
        // Continent outline: near the coast the noise is sampled at a
        // position displaced sideways by every octave finer than its own,
        // each by a share of its wavelength (thinning at finer octaves), and
        // a plate step is added, so coasts get capes, bays, islands, straight
        // stretches and corners at every scale down to the pixel. The
        // displacement fades out away from the coast, where it would only
        // wrinkle the relief.
        let coastal = 1.0
            - smoothstep(
                OUTLINE_BAND.0,
                OUTLINE_BAND.1,
                (coarse + fine - self.shore).abs(),
            );
        let (coarse, fine) = if coastal > 0.0 {
            let (mut qx, mut qy, mut qz) = (nx, ny, nz);
            let mut f = 0.8 * s * 16.0;
            let mut share = COAST_WARP * coastal as f64;
            for k in 0..self.coast_octaves {
                let a = share / f;
                let o = 17.0 * k as f64;
                qx += a * self.coast_noise.get([nx * f + o, ny * f, nz * f]);
                qy += a * self.coast_noise.get([nx * f, ny * f + o, nz * f + 3.1]);
                qz += a * self.coast_noise.get([nx * f + 5.7, ny * f, nz * f + o]);
                f *= 2.0;
                share *= COAST_WARP_DECAY;
            }
            let (coarse, fine) = shape(qx, qy, qz);
            let pf = 2.5 * s;
            let plate = PLATE_STEP
                * cell_value(self.plate_seed, qx * pf, qy * pf, qz * pf, PLATE_SHARPNESS);
            (coarse + coastal * plate, fine)
        } else {
            (coarse, fine)
        };
        // The level the relief is measured from: a wet world's crust line;
        // a dry world's waterline, as before its crust floated.
        let (line, onset) = if self.dry_waterline.is_some() {
            (self.waterline, DRY_MOUNTAIN_ONSET)
        } else {
            (self.crust_line, MOUNTAIN_ONSET)
        };
        // Crust formed where the plates parted, since the present (or
        // before it, in the past): ocean floor, smoother than the old one,
        // below that level, where the noise makes no mountains.
        let (coarse, fine) = if origin.new_crust {
            (line - NEW_CRUST_DEPTH, NEW_CRUST_ROUGHNESS * fine)
        } else {
            (coarse, fine)
        };
        let mountain_weight = ((coarse + fine - line - onset) * 2.5).clamp(0.0, 1.0);
        // Low ground inland keeps a share of the fine octaves: plains and
        // deserts are smooth, coasts keep their outline and mountains their
        // foothills.
        // Away from the coast both ways: inland plains, and abyssal plains
        // on the sea floor (seen on planets whose seas have dried up).
        let inland = smoothstep(COAST_BAND.0, COAST_BAND.1, (coarse + fine - line).abs());
        let roughness = (1.0 - (1.0 - PLAIN_ROUGHNESS) * inland).max(mountain_weight);
        // The crust floats by its thickness: continents as a platform, the
        // ocean floor at the old floor and below (see `floating`), the sea
        // filling them from its water. New crust is laid at the old floor
        // already.
        let continent = if let Some(waterline) = self.dry_waterline {
            // No sea ever shaped a dry world: plains on its platform,
            // rolling with the continents' noise.
            waterline
                + DRY_PLATFORM_KM / crate::craters::ELEVATION_KM
                + (coarse + PLAIN_ROUGHNESS * fine) * DRY_PLAINS_KM * self.relief_scale
                    / crate::craters::ELEVATION_KM
        } else if origin.new_crust {
            CRUST_DATUM + fine * roughness
        } else {
            floating(coarse + fine * roughness - self.crust_line, &self.crust) + CRUST_DATUM
        };

        // Ridged mountains blended only onto elevated terrain: a main ridge,
        // with two finer ridged octaves concentrated on it.
        let ridge = |f: f64, o: f64| {
            ridged(
                &self.elevation_noise,
                wnx * f + o,
                wny * f - o,
                nz * f + 0.5 * o,
            )
        };
        let main = ridge(5.0 * s, 0.0);
        let detail = 0.6 * ridge(10.7 * s, 23.0) + 0.4 * ridge(22.1 * s, 41.0);
        let mountain = main * (1.0 - MOUNTAIN_DETAIL + MOUNTAIN_DETAIL * detail);

        // Volcanic zones are broad; volcanoes are cones within them, which
        // rise above the terrain (islands at sea). Away from a cone the zone
        // stays below the ash threshold.
        let volcanic_raw = fbm(&self.volcano_noise, nx * s, ny * s, nz * s, 3);
        // Plate boundaries: their own relief, ranges where they close, and
        // volcanic arcs over subducting plates.
        let plates = self
            .tectonics
            .as_ref()
            .map_or(crate::plates::Relief::default(), |t| t.relief(map_point));
        // Relief scales with gravity as the mountains do.
        // The young floor's swell is thermal isostasy: the same at any
        // gravity.
        let tectonic_uplift = plates.uplift * self.relief_scale + plates.swell;
        let mountain_weight =
            (mountain_weight * self.noise_mountains * self.highland([nx, ny, nz]))
                .max(plates.orogeny);
        let broad = ((volcanic_raw - self.volcanic_threshold) * 4.0)
            .clamp(0.0, 1.0)
            .max(plates.arc * self.arc_volcanism);
        let cone = if broad > 0.0 {
            self.cone([nx, ny, nz])
        } else {
            0.0
        };
        // Volcanic arcs: a chain of volcanoes, much closer together than
        // the cones of volcanic provinces.
        let arc_cone = if plates.arc > ARC_MIN {
            self.cone_on(
                map_point,
                self.arc_cone_seed,
                ARC_CONE_CELL_KM,
                ARC_CONE_PRESENCE,
            )
        } else {
            0.0
        };
        // Arc volcanoes are stratovolcanoes, standing higher than the
        // cones of volcanic provinces: their summits reach the highland,
        // and become volcanoes and lava fields.
        let (cone, cone_height) = volcanoes(cone, arc_cone, plates.arc);
        // Volcanoes load the crust as mountains do: the tallest stand on
        // light worlds (Olympus Mons).
        let cone_height = cone_height * self.relief_scale;
        let volcanic_zone = broad * (VOLCANIC_BACKGROUND + (1.0 - VOLCANIC_BACKGROUND) * cone);

        let elevation = within_range(
            continent
                + tectonic_uplift
                + mountain * mountain_weight * self.mountain_blend
                + broad * cone_height,
        );
        let cone_relief = elevation
            - within_range(
                continent + tectonic_uplift + mountain * mountain_weight * self.mountain_blend,
            );

        // Low latitudes of a world whose liquid gathers at the poles: never
        // under it, whatever the relief (its sea level is -infinity there).
        let elevation = match self.dry_band {
            Some((sin_lat, floor)) if map_point[2].abs() < sin_lat => elevation.max(floor),
            _ => elevation,
        };

        // The thick crust's Airy step, which the highland band ignores on
        // worlds without plates: its height above the thin crust's datum.
        let platform = if self.platform_highlands {
            (continent - CRUST_DATUM).max(0.0)
        } else {
            0.0
        };

        // Positive sea_level raises the waterline (more ocean); negative lowers it.
        let biome_elevation = (elevation - self.sea_level).clamp(-1.0, 1.0);

        // The climate stays at the map point.
        let [
            temperature_anomaly,
            moisture,
            variability_t,
            variability_m,
            highland_jitter,
        ] = climate(map_point);

        Sample {
            warp_x: warp_x as f32,
            warp_y: warp_y as f32,
            continent,
            mountain,
            mountain_weight,
            elevation,
            biome_elevation,
            moisture,
            volcanic_raw,
            volcanic_zone,
            temperature_anomaly,
            variability_t,
            variability_m,
            highland_jitter,
            tectonic_uplift,
            cone_relief,
            platform,
        }
    }
}

/// How a debug layer is coloured: signed values span `[-1, 1]`, unsigned `[0, 1]`.
#[derive(Clone, Copy)]
pub enum Ramp {
    Signed,
    Unsigned,
}

/// An intermediate layer for the diagnostic maps, or a band of its rows.
pub struct DebugLayer {
    pub name: &'static str,
    pub ramp: Ramp,
    /// Row-major values: a whole layer, or the next rows of one.
    pub values: Vec<f32>,
}

/// Receives the diagnostic layers as the generation computes them, so they
/// can be written and freed one by one instead of all being kept until the
/// export (they would cost about twenty map-sized layers of memory).
///
/// Most layers come whole, from the generation thread. The noise layers
/// are sampled band by band: they come as successive [`DebugLayer`]s of the
/// same name holding the next rows, top to bottom, one band of every noise
/// layer at a time from parallel threads. A layer is complete once it has
/// one value per pixel. [`NoiseMapWriter`](crate::export::noise_maps::NoiseMapWriter)
/// writes them as PNGs.
pub trait DebugSink: Sync {
    /// Takes a whole layer or the next band of one.
    fn layer(&self, layer: DebugLayer);
}

/// The output of [`generate`]: the world, and what its export reuses.
pub struct Generated {
    pub world: World,
    /// Anti-aliased river coverage of the map (see [`rivers::coverage`]),
    /// computed for the biomes and handed over to the export.
    pub river_coverage: Vec<f32>,
}

/// A noise layer exported as a diagnostic map: name, colour ramp, extractor.
type NoiseLayer = (&'static str, Ramp, fn(&Sample) -> f32);

const NOISE_LAYERS: &[NoiseLayer] = &[
    ("noise_warp_x", Ramp::Signed, |s| s.warp_x),
    ("noise_warp_y", Ramp::Signed, |s| s.warp_y),
    ("noise_continent", Ramp::Signed, |s| s.continent),
    ("noise_mountain", Ramp::Unsigned, |s| s.mountain),
    ("noise_mountain_wt", Ramp::Unsigned, |s| s.mountain_weight),
    ("noise_elevation", Ramp::Signed, |s| s.elevation),
    ("noise_biome_elev", Ramp::Signed, |s| s.biome_elevation),
    ("noise_moisture", Ramp::Signed, |s| s.moisture),
    ("noise_volcanic_raw", Ramp::Signed, |s| s.volcanic_raw),
    ("noise_volcanic_zone", Ramp::Unsigned, |s| s.volcanic_zone),
    ("noise_temperature_anomaly", Ramp::Signed, |s| {
        s.temperature_anomaly
    }),
    ("noise_variability_t", Ramp::Signed, |s| s.variability_t),
    ("noise_variability_m", Ramp::Signed, |s| s.variability_m),
    ("noise_highland_jitter", Ramp::Signed, |s| s.highland_jitter),
    ("noise_tectonic", Ramp::Signed, |s| s.tectonic_uplift * 4.0),
];

/// Coast distances are computed up to this value; further pixels get the cap.
/// Also the full scale of the `coast_distance` debug map.
pub const COAST_DISTANCE_CAP_KM: f32 = 2_000.0;

/// Full scale of the `flow` debug map: log10 of the drainage area in km².
const FLOW_LOG_MAX: f32 = 7.0;

/// A generation step, reported to progress callbacks as it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Stage {
    Noise,
    Erosion,
    Temperature,
    Coast,
    Moisture,
    Rivers,
    Biomes,
    /// Writing the output files (reported by the caller of [`export_world`](crate::export::export_world)).
    Export,
}

impl Stage {
    /// Every stage, in the order they run.
    pub const ALL: [Stage; 8] = [
        Stage::Noise,
        Stage::Erosion,
        Stage::Temperature,
        Stage::Coast,
        Stage::Moisture,
        Stage::Rivers,
        Stage::Biomes,
        Stage::Export,
    ];
}

/// What [`generate_with_progress`] produces besides the world.
#[derive(Clone, Copy, Default)]
pub struct GenerateOptions<'a> {
    /// Receives every intermediate layer for the diagnostic maps as soon as
    /// it is computed. None (the default) skips them, and the work and
    /// memory they need.
    pub debug: Option<&'a dyn DebugSink>,
}

/// The noise fields later passes need, one value per pixel (much smaller
/// than keeping every [`Sample`]).
struct NoiseFields {
    /// Without the volcanic cones, which come in `cones`.
    elevation: Vec<f32>,
    cones: Vec<f32>,
    moisture: Vec<f32>,
    volcanic_zone: Vec<f32>,
    temperature_anomaly: Vec<f32>,
    variability_t: Vec<f32>,
    variability_m: Vec<f32>,
    highland_offset: Vec<f32>,
}

/// Rows sampled at a time by [`NoiseFields::sample`]: enough to keep every
/// core busy, few enough that the band of samples stays small (about 30 MB
/// at 8192 pixels of width).
const BAND_ROWS: usize = 64;

impl NoiseFields {
    /// Samples every pixel, band by band, writing each band's fields
    /// straight into their layers (and, when `debug` is given, handing it
    /// each band of the noise debug layers): every sample is never held at
    /// once.
    fn sample(sampler: &Sampler, width: u32, height: u32, debug: Option<&dyn DebugSink>) -> Self {
        let n = (width * height) as usize;
        let w = width as usize;
        let mut f = Self {
            elevation: vec![0.0; n],
            cones: vec![0.0; n],
            moisture: vec![0.0; n],
            volcanic_zone: vec![0.0; n],
            temperature_anomaly: vec![0.0; n],
            variability_t: vec![0.0; n],
            variability_m: vec![0.0; n],
            highland_offset: vec![0.0; n],
        };
        let climate = ClimateGrids::new(sampler, width, height);
        let climate = &climate;
        let mut band: Vec<Sample> = Vec::with_capacity(BAND_ROWS * w);
        for y0 in (0..height as usize).step_by(BAND_ROWS) {
            let y1 = (y0 + BAND_ROWS).min(height as usize);
            band.clear();
            band.par_extend((y0..y1).into_par_iter().flat_map_iter(|y| {
                (0..w).map(move |x| climate.sample(sampler, x as u32, y as u32))
            }));
            let range = y0 * w..y1 * w;
            let fill = |layer: &mut [f32], value: fn(&Sample) -> f32| {
                layer[range.clone()]
                    .par_iter_mut()
                    .zip(band.par_iter())
                    .for_each(|(v, s)| *v = value(s));
            };
            fill(&mut f.elevation, |s| s.elevation - s.cone_relief);
            fill(&mut f.cones, |s| s.cone_relief);
            fill(&mut f.moisture, |s| s.moisture);
            fill(&mut f.volcanic_zone, |s| s.volcanic_zone);
            fill(&mut f.temperature_anomaly, |s| s.temperature_anomaly);
            fill(&mut f.variability_t, |s| s.variability_t);
            fill(&mut f.variability_m, |s| s.variability_m);
            fill(&mut f.highland_offset, Sample::highland_offset);
            if let Some(sink) = debug {
                // One task per layer: each one is coloured and encoded as
                // its band arrives.
                NOISE_LAYERS.par_iter().for_each(|&(name, ramp, value)| {
                    sink.layer(DebugLayer {
                        name,
                        ramp,
                        values: band.iter().map(value).collect(),
                    })
                });
            }
        }
        f
    }
}

/// The climate fields of a generation: each one on a coarse grid when its
/// finest octave spans enough pixels (see [`crate::coarse::step_for`]),
/// else sampled at every pixel.
struct ClimateGrids {
    fields: [Option<crate::coarse::CoarseField>; ClimateField::COUNT],
}

impl ClimateGrids {
    fn new(sampler: &Sampler, width: u32, height: u32) -> Self {
        let grid = Grid {
            width,
            height,
            ..sampler.grid
        };
        Self {
            fields: ClimateField::ALL.map(|field| {
                let step = crate::coarse::step_for(sampler.climate_frequency(field), width);
                (step > 1).then(|| {
                    crate::coarse::CoarseField::new(&grid, step, |p| sampler.climate(field, p))
                })
            }),
        }
    }

    /// The sample of pixel `(x, y)`, its climate from the grids.
    fn sample(&self, sampler: &Sampler, x: u32, y: u32) -> Sample {
        sampler.sample_with(x, y, |p| {
            std::array::from_fn(|k| match &self.fields[k] {
                Some(grid) => grid.at(x, y),
                None => sampler.climate(ClimateField::ALL[k], p),
            })
        })
    }
}

/// Rows of sea-level temperature on `grid`, evened out by dense air, then
/// moved by the host star, without the planet's warming. A tidally locked
/// planet's climate follows the angle from the point under its star
/// instead of the latitude.
fn sea_level_rows(params: &WorldParams, grid: &Grid) -> (Vec<f32>, Option<LockedClimate>) {
    let star_climate = star_climate(params);
    let locked = crate::spin::star_info(params).tidally_locked.then(|| {
        let mut locked = LockedClimate::new();
        locked.transport_heat(params.pressure_bar);
        star_temperatures(locked.profile_mut(), star_climate);
        locked
    });
    let rows = match &locked {
        Some(locked) => locked.rows(grid),
        None => {
            let mut rows = row_temperatures(grid, params.axial_tilt_deg);
            transport_heat(&mut rows, grid, params.pressure_bar);
            star_temperatures(&mut rows, star_climate);
            rows
        }
    };
    (rows, locked)
}

/// Elevation relative to the shifted sea level: what biome selection sees.
fn relief_above(elevation: f32, sea_level: f32) -> f32 {
    (elevation - sea_level).clamp(-1.0, 1.0)
}

/// The area-weighted mean of the planet's surface temperature, in degrees
/// Celsius, with its warming (shift and greenhouse) left out, sampled on a
/// coarse grid: the sea-level rows of temperature minus the cooling of the
/// land by altitude (the lapse rate over the relief of a coarse elevation
/// pass at `params.sea_level`), highlands included; a tidally locked
/// planet's cells by their angle from the point under its star, dayside
/// and nightside, as [`generate`] reads them. Regional variability
/// and the climate's anomalies average out and are not included.
pub fn mean_surface_temperature_c(params: &WorldParams) -> f32 {
    let mut coarse = params.clone();
    (coarse.width, coarse.height) = LAND_GRID;
    let grid = coarse.grid();
    let (rows, locked) = sea_level_rows(&coarse, &grid);
    let lapse = crate::climate::lapse_rate(&coarse);
    let flatten = relief_share(&coarse);
    let rules_waterline = coarse.look.rules.waterline;
    let waterline = coarse.sea_level + rules_waterline;
    let frame = (coarse.look.geology.regime == Regime::Plates && coarse.drift_ma == 0.0)
        .then(|| crate::plates::PlateFrame::new(&coarse));
    let sampler = Sampler::with_frame(&coarse, frame.as_ref());
    let (sum, total) = (0..grid.height)
        .into_par_iter()
        .map(|y| {
            let w = grid.latitude(y).cos();
            let row = rows[y as usize];
            let mean: f64 = (0..grid.width)
                .map(|x| {
                    // A locked planet's cell by its angle from the point
                    // under its star, as `generate` reads it: its rows
                    // hold only their warmest (dayside) temperature.
                    let row = match &locked {
                        Some(locked) => locked.at_angle(LockedClimate::angle_at(&grid, x, y)),
                        None => row,
                    };
                    let mut e = sampler.sample(x, y).elevation;
                    if flatten < 1.0 {
                        e = waterline + (e - waterline) * flatten;
                    }
                    let above_water = relief_above(e, coarse.sea_level) - rules_waterline;
                    f64::from(temperature(row, above_water, lapse))
                })
                .sum::<f64>()
                / f64::from(grid.width);
            (w * mean, w)
        })
        .reduce(|| (0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1));
    (sum / total) as f32
}

/// The `climate.temperature` shift that brings the grid's mean surface
/// temperature to `target_k` (kelvin), clamped to [`TEMPERATURE_RANGE`]:
/// the target minus [`mean_surface_temperature_c`], which includes the
/// cooling of the land by altitude.
pub fn warming_for(params: &WorldParams, target_k: f64) -> f32 {
    let target_c = (target_k - 273.15) as f32;
    target_c.clamp(TEMPERATURE_RANGE.0, TEMPERATURE_RANGE.1) - mean_surface_temperature_c(params)
}

/// Size of the grid [`sea_level_for_land_fraction`] samples the relief on.
const LAND_GRID: (u32, u32) = (256, 128);
/// Most passes [`sea_level_for_land_fraction`] makes, and the land share
/// within which of its target it stops.
const LAND_PASSES: usize = 16;
const LAND_TOLERANCE: f64 = 0.003;

/// The sea level that leaves `fraction` of the planet's surface (by area,
/// in `[0, 1]`) above the waterline, for the other parameters of `params`.
///
/// The land share falls as the sea level rises: the sea level is found by
/// regula falsi (the Illinois variant) on the share, sampled on a coarse
/// grid at each estimate (the crust floats by its thickness whatever the
/// sea, but the coasts' outline is drawn around the waterline, so no single
/// sampling gives it exactly). The
/// plates themselves are built once ([`crate::plates::PlateFrame`]); only
/// the crust on their boundaries, and so their relief and slab pull, is
/// found again at each pass. About 0.25 s on an Earth-like world.
/// Erosion runs later and is not modelled, yet on test worlds the land
/// share lands within about 0.02 of the target.
///
/// With [`WorldParams::liquid_poleward_deg`], the ground nearer the
/// equator is land at any sea level (the sampler lifts it), so the share
/// counts it and the sea level found leaves the liquid poleward only.
pub fn sea_level_for_land_fraction(params: &WorldParams, fraction: f32) -> f32 {
    let limits = crate::request::SEA_LEVEL;
    let mut coarse = params.clone();
    (coarse.width, coarse.height) = LAND_GRID;
    let grid = coarse.grid();
    let weights: Vec<f64> = (0..grid.height).map(|y| grid.latitude(y).cos()).collect();
    let total: f64 = weights.iter().sum::<f64>() * grid.width as f64;
    let target = f64::from(fraction.clamp(0.0, 1.0));
    // The plates do not depend on the sea level, only their crust: built
    // once (at the present; drifted plates move with their crust).
    let frame = (coarse.look.geology.regime == Regime::Plates && coarse.drift_ma == 0.0)
        .then(|| crate::plates::PlateFrame::new(&coarse));
    // The land share minus the target, at a sea level.
    let excess = |level: f32| {
        let p = WorldParams {
            sea_level: level,
            ..coarse.clone()
        };
        let waterline = level + p.look.rules.waterline;
        let sampler = Sampler::with_frame(&p, frame.as_ref());
        let land: f64 = (0..grid.height)
            .into_par_iter()
            .map(|y| {
                let w = weights[y as usize];
                (0..grid.width)
                    .filter(|&x| sampler.sample(x, y).elevation > waterline)
                    .count() as f64
                    * w
            })
            .sum();
        land / total - target
    };
    let (mut lo, mut hi) = (limits.min, limits.max);
    let (mut f_lo, mut f_hi) = (excess(lo), excess(hi));
    // Out of reach: the nearest limit.
    if f_lo <= 0.0 {
        return lo;
    }
    if f_hi >= 0.0 {
        return hi;
    }
    let mut level = 0.5 * (lo + hi);
    let mut side = 0;
    for _ in 0..LAND_PASSES {
        level = ((f64::from(lo) * f_hi - f64::from(hi) * f_lo) / (f_hi - f_lo)) as f32;
        let f = excess(level);
        if f.abs() < LAND_TOLERANCE {
            break;
        }
        // More land than asked: raise the sea.
        if f > 0.0 {
            (lo, f_lo) = (level, f);
            if side == 1 {
                f_hi *= 0.5;
            }
            side = 1;
        } else {
            (hi, f_hi) = (level, f);
            if side == -1 {
                f_lo *= 0.5;
            }
            side = -1;
        }
    }
    level
}

/// Mean density of sea water, kg/m³ (Earth's oceans, 1025: Gill 1982,
/// *Atmosphere-Ocean Dynamics*, appendix 3).
const SEA_WATER_KG_M3: f64 = 1025.0;

/// Depth of a global layer of `oceans` Earth oceans ([`OCEAN_KG`]) spread
/// over a body of `radius_earth` Earth radii, m.
///
/// Earth's 1.4e21 kg at 1025 kg/m³ is 1.37e18 m³, over the 5.1e14 m² of a
/// sphere of 6.371e6 m: 2.68 km (the 1.335e9 km³ of Charette & Smith 2010
/// over Earth's surface gives 2.62 km). The layer thins as the surface
/// grows, `1 / radius²`. Pure geometry; [`sea_level_for_water`] turns it
/// into a waterline.
pub fn ocean_depth_m(oceans: f64, radius_earth: f64) -> f64 {
    let radius = radius_earth * crate::exomoon::EARTH_M;
    oceans * OCEAN_KG / SEA_WATER_KG_M3 / (4.0 * PI * radius * radius)
}

/// The sea level at which a global layer of water `depth_m` deep, in
/// metres, fills the relief below it: the mean depth of the water over
/// the sphere (by area) is `depth_m`, found by bisection within
/// [`crate::request::SEA_LEVEL`].
///
/// Elevation is in units of [`crate::craters::ELEVATION_KM`] km; the
/// depth below the waterline is converted to metres with it. The relief
/// is sampled as in [`sea_level_for_land_fraction`] (same coarse grid,
/// same plate frame). The crust does not follow the sea (see [`floating`]):
/// the water fills the ocean basins, then the shelves, and less of it
/// leaves the basins' floors dry. No water leaves the lowest sea level;
/// more than the relief holds, the highest.
///
/// Earth's ocean ([`ocean_depth_m`]) over the terran look leaves 0.24 to
/// 0.35 of the surface dry by seed on the test grid (0.26 for the test's
/// seed), against 0.29 on Earth (ETOPO1, Amante & Eakins 2009): the
/// calibration of the floating crust (see [`floating`]) is what puts the
/// shore near the shelf.
pub fn sea_level_for_water(params: &WorldParams, depth_m: f64) -> f32 {
    let limits = crate::request::SEA_LEVEL;
    let target = depth_m;
    if target <= 0.0 {
        return limits.min;
    }
    let mut coarse = params.clone();
    (coarse.width, coarse.height) = LAND_GRID;
    let grid = coarse.grid();
    let weights: Vec<f64> = (0..grid.height).map(|y| grid.latitude(y).cos()).collect();
    let total: f64 = weights.iter().sum::<f64>() * grid.width as f64;
    let frame = (coarse.look.geology.regime == Regime::Plates && coarse.drift_ma == 0.0)
        .then(|| crate::plates::PlateFrame::new(&coarse));
    let metres = f64::from(crate::craters::ELEVATION_KM) * 1000.0;
    // The mean depth of the water at a sea level, minus the target, m.
    let excess = |level: f32| {
        let p = WorldParams {
            sea_level: level,
            ..coarse.clone()
        };
        let waterline = level + p.look.rules.waterline;
        let sampler = Sampler::with_frame(&p, frame.as_ref());
        let water: f64 = (0..grid.height)
            .into_par_iter()
            .map(|y| {
                let w = weights[y as usize];
                (0..grid.width)
                    .map(|x| f64::from((waterline - sampler.sample(x, y).elevation).max(0.0)))
                    .sum::<f64>()
                    * w
            })
            .sum();
        water / total * metres - target
    };
    let (mut lo, mut hi) = (limits.min, limits.max);
    if excess(hi) <= 0.0 {
        return hi;
    }
    for _ in 0..WATER_PASSES {
        let mid = 0.5 * (lo + hi);
        if excess(mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Sea level of a dry world (see [`crate::biome::BiomeRules::dry`]): the
/// datum of its relief only, set so that its plains, 0.13 below it, stand
/// within the linear part of the elevation range ([`RANGE_KNEE`]), with
/// room for basins and craters below them and for mountains above.
pub const DRY_SEA_LEVEL: f32 = -0.1;
/// Relief of a dry world's plains, km per unit of the continents' noise
/// (its coarse octaves and [`PLAIN_ROUGHNESS`] of its fine ones) at
/// Earth's gravity, scaled as `1/g` ([`relief_scale`]) like everything
/// that loads the crust. With no sea to level them, the plains of a dry
/// world roll: Venus's lowlands and upland plains spread its unimodal
/// hypsometry over about ±1 km, more than 80 % of the surface within 1 km
/// of the mean radius (Ford & Pettengill 1992, doi:10.1029/92JE01085).
/// Calibration on the `venus-like` archetype (seed 12345, 2048 × 1024,
/// with its highlands, see [`HIGHLANDS`]): 3 km puts 79 % of its area
/// within ±1 km of its median, 2 km 85 %.
const DRY_PLAINS_KM: f32 = 3.0;
/// The noise's span over a world's plains, its units: about 1.5 from the
/// lowest to the highest (measured on `venus-like`: its plains span
/// about 4.9 km at 3.3 km per unit), for [`noise_span_km`].
const DRY_PLAINS_SPAN: f32 = 1.5;

/// Bisection passes of [`sea_level_for_water`]: the range of 2 narrows to
/// 2/2^12, under 0.001 (25 m of relief).
const WATER_PASSES: usize = 12;

/// The host star's action on the climate of world `params`: its light,
/// regulated by a carbon cycle on wet worlds (fully with plate tectonics,
/// partly under a stagnant lid), and the planet's own heat.
fn star_climate(params: &WorldParams) -> StarClimate {
    let look = &params.look;
    StarClimate {
        star: params.star,
        orbit_au: params.orbit_au,
        eccentricity: params.eccentricity,
        regulation: carbon_cycle(look.geology.regime, params.pressure_bar),
        // A giant's moon: its tidal heat adds to its own, as radiated power.
        internal_heat_k: crate::exomoon::HostGiant::draw(params)
            .map_or(look.climate.internal_heat, |h| {
                (look.climate.internal_heat.powi(4) + h.tidal_heat_k.powi(4)).powf(0.25)
            }),
    }
}

/// The wind belts of world `params` (see [`Belts`]), from its day: the
/// rotation its moons draw, its year if locked to its star (two thirds of
/// it in the 3:2 resonance, half in 2:1), Earth's 24 h for a world without moons.
pub fn belts(params: &WorldParams) -> Belts {
    Belts::new(
        sidereal_day_hours(params),
        f64::from(params.circumference_km),
    )
}

/// The time world `params` takes to turn once, hours: the day its system
/// set (a moon of a system's planet, [`WorldParams::day_hours`]); a
/// giant's moon's orbit around it, which it is locked to; its year if
/// locked to its star (two thirds of it in the 3:2 resonance, half in
/// 2:1); the rotation its moons draw; Earth's 24 h for a world without
/// moons.
pub(crate) fn sidereal_day_hours(params: &WorldParams) -> f64 {
    if let Some(day) = params.day_hours {
        return f64::from(day);
    }
    if let Some(host) = crate::exomoon::HostGiant::draw(params) {
        return f64::from(host.period_days) * 24.0;
    }
    let star = crate::spin::star_info(params);
    if star.tidally_locked {
        f64::from(star.year_days) * 24.0
    } else if star.spin_turns > 0.0 {
        // Three turns every two orbits in the 3:2 resonance, two in 2:1.
        f64::from(star.year_days) * 24.0 / f64::from(star.spin_turns)
    } else {
        crate::moons::MoonSystem::draw(params).map_or(24.0, |m| f64::from(m.planet_day_hours))
    }
}

/// The seasonal cycle of world `params` (see [`Seasons`]): none on a
/// planet locked to its star, whose sun never moves in its sky.
pub fn seasons(params: &WorldParams) -> Seasons {
    let grid = params.grid();
    let info = crate::spin::star_info(params);
    if info.tidally_locked {
        return Seasons::none(grid.height);
    }
    let star = star_climate(params);
    // Without a longitude of perihelion, the seasons ignore the distance to
    // the star.
    let orbit = params
        .perihelion_longitude_deg
        .filter(|_| params.eccentricity > 0.0)
        .map(|perihelion_longitude_deg| EccentricOrbit {
            eccentricity: params.eccentricity,
            perihelion_longitude_deg,
        });
    Seasons::new(
        &grid,
        params.axial_tilt_deg,
        params.pressure_bar,
        orbit,
        DayCycle::new(sidereal_day_hours(params) / 24.0, f64::from(info.year_days)).map(|day| {
            day.with_air(
                crate::climate::radiative_time_days(
                    params.pressure_bar,
                    params.gravity_modifier() * 9.81,
                    info.equilibrium_k,
                ),
                crate::climate::wave_time_days(
                    params.circumference_km / std::f32::consts::TAU,
                    info.equilibrium_k,
                ),
            )
        }),
        |rows| star_temperatures(rows, star),
    )
}

/// Generates a world, running every pass in order, without debug layers.
pub fn generate(params: &WorldParams) -> Generated {
    generate_with_progress(params, GenerateOptions::default(), |_| {})
}

/// Like [`generate`], with options, calling `progress` as each stage starts.
pub fn generate_with_progress(
    params: &WorldParams,
    options: GenerateOptions,
    progress: impl Fn(Stage),
) -> Generated {
    // A giant has no ground: its cloud bands have a pipeline of their own.
    if params.giant_mass_earth.is_some() {
        return crate::giant_world::generate(params, &progress);
    }
    let (width, height) = (params.width, params.height);
    let look = &params.look;
    let sink = options.debug;
    // Hands a debug layer to the sink, computing it only when there is one.
    let debug = |name: &'static str, ramp: Ramp, values: &dyn Fn() -> Vec<f32>| {
        if let Some(sink) = sink {
            sink.layer(DebugLayer {
                name,
                ramp,
                values: values(),
            });
        }
    };

    // ── Pass 1: noise ─────────────────────────────────────────────────────────
    progress(Stage::Noise);
    // Every pixel is independent: sample rows in parallel. The result stays
    // deterministic because each pixel only depends on its coordinates.
    let sampler = Sampler::new(params);
    let fields = NoiseFields::sample(&sampler, width, height, sink);

    let rules = &look.rules;
    let grid = params.grid();
    let n = params.pixel_count();
    // The tectonic plates, as diagnostic maps: those shaping the relief
    // (at the world's epoch) on worlds with plate tectonics, the present
    // ones on the others.
    if let Some(sink) = sink {
        let own;
        let map = match &sampler.tectonics {
            Some(map) => map,
            None => {
                own = crate::plates::PlateMap::new(params);
                &own
            }
        };
        let count = map.plate.iter().copied().max().unwrap_or(0).max(1);
        let at = |i: usize| {
            let (x, y) = (i as u32 % width, i as u32 / width);
            let lon = (f64::from(x) + 0.5) / f64::from(width) * 2.0 * PI;
            let lat = grid.latitude(y);
            map.index_at([lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()])
        };
        sink.layer(DebugLayer {
            name: "plates",
            ramp: Ramp::Unsigned,
            values: (0..n)
                .into_par_iter()
                .map(|i| f32::from(map.plate[at(i)]) / f32::from(count))
                .collect(),
        });
        sink.layer(DebugLayer {
            name: "plate_boundaries",
            ramp: Ramp::Signed,
            // Convergence (10 cm/yr full scale), fading 400 km from the
            // boundary: red where plates close, blue where they part.
            values: (0..n)
                .into_par_iter()
                .map(|i| {
                    let k = at(i);
                    let fade = (-map.boundary_km[k] / 400.0).exp();
                    (map.convergence[k] / 10.0 * fade).clamp(-1.0, 1.0)
                })
                .collect(),
        });
    }

    // ── Pass 2: hydraulic erosion ─────────────────────────────────────────────
    progress(Stage::Erosion);
    let NoiseFields {
        mut elevation,
        cones,
        moisture: moisture_noise,
        volcanic_zone: raw_volcanic_zone,
        temperature_anomaly: anomaly,
        variability_t,
        variability_m,
        highland_offset,
    } = fields;
    // Small and airless bodies cannot hold the relief of a planet: it is
    // flattened towards the waterline (see `relief_share`), and craters
    // shape them.
    let flatten = relief_share(params);
    if flatten < 1.0 {
        // Towards the waterline, so that the coasts stay where the sea
        // left them: the sea level itself sits above the altitude lines.
        let waterline = params.sea_level + look.rules.waterline;
        elevation
            .par_iter_mut()
            .for_each(|e| *e = waterline + (*e - waterline) * flatten);
    }
    // Rows of sea-level temperature, evened out by dense air, then moved by
    // the host star.
    let (row_temperature, locked) = sea_level_rows(params, &grid);
    let seasons = seasons(params);
    // The planet's shift and its atmosphere's greenhouse warming.
    let warming = look.climate.temperature + look.atmosphere.greenhouse;
    // Water carves where it thaws in summer.
    let thaw = crate::erosion::thaw(&seasons.land_summer(&row_temperature), warming, rules.polar);
    // Impact craters: those of the wet era are worn by the erosion, the
    // younger ones are drawn over it. Their height change is only kept for
    // its debug map.
    let craters = CraterField::new(params, base_min_diameter_km(&grid));
    let mut crater_relief = sink.map(|_| elevation.clone());
    craters.apply(&mut elevation, &grid, Era::Wet);
    if let Some(relief) = crater_relief.as_mut() {
        relief
            .par_iter_mut()
            .zip(&elevation)
            .for_each(|(r, e)| *r = e - *r);
    }
    // The height change is only kept for its debug map.
    let mut erosion = sink.map(|_| vec![0.0; n]);
    erode(
        &mut elevation,
        (width, height),
        params.open_waterline(),
        params.erosion,
        params.seed,
        &thaw,
        erosion.as_deref_mut(),
    );
    if let (Some(sink), Some(mut values)) = (sink, erosion) {
        // ×10: height changes are small next to the [-1, 1] elevation range.
        values
            .par_iter_mut()
            .for_each(|d| *d = (*d * 10.0).clamp(-1.0, 1.0));
        sink.layer(DebugLayer {
            name: "erosion",
            ramp: Ramp::Signed,
            values,
        });
    }
    // Volcanic cones are young: they stand on the worn relief instead of
    // being worn down with it (eroded, they no longer reached the highland
    // where volcano and lava field biomes are drawn).
    elevation
        .par_iter_mut()
        .zip(&cones)
        .for_each(|(e, c)| *e = (*e + c).clamp(-1.0, 1.0));
    drop(cones);
    // The young craters' height change, kept to export the elevation
    // without them (the surface the time slider draws them on).
    let mut young_craters = elevation.clone();
    let mare = craters.apply(&mut elevation, &grid, Era::Dry);
    drop(craters);
    // The bright ejecta and rays of fresh craters, down to the pixel, over
    // the dark lava of the maria.
    let pixel_km = f64::from(grid.circumference_km) / f64::from(width);
    let mut albedo = CraterField::new(params, RAY_MIN_PIXELS * pixel_km).albedo(&grid);
    darken_maria(&mut albedo, &mare);
    young_craters
        .par_iter_mut()
        .zip(&elevation)
        .for_each(|(d, e)| *d = e - *d);
    if let (Some(sink), Some(mut values)) = (sink, crater_relief) {
        // The young craters' change added to the old ones' (the erosion in
        // between left out); ×5: a crater moves the elevation by a few
        // tenths at most.
        values
            .par_iter_mut()
            .zip(&young_craters)
            .for_each(|(r, d)| *r = ((*r + d) * 5.0).clamp(-1.0, 1.0));
        sink.layer(DebugLayer {
            name: "craters",
            ramp: Ramp::Signed,
            values,
        });
    }
    keep_band_dry(params, &grid, &mut elevation, None);
    // Elevation relative to the shifted sea level: what biome selection sees.
    // Computed where needed rather than kept as a layer.
    let relief = |e: f32| relief_above(e, params.sea_level);

    // ── Pass 3: temperature ───────────────────────────────────────────────────
    progress(Stage::Temperature);
    let lapse = crate::climate::lapse_rate(params);
    let mut raw_temperature: Vec<f32> = elevation
        .par_iter()
        .enumerate()
        .map(|(i, &e)| {
            let above_water = relief(e) - rules.waterline;
            let sea_level = match &locked {
                Some(locked) => locked.at_angle(LockedClimate::angle_at(
                    &grid,
                    (i % width as usize) as u32,
                    (i / width as usize) as u32,
                )),
                None => row_temperature[i / width as usize],
            };
            temperature(sea_level, above_water, lapse)
                + temperature_anomaly(anomaly[i], above_water >= 0.0)
        })
        .collect();
    drop(anomaly);
    // The sea off the west coasts of the subtropics, cooled by the
    // eastern-boundary currents (the moisture pass reads it). A locked
    // planet's winds do not blow along its coasts, and a world with no
    // liquid sea has no currents.
    let belts = belts(params);
    if locked.is_none() && params.pressure_bar >= LIQUID_WATER_BAR {
        let sea: Vec<bool> = elevation
            .par_iter()
            .map(|&e| relief(e) < rules.open_water())
            .collect();
        let cooling = crate::climate::cold_currents(&grid, &belts, &sea);
        raw_temperature
            .par_iter_mut()
            .zip(&cooling)
            .for_each(|(t, c)| *t += c);
    }
    debug("temperature", Ramp::Unsigned, &|| {
        // −30 °C to 40 °C over the ramp.
        raw_temperature
            .par_iter()
            .map(|t| ((t + 30.0) / 70.0).clamp(0.0, 1.0))
            .collect()
    });

    // ── Pass 4: distance to the coast ─────────────────────────────────────────
    progress(Stage::Coast);
    // Shores are the land pixels close to open water.
    let open_water: Vec<bool> = elevation
        .par_iter()
        .map(|&e| relief(e) < rules.open_water())
        .collect();
    let height_above_water: Vec<f32> = elevation
        .par_iter()
        .map(|&e| relief(e) - rules.open_water())
        .collect();
    // The distance to the coast runs on one core (a Dijkstra), and only the
    // biomes read it: the moisture pass runs beside it.
    progress(Stage::Moisture);
    // ── Pass 5: moisture ──────────────────────────────────────────────────────
    // Winds carry ocean evaporation inland; it rains out with distance and on
    // windward slopes, leaving rain shadows behind mountains, where the air
    // comes down warmer (foehn).
    let (
        coast_distance_km,
        Moisture {
            moisture: raw_moisture,
            precipitation,
            foehn,
            ..
        },
    ) = rayon::join(
        || grid.distance_to(&open_water, COAST_DISTANCE_CAP_KM),
        || {
            moisture(&MoistureInput {
                grid,
                height_above_water: &height_above_water,
                temperature: &raw_temperature,
                noise: &moisture_noise,
                belts,
            })
        },
    );
    drop(open_water);
    debug("coast_distance", Ramp::Unsigned, &|| {
        coast_distance_km
            .par_iter()
            .map(|d| d / COAST_DISTANCE_CAP_KM)
            .collect()
    });
    drop(moisture_noise);
    debug("moisture", Ramp::Signed, &|| raw_moisture.clone());

    // Planet-type global offsets shift the whole climate before rivers and
    // biomes. Store the climate exactly as biome selection sees it, so the
    // exported data explains the biome of every pixel.
    let shift = look.climate;
    let (dt, dm, dvz) = (warming, shift.moisture, shift.volcanic);
    // Too thin an atmosphere keeps no liquid water: no rain anywhere, so no
    // rivers or lakes.
    let liquid_water = params.pressure_bar >= LIQUID_WATER_BAR;
    // Regional variability is part of the stored climate, so the exported
    // layers still explain every biome.
    // Shifted in place: the raw fields are not needed any more.
    let mut moisture = raw_moisture;
    moisture
        .par_iter_mut()
        .zip(&variability_m)
        .for_each(|(m, v)| {
            *m = if liquid_water {
                (*m + dm + regional_variability(*v, Field::Moisture)).clamp(-1.0, 1.0)
            } else {
                -1.0
            }
        });
    // The foehn warms the lee of ranges where rain fell on the climb.
    let foehn_share = if liquid_water { 1.0 } else { 0.0 };
    let mut temperature = raw_temperature;
    temperature
        .par_iter_mut()
        .zip(&variability_t)
        .zip(&foehn)
        .for_each(|((t, v), f)| {
            *t = (*t + dt + regional_variability(*v, Field::Temperature) + foehn_share * f)
                .clamp(TEMPERATURE_RANGE.0, TEMPERATURE_RANGE.1)
        });
    drop((variability_t, variability_m, foehn));

    // ── Pass 6: rivers and lakes ──────────────────────────────────────────────
    progress(Stage::Rivers);
    // Clouds form where air rises and rains (see the clouds module). They
    // are computed beside the hydrology, whose priority-flood runs on one
    // core: the clouds' noise fills the others.
    let cloud_input = CloudInput {
        grid,
        kind: if params.pressure_bar < AIRLESS_BAR {
            CloudKind::None
        } else {
            look.atmosphere.clouds
        },
        cover_shift: look.atmosphere.cloud_cover,
        precipitation: &precipitation,
        belts,
        seed: params.seed,
    };
    let (clouds, mut water) = rayon::join(
        || crate::clouds::clouds(&cloud_input),
        || {
            // Lava does not depend on rain: the flows of a volcanic world
            // run off everywhere alike.
            let lava = look.remap(Biome::River).is_lava();
            let runoff: Vec<f32> = moisture
                .par_iter()
                .zip(&temperature)
                .map(|(&m, &t)| {
                    if lava {
                        1.0
                    } else if rules.dry {
                        // No liquid at the surface: nothing runs off.
                        0.0
                    } else {
                        crate::hydrology::runoff_share(m, t)
                    }
                })
                .collect();
            let mut water = hydrology(&grid, &height_above_water, &runoff, params.seed);
            if rules.dry {
                // Nor does any pool: lava runs through its pits.
                water.lake.iter_mut().for_each(|l| *l = false);
            }
            water
        },
    );
    if let Some(sink) = sink {
        sink.layer(DebugLayer {
            name: "precipitation",
            ramp: Ramp::Signed,
            values: precipitation,
        });
    } else {
        drop(precipitation);
    }
    if !clouds.is_empty() {
        debug("clouds", Ramp::Unsigned, &|| clouds.clone());
    }
    // Planets without anything liquid to flow (rivers remapped to dry
    // ground) have no rivers to draw, carve or end in deltas: only the
    // drainage flow is kept. Lava rivers of volcanic worlds still flow.
    let flowing = look.remap(Biome::River);
    let traced = (flowing.is_water() || flowing.is_lava()).then(|| rivers::trace(&grid, &water));
    // The receivers and river pixels only serve the tracing.
    water.release_routing();
    let mut river_lines = match traced {
        Some(traced) => rivers::shape(&traced, &grid, &water.filled, params.seed),
        None => Vec::new(),
    };
    // Deltas and estuaries at the mouths of the large rivers.
    let deltas = delta::outlets(&mut river_lines, &grid, &height_above_water, params.seed);
    drop(height_above_water);
    // Valleys and floodplains along the rivers. Carving never creates water,
    // so the coast distance and the drainage stay valid.
    let waterline = params.open_waterline();
    let mut above: Vec<f32> = elevation.par_iter().map(|e| e - waterline).collect();
    let mut floodplain = carve::carve(&river_lines, &grid, &water.filled, &water.lake, &mut above);
    // Only the flow and the lakes are needed from here on.
    let (flow_km2, mut lake) = water.into_flow_and_lakes();
    let delta_plain = delta::plains(&deltas, &river_lines, &grid, &mut above, &mut moisture);
    floodplain
        .par_iter_mut()
        .zip(delta_plain)
        .for_each(|(f, d)| *f = f.max(d));
    elevation
        .par_iter_mut()
        .zip(above)
        .for_each(|(e, a)| *e = a + waterline);
    keep_band_dry(params, &grid, &mut elevation, Some(&mut lake));
    // Rivers water their floodplain: green strips across dry land.
    moisture
        .par_iter_mut()
        .zip(&floodplain)
        .for_each(|(m, f)| *m = (*m + RIPARIAN_MOISTURE * f).clamp(-1.0, 1.0));
    debug("elevation", Ramp::Signed, &|| elevation.clone());
    debug("floodplain", Ramp::Unsigned, &|| floodplain.clone());
    // The biome layer marks rivers where they are drawn, so it matches the map.
    let river_coverage =
        rivers::coverage(&river_lines, width, height, params.circumference_km as f64);
    debug("flow", Ramp::Unsigned, &|| {
        flow_km2
            .par_iter()
            .map(|f| (f + 1.0).log10() / FLOW_LOG_MAX)
            .collect()
    });

    // ── Pass 7: biomes ────────────────────────────────────────────────────────
    progress(Stage::Biomes);
    let mut volcanic_zone = raw_volcanic_zone;
    volcanic_zone
        .par_iter_mut()
        .for_each(|v| *v = (*v + dvz).clamp(0.0, 1.0));
    let biomes: Vec<Biome> = (0..n)
        .into_par_iter()
        .map(|i| {
            let land = relief(elevation[i]) >= rules.open_water();
            let input = BiomeInput {
                elevation: relief(elevation[i]),
                moisture: moisture[i],
                temperature: temperature[i],
                seasonality: seasons.at((i / width as usize) as u32, land, coast_distance_km[i]),
                volcanic_zone: volcanic_zone[i],
                coast_distance_km: coast_distance_km[i],
                lake: lake[i],
                river: !lake[i] && river_coverage[i] >= rivers::RIVER_COVERAGE,
                highland_offset: highland_offset[i],
            };
            look.remap(rules.choose(input))
        })
        .collect();

    // Later passes (valleys, deltas) change the elevation additively: they
    // stay in the base.
    young_craters
        .par_iter_mut()
        .zip(&elevation)
        .for_each(|(d, e)| *d = e - *d);
    let elevation_base = Some(young_craters);
    Generated {
        world: World {
            params: params.clone(),
            width,
            height,
            elevation,
            elevation_base,
            moisture,
            temperature,
            seasons,
            lapse_rate: lapse,
            flow_km2,
            volcanic_zone,
            coast_distance_km,
            floodplain,
            highland_offset,
            biomes,
            rivers: river_lines,
            albedo,
            clouds,
            surface: Vec::new(),
        },
        river_coverage,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::climate::EARTH_AXIAL_TILT_DEG;
    use crate::look::Look;

    #[test]
    fn volcanoes_rise_without_a_step() {
        // A full arc volcano, where the arc fades out: nothing at the
        // threshold, then a gradual rise (no cliff of 0.4 at ARC_MIN).
        assert_eq!(volcanoes(0.0, 1.0, ARC_MIN).1, 0.0);
        let mut last = 0.0;
        for k in 1..=100 {
            let arc = ARC_MIN + (ARC_FULL - ARC_MIN) * k as f32 / 100.0;
            let height = volcanoes(0.0, 1.0, arc).1;
            assert!(
                height >= last && height - last < 0.01,
                "{arc}: {last} -> {height}"
            );
            last = height;
        }
        assert_eq!(volcanoes(0.0, 1.0, 1.0).1, ARC_CONE_HEIGHT);
        // Where arc volcanoes take over from a province's cones, the
        // height is continuous too (it jumped from 0.25 to 0.4 times the
        // cone).
        let below = volcanoes(0.6, 0.599, 1.0).1;
        let above = volcanoes(0.6, 0.601, 1.0).1;
        assert!((above - below).abs() < 0.01, "{below} -> {above}");
    }

    fn earth_params() -> WorldParams {
        let mut p = params(7, "terran");
        (p.width, p.height) = LAND_GRID;
        p
    }

    /// Share of the surface above the waterline at `level`, by area, on
    /// the grid of [`sea_level_for_land_fraction`].
    fn land_share(params: &WorldParams, level: f32) -> f64 {
        let p = WorldParams {
            sea_level: level,
            ..params.clone()
        };
        let grid = p.grid();
        let waterline = level + p.look.rules.waterline;
        let sampler = Sampler::new(&p);
        let (mut land, mut total) = (0.0, 0.0);
        for y in 0..grid.height {
            let w = grid.latitude(y).cos();
            for x in 0..grid.width {
                total += w;
                if sampler.sample(x, y).elevation > waterline {
                    land += w;
                }
            }
        }
        land / total
    }

    #[test]
    fn earths_ocean_depth_is_two_and_a_half_kilometres_or_so() {
        let d = ocean_depth_m(1.0, 1.0);
        assert!((d - 2680.0).abs() < 30.0, "{d}");
        assert!((ocean_depth_m(2.0, 2.0) - d / 2.0).abs() < 1e-6);
    }

    #[test]
    fn earths_ocean_leaves_earths_land() {
        let params = earth_params();
        let level = sea_level_for_water(&params, ocean_depth_m(1.0, 1.0));
        let land = land_share(&params, level);
        assert!((land - 0.29).abs() < 0.08, "{land}");
    }

    #[test]
    fn no_water_leaves_no_sea() {
        let params = earth_params();
        assert_eq!(
            sea_level_for_water(&params, 0.0),
            crate::request::SEA_LEVEL.min
        );
    }

    #[test]
    fn a_deep_ocean_drowns_every_summit() {
        let params = earth_params();
        let level = sea_level_for_water(&params, ocean_depth_m(10.0, 1.0));
        // The highest sea level of the range ([`crate::request::SEA_LEVEL`])
        // holds about 26 km of water on average, short of 10 oceans on a
        // few summits: the sea stops at the range, with under 1 % dry.
        assert_eq!(level, crate::request::SEA_LEVEL.max);
        let land = land_share(&params, level);
        assert!(land < 0.01, "{land}");
    }

    #[test]
    fn the_world_lands_on_the_models_temperature() {
        // A terran preset world (seed 7, 128x64) at 1 AU round a G2V, with
        // greenhouse 0 and climate.temperature = warming_for(&params, t).
        for target_k in [250.0, 288.0, 330.0] {
            let mut params = params(7, "terran");
            (params.width, params.height) = (128, 64);
            params.look.atmosphere.greenhouse = 0.0;
            params.look.climate.temperature = warming_for(&params, target_k);
            let mean = mean_k(&params);
            eprintln!("target {target_k} K: mean {mean:.2} K");
            assert!((mean - target_k).abs() < 5.0, "{mean} vs {target_k}");
        }
        // The same world locked to an M4V in its habitable zone: its
        // dayside is far warmer than its nightside, and the mean is taken
        // over both (seed 7, 128x64).
        let m4 = crate::star::parse_type("M4V").unwrap();
        let orbit = crate::star::distance_au(crate::star::values(m4).luminosity, 1.0);
        for target_k in [180.0, 250.0, 288.0] {
            let mut params = params(7, "terran");
            (params.width, params.height) = (128, 64);
            (params.star, params.orbit_au) = (m4, orbit);
            params.locked_to_star = Some(true);
            params.look.atmosphere.greenhouse = 0.0;
            params.look.climate.temperature = warming_for(&params, target_k);
            let mean = mean_k(&params);
            eprintln!("locked, target {target_k} K: mean {mean:.2} K");
            assert!(
                (mean - target_k).abs() < 5.0,
                "locked: {mean} vs {target_k}"
            );
        }
        // Earth's, Mars's and Venus's models through
        // `planet::interim::params_for` and `calibrate` at their orbits round the Sun
        // (seed 1, 128x64): Venus at the grid's ceiling, above it.
        use crate::planet::reference::{model_inputs, reference, sun_at};
        use rand::SeedableRng;
        for name in ["Earth", "Mars", "Venus"] {
            let body = reference(name);
            let model = crate::planet::planet_model(&sun_at(body.orbit_au), model_inputs(body));
            let placed = crate::request::Placed {
                seed: 1,
                star: crate::star::SUN,
                age_gyr: 4.6,
                orbit_au: body.orbit_au as f32,
                eccentricity: 0.0,
                metallicity: 0.0,
                axial_tilt_deg: None,
                perihelion_longitude_deg: None,
                drift_ma: 0.0,
                width: 128,
                height: 64,
                moons: crate::moons::MoonsMode::Auto,
                giants: crate::giants::GiantsMode::None,
                host: crate::exomoon::HostMode::None,
            };
            let mut params = crate::planet::interim::params_for(
                &model,
                &placed,
                &mut rand::rngs::StdRng::seed_from_u64(1),
            );
            crate::planet::interim::calibrate(&mut params, &model);
            let model_k = model.surface.unwrap().climate.temperature_k;
            let target_k = model_k.min(f64::from(TEMPERATURE_RANGE.1) + 273.15);
            let mean = mean_k(&params);
            eprintln!("{name}: model {model_k:.2} K, target {target_k:.2} K, mean {mean:.2} K");
            assert!(
                (mean - target_k).abs() < 5.0,
                "{name}: {mean} vs {target_k}"
            );
        }
    }

    /// Area-weighted mean temperature of the world `params` generates, K.
    fn mean_k(params: &WorldParams) -> f64 {
        let world = generate(params).world;
        let grid = params.grid();
        let (mut sum, mut total) = (0.0_f64, 0.0_f64);
        for (i, &t) in world.temperature.iter().enumerate() {
            let w = grid.latitude((i / world.width as usize) as u32).cos();
            sum += w * f64::from(t);
            total += w;
        }
        sum / total + 273.15
    }

    #[test]
    fn a_lava_world_aims_at_the_grid_ceiling() {
        let params = earth_params();
        let w = warming_for(&params, 1800.0);
        assert_eq!(w + mean_surface_temperature_c(&params), TEMPERATURE_RANGE.1);
    }

    pub fn params(seed: u32, planet: &str) -> WorldParams {
        WorldParams {
            age_gyr: crate::star::DEFAULT_AGE_GYR,
            width: 64,
            height: 32,
            seed,
            class: planet.to_string(),
            archetype: None,
            model: None,
            look: Look::load(planet),
            sea_level: 0.1,
            volcanic_intensity: 0.5,
            circumference_km: EARTH_CIRCUMFERENCE_KM,
            axial_tilt_deg: EARTH_AXIAL_TILT_DEG,
            erosion: 1.0,
            pressure_bar: 1.0,
            surface_age_ga: 0.4,
            star: crate::star::SUN,
            orbit_au: 1.0,
            eccentricity: 0.0,
            drift_ma: 0.0,
            moons: crate::moons::MoonsMode::Auto,
            shape: None,
            metallicity: 0.0,
            giants: crate::giants::GiantsMode::None,
            giant_mass_earth: None,
            host: crate::exomoon::HostMode::None,
            given_rings: None,
            impact_factor: None,
            locked_to_star: None,
            ridge: None,
            perihelion_longitude_deg: None,
            resonance_to_star: None,
            moon: false,
            day_hours: None,
            liquid_poleward_deg: None,
            gravity: None,
        }
    }

    /// The parameters of a world as tests pin them, at `width` × `height`:
    ///
    /// - a built-in archetype (`earth-like`, `mars-like`…): its world as a
    ///   request resolves it (its star, orbit, size and model), its sea
    ///   level and temperature calibrated on its model, without moons;
    /// - a built-in look (`terran`, `caustic`…): [`params`], without the
    ///   planet model, with the look's own regime decided from its size
    ///   and air.
    pub fn preset_params(planet: &str, seed: u32, width: u32, height: u32) -> WorldParams {
        if crate::archetype::built_in_names().contains(&planet) {
            let request = crate::request::WorldRequest {
                planet: Some(planet.to_string()),
                seed: Some(seed),
                width: Some(width),
                height: Some(height),
                moons: Some(crate::moons::MoonsMode::None),
                ..Default::default()
            };
            let resolved = request
                .resolve(
                    &mut <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(u64::from(seed)),
                )
                .unwrap();
            let mut p = resolved.params.clone();
            crate::planet::interim::calibrate(&mut p, resolved.model());
            return p;
        }
        let mut p = params(seed, planet);
        (p.width, p.height) = (width, height);
        p.look.geology.regime = crate::geology::regime_of(&p);
        p
    }

    /// An archetype's world resolved at a small size.
    fn example_world(planet: &str, star: Option<&str>, orbit_au: Option<f32>) -> WorldParams {
        let mut p = preset_params(planet, 1, 192, 96);
        if let Some(star) = star {
            p.star = crate::star::parse_type(star).unwrap();
        }
        if let Some(au) = orbit_au {
            p.orbit_au = au;
            p.eccentricity = 0.1;
            // Its spin holds at its own orbit only.
            p.locked_to_star = None;
            p.resonance_to_star = None;
        }
        p.moons = crate::moons::MoonsMode::None;
        // Aimed again at the temperature of its model, at the star and
        // orbit it was moved to.
        if let Some(model) = p.model.clone().filter(|m| m.surface.is_some()) {
            crate::planet::interim::calibrate(&mut p, &model);
        }
        p
    }

    #[test]
    fn air_tames_the_day_of_a_slow_rotator_and_an_airless_world_keeps_it() {
        // An earth-like world in the 3:2 resonance (an M2V dwarf's habitable
        // zone, e = 0.1), 1 bar of air: moderate swings and tropical biomes
        // (the swing was 93 C at the equator without the air, 1.5 C
        // before the day cycle).
        // Capture into 3:2 is drawn by eccentricity: pin it, as a stored
        // world would.
        let mut params = example_world("earth-like", Some("M2V"), Some(0.2));
        assert!(crate::spin::despun(&crate::spin::star_info(&params)));
        params.locked_to_star = Some(false);
        params.resonance_to_star = Some(crate::spin::RESONANCE_TURNS_PER_ORBIT);
        assert!(crate::spin::star_info(&params).spin_orbit_3_2);
        assert!(params.pressure_bar > 0.5);
        let world = generate(&params).world;
        let swing = world.seasons.half_range(96 / 2, true);
        assert!((3.0..15.0).contains(&swing), "{swing}");
        let land = world.biomes.iter().filter(|b| !b.is_water()).count().max(1) as f32;
        let tropical = world
            .biomes
            .iter()
            .filter(|b| matches!(b, Biome::Jungle | Biome::Savanna))
            .count() as f32;
        assert!(tropical / land > 0.01, "{}", tropical / land);
        // Mercury, with no air to carry the heat, keeps the whole swing.
        let mercury = example_world("mercury-like", None, None);
        assert!(crate::spin::star_info(&mercury).spin_orbit_3_2);
        assert!(seasons(&mercury).half_range(96 / 2, true) > 100.0);
    }

    #[test]
    fn a_world_without_a_perihelion_keeps_its_circular_seasons() {
        // Without a longitude of perihelion, an eccentric orbit whose
        // seasons ignore the distance to the star.
        let mut old = params(5, "terran");
        old.eccentricity = 0.2;
        old.perihelion_longitude_deg = None;
        let grid = old.grid();
        let star = star_climate(&old);
        let circular = Seasons::new(
            &grid,
            old.axial_tilt_deg,
            old.pressure_bar,
            None,
            DayCycle::new(
                sidereal_day_hours(&old) / 24.0,
                f64::from(crate::spin::star_info(&old).year_days),
            )
            .map(|d| {
                d.with_air(
                    crate::climate::radiative_time_days(
                        old.pressure_bar,
                        old.gravity_modifier() * 9.81,
                        crate::spin::star_info(&old).equilibrium_k,
                    ),
                    crate::climate::wave_time_days(
                        old.circumference_km / std::f32::consts::TAU,
                        crate::spin::star_info(&old).equilibrium_k,
                    ),
                )
            }),
            |rows| star_temperatures(rows, star),
        );
        assert_eq!(seasons(&old), circular);
        // With a longitude the seasons change.
        let mut dated = old.clone();
        dated.perihelion_longitude_deg = Some(90.0);
        assert_ne!(seasons(&dated), circular);
    }

    fn small_world(seed: u32, planet: &str) -> World {
        generate(&params(seed, planet)).world
    }

    #[test]
    fn same_seed_produces_same_world() {
        let a = small_world(42, "terran");
        let b = small_world(42, "terran");
        assert_eq!(a.biomes, b.biomes);
        assert_eq!(a.elevation, b.elevation);
    }

    #[test]
    fn seed_near_u32_max_does_not_overflow() {
        small_world(u32::MAX, "terran");
    }

    #[test]
    fn every_layer_has_one_value_per_pixel() {
        let w = small_world(7, "frozen");
        for len in [
            w.elevation.len(),
            w.moisture.len(),
            w.temperature.len(),
            w.biomes.len(),
        ] {
            assert_eq!(len, 64 * 32);
        }
    }

    #[test]
    fn stored_climate_includes_planet_offsets() {
        // Without erosion, which only runs where it thaws, so the relief
        // (and its lapse rate) is the same on both.
        let world = |planet: &str| {
            let mut p = params(5, planet);
            // Without plate tectonics on either, which would add relief to
            // the terran world only.
            p.look.geology.regime = crate::geology::Regime::Episodic;
            generate(&WorldParams { erosion: 0.0, ..p }).world
        };
        // Both around the Sun at 1 AU, where the star changes nothing.
        let (caustic, terran) = (world("caustic"), world("terran"));
        // The caustic look warms every pixel by its shift and greenhouse.
        let look = &caustic.params.look;
        let shift = look.climate.temperature + look.atmosphere.greenhouse;
        assert_eq!(look.climate.temperature, 4.5);
        for (c, t) in caustic.temperature.iter().zip(&terran.temperature) {
            assert!((c - (t + shift)).abs() < 1e-3, "{c} vs {t}");
        }
        assert!(caustic.moisture.iter().all(|m| (-1.0..=1.0).contains(m)));
    }

    #[test]
    fn rocky_gravity_follows_the_mass_radius_relation() {
        let g = |circumference_km| {
            WorldParams {
                circumference_km,
                ..params(0, "terran")
            }
            .gravity_modifier()
        };
        assert!((g(EARTH_CIRCUMFERENCE_KM) - 1.0).abs() < 1e-6);
        // Equatorial circumferences (NASA): Mars 0.38 g (Earth's density
        // gave 0.53), Venus 0.90, the Moon 0.165.
        let mars = g(21_344.0);
        assert!((0.32..0.40).contains(&mars), "{mars}");
        let venus = g(38_025.0);
        assert!((0.88..0.95).contains(&venus), "{venus}");
        let moon = g(10_921.0);
        assert!((moon - 0.165).abs() < 0.005, "{moon}");
        // Heavier with size, faster than the radius, up to 5 g.
        assert!(g(2.0 * EARTH_CIRCUMFERENCE_KM) > 3.0);
        assert_eq!(g(1_000_000.0), 5.0);
    }

    /// Heights (km) with the area of their pixel.
    type Heights = Vec<(f32, f32)>;

    /// Land heights and sea depths of a world above (below) its
    /// waterline, km, each with its pixel's area.
    fn hypsometry(w: &World) -> (Heights, Heights) {
        let p = &w.params;
        let waterline = p.sea_level + p.look.rules.waterline;
        let grid = p.grid();
        let (mut land, mut sea) = (Vec::new(), Vec::new());
        for (i, &e) in w.elevation.iter().enumerate() {
            let a = grid.latitude((i / w.width as usize) as u32).cos() as f32;
            let h = (e - waterline) * crate::craters::ELEVATION_KM;
            if h >= 0.0 {
                land.push((h, a));
            } else {
                sea.push((-h, a));
            }
        }
        (land, sea)
    }

    /// Share of the area of `v` above `km`, %.
    fn above(v: &[(f32, f32)], km: f32) -> f32 {
        let total: f32 = v.iter().map(|x| x.1).sum();
        100.0 * v.iter().filter(|x| x.0 > km).map(|x| x.1).sum::<f32>() / total
    }

    #[test]
    fn earth_like_worlds_have_earths_hypsometry() {
        // ETOPO 2022 (ice surface): land median 0.42 km, 56 % under 500 m,
        // 10.8 % above 2 km, 1.8 % above 4 km; 1 % of the sea floor deeper
        // than 6 km. Each seed within a band around them.
        let earth = "earth-like";
        for seed in [1u32, 2, 3] {
            let p = preset_params(earth, seed, 256, 128);
            let (land, sea) = hypsometry(&generate(&p).world);
            // The land share of Earth's oceans (0.29).
            let area = |v: &[(f32, f32)]| v.iter().map(|x| x.1).sum::<f32>();
            let share = area(&land) / (area(&land) + area(&sea));
            assert!((share - 0.29).abs() < 0.05, "seed {seed}: land {share}");
            let lowland = 100.0 - above(&land, 0.5);
            let (high, higher, deep) = (above(&land, 2.0), above(&land, 4.0), above(&sea, 6.0));
            let report = format!("seed {seed}: {lowland} {high} {higher} {deep}");
            assert!((45.0..75.0).contains(&lowland), "{report}");
            assert!((4.0..15.0).contains(&high), "{report}");
            assert!((0.5..4.0).contains(&higher), "{report}");
            assert!(deep < 3.0, "{report}");
            // The old ramp: a fifth of the land above 4 km.
            assert!(above(&sea, 1.0) > 70.0, "{report}");
        }
    }

    /// The area share, %, of a world's surface under liquid (open water or
    /// lake), and the lowest |latitude|, degrees, where there is some.
    fn liquid_share(w: &World) -> (f64, f32) {
        let grid = w.params.grid();
        let (mut liquid, mut total, mut nearest) = (0.0f64, 0.0f64, 90.0f32);
        for (i, b) in w.biomes.iter().enumerate() {
            let y = (i / w.width as usize) as u32;
            let area = grid.latitude(y).cos();
            total += area;
            let relief = relief_above(w.elevation[i], w.params.sea_level);
            if relief < w.params.look.rules.open_water() || *b == Biome::Lake {
                liquid += area;
                nearest = nearest.min(grid.latitude(y).to_degrees().abs() as f32);
            }
        }
        (100.0 * liquid / total, nearest)
    }

    #[test]
    fn hydrocarbon_liquid_stays_poleward() {
        let mut p = preset_params("earth-like", 3, 256, 128);
        p.liquid_poleward_deg = Some(55.0);
        p.sea_level = sea_level_for_land_fraction(&p, 0.99);
        let w = generate(&p).world;
        let (share, nearest) = liquid_share(&w);
        assert!(nearest >= 54.0, "liquid down to {nearest} degrees");
        assert!((0.5..2.0).contains(&share), "{share} % under liquid");
    }

    #[test]
    fn the_liquid_mask_reaches_the_sampler_and_the_land_search() {
        let mut p = preset_params("earth-like", 3, 128, 64);
        p.liquid_poleward_deg = Some(55.0);
        let sampler = Sampler::new(&p);
        let grid = p.grid();
        for y in 0..p.height {
            if grid.latitude(y).to_degrees().abs() < 55.0 {
                for x in 0..p.width {
                    let e = sampler.sample(x, y).elevation;
                    assert!(
                        e > p.open_waterline(),
                        "({x}, {y}) at {e} vs {} {}",
                        p.open_waterline(),
                        p.sea_level
                    );
                }
            }
        }
        // The same sea level leaves more land with the mask than without.
        let mut q = p.clone();
        q.liquid_poleward_deg = None;
        assert!(sea_level_for_land_fraction(&p, 0.5) > sea_level_for_land_fraction(&q, 0.5));
    }

    #[test]
    fn water_worlds_are_unchanged() {
        // The field absent, the world is the one without the mask: the
        // golden tests of the water worlds above hold, and a world with
        // the field None serialises without it.
        let p = preset_params("earth-like", 3, 64, 32);
        assert_eq!(p.liquid_poleward_deg, None);
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("liquid_poleward"), "{json}");
        let a = generate(&p).world;
        let b = generate(&WorldParams {
            liquid_poleward_deg: None,
            ..p
        })
        .world;
        assert_eq!(a.elevation, b.elevation);
        assert_eq!(a.biomes, b.biomes);
    }

    #[test]
    fn earth_like_worlds_are_dry_where_earth_is_dry() {
        // Köppen's arid climates B cover 30.2 % of Earth's land (Peel et al.
        // 2007), the tropical ones 19.0 %. Four seeds together, as the
        // calibration was made.
        let earth = "earth-like";
        let (mut land, mut arid, mut tropical) = (0.0f64, 0.0f64, 0.0f64);
        // Mean rain of the 0-10° belt and of the 20-35° belts, mm.
        let mut belt = [(0.0f64, 0.0f64); 2];
        for seed in [1u32, 2, 3, 4] {
            let p = preset_params(earth, seed, 512, 256);
            let w = generate(&p).world;
            let grid = p.grid();
            for i in 0..w.biomes.len() {
                let latitude = grid
                    .latitude((i / w.width as usize) as u32)
                    .to_degrees()
                    .abs();
                let area = grid.latitude((i / w.width as usize) as u32).cos();
                let rain = f64::from(crate::climate::rain_mm(w.moisture[i]));
                if latitude < 10.0 {
                    belt[0].0 += rain * area;
                    belt[0].1 += area;
                } else if (20.0..35.0).contains(&latitude) {
                    belt[1].0 += rain * area;
                    belt[1].1 += area;
                }
                let b = w.biomes[i];
                if b.is_water() && !matches!(b, Biome::Lake | Biome::River) {
                    continue;
                }
                land += area;
                match b {
                    Biome::Desert | Biome::Shrubland => arid += area,
                    Biome::Jungle | Biome::Savanna => tropical += area,
                    _ => {}
                }
            }
        }
        let (arid, tropical) = (arid / land, tropical / land);
        assert!((0.25..0.36).contains(&arid), "arid {arid}");
        assert!((0.12..0.24).contains(&tropical), "tropical {tropical}");
        // The ITCZ is wet and the subtropical highs dry (GPCP: 1 900 mm at
        // 5° N, 665 mm at 25° N).
        let (itcz, highs) = (belt[0].0 / belt[0].1, belt[1].0 / belt[1].1);
        assert!(itcz > 1.8 * highs, "{itcz} vs {highs}");
    }

    #[test]
    fn the_crust_floats_by_its_thickness() {
        let p = earth_params();
        let crust = crust_of(&p);
        let km = |x: f32| floating(x, &crust) * crate::craters::ELEVATION_KM;
        // Airy: continental crust stands above oceanic crust by
        // (tc − to)(1 − ρc/ρm), reached inland as the platform rises.
        let contrast = (crust.continental_km - crust.oceanic_km)
            * (1.0 - crust.crust_density / crust.mantle_density);
        assert!(contrast > 3.5 && contrast < 6.0, "{contrast}");
        assert_eq!(km(0.0), 0.0);
        assert!(km(3.0) <= contrast + 1e-4 && km(3.0) > 0.99 * contrast);
        // The old floor lies about `plates::OLD_FLOOR_KM` below the shelf
        // on Earth's crust.
        let shelf = km(SLOPE_NOISE + SHELF_NOISE);
        assert!(
            (shelf - (crate::plates::OLD_FLOOR_KM - SHELF_KM)).abs() < 0.5,
            "{shelf}"
        );
        // The abyss deepens below the old floor towards Parsons &
        // Sclater's 6.4 km.
        let abyss = ABYSS_KM - crate::plates::OLD_FLOOR_KM;
        assert!(km(-2.0) >= -abyss && km(-2.0) < -0.95 * abyss);
        // Monotonic.
        let mut last = f32::MAX;
        for k in 0..600 {
            let v = km(3.0 - k as f32 / 120.0);
            assert!(v <= last);
            last = v;
        }
        // Gravity does not enter: a crust floats as high under any g.
        let heavy = WorldParams {
            circumference_km: 2.0 * EARTH_CIRCUMFERENCE_KM,
            ..p.clone()
        };
        assert_eq!(crust_of(&heavy), crust);
        // A world without plates takes Mars's dichotomy, its highlands
        // above its lowlands by a few km.
        let mut stagnant = p;
        stagnant.look.geology.regime = Regime::Stagnant;
        let mars = crust_of(&stagnant);
        let dichotomy = (mars.continental_km - mars.oceanic_km)
            * (1.0 - mars.crust_density / mars.mantle_density);
        assert!((3.5..6.0).contains(&dichotomy), "{dichotomy}");
    }

    #[test]
    fn the_crust_line_does_not_move_with_the_sea() {
        let p = earth_params();
        let mask = |level: f32| {
            let sampler = Sampler::new(&WorldParams {
                sea_level: level,
                ..p.clone()
            });
            let grid = p.grid();
            (0..grid.height)
                .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
                .map(|(x, y)| sampler.continental(grid.unit_vector_at(f64::from(x), f64::from(y))))
                .collect::<Vec<bool>>()
        };
        assert_eq!(mask(0.0), mask(0.4));
    }

    #[test]
    fn less_water_exposes_the_basins_too() {
        // A wet world with little water fills its lowest basins, not a
        // ring round a raised platform: the land share rises as the water
        // falls, and land comes out on oceanic crust too.
        let p = earth_params();
        let grid = p.grid();
        let mut last = 0.0;
        for oceans in [1.0, 0.5, 0.1] {
            let level = sea_level_for_water(&p, ocean_depth_m(oceans, 1.0));
            let land = land_share(&p, level);
            assert!(land > last, "{oceans} oceans: land {land} after {last}");
            last = land;
            if oceans == 0.1 {
                let q = WorldParams {
                    sea_level: level,
                    ..p.clone()
                };
                let waterline = level + q.look.rules.waterline;
                let sampler = Sampler::new(&q);
                let (mut basins, mut total) = (0.0, 0.0);
                for y in 0..grid.height {
                    let w = grid.latitude(y).cos();
                    for x in 0..grid.width {
                        total += w;
                        let point = grid.unit_vector_at(f64::from(x), f64::from(y));
                        if sampler.sample(x, y).elevation > waterline && !sampler.continental(point)
                        {
                            basins += w;
                        }
                    }
                }
                assert!(
                    basins / total > 0.05,
                    "land on oceanic crust {}",
                    basins / total
                );
            }
        }
    }

    #[test]
    fn relief_stands_as_one_over_gravity_up_to_a_cap() {
        assert_eq!(relief_scale(1.0), 1.0);
        assert_eq!(relief_scale(2.0), 0.5);
        // A Mars-sized world: 1/g is 2.9, capped below Olympus Mons's 2.5.
        let mars = crate::world::rocky_gravity(21_344.0 / EARTH_CIRCUMFERENCE_KM);
        assert_eq!(relief_scale(mars), MAX_RELIEF_SCALE);
        // The volcanoes scale too: a light world's stand higher.
        let cone = |gravity: f32| {
            let p = WorldParams {
                width: 256,
                height: 128,
                gravity: Some(gravity),
                volcanic_intensity: 1.0,
                ..params(5, "terran")
            };
            let s = Sampler::new(&p);
            (0..p.height)
                .flat_map(|y| (0..p.width).map(move |x| (x, y)))
                .map(|(x, y)| s.sample(x, y).cone_relief)
                .fold(0.0f32, f32::max)
        };
        let (light, earth) = (cone(0.6), cone(1.0));
        assert!(earth > 0.0 && light > 1.5 * earth, "{light} {earth}");
    }

    /// The five climate values of a sample, in [`ClimateField::ALL`] order.
    fn climate_of(s: &Sample) -> [f32; ClimateField::COUNT] {
        [
            s.temperature_anomaly,
            s.moisture,
            s.variability_t,
            s.variability_m,
            s.highland_jitter,
        ]
    }

    /// Where every step is 1, the generation's grids are the per-pixel
    /// sampling itself, bit for bit.
    #[test]
    fn climate_grids_at_step_one_are_the_exact_sampling() {
        let p = params(5, "terran");
        let sampler = Sampler::new(&p);
        let grids = ClimateGrids::new(&sampler, p.width, p.height);
        assert!(grids.fields.iter().all(Option::is_none));
        for y in 0..p.height {
            for x in 0..p.width {
                let (a, b) = (grids.sample(&sampler, x, y), sampler.sample(x, y));
                assert_eq!(
                    climate_of(&a).map(f32::to_bits),
                    climate_of(&b).map(f32::to_bits)
                );
                assert_eq!(a.elevation.to_bits(), b.elevation.to_bits());
            }
        }
    }

    /// On a large map, each climate field read from its coarse grid stays
    /// within 1 % of its range of the field sampled at the pixel: the right
    /// field in the right place.
    #[test]
    fn coarse_climate_grids_follow_each_exact_field() {
        // At 8192 pixels every field is coarse: the variability fields'
        // finest octave spans 13.6 pixels (step 2).
        let p = WorldParams {
            width: 8192,
            height: 4096,
            ..params(11, "terran")
        };
        let sampler = Sampler::new(&p);
        let grids = ClimateGrids::new(&sampler, p.width, p.height);
        assert!(grids.fields.iter().all(Option::is_some));
        let mut worst = [0.0f32; ClimateField::COUNT];
        for y in (0..p.height).step_by(73) {
            for x in (0..p.width).step_by(59) {
                let (a, b) = (grids.sample(&sampler, x, y), sampler.sample(x, y));
                for (k, (u, v)) in climate_of(&a).iter().zip(climate_of(&b)).enumerate() {
                    worst[k] = worst[k].max((u - v).abs());
                }
                assert_eq!(a.elevation.to_bits(), b.elevation.to_bits());
            }
        }
        // fbm spans about [-0.7, 0.7].
        assert!(worst.iter().all(|&w| w < 0.01), "{worst:?}");
    }

    #[test]
    fn elevations_are_compressed_into_range_not_clipped() {
        assert_eq!(within_range(0.5), 0.5);
        assert_eq!(within_range(-0.85), -0.85);
        let (a, b) = (within_range(1.1), within_range(1.4));
        assert!(a < b && b < 1.0, "{a} {b}");
        assert!(within_range(-3.0) >= -1.0);
    }

    /// A dry Venus-sized world without plates is plains with a few
    /// highland provinces, not a mountain fabric: most of it within 1 km
    /// of its median, as on Venus (more than 80 % within 1 km of the mean
    /// radius, Ford & Pettengill 1992), yet mountains in its highlands,
    /// even at 1.4 g.
    #[test]
    fn a_world_without_plates_is_plains_and_highlands() {
        // Area-weighted shares of a world within 1 km of its median
        // elevation and more than 2 km above it.
        let hypsometry = |p: &WorldParams| {
            let s = Sampler::new(p);
            let mut cells: Vec<(f32, f32)> = (0..p.height)
                .flat_map(|y| (0..p.width).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let lat = (0.5 - (y as f32 + 0.5) / p.height as f32) * PI as f32;
                    let km = s.sample(x, y).elevation * crate::craters::ELEVATION_KM;
                    (km, lat.cos())
                })
                .collect();
            cells.sort_by(|a, b| a.0.total_cmp(&b.0));
            let total: f32 = cells.iter().map(|c| c.1).sum();
            let mut acc = 0.0;
            let median = cells
                .iter()
                .find(|c| {
                    acc += c.1;
                    acc >= total / 2.0
                })
                .unwrap()
                .0;
            let share = |keep: &dyn Fn(f32) -> bool| {
                cells
                    .iter()
                    .filter(|c| keep(c.0 - median))
                    .map(|c| c.1)
                    .sum::<f32>()
                    / total
            };
            (share(&|d| d.abs() <= 1.0), share(&|d| d > 2.0))
        };
        for regime in [Regime::Stagnant, Regime::Episodic, Regime::HeatPipe] {
            for (seed, gravity) in [(12345, 0.9), (7, 0.9), (12345, 1.4)] {
                let mut p = WorldParams {
                    width: 256,
                    height: 128,
                    sea_level: DRY_SEA_LEVEL,
                    pressure_bar: 90.0,
                    volcanic_intensity: 0.2,
                    gravity: Some(gravity),
                    ..params(seed, "terran")
                };
                p.look.rules.dry = true;
                p.look.geology.regime = regime;
                let (plains, high) = hypsometry(&p);
                assert!(plains > 0.7, "{regime:?} {seed} {gravity}: {plains}");
                assert!(high > 0.005, "{regime:?} {seed} {gravity}: {high}");
            }
        }
    }

    #[test]
    fn plateaus_without_plates_are_not_mountains() {
        // The highland band ignores the Airy step of the thick crust on
        // worlds without plates (`platform_highlands`). Measured from the
        // sea, `mars-like`'s whole highland crust was mountain: 41 % of the
        // sphere on seeds 1-3. Main, before the crust floated, gave about
        // 9 % (9.2, 9.1, 7.2 %); ignoring the step gives 14.8, 10.8 and
        // 16.1 % at 256 x 128. Heavily cratered seeds (4-6) reach 20-35 %
        // on main too, from their crater rims, and are left out.
        for seed in [1u32, 2, 3] {
            let p = preset_params("mars-like", seed, 256, 128);
            assert_ne!(p.look.geology.regime, Regime::Plates);
            let w = generate(&p).world;
            let grid = p.grid();
            let (mut total, mut high) = (0.0f64, 0.0f64);
            for (i, b) in w.biomes.iter().enumerate() {
                let a = grid.latitude((i / w.width as usize) as u32).cos();
                total += a;
                if matches!(b, Biome::Mountain | Biome::Snow) {
                    high += a;
                }
            }
            let share = high / total;
            assert!((0.02..0.20).contains(&share), "seed {seed}: {share}");
        }
    }

    /// The crust floating by its thickness (part 3c) leaves dry worlds as
    /// they were: their plains on [`DRY_PLATFORM_KM`] (0.5 km) above their
    /// waterline, their mountains from 0.15 above it, their inland band
    /// centred on it.
    #[test]
    fn dry_worlds_keep_their_plains_and_relief_datum() {
        for (name, expected) in [
            // `Sampler::sample` at the points below on origin/main (e243b4b).
            (
                "venus-like",
                [
                    -0.215725,
                    -0.248_503_3,
                    -0.210_015_6,
                    -0.217_175_4,
                    -0.260_295_3,
                    -0.230_008_3,
                ],
            ),
            (
                "mercury-like",
                [
                    -0.204_925_6,
                    -0.270_677_6,
                    -0.191_738_3,
                    -0.222_719_1,
                    -0.235_384_9,
                    -0.279_495_4,
                ],
            ),
        ] {
            let p = preset_params(name, 1, 256, 128);
            assert!(p.look.rules.dry, "{name}");
            // The span of the old formula, with the old 0.5 km platform.
            let scale = relief_scale(p.gravity_modifier());
            let span = 0.5
                + DRY_PLAINS_KM * DRY_PLAINS_SPAN * scale
                + MOUNTAIN_BLEND * scale * crate::craters::ELEVATION_KM;
            assert!((noise_span_km(&p) - span).abs() < 1e-4, "{name}");
            let sampler = Sampler::new(&p);
            for (&(x, y), e) in [(3, 7), (40, 20), (100, 64), (200, 100), (128, 30), (77, 90)]
                .iter()
                .zip(expected)
            {
                let got = sampler.sample(x, y).elevation;
                assert!(
                    (got - e).abs() < 1e-4,
                    "{name} at ({x}, {y}): {got} against {e}"
                );
            }
        }
    }

    /// A dry world has no sea, whatever its low ground, and its relief
    /// stands in the linear part of the range: nothing pinned at its floor.
    #[test]
    fn a_dry_world_has_no_sea_and_room_for_its_relief() {
        let mut p = WorldParams {
            width: 256,
            height: 128,
            sea_level: DRY_SEA_LEVEL,
            pressure_bar: 0.0,
            gravity: Some(0.4),
            erosion: 1.0,
            ..params(3, "terran")
        };
        p.look.rules.dry = true;
        p.look.geology.regime = Regime::Stagnant;
        let w = generate(&p).world;
        let water = w.biomes.iter().filter(|b| b.is_water()).count();
        assert_eq!(water, 0);
        let mut e = w.elevation.clone();
        e.sort_by(f32::total_cmp);
        let (lowest, median) = (e[0], e[e.len() / 2]);
        assert!(lowest > -RANGE_KNEE, "{lowest}");
        // Its plains lie below the waterline's altitude lines, not on
        // them: plains, not highland.
        assert!(
            (median - p.sea_level - p.look.rules.highland) < 0.0,
            "{median}"
        );
        // Its low ground lies below the waterline, and is land.
        assert!(lowest < p.sea_level + p.look.rules.waterline, "{lowest}");
    }

    /// An airless world without plates keeps the relief impacts give it,
    /// whether its lid is stagnant or overturns now and then; a heat-pipe
    /// world (Io) is not held to it.
    #[test]
    fn airless_worlds_without_plates_keep_impact_relief() {
        let mut p = WorldParams {
            pressure_bar: 0.0,
            gravity: Some(1.0),
            ..params(0, "terran")
        };
        for regime in [Regime::Stagnant, Regime::Episodic] {
            p.look.geology.regime = regime;
            let span = relief_share(&p) * noise_span_km(&p);
            assert!((span - IMPACT_RELIEF_KM).abs() < 1e-3, "{regime:?} {span}");
        }
        p.look.geology.regime = Regime::HeatPipe;
        assert_eq!(relief_share(&p), 1.0);
        // With air, an episodic world keeps its whole relief.
        p.look.geology.regime = Regime::Episodic;
        p.pressure_bar = 90.0;
        assert_eq!(relief_share(&p), 1.0);
    }

    #[test]
    fn small_bodies_keep_relief_within_their_caps() {
        // A moon of `radius_km` and `density` (g/cm³), its span of
        // elevation as generated (craters included), km.
        let span = |radius_km: f32, density: f64, composition| {
            let planet = params(0, "terran");
            let mut p = crate::moons::small_body_params(&planet, composition, radius_km, 7, None);
            let r = f64::from(radius_km);
            let mass = 4.0 / 3.0 * PI * r.powi(3) * density * 1e12;
            p.gravity = Some((6.674e-20 * mass / (r * r) * 1000.0 / 9.81) as f32);
            (p.width, p.height) = (256, 128);
            let w = generate(&p).world;
            let (lo, hi) = w
                .elevation
                .iter()
                .fold((f32::MAX, f32::MIN), |(a, b), &e| (a.min(e), b.max(e)));
            (hi - lo) * crate::craters::ELEVATION_KM
        };
        use crate::moons::Composition::{Ice, Rock};
        // A Moon: about the Moon's 13 km.
        let moon = span(1737.0, 3.34, Rock);
        assert!((9.0..18.0).contains(&moon), "Moon {moon}");
        // An icy moon of Ganymede's size holds less than rock: its noise
        // relief 0.4 of rock's and its complex craters 0.4 as deep; it
        // turns complex sooner (`ICE_TRANSITION`), but its simple craters
        // keep rock's bowl (#311).
        let ice = span(2634.0, 1.94, Ice);
        assert!(ice < 0.8 * moon, "ice {ice} rock {moon}");
        // A moonlet: within 8 % of its radius, craters aside.
        let p = crate::moons::small_body_params(&params(0, "terran"), Rock, 32.0, 7, None);
        assert!(relief_share(&p) * noise_span_km(&p) <= SMALL_BODY_RELIEF * 32.0 + 1e-3);
        // Planets keep their whole relief.
        assert_eq!(relief_share(&params(0, "terran")), 1.0);
    }

    #[test]
    fn an_icy_moon_asked_for_alone_has_ices_gravity() {
        let p = WorldParams {
            class: "icy-moon".into(),
            look: Look::moon(crate::moons::Composition::Ice),
            circumference_km: 9_800.0,
            ..params(0, "terran")
        };
        let radius = 9_800.0 / EARTH_CIRCUMFERENCE_KM;
        assert!((p.gravity_modifier() - 1.9 / 5.513 * radius).abs() < 1e-6);
        assert!(icy(&p));
    }

    #[test]
    fn a_moon_world_has_the_gravity_of_its_card() {
        // The moons of small planets, which often have one (rocky and icy).
        let mut seen = 0;
        for seed in 0..40 {
            let planet = WorldParams {
                circumference_km: 20_000.0,
                ..params(seed, "terran")
            };
            let Some(system) = crate::moons::MoonSystem::draw(&planet) else {
                continue;
            };
            for moon in &system.moons {
                let world = crate::moons::body_params(&planet, moon);
                assert_eq!(world.gravity_modifier(), moon.gravity);
                // Not the rocky planets' rule for its size.
                let rule =
                    crate::world::rocky_gravity(world.circumference_km / EARTH_CIRCUMFERENCE_KM);
                assert_ne!(world.gravity_modifier(), rule);
                seen += 1;
            }
        }
        assert!(seen > 5, "{seen}");
    }

    #[test]
    fn progress_reports_every_generation_stage_in_order() {
        let seen = std::sync::Mutex::new(Vec::new());
        generate_with_progress(&params(1, "terran"), GenerateOptions::default(), |s| {
            seen.lock().unwrap().push(s)
        });
        assert_eq!(seen.into_inner().unwrap(), Stage::ALL[..7]);
    }

    /// A sink that keeps every debug layer, joining the bands of each one.
    #[derive(Default)]
    struct Collect(std::sync::Mutex<Vec<DebugLayer>>);

    impl DebugSink for Collect {
        fn layer(&self, piece: DebugLayer) {
            let mut layers = self.0.lock().unwrap();
            match layers.iter_mut().find(|l| l.name == piece.name) {
                Some(layer) => layer.values.extend(piece.values),
                None => layers.push(piece),
            }
        }
    }

    /// Generates with every debug layer, collected.
    fn generate_with_debug(params: &WorldParams) -> (Generated, Vec<DebugLayer>) {
        let sink = Collect::default();
        let g = generate_with_progress(params, GenerateOptions { debug: Some(&sink) }, |_| {});
        (g, sink.0.into_inner().unwrap())
    }

    #[test]
    fn skipping_debug_layers_gives_the_same_world() {
        let p = params(13, "volcanic");
        let (full, debug) = generate_with_debug(&p);
        let lean = generate(&p);
        assert_eq!(debug.len(), NOISE_LAYERS.len() + 12);
        assert_eq!(full.world.biomes, lean.world.biomes);
        assert_eq!(full.world.elevation, lean.world.elevation);
        assert_eq!(full.world.moisture, lean.world.moisture);
        assert_eq!(full.world.rivers.len(), lean.world.rivers.len());
        assert_eq!(full.river_coverage, lean.river_coverage);
    }

    #[test]
    fn dry_planets_have_no_rivers() {
        let barren = generate(&params(3, "barren")).world;
        assert!(barren.rivers.is_empty());
        assert!(barren.floodplain.iter().all(|&f| f == 0.0));
        assert!(!generate(&params(3, "terran")).world.rivers.is_empty());
        assert!(!generate(&params(3, "volcanic")).world.rivers.is_empty());
    }

    #[test]
    fn debug_layers_match_the_world() {
        let p = params(11, "terran");
        let (g, debug) = generate_with_debug(&p);
        let n = p.pixel_count();
        assert!(debug.iter().all(|l| l.values.len() == n));
        let elevation = debug.iter().find(|l| l.name == "elevation").unwrap();
        assert_eq!(elevation.values, g.world.elevation);
    }

    #[test]
    fn handed_over_river_coverage_matches_the_rivers() {
        let g = generate(&params(3, "terran"));
        let w = &g.world;
        let coverage = rivers::coverage(
            &w.rivers,
            w.width,
            w.height,
            w.params.circumference_km as f64,
        );
        assert_eq!(g.river_coverage, coverage);
    }
}
