//! A main-sequence star's interior: its convective and radiative layers and
//! its density, pressure, temperature and fusion share from the centre to
//! the photosphere ([`profile`]).
//!
//! How it is read (`STELLAR.md`, Reading the tables):
//!
//! - From 0.30 M☉, the YREC profiles: at each profile mass and tabulated
//!   [Fe/H] around the star, the two snapshots around the star's xc_frac
//!   (read from the tracks as [`main_sequence`](super::main_sequence) reads
//!   them) linear in xc_frac, held at the first or last snapshot outside;
//!   then linear in log mass, then in [Fe/H]. Each step goes sample by
//!   sample at equal r/R: log ρ, log P, log T, m/M and the fusion share
//!   linear. Layer bounds (r and m) are linear between layers of the same
//!   rank; a layer present in one profile only is kept when that profile is
//!   the nearer one ([`blend_layers`]).
//! - Below 0.30 M☉, the n = 1.5 polytrope scaled to the star's mass and
//!   radius: one convective layer, temperature T_c θ with T_c from Baraffe
//!   et al. (2015), fusion share from ε ∝ ρ T^ν with the pp-chain ν at T_c.

use super::polytrope::{INDEX, burning_exponent, polytrope};
use super::tables::{ProfileLayer, ProfileSample, ProfileTrack, Snapshot, profiles};
use super::values::{TrackClock, low_mass_central_temperature, track_xc_frac};
use super::{StellarError, bracket, bracket_clamped, bracket_log, lerp, main_sequence};
use crate::constants::{GRAVITATIONAL_CONSTANT, SOLAR_MASS, SOLAR_RADIUS};
use crate::units::{Dex, Fraction, Gyr, Kelvin, KilogramPerCubicMetre, Metre, Pascal, SolarMass};
use std::f64::consts::PI;

/// A layer of the star, from the outer bound of the one inside it (or the
/// centre) to its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StellarLayer {
    /// Fraction of the star's photospheric radius.
    pub outer_radius: Fraction,
    /// Mass inside the outer radius, fraction of the star's.
    pub outer_mass: Fraction,
    pub convective: bool,
}

/// The state of the star at one radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StellarSample {
    /// Fraction of the star's photospheric radius.
    pub radius: Fraction,
    /// Mass inside the radius, fraction of the star's.
    pub mass: Fraction,
    pub density: KilogramPerCubicMetre,
    pub pressure: Pascal,
    pub temperature: Kelvin,
    /// The transport of the layer that holds the sample.
    pub convective: bool,
    /// Nuclear power released inside the radius over the star's total.
    pub fusion_share: Fraction,
}

/// A star's interior, centre out.
#[derive(Clone, Debug, PartialEq)]
pub struct StellarProfile {
    /// The last one ends at the photosphere (radius and mass 1).
    pub layers: Vec<StellarLayer>,
    /// From the centre (radius 0) to the photosphere (radius 1).
    pub samples: Vec<StellarSample>,
}

/// The interior of a main-sequence star of `mass` and `metallicity` at
/// `age`, between its ZAMS and the end of its [`life`](super::life), with
/// the errors of [`main_sequence`].
pub fn profile(
    mass: SolarMass,
    metallicity: Dex,
    age: Gyr,
) -> Result<StellarProfile, StellarError> {
    let star = main_sequence(mass, metallicity, age)?;
    if mass.0 < POLYTROPE_BELOW {
        let radius = Metre(star.radius.0 * SOLAR_RADIUS.0);
        Ok(polytrope_profile(
            mass,
            radius,
            star.effective_temperature,
            age,
        ))
    } else {
        Ok(yrec(mass.0, metallicity.0, age.0).into_profile())
    }
}

/// Definition: below 0.30 M☉, the lightest YREC profile, the star is the
/// polytrope; from it, the YREC profiles (no YREC run succeeds below it).
const POLYTROPE_BELOW: f64 = 0.30;

/// Definition: g cm⁻³ in kg m⁻³.
const KG_M3_PER_G_CM3: f64 = 1e3;
/// Definition: dyn cm⁻² in Pa.
const PA_PER_DYN_CM2: f64 = 0.1;

