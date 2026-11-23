//! What a planet loses to space under its star's X-ray and
//! extreme-ultraviolet (XUV) light over its life: the energy-limited
//! escape of a hydrogen-helium envelope (and of the hydrogen of a steam
//! atmosphere), and the cosmic shoreline that splits the bodies with and
//! without air.
//!
//! **Energy-limited escape** (Owen & Wu 2017, ApJ 847, 29,
//! doi:10.3847/1538-4357/aa890a, section 2.2, eq. 19, after Lecavelier
//! des Etangs 2007 and Erkaev et al. 2007): the paper's rate `Ṁ = η π R³
//! L_HE / (4 π a² G M)`, integrated over the XUV energy per unit area
//! received, `F = ∫ L_HE / (4 π a²) dt`, gives the mass lost
//! `ΔM = η π R³ F / (G M)` ([`energy_limited_loss`]). It is what opens
//! the radius valley: nothing here is tuned to the valley's position.
//!
//! **Cosmic shoreline** (Zahnle & Catling 2017, ApJ 843, 122,
//! doi:10.3847/1538-4357/aa7846): the bodies with and without air are
//! split by `I_XUV ∝ v_esc⁴`, `I_XUV` the cumulative XUV fluence relative
//! to Earth's. Their line is "drawn by eye" and has no width in the
//! paper, so its position and steepness here are calibrations
//! ([`shoreline_kept`]).
//!
//! Ported from the tag `legacy-final` (`planet/escape.rs` and
//! `planet/atmosphere.rs` l.126-146, 182-185, 288-320), with the sources
//! checked in the study
//! [21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit).

use crate::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{Fraction, JoulePerSquareMetre, Kilogram, Metre, MetrePerSecond};
use crate::xuv::EARTH_XUV_FLUENCE;

/// Source: efficiency `η` of the XUV photons at removing mass, Owen & Wu
/// 2017, ApJ 847, 29, doi:10.3847/1538-4357/aa890a, section 2.2, after
/// eq. 19: "of order 0.1 for low-mass planets ... For simplicity, we adopt
/// a constant eta = 0.1". It falls for deep potential wells (their
/// section 4.5, eq. 31, `η = 0.1 (v_esc / 15 km/s)^-2`), which their own
/// footnote says is "suitable in a narrow range of parameter space": the
/// constant is kept, and a giant loses next to nothing with it already.
pub const EFFICIENCY: f64 = 0.1;

/// Calibration: share of a planet's mass under which its hydrogen-helium
/// envelope is gone, what is left being a thin atmosphere, not a hydrogen
/// shell. One tenth of the smallest envelope of Owen & Wu 2017's sample
/// (`X_min = 0.01`, section 3.3; the planets that keep an envelope keep
/// "roughly ~1 % in mass", where the smaller ones are bare cores). No
/// paper searched gives a threshold under which an envelope counts as
/// gone (study 21, part 2).
pub const MIN_ENVELOPE: f64 = 1.0e-3;

/// Definition: mass of water whose hydrogen is one unit of mass of
/// hydrogen, by stoichiometry: H₂O is 18 for 2 of hydrogen.
pub const WATER_PER_HYDROGEN: f64 = 9.0;

/// Calibration: escape velocity at which the cosmic shoreline crosses
/// Earth's XUV fluence. Zahnle & Catling 2017 (ApJ 843, 122,
/// doi:10.3847/1538-4357/aa7846) draw their line "by eye" on their
/// Figures 1 and 2 and print no equation, constant or crossing velocity;
/// the position was not read off the figures. Fitted on the Solar System
/// bodies: Mars keeps a trace (the paper places it at the line), Titan a
/// bar, Venus and Earth their air (study 21, part 7).
pub const SHORELINE_VELOCITY: MetrePerSecond = MetrePerSecond(6_500.0);

