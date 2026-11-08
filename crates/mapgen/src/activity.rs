//! The magnetic activity of a system's star: how fast it turns, how strong
//! its dynamo is, the cycle it runs, where and how often its active
//! regions emerge, and what its photosphere looks like (granulation, limb
//! darkening). See `docs/science/star-activity.md`.
//!
//! [`star_activity`] derives it from the star ([`StarInfo`]) and the
//! system's seed. The relations are private functions of the temperature,
//! mass, radius and age, so they can be tested without a star.
//!
//! **Rotation.** Below the Kraft break (6 250 K) a star with a convective
//! envelope spins down with age: gyrochronology (Mamajek & Hillenbrand
//! 2008). A fully convective M dwarf follows Engle & Guinan (2018).
//! Hotter stars keep the rotation they were born with (Kraft 1967): a
//! radiative star turns at 150 km/s (Royer et al. 2007).
//!
//! **Activity.** The Rossby number `Ro = P / τ_c` (turnover time of
//! Wright et al. 2018) sets the X-ray activity, saturated below
//! `Ro = 0.13` and falling as `Ro^−2.70` above (Wright et al. 2011). The
//! spot coverage, facula ratio, cycle and active band follow from it.
//!
//! **The regions themselves** (their emergence, growth and decay over
//! time) are drawn in the browser, from [`StarActivity::seed`] and these
//! parameters (`web/src/lib/starActivity.ts`).
use crate::giants::{normal, system_age_gyr};
use crate::noise::{stream, stream_seed};
use crate::star::{SUN_TEFF_K, StarInfo};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Where a star carries its heat outwards near the surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Envelope {
    /// No convective envelope (at or above [`RADIATIVE_TEFF_K`]): no
    /// dynamo, no spots, no granulation.
    Radiative,
    /// A convective envelope over a radiative core, as the Sun.
    Convective,
    /// Convective throughout (below [`FULLY_CONVECTIVE_MASS`]).
    FullyConvective,
}

/// The activity of a system's star, as `system.json` stores it on the
/// star (`SystemStar::activity`), derived on read when absent.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct StarActivity {
    pub envelope: Envelope,
    /// Age of the star, Gyr ([`system_age_gyr`]).
    pub age_gyr: f32,
    /// Equatorial sidereal rotation period, days.
    pub rotation_days: f32,
    /// Differential rotation: Ω(λ) = Ω_eq (1 − shear sin²λ).
    pub shear: f32,
    /// Convective turnover time, days; 0 when radiative.
    pub turnover_days: f32,
    /// Rossby number `rotation_days / turnover_days`; 0 when radiative.
    pub rossby: f32,
    /// X-ray activity `(Lx/Lbol) / (Lx/Lbol)_sat`, in (0, 1]; 0 when
    /// radiative.
    pub activity: f32,
    /// Period of the activity cycle, years; 0: no cycle.
    pub cycle_years: f32,
    /// Phase of the cycle at day 0, in [0, 1).
    pub cycle_phase: f32,
    /// How much the region rate varies over the cycle, in [0, 1]: the
    /// rate at minimum is `1 − cycle_depth` of the rate at maximum.
    pub cycle_depth: f32,
    /// Latitude where regions emerge at the start of a cycle and at its
    /// end, degrees.
    pub band_deg: [f32; 2],
    /// Share of the visible hemisphere under spots at cycle maximum.
    pub spot_coverage: f32,
    /// Facula area over spot area.
    pub facula_ratio: f32,
    /// New regions per day at cycle maximum.
    pub region_rate: f32,
    /// Median peak spot area of a region, share of the hemisphere.
    pub region_area: f32,
    /// Standard deviation of ln(peak area).
    pub region_sigma: f32,
    /// Spot decay rate W, share of the hemisphere per day.
    pub decay_per_day: f32,
    /// Granule size, star radii.
    pub granule_size: f32,
    /// Granule intensity contrast; 0 when radiative.
    pub granule_contrast: f32,
    /// Granule lifetime, days.
    pub granule_lifetime_days: f32,
    /// Quadratic limb darkening `I(μ) = 1 − u1 (1 − μ) − u2 (1 − μ)²`:
    /// `[u1, u2]`.
    pub limb: [f32; 2],
    /// Seed of the region timeline (drawn in the browser).
    pub seed: u32,
}

