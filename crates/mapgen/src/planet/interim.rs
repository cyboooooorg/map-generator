//! Transitional mapping of a [`PlanetModel`] onto today's looks, so that
//! a planet the model describes renders with the rules, palette and remap
//! of the built-in look closest to it. Deleted by #424, when the model
//! alone draws a planet.
use rand::RngExt;

use super::classify::ICE_WORLD_MIN_OCEANS;
use super::{PlanetClass, PlanetModel, SurfaceLiquid};
use crate::archetype::ParamDefault;
use crate::atmosphere::{AIRLESS_BAR, Atmosphere, CloudKind};
use crate::biome::Biome;
use crate::generation::{
    ocean_depth_m, sea_level_for_land_fraction, sea_level_for_water, warming_for,
};
use crate::geology::Regime;
use crate::look::Look;
use crate::request::fallback;
use crate::world::WorldParams;

/// Mean surface temperature under which a dry body with water renders
/// frozen, K: water's freezing point.
const FREEZING_K: f64 = 273.0;

/// Least share of water and ices of a dry, cold body that renders frozen:
/// the ice world's threshold of [`super::classify`] (Europa, about 8 %,
/// is one; Earth, 0.02 %, is not).
const FROZEN_MIN_WATER: f64 = 0.05;

/// The built-in look whose rules and remap render `model`, the first
/// rule that holds:
///
/// 1. a giant class: `gas-giant`, `hot-jupiter` or `ice-giant` (a hot
///    Neptune renders as a hot Jupiter, its methane gone; a mini-Neptune
///    as an ice giant);
/// 2. a lava world or a heat-pipe regime: `volcanic`;
/// 3. a runaway greenhouse: `caustic` (the Venus look; the spec names
///    barren or volcanic, caustic is closer to Venus and exists);
/// 4. liquid hydrocarbons: `frozen`;
/// 5. liquid water: `terran`;
/// 6. no liquid, a mean temperature under 273 K and at least 5 % water,
///    or at least 0.01 Earth oceans kept at the surface (its sea level is
///    calibrated on that water, which the barren look would draw as
///    rock): `frozen`;
/// 7. otherwise `barren`.
///
/// A body that is no giant whose envelope hides its surface (the model
/// derives none under at least [`super::escape::MIN_ENVELOPE`] of its
/// mass) renders with its class's look: `frozen` for an ice world,
/// `barren` for the others (rocky, super-Earth). The surface under the
/// envelope is not derived yet (a later part of #423).
pub fn base_of(model: &PlanetModel) -> &'static str {
    match model.class {
        PlanetClass::GasGiant => return "gas-giant",
        PlanetClass::HotJupiter | PlanetClass::HotNeptune => return "hot-jupiter",
        PlanetClass::IceGiant | PlanetClass::MiniNeptune => return "ice-giant",
        _ => {}
    }
    let surface = model.surface.as_ref();
    let liquid = surface.map_or(SurfaceLiquid::None, |s| s.climate.liquid);
    if model.class == PlanetClass::LavaWorld
        || liquid == SurfaceLiquid::Lava
        || surface.is_some_and(|s| s.regime == Regime::HeatPipe)
    {
        return "volcanic";
    }
    if model.class == PlanetClass::RunawayGreenhouse {
        return "caustic";
    }
    let Some(surface) = surface else {
        return if model.class == PlanetClass::IceWorld {
            "frozen"
        } else {
            "barren"
        };
    };
    match liquid {
        SurfaceLiquid::Hydrocarbons => "frozen",
        SurfaceLiquid::Water => "terran",
        _ if surface.climate.temperature_k < FREEZING_K
            && (model.inputs.water_fraction >= FROZEN_MIN_WATER
                || (!surface.water.runaway && surface.water.oceans >= ICE_WORLD_MIN_OCEANS)) =>
        {
            "frozen"
        }
        _ => "barren",
    }
}

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// Earth's mean surface heat flux, W/m²: 44 TW over its surface (Pollack,
/// Hurter & Johnson 1993, doi:10.1029/93RG01249, 87 mW/m²). The flux at
/// which a world is as volcanic as the scale's middle.
const EARTH_HEAT_FLUX_W_M2: f64 = 0.087;

/// Volcanism of a world with Earth's heat flux, on the `[0, 1]` scale of
/// [`crate::request::VOLCANIC`]. Calibration, no source: the middle of
/// the range worlds draw from ([`fallback::VOLCANIC`]).
const EARTH_VOLCANIC: f64 = 0.5;

/// Surface pressure under which water and hydrocarbons carve less, bar:
/// [`crate::atmosphere::LIQUID_WATER_BAR`], under which the generator
/// keeps no liquid. The erosion scales as `min(1, P / 0.01)`.
const ERODING_BAR: f64 = crate::atmosphere::LIQUID_WATER_BAR as f64;

/// Erosion of a world with liquid hydrocarbons against one with liquid
/// water. Calibration, no source: Titan's river networks are fewer and
/// fainter than Earth's; half.
const HYDROCARBON_EROSION: f32 = 0.5;

