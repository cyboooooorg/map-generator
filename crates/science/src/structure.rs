//! Hydrostatic structure of a layered planet: radius, moment of inertia and
//! the profiles of density and pressure, from the mass of each layer and
//! the equation of state of its material.
//!
//! The integration runs in the mass coordinate `t = (m/M)^(1/3)` from the
//! centre out, so the centre is regular and the layer boundaries are exact
//! (layer masses are given, never found). The state is `(r, P, I)`:
//!
//! - `dr/dt = 3 M t^2 / (4 pi r^2 rho)`,
//! - `dP/dt = -3 G M^2 t^5 / (4 pi r^4)`,
//! - `dI/dt = 2 M t^2 r^2`, so `I = (2/3) int r^2 dm = (8 pi / 3) int rho r^4 dr`,
//!
//! with `rho = rho(P)` from the material's phase at each stage of a
//! classical fourth-order Runge-Kutta step. A phase transition of a material
//! (a pressure bound of its [`Phase`](crate::eos::Phase)s) ends a step
//! exactly: the step length at which the pressure meets the bound is found
//! by Brent's method, the step is taken to it, and the rest of the step runs
//! in the new phase, so no step straddles a density jump and the error stays
//! fourth order. The central pressure is found by shooting (Brent's method
//! on its logarithm) until the pressure at the surface is the requested one.
//! The method is the study's prototype
//! (`docs/superpowers/studies/2026-11-21-planet-interior/prototype/solve.py`)
//! with the transitions located.

use crate::constants::GRAVITATIONAL_CONSTANT;
use crate::eos::{DensityLaw, EosError, MaterialEos};
use crate::units::{Kilogram, KilogramPerCubicMetre, Metre, Pascal};
use std::cell::Cell;
use std::f64::consts::PI;

/// Calibration: fourth-order Runge-Kutta steps per layer, equal in the mass
/// coordinate (a phase transition splits the step it falls in, so a phase
/// shell gets the steps of the layer that fall inside it). Fitted on the
/// accuracy goal of the plan brief, 1e-4 relative on R and C/MR^2 for Earth
/// and a 20 Earth-mass planet: over 5, 10, 20, 40 and 80 steps against a
/// 1280-step reference, 20 is the smallest count within it (20 Earth masses:
/// R 5e-6, C/MR^2 1.2e-5; Earth: 7e-8, 2.4e-7). With 10 steps the doubling
/// check passes (C/MR^2 9.2e-5) but the error against the reference is
/// 1.03e-4. The order is four (errors fall 13 to 16 times per doubling);
/// the prototype (`prototype-report.md`, convergence) expected about 40.
/// The test `the_steps_converge` checks it.
pub const STEPS_PER_LAYER: usize = 20;

/// Definition: relative tolerance on the central pressure of the shooting
/// (plan brief, Task 3: 1e-10).
const CENTRAL_PRESSURE_TOLERANCE: f64 = 1e-10;

/// Definition: tolerance on the step length that reaches a phase
/// transition, relative to the step.
const TRANSITION_TOLERANCE: f64 = 1e-12;

/// Definition: largest relative miss of the surface pressure, against the
/// central pressure, accepted from the converged shooting; a genuine root
/// misses by about [`CENTRAL_PRESSURE_TOLERANCE`], so a larger miss means the
/// shooting closed on the edge of a material's range, not on a solution.
const SURFACE_MISS: f64 = 1e-6;

/// Definition: iteration cap of Brent's method; it halves the bracket at
/// worst every other step, so 200 iterations exceed `f64` resolution.
const MAX_ITERATIONS: usize = 200;

/// Definition: cap on the doublings (or halvings) of the central pressure
/// while bracketing it, a factor 2^64 either way from the first guess.
const MAX_EXPANSIONS: usize = 64;

/// One layer of the planet, from the centre out.
#[derive(Clone, Copy, Debug)]
pub struct LayerSpec<'a> {
    /// Mass of the layer; a zero-mass layer is skipped.
    pub mass: Kilogram,
    /// Its material.
    pub material: &'a MaterialEos,
}

/// The state at one point of the integration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// Distance from the centre.
    pub radius: Metre,
    /// Mass inside the radius.
    pub enclosed_mass: Kilogram,
    /// Density; on a boundary, that of the shell inside it.
    pub density: KilogramPerCubicMetre,
    /// Pressure.
    pub pressure: Pascal,
}

/// A shell of one phase of one layer's material. A phase change inside a
/// layer is a shell boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shell {
    /// Outer radius of the shell.
    pub outer_radius: Metre,
    /// Mass of the shell.
    pub mass: Kilogram,
    /// Index of the layer in the slice given to [`integrate`].
    pub layer: usize,
    /// Index of the phase in the layer's material.
    pub phase: usize,
}

/// A planet in hydrostatic equilibrium.
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    /// Surface radius.
    pub radius: Metre,
    /// Shells from the centre out.
    pub shells: Vec<Shell>,
    /// The state at the centre and at the end of every step, outwards.
    pub samples: Vec<Sample>,
    /// Normalised moment of inertia `C / (M R^2)`.
    pub moment_of_inertia: f64,
}

