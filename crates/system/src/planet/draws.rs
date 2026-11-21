//! The draws of a planet's composition, each from its own sub-seed of the
//! seed the caller passes (`planet/cmf`, `planet/water`,
//! `planet/radiogenic`), so that changing one draw never moves another.
//! `class_of` will pass the trial seed, `model` the body seed.
//!
//! Why a planet is a water world is recorded beside its water: a flag
//! derived from the orbit and the ice line (formed beyond it) and, inside
//! the ice line only, a drawn flag (migrated in from beyond it). The water
//! fraction is a drawn leaf of its own.
//!
//! Every draw is uniform between published bounds (log-uniform for the dry
//! water, which spans three decades): one uniform number in the open
//! interval (0, 1) per value, from a ChaCha8 stream. Same-platform results
//! are bit-identical.
//!
//! The draw parameters live here, not in `science`: they are properties of
//! how the engine draws a planet, not laws of physics.

use super::codes;
use super::interior::InteriorError;
use crate::Trial;
use crate::random::uniform;
use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::units::{Dex, Fraction};
use timeline::{Code, Provenance, Seed, Trace, Traced};

/// Source: Elkins-Tanton & Seager 2008, "Coreless Terrestrial Exoplanets"
/// (arXiv:0808.1908), section 2, p. 2-3: a planet may form with no
/// metallic core, so a core mass fraction of 0 is the physical floor
/// (`draw-bounds.md` section 1; a measured planet's fit reaches it, Brinkman
/// et al. 2024, arXiv:2409.08361, Table 2, p. 6). The lower bound of the
/// uniform draw.
const CMF_MIN: f64 = 0.0;

/// Source: Plotnykov & Valencia 2020, MNRAS 499, 932 (arXiv:2010.06480),
/// abstract and Fig. 8 caption p. 10: "what seems to be the maximum iron
/// enrichment that rocky planets attain during formation (rho0 ~ 6 and cmf
/// ~ 0.8)". The upper bound of the uniform draw.
const CMF_MAX: f64 = 0.8;

/// Source: Raymond, Quinn & Lunine 2007, Astrobiology 7
/// (arXiv:astro-ph/0510285), Table 1 p. 20, solar system rows (water
/// contents of Morbidelli et al. 2000): Mercury, the driest planet, 1e-5 of
/// its mass. The lower bound of the log-uniform dry draw
/// (`draw-bounds.md` section 2a; Izidoro & Piani 2022, arXiv:2302.02674
/// p. 9, give <= 7e-5 for the Vestan mantle).
const DRY_WATER_MIN: f64 = 1.0e-5;

/// Source: Izidoro & Piani 2022, "Origin of Water in the Terrestrial
/// Planets" (arXiv:2302.02674), p. 7: planets formed inside the snowline
/// are expected to hold "potentially lower than 1 wt.%" of water, 1e-2 of
/// their mass. The upper bound of the log-uniform dry draw.
const DRY_WATER_MAX: f64 = 1.0e-2;

/// Source: Marboeuf et al. 2014, A&A 570, A35 (arXiv:1407.7271), p. 11-12:
/// the ices of planetesimals beyond the snow line are 10 % to 60 % of
/// their total mass, so 0.1 (all ices, an upper estimate of the least
/// water share; `draw-bounds.md` section 2b). The lower bound of the
/// uniform icy draw.
const ICY_WATER_MIN: f64 = 0.1;

/// Source: Zeng et al. 2019, PNAS 116, 9723, p. 7 and p. 20: the ice
/// fraction of icy planets is at most 2/3 (solar rock to water ratio),
/// rounded to 0.67. The upper bound of the uniform icy draw.
const ICY_WATER_MAX: f64 = 0.67;

/// Calibration: chance that a planet inside the ice line formed beyond it
/// and migrated in with its ices, 0.1, as in legacy. Searched: a
/// published fraction of migrated water-rich planets; none found (Luque &
/// Palle 2022 say they exist, with no fraction in the main text; study
/// `heat-and-composition.md` section B7).
const MIGRATION_CHANCE: f64 = 0.1;

/// Source: Nimmo et al. 2020, arXiv:2011.04791, p. 5: the 1-99 % range of
/// [Eu/Mg] in the Hypatia catalogue is about -0.5 to +0.5 dex (30 % to
/// 300 %), "conservative" (appendix p. 13). The half-width of the uniform
/// radiogenic draw, dex of Earth's (`draw-bounds.md` section 3).
const RADIOGENIC_HALF_RANGE_DEX: f64 = 0.5;

/// The composition a trial draws. The core mass fraction, the water
/// fraction, the migration flag and the radiogenic abundance are drawn
/// leaves of the trace.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Drawn {
    /// Core mass fraction of the planet without its water: the core is
    /// `cmf (1 - w) M`, the share of the refractory mass.
    pub core_mass_fraction: Traced<Fraction>,
    /// Whether the planet formed at or beyond the ice line, derived from
    /// the semi-major axis and the ice line.
    pub formed_beyond_ice_line: Traced<bool>,
    /// Whether the planet migrated in from beyond the ice line, drawn with
    /// [`MIGRATION_CHANCE`]. `None` beyond the ice line, where there is
    /// nothing to migrate from (its uniform is still consumed).
    pub migrated: Option<Traced<bool>>,
    /// Water mass fraction of the whole planet (bulk water): icy for a
    /// water world, dry otherwise.
    pub water_fraction: Traced<Fraction>,
    /// Radiogenic abundance, dex of Earth's.
    pub radiogenic_abundance: Traced<Dex>,
}

