//! A star's present values: when it reaches and leaves the main sequence
//! ([`life`]) and its radius, luminosity and effective temperature at an
//! age ([`main_sequence`]).
//!
//! How the tables are read (`STELLAR.md`, Reading the tables):
//!
//! - YREC (0.3-8 M☉): linear in log mass between the two bracketing
//!   tracks of each bracketing metallicity, then linear in [Fe/H]; log L,
//!   log R and log T_eff are interpolated. The star's ZAMS and TAMS ages
//!   are interpolated the same way in log t. At each bracketing
//!   metallicity, when both tracks around the mass reach their TAMS, each
//!   is read at the star's fractional main-sequence age at that metallicity
//!   τ = (t − t_ZAMS)/(t_TAMS − t_ZAMS); otherwise at the star's absolute
//!   age, each track held at its first or last row outside its ages. Below
//!   the lightest track that reaches its TAMS, the TAMS age is extrapolated
//!   in log t against log mass from the two lightest that do.
//! - Baraffe (0.08-0.35 M☉): bilinear in log mass and log age, held at the
//!   last row (10 Gyr) beyond it, plus the Dartmouth offset in dex,
//!   linear in mass over 0.15-0.3 M☉ (held outside) and in [Fe/H].
//! - Between 0.30 and 0.35 M☉ the two are blended in log, the YREC weight
//!   linear in mass from 0 to 1.

use super::tables::{LowMassTrack, Track, tables};
use super::{Bracket, bracket, bracket_clamped, bracket_log, lerp};
use crate::units::{Dex, Gyr, Kelvin, SolarLuminosity, SolarMass, SolarRadius};
use std::fmt;

/// Definition: lowest mass of the tables (the lightest Baraffe track),
/// near the hydrogen-burning limit.
pub const MIN_MASS: SolarMass = SolarMass(0.08);
/// Definition: highest mass of the tables (the heaviest YREC track).
pub const MAX_MASS: SolarMass = SolarMass(8.0);
/// Definition: lowest [Fe/H] of the tables (YREC grid).
pub const MIN_METALLICITY: Dex = Dex(-1.0);
/// Definition: highest [Fe/H] of the tables (YREC grid).
pub const MAX_METALLICITY: Dex = Dex(0.5);

/// Definition: the YREC runs stop at 20 Gyr; the Baraffe tracks are held
/// at their last row (10 Gyr) up to the same age (`STELLAR.md`).
const TABLE_END: Gyr = Gyr(20.0);

/// Definition: the YREC/Baraffe seam starts at 0.30 M☉, the lightest YREC
/// track: below it, Baraffe alone (`STELLAR.md`, The seam at 0.3 M☉).
const SEAM_START: f64 = 0.30;
/// Definition: the YREC/Baraffe seam ends at 0.35 M☉: from it, YREC alone
/// (`STELLAR.md`, The seam at 0.3 M☉).
const SEAM_END: f64 = 0.35;

/// A main-sequence star's present values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MainSequence {
    pub radius: SolarRadius,
    pub luminosity: SolarLuminosity,
    pub effective_temperature: Kelvin,
}

/// When a star leaves the main sequence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lifetime {
    /// At its terminal-age main sequence (TAMS), this age.
    Ends(Gyr),
    /// After this age, the end of the tables (20 Gyr): the tracks stop
    /// before the star leaves the main sequence.
    Beyond(Gyr),
}

impl Lifetime {
    /// The last age the tables answer for: the TAMS, or the end of the
    /// tables.
    pub fn age(self) -> Gyr {
        match self {
            Self::Ends(age) | Self::Beyond(age) => age,
        }
    }
}

/// A star's main-sequence life.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Life {
    /// Age at the zero-age main sequence (ZAMS).
    pub zams: Gyr,
    pub end: Lifetime,
}

/// Why the tables have no answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StellarError {
    /// Outside [`MIN_MASS`]..=[`MAX_MASS`].
    MassOutOfRange(SolarMass),
    /// Outside [`MIN_METALLICITY`]..=[`MAX_METALLICITY`].
    MetallicityOutOfRange(Dex),
    /// Still contracting towards the main sequence.
    BeforeZams { zams: Gyr },
    /// Past the TAMS, or past the end of the tables (20 Gyr) for a star
    /// whose life is [`Lifetime::Beyond`]; `tams` is that age.
    AfterTams { tams: Gyr },
}

