//! The steam envelope of a runaway water-rich planet and the steam world
//! line.
//!
//! Turbet et al. 2020, "Revised mass-radius relationships for water-rich
//! rocky planets more irradiated than the runaway greenhouse limit"
//! (*A&A* 638, A41, doi:10.1051/0004-6361/201937151, arXiv:1911.08878v3):
//! Eq. 2 (in the form of Appendix C, C.8) for the thickness of an
//! isothermal steam envelope up to its transit radius, C.9 and C.10 for
//! the steam's weight at the core, and Eq. 3 with Table 2 for its
//! temperature. Pierrehumbert 2023 (*ApJ* 944, 20,
//! doi:10.3847/1538-4357/acafdf) § 1 for the line past which no surface
//! shows: a column whose base is supercritical.
//!
//! Values, sources and the one calibration (the thickness cap): study 26,
//! in `docs/science/planet-atmospheres.md` ("Boiling and the steam
//! envelope of the rewrite (study 26)") and the Wiki's
//! `26-boiling-and-steam-audit`. Past about 10 % water the fit's core is
//! compressed and Aguichine et al. 2021 would apply; the laws here stay
//! Turbet's, clamped (a documented departure, issue #684).

use crate::constants::{
    GRAVITATIONAL_CONSTANT, MOLAR_GAS_CONSTANT, PASCALS_PER_BAR, STANDARD_GRAVITY,
};
use crate::radiation::gases::H2O_MOLAR_MASS;
use crate::units::{
    Bar, EarthFlux, Fraction, Kelvin, Kilogram, Metre, MetrePerSecondSquared, Pascal,
};
use crate::water::{CRITICAL_POINT_TEMPERATURE, CRITICAL_PRESSURE};
use std::f64::consts::PI;

/// Source: Turbet et al. 2020, Table 2 of arXiv:1911.08878v3 (the version
/// published in A&A, with Table 2's typos corrected), `α₁` to `α₆`, which
/// centre and scale the inputs of Eq. 3. v1 and v2 print a wrong α
/// (`α₁` = +3.550, the others in another order): never use them.
pub const TURBET_ALPHA: [f64; 6] = [-3.550, 1.310, 1.099, 4.683e-1, 7.664e-1, 4.224e-1];

/// Source: Turbet et al. 2020, Table 2 of arXiv:1911.08878v3, `β₁` to
/// `β₁₀`, the weights of the terms of Eq. 3.
pub const TURBET_BETA: [f64; 10] = [
    2.846, 1.555e-1, 8.777e-2, 6.045e-2, 1.143e-2, 1.736e-2, 1.859e-2, 4.314e-2, 3.393e-2,
    -1.034e-2,
];

/// Source: Turbet et al. 2020 § 2.2, the pressure of the transit radius:
/// it "varies little across the range of parameters we explored and is
/// roughly equal to 10⁻¹ Pa".
pub const TRANSIT_PRESSURE: Pascal = Pascal(0.1);

/// Source: Turbet et al. 2020 § 2.2, Eq. 3 valid "for irradiation from 1
/// to 30 S⊕".
pub const INSOLATION_RANGE: (EarthFlux, EarthFlux) = (EarthFlux(1.0), EarthFlux(30.0));

/// Source: Turbet et al. 2020 § 2.2, Eq. 3 valid for a "surface gravity
/// from 0.2 to 6 g⊕". `g⊕` is `STANDARD_GRAVITY` (Definition: 9.806 65
/// m/s², CGPM 1901).
pub const GRAVITY_RANGE: (MetrePerSecondSquared, MetrePerSecondSquared) = (
    MetrePerSecondSquared(0.2 * STANDARD_GRAVITY.0),
    MetrePerSecondSquared(6.0 * STANDARD_GRAVITY.0),
);

/// Source: Turbet et al. 2020 § 2.1, the steam pressures of the
/// atmospheric structures the fit was made on, 27 bar to 270 kbar (in
/// Pa). § 2.2 states Eq. 3 valid to 27 kbar only, but clamped there a
/// 4.78 M⊕ body with 12 % water comes out smaller than the same water
/// condensed, against the paper's own result (§ 3.1); up to the simulated
/// 270 kbar the polynomial is still on its grid (study 26).
pub const STEAM_PRESSURE_RANGE: (Pascal, Pascal) = (Pascal(2.7e6), Pascal(2.7e10));

