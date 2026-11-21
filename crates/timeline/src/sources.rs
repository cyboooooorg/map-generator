//! The stars lighting a planet: one entry per star, the primary first.

use crate::{ContractError, NodeId, Traced, TracedFields};
use science::units::{EarthFlux, Kelvin, SolarLuminosity, SolarMass};
use serde::{Deserialize, Serialize};

/// One star as a planet sees it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StellarSource {
    /// Orbit-mean bolometric flux, eccentricity factor applied.
    pub flux: Traced<EarthFlux>,
    /// Equilibrium temperature from this star alone, at Earth's Bond
    /// albedo 0.3 (255 K at one Earth flux), eccentricity factor applied.
    pub equilibrium: Traced<Kelvin>,
    /// Effective temperature of the star.
    pub effective_temperature: Traced<Kelvin>,
    /// Mass of the star.
    pub mass: Traced<SolarMass>,
    /// Luminosity of the star.
    pub luminosity: Traced<SolarLuminosity>,
}

impl TracedFields for StellarSource {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        self.flux.for_each_node(f);
        self.equilibrium.for_each_node(f);
        self.effective_temperature.for_each_node(f);
        self.mass.for_each_node(f);
        self.luminosity.for_each_node(f);
    }
}

/// One to three stars, the primary first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<StellarSource>")]
pub struct StellarSources(Vec<StellarSource>);

impl TryFrom<Vec<StellarSource>> for StellarSources {
    type Error = ContractError;

    fn try_from(list: Vec<StellarSource>) -> Result<Self, Self::Error> {
        StellarSources::new(list)
    }
}

impl StellarSources {
    /// The sources of a planet: one to three, else `NoSource` or
    /// `TooManySources`, with masses in non-increasing order (primary
    /// first), else `SourcesNotOrdered`.
    pub fn new(list: Vec<StellarSource>) -> Result<Self, ContractError> {
        match list.len() {
            0 => return Err(ContractError::NoSource),
            1..=3 => {}
            n => return Err(ContractError::TooManySources(n)),
        }
        if list
            .windows(2)
            .any(|pair| pair[0].mass.value().0 < pair[1].mass.value().0)
        {
            return Err(ContractError::SourcesNotOrdered);
        }
        Ok(StellarSources(list))
    }

    /// The first star.
    pub fn primary(&self) -> &StellarSource {
        &self.0[0]
    }

    /// The stars, primary first.
    pub fn iter(&self) -> std::slice::Iter<'_, StellarSource> {
        self.0.iter()
    }

    /// The number of stars, 1 to 3.
    // Never empty by construction, so no `is_empty`.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The sum of the fluxes. With one star, its flux as stored.
    pub fn total_flux(&self) -> EarthFlux {
        if self.0.len() == 1 {
            return *self.primary().flux.value();
        }
        EarthFlux(self.0.iter().map(|s| s.flux.value().0).sum())
    }

    /// The equilibrium temperature under all the stars, (Σ T⁴)^(1/4).
    /// With one star, its value as stored.
    pub fn equilibrium_k(&self) -> Kelvin {
        if self.0.len() == 1 {
            return *self.primary().equilibrium.value();
        }
        let sum: f64 = self.0.iter().map(|s| s.equilibrium.value().0.powi(4)).sum();
        Kelvin(sum.sqrt().sqrt())
    }

    /// The mean of `f` weighted by flux, Σ flux·f / Σ flux. With one
    /// star, `f` of the primary.
    pub fn weighted(&self, f: impl Fn(&StellarSource) -> f64) -> f64 {
        if self.0.len() == 1 {
            return f(self.primary());
        }
        let total: f64 = self.0.iter().map(|s| s.flux.value().0).sum();
        let sum: f64 = self.0.iter().map(|s| s.flux.value().0 * f(s)).sum();
        sum / total
    }
}

impl TracedFields for StellarSources {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        self.0.for_each_node(f);
    }
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
