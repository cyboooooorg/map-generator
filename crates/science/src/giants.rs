//! Radius and internal luminosity of a giant planet, read in the tables of
//! Fortney, Marley & Barnes 2007 (ApJ 659, 1661), and the radius of their
//! ice-rock core (their Eq. 7).
//!
//! The table is `crates/science/data/giants-fortney2007.tsv`, the 25 `.dat`
//! files of the authors' page as published; its provenance, units and
//! validity limits are in `crates/science/data/GIANTS.md`. The study behind
//! the choices is
//! `docs/superpowers/studies/2026-11-21-planet-model-2a/giants-and-envelopes.md`
//! (§1.3 interpolation and limits, §1.4 core radius, §1.6 test values).
//!
//! The radius is at 1 bar and uninflated (no extra heating of hot
//! Jupiters); the luminosity is the model's internal luminosity `L`.

use crate::constants::EARTH_EQUATORIAL_RADIUS;
use crate::units::{EarthFlux, EarthMass, EarthRadius, Gyr, Metre, Watt};
use std::sync::LazyLock;

/// The Fortney et al. 2007 table, 4 550 rows.
const TABLE: &str = include_str!("../data/giants-fortney2007.tsv");

/// Source: Jupiter's equatorial radius at 1 bar, 71 492 km (NASA Jupiter
/// Fact Sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/, read 2026-11-21):
/// the R_J of Fortney et al. 2007 (study `giants-and-envelopes.md`,
/// Conventions), the unit of the table's radii.
pub const JUPITER_RADIUS: Metre = Metre(71_492e3);

/// Definition: watts in one erg per second (1 erg = 1e-7 J, CGS to SI).
const WATTS_PER_ERG_PER_SECOND: f64 = 1e-7;

/// Source: lower mass bound of the Thorngren et al. 2016 heavy-element fit
/// ("The Mass-Metallicity Relation for Giant Planets", ApJ 831, 64,
/// arXiv:1511.07854, abstract: "20 M_earth < M < 20 M_Jup"), the edge
/// between ice and gas giants (study `draw-bounds.md`, Decisions, 1).
pub const GAS_GIANT_MIN_MASS: EarthMass = EarthMass(20.0);

/// Source: Fortney et al. 2007, the core of every giant of Tables 2 to 4
/// and of the `.dat` files: 50/50 ice and rock by mass (study §1.1).
const ICE_MASS_FRACTION: f64 = 0.5;

/// Source: Fortney et al. 2007 Eq. 7 (arXiv:astro-ph/0612671 v4, which
/// corrects the coefficients), for an ice-rock sphere:
/// `R = (a1 imf + a0) (log M)^2 + (b1 imf + b0) log M + (c1 imf + c0)`,
/// `R` in Earth radii, `M` in Earth masses, `log` decimal (study §1.4).
/// Each pair is `[x1, x0]`, in the order a, b, c.
const EQ_7: [[f64; 2]; 3] = [[0.0912, 0.1603], [0.3330, 0.7387], [0.4639, 1.1193]];

/// Definition: the age after which [`core_bounds`] asks every distance to
/// bracket the mass. The 0.1 Gyr block itself is left out: at 0.02 au, a
/// 10 M⊕ core starts at 21.54 M⊕ there, and such a missing corner falls
/// back to the next age (see [`giant`]).
const CORE_BOUNDS_AFTER_AGE: Gyr = Gyr(0.1);

/// A giant planet read in the tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Giant {
    /// Radius at 1 bar.
    pub radius: EarthRadius,
    /// Internal luminosity (the heat the interior loses).
    pub luminosity: Watt,
    /// The age the tables were read at: the age asked, clamped to the
    /// tables' 1 Myr to 10 Gyr. When an interpolation corner had no row and
    /// fell back to an older age, the geometric mean of the ages actually
    /// read, weighted as the corners.
    pub age_used: Gyr,
    /// The mass the tables were read at: the mass asked, or the tables'
    /// top mass (3 593.81 M⊕) for a heavier giant (see [`mass_read`]).
    pub mass_used: EarthMass,
}

