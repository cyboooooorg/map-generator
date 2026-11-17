//! Transitional mapping of a [`PlanetModel`] onto today's looks, so that
//! a planet the model describes renders with the rules, palette and remap
//! of the built-in look closest to it. Deleted by #424, when the model
//! alone draws a planet.
use rand::RngExt;

use super::albedo::{EARTH_CLOUD_COVER, cover, sulphuric_deck};
use super::classify::ICE_WORLD_MIN_OCEANS;
use super::surface_age::Renewal;
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
///    as an ice giant); a steam world: `steam-world`, a grey cloud deck
///    (a planet drawn with the giants' deck renderer, see
///    [`renders_as_giant`]);
/// 2. a lava world, a lava liquid, or a heat-pipe regime without liquid
///    water: `volcanic` (a heat-pipe body under a water ocean takes rule 5:
///    the ocean covers its volcanism, which the world's volcanic intensity
///    keeps visible; one whose crust is an ice shell, [`cryovolcanic`],
///    takes the rules below: no lava crosses its shell);
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
        PlanetClass::SteamWorld => return "steam-world",
        _ => {}
    }
    let surface = model.surface.as_ref();
    let liquid = surface.map_or(SurfaceLiquid::None, |s| s.climate.liquid);
    if model.class == PlanetClass::LavaWorld
        || liquid == SurfaceLiquid::Lava
        || (liquid != SurfaceLiquid::Water
            && surface.is_some_and(|s| s.regime == Regime::HeatPipe)
            && !cryovolcanic(model))
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

/// Whether the volcanism of `model`'s surface is cryovolcanism: its
/// crust is an ice shell (`crate::generation::model_crust_kind` is
/// ice, and the model derives its [`super::IceShell`]) over its rock.
/// Its rock may be in any regime, the heat pipe of a tidally heated one
/// included (`super::regime::regime` reads the heat flux alone), but no
/// silicate magma reaches its surface: about three times as dense as ice,
/// it cannot rise through the water layer and its shell, and what erupts
/// is water from the ocean or from melt in the shell, frozen into fresh
/// ice (Kargel 1995, doi:10.1007/BF00613296; Fagents 2003,
/// doi:10.1029/2003JE002128). Its look draws its volcanic zones as such
/// ([`crate::biome::BiomeRules::cryovolcanic`], see [`look_for`]). A
/// frozen ocean over a rocky crust (a snowball Earth) is no ice shell
/// crust: its volcanoes stand on rock.
pub fn cryovolcanic(model: &PlanetModel) -> bool {
    model.ice_shell.is_some()
        && crate::generation::model_crust_kind(model) == crate::generation::CrustKind::Ice
}

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// Earth's mean surface heat flux, W/m²: 44 TW over its surface (Pollack,
/// Hurter & Johnson 1993, doi:10.1029/93RG01249, 87 mW/m²). The flux at
/// which a world is as volcanic as the scale's middle. It is the whole
/// flux, radiogenic heat and secular cooling ([`super::secular_w`]),
/// as the model's [`super::InternalHeat::flux_w_m2`] is: Earth's model
/// gives about 0.090 W/m², a volcanism of about 0.5.
pub(super) const EARTH_HEAT_FLUX_W_M2: f64 = 0.087;

/// Volcanism of a world with Earth's heat flux, on the `[0, 1]` scale of
/// [`crate::request::VOLCANIC`]. Calibration, no source: the middle of
/// the range worlds draw from ([`fallback::VOLCANIC`]).
pub(super) const EARTH_VOLCANIC: f64 = 0.5;

/// Volcanism of a body with the internal heat flux `flux_w_m2` in
/// `regime`, before the world's range clamps it: `0.5 √(F / 0.087 W/m²)`
/// ([`EARTH_VOLCANIC`], [`EARTH_HEAT_FLUX_W_M2`]), 1 in the heat-pipe
/// regime, halved under a stagnant lid. The climate reads it too, as the
/// rate at which volcanoes give a snowball its CO₂
/// ([`super::climate::snowball_chance`]).
pub(super) fn volcanism(flux_w_m2: f64, regime: Regime) -> f64 {
    match regime {
        Regime::HeatPipe => 1.0,
        regime => {
            let v = EARTH_VOLCANIC * (flux_w_m2.max(0.0) / EARTH_HEAT_FLUX_W_M2).sqrt();
            if regime == Regime::Stagnant {
                0.5 * v
            } else {
                v
            }
        }
    }
}

/// Surface pressure under which water and hydrocarbons carve less, bar:
/// [`crate::atmosphere::LIQUID_WATER_BAR`], under which the generator
/// keeps no liquid. The erosion scales as `min(1, P / 0.01)`.
pub(super) const ERODING_BAR: f64 = crate::atmosphere::LIQUID_WATER_BAR as f64;

/// Erosion of a world with liquid hydrocarbons against one with liquid
/// water. Calibration, no source: Titan's river networks are fewer and
/// fainter than Earth's; half.
const HYDROCARBON_EROSION: f32 = 0.5;

/// Age of the surface of a world in the episodic regime, Ga: Venus's,
/// 0.3 to 0.8 Ga from its craters (Strom, Schaber & Dawson 1994,
/// *JGR* 99, about 0.3 Ga; McKinnon et al. 1997, *Venus II*, about
/// 0.75 Ga).
pub(super) const EPISODIC_SURFACE_AGE_GA: ParamDefault = ParamDefault::Range([0.3, 0.8]);

