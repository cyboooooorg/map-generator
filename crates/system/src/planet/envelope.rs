//! The primordial hydrogen-helium envelope of a planet, by mass regime.
//!
//! One uniform number per draw whatever the regime, so the stream never
//! depends on it. The regimes follow `draw-bounds.md` section 4 of the
//! planet model 2a studies: none below 0.1 M_E, Lammer et al. 2014's
//! nebula-captured envelopes up to 5 M_E, mini-Neptunes up to 10 M_E, ice
//! giants up to 20 M_E, and from there a gas giant's heavy-element mass,
//! the envelope being the rest. The draw is the envelope before any loss
//! after the disc disperses.

use super::interior::InteriorError;
use crate::random::uniform;
use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::constants::{EARTH_MASS_PARAMETER, JUPITER_MASS_PARAMETER};
use science::envelope::HIGHEST_TABULATED;
use science::giants::{GAS_GIANT_MIN_MASS, core_bounds};
use science::units::EarthMass;
use timeline::Seed;

/// Calibration: 0.1 M_E, the lowest mass Lammer et al. 2014
/// (arXiv:1401.2765, abstract) model with a captured envelope; below it
/// no envelope is drawn. Searched: a measured or modelled threshold below
/// which no hydrogen is captured; none found, only Stoekl et al. 2015
/// (arXiv:1505.06859) saying that near Mars's mass the atmosphere changes
/// fast (`draw-bounds.md` section 4a, "Mass below which no envelope is
/// kept").
const NO_ENVELOPE_BELOW: f64 = 0.1;

/// Source: Lammer et al. 2014, "Origin and loss of nebula-captured
/// hydrogen envelopes from sub- to super-Earths in the habitable zone of
/// Sun-like stars" (MNRAS, arXiv:1401.2765), Tables 1 to 3 p. 4-5: for a
/// rocky core of each mass (M_E) in a nebula at 1 au, the minimum and
/// maximum envelope mass fraction over the grid of dust depletion
/// (0.001 to 0.1) and accretion rate (1e-5 to 1e-8 per year), as tabulated
/// in `draw-bounds.md` section 4a. Rows: (core mass, minimum, maximum).
const LAMMER_2014: [(f64, f64, f64); 6] = [
    (0.1, 4.4e-8, 1.0e-4),
    (0.5, 1.5e-6, 5.9e-3),
    (1.0, 8.1e-6, 2.5e-2),
    (2.0, 6.1e-5, 0.11),
    (3.0, 2.0e-4, 0.33),
    (5.0, 8.7e-4, 0.35),
];

/// Source: Lopez & Fortney 2014 (ApJ 792, 1, arXiv:1311.0329) p. 8: "there
/// is a fundamental change in the relationship around ~10 M_earth", above
/// which the envelope fraction rises steadily toward the gas giants. The
/// upper edge of the mini-Neptune regime (`draw-bounds.md` section 4b).
const MINI_NEPTUNE_MAX_MASS: f64 = 10.0;

/// Source: Helled et al. 2011, "Interior Models of Uranus and Neptune"
/// (arXiv:1010.5546) p. 8: hydrogen and helium are 9 % to 25 % of
/// Uranus's and Neptune's masses (derived in `draw-bounds.md` section 4c);
/// 0.09 is the lowest. The lower bound of the ice giants' uniform draw.
const ICE_GIANT_ENVELOPE_MIN: f64 = 0.09;

/// Source: Thorngren et al. 2016, "The Mass-Metallicity Relation for Giant
/// Planets" (ApJ 831, 64, arXiv:1511.07854), section V.1 p. 9: a fit to
/// the log of 47 planets gives `Mz = (57.9 +/- 7.03) M^(0.61 +/- 0.08)`,
/// `Mz` in Earth masses and `M` in Jupiter masses (the paper's unit for
/// planet masses there; the only reading under which `Mz < M`). This is the
/// 57.9 M_E of heavy elements at one Jupiter mass.
const HEAVY_MASS_AT_ONE_JUPITER: f64 = 57.9;

/// Source: Thorngren et al. 2016 (arXiv:1511.07854), section V.1 p. 9, the
/// exponent of the fit above, 0.61.
const HEAVY_MASS_EXPONENT: f64 = 0.61;

/// Source: Thorngren et al. 2016 (arXiv:1511.07854), section V.1 p. 9: "the
/// intrinsic spread was the factor 10^sigma = 1.82", the 1-sigma scatter of
/// the data around the fit, as a factor.
const HEAVY_MASS_SPREAD: f64 = 1.82;