/// Calibration: the steam envelope's thickness is capped at this many
/// core radii. Eq. 2 has no finite thickness once `R T ln(P_surf/P_t) /
/// (M g)` reaches the core's radius (an unbound envelope, the "U-shape"
/// of Turbet et al. § 3.1; C.9 "does not hold for inflated
/// atmospheres"). The cap binds once Eq. 2's thickness reaches
/// `THICKNESS_CAP × R_core`, that is once `R T ln(P_surf/P_t) / (M g)`
/// reaches `THICKNESS_CAP / (1 + THICKNESS_CAP)` of the core's radius
/// (half of it at 1), so it also caps bound envelopes thicker than the
/// cap (the worked example's core past about 5.7 S⊕). Legacy's value, kept. Searched and not found: a
/// published thickness or radius past which a steam envelope stops being
/// bound, in Turbet et al. 2020, Aguichine et al. 2021 and Pierrehumbert
/// 2023 (`docs/science/planet-atmospheres.md`, "The steam envelope
/// (Turbet et al. 2020)", "Calibration of the thickness cap").
pub const THICKNESS_CAP: f64 = 1.0;

/// Definition: water's critical pressure in bar, `CRITICAL_PRESSURE` over
/// `PASCALS_PER_BAR` (220.64 bar): at and past it, with the critical
/// temperature, the base of the column is supercritical and no surface
/// shows (Pierrehumbert 2023 § 1). User decision of 2026-11-24 (study 26).
pub const STEAM_WORLD_MIN_BAR: Bar = Bar(CRITICAL_PRESSURE.0 / PASCALS_PER_BAR);

/// The steam's weight at the core, `x/(1−x) g²/(4πG)` (Turbet et al. 2020,
/// C.9 and C.10), `x` the water's share of the whole planet and `g` the
/// core's gravity: the mass of the steam, `x/(1−x) M_core`, times `g` over
/// the core's area.
pub fn base_pressure(water_share: Fraction, core_gravity: MetrePerSecondSquared) -> Pascal {
    let x = water_share.get();
    Pascal(x / (1.0 - x) * core_gravity.0 * core_gravity.0 / (4.0 * PI * GRAVITATIONAL_CONSTANT))
}

/// The temperature of the isothermal steam envelope above its
/// photosphere: Eq. 3 and Table 2 of Turbet et al. 2020, `log₁₀ T = β₁ +
/// β₂x + β₃y + β₄z + β₅xy + β₆y² + β₇x³ + β₈x²y + β₉xy² + β₁₀y⁴`, `x =
/// (log₁₀ x_H₂O − α₁)/α₂`, `y = (log₁₀ g − α₃)/α₄` (m/s²), `z = (log₁₀ S
/// − α₅)/α₆` (S⊕).
///
/// The fit is a polynomial that runs off its grid (its `x³` term grows
/// fast), so its inputs are clamped: `S` to [`INSOLATION_RANGE`], `g` to
/// [`GRAVITY_RANGE`], and the share to the shares whose
/// [`base_pressure`] at the clamped `g` lies in [`STEAM_PRESSURE_RANGE`].
/// Ported from legacy `steam_temperature_k` (`legacy-final`,
/// `planet/interior.rs`).
pub fn steam_temperature(
    water_share: Fraction,
    core_gravity: MetrePerSecondSquared,
    insolation: EarthFlux,
) -> Kelvin {
    let g = core_gravity.0.clamp(GRAVITY_RANGE.0.0, GRAVITY_RANGE.1.0);
    // The share whose steam weighs `p` at `g`, the inverse of
    // `base_pressure`.
    let share_at = |p: Pascal| {
        let ratio = p.0 * 4.0 * PI * GRAVITATIONAL_CONSTANT / (g * g);
        ratio / (1.0 + ratio)
    };
    let f = water_share.get().clamp(
        share_at(STEAM_PRESSURE_RANGE.0),
        share_at(STEAM_PRESSURE_RANGE.1),
    );
    let s = insolation
        .0
        .clamp(INSOLATION_RANGE.0.0, INSOLATION_RANGE.1.0);
    let [a1, a2, a3, a4, a5, a6] = TURBET_ALPHA;
    let b = TURBET_BETA;
    let x = (f.log10() - a1) / a2;
    let y = (g.log10() - a3) / a4;
    let z = (s.log10() - a5) / a6;
    let log_t = b[0]
        + b[1] * x
        + b[2] * y
        + b[3] * z
        + b[4] * x * y
        + b[5] * y * y
        + b[6] * x.powi(3)
        + b[7] * x * x * y
        + b[8] * x * y * y
        + b[9] * y.powi(4);
    Kelvin(10f64.powf(log_t))
}

