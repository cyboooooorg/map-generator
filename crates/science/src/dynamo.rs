//! The iron core of a rocky body: how fast it cools, whether it is still
//! liquid, whether it drives a dynamo and how strong a field it gives at
//! the surface.
//!
//! **Cooling time.** A parametrised thermal history in the style of
//! Stevenson, Spohn & Schubert 1983 (*Icarus* 54, 466,
//! doi:10.1016/0019-1035(83)90241-5, not reached) and Driscoll & Barnes
//! 2015 (doi:10.1089/ast.2015.1325), cut down to one cooling time per
//! body. The core's excess heat, `ρ c (4/3)π r_c³ ΔT`, leaves through its
//! surface across the mantle of thickness `d` at `Nu k ΔT / d`, so it
//! decays as `e^(-t/τ)` with `τ = ρ c r_c d / (3 k Nu)`
//! ([`cooling_time`]). The levers are the core's size, the mantle's
//! thickness and the regime's Nusselt number.
//!
//! **Phases** ([`core_phase`]). Under plates the core-mantle flux stays
//! above the adiabat and a thermal dynamo runs until the core freezes
//! (Nimmo & Stevenson 2000, doi:10.1029/1999JE001216). Under a lid the
//! thermal dynamo stops at [`THERMAL_DYNAMO_END`] `τ`; from
//! [`INNER_CORE_START`] `τ` an inner core grows and drives a deep
//! compositional dynamo; from [`CORE_FROZEN`] `τ` the core is solid.
//!
//! **Field** ([`surface_field`]). The heat flux out of the core is
//! `q = ρ c r_c ΔT₀ e^(-t/τ) / (3τ)`; the rms field in the core follows
//! the energy-flux scaling of Christensen, Holzwarth & Reiners 2009
//! (*Nature* 457, 167, doi:10.1038/nature07626, eq. 2),
//! `B² / 2µ₀ = c f_ohm ρ^(1/3) (F q)^(2/3)` with `f_ohm = 1`
//! ([`rms_field`]); a share of it is the equatorial dipole at the core's
//! surface, carried to the planet's surface by `(r_c / R)³`.
//!
//! Ported from the tag `legacy-final` (`planet/core.rs` l.21-205), with
//! the sources checked in the study
//! [21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit),
//! part 4. See `docs/science/planet-atmospheres.md`, "Core and dynamo".

use std::f64::consts::PI;

use crate::units::{Gyr, KilogramPerCubicMetre, Metre, Second, Tesla, WattPerSquareMetre, Year};

/// Source: specific heat of the core, J/(kg K), Christensen, Holzwarth &
/// Reiners 2009, *Nature* 457, 167, doi:10.1038/nature07626, main text
/// before Fig. 1: "cp = 840 in SI units" for Earth's core. The legacy's
/// 800 (Stevenson et al. 1983, not reached) made `τ` 5 % shorter.
pub const CORE_HEAT_CAPACITY: f64 = 840.0;

/// Calibration: thermal conductivity of the mantle, W/(m K), of the
/// order of the mantle's value. Stevenson et al. 1983, the legacy's
/// source, was not reached (paywalled); searched, no value read (study
/// 21, part 4).
pub const MANTLE_CONDUCTIVITY: f64 = 4.0;

/// Calibration: the core's initial excess temperature over the one it
/// tends to, K, set so that Earth's core loses about 10 TW today, the
/// middle of the 5 to 15 TW quoted for Lay, Hernlund & Buffett 2008
/// (doi:10.1038/ngeo.2007.44) and Nimmo 2015 (Treatise on Geophysics 8),
/// both read secondhand (study 21, part 4).
pub const INITIAL_EXCESS: f64 = 2_800.0;

/// Calibration: cooling times after which the thermal dynamo of a body
/// under a lid stops, its core-mantle flux below the adiabat (Nimmo &
/// Stevenson 2000). Fitted on Mars's dynamo ending near 0.6 Gyr, 4.0 Ga
/// ago (Lillis et al. 2013: between 4.0 and 4.1 Ga; Mittelholz et al.
/// 2020 argue for a field near 3.7 Ga; both abstracts only; study 21,
/// part 4). Set with a heat capacity of 800 J/(kg K).
pub const THERMAL_DYNAMO_END: f64 = 0.075;

/// Calibration: cooling times after which an inner core grows (the
/// centre reaches the liquidus). Fitted on Earth's inner core nucleating
/// near 4.0 Gyr, about 0.5 Ga ago (Bono et al. 2019: 565 Ma; other work
/// reaches 1 to 1.5 Ga; secondhand), Mercury's before 4.567 Gyr, Mars's
/// and Venus's after (study 21, part 4). Set with a heat capacity of
/// 800 J/(kg K).
pub const INNER_CORE_START: f64 = 1.0;

