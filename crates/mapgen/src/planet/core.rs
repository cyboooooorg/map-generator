//! The iron core of a planet, whether it is still liquid and whether it
//! drives a dynamo (part 2 of the physical planet model, issue #423).
//!
//! A parametrised thermal history in the style of Stevenson, Spohn &
//! Schubert 1983 (*Icarus* 54, 466, doi:10.1016/0019-1035(83)90241-5) and
//! Driscoll & Barnes 2015 (doi:10.1089/ast.2015.1325), cut down to one
//! cooling time per body: the core's excess heat leaves through the
//! mantle, quickly under plates and slowly under a lid; it first drives a
//! thermal dynamo, then grows an inner core whose light elements drive a
//! compositional one, and at last freezes. The field follows the
//! energy-flux scaling of Christensen 2010.
use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use super::{Interior, Material, PlanetInputs};
use crate::exomoon::{EARTH_KG, EARTH_M};
use crate::geology::Regime;

/// Seconds in a gigayear (Julian years).
const GYR_S: f64 = 3.155_76e16;

/// Specific heat of the core, J/(kg·K). Stevenson et al. 1983 take about
/// 800 for iron alloy. Value from memory, not checked against the paper.
const CORE_HEAT_CAPACITY: f64 = 800.0;

/// Thermal conductivity of the mantle, W/(m·K). Stevenson et al. 1983
/// take 4. Value from memory, not checked against the paper.
const MANTLE_CONDUCTIVITY: f64 = 4.0;

/// The core's initial excess temperature over the one it tends to, K.
/// Calibration, no source: with it Earth's core loses about 10 TW today,
/// the middle of the 5 to 15 TW of Lay, Hernlund & Buffett 2008
/// (doi:10.1038/ngeo.2007.44, range not checked against the paper).
const INITIAL_EXCESS_K: f64 = 2800.0;

/// The thermal dynamo of a body under a lid stops at this many cooling
/// times: its core-mantle heat flux has fallen below the adiabat, as
/// Mars's did (Stevenson et al. 1983; Nimmo & Stevenson 2000,
/// doi:10.1029/1999JE001216). Calibration, no source: Mars's dynamo stops
/// near 0.6 Gyr (its crust records none after about 4.1 Ga).
const THERMAL_DYNAMO_END_TAU: f64 = 0.075;

/// An inner core starts to grow at this many cooling times (the centre
/// reaches the liquidus). Calibration, no source: Earth's inner core
/// nucleates near 4 Gyr (about 0.5 Ga old, within the young-inner-core
/// estimates), Mercury's before 4.6 Gyr, Mars's and Venus's after.
const INNER_CORE_START_TAU: f64 = 1.0;

/// The core is solid throughout at this many cooling times. Calibration,
/// no source: the Moon's core freezes near 3.3 Gyr (its dynamo stopped
/// 1 to 2.5 Ga ago), Mercury's is still liquid at 4.6 Gyr.
const CORE_FROZEN_TAU: f64 = 2.25;

/// The convective speed-up of the core's cooling through the mantle, a
/// Nusselt number over conduction across the whole mantle, by regime.
/// Calibration, no source: plates 50 (Earth's inner core near 4 Gyr),
/// episodic 10 (Venus has no dynamo at 4.6 Gyr), a stagnant lid 5 (Mars,
/// Mercury and the Moon). A heat pipe cools the lithosphere, not the
/// tidally heated mantle below: it takes the lid's value (no target).
fn nusselt(regime: Regime) -> f64 {
    match regime {
        Regime::Plates => 50.0,
        Regime::Episodic => 10.0,
        Regime::Stagnant | Regime::HeatPipe => 5.0,
    }
}

/// Magnetic constant µ₀, T·m/A.
const MU_0: f64 = 4.0e-7 * PI;

/// Prefactor `c` of the energy-flux scaling for a dipolar dynamo, 0.63:
/// Christensen, Holzwarth & Reiners 2009 (*Nature* 457, 167,
/// doi:10.1038/nature07626, p. 168; checked). Christensen 2010
/// (doi:10.1007/s11214-009-9553-2) gives 0.23 for a multipolar one,
/// not used here. The efficiency `F` and the ohmic share `f_ohm` are
/// taken as 1.
const FLUX_SCALING_C: f64 = 0.63;