/// Age of the surface of a world in the episodic regime, Ga: Venus's,
/// 0.3 to 0.8 Ga from its craters (Strom, Schaber & Dawson 1994,
/// *JGR* 99, about 0.3 Ga; McKinnon et al. 1997, *Venus II*, about
/// 0.75 Ga).
const EPISODIC_SURFACE_AGE_GA: ParamDefault = ParamDefault::Range([0.3, 0.8]);

/// Age of the surface of a world in the heat-pipe regime, Ga: Io's, which
/// shows no impact crater, so that its volcanoes resurface it faster than
/// craters form; younger than about a million years (Johnson et al. 1979,
/// *Nature* 280: about 1 cm of lava a year).
const HEAT_PIPE_SURFACE_AGE_GA: ParamDefault = ParamDefault::Range([0.0, 0.001]);

/// Share of the system's age a stagnant lid's surface keeps, a range:
/// Mars's oldest terrains are about 4.1 Ga old and Mercury's about 4.0
/// (Fassett 2016, *JGR Planets* 121), 0.7 to 0.95 of 4.57 Ga
/// allowing for younger lids.
const STAGNANT_SURFACE_AGE_SHARE: ParamDefault = ParamDefault::Range([0.7, 0.95]);

/// Share of a hydrocarbon world's surface that is dry land: Titan's lakes
/// and seas cover about 1 % of it (Hayes 2016,
/// doi:10.1146/annurev-earth-060115-012247; Stofan et al. 2007,
/// doi:10.1038/nature05438, mapped them). The model does not size them.
const HYDROCARBON_LAND_FRACTION: f32 = 0.99;

/// Latitude, degrees, poleward of which a hydrocarbon world's surface
/// liquid lies; the low latitudes are dry ground. Titan's lakes and seas
/// lie almost all (about 97 % of their area) north of 55 N, its low
/// latitudes being dune fields (Hayes 2016,
/// doi:10.1146/annurev-earth-060115-012247; Lorenz et al. 2006,
/// doi:10.1126/science.1123257): the circulation carries methane
/// poleward, evaporation beats rain near the equator and liquid gathers
/// near the poles (Mitchell 2008, doi:10.1029/2007JE003017; Schneider et
/// al. 2012, doi:10.1038/nature10666).
///
/// A calibration with Titan as its target, applied to every hydrocarbon
/// world, planet or moon (see [`WorldParams::liquid_poleward_deg`]); no
/// source gives it as a function of the world. The north-south asymmetry
/// of Titan's lakes (Aharonson et al. 2009) is not modelled.
pub const POLAR_LIQUID_LAT_DEG: f32 = 55.0;

/// Surface pressure past which the sky yellows toward caustic's, bar, and
/// the one at which it is wholly caustic's. Calibration, no source: Venus's
/// sky (92 bar) is yellow, Earth's (1 bar) blue; from 10 to 100 bar on a
/// logarithmic scale.
const YELLOW_BAR: (f64, f64) = (10.0, 100.0);

/// Rayleigh colour of caustic's sky, the Venus look (`looks/caustic.toml`).
const CAUSTIC_RAYLEIGH: [f32; 3] = [0.4, 0.55, 0.25];

/// Haze density and colour of caustic's sky (`looks/caustic.toml`).
const CAUSTIC_HAZE: f32 = 0.3;
const CAUSTIC_HAZE_COLOUR: [f32; 3] = [0.85, 0.95, 0.45];

/// Pressure (bar) within which the sky is Earth's, a tenth either way:
/// Earth's air renders as Earth's. Calibration, no source.
const EARTH_BAR: (f64, f64) = (0.8, 1.25);

/// Methane haze: CH₄ share of the air and surface pressure (bar) over
/// which it forms, its density and Titan's orange. Calibration, no source:
/// Titan (5 % CH₄ near the surface, 1.5 bar) has an opaque orange haze.
const HAZE_CH4_SHARE: f64 = 0.01;
const HAZE_MIN_BAR: f64 = 0.5;
const METHANE_HAZE: f32 = 0.7;
const METHANE_HAZE_COLOUR: [f32; 3] = [0.9, 0.6, 0.3];

/// Partial pressures over which clouds form, bar. Calibration, no source:
/// water where its vapour reaches 1e-3 bar (Earth's air holds about
/// 1e-2); sulfuric acid where SO₂ reaches 1e-3 bar (the model gives Venus
/// 1.7e-2 bar, Mars 3e-6). The model's Venus keeps no water vapour (its
/// runaway lost it; the real one has 30 ppm), so the acid deck does not
/// ask for any.
const SULFURIC_MIN_BAR: f64 = 1.0e-3;
const WATER_CLOUD_MIN_BAR: f64 = 1.0e-3;

/// Surface pressure under which a dry air lifts dust clouds, bar.
/// Calibration, no source: Mars's 6 mbar does, Titan's 1.5 bar does not.
const DUST_MAX_BAR: f64 = 0.1;

