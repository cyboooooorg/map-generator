//! What kind of planet the model gives (spec "Derived, in order", the
//! class): read off the derived mass, envelope, water and radius, never
//! fed back into them ([`classify`]).
use serde::{Deserialize, Serialize};

use super::{GIANT_MIN_ENVELOPE, GIANT_MIN_MASS_EARTH, Interior, PlanetInputs};

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
}

impl PlanetClass {
    /// The class as stored: `rocky`, `super-earth`, `ice-world`,
    /// `mini-neptune`, `ice-giant`, `gas-giant`, `hot-jupiter`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Rocky => "rocky",
            Self::SuperEarth => "super-earth",
            Self::IceWorld => "ice-world",
            Self::MiniNeptune => "mini-neptune",
            Self::IceGiant => "ice-giant",
            Self::GasGiant => "gas-giant",
            Self::HotJupiter => "hot-jupiter",
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

/// Classifies a planet from its `inputs`, its `interior` and its orbital
/// period, days. The first rule that holds wins:
///
/// 1. a giant (at least [`GIANT_MIN_MASS_EARTH`] and
///    [`GIANT_MIN_ENVELOPE`]): a gas giant from half its mass in hydrogen
///    and helium, an ice giant below; a gas giant within 10 days of its
///    star is a hot Jupiter;
/// 2. an envelope of at least 0.1 % of the mass and a radius of at least
///    1.6 R⊕: a mini-Neptune;
/// 3. at least 5 % water and ices: an ice world;
/// 4. a super-Earth from 1.25 R⊕, rocky below.
///
/// The class is read off the model; nothing derives from it.
pub fn classify(inputs: &PlanetInputs, interior: &Interior, period_days: f64) -> PlanetClass {
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
    } else if inputs.water_fraction >= ICE_WORLD_MIN_WATER {
        PlanetClass::IceWorld
    } else if radius >= SUPER_EARTH_MIN_RADIUS {
        PlanetClass::SuperEarth
    } else {
        PlanetClass::Rocky
    }
}

#[cfg(test)]
mod tests {
    use super::PlanetClass::*;
    use super::*;
    use crate::planet::reference::{inputs_of, with_envelope};
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
            ("Venus", Rocky),
            ("Earth", Rocky),
            ("Mars", Rocky),
            ("Ganymede", IceWorld),
            ("Titan", IceWorld),
            ("Europa", IceWorld),
            ("Jupiter", GasGiant),
            ("Saturn", GasGiant),
            ("Uranus", IceGiant),
            ("Neptune", IceGiant),
        ];
        for (name, class) in expect {
            // A moon sits at its planet's orbit (its reference orbit).
            let body = reference(name);
            assert_eq!(class_of(inputs_of(body), body.orbit_au), class, "{name}");
        }
        // Every reference body gets a class.
        for body in SOLAR_BODIES {
            class_of(inputs_of(body), body.orbit_au);
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
        // At 0.1 AU such an envelope is photoevaporated (see `escape`).
        assert_eq!(class_of(with_envelope(5.0, 0.02), 0.5), MiniNeptune);
        assert_eq!(class_of(with_envelope(5.0, 0.0), 0.5), SuperEarth);
        assert_eq!(class_of(with_envelope(5.0, 0.02), 0.1), SuperEarth);
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
        ] {
            let json = serde_json::to_string(&class).unwrap();
            assert_eq!(json, format!("\"{}\"", class.key()));
        }
        assert_eq!(SuperEarth.key(), "super-earth");
        assert_eq!(HotJupiter.key(), "hot-jupiter");
    }
}
