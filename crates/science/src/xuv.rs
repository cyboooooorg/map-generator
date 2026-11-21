//! The X-ray and extreme-ultraviolet (XUV) light of a star over its life,
//! which drives the escape of a planet's envelope and atmosphere, and the
//! star's Lyman α line, which photolyses methane.
//!
//! A star with a convective envelope emits in each band a fixed share of
//! its luminosity at the zero-age main sequence (ZAMS) while its rotation
//! is fast (saturation, up to [`saturation`]), then that emission decays
//! as a power of age ([`ratio`]): the measured history of the Sun is a
//! power law of age in absolute flux (Ribas et al. 2005), while its
//! bolometric luminosity is not. A star without a convective envelope
//! (effective temperature at or above [`DYNAMO_LIMIT`]) emits no coronal
//! XUV ([`has_corona`]); the Lyman α line is not cut. [`history`] reads
//! the star's ZAMS and present luminosity and its temperature history from
//! the stellar tables ([`crate::stellar`]) once, for every band and planet.
//!
//! Ported from the tag `legacy-final` (`planet/xuv.rs`), which took the
//! present luminosity for the whole history, with the sources checked and
//! the ZAMS anchor added in the study
//! `docs/superpowers/studies/2026-11-21-xuv-history.md`.

use crate::stellar::{StellarError, life, main_sequence};
use crate::units::{
    Dex, Gyr, JoulePerSquareMetre, Kelvin, Second, SolarMass, WattPerSquareMetre, Year,
};

/// One band of a star's high-energy light: its luminosity over the star's
/// ZAMS luminosity while the star is saturated, and the power of age by
/// which it decays after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    /// Band luminosity over the ZAMS bolometric luminosity while
    /// saturated.
    pub saturated_ratio: f64,
    /// `L_band ∝ t^-decay_exponent` after saturation. Not 1.
    pub decay_exponent: f64,
    /// Whether the band comes from the corona, hence only while the star
    /// has one ([`has_corona`]).
    pub coronal: bool,
}

/// Source: the XUV flux of the Sun at 1 au against its age (1 to 1180 Å),
/// `F = 29.7 τ^-1.23` erg s⁻¹ cm⁻² (= mW/m²), τ in Gyr: Ribas et al. 2005,
/// ApJ 622, 680, doi:10.1086/427977, Table 5, from five solar analogues
/// aged 0.1 to 6.7 Gyr. `(flux at 1 Gyr in W/m², exponent)`.
pub const RIBAS_XUV: (f64, f64) = (29.7e-3, 1.23);

/// Source: the H I Lyman α flux of the Sun at 1 au against its age,
/// `F = 19.2 τ^-0.72` erg s⁻¹ cm⁻²: Ribas et al. 2005, eq. 2, from two
/// solar analogues ("preliminary"). `(flux at 1 Gyr in W/m², exponent)`.
pub const RIBAS_LYMAN_ALPHA: (f64, f64) = (19.2e-3, 0.72);

/// Source: age of the youngest solar analogue of Ribas et al. 2005 (EK Dra,
/// 0.1 Gyr): their fits start there.
pub const RIBAS_START: Gyr = Gyr(0.1);

/// Calibration: the XUV band (1 to 1180 Å). Exponent: [`RIBAS_XUV`]'s.
/// Share: [`RIBAS_XUV`] at [`RIBAS_START`] over today's solar constant
/// (3.706e-4, the `legacy-final` value), times the present over the ZAMS
/// luminosity of the 1 M☉, [Fe/H] 0 track of the stellar tables, so that
/// this Sun follows Ribas et al.'s fit at every age. Checked by the test
/// `the_sun_follows_ribas`.
pub const XUV: Band = Band {
    saturated_ratio: 5.220e-4,
    decay_exponent: 1.23,
    coronal: true,
};