/// Calibration: cooling times after which the core is solid throughout.
/// Fitted on the Moon's core freezing near 3.3 Gyr, inside the 1 to
/// 2.5 Ga window of a weak lunar field (Tikoo et al. 2017, secondhand),
/// with Mercury's still liquid at 4.567 Gyr (Anderson et al. 2011; study
/// 21, part 4). Set with a heat capacity of 800 J/(kg K).
pub const CORE_FROZEN: f64 = 2.25;

/// Calibration: Nusselt number of the core's cooling through a mantle
/// under plates, the convective speed-up over conduction across the
/// whole mantle. Fitted on Earth's inner core starting near 4.0 Gyr; no
/// paper sets a Nusselt number for a cooling law of this form (study 21,
/// part 4).
pub const NUSSELT_PLATES: f64 = 50.0;

/// Calibration: Nusselt number under an episodic regime, fitted on Venus
/// having no dynamo at 4.567 Gyr (study 21, part 4).
pub const NUSSELT_EPISODIC: f64 = 10.0;

/// Calibration: Nusselt number under a stagnant lid, fitted on Mars,
/// Mercury and the Moon together (study 21, part 4). A heat pipe cools
/// the lithosphere, not the mantle below, and takes the same value (no
/// target).
pub const NUSSELT_LID: f64 = 5.0;

/// Definition: magnetic constant `µ₀ = 4π × 10⁻⁷` T m/A, its exact value
/// in the SI before the 2019 redefinition, kept by the legacy.
const MU_0: f64 = 4.0e-7 * PI;

/// Source: prefactor `c` of the energy-flux scaling for a dipolar
/// dynamo, Christensen, Holzwarth & Reiners 2009, p. 168, eq. 2: "It
/// provides the constant of proportionality as c = 0.63". A multipolar
/// dynamo has 0.23 (Christensen 2010, Space Sci. Rev. 152, 565,
/// doi:10.1007/s11214-009-9553-2), not used here.
pub const FLUX_SCALING_C: f64 = 0.63;

/// Source: efficiency `F` of the conversion of the core's heat flux into
/// buoyancy flux, 0.52 for Earth's core with a constant total flux
/// (0.27 with a flux falling to zero at the top): Christensen, Holzwarth
/// & Reiners 2009, p. 168, "is obtained as 0.52 or 0.27". The field goes
/// as `F^(1/3)` = 0.804, since `B²` goes as `(F q)^(2/3)` (eq. 2).
pub const DYNAMO_EFFICIENCY: f64 = 0.52;

/// Source: ratio of the dipole at the core's surface to the rms field in
/// the core, 1/7: Christensen, Holzwarth & Reiners 2009, pp. 168-169,
/// the paragraph on the planetary data: "we find a typical ratio B/B_dip
/// of around seven for dynamos with an Earth-like magnetic power
/// spectrum".
pub const DIPOLE_OVER_FIELD: f64 = 1.0 / 7.0;

/// Definition: equatorial value of a dipole over its rms over a sphere,
/// `1/√2` (the paper's 0.26 mT for Earth's core surface is the rms:
/// `0.26 mT × (3480 / 6371)³ / √2` = 30.0 µT).
pub const RMS_TO_EQUATORIAL: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Calibration: what is left of the legacy's dipole share 0.067 once
/// `F^(1/3)`, [`DIPOLE_OVER_FIELD`] and [`RMS_TO_EQUATORIAL`] are taken
/// out, `0.067 / (0.804 × 0.143 × 0.707)` = 0.825, fitted on Earth's
/// surface equatorial dipole of about 30 µT. It holds what the law
/// simplifies: the core's heat flow against the paper's reference flux
/// and the ohmic share `f_ohm ≤ 1` (study 21, part 4).
pub const DIPOLE_REMAINDER: f64 = 0.82;

/// Calibration: share of the rms field (taken with `F = 1`) that is the
/// equatorial dipole at the core's surface for a deep compositional
/// dynamo under a stably stratified top, whose field is mostly
/// multipolar and screened (Christensen 2006, doi:10.1038/nature05342).
/// Fitted on Mercury's centred dipole, 195 ± 10 nT R_M³ (Anderson et al.
/// 2011, *Science* 333, 1859, doi:10.1126/science.1211001, p. 1861),
/// about 0.2 µT at the equator. Not split into factors: Christensen et
/// al. 2009 say their scaling is "either hard to test or not applicable"
/// at Mercury, so `F`, 1/7 and `c` do not carry over; the legacy's law
/// with `c` and `F = 1` is kept under it (study 21, part 4).
pub const DEEP_DIPOLE_SHARE: f64 = 3.0e-4;

