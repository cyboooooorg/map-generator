//! Transitional mapping of a [`PlanetModel`] onto today's presets, so that
//! a planet the model describes renders with the rules, palette and remap
//! of the built-in preset closest to it. Deleted by #424, when archetypes
//! replace the presets.
use rand::RngExt;

use super::{PlanetClass, PlanetModel, SurfaceLiquid};
use crate::atmosphere::{AIRLESS_BAR, Atmosphere, CloudKind};
use crate::biome::Biome;
use crate::generation::{
    ocean_depth_m, sea_level_for_land_fraction, sea_level_for_water, warming_for,
};
use crate::geology::Regime;
use crate::preset::{ParamDefault, Preset};
use crate::request::{WorldRequest, fallback};
use crate::star::StarSpec;
use crate::world::WorldParams;

/// Mean surface temperature under which a dry body with water renders
/// frozen, K: water's freezing point.
const FREEZING_K: f64 = 273.0;

/// Least share of water and ices of a dry, cold body that renders frozen:
/// the ice world's threshold of [`super::classify`] (Europa, about 8 %,
/// is one; Earth, 0.02 %, is not).
const FROZEN_MIN_WATER: f64 = 0.05;

/// The built-in preset whose rules and remap render `model`, the first
/// rule that holds:
///
/// 1. a giant class: `gas-giant`, `hot-jupiter` or `ice-giant` (a
///    mini-Neptune renders as an ice giant);
/// 2. a lava world or a heat-pipe regime: `volcanic`;
/// 3. a runaway greenhouse: `caustic` (the Venus look; the spec names
///    barren or volcanic, caustic is closer to Venus and exists);
/// 4. liquid hydrocarbons: `frozen`;
/// 5. liquid water: `terran`;
/// 6. no liquid, a mean temperature under 273 K and at least 5 % water:
///    `frozen`;
/// 7. otherwise `barren` (a body with no surface under a thin envelope
///    too: the model gives it no temperature at the surface).
pub fn base_of(model: &PlanetModel) -> &'static str {
    match model.class {
        PlanetClass::GasGiant => return "gas-giant",
        PlanetClass::HotJupiter => return "hot-jupiter",
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
        return "barren";
    };
    match liquid {
        SurfaceLiquid::Hydrocarbons => "frozen",
        SurfaceLiquid::Water => "terran",
        _ if surface.climate.temperature_k < FREEZING_K
            && model.inputs.water_fraction >= FROZEN_MIN_WATER =>
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
/// the range the presets draw from ([`fallback::VOLCANIC`]).
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

/// Column of air (pressure over gravity, Earth's = 1) within which the sky
/// keeps Earth's colours and density. Calibration, no source: a quarter
/// either way, so that Earth's air renders as Earth's.
const EARTH_COLUMN: (f64, f64) = (0.8, 1.25);

/// Surface pressure past which the sky yellows toward caustic's, bar, and
/// the one at which it is wholly caustic's. Calibration, no source: Venus's
/// sky (92 bar) is yellow, Earth's (1 bar) blue; from 10 to 100 bar on a
/// logarithmic scale.
const YELLOW_BAR: (f64, f64) = (10.0, 100.0);

/// Rayleigh colour of caustic's sky, the Venus look (`presets/caustic.toml`).
const CAUSTIC_RAYLEIGH: [f32; 3] = [0.4, 0.55, 0.25];

/// Largest Rayleigh channel the atmosphere accepts
/// ([`Atmosphere::validate`]).
const MAX_RAYLEIGH: f32 = 10.0;

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

/// The preset a planet with a surface renders with: its [`base_of`]
/// preset's rules, palette and remap, named after its class, with what the
/// model derived: no temperature shift nor greenhouse ([`params_for`]'s
/// finish aims the world at the model's temperature), the internal heat
/// `(F / σ)^¼` of its heat flux `F` (the temperature it alone would keep
/// the surface at), its regime, its sky ([`sky_of`]), and the lifeless
/// remap after the base one when life did not arise ([`LIFELESS`]). Its
/// orbit comes from the system and its sea level from its water: no star
/// spec, no land share.
///
/// # Panics
///
/// When `model` has no surface (a giant).
pub fn preset_for(model: &PlanetModel) -> Preset {
    let surface = model.surface.as_ref().expect("a planet with a surface");
    let mut preset = Preset::load(base_of(model)).expect("built-in presets load");
    preset.name = model.class.key().to_string();
    preset.locales.clear();
    preset.climate.temperature = 0.0;
    let flux = model.heat.flux_w_m2(model.interior.radius_earth);
    preset.climate.internal_heat = (flux / SIGMA).powf(0.25) as f32;
    preset.geology.regime = surface.regime;
    preset.atmosphere = sky_of(model);
    preset.atmosphere.greenhouse = 0.0;
    preset.star = StarSpec::default();
    preset.giant = None;
    preset.defaults.land_fraction = None;
    preset.defaults.sea_level = None;
    preset.derived = true;
    if surface.life.arose {
        preset
    } else {
        preset.then_remap(&LIFELESS)
    }
}

/// The sky of a planet's air: Earth's ([`Atmosphere::default`]) for an air
/// of Earth's column ([`EARTH_COLUMN`]); otherwise Earth's Rayleigh colour
/// scaled by the column (`P / g`, Earth's = 1, as much gas over each
/// square metre) and yellowed toward caustic's past 10 bar
/// ([`YELLOW_BAR`]), its brightest channel held to the atmosphere's
/// limit. A methane haze of Titan's orange where CH₄ is over 1 % of an air
/// thicker than 0.5 bar. Clouds, the first that holds: none under
/// [`AIRLESS_BAR`]; water where water or hydrocarbons are liquid or water
/// vapour reaches [`WATER_CLOUD_MIN_BAR`] (methane clouds render as water
/// ones, no other kind exists); sulfuric where SO₂ reaches
/// [`SULFURIC_MIN_BAR`]; dust on a dry air under [`DUST_MAX_BAR`];
/// otherwise none.
///
/// # Panics
///
/// When `model` has no surface (a giant).
pub fn sky_of(model: &PlanetModel) -> Atmosphere {
    let surface = model.surface.as_ref().expect("a planet with a surface");
    let gases = &surface.gases;
    let pressure = gases.total();
    let column = pressure / model.interior.gravity.max(1e-6);
    let earth = Atmosphere::default();
    let mut sky = earth.clone();
    if !(EARTH_COLUMN.0..=EARTH_COLUMN.1).contains(&column) {
        let t = ((pressure / YELLOW_BAR.0).log10() / (YELLOW_BAR.1 / YELLOW_BAR.0).log10())
            .clamp(0.0, 1.0) as f32;
        let colour: [f32; 3] = std::array::from_fn(|i| {
            earth.rayleigh[i] + (CAUSTIC_RAYLEIGH[i] - earth.rayleigh[i]) * t
        });
        let brightest = colour.iter().copied().fold(0.0_f32, f32::max);
        let scale = (column as f32).min(MAX_RAYLEIGH / brightest);
        sky.rayleigh = colour.map(|c| c * scale);
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

/// The world parameters of a planet with a surface: `request` (its seed,
/// star, orbit, map size…) resolved with [`preset_for`]'s preset, carrying
/// what the model derived:
///
/// - its circumference, `2π R`;
/// - its surface pressure, the air's total (within
///   [`crate::request::PRESSURE_BAR`]);
/// - its erosion: 1 with liquid water, [`HYDROCARBON_EROSION`] with
///   hydrocarbons, none otherwise, times `min(1, P / 0.01 bar)`;
/// - its volcanism, `0.5 √(F / 0.087 W/m²)` of its heat flux `F` against
///   Earth's ([`EARTH_HEAT_FLUX_W_M2`]), 1 in the heat-pipe regime, halved
///   under a stagnant lid, within [`crate::request::VOLCANIC`];
/// - its surface age by regime: plates as the generator's default
///   ([`fallback::SURFACE_AGE_GA`]), episodic Venus's
///   ([`EPISODIC_SURFACE_AGE_GA`]), heat-pipe Io's
///   ([`HEAT_PIPE_SURFACE_AGE_GA`]), a stagnant lid most of the system's
///   age ([`STAGNANT_SURFACE_AGE_SHARE`]);
///
/// then [`finish`]es them: its gravity, regime, sea level and temperature.
///
/// # Panics
///
/// When `model` has no surface (a giant).
pub fn params_for(
    model: &PlanetModel,
    request: WorldRequest,
    rng: &mut impl RngExt,
) -> WorldParams {
    let surface = model.surface.as_ref().expect("a planet with a surface");
    let preset = preset_for(model);
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
    let limits = crate::request::VOLCANIC;
    let volcanic = (volcanic as f32).clamp(limits.min, limits.max);
    let age_gyr = request.age_gyr.unwrap_or(crate::star::DEFAULT_AGE_GYR);
    let surface_age_ga = match surface.regime {
        Regime::Plates => fallback::SURFACE_AGE_GA.pick(rng),
        Regime::Episodic => EPISODIC_SURFACE_AGE_GA.pick(rng),
        Regime::HeatPipe => HEAT_PIPE_SURFACE_AGE_GA.pick(rng),
        Regime::Stagnant => age_gyr * STAGNANT_SURFACE_AGE_SHARE.pick(rng),
    };
    let ages = crate::request::SURFACE_AGE_GA;
    let pressures = crate::request::PRESSURE_BAR;
    let request = WorldRequest {
        circumference_km: Some(
            (2.0 * std::f64::consts::PI * model.interior.radius_earth * crate::exomoon::EARTH_M
                / 1000.0) as f32,
        ),
        pressure_bar: Some((pressure as f32).clamp(pressures.min, pressures.max)),
        erosion: Some(erosion),
        volcanic: Some(volcanic),
        surface_age_ga: Some(surface_age_ga.clamp(ages.min, ages.max)),
        ..request
    };
    let mut params = match request.resolve_with_preset(preset.clone(), preset.name.clone(), rng) {
        Ok(resolved) => resolved.params,
        // An orbit or star the request refuses (its own limits): resolved
        // without them, then put back as they are.
        Err(_) => {
            let bare = WorldRequest {
                star: None,
                orbit_au: None,
                eccentricity: None,
                age_gyr: None,
                metallicity: None,
                ..request.clone()
            };
            let mut params = bare
                .resolve_with_preset(preset.clone(), preset.name.clone(), rng)
                .expect("a derived preset resolves without an orbit")
                .params;
            if let Some(star) = &request.star {
                params.star = crate::star::parse_type(star).unwrap_or(params.star);
            }
            params.age_gyr = request.age_gyr.unwrap_or(params.age_gyr);
            params.metallicity = request.metallicity.unwrap_or(params.metallicity);
            params.orbit_au = request.orbit_au.unwrap_or(params.orbit_au);
            params.eccentricity = request.eccentricity.unwrap_or(params.eccentricity);
            params
        }
    };
    finish(&mut params, model);
    params
}

/// Completes the parameters of a planet with a surface, in this order
/// (each step reads the ones before): its gravity, the model's (and the
/// tilt of a planet the star's tides despin at that mass, none); its
/// regime, the model's (not [`crate::geology::regime_of`]'s); its sea
/// level, where its oceans ([`ocean_depth_m`]) fill the relief
/// ([`sea_level_for_water`]; a runaway's water is steam, none on the
/// ground), or for a hydrocarbon world, whose lakes the model does not
/// size, Titan's share of land ([`HYDROCARBON_LAND_FRACTION`]); then the
/// temperature shift that brings the world's mean to the model's
/// ([`warming_for`]).
fn finish(params: &mut WorldParams, model: &PlanetModel) {
    let surface = model.surface.as_ref().expect("a planet with a surface");
    params.gravity = Some(model.interior.gravity as f32);
    // The star's tides read the mass, which the gravity has just changed:
    // a planet they now despin stands upright (`crate::spin::upright`). One
    // they no longer despin keeps the tilt the resolution left it.
    params.axial_tilt_deg = crate::spin::upright(params, params.axial_tilt_deg);
    params.planet.geology.regime = surface.regime;
    params.sea_level = if surface.climate.liquid == SurfaceLiquid::Hydrocarbons {
        sea_level_for_land_fraction(params, HYDROCARBON_LAND_FRACTION)
    } else {
        let oceans = if surface.water.runaway {
            0.0
        } else {
            surface.water.oceans
        };
        sea_level_for_water(params, ocean_depth_m(oceans, model.interior.radius_earth))
    };
    params.planet.climate.temperature = warming_for(params, surface.climate.temperature_k);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::planet_model;
    use crate::planet::reference::{model_inputs, reference, sun_at};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// A request for a world at Earth's orbit round the Sun, 128×64.
    fn earth_request() -> WorldRequest {
        WorldRequest {
            seed: Some(1),
            star: Some("G2V".into()),
            age_gyr: Some(4.6),
            orbit_au: Some(1.0),
            eccentricity: Some(0.0167),
            metallicity: Some(0.0),
            moons: Some(crate::moons::MoonsMode::Auto),
            giants: Some(crate::giants::GiantsMode::None),
            host: Some(crate::exomoon::HostMode::None),
            width: Some(128),
            height: Some(64),
            ..WorldRequest::default()
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
    }
    #[test]
    fn a_lifeless_earth_has_no_forest() {
        let mut earth = model_of("Earth");
        earth.surface.as_mut().unwrap().life.arose = false;
        let p = preset_for(&earth);
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
        let back: Preset = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        for b in Biome::ALL {
            assert_eq!(back.remap(b), p.remap(b), "{b:?}");
        }
    }

    #[test]
    fn a_living_earth_keeps_its_forests() {
        let earth = model_of("Earth");
        assert!(earth.surface.unwrap().life.arose);
        let p = preset_for(&earth);
        assert_eq!(p.remap(Biome::Forest), Biome::Forest);
        assert_eq!(p.name, "rocky");
        assert!(p.derived);
    }

    #[test]
    fn earths_sky_is_earths() {
        assert_eq!(sky_of(&model_of("Earth")), Atmosphere::default());
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
            earth_request(),
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
        assert_eq!(params.planet.geology.regime, Regime::Plates);
        // land share and mean temperature checked by Tasks 3 and 4
    }
}
