//! The sourced laws the family proposers draw from, and the eccentricity
//! drawn once a trial is classified.
//!
//! Study: `docs/superpowers/studies/2026-11-21-architecture-families`
//! (`S` = `small-planets-and-spacing.md`, `G` = `giant-planets.md`);
//! `B9` = section B9 of
//! `docs/superpowers/studies/2026-11-21-planet-interior/heat-and-composition.md`.
//! A period law truncated at the stellar surface leaves the Roche floor
//! to the placement loop, which knows the classified density.

use super::{FULTON_OUTER_SUPPORT, Family, FamilyContext, HOT_JUPITER_MAX_PERIOD};
use crate::random::{
    beta, beta_half, broken_power_law, half_gaussian, log_normal, log_uniform, normal, rayleigh,
    uniform,
};
use rand_chacha::ChaCha8Rng;
use science::orbit::{self, ICE_LINE_OF_THE_SUN};
use science::units::{AstronomicalUnit, Day, EarthMass, Kilogram, SolarMass};
use timeline::PlanetClass;

/// Source: one Jupiter mass is 317.8 Earth masses (NASA Jupiter fact
/// sheet, ratio-to-Earth table; G 3.2, G 4.2).
pub(crate) const EARTH_MASSES_PER_JUPITER_MASS: f64 = 317.8;

/// Source: the lower bound of Neil & Rogers 2020's mass fit, 0.1 M⊕
/// (p3; B9).
pub(crate) const SMALL_MASS_MIN: EarthMass = EarthMass(0.1);

/// Source: the lower mass of a giant in CLS IV, "0.1 M_J or 30 M_earth",
/// roughly where runaway gas accretion can begin (Rosenthal 2023 p2;
/// G 4.2): a small planet stays below it.
pub(crate) const SMALL_MASS_MAX: EarthMass = EarthMass(30.0);

/// Source: Neil & Rogers 2020 Table 1 (p7), model 2 (envelope mass
/// loss): mean of `ln M`, M in Earth masses (B9; median e^1.00 = 2.7 M⊕).
pub(crate) const NEIL_ROGERS_MU: f64 = 1.00;

/// Source: Neil & Rogers 2020 Table 1 (p7), model 2: standard deviation
/// of `ln M` (B9).
pub(crate) const NEIL_ROGERS_SIGMA: f64 = 1.65;

/// Source: EPOS I Table 2 (p12), the period at which the innermost
/// planet of a multi-planet system peaks, 12 (+3/-2) d (S 1.2).
pub(crate) const INNERMOST_PEAK: Day = Day(12.0);

/// Source: EPOS I Table 2 (p12), index of `dN/dlog P` of the innermost
/// planet below its peak, 1.6 (+0.4/-0.2) (S 1.2).
pub(crate) const INNERMOST_BELOW: f64 = 1.6;

/// Source: EPOS I Table 2 (p12), index of `dN/dlog P` of the innermost
/// planet above its peak, -0.9 (+0.4/-0.5) (S 1.2).
pub(crate) const INNERMOST_ABOVE: f64 = -0.9;

/// Source: EPOS I eq. 11 and Table 2 (p5, p12), mean of `log10 D` of
/// adjacent pairs, `D = 2 (P^(2/3) - 1) / (P^(2/3) + 1)` (S 1.5).
pub(crate) const SPACING_LOG10_MEAN: f64 = -0.39;

/// Source: EPOS I Table 2 (p12), standard deviation of `log10 D`,
/// 0.18 (+0.05/-0.04) (S 1.5).
pub(crate) const SPACING_LOG10_SIGMA: f64 = 0.18;

/// Definition: `D` is the fractional separation `2 (a2 - a1) / (a1 + a2)`,
/// below 2 for any finite pair; at 2 and above no period ratio exists.
const SPACING_D_LIMIT: f64 = 2.0;