/// Calibration: the Lyman α line (1216 Å), from [`RIBAS_LYMAN_ALPHA`] as
/// [`XUV`] (7.404e-5 times the same luminosity ratio). Its saturation time
/// is taken as the XUV's, and the line's larger share of an M dwarf's
/// light is not modelled. The line is chromospheric, and photospheric in
/// hot stars, so it is not cut at [`DYNAMO_LIMIT`]: an A or B star gets
/// the solar law, a lower bound, since their photosphere emits there.
pub const LYMAN_ALPHA: Band = Band {
    saturated_ratio: 1.043e-4,
    decay_exponent: 0.72,
    coronal: false,
};

/// Calibration: saturation time against the star's mass, `(M / M☉,
/// t_sat / Gyr)`, interpolated in log-log and extrapolated along the end
/// segments ([`saturation`]). Anchors, not a published relation:
///
/// - 1 M☉, 0.1 Gyr: Jackson, Davis & Wheatley 2012 (MNRAS 422, 2024,
///   Table 2) give `log τ_sat = 7.84` (70 Myr) for the B−V bin that holds
///   the Sun, and 10^7.8 to 10^8.3 yr over FGK stars; 0.1 Gyr is where
///   Ribas et al.'s fits start.
/// - 0.5 M☉, 0.2 Gyr and 0.25 M☉, 2 Gyr: Johnstone et al. 2021 (A&A 649,
///   A96), whose slow and fast rotator tracks stay "almost identical until
///   200 Myr and 2 Gyr respectively", when the rotation spread opens, read
///   as the end of saturation.
///
/// The slope is `M^-1` above 0.5 M☉ and `M^-3.3` below: 42 Gyr at 0.1 M☉,
/// saturated all its life.
const SATURATION: [(f64, f64); 3] = [(0.25, 2.0), (0.5, 0.2), (1.0, 0.1)];

/// Definition: least mass the saturation law is evaluated at, M☉, below
/// the lightest star of the tables.
const SATURATION_FLOOR: f64 = 0.05;

/// Source: effective temperature at and above which a star has no
/// convective envelope, hence no dynamo and no corona: the granulation
/// boundary of Gray & Nagel 1989 (ApJ 341, 421) crosses the main sequence
/// near F0, 7 220 K in Pecaut & Mamajek's sequence (the limit of
/// `legacy-final`'s `activity.rs`). A stars show no X-rays of their own
/// (Schröder & Schmitt 2007); O and B stars emit `L_X/L_bol ≈ 1e-7` from
/// wind shocks, neglected here.
pub const DYNAMO_LIMIT: Kelvin = Kelvin(7_200.0);

/// Definition: ages at which [`history`] samples the effective
/// temperature between the ZAMS and the star's age, to find where the
/// star gains or loses its corona.
const TEMPERATURE_SAMPLES: u32 = 64;

/// Definition: bisection steps that place a corona boundary, each halving
/// the interval between two samples (60 reach the f64 resolution).
const BISECTION_STEPS: u32 = 60;

/// Definition: XUV energy per unit area the Sun delivered at 1 au from 0
/// to [`crate::heat::EARTH_AGE`], J/m²: [`RIBAS_XUV`] integrated, held at
/// its [`RIBAS_START`] value before it (the model's Sun, checked by the
/// test `earth_fluence_is_the_ribas_integral`). The reference of escape
/// laws calibrated on Earth.
pub const EARTH_XUV_FLUENCE: JoulePerSquareMetre = JoulePerSquareMetre(5.64e15);

/// Saturation time of a star of `mass`: the log-log interpolation of
/// [`SATURATION`], extrapolated along its end segments, the mass floored
/// at 0.05 M☉. The helpers below take finite inputs; [`history`] checks
/// its own through the stellar tables.
pub fn saturation(mass: SolarMass) -> Gyr {
    let x = mass.0.max(SATURATION_FLOOR).ln();
    let i = usize::from(x >= SATURATION[1].0.ln());
    let (m0, t0) = SATURATION[i];
    let (m1, t1) = SATURATION[i + 1];
    let slope = (t1 / t0).ln() / (m1 / m0).ln();
    Gyr((t0.ln() + slope * (x - m0.ln())).exp())
}