impl fmt::Display for StellarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MassOutOfRange(mass) => write!(
                f,
                "mass {} Msun outside {}-{} Msun",
                mass.0, MIN_MASS.0, MAX_MASS.0
            ),
            Self::MetallicityOutOfRange(feh) => write!(
                f,
                "[Fe/H] {} dex outside {} to {} dex",
                feh.0, MIN_METALLICITY.0, MAX_METALLICITY.0
            ),
            Self::BeforeZams { zams } => {
                write!(
                    f,
                    "before the main sequence, which starts at {} Gyr",
                    zams.0
                )
            }
            Self::AfterTams { tams } => {
                write!(
                    f,
                    "after the main sequence, which the tables end at {} Gyr",
                    tams.0
                )
            }
        }
    }
}

impl std::error::Error for StellarError {}

/// When a star of `mass` and `metallicity` reaches and leaves the main
/// sequence.
pub fn life(mass: SolarMass, metallicity: Dex) -> Result<Life, StellarError> {
    check(mass, metallicity)?;
    let (m, feh) = (mass.0, metallicity.0);
    let w = yrec_weight(m);
    let low_zams = (w < 1.0).then(|| low_mass_zams(m));
    let high = (w > 0.0).then(|| yrec_life(m, feh, &yrec_tracks(m, feh)));
    Ok(match (low_zams, high) {
        (Some(zams), None) => Life {
            zams: Gyr(zams),
            end: Lifetime::Beyond(TABLE_END),
        },
        (None, Some(high)) => high,
        (Some(zams), Some(high)) => Life {
            zams: Gyr(lerp(zams.ln(), high.zams.0.ln(), w).exp()),
            end: Lifetime::Beyond(TABLE_END),
        },
        (None, None) => unreachable!("the YREC weight is in [0, 1]"),
    })
}

/// Radius, luminosity and effective temperature of a star of `mass` and
/// `metallicity` at `age`, between its ZAMS and the end of its [`life`]
/// (both included).
pub fn main_sequence(
    mass: SolarMass,
    metallicity: Dex,
    age: Gyr,
) -> Result<MainSequence, StellarError> {
    let life = life(mass, metallicity)?;
    if age.0.is_nan() || age.0 < life.zams.0 {
        return Err(StellarError::BeforeZams { zams: life.zams });
    }
    let end = life.end.age();
    if age.0 > end.0 {
        return Err(StellarError::AfterTams { tams: end });
    }
    let (m, feh, t) = (mass.0, metallicity.0, age.0);
    let w = yrec_weight(m);
    let mut logs = Logs::default();
    if w < 1.0 {
        logs.add(low_mass_logs(m, feh, t), 1.0 - w);
    }
    if w > 0.0 {
        logs.add(yrec_logs(m, feh, t), w);
    }
    Ok(MainSequence {
        radius: SolarRadius(10f64.powf(logs.log_r)),
        luminosity: SolarLuminosity(10f64.powf(logs.log_l)),
        effective_temperature: Kelvin(10f64.powf(logs.log_teff)),
    })
}

fn check(mass: SolarMass, metallicity: Dex) -> Result<(), StellarError> {
    if !(MIN_MASS.0..=MAX_MASS.0).contains(&mass.0) {
        return Err(StellarError::MassOutOfRange(mass));
    }
    if !(MIN_METALLICITY.0..=MAX_METALLICITY.0).contains(&metallicity.0) {
        return Err(StellarError::MetallicityOutOfRange(metallicity));
    }
    Ok(())
}

/// Weight of YREC in the seam: 0 below [`SEAM_START`], 1 from [`SEAM_END`].
fn yrec_weight(mass: f64) -> f64 {
    ((mass - SEAM_START) / (SEAM_END - SEAM_START)).clamp(0.0, 1.0)
}

/// Decimal logarithms of L, R and T_eff, summed with weights.
#[derive(Clone, Copy, Debug, Default)]
struct Logs {
    log_l: f64,
    log_r: f64,
    log_teff: f64,
}

impl Logs {
    fn add(&mut self, other: Logs, weight: f64) {
        self.log_l += other.log_l * weight;
        self.log_r += other.log_r * weight;
        self.log_teff += other.log_teff * weight;
    }
}