/// Source: the "wall" of Weiss 2018 (p8): essentially no adjacent pair
/// has a period ratio below 1.2 (S 1.5).
pub(crate) const WEISS_WALL: f64 = 1.2;

/// Source: He 2019 (p27), the fraction of planets near a mean-motion
/// resonance with another, f_mmr = 0.29 ± 0.04 in their clustered model;
/// applied per pair (S 1.5, study row 1).
pub(crate) const NEAR_RESONANCE_CHANCE: f64 = 0.29;

/// Source: He 2019 (p5), "near an MMR" is within 5 % exterior to the
/// resonance, e.g. 1.5-1.575 for 3:2 (S 1.5). The ratio is drawn
/// uniformly in that window (a Definition: no shape is given).
pub(crate) const NEAR_RESONANCE_WINDOW: f64 = 0.05;

/// Source: the 3:2 and 2:1 resonances, wide of which Fabrycky 2014 (p7)
/// finds the main peaks (S 1.5). Picked with odds 1/2 each (Definition,
/// a choice of the plan).
pub(crate) const RESONANCES: [f64; 2] = [1.5, 2.0];

/// Source: the masses of the rocky planets Raymond 2006 forms, 0.4-2.6
/// Earth masses (abstract p1; S 5). Log-uniform between them is a
/// Definition: the source gives the range only.
pub(crate) const ROCKY_MASS: (EarthMass, EarthMass) = (EarthMass(0.4), EarthMass(2.6));

/// Definition: Rayleigh scale of the rocky planets' eccentricities, a
/// shape around Raymond 2006's "about 0.05" (S 5).
pub(crate) const ROCKY_ECCENTRICITY_SIGMA: f64 = 0.05;

/// Source: Fang & Margot 2013 eq. 7 (p3), the shifted Rayleigh of the
/// spacing in mutual Hill radii starts at 3.5 (S 2.2).
pub(crate) const HILL_OFFSET: f64 = 3.5;

/// Source: Fang & Margot 2013 (p4), the scale of that shifted Rayleigh,
/// 14.5 (mean 21.7; S 2.2).
pub(crate) const HILL_SIGMA: f64 = 14.5;

/// Source: Fulton 2021 Table 1 (p6), the break of the giants' broken
/// power law in a, 3.6 (+2.0/-1.8) au for Sun-like hosts (G 3.1). Scaled
/// by the ice line (a Definition, study row 2).
pub(crate) const FULTON_BREAK: AstronomicalUnit = AstronomicalUnit(3.6);

/// Source: Fulton 2021 (p5), the index of `dN/dln a` inside the break,
/// 0.72 (+0.16/-0.20) (G 3.1).
pub(crate) const FULTON_INSIDE: f64 = 0.72;

/// Source: Fulton 2021 Table 1 (p6), the index of `dN/dln a` beyond the
/// break, -0.86 ± 0.41 (G 3.1).
pub(crate) const FULTON_BEYOND: f64 = -0.86;

/// Source: the range of `ln(a2/a1)` of adjacent giants: Rosenthal 2023's
/// sampling floor 0.5 (p8, a ratio of 1.65) and "less than 10" (p10)
/// (G 3.3). Uniform between them is a Definition: the study gives no
/// width for the peak near 3.
pub(crate) const NEXT_GIANT_LN_RATIO: (f64, f64) = (0.5, std::f64::consts::LN_10);

/// Source: Fulton 2021 (p6), giant occurrence is constant in ln M from
/// 30 to 900 Earth masses at 1-5 au (G 3.2): log-uniform.
pub(crate) const SOLAR_LIKE_GIANT_MASS: (EarthMass, EarthMass) =
    (EarthMass(30.0), EarthMass(900.0));

/// Source: a hot Jupiter weighs at least 0.25 M_J (Dawson & Johnson 2018
/// p2; G 1.1) and at most 13 M_J (study row 3). Log-uniform, in Jupiter
/// masses: the shape is a Definition.
pub(crate) const HOT_JUPITER_MASS: (f64, f64) = (0.25, 13.0);

