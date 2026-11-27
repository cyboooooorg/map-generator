//! The seam between the draw (mapgen3) and the planet model (mapgen1).
//!
//! A [`Trial`] runs before letters exist (they follow distance once
//! every orbit is placed), so it has no designation and its own seed:
//! `system_seed.derive("trial/<archetype id>/<k>")` or
//! `"trial/slot-<n>"` (`draw/place.rs`), unique per planet, as legacy
//! `system.rs:896-920` (`git show legacy-final:<path>`).
//!
//! `planet::class_of` runs the planet model's chain on a trial (its own
//! seed, no spin, on a scratch copy of the trace) to read the class.
//!
//! The [`Placement`] adds the designation, the body seed and the spin.
//! `planet::model` runs the same chain on it from the trial's seed, as
//! `class_of` does, and the spin is drawn from that seed too, so that
//! the obliquity the climate reads is the placement's. The draw
//! classifies the circular trial and `model` runs on the eccentric one.
//! The body seed stays for what is drawn after the designation (nothing
//! in the planet model). The orbit's period is the draw's (Kepler),
//! never recomputed.
//!
//! The orbit's perihelion longitude draws from
//! `system_seed.derive("orbit/<label>")`, where the label is the trial's
//! (`earth-like`, `earth-like/2`, `slot-3`), not from the trial seed.
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