/// Why a structure could not be integrated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StructureError {
    /// No layer has mass.
    NoMass,
    /// A mass or the surface pressure is negative or not finite, or the
    /// result is not finite.
    NotFinite,
    /// No central pressure gives the surface pressure.
    NoConvergence,
    /// An equation of state failed at the solution (a pressure outside the
    /// range of a material).
    Eos(EosError),
}

impl std::fmt::Display for StructureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMass => write!(f, "structure-no-mass"),
            Self::NotFinite => write!(f, "structure-not-finite"),
            Self::NoConvergence => write!(f, "structure-no-convergence"),
            Self::Eos(error) => write!(f, "structure-eos({error})"),
        }
    }
}

impl std::error::Error for StructureError {}

impl From<EosError> for StructureError {
    fn from(error: EosError) -> Self {
        Self::Eos(error)
    }
}

/// Integrates the hydrostatic structure of `layers` (centre out) with the
/// pressure `surface_pressure` at the surface.
///
/// # Errors
///
/// [`StructureError::NoMass`] when no layer has mass;
/// [`StructureError::NotFinite`] for a negative or non-finite mass or
/// surface pressure; [`StructureError::NoConvergence`] when the central
/// pressure cannot be bracketed or found; [`StructureError::Eos`] when the
/// solution needs a pressure outside a material's range.
pub fn integrate(
    layers: &[LayerSpec<'_>],
    surface_pressure: Pascal,
) -> Result<Structure, StructureError> {
    integrate_with_steps(layers, surface_pressure, STEPS_PER_LAYER)
}

/// [`integrate`] with `steps` Runge-Kutta steps per layer.
fn integrate_with_steps(
    layers: &[LayerSpec<'_>],
    surface_pressure: Pascal,
    steps: usize,
) -> Result<Structure, StructureError> {
    let ps = surface_pressure.0;
    if !ps.is_finite() || ps < 0.0 {
        return Err(StructureError::NotFinite);
    }
    if layers
        .iter()
        .any(|layer| !layer.mass.0.is_finite() || layer.mass.0 < 0.0)
    {
        return Err(StructureError::NotFinite);
    }
    let mass: f64 = layers.iter().map(|layer| layer.mass.0).sum();
    if !mass.is_finite() {
        return Err(StructureError::NotFinite);
    }
    if mass <= 0.0 {
        return Err(StructureError::NoMass);
    }
    let mut cumulative = 0.0;
    let mut model_layers = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        if layer.mass.0 == 0.0 {
            continue;
        }
        cumulative += layer.mass.0;
        model_layers.push(Layer {
            index,
            material: layer.material,
            t_end: (cumulative / mass).cbrt(),
        });
    }
    if let Some(last) = model_layers.last_mut() {
        last.t_end = 1.0;
    }
    let model = Model {
        mass,
        layers: model_layers,
        steps,
    };
    let pc = model.central_pressure(layers, ps)?;
    let mut trace = Trace::default();
    let surface = model.shoot(pc, Some(&mut trace))?;
    if (surface.p - ps).abs() > SURFACE_MISS * pc {
        return Err(StructureError::NoConvergence);
    }
    let structure = Structure {
        radius: Metre(surface.r),
        shells: trace.shells,
        samples: trace.samples,
        moment_of_inertia: surface.i / (mass * surface.r * surface.r),
    };
    let finite = structure.radius.0.is_finite()
        && structure.moment_of_inertia.is_finite()
        && structure
            .samples
            .iter()
            .all(|s| s.radius.0.is_finite() && s.density.0.is_finite() && s.pressure.0.is_finite());
    if finite {
        Ok(structure)
    } else {
        Err(StructureError::NotFinite)
    }
}

/// A non-empty layer with its outer mass coordinate.
struct Layer<'a> {
    index: usize,
    material: &'a MaterialEos,
    t_end: f64,
}

/// The integration state at mass coordinate `t`.
#[derive(Clone, Copy, Debug)]
struct State {
    t: f64,
    r: f64,
    p: f64,
    i: f64,
}

/// Samples and shells recorded by the final shot.
#[derive(Default)]
struct Trace {
    samples: Vec<Sample>,
    shells: Vec<Shell>,
}

struct Model<'a> {
    mass: f64,
    layers: Vec<Layer<'a>>,
    steps: usize,
}