/// Why a giant cannot be read in the tables.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GiantError {
    /// The mass or the core lies outside the rows of the tables: a core
    /// outside 0 to 100 M⊕, or a mass that a corner of the interpolation
    /// has no row for at its age nor at any older one.
    OutOfTable { mass: EarthMass, core: EarthMass },
}

impl std::fmt::Display for GiantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfTable { mass, core } => write!(
                f,
                "no Fortney et al. 2007 row for {} M⊕ with a {} M⊕ core",
                mass.0, core.0
            ),
        }
    }
}

impl std::error::Error for GiantError {}

/// The radius, internal luminosity and age read of a giant of `mass` with a
/// heavy-element core of `core`, at `age`, under the bolometric `flux` of
/// its star.
///
/// The tables are for a solar-luminosity star; another star is read at the
/// flux-equivalent distance `d_eq = 1 au / sqrt(flux / F⊕)` (study §1.3).
/// Radius and `log L` are interpolated linearly in `log mass`, `log age`,
/// `core` and `log d_eq`. The age is clamped to 1 Myr to 10 Gyr and `d_eq`
/// to 0.02 to 9.5 au; a mass above the tables is read at their top mass
/// ([`mass_read`], reported in [`Giant::mass_used`]); a lighter mass and
/// the core are never extrapolated. An
/// interpolation corner whose block has no row at the mass (a young,
/// irradiated, light model the authors excluded) takes the nearest older
/// age that has one, and [`Giant::age_used`] says so.
///
/// # Errors
///
/// [`GiantError::OutOfTable`] when the core is outside 0 to 100 M⊕, or a
/// corner has no row at the mass at its age or any older one.
pub fn giant(
    mass: EarthMass,
    core: EarthMass,
    age: Gyr,
    flux: EarthFlux,
) -> Result<Giant, GiantError> {
    let refused = GiantError::OutOfTable { mass, core };
    let grid = &*GRID;
    if !(mass.0 > 0.0 && mass.0.is_finite()) {
        return Err(refused);
    }
    let cores = bracket(&grid.cores, core.0).ok_or(refused)?;
    let age = clamp(age.0, &grid.ages);
    let ages = bracket(&grid.log_ages, age.log10()).ok_or(refused)?;
    let d_eq = clamp(1.0 / flux.0.sqrt(), &grid.distances);
    let distances = bracket(&grid.log_distances, d_eq.log10()).ok_or(refused)?;
    let mass_used = mass_read(mass, grid);
    let log_mass = mass_used.0.log10();

    let (mut radius_rj, mut log_l, mut log_age_read) = (0.0, 0.0, 0.0);
    let mut fell_back = false;
    // The one age every corner read at, if they all read at the same one.
    let mut one_age: Option<Option<usize>> = None;
    for &(c, wc) in &cores {
        for &(d, wd) in &distances {
            for &(a, wa) in &ages {
                let weight = wc * wd * wa;
                let (read_at, row) = (a..grid.ages.len())
                    .find_map(|older| grid.block(c, d, older).at(log_mass).map(|r| (older, r)))
                    .ok_or(refused)?;
                fell_back |= read_at != a;
                one_age = match one_age {
                    None => Some(Some(read_at)),
                    Some(Some(seen)) if seen == read_at => Some(Some(seen)),
                    Some(_) => Some(None),
                };
                radius_rj += weight * row.radius_rj;
                log_l += weight * row.log_l;
                log_age_read += weight * grid.log_ages[read_at];
            }
        }
    }
    Ok(Giant {
        radius: EarthRadius(radius_rj * JUPITER_RADIUS.0 / EARTH_EQUATORIAL_RADIUS.0),
        luminosity: Watt(10f64.powf(log_l) * WATTS_PER_ERG_PER_SECOND),
        age_used: Gyr(match (fell_back, one_age.flatten()) {
            (false, _) => age,
            (true, Some(read_at)) => grid.ages[read_at],
            (true, None) => 10f64.powf(log_age_read),
        }),
        mass_used,
    })
}

