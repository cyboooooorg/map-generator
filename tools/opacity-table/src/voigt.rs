//! The Voigt line shape: the convolution of a Doppler (Gaussian) and a
//! Lorentz profile.

/// The Voigt function `K(x, y) = Re w(x + iy)`, `w` the Faddeeva
/// function, by Humlíček's four-region rational approximation W4
/// (Humlíček 1982, *JQSRT* 27, 437), accurate to about 1e-4 of the
/// value. `x` is the distance from the line centre and `y` the Lorentz
/// half width, both in units of the Doppler 1/e half width;
/// `∫ K dx = √π`.
pub fn voigt(x: f64, y: f64) -> f64 {
    let t = C::new(y, -x);
    let s = x.abs() + y;
    if s >= 15.0 {
        // Region I.
        (t * 0.564_189_6 / (t * t + 0.5)).re
    } else if s >= 5.5 {
        // Region II.
        let u = t * t;
        (t * (u * 0.564_189_6 + 1.410_474) / (u * (u + 3.0) + 0.75)).re
    } else if y >= 0.195 * x.abs() - 0.176 {
        // Region III.
        let num = ((((t * 0.564_223_6 + 3.778_987) * t + 11.964_82) * t + 20.209_33) * t) + 16.4955;
        let den =
            (((((t + 6.699_398) * t + 21.692_74) * t + 39.271_21) * t + 38.823_63) * t) + 16.4955;
        (num / den).re
    } else {
        // Region IV.
        let u = t * t;
        // `c - u x`, the nested form of the published polynomials.
        let step = |c: f64, x: C| C::new(c, 0.0) - u * x;
        let num = t * step(
            36183.31,
            step(
                3321.9905,
                step(
                    1540.787,
                    step(
                        219.0313,
                        step(35.76683, step(1.320522, C::new(0.56419, 0.0))),
                    ),
                ),
            ),
        );
        let den = step(
            32066.6,
            step(
                24322.84,
                step(
                    9022.228,
                    step(
                        2186.181,
                        step(364.2191, step(61.57037, C::new(1.841439, 0.0) - u)),
                    ),
                ),
            ),
        );
        (u.exp() - num / den).re
    }
}

/// A complex number, for [`voigt`].
#[derive(Clone, Copy, Debug)]
struct C {
    re: f64,
    im: f64,
}

impl C {
    const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    fn exp(self) -> Self {
        let m = self.re.exp();
        Self::new(m * self.im.cos(), m * self.im.sin())
    }
}

impl std::ops::Add<f64> for C {
    type Output = C;
    fn add(self, r: f64) -> C {
        C::new(self.re + r, self.im)
    }
}

impl std::ops::Sub<f64> for C {
    type Output = C;
    fn sub(self, r: f64) -> C {
        C::new(self.re - r, self.im)
    }
}

impl std::ops::Mul<f64> for C {
    type Output = C;
    fn mul(self, r: f64) -> C {
        C::new(self.re * r, self.im * r)
    }
}

impl std::ops::Mul for C {
    type Output = C;
    fn mul(self, r: C) -> C {
        C::new(
            self.re * r.re - self.im * r.im,
            self.re * r.im + self.im * r.re,
        )
    }
}

impl std::ops::Sub for C {
    type Output = C;
    fn sub(self, r: C) -> C {
        C::new(self.re - r.re, self.im - r.im)
    }
}

impl std::ops::Div for C {
    type Output = C;
    fn div(self, r: C) -> C {
        let d = r.re * r.re + r.im * r.im;
        C::new(
            (self.re * r.re + self.im * r.im) / d,
            (self.im * r.re - self.re * r.im) / d,
        )
    }
}

/// A normalised Voigt line profile, cm, at `distance` cm⁻¹ from its
/// centre: `K(x, y) / (α_D √π)` with `α_D` the Doppler 1/e half width
/// and `lorentz` the Lorentz half width at half maximum, both cm⁻¹.
pub fn profile(distance: f64, doppler_e: f64, lorentz: f64) -> f64 {
    voigt(distance / doppler_e, lorentz / doppler_e) / (doppler_e * std::f64::consts::PI.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_centre_matches_the_exact_value() {
        // K(0, y) = exp(y²) erfc(y); erfc(1) = 0.157299207050285.
        let exact = 1f64.exp() * 0.157_299_207_050_285;
        let k = voigt(0.0, 1.0);
        assert!((k / exact - 1.0).abs() < 2e-4, "{k} vs {exact}");
    }

    #[test]
    fn every_region_matches_the_integral() {
        // K(x, y) = (y/π) ∫ exp(−t²) / ((x − t)² + y²) dt, by the trapezoid
        // rule over ±12 with 2e6 steps (Python).
        for (x, y, exact) in [
            (2.5, 0.01, 0.003_230_557_656_591_501),
            (4.0, 0.1, 0.003_921_752_098_962_421),
            (1.5, 0.05, 0.120_568_642_363_957_52),
        ] {
            let k = voigt(x, y);
            assert!(
                (k / exact - 1.0).abs() < 1e-3,
                "K({x}, {y}) = {k} vs {exact}"
            );
        }
    }

    #[test]
    fn small_y_is_doppler_and_large_y_is_lorentz() {
        for x in [0.0, 0.5, 1.0, 2.0, 3.0] {
            let k = voigt(x, 1e-6);
            let gauss = f64::exp(-x * x);
            assert!((k - gauss).abs() < 2e-4, "x {x}: {k} vs {gauss}");
        }
        for x in [0.0, 10.0, 100.0] {
            let y = 50.0;
            let k = voigt(x, y);
            let lorentz = y / (std::f64::consts::PI.sqrt() * (x * x + y * y));
            assert!((k / lorentz - 1.0).abs() < 1e-3, "x {x}: {k} vs {lorentz}");
        }
    }

    #[test]
    fn the_profile_is_normalised() {
        for (doppler, lorentz) in [(1e-3, 1e-5), (1e-3, 1e-3), (1e-3, 0.1)] {
            // Trapezoid over ±2000 widths, plus the Lorentz tails beyond.
            let width = doppler + lorentz;
            let edge = 2000.0 * width;
            let n = 400_000;
            let h = 2.0 * edge / n as f64;
            let mut sum = 0.0;
            for i in 0..=n {
                let d = -edge + i as f64 * h;
                let w = if i == 0 || i == n { 0.5 } else { 1.0 };
                sum += w * profile(d, doppler, lorentz) * h;
            }
            sum += 2.0 * lorentz / (std::f64::consts::PI * edge);
            assert!((sum - 1.0).abs() < 2e-3, "{doppler} {lorentz}: {sum}");
        }
    }
}
