//! The primordial spin of a planet: rotation period and obliquity, drawn
//! from the `spin` stream of the planet's trial seed. This is the birth value: tidal
//! despinning and Cassini states are a later step (#604).

use crate::SystemError;
use crate::archetype::ArchetypePlacement;
use crate::random::{log_uniform, normal, uniform};
use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::constants::{DAY, EARTH_EQUATORIAL_RADIUS, EARTH_MASS_PARAMETER};
use science::units::{Day, Degree, EarthMass, EarthRadius, Fraction};
use timeline::{Code, ContractError, PlanetClass, Provenance, Seed, Spin, Trace, Traced};

/// Cause codes of the spin's nodes.
mod codes {
    pub(crate) const ROTATION_PERIOD: &str = "system.draw.rotation-period";
    pub(crate) const BREAK_UP_FRACTION: &str = "system.draw.break-up-fraction";
    pub(crate) const OBLIQUITY: &str = "system.draw.obliquity";
}

/// Source: Kokubo & Genda 2010 (ApJL 714, L21) p19 table 2, realistic
/// model: the spin vector has three independent Gaussian components of
/// zero mean and `sigma = 0.41 omega_cr` (derived from the table).
const ROCKY_SIGMA_OVER_CRITICAL: f64 = 0.41;

/// Definition: draws of the rocky spin vector before the last one is
/// scaled below break-up; the rejection law (`|omega| >= omega_cr`) keeps
/// 88 % of draws, so reaching the cap has a probability below `1e-60`.
const ROCKY_MAX_DRAWS: usize = 64;

/// Definition: the fraction of the critical rate kept by the last draw
/// when the rejection reaches its cap, never reached in practice.
const ROCKY_FALLBACK_FRACTION: f64 = 0.99;

/// Definition: bounds of the giants' spin as a fraction `f` of the
/// break-up rate, log-uniform. A flattening of Bryan et al. 2018
/// (AJ 156, 100) p3 (mean 0.126) and Bryan et al. 2020 (factor 4 to 5
/// of variation, p1): a ratio of 5.7 holding 0.126 and the Solar
/// System's giants, Jupiter 0.30 and Saturn 0.40 included (derived; the
/// upper bound raised from 0.35 to hold Saturn, user, 2026-11-22).
const GIANT_FRACTION_RANGE: (f64, f64) = (0.07, 0.40);

/// Definition: the classes drawn with the giant law (a gas-accreting
/// spin); every other class takes the rocky law. Ice giants share it for
/// want of a sourced primordial spin (Uranus 0.17 and Neptune 0.16 lie
/// inside the range, derived).
pub(crate) fn is_giant(class: PlanetClass) -> bool {
    matches!(
        class,
        PlanetClass::GasGiant | PlanetClass::HotJupiter | PlanetClass::IceGiant
    )
}

/// Source: isotropic orientation, `p(eps) = sin(eps) / 2` on 0 to 180 deg
/// (Kokubo & Genda 2010 p11-12; Miguel & Brunini 2010, MNRAS 402, 1075),
/// by `eps = arccos(1 - 2u)`. Above 90 deg the spin is retrograde. For
/// giants this law is a Definition: no source gives their obliquity.
fn isotropic_obliquity(u: f64) -> Degree {
    Degree((1.0 - 2.0 * u).acos().to_degrees())
}