/// Age of the surface of a world in the heat-pipe regime, Ga: Io's, which
/// shows no impact crater, so that its volcanoes resurface it faster than
/// craters form; younger than about a million years (Johnson et al. 1979,
/// *Nature* 280: about 1 cm of lava a year).
pub(super) const HEAT_PIPE_SURFACE_AGE_GA: ParamDefault = ParamDefault::Range([0.0, 0.001]);

/// Share of the system's age a stagnant lid's surface keeps, a range:
/// Mars's oldest terrains are about 4.1 Ga old and Mercury's about 4.0
/// (Fassett 2016, *JGR Planets* 121), 0.7 to 0.95 of 4.57 Ga
/// allowing for younger lids.
pub(super) const STAGNANT_SURFACE_AGE_SHARE: ParamDefault = ParamDefault::Range([0.7, 0.95]);

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
/// Pale cream-yellow: Venus in visible light, about (0.93, 0.89, 0.76) in
/// sRGB, (0.85, 0.77, 0.54) in the linear light the viewer's sky takes
/// (its deck reflects blue less than red, Bond albedo 0.76; Pérez-Hoyos et
/// al. 2018, JGR Planets 123, 145, doi:10.1002/2017JE005406, and the
/// Mariner 10 and MESSENGER true-colour composites; from memory, check).
const CAUSTIC_HAZE_COLOUR: [f32; 3] = [0.85, 0.77, 0.54];

/// Pressure (bar) within which the sky is Earth's, a tenth either way:
/// Earth's air renders as Earth's. Calibration, no source.
const EARTH_BAR: (f64, f64) = (0.8, 1.25);

/// Methane haze: CH₄ share of the air and surface pressure (bar) over
/// which it forms, its density and Titan's orange. Calibration, no source
/// for the thresholds: Titan (5 % CH₄ near the surface, 1.5 bar) has an
/// opaque orange haze, photochemical organics (tholins) made from its
/// methane by sunlight high in its air.
///
/// Its density and colour are a calibration with Titan's Cassini
/// true-colour images as their target: from space the haze hides the
/// surface in visible light and the disc is an even orange-brown, with a
/// thin bluish layer at the limb (Porco et al. 2005,
/// doi:10.1038/nature03436). A density over 0.5 is a haze that hides the
/// ground to the viewer (`web/src/lib/sky.ts`, `hazeDepth`: 0.9 is a
/// zenith optical depth of about 6.6, Huygens measured several in the
/// visible, Tomasko et al. 2005, doi:10.1038/nature04126); the colour is
/// the share of the light it scatters per channel, the rest absorbed: the
/// haze absorbs blue most (Tomasko et al. 2008,
/// doi:10.1016/j.pss.2007.11.019), which makes Titan orange.
const HAZE_CH4_SHARE: f64 = 0.01;
const HAZE_MIN_BAR: f64 = 0.5;
const METHANE_HAZE: f32 = 0.9;
const METHANE_HAZE_COLOUR: [f32; 3] = [0.95, 0.55, 0.15];

/// Partial pressures over which clouds form, bar. Calibration, no source:
/// water where its vapour reaches 1e-3 bar (Earth's air holds about
/// 1e-2). The sulphuric deck is the albedo's
/// ([`super::albedo::sulphuric_deck`], 1e-3 bar of SO₂ in a warm air: the
/// model gives Venus 1.7e-2 bar, Mars 3e-6). The model's Venus keeps no
/// water vapour (its runaway lost it; the real one has 30 ppm), so the
/// acid deck does not ask for any.
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

/// The liquids of a hydrocarbon world (target, sources), applied after
/// its base remap: its seas and lakes are methane and ethane, never
/// frozen water, whatever its `frozen` base look makes of them. Every
/// open water pixel of such a world is that liquid: its sea level is set
/// by its share of liquid ([`HYDROCARBON_LAND_FRACTION`]), not by water.
const HYDROCARBON_LIQUIDS: [(Biome, &[Biome]); 2] = [
    (
        Biome::HydrocarbonSea,
        &[
            Biome::FrozenOcean,
            Biome::DeepOcean,
            Biome::Ocean,
            Biome::SeaIce,
        ],
    ),
    (Biome::HydrocarbonLake, &[Biome::FrozenLake, Biome::Lake]),
];