/// Source: the outer companion of a hot Jupiter, 1-13 M_J (Dawson &
/// Johnson 2018 p32, combining Knutson 2014 and Bryan 2016; G 1.5).
/// Log-uniform, in Jupiter masses: the shape is a Definition.
pub(crate) const COMPANION_MASS: (f64, f64) = (1.0, 13.0);

/// Source: the outer companion's band, 1-20 au (Dawson & Johnson 2018
/// p32; G 1.5), scaled by the ice line (a Definition, study row 3).
/// Log-uniform: the shape is a Definition.
pub(crate) const COMPANION_AXIS: (AstronomicalUnit, AstronomicalUnit) =
    (AstronomicalUnit(1.0), AstronomicalUnit(20.0));

/// Source: an eccentric giant weighs 0.3-13 M_J (Zhu & Wu 2018's cold
/// Jupiter cut, G 2.4; the 13 M_J bound, study row 4). Log-uniform, in
/// Jupiter masses: the shape is a Definition. A sparse giant too.
pub(crate) const ECCENTRIC_GIANT_MASS: (f64, f64) = (0.3, 13.0);

/// Source: Rosenthal 2023 (p4, Table 1 p9), the eccentricity Beta(a, b)
/// of single giants beyond 0.3 au, a = 0.60, b = 1.45 (G 2.1).
pub(crate) const SINGLE_GIANT_BETA: (f64, f64) = (0.60, 1.45);

/// Source: Rosenthal 2023 (p4, Table 1 p9), the eccentricity Beta(a, b)
/// of giants in multi-giant systems, a = 1.06, b = 3.68 (G 2.1).
pub(crate) const MULTI_GIANT_BETA: (f64, f64) = (1.06, 3.68);

/// Calibration: the median of [`SINGLE_GIANT_BETA`], 0.2195 by its
/// numeric CDF (a test checks it), where its halves split.
pub(crate) const SINGLE_GIANT_BETA_MEDIAN: f64 = 0.220;

/// Calibration: the median of [`MULTI_GIANT_BETA`], 0.1834 by its numeric
/// CDF (a test checks it), where its halves split.
pub(crate) const MULTI_GIANT_BETA_MEDIAN: f64 = 0.183;

/// Source: Dawson & Johnson 2018 (p17), 31 of 228 hot Jupiters at
/// 3-10 d have e < 0.1, the others being poorly constrained or still
/// circularising (G 1.3). Uniform below it is a Definition: they give no
/// shape.
pub(crate) const HOT_JUPITER_MAX_ECCENTRICITY: f64 = 0.1;

/// Source: Van Eylen 2019 (abstract p1, Table 3 p7), the half-Gaussian
/// scale of the eccentricities of single-transiting planets, 0.32 ± 0.06
/// (S 4).
pub(crate) const SINGLES_HALF_GAUSSIAN_SIGMA: f64 = 0.32;

/// Source: the Rayleigh scale of compact systems' eccentricities, from
/// He 2019 Table 5 (p16, 0.020) to Mills 2019 (p8, 0.0355) (S 3.1), one
/// value per system. Uniform between them is a Definition.
pub(crate) const COMPACT_SIGMA_E: (f64, f64) = (0.02, 0.035);

/// Source: the largest σ_e of Pu & Wu 2015's K50 fit, 0.05 (eq. 14
/// scan; S 2.3). Families without a measured σ_e take it (a Definition:
/// the upper end of the fit).
pub(crate) const K50_SIGMA_E_MAX: f64 = 0.05;

/// The ice line over the Sun's: the factor that scales the giants' axes
/// with the host (study, "Host scaling").
pub(crate) fn ice_scale(ice_line: AstronomicalUnit) -> f64 {
    ice_line.0 / ICE_LINE_OF_THE_SUN.0
}

