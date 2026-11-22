//! A provisional classifier for the draw, until the planet model's
//! `class_of` is public. The CLI and the tests use it.

use science::units::{EarthRadius, KilogramPerCubicMetre};
use timeline::{PlanetClass, Trace};

use crate::{SystemError, Trial, TrialOutcome};

/// Source: the mean density of the Earth, 5 514 kg/m³ (NASA Earth fact
/// sheet), to turn a stand-in density into Earth radii.
const EARTH_DENSITY: f64 = 5514.0;

/// A stand-in for the planet model until mapgen1's lands: below 2 M⊕
/// rocky, below 10 a super-Earth, below 30 a mini-Neptune, else a gas
/// giant (a hot Jupiter inside 0.1 au); small planets of radius M^0.27,
/// giants of density 1 300 kg/m³.
///
/// # Errors
///
/// Never: the signature is the one `draw_with` takes.
pub fn classify(trial: &Trial, _: &Trace) -> Result<TrialOutcome, SystemError> {
    let mass = trial.mass.value().0;
    let a = trial.orbit.semi_major_axis.value().0;
    let class = match mass {
        m if m < 2.0 => PlanetClass::Rocky,
        m if m < 10.0 => PlanetClass::SuperEarth,
        m if m < 30.0 => PlanetClass::MiniNeptune,
        _ if a < 0.1 => PlanetClass::HotJupiter,
        _ => PlanetClass::GasGiant,
    };
    let (radius, density) = if mass < 30.0 {
        (mass.powf(0.27), 5500.0)
    } else {
        ((mass * EARTH_DENSITY / 1300.0).cbrt(), 1300.0)
    };
    Ok(TrialOutcome {
        class,
        radius: EarthRadius(radius),
        mean_density: KilogramPerCubicMetre(density),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::families::tests::placed;

    #[test]
    fn stand_in_classes_follow_the_mass_steps() {
        let class_of = |mass: f64, au: f64| {
            let mut trace = Trace::new();
            let trial = placed(&mut trace, mass, au, 0.0, false).trial;
            classify(&trial, &trace).unwrap().class
        };
        assert_eq!(class_of(1.0, 1.0), PlanetClass::Rocky);
        assert_eq!(class_of(5.0, 1.0), PlanetClass::SuperEarth);
        assert_eq!(class_of(20.0, 1.0), PlanetClass::MiniNeptune);
        assert_eq!(class_of(100.0, 1.0), PlanetClass::GasGiant);
        assert_eq!(class_of(100.0, 0.05), PlanetClass::HotJupiter);
    }
}