/// Effective temperature at and above which a star has no convective
/// envelope, K. The granulation boundary of Gray & Nagel (1989, ApJ 341,
/// 421) crosses the main sequence near F0, 7 220 K in
/// Pecaut & Mamajek's sequence; X-ray activity also fades out near
/// B−V ≈ 0.3 (Schmitt 1997, A&A 318, 215).
pub const RADIATIVE_TEFF_K: f32 = 7_200.0;

/// Mass below which a dwarf is convective throughout, M☉ (Chabrier &
/// Baraffe 1997, A&A 327, 1039).
pub const FULLY_CONVECTIVE_MASS: f32 = 0.35;

/// The Kraft break, K: hotter stars are not magnetically braked (Kraft
/// 1967, ApJ 150, 551).
const KRAFT_TEFF_K: f32 = 6_250.0;

/// Gyrochronology `P = a (B−V − c)^b t^n` (P in days, t in Myr):
/// Mamajek & Hillenbrand (2008, ApJ 687, 1264, arXiv:0807.1686,
/// table 10): a = 0.407 ± 0.021, b = 0.325 ± 0.024, c = 0.495 ± 0.010,
/// n = 0.566 ± 0.008 (verified against the paper).
const GYRO: [f32; 4] = [0.407, 0.325, 0.495, 0.566];

/// B−V range the gyrochronology is applied over. M&H calibrated
/// 0.5 < B−V < 0.9: beyond it the law is extrapolated.
const GYRO_BV: (f32, f32) = (0.45, 1.6);

/// σ of the log-normal scatter of rotation periods: **calibration**, from
/// the 15–20 % spread of gyro ages in Barnes (2007, ApJ 669, 1167).
const ROTATION_SCATTER: f64 = 0.15;

/// Equatorial velocity at the hot edge of the Kraft break, km/s:
/// **calibration** (the ramp from the gyro value at 6 250 K is linear).
const KRAFT_EDGE_KMS: f32 = 100.0;

/// Equatorial velocity of a radiative star, km/s: typical of A stars,
/// whose v sin i distributions peak between about 100 and 200 km/s
/// (Royer et al. 2007, A&A 463, 671; not verified).
const RADIATIVE_KMS: f32 = 150.0;

/// Age–rotation law of M2.5–6.0 V dwarfs, `age (Gyr) = 0.012 + 0.061 P`
/// (P in days): Engle & Guinan (2018, RNAAS 2, 34,
/// doi:10.3847/2515-5172/aab1f8, eq. 2, verified).
const MID_M_AGE: (f32, f32) = (0.012, 0.061);

/// Clamp of a fully convective dwarf's period, days.
const MID_M_PERIOD: (f32, f32) = (0.2, 150.0);

/// Convective turnover time `log10 τ_c = 2.33 − 1.50 M + 0.31 M²` (days,
/// M in M☉): Wright et al. (2018, MNRAS 479, 2351,
/// doi:10.1093/mnras/sty1670, eq. 6, verified), valid for 0.08–1.36 M☉.
const TURNOVER: [f32; 3] = [2.33, -1.50, 0.31];
const TURNOVER_MASS: (f32, f32) = (0.08, 1.36);

/// Saturation Rossby number and the slope of `Lx/Lbol ∝ Ro^β` above it:
/// Wright et al. (2011, ApJ 743, 48, arXiv:1109.4634,
/// unbiased sample, verified).
const RO_SAT: f32 = 0.13;
const ACTIVITY_SLOPE: f32 = -2.70;

/// The Sun's sidereal rotation period, days, at which this model's
/// activity is the Sun's ([`sun_activity`]).
const SUN_ROTATION_DAYS: f32 = 25.4;

/// Share of the hemisphere under spots at the Sun's cycle maximum:
/// **calibration** anchor, about 0.3 % (3 000 MSH; Hathaway 2015, Living
/// Rev. Sol. Phys. 12, 4, doi:10.1007/lrsp-2015-4; not verified).
const SUN_SPOT_COVERAGE: f32 = 0.003;
/// Exponent of `spot_coverage ∝ activity^0.64` and its ceiling, about
/// 30 % on saturated stars (Strassmeier 2009, A&ARv 17, 251): **calibration** between the two
/// anchors.
const SPOT_EXPONENT: f32 = 0.64;
const MAX_SPOT_COVERAGE: f32 = 0.3;