/// The look a planet with a surface renders with: its [`base_of`] look's
/// rules, palette and remap with what the model derived: no temperature
/// shift nor greenhouse ([`calibrate`] aims the world at the model's
/// temperature), the internal heat `(F / σ)^¼` of its heat flux `F` (the
/// temperature it alone would keep the surface at), its regime, its sky
/// ([`sky_of`]), the hydrocarbon seas and lakes of a world whose liquid is
/// methane ([`HYDROCARBON_LIQUIDS`]), and the lifeless remap after the
/// base one when life did not arise ([`LIFELESS`]). Its orbit comes from
/// the system and its sea level from its water.
///
/// A [`cryovolcanic`] body draws its volcanic zones as fresh ice
/// ([`crate::biome::BiomeRules::cryovolcanic`]); in the heat-pipe regime
/// its zones spread as an Io's do, with the `volcanic` look's shift
/// ([`IO_LIKE`]): water erupts and floods its lows as fast as lava
/// resurfaces Io (its surface is as young, [`HEAT_PIPE_SURFACE_AGE_GA`]),
/// and fresh flows cover the cores of its volcanic provinces
/// ([`CRYOVOLCANIC_VENT_ZONE`]).
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
    if cryovolcanic(model) {
        look.rules.cryovolcanic = true;
        if surface.regime == Regime::HeatPipe {
            look.climate.volcanic = Look::load(IO_LIKE).climate.volcanic;
            look.rules.vent_zone = CRYOVOLCANIC_VENT_ZONE;
        }
    }
    look.atmosphere = sky_of(model);
    look.atmosphere.greenhouse = 0.0;
    look.giant = None;
    if surface.climate.liquid == SurfaceLiquid::Hydrocarbons {
        look = look.then_remap(&HYDROCARBON_LIQUIDS);
    }
    if surface.life.arose {
        look
    } else {
        look.then_remap(&LIFELESS)
    }
}

/// The look whose volcanic shift a [`cryovolcanic`] body in the heat-pipe
/// regime takes ([`look_for`]): Io's, `volcanic`, whose shift (0.5)
/// turns every lowland into a volcanic zone.
const IO_LIKE: &str = "volcanic";

/// Volcanic zone past which a [`cryovolcanic`] body in the heat-pipe
/// regime draws fresh ice flows ([`crate::biome::BiomeRules::vent_zone`]),
/// with [`IO_LIKE`]'s shift of 0.5: the cones of its volcanic provinces
/// and their cores, where their background zone (the province's strength
/// times `generation::VOLCANIC_BACKGROUND`, 0.12) passes 0.05, about
/// 15 % of the surface at a volcanic intensity of 1 (14 to 15 % on the
/// hot icy moons of the tests at 256 × 128). Calibration,
/// no source for the value: between Enceladus, whose active south polar
/// terrain is about a tenth of its surface (Porco et al. 2006,
/// doi:10.1126/science.1123013), and an Io, resurfaced everywhere; Io's
/// own 0.9 leaves the flows to the cones' summits, a few pixels.
const CRYOVOLCANIC_VENT_ZONE: f32 = 0.55;

/// What a built-in look draws when the model leaves it open (the
/// `[defaults]` of the former preset files): its pressure, surface age,
/// tilt and erosion, each a value or a range, `None` for the generator's
/// [`fallback`]. Calibration, no source: the values the presets drew
/// from, which keep Venus's air dense, a frozen world's thin, a giant's
/// (and a steam world's) sky thin above its cloud tops and the tilts of the ice giants
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
        // A steam world shows its cloud deck as a giant does (a thin sky
        // above the cloud tops, no surface age: no ground is seen), but
        // it is a planet: its tilt is drawn as a planet's.
        "steam-world" => giant(None),
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
/// sulfuric where the albedo makes a deck ([`sulphuric_deck`] of the
/// stored SO₂, washed out as the albedo washed it: SO₂ in a warm air; a
/// frozen world's makes none, and a wet body's air has washed it out);
/// methane where hydrocarbons are liquid (methane condensed into seas,
/// [`super::climate`]); water where water is liquid or its vapour
/// reaches [`WATER_CLOUD_MIN_BAR`]; dust on a dry air under
/// [`DUST_MAX_BAR`]; otherwise none. The cover follows the albedo's,
/// relative to Earth's: a water cloud cover ([`cover`]) or the deck's
/// share of the disc, total once the deck is whole.
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
    let hydrocarbons = surface.climate.liquid == SurfaceLiquid::Hydrocarbons;
    // The stored air is washed already (`climate::washed`), as the
    // albedo saw it.
    let deck = sulphuric_deck(surface.climate.temperature_k, gases.so2);
    sky.clouds = if pressure < AIRLESS_BAR as f64 {
        CloudKind::None
    } else if deck > 0.0 {
        CloudKind::Sulfuric
    } else if hydrocarbons {
        CloudKind::Methane
    } else if surface.climate.liquid == SurfaceLiquid::Water || gases.h2o >= WATER_CLOUD_MIN_BAR {
        CloudKind::Water
    } else if pressure < DUST_MAX_BAR {
        CloudKind::Dust
    } else {
        CloudKind::None
    };
    // A cover of the disc on the look's scale: Earth's two-thirds the
    // zero, none -1, overcast 1.
    let shift = |cloud: f64| {
        let earth = EARTH_CLOUD_COVER;
        let shift = if cloud < earth {
            (cloud - earth) / earth
        } else {
            (cloud - earth) / (1.0 - earth)
        };
        shift.clamp(-1.0, 1.0) as f32
    };
    sky.cloud_cover = match sky.clouds {
        // The deck covers what the albedo's does: all of the disc once
        // whole.
        CloudKind::Sulfuric => shift(deck),
        // Water clouds follow the vapour (the albedo's cover).
        CloudKind::Water => shift(
            cover(
                surface.climate.temperature_k,
                gases,
                &surface.water,
                &super::Gases::default(),
            )
            .cloud,
        ),
        _ => 0.0,
    };
    sky
}

