//! The star of the system: asked or drawn mass, metallicity and date,
//! its main-sequence values and its interior, each traced.
//!
//! The class (M, K, G, F, A, B) is only a bucket for the mass draw: the
//! star's temperature comes from the stellar models, so a metal-poor
//! 1 M☉ star is hotter than G2. The class is not traced: the trace has
//! no node type for a class index.

// Consumed by the system draw, next pull request of the system-draw plan.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the system draw, next pull request")
)]

use crate::SystemError;
use crate::random::uniform;
use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::constants::{AGE_OF_UNIVERSE, SOLAR_MASS, SOLAR_RADIUS};
use science::stellar::{
    Lifetime, MAX_MASS, MAX_METALLICITY, MIN_MASS, MIN_METALLICITY, StellarError, StellarProfile,
    life, main_sequence, profile,
};
use science::units::{Dex, Gyr, Kilogram, Metre, SolarMass};
use timeline::{
    Code, InteriorProfile, Layer, Material, MatterState, Provenance, RadialSample, RadialSamples,
    Seed, StarPhase, StarState, Trace, Traced, Transport,
};

/// Source: Pecaut & Mamajek 2013 (ApJS 208, 9), table version
/// 2022.04.16, mass at the start of each class: M 0.08, K 0.57, G 0.88,
/// F 1.06, A 1.61, B 2.18 M☉; the last bound is the top of the tables.
pub(crate) const CLASS_BOUNDS: [f64; 7] = [0.08, 0.57, 0.88, 1.06, 1.61, 2.18, 8.0];

/// A star at the present, with the date and the end of its life.
pub struct Star {
    pub state: StarState,
    pub age: Traced<Gyr>,
    /// The end of the main sequence; 20 Gyr (the end of the tables) for
    /// a star that outlasts them.
    pub lifetime_end: Traced<Gyr>,
}

/// Cause codes of the star's nodes.
mod codes {
    pub(crate) const MASS: &str = "system.star.mass";
    pub(crate) const METALLICITY: &str = "system.star.metallicity";
    pub(crate) const AGE: &str = "system.star.age";
    pub(crate) const LIFETIME_END: &str = "system.star.lifetime-end";
    pub(crate) const RADIUS: &str = "system.star.radius";
    pub(crate) const LUMINOSITY: &str = "system.star.luminosity";
    pub(crate) const EFFECTIVE_TEMPERATURE: &str = "system.star.effective-temperature";
    pub(crate) const INTERIOR: &str = "system.star.interior";
}

/// Definition: the number of spectral classes (M, K, G, F, A, B), one
/// chance in six each.
const CLASSES: usize = CLASS_BOUNDS.len() - 1;