/// A profile being interpolated, at the sample radii of the table.
#[derive(Clone, Debug, PartialEq)]
struct Blend {
    layers: Vec<ProfileLayer>,
    samples: Vec<ProfileSample>,
}

impl Blend {
    fn of(snapshot: &Snapshot) -> Self {
        Self {
            layers: snapshot.layers.clone(),
            samples: snapshot.samples.clone(),
        }
    }

    /// `self` at weight 0, `other` at weight 1.
    fn towards(&self, other: &Self, weight: f64) -> Self {
        let samples = self
            .samples
            .iter()
            .zip(&other.samples)
            .map(|(a, b)| ProfileSample {
                m: lerp(a.m, b.m, weight),
                log_rho: lerp(a.log_rho, b.log_rho, weight),
                log_p: lerp(a.log_p, b.log_p, weight),
                log_t: lerp(a.log_t, b.log_t, weight),
                fusion: lerp(a.fusion, b.fusion, weight),
            })
            .collect();
        Self {
            layers: blend_layers(&self.layers, &other.layers, weight),
            samples,
        }
    }

    fn into_profile(self) -> StellarProfile {
        let radii = &profiles().radii;
        let layers: Vec<StellarLayer> = self
            .layers
            .iter()
            .map(|l| StellarLayer {
                outer_radius: fraction(l.r),
                outer_mass: fraction(l.m),
                convective: l.convective,
            })
            .collect();
        let samples = radii
            .iter()
            .zip(&self.samples)
            .map(|(&r, s)| StellarSample {
                radius: fraction(r),
                mass: fraction(s.m),
                density: KilogramPerCubicMetre(10f64.powf(s.log_rho) * KG_M3_PER_G_CM3),
                pressure: Pascal(10f64.powf(s.log_p) * PA_PER_DYN_CM2),
                temperature: Kelvin(10f64.powf(s.log_t)),
                convective: convective_at(&self.layers, r),
                fusion_share: fraction(s.fusion),
            })
            .collect();
        StellarProfile { layers, samples }
    }
}

/// A share computed from shares, kept within 0 to 1 against rounding.
fn fraction(value: f64) -> Fraction {
    Fraction::new(value.clamp(0.0, 1.0)).expect("a finite share")
}

/// The transport of the layer that holds radius `r` (a bound belongs to
/// the layer inside it).
fn convective_at(layers: &[ProfileLayer], r: f64) -> bool {
    let i = layers.partition_point(|l| l.r < r).min(layers.len() - 1);
    layers[i].convective
}

/// Weighted mean of one or two blends (weights summing to 1).
fn mix(parts: Vec<(Blend, f64)>) -> Blend {
    let mut parts = parts.into_iter();
    let (first, _) = parts.next().expect("at least one part");
    match parts.next() {
        Some((second, weight)) => first.towards(&second, weight),
        None => first,
    }
}

/// The YREC profile: snapshots in xc_frac, then log mass, then [Fe/H].
fn yrec(mass: f64, feh: f64, age: f64) -> Blend {
    let table = profiles();
    let by_feh = bracket(&table.metallicities, feh).expect("metallicity checked");
    let at_feh = by_feh.terms().map(|(i, w_feh)| {
        let tabulated = table.metallicities[i];
        let clock = TrackClock::new(mass, tabulated, age);
        let tracks = table.tracks_at(tabulated);
        let masses: Vec<f64> = tracks.iter().map(|t| t.mass).collect();
        let by_mass = bracket_log(&masses, mass).expect("the profiles cover 0.3-8 Msun");
        let at_mass = by_mass.terms().map(|(j, w)| {
            let xc_frac = track_xc_frac(&clock, tracks[j].mass, tabulated);
            (at_xc_frac(&tracks[j], xc_frac), w)
        });
        (mix(at_mass.collect()), w_feh)
    });
    mix(at_feh.collect())
}

/// The snapshots of `track` around `xc_frac`, linear in it; the first or
/// last snapshot alone outside them.
fn at_xc_frac(track: &ProfileTrack, xc_frac: f64) -> Blend {
    let snaps = &track.snapshots;
    let at = bracket_clamped(snaps.len(), |i| snaps[i].xc_frac, xc_frac, |v| v);
    mix(at.terms().map(|(i, w)| (Blend::of(&snaps[i]), w)).collect())
}