/// Whether a star of effective temperature `t_eff` has a convective
/// envelope, hence a corona: below [`DYNAMO_LIMIT`].
pub fn has_corona(t_eff: Kelvin) -> bool {
    t_eff.0 < DYNAMO_LIMIT.0
}

/// Band luminosity of a star of `mass` with a corona at `age`, over its
/// ZAMS luminosity: saturated up to [`saturation`], then decaying.
pub fn ratio(band: Band, mass: SolarMass, age: Gyr) -> f64 {
    let t_sat = saturation(mass).0;
    band.saturated_ratio * (age.0.max(t_sat) / t_sat).powf(-band.decay_exponent)
}

/// Time integral of [`ratio`] from 0 to `age`, Gyr: constant while
/// saturated, then the integral of the power law.
pub fn integrated_ratio(band: Band, mass: SolarMass, age: Gyr) -> Gyr {
    let t_sat = saturation(mass).0;
    let age = age.0.max(0.0);
    if age <= t_sat {
        return Gyr(band.saturated_ratio * age);
    }
    let n = band.decay_exponent - 1.0;
    let decayed = t_sat * (1.0 - (age / t_sat).powf(-n)) / n;
    Gyr(band.saturated_ratio * (t_sat + decayed))
}

/// Age from which [`ratio`] for a star of `mass` is at most `share`: 0 if
/// it never exceeds it (at or above the saturated share), infinite if it
/// never falls to it (zero or less).
pub fn age_below_ratio(band: Band, mass: SolarMass, share: f64) -> Gyr {
    if share >= band.saturated_ratio {
        Gyr(0.0)
    } else if share <= 0.0 {
        Gyr(f64::INFINITY)
    } else {
        Gyr(saturation(mass).0 * (share / band.saturated_ratio).powf(-1.0 / band.decay_exponent))
    }
}

/// What a planet receives from a star in one band, relative to the star's
/// present bolometric light at the planet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exposure {
    /// Present band flux over present bolometric flux; 0 for a star
    /// without a corona today.
    pub present_ratio: f64,
    /// Band energy received from 0 to the age, in units of the present
    /// bolometric flux, Gyr.
    pub integrated: Gyr,
}

impl Exposure {
    /// The band flux at a planet whose present bolometric flux from the
    /// star is `flux`.
    pub fn flux(self, flux: WattPerSquareMetre) -> WattPerSquareMetre {
        flux * self.present_ratio
    }

    /// The band energy per unit area a planet whose present bolometric
    /// flux from the star is `flux` received over the star's age. The
    /// orbit is taken as it is today.
    pub fn fluence(self, flux: WattPerSquareMetre) -> JoulePerSquareMetre {
        JoulePerSquareMetre(flux.0 * Second::from(Year::from(self.integrated)).0)
    }
}

/// The high-energy history of one star at its present age, which serves
/// every band and every planet around it.
#[derive(Clone, Debug, PartialEq)]
pub struct History {
    mass: SolarMass,
    age: Gyr,
    /// ZAMS over present luminosity.
    scale: f64,
    /// Ages, from 0, at which the star has a corona.
    corona: Vec<(f64, f64)>,
    corona_now: bool,
}

