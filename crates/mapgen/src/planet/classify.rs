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
}

impl PlanetClass {
    /// The class as stored: `rocky`, `super-earth`, `ice-world`,
    /// `mini-neptune`, `ice-giant`, `gas-giant`, `hot-jupiter`,
    /// `hot-neptune`, `lava-world`, `volcanic-world`, `runaway-greenhouse`, `hydrocarbon-world`,
    /// `ocean-world`.
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
pub const PLANET_CLASSES: [&str; 13] = [
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
const METHANE_TRIPLE_BAR: f64 = 0.1169;

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
/// 3. with a `surface` (an enveloped body has none): a lava world (liquid
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
    if liquid == SurfaceLiquid::Lava {
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
            volatiles: volatile_rule(1.0 - water, water),
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
    }

    #[test]
    fn a_migrated_water_world_in_the_habitable_zone_is_an_ocean_world() {
        // 1 Earth mass, 30 % water (some 1300 oceans), at 1 AU, with the
        // volatiles `draw_inputs` gives its rock and its ice, without
        // their scatter: the ocean keeps most of them out of the air.
        let inputs = PlanetInputs {
            water_fraction: 0.3,
            volatiles: crate::planet::inputs::volatile_rule(0.7, 0.3),
            ..reference::inputs(1.0, 0.325, 0.0)
        };
        let model = planet_model(&sun_at(1.0), inputs);
        let surface = model.surface.expect("a bare body has a surface");
        assert_eq!(model.class, OceanWorld, "{surface:?}");
        assert_eq!(surface.climate.liquid, SurfaceLiquid::Water, "{surface:?}");
    }

    #[test]
    fn a_water_topped_body_past_the_critical_point_is_a_steam_runaway() {
        // 1 Earth mass, 1 % water (some 40 oceans) at 1 AU, below the
        // runaway limit of the insolation: the 20 bar of nitrogen its ice
        // brings carries the vapour feedback past 647 K.
        let inputs = PlanetInputs {
            water_fraction: 0.01,
            volatiles: crate::planet::inputs::volatile_rule(0.99, 0.01),
            ..reference::inputs(1.0, 0.325, 0.0)
        };
        let model = planet_model(&sun_at(1.0), inputs);
        let surface = model.surface.expect("a bare body has a surface");
        assert!(surface.water.kept_kg > 0.0, "{surface:?}");
        assert!(surface.water.runaway, "{surface:?}");
        assert!(surface.water.boiled, "{surface:?}");
        assert!(surface.climate.temperature_k >= 647.0, "{surface:?}");
        assert_ne!(surface.climate.liquid, SurfaceLiquid::Lava, "{surface:?}");
        assert_ne!(surface.climate.liquid, SurfaceLiquid::Water, "{surface:?}");
        assert_ne!(surface.regime, crate::geology::Regime::Plates);
        assert_eq!(model.class, RunawayGreenhouse);
    }

    #[test]
    fn a_trace_of_water_does_not_boil_a_temperate_planet() {
        // 0.2 M⊕ at 1.2 AU with 1e-9 of water (2.6e-4 bar if all of it
        // were vapour) and the volatiles of 1 % of ice. Its vapour is
        // bounded by the water it has, so it cannot carry the surface past
        // 647 K. Before the vapour was capped, the first pass put hundreds
        // of bar of steam in its air, boiled, and the planet came out a
        // runaway greenhouse at 296 K.
        let inputs = PlanetInputs {
            volatiles: volatile_rule(1.0 - 1.0e-9, 0.01),
            ..reference::inputs(0.2, 0.325, 1.0e-9)
        };
        let model = planet_model(&sun_at(1.2), inputs);
        let surface = model.surface.expect("a bare body has a surface");
        assert!(!surface.water.runaway, "{surface:?}");
        assert_ne!(model.class, RunawayGreenhouse);
        assert!(surface.climate.temperature_k < 400.0, "{surface:?}");
    }

    #[test]
    fn a_boil_whose_steam_cools_is_not_kept() {
        // 0.2 M⊕ at 1.5 AU with 0.1 % of water (0.85 Earth oceans). Its
        // vapour feedback carries the first pass past 647 K, but its steam
        // escapes to nothing and the dry re-run sits at 293 K: the boil is
        // not self-consistent, so the first pass stands, not a runaway
        // greenhouse at room temperature.
        let inputs = PlanetInputs {
            volatiles: volatile_rule(0.999, 1.0e-3),
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

    #[test]
    fn a_runaway_keeps_in_its_air_the_water_it_keeps() {
        // 0.2 M⊕ with 5 % and 1 % of water at 1.0 and 1.2 AU: every kg of
        // water a runaway keeps is steam in its air, none of it neither
        // vapour nor liquid.
        let mut runaways = 0;
        for (water, orbit_au) in [(0.05, 1.0), (0.01, 1.2)] {
            let inputs = PlanetInputs {
                volatiles: volatile_rule(1.0 - water, water),
                ..reference::inputs(0.2, 0.325, water)
            };
            let model = planet_model(&sun_at(orbit_au), inputs);
            let surface = model.surface.expect("a bare body has a surface");
            if !surface.water.runaway {
                continue;
            }
            runaways += 1;
            let radius = model.interior.radius_earth * crate::exomoon::EARTH_M;
            let mass = model.inputs.mass_earth * crate::exomoon::EARTH_KG;
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
            serde_json::Value::Null if path != "/surface" && path != "/ice_shell" => {
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