/// The layers between `a` (weight 0) and `b` (weight 1).
///
/// The two lists are aligned rank by rank: a list without the other's
/// convective core gets an empty one at the centre, and the shorter list
/// gets empty layers at the photosphere, alternating, until both have the
/// same transports at every rank. Bounds (r and m) are then linear in the
/// weight. A layer that is empty in the nearer list (`a` below weight 0.5)
/// is dropped, and its neighbours, of the same transport, merge; the last
/// layer ends at r = m = 1.
fn blend_layers(a: &[ProfileLayer], b: &[ProfileLayer], weight: f64) -> Vec<ProfileLayer> {
    let mark = |layers: &[ProfileLayer]| -> Vec<(ProfileLayer, bool)> {
        layers.iter().map(|&l| (l, false)).collect()
    };
    let (mut a, mut b) = (mark(a), mark(b));
    if a[0].0.convective != b[0].0.convective {
        let core = ProfileLayer {
            r: 0.0,
            m: 0.0,
            convective: true,
        };
        let lacking = if a[0].0.convective { &mut b } else { &mut a };
        lacking.insert(0, (core, true));
    }
    let ranks = a.len().max(b.len());
    for list in [&mut a, &mut b] {
        while list.len() < ranks {
            let outer = ProfileLayer {
                r: 1.0,
                m: 1.0,
                convective: !list[list.len() - 1].0.convective,
            };
            list.push((outer, true));
        }
    }
    let near_a = weight < 0.5;
    let mut layers: Vec<ProfileLayer> = Vec::new();
    for (&(la, empty_a), &(lb, empty_b)) in a.iter().zip(&b) {
        if (near_a && empty_a) || (!near_a && empty_b) {
            continue;
        }
        let layer = ProfileLayer {
            r: lerp(la.r, lb.r, weight),
            m: lerp(la.m, lb.m, weight),
            convective: la.convective,
        };
        match layers.last_mut() {
            Some(last) if last.convective == layer.convective => *last = layer,
            _ => layers.push(layer),
        }
    }
    let last = layers.len() - 1;
    layers[last].r = 1.0;
    layers[last].m = 1.0;
    layers
}

