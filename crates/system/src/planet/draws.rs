//! The draws of a planet's composition, each from its own sub-seed of the
//! trial seed (`planet/cmf`, `planet/water`, `planet/radiogenic`), so that
//! changing one draw never moves another.
//!
//! The distributions are written by hand on a ChaCha8 stream: a uniform
//! number in the open interval (0, 1) and a normal number by the Box-Muller
//! transform, nothing else. Same-platform results are bit-identical.
//!
//! The draw parameters live here, not in `science`: they are properties of
//! how the engine draws a planet, not laws of physics.

use super::codes;
use super::interior::InteriorError;
use crate::Trial;
use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};
use science::units::{Dex, Fraction};
use timeline::{Code, Provenance, Seed, Trace, Traced};

/// Source: Plotnykov & Valencia 2020, MNRAS 499, 932, abstract: the core
/// mass fraction inferred from stars is 0.32 +0.14/-0.12 (and 0.325 for
/// Earth, Wang et al. 2018). Read as the median and the 68 % interval
/// (16th and 84th percentiles), the median of the draw.
const CMF_MEDIAN: f64 = 0.32;

/// Source: Plotnykov & Valencia 2020, abstract: the lower uncertainty,
/// 0.12, the distance from the median to the 16th percentile (a 68 %
/// interval), so the scale of the draw below the median.
const CMF_SCALE_BELOW: f64 = 0.12;

/// Source: Plotnykov & Valencia 2020, abstract: the upper uncertainty,
/// 0.14, the distance from the median to the 84th percentile, so the
/// scale of the draw above the median.
const CMF_SCALE_ABOVE: f64 = 0.14;

/// Source: Marty 2012, Earth Planet. Sci. Lett. 313, 56, p. 18: the bulk
/// Earth holds 2700 +/- 1350 ppm of water by mass, the median of the dry
/// draw.
const DRY_WATER_MEDIAN: f64 = 2.7e-3;

/// Calibration: sigma of the natural logarithm of the dry water fraction,
/// 0.5, derived from Marty 2012's 2700 +/- 1350 ppm read as one sigma
/// (1350 / 2700, the small-spread approximation of a log-normal).
const DRY_WATER_SIGMA_LN: f64 = 0.5;

/// Source: Zeng et al. 2019, PNAS 116, 9723, p. 7 and p. 20: the ice
/// fraction of icy planets is uniform between 1/2 (rock to water 1:1) and
/// 2/3 (solar rock to water ratio, or clathrate and hydrate ices); the
/// lower bound.
const ICY_WATER_MIN: f64 = 0.5;

/// Source: Zeng et al. 2019, PNAS 116, 9723, p. 7 and p. 20: the upper
/// bound of the ice fraction of icy planets, 2/3 (rounded to 0.67).
const ICY_WATER_MAX: f64 = 0.67;

/// Calibration: chance that a planet inside the ice line formed beyond it
/// and migrated in with its ices, 0.1, as in legacy. Searched: a
/// published fraction of migrated water-rich planets; none found (Luque &
/// Palle 2022 say they exist, with no fraction in the main text; study
/// `heat-and-composition.md` section B7).
const MIGRATION_CHANCE: f64 = 0.1;

/// Source: Nimmo et al. 2020, p. 5: a 10-90 % range of [Eu/Mg] of about
/// -0.3 to +0.3 dex among likely planet hosts, i.e. a sigma of 0.3 / 1.28
/// = 0.23 dex if Gaussian (derived from that range; Unterborn et al. 2015, whose Th
/// ranges from -0.23 to +0.40 dex, agrees). The spread of the radiogenic
/// abundance, dex of Earth's.
const RADIOGENIC_SIGMA_DEX: f64 = 0.23;

/// Definition: a mass fraction is at least 0.
const MASS_FRACTION_MIN: f64 = 0.0;

/// Definition: a mass fraction is at most 1.
const MASS_FRACTION_MAX: f64 = 1.0;

/// Definition: bits of the mantissa of an `f64`, so that 53 random bits
/// make a uniform number of the grid `k / 2^53`.
const MANTISSA_BITS: u32 = 53;

/// The composition a trial draws; its nodes are leaves of the trace.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Drawn {
    /// Core mass fraction, of the whole planet.
    pub core_mass_fraction: Traced<Fraction>,
    /// Water mass fraction, of the whole planet.
    pub water_fraction: Traced<Fraction>,
    /// Radiogenic abundance, dex of Earth's.
    pub radiogenic_abundance: Traced<Dex>,
}

/// The labels the sub-seeds are derived with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Labels {
    pub core: &'static str,
    pub water: &'static str,
    pub radiogenic: &'static str,
}

