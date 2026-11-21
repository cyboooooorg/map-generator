//! The n = 1.5 polytrope: the structure of a fully convective star below
//! 0.30 M☉, where the YREC runs fail (`STELLAR.md`, `stellar-low-mass.tsv`).
//!
//! The Lane–Emden equation θ'' + 2θ'/ξ + θⁿ = 0 is integrated once by RK4
//! at a fixed step in ξ, so every star gets the same solution, scaled to its
//! mass and radius: r/R = ξ/ξ₁, ρ = ρ_c θⁿ, P = P_c θⁿ⁺¹, T = T_c θ
//! (Chandrasekhar 1939, *An Introduction to the Study of Stellar Structure*,
//! ch. IV).

use super::bracket;
use std::sync::OnceLock;

/// Definition: polytropic index of a fully convective, non-degenerate
/// ideal gas (adiabatic, γ = 5/3).
pub(crate) const INDEX: f64 = 1.5;

/// Definition: the fixed RK4 step in ξ, small enough for six digits of ξ₁
/// and ρ_c/ρ̄. The step and the sequence of operations are fixed, but
/// `sqrt` aside the functions used here and in the fusion share (`powf`,
/// `cbrt`) come from the platform's libm, so values may differ in their
/// last bits from one platform to another.
const STEP: f64 = 1e-3;

/// Source: coefficient of the Gamow exponent of the p + p reaction,
/// τ = 33.80 T₆^(−1/3) with T₆ = T / 10⁶ K (Clayton 1968, *Principles of
/// Stellar Evolution and Nucleosynthesis*, Table 4-2; Kippenhahn, Weigert
/// & Weiss 2012, *Stellar Structure and Evolution*, §18.5.1).
const PP_GAMOW_COEFFICIENT: f64 = 33.80;

/// Temperature exponent ν = d ln ε / d ln T of the pp-chain rate at
/// `temperature` (K): ν = τ/3 − 2/3 for a non-resonant rate
/// (Kippenhahn, Weigert & Weiss 2012, §18.5.1), so
/// ν ≈ 11.3 T₆^(−1/3) − 2/3: about 6 at 5 × 10⁶ K and 4 near 1.5 × 10⁷ K.
/// It falls as T rises, so the coolest cores burn the most concentrated.
pub(crate) fn burning_exponent(temperature: f64) -> f64 {
    let t6 = temperature / 1e6;
    PP_GAMOW_COEFFICIENT / t6.cbrt() / 3.0 - 2.0 / 3.0
}

/// The solution, from the centre to the surface θ = 0.
#[derive(Debug)]
pub(crate) struct Polytrope {
    /// ξ at each step, from 0 to ξ₁.
    xi: Vec<f64>,
    /// θ at each step, from 1 to 0.
    theta: Vec<f64>,
    /// Mass inside ξ over the total, ξ²|θ'| / (ξ₁²|θ'₁|).
    mass: Vec<f64>,
    /// |θ'| at the surface.
    surface_slope: f64,
}

/// A point of the polytrope, at a fraction of its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Point {
    pub theta: f64,
    /// Mass inside, fraction of the total.
    pub mass: f64,
}

/// The nuclear power inside each step of a [`Polytrope`], fraction of the
/// total, for one burning exponent.
#[derive(Debug)]
pub(crate) struct Fusion<'a> {
    polytrope: &'a Polytrope,
    share: Vec<f64>,
}

impl Fusion<'_> {
    /// The share inside `x` = r/R (0 to 1), linear between steps.
    pub fn at(&self, x: f64) -> f64 {
        self.polytrope.interpolate(&self.share, x)
    }
}

impl Polytrope {
    /// The first zero ξ₁ of θ.
    pub fn surface(&self) -> f64 {
        self.xi[self.xi.len() - 1]
    }

    /// |θ'(ξ₁)|.
    pub fn surface_slope(&self) -> f64 {
        self.surface_slope
    }