/// The [`History`] of a star of `mass` and `metallicity` at `age`, which
/// must lie on its main sequence (as every drawn star's does), else the
/// [`StellarError`] of [`main_sequence`]. The ZAMS and present luminosity
/// come from the stellar tables, and the corona from the effective
/// temperature along the track ([`has_corona`]). Before the ZAMS, where
/// [`main_sequence`] gives no values, the star is taken at its ZAMS
/// temperature and the band at its saturated share of the ZAMS
/// luminosity, from age 0: a lower bound, since a contracting star is
/// brighter. The decay runs on the star's age even for a star that gains
/// its corona late, by cooling through [`DYNAMO_LIMIT`].
pub fn history(mass: SolarMass, metallicity: Dex, age: Gyr) -> Result<History, StellarError> {
    let present = main_sequence(mass, metallicity, age)?;
    let zams = life(mass, metallicity)?.zams;
    let at_zams = main_sequence(mass, metallicity, zams)?;
    let corona_at = |t: f64| -> Result<bool, StellarError> {
        Ok(has_corona(
            main_sequence(mass, metallicity, Gyr(t))?.effective_temperature,
        ))
    };

    let mut corona: Vec<(f64, f64)> = Vec::new();
    let mut start = has_corona(at_zams.effective_temperature).then_some(0.0);
    let mut previous = (zams.0, start.is_some());
    for i in 1..=TEMPERATURE_SAMPLES {
        // The last sample is the age itself, so that rounding cannot step
        // past the end of the tables.
        let t = if i == TEMPERATURE_SAMPLES {
            age.0
        } else {
            zams.0 + (age.0 - zams.0) * f64::from(i) / f64::from(TEMPERATURE_SAMPLES)
        };
        let now = corona_at(t)?;
        if now != previous.1 {
            let edge = boundary(previous.0, t, previous.1, &corona_at)?;
            match start.take() {
                Some(from) => corona.push((from, edge)),
                None => start = Some(edge),
            }
        }
        previous = (t, now);
    }
    if let Some(from) = start {
        corona.push((from, age.0));
    }
    Ok(History {
        mass,
        age,
        scale: at_zams.luminosity.0 / present.luminosity.0,
        corona,
        corona_now: has_corona(present.effective_temperature),
    })
}

impl History {
    /// The star's ZAMS over its present luminosity: turns [`ratio`] into a
    /// share of the present light.
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// The ages from 0 at which the star has a corona, in order.
    pub fn corona(&self) -> &[(f64, f64)] {
        &self.corona
    }

    /// Band energy a planet received between ages `from` and `to` (within
    /// 0 and the star's age), in units of the present bolometric flux,
    /// Gyr; a coronal band counts only while the star has a corona.
    pub fn integrated_between(&self, band: Band, from: Gyr, to: Gyr) -> Gyr {
        let (from, to) = (from.0.max(0.0), to.0.min(self.age.0));
        let all = [(0.0, self.age.0)];
        let spans: &[(f64, f64)] = if band.coronal { &self.corona } else { &all };
        let sum: f64 = spans
            .iter()
            .map(|&(a, b)| (a.max(from), b.min(to)))
            .filter(|(a, b)| b > a)
            .map(|(a, b)| {
                integrated_ratio(band, self.mass, Gyr(b)).0
                    - integrated_ratio(band, self.mass, Gyr(a)).0
            })
            .sum();
        Gyr(sum * self.scale)
    }

    /// The [`Exposure`] in `band` of a planet around the star.
    pub fn exposure(&self, band: Band) -> Exposure {
        let present_ratio = if band.coronal && !self.corona_now {
            0.0
        } else {
            ratio(band, self.mass, self.age) * self.scale
        };
        Exposure {
            present_ratio,
            integrated: self.integrated_between(band, Gyr(0.0), self.age),
        }
    }
}

/// The [`Exposure`] in `band` of a planet around a star of `mass` and
/// `metallicity` at `age`: [`history`] for a single band.
pub fn exposure(
    band: Band,
    mass: SolarMass,
    metallicity: Dex,
    age: Gyr,
) -> Result<Exposure, StellarError> {
    Ok(history(mass, metallicity, age)?.exposure(band))
}

/// The age between `a` and `b` where `corona` turns from `at_a`, by
/// bisection.
fn boundary(
    mut a: f64,
    mut b: f64,
    at_a: bool,
    corona: &impl Fn(f64) -> Result<bool, StellarError>,
) -> Result<f64, StellarError> {
    for _ in 0..BISECTION_STEPS {
        let mid = 0.5 * (a + b);
        if corona(mid)? == at_a {
            a = mid;
        } else {
            b = mid;
        }
    }
    Ok(0.5 * (a + b))
}

#[cfg(test)]
#[path = "xuv_tests.rs"]
mod tests;
