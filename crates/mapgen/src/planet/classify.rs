//! What kind of planet the model gives (spec "Derived, in order", the
//! class): read off the derived mass, envelope, water and radius, never
//! fed back into them ([`classify`]).
use serde::{Deserialize, Serialize};

use super::climate::SurfaceLiquid;
use super::{GIANT_MIN_ENVELOPE, GIANT_MIN_MASS_EARTH, Interior, PlanetInputs, Surface};
use crate::geology::Regime;

/// The kind of a planet, from its derived quantities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum PlanetClass {
    Rocky,
    SuperEarth,
    IceWorld,
    MiniNeptune,
    IceGiant,
    GasGiant,
    HotJupiter,
    HotNeptune,
    LavaWorld,
    /// A solid surface in the heat-pipe regime (Io; Moore & Webb 2013,
    /// doi:10.1038/nature12424) that no earlier class of [`classify`] claims.
    VolcanicWorld,
    RunawayGreenhouse,
    HydrocarbonWorld,
    OceanWorld,
    /// A runaway whose water is a deep supercritical column hiding its
    /// surface ([`super::climate::hides_surface`]): a world of steam over
    /// rock, inflated by it, with no hydrogen and helium envelope (#492;
    /// Turbet et al. 2020, doi:10.1051/0004-6361/201937151).
    SteamWorld,
}

impl PlanetClass {
    /// The class as stored: `rocky`, `super-earth`, `ice-world`,
    /// `mini-neptune`, `ice-giant`, `gas-giant`, `hot-jupiter`,
    /// `hot-neptune`, `lava-world`, `volcanic-world`, `runaway-greenhouse`, `hydrocarbon-world`,
    /// `ocean-world`, `steam-world`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Rocky => "rocky",
            Self::SuperEarth => "super-earth",
            Self::IceWorld => "ice-world",
            Self::MiniNeptune => "mini-neptune",
            Self::IceGiant => "ice-giant",
            Self::GasGiant => "gas-giant",
            Self::HotJupiter => "hot-jupiter",
            Self::HotNeptune => "hot-neptune",
            Self::LavaWorld => "lava-world",
            Self::VolcanicWorld => "volcanic-world",
            Self::RunawayGreenhouse => "runaway-greenhouse",
            Self::HydrocarbonWorld => "hydrocarbon-world",
            Self::OceanWorld => "ocean-world",
            Self::SteamWorld => "steam-world",
        }
    }

    /// Whether the class is a giant: a gas giant, an ice giant, a hot
    /// Jupiter, a hot Neptune or a mini-Neptune, whose envelope hides any surface and
    /// whose world is generated as a giant's.
    pub fn is_giant(self) -> bool {
        matches!(
            self,
            Self::GasGiant
                | Self::IceGiant
                | Self::HotJupiter
                | Self::HotNeptune
                | Self::MiniNeptune
        )
    }

    /// Whether the class is a giant within a few days of its star: a hot
    /// Jupiter or a hot Neptune.
    pub fn is_hot_giant(self) -> bool {
        matches!(self, Self::HotJupiter | Self::HotNeptune)
    }

    /// Whether the class is a giant to the small bodies: a gas giant, an
    /// ice giant, a hot Jupiter or a hot Neptune, massive enough to shape belts, empty
    /// trojan points and stir impactors (see `crate::belts`). A
    /// mini-Neptune, a few Earth masses, is not, though its world is a
    /// giant's ([`Self::is_giant`]).
    pub fn sculpts_belts(self) -> bool {
        matches!(
            self,
            Self::GasGiant | Self::IceGiant | Self::HotJupiter | Self::HotNeptune
        )
    }
}

/// The keys of every [`PlanetClass`] ([`PlanetClass::key`]), in declaration
/// order: the types a planet of a system is stored as. The server and the
/// web read the same list.
pub const PLANET_CLASSES: [&str; 14] = [
    "rocky",
    "super-earth",
    "ice-world",
    "mini-neptune",
    "ice-giant",
    "gas-giant",
    "hot-jupiter",
    "hot-neptune",
    "lava-world",
    "volcanic-world",
    "runaway-greenhouse",
    "hydrocarbon-world",
    "ocean-world",
    "steam-world",
];

/// Least hydrogen and helium share of a gas giant; a giant with less is an
/// ice giant. Calibration, no source: it parts Jupiter and Saturn (about
/// 0.85 to 0.95 of their mass, Guillot 2005,
/// doi:10.1146/annurev.earth.33.092203.122639) from Uranus and Neptune
/// (about 0.1 to 0.2, Helled et al. 2011, doi:10.1088/0004-637X/726/1/15).
pub(crate) const GAS_GIANT_MIN_ENVELOPE: f64 = 0.5;

/// Longest orbital period of a hot Jupiter or a hot Neptune, days (Wright
/// et al. 2012, doi:10.1088/0004-637X/753/2/160: giants within 10 days of
/// their star, about 1 % of Sun-like stars; Neptune-mass planets that
/// close are rare, the "Neptune desert", Mazeh, Holczer & Faigler 2016,
/// doi:10.1051/0004-6361/201528065).
const HOT_GIANT_MAX_PERIOD_DAYS: f64 = 10.0;

/// Least envelope share that makes a planet above the radius valley a
/// mini-Neptune rather than a bare rock. Calibration, no source: an
/// envelope of 0.1 % of the mass lifts a 5 M⊕ core by about 0.15 R⊕ on the
/// fit of Lopez & Fortney (2014, doi:10.1088/0004-637X/792/1/1); a thinner
/// one is an atmosphere, not a gas layer.
const MINI_NEPTUNE_MIN_ENVELOPE: f64 = 1e-3;

/// Radius, Earth radii, above which an enveloped planet is a mini-Neptune
/// (Rogers 2015, doi:10.1088/0004-637X/801/1/41: most planets larger than
/// 1.6 R⊕ are not rocky).
const MINI_NEPTUNE_MIN_RADIUS: f64 = 1.6;

/// Least share of water and ices of an ice world. Calibration, no source:
/// Europa (about 8 %) is one, Earth (0.02 %) is not.
pub const ICE_WORLD_MIN_WATER: f64 = 0.05;

/// Earth oceans an ice world keeps at its surface, at least, and a cold dry
/// body keeps to render frozen. A little over twice Mars's water: its polar
/// deposits and ground ice, about 34 m of global layer (Carr & Head 2015,
/// doi:10.1002/2014GL062464), are about 0.004 oceans, and Mars is no ice
/// world. Calibration, no other source: a planet whose bulk is icy but whose surface holds under
/// 1 % of an ocean has lost its ice to its star (#452) and is a dry rock.
/// [`super::interim::base_of`] uses the same threshold for the frozen
/// look.
pub const ICE_WORLD_MIN_OCEANS: f64 = 0.01;