/// Whether the world of `model` renders as a giant's: its [`base_of`]
/// look is a giant's (a gas giant, a hot Jupiter, an ice giant or a
/// mini-Neptune) or a steam world's grey cloud deck. Only the look: a
/// steam world is a planet ([`PlanetClass::is_giant`] is false), whose
/// moons, tides, spin, neighbours and cards are a planet's.
pub fn renders_as_giant(model: &PlanetModel) -> bool {
    matches!(
        base_of(model),
        "gas-giant" | "hot-jupiter" | "ice-giant" | "steam-world"
    )
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
///     system's age ([`STAGNANT_SURFACE_AGE_SHARE`]), an ice lid less the
///     more heat it lets out ([`super::surface_age::ICE_LID_HALF_FLUX_W_M2`]: Europa's is
///     young, Callisto's old);
///   - its gravity, the model's (the star's tides, the moons, the tilt
///     they leave it and its tidal flats see the model's mass);
///   - its regime, the model's (its look's, see [`look_for`]), not
///     [`crate::geology::regime_of`]'s;
/// - a giant ([`PlanetClass::is_giant`]): its giant look at the model's
///   mass, a mini-Neptune's too, however light (issue #446), its radius
///   the giant look's (`giant_radius_of` for the model's composition and
///   light, the model's own radius), no ground, no giant neighbours and
///   nobody's moon;
/// - a steam world: the `steam-world` look, a giant's cloud deck at the
///   model's radius (no ground, a giant's thin render sky and no surface
///   age), but a planet otherwise: no `giant_mass_earth`, the model's
///   gravity, a planet's tilt draw, moons, tides and spin, its giant
///   neighbours and host as asked;
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
    use crate::request::{PRESSURE_BAR, SEA_LEVEL, SURFACE_AGE_GA, VOLCANIC};
    let base = base_of(model);
    // Drawn as a cloud deck (a giant or a steam world), and a giant: a
    // steam world is a planet, only its look is a giant's deck.
    let deck = renders_as_giant(model);
    let giant = model.class.is_giant();
    let surface = model.surface.as_ref().filter(|_| !deck);
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
    let main = match model.terrain.main() {
        Some(unit) => *unit,
        None => *super::surface_age::terrain(
            model,
            f64::from(placed.age_gyr),
            &super::Gases::default(),
            None,
        )
        .units
        .first()
        .expect("a terrain has a unit"),
    };
    let regime_age = match (main.renewal, surface.map(|s| s.regime)) {
        (Renewal::LiquidCycle, _) => {
            main.age_gyr as f32 * super::surface_age::LIQUID_CYCLE_AGE_SHARE.pick(rng)
        }
        (Renewal::VolatileIce, _) => {
            main.age_gyr as f32 * super::surface_age::VOLATILE_ICE_AGE_SHARE.pick(rng)
        }
        (_, Some(Regime::Plates) | None) => fallback::SURFACE_AGE_GA.pick(rng),
        (_, Some(Regime::Episodic)) => EPISODIC_SURFACE_AGE_GA.pick(rng),
        (_, Some(Regime::HeatPipe)) => HEAT_PIPE_SURFACE_AGE_GA.pick(rng),
        (_, Some(Regime::Stagnant)) => main.age_gyr as f32 * STAGNANT_SURFACE_AGE_SHARE.pick(rng),
    };
    let radius_km = (model.interior.radius_earth * crate::exomoon::EARTH_M / 1000.0) as f32;
    let model_circumference = 2.0 * std::f32::consts::PI * radius_km;
    let star = crate::star::StarInfo::new(
        placed.star,
        placed.orbit_au,
        placed.eccentricity,
        placed.age_gyr,
    );
    // The model's own mass, a light mini-Neptune's too, so that the body,
    // its world and the moons drawn from it agree (issue #446).
    let giant_mass = model.inputs.mass_earth as f32;
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
                let volcanic = volcanism(flux, surface.regime);
                (
                    sea_level,
                    model_circumference,
                    (pressure as f32).clamp(PRESSURE_BAR.min, PRESSURE_BAR.max),
                    erosion,
                    volcanic as f32,
                    regime_age,
                )
            }
            None if deck => (
                sea_level,
                // A steam world keeps the model's radius, the steam's
                // inflation included, not a giant law's.
                if model.class == PlanetClass::SteamWorld {
                    model_circumference
                } else {
                    crate::giants::giant_radius_of(
                        giant_mass,
                        &crate::giants::GiantComposition::of(&model.inputs),
                        &super::Exposure::of(&star),
                    ) * 40_075.0
                },
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
        asked_moons: Vec::new(),
        moon_sizes: Vec::new(),
        // Hydrocarbons gather near the poles, whatever the world.
        liquid_poleward_deg: surface
            .is_some_and(|s| s.climate.liquid == SurfaceLiquid::Hydrocarbons)
            .then_some(POLAR_LIQUID_LAT_DEG),
        ridge: None,
        // A giant's gravity follows from its own mass (a steam world's is its
        // model's, a planet's).
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
/// runaway's water is steam, none on the ground; an ice crust's lies under
/// its shell, none on the ground either, see `under_shell`; with no water
/// at all the
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
        // A runaway's water is steam; an ice crust's lies under its shell.
        let oceans = if surface.water.runaway || under_shell(params, model) {
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

/// Whether the water of the world `params` of `model` lies under its
/// surface, an ice shell over it ([`super::IceShell`]): its crust is ice
/// ([`crate::generation::icy`]: an ice world's share of water, and no
/// liquid water at its surface) and the model gives it a shell (a frozen
/// surface over its water layer). That ice is then its crust, the relief
/// the world draws with its regime's highlands and craters (Europa,
/// Ganymede, Callisto, Enceladus), not a frozen sea over a rocky floor:
/// no sea. Water under a shell is no surface ocean, liquid or frozen. A
/// frozen sea on a rocky planet (a snowball Earth, a cold Mars that kept
/// its water: under an ice world's share of water, a rock crust) still
/// fills its basins, and so does liquid water (no shell); hydrocarbons
/// are sized apart.
fn under_shell(params: &WorldParams, model: &PlanetModel) -> bool {
    model.ice_shell.is_some() && crate::generation::icy(params)
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
    fn a_heat_pipe_ocean_world_renders_as_water() {
        // Earth's model under strong tidal heating (mgs-81447-3145862552 b:
        // a super-Earth with 2390 Earth oceans at 287 K in the heat-pipe
        // regime): the ocean covers the volcanism, it is not a lava look.
        let mut ocean = model_of("Earth");
        ocean.surface.as_mut().unwrap().regime = Regime::HeatPipe;
        assert_eq!(
            ocean.surface.as_ref().unwrap().climate.liquid,
            SurfaceLiquid::Water
        );
        assert_eq!(base_of(&ocean), "terran");
        // Io-like: a heat pipe without surface water stays volcanic.
        let mut io = model_of("Mars");
        io.surface.as_mut().unwrap().regime = Regime::HeatPipe;
        assert_eq!(base_of(&io), "volcanic");
        // A water liquid never overrides a lava world.
        let mut lava = model_of("Earth");
        lava.class = PlanetClass::LavaWorld;
        assert_eq!(base_of(&lava), "volcanic");
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

    /// The steam world of #492: 1679 K, 409 942 bar of steam, 1.700 R⊕
    /// at 0.018 AU of an M9V.
    fn steam_model() -> PlanetModel {
        use crate::planet::inputs::volatile_rule;
        use crate::planet::{Context, reference};
        use crate::star::{DEFAULT_AGE_GYR, StarInfo, parse_type};
        let (iron, rock, water) = (1.06, 3.16, 0.56);
        let mass = iron + rock + water;
        let inputs = crate::planet::PlanetInputs {
            volatiles: volatile_rule(1.0 - water / mass, water / mass, iron / (iron + rock)),
            ..reference::inputs(mass, iron / (iron + rock), water / mass)
        };
        let star = StarInfo::new(parse_type("M9V").unwrap(), 0.018, 0.0, DEFAULT_AGE_GYR);
        planet_model(&Context::of(&star, 0.0, 0.1), inputs)
    }

    #[test]
    fn a_steam_world_renders_as_grey_clouds() {
        use crate::star::parse_type;
        let model = steam_model();
        assert_eq!(model.class, PlanetClass::SteamWorld);
        assert_eq!(base_of(&model), "steam-world");
        assert!(renders_as_giant(&model));
        let mut placed = earth_placed(1);
        placed.star = parse_type("M9V").unwrap();
        placed.orbit_au = 0.018;
        placed.eccentricity = 0.0;
        let params = params_for(&model, &placed, &mut StdRng::seed_from_u64(1));
        let radius_km = model.interior.radius_earth * crate::exomoon::EARTH_M / 1000.0;
        let expect = 2.0 * std::f64::consts::PI * radius_km;
        assert!(
            (f64::from(params.circumference_km) - expect).abs() < 1e-3 * expect,
            "{} vs {expect}",
            params.circumference_km
        );
        assert_eq!(params.class, "steam-world");
        let stored = params.model.as_ref().unwrap();
        assert!(stored.surface.as_ref().unwrap().gases.total() > 1e5);
        assert!(params.look.giant.is_some());
        // Drawn as a deck, but a planet: no giant mass, its model's
        // gravity, a giant's thin render sky, no surface age.
        assert!(params.is_steam_world() && params.renders_as_deck());
        assert_eq!(params.giant_mass_earth, None);
        assert_eq!(
            params.deck_mass_earth(),
            Some(model.inputs.mass_earth as f32)
        );
        assert_eq!(params.gravity, Some(model.interior.gravity as f32));
        assert_eq!(params.pressure_bar, 0.3);
        assert_eq!(params.surface_age_ga, crate::request::SURFACE_AGE_GA.min);
        // `world.json` tells no giant, only the deck it is drawn with.
        assert!(crate::giant_world::info(&params).is_none());
        let deck = crate::giant_world::deck(&params).expect("a cloud deck");
        assert_eq!(deck.kind, crate::giants::GiantKind::Steam);
        assert_eq!(
            deck.rotation_hours,
            crate::generation::sidereal_day_hours(&params) as f32
        );
    }

    #[test]
    fn a_steam_world_keeps_a_planets_moons_spin_and_neighbours() {
        // The steam world of #492 placed round the Sun at 1 AU (its moons
        // and spin are free there): its moons, spin and tides are those of
        // a rocky planet of its mass, radius and gravity, its neighbours
        // and host as asked.
        let model = steam_model();
        let mut with_moons = 0;
        for seed in 1..=200 {
            let mut placed = earth_placed(seed);
            placed.giants = crate::giants::GiantsMode::Auto;
            placed.host = crate::exomoon::HostMode::Auto;
            let params = params_for(&model, &placed, &mut StdRng::seed_from_u64(u64::from(seed)));
            assert_eq!(params.giants, crate::giants::GiantsMode::Auto);
            assert_eq!(params.host, crate::exomoon::HostMode::Auto);
            let mut rocky = params.clone();
            // The same world without its deck: a rocky planet.
            rocky.look.giant = None;
            assert!(!rocky.renders_as_deck());
            let moons = crate::moons::MoonSystem::draw(&params);
            assert_eq!(moons, crate::moons::MoonSystem::draw(&rocky), "seed {seed}");
            assert_eq!(
                crate::spin::star_info(&params),
                crate::spin::star_info(&rocky),
                "seed {seed}"
            );
            assert_eq!(
                crate::generation::sidereal_day_hours(&params),
                crate::generation::sidereal_day_hours(&rocky),
                "seed {seed}"
            );
            with_moons += usize::from(moons.is_some_and(|m| !m.moons.is_empty()));
        }
        assert!(with_moons > 0, "no seed drew moons");
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
        let sky = sky_of(&model_of("Earth"));
        assert!(sky.cloud_cover.abs() < 0.05, "{}", sky.cloud_cover);
        assert_eq!(
            Atmosphere {
                cloud_cover: 0.0,
                ..sky
            },
            Atmosphere::default()
        );
    }

    /// Earth's inputs at `au` round the Sun with the snowball draw
    /// `draw` (0: a snowball wherever the climate has a temperate state).
    fn earth_model_at(au: f64, draw: f64) -> PlanetModel {
        let inputs = crate::planet::PlanetInputs {
            snowball_draw: draw,
            ..model_inputs(reference("Earth"))
        };
        planet_model(&sun_at(au), inputs)
    }

    #[test]
    fn titans_clouds_are_methane() {
        assert_eq!(sky_of(&model_of("Titan")).clouds, CloudKind::Methane);
    }

    #[test]
    fn a_frozen_earth_has_no_acid_cloud() {
        for draw in [0.0, 1.0] {
            let model = earth_model_at(1.8, draw);
            assert_ne!(sky_of(&model).clouds, CloudKind::Sulfuric, "{draw}");
        }
    }

    /// A snowball and a frozen water world past the outer edge render the
    /// same: the frozen base, a frozen ocean and the relief of the land.
    #[test]
    fn a_snowball_renders_frozen() {
        use crate::biome::Biome;
        for (au, draw) in [(1.2, 0.0), (2.0, 1.0)] {
            let model = earth_model_at(au, draw);
            assert_eq!(base_of(&model), "frozen", "{au} AU");
            let mut placed = earth_placed(1);
            (placed.width, placed.height, placed.orbit_au) = (256, 128, au as f32);
            let mut params = params_for(&model, &placed, &mut StdRng::seed_from_u64(1));
            calibrate(&mut params, &model);
            let world = crate::generation::generate(&params).world;
            let has = |wanted: &[Biome]| world.biomes.iter().any(|b| wanted.contains(b));
            assert!(has(&[Biome::FrozenOcean]), "{au} AU: no frozen ocean");
            assert!(
                has(&[
                    Biome::Mountain,
                    Biome::Snow,
                    Biome::IceCap,
                    Biome::GlacialPlain
                ]),
                "{au} AU: no frozen land"
            );
        }
    }

    #[test]
    fn a_dense_air_keeps_a_colour_not_a_column() {
        let sky = sky_of(&model_of("Venus"));
        let brightest = sky.rayleigh.iter().copied().fold(0.0_f32, f32::max);
        assert!(brightest <= 1.0, "{:?}", sky.rayleigh);
        assert!(sky.haze >= 0.25, "{}", sky.haze);
    }

    /// Titan's air (N₂ with 5 % CH₄, 1.5 bar) gets the methane haze, thick
    /// enough to hide its ground to the viewer (over 0.5, see
    /// `METHANE_HAZE`), orange (red over green over blue); Earth, Venus
    /// and Mars stay under that density.
    #[test]
    fn titans_haze_is_thick_and_orange() {
        let sky = sky_of(&model_of("Titan"));
        assert_eq!(sky.haze, METHANE_HAZE);
        assert!(sky.haze > 0.5);
        let [r, g, b] = sky.haze_colour;
        assert!(r > g && g > 2.0 * b, "{:?}", sky.haze_colour);
        for name in ["Earth", "Venus", "Mars"] {
            let haze = sky_of(&model_of(name)).haze;
            assert!(haze <= 0.5, "{name}: {haze}");
        }
    }

    /// A hydrocarbon world's seas and lakes are its own dark liquid, never
    /// the frozen water of its `frozen` base look; a water world keeps
    /// its water.
    #[test]
    fn titans_seas_are_hydrocarbons() {
        let titan = look_for(&model_of("Titan"));
        for b in [Biome::DeepOcean, Biome::Ocean, Biome::SeaIce] {
            assert_eq!(titan.remap(b), Biome::HydrocarbonSea, "{b:?}");
        }
        assert_eq!(titan.remap(Biome::Lake), Biome::HydrocarbonLake);
        let earth = look_for(&model_of("Earth"));
        assert_eq!(earth.remap(Biome::Ocean), Biome::Ocean);
        assert_eq!(earth.remap(Biome::Lake), Biome::Lake);
    }

    #[test]
    fn venus_has_a_sulfuric_deck_and_mars_dust() {
        let venus = sky_of(&model_of("Venus"));
        assert_eq!(venus.clouds, CloudKind::Sulfuric);
        assert!(venus.cloud_cover > 0.5, "{}", venus.cloud_cover);
        assert_eq!(sky_of(&model_of("Mars")).clouds, CloudKind::Dust);
    }

    /// The look draws a sulphuric deck where the albedo has one: a dry
    /// warm air of 5 bar whose SO₂, washed once (what the model stores),
    /// is over the deck's threshold or under it.
    #[test]
    fn the_look_and_the_albedo_see_the_same_deck() {
        use super::super::Gases;
        use super::super::albedo::DECK_ALBEDO;
        use super::super::atmosphere::wash_out;
        use super::super::climate::derived_albedo;
        let mut model = model_of("Venus");
        for so2 in [1.5e-3, 3.0e-3] {
            let surface = model.surface.as_mut().unwrap();
            assert!(!surface.water.is_wet());
            surface.climate.temperature_k = 400.0;
            surface.climate.liquid = SurfaceLiquid::None;
            let air = Gases {
                n2: 5.0,
                so2,
                ..Gases::default()
            };
            let albedo = derived_albedo(400.0, &air, &surface.water, &Gases::default());
            surface.gases = wash_out(air, SurfaceLiquid::None);
            let sky = sky_of(&model);
            let albedo_deck = albedo == DECK_ALBEDO;
            let sky_deck = sky.clouds == CloudKind::Sulfuric && sky.cloud_cover == 1.0;
            assert_eq!(albedo_deck, sky_deck, "{so2}: {albedo} {sky:?}");
            assert_eq!(albedo_deck, so2 > 2.0e-3, "{so2}: {albedo}");
        }
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
        // Earth's 0.087-0.092 W/m² through `EARTH_VOLCANIC √(F / 0.087)`.
        assert!(
            (0.45..=0.55).contains(&params.volcanic_intensity),
            "{}",
            params.volcanic_intensity
        );
        // land share and mean temperature checked by Tasks 3 and 4
    }

    /// A moon of `mass_earth` with `ice_fraction` of ices, born in its
    /// giant's disk, on an orbit of `a_m` and `eccentricity` around a host
    /// of `host_kg`, its giant at `orbit_au` round a G2V star.
    fn icy_moon(
        orbit_au: f32,
        mass_earth: f64,
        ice_fraction: f64,
        host_kg: f64,
        a_m: f64,
        eccentricity: f64,
    ) -> PlanetModel {
        use crate::planet::inputs::{Context, HostTide, InputOverrides};
        use crate::planet::moon::{MoonBirth, MoonFormation, derive_moon_model};
        let star = crate::star::StarInfo::new(crate::star::SUN, orbit_au, 0.0, 4.57);
        let formation = MoonFormation {
            mass_earth,
            ice_fraction,
            birth: MoonBirth::Disk,
            tide: HostTide {
                host_kg,
                a_m,
                eccentricity,
                resonance: None,
            },
            capture_age_share: None,
        };
        derive_moon_model(
            1,
            "p1/m1",
            &Context::of(&star, 0.0, 2.7),
            &formation,
            &InputOverrides::default(),
        )
    }

    /// The moon e VII of mgs-84157-2691483901 (#550): Titan's size (2,673
    /// km), 55 % ices, 5e14 W of tidal heat from a Saturn-mass giant
    /// (6.2e26 kg) at 391,000 km, e = 0.027: 5.6 W/m², twice Io's flux,
    /// under a 24 bar N₂–CH₄ air with hydrocarbon lakes.
    fn hot_titan() -> PlanetModel {
        icy_moon(9.7, 0.023_85, 0.5516, 6.22e26, 3.914e8, 0.0272)
    }

    /// Europa (Jupiter, 671,000 km, 8 % ices) on an orbit five times as
    /// eccentric (0.05): 3e13 W of tides, over 1 W/m².
    fn hot_europa() -> PlanetModel {
        icy_moon(5.2, 0.008, 0.08, 1.898e27, 6.709e8, 0.05)
    }

    /// Enceladus (Saturn, 238,000 km, 40 % ices) on an orbit of e = 0.1.
    fn hot_enceladus() -> PlanetModel {
        icy_moon(9.54, 1.81e-5, 0.4, 5.683e26, 2.38e8, 0.1)
    }

    fn io() -> PlanetModel {
        icy_moon(5.2, 0.015, 0.0, 1.898e27, 4.217e8, 0.0041)
    }

    /// The generated world of `model`, a moon round the Sun at `orbit_au`.
    fn moon_world(model: &PlanetModel, orbit_au: f32, seed: u32) -> crate::world::World {
        let mut placed = earth_placed(seed);
        (placed.width, placed.height, placed.orbit_au) = (256, 128, orbit_au);
        placed.eccentricity = 0.05;
        placed.moons = crate::moons::MoonsMode::None;
        let mut params = params_for(model, &placed, &mut StdRng::seed_from_u64(seed.into()));
        calibrate(&mut params, model);
        crate::generation::generate(&params).world
    }

    /// Silicate volcanism: magma at the surface.
    const SILICATE: [Biome; 5] = [
        Biome::Volcano,
        Biome::LavaField,
        Biome::AshLand,
        Biome::MagmaSea,
        Biome::ScorchedWaste,
    ];

    /// An icy moon whose tides put it in the heat-pipe regime keeps that
    /// regime (its rock's), but its ice shell over an ocean takes no
    /// silicate magma: its volcanism is cryovolcanism (#550).
    #[test]
    fn a_hot_icy_ocean_moon_erupts_ice_not_lava() {
        for (name, model, au) in [
            ("hot Titan (e VII)", hot_titan(), 9.7),
            ("hot Europa", hot_europa(), 5.2),
            ("hot Enceladus", hot_enceladus(), 9.54),
        ] {
            let surface = model.surface.as_ref().unwrap();
            assert_eq!(surface.regime, Regime::HeatPipe, "{name}");
            let shell = model.ice_shell.expect("an ice shell");
            assert!(shell.ocean_km > 0.0, "{name}: {shell:?}");
            assert_ne!(base_of(&model), "volcanic", "{name}");
            let world = moon_world(&model, au, 1);
            let count = |wanted: Biome| world.biomes.iter().filter(|&&b| b == wanted).count();
            let lava: usize = SILICATE.iter().map(|&b| count(b)).sum();
            let (flows, plains) = (count(Biome::FreshIceFlow), count(Biome::CryovolcanicPlain));
            eprintln!(
                "{name}: {lava} silicate, {flows} fresh ice flow, {plains} cryovolcanic plain of {}",
                world.biomes.len()
            );
            assert_eq!(lava, 0, "{name}: {lava} silicate volcanic pixels");
            assert!(flows > 0, "{name}: no fresh ice flow");
            assert!(plains > 0, "{name}: no cryovolcanic plain");
            // Young: an Io's surface age.
            assert!(world.params.surface_age_ga < 0.01, "{name}");
        }
    }

    /// An icy ocean moon outside the heat pipe (Europa, its 35 mW/m² in
    /// the stagnant regime) draws no silicate volcano either, whatever
    /// volcanism its world is given: its zones are fresh ice.
    ///
    /// Its `frozen` look lowers the volcanic zone by 0.3, so that even at
    /// full intensity only the core of a cone in a full province passes
    /// the vent threshold: a pixel or none on a small map, by the seed's
    /// luck (#549's band-limited octaves took the last one of seed 1).
    /// The world is drawn a second time without that shift, so that its
    /// zones reach the vents whatever the seed, and every pixel of the
    /// ladder is checked.
    #[test]
    fn a_stagnant_icy_ocean_moon_has_no_silicate_volcano() {
        let model = icy_moon(5.2, 0.008, 0.08, 1.898e27, 6.709e8, 0.009);
        assert_eq!(model.surface.as_ref().unwrap().regime, Regime::Stagnant);
        assert!(cryovolcanic(&model));
        let look = look_for(&model);
        assert!(look.rules.cryovolcanic);
        let mut placed = earth_placed(1);
        (placed.width, placed.height, placed.orbit_au) = (256, 128, 5.2);
        placed.moons = crate::moons::MoonsMode::None;
        let mut params = params_for(&model, &placed, &mut StdRng::seed_from_u64(1));
        calibrate(&mut params, &model);
        params.volcanic_intensity = 1.0;
        for shift in [params.look.climate.volcanic, 0.0] {
            params.look.climate.volcanic = shift;
            let world = crate::generation::generate(&params).world;
            let lava = world.biomes.iter().filter(|b| SILICATE.contains(b)).count();
            assert_eq!(lava, 0, "shift {shift}: {lava} silicate volcanic pixels");
            let mut vents = 0;
            for (&biome, &vz) in world.biomes.iter().zip(&world.volcanic_zone) {
                if vz > look.rules.vent_zone && !biome.is_water() {
                    assert_eq!(biome, Biome::FreshIceFlow, "shift {shift}: zone {vz}");
                    vents += 1;
                }
            }
            if shift == 0.0 {
                assert!(vents > 0, "no volcanic zone reaches the vents");
            }
        }
    }

    /// Rock under a frozen sea (a snowball Earth) keeps its volcanoes:
    /// only an ice shell crust is cryovolcanic.
    #[test]
    fn only_an_ice_shell_crust_is_cryovolcanic() {
        assert!(!cryovolcanic(&earth_model_at(2.0, 1.0)));
        assert!(!cryovolcanic(&io()));
        assert!(!cryovolcanic(&model_of("Earth")));
        assert!(cryovolcanic(&hot_titan()));
        assert!(!look_for(&io()).rules.cryovolcanic);
    }

    /// Io, a heat pipe of rock with no ice, keeps its lava.
    #[test]
    fn io_keeps_its_lava() {
        let model = io();
        assert_eq!(base_of(&model), "volcanic");
        let world = moon_world(&model, 5.2, 1);
        let lava = world
            .biomes
            .iter()
            .filter(|b| matches!(b, Biome::Volcano | Biome::LavaField))
            .count();
        assert!(lava > 0, "Io has no lava");
    }
}