/// The labels of the engine.
pub(crate) const LABELS: Labels = Labels {
    core: "planet/cmf",
    water: "planet/water",
    radiogenic: "planet/radiogenic",
};

/// Draws the composition of `trial` and records it.
pub(crate) fn draw(trial: &Trial, trace: &mut Trace) -> Result<Drawn, InteriorError> {
    draw_with(trial, trace, &LABELS)
}

/// [`draw`] with other labels, to show the draws are independent.
pub(crate) fn draw_with(
    trial: &Trial,
    trace: &mut Trace,
    labels: &Labels,
) -> Result<Drawn, InteriorError> {
    let beyond = trial.orbit.semi_major_axis.value().0 >= trial.ice_line.value().0;
    let cmf = core_mass_fraction(trial.seed.derive(labels.core));
    let water = water_fraction(trial.seed.derive(labels.water), beyond);
    let radiogenic = radiogenic_abundance(trial.seed.derive(labels.radiogenic));
    record(trace, cmf, water, radiogenic)
}

fn record(
    trace: &mut Trace,
    cmf: f64,
    water: f64,
    radiogenic: f64,
) -> Result<Drawn, InteriorError> {
    let cmf = Fraction::try_from(cmf).map_err(InteriorError::NotAFraction)?;
    let water = Fraction::try_from(water).map_err(InteriorError::NotAFraction)?;
    Ok(Drawn {
        core_mass_fraction: trace.leaf(
            Code::of(codes::CORE_MASS_FRACTION),
            cmf,
            Provenance::Drawn,
        )?,
        water_fraction: trace.leaf(Code::of(codes::WATER_FRACTION), water, Provenance::Drawn)?,
        radiogenic_abundance: trace.leaf(
            Code::of(codes::RADIOGENIC_ABUNDANCE),
            Dex(radiogenic),
            Provenance::Drawn,
        )?,
    })
}

/// A uniform number in the open interval (0, 1).
fn uniform(rng: &mut ChaCha8Rng) -> f64 {
    let grid = 1.0 / (1u64 << MANTISSA_BITS) as f64;
    ((rng.next_u64() >> (u64::BITS - MANTISSA_BITS)) as f64 + 0.5) * grid
}

/// A standard normal number, by the Box-Muller transform (Box & Muller
/// 1958, Ann. Math. Stat. 29, 610), two uniforms per number.
fn normal(rng: &mut ChaCha8Rng) -> f64 {
    let radius = (-2.0 * uniform(rng).ln()).sqrt();
    radius * (std::f64::consts::TAU * uniform(rng)).cos()
}

/// The core mass fraction: `0.32 + 0.12 z` for `z < 0` and `0.32 + 0.14 z`
/// otherwise, `z` standard normal: half the mass on each side of the
/// median 0.32, the 16th percentile at 0.20 and the 84th at 0.46.
/// Clamped to a mass fraction, 0 to 1.
pub(crate) fn core_mass_fraction(seed: Seed) -> f64 {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
    let z = normal(&mut rng);
    let scale = if z < 0.0 {
        CMF_SCALE_BELOW
    } else {
        CMF_SCALE_ABOVE
    };
    (CMF_MEDIAN + scale * z).clamp(MASS_FRACTION_MIN, MASS_FRACTION_MAX)
}

/// The water fraction: beyond the ice line, or with the migration chance
/// inside it, uniform from 0.5 to 0.67; otherwise log-normal around
/// 2700 ppm. The three draws (normal, uniform, uniform) are made in every
/// case, so the stream does not depend on the branch.
pub(crate) fn water_fraction(seed: Seed, beyond_ice_line: bool) -> f64 {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
    let z = normal(&mut rng);
    let icy = ICY_WATER_MIN + (ICY_WATER_MAX - ICY_WATER_MIN) * uniform(&mut rng);
    let migrated = uniform(&mut rng) < MIGRATION_CHANCE;
    if beyond_ice_line || migrated {
        icy
    } else {
        DRY_WATER_MEDIAN * (DRY_WATER_SIGMA_LN * z).exp()
    }
}

/// The radiogenic abundance, dex of Earth's: a normal of mean 0 and sigma
/// 0.23.
pub(crate) fn radiogenic_abundance(seed: Seed) -> f64 {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
    RADIOGENIC_SIGMA_DEX * normal(&mut rng)
}

#[cfg(test)]
impl Drawn {
    /// A composition set by hand, for a test that needs Earth's.
    pub(crate) fn fixed(
        trace: &mut Trace,
        core_mass_fraction: f64,
        water_fraction: f64,
        radiogenic_abundance: f64,
    ) -> Drawn {
        record(
            trace,
            core_mass_fraction,
            water_fraction,
            radiogenic_abundance,
        )
        .unwrap()
    }
}