/// The period of the innermost planet: EPOS I's broken power law in
/// `dN/dlog P`, peak at 12 d, index +1.6 below and -0.9 above (S 1.2),
/// truncated to `[lo, hi)` (`lo` the period at the stellar surface).
pub(crate) fn innermost_period(rng: &mut ChaCha8Rng, lo: Day, hi: Day) -> Day {
    Day(broken_power_law(
        rng,
        INNERMOST_PEAK.0,
        INNERMOST_BELOW,
        INNERMOST_ABOVE,
        lo.0,
        hi.0,
    ))
}

/// The period ratio of an adjacent pair by EPOS I's spacing law (S 1.5):
/// `log10 D` normal, `P2/P1 = ((2 + D)/(2 - D))^1.5`, drawn again when
/// `D >= 2` or when the ratio falls below the Weiss wall.
pub(crate) fn spacing_ratio(rng: &mut ChaCha8Rng) -> f64 {
    loop {
        let d = 10f64.powf(SPACING_LOG10_MEAN + SPACING_LOG10_SIGMA * normal(rng));
        if d >= SPACING_D_LIMIT {
            continue;
        }
        let ratio = ((2.0 + d) / (2.0 - d)).powf(1.5);
        if ratio >= WEISS_WALL {
            return ratio;
        }
    }
}

/// The period ratio of an adjacent compact pair, and whether it is near
/// a resonance: with [`NEAR_RESONANCE_CHANCE`], uniform within 5 %
/// exterior to 3:2 or 2:1 (odds 1/2); otherwise [`spacing_ratio`].
pub(crate) fn compact_ratio(rng: &mut ChaCha8Rng) -> (f64, bool) {
    if uniform(rng) < NEAR_RESONANCE_CHANCE {
        let resonance = RESONANCES[usize::from(uniform(rng) >= 0.5)];
        (
            resonance * (1.0 + NEAR_RESONANCE_WINDOW * uniform(rng)),
            true,
        )
    } else {
        (spacing_ratio(rng), false)
    }
}

/// A small planet's mass: Neil & Rogers 2020 model 2, `ln M` normal
/// (1.00, 1.65) (B9), drawn again outside `[0.1, 30)` M⊕.
pub(crate) fn small_planet_mass(rng: &mut ChaCha8Rng) -> EarthMass {
    loop {
        let mass = log_normal(rng, NEIL_ROGERS_MU, NEIL_ROGERS_SIGMA);
        if (SMALL_MASS_MIN.0..SMALL_MASS_MAX.0).contains(&mass) {
            return EarthMass(mass);
        }
    }
}

/// A rocky planet of a solar-like system: log-uniform in [`ROCKY_MASS`].
pub(crate) fn rocky_mass(rng: &mut ChaCha8Rng) -> EarthMass {
    EarthMass(log_uniform(rng, ROCKY_MASS.0.0, ROCKY_MASS.1.0))
}

/// A mass log-uniform in a range of Jupiter masses, in Earth masses.
pub(crate) fn jupiter_masses(rng: &mut ChaCha8Rng, (lo, hi): (f64, f64)) -> EarthMass {
    EarthMass(log_uniform(rng, lo, hi) * EARTH_MASSES_PER_JUPITER_MASS)
}

