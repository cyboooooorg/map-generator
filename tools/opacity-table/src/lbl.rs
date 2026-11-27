//! Line-by-line absorption cross-sections over one spectral bin.
//!
//! Each line is a Voigt profile cut at [`Cutoff::distance_cm`] from its
//! centre, less the pedestal ([`Pedestal`]) that MT_CKD's continua assume
//! LBLRTM removes inside the cutoff: the line contribution beyond 25 cm⁻¹
//! and the pedestal under it belong to the continuum (Mlawer et al.
//! 2012, *Phil. Trans. R. Soc. A* 370, 2520, §2). For water the pedestal
//! is the profile's value at the cutoff (the "plinth"); for CO₂, LBLRTM's
//! fourth function (`oprop.f90`, the `fcnt_fn` of its CO₂ branch, read on
//! github.com/AER-RC/LBLRTM at the commit MT_CKD 4.3 pins) subtracts
//! `(2 − d²/B²)` times the Lorentz value at the cutoff `B`, which its CO₂
//! continuum assumes since MT_CKD 2.0 ("Fundamental change in the lblrtm
//! fourth function with consequent changes in continuum", Nov 2007).
//! For every other molecule (methane and ozone here, which MT_CKD gives no
//! continuum) LBLRTM subtracts the flat Lorentz value at the cutoff
//! (`CONVF4`, "all molecules other than co2": `f4fn = sil/(α² + d²) −
//! f4bnd`, `f4bnd = sil/(α² + B²)`), as for water.
//!
//! The bin is sampled on a uniform grid fine enough to resolve its
//! narrowest line ([`sample_step`]). Summing every line on every sample is
//! too slow at low pressure, where the grid is fine and the cutoff wide,
//! so the wings are summed on coarser grids and interpolated: grid `j` has
//! the step `h₀ 4ʲ`, a line adds its profile to grid `j` only where it is
//! at least `D h_j` from the line's centre (`D` = [`SMOOTH_STEPS`]), and
//! nearer the centre a finer grid takes over and adds the profile minus
//! its interpolation from the grid above, so that each sample receives the
//! exact profile near the line and the interpolated wing far from it. A
//! line's wing `1/d²` interpolated linearly over steps `h ≤ d / D` errs
//! by at most `0.75 (h/d)²`, 0.3 % (checked against the direct sum in the
//! tests).

use crate::voigt::profile;

/// Lines nearer than this many steps of a grid are summed on the grid
/// below it (see the module documentation).
pub const SMOOTH_STEPS: f64 = 16.0;

/// Ratio of the steps of two successive grids.
const GRID_RATIO: usize = 4;

/// One line at the temperature and pressure of the spectrum.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    /// Centre, cm⁻¹ (shifted by the pressure).
    pub centre: f64,
    /// Intensity, cm⁻¹/(molecule cm⁻²).
    pub strength: f64,
    /// Doppler 1/e half width, cm⁻¹.
    pub doppler_e: f64,
    /// Lorentz half width at half maximum, cm⁻¹.
    pub lorentz: f64,
}

impl Shape {
    /// Half width at half maximum of the Voigt profile, cm⁻¹ (Olivero &
    /// Longbothum 1977, *JQSRT* 17, 233, to 0.02 %).
    pub fn half_width(&self) -> f64 {
        let doppler = self.doppler_e * std::f64::consts::LN_2.sqrt();
        0.5346 * self.lorentz + (0.2166 * self.lorentz * self.lorentz + doppler * doppler).sqrt()
    }
}

/// What is subtracted from a line inside its cutoff (see the module
/// documentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pedestal {
    /// Nothing (the synthetic bands of the tests).
    #[cfg_attr(not(test), allow(dead_code))]
    None,
    /// The profile's value at the cutoff (water, MT_CKD's plinth; methane
    /// and ozone, LBLRTM's `CONVF4` for every molecule but CO₂).
    Flat,
    /// `(2 − d²/B²)` times the Lorentz value at the cutoff `B` (CO₂,
    /// LBLRTM's fourth function).
    Quadratic,
}

/// Where the lines are cut.
#[derive(Clone, Copy, Debug)]
pub struct Cutoff {
    /// Distance from the centre beyond which a line adds nothing, cm⁻¹.
    pub distance_cm: f64,
    /// What is subtracted inside it.
    pub pedestal: Pedestal,
}

/// The cross-section of `line` at `distance` cm⁻¹ from its centre, cm².
fn contribution(line: &Shape, distance: f64, cutoff: Cutoff) -> f64 {
    let b = cutoff.distance_cm;
    if distance >= b {
        return 0.0;
    }
    let mut f = profile(distance, line.doppler_e, line.lorentz);
    match cutoff.pedestal {
        Pedestal::None => {}
        Pedestal::Flat => f -= profile(b, line.doppler_e, line.lorentz),
        Pedestal::Quadratic => {
            let a = line.lorentz;
            let at_cut = a / (std::f64::consts::PI * (a * a + b * b));
            f -= (2.0 - distance * distance / (b * b)) * at_cut;
        }
    }
    line.strength * f.max(0.0)
}