/// The YREC tracks around `mass` and `feh`, with their weights (summing
/// to 1): log mass within each bracketing metallicity, then [Fe/H].
fn yrec_tracks(mass: f64, feh: f64) -> Vec<(&'static Track, f64)> {
    let t = tables();
    let mut corners = Vec::new();
    let by_feh = bracket(&t.metallicities, feh).expect("metallicity checked against the grid");
    for (i, w_feh) in by_feh.terms() {
        let tracks = t.tracks_at(t.metallicities[i]);
        let masses: Vec<f64> = tracks.iter().map(|track| track.mass).collect();
        let by_mass = bracket_log(&masses, mass).expect("the YREC grid covers 0.3-8 Msun");
        corners.extend(by_mass.terms().map(|(j, w)| (&tracks[j], w * w_feh)));
    }
    corners
}

/// Weighted mean of `ln value(track)`, exponentiated.
fn log_mean(tracks: &[(&Track, f64)], value: impl Fn(&Track) -> f64) -> f64 {
    tracks
        .iter()
        .map(|(track, w)| value(track).ln() * w)
        .sum::<f64>()
        .exp()
}

fn yrec_life(mass: f64, feh: f64, tracks: &[(&Track, f64)]) -> Life {
    let zams = Gyr(log_mean(tracks, Track::zams));
    let end = match yrec_tams(mass, feh) {
        Some(tams) if tams <= TABLE_END.0 => Lifetime::Ends(Gyr(tams)),
        _ => Lifetime::Beyond(TABLE_END),
    };
    Life { zams, end }
}

/// The star's TAMS age, Gyr: linear in log t against log mass at each
/// bracketing metallicity ([`pair_tams`]), then against [Fe/H]; `None` when
/// one of them never ends within the tables.
fn yrec_tams(mass: f64, feh: f64) -> Option<f64> {
    let t = tables();
    let by_feh = bracket(&t.metallicities, feh).expect("metallicity checked against the grid");
    let mut log_tams = 0.0;
    for (i, w_feh) in by_feh.terms() {
        let tracks = t.tracks_at(t.metallicities[i]);
        log_tams += pair_tams(tracks, mass)?.ln() * w_feh;
    }
    Some(log_tams.exp())
}

/// The TAMS age at `mass` among the `tracks` of one metallicity, Gyr.
///
/// Both bracketing tracks end: interpolated in log t against log mass.
/// Only the heavier one ends at t_u (the lighter stops at 20 Gyr): log t
/// extrapolated against log mass from it and the next heavier track that
/// ends, at least t_u. Neither ends: `None`. The caller turns an age past
/// the end of the tables into [`Lifetime::Beyond`].
fn pair_tams(tracks: &[Track], mass: f64) -> Option<f64> {
    let masses: Vec<f64> = tracks.iter().map(|track| track.mass).collect();
    let by_mass = bracket_log(&masses, mass).expect("the YREC grid covers 0.3-8 Msun");
    if by_mass.terms().all(|(j, _)| tracks[j].reaches_tams()) {
        let log_tams: f64 = by_mass.terms().map(|(j, w)| tracks[j].end().ln() * w).sum();
        return Some(log_tams.exp());
    }
    let upper = &tracks[by_mass.hi];
    if by_mass.lo == by_mass.hi || !upper.reaches_tams() {
        return None;
    }
    let next = tracks[by_mass.hi + 1..]
        .iter()
        .find(|track| track.reaches_tams())
        .expect("the heaviest tracks reach their TAMS");
    let slope = (next.end() / upper.end()).ln() / (next.mass / upper.mass).ln();
    let tams = upper.end() * (mass / upper.mass).powf(slope);
    Some(tams.max(upper.end()))
}

/// How the YREC tracks of one tabulated metallicity are read for a star at
/// an age: at its fractional main-sequence age τ at that metallicity when
/// both tracks around its mass reach their TAMS, otherwise at its absolute
/// age. The choice is made per metallicity, so a star read between two
/// metallicities is continuous as its [Fe/H] crosses a grid value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TrackClock {
    age: f64,
    tau: Option<f64>,
}

impl TrackClock {
    /// The clock of a star of `mass` (in the YREC range) at `age` among the
    /// tracks of the tabulated metallicity `feh`: τ from the ZAMS age (log
    /// mean of the two tracks) and the TAMS age ([`pair_tams`]) there.
    pub fn new(mass: f64, feh: f64, age: f64) -> Self {
        let tracks = tables().tracks_at(feh);
        let masses: Vec<f64> = tracks.iter().map(|track| track.mass).collect();
        let by_mass = bracket_log(&masses, mass).expect("the YREC grid covers 0.3-8 Msun");
        let tau = if by_mass.terms().all(|(j, _)| tracks[j].reaches_tams()) {
            let log_zams: f64 = by_mass
                .terms()
                .map(|(j, w)| tracks[j].zams().ln() * w)
                .sum();
            let zams = log_zams.exp();
            pair_tams(tracks, mass).map(|tams| (age - zams) / (tams - zams))
        } else {
            None
        };
        Self { age, tau }
    }