/// Facula over spot area: `10 (activity / sun)^−0.32` within [1, 15],
/// **calibration**. Active stars are spot-dominated (Lockwood et al.
/// 2007, ApJ 664, 1234; Shapiro et al. 2014, A&A 569, A38).
const SUN_FACULA_RATIO: f32 = 10.0;
const FACULA_EXPONENT: f32 = -0.32;
const FACULA_RATIO: (f32, f32) = (1.0, 15.0);

/// Rotations per activity cycle, `P_cyc / P_rot`, on the inactive (I,
/// Ro ≥ 1) and active (A, Ro < 1) sequences of Böhm-Vitense (2007, ApJ
/// 657, 486): about 90 and about 300 (her Ab
/// sequence; Aa runs at 400–500). Read from the summary of Distefano et
/// al. (2017, A&A 606, A58, sect. 5.1); the paper itself was unreachable.
/// The Sun (about 160) lies between the two sequences, so this model gives
/// a Sun-like star a cycle of about 6.5 years, not 11.
const INACTIVE_ROTATIONS_PER_CYCLE: f32 = 90.0;
const ACTIVE_ROTATIONS_PER_CYCLE: f32 = 300.0;
/// σ of the log-normal scatter of cycle periods: **calibration**.
const CYCLE_SCATTER: f64 = 0.15;
/// Cycle depth of a saturated star and of an inactive one (Ro ≥ 1),
/// linear in log Ro between: **calibration**.
const CYCLE_DEPTH: (f32, f32) = (0.3, 0.9);

/// Mean latitude of sunspot emergence at the start and the end of a
/// cycle, degrees: `λ = 28° exp(−t / 90 months)` stops near 7° (Hathaway
/// 2011, Sol. Phys. 273, 221, arXiv:1108.1722, as quoted by
/// Hathaway 2015, sect. 4.6, verified).
const SUN_BAND_DEG: [f32; 2] = [28.0, 7.0];
/// How far rapid rotators push the band poleward, at Ro_sat, degrees:
/// **calibration** (the mechanism, Coriolis deflection of rising flux
/// tubes, is Schüssler & Solanki 1992, A&A 264, L13).
const POLEWARD_SHIFT_DEG: f32 = 40.0;
/// Band of a fully convective dwarf, degrees: **calibration** (Doppler
/// images of M dwarfs show spots at every latitude, Morin et al. 2008,
/// MNRAS 390, 567; not verified).
const FULLY_CONVECTIVE_BAND_DEG: [f32; 2] = [75.0, 0.0];

/// Log-normal distribution of the peak (maximum development) total area
/// of sunspot groups, Baumann & Solanki (2005, A&A 443, 1061,
/// arXiv:astro-ph/0510516, table 1): `⟨A⟩ = 62.2 MSH`,
/// `σ_A = 2.45` in their form `ln dN/dA ∝ −(ln A − ln⟨A⟩)² / (2 ln σ_A)`,
/// which is a median of `⟨A⟩ σ_A` = 152 MSH and σ(ln A) = `√ln σ_A` =
/// 0.95. The log-normal form is Bogdan et al.'s (1988, ApJ 327, 451),
/// whose umbral areas of single spots are not region areas.
const SUN_REGION_MEAN_MSH: f32 = 62.2;
const SUN_REGION_WIDTH: f32 = 2.45;
/// One millionth of a solar hemisphere (MSH), as a share of it.
const MSH: f32 = 1e-6;

/// Spot decay rate of the Gnevyshev–Waldmeier rule `A_peak = W T`, about
/// 10 MSH/day (Petrovay & van Driel-Gesztelyi 1997, Sol. Phys. 176, 249,
/// arXiv:astro-ph/9706029). The same on every star: **calibration**
/// (Giles et al. 2017, MNRAS 472, 1618, find longer lives for larger
/// spots, consistent with a constant W).
const DECAY_MSH_PER_DAY: f32 = 10.0;

/// Longest lifetime of a region, days (the timeline caps it).
pub const MAX_LIFETIME_DAYS: f32 = 120.0;

/// Solar differential rotation, equator minus pole, rad/day (Snodgrass &
/// Ulrich 1990, ApJ 351, 309; not verified, paper unreachable), scaled as
/// `ΔΩ ∝ Teff^8.92` (Barnes et al. 2005, MNRAS 357, L1,
/// doi:10.1111/j.1745-3933.2005.08587.x, 8.92 ± 0.31, verified). The
/// shear is clamped to [0, 0.5].
const SUN_DELTA_OMEGA: f32 = 0.055;
const DELTA_OMEGA_EXPONENT: f32 = 8.92;
const MAX_SHEAR: f32 = 0.5;