/// The primordial spin of a planet of mass `mass` and radius `radius`
/// (the trial's own, which the traced mass cannot give).
///
/// Drawn from `seed.derive("spin")`: a rocky planet takes one
/// 3-vector spin (its length gives the period, its angle to the orbit
/// normal the obliquity); a giant takes the obliquity then the rate
/// fraction. The rate as a fraction of break-up is the drawn leaf
/// (`system.draw.break-up-fraction`) and the period is recorded from it
/// and the mass. An archetype's obliquity replaces the drawn one
/// (`Provenance::Input`) after the draw, so asking it never shifts the
/// period.
///
/// Rocky: Kokubo & Genda 2010, Gaussian spin vector of
/// `sigma = 0.41 omega_cr` with `omega_cr = sqrt(GM / R^3)`, redrawn
/// when `|omega| >= omega_cr` (Definition, from Miguel & Brunini 2010:
/// faster planets do not survive). Giant: `P = 2 pi R / (f v_break)`,
/// `v_break = sqrt(GM / R)`, `f` log-uniform on 0.07 to 0.40.
pub(crate) fn spin(
    class: PlanetClass,
    mass: &Traced<EarthMass>,
    radius: EarthRadius,
    archetype: Option<&ArchetypePlacement>,
    seed: Seed,
    trace: &mut Trace,
) -> Result<Spin, SystemError> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.derive("spin").get());
    let gm = EARTH_MASS_PARAMETER * mass.value().0;
    let radius_metres = EARTH_EQUATORIAL_RADIUS.0 * radius.0;
    // The break-up angular rate, `sqrt(GM / R^3)`: a giant's `v_break / R`.
    let critical = (gm / radius_metres.powi(3)).sqrt();
    let (fraction, drawn_obliquity) = if is_giant(class) {
        let obliquity = isotropic_obliquity(uniform(&mut rng));
        let fraction = log_uniform(&mut rng, GIANT_FRACTION_RANGE.0, GIANT_FRACTION_RANGE.1);
        (fraction, obliquity)
    } else {
        // The vector is drawn in units of `omega_cr`, so that neither the
        // mass nor the radius touches the draw: the obliquity is a
        // function of the seed alone, the same for every non-giant class.
        let mut vector = [0.0_f64; 3];
        for _ in 0..ROCKY_MAX_DRAWS {
            vector = [
                ROCKY_SIGMA_OVER_CRITICAL * normal(&mut rng),
                ROCKY_SIGMA_OVER_CRITICAL * normal(&mut rng),
                ROCKY_SIGMA_OVER_CRITICAL * normal(&mut rng),
            ];
            if length(vector) < 1.0 {
                break;
            }
        }
        let rate = length(vector);
        if rate >= 1.0 {
            let scale = ROCKY_FALLBACK_FRACTION / rate;
            vector = vector.map(|component| component * scale);
        }
        let rate = length(vector);
        let obliquity = Degree((vector[2] / rate).clamp(-1.0, 1.0).acos().to_degrees());
        (rate, obliquity)
    };
    let fraction = Fraction::new(fraction).ok_or_else(|| {
        SystemError::Contract(ContractError::NotFinite {
            code: codes::BREAK_UP_FRACTION.to_owned(),
        })
    })?;
    let fraction = trace.leaf(
        Code::of(codes::BREAK_UP_FRACTION),
        fraction,
        Provenance::Drawn,
    )?;
    let period_seconds = std::f64::consts::TAU / (fraction.value().get() * critical);
    let period = trace.record(
        Code::of(codes::ROTATION_PERIOD),
        Day(period_seconds / DAY.0),
        &[fraction.node(), mass.node()],
    )?;
    let obliquity = match archetype.and_then(|placed| placed.obliquity) {
        Some(asked) => trace.leaf(Code::of(codes::OBLIQUITY), asked, Provenance::Input)?,
        None => trace.leaf(
            Code::of(codes::OBLIQUITY),
            drawn_obliquity,
            Provenance::Drawn,
        )?,
    };
    Ok(Spin {
        rotation_period: period,
        obliquity,
    })
}