/// The next rocky planet out from `inner` (its mass and axis): the
/// spacing in mutual Hill radii from Fang & Margot's shifted Rayleigh
/// (offset 3.5, scale 14.5; S 2.2), on circular orbits (the eccentricity
/// comes after classifying), truncated by its inverse CDF so that the
/// planet stays inside `limit` (the ice line). With `R_H = k (a1 + a2)/2`,
/// `k = ((m1 + m2)/3M)^⅓`, a spacing Δ puts it at
/// `a1 (1 + Δk/2) / (1 - Δk/2)`. When `inner` sits within 3.5 Hill radii
/// of the limit, no spacing fits: the planet goes at the offset, past the
/// limit, for the placement loop to judge.
pub(crate) fn next_rocky_axis(
    rng: &mut ChaCha8Rng,
    (inner_mass, inner_a): (EarthMass, AstronomicalUnit),
    mass: EarthMass,
    host: SolarMass,
    limit: AstronomicalUnit,
) -> AstronomicalUnit {
    let masses = Kilogram::from(inner_mass).0 + Kilogram::from(mass).0;
    let k = (masses / (3.0 * Kilogram::from(host).0)).cbrt();
    let room = limit.0 / inner_a.0;
    let widest = if room > 1.0 {
        2.0 * (room - 1.0) / ((room + 1.0) * k)
    } else {
        HILL_OFFSET
    };
    let u = uniform(rng);
    let span = widest - HILL_OFFSET;
    let delta = if span > 0.0 {
        // Rayleigh CDF F(x) = 1 - exp(-x²/2σ²), drawn in [0, F(span)).
        let top = -(-span * span / (2.0 * HILL_SIGMA * HILL_SIGMA)).exp_m1();
        HILL_OFFSET + HILL_SIGMA * (-2.0 * (-u * top).ln_1p()).sqrt()
    } else {
        HILL_OFFSET
    };
    let half = delta * k / 2.0;
    AstronomicalUnit(inner_a.0 * (1.0 + half) / (1.0 - half))
}

/// The first giant's axis: Fulton 2021's broken power law in `dN/dln a`
/// (index +0.72 inside the 3.6 au break, -0.86 beyond, support to 30 au;
/// G 3.1), every bound scaled by `ice_line / 2.7 au`, truncated below the
/// ice line.
pub(crate) fn fulton_axis(rng: &mut ChaCha8Rng, ice_line: AstronomicalUnit) -> AstronomicalUnit {
    let scale = ice_scale(ice_line);
    AstronomicalUnit(broken_power_law(
        rng,
        FULTON_BREAK.0 * scale,
        FULTON_INSIDE,
        FULTON_BEYOND,
        ice_line.0,
        FULTON_OUTER_SUPPORT.0 * scale,
    ))
}

/// The next giant out from one at `inner`: `ln(a2/a1)` uniform in
/// [`NEXT_GIANT_LN_RATIO`] (G 3.3).
pub(crate) fn next_giant_axis(rng: &mut ChaCha8Rng, inner: AstronomicalUnit) -> AstronomicalUnit {
    let (lo, hi) = NEXT_GIANT_LN_RATIO;
    AstronomicalUnit(inner.0 * (lo + (hi - lo) * uniform(rng)).exp())
}

/// A hot Jupiter's axis: its period log-uniform from that of the stellar
/// surface to 10 d (G 1.1; the shape is a Definition, no density was
/// read). The Roche floor needs the classified density: the placement
/// loop checks it.
pub(crate) fn hot_jupiter_axis(
    rng: &mut ChaCha8Rng,
    host: SolarMass,
    stellar_radius: AstronomicalUnit,
) -> AstronomicalUnit {
    let surface = orbit::period(stellar_radius, host);
    let period = log_uniform(rng, surface.0, HOT_JUPITER_MAX_PERIOD.0);
    orbit::semi_major_axis(Day(period), host)
}

/// A hot Jupiter's outer companion: log-uniform in [`COMPANION_AXIS`]
/// scaled by `ice_line / 2.7 au`.
pub(crate) fn companion_axis(rng: &mut ChaCha8Rng, ice_line: AstronomicalUnit) -> AstronomicalUnit {
    let scale = ice_scale(ice_line);
    AstronomicalUnit(log_uniform(
        rng,
        COMPANION_AXIS.0.0 * scale,
        COMPANION_AXIS.1.0 * scale,
    ))
}

