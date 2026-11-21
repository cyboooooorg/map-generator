//! Orbit laws of the system draw: Kepler's third law, the light an orbit
//! receives and the temperature it sets, the ice line, the Roche floor, and
//! the spacing of neighbours in mutual Hill radii with its stability floors.
//!
//! The numbers and their sources were read in the study
//! `docs/superpowers/studies/2026-11-21-architecture-families/` (blocks
//! "Mutual Hill radius", "Stability floor between neighbours", "Inner
//! edge", "Host scaling" of its README, and sections 2.1, 2.3, 5 and 6 of
//! `small-planets-and-spacing.md`, written `S` below). The flux and
//! temperature factors are ported from the tag `legacy-final`
//! (`star.rs`).

use crate::constants::{ASTRONOMICAL_UNIT, SOLAR_MASS_PARAMETER};
use crate::units::{
    AstronomicalUnit, Day, EarthFlux, EarthMass, Fraction, Gyr, Kelvin, Kilogram,
    KilogramPerCubicMetre, Radian, Second, SolarLuminosity, SolarMass, Year,
};
use std::f64::consts::TAU;

/// Calibration: equilibrium temperature of a fast rotator under one Earth
/// flux with Earth's Bond albedo 0.3, 255 K (legacy `star.rs`
/// `equilibrium_k`): `(S (1 - 0.3) / 4σ)^¼` = 254.6 K with 1361 W/m².
pub const EARTH_EQUILIBRIUM: Kelvin = Kelvin(255.0);

/// Source: the ice line of a 1 M☉ host, 2.7 au: Ida & Lin 2005, quoted by
/// Kennedy & Kenyon 2008 (Fig. 1, p4), `a_snow = 2.7 au × M / M☉`
/// (architecture study, "Host scaling"; G 3.4). Decided by the user on
/// 2026-11-21. The linear form overshoots above 3 M☉: 21.6 au at 8 M☉,
/// where Kennedy & Kenyon's own model finds 10 to 15 au.
pub const ICE_LINE_OF_THE_SUN: AstronomicalUnit = AstronomicalUnit(2.7);

/// Source: the Roche period of an incompressible fluid planet of
/// 1 g cm⁻³, 12.6 h, in days: Rappaport et al. 2013 (ApJL 773, L15)
/// eq. 2, independent of the host (S 6).
pub const ROCHE_PERIOD_AT_REFERENCE: Day = Day(12.6 / 24.0);

/// Definition: 1 g cm⁻³, the density [`ROCHE_PERIOD_AT_REFERENCE`] is
/// quoted at.
pub const ROCHE_REFERENCE_DENSITY: KilogramPerCubicMetre = KilogramPerCubicMetre(1000.0);

/// Source: the Hill stability floor of two planets on circular coplanar
/// orbits, `Δ > 2√3` mutual Hill radii (Gladman 1993, read in Fabrycky et
/// al. 2014 eq. 8, Pu & Wu 2015 p2, Petit et al. 2020 eq. 1; S 2.3).
pub const TWO_PLANET_FLOOR: f64 = 3.464_101_615_137_754_6;

/// Source: Pu & Wu 2015 (ApJ 807, 44) eq. 12, `K50 = 0.7 log10(t / T1) +
/// 2.87`: `(slope per decade of innermost orbits, intercept)` (S 2.3).
const PU_WU_K50: (f64, f64) = (0.7, 2.87);

/// Source: Pu & Wu 2015 eq. 14, one more Hill radius of `K50` per 0.01 of
/// the Rayleigh eccentricity scale and per 0.04 rad of the inclination
/// scale: `(σ_e per Hill radius, σ_inc per Hill radius in rad)` (S 2.3).
/// Their scan covers σ_e ≤ 0.05 and σ_inc ≤ 0.16 rad.
const PU_WU_SCALES: (f64, f64) = (0.01, 0.04);

/// Orbital period at semi-major axis `a` around a host of `host` solar
/// masses: Kepler's third law `2π sqrt(a³ / GM)`, with the nominal solar
/// mass parameter. The planet's mass is neglected (3e-6 of the host's for
/// the Earth, 1e-3 for Jupiter).
pub fn period(a: AstronomicalUnit, host: SolarMass) -> Day {
    let a = a.0 * ASTRONOMICAL_UNIT.0;
    let gm = SOLAR_MASS_PARAMETER * host.0;
    Day::from(Second(TAU * (a * a * a / gm).sqrt()))
}

/// Semi-major axis of an orbit of `period` around `host`: the inverse of
/// [`period`], planet mass neglected likewise.
pub fn semi_major_axis(period: Day, host: SolarMass) -> AstronomicalUnit {
    let n = Second::from(period).0 / TAU;
    let gm = SOLAR_MASS_PARAMETER * host.0;
    AstronomicalUnit((gm * n * n).cbrt() / ASTRONOMICAL_UNIT.0)
}

/// Flux on a circular orbit of radius `a` around a star of `luminosity`:
/// `L / a²` in Earth fluxes. One L☉ at 1 au is one Earth flux: the IAU
/// 2015 nominal values give 3.828e26 W / 4π au² = 1361.2 W/m², 0.02 %
/// above the nominal irradiance.
pub fn circular_flux(luminosity: SolarLuminosity, a: AstronomicalUnit) -> EarthFlux {
    EarthFlux(luminosity.0 / (a.0 * a.0))
}

