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
/// and ρ_c/ρ̄; fixed so the solution is the same on every platform.
const STEP: f64 = 1e-3;

/// Calibration: temperature exponent ν of the hydrogen burning rate
/// ε ∝ ρ T^ν of the pp chain near 4 × 10⁶ K, the central temperature of the
/// lightest stars (Kippenhahn, Weigert & Weiss 2012, *Stellar Structure and
/// Evolution*, §18.5.1: ν ≈ 6 at 5 × 10⁶ K, falling towards lower T). It
/// sets how fast the fusion share rises from the centre.
const BURNING_EXPONENT: f64 = 4.0;

/// The solution, from the centre to the surface θ = 0.
#[derive(Debug)]
pub(crate) struct Polytrope {
    /// ξ at each step, from 0 to ξ₁.
    xi: Vec<f64>,
    /// θ at each step, from 1 to 0.
    theta: Vec<f64>,
    /// Mass inside ξ over the total, ξ²|θ'| / (ξ₁²|θ'₁|).
    mass: Vec<f64>,
    /// Power inside ξ over the total, ∫ ρ ε r² dr with ε ∝ ρ T^ν.
    fusion: Vec<f64>,
    /// |θ'| at the surface.
    surface_slope: f64,
}

/// A point of the polytrope, at a fraction of its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Point {
    pub theta: f64,
    /// Mass inside, fraction of the total.
    pub mass: f64,
    /// Nuclear power inside, fraction of the total.
    pub fusion: f64,
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
        let xi = (x * self.surface()).clamp(0.0, self.surface());
        let at = bracket(&self.xi, xi).expect("ξ within the solution");
        let value = |v: &[f64]| v[at.lo] + (v[at.hi] - v[at.lo]) * at.weight;
        Point {
            theta: value(&self.theta),
            mass: value(&self.mass),
            fusion: value(&self.fusion),
        }
    }
}

/// The solution, integrated on first use.
pub(crate) fn polytrope() -> &'static Polytrope {
    static SOLUTION: OnceLock<Polytrope> = OnceLock::new();
    SOLUTION.get_or_init(solve)
}

/// θ'' = −θⁿ − 2θ'/ξ as a first-order system in (θ, θ'); θ is taken as 0
/// past the surface, where θⁿ has no real value.
fn slope(xi: f64, [theta, dtheta]: [f64; 2]) -> [f64; 2] {
    [dtheta, -theta.max(0.0).powf(INDEX) - 2.0 * dtheta / xi]
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
    // dL ∝ ρ ε r² dr ∝ ρ² T^ν r² dr ∝ θ^(2n+ν) ξ² dξ, by trapezoids.
    let power = |i: usize| theta[i].powf(2.0 * INDEX + BURNING_EXPONENT) * xi[i] * xi[i];
    let mut fusion = vec![0.0];
    for i in 1..xi.len() {
        let previous = fusion[i - 1];
        fusion.push(previous + (power(i - 1) + power(i)) / 2.0 * (xi[i] - xi[i - 1]));
    }
    let total_power = fusion[fusion.len() - 1];
    let fusion = fusion.into_iter().map(|f| f / total_power).collect();
    Polytrope {
        xi,
        theta,
        mass,
        fusion,
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
        assert_eq!((centre.theta, centre.mass, centre.fusion), (1.0, 0.0, 0.0));
        assert_eq!(
            (surface.theta, surface.mass, surface.fusion),
            (0.0, 1.0, 1.0)
        );
    }
}
