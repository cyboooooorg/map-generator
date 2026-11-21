//! Thickness of a mini-Neptune's hydrogen-helium envelope, read in the
//! tables of Lopez & Fortney 2014 ("Understanding the Mass-Radius Relation
//! for Sub-Neptunes", ApJ 792, 1, arXiv:1311.0329), Tables 2 to 4 (solar
//! metallicity, 100 Myr, 1 Gyr and 10 Gyr).
//!
//! The table is `crates/science/data/mini-neptunes-lopez2014.tsv`; its
//! provenance, units and validity limits are in
//! `crates/science/data/GIANTS.md`. The study behind the choices is
//! `docs/superpowers/studies/2026-11-21-planet-model-2a/giants-and-envelopes.md`
//! (§2.1 the model, §2.2 the tables, §2.4 the proposal).
//!
//! The tables give the planet radius at 20 mbar (core, convective envelope
//! and the isothermal atmosphere above it). The thickness is that radius
//! minus the authors' own core radius (their Eq. 1), so that a caller adds
//! it to a core of its own.

use crate::giants::{bracket, clamp};
use crate::units::{EarthFlux, EarthMass, EarthRadius, Fraction, Gyr};
use std::sync::LazyLock;

/// The Lopez & Fortney 2014 Tables 2 to 4, 792 rows.
const TABLE: &str = include_str!("../data/mini-neptunes-lopez2014.tsv");

/// Source: Lopez & Fortney 2014 Tables 2 to 4, the first envelope column,
/// 0.01 % of the planet mass.
pub const LOWEST_TABULATED: Fraction = Fraction::new(1e-4).unwrap();

/// Source: Lopez & Fortney 2014 Tables 2 to 4, the last envelope column,
/// 20 % of the planet mass.
pub const HIGHEST_TABULATED: Fraction = Fraction::new(0.20).unwrap();

/// Source: Lopez & Fortney 2014 Tables 2 to 4, the lightest mass row,
/// 1 M⊕.
pub const MIN_MASS: EarthMass = EarthMass(1.0);

/// Source: Lopez & Fortney 2014 Tables 2 to 4, the heaviest mass row,
/// 20 M⊕.
pub const MAX_MASS: EarthMass = EarthMass(20.0);

/// Source: Lopez & Fortney 2014 Eq. 1, the radius of their Earth-like
/// core, `R_core = (M_core / M⊕)^0.25` R⊕ ("to within about 2 %"; study
/// §2.1).
const CORE_RADIUS_EXPONENT: f64 = 0.25;

/// Definition: percent to fraction, the unit of the table's envelope
/// column.
const PER_CENT: f64 = 100.0;

/// Why an envelope cannot be read in the tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnvelopeError {
    /// The mass is above [`MAX_MASS`] (or not a positive finite number),
    /// or the envelope above [`HIGHEST_TABULATED`].
    OutOfTable { mass: EarthMass, envelope: Fraction },
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfTable { mass, envelope } => write!(
                f,
                "no Lopez & Fortney 2014 row for {} M⊕ with a {} envelope",
                mass.0,
                envelope.get()
            ),
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// The thickness of the hydrogen-helium envelope of a planet of
/// `total_mass`, `envelope` of it in hydrogen and helium, under the
/// bolometric `flux` of its star at `age`: the table radius at 20 mbar
/// minus Lopez & Fortney's core radius `M_core^0.25` (their Eq. 1, with
/// `M_core = (1 − envelope) total_mass`). The caller adds it to its own
/// core.
///
/// The radius is interpolated linearly in `log mass`, `log envelope`,
/// `log flux` and `log age`. The flux is clamped to 0.1 to 1000 F⊕ and the
/// age to 0.1 to 10 Gyr. A planet below [`MIN_MASS`] is read at
/// [`MIN_MASS`] (see `mass_read`); an envelope below [`LOWEST_TABULATED`]
/// scales the thickness at [`LOWEST_TABULATED`] down to zero (see
/// `below_the_tables`). The thickness is never negative: at every node the
/// table radius exceeds the core radius, and the core radius is convex in
/// `log mass`.
///
/// # Errors
///
/// [`EnvelopeError::OutOfTable`] when the mass is above [`MAX_MASS`] or not
/// a positive finite number, or the envelope above [`HIGHEST_TABULATED`].
pub fn envelope_thickness(
    total_mass: EarthMass,
    envelope: Fraction,
    flux: EarthFlux,
    age: Gyr,
) -> Result<EarthRadius, EnvelopeError> {
    let refused = EnvelopeError::OutOfTable {
        mass: total_mass,
        envelope,
    };
    if !(total_mass.0 > 0.0 && total_mass.0 <= MAX_MASS.0)
        || envelope.get() > HIGHEST_TABULATED.get()
    {
        return Err(refused);
    }
    let grid = &*GRID;
    let mass = mass_read(total_mass);
    let f = envelope.get().max(LOWEST_TABULATED.get());
    let axes = [
        bracket(&grid.log_ages, clamp(age.0, &grid.ages).log10()),
        bracket(&grid.log_fluxes, clamp(flux.0, &grid.fluxes).log10()),
        bracket(&grid.log_masses, mass.0.log10()),
        bracket(&grid.log_envelopes, f.log10()),
    ];
    let [Some(ages), Some(fluxes), Some(masses), Some(envelopes)] = axes else {
        return Err(refused);
    };
    let mut radius = 0.0;
    for &(a, wa) in &ages {
        for &(l, wl) in &fluxes {
            for &(m, wm) in &masses {
                for &(e, we) in &envelopes {
                    radius += wa * wl * wm * we * grid.radius(a, l, m, e);
                }
            }
        }
    }
    let core = (mass.0 * (1.0 - f)).powf(CORE_RADIUS_EXPONENT);
    Ok(EarthRadius(below_the_tables(radius - core, envelope)))
}