/// Flux of an orbit of eccentricity `e` averaged over its year, relative
/// to a circular orbit of the same semi-major axis: `1 / sqrt(1 - e²)`
/// (legacy `star.rs`; 1.0001 for the Earth, 1.02 at e = 0.2). Infinite at
/// `e = 1`, which is no orbit.
pub fn mean_flux_factor(e: Fraction) -> f64 {
    1.0 / (1.0 - e.get() * e.get()).sqrt()
}

/// Equilibrium temperature of an orbit of eccentricity `e` averaged over
/// its year, relative to a circular orbit of the same semi-major axis:
/// `1 - e²/16 - 15e⁴/1024` (Méndez & Rivera-Valentín 2017, ApJL 837, L1,
/// eq. 16, as legacy `star.rs`; 0.997 at e = 0.2). It falls while the
/// mean flux rises: the planet spends longer far out.
pub fn mean_temperature_factor(e: Fraction) -> f64 {
    let e2 = e.get() * e.get();
    1.0 - e2 / 16.0 - 15.0 * e2 * e2 / 1024.0
}

/// Year-averaged equilibrium temperature under the **circular** flux
/// `circular` ([`circular_flux`] at the semi-major axis) on an orbit of
/// eccentricity `e`: `255 K × S^¼ × mean_temperature_factor(e)`. The
/// factor carries the eccentricity, so `circular` must not include
/// [`mean_flux_factor`].
pub fn equilibrium(circular: EarthFlux, e: Fraction) -> Kelvin {
    EARTH_EQUILIBRIUM * (circular.0.powf(0.25) * mean_temperature_factor(e))
}

/// The disk's ice line around a host of `host` solar masses:
/// [`ICE_LINE_OF_THE_SUN`] times the mass (Ida & Lin 2005, overshoot above
/// 3 M☉ stated there).
pub fn ice_line(host: SolarMass) -> AstronomicalUnit {
    ICE_LINE_OF_THE_SUN * host.0
}

/// Shortest period a fluid planet of mean `density` survives on, whatever
/// the host: `12.6 h × (ρ / 1 g cm⁻³)^-½` (Rappaport et al. 2013 eq. 2,
/// Roche coefficient 2.44; S 6). For a 1 M_J planet of 1.3 R_J (604 kg/m³)
/// it gives 0.68 d; Dawson & Johnson 2018 eq. 15 quote 0.79 d for the same
/// planet with the coefficient 2.7, (2.7 / 2.44)^(3/2) = 1.16 times
/// longer.
pub fn roche_period(density: KilogramPerCubicMetre) -> Day {
    ROCHE_PERIOD_AT_REFERENCE * (density.0 / ROCHE_REFERENCE_DENSITY.0).powf(-0.5)
}

/// One planet of a neighbouring pair, for [`mutual_hill_spacing`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pair {
    /// The planet's mass.
    pub mass: EarthMass,
    /// Its semi-major axis.
    pub a: AstronomicalUnit,
    /// Its eccentricity.
    pub e: Fraction,
}

/// Spacing of two neighbours in mutual Hill radii, periapsis of the outer
/// over apoapsis of the inner (He et al. 2019 eq. 10-11, He et al. 2020
/// eq. 8; S 2.1): `(a_out (1 - e_out) - a_in (1 + e_in)) / R_H`, with
/// `R_H = ((m_in + m_out) / 3M)^⅓ (a_in + a_out) / 2` (Weiss et al. 2018
/// eq. 3, Fabrycky et al. 2014 eq. 2, Pu & Wu 2015 eq. 1). Circular orbits
/// give the usual `(a_out - a_in) / R_H`; overlapping orbits give a
/// negative spacing.
pub fn mutual_hill_spacing(inner: Pair, outer: Pair, host: SolarMass) -> f64 {
    let masses = Kilogram::from(inner.mass).0 + Kilogram::from(outer.mass).0;
    let ratio = masses / Kilogram::from(host).0;
    let hill = (ratio / 3.0).cbrt() * (inner.a.0 + outer.a.0) / 2.0;
    let gap = outer.a.0 * (1.0 - outer.e.get()) - inner.a.0 * (1.0 + inner.e.get());
    gap / hill
}

/// Median-survival spacing of a system of three or more planets, in mutual
/// Hill radii (Pu & Wu 2015 eq. 12 and 14; S 2.3): `0.7 log10(t / T1) +
/// 2.87 + σ_e / 0.01 + σ_inc / 0.04`, with `t` the system's `age`, `T1`
/// its `innermost` period and `sigma_e`, `sigma_inc` the Rayleigh scales
/// of its eccentricities and mutual inclinations. About 10 at 1 Gyr for
/// circular coplanar orbits, 12 for Kepler-like ones. Fitted for
/// σ_e ≤ 0.05 and σ_inc ≤ 0.16 rad, on 7 planets of 3 to 9 M⊕; beyond,
/// an extrapolation. Half of such systems survive to `t` at this spacing:
/// a population median, not a guarantee.
pub fn k50(age: Gyr, innermost: Day, sigma_e: f64, sigma_inc: Radian) -> f64 {
    let age = Second::from(Year::from(age)).0;
    let orbits = age / Second::from(innermost).0;
    let (slope, intercept) = PU_WU_K50;
    let (per_e, per_inc) = PU_WU_SCALES;
    slope * orbits.log10() + intercept + sigma_e / per_e + sigma_inc.0 / per_inc
}

#[cfg(test)]
#[path = "orbit_tests.rs"]
mod tests;