/// The biomes that need life, and what they are without it (target,
/// sources), applied to a lifeless world after its base remap.
const LIFELESS: [(Biome, &[Biome]); 4] = [
    (
        Biome::Desert,
        &[Biome::Jungle, Biome::Savanna, Biome::Shrubland],
    ),
    (Biome::DustPlain, &[Biome::Plain]),
    (Biome::RockyWaste, &[Biome::Forest, Biome::Wetland]),
    (Biome::Tundra, &[Biome::Taiga]),
];

/// The look a planet with a surface renders with: its [`base_of`] look's
/// rules, palette and remap with what the model derived: no temperature
/// shift nor greenhouse ([`calibrate`] aims the world at the model's
/// temperature), the internal heat `(F / σ)^¼` of its heat flux `F` (the
/// temperature it alone would keep the surface at), its regime, its sky
/// ([`sky_of`]), and the lifeless remap after the base one when life did
/// not arise ([`LIFELESS`]). Its orbit comes from the system and its sea
/// level from its water.
///
/// # Panics
///
/// When `model` has no surface (a giant).
pub fn look_for(model: &PlanetModel) -> Look {
    let surface = model.surface.as_ref().expect("a planet with a surface");
    let mut look = Look::load(base_of(model));
    look.climate.temperature = 0.0;
    let flux = model.heat.flux_w_m2(model.interior.radius_earth);
    look.climate.internal_heat = (flux / SIGMA).powf(0.25) as f32;
    look.geology.regime = surface.regime;
    look.atmosphere = sky_of(model);
    look.atmosphere.greenhouse = 0.0;
    look.giant = None;
    if surface.life.arose {
        look
    } else {
        look.then_remap(&LIFELESS)
    }
}

/// What a built-in look draws when the model leaves it open (the
/// `[defaults]` of the former preset files): its pressure, surface age,
/// tilt and erosion, each a value or a range, `None` for the generator's
/// [`fallback`]. Calibration, no source: the values the presets drew
/// from, which keep Venus's air dense, a frozen world's thin, a giant's
/// sky thin above its cloud tops and the tilts of the ice giants
/// and hot Jupiters (tides have righted a hot one's; Uranus lies on its
/// side, 97.8° or 82° spinning backwards).
#[derive(Default)]
struct LookDraws {
    pressure: Option<ParamDefault>,
    surface_age: Option<ParamDefault>,
    axial_tilt: Option<ParamDefault>,
    erosion: Option<ParamDefault>,
}

/// The [`LookDraws`] of the built-in look `base`.
fn draws_of(base: &str) -> LookDraws {
    use ParamDefault::{Fixed, Range};
    let giant = |axial_tilt| LookDraws {
        pressure: Some(Fixed(0.3)),
        surface_age: Some(Fixed(0.0)),
        axial_tilt,
        erosion: None,
    };
    match base {
        "barren" => LookDraws {
            pressure: Some(Range([0.0, 0.01])),
            surface_age: Some(Range([3.0, 4.2])),
            // Water shaped its valleys long ago.
            erosion: Some(Fixed(1.0)),
            ..LookDraws::default()
        },
        "caustic" => LookDraws {
            pressure: Some(Range([2.0, 5.0])),
            surface_age: Some(Range([0.3, 0.8])),
            ..LookDraws::default()
        },
        "frozen" => LookDraws {
            pressure: Some(Range([0.3, 1.0])),
            surface_age: Some(Range([0.5, 2.0])),
            ..LookDraws::default()
        },
        "volcanic" => LookDraws {
            pressure: Some(Range([5.0, 20.0])),
            surface_age: Some(Range([0.001, 0.05])),
            ..LookDraws::default()
        },
        "gas-giant" => giant(None),
        "hot-jupiter" => giant(Some(Range([0.0, 3.0]))),
        "ice-giant" => giant(Some(Range([0.0, 90.0]))),
        _ => LookDraws::default(),
    }
}

