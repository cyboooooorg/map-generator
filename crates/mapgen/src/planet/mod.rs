//! The physical planet model (track A, issue #423): what a planet is made
//! of and what follows from it, derived from its mass and composition
//! rather than from a type name (see
//! `docs/superpowers/specs/2026-11-11-physical-planet-model-design.md`).
//!
//! [`PlanetInputs`] holds the drawn quantities; [`interior::solid_interior`]
//! derives the iron, rock and water layers, and [`interior::interior`] adds
//! the hydrogen and helium envelope under the star's [`Exposure`], giving
//! the radius, the bulk density and the surface gravity.
//! [`heat::InternalHeat`] holds the heat it makes inside: radiogenic
//! ([`heat::radiogenic_w`]) and tidal ([`heat::tidal_power_w`]).
//! [`reference::SOLAR_BODIES`] lists the Solar System bodies the relations
//! are checked against.
use serde::{Deserialize, Serialize};

pub mod heat;
pub mod interior;
pub mod reference;

pub use heat::{EARTH_SILICATE_KG, InternalHeat, radiogenic_w, tidal_power_w};
pub use interior::{GIANT_MIN_ENVELOPE, GIANT_MIN_MASS_EARTH, interior, solid_interior};
pub use reference::{ReferenceBody, SOLAR_BODIES, reference};

/// Kilograms of each volatile per kilogram of planet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Volatiles {
    pub co2: f64,
    pub n2: f64,
    pub ch4: f64,
    pub nh3: f64,
    pub s: f64,
}

/// What a planet is made of: its mass and the shares of its components.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanetInputs {
    /// Mass, Earth masses.
    pub mass_earth: f64,
    /// Iron core share of the refractory (iron + rock) mass. Earth 0.325.
    pub core_mass_fraction: f64,
    /// Share of the total mass in water and ices.
    pub water_fraction: f64,
    /// Hydrogen and helium share of the total mass, at formation.
    pub envelope_fraction: f64,
    pub volatiles: Volatiles,
    /// Radiogenic abundance at formation, relative to Earth's (1.0).
    pub radiogenic: f64,
}

/// What a layer of a planet is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Material {
    Iron,
    Rock,
    Water,
    Envelope,
}

/// One shell of a planet, from the outer radius of the layer below.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub material: Material,
    /// Mass of the layer, Earth masses.
    pub mass_earth: f64,
    /// Outer radius of the layer, Earth radii.
    pub outer_radius_earth: f64,
}

/// What a planet's star gives it, which sets the size of a hydrogen and
/// helium envelope.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exposure {
    /// Light received, Earth's = 1.
    pub insolation: f64,
    /// Equilibrium temperature, K.
    pub equilibrium_k: f64,
    /// Age of the system, Gyr.
    pub age_gyr: f64,
}

/// The layered structure of a planet and its bulk quantities.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interior {
    /// Innermost first; the last layer's outer radius is the radius.
    pub layers: Vec<Layer>,
    /// Radius, Earth radii.
    pub radius_earth: f64,
    /// Bulk density, g/cm³.
    pub density: f64,
    /// Surface gravity, Earth's = 1.
    pub gravity: f64,
}