/// Source: power of the escape velocity along the cosmic shoreline,
/// `I_XUV ∝ v_esc⁴`: Zahnle & Catling 2017, abstract ("The dividing line
/// goes at I proportional to v_esc^4") and Figure 2.
pub const SHORELINE_EXPONENT: i32 = 4;

/// Calibration: steepness `k` of the share kept across the shoreline,
/// `1 / (1 + x^k)`. Zahnle & Catling 2017 give the line no width (section
/// 10: "we do not know if the shoreline is broad or narrow"). Fitted on
/// the Solar System bodies: Titan, about 0.1 dex on the side of the air,
/// keeps most; Mars, about 0.4 dex on the other side, keeps under 1 %
/// (study 21, part 7).
pub const SHORELINE_STEEPNESS: f64 = 6.0;

/// Calibration: factor on the XUV fluence a body without a dynamo meets:
/// the stellar wind reaches its upper atmosphere and picks up its ions.
/// No factor is published (Zahnle & Catling 2017, section 6, on the
/// solar wind's erosion of Mars, is the nearest read and gives none);
/// kept small enough that Venus, with no dynamo, keeps its air. Tied to
/// issue #344 (study 21, part 7).
pub const UNMAGNETISED_ESCAPE: f64 = 2.0;

/// Mass a body of `radius` and `mass` can lose to the XUV energy per
/// unit area `fluence` it received: `η π R³ F / (G M)` (Owen & Wu 2017,
/// eq. 19, integrated over the age), with `η` [`EFFICIENCY`]. It is not
/// capped by what the body holds; the caller caps it. Takes finite
/// inputs and a positive mass.
pub fn energy_limited_loss(
    radius: Metre,
    mass: Kilogram,
    fluence: JoulePerSquareMetre,
) -> Kilogram {
    Kilogram(
        EFFICIENCY * std::f64::consts::PI * radius.0.powi(3) * fluence.0
            / (GRAVITATIONAL_CONSTANT * mass.0),
    )
}

/// Escape velocity at the surface of a body of `mass` and `radius`:
/// `v = √(2 G M / R)`.
pub fn escape_velocity(mass: Kilogram, radius: Metre) -> MetrePerSecond {
    MetrePerSecond((2.0 * GRAVITATIONAL_CONSTANT * mass.0 / radius.0).sqrt())
}

/// Share of its air a body of surface `escape_velocity` keeps after the
/// XUV energy per unit area `fluence`, on the cosmic shoreline:
///
/// ```text
/// kept = 1 / (1 + (φ s / (v / v₀)⁴)^k)
/// ```
///
/// with `φ` the fluence over [`EARTH_XUV_FLUENCE`], `v₀`
/// [`SHORELINE_VELOCITY`], `k` [`SHORELINE_STEEPNESS`] and `s` 1 for a
/// `magnetised` body, [`UNMAGNETISED_ESCAPE`] for one without a dynamo.
/// Half on the line, nearly all well below it, nearly nothing well above.
/// Every gas is kept in the same share: the shoreline is a bulk rule, not
/// a species-by-species escape. No fluence keeps all the air, whatever
/// the velocity. Takes finite inputs.
pub fn shoreline_kept(
    escape_velocity: MetrePerSecond,
    fluence: JoulePerSquareMetre,
    magnetised: bool,
) -> Fraction {
    let phi = fluence.0.max(0.0) / EARTH_XUV_FLUENCE.0;
    if phi == 0.0 {
        return Fraction::new(1.0).expect("one is a fraction");
    }
    let shield = if magnetised { 1.0 } else { UNMAGNETISED_ESCAPE };
    let line = (escape_velocity.0 / SHORELINE_VELOCITY.0).powi(SHORELINE_EXPONENT);
    let exposure = phi * shield / line;
    let kept = 1.0 / (1.0 + exposure.powf(SHORELINE_STEEPNESS));
    Fraction::new(kept).expect("a share in 0 to 1 for finite inputs")
}

#[cfg(test)]
#[path = "escape_tests.rs"]
mod tests;