/// The sky of a planet's air: its colour and aerosols, not its thickness
/// (the viewer scales the Rayleigh depth with the surface pressure, so
/// counting the column here too would make dense airs opaque). Earth's
/// ([`Atmosphere::default`]) for an air of Earth's pressure
/// ([`EARTH_BAR`]); otherwise Earth's Rayleigh colour, haze and haze
/// colour yellowed toward caustic's past 10 bar ([`YELLOW_BAR`]), so that
/// a Venus reads pale yellow. No haze under [`AIRLESS_BAR`]. A methane
/// haze of Titan's orange where CH₄ is over 1 % of an air thicker than
/// 0.5 bar. Clouds, the first that holds: none under [`AIRLESS_BAR`];
/// water where water or hydrocarbons are liquid or water vapour reaches
/// [`WATER_CLOUD_MIN_BAR`] (methane clouds render as water ones, no other
/// kind exists); sulfuric where SO₂ reaches [`SULFURIC_MIN_BAR`]; dust on
/// a dry air under [`DUST_MAX_BAR`]; otherwise none.
///
/// # Panics
///
/// When `model` has no surface (a giant).
pub fn sky_of(model: &PlanetModel) -> Atmosphere {
    let surface = model.surface.as_ref().expect("a planet with a surface");
    let gases = &surface.gases;
    let pressure = gases.total();
    let earth = Atmosphere::default();
    let mut sky = earth.clone();
    if !(EARTH_BAR.0..=EARTH_BAR.1).contains(&pressure) {
        let t = ((pressure / YELLOW_BAR.0).log10() / (YELLOW_BAR.1 / YELLOW_BAR.0).log10())
            .clamp(0.0, 1.0) as f32;
        let mix = |from: f32, to: f32| from + (to - from) * t;
        sky.rayleigh = std::array::from_fn(|i| mix(earth.rayleigh[i], CAUSTIC_RAYLEIGH[i]));
        sky.haze = mix(earth.haze, CAUSTIC_HAZE);
        sky.haze_colour =
            std::array::from_fn(|i| mix(earth.haze_colour[i], CAUSTIC_HAZE_COLOUR[i]));
        if pressure < AIRLESS_BAR as f64 {
            sky.haze = 0.0;
        }
    }
    if pressure > HAZE_MIN_BAR && gases.ch4 > HAZE_CH4_SHARE * pressure {
        sky.haze = METHANE_HAZE;
        sky.haze_colour = METHANE_HAZE_COLOUR;
    }
    let wet = matches!(
        surface.climate.liquid,
        SurfaceLiquid::Water | SurfaceLiquid::Hydrocarbons
    );
    sky.clouds = if pressure < AIRLESS_BAR as f64 {
        CloudKind::None
    } else if wet || gases.h2o >= WATER_CLOUD_MIN_BAR {
        CloudKind::Water
    } else if gases.so2 >= SULFURIC_MIN_BAR {
        CloudKind::Sulfuric
    } else if pressure < DUST_MAX_BAR {
        CloudKind::Dust
    } else {
        CloudKind::None
    };
    sky
}

/// Whether the world of `model` renders as a giant's: its [`base_of`]
/// look is a giant's (a gas giant, a hot Jupiter, an ice giant or a
/// mini-Neptune).
pub fn renders_as_giant(model: &PlanetModel) -> bool {
    matches!(base_of(model), "gas-giant" | "hot-jupiter" | "ice-giant")
}