/// The star at the present date. `host_mass`, `metallicity` and `present`
/// are taken as given when asked, drawn when `None`, on one ChaCha8
/// stream of `seed.derive("star")` in this order: class (mass not
/// asked), mass, metallicity, date.
pub(crate) fn star(
    host_mass: Option<SolarMass>,
    metallicity: Option<Dex>,
    present: Option<Gyr>,
    seed: Seed,
    trace: &mut Trace,
) -> Result<Star, SystemError> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.derive("star").get());
    // The class, mass and metallicity are drawn even when asked, and the
    // drawn values discarded, so that asking one never moves another draw.
    let drawn_mass = draw_class_and_mass(&mut rng).1;
    let drawn_metallicity =
        Dex(MIN_METALLICITY.0 + (MAX_METALLICITY.0 - MIN_METALLICITY.0) * uniform(&mut rng));
    let (mass_value, mass_provenance) = match host_mass {
        Some(mass) => (mass, Provenance::Input),
        None => (drawn_mass, Provenance::Drawn),
    };
    if !(MIN_MASS.0..=MAX_MASS.0).contains(&mass_value.0) {
        return Err(SystemError::HostMassOutOfRange { mass: mass_value });
    }
    let (metallicity_value, metallicity_provenance) = match metallicity {
        Some(value) => (value, Provenance::Input),
        None => (drawn_metallicity, Provenance::Drawn),
    };
    if !(MIN_METALLICITY.0..=MAX_METALLICITY.0).contains(&metallicity_value.0) {
        return Err(SystemError::MetallicityOutOfRange {
            metallicity: metallicity_value,
        });
    }
    let life = life(mass_value, metallicity_value)?;
    let end = Gyr(life.end.age().0.min(AGE_OF_UNIVERSE.0));
    // The interval of the drawn date is never empty.
    debug_assert!(life.zams.0 <= end.0, "zams <= min(end, 13.8 Gyr)");
    let (age_value, age_provenance) = match present {
        Some(age) => (age, Provenance::Input),
        None => (
            Gyr(life.zams.0 + (end.0 - life.zams.0) * uniform(&mut rng)),
            Provenance::Drawn,
        ),
    };
    if age_value.0.is_nan() || age_value.0 < life.zams.0 {
        return Err(SystemError::PresentBeforeZams {
            present: age_value,
            zams: life.zams,
        });
    }
    if age_value.0 > end.0 {
        return Err(SystemError::PresentAfterLife {
            present: age_value,
            end,
        });
    }
    let mass = trace.leaf(Code::of(codes::MASS), mass_value, mass_provenance)?;
    let metallicity = trace.leaf(
        Code::of(codes::METALLICITY),
        metallicity_value,
        metallicity_provenance,
    )?;
    let age = trace.leaf(Code::of(codes::AGE), age_value, age_provenance)?;
    let causes = [mass.node(), metallicity.node(), age.node()];
    let lifetime_end = trace.record(
        Code::of(codes::LIFETIME_END),
        match life.end {
            Lifetime::Ends(age) | Lifetime::Beyond(age) => age,
        },
        &causes[..2],
    )?;
    let values = main_sequence(mass_value, metallicity_value, age_value)
        .map_err(|error| present_error(error, age_value))?;
    let stellar = profile(mass_value, metallicity_value, age_value)
        .map_err(|error| present_error(error, age_value))?;
    let radius = trace.record(Code::of(codes::RADIUS), values.radius, &causes)?;
    let luminosity = trace.record(Code::of(codes::LUMINOSITY), values.luminosity, &causes)?;
    let effective_temperature = trace.record(
        Code::of(codes::EFFECTIVE_TEMPERATURE),
        values.effective_temperature,
        &causes,
    )?;
    let interior = interior(&stellar, values.radius.0, mass_value, &causes, trace)?;
    Ok(Star {
        state: StarState {
            phase: StarPhase::MainSequence,
            mass,
            radius,
            luminosity,
            effective_temperature,
            metallicity,
            interior: Some(interior),
        },
        age,
        lifetime_end,
    })
}

/// Maps the errors of the tables that depend on the date to the system's.
fn present_error(error: StellarError, present: Gyr) -> SystemError {
    match error {
        StellarError::BeforeZams { zams } => SystemError::PresentBeforeZams { present, zams },
        StellarError::AfterTams { tams } => SystemError::PresentAfterLife {
            present,
            end: Gyr(tams.0.min(AGE_OF_UNIVERSE.0)),
        },
        other => other.into(),
    }
}

/// The class, one in six, then the mass log-uniform within the class
/// (`Definition:`).
fn draw_class_and_mass(rng: &mut ChaCha8Rng) -> (usize, SolarMass) {
    let class = ((uniform(rng) * CLASSES as f64) as usize).min(CLASSES - 1);
    let (low, high) = (CLASS_BOUNDS[class].ln(), CLASS_BOUNDS[class + 1].ln());
    let mass = (low + (high - low) * uniform(rng)).exp();
    (
        class,
        SolarMass(mass.clamp(CLASS_BOUNDS[class], CLASS_BOUNDS[class + 1])),
    )
}