/// The state of an iron core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorePhase {
    /// At least partly liquid (an inner core may be growing).
    Liquid {
        /// A dynamo runs in the core.
        dynamo: bool,
    },
    /// Solid throughout: no dynamo.
    Frozen,
}

/// The cooling time of a core of `core_density` and `core_radius` under
/// a mantle of `mantle_thickness`, with the regime's `nusselt` number
/// ([`NUSSELT_PLATES`], [`NUSSELT_EPISODIC`], [`NUSSELT_LID`]):
/// `τ = ρ c r_c d / (3 k Nu)`. Zero without a core or a mantle.
#[must_use]
pub fn cooling_time(
    core_density: KilogramPerCubicMetre,
    core_radius: Metre,
    mantle_thickness: Metre,
    nusselt: f64,
) -> Gyr {
    let seconds = core_density.0 * CORE_HEAT_CAPACITY * core_radius.0 * mantle_thickness.0
        / (3.0 * MANTLE_CONDUCTIVITY * nusselt);
    Gyr::from(Year::from(Second(seconds)))
}

/// The phase of a core of cooling time `tau` at `age`; `plates` stands
/// for a regime of plate tectonics, which keeps the thermal dynamo
/// running until the core freezes. A core without a cooling time (no
/// mantle to cool through) is liquid without a dynamo: the model does
/// not apply (the legacy's choice).
#[must_use]
pub fn core_phase(age: Gyr, tau: Gyr, plates: bool) -> CorePhase {
    if tau.0 <= 0.0 {
        return CorePhase::Liquid { dynamo: false };
    }
    let t = age.0 / tau.0;
    if t >= CORE_FROZEN {
        return CorePhase::Frozen;
    }
    let thermal = plates || t < THERMAL_DYNAMO_END;
    let compositional = t >= INNER_CORE_START;
    CorePhase::Liquid {
        dynamo: thermal || compositional,
    }
}

/// The rms field inside a core of `core_density` losing `core_flux`
/// through its surface, with the buoyancy `efficiency` `F`: Christensen,
/// Holzwarth & Reiners 2009, eq. 2, `B² / 2µ₀ = c f_ohm ρ^(1/3)
/// (F q)^(2/3)`, with [`FLUX_SCALING_C`] and `f_ohm = 1`.
#[must_use]
pub fn rms_field(
    core_density: KilogramPerCubicMetre,
    core_flux: WattPerSquareMetre,
    efficiency: f64,
) -> Tesla {
    Tesla(
        (2.0 * MU_0
            * FLUX_SCALING_C
            * core_density.0.cbrt()
            * (efficiency * core_flux.0).powf(2.0 / 3.0))
        .sqrt(),
    )
}

/// The surface equatorial dipole field of a core of `core_density` and
/// `core_radius`, of cooling time `tau`, in a planet of `planet_radius`
/// at `age`, assuming a dynamo runs (the caller reads [`core_phase`]).
///
/// The heat flux out of the core is `q = ρ c r_c ΔT₀ e^(-t/τ) / (3τ)`.
/// A dynamo driven from the top of the core has the rms field of
/// [`rms_field`] with [`DYNAMO_EFFICIENCY`], of which
/// [`DIPOLE_OVER_FIELD`] × [`RMS_TO_EQUATORIAL`] × [`DIPOLE_REMAINDER`]
/// is the equatorial dipole at the core's surface; a `deep`
/// compositional dynamo, under a stable top, has [`DEEP_DIPOLE_SHARE`]
/// of the field taken with `F = 1` (the legacy's choice). The field falls
/// as `(r_c / R)³` to the surface. Zero without a core or a cooling time.
#[must_use]
pub fn surface_field(
    core_density: KilogramPerCubicMetre,
    core_radius: Metre,
    planet_radius: Metre,
    age: Gyr,
    tau: Gyr,
    deep: bool,
) -> Tesla {
    if core_radius.0 <= 0.0 || tau.0 <= 0.0 {
        return Tesla(0.0);
    }
    let tau_s = Second::from(Year::from(tau)).0;
    let flux = core_density.0
        * CORE_HEAT_CAPACITY
        * core_radius.0
        * INITIAL_EXCESS
        * (-age.0 / tau.0).exp()
        / (3.0 * tau_s);
    let flux = WattPerSquareMetre(flux);
    let dipole = if deep {
        rms_field(core_density, flux, 1.0).0 * DEEP_DIPOLE_SHARE
    } else {
        rms_field(core_density, flux, DYNAMO_EFFICIENCY).0
            * DIPOLE_OVER_FIELD
            * RMS_TO_EQUATORIAL
            * DIPOLE_REMAINDER
    };
    Tesla(dipole * (core_radius.0 / planet_radius.0).powi(3))
}

#[cfg(test)]
#[path = "dynamo_tests.rs"]
mod tests;