/// The world parameters of a planet from its `model`, placed as `placed`
/// says (its seed, star, orbit, map size, moons…), with three kinds of
/// world:
///
/// - a planet with a surface, with [`look_for`]'s look and what the
///   model derived:
///   - its circumference, `2π R`;
///   - its surface pressure, the air's total (within
///     [`crate::request::PRESSURE_BAR`]);
///   - its erosion: 1 with liquid water, [`HYDROCARBON_EROSION`] with
///     hydrocarbons, none otherwise, times `min(1, P / 0.01 bar)`;
///   - its volcanism, `0.5 √(F / 0.087 W/m²)` of its heat flux `F`
///     against Earth's ([`EARTH_HEAT_FLUX_W_M2`]), 1 in the heat-pipe
///     regime, halved under a stagnant lid, within
///     [`crate::request::VOLCANIC`];
///   - its surface age by regime: plates as the generator's default
///     ([`fallback::SURFACE_AGE_GA`]), episodic Venus's
///     ([`EPISODIC_SURFACE_AGE_GA`]), heat-pipe Io's
///     ([`HEAT_PIPE_SURFACE_AGE_GA`]), a stagnant lid most of the
///     system's age ([`STAGNANT_SURFACE_AGE_SHARE`]);
///   - its gravity, the model's (the star's tides, the moons, the tilt
///     they leave it and its tidal flats see the model's mass);
///   - its regime, the model's (its look's, see [`look_for`]), not
///     [`crate::geology::regime_of`]'s;
/// - a giant ([`renders_as_giant`]): its giant look at the model's mass
///   (no less than the smallest giant's, [`crate::request::GIANT_MASS`]:
///   a mini-Neptune renders as an ice giant of that mass), its radius the
///   giant look's (`giant_radius`: issue #446 tracks the gap), no ground,
///   no giant neighbours and nobody's moon;
/// - a body that is no giant whose envelope hides its surface: its class's
///   look ([`base_of`]: `frozen` for an ice world, `barren` otherwise) at
///   the model's radius and gravity, with no ocean and no temperature
///   shift (its generation aims it at its bare temperature,
///   [`calibrate_enveloped`]). The surface under such an envelope is not
///   derived yet (a later part of #423).
///
/// What the model leaves open comes from the look's own draws, else the
/// generator's [`fallback`], drawn from `rng` in a fixed order (sea level,
/// volcanism, tilt, erosion, pressure, surface age) whatever is set. A
/// tilt `placed` leaves open is then steadied by the moons and stood
/// upright by the star's tides; the moons widen or narrow the tidal flats;
/// the moon of a giant takes no moons of its own and the giant's tidal
/// heat raises its volcanism.
///
/// The sea level and the temperature shift of a planet with a surface are
/// [`calibrate`]'s, when the world is generated.
pub fn params_for(
    model: &PlanetModel,
    placed: &crate::request::Placed,
    rng: &mut impl RngExt,
) -> WorldParams {
    use crate::request::{GIANT_MASS, PRESSURE_BAR, SEA_LEVEL, SURFACE_AGE_GA, VOLCANIC};
    let base = base_of(model);
    let giant = renders_as_giant(model);
    let surface = model.surface.as_ref().filter(|_| !giant);
    let mut look = match surface {
        Some(_) => look_for(model),
        None => Look::load(base),
    };
    let d = draws_of(base);
    let pick = |own: Option<ParamDefault>, fb: ParamDefault, mut rng: &mut dyn rand::Rng| {
        own.unwrap_or(fb).pick(&mut rng)
    };
    // Drawn in this order whatever is used, so that no draw shifts another.
    let sea_level = pick(None, fallback::SEA_LEVEL, rng);
    let look_volcanic = pick(None, fallback::VOLCANIC, rng);
    let drawn_tilt = pick(d.axial_tilt, fallback::AXIAL_TILT_DEG, rng);
    let look_erosion = pick(d.erosion, fallback::EROSION, rng);
    let look_pressure = pick(d.pressure, fallback::PRESSURE_BAR, rng);
    let look_age = pick(d.surface_age, fallback::SURFACE_AGE_GA, rng);
    let regime_age = match surface.map(|s| s.regime) {
        Some(Regime::Plates) | None => fallback::SURFACE_AGE_GA.pick(rng),
        Some(Regime::Episodic) => EPISODIC_SURFACE_AGE_GA.pick(rng),
        Some(Regime::HeatPipe) => HEAT_PIPE_SURFACE_AGE_GA.pick(rng),
        Some(Regime::Stagnant) => placed.age_gyr * STAGNANT_SURFACE_AGE_SHARE.pick(rng),
    };
    let radius_km = (model.interior.radius_earth * crate::exomoon::EARTH_M / 1000.0) as f32;
    let model_circumference = 2.0 * std::f32::consts::PI * radius_km;
    let star = crate::star::StarInfo::new(
        placed.star,
        placed.orbit_au,
        placed.eccentricity,
        placed.age_gyr,
    );
    let giant_mass = (model.inputs.mass_earth as f32).clamp(GIANT_MASS.min, GIANT_MASS.max);
    let (sea_level, circumference_km, pressure_bar, erosion, volcanic, surface_age_ga) =
        match surface {
            Some(surface) => {
                let pressure = surface.gases.total();
                let flux = model.heat.flux_w_m2(model.interior.radius_earth);
                let wet = match surface.climate.liquid {
                    SurfaceLiquid::Water => 1.0,
                    SurfaceLiquid::Hydrocarbons => HYDROCARBON_EROSION,
                    _ => 0.0,
                };
                let erosion = wet * (pressure / ERODING_BAR).min(1.0) as f32;
                let volcanic = match surface.regime {
                    Regime::HeatPipe => 1.0,
                    regime => {
                        let v = EARTH_VOLCANIC * (flux / EARTH_HEAT_FLUX_W_M2).sqrt();
                        if regime == Regime::Stagnant {
                            0.5 * v
                        } else {
                            v
                        }
                    }
                };
                (
                    sea_level,
                    model_circumference,
                    (pressure as f32).clamp(PRESSURE_BAR.min, PRESSURE_BAR.max),
                    erosion,
                    volcanic as f32,
                    regime_age,
                )
            }
            None if giant => (
                sea_level,
                crate::giants::giant_radius(giant_mass, star.equilibrium_k) * 40_075.0,
                look_pressure,
                0.0,
                0.0,
                look_age,
            ),
            None => {
                // Without liquid water, rain carves nothing, unless the
                // look sets its erosion (a planet that was wetter once).
                let erosion =
                    if d.erosion.is_none() && look_pressure < crate::atmosphere::LIQUID_WATER_BAR {
                        0.0
                    } else {
                        look_erosion
                    };
                // Its generation sets the whole temperature shift (see
                // `crate::system::generation_params`).
                look.climate.temperature = 0.0;
                look.atmosphere.greenhouse = 0.0;
                (
                    SEA_LEVEL.min,
                    model_circumference,
                    look_pressure,
                    erosion,
                    look_volcanic,
                    look_age,
                )
            }
        };
    let tilt_drawn = placed.axial_tilt_deg.is_none() && d.axial_tilt.is_none();
    let mut params = WorldParams {
        width: placed.width,
        height: placed.height,
        seed: placed.seed,
        class: model.class.key().to_string(),
        archetype: None,
        model: Some(model.clone()),
        look,
        sea_level,
        volcanic_intensity: volcanic.clamp(VOLCANIC.min, VOLCANIC.max),
        circumference_km,
        axial_tilt_deg: placed.axial_tilt_deg.unwrap_or(drawn_tilt),
        erosion,
        pressure_bar,
        // A surface is never older than its system.
        surface_age_ga: surface_age_ga
            .clamp(SURFACE_AGE_GA.min, SURFACE_AGE_GA.max)
            .min(placed.age_gyr),
        star: placed.star,
        orbit_au: placed.orbit_au,
        eccentricity: placed.eccentricity,
        drift_ma: placed.drift_ma,
        moons: placed.moons,
        shape: None,
        metallicity: placed.metallicity,
        giants: if giant {
            crate::giants::GiantsMode::None
        } else {
            placed.giants
        },
        giant_mass_earth: giant.then_some(giant_mass),
        host: if giant {
            crate::exomoon::HostMode::None
        } else {
            placed.host
        },
        given_rings: None,
        impact_factor: None,
        locked_to_star: None,
        resonance_to_star: None,
        moon: false,
        day_hours: None,
        // Hydrocarbons gather near the poles, whatever the world.
        liquid_poleward_deg: surface
            .is_some_and(|s| s.climate.liquid == SurfaceLiquid::Hydrocarbons)
            .then_some(POLAR_LIQUID_LAT_DEG),
        ridge: None,
        // A giant's gravity follows from its own mass.
        gravity: (!giant).then_some(model.interior.gravity as f32),
        age_gyr: placed.age_gyr,
        // Drawn from its own stream: the other draws keep their values.
        perihelion_longitude_deg: Some(placed.perihelion_longitude_deg.unwrap_or_else(|| {
            use rand::SeedableRng;
            let stream = crate::noise::stream_seed(placed.seed, crate::noise::stream::PERIHELION);
            let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(stream));
            rng.random::<f32>() * 360.0
        })),
    };
    // The moon of a giant: no moons of its own (the giant's tides take
    // them). Whether it is one depends on the seed and the request only.
    if crate::exomoon::HostGiant::draw(&params).is_some() {
        params.moons = crate::moons::MoonsMode::None;
    }
    // What the moons do to their planet: a large one keeps a drawn tilt in
    // the usual range (a free one ranges wider), and the tides (moons and
    // star) widen or narrow the tidal flats. Applied to the parameters, so
    // `world.json` (its tilt, its look's shore width) explains the world
    // as generated.
    let system = crate::moons::MoonSystem::draw(&params);
    let effects = crate::moons::PlanetEffects::of(&params, system.as_ref());
    if tilt_drawn && let ParamDefault::Range(range) = fallback::AXIAL_TILT_DEG {
        params.axial_tilt_deg = effects.tilt(params.axial_tilt_deg, (range[0], range[1]));
    }
    // The tides that despin a planet also stand it upright, its axis along
    // its orbit's (see `crate::spin::upright`).
    if tilt_drawn && !giant {
        params.axial_tilt_deg = crate::spin::upright(&params, params.axial_tilt_deg);
    }
    params.look.rules.shore_width_km *= effects.shore_factor;
    // Tidal heat raises the volcanism of a giant's moon, so that
    // `host.tidal_flux` in `world.json` explains it.
    if let Some(host) = crate::exomoon::HostGiant::draw(&params) {
        params.volcanic_intensity = params.volcanic_intensity.max(host.volcanism());
    }
    // A body whose surface the model does not derive (a giant, or a world
    // under its envelope) has the regime of its base look, a plate look's
    // decided by its size and water, applied to its look so that
    // `world.json` tells the regime it was generated with.
    if surface.is_none() {
        params.look.geology.regime = crate::geology::regime_of(&params);
    }
    params
}