/// The profile as an interior: layers in metres and kilograms (the mass
/// of each layer, not the mass inside it; a thin surface layer can weigh
/// 0 kg), samples in metres. Every node derives from the star's causes.
fn interior(
    stellar: &StellarProfile,
    radius_in_suns: f64,
    mass: SolarMass,
    causes: &[timeline::NodeId],
    trace: &mut Trace,
) -> Result<InteriorProfile, SystemError> {
    let radius = Metre(radius_in_suns * SOLAR_RADIUS.0);
    let star_mass = mass.0 * SOLAR_MASS.0;
    let mut inside = 0.0;
    let mut layers = Vec::with_capacity(stellar.layers.len());
    for (index, layer) in stellar.layers.iter().enumerate() {
        let code = |field: &str| Code::new(format!("system.star.layer-{index}.{field}"));
        let share = (layer.outer_mass.get() - inside).max(0.0);
        inside = layer.outer_mass.get();
        let transport = if layer.convective {
            Transport::Convective
        } else {
            Transport::Radiative
        };
        layers.push(Layer {
            outer_radius: trace.record(
                code("outer-radius")?,
                Metre(layer.outer_radius.get() * radius.0),
                causes,
            )?,
            mass: trace.record(code("mass")?, Kilogram(share * star_mass), causes)?,
            material: trace.record(code("material")?, Material::Plasma, causes)?,
            state: trace.record(code("state")?, MatterState::Plasma, causes)?,
            transport: Some(trace.record(code("transport")?, transport, causes)?),
        });
    }
    let samples = stellar
        .samples
        .iter()
        .map(|sample| RadialSample {
            radius: Metre(sample.radius.get() * radius.0),
            density: sample.density,
            pressure: sample.pressure,
            temperature: sample.temperature,
            transport: Some(if sample.convective {
                Transport::Convective
            } else {
                Transport::Radiative
            }),
            fusion_share: Some(sample.fusion_share),
        })
        .collect();
    let samples = trace.record(Code::of(codes::INTERIOR), RadialSamples(samples), causes)?;
    Ok(InteriorProfile { layers, samples })
}

#[cfg(test)]
mod tests {
    use super::*;
    use timeline::{Origin, Provenance};

    fn draw(seed: u64) -> (Star, Trace) {
        let mut trace = Trace::new();
        let star = star(None, None, None, Seed::new(seed), &mut trace).unwrap();
        (star, trace)
    }

    fn provenance(trace: &Trace, node: timeline::NodeId) -> Provenance {
        match trace.node(node).unwrap().origin {
            Origin::Leaf(p) => p,
            _ => panic!("not a leaf"),
        }
    }

    #[test]
    fn an_asked_star_is_kept() {
        let mut trace = Trace::new();
        let star = star(
            Some(SolarMass(1.0)),
            Some(Dex(0.2)),
            Some(Gyr(3.0)),
            Seed::new(1),
            &mut trace,
        )
        .unwrap();
        assert_eq!(star.state.mass.value().0, 1.0);
        assert_eq!(star.state.metallicity.value().0, 0.2);
        assert_eq!(star.age.value().0, 3.0);
        for node in [
            star.state.mass.node(),
            star.state.metallicity.node(),
            star.age.node(),
        ] {
            assert_eq!(provenance(&trace, node), Provenance::Input);
        }
    }