/// Share of the core's rms field in the dipole at the core's surface for
/// a dipolar dynamo driven from the top of the core. Calibration, no
/// source: Earth's surface field about 30 µT.
const DIPOLE_SHARE: f64 = 0.067;

/// The same share for a compositional dynamo under a stably stratified
/// top of the core, whose field is mostly multipolar and screened by the
/// stable layer (Christensen 2006, doi:10.1038/nature05342). Calibration,
/// no source: Mercury's equatorial dipole of about 0.2 to 0.3 µT (Anderson
/// et al. 2011, doi:10.1126/science.1211001, 195 nT R_M³, not checked).
const DEEP_DIPOLE_SHARE: f64 = 3.0e-4;

/// The state of a planet's iron core.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum CoreState {
    /// No iron core.
    None,
    /// At least partly liquid (an inner core may be growing).
    Liquid,
    /// Solid throughout.
    Frozen,
}

/// A planet's core and its dynamo.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Core {
    pub state: CoreState,
    /// A dynamo runs in the core.
    pub dynamo: bool,
    /// Surface equatorial dipole field, µT; 0 without a dynamo.
    pub field_ut: f64,
}

const NO_CORE: Core = Core {
    state: CoreState::None,
    dynamo: false,
    field_ut: 0.0,
};

/// The core of a planet of `interior` in `regime` at `age_gyr`. A
/// calibration on Earth, Venus, Mars, Mercury and the Moon (see the
/// constants); the present regime is taken for the whole history.
///
/// **Cooling time.** The core's excess heat, `ρ c (4/3)π r_c³ ΔT`, leaves
/// through its surface across the mantle of thickness `d = R − r_c` at
/// `Nu k ΔT / d`, so it decays as `e^(−t/τ)` with
/// `τ = ρ c r_c d / (3 k Nu)`. The levers are the core's size (a big
/// core holds its heat longer), the mantle's thickness (Mercury's thin
/// mantle lets its core cool four times faster than Mars's, which is why
/// its inner core grows and Mars's does not yet) and the regime's
/// Nusselt number (plates cool the core ten times faster than a lid).
///
/// **Phases.** Under plates the core-mantle flux stays above the adiabat
/// and a thermal dynamo runs until the core freezes (Nimmo & Stevenson
/// 2000). Under a lid the thermal dynamo stops at
/// [`THERMAL_DYNAMO_END_TAU`] `τ`; from [`INNER_CORE_START_TAU`] `τ` an
/// inner core grows and drives a deep compositional dynamo; from
/// [`CORE_FROZEN_TAU`] `τ` the core is solid and the dynamo is gone.
/// Sulphur is not a lever: it would lower the liquidus and delay all of
/// this (Stevenson et al. 1983), but no target needs it.
///
/// **Field.** The heat flux out of the core is
/// `q = ρ c r_c ΔT₀ e^(−t/τ) / (3τ)`; the rms field inside the core is
/// `B² / 2µ₀ = c ρ^(1/3) q^(2/3)` (Christensen 2010), of which
/// [`DIPOLE_SHARE`] (or [`DEEP_DIPOLE_SHARE`] for a deep dynamo) is the
/// dipole at the core's surface, carried to the planet's surface by
/// `(r_c / R)³`.
pub fn core(inputs: &PlanetInputs, interior: &Interior, regime: Regime, age_gyr: f64) -> Core {
    let Some(iron) = interior
        .layers
        .iter()
        .find(|l| l.material == Material::Iron && l.mass_earth > 0.0)
    else {
        return NO_CORE;
    };
    if inputs.core_mass_fraction <= 0.0 {
        return NO_CORE;
    }
    let r_core = iron.outer_radius_earth * EARTH_M;
    let radius = interior.radius_earth * EARTH_M;
    let density = iron.mass_earth * EARTH_KG / (4.0 / 3.0 * PI * r_core.powi(3));
    let mantle = (radius - r_core).max(0.0);
    let tau_s = density * CORE_HEAT_CAPACITY * r_core * mantle
        / (3.0 * MANTLE_CONDUCTIVITY * nusselt(regime));
    if tau_s <= 0.0 {
        // No mantle to cool through: the model does not apply.
        return Core {
            state: CoreState::Liquid,
            dynamo: false,
            field_ut: 0.0,
        };
    }
    let t = age_gyr * GYR_S / tau_s;
    if t >= CORE_FROZEN_TAU {
        return Core {
            state: CoreState::Frozen,
            dynamo: false,
            field_ut: 0.0,
        };
    }
    let thermal = regime == Regime::Plates || t < THERMAL_DYNAMO_END_TAU;
    let compositional = t >= INNER_CORE_START_TAU;
    if !thermal && !compositional {
        return Core {
            state: CoreState::Liquid,
            dynamo: false,
            field_ut: 0.0,
        };
    }
    let flux =
        density * CORE_HEAT_CAPACITY * r_core * INITIAL_EXCESS_K * (-t).exp() / (3.0 * tau_s);
    let b_rms = (2.0 * MU_0 * FLUX_SCALING_C * density.cbrt() * flux.powf(2.0 / 3.0)).sqrt();
    let share = if thermal {
        DIPOLE_SHARE
    } else {
        DEEP_DIPOLE_SHARE
    };
    Core {
        state: CoreState::Liquid,
        dynamo: true,
        field_ut: share * b_rms * (r_core / radius).powi(3) * 1.0e6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::interior::interior;
    use crate::planet::reference::{exposure_at, inputs, inputs_of, reference};

    /// The Solar System's age, Gyr.
    const NOW: f64 = 4.567;

    /// The core of the reference body `name` in `regime` at `age_gyr`.
    fn core_of(name: &str, regime: Regime, age_gyr: f64) -> Core {
        let b = reference(name);
        let i = inputs_of(b);
        core(&i, &interior(&i, &exposure_at(b.orbit_au)), regime, age_gyr)
    }

    #[test]
    fn the_solar_system_has_its_dynamos() {
        // Regimes as `regime::regime` derives them (Task 3).
        let earth = core_of("Earth", Regime::Plates, NOW);
        assert!(earth.dynamo);
        assert!(
            (20.0..=70.0).contains(&earth.field_ut),
            "{}",
            earth.field_ut
        );
        let mercury = core_of("Mercury", Regime::Stagnant, NOW);
        assert!(mercury.dynamo);
        assert!(
            mercury.field_ut > 0.0 && mercury.field_ut < 2.0,
            "{}",
            mercury.field_ut
        );
        let mars = core_of("Mars", Regime::Stagnant, NOW);
        assert!(!mars.dynamo);
        assert_eq!(mars.field_ut, 0.0);
        assert!(core_of("Mars", Regime::Stagnant, 0.3).dynamo);
        assert!(!core_of("Venus", Regime::Episodic, NOW).dynamo);
        assert!(!core_of("Moon", Regime::Stagnant, NOW).dynamo);
    }

    #[test]
    fn no_iron_no_core() {
        let i = inputs(1.0, 0.0, 0.0);
        let c = core(&i, &interior(&i, &exposure_at(1.0)), Regime::Plates, NOW);
        assert_eq!(c.state, CoreState::None);
        assert!(!c.dynamo);
        assert_eq!(c.field_ut, 0.0);
    }

    #[test]
    fn cores_freeze_with_age() {
        for name in ["Mercury", "Venus", "Earth", "Mars", "Moon"] {
            for regime in [
                Regime::Plates,
                Regime::Stagnant,
                Regime::Episodic,
                Regime::HeatPipe,
            ] {
                let mut frozen = false;
                let mut froze_at_all = false;
                for step in 0..=400 {
                    let age = f64::from(step) * 0.05;
                    let c = core_of(name, regime, age);
                    assert_ne!(c.state, CoreState::None, "{name}");
                    if frozen {
                        assert_eq!(c.state, CoreState::Frozen, "{name} {regime:?} {age}");
                    }
                    frozen = c.state == CoreState::Frozen;
                    froze_at_all |= frozen;
                    if frozen {
                        assert!(!c.dynamo, "{name} {regime:?} {age}");
                    }
                }
                if name == "Moon" && regime == Regime::Stagnant {
                    assert!(froze_at_all, "the Moon's core freezes within 20 Gyr");
                }
            }
        }
    }
}
