//! The seam between the draw (mapgen3) and the planet model (mapgen1).
//!
//! A [`Trial`] runs before letters exist (they follow distance once
//! every orbit is placed), so it has no designation and its own seed:
//! `system_seed.derive("trial/<archetype id>")` or
//! `"trial/slot-<n>"`, as legacy `system.rs:896-920`
//! (`git show legacy-final:<path>`).
//!
//! `planet::class_of` runs the planet model's chain on a trial (its own
//! seed, no spin, on a scratch copy of the trace) to read the class.
//!
//! The [`Placement`] adds the designation, the body seed and the spin,
//! and `planet::model` runs the same chain on it, drawing only from the
//! body seed, never from `trial.seed` (two planets of one archetype
//! share their trial streams). The orbit's period is the draw's (Kepler),
//! never recomputed.
//!
//! Neither type is serialised.

use science::units::{AstronomicalUnit, Dex, EarthMass, EarthRadius, Gyr, KilogramPerCubicMetre};
use timeline::{Designation, Orbit, PlanetClass, Seed, Spin, StellarSources, Traced};

use crate::ArchetypeId;

/// A candidate body, before designations.
#[derive(Clone, Debug, PartialEq)]
pub struct Trial {
    pub seed: Seed,
    pub mass: Traced<EarthMass>,
    pub orbit: Orbit,
    pub sources: StellarSources,
    pub age: Traced<Gyr>,
    pub metallicity: Traced<Dex>,
    pub ice_line: Traced<AstronomicalUnit>,
    pub archetype: Option<ArchetypeId>,
}

/// A trial once the orbits are placed and the letters assigned.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub designation: Designation,
    pub seed: Seed,
    pub spin: Spin,
    pub trial: Trial,
}

/// What `class_of` reads from a trial: the class and the bulk values the
/// draw needs to place it.
///
/// It carries plain values, not traced ones, because `class_of` runs on a
/// scratch trace whose node ids mean nothing in the draw's trace. The
/// draw records the kept class as `system.draw.class`, from the trial's
/// mass, orbit and flux nodes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrialOutcome {
    pub class: PlanetClass,
    pub radius: EarthRadius,
    pub mean_density: KilogramPerCubicMetre,
}