    #[test]
    fn classes_are_drawn_evenly() {
        let mut counts = [0u32; 6];
        for seed in 0..6000u64 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            counts[draw_class_and_mass(&mut rng).0] += 1;
        }
        for count in counts {
            assert!((900..=1100).contains(&count), "{counts:?}");
        }
    }

    #[test]
    fn masses_stay_in_their_class_bounds() {
        for seed in 0..600u64 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let (class, mass) = draw_class_and_mass(&mut rng);
            assert!(mass.0 >= CLASS_BOUNDS[class] && mass.0 <= CLASS_BOUNDS[class + 1]);
            assert!((0.08..=8.0).contains(&mass.0));
        }
    }

    #[test]
    fn metallicity_is_drawn_across_the_range() {
        let values: Vec<f64> = (0..600)
            .map(|seed| draw(seed).0.state.metallicity.value().0)
            .collect();
        assert!(values.iter().all(|v| (-1.0..=0.5).contains(v)));
        assert!(values.iter().cloned().fold(f64::MAX, f64::min) < -0.9);
        assert!(values.iter().cloned().fold(f64::MIN, f64::max) > 0.4);
    }

    #[test]
    fn the_drawn_date_lies_in_the_life() {
        for seed in 0..600 {
            let (star, _) = draw(seed);
            let life = life(*star.state.mass.value(), *star.state.metallicity.value()).unwrap();
            let age = star.age.value().0;
            assert!(age >= life.zams.0, "{seed}");
            assert!(age <= life.end.age().0.min(AGE_OF_UNIVERSE.0), "{seed}");
        }
    }

    #[test]
    fn a_b_star_draws_a_young_date() {
        let mut trace = Trace::new();
        let star = star(Some(SolarMass(6.0)), None, None, Seed::new(5), &mut trace).unwrap();
        assert!(star.age.value().0 < 0.1);
        assert_eq!(provenance(&trace, star.age.node()), Provenance::Drawn);
    }

    #[test]
    fn a_date_before_zams_is_refused() {
        let mut trace = Trace::new();
        let error = star(
            Some(SolarMass(0.08)),
            Some(Dex(0.0)),
            Some(Gyr(0.1)),
            Seed::new(1),
            &mut trace,
        )
        .err()
        .unwrap();
        assert!(
            matches!(error, SystemError::PresentBeforeZams { present, .. } if present.0 == 0.1)
        );
    }

    #[test]
    fn a_date_after_the_life_is_refused() {
        let mut trace = Trace::new();
        let error = star(
            Some(SolarMass(2.0)),
            Some(Dex(0.0)),
            Some(Gyr(5.0)),
            Seed::new(1),
            &mut trace,
        )
        .err()
        .unwrap();
        assert!(
            matches!(error, SystemError::PresentAfterLife { present, end }
            if present.0 == 5.0 && end.0 < 5.0)
        );
        let error = star(
            Some(SolarMass(0.5)),
            Some(Dex(0.0)),
            Some(Gyr(14.0)),
            Seed::new(1),
            &mut trace,
        )
        .err()
        .unwrap();
        assert!(
            matches!(error, SystemError::PresentAfterLife { end, .. } if end.0 == AGE_OF_UNIVERSE.0)
        );
    }

    #[test]
    fn a_host_above_eight_is_refused() {
        let mut trace = Trace::new();
        let error = star(Some(SolarMass(9.0)), None, None, Seed::new(1), &mut trace)
            .err()
            .unwrap();
        assert!(matches!(error, SystemError::HostMassOutOfRange { .. }));
        let error = star(None, Some(Dex(0.6)), None, Seed::new(1), &mut trace)
            .err()
            .unwrap();
        assert!(matches!(error, SystemError::MetallicityOutOfRange { .. }));
    }

    #[test]
    fn asking_the_mass_keeps_the_other_draws() {
        let (free, _) = draw(42);
        let mut trace = Trace::new();
        let asked = star(
            Some(*free.state.mass.value()),
            None,
            None,
            Seed::new(42),
            &mut trace,
        )
        .unwrap();
        assert_eq!(
            asked.state.metallicity.value(),
            free.state.metallicity.value()
        );
        assert_eq!(asked.age.value(), free.age.value());
    }

    #[test]
    fn the_mass_edges_are_accepted() {
        for mass in [0.08, 8.0] {
            for metallicity in [-1.0, 0.5] {
                let mut trace = Trace::new();
                let star = star(
                    Some(SolarMass(mass)),
                    Some(Dex(metallicity)),
                    None,
                    Seed::new(3),
                    &mut trace,
                )
                .unwrap();
                star.state.validate().unwrap();
            }
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_star() {
        let (a, _) = draw(42);
        let (b, _) = draw(42);
        assert_eq!(a.state, b.state);
        assert_eq!(a.age, b.age);
    }

    #[test]
    fn the_state_validates() {
        for seed in 0..20 {
            let (star, _) = draw(seed);
            star.state.validate().unwrap();
            let interior = star.state.interior.as_ref().unwrap();
            assert!(!interior.layers.is_empty());
        }
    }

    #[test]
    fn the_interior_traces_back_to_the_mass() {
        let (star, trace) = draw(7);
        let interior = star.state.interior.unwrap();
        let node = trace.node(interior.samples.node()).unwrap();
        let Origin::Derived { from } = &node.origin else {
            panic!("a leaf");
        };
        assert!(from.contains(&star.state.mass.node()));
        assert_eq!(node.code.as_str(), "system.star.interior");
    }
}