/// The lowest and highest tabulated cores (0, 10, 25, 50 or 100 M⊕) whose
/// rows bracket `mass` at every distance and every tabulated age after
/// 0.1 Gyr: the range a giant's core is drawn in. [`giant`] reads any core
/// between them at every age and flux, a young corner without its row
/// falling back to an older age.
///
/// A mass above the tables is read at their top mass, as in [`giant`].
/// `None` when no core qualifies (below 12.92 M⊕): [`giant`] refuses such
/// a mass at some age or distance.
pub fn core_bounds(mass: EarthMass) -> Option<(EarthMass, EarthMass)> {
    let grid = &*GRID;
    let log_mass = mass_read(mass, grid).0.log10();
    let first_age = grid.ages.partition_point(|&a| a <= CORE_BOUNDS_AFTER_AGE.0);
    let covered: Vec<f64> = (0..grid.cores.len())
        .filter(|&c| {
            (0..grid.distances.len()).all(|d| {
                (first_age..grid.ages.len()).all(|a| grid.block(c, d, a).at(log_mass).is_some())
            })
        })
        .map(|c| grid.cores[c])
        .collect();
    Some((EarthMass(*covered.first()?), EarthMass(*covered.last()?)))
}

/// Calibration: a giant heavier than the tables' top mass (3 593.81 M⊕,
/// 11.3 M_J) is read at that mass. Giants are drawn up to about 13 M_J
/// (4 130 M⊕) and no table of Fortney et al. 2007 goes beyond; their
/// radius is nearly flat there (at 4.5 Gyr, 9.5 au and no core, 1.055 to
/// 1.023 R_J from 2 154 to 3 594 M⊕, Table 4), so the top row stands in
/// for the heavier ones. What was searched and not found: the page
/// [What are planets made of?](../../../docs/science/planet-interiors.md),
/// section on giants (written with the planet model PR 2a).
fn mass_read(mass: EarthMass, grid: &Grid) -> EarthMass {
    EarthMass(mass.0.min(grid.top_mass))
}

/// The radius of the 50/50 ice-rock core of Fortney et al. 2007's giants,
/// by their Eq. 7 (decimal logarithm, study §1.4). The fit holds over
/// 0.01 to 100 M⊕, to 2.5 % on average against their Table 1.
pub fn ice_rock_core_radius(mass: EarthMass) -> EarthRadius {
    let x = mass.0.log10();
    let [a, b, c] = EQ_7.map(|[slope, offset]| slope * ICE_MASS_FRACTION + offset);
    EarthRadius(a * x * x + b * x + c)
}

/// One row of a block, the mass and luminosity as decimal logarithms.
#[derive(Clone, Copy, Debug)]
struct Row {
    log_mass: f64,
    radius_rj: f64,
    /// Decimal logarithm of `L` in erg/s.
    log_l: f64,
}

/// The rows of one core, distance and age, by increasing mass.
#[derive(Debug, Default)]
struct Block(Vec<Row>);

impl Block {
    /// The row at `log_mass`, interpolated linearly between its neighbours;
    /// `None` outside the block's masses.
    fn at(&self, log_mass: f64) -> Option<Row> {
        let rows = &self.0;
        let (first, last) = (rows.first()?, rows.last()?);
        if !(log_mass >= first.log_mass && log_mass <= last.log_mass) {
            return None;
        }
        let i = rows.partition_point(|r| r.log_mass <= log_mass);
        if i == rows.len() {
            return Some(*last);
        }
        let (lo, hi) = (rows[i - 1], rows[i]);
        let t = (log_mass - lo.log_mass) / (hi.log_mass - lo.log_mass);
        Some(Row {
            log_mass,
            radius_rj: lo.radius_rj + t * (hi.radius_rj - lo.radius_rj),
            log_l: lo.log_l + t * (hi.log_l - lo.log_l),
        })
    }
}

