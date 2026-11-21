//! Archetypes: the named kinds of body a request can ask for.
//!
//! An archetype is split in two halves so that two sessions can work
//! at once:
//!
//! ```text
//! Archetype { id: ArchetypeId, placement: ArchetypePlacement, targets: ArchetypeTargets }
//! ```
//!
//! - `placement` is read by the system draw and filled by mapgen3: the
//!   host, the insolation or period band, the mass draw, the
//!   eccentricity, the tilt and the age.
//! - `targets` is filled by mapgen1: the planet's inputs, the
//!   present-day targets and their tolerances.
//!
//! The backward search moves only the planet's own inputs and records an
//! unreachable target as an outcome with its cause, never as a silent
//! miss.

use std::collections::BTreeMap;

use science::units::{Day, Degree, EarthFlux, EarthMass, Gyr, SolarMass};
use serde::{Deserialize, Serialize};
use timeline::Code;

use crate::SystemError;

/// The identifier of an archetype (`earth-like`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArchetypeId(pub Code);

/// Where an archetype is placed: the half of an archetype that the
/// system draw reads. Inside a range, mass and insolation are drawn
/// log-uniform and eccentricity uniform (`Definition:`); a fixed value
/// is a zero-width range.
#[derive(Clone, Debug, PartialEq)]
pub struct ArchetypePlacement {
    /// The orbit band.
    pub band: Band,
    /// Mass range.
    pub mass: (EarthMass, EarthMass),
    /// Eccentricity range; `None` leaves it to the family.
    pub eccentricity: Option<(f64, f64)>,
    /// Axial tilt, when the archetype fixes it.
    pub obliquity: Option<Degree>,
    /// The host the archetype names; `None` leaves the star free.
    pub host: Option<Host>,
}

/// The orbit band of an archetype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Band {
    /// A range of the circular flux `L/a²`, as legacy.
    Insolation(EarthFlux, EarthFlux),
    /// A range of orbital periods.
    Period(Day, Day),
}

/// The host an archetype names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Host {
    /// Host mass.
    pub mass: SolarMass,
    /// Age of the system, when the archetype sets one.
    pub age: Option<Gyr>,
}

/// Source: Pecaut & Mamajek (2013, ApJS 208, 9), mass of G2V, via the
/// legacy star table (`legacy-final` crates/mapgen/src/star.rs).
const G2V: SolarMass = SolarMass(1.00);
/// Source: Pecaut & Mamajek (2013), mass of K0V, via the legacy star
/// table (`legacy-final` crates/mapgen/src/star.rs).
const K0V: SolarMass = SolarMass(0.88);
/// Source: Pecaut & Mamajek (2013), mass of K2V, via the legacy star
/// table (`legacy-final` crates/mapgen/src/star.rs).
const K2V: SolarMass = SolarMass(0.82);
/// Source: legacy-final archetypes (`age_gyr`), the age of the Solar
/// System.
const SOLAR_AGE: Gyr = Gyr(4.57);

const fn mass(lo: f64, hi: f64) -> (EarthMass, EarthMass) {
    (EarthMass(lo), EarthMass(hi))
}

const fn flux(lo: f64, hi: f64) -> Band {
    Band::Insolation(EarthFlux(lo), EarthFlux(hi))
}

const fn period(lo: f64, hi: f64) -> Band {
    Band::Period(Day(lo), Day(hi))
}

const fn host(mass: SolarMass, age: Option<Gyr>) -> Option<Host> {
    Some(Host { mass, age })
}