/// The polytrope scaled to a star of `mass` and `radius`: ρ_c = ρ̄ ξ₁/(3|θ'₁|),
/// P_c = G M² / (4π (n + 1) θ'₁² R⁴).
///
/// Calibration: the temperature is T_c θ with T_c from Baraffe et al.
/// (2015) at the star's mass and `age`, not μ m_u P_c/(k ρ_c): the ideal gas
/// misses the partial degeneracy of these stars and overshoots T_c by a
/// third at 0.1 M☉. T is held at T_eff where T_c θ falls below it, at the
/// photosphere (r = 1, where θ = 0).
fn polytrope_profile(mass: SolarMass, radius: Metre, teff: Kelvin, age: Gyr) -> StellarProfile {
    let solution = polytrope();
    let (m, r) = (mass.0 * SOLAR_MASS.0, radius.0);
    let mean_density = m / (4.0 / 3.0 * PI * r.powi(3));
    let central_density = mean_density * solution.central_condensation();
    let slope = solution.surface_slope();
    let central_pressure =
        GRAVITATIONAL_CONSTANT * m * m / (4.0 * PI * (INDEX + 1.0) * slope * slope * r.powi(4));
    let central_temperature = low_mass_central_temperature(mass.0, age.0);
    let fusion = solution.fusion(burning_exponent(central_temperature));
    let whole = StellarLayer {
        outer_radius: fraction(1.0),
        outer_mass: fraction(1.0),
        convective: true,
    };
    let samples = profiles()
        .radii
        .iter()
        .map(|&x| {
            let point = solution.at(x);
            let theta_n = point.theta * point.theta.sqrt(); // θⁿ, n = 1.5
            StellarSample {
                radius: fraction(x),
                mass: fraction(point.mass),
                density: KilogramPerCubicMetre(central_density * theta_n),
                pressure: Pascal(central_pressure * theta_n * point.theta),
                temperature: Kelvin((central_temperature * point.theta).max(teff.0)),
                convective: true,
                fusion_share: fraction(fusion.at(x)),
            }
        })
        .collect();
    StellarProfile {
        layers: vec![whole],
        samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stellar::{MAX_METALLICITY, MIN_METALLICITY, life};

    /// Source: the Sun's age, 4.57 Gyr (Bahcall, Serenelli & Basu 2005,
    /// ApJ 621, L85).
    const SOLAR_AGE: Gyr = Gyr(4.57);

    /// Source: B23 standard solar model, `struct+nu_SF3_MB22m.dat`
    /// (Herrera & Serenelli 2023, Zenodo 10174172, CC-BY 4.0), as quoted in
    /// `STELLAR.md` (The Sun: YREC against B23): r/R, ρ (g cm⁻³),
    /// P (dyn cm⁻²), T (K).
    const B23: [[f64; 4]; 13] = [
        [0.0, 150.0, 2.297e17, 1.558e7],
        [0.05, 126.3, 1.965e17, 1.482e7],
        [0.10, 86.35, 1.332e17, 1.307e7],
        [0.20, 34.82, 4.308e16, 9.359e6],
        [0.25, 20.83, 2.206e16, 7.922e6],
        [0.30, 12.04, 1.094e16, 6.775e6],
        [0.40, 3.922, 2.701e15, 5.121e6],
        [0.50, 1.353, 7.279e14, 3.993e6],
        [0.60, 0.5105, 2.163e14, 3.138e6],
        [0.70, 0.2118, 6.696e13, 2.315e6],
        [0.80, 0.09293, 1.722e13, 1.356e6],
        [0.90, 0.02639, 2.113e12, 5.918e5],
        [0.95, 0.00781, 2.778e11, 2.673e5],
    ];
    /// Definition: tolerance on the B23 central density (YREC: −2.6 %).
    const CENTRAL_DENSITY_TOLERANCE: f64 = 0.05;
    /// Definition: tolerance on the B23 central temperature (YREC: −1.4 %).
    const CENTRAL_TEMPERATURE_TOLERANCE: f64 = 0.03;
    /// Definition: tolerance on ρ, P and T at 0.05-0.95 R against B23 (YREC:
    /// within 7.1 %).
    const B23_TOLERANCE: f64 = 0.08;
    /// Definition: g cm⁻³ in kg m⁻³.
    const KG_M3_PER_G_CM3: f64 = 1e3;
    /// Definition: dyn cm⁻² in Pa.
    const PA_PER_DYN_CM2: f64 = 0.1;
    /// Definition: two radii of the table are the same within this.
    const SAME_RADIUS: f64 = 1e-9;

    /// Source: base of the Sun's convective envelope, 0.713 R
    /// (Basu & Antia 1997, MNRAS 287, 189); the window allows YREC's 0.726.
    const ENVELOPE_BASE: std::ops::Range<f64> = 0.70..0.74;

    /// Source: B23 L(r)/L = 0.987 at 0.25 R (Herrera & Serenelli 2023,
    /// `STELLAR.md`); 0.99 is passed only near 0.26 R, in B23 as in YREC.
    const SOLAR_CORE_SHARE: f64 = 0.98;
    /// Definition: radius inside which the Sun's power is checked.
    const SOLAR_CORE_RADIUS: f64 = 0.25;

    /// Source: central density of a 0.08 M☉ star at 5 Gyr, 637.8 g cm⁻³
    /// (Baraffe et al. 2015, BHAC15, `STELLAR.md`).
    const LATE_M_DWARF_CENTRAL_DENSITY: f64 = 637.8;
    /// Definition: tolerance of the polytrope against BHAC15's structure.
    const LATE_M_DWARF_TOLERANCE: f64 = 0.10;

    /// Definition: masses spanning the polytrope, the seam and YREC, M☉.
    const MASSES: [f64; 5] = [0.08, 0.3, 1.0, 2.0, 8.0];
    /// Definition: the fusion share ends at 1 within this.
    const SHARE_END: f64 = 1e-6;

    fn sun() -> StellarProfile {
        profile(SolarMass(1.0), Dex(0.0), SOLAR_AGE).unwrap()
    }

    fn sample_at(p: &StellarProfile, r: f64) -> StellarSample {
        *p.samples
            .iter()
            .find(|s| (s.radius.get() - r).abs() < SAME_RADIUS)
            .unwrap_or_else(|| panic!("no sample at r/R {r}"))
    }

    #[test]
    fn the_sun_matches_the_standard_solar_model() {
        let p = sun();
        let centre = sample_at(&p, 0.0);
        let [_, rho_c, _, t_c] = B23[0];
        let rho = centre.density.0 / KG_M3_PER_G_CM3;
        assert!(
            (rho / rho_c - 1.0).abs() < CENTRAL_DENSITY_TOLERANCE,
            "ρ_c {rho}"
        );
        let t = centre.temperature.0;
        assert!(
            (t / t_c - 1.0).abs() < CENTRAL_TEMPERATURE_TOLERANCE,
            "T_c {t}"
        );
        for &[r, rho, pressure, temperature] in &B23[1..] {
            let s = sample_at(&p, r);
            let got = [
                s.density.0 / KG_M3_PER_G_CM3,
                s.pressure.0 / PA_PER_DYN_CM2,
                s.temperature.0,
            ];
            for (got, want) in got.into_iter().zip([rho, pressure, temperature]) {
                assert!(
                    (got / want - 1.0).abs() < B23_TOLERANCE,
                    "r/R {r}: {got} vs B23 {want}"
                );
            }
        }
    }

    #[test]
    fn the_sun_has_a_convective_envelope_from_0_71() {
        let p = sun();
        let n = p.layers.len();
        assert!(n >= 2, "{:?}", p.layers);
        assert!(p.layers[n - 1].convective);
        let base = p.layers[n - 2].outer_radius.get();
        assert!(ENVELOPE_BASE.contains(&base), "{base}");
    }

    #[test]
    fn the_sun_burns_inside_a_quarter_radius() {
        let share = sample_at(&sun(), SOLAR_CORE_RADIUS).fusion_share.get();
        assert!(share >= SOLAR_CORE_SHARE, "{share}");
    }

    #[test]
    fn a_red_dwarf_is_convective_throughout() {
        let p = profile(SolarMass(0.2), Dex(0.0), Gyr(5.0)).unwrap();
        assert_eq!(p.layers.len(), 1);
        assert!(p.layers[0].convective);
        assert!(p.samples.iter().all(|s| s.convective));
    }

    #[test]
    fn a_late_m_dwarf_central_density_matches_baraffe() {
        let p = profile(SolarMass(0.08), Dex(0.0), Gyr(5.0)).unwrap();
        let rho = p.samples[0].density.0 / KG_M3_PER_G_CM3;
        let error = rho / LATE_M_DWARF_CENTRAL_DENSITY - 1.0;
        assert!(error.abs() < LATE_M_DWARF_TOLERANCE, "ρ_c {rho}");
    }

    /// Source: central temperature of a 0.1 M☉ star at 5 Gyr, log T_c 6.661
    /// (Baraffe et al. 2015, BHAC15 tracks).
    const LATE_M_DWARF_CENTRAL_TEMPERATURE: f64 = 4.58e6;
    /// Definition: tolerance on the BHAC15 central temperature.
    const CENTRAL_TEMPERATURE_FROM_BARAFFE: f64 = 0.03;

    #[test]
    fn a_late_m_dwarf_central_temperature_matches_baraffe() {
        let p = profile(SolarMass(0.1), Dex(0.0), Gyr(5.0)).unwrap();
        let t = p.samples[0].temperature.0;
        let error = t / LATE_M_DWARF_CENTRAL_TEMPERATURE - 1.0;
        assert!(error.abs() < CENTRAL_TEMPERATURE_FROM_BARAFFE, "T_c {t}");
    }

    #[test]
    fn a_hot_star_has_a_convective_core() {
        let zams = life(SolarMass(3.0), Dex(0.0)).unwrap().zams;
        let p = profile(SolarMass(3.0), Dex(0.0), zams).unwrap();
        assert!(p.layers[0].convective, "{:?}", p.layers);
        assert!(p.layers.iter().any(|l| !l.convective));
        assert!(!p.layers[p.layers.len() - 1].convective, "{:?}", p.layers);
    }

    #[test]
    fn profiles_are_ordered_and_finite() {
        for mass in MASSES {
            for feh in [MIN_METALLICITY, MAX_METALLICITY] {
                let life = life(SolarMass(mass), feh).unwrap();
                let (start, end) = (life.zams.0, life.end.age().0);
                for age in [start, (start + end) / 2.0, end] {
                    let p = profile(SolarMass(mass), feh, Gyr(age)).unwrap();
                    let at = format!("{mass} Msun, {} dex, {age} Gyr", feh.0);
                    check_ordered(&p, &at);
                }
            }
        }
    }

    fn layer(r: f64, convective: bool) -> ProfileLayer {
        ProfileLayer {
            r,
            m: r,
            convective,
        }
    }

    #[test]
    fn a_layer_of_one_profile_follows_the_nearer() {
        // As 1 M☉ near its ZAMS: a transient convective core in `b` only.
        let a = [layer(0.7, false), layer(1.0, true)];
        let b = [layer(0.1, true), layer(0.72, false), layer(1.0, true)];
        let near_a = blend_layers(&a, &b, 0.25);
        assert_eq!(near_a.len(), 2, "{near_a:?}");
        assert!(!near_a[0].convective && (near_a[0].r - 0.705).abs() < SAME_RADIUS);
        let near_b = blend_layers(&a, &b, 0.75);
        assert_eq!(near_b.len(), 3, "{near_b:?}");
        assert!(near_b[0].convective && (near_b[0].r - 0.075).abs() < SAME_RADIUS);
        assert!((near_b[1].r - 0.715).abs() < SAME_RADIUS);
        // Thin surface zones in `a` only, dropped once `b` is nearer.
        let a = [
            layer(0.1, true),
            layer(0.98, false),
            layer(0.99, true),
            layer(1.0, false),
        ];
        let b = [layer(0.1, true), layer(1.0, false)];
        assert_eq!(blend_layers(&a, &b, 0.25).len(), 4);
        let near_b = blend_layers(&a, &b, 0.75);
        assert_eq!(near_b.len(), 2, "{near_b:?}");
        assert_eq!((near_b[1].r, near_b[1].m), (1.0, 1.0));
    }

    fn check_ordered(p: &StellarProfile, at: &str) {
        let s = &p.samples;
        assert_eq!(s[0].radius.get(), 0.0, "{at}");
        assert_eq!(s[s.len() - 1].radius.get(), 1.0, "{at}");
        for pair in s.windows(2) {
            assert!(pair[0].radius < pair[1].radius, "{at}: radii");
            assert!(pair[0].mass <= pair[1].mass, "{at}: masses");
            assert!(pair[0].fusion_share <= pair[1].fusion_share, "{at}: fusion");
        }
        let share = s[s.len() - 1].fusion_share.get();
        assert!(
            (share - 1.0).abs() < SHARE_END,
            "{at}: fusion ends at {share}"
        );
        for sample in s {
            let values = [sample.density.0, sample.pressure.0, sample.temperature.0];
            assert!(
                values.iter().all(|v| v.is_finite() && *v >= 0.0),
                "{at}: {values:?}"
            );
            assert!(sample.temperature.0 > 0.0, "{at}");
        }
        let l = &p.layers;
        for pair in l.windows(2) {
            assert!(pair[0].outer_radius < pair[1].outer_radius, "{at}: {l:?}");
            assert!(pair[0].outer_mass <= pair[1].outer_mass, "{at}: {l:?}");
            assert_ne!(pair[0].convective, pair[1].convective, "{at}: {l:?}");
        }
        let last = l[l.len() - 1];
        assert_eq!(
            (last.outer_radius.get(), last.outer_mass.get()),
            (1.0, 1.0),
            "{at}"
        );
    }
}