/// Solar granule size, Mm, scaled with the pressure scale height
/// `H_p ∝ T / g` (Trampedach et al. 2013, ApJ 769, 18,
/// doi:10.1088/0004-637X/769/1/18; Magic et al. 2013, A&A 557, A26; not
/// verified).
const SUN_GRANULE_MM: f32 = 1.3;
/// The solar radius, Mm.
const SUN_RADIUS_MM: f32 = 695.7;
/// Solar granule contrast `0.15 (Teff / T☉)²`, capped at 0.25, fading out
/// from 6 800 K to [`RADIATIVE_TEFF_K`]: **calibration** on the trend of
/// Magic et al. (2013).
const SUN_GRANULE_CONTRAST: f32 = 0.15;
const MAX_GRANULE_CONTRAST: f32 = 0.25;
const GRANULE_FADE_K: f32 = 6_800.0;
/// Solar granule lifetime, minutes, scaled with the granule size:
/// **calibration** (Title et al. 1989, ApJ 336, 475; not verified:
/// secondary sources quote exponential time constants of 2.5–6.3 min).
const SUN_GRANULE_MINUTES: f32 = 8.0;

/// Quadratic limb darkening `(Teff, u1, u2)`: Claret & Bloemen (2011, A&A
/// 529, A75, VizieR J/A+A/529/A75), Johnson V, log g = 4.5, [M/H] = 0,
/// ATLAS. Not verified: the VizieR catalogue was unreachable, so these
/// entries are approximate values of that table and must be checked.
const LIMB: [(f32, f32, f32); 7] = [
    (3_500.0, 0.62, 0.17),
    (4_000.0, 0.71, 0.07),
    (5_000.0, 0.60, 0.15),
    (5_750.0, 0.47, 0.24),
    (6_500.0, 0.37, 0.29),
    (7_500.0, 0.28, 0.33),
    (10_000.0, 0.18, 0.37),
];