/// Sets the sea level and the temperature shift of the world of a planet
/// with a surface, from [`params_for`]'s parameters, in this order (the
/// warming reads the sea level): its sea level, where its oceans
/// ([`ocean_depth_m`]) fill the relief ([`sea_level_for_water`]; a
/// runaway's water is steam, none on the ground; with no water at all the
/// world is [`dry`](crate::biome::BiomeRules::dry), its sea level
/// [`DRY_SEA_LEVEL`](crate::generation::DRY_SEA_LEVEL)), or for a hydrocarbon
/// world, whose lakes the model does not size, Titan's share of land
/// ([`HYDROCARBON_LAND_FRACTION`]) counted over the whole surface, the
/// low latitudes (see [`POLAR_LIQUID_LAT_DEG`]) being land whatever the
/// sea level; then the temperature shift that brings
/// the world's mean to the model's ([`warming_for`]).
///
/// Both pass over a coarse grid of the relief (about a second in all in
/// release), so a system's draw leaves them to the generation of the
/// world (`crate::system::generation_params`): the drawn parameters keep
/// the sea level the resolution drew and no temperature shift.
///
/// # Panics
///
/// When `model` has no surface (a giant).
pub fn calibrate(params: &mut WorldParams, model: &PlanetModel) {
    let surface = model.surface.as_ref().expect("a planet with a surface");
    params.sea_level = if surface.climate.liquid == SurfaceLiquid::Hydrocarbons {
        sea_level_for_land_fraction(params, HYDROCARBON_LAND_FRACTION)
    } else {
        let oceans = if surface.water.runaway {
            0.0
        } else {
            surface.water.oceans
        };
        let depth_m = ocean_depth_m(oceans, model.interior.radius_earth);
        // No water at the surface: no sea, and the sea level only sets the
        // datum of the relief, where it has room (see `DRY_SEA_LEVEL`).
        params.look.rules.dry = depth_m <= 0.0;
        if params.look.rules.dry {
            crate::generation::DRY_SEA_LEVEL
        } else {
            sea_level_for_water(params, depth_m)
        }
    };
    params.look.climate.temperature = warming_for(params, surface.climate.temperature_k);
}

