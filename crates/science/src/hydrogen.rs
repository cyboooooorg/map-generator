//! Where a giant's hydrogen turns metallic: a boundary pressure, and the
//! radius at which an index-one polytrope of the planet reaches it.
//!
//! The study behind the choices is
//! `docs/superpowers/studies/2026-11-21-planet-model-2a/giants-and-envelopes.md`
//! (§5.1 the published pressures, §5.2 the polytrope, §5.3 the Jupiter
//! layers it is checked against).

use crate::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{Kilogram, Metre, Pascal};
use std::f64::consts::PI;

/// Calibration: the pressure at which hydrogen is taken to turn metallic,
/// 100 GPa (1 Mbar), the "ionization boundary located around P about
/// 1 Mbar" of Debras & Chabrier 2019 (ApJ 872, arXiv:1901.05697). The
/// published transition spans 50 to 200 GPa: onset at 50 GPa in Wahl et
/// al. 2017 (GRL 44, 4649), critical point near 120 GPa in Morales et al.
/// 2010 (PNAS 107, 12799), reflectivity near 200 GPa in Celliers et al.
/// 2018 (Science 361, 677), and the laboratory results differ by up to
/// 95 GPa (study `giants-and-envelopes.md` §5.1). No single value is
/// measured; the boundary is a pressure only, without a temperature. The
/// page [What are planets made of?](../../../docs/science/planet-interiors.md),
/// section on giants, says what was searched.
pub const METALLIC_PRESSURE: Pascal = Pascal(100e9);

/// Definition: the bisection halves the interval `(0, π)` of `xi` this
/// many times, down to `π / 2^60`, below the precision of an `f64` radius.
const BISECTION_STEPS: u32 = 60;

/// An index-one polytrope (`P = K rho²`) of `mass` and `radius`: Hubbard
/// 2013 (ApJ 768, 43), Sect. III.3, Eqs. 61 and 62, with the constants of
/// the Lane-Emden solution `theta = sin xi / xi` (study §5.2):
///
/// ```text
/// rho(r) = rho_c sin(xi) / xi,  xi = π r / R
/// rho_c  = (π² / 3) rho_mean,   rho_mean = 3 M / (4 π R³)
/// K      = 2 G R² / π,          P(r) = K rho(r)²
/// ```
#[derive(Clone, Copy, Debug)]
struct Polytrope {
    radius: f64,
    /// `K rho_c²`, the central pressure.
    central_pressure: f64,
}

impl Polytrope {
    fn new(mass: Kilogram, radius: Metre) -> Self {
        let r = radius.0;
        let mean_density = mass.0 / (4.0 / 3.0 * PI * r.powi(3));
        let central_density = PI * PI / 3.0 * mean_density;
        let k = 2.0 * GRAVITATIONAL_CONSTANT * r * r / PI;
        Self {
            radius: r,
            central_pressure: k * central_density * central_density,
        }
    }

    /// The pressure at `xi`, decreasing from the centre (0) to the surface
    /// (π).
    fn pressure(&self, xi: f64) -> f64 {
        let theta = if xi == 0.0 { 1.0 } else { xi.sin() / xi };
        self.central_pressure * theta * theta
    }
}

/// The radius at which the index-one polytrope of a planet of `mass` and
/// `radius` reaches `pressure` (study §5.2), found by bisection on `xi` in
/// `(0, π)`: pressure falls monotonically outwards. A pressure at or below
/// zero is reached at the surface.
///
/// The polytrope stands for a Jupiter-like giant only, and roughly: it
/// places 1 Mbar at 0.853 of Jupiter's radius, inside the 0.80 to 0.93 of
/// Debras & Chabrier 2019 (study §5.2, §5.3). A planet with a large core is
/// not an index-one body.
///
/// `None` when the central pressure is below `pressure` (the planet never
/// reaches it), or when an input is not a finite number.
pub fn polytrope_radius_at(mass: Kilogram, radius: Metre, pressure: Pascal) -> Option<Metre> {
    if ![mass.0, radius.0, pressure.0].iter().all(|v| v.is_finite())
        || mass.0 <= 0.0
        || radius.0 <= 0.0
    {
        return None;
    }
    if pressure.0 <= 0.0 {
        return Some(radius);
    }
    let polytrope = Polytrope::new(mass, radius);
    if polytrope.central_pressure < pressure.0 {
        return None;
    }
    let (mut inner, mut outer) = (0.0, PI);
    for _ in 0..BISECTION_STEPS {
        let middle = 0.5 * (inner + outer);
        if polytrope.pressure(middle) > pressure.0 {
            inner = middle;
        } else {
            outer = middle;
        }
    }
    Some(Metre(0.5 * (inner + outer) / PI * polytrope.radius))
}

/// The mass inside `within` of the index-one polytrope of a planet of
/// `mass` and `radius`: the integral of `rho` gives
/// `m(xi) = M (sin xi − xi cos xi) / π` (the same Lane-Emden solution,
/// study §5.2). Clamped to `[0, M]` outside `[0, R]`.
pub fn polytrope_mass_within(mass: Kilogram, radius: Metre, within: Metre) -> Kilogram {
    let xi = (within.0 / radius.0).clamp(0.0, 1.0) * PI;
    Kilogram(mass.0 * (xi.sin() - xi * xi.cos()) / PI)
}

#[cfg(test)]
#[path = "hydrogen_tests.rs"]
mod tests;