/// Calibration: a planet lighter than the tables' 1 M⊕ is read at 1 M⊕
/// (plan call of the planet model PR 2a: it keeps the radius continuous
/// at 1 M⊕ and underestimates a light planet's envelope). No model of
/// Lopez & Fortney goes below 1 M⊕. What was searched and not found: the
/// page [What are planets made of?](../../../docs/science/planet-interiors.md),
/// section on mini-Neptunes (written with the planet model PR 2a).
fn mass_read(mass: EarthMass) -> EarthMass {
    EarthMass(mass.0.max(MIN_MASS.0))
}

/// Calibration: below the tables' lowest envelope (1e-4 of the mass) the
/// thickness at 1e-4 is scaled by `envelope / 1e-4`, linearly to zero
/// with no envelope. The tables stop at 0.01 % and no model of a thinner
/// envelope was found; the rule gives no thickness without gas and is
/// continuous at 1e-4. What was searched and not found: the page
/// [What are planets made of?](../../../docs/science/planet-interiors.md),
/// section on mini-Neptunes (written with the planet model PR 2a).
fn below_the_tables(thickness_at_lowest: f64, envelope: Fraction) -> f64 {
    thickness_at_lowest * (envelope.get() / LOWEST_TABULATED.get()).min(1.0)
}

/// The table as a full grid over age, flux, mass and envelope.
#[derive(Debug)]
struct Grid {
    /// Ages, Gyr, increasing.
    ages: Vec<f64>,
    log_ages: Vec<f64>,
    /// Fluxes, F⊕, increasing.
    fluxes: Vec<f64>,
    log_fluxes: Vec<f64>,
    /// Decimal logarithms of the masses (M⊕), increasing.
    log_masses: Vec<f64>,
    /// Decimal logarithms of the envelope fractions (not percent),
    /// increasing.
    log_envelopes: Vec<f64>,
    /// Radii in R⊕, indexed by age, then flux, then mass, then envelope.
    radii: Vec<f64>,
}

impl Grid {
    fn index(&self, age: usize, flux: usize, mass: usize, envelope: usize) -> usize {
        ((age * self.fluxes.len() + flux) * self.log_masses.len() + mass) * self.log_envelopes.len()
            + envelope
    }

    fn radius(&self, age: usize, flux: usize, mass: usize, envelope: usize) -> f64 {
        self.radii[self.index(age, flux, mass, envelope)]
    }
}

/// The parsed table, built on first use.
static GRID: LazyLock<Grid> = LazyLock::new(|| parse(TABLE));

/// Parses the table; a malformed line, or a grid with a missing or
/// repeated cell, panics.
fn parse(text: &str) -> Grid {
    const FILE: &str = "mini-neptunes-lopez2014.tsv";
    let mut rows: Vec<(usize, [f64; 5])> = Vec::new();
    let mut header_seen = false;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if !header_seen {
            header_seen = true;
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 5 {
            panic!("{FILE} line {number}: {} fields, expected 5", fields.len());
        }
        let values = std::array::from_fn(|i| match fields[i].trim().parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => v,
            _ => panic!(
                "{FILE} line {number}: {:?} is not a valid number",
                fields[i]
            ),
        });
        rows.push((number, values));
    }

    let axis = |column: usize| {
        let mut values: Vec<f64> = rows.iter().map(|(_, r)| r[column]).collect();
        values.sort_by(f64::total_cmp);
        values.dedup();
        values
    };
    let (ages, fluxes, masses, percents) = (axis(0), axis(1), axis(2), axis(3));
    let cells = ages.len() * fluxes.len() * masses.len() * percents.len();
    if rows.len() != cells {
        panic!("{FILE}: {} rows, expected {cells}", rows.len());
    }
    let log = |axis: &[f64]| axis.iter().map(|v| v.log10()).collect::<Vec<f64>>();
    let mut grid = Grid {
        log_ages: log(&ages),
        log_fluxes: log(&fluxes),
        log_masses: log(&masses),
        log_envelopes: percents.iter().map(|p| (p / PER_CENT).log10()).collect(),
        radii: vec![f64::NAN; cells],
        ages,
        fluxes,
    };
    let position = |axis: &[f64], v: f64| axis.iter().position(|&g| g == v).expect("on its axis");
    let mut seen = vec![false; cells];
    for (number, [age, flux, mass, percent, radius]) in rows {
        let index = grid.index(
            position(&grid.ages, age),
            position(&grid.fluxes, flux),
            position(&masses, mass),
            position(&percents, percent),
        );
        if std::mem::replace(&mut seen[index], true) {
            panic!("{FILE} line {number}: cell repeated");
        }
        grid.radii[index] = radius;
    }
    grid
}

#[cfg(test)]
#[path = "envelope_tests.rs"]
mod tests;
