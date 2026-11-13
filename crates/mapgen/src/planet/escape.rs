//! Photoevaporation: the hydrogen and helium envelope a planet loses to
//! the star's X-ray and extreme-ultraviolet light over its system's age.
//!
//! Energy-limited escape (Owen & Wu 2017, ApJ 847, 29,
//! doi:10.3847/1538-4357/aa890a, eq. 19, after Lecavelier des Etangs 2007
//! and Erkaev et al. 2007): the mass lost is `η π R³ F / (G M)`, with `F`
//! the XUV energy per unit area received, `R` the planet's radius and `M`
//! its mass. It is what opens the radius valley: nothing here is tuned to
//! the valley's position, the measurement is in the census.
use super::{Interior, PlanetInputs};
use crate::exomoon::{EARTH_KG, EARTH_M, G};
use crate::planet::xuv::EARTH_XUV_FLUENCE_J_M2;

/// Efficiency `η` of the XUV photons at removing mass. Owen & Wu 2017
/// (checked in section III.1 of the paper): "of order 0.1 for low-mass
/// planets", adopted there as a constant `η = 0.1` (Lopez & Fortney
/// 2013). It falls for deep potential wells (their eq. 31), which this
/// model ignores: giants lose next to nothing either way.
const EFFICIENCY: f64 = 0.1;

/// Below this share of the planet's mass, an envelope is gone: what is
/// left is a thin atmosphere of the later steps, not a hydrogen shell.
pub const MIN_ENVELOPE: f64 = 1.0e-3;

/// Mass a body of `mass_earth` and the radius of `interior` can lose to
/// `fluence_rel` times Earth's XUV fluence, kg: Owen & Wu 2017, eq. 19,
/// integrated over the age. It is not capped by what the body holds.
pub fn energy_limited_loss_kg(interior: &Interior, mass_earth: f64, fluence_rel: f64) -> f64 {
    let radius = interior.radius_earth * EARTH_M;
    let fluence = fluence_rel * EARTH_XUV_FLUENCE_J_M2;
    EFFICIENCY * std::f64::consts::PI * radius.powi(3) * fluence / (G * mass_earth * EARTH_KG)
}

/// The inputs of a planet once photoevaporation has taken its share of the
/// envelope: `envelope_fraction` reduced (to `0.0` once under
/// [`MIN_ENVELOPE`]) and `mass_earth` lowered by the envelope lost. The
/// loss is [`energy_limited_loss_kg`] at the radius of `interior`, which
/// is the radius before the loss (an upper bound: the planet shrinks as it
/// thins), capped by the envelope.
pub fn envelope_after_escape(
    inputs: &PlanetInputs,
    interior: &Interior,
    fluence_rel: f64,
) -> PlanetInputs {
    let envelope = inputs.mass_earth * inputs.envelope_fraction;
    if envelope <= 0.0 {
        return inputs.clone();
    }
    let lost = (energy_limited_loss_kg(interior, inputs.mass_earth, fluence_rel) / EARTH_KG)
        .clamp(0.0, envelope);
    let mass = inputs.mass_earth - lost;
    let left = envelope - lost;
    let fraction = left / mass;
    let (mass, fraction) = if fraction < MIN_ENVELOPE {
        (inputs.mass_earth - envelope, 0.0)
    } else {
        (mass, fraction)
    };
    PlanetInputs {
        mass_earth: mass,
        envelope_fraction: fraction,
        ..inputs.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::interior::interior;
    use crate::planet::reference::{exposure_at, inputs_of, reference, with_envelope};

    /// Inputs after `fluence_rel` at `orbit_au` (the exposure sets the
    /// envelope's radius).
    fn after(inputs: &PlanetInputs, orbit_au: f64, fluence_rel: f64) -> PlanetInputs {
        let before = interior(inputs, &exposure_at(orbit_au));
        envelope_after_escape(inputs, &before, fluence_rel)
    }

    /// Fluence of the Sun at `orbit_au` over 5 Gyr, relative to Earth's.
    fn sun_fluence(orbit_au: f64) -> f64 {
        1.0 / (orbit_au * orbit_au) * 1.03
    }

    #[test]
    fn a_close_mini_neptune_is_stripped_to_its_core() {
        let inputs = with_envelope(5.0, 0.01);
        let left = after(&inputs, 0.05, sun_fluence(0.05));
        assert_eq!(left.envelope_fraction, 0.0);
        let radius = interior(&left, &exposure_at(0.05)).radius_earth;
        assert!(radius < 1.8, "{radius}");
    }

    #[test]
    fn giants_and_temperate_mini_neptunes_keep_their_envelope() {
        let jupiter = inputs_of(reference("Jupiter"));
        let left = after(&jupiter, 0.05, sun_fluence(0.05));
        let kept = left.mass_earth * left.envelope_fraction
            / (jupiter.mass_earth * jupiter.envelope_fraction);
        assert!(kept >= 0.99, "{kept}");

        let mini = with_envelope(5.0, 0.01);
        let left = after(&mini, 0.5, sun_fluence(0.5));
        let kept = left.mass_earth * left.envelope_fraction / (mini.mass_earth * 0.01);
        assert!(kept >= 0.5, "{kept}");
    }

    #[test]
    fn rocky_bodies_are_untouched() {
        let earth = inputs_of(reference("Earth"));
        assert_eq!(after(&earth, 1.0, 1.0), earth);
        assert_eq!(after(&earth, 0.05, 400.0), earth);
    }

    #[test]
    fn loss_never_exceeds_the_envelope() {
        for (mass, envelope) in [(2.0, 0.005), (5.0, 0.01), (10.0, 0.2), (300.0, 0.9)] {
            let inputs = with_envelope(mass, envelope);
            let core = mass * (1.0 - envelope);
            for fluence in [0.0, 1.0, 1.0e2, 1.0e4, 1.0e6] {
                let left = after(&inputs, 0.05, fluence);
                assert!(
                    (0.0..=envelope).contains(&left.envelope_fraction),
                    "{mass} {fluence}: {}",
                    left.envelope_fraction
                );
                assert!(left.mass_earth >= core - 1e-9, "{mass} {fluence}");
                assert!(left.mass_earth <= mass + 1e-9);
            }
        }
    }
}
