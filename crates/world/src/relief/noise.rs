//! Fractal noise over the sphere: Perlin octaves whose amplitudes fall by
//! the persistence of the Hurst exponent, so the structure function of the
//! field follows `|f(p) - f(q)| ~ distance^H`.

use ::noise::{NoiseFn, Perlin};
use science::relief::spectrum::{LACUNARITY, persistence};
use timeline::Seed;

/// Measured: root mean square of one Perlin octave (the `noise` crate's
/// gradient noise), measured on the noise by the test
/// `the_noise_constants_are_measured_on_the_noise`; a numerical
/// normaliser of the noise library, not a physical value nor an exact
/// definition.
pub(crate) const PERLIN_RMS: f64 = 0.314;

/// Measured: mean of `|n|` for one Perlin octave `n`, measured by the
/// same test; a numerical normaliser, not a physical value.
const PERLIN_ABS_MEAN: f64 = 0.256;

/// Measured: standard deviation of `|n|` for one Perlin octave `n`,
/// measured by the same test; a numerical normaliser, not a physical
/// value.
const PERLIN_ABS_STD: f64 = 0.182;

/// The standard deviation, per unit of its value, of a legacy noise: a sum
/// of `octaves` Perlin octaves of gain `gain` divided by the octaves' total
/// amplitude `Σ gainⁱ` (the legacy's `fbm` normalisation). Its octaves are
/// independent, each of root mean square [`PERLIN_RMS`], so the sum has
/// the deviation `PERLIN_RMS · √(Σ gain²ⁱ) / Σ gainⁱ`. Multiplying a
/// unit-variance fBm by it brings the fBm to the legacy's unit, where the
/// legacy calibrations were fitted.
pub(crate) fn legacy_unit(octaves: u32, gain: f64) -> f64 {
    let weights: Vec<f64> = (0..octaves).map(|i| gain.powi(i as i32)).collect();
    let total: f64 = weights.iter().sum();
    let power: f64 = weights.iter().map(|w| w * w).sum();
    PERLIN_RMS * power.sqrt() / total
}

/// The spectral shape of a noise field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spectrum {
    /// Hurst exponent `H`: the roughness, through the octave gain
    /// `persistence(H)`.
    pub hurst: f64,
    /// Number of octaves summed.
    pub octaves: u32,
    /// Frequency of the first octave on the unit sphere: its wavelength is
    /// `1 / base_frequency` radii (at 1, the body's radius).
    pub base_frequency: f64,
}

/// The octaves of one seed and spectrum, built once: building a Perlin
/// table costs far more than evaluating it, so evaluate many points
/// through one `Octaves`.
pub struct Octaves {
    perlins: Vec<Perlin>,
    gain: f64,
    base_frequency: f64,
}

impl Octaves {
    /// The octaves of `spectrum`, each `Perlin` seeded with the low 32
    /// bits of `seed.derive("octave/<i>")`.
    pub fn new(seed: Seed, spectrum: &Spectrum) -> Octaves {
        Octaves {
            perlins: (0..spectrum.octaves)
                .map(|i| Perlin::new(seed.derive(&format!("octave/{i}")).get() as u32))
                .collect(),
            gain: persistence(spectrum.hurst),
            base_frequency: spectrum.base_frequency,
        }
    }

    /// Sum of `octave(n)` (`n` the Perlin value of the octave), weighted
    /// by `persistence^i` and divided by the root of the summed squared
    /// weights: unit variance when `octave` has unit variance.
    fn sum(&self, p: [f64; 3], octave: impl Fn(f64) -> f64) -> f64 {
        let mut weight = 1.0;
        let mut frequency = self.base_frequency;
        let mut sum = 0.0;
        let mut power = 0.0;
        for perlin in &self.perlins {
            let n = perlin.get([p[0] * frequency, p[1] * frequency, p[2] * frequency]);
            sum += weight * octave(n);
            power += weight * weight;
            weight *= self.gain;
            frequency *= LACUNARITY;
        }
        sum / power.sqrt()
    }

    /// Fractional Brownian motion at `p`: zero mean, unit variance.
    pub fn fbm(&self, p: [f64; 3]) -> f64 {
        self.sum(p, |n| n / PERLIN_RMS)
    }

    /// Ridged noise at `p`: the octaves are `-|n|`, so crests sit where
    /// the Perlin octaves cross zero; zero mean, unit variance.
    pub fn ridged(&self, p: [f64; 3]) -> f64 {
        self.sum(p, |n| (PERLIN_ABS_MEAN - n.abs()) / PERLIN_ABS_STD)
    }
}

/// Fractional Brownian motion at `p`; see [`Octaves::fbm`].
pub fn fbm(seed: Seed, spectrum: &Spectrum, p: [f64; 3]) -> f64 {
    Octaves::new(seed, spectrum).fbm(p)
}

/// Ridged noise at `p`; see [`Octaves::ridged`].
pub fn ridged(seed: Seed, spectrum: &Spectrum, p: [f64; 3]) -> f64 {
    Octaves::new(seed, spectrum).ridged(p)
}

#[cfg(test)]
#[path = "noise_tests.rs"]
mod tests;