fn length(vector: [f64; 3]) -> f64 {
    vector
        .iter()
        .map(|component| component * component)
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArchetypeId;
    use crate::archetype::placement;
    use timeline::Origin;

    const SAMPLES: u64 = 2_000;

    fn earth(trace: &mut Trace) -> Traced<EarthMass> {
        trace
            .leaf(Code::of("test.mass"), EarthMass(1.0), Provenance::Input)
            .unwrap()
    }

    fn jupiter(trace: &mut Trace) -> Traced<EarthMass> {
        trace
            .leaf(Code::of("test.mass"), EarthMass(317.8), Provenance::Input)
            .unwrap()
    }

    fn draw(class: PlanetClass, seed: u64) -> (Spin, Trace) {
        let mut trace = Trace::new();
        let (mass, radius) = match class {
            PlanetClass::GasGiant | PlanetClass::HotJupiter | PlanetClass::IceGiant => {
                (jupiter(&mut trace), EarthRadius(11.2))
            }
            _ => (earth(&mut trace), EarthRadius(1.0)),
        };
        let spin = spin(class, &mass, radius, None, Seed::new(seed), &mut trace).unwrap();
        (spin, trace)
    }

    /// Break-up period of a body, in days.
    fn break_up_days(mass: f64, radius: f64, power: i32) -> f64 {
        let (gm, r) = (
            EARTH_MASS_PARAMETER * mass,
            EARTH_EQUATORIAL_RADIUS.0 * radius,
        );
        std::f64::consts::TAU * (r.powi(power) / gm).sqrt() / DAY.0
    }

    #[test]
    fn the_rocky_obliquity_depends_only_on_the_seed() {
        let classes = [
            PlanetClass::Rocky,
            PlanetClass::SuperEarth,
            PlanetClass::OceanWorld,
            PlanetClass::SteamWorld,
            PlanetClass::MiniNeptune,
        ];
        let obliquity = |class, mass: f64, radius: f64, seed: u64| {
            let mut trace = Trace::new();
            let mass = trace
                .leaf(Code::of("test.mass"), EarthMass(mass), Provenance::Input)
                .unwrap();
            spin(
                class,
                &mass,
                EarthRadius(radius),
                None,
                Seed::new(seed),
                &mut trace,
            )
            .unwrap()
            .obliquity
            .value()
            .0
        };
        for seed in 0..200 {
            let reference = obliquity(PlanetClass::Rocky, 1.0, 1.0, seed);
            for class in classes {
                for mass in [0.1, 1.0, 5.0] {
                    for radius in [0.5, 1.0, 1.643, 2.5] {
                        assert_eq!(obliquity(class, mass, radius, seed), reference);
                    }
                }
            }
        }
    }

    #[test]
    fn rocky_obliquity_is_isotropic_over_the_half_turn() {
        let obliquities: Vec<f64> = (0..SAMPLES)
            .map(|seed| draw(PlanetClass::Rocky, seed).0.obliquity.value().0)
            .collect();
        let mean = obliquities.iter().sum::<f64>() / SAMPLES as f64;
        let retrograde = obliquities.iter().filter(|&&e| e > 90.0).count() as f64;
        assert!((mean - 90.0).abs() < 2.0, "mean {mean}");
        assert!((retrograde / SAMPLES as f64 - 0.5).abs() < 0.04);
        assert!(obliquities.iter().all(|e| (0.0..=180.0).contains(e)));
    }

    #[test]
    fn rocky_rate_follows_the_gaussian_vector_below_break_up() {
        let periods: Vec<f64> = (0..SAMPLES)
            .map(|seed| {
                draw(PlanetClass::SuperEarth, seed)
                    .0
                    .rotation_period
                    .value()
                    .0
            })
            .collect();
        let critical = break_up_days(1.0, 1.0, 3);
        assert!(periods.iter().all(|&p| p > critical), "above break-up");
        // Mean rate of the truncated Maxwell law: the rate is 2 pi / P.
        let mean_rate = periods.iter().map(|p| critical / p).sum::<f64>() / SAMPLES as f64;
        // Untruncated mean 1.596 sigma = 0.654; the truncation lowers it.
        assert!((0.55..0.66).contains(&mean_rate), "mean rate {mean_rate}");
    }

    #[test]
    fn giant_period_stays_in_the_fraction_range() {
        let slow = break_up_days(317.8, 11.2, 3) / 0.07;
        let fast = break_up_days(317.8, 11.2, 3) / 0.40;
        for seed in 0..SAMPLES {
            let (spin, _) = draw(PlanetClass::GasGiant, seed);
            let period = spin.rotation_period.value().0;
            assert!(period >= fast * 0.999 && period <= slow * 1.001, "{period}");
            let obliquity = spin.obliquity.value().0;
            assert!((0.0..=180.0).contains(&obliquity));
        }
    }

    #[test]
    fn giant_fraction_is_log_uniform() {
        let ln_mean = (0..SAMPLES)
            .map(|seed| {
                let period = draw(PlanetClass::IceGiant, seed)
                    .0
                    .rotation_period
                    .value()
                    .0;
                (break_up_days(317.8, 11.2, 3) / period).ln()
            })
            .sum::<f64>()
            / SAMPLES as f64;
        let expected = (0.07_f64.ln() + 0.40_f64.ln()) / 2.0;
        assert!((ln_mean - expected).abs() < 0.04, "{ln_mean} vs {expected}");
    }

    #[test]
    fn hot_jupiters_take_the_giant_law_and_others_the_rocky_one() {
        // A giant period is far above break-up (f <= 0.40); a rocky one
        // can come close to it. Same stream, same mass: only the law differs.
        let giant = draw(PlanetClass::HotJupiter, 7).0;
        let ratio = break_up_days(317.8, 11.2, 3) / giant.rotation_period.value().0;
        assert!((0.07..=0.40).contains(&ratio) || (ratio - 0.40).abs() < 1e-9);
        let fast_rocky = (0..SAMPLES)
            .map(|seed| {
                draw(PlanetClass::OceanWorld, seed)
                    .0
                    .rotation_period
                    .value()
                    .0
            })
            .fold(f64::INFINITY, f64::min);
        assert!(fast_rocky < break_up_days(1.0, 1.0, 3) / 0.40);
    }

    #[test]
    fn an_archetype_obliquity_is_kept_as_input_without_moving_the_period() {
        let id = ArchetypeId(Code::new("earth-like".to_string()).unwrap());
        let archetype = placement(&id).unwrap();
        let mut trace = Trace::new();
        let mass = earth(&mut trace);
        let asked = spin(
            PlanetClass::Rocky,
            &mass,
            EarthRadius(1.0),
            Some(archetype),
            Seed::new(5),
            &mut trace,
        )
        .unwrap();
        let free = draw(PlanetClass::Rocky, 5).0;
        assert_eq!(asked.obliquity.value().0, archetype.obliquity.unwrap().0);
        assert_eq!(asked.rotation_period.value(), free.rotation_period.value());
        let node = trace.node(asked.obliquity.node()).unwrap();
        assert!(matches!(node.origin, Origin::Leaf(Provenance::Input)));
    }

    #[test]
    fn a_drawn_obliquity_is_a_drawn_leaf_and_the_period_is_computed() {
        let (spin, trace) = draw(PlanetClass::Rocky, 3);
        let node = trace.node(spin.obliquity.node()).unwrap();
        assert!(matches!(node.origin, Origin::Leaf(Provenance::Drawn)));
        assert_eq!(node.code, Code::of(codes::OBLIQUITY));
        let period = trace.node(spin.rotation_period.node()).unwrap();
        assert_eq!(period.code, Code::of(codes::ROTATION_PERIOD));
        let Origin::Derived { from } = &period.origin else {
            panic!("{:?}", period.origin);
        };
        let fraction = trace.node(from[0]).unwrap();
        assert_eq!(fraction.code, Code::of(codes::BREAK_UP_FRACTION));
        assert!(matches!(fraction.origin, Origin::Leaf(Provenance::Drawn)));
    }

    #[test]
    fn the_same_seed_gives_the_same_spin() {
        assert_eq!(
            draw(PlanetClass::Rocky, 9).0.rotation_period.value(),
            draw(PlanetClass::Rocky, 9).0.rotation_period.value()
        );
    }
}