/// The activity of a system's star, deterministic in `system_seed`.
///
/// Its random draws come from the stream [`stream::STAR_ACTIVITY`], in a
/// fixed order (rotation scatter, cycle scatter, cycle phase, region
/// seed), so a star keeps its activity across versions.
pub fn star_activity(info: &StarInfo, system_seed: u32) -> StarActivity {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(system_seed, stream::STAR_ACTIVITY)));
    let rotation_scatter = (ROTATION_SCATTER * normal(&mut rng)).exp() as f32;
    let cycle_scatter = (CYCLE_SCATTER * normal(&mut rng)).exp() as f32;
    let cycle_phase = rng.random::<f32>();
    let seed = rng.random::<u32>();

    let (teff, mass, radius) = (info.teff_k, info.mass, info.radius);
    let age_gyr = system_age_gyr(mass);
    let envelope = envelope(teff, mass);
    let rotation_days = rotation_days(envelope, teff, radius, age_gyr) * rotation_scatter;
    let rotation_days = match envelope {
        Envelope::FullyConvective => rotation_days.clamp(MID_M_PERIOD.0, MID_M_PERIOD.1),
        _ => rotation_days,
    };
    let shear = shear(teff, rotation_days);
    let granule_size = granule_size(teff, mass, radius);
    let granule_lifetime_days =
        SUN_GRANULE_MINUTES / 1_440.0 * granule_size * radius * SUN_RADIUS_MM / SUN_GRANULE_MM;
    let decay_per_day = DECAY_MSH_PER_DAY * MSH;
    let quiet = StarActivity {
        envelope,
        age_gyr,
        rotation_days,
        shear,
        turnover_days: 0.0,
        rossby: 0.0,
        activity: 0.0,
        cycle_years: 0.0,
        cycle_phase,
        cycle_depth: 0.0,
        band_deg: [0.0, 0.0],
        spot_coverage: 0.0,
        facula_ratio: 0.0,
        region_rate: 0.0,
        region_area: 0.0,
        region_sigma: 0.0,
        decay_per_day,
        granule_size,
        granule_contrast: 0.0,
        granule_lifetime_days,
        limb: limb_quadratic(teff),
        seed,
    };
    if envelope == Envelope::Radiative {
        return quiet;
    }

    let turnover_days = turnover_days(mass);
    let rossby = rotation_days / turnover_days;
    let activity = rossby_activity(rossby);
    let relative = activity / sun_activity();
    let spot_coverage = (SUN_SPOT_COVERAGE * relative.powf(SPOT_EXPONENT)).min(MAX_SPOT_COVERAGE);
    let facula_ratio =
        (SUN_FACULA_RATIO * relative.powf(FACULA_EXPONENT)).clamp(FACULA_RATIO.0, FACULA_RATIO.1);
    let rotations = if rossby >= 1.0 {
        INACTIVE_ROTATIONS_PER_CYCLE
    } else {
        ACTIVE_ROTATIONS_PER_CYCLE
    };
    let cycle_years = rotations * rotation_days * cycle_scatter / 365.25;
    // 0 at Ro_sat, 1 at Ro = 1, linear in log Ro.
    let inactive = (rossby.max(RO_SAT).ln() - RO_SAT.ln()) / -RO_SAT.ln();
    let cycle_depth = CYCLE_DEPTH.0 + (CYCLE_DEPTH.1 - CYCLE_DEPTH.0) * inactive.clamp(0.0, 1.0);
    let band_deg = active_band(envelope, rossby);
    let width = SUN_REGION_WIDTH.ln();
    let region_sigma = width.sqrt();
    let region_area =
        SUN_REGION_MEAN_MSH * SUN_REGION_WIDTH * MSH * (spot_coverage / SUN_SPOT_COVERAGE).sqrt();
    // A region of peak area A grows and decays linearly over A / W days:
    // A² / (2W) area-days. The rate keeps the mean spotted area at cycle
    // maximum at `spot_coverage`, with E[A²] = median² exp(2σ²).
    let mean_square = region_area * region_area * (2.0 * width).exp();
    let region_rate = spot_coverage * 2.0 * decay_per_day / mean_square;
    let granule_contrast = granule_contrast(teff);
    StarActivity {
        turnover_days,
        rossby,
        activity,
        cycle_years,
        cycle_depth,
        band_deg,
        spot_coverage,
        facula_ratio,
        region_rate,
        region_area,
        region_sigma,
        granule_contrast,
        ..quiet
    }
}

/// The envelope of a star of `teff_k` and `mass` (M☉).
fn envelope(teff_k: f32, mass: f32) -> Envelope {
    if teff_k >= RADIATIVE_TEFF_K {
        Envelope::Radiative
    } else if mass < FULLY_CONVECTIVE_MASS {
        Envelope::FullyConvective
    } else {
        Envelope::Convective
    }
}

/// Equatorial rotation period, days, before the seeded scatter.
fn rotation_days(envelope: Envelope, teff_k: f32, radius: f32, age_gyr: f32) -> f32 {
    // Period of a star of `radius` turning at `kms` km/s, days.
    let period = |kms: f32| TAU * radius * SUN_RADIUS_MM * 1_000.0 / kms / 86_400.0;
    match envelope {
        Envelope::Radiative => period(RADIATIVE_KMS),
        Envelope::FullyConvective => ((age_gyr - MID_M_AGE.0) / MID_M_AGE.1).max(MID_M_PERIOD.0),
        Envelope::Convective if teff_k < KRAFT_TEFF_K => gyro_period_days(teff_k, age_gyr),
        Envelope::Convective => {
            let gyro = period(1.0) / gyro_period_days(KRAFT_TEFF_K, age_gyr);
            let f = (teff_k - KRAFT_TEFF_K) / (RADIATIVE_TEFF_K - KRAFT_TEFF_K);
            period(gyro + (KRAFT_EDGE_KMS - gyro) * f)
        }
    }
}

/// Gyrochronology period, days, of a star of `teff_k` at `age_gyr`
/// ([`GYRO`]).
fn gyro_period_days(teff_k: f32, age_gyr: f32) -> f32 {
    let [a, b, c, n] = GYRO;
    let bv = bv_from_teff(teff_k).clamp(GYRO_BV.0, GYRO_BV.1);
    a * (bv - c).max(1e-3).powf(b) * (age_gyr * 1_000.0).powf(n)
}