/// Radius, Earth radii, from which a rocky planet is a super-Earth
/// (Borucki et al. 2011, doi:10.1088/0004-637X/736/1/19: Earth-size below
/// 1.25 R⊕, super-Earth-size 1.25 to 2).
const SUPER_EARTH_MIN_RADIUS: f64 = 1.25;

/// Surface pressure above which a runaway greenhouse gives a class of
/// its own, bar. Calibration, no source: a runaway with less than 1 bar
/// has lost its air and is a bare rock (Mercury, Mars).
const DENSE_AIR_BAR: f64 = 1.0;

/// Surface pressure above which liquid hydrocarbons make a hydrocarbon
/// world, bar: methane's triple-point pressure (NIST Chemistry WebBook,
/// 0.1169 ± 0.0006 bar), under which no liquid methane is stable. A cold
/// body whose outgassing barely outruns its star's ultraviolet keeps a
/// trace of methane over its ice under a thinner air
/// ([`super::atmosphere::methane_photolysis`]), and stays an ice world.
pub(crate) const METHANE_TRIPLE_BAR: f64 = 0.1169;

/// Earth oceans of water kept from which an ocean world drowns all land.
/// Calibration: Cowan & Abbot 2014, doi:10.1088/0004-637X/781/1/27, find
/// that the water a planet's basins hold is a few oceans at most, so ten
/// covers the continents.
pub const OCEAN_WORLD_OCEANS: f64 = 10.0;

/// Classifies a planet from its `inputs`, its `interior` and its orbital
/// period, days. The first rule that holds wins:
///
/// 1. a giant (at least [`GIANT_MIN_MASS_EARTH`] and
///    [`GIANT_MIN_ENVELOPE`]): a gas giant from half its mass in hydrogen
///    and helium, an ice giant below; a giant within 10 days of its star
///    is a hot Jupiter (a gas giant) or a hot Neptune (an ice giant);
/// 2. an envelope of at least 0.1 % of the mass and a radius of at least
///    1.6 R⊕: a mini-Neptune;
/// 3. with a `surface` (an enveloped body has none): a steam world (a
///    deep supercritical steam column hides the surface, see
///    [`super::climate::hides_surface`]; tested first, since no lava or
///    rock shows under it), a lava world (liquid
///    lava), a runaway greenhouse (runaway and over
///    [`DENSE_AIR_BAR`]), a hydrocarbon world (liquid hydrocarbons under
///    more than [`METHANE_TRIPLE_BAR`]), an
///    ocean world (liquid water and at least [`OCEAN_WORLD_OCEANS`]);
/// 4. at least 5 % water and ices, and with a `surface`, at least
///    [`ICE_WORLD_MIN_OCEANS`] of them kept there: an ice world (a body
///    that lost its ice falls through);
/// 5. a volcanic world: a body that would be rocky or a super-Earth
///    (no ice world: silicate volcanism is Io's, an icy body's is
///    cryovolcanism, which the model does not describe) with a solid
///    surface in the heat-pipe regime (Moore & Webb 2013,
///    doi:10.1038/nature12424);
/// 6. a super-Earth from 1.25 R⊕, rocky below.
///
/// The class is read off the model; nothing derives from it.
pub fn classify(
    inputs: &PlanetInputs,
    interior: &Interior,
    period_days: f64,
    surface: Option<&Surface>,
) -> PlanetClass {
    let envelope = inputs.envelope_fraction;
    let radius = interior.radius_earth;
    if inputs.mass_earth >= GIANT_MIN_MASS_EARTH && envelope >= GIANT_MIN_ENVELOPE {
        if envelope < GAS_GIANT_MIN_ENVELOPE {
            if period_days <= HOT_GIANT_MAX_PERIOD_DAYS {
                PlanetClass::HotNeptune
            } else {
                PlanetClass::IceGiant
            }
        } else if period_days <= HOT_GIANT_MAX_PERIOD_DAYS {
            PlanetClass::HotJupiter
        } else {
            PlanetClass::GasGiant
        }
    } else if envelope >= MINI_NEPTUNE_MIN_ENVELOPE && radius >= MINI_NEPTUNE_MIN_RADIUS {
        PlanetClass::MiniNeptune
    } else if let Some(class) = surface.and_then(surface_class) {
        class
    } else if inputs.water_fraction >= ICE_WORLD_MIN_WATER
        && surface.is_none_or(|s| s.water.oceans >= ICE_WORLD_MIN_OCEANS)
    {
        PlanetClass::IceWorld
    } else if surface.is_some_and(|s| s.regime == Regime::HeatPipe) {
        PlanetClass::VolcanicWorld
    } else if radius >= SUPER_EARTH_MIN_RADIUS {
        PlanetClass::SuperEarth
    } else {
        PlanetClass::Rocky
    }
}