/// Step of the samples of the bin from `lo` to `hi` cm⁻¹: half the
/// narrowest half width of the lines centred in it (or within 1 cm⁻¹),
/// at most 1/400 of the bin, and at least the bin over `max_samples`.
pub fn sample_step(lines: &[Shape], lo: f64, hi: f64, max_samples: usize) -> f64 {
    let narrowest = lines
        .iter()
        .filter(|l| l.centre >= lo - 1.0 && l.centre <= hi + 1.0)
        .map(Shape::half_width)
        .fold(f64::INFINITY, f64::min);
    let width = hi - lo;
    let step = (0.5 * narrowest).min(width / 400.0);
    let samples = (width / step).ceil().clamp(1.0, max_samples as f64);
    width / samples
}

/// Absorption cross-section, cm²/molecule, of `lines` at the samples
/// `lo + (n + ½) h` of the bin, `h` the step of [`sample_step`]; lines
/// farther than the cutoff from the bin add nothing.
pub fn bin_spectrum(
    lines: &[Shape],
    lo: f64,
    hi: f64,
    cutoff: Cutoff,
    max_samples: usize,
) -> Vec<f64> {
    let h0 = sample_step(lines, lo, hi, max_samples);
    let samples = ((hi - lo) / h0).round() as usize;
    let cut = cutoff.distance_cm;
    // Grids 0..=top: grid j has the step h0 4^j; the top grid is the
    // coarsest whose next would reach the cutoff in SMOOTH_STEPS steps.
    let mut top = 0;
    while SMOOTH_STEPS * h0 * (GRID_RATIO.pow(top as u32 + 1) as f64) < cut {
        top += 1;
    }
    let grids: Vec<Grid> = (0..=top)
        .map(|j| Grid::new(j, h0, lo, hi, samples))
        .collect();
    let mut values: Vec<Vec<f64>> = grids.iter().map(|g| vec![0.0; g.count]).collect();
    // The line's profile at the nodes of the grid above, kept for the
    // grid below's correction: (first node, values).
    let mut above: (usize, Vec<f64>) = (0, Vec::new());
    let mut here: Vec<f64> = Vec::new();
    for line in lines {
        let c = line.centre;
        if c < lo - cut || c > hi + cut {
            continue;
        }
        for j in (0..=top).rev() {
            let grid = &grids[j];
            // A line reaches grid j up to SMOOTH_STEPS steps of grid j+1.
            let reach = if j == top {
                cut
            } else {
                SMOOTH_STEPS * grids[j + 1].step
            };
            let (first, last) = grid.within(c - reach, c + reach);
            here.clear();
            for (m, value) in values[j].iter_mut().enumerate().take(last).skip(first) {
                let x = grid.node(m);
                let f = contribution(line, (x - c).abs(), cutoff);
                here.push(f);
                let correction = if j == top {
                    0.0
                } else {
                    grids[j + 1].interpolate(x, |n| {
                        n.checked_sub(above.0)
                            .and_then(|i| above.1.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                };
                *value += f - correction;
            }
            above.0 = first;
            std::mem::swap(&mut above.1, &mut here);
        }
    }
    // Interpolate each grid onto the one below and add.
    for j in (1..=top).rev() {
        let (lower, upper) = values.split_at_mut(j);
        let upper = &upper[0];
        for (m, value) in lower[j - 1].iter_mut().enumerate() {
            let x = grids[j - 1].node(m);
            *value += grids[j].interpolate(x, |n| upper.get(n).copied().unwrap_or(0.0));
        }
    }
    values.swap_remove(0)
}

/// One grid of the layered sum (see the module documentation).
struct Grid {
    /// Position of node 0, cm⁻¹.
    origin: f64,
    /// Step, cm⁻¹.
    step: f64,
    /// Number of nodes.
    count: usize,
}

impl Grid {
    /// Grid `j`: grid 0 holds the samples `lo + (n + ½) h0`; the coarser
    /// ones start one step below `lo` and end at least one step above
    /// `hi`, so that they bracket every node of the grid below.
    fn new(j: usize, h0: f64, lo: f64, hi: f64, samples: usize) -> Self {
        if j == 0 {
            return Self {
                origin: lo + 0.5 * h0,
                step: h0,
                count: samples,
            };
        }
        let step = h0 * GRID_RATIO.pow(j as u32) as f64;
        Self {
            origin: lo - step,
            step,
            count: ((hi - lo) / step).ceil() as usize + 3,
        }
    }

    fn node(&self, m: usize) -> f64 {
        self.origin + m as f64 * self.step
    }

    /// Nodes strictly between `a` and `b`, as a half-open index range.
    fn within(&self, a: f64, b: f64) -> (usize, usize) {
        let first = ((a - self.origin) / self.step).floor() + 1.0;
        let last = ((b - self.origin) / self.step).ceil();
        let clamp = |v: f64| v.clamp(0.0, self.count as f64) as usize;
        (clamp(first), clamp(last))
    }

    /// The linear interpolation at `x` of the values `at(m)` at the nodes.
    fn interpolate(&self, x: f64, at: impl Fn(usize) -> f64) -> f64 {
        let t = (x - self.origin) / self.step;
        let m = t.floor().max(0.0) as usize;
        let f = t - m as f64;
        at(m) * (1.0 - f) + at(m + 1) * f
    }
}

/// [`bin_spectrum`] summed directly, line by line on every sample, with
/// `samples` samples (for the tests).
#[cfg(test)]
pub fn direct_spectrum(
    lines: &[Shape],
    lo: f64,
    hi: f64,
    cutoff: Cutoff,
    samples: usize,
) -> Vec<f64> {
    let h = (hi - lo) / samples as f64;
    (0..samples)
        .map(|n| {
            let x = lo + (n as f64 + 0.5) * h;
            lines
                .iter()
                .map(|l| contribution(l, (x - l.centre).abs(), cutoff))
                .sum()
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small deterministic generator (64-bit LCG, Knuth's MMIX
    /// constants) for synthetic line sets.
    pub(crate) struct Lcg(pub u64);

    impl Lcg {
        pub(crate) fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// `count` lines spread over `lo - 30` to `hi + 30` cm⁻¹, strengths
    /// log-uniform over six decades below 1e-18 cm/molecule (a CO₂ or
    /// water band), with the given Lorentz and Doppler widths.
    pub(crate) fn synthetic(
        count: usize,
        lo: f64,
        hi: f64,
        lorentz: f64,
        doppler_e: f64,
    ) -> Vec<Shape> {
        let mut rng = Lcg(42);
        (0..count)
            .map(|_| Shape {
                centre: lo - 30.0 + (hi - lo + 60.0) * rng.next(),
                strength: 1e-18 * 10f64.powf(-6.0 * rng.next()),
                doppler_e,
                lorentz: lorentz * (0.6 + 0.8 * rng.next()),
            })
            .collect()
    }

    const WATER: Cutoff = Cutoff {
        distance_cm: 25.0,
        pedestal: Pedestal::Flat,
    };

    const CO2: Cutoff = Cutoff {
        distance_cm: 25.0,
        pedestal: Pedestal::Quadratic,
    };

    #[test]
    fn the_layered_sum_matches_the_direct_sum() {
        // From 1 bar to 1e-4 bar: Lorentz-dominated to Doppler-dominated.
        for lorentz in [0.07, 1e-3, 7e-6] {
            let lines = synthetic(150, 600.0, 620.0, lorentz, 6e-4);
            let fast = bin_spectrum(&lines, 600.0, 620.0, WATER, 4_000_000);
            assert!(fast.len() >= 400, "{} samples", fast.len());
            let slow = direct_spectrum(&lines, 600.0, 620.0, WATER, fast.len());
            let mean_slow = slow.iter().sum::<f64>() / slow.len() as f64;
            for (n, (f, s)) in fast.iter().zip(&slow).enumerate() {
                assert!(
                    (f - s).abs() <= 5e-3 * s + 1e-6 * mean_slow,
                    "Lorentz {lorentz}: sample {n} {f:e} vs {s:e}"
                );
            }
        }
    }

    #[test]
    fn the_plinth_takes_the_value_at_the_cutoff() {
        let line = Shape {
            centre: 0.0,
            strength: 1.0,
            doppler_e: 1e-3,
            lorentz: 0.1,
        };
        let at_cut = profile(25.0, 1e-3, 0.1);
        let near = contribution(&line, 1.0, WATER);
        assert!((near - (profile(1.0, 1e-3, 0.1) - at_cut)).abs() < 1e-15);
        assert_eq!(contribution(&line, 25.0, WATER), 0.0);
        assert_eq!(contribution(&line, 30.0, WATER), 0.0);
    }

    #[test]
    fn the_co2_pedestal_is_lblrtms_fourth_function() {
        // LBLRTM's CO₂ branch: `f4fn = sil/(α² + d²) − (2 − d²/B²) sil/(α² + B²)`
        // with `sil = S α / π`, so that the line meets zero at the cutoff.
        let line = Shape {
            centre: 0.0,
            strength: 1.0,
            doppler_e: 1e-4,
            lorentz: 0.5,
        };
        let lorentz = |d: f64| 0.5 / (std::f64::consts::PI * (0.25 + d * d));
        for d in [5.0, 12.0, 20.0, 24.0] {
            let expected = lorentz(d) - (2.0 - d * d / 625.0) * lorentz(25.0);
            let got = contribution(&line, d, CO2);
            assert!(
                (got / expected - 1.0).abs() < 1e-3,
                "{d}: {got} vs {expected}"
            );
        }
        assert!(contribution(&line, 24.999, CO2) < 1e-6 * lorentz(24.999));
        assert_eq!(contribution(&line, 25.0, CO2), 0.0);
    }
}