    /// Central over mean density, ξ₁ / (3|θ'₁|).
    pub fn central_condensation(&self) -> f64 {
        self.surface() / (3.0 * self.surface_slope)
    }

    /// The point at `x` = r/R (0 to 1), linear between steps.
    pub fn at(&self, x: f64) -> Point {
        Point {
            theta: self.interpolate(&self.theta, x),
            mass: self.interpolate(&self.mass, x),
        }
    }

    /// The share of the nuclear power inside each radius for a rate
    /// ε ∝ ρ T^ν, ν = `burning_exponent`: dL ∝ ρ ε r² dr ∝ ρ² T^ν r² dr,
    /// and with ρ ∝ θⁿ, T ∝ θ, dL ∝ θ^(2n + ν) ξ² dξ = θ^(3 + ν) ξ² dξ,
    /// integrated by trapezoids on the solution's steps.
    pub fn fusion(&self, burning_exponent: f64) -> Fusion<'_> {
        let (xi, theta) = (&self.xi, &self.theta);
        let power = |i: usize| theta[i].powf(2.0 * INDEX + burning_exponent) * xi[i] * xi[i];
        let mut share = vec![0.0];
        for i in 1..xi.len() {
            let previous = share[i - 1];
            share.push(previous + (power(i - 1) + power(i)) / 2.0 * (xi[i] - xi[i - 1]));
        }
        let total = share[share.len() - 1];
        Fusion {
            polytrope: self,
            share: share.into_iter().map(|f| f / total).collect(),
        }
    }

    /// `values` (one per step) at `x` = r/R, linear between steps.
    fn interpolate(&self, values: &[f64], x: f64) -> f64 {
        let xi = (x * self.surface()).clamp(0.0, self.surface());
        let at = bracket(&self.xi, xi).expect("ξ within the solution");
        values[at.lo] + (values[at.hi] - values[at.lo]) * at.weight
    }
}

/// The solution, integrated on first use.
pub(crate) fn polytrope() -> &'static Polytrope {
    static SOLUTION: OnceLock<Polytrope> = OnceLock::new();
    SOLUTION.get_or_init(solve)
}

/// θ'' = −θⁿ − 2θ'/ξ as a first-order system in (θ, θ'), θⁿ = θ√θ for
/// n = 1.5; θ is taken as 0 past the surface, where θⁿ has no real value.
fn slope(xi: f64, [theta, dtheta]: [f64; 2]) -> [f64; 2] {
    let theta = theta.max(0.0);
    [dtheta, -theta * theta.sqrt() - 2.0 * dtheta / xi]
}