/// B−V of a star of `teff_k`, inverting Ballesteros (2012, EPL 97, 34008):
/// `T = 4600 (1 / (0.92 BV + 1.7) + 1 / (0.92 BV + 0.62))`, by bisection
/// on [−0.3, 2.0].
fn bv_from_teff(teff_k: f32) -> f32 {
    let t = |bv: f32| 4_600.0 * (1.0 / (0.92 * bv + 1.7) + 1.0 / (0.92 * bv + 0.62));
    let (mut lo, mut hi) = (-0.3_f32, 2.0_f32);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        // T falls as B−V rises.
        if t(mid) > teff_k {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Convective turnover time, days, of a star of `mass` (M☉), clamped to
/// the calibrated range ([`TURNOVER`]).
fn turnover_days(mass: f32) -> f32 {
    let m = mass.clamp(TURNOVER_MASS.0, TURNOVER_MASS.1);
    10f32.powf(TURNOVER[0] + TURNOVER[1] * m + TURNOVER[2] * m * m)
}

/// `(Lx/Lbol) / (Lx/Lbol)_sat` at Rossby number `ro`: 1 up to
/// [`RO_SAT`], `(Ro / Ro_sat)^−2.70` beyond.
fn rossby_activity(ro: f32) -> f32 {
    if ro <= RO_SAT {
        1.0
    } else {
        (ro / RO_SAT).powf(ACTIVITY_SLOPE)
    }
}

/// This model's activity for the Sun (1 M☉, [`SUN_ROTATION_DAYS`]).
fn sun_activity() -> f32 {
    rossby_activity(SUN_ROTATION_DAYS / turnover_days(1.0))
}

/// Shear of the differential rotation of a star of `teff_k` turning in
/// `rotation_days` ([`SUN_DELTA_OMEGA`]).
fn shear(teff_k: f32, rotation_days: f32) -> f32 {
    let delta_omega = SUN_DELTA_OMEGA * (teff_k / SUN_TEFF_K).powf(DELTA_OMEGA_EXPONENT);
    (delta_omega * rotation_days / TAU).clamp(0.0, MAX_SHEAR)
}

/// Emergence latitudes at the start and the end of a cycle, degrees: the
/// Sun's band, pushed poleward as Ro falls from the Sun's to Ro_sat; a
/// fully convective dwarf's spots reach every latitude.
fn active_band(envelope: Envelope, rossby: f32) -> [f32; 2] {
    if envelope == Envelope::FullyConvective {
        return FULLY_CONVECTIVE_BAND_DEG;
    }
    let sun = (SUN_ROTATION_DAYS / turnover_days(1.0)).ln();
    let f = ((sun - rossby.ln()) / (sun - RO_SAT.ln())).clamp(0.0, 1.0);
    SUN_BAND_DEG.map(|deg| deg + POLEWARD_SHIFT_DEG * f)
}

/// Granule size, star radii: [`SUN_GRANULE_MM`] scaled by `T / g`, with
/// `g / g☉ = M / R²`.
fn granule_size(teff_k: f32, mass: f32, radius: f32) -> f32 {
    let gravity = mass / (radius * radius);
    SUN_GRANULE_MM * (teff_k / SUN_TEFF_K) / gravity / (radius * SUN_RADIUS_MM)
}

/// Granule contrast of a star of `teff_k` ([`SUN_GRANULE_CONTRAST`]).
fn granule_contrast(teff_k: f32) -> f32 {
    let fade = ((RADIATIVE_TEFF_K - teff_k) / (RADIATIVE_TEFF_K - GRANULE_FADE_K)).clamp(0.0, 1.0);
    (SUN_GRANULE_CONTRAST * (teff_k / SUN_TEFF_K).powi(2)).min(MAX_GRANULE_CONTRAST) * fade
}

/// Quadratic limb-darkening coefficients `[u1, u2]` at `teff_k`, linear
/// in Teff between the nodes of [`LIMB`], held beyond them.
fn limb_quadratic(teff_k: f32) -> [f32; 2] {
    let first = LIMB[0];
    if teff_k <= first.0 {
        return [first.1, first.2];
    }
    for w in LIMB.windows(2) {
        let ((t0, a0, b0), (t1, a1, b1)) = (w[0], w[1]);
        if teff_k <= t1 {
            let f = (teff_k - t0) / (t1 - t0);
            return [a0 + (a1 - a0) * f, b0 + (b1 - b0) * f];
        }
    }
    let last = LIMB[LIMB.len() - 1];
    [last.1, last.2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::star::{StarInfo, parse_type};

    fn star(kind: &str) -> StarInfo {
        StarInfo::new(parse_type(kind).unwrap(), 1.0, 0.0)
    }

    /// The median of `f` over the activities of seeds 0 to 200.
    fn median(info: &StarInfo, f: impl Fn(&StarActivity) -> f32) -> f32 {
        let mut values: Vec<f32> = (0..201).map(|seed| f(&star_activity(info, seed))).collect();
        values.sort_by(f32::total_cmp);
        values[values.len() / 2]
    }

    #[test]
    fn turnover_matches_wright_2018() {
        assert!((turnover_days(1.0) - 13.8).abs() < 0.3);
    }

    #[test]
    fn activity_saturates_then_falls() {
        assert_eq!(rossby_activity(0.05), 1.0);
        assert_eq!(rossby_activity(0.13), 1.0);
        assert!((rossby_activity(1.3) - 10f32.powf(-2.70)).abs() < 1e-4);
    }

    #[test]
    fn sun_like_star_rotates_and_cycles_like_the_sun() {
        let g2v = star("G2V");
        let rotation = median(&g2v, |a| a.rotation_days);
        assert!((22.0..30.0).contains(&rotation), "{rotation}");
        let rossby = median(&g2v, |a| a.rossby);
        assert!((1.4..2.5).contains(&rossby), "{rossby}");
        let cycle = median(&g2v, |a| a.cycle_years);
        // Böhm-Vitense's inactive sequence (90 rotations a cycle) gives the
        // Sun about 6.5 years: it lies between her two sequences.
        assert!((5.0..8.0).contains(&cycle), "{cycle}");
        let spots = median(&g2v, |a| a.spot_coverage);
        assert!((0.002..0.005).contains(&spots), "{spots}");
        let band = [
            median(&g2v, |a| a.band_deg[0]),
            median(&g2v, |a| a.band_deg[1]),
        ];
        // Hathaway (2011): 28° at the start of a cycle, 7° at its end.
        assert!((band[0] - 28.0).abs() <= 3.0, "{band:?}");
        assert!((band[1] - 7.0).abs() <= 3.0, "{band:?}");
        let activity = star_activity(&g2v, 1234);
        assert_eq!(activity.envelope, Envelope::Convective);
        assert!(
            (0.4..0.6).contains(&activity.limb[0]),
            "{:?}",
            activity.limb
        );
        assert!(
            (0.0015..0.0025).contains(&activity.granule_size),
            "{}",
            activity.granule_size
        );
        let rate = median(&g2v, |a| a.region_rate);
        assert!((0.3..3.0).contains(&rate), "{rate}");
    }

    #[test]
    fn hot_star_is_quiet() {
        let a = star_activity(&star("A0V"), 1234);
        assert_eq!(a.envelope, Envelope::Radiative);
        assert_eq!(a.activity, 0.0);
        assert_eq!(a.spot_coverage, 0.0);
        assert_eq!(a.region_rate, 0.0);
        assert_eq!(a.granule_contrast, 0.0);
        assert_eq!(a.cycle_years, 0.0);
        assert!(a.rotation_days < 3.0, "{}", a.rotation_days);
    }

    #[test]
    fn convective_boundary_is_7200_k() {
        assert_eq!(envelope(7_199.0, 1.6), Envelope::Convective);
        assert_eq!(envelope(7_200.0, 1.6), Envelope::Radiative);
    }

    #[test]
    fn fully_convective_dwarf_is_active() {
        let (g2v, m5v) = (star("G2V"), star("M5V"));
        assert!(m5v.mass < 0.35);
        let a = star_activity(&m5v, 1234);
        assert_eq!(a.envelope, Envelope::FullyConvective);
        assert!(a.activity > median(&g2v, |a| a.activity), "{}", a.activity);
        assert!(
            a.spot_coverage > median(&g2v, |a| a.spot_coverage),
            "{}",
            a.spot_coverage
        );
        assert_eq!(a.band_deg, [75.0, 0.0]);
    }

    #[test]
    fn same_seed_same_activity() {
        let g2v = star("G2V");
        assert_eq!(star_activity(&g2v, 7), star_activity(&g2v, 7));
        assert_ne!(
            star_activity(&g2v, 7).rotation_days,
            star_activity(&g2v, 8).rotation_days
        );
    }
}