/// Calibration: the heavy-element mass is drawn within two sigma of the
/// fit, a factor `1.82^2` either side. The paper gives the 1-sigma scatter
/// and no extremes. Searched: a published range of heavy-element masses
/// per planet mass; none found (`draw-bounds.md` section 4d and
/// Decisions, 2).
const HEAVY_MASS_SIGMAS: i32 = 2;

/// What the envelope draw returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnvelopeDraw {
    /// Envelope mass fraction of the whole planet.
    pub fraction: f64,
    /// Total heavy-element mass of a gas giant (`None` below
    /// [`GAS_GIANT_MIN_MASS`]).
    pub heavy_mass: Option<EarthMass>,
}

/// The primordial envelope of a planet of `mass`, one uniform number from
/// `seed` in every regime.
///
/// - below 0.1 M_E: none;
/// - 0.1 to 5 M_E: log-uniform between Lammer et al. 2014's minimum and
///   maximum, each interpolated log-linearly in mass between the grid
///   masses, the maximum capped at the edge of the Lopez & Fortney 2014
///   tables (0.20);
/// - 5 to 10 M_E: log-uniform from the 5 M_E minimum to 0.20;
/// - 10 to 20 M_E: uniform from 0.09 to 0.20;
/// - from 20 M_E: heavy-element mass log-uniform within a factor `1.82^2`
///   of Thorngren et al. 2016's fit, inside the cores the giant tables
///   cover; the envelope is `1 - Mz / M`.
pub(crate) fn envelope_fraction(
    seed: Seed,
    mass: EarthMass,
) -> Result<EnvelopeDraw, InteriorError> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
    let u = uniform(&mut rng);
    let m = mass.0;
    let cap = HIGHEST_TABULATED.get();
    let last = LAMMER_2014[LAMMER_2014.len() - 1];
    let fraction = if m < NO_ENVELOPE_BELOW {
        0.0
    } else if m < last.0 {
        let (low, high) = lammer_bounds(m);
        log_uniform(u, low, high.min(cap))
    } else if m < MINI_NEPTUNE_MAX_MASS {
        log_uniform(u, last.1, cap)
    } else if m < GAS_GIANT_MIN_MASS.0 {
        ICE_GIANT_ENVELOPE_MIN + (cap - ICE_GIANT_ENVELOPE_MIN) * u
    } else {
        let heavy = heavy_mass(u, mass)?;
        return Ok(EnvelopeDraw {
            fraction: 1.0 - heavy.0 / m,
            heavy_mass: Some(heavy),
        });
    };
    Ok(EnvelopeDraw {
        fraction,
        heavy_mass: None,
    })
}

/// Lammer et al. 2014's minimum and maximum at `mass` (within the grid),
/// interpolated linearly in the logarithms of mass and of the fraction.
fn lammer_bounds(mass: f64) -> (f64, f64) {
    let upper = LAMMER_2014.partition_point(|row| row.0 <= mass);
    let (a, b) = (LAMMER_2014[upper - 1], LAMMER_2014[upper]);
    let t = (mass / a.0).ln() / (b.0 / a.0).ln();
    let at = |lo: f64, hi: f64| (lo.ln() + t * (hi / lo).ln()).exp();
    (at(a.1, b.1), at(a.2, b.2))
}

/// A log-uniform number from `low` to `high` for the uniform `u`.
fn log_uniform(u: f64, low: f64, high: f64) -> f64 {
    (low.ln() + (high / low).ln() * u).exp()
}

/// A gas giant's heavy-element mass: log-uniform within
/// `HEAVY_MASS_SPREAD^HEAVY_MASS_SIGMAS` of Thorngren et al. 2016's fit,
/// intersected with the cores of the giant tables.
fn heavy_mass(u: f64, mass: EarthMass) -> Result<EarthMass, InteriorError> {
    let (core_low, core_high) = core_bounds(mass).ok_or(InteriorError::NoHeavyMassRange(mass.0))?;
    let jupiter = JUPITER_MASS_PARAMETER / EARTH_MASS_PARAMETER;
    let fit = HEAVY_MASS_AT_ONE_JUPITER * (mass.0 / jupiter).powf(HEAVY_MASS_EXPONENT);
    let band = HEAVY_MASS_SPREAD.powi(HEAVY_MASS_SIGMAS);
    let low = (fit / band).max(core_low.0);
    let high = (fit * band).min(core_high.0).min(mass.0);
    if low >= high {
        return Err(InteriorError::NoHeavyMassRange(mass.0));
    }
    Ok(EarthMass(log_uniform(u, low, high)))
}