fn rk4_step(xi: f64, y: [f64; 2], h: f64) -> [f64; 2] {
    let shift = |y: [f64; 2], k: [f64; 2], f: f64| [y[0] + k[0] * f, y[1] + k[1] * f];
    let k1 = slope(xi, y);
    let k2 = slope(xi + h / 2.0, shift(y, k1, h / 2.0));
    let k3 = slope(xi + h / 2.0, shift(y, k2, h / 2.0));
    let k4 = slope(xi + h, shift(y, k3, h));
    std::array::from_fn(|i| y[i] + h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
}

/// Definition: coefficients of the series of θ about the centre,
/// θ = 1 − ξ²/6 + n ξ⁴/120 (Chandrasekhar 1939, ch. IV, eq. 77), which
/// starts the integration one step out, clear of the 2θ'/ξ singularity.
const SERIES: [f64; 2] = [6.0, 120.0];

fn solve() -> Polytrope {
    let [c2, c4] = SERIES;
    let mut xi = vec![0.0, STEP];
    let mut theta = vec![1.0, 1.0 - STEP * STEP / c2 + INDEX * STEP.powi(4) / c4];
    let mut dtheta = vec![0.0, -2.0 * STEP / c2 + 4.0 * INDEX * STEP.powi(3) / c4];
    loop {
        let last = xi.len() - 1;
        let (x, y) = (xi[last], [theta[last], dtheta[last]]);
        let next = rk4_step(x, y, STEP);
        if next[0] > 0.0 {
            xi.push(x + STEP);
            theta.push(next[0]);
            dtheta.push(next[1]);
            continue;
        }
        // The surface lies in this step: linear in ξ between its ends.
        let weight = y[0] / (y[0] - next[0]);
        xi.push(x + STEP * weight);
        theta.push(0.0);
        dtheta.push(y[1] + (next[1] - y[1]) * weight);
        break;
    }
    let (surface, surface_slope) = (xi[xi.len() - 1], -dtheta[dtheta.len() - 1]);
    let total_mass = surface * surface * surface_slope;
    let mass = xi
        .iter()
        .zip(&dtheta)
        .map(|(x, d)| x * x * -d / total_mass)
        .collect();
    Polytrope {
        xi,
        theta,
        mass,
        surface_slope,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source: first zero of the n = 1.5 Lane–Emden function, ξ₁ = 3.65375
    /// (Chandrasekhar 1939, Table 4).
    const XI_1: f64 = 3.65375;
    /// Source: central over mean density of the n = 1.5 polytrope,
    /// ρ_c/ρ̄ = 5.99071 (Chandrasekhar 1939, Table 4).
    const CENTRAL_CONDENSATION: f64 = 5.99071;
    /// Source: −ξ₁²θ'(ξ₁) = 2.71406 for n = 1.5 (Chandrasekhar 1939,
    /// Table 4).
    const SURFACE_MASS: f64 = 2.71406;
    /// Definition: the table's six digits, as a relative tolerance.
    const TABLE_PRECISION: f64 = 1e-5;

    #[test]
    fn the_polytrope_has_its_textbook_shape() {
        let p = polytrope();
        let close = |got: f64, want: f64| (got / want - 1.0).abs() < TABLE_PRECISION;
        assert!(close(p.surface(), XI_1), "ξ₁ {}", p.surface());
        assert!(
            close(p.central_condensation(), CENTRAL_CONDENSATION),
            "ρ_c/ρ̄ {}",
            p.central_condensation()
        );
        let mass = p.surface() * p.surface() * p.surface_slope();
        assert!(close(mass, SURFACE_MASS), "ξ₁²|θ'₁| {mass}");
        let (centre, surface) = (p.at(0.0), p.at(1.0));
        assert_eq!((centre.theta, centre.mass), (1.0, 0.0));
        assert_eq!((surface.theta, surface.mass), (0.0, 1.0));
        let fusion = p.fusion(burning_exponent(CORE_5_MK));
        assert_eq!((fusion.at(0.0), fusion.at(1.0)), (0.0, 1.0));
    }

    /// Source: ν = 11.3 T₆^(−1/3) − 2/3 of the pp chain at 5 × 10⁶ K
    /// (Kippenhahn, Weigert & Weiss 2012, §18.5.1).
    const PP_EXPONENT_AT_5_MK: f64 = 5.94;
    /// Definition: the core temperature of that value, K.
    const CORE_5_MK: f64 = 5e6;
    /// Source: the Sun's central temperature, 1.57 × 10⁷ K (Bahcall,
    /// Serenelli & Basu 2005, ApJ 621, L85).
    const SOLAR_CORE: f64 = 1.57e7;
    /// Definition: tolerance on ν, the rounding of the coefficient 11.3.
    const EXPONENT_TOLERANCE: f64 = 0.1;

    #[test]
    fn the_burning_exponent_follows_the_pp_chain() {
        let nu = burning_exponent(CORE_5_MK);
        assert!(
            (nu - PP_EXPONENT_AT_5_MK).abs() < EXPONENT_TOLERANCE,
            "ν {nu}"
        );
        assert!(burning_exponent(SOLAR_CORE) < nu, "ν falls as T rises");
    }
}