impl Model<'_> {
    /// Derivatives `(dr/dt, dP/dt, dI/dt)` at `(t, r, P)`; at the centre
    /// (`r = 0`), their limits for `r = t (3M / (4 pi rho))^(1/3)`.
    ///
    /// Below zero pressure the density is the one at zero pressure: a stage
    /// of the last step can predict a negative pressure near the surface
    /// (1 bar is small against the drop over a step), and a shot from too
    /// low a central pressure runs below zero before the surface, which the
    /// shooting needs as a finite mismatch. The prototype clamps the same
    /// way.
    fn derivatives(&self, law: &DensityLaw, t: f64, r: f64, p: f64) -> Result<[f64; 3], EosError> {
        let rho = law.density(Pascal(p.max(0.0)))?.0;
        let m = self.mass;
        if r == 0.0 {
            return Ok([(3.0 * m / (4.0 * PI * rho)).cbrt(), 0.0, 0.0]);
        }
        let t2 = t * t;
        let r2 = r * r;
        Ok([
            3.0 * m * t2 / (4.0 * PI * r2 * rho),
            -3.0 * GRAVITATIONAL_CONSTANT * m * m * t2 * t2 * t / (4.0 * PI * r2 * r2),
            2.0 * m * t2 * r2,
        ])
    }

    /// One classical Runge-Kutta step of length `h` in the phase `law`.
    fn step(&self, law: &DensityLaw, s: &State, h: f64) -> Result<State, EosError> {
        let half = 0.5 * h;
        let a = self.derivatives(law, s.t, s.r, s.p)?;
        let b = self.derivatives(law, s.t + half, s.r + half * a[0], s.p + half * a[1])?;
        let c = self.derivatives(law, s.t + half, s.r + half * b[0], s.p + half * b[1])?;
        let d = self.derivatives(law, s.t + h, s.r + h * c[0], s.p + h * c[1])?;
        let sum = |k: usize| a[k] + 2.0 * (b[k] + c[k]) + d[k];
        Ok(State {
            t: s.t + h,
            r: s.r + h / 6.0 * sum(0),
            p: s.p + h / 6.0 * sum(1),
            i: s.i + h / 6.0 * sum(2),
        })
    }

    /// Integrates from the centre at pressure `pc` to the surface; records
    /// samples and shells into `trace` when given.
    fn shoot(&self, pc: f64, mut trace: Option<&mut Trace>) -> Result<State, EosError> {
        let mut s = State {
            t: 0.0,
            r: 0.0,
            p: pc,
            i: 0.0,
        };
        for (number, layer) in self.layers.iter().enumerate() {
            let phases = layer.material.phases();
            // Outwards the pressure falls: on a bound, the phase below it.
            let mut phase = phases
                .iter()
                .position(|phase| s.p <= phase.up_to.0)
                .unwrap_or(phases.len() - 1);
            if number == 0
                && let Some(trace) = trace.as_deref_mut()
            {
                trace.samples.push(Sample {
                    radius: Metre(0.0),
                    enclosed_mass: Kilogram(0.0),
                    density: phases[phase].density_law.density(Pascal(pc))?,
                    pressure: Pascal(pc),
                });
            }
            let mut shell_start = s.t;
            let t_start = s.t;
            let width = (layer.t_end - t_start) / self.steps as f64;
            for k in 1..=self.steps {
                let target = if k == self.steps {
                    layer.t_end
                } else {
                    t_start + width * k as f64
                };
                loop {
                    let law = &phases[phase].density_law;
                    let h = target - s.t;
                    let mut end = self.step(law, &s, h)?;
                    end.t = target;
                    let bound = if phase > 0 {
                        phases[phase - 1].up_to.0
                    } else {
                        f64::NEG_INFINITY
                    };
                    if end.p >= bound {
                        s = end;
                        if let Some(trace) = trace.as_deref_mut() {
                            trace.samples.push(self.sample(law, &s)?);
                        }
                        break;
                    }
                    let to_bound = self.step_to_bound(law, &s, h, bound)?;
                    s = to_bound;
                    if let Some(trace) = trace.as_deref_mut() {
                        trace.samples.push(self.sample(law, &s)?);
                        trace.shells.push(self.shell(layer, phase, shell_start, &s));
                    }
                    shell_start = s.t;
                    phase -= 1;
                }
            }
            if let Some(trace) = trace.as_deref_mut() {
                trace.shells.push(self.shell(layer, phase, shell_start, &s));
            }
        }
        Ok(s)
    }

    /// The state where the pressure meets `bound` inside the step of length
    /// `h` from `s` (the pressure is above the bound at `s` and below it at
    /// the end of the step).
    fn step_to_bound(
        &self,
        law: &DensityLaw,
        s: &State,
        h: f64,
        bound: f64,
    ) -> Result<State, EosError> {
        let above = s.p - bound;
        let below = self.step(law, s, h)?.p - bound;
        let root = brent(
            |length| Ok::<f64, EosError>(self.step(law, s, length)?.p - bound),
            0.0,
            h,
            above,
            below,
            TRANSITION_TOLERANCE * h,
        )?
        .unwrap_or(h);
        let mut end = self.step(law, s, root)?;
        end.p = bound;
        Ok(end)
    }

    /// The recorded state at `s`. The pressure is floored at 0 as in
    /// [`Model::derivatives`]: at a zero surface pressure the shooting
    /// converges to within its tolerance on either side of 0, and a
    /// pressure a hair below 0 is the surface, not a material out of range.
    fn sample(&self, law: &DensityLaw, s: &State) -> Result<Sample, EosError> {
        let p = s.p.max(0.0);
        Ok(Sample {
            radius: Metre(s.r),
            enclosed_mass: Kilogram(self.mass * s.t * s.t * s.t),
            density: law.density(Pascal(p))?,
            pressure: Pascal(p),
        })
    }

    fn shell(&self, layer: &Layer<'_>, phase: usize, t_start: f64, s: &State) -> Shell {
        Shell {
            outer_radius: Metre(s.r),
            mass: Kilogram(self.mass * (s.t.powi(3) - t_start.powi(3))),
            layer: layer.index,
            phase,
        }
    }

    /// Mismatch of the surface pressure for the central pressure `exp(x)`:
    /// `(P(M) - ps) / pc`, or `+1` when a material leaves its range at a
    /// positive pressure (too high a central pressure), noted in
    /// `out_of_range`.
    fn mismatch(
        &self,
        x: f64,
        ps: f64,
        out_of_range: &Cell<Option<EosError>>,
    ) -> Result<f64, EosError> {
        let pc = x.exp();
        match self.shoot(pc, None) {
            Ok(s) => Ok((s.p - ps) / pc),
            Err(error @ EosError::OutOfRange { pressure }) if pressure.0 > 0.0 => {
                out_of_range.set(Some(error));
                Ok(1.0)
            }
            Err(error) => Err(error),
        }
    }

    /// The central pressure that gives `ps` at the surface: bracketed by
    /// doubling or halving from a uniform sphere at the surface densities,
    /// then found by Brent's method on its logarithm.
    fn central_pressure(&self, layers: &[LayerSpec<'_>], ps: f64) -> Result<f64, StructureError> {
        let mut volume = 0.0;
        for layer in layers.iter().filter(|layer| layer.mass.0 > 0.0) {
            volume += layer.mass.0 / layer.material.density(Pascal(ps))?.0;
        }
        let radius = (3.0 * volume / (4.0 * PI)).cbrt();
        let guess =
            ps + 3.0 * GRAVITATIONAL_CONSTANT * self.mass * self.mass / (8.0 * PI * radius.powi(4));
        let out_of_range = Cell::new(None);
        let mut f = |x: f64| self.mismatch(x, ps, &out_of_range);
        let (mut a, mut fa) = (guess.ln(), f(guess.ln())?);
        if fa == 0.0 {
            return Ok(guess);
        }
        let direction = if fa < 0.0 {
            2.0_f64.ln()
        } else {
            -(2.0_f64.ln())
        };
        let mut bracket = None;
        for _ in 0..MAX_EXPANSIONS {
            let b = a + direction;
            let fb = f(b)?;
            if fb == 0.0 {
                return Ok(b.exp());
            }
            if fb.signum() != fa.signum() {
                bracket = Some((a, b, fa, fb));
                break;
            }
            (a, fa) = (b, fb);
        }
        let (a, b, fa, fb) = bracket.ok_or(StructureError::NoConvergence)?;
        let root = brent(&mut f, a, b, fa, fb, CENTRAL_PRESSURE_TOLERANCE)?
            .ok_or(StructureError::NoConvergence)?;
        let pc = root.exp();
        // A root on the edge of a material's range is not a solution.
        if let Some(error) = out_of_range.get()
            && f(root)? == 1.0
        {
            return Err(StructureError::Eos(error));
        }
        Ok(pc)
    }
}