    /// The bracket of rows at which `track` is read, linear in age and held
    /// at its first or last row outside its ages.
    pub fn rows(&self, track: &Track) -> Bracket {
        let track_age = match self.tau {
            Some(tau) => lerp(track.zams(), track.end(), tau),
            None => self.age,
        };
        let rows = &track.rows;
        bracket_clamped(rows.len(), |i| rows[i].age, track_age, |v| v)
    }
}

/// Central hydrogen over its ZAMS value read by `clock` at `mass` among the
/// tracks of the tabulated metallicity `feh`: linear in log mass between
/// the two tracks around it (a grid mass uses its track alone).
pub(crate) fn track_xc_frac(clock: &TrackClock, mass: f64, feh: f64) -> f64 {
    let tracks = tables().tracks_at(feh);
    let masses: Vec<f64> = tracks.iter().map(|track| track.mass).collect();
    let by_mass = bracket_log(&masses, mass).expect("the YREC grid covers 0.3-8 Msun");
    by_mass
        .terms()
        .map(|(j, w)| {
            let rows = &tracks[j].rows;
            let at = clock.rows(&tracks[j]);
            lerp(rows[at.lo].xc_frac, rows[at.hi].xc_frac, at.weight) * w
        })
        .sum()
}

/// Log L, R and T_eff from the YREC tracks: at each bracketing
/// metallicity, the two tracks around `mass` read by that metallicity's
/// [`TrackClock`] and linear in log mass; then linear in [Fe/H].
fn yrec_logs(mass: f64, feh: f64, age: f64) -> Logs {
    let t = tables();
    let by_feh = bracket(&t.metallicities, feh).expect("metallicity checked against the grid");
    let mut logs = Logs::default();
    for (i, w_feh) in by_feh.terms() {
        let tabulated = t.metallicities[i];
        let tracks = t.tracks_at(tabulated);
        let clock = TrackClock::new(mass, tabulated, age);
        let masses: Vec<f64> = tracks.iter().map(|track| track.mass).collect();
        let by_mass = bracket_log(&masses, mass).expect("the YREC grid covers 0.3-8 Msun");
        for (j, w) in by_mass.terms() {
            let rows = &tracks[j].rows;
            let at = clock.rows(&tracks[j]);
            logs.add(
                Logs {
                    log_l: lerp(rows[at.lo].log_l, rows[at.hi].log_l, at.weight),
                    log_r: lerp(rows[at.lo].log_r, rows[at.hi].log_r, at.weight),
                    log_teff: lerp(rows[at.lo].log_teff, rows[at.hi].log_teff, at.weight),
                },
                w * w_feh,
            );
        }
    }
    logs
}

/// The Baraffe tracks around `mass`, linear in log mass.
fn low_mass_bracket(mass: f64) -> (&'static [LowMassTrack], Bracket) {
    let tracks = &tables().low_mass;
    let masses: Vec<f64> = tracks.iter().map(|track| track.mass).collect();
    let by_mass = bracket_log(&masses, mass).expect("the Baraffe grid covers 0.08-0.35 Msun");
    (tracks, by_mass)
}

fn low_mass_zams(mass: f64) -> f64 {
    let (tracks, by_mass) = low_mass_bracket(mass);
    by_mass
        .terms()
        .map(|(i, w)| tracks[i].zams.ln() * w)
        .sum::<f64>()
        .exp()
}

fn low_mass_logs(mass: f64, feh: f64, age: f64) -> Logs {
    let (tracks, by_mass) = low_mass_bracket(mass);
    let mut logs = Logs::default();
    for (i, w) in by_mass.terms() {
        let rows = &tracks[i].rows;
        let at = bracket_clamped(rows.len(), |k| rows[k].age, age, f64::ln);
        logs.add(
            Logs {
                log_l: lerp(rows[at.lo].log_l, rows[at.hi].log_l, at.weight),
                log_r: lerp(rows[at.lo].log_r, rows[at.hi].log_r, at.weight),
                log_teff: lerp(rows[at.lo].log_teff, rows[at.hi].log_teff, at.weight),
            },
            w,
        );
    }
    logs.add(dartmouth_offset(mass, feh), 1.0);
    logs
}