/// The 14 planet archetypes of `legacy-final`, by id. Moons are not
/// here: they belong to T2. A fixed value is a zero-width range.
static PLACEMENTS: [(&str, ArchetypePlacement); 14] = [
    (
        // Source: legacy-final archetypes/earth-like.toml
        "earth-like",
        ArchetypePlacement {
            band: flux(1.0, 1.0),
            mass: mass(1.0, 1.0),
            eccentricity: Some((0.017, 0.017)),
            obliquity: Some(Degree(23.44)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
    (
        // Source: legacy-final archetypes/hot-jupiter.toml
        "hot-jupiter",
        ArchetypePlacement {
            band: period(1.0, 5.0),
            mass: mass(100.0, 1000.0),
            eccentricity: None,
            obliquity: None,
            host: None,
        },
    ),
    (
        // Source: legacy-final archetypes/hot-neptune.toml
        "hot-neptune",
        ArchetypePlacement {
            band: period(2.0, 8.0),
            mass: mass(10.0, 30.0),
            eccentricity: None,
            obliquity: None,
            host: None,
        },
    ),
    (
        // Source: legacy-final archetypes/jupiter-like.toml
        "jupiter-like",
        ArchetypePlacement {
            band: flux(0.033, 0.041),
            mass: mass(286.0, 350.0),
            eccentricity: Some((0.049, 0.049)),
            obliquity: Some(Degree(3.1)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
    (
        // Source: legacy-final archetypes/lava-world.toml
        "lava-world",
        ArchetypePlacement {
            band: flux(1000.0, 3000.0),
            mass: mass(1.0, 5.0),
            eccentricity: Some((0.0, 0.05)),
            obliquity: None,
            host: host(K0V, None),
        },
    ),
    (
        // Source: legacy-final archetypes/mars-like.toml
        "mars-like",
        ArchetypePlacement {
            band: flux(0.431, 0.431),
            mass: mass(0.1074, 0.1074),
            eccentricity: Some((0.093, 0.093)),
            obliquity: Some(Degree(25.19)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
    (
        // Source: legacy-final archetypes/mercury-like.toml
        "mercury-like",
        ArchetypePlacement {
            band: flux(6.0, 7.3),
            mass: mass(0.05, 0.06),
            eccentricity: Some((0.206, 0.206)),
            obliquity: Some(Degree(0.03)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
    (
        // Source: legacy-final archetypes/mini-neptune.toml
        "mini-neptune",
        ArchetypePlacement {
            band: flux(5.0, 50.0),
            mass: mass(5.0, 12.0),
            eccentricity: Some((0.0, 0.05)),
            obliquity: None,
            host: host(K2V, None),
        },
    ),
    (
        // Source: legacy-final archetypes/neptune-like.toml
        "neptune-like",
        ArchetypePlacement {
            band: flux(0.00109, 0.00113),
            mass: mass(15.5, 19.0),
            eccentricity: Some((0.009, 0.009)),
            obliquity: Some(Degree(28.3)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
    (
        // Source: legacy-final archetypes/ocean-world.toml
        "ocean-world",
        ArchetypePlacement {
            band: flux(0.9, 1.1),
            mass: mass(1.0, 3.0),
            eccentricity: Some((0.0, 0.05)),
            obliquity: None,
            host: host(G2V, None),
        },
    ),
    (
        // Source: legacy-final archetypes/saturn-like.toml
        "saturn-like",
        ArchetypePlacement {
            band: flux(0.010, 0.012),
            mass: mass(86.0, 105.0),
            eccentricity: Some((0.057, 0.057)),
            obliquity: Some(Degree(26.7)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
    (
        // Source: legacy-final archetypes/super-earth.toml
        "super-earth",
        ArchetypePlacement {
            band: flux(0.4, 1.0),
            mass: mass(2.5, 6.0),
            eccentricity: Some((0.0, 0.05)),
            obliquity: None,
            host: None,
        },
    ),
    (
        // Source: legacy-final archetypes/uranus-like.toml
        "uranus-like",
        ArchetypePlacement {
            band: flux(0.0024, 0.0030),
            mass: mass(13.0, 16.0),
            eccentricity: Some((0.046, 0.046)),
            obliquity: Some(Degree(82.2)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
    (
        // Source: legacy-final archetypes/venus-like.toml
        "venus-like",
        ArchetypePlacement {
            band: flux(1.7, 2.1),
            mass: mass(0.73, 0.90),
            eccentricity: Some((0.0068, 0.0068)),
            obliquity: Some(Degree(2.6)),
            host: host(G2V, Some(SOLAR_AGE)),
        },
    ),
];

/// The placement of a planet archetype; `None` for a moon or an unknown
/// id.
pub fn placement(id: &ArchetypeId) -> Option<&'static ArchetypePlacement> {
    PLACEMENTS
        .iter()
        .find(|(name, _)| *name == id.0.as_str())
        .map(|(_, placement)| placement)
}

/// The host mass and date that the asked archetypes and the request
/// settle on. Archetypes with a count of 0 are skipped; two that name
/// different host masses are refused. A mass or date the request names
/// wins over the archetypes' (a star named is honoured, whatever it
/// does to the planets asked); an archetype's age is kept only when the
/// request names no date.
pub(crate) fn asked_host(
    asked: &BTreeMap<ArchetypeId, u32>,
    host_mass: Option<SolarMass>,
    present: Option<Gyr>,
) -> Result<(Option<SolarMass>, Option<Gyr>), SystemError> {
    let mut named: Option<Host> = None;
    for (id, &count) in asked {
        if count == 0 {
            continue;
        }
        let Some(placement) = placement(id) else {
            return Err(SystemError::UnknownArchetype {
                archetype: id.clone(),
            });
        };
        let Some(host) = placement.host else {
            continue;
        };
        match &mut named {
            None => named = Some(host),
            Some(first) if first.mass != host.mass => {
                return Err(SystemError::IncompatibleHost {
                    archetype: id.clone(),
                });
            }
            Some(first) => first.age = first.age.or(host.age),
        }
    }
    Ok((
        host_mass.or(named.map(|h| h.mass)),
        present.or(named.and_then(|h| h.age)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: [&str; 14] = [
        "earth-like",
        "hot-jupiter",
        "hot-neptune",
        "jupiter-like",
        "lava-world",
        "mars-like",
        "mercury-like",
        "mini-neptune",
        "neptune-like",
        "ocean-world",
        "saturn-like",
        "super-earth",
        "uranus-like",
        "venus-like",
    ];

    fn id(text: &str) -> ArchetypeId {
        ArchetypeId(Code::new(text.to_string()).unwrap())
    }

    fn asked(items: &[(&str, u32)]) -> BTreeMap<ArchetypeId, u32> {
        items.iter().map(|&(n, c)| (id(n), c)).collect()
    }

    #[test]
    fn every_legacy_planet_archetype_is_placed() {
        for name in IDS {
            assert!(placement(&id(name)).is_some(), "{name}");
        }
        assert_eq!(PLACEMENTS.len(), 14);
    }

    #[test]
    fn earth_like_sits_at_one_earth_flux() {
        let earth = placement(&id("earth-like")).unwrap();
        assert_eq!(earth.band, flux(1.0, 1.0));
        assert_eq!(earth.mass, mass(1.0, 1.0));
        assert_eq!(earth.host.unwrap().mass, SolarMass(1.0));
        assert_eq!(earth.host.unwrap().age, Some(Gyr(4.57)));
    }

    #[test]
    fn two_archetypes_with_different_hosts_are_refused() {
        let result = asked_host(&asked(&[("earth-like", 1), ("lava-world", 1)]), None, None);
        assert_eq!(
            result,
            Err(SystemError::IncompatibleHost {
                archetype: id("lava-world")
            })
        );
    }

    #[test]
    fn an_asked_host_mass_wins() {
        let result = asked_host(&asked(&[("earth-like", 1)]), Some(SolarMass(0.5)), None);
        assert_eq!(result, Ok((Some(SolarMass(0.5)), Some(Gyr(4.57)))));
        let named = asked_host(&asked(&[("earth-like", 1)]), None, None);
        assert_eq!(named, Ok((Some(SolarMass(1.0)), Some(Gyr(4.57)))));
    }

    #[test]
    fn an_asked_date_wins() {
        let result = asked_host(&asked(&[("earth-like", 1)]), None, Some(Gyr(2.0)));
        assert_eq!(result, Ok((Some(SolarMass(1.0)), Some(Gyr(2.0)))));
    }

    #[test]
    fn a_count_of_zero_is_ignored() {
        let result = asked_host(&asked(&[("earth-like", 1), ("lava-world", 0)]), None, None);
        assert_eq!(result, Ok((Some(SolarMass(1.0)), Some(Gyr(4.57)))));
        let hostless = asked_host(&asked(&[("hot-neptune", 2)]), None, None);
        assert_eq!(hostless, Ok((None, None)));
    }

    #[test]
    fn a_moon_or_unknown_id_is_refused() {
        assert!(placement(&id("titan-like")).is_none());
        assert!(placement(&id("nonsense")).is_none());
        let result = asked_host(&asked(&[("titan-like", 1)]), None, None);
        assert_eq!(
            result,
            Err(SystemError::UnknownArchetype {
                archetype: id("titan-like")
            })
        );
    }
}