/// Root of `f` in `[a, b]`, where `fa = f(a)` and `fb = f(b)` have opposite
/// signs, to the absolute tolerance `tol` (Brent 1973, as in Press et al.,
/// Numerical Recipes, `zbrent`). `None` when [`MAX_ITERATIONS`] is reached.
fn brent<E>(
    mut f: impl FnMut(f64) -> Result<f64, E>,
    mut a: f64,
    mut b: f64,
    mut fa: f64,
    mut fb: f64,
    tol: f64,
) -> Result<Option<f64>, E> {
    let (mut c, mut fc) = (b, fb);
    let (mut d, mut e) = (b - a, b - a);
    for _ in 0..MAX_ITERATIONS {
        if (fb > 0.0 && fc > 0.0) || (fb < 0.0 && fc < 0.0) {
            (c, fc) = (a, fa);
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            (a, fa) = (b, fb);
            (b, fb) = (c, fc);
            (c, fc) = (a, fa);
        }
        let tol1 = 2.0 * f64::EPSILON * b.abs() + 0.5 * tol;
        let xm = 0.5 * (c - b);
        if xm.abs() <= tol1 || fb == 0.0 {
            return Ok(Some(b));
        }
        if e.abs() >= tol1 && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (mut p, mut q) = if a == c {
                (2.0 * xm * s, 1.0 - s)
            } else {
                let qa = fa / fc;
                let r = fb / fc;
                (
                    s * (2.0 * xm * qa * (qa - r) - (b - a) * (r - 1.0)),
                    (qa - 1.0) * (r - 1.0) * (s - 1.0),
                )
            };
            if p > 0.0 {
                q = -q;
            }
            p = p.abs();
            let interpolation_limit = 3.0 * xm * q - (tol1 * q).abs();
            if 2.0 * p < interpolation_limit.min((e * q).abs()) {
                e = d;
                d = p / q;
            } else {
                d = xm;
                e = d;
            }
        } else {
            d = xm;
            e = d;
        }
        (a, fa) = (b, fb);
        b += if d.abs() > tol1 { d } else { tol1.copysign(xm) };
        fb = f(b)?;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{EARTH_MASS, EARTH_MEAN_RADIUS};
    use crate::eos::{
        DensityLaw, Eos, MaterialEos, Phase, core_alloy, mantle, water, water_on_melting_curve,
    };
    use crate::units::{Kelvin, Kilogram, KilogramPerCubicMetre, Metre, Pascal};

    /// Definition: one bar, the surface pressure of the prototype
    /// (`prototype/solve.py`, `PS`).
    const SURFACE: Pascal = Pascal(1e5);

    /// Definition: acceptance tolerance of the interior model on radius and
    /// C/MR^2 for the bodies of the Solar System (issue #534, plan brief).
    const BODY_TOLERANCE: f64 = 0.02;
    /// Definition: Earth's tolerance on R, C/MR^2 and the core radius (plan
    /// brief, Task 3).
    const EARTH_TOLERANCE: f64 = 0.01;
    /// Definition: Earth's tolerance on the central pressure (plan brief,
    /// Task 3).
    const EARTH_CENTRAL_PRESSURE_TOLERANCE: f64 = 0.03;
    /// Definition: relative change allowed when the steps double (plan
    /// brief, Task 3).
    const STEP_TOLERANCE: f64 = 1e-4;
    /// Definition: tolerance of the uniform sphere against its closed form
    /// (plan brief, Task 3).
    const UNIFORM_TOLERANCE: f64 = 1e-6;
    /// Definition: C/MR^2 of a uniform sphere, 2/5.
    const UNIFORM_MOMENT: f64 = 0.4;
    /// Definition: relative rounding allowed on the total mass, summed from
    /// `t^3` at the surface (`t = 1`).
    const MASS_ROUNDING: f64 = 1e-12;
    /// Definition: relative miss allowed on the surface pressure: the
    /// shooting stops at 1e-10 relative on the central pressure, about
    /// 40 Pa for Earth against 1e5 Pa.
    const SURFACE_PRESSURE_MISS: f64 = 1e-3;

    /// Source: Earth's core mass fraction 0.325 (PREM, Rodriguez-Mozos and
    /// Moya 2022, arXiv:2203.01065, Table 1; Zeng et al. 2016, Table 1;
    /// study `equations-of-state.md`, section 4).
    const EARTH_CMF: f64 = 0.325;
    /// Source: Earth C/MR^2 0.3307 (Williams 1994 as quoted in Margot et al.
    /// 2018, arXiv:1806.02024, section 5.2; study section 4).
    const EARTH_MOMENT: f64 = 0.3307;
    /// Source: PREM central pressure 363.852 GPa (Dziewonski and Anderson
    /// 1981 as tabulated by Rodriguez-Mozos and Moya 2022, Table 1; study
    /// section 4).
    const EARTH_CENTRAL_PRESSURE: Pascal = Pascal(363.852e9);
    /// Source: PREM core-mantle boundary radius 3480 km (same table).
    const EARTH_CMB_RADIUS: Metre = Metre(3480e3);

    /// Source: Venus mass 4.8673e24 kg (NASA NSSDC Venus fact sheet, as
    /// read in the study's `prototype-report.md`).
    const VENUS_MASS: Kilogram = Kilogram(4.8673e24);
    /// Source: Venus mean radius 6051.8 km (same fact sheet).
    const VENUS_RADIUS: Metre = Metre(6051.8e3);
    /// Source: Venus core mass fraction 0.30 (Skinner, Pudritz and Cloutier
    /// 2026, arXiv:2604.15304, section 3.4, citing Shah et al. 2022; read in
    /// `prototype-report.md`).
    const VENUS_CMF: f64 = 0.30;
    /// Source: Venus C/MR^2 0.337 +/- 0.024 (Margot et al. 2021, Nature
    /// Astronomy 5, 676, Table 2; study section 4).
    const VENUS_MOMENT: f64 = 0.337;

    /// Source: Mars mass 0.64169e24 kg (NASA NSSDC Mars fact sheet, as read
    /// in `prototype-report.md`).
    const MARS_MASS: Kilogram = Kilogram(0.64169e24);
    /// Source: Mars mean radius 3389.5 km (same fact sheet).
    const MARS_RADIUS: Metre = Metre(3389.5e3);
    /// Source: Mars core mass fraction 0.21 (Khan et al. 2023, as quoted by
    /// Skinner et al. 2026, section 3.2; `prototype-report.md`).
    const MARS_CMF: f64 = 0.21;
    /// Source: Mars C/MR^2 0.3638 +/- 0.0001 (Konopliv et al. 2016, quoted
    /// by Dehant et al. 2020, arXiv:1910.03899; study section 4).
    const MARS_MOMENT: f64 = 0.3638;

    /// Source: Mercury mass 0.33010e24 kg (NASA NSSDC Mercury fact sheet,
    /// as read in `prototype-report.md`).
    const MERCURY_MASS: Kilogram = Kilogram(0.33010e24);
    /// Source: Mercury mean radius 2439.7 km (same fact sheet).
    const MERCURY_RADIUS: Metre = Metre(2439.7e3);
    /// Source: Mercury core mass fraction 0.7136 (Skinner et al. 2026, from
    /// the 1975 km core radius of Genova et al. 2019; `prototype-report.md`).
    const MERCURY_CMF: f64 = 0.7136;
    /// Source: Mercury C/MR^2 0.346 +/- 0.009 (Margot et al. 2018,
    /// arXiv:1806.02024, eq. 19; study section 4).
    const MERCURY_MOMENT: f64 = 0.346;

    /// Source: Moon mass 0.07346e24 kg (NASA NSSDC Moon fact sheet, as read
    /// in `prototype-report.md`).
    const MOON_MASS: Kilogram = Kilogram(0.07346e24);
    /// Source: Moon mean radius 1737.4 km (same fact sheet).
    const MOON_RADIUS: Metre = Metre(1737.4e3);
    /// Source: Moon core mass fraction 0.0168 (Skinner et al. 2026, citing
    /// Viswanathan et al. 2019; `prototype-report.md`).
    const MOON_CMF: f64 = 0.0168;
    /// Source: Moon C/MR^2 0.392728 +/- 0.000012 (Williams et al. 2014, JGR
    /// Planets 119, 1546, abstract; study section 4).
    const MOON_MOMENT: f64 = 0.392_728;

    /// Source: Ganymede mass 1481.9e20 kg (NASA NSSDC Jovian satellite fact
    /// sheet, as read in `prototype-report.md`).
    const GANYMEDE_MASS: Kilogram = Kilogram(1481.9e20);
    /// Source: Ganymede radius 2634.1 km, inside the 2632 to 2634 km of
    /// Rodriguez-Mozos and Moya 2022, Table 5, and Gao and Stevenson 2013
    /// (study `equations-of-state.md`, sections 2.7.6 and 5.1).
    const GANYMEDE_RADIUS: Metre = Metre(2634.1e3);
    /// Source: Ganymede core, mantle and ice mass fractions 0.085, 0.46 and
    /// 0.455 (Rodriguez-Mozos and Moya 2022, Table 5, model column;
    /// `prototype-report.md`).
    const GANYMEDE_SPLIT: [f64; 3] = [0.085, 0.46, 0.455];
    /// Source: Ganymede C/MR^2 0.3115 +/- 0.003 (Rodriguez-Mozos and Moya
    /// 2022, Table 5, citing Schubert et al. 2004; study section 4).
    const GANYMEDE_MOMENT: f64 = 0.3115;

    /// Source: Titan mass 1345.5e20 kg (NASA NSSDC Saturnian satellite fact
    /// sheet, as read in `prototype-report.md`).
    const TITAN_MASS: Kilogram = Kilogram(1345.5e20);
    /// Source: Titan radius 2574.7 km (study section 2.7.6 target; the NASA
    /// sheet gives 2575 km).
    const TITAN_RADIUS: Metre = Metre(2574.7e3);
    /// Calibration: assumed, not sourced. Titan with no iron core, 54.5 %
    /// rock and 45.5 % ice, the prototype's assumption
    /// (`prototype-report.md`, Ganymede's ice fraction); no published Titan
    /// split was found. The Titan radius check is indicative only.
    const TITAN_SPLIT: [f64; 3] = [0.0, 0.545, 0.455];

    /// Source: 5 Earth-mass Earth-like planet, 9860 km from Zeng et al. 2016
    /// `R = (1.07 - 0.21 cmf) M^(1/3.7)` at cmf 0.325 (`prototype-report.md`).
    const SUPER_EARTH_RADIUS: Metre = Metre(9860e3);
    /// Definition: 5 Earth masses, the super-Earths of the plan brief (a
    /// test choice).
    const SUPER_EARTH_MASSES: f64 = 5.0;
    /// Definition: 20 Earth masses, the largest rocky planet of the plan
    /// brief (a test choice).
    const LARGE_SUPER_EARTH_MASSES: f64 = 20.0;
    /// Source: 5 Earth masses with 50 % water, 1.911 Earth radii at 300 K
    /// (Zeng et al. 2019, PNAS 116, 9723; `prototype-report.md`).
    const WATER_WORLD_RADII: f64 = 1.911;
    /// Calibration: assumed, not sourced. The water world's split, 0.1625 /
    /// 0.3375 / 0.5: an Earth-like rock part (core 0.325 of it) under 50 %
    /// water, the prototype's assumption (`prototype-report.md`); Zeng et
    /// al. 2019 give the water share, not the split of the rest.
    const WATER_WORLD_SPLIT: [f64; 3] = [0.1625, 0.3375, 0.5];
    /// Source: 300 K, the isotherm of the Zeng et al. 2019 curve.
    const WATER_WORLD_TEMPERATURE: Kelvin = Kelvin(300.0);

    /// Definition: masses, Earth masses, of the zero-pressure sweep (test
    /// choices across the engine's rocky range, 0.1 to 10).
    const ZERO_PRESSURE_MASSES: [f64; 6] = [0.1, 0.3, 1.0, 2.0, 5.0, 10.0];

    /// Definition: density of the test-only uniform material.
    const UNIFORM_DENSITY: f64 = 5000.0;
    /// Definition: a bulk modulus so large that the density is constant to
    /// `f64` precision at any pressure of the test.
    const RIGID_MODULUS: f64 = 1e40;

    fn relative(value: f64, reference: f64) -> f64 {
        (value / reference - 1.0).abs()
    }

    fn rocky(mass: Kilogram, cmf: f64) -> Structure {
        let (core, mantle) = (core_alloy(), mantle());
        integrate(
            &[
                LayerSpec {
                    mass: mass * cmf,
                    material: &core,
                },
                LayerSpec {
                    mass: mass * (1.0 - cmf),
                    material: &mantle,
                },
            ],
            SURFACE,
        )
        .expect("rocky planet integrates")
    }

    fn three_layers(
        mass: Kilogram,
        split: [f64; 3],
        water: &MaterialEos,
        steps: usize,
    ) -> Result<Structure, StructureError> {
        let (core, mantle) = (core_alloy(), mantle());
        integrate_with_steps(
            &[
                LayerSpec {
                    mass: mass * split[0],
                    material: &core,
                },
                LayerSpec {
                    mass: mass * split[1],
                    material: &mantle,
                },
                LayerSpec {
                    mass: mass * split[2],
                    material: water,
                },
            ],
            SURFACE,
            steps,
        )
    }

    fn check_body(name: &str, s: &Structure, radius: Metre, moment: Option<f64>) {
        let r_error = relative(s.radius.0, radius.0);
        assert!(
            r_error < BODY_TOLERANCE,
            "{name}: R {} m, error {r_error}",
            s.radius.0
        );
        if let Some(moment) = moment {
            let c_error = relative(s.moment_of_inertia, moment);
            assert!(
                c_error < BODY_TOLERANCE,
                "{name}: C/MR^2 {}, error {c_error}",
                s.moment_of_inertia
            );
        }
    }

    fn cmb_radius(s: &Structure) -> Metre {
        s.shells
            .iter()
            .filter(|shell| shell.layer == 0)
            .map(|shell| shell.outer_radius)
            .next_back()
            .expect("a core shell")
    }

    #[test]
    fn a_uniform_sphere_has_two_fifths() {
        let rigid = MaterialEos::new(vec![Phase {
            up_to: Pascal(f64::INFINITY),
            density_law: DensityLaw::Single(Eos::BirchMurnaghan2 {
                rho0: KilogramPerCubicMetre(UNIFORM_DENSITY),
                k0: Pascal(RIGID_MODULUS),
            }),
        }]);
        let s = integrate(
            &[LayerSpec {
                mass: EARTH_MASS,
                material: &rigid,
            }],
            SURFACE,
        )
        .unwrap();
        let closed_form =
            (3.0 * EARTH_MASS.0 / (4.0 * std::f64::consts::PI * UNIFORM_DENSITY)).cbrt();
        assert!(
            relative(s.moment_of_inertia, UNIFORM_MOMENT) < UNIFORM_TOLERANCE,
            "{}",
            s.moment_of_inertia
        );
        assert!(
            relative(s.radius.0, closed_form) < UNIFORM_TOLERANCE,
            "{}",
            s.radius.0
        );
    }

    #[test]
    fn earth_has_its_radius_moment_and_core() {
        let s = rocky(EARTH_MASS, EARTH_CMF);
        let central = s.samples[0].pressure;
        assert!(
            relative(s.radius.0, EARTH_MEAN_RADIUS.0) < EARTH_TOLERANCE,
            "R {}",
            s.radius.0
        );
        assert!(
            relative(s.moment_of_inertia, EARTH_MOMENT) < EARTH_TOLERANCE,
            "C/MR^2 {}",
            s.moment_of_inertia
        );
        assert!(
            relative(central.0, EARTH_CENTRAL_PRESSURE.0) < EARTH_CENTRAL_PRESSURE_TOLERANCE,
            "Pc {}",
            central.0
        );
        let cmb = cmb_radius(&s);
        assert!(
            relative(cmb.0, EARTH_CMB_RADIUS.0) < EARTH_TOLERANCE,
            "CMB {}",
            cmb.0
        );
    }

    #[test]
    fn the_terrestrial_planets_and_the_moon_within_two_percent() {
        for (name, mass, radius, cmf, moment) in [
            ("Venus", VENUS_MASS, VENUS_RADIUS, VENUS_CMF, VENUS_MOMENT),
            ("Mars", MARS_MASS, MARS_RADIUS, MARS_CMF, MARS_MOMENT),
            (
                "Mercury",
                MERCURY_MASS,
                MERCURY_RADIUS,
                MERCURY_CMF,
                MERCURY_MOMENT,
            ),
            ("Moon", MOON_MASS, MOON_RADIUS, MOON_CMF, MOON_MOMENT),
        ] {
            check_body(name, &rocky(mass, cmf), radius, Some(moment));
        }
    }

    #[test]
    fn icy_bodies_within_two_percent() {
        // Both are colder than 273.31 K at the surface: their ice follows the
        // melting curve (Zeng and Sasselov 2013).
        let ice = water_on_melting_curve();
        let ganymede = three_layers(GANYMEDE_MASS, GANYMEDE_SPLIT, &ice, STEPS_PER_LAYER).unwrap();
        check_body(
            "Ganymede",
            &ganymede,
            GANYMEDE_RADIUS,
            Some(GANYMEDE_MOMENT),
        );
        // Titan's radius only: its C/MR^2 (0.3414, Iess et al. 2010) needs a
        // partly differentiated interior (rock mixed into the ice), out of
        // scope of this fully differentiated model.
        let titan = three_layers(TITAN_MASS, TITAN_SPLIT, &ice, STEPS_PER_LAYER).unwrap();
        check_body("Titan", &titan, TITAN_RADIUS, None);
    }

    #[test]
    fn super_earths_follow_the_published_curves() {
        let earth_like = rocky(EARTH_MASS * SUPER_EARTH_MASSES, EARTH_CMF);
        check_body("5 Earth masses", &earth_like, SUPER_EARTH_RADIUS, None);

        let water = water(WATER_WORLD_TEMPERATURE).unwrap();
        let water_world = three_layers(
            EARTH_MASS * SUPER_EARTH_MASSES,
            WATER_WORLD_SPLIT,
            &water,
            STEPS_PER_LAYER,
        )
        .unwrap();
        check_body(
            "5 Earth masses, 50 % water",
            &water_world,
            EARTH_MEAN_RADIUS * WATER_WORLD_RADII,
            None,
        );
    }

    #[test]
    fn a_twenty_earth_mass_planet_converges_unvalidated() {
        // Smoke test only: no published curve is checked at 20 Earth masses
        // (the cold Birch-Murnaghan laws are past their validated range,
        // issue #616); the integration must converge to finite values.
        let large = rocky(EARTH_MASS * LARGE_SUPER_EARTH_MASSES, EARTH_CMF);
        assert!(large.radius.0.is_finite() && large.moment_of_inertia.is_finite());
        assert!(large.samples.iter().all(|s| s.pressure.0.is_finite()
            && s.density.0.is_finite()
            && s.radius.0.is_finite()));
    }

    #[test]
    fn empty_layers_are_skipped() {
        let (core, mantle, ice) = (core_alloy(), mantle(), water_on_melting_curve());
        let layers = |core_mass, mantle_mass| {
            [
                LayerSpec {
                    mass: core_mass,
                    material: &core,
                },
                LayerSpec {
                    mass: mantle_mass,
                    material: &mantle,
                },
                LayerSpec {
                    mass: Kilogram(0.0),
                    material: &ice,
                },
            ]
        };
        // A Moon mass of rock stays below the 23.83 GPa of the lower mantle.
        let s = integrate(&layers(Kilogram(0.0), MOON_MASS), SURFACE).unwrap();
        assert_eq!(s.shells.len(), 1, "{:?}", s.shells);
        assert_eq!((s.shells[0].layer, s.shells[0].phase), (1, 0));
        assert!(relative(s.shells[0].mass.0, MOON_MASS.0) < UNIFORM_TOLERANCE);
        assert_eq!(
            integrate(&layers(Kilogram(0.0), Kilogram(0.0)), SURFACE),
            Err(StructureError::NoMass)
        );
        assert_eq!(integrate(&[], SURFACE), Err(StructureError::NoMass));
    }

    #[test]
    fn rocky_planets_integrate_at_zero_surface_pressure() {
        // The engine integrates at 0 Pa until an atmosphere sets the surface
        // pressure: the converged surface pressure lands a hair on either
        // side of 0, and the surface must still read a density.
        let (core, mantle) = (core_alloy(), mantle());
        for masses in ZERO_PRESSURE_MASSES {
            let mass = EARTH_MASS * masses;
            let layers = [
                LayerSpec {
                    mass: mass * EARTH_CMF,
                    material: &core,
                },
                LayerSpec {
                    mass: mass * (1.0 - EARTH_CMF),
                    material: &mantle,
                },
            ];
            let s = integrate(&layers, Pascal(0.0));
            assert!(s.is_ok(), "{masses} Earth masses: {s:?}");
            let surface = s.unwrap().samples.last().copied().unwrap();
            assert!(surface.pressure.0 >= 0.0, "{masses}: {surface:?}");
        }
    }

    #[test]
    fn the_steps_converge() {
        let (core, mantle) = (core_alloy(), mantle());
        for masses in [1.0, LARGE_SUPER_EARTH_MASSES] {
            let mass = EARTH_MASS * masses;
            let layers = [
                LayerSpec {
                    mass: mass * EARTH_CMF,
                    material: &core,
                },
                LayerSpec {
                    mass: mass * (1.0 - EARTH_CMF),
                    material: &mantle,
                },
            ];
            let coarse = integrate_with_steps(&layers, SURFACE, STEPS_PER_LAYER).unwrap();
            let fine = integrate_with_steps(&layers, SURFACE, 2 * STEPS_PER_LAYER).unwrap();
            let (r, c) = (
                relative(coarse.radius.0, fine.radius.0),
                relative(coarse.moment_of_inertia, fine.moment_of_inertia),
            );
            assert!(
                r < STEP_TOLERANCE && c < STEP_TOLERANCE,
                "{masses} Earth masses: R {r}, C {c}"
            );
        }
    }

    #[test]
    fn samples_are_ordered_and_finite() {
        let s = rocky(EARTH_MASS, EARTH_CMF);
        for pair in s.samples.windows(2) {
            assert!(pair[1].radius > pair[0].radius, "{pair:?}");
            assert!(pair[1].enclosed_mass > pair[0].enclosed_mass, "{pair:?}");
            assert!(pair[1].pressure < pair[0].pressure, "{pair:?}");
        }
        assert!(s.samples.iter().all(|x| x.radius.0.is_finite()
            && x.enclosed_mass.0.is_finite()
            && x.density.0.is_finite()
            && x.pressure.0.is_finite()));
        // A sample on a boundary belongs to the shell inside it.
        let shell_of = |r: Metre| s.shells.iter().position(|shell| r <= shell.outer_radius);
        for pair in s.samples.windows(2) {
            if shell_of(pair[0].radius) == shell_of(pair[1].radius) {
                assert!(pair[1].density <= pair[0].density, "{pair:?}");
            }
        }
        assert_eq!(s.samples.last().unwrap().radius, s.radius);
        assert!(relative(s.samples.last().unwrap().enclosed_mass.0, EARTH_MASS.0) < MASS_ROUNDING);
        assert!(relative(s.samples.last().unwrap().pressure.0, SURFACE.0) < SURFACE_PRESSURE_MISS);
    }
}