/// Central temperature of a Baraffe star of `mass` at `age`, K: log T_c
/// bilinear in log mass and log age, held at the last row (10 Gyr) beyond
/// it, at solar metallicity (no Dartmouth offset).
pub(crate) fn low_mass_central_temperature(mass: f64, age: f64) -> f64 {
    let (tracks, by_mass) = low_mass_bracket(mass);
    let log_tc: f64 = by_mass
        .terms()
        .map(|(i, w)| {
            let rows = &tracks[i].rows;
            let at = bracket_clamped(rows.len(), |k| rows[k].age, age, f64::ln);
            lerp(rows[at.lo].log_tc, rows[at.hi].log_tc, at.weight) * w
        })
        .sum();
    10f64.powf(log_tc)
}

/// The Dartmouth offset at `mass` (held outside the tabulated masses) and
/// `feh` (zero at [Fe/H] 0), linear in both.
fn dartmouth_offset(mass: f64, feh: f64) -> Logs {
    let offsets = &tables().offsets;
    let mut fehs: Vec<f64> = offsets.iter().map(|o| o.feh).collect();
    fehs.push(0.0);
    fehs.sort_by(f64::total_cmp);
    fehs.dedup();
    let by_feh = bracket(&fehs, feh).expect("the Dartmouth offsets span the metallicities");
    let mut total = Logs::default();
    for (i, w_feh) in by_feh.terms() {
        let row: Vec<_> = offsets.iter().filter(|o| o.feh == fehs[i]).collect();
        if row.is_empty() {
            continue; // [Fe/H] 0: no offset
        }
        let at = bracket_clamped(row.len(), |k| row[k].mass, mass, |v| v);
        let (a, b) = (row[at.lo], row[at.hi]);
        total.add(
            Logs {
                log_l: lerp(a.d_log_l, b.d_log_l, at.weight),
                log_r: lerp(a.d_log_r, b.d_log_r, at.weight),
                log_teff: lerp(a.d_log_teff, b.d_log_teff, at.weight),
            },
            w_feh,
        );
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::SOLAR_EFFECTIVE_TEMPERATURE;
    use crate::units::{Dex, Gyr, SolarMass};

    /// Source: the Sun's age, 4.57 Gyr (Bahcall, Serenelli & Basu 2005,
    /// ApJ 621, L85).
    const SOLAR_AGE: Gyr = Gyr(4.57);

    #[test]
    fn the_sun_at_its_age() {
        // Source: IAU 2015 B3 nominal R, L and T_eff of the Sun.
        let s = main_sequence(SolarMass(1.0), Dex(0.0), SOLAR_AGE).unwrap();
        assert!((s.radius.0 - 1.0).abs() < 0.01, "{}", s.radius.0);
        assert!((s.luminosity.0 - 1.0).abs() < 0.02, "{}", s.luminosity.0);
        assert!(
            (s.effective_temperature.0 - SOLAR_EFFECTIVE_TEMPERATURE.0).abs() < 30.0,
            "{}",
            s.effective_temperature.0
        );
    }

    #[test]
    fn the_sun_leaves_the_main_sequence_near_ten_gyr() {
        match life(SolarMass(1.0), Dex(0.0)).unwrap().end {
            Lifetime::Ends(t) => assert!(9.0 < t.0 && t.0 < 11.0, "{}", t.0),
            Lifetime::Beyond(t) => panic!("Beyond({})", t.0),
        }
    }

    #[test]
    fn a_late_m_dwarf_matches_baraffe() {
        // Source: BHAC15 (Baraffe et al. 2015), 0.1 Msun at 5 Gyr: 2811 K,
        // 0.124 Rsun, log L -3.061.
        let s = main_sequence(SolarMass(0.1), Dex(0.0), Gyr(5.0)).unwrap();
        assert!((s.effective_temperature.0 / 2811.0 - 1.0).abs() < 0.01);
        assert!((s.radius.0 / 0.124 - 1.0).abs() < 0.01);
        assert!((s.luminosity.0.log10() + 3.061).abs() < 0.01);
    }

    #[test]
    fn a_late_m_dwarf_contracts_before_its_zams() {
        let zams = life(SolarMass(0.08), Dex(0.0)).unwrap().zams;
        assert!(zams.0 > 0.5, "{}", zams.0);
        assert!(matches!(
            main_sequence(SolarMass(0.08), Dex(0.0), Gyr(0.1)),
            Err(StellarError::BeforeZams { .. })
        ));
    }

    #[test]
    fn the_seam_at_three_tenths_is_smooth() {
        let below = main_sequence(SolarMass(0.29), Dex(0.0), Gyr(5.0)).unwrap();
        let above = main_sequence(SolarMass(0.31), Dex(0.0), Gyr(5.0)).unwrap();
        let ratio = above.effective_temperature.0 / below.effective_temperature.0;
        assert!((ratio - 1.0).abs() < 0.03, "{ratio}");
        assert!(above.luminosity.0 > below.luminosity.0);
    }

    #[test]
    fn metal_poor_stars_are_hotter() {
        let teff = |feh| {
            main_sequence(SolarMass(1.0), Dex(feh), Gyr(1.0))
                .unwrap()
                .effective_temperature
                .0
        };
        assert!(teff(-1.0) > teff(0.0));
        assert!(teff(0.0) > teff(0.5));
    }

    #[test]
    fn the_range_edges_are_finite() {
        for mass in [MIN_MASS, MAX_MASS] {
            for feh in [MIN_METALLICITY, MAX_METALLICITY] {
                let zams = life(mass, feh).unwrap().zams;
                let s = main_sequence(mass, feh, zams).unwrap();
                let values = [s.radius.0, s.luminosity.0, s.effective_temperature.0];
                assert!(
                    values.iter().all(|v| v.is_finite() && *v > 0.0),
                    "{} {}: {values:?}",
                    mass.0,
                    feh.0
                );
            }
        }
    }

    #[test]
    fn out_of_range_is_refused() {
        assert!(matches!(
            life(SolarMass(0.07), Dex(0.0)),
            Err(StellarError::MassOutOfRange(_))
        ));
        assert!(matches!(
            life(SolarMass(8.1), Dex(0.0)),
            Err(StellarError::MassOutOfRange(_))
        ));
        assert!(matches!(
            main_sequence(SolarMass(1.0), Dex(-1.1), Gyr(1.0)),
            Err(StellarError::MetallicityOutOfRange(_))
        ));
        assert!(matches!(
            main_sequence(SolarMass(1.0), Dex(0.6), Gyr(1.0)),
            Err(StellarError::MetallicityOutOfRange(_))
        ));
    }

    #[test]
    fn the_lifetime_is_continuous_below_the_first_ending_track() {
        // At [Fe/H] -1, 0.8 Msun is the lightest track that reaches its TAMS.
        let end = |mass: f64| match life(SolarMass(mass), Dex(-1.0)).unwrap().end {
            Lifetime::Ends(t) => t.0,
            Lifetime::Beyond(_) => f64::INFINITY,
        };
        let (below, at) = (end(0.79), end(0.8));
        assert!((below / at - 1.0).abs() < 0.1, "{below} vs {at}");
        let ends: Vec<f64> = (70..=100).map(|i| end(f64::from(i) / 100.0)).collect();
        for (i, pair) in ends.windows(2).enumerate() {
            assert!(pair[1] <= pair[0], "step {i} from 0.7 Msun: {pair:?}");
        }
    }

    #[test]
    fn values_are_continuous_across_a_grid_metallicity() {
        // Near 0.8-1 Msun some tracks stop before their TAMS: the τ or
        // absolute-age reading must not jump as [Fe/H] leaves a grid value.
        let log_l = |mass: f64, feh: f64, age: f64| {
            main_sequence(SolarMass(mass), Dex(feh), Gyr(age))
                .unwrap()
                .luminosity
                .0
                .log10()
        };
        for (mass, age) in [(0.85, 8.0), (0.95, 8.0), (0.95, 3.0)] {
            for (grid, off) in [(0.0, 0.001), (-0.5, -0.499)] {
                let jump = (log_l(mass, off, age) - log_l(mass, grid, age)).abs();
                assert!(jump < 0.002, "{mass} Msun {age} Gyr at {grid}: {jump} dex");
            }
        }
    }

    #[test]
    fn a_b_star_lives_briefly() {
        match life(SolarMass(8.0), Dex(0.0)).unwrap().end {
            Lifetime::Ends(t) => assert!(t.0 < 0.06, "{}", t.0),
            Lifetime::Beyond(t) => panic!("Beyond({})", t.0),
        }
    }
}