/// Temperature of a bare surface under the starlight `equilibrium_k`
/// (K) and the internal heat flux `heat_flux_w_m2` (W/m²), K:
/// [`super::surface_temperature_k`] with no air, `(T_eq⁴ + F/σ)^¼`.
fn bare_temperature_k(equilibrium_k: f64, heat_flux_w_m2: f64) -> f64 {
    (equilibrium_k.max(0.0).powi(4) + heat_flux_w_m2.max(0.0) / SIGMA).powf(0.25)
}

/// Sets the temperature shift of the world of a body that is no giant
/// whose envelope hides its surface ([`base_of`]): the model derives no
/// surface temperature under the envelope, so the world aims at the
/// temperature a bare surface keeps at its orbit (the equilibrium of
/// `params`'s star and orbit, plus its internal heat: no greenhouse), with
/// [`warming_for`]. No sea: its world keeps none. The surface under the
/// envelope is not derived yet (a later part of #423).
pub fn calibrate_enveloped(params: &mut WorldParams, model: &PlanetModel) {
    let equilibrium_k = f64::from(crate::spin::star_info(params).equilibrium_k);
    let flux = model.heat.flux_w_m2(model.interior.radius_earth);
    params.look.climate.temperature = warming_for(params, bare_temperature_k(equilibrium_k, flux));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::planet_model;
    use crate::planet::reference::{model_inputs, reference, sun_at};
    use crate::request::Placed;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// A world at Earth's orbit round the Sun, 128×64.
    fn earth_placed(seed: u32) -> Placed {
        Placed {
            seed,
            star: crate::star::SUN,
            age_gyr: 4.6,
            orbit_au: 1.0,
            eccentricity: 0.0167,
            metallicity: 0.0,
            axial_tilt_deg: None,
            perihelion_longitude_deg: None,
            drift_ma: 0.0,
            width: 128,
            height: 64,
            moons: crate::moons::MoonsMode::Auto,
            giants: crate::giants::GiantsMode::None,
            host: crate::exomoon::HostMode::None,
        }
    }

    /// The model of the reference body `name` at its orbit round the Sun.
    fn model_of(name: &str) -> PlanetModel {
        let body = reference(name);
        planet_model(&sun_at(body.orbit_au), model_inputs(body))
    }

    #[test]
    fn solar_bodies_render_with_their_look() {
        // Earth, Venus, Mars, Mercury and Titan's inputs from
        // planet::reference at their orbits round the Sun.
        let expect = [
            ("Earth", "terran"),
            ("Venus", "caustic"),
            ("Mars", "barren"),
            ("Mercury", "barren"),
            ("Titan", "frozen"),
        ];
        for (name, base) in expect {
            assert_eq!(base_of(&model_of(name)), base, "{name}");
        }
    }

    #[test]
    fn a_cold_rock_with_ice_renders_frozen() {
        // Mars's model with 0.4 Earth oceans of water kept at its surface
        // (a cold M-dwarf planet of the coherence check, mgs-65436-108 b),
        // its bulk still dry by mass: its ice shows. A trace, as much as
        // Mars keeps, leaves it barren.
        let mut icy = model_of("Mars");
        icy.surface.as_mut().unwrap().water.oceans = 0.4;
        assert_eq!(base_of(&icy), "frozen");
        icy.surface.as_mut().unwrap().water.oceans = 0.004;
        assert_eq!(base_of(&icy), "barren");
    }

    #[test]
    fn the_tides_read_the_models_mass() {
        // Titan's model (gravity 0.14, under the rocky relation's at its
        // size) and Mercury's (0.38, over it), seeds 1 to 12 at 1 AU: the
        // tidal flats widen with the tides of the moons drawn at the
        // model's mass, as the world draws them when generated.
        let mut checked = 0;
        for name in ["Titan", "Mercury"] {
            let model = model_of(name);
            for seed in 1..=12 {
                let params = params_for(
                    &model,
                    &earth_placed(seed),
                    &mut StdRng::seed_from_u64(seed.into()),
                );
                assert_eq!(params.gravity, Some(model.interior.gravity as f32));
                let moons = crate::moons::MoonSystem::draw(&params);
                let effects = crate::moons::PlanetEffects::of(&params, moons.as_ref());
                let base = look_for(&model).rules.shore_width_km;
                assert_eq!(
                    params.look.rules.shore_width_km,
                    base * effects.shore_factor,
                    "{name} {seed}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 24);
    }

    #[test]
    fn only_hydrocarbon_worlds_keep_their_liquid_near_the_poles() {
        let params = |name: &str| {
            params_for(
                &model_of(name),
                &earth_placed(1),
                &mut StdRng::seed_from_u64(1),
            )
        };
        assert_eq!(
            params("Titan").liquid_poleward_deg,
            Some(POLAR_LIQUID_LAT_DEG)
        );
        for name in ["Earth", "Mars", "Venus"] {
            assert_eq!(params(name).liquid_poleward_deg, None, "{name}");
        }
    }

    #[test]
    fn a_titan_has_its_lakes_near_the_poles() {
        let model = model_of("Titan");
        for seed in [1, 2] {
            let mut placed = earth_placed(seed);
            (placed.width, placed.height) = (256, 128);
            let mut params = params_for(&model, &placed, &mut StdRng::seed_from_u64(seed.into()));
            calibrate(&mut params, &model);
            let world = crate::generation::generate(&params).world;
            let grid = params.grid();
            let (mut liquid, mut total) = (0.0f64, 0.0f64);
            for (i, biome) in world.biomes.iter().enumerate() {
                let y = (i / world.width as usize) as u32;
                let area = grid.latitude(y).cos();
                total += area;
                let relief = world.elevation[i] - params.sea_level;
                if relief < params.look.rules.open_water() || *biome == crate::biome::Biome::Lake {
                    liquid += area;
                    let lat = grid.latitude(y).to_degrees().abs();
                    assert!(
                        lat >= f64::from(POLAR_LIQUID_LAT_DEG) - 1.5,
                        "seed {seed}: {lat}"
                    );
                }
            }
            let share = 100.0 * liquid / total;
            assert!((0.3..3.0).contains(&share), "seed {seed}: {share} %");
        }
    }

    #[test]
    fn giants_render_as_giants() {
        for (name, base) in [
            ("Jupiter", "gas-giant"),
            ("Saturn", "gas-giant"),
            ("Uranus", "ice-giant"),
            ("Neptune", "ice-giant"),
        ] {
            assert_eq!(base_of(&model_of(name)), base, "{name}");
        }
        let mut mini = model_of("Neptune");
        mini.class = PlanetClass::MiniNeptune;
        assert_eq!(base_of(&mini), "ice-giant");
        let mut hot = model_of("Jupiter");
        hot.class = PlanetClass::HotJupiter;
        assert_eq!(base_of(&hot), "hot-jupiter");
        hot.class = PlanetClass::HotNeptune;
        assert_eq!(base_of(&hot), "hot-jupiter");
    }
    #[test]
    fn a_lifeless_earth_has_no_forest() {
        let mut earth = model_of("Earth");
        earth.surface.as_mut().unwrap().life.arose = false;
        let p = look_for(&earth);
        for b in [
            Biome::Forest,
            Biome::Jungle,
            Biome::Taiga,
            Biome::Savanna,
            Biome::Plain,
            Biome::Shrubland,
            Biome::Wetland,
        ] {
            assert_ne!(p.remap(b), b, "{b:?}");
        }
        assert_eq!(p.remap(Biome::Taiga), Biome::Tundra);
        // The composed remap reads back from JSON the same.
        let back: Look = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        for b in Biome::ALL {
            assert_eq!(back.remap(b), p.remap(b), "{b:?}");
        }
    }

    #[test]
    fn a_living_earth_keeps_its_forests() {
        let earth = model_of("Earth");
        assert!(earth.surface.unwrap().life.arose);
        let p = look_for(&earth);
        assert_eq!(p.remap(Biome::Forest), Biome::Forest);
    }

    #[test]
    fn earths_sky_is_earths() {
        assert_eq!(sky_of(&model_of("Earth")), Atmosphere::default());
    }

    #[test]
    fn a_dense_air_keeps_a_colour_not_a_column() {
        let sky = sky_of(&model_of("Venus"));
        let brightest = sky.rayleigh.iter().copied().fold(0.0_f32, f32::max);
        assert!(brightest <= 1.0, "{:?}", sky.rayleigh);
        assert!(sky.haze >= 0.25, "{}", sky.haze);
    }

    #[test]
    fn venus_has_a_sulfuric_deck_and_mars_dust() {
        assert_eq!(sky_of(&model_of("Venus")).clouds, CloudKind::Sulfuric);
        assert_eq!(sky_of(&model_of("Mars")).clouds, CloudKind::Dust);
    }

    #[test]
    fn every_solar_sky_is_valid() {
        for name in ["Mercury", "Venus", "Earth", "Mars", "Titan", "Io", "Europa"] {
            let model = model_of(name);
            if model.surface.is_some() {
                sky_of(&model)
                    .validate()
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
            }
        }
    }

    #[test]
    fn earths_world_is_earth_sized_wet_and_mild() {
        let params = params_for(
            &model_of("Earth"),
            &earth_placed(1),
            &mut StdRng::seed_from_u64(1),
        );
        assert!(
            (params.circumference_km - 40_075.0).abs() < 2_000.0,
            "{}",
            params.circumference_km
        );
        assert!(
            (params.pressure_bar - 1.0).abs() < 0.5,
            "{}",
            params.pressure_bar
        );
        assert_eq!(params.look.geology.regime, Regime::Plates);
        // land share and mean temperature checked by Tasks 3 and 4
    }
}