/// The table as a grid of blocks over core, distance and age.
#[derive(Debug)]
struct Grid {
    /// Core masses, M⊕, increasing.
    cores: Vec<f64>,
    /// Distances, au, increasing.
    distances: Vec<f64>,
    log_distances: Vec<f64>,
    /// Ages, Gyr, increasing.
    ages: Vec<f64>,
    log_ages: Vec<f64>,
    /// The largest mass of any row, M⊕.
    top_mass: f64,
    /// Indexed by core, then distance, then age.
    blocks: Vec<Block>,
}

impl Grid {
    fn index(&self, core: usize, distance: usize, age: usize) -> usize {
        (core * self.distances.len() + distance) * self.ages.len() + age
    }

    fn block(&self, core: usize, distance: usize, age: usize) -> &Block {
        &self.blocks[self.index(core, distance, age)]
    }
}

/// The parsed table, built on first use.
static GRID: LazyLock<Grid> = LazyLock::new(|| parse(TABLE));

/// `x` clamped to the first and last values of the increasing `grid`.
fn clamp(x: f64, grid: &[f64]) -> f64 {
    x.max(grid[0]).min(grid[grid.len() - 1])
}

/// The one or two indices of the increasing `grid` around `x`, with their
/// linear weights, a zero weight left out; `None` outside the grid.
fn bracket(grid: &[f64], x: f64) -> Option<Vec<(usize, f64)>> {
    if !(x >= grid[0] && x <= grid[grid.len() - 1]) {
        return None;
    }
    let i = grid.partition_point(|&g| g <= x);
    if i == grid.len() || grid[i - 1] == x {
        return Some(vec![(i - 1, 1.0)]);
    }
    let t = (x - grid[i - 1]) / (grid[i] - grid[i - 1]);
    Some(vec![(i - 1, 1.0 - t), (i, t)])
}

/// Parses the table; a malformed line panics with its line number.
fn parse(text: &str) -> Grid {
    const FILE: &str = "giants-fortney2007.tsv";
    let mut rows: Vec<(usize, [f64; 6])> = Vec::new();
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
        if fields.len() != 6 {
            panic!("{FILE} line {number}: {} fields, expected 6", fields.len());
        }
        // Every column is positive but the core, which may be zero.
        let values = std::array::from_fn(|i| match fields[i].trim().parse::<f64>() {
            Ok(v) if v.is_finite() && (v > 0.0 || (i == 0 && v == 0.0)) => v,
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
    let (cores, distances, ages) = (axis(0), axis(1), axis(2));
    let mut grid = Grid {
        log_distances: distances.iter().map(|d| d.log10()).collect(),
        log_ages: ages.iter().map(|a| a.log10()).collect(),
        blocks: (0..cores.len() * distances.len() * ages.len())
            .map(|_| Block::default())
            .collect(),
        top_mass: rows.iter().map(|(_, r)| r[3]).fold(0.0, f64::max),
        cores,
        distances,
        ages,
    };
    let position = |axis: &[f64], v: f64| axis.iter().position(|&g| g == v).expect("on its axis");
    for (number, [core, distance, age, mass, radius_rj, luminosity]) in rows {
        let index = grid.index(
            position(&grid.cores, core),
            position(&grid.distances, distance),
            position(&grid.ages, age),
        );
        let block = &mut grid.blocks[index].0;
        let log_mass = mass.log10();
        if block.last().is_some_and(|r| r.log_mass >= log_mass) {
            panic!("{FILE} line {number}: mass {mass} does not increase");
        }
        block.push(Row {
            log_mass,
            radius_rj,
            log_l: luminosity.log10(),
        });
    }
    if let Some(i) = grid.blocks.iter().position(|b| b.0.len() < 2) {
        panic!("{FILE}: block {i} (core, distance, age) has fewer than two rows");
    }
    grid
}

#[cfg(test)]
#[path = "giants_tests.rs"]
mod tests;
