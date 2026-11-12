//! What kind of planet the model gives (spec "Derived, in order", the
//! class): read off the derived mass, envelope, water and radius, never
//! fed back into them ([`classify`]).
use serde::{Deserialize, Serialize};

use super::climate::SurfaceLiquid;
use super::{GIANT_MIN_ENVELOPE, GIANT_MIN_MASS_EARTH, Interior, PlanetInputs, Surface};

/// The kind of a planet, from its derived quantities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanetClass {
    Rocky,
    SuperEarth,
    IceWorld,
    MiniNeptune,
    IceGiant,
    GasGiant,
    HotJupiter,
    LavaWorld,
    RunawayGreenhouse,
    HydrocarbonWorld,
    OceanWorld,
}

impl PlanetClass {
    /// The class as stored: `rocky`, `super-earth`, `ice-world`,
    /// `mini-neptune`, `ice-giant`, `gas-giant`, `hot-jupiter`,
    /// `lava-world`, `runaway-greenhouse`, `hydrocarbon-world`,
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
            Self::LavaWorld => "lava-world",
            Self::RunawayGreenhouse => "runaway-greenhouse",
            Self::HydrocarbonWorld => "hydrocarbon-world",
            Self::OceanWorld => "ocean-world",
        }
    }
}

/// Least hydrogen and helium share of a gas giant; a giant with less is an
/// ice giant. Calibration, no source: it parts Jupiter and Saturn (about
/// 0.85 to 0.95 of their mass, Guillot 2005,
/// doi:10.1146/annurev.earth.33.092203.122639) from Uranus and Neptune
/// (about 0.1 to 0.2, Helled et al. 2011, doi:10.1088/0004-637X/726/1/15).
const GAS_GIANT_MIN_ENVELOPE: f64 = 0.5;

/// Longest orbital period of a hot Jupiter, days (Wright et al. 2012,
/// doi:10.1088/0004-637X/753/2/160: giants within 10 days of their star,
/// about 1 % of Sun-like stars).
const HOT_JUPITER_MAX_PERIOD_DAYS: f64 = 10.0;

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
const ICE_WORLD_MIN_WATER: f64 = 0.05;

/// Radius, Earth radii, from which a rocky planet is a super-Earth
/// (Borucki et al. 2011, doi:10.1088/0004-637X/736/1/19: Earth-size below
/// 1.25 R⊕, super-Earth-size 1.25 to 2).
const SUPER_EARTH_MIN_RADIUS: f64 = 1.25;

/// Surface pressure above which a runaway greenhouse or a sea of
/// hydrocarbons gives a class of its own, bar. Calibration, no source: a
/// runaway with less than 1 bar has lost its air and is a bare rock
/// (Mercury, Mars); Titan's 1.5 bar holds its seas, while the model
/// outgasses 0.2 to 0.5 bar of methane on Ganymede and Callisto, far more
/// than they hold, and they stay ice worlds.
const DENSE_AIR_BAR: f64 = 1.0;

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
///    and helium, an ice giant below; a gas giant within 10 days of its
///    star is a hot Jupiter;
/// 2. an envelope of at least 0.1 % of the mass and a radius of at least
///    1.6 R⊕: a mini-Neptune;
/// 3. with a `surface` (an enveloped body has none): a lava world (liquid
///    lava), a runaway greenhouse (runaway and over
///    [`DENSE_AIR_BAR`]), a hydrocarbon world (liquid hydrocarbons under
///    more than [`DENSE_AIR_BAR`]), an
///    ocean world (liquid water and at least [`OCEAN_WORLD_OCEANS`]);
/// 4. at least 5 % water and ices: an ice world;
/// 5. a super-Earth from 1.25 R⊕, rocky below.
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
            PlanetClass::IceGiant
        } else if period_days <= HOT_JUPITER_MAX_PERIOD_DAYS {
            PlanetClass::HotJupiter
        } else {
            PlanetClass::GasGiant
        }
    } else if envelope >= MINI_NEPTUNE_MIN_ENVELOPE && radius >= MINI_NEPTUNE_MIN_RADIUS {
        PlanetClass::MiniNeptune
    } else if let Some(class) = surface.and_then(surface_class) {
        class
    } else if inputs.water_fraction >= ICE_WORLD_MIN_WATER {
        PlanetClass::IceWorld
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
    } else if liquid == SurfaceLiquid::Hydrocarbons && surface.gases.total() > DENSE_AIR_BAR {
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
            ("Titan", HydrocarbonWorld),
            ("Europa", IceWorld),
            ("Jupiter", GasGiant),
            ("Saturn", GasGiant),
            ("Uranus", IceGiant),
            ("Neptune", IceGiant),
        ];
        for (name, class) in expect {
            // A moon sits at its planet's orbit (its reference orbit).
            let body = reference(name);
            assert_eq!(class_of(model_inputs(body), body.orbit_au), class, "{name}");
        }
        // Every reference body gets a class.
        for body in SOLAR_BODIES {
            class_of(model_inputs(body), body.orbit_au);
        }
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
        for class in [
            Rocky,
            SuperEarth,
            IceWorld,
            MiniNeptune,
            IceGiant,
            GasGiant,
            HotJupiter,
            LavaWorld,
            RunawayGreenhouse,
            HydrocarbonWorld,
            OceanWorld,
        ] {
            let json = serde_json::to_string(&class).unwrap();
            assert_eq!(json, format!("\"{}\"", class.key()));
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
        assert!(surface.climate.temperature_k >= 647.0, "{surface:?}");
        assert_ne!(surface.climate.liquid, SurfaceLiquid::Lava, "{surface:?}");
        assert_ne!(surface.climate.liquid, SurfaceLiquid::Water, "{surface:?}");
        assert_ne!(surface.regime, crate::geology::Regime::Plates);
        assert_eq!(model.class, RunawayGreenhouse);
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
                            let json = serde_json::to_value(&model).unwrap();
                            assert_all_finite(
                                &json,
                                &format!("{star} {mass} {insolation} {age} {envelope}"),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Every number in `value` is finite and not negative.
    fn assert_all_finite(value: &serde_json::Value, what: &str) {
        match value {
            serde_json::Value::Number(n) => {
                let x = n.as_f64().unwrap();
                assert!(x.is_finite() && x >= 0.0, "{what}: {x}");
            }
            serde_json::Value::Array(a) => a.iter().for_each(|v| assert_all_finite(v, what)),
            serde_json::Value::Object(o) => o.values().for_each(|v| assert_all_finite(v, what)),
            _ => {}
        }
    }
}