/// A steam envelope on its core, every value from one call
/// ([`envelope`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Envelope {
    /// The steam's weight at the core, [`base_pressure`].
    pub base_pressure: Pascal,
    /// The core's gravity, `G M_core / R_core²`.
    pub core_gravity: MetrePerSecondSquared,
    /// Eq. 3's temperature, [`steam_temperature`].
    pub temperature: Kelvin,
    /// Eq. 2's thickness up to the transit radius, capped at
    /// [`THICKNESS_CAP`] core radii.
    pub thickness: Metre,
}

/// The steam envelope of `water_share` (the water's share of the whole
/// planet, Turbet et al. Appendix D: `M_planet = M_core/(1−x)`) on a core
/// of `core_mass` and `core_radius`, under `insolation`.
///
/// Turbet et al. 2020 Eq. 2, as Appendix C writes it (C.8): `1/z = M g /
/// (R T ln(P_surf/P_t)) − 1/R_core`, an isothermal envelope in
/// hydrostatic balance under a gravity falling as `1/r²`, `M` the molar
/// mass of water, `g` the core's gravity, `P_surf` the [`base_pressure`],
/// `P_t` the [`TRANSIT_PRESSURE`] and `T` the [`steam_temperature`]
/// (clamped to its grid; Eq. 2 itself takes the planet's own share and
/// gravity). The thickness is 0 without steam, when `P_surf <= P_t` or
/// on an empty core (no mass or no radius, legacy's guard; its gravity
/// and base pressure are then 0, so every field stays finite), and `THICKNESS_CAP × R_core` when
/// `(1/z) R_core THICKNESS_CAP <= 1`, which caps both unbound envelopes
/// and bound ones past the cap. Ported from legacy `turbet_thickness`
/// (`legacy-final`, `planet/interior.rs`).
pub fn envelope(
    core_mass: Kilogram,
    core_radius: Metre,
    water_share: Fraction,
    insolation: EarthFlux,
) -> Envelope {
    let empty_core = core_mass.0 <= 0.0 || core_radius.0 <= 0.0;
    let core_gravity = if empty_core {
        MetrePerSecondSquared(0.0)
    } else {
        MetrePerSecondSquared(
            GRAVITATIONAL_CONSTANT * core_mass.0 / (core_radius.0 * core_radius.0),
        )
    };
    let base_pressure = if empty_core {
        Pascal(0.0)
    } else {
        base_pressure(water_share, core_gravity)
    };
    let temperature = steam_temperature(water_share, core_gravity, insolation);
    let depth = (base_pressure.0 / TRANSIT_PRESSURE.0).ln();
    let thickness = if empty_core || water_share.get() <= 0.0 || depth <= 0.0 {
        Metre(0.0)
    } else {
        let inverse = H2O_MOLAR_MASS * core_gravity.0
            / (MOLAR_GAS_CONSTANT * temperature.0 * depth)
            - 1.0 / core_radius.0;
        if inverse * core_radius.0 * THICKNESS_CAP <= 1.0 {
            core_radius * THICKNESS_CAP
        } else {
            Metre(1.0 / inverse)
        }
    };
    Envelope {
        base_pressure,
        core_gravity,
        temperature,
        thickness,
    }
}

/// Whether a runaway's steam hides its surface: a runaway whose steam
/// weighs at least [`STEAM_WORLD_MIN_BAR`] over a surface at or past
/// water's critical temperature has a supercritical base, with no
/// distinction left between the fluid and the gas (Pierrehumbert 2023
/// § 1). The temperature condition holds by construction past the
/// pressure (a steam column is never colder than its saturation
/// temperature) and stays as a guard.
pub fn hides_surface(runaway: bool, steam: Bar, surface: Kelvin) -> bool {
    runaway && steam >= STEAM_WORLD_MIN_BAR && surface >= CRITICAL_POINT_TEMPERATURE
}

#[cfg(test)]
#[path = "steam_tests.rs"]
mod tests;