/// A sparse planet's axis: its period log-uniform from that of the
/// stellar surface to that of 30 au × `ice_line / 2.7 au` (Fulton 2021's
/// survey edge, G 3.1; the bound is a Definition). A period log-uniform
/// is an axis log-uniform (Kepler: `P ∝ a^1.5`), so the axis is drawn.
pub(crate) fn sparse_axis(
    rng: &mut ChaCha8Rng,
    stellar_radius: AstronomicalUnit,
    ice_line: AstronomicalUnit,
) -> AstronomicalUnit {
    let edge = FULTON_OUTER_SUPPORT.0 * ice_scale(ice_line);
    AstronomicalUnit(log_uniform(rng, stellar_radius.0, edge))
}

/// The eccentricity of a classified trial, by the group of its `class`,
/// drawn again while 1 or more:
///
/// - `HotJupiter`: uniform below 0.1 (G 1.3);
/// - `GasGiant`, `IceGiant`: the family's giant Beta (G 2.1): solar-like
///   the lower half of the multi-giant law (the upper half when the
///   system is stirred), hot-Jupiter companion the whole multi-giant law,
///   eccentric giants the upper half of the single-giant law (one giant)
///   or of the multi-giant law (two), sparse the whole single-giant law;
///   a compact system has no giant law and draws as its small planets;
/// - every other class: compact the system's Rayleigh σ_e, solar-like a
///   Rayleigh of 0.05, the other families the singles' half-Gaussian of
///   0.32 (Van Eylen 2019, S 4).
pub(crate) fn eccentricity(ctx: &FamilyContext, class: PlanetClass, rng: &mut ChaCha8Rng) -> f64 {
    loop {
        let e = match class {
            PlanetClass::HotJupiter => HOT_JUPITER_MAX_ECCENTRICITY * uniform(rng),
            PlanetClass::GasGiant | PlanetClass::IceGiant => giant_eccentricity(ctx, rng),
            _ => small_eccentricity(ctx, rng),
        };
        if e < 1.0 {
            return e;
        }
    }
}

fn giant_eccentricity(ctx: &FamilyContext, rng: &mut ChaCha8Rng) -> f64 {
    let (single, multi) = (SINGLE_GIANT_BETA, MULTI_GIANT_BETA);
    match ctx.family {
        Family::Compact => small_eccentricity(ctx, rng),
        Family::SolarLike => beta_half(rng, multi.0, multi.1, MULTI_GIANT_BETA_MEDIAN, ctx.stirred),
        Family::HotJupiter => beta(rng, multi.0, multi.1),
        Family::EccentricGiants if ctx.counts.giants <= 1 => {
            beta_half(rng, single.0, single.1, SINGLE_GIANT_BETA_MEDIAN, true)
        }
        Family::EccentricGiants => beta_half(rng, multi.0, multi.1, MULTI_GIANT_BETA_MEDIAN, true),
        Family::Sparse => beta(rng, single.0, single.1),
    }
}

fn small_eccentricity(ctx: &FamilyContext, rng: &mut ChaCha8Rng) -> f64 {
    match ctx.family {
        Family::Compact => rayleigh(rng, ctx.sigma_e),
        Family::SolarLike => rayleigh(rng, ROCKY_ECCENTRICITY_SIGMA),
        Family::HotJupiter | Family::EccentricGiants | Family::Sparse => {
            half_gaussian(rng, SINGLES_HALF_GAUSSIAN_SIGMA)
        }
    }
}

/// The σ_e that `orbit::k50` takes for the system: compact its own drawn
/// scale, solar-like the rocky planets' 0.05, the others the upper end of
/// Pu & Wu's fit; clamped to the fit, `[0, 0.05]`.
pub(crate) fn k50_sigma_e(ctx: &FamilyContext) -> f64 {
    let sigma = match ctx.family {
        Family::Compact => ctx.sigma_e,
        Family::SolarLike => ROCKY_ECCENTRICITY_SIGMA,
        Family::HotJupiter | Family::EccentricGiants | Family::Sparse => K50_SIGMA_E_MAX,
    };
    sigma.clamp(0.0, K50_SIGMA_E_MAX)
}

#[cfg(test)]
#[path = "laws_tests.rs"]
mod tests;