impl Drawn {
    /// A water world: formed beyond the ice line or migrated from there.
    /// Its water is a layer of its own; a dry planet's stays bound in the
    /// mantle.
    pub(crate) fn is_water_world(&self) -> bool {
        *self.formed_beyond_ice_line.value() || self.migrated.is_some_and(|m| *m.value())
    }
}

/// The three values of the water sub-seed, drawn in this fixed order
/// whatever the branch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaterDraws {
    /// The dry water fraction, log-uniform from [`DRY_WATER_MIN`] to
    /// [`DRY_WATER_MAX`].
    pub dry: f64,
    /// The icy water fraction, uniform from [`ICY_WATER_MIN`] to
    /// [`ICY_WATER_MAX`].
    pub icy: f64,
    /// Whether the planet migrated in (it matters only inside the ice line).
    pub migrated: bool,
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

/// Draws the composition of `trial` from `seed` and records it. The
/// caller chooses the seed: `class_of` the trial's, `model` the body's.
pub(crate) fn draw(seed: Seed, trial: &Trial, trace: &mut Trace) -> Result<Drawn, InteriorError> {
    draw_with(seed, trial, trace, &LABELS)
}

/// [`draw`] with other labels, to show the draws are independent.
pub(crate) fn draw_with(
    seed: Seed,
    trial: &Trial,
    trace: &mut Trace,
    labels: &Labels,
) -> Result<Drawn, InteriorError> {
    let beyond = formed_beyond_ice_line(trial);
    let cmf = core_mass_fraction(seed.derive(labels.core));
    let water = water_draws(seed.derive(labels.water));
    let radiogenic = radiogenic_abundance(seed.derive(labels.radiogenic));
    let migrated = (!beyond).then_some(water.migrated);
    let fraction = if beyond || water.migrated {
        water.icy
    } else {
        water.dry
    };
    record(trial, trace, beyond, cmf, fraction, migrated, radiogenic)
}

/// Whether `trial` sits at or beyond its ice line.
fn formed_beyond_ice_line(trial: &Trial) -> bool {
    trial.orbit.semi_major_axis.value().0 >= trial.ice_line.value().0
}

fn record(
    trial: &Trial,
    trace: &mut Trace,
    beyond: bool,
    cmf: f64,
    water_fraction: f64,
    migrated: Option<bool>,
    radiogenic: f64,
) -> Result<Drawn, InteriorError> {
    let cmf = Fraction::try_from(cmf).map_err(InteriorError::NotAFraction)?;
    let fraction = Fraction::try_from(water_fraction).map_err(InteriorError::NotAFraction)?;
    let core_mass_fraction =
        trace.leaf(Code::of(codes::CORE_MASS_FRACTION), cmf, Provenance::Drawn)?;
    let formed_beyond_ice_line = trace.record(
        Code::of(codes::FORMED_BEYOND_ICE_LINE),
        beyond,
        &[trial.orbit.semi_major_axis.node(), trial.ice_line.node()],
    )?;
    let migrated = migrated
        .map(|m| trace.leaf(Code::of(codes::MIGRATED), m, Provenance::Drawn))
        .transpose()?;
    let water_fraction =
        trace.leaf(Code::of(codes::WATER_FRACTION), fraction, Provenance::Drawn)?;
    Ok(Drawn {
        core_mass_fraction,
        formed_beyond_ice_line,
        migrated,
        water_fraction,
        radiogenic_abundance: trace.leaf(
            Code::of(codes::RADIOGENIC_ABUNDANCE),
            Dex(radiogenic),
            Provenance::Drawn,
        )?,
    })
}

/// A uniform number from `low` to `high`.
fn uniform_between(rng: &mut ChaCha8Rng, low: f64, high: f64) -> f64 {
    low + (high - low) * uniform(rng)
}

/// The core mass fraction: uniform from [`CMF_MIN`] to [`CMF_MAX`], a share
/// of the refractory mass (the planet without its water).
pub(crate) fn core_mass_fraction(seed: Seed) -> f64 {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
    uniform_between(&mut rng, CMF_MIN, CMF_MAX)
}

/// The water values: the dry fraction (log-uniform), the icy fraction
/// (uniform) and the migration flag, drawn in this order whatever the
/// branch, so the stream does not depend on it.
pub(crate) fn water_draws(seed: Seed) -> WaterDraws {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
    let dry = uniform_between(&mut rng, DRY_WATER_MIN.ln(), DRY_WATER_MAX.ln()).exp();
    let icy = uniform_between(&mut rng, ICY_WATER_MIN, ICY_WATER_MAX);
    let migrated = uniform(&mut rng) < MIGRATION_CHANCE;
    WaterDraws { dry, icy, migrated }
}

/// The radiogenic abundance, dex of Earth's: uniform from -0.5 to +0.5.
pub(crate) fn radiogenic_abundance(seed: Seed) -> f64 {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.get());
    uniform_between(
        &mut rng,
        -RADIOGENIC_HALF_RANGE_DEX,
        RADIOGENIC_HALF_RANGE_DEX,
    )
}

#[cfg(test)]
impl Drawn {
    /// A composition set by hand for `trial`, for a test that needs Earth's
    /// or a given kind of planet; whether it formed beyond the ice line
    /// still comes from the trial. `migrated` is `None` beyond the ice line.
    pub(crate) fn fixed(
        trial: &Trial,
        trace: &mut Trace,
        core_mass_fraction: f64,
        water_fraction: f64,
        radiogenic_abundance: f64,
        migrated: Option<bool>,
    ) -> Drawn {
        record(
            trial,
            trace,
            formed_beyond_ice_line(trial),
            core_mass_fraction,
            water_fraction,
            migrated,
            radiogenic_abundance,
        )
        .unwrap()
    }
}