/// The class a surface gives, if it gives one (see [`classify`]).
fn surface_class(surface: &Surface) -> Option<PlanetClass> {
    let liquid = surface.climate.liquid;
    if super::climate::hides_surface(surface) {
        Some(PlanetClass::SteamWorld)
    } else if liquid == SurfaceLiquid::Lava {
        Some(PlanetClass::LavaWorld)
    } else if surface.water.runaway && surface.gases.total() > DENSE_AIR_BAR {
        Some(PlanetClass::RunawayGreenhouse)
    } else if liquid == SurfaceLiquid::Hydrocarbons && surface.gases.total() > METHANE_TRIPLE_BAR {
        Some(PlanetClass::HydrocarbonWorld)
    } else if liquid == SurfaceLiquid::Water && surface.water.oceans >= OCEAN_WORLD_OCEANS {
        Some(PlanetClass::OceanWorld)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::PlanetClass::*;
    use super::*;
    use crate::exomoon::EARTH_KG;
    use crate::planet::inputs::volatile_rule;
    use crate::planet::reference::{inputs_of, model_inputs, with_envelope};
    use crate::planet::{Context, SOLAR_BODIES, planet_model, reference};
    use crate::star::{DEFAULT_AGE_GYR, StarInfo, parse_type};

    /// The context of a body at `orbit_au` round the Sun, on a circular
    /// orbit, at 4.6 Gyr.
    fn sun_at(orbit_au: f64) -> Context {
        let sun = parse_type("G2V").unwrap();
        let star = StarInfo::new(sun, orbit_au as f32, 0.0, DEFAULT_AGE_GYR);
        Context::of(&star, 0.0, 2.7)
    }

    fn class_of(inputs: PlanetInputs, orbit_au: f64) -> PlanetClass {
        planet_model(&sun_at(orbit_au), inputs).class
    }

    #[test]
    fn the_solar_system_classifies_as_itself() {
        let expect = [
            ("Mercury", Rocky),
            ("Venus", RunawayGreenhouse),
            ("Earth", Rocky),
            ("Mars", Rocky),
            ("Ganymede", IceWorld),
            ("Callisto", IceWorld),
            ("Titan", HydrocarbonWorld),
            ("Europa", IceWorld),
            ("Jupiter", GasGiant),
            ("Saturn", GasGiant),
            ("Uranus", IceGiant),
            ("Neptune", IceGiant),
        ];
        // Venus ran away by the insolation limit, not by boiling.
        let venus = reference("Venus");
        let model = planet_model(&sun_at(venus.orbit_au), model_inputs(venus));
        let water = model.surface.expect("Venus has a surface").water;
        assert!(water.runaway && !water.boiled, "{water:?}");
        for (name, class) in expect {
            // A moon sits at its planet's orbit (its reference orbit).
            let body = reference(name);
            assert_eq!(class_of(model_inputs(body), body.orbit_au), class, "{name}");
        }
        // Photolysis leaves Jupiter's moons no methane, so no seas of it.
        for name in ["Ganymede", "Callisto"] {
            let body = reference(name);
            let model = planet_model(&sun_at(body.orbit_au), model_inputs(body));
            let surface = model.surface.expect("an icy moon has a surface");
            assert_eq!(surface.gases.ch4, 0.0, "{name}");
            assert_eq!(
                surface.climate.liquid,
                crate::planet::SurfaceLiquid::None,
                "{name}"
            );
        }
        // Every reference body gets a class.
        for body in SOLAR_BODIES {
            class_of(model_inputs(body), body.orbit_au);
        }
    }

    /// A bare planet of `mass` M_earth with `water` as its water share, the
    /// volatiles of the rule without scatter, Earth's core share, at
    /// `orbit_au` round the Sun.
    fn model_at(mass: f64, water: f64, orbit_au: f64) -> crate::planet::PlanetModel {
        let inputs = PlanetInputs {
            volatiles: volatile_rule(1.0 - water, water, 0.325),
            ..reference::inputs(mass, 0.325, water)
        };
        planet_model(&sun_at(orbit_au), inputs)
    }

    #[test]
    fn a_planet_that_lost_its_water_is_not_an_ice_world() {
        // The case of #452 (mgs-65793-1225983567 b: 0.22 M_earth, 37 %
        // water, hot): at 0.6 AU these inputs keep 6e-4 oceans in a thin
        // air that gives no runaway class, so only the bulk made it an ice
        // world. (At 0.75 AU the same inputs are a runaway greenhouse.)
        let model = model_at(0.22, 0.37, 0.6);
        let s = model.surface.as_ref().unwrap();
        assert!(s.water.oceans < 0.01, "{s:?}");
        assert_ne!(model.class, IceWorld);
    }

    #[test]
    fn a_cold_icy_planet_stays_an_ice_world() {
        let model = model_at(0.5, 0.3, 4.0);
        assert_eq!(model.class, IceWorld);
    }

    #[test]
    fn a_close_gas_giant_is_a_hot_jupiter() {
        let jupiter = inputs_of(reference("Jupiter"));
        // 4.1 days at 0.05 AU, 32.7 at 0.2 AU.
        assert_eq!(class_of(jupiter.clone(), 0.05), HotJupiter);
        assert!(sun_at(0.2).period_days() > 10.0);
        assert_eq!(class_of(jupiter, 0.2), GasGiant);
    }

    #[test]
    fn a_close_ice_giant_is_a_hot_neptune() {
        let neptune = with_envelope(12.0, 0.16);
        assert_eq!(class_of(neptune.clone(), 0.079), HotNeptune);
        assert_eq!(class_of(neptune, 5.0), IceGiant);
        assert_eq!(class_of(inputs_of(reference("Uranus")), 19.2), IceGiant);
        assert_eq!(class_of(inputs_of(reference("Neptune")), 30.1), IceGiant);
    }

    #[test]
    fn an_enveloped_planet_above_the_valley_is_a_mini_neptune() {
        // At 0.05 AU such an envelope is photoevaporated (see `escape`); at
        // 0.1 AU, since the XUV fluence follows Ribas et al. 2005, it keeps
        // about a third of it.
        assert_eq!(class_of(with_envelope(5.0, 0.02), 0.5), MiniNeptune);
        assert_eq!(class_of(with_envelope(5.0, 0.0), 0.5), SuperEarth);
        assert_eq!(class_of(with_envelope(5.0, 0.02), 0.05), SuperEarth);
        assert_eq!(class_of(with_envelope(5.0, 0.02), 0.1), MiniNeptune);
    }

    #[test]
    fn keys_are_kebab_case_as_serialized() {
        for (i, class) in [
            Rocky,
            SuperEarth,
            IceWorld,
            MiniNeptune,
            IceGiant,
            GasGiant,
            HotJupiter,
            HotNeptune,
            LavaWorld,
            VolcanicWorld,
            RunawayGreenhouse,
            HydrocarbonWorld,
            OceanWorld,
            SteamWorld,
        ]
        .into_iter()
        .enumerate()
        {
            let json = serde_json::to_string(&class).unwrap();
            assert_eq!(json, format!("\"{}\"", class.key()));
            assert_eq!(PLANET_CLASSES[i], class.key());
            let giant = matches!(
                class,
                GasGiant | IceGiant | HotJupiter | HotNeptune | MiniNeptune
            );
            assert_eq!(class.is_giant(), giant, "{class:?}");
            assert_eq!(
                class.sculpts_belts(),
                giant && class != MiniNeptune,
                "{class:?}"
            );
        }
        assert_eq!(SuperEarth.key(), "super-earth");
        assert_eq!(HotJupiter.key(), "hot-jupiter");
        assert_eq!(RunawayGreenhouse.key(), "runaway-greenhouse");
        assert_eq!(HydrocarbonWorld.key(), "hydrocarbon-world");
        assert_eq!(SteamWorld.key(), "steam-world");
    }

    #[test]
    fn a_migrated_water_world_in_the_habitable_zone_is_an_ocean_world() {
        // 1 Earth mass, 30 % water (some 1300 oceans), at 1 AU, with the
        // volatiles `draw_inputs` gives its rock and its ice, without
        // their scatter: the ocean keeps most of them out of the air.
        let inputs = PlanetInputs {
            water_fraction: 0.3,
            volatiles: crate::planet::inputs::volatile_rule(0.7, 0.3, 0.325),
            ..reference::inputs(1.0, 0.325, 0.0)
        };
        let model = planet_model(&sun_at(1.0), inputs);
        let surface = model.surface.expect("a bare body has a surface");
        assert_eq!(model.class, OceanWorld, "{surface:?}");
        assert_eq!(surface.climate.liquid, SurfaceLiquid::Water, "{surface:?}");
    }

    #[test]
    fn a_water_topped_body_under_thick_nitrogen_keeps_its_ocean() {
        // 1 Earth mass, 1 % water (some 40 oceans) at 1 AU, below the
        // runaway limit of the insolation, under the 20 bar of nitrogen
        // its ice brings. The old Eddington core carried its vapour feedback
        // past 647 K and boiled it (issue #441); the moist limit does not
        // depend on the nitrogen, and the column keeps the ocean warm but
        // liquid.
        let inputs = PlanetInputs {
            water_fraction: 0.01,
            volatiles: crate::planet::inputs::volatile_rule(0.99, 0.01, 0.325),
            ..reference::inputs(1.0, 0.325, 0.0)
        };
        let model = planet_model(&sun_at(1.0), inputs);
        let surface = model.surface.expect("a bare body has a surface");
        assert!(surface.gases.n2 > 10.0, "{surface:?}");
        assert!(!surface.water.runaway, "{surface:?}");
        assert!(!surface.water.boiled, "{surface:?}");
        assert!(surface.climate.temperature_k < 330.0, "{surface:?}");
        assert_eq!(surface.climate.liquid, SurfaceLiquid::Water, "{surface:?}");
        assert_ne!(model.class, RunawayGreenhouse);
    }

    #[test]
    fn a_runaway_keeps_in_its_air_the_water_it_keeps() {
        // 0.2 M⊕ with 5 % of water at 0.8 and 0.9 AU, inside the runaway
        // limit of the insolation: every kg of water a runaway keeps is
        // steam in its air, none of it neither vapour nor liquid. (At 1.0
        // and 1.2 AU the old Eddington core boiled them, issue #441.)
        let mut runaways = 0;
        for (water, orbit_au) in [(0.05, 0.8), (0.05, 0.9)] {
            let inputs = PlanetInputs {
                volatiles: volatile_rule(1.0 - water, water, 0.325),
                ..reference::inputs(0.2, 0.325, water)
            };
            let model = planet_model(&sun_at(orbit_au), inputs);
            let surface = model.surface.expect("a bare body has a surface");
            if !surface.water.runaway {
                continue;
            }
            runaways += 1;
            // On the body the climate ran on: the model then weighs only
            // the water its pass kept, without solving the air again.
            let context = sun_at(orbit_au);
            let interior = crate::planet::explain::surface_interior(&context, &model);
            let inputs = crate::planet::explain::surface_inputs(&context, &model);
            let radius = interior.radius_earth * crate::exomoon::EARTH_M;
            let mass = inputs.mass_earth * crate::exomoon::EARTH_KG;
            let gravity = crate::exomoon::G * mass / (radius * radius);
            let bar_per_kg = gravity / (4.0 * std::f64::consts::PI * radius * radius) / 1.0e5;
            let kept_bar = surface.water.kept_kg * bar_per_kg;
            assert!(
                (surface.gases.h2o - kept_bar).abs() <= 1e-9 * kept_bar.max(1e-12),
                "{water} {orbit_au}: {} bar in the air, {kept_bar} bar kept",
                surface.gases.h2o
            );
        }
        assert!(runaways > 0);
    }

    #[test]
    fn a_trace_of_water_does_not_boil_a_temperate_planet() {
        // 0.2 M⊕ at 1.2 AU with 1e-9 of water (2.6e-4 bar if all of it
        // were vapour) and the volatiles of 1 % of ice. Its vapour is
        // bounded by the water it has, so it cannot hold its outgoing
        // radiation at the moist limit. Before the vapour was capped, the
        // first pass put hundreds of bar of steam in its air, boiled, and
        // the planet came out a runaway greenhouse at 296 K.
        let inputs = PlanetInputs {
            volatiles: volatile_rule(1.0 - 1.0e-9, 0.01, 0.325),
            ..reference::inputs(0.2, 0.325, 1.0e-9)
        };
        let model = planet_model(&sun_at(1.2), inputs);
        let surface = model.surface.expect("a bare body has a surface");
        assert!(!surface.water.runaway, "{surface:?}");
        assert_ne!(model.class, RunawayGreenhouse);
        assert!(surface.climate.temperature_k < 400.0, "{surface:?}");
    }

    #[test]
    fn a_cool_small_planet_does_not_boil() {
        // 0.2 M⊕ at 1.5 AU with 0.1 % of water (0.85 Earth oceans). The
        // old Eddington core carried its vapour feedback past 647 K, then
        // refused the boil because its steam escaped to nothing and the
        // dry re-run sat at 293 K. With a moist limit its light (some
        // 107 W/m²) is far under the plateau: a wet balance exists, it
        // does not boil and keeps all its water.
        let inputs = PlanetInputs {
            volatiles: volatile_rule(0.999, 1.0e-3, 0.325),
            ..reference::inputs(0.2, 0.325, 1.0e-3)
        };
        let all = inputs.water_fraction * inputs.mass_earth * crate::exomoon::EARTH_KG;
        let model = planet_model(&sun_at(1.5), inputs);
        let surface = model.surface.expect("a bare body has a surface");
        assert!(!surface.water.runaway, "{surface:?}");
        assert!(!surface.water.boiled, "{surface:?}");
        assert_eq!(surface.water.kept_kg, all, "{surface:?}");
        assert_ne!(model.class, RunawayGreenhouse);
    }

    /// The context of a body round an F0V star (about 7200 K, the hot end
    /// of Kopparapu et al.'s fit) that gives it `flux` S⊕, on a circular
    /// orbit, at 2 Gyr (on its main sequence).
    fn f0v_at(flux: f64) -> Context {
        let kind = parse_type("F0V").unwrap();
        let luminosity = StarInfo::new(kind, 1.0, 0.0, 2.0).luminosity;
        let orbit_au = (f64::from(luminosity) / flux).sqrt();
        let context = Context::of(&StarInfo::new(kind, orbit_au as f32, 0.0, 2.0), 0.0, 5.0);
        assert!((context.insolation - flux).abs() < 1e-3 * flux);
        context
    }

    /// Flux, S⊕, inside the runaway limit of the insolation of an F0V star
    /// (1.296 S⊕ at its 7220 K) whose absorbed light (about 307 W/m² at
    /// the Bond albedo of an ice-free ocean world, about 0.3) exceeds the
    /// moist plateau (about 284 W/m²): an Earth-like wet planet there has
    /// no wet balance and boils, under a thin enough air. (1.24 S⊕ under
    /// the fixed albedo 0.3; the derived albedo of a planet with ice caps
    /// lets one at 1.24 S⊕ keep its ocean, and since the Rayleigh
    /// scattering follows the star's spectrum an F star's blue light
    /// raises the albedo of an Earth under 2.4 bar of N₂ to about 0.40,
    /// under the plateau up to the runaway limit.)
    const F0V_BOILING_FLUX: f64 = 1.28;

    #[test]
    fn a_boil_that_escapes_all_its_water_ends_dry() {
        // The case of #442 (census planet 458 c: 0.906 S⊕ round its star,
        // 1.05 bar of N₂, 93 bar of CO₂ outgassed, 282 bar of vapour). Under
        // the column core 458 c's inputs no longer boil (a Sun-like 1 M⊕
        // heat-pipe planet at 0.906 S⊕ with 93 bar of CO₂ balances at
        // 499 K, its ocean liquid), so the case is rebuilt where the moist
        // limit makes a boil: the Earth at 1.24 S⊕ round an F0V star,
        // inside its runaway limit, 0.03 % of water (342 bar if all of it
        // were vapour), the rule's N₂ (1.03 bar in the first pass) and CO₂
        // set to 65 and 93 bar in the dry air. Its steam escapes to
        // nothing; the dry air sits at about 630 K and 680 K, under and
        // over water's critical point: in both the end state is the dry
        // runaway (the old rule refused the boil whose dry re-run sat
        // under 647 K and kept the wet first pass).
        for co2 in [6.0e-5, 8.6e-5] {
            let inputs = PlanetInputs {
                water_fraction: 3.0e-4,
                volatiles: crate::planet::Volatiles {
                    co2,
                    ..volatile_rule(1.0 - 3.0e-4, 3.0e-4, 0.325)
                },
                ..model_inputs(reference("Earth"))
            };
            let model = planet_model(&f0v_at(F0V_BOILING_FLUX), inputs);
            let surface = model.surface.expect("a bare body has a surface");
            assert!(surface.water.runaway, "{co2}: {surface:?}");
            assert!(surface.water.boiled, "{co2}: {surface:?}");
            assert_eq!(surface.water.kept_kg, 0.0, "{co2}: {surface:?}");
            assert_eq!(surface.gases.h2o, 0.0, "{co2}: {surface:?}");
            assert_ne!(
                surface.climate.liquid,
                SurfaceLiquid::Water,
                "{co2}: {surface:?}"
            );
            assert_ne!(surface.regime, crate::geology::Regime::Plates);
            assert_eq!(model.class, RunawayGreenhouse, "{co2}: {surface:?}");
        }
    }

    /// The Earth at 1.28 S⊕ round an F0V star (see F0V_BOILING_FLUX)
    /// with the volatiles of 0.1 % of ice, a tenth of their N₂, and
    /// just enough water that the steam pass's energy-limited escape
    /// leaves 0.03 bar of it (some 1.6e17 kg): its first pass boils
    /// (0.24 bar of N₂, no wet balance), but so little vapour cannot
    /// hold the outgoing radiation at the moist limit, so the leftover
    /// recondenses into a small ocean. (Under the fixed albedo 0.3,
    /// with 0.1 bar left the balance was at 404 K, over the boiling
    /// point, all of it vapour; from about 1 bar there is none and the
    /// steam runaway stands. The rule's 2.4 bar of N₂ scatter the
    /// F star's blue light enough, since its Rayleigh scattering
    /// follows the star's spectrum, that the first pass keeps a wet
    /// balance and does not boil: issue #522.)
    ///
    /// Its context, the water it formed with (kg) and its model.
    fn recondensed() -> (Context, f64, crate::planet::PlanetModel) {
        let context = f0v_at(F0V_BOILING_FLUX);
        const N2_SHARE: f64 = 0.1;
        let rule = volatile_rule(1.0 - 1.0e-3, 1.0e-3, 0.325);
        let mut inputs = PlanetInputs {
            volatiles: crate::planet::Volatiles {
                n2: N2_SHARE * rule.n2,
                ..rule
            },
            ..model_inputs(reference("Earth"))
        };
        let left_bar = 0.03;
        let mass_kg = inputs.mass_earth * crate::exomoon::EARTH_KG;
        let fluence = crate::planet::xuv_fluence(&context);
        for _ in 0..3 {
            // The water the steam pass loses (9 kg per kg of hydrogen,
            // see `regime::steam`) plus `left_bar` of it; the interior
            // barely moves with the water share, so three rounds settle it.
            let interior = crate::planet::interior(&inputs, &context.exposure());
            let lost =
                9.0 * crate::planet::energy_limited_loss_kg(&interior, inputs.mass_earth, fluence);
            let bar_per_kg = crate::planet::atmosphere::bar_per_kg(inputs.mass_earth, &interior);
            inputs.water_fraction = (lost + left_bar / bar_per_kg) / mass_kg;
        }
        let all = inputs.water_fraction * mass_kg;
        let model = planet_model(&context, inputs);
        (context, all, model)
    }

    #[test]
    fn a_boil_whose_leftover_recondenses_keeps_its_ocean() {
        let (_, all, model) = recondensed();
        let surface = model.surface.expect("a bare body has a surface");
        assert!(!surface.water.runaway, "{surface:?}");
        assert!(surface.water.boiled, "{surface:?}");
        assert!(surface.water.kept_kg > 0.0, "{surface:?}");
        assert!(surface.water.kept_kg < 1e-3 * all, "{surface:?}");
        assert_eq!(surface.climate.liquid, SurfaceLiquid::Water, "{surface:?}");
        assert_ne!(model.class, RunawayGreenhouse);
    }

    #[test]
    fn a_recondensed_boil_shows_in_the_tree() {
        // The water tree says the ocean boiled once, with the water
        // escape took, in Earth oceans (#517).
        let (context, all, model) = recondensed();
        let surface = model.surface.as_ref().expect("a bare body has a surface");
        assert!(
            surface.water.boiled && !surface.water.runaway,
            "{surface:?}"
        );
        let explanation = crate::planet::explain(&context, &model).expect("an explanation");
        let water = explanation
            .trees
            .iter()
            .find(|c| c.quantity == crate::planet::Quantity::WaterKept)
            .expect("a water tree");
        let note = water
            .causes
            .iter()
            .find(|c| c.label.as_deref() == Some("boiled_recondensed"))
            .unwrap_or_else(|| panic!("a boiled_recondensed note: {water:#?}"));
        let lost = (all - surface.water.kept_kg) / crate::planet::OCEAN_KG;
        assert!(
            (note.value - lost).abs() <= 1e-9 * lost,
            "{} vs {lost}",
            note.value
        );
    }

    /// The first cause of `quantity` in `tree`, itself or below it.
    fn cause_of(
        tree: &crate::planet::Cause,
        quantity: crate::planet::Quantity,
    ) -> Option<&crate::planet::Cause> {
        if tree.quantity == quantity {
            return Some(tree);
        }
        tree.causes.iter().find_map(|c| cause_of(c, quantity))
    }

    #[test]
    fn a_runaway_weighs_the_water_its_own_pass_kept() {
        // A runaway's kept pass loses more of its steam to the cosmic
        // shoreline and the non-thermal loss than the energy-limited
        // escape it was handed: the mass, the layers and both trees are
        // those of the water the card shows (#513). Bodies with 10 % of
        // water at 0.4 AU round a K5V star, near their shoreline: at
        // 0.3 M⊕ the pass keeps under 1 % of the steam, at 0.6 M⊕ about
        // three quarters, deep enough to hide the surface; and the
        // super-Earth of #492, whose steam hides it too.
        let k5v = |mass: f64| {
            let water = 0.1;
            let inputs = PlanetInputs {
                volatiles: volatile_rule(1.0 - water, water, 0.325),
                ..reference::inputs(mass, 0.25, water)
            };
            let star = StarInfo::new(parse_type("K5V").unwrap(), 0.4, 0.0, DEFAULT_AGE_GYR);
            let context = Context::of(&star, 0.0, 0.1);
            let model = planet_model(&context, inputs);
            (context, model)
        };
        let star = StarInfo::new(parse_type("M9V").unwrap(), 0.018, 0.0, DEFAULT_AGE_GYR);
        let super_earth = (Context::of(&star, 0.0, 0.1), the_492_super_earth());
        let mut hidden = false;
        let mut short = false;
        for (name, (context, model)) in [
            ("0.3 M⊕", k5v(0.3)),
            ("0.6 M⊕", k5v(0.6)),
            ("#492", super_earth),
        ] {
            let surface = model.surface.as_ref().expect("a bare body has a surface");
            assert!(surface.water.runaway, "{name}: {surface:?}");
            let formed = crate::planet::body_interior(&context, model.formed_inputs());
            let fluence = crate::planet::xuv_fluence(&context);
            let handed = crate::planet::steam(model.formed_inputs(), &formed, fluence).kept_kg;
            println!(
                "{name}: handed {handed:.3e} kg, kept {:.3e} kg",
                surface.water.kept_kg
            );
            short |= surface.water.kept_kg < (1.0 - 1e-3) * handed;
            let m = &model.inputs;
            let water_kg = m.water_fraction * m.mass_earth * EARTH_KG;
            let kept = surface.water.kept_kg;
            assert!(
                (water_kg - kept).abs() <= 1e-9 * kept,
                "{name}: {water_kg} vs {kept}"
            );
            let explanation = crate::planet::explain(&context, &model).expect("an explanation");
            let mass =
                cause_of(&explanation.trees[0], crate::planet::Quantity::Mass).expect("a mass");
            let water = explanation
                .trees
                .iter()
                .find(|c| c.quantity == crate::planet::Quantity::WaterKept)
                .expect("a water tree");
            let mass_kept = cause_of(mass, crate::planet::Quantity::WaterKept).expect("water kept");
            let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs());
            assert!(
                close(mass_kept.value, water.value),
                "{name}: {mass:#?} vs {water:#?}"
            );
            let escape = |tree: &crate::planet::Cause| {
                cause_of(tree, crate::planet::Quantity::Escape).map(|c| c.value)
            };
            let (a, b) = (escape(mass_kept).unwrap(), escape(water).unwrap());
            assert!(close(a, b), "{name}: escape {a} vs {b}");
            if crate::planet::climate::hides_surface(surface) {
                hidden = true;
                let layers: f64 = model.interior.layers.iter().map(|l| l.mass_earth).sum();
                assert!(
                    close(layers, m.mass_earth),
                    "{name}: {layers} vs {}",
                    m.mass_earth
                );
            }
        }
        assert!(hidden, "no steam world hides its surface");
        assert!(short, "no pass kept less than it was handed");
    }

    /// A cause of `quantity` in `tree`, itself or below it.
    fn find(tree: &crate::planet::Cause, quantity: crate::planet::Quantity) -> bool {
        tree.quantity == quantity || tree.causes.iter().any(|c| find(c, quantity))
    }

    #[test]
    fn a_steam_runaway_weighs_only_the_water_it_kept() {
        // The Earth with 1 % of water (some 43 Earth oceans) at 1.4 S⊕
        // round an F0V star, over its runaway limit: escape takes part of
        // the steam and leaves it most (#513). (A boil that keeps steam
        // needs a fine-tuned water share: at the boiling flux the boils
        // lose all of it, and more water than escape takes keeps a wet
        // balance; `a_dry_runaway_keeps_its_dry_mass` takes a boil.)
        let context = f0v_at(1.4);
        let water = 0.01;
        let inputs = PlanetInputs {
            water_fraction: water,
            volatiles: volatile_rule(1.0 - water, water, 0.325),
            ..model_inputs(reference("Earth"))
        };
        let model = planet_model(&context, inputs.clone());
        let surface = model.surface.as_ref().expect("a bare body has a surface");
        assert!(surface.water.runaway, "{surface:?}");
        assert_eq!(model.before_steam.as_ref(), Some(&inputs));
        let formed = crate::planet::body_interior(&context, &inputs);
        // The water its kept pass left: what the energy-limited escape
        // left less what the shoreline took of the steam.
        let kept_kg = surface.water.kept_kg;
        assert!(kept_kg > 0.0);
        let lost_kg = inputs.water_fraction * inputs.mass_earth * EARTH_KG - kept_kg;
        let mass = model.drawn_mass_earth - lost_kg / EARTH_KG;
        let m = &model.inputs;
        assert!(
            (m.mass_earth - mass).abs() <= 1e-9 * mass,
            "{m:?} vs {mass}"
        );
        let water_kg = m.water_fraction * m.mass_earth * EARTH_KG;
        assert!((water_kg - kept_kg).abs() <= 1e-9 * kept_kg, "{m:?}");
        // The layers and gravity are the kept mass's: no larger than the
        // same body's with all its water (a hidden surface's steam layer
        // was already the steam kept), and lighter.
        let with_all = if crate::planet::climate::hides_surface(surface) {
            crate::planet::steam_interior(
                &formed,
                inputs.mass_earth,
                surface.water.kept_kg,
                &context.exposure(),
            )
        } else {
            formed
        };
        let i = &model.interior;
        assert!(
            i.radius_earth <= with_all.radius_earth * (1.0 + 1e-12),
            "{i:?} vs {with_all:?}"
        );
        assert!(i.gravity < with_all.gravity, "{i:?} vs {with_all:?}");
        let gravity = m.mass_earth / i.radius_earth.powi(2);
        assert!((i.gravity - gravity).abs() <= 1e-9 * gravity, "{i:?}");
        // The formation is the drawn one: the escaped water is not an
        // envelope, and the water is the water formed.
        let formation = model.formation();
        assert_eq!(formation.overrides.envelope_fraction, Some(0.0));
        let oceans = water * inputs.mass_earth * EARTH_KG / crate::planet::OCEAN_KG;
        let formed_oceans = formation.overrides.water_oceans.unwrap();
        assert!((formed_oceans - oceans).abs() <= 1e-9 * oceans);
        // Explain replays it, and its mass shows the water lost.
        let explanation = crate::planet::explain(&context, &model).expect("an explanation");
        let mass = explanation.trees[0]
            .causes
            .iter()
            .find(|c| c.quantity == crate::planet::Quantity::Mass)
            .expect("a mass");
        assert!(find(mass, crate::planet::Quantity::Escape), "{mass:#?}");
    }

    #[test]
    fn a_runaway_that_lost_nothing_keeps_its_mass_tree() {
        // A dry Earth at 0.7 AU round the Sun runs away by the insolation
        // limit with no water to lose: its mass did not change, so the
        // model keeps no inputs from before and the mass tree no escape.
        let model = model_at(1.0, 0.0, 0.7);
        let surface = model.surface.as_ref().expect("a bare body has a surface");
        assert!(surface.water.runaway, "{surface:?}");
        assert_eq!(model.before_steam, None);
        let explanation = crate::planet::explain(&sun_at(0.7), &model).expect("an explanation");
        assert!(
            !find(&explanation.trees[0], crate::planet::Quantity::Escape),
            "{:#?}",
            explanation.trees[0]
        );
    }

    #[test]
    fn a_dry_runaway_keeps_its_dry_mass() {
        // The dry boil of `a_boil_that_escapes_all_its_water_ends_dry`.
        let water = 3.0e-4;
        let context = f0v_at(F0V_BOILING_FLUX);
        let inputs = PlanetInputs {
            water_fraction: water,
            volatiles: crate::planet::Volatiles {
                co2: 6.0e-5,
                ..volatile_rule(1.0 - water, water, 0.325)
            },
            ..model_inputs(reference("Earth"))
        };
        let model = planet_model(&context, inputs.clone());
        let surface = model.surface.as_ref().expect("a bare body has a surface");
        assert!(surface.water.boiled && surface.water.runaway, "{surface:?}");
        assert_eq!(surface.water.kept_kg, 0.0);
        let m = &model.inputs;
        assert_eq!(m.water_fraction, 0.0);
        assert!(m.mass_earth > 0.0);
        let dry = inputs.mass_earth * (1.0 - water);
        assert!((m.mass_earth - dry).abs() <= 1e-9 * dry, "{m:?}");
        let explanation = crate::planet::explain(&context, &model).expect("an explanation");
        assert!(
            find(&explanation.trees[0], crate::planet::Quantity::Escape),
            "{explanation:#?}"
        );
    }

    /// The model of the case of #492 (system mgs-81447-3145862552, body
    /// b): 4.78 M⊕ of iron 1.06, rock 3.16 and water 0.56 M⊕ (12 %), the
    /// volatiles of the rule without scatter, round an M9V star at
    /// 0.018 AU.
    fn the_492_super_earth() -> crate::planet::PlanetModel {
        let (iron, rock, water) = (1.06, 3.16, 0.56);
        let mass = iron + rock + water;
        let inputs = PlanetInputs {
            volatiles: volatile_rule(1.0 - water / mass, water / mass, 0.325),
            ..reference::inputs(mass, iron / (iron + rock), water / mass)
        };
        let star = StarInfo::new(parse_type("M9V").unwrap(), 0.018, 0.0, DEFAULT_AGE_GYR);
        planet_model(&Context::of(&star, 0.0, 0.1), inputs)
    }

    #[test]
    fn the_492_super_earth_is_a_steam_world() {
        // Its water runs away under the insolation and stays a deep
        // supercritical column, hundreds of thousands of bar: the surface
        // is hidden and the steam inflates the radius (1.64 R⊕ with the
        // water condensed).
        let model = the_492_super_earth();
        let surface = model.surface.expect("a bare body has a surface");
        assert_eq!(model.class, SteamWorld, "{surface:?}");
        assert!(surface.gases.h2o > 1e5, "{surface:?}");
        // Past the moist plateau, not held at water's critical point.
        assert!(surface.climate.temperature_k > 1000.0, "{surface:?}");
        // The equilibrium temperature it kept is the one under the runaway
        // albedo, which absorbs more than Earth's 0.3 does (about 252 K at
        // its 0.95 S⊕).
        assert!(
            (258.0..280.0).contains(&surface.climate.equilibrium_k),
            "{surface:?}"
        );
        let top = model.interior.layers.last().unwrap();
        assert_eq!(top.material, crate::planet::Material::Steam);
        // Larger than the same body with its water condensed, and than
        // the Turbet et al. fit clamped to its narrower Eq. 3 range
        // (27 kbar), which gave 1.61 R⊕.
        let condensed = crate::planet::solid_interior(&model.inputs).radius_earth;
        assert!((1.6..1.7).contains(&condensed), "{condensed}");
        let radius = model.interior.radius_earth;
        println!(
            "#492 b: {:.0} K, {:.0} bar of steam, {radius:.3} R⊕ ({condensed:.3} \
             condensed), {:.2} g/cm³, {:.2} g⊕",
            surface.climate.temperature_k,
            surface.gases.h2o,
            model.interior.density,
            model.interior.gravity
        );
        assert!(radius > condensed, "{radius} vs {condensed}");
        assert!(radius > 1.61, "{:?}", model.interior);
        assert!(!SteamWorld.is_giant());
    }

    #[test]
    fn thin_steam_over_rock_stays_a_runaway_greenhouse() {
        // 1 M⊕ with 0.01 % of water (about 115 bar if all of it were
        // steam) at 0.7 AU round the Sun, inside the runaway limit of the
        // insolation: the steam column is far under ten times the
        // critical pressure, the rock stays visible.
        let model = model_at(1.0, 1.0e-4, 0.7);
        let surface = model.surface.expect("a bare body has a surface");
        assert!(surface.water.runaway, "{surface:?}");
        assert!(
            !crate::planet::climate::hides_surface(&surface),
            "{surface:?}"
        );
        assert_eq!(model.class, RunawayGreenhouse, "{surface:?}");
        assert!(
            model
                .interior
                .layers
                .iter()
                .all(|l| l.material != crate::planet::Material::Steam)
        );
    }

    #[test]
    fn a_hot_icy_moon_is_no_steam_world_without_a_deep_column() {
        // Europa's make (8 % water) at 0.5 AU round the Sun: its ice runs
        // away, but a body that small holds too thin a steam column (all
        // its water would weigh about 1300 bar) to hide its surface.
        for orbit_au in [0.3, 0.5, 0.7] {
            let europa = model_inputs(reference("Europa"));
            let model = planet_model(&sun_at(orbit_au), europa);
            let surface = model.surface.expect("a bare body has a surface");
            assert!(surface.water.runaway, "{orbit_au}: {surface:?}");
            let hidden = crate::planet::climate::hides_surface(&surface);
            assert_eq!(model.class == SteamWorld, hidden, "{orbit_au}: {surface:?}");
            assert_ne!(model.class, SteamWorld, "{orbit_au}: {surface:?}");
        }
    }

    #[test]
    fn enveloped_bodies_have_no_surface() {
        let context = sun_at(30.1);
        let neptune = planet_model(&context, inputs_of(reference("Neptune")));
        assert_eq!(neptune.surface, None);
        let earth = planet_model(&sun_at(1.0), inputs_of(reference("Earth")));
        assert!(earth.surface.is_some());
    }

    #[test]
    fn every_derived_value_is_finite_on_extreme_inputs() {
        for star in ["M5V", "A0V"] {
            for mass in [0.01, 20.0] {
                for insolation in [1e-3, 1e4] {
                    for age in [0.5, 13.0] {
                        for envelope in [0.0, 0.02] {
                            let star_info =
                                StarInfo::new(parse_type(star).unwrap(), 1.0, 0.0, age as f32);
                            let mut context = Context::of(&star_info, 0.0, 2.7);
                            context.insolation = insolation;
                            context.equilibrium_k =
                                f64::from(crate::star::equilibrium_k(insolation as f32));
                            let inputs = PlanetInputs {
                                water_fraction: 0.1,
                                envelope_fraction: envelope,
                                volatiles: crate::planet::inputs::volatile_rule(
                                    0.88 - envelope,
                                    0.1,
                                    0.325,
                                ),
                                ..reference::inputs(mass, 0.325, 0.1)
                            };
                            let model = planet_model(&context, inputs);
                            let what = format!("{star} {mass} {insolation} {age} {envelope}");
                            assert_eq!(not_finite(&model), None, "{what}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_finiteness_check_sees_nan_and_infinity() {
        let model = planet_model(&sun_at(1.0), model_inputs(reference("Earth")));
        assert_eq!(not_finite(&model), None);
        let mut nan = model.clone();
        nan.heat.radiogenic_w = f64::NAN;
        assert!(not_finite(&nan).is_some());
        let mut deep = model.clone();
        let mut surface = deep.surface.expect("Earth has a surface");
        surface.climate.temperature_k = f64::INFINITY;
        deep.surface = Some(surface);
        assert!(not_finite(&deep).is_some());
        let mut negative = model;
        negative.xuv_fluence = -1.0;
        assert!(not_finite(&negative).is_some());
    }

    /// A Titan with half its nitrogen at 12.4 AU cools to about 64 K:
    /// under the liquidus of its methane–ethane sea, nitrogen condenses
    /// into its seas, which stay liquid, and it stays a hydrocarbon world
    /// (issue #524; 12 AU before the anti-greenhouse and the ethane of
    /// issue #530). The whole Titan no longer settles there since the
    /// anti-greenhouse was anchored (issue #563): it stays at 84.6 K out
    /// to 13.03 AU and falls past it to 57 K, its air frozen out. Its
    /// reducing air outgasses no CO₂, whose frost
    /// would brighten it once its haze thins (issue #557): the test no
    /// longer takes the CO₂ out of its inputs.
    #[test]
    fn condensed_nitrogen_keeps_the_hydrocarbon_class() {
        let mut inputs = model_inputs(reference("Titan"));
        inputs.volatiles.n2 *= 0.5;
        let model = planet_model(&sun_at(12.4), inputs);
        let s = model.surface.expect("a surface");
        let t = s.climate.temperature_k;
        // Methane and ethane, with the nitrogen the air dissolves, would
        // have frozen: the nitrogen condensed into the sea keeps it liquid.
        assert!(
            t < super::super::climate::methane_freezes_k(s.gases.n2),
            "{s:?}"
        );
        assert!(t >= crate::planet::atmosphere::NITROGEN_TRIPLE_K, "{s:?}");
        assert_eq!(
            s.climate.liquid,
            crate::planet::SurfaceLiquid::Hydrocarbons,
            "{s:?}"
        );
        assert_eq!(model.class, HydrocarbonWorld, "{s:?}");
    }

    /// The path of the first number of `model` that is not finite or is
    /// negative, if any. serde_json writes a non-finite `f64` as `null`,
    /// so a `null` counts as one, except where an `Option` is expected
    /// (`surface`, `None` for an enveloped body; `ice_shell`, `None`
    /// without a frozen water layer).
    fn not_finite(model: &crate::planet::PlanetModel) -> Option<String> {
        let json = serde_json::to_value(model).unwrap();
        first_bad(&json, "")
    }

    fn first_bad(value: &serde_json::Value, path: &str) -> Option<String> {
        match value {
            serde_json::Value::Number(n) => {
                let x = n.as_f64().unwrap();
                (!(x.is_finite() && x >= 0.0)).then(|| format!("{path}: {x}"))
            }
            serde_json::Value::Null
                if path != "/surface" && path != "/ice_shell" && path != "/geysers" =>
            {
                Some(format!("{path}: null"))
            }
            serde_json::Value::Array(a) => a
                .iter()
                .enumerate()
                .find_map(|(i, v)| first_bad(v, &format!("{path}/{i}"))),
            serde_json::Value::Object(o) => o
                .iter()
                .find_map(|(k, v)| first_bad(v, &format!("{path}/{k}"))),
            _ => None,
        }
    }
}
