use super::*;
use ::noise::{NoiseFn, Perlin};
use timeline::Seed;

fn spectrum(hurst: f64) -> Spectrum {
    Spectrum {
        hurst,
        octaves: 12,
        base_frequency: 1.0,
    }
}

/// Deterministic quasi-random point and direction (golden-ratio sequences).
fn pair(i: u32, d: f64) -> ([f64; 3], [f64; 3]) {
    let t = f64::from(i);
    let p = [
        (t * 0.618_034).fract() * 2.0 - 1.0,
        (t * 0.754_878).fract() * 2.0 - 1.0,
        (t * 0.569_840).fract() * 2.0 - 1.0,
    ];
    let u = (t * 0.381_966).fract() * 2.0 - 1.0;
    let phi = (t * 0.465_571).fract() * std::f64::consts::TAU;
    let s = (1.0 - u * u).sqrt();
    let q = [
        p[0] + d * s * phi.cos(),
        p[1] + d * s * phi.sin(),
        p[2] + d * u,
    ];
    (p, q)
}

fn structure(seed: Seed, s: &Spectrum, d: f64) -> f64 {
    let n = 10_000;
    let octaves = Octaves::new(seed, s);
    (0..n)
        .map(|i| {
            let (p, q) = pair(i, d);
            (octaves.fbm(p) - octaves.fbm(q)).abs()
        })
        .sum::<f64>()
        / f64::from(n)
}

#[test]
fn fbm_is_deterministic() {
    let s = spectrum(0.8);
    let p = [0.3, -0.5, 0.7];
    let a = Seed::new(1);
    assert_eq!(fbm(a, &s, p), fbm(a, &s, p));
    assert_ne!(fbm(a, &s, p), fbm(Seed::new(2), &s, p));
    assert_eq!(ridged(a, &s, p), ridged(a, &s, p));
}

#[test]
fn fbm_follows_its_hurst_exponent() {
    for hurst in [0.48, 0.8] {
        let s = spectrum(hurst);
        let seed = Seed::new(7);
        let (d1, d2) = (0.004, 0.016);
        let slope = (structure(seed, &s, d2) / structure(seed, &s, d1)).ln() / (d2 / d1).ln();
        assert!((slope - hurst).abs() < 0.05, "H = {hurst}: slope {slope}");
    }
}

#[test]
fn fbm_and_ridged_have_unit_variance() {
    let s = spectrum(0.7);
    for f in [fbm, ridged] {
        let n = 5_000;
        let v: Vec<f64> = (0..n)
            .map(|i| f(Seed::new(3), &s, pair(i, 0.0).0))
            .collect();
        let mean = v.iter().sum::<f64>() / f64::from(n);
        let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / f64::from(n);
        assert!(mean.abs() < 0.1, "{mean}");
        assert!((var - 1.0).abs() < 0.15, "{var}");
    }
}

/// The constants of the module are those of the noise: sampled at 200 000
/// points spread over many lattice cells.
#[test]
fn the_noise_constants_are_measured_on_the_noise() {
    let n = 200_000u32;
    let (mut s2, mut a1) = (0.0, 0.0);
    for seed in 0..4u32 {
        let perlin = Perlin::new(seed);
        for i in 0..n / 4 {
            let t = f64::from(i);
            let v = perlin.get([
                (t * 0.618_034).fract() * 97.0,
                (t * 0.754_878).fract() * 89.0,
                (t * 0.569_840).fract() * 83.0,
            ]);
            s2 += v * v;
            a1 += v.abs();
        }
    }
    let m = f64::from(n);
    let rms = (s2 / m).sqrt();
    let abs_mean = a1 / m;
    let abs_std = (s2 / m - abs_mean * abs_mean).sqrt();
    assert!((rms - PERLIN_RMS).abs() < 0.005, "{rms}");
    assert!((abs_mean - PERLIN_ABS_MEAN).abs() < 0.005, "{abs_mean}");
    assert!((abs_std - PERLIN_ABS_STD).abs() < 0.005, "{abs_std}");
}
