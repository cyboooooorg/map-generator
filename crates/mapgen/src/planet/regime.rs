//! The runaway greenhouse, the water a planet keeps and its tectonic
//! regime, derived from its light, its mass and its heat (part 2 of the
//! physical planet model, issue #423).
use serde::{Deserialize, Serialize};

use super::escape::energy_limited_loss_kg;
use super::{Context, Interior, InternalHeat, PlanetInputs};
use crate::exomoon::EARTH_KG;
use crate::geology::{PLATES_MIN_MASS_EARTH, Regime};
use crate::star::habitable_fluxes;

/// Mass of one Earth ocean, kg: 1.4e21 (Earth's oceans hold
/// 1.335e21 kg, Charette & Smith 2010, *Oceanography* 23(2); rounded up
/// with the ice caps and groundwater).
pub const OCEAN_KG: f64 = 1.4e21;

/// Mass of water per unit mass of hydrogen lost: `H₂O` is 18 for 2 of
/// hydrogen. Stoichiometry. The oxygen left behind is not tracked.
const WATER_PER_HYDROGEN: f64 = 9.0;

/// Internal heat flux above which a body loses its heat through volcanic
/// heat pipes rather than by plates or conduction, W/m². Moore & Webb 2013
/// (*Nature* 501, 501, doi:10.1038/nature12424) model Io's heat pipe,
/// about 2 W/m² (Earth's mean is 0.087). Value from memory, not checked
/// against the paper.
pub const HEAT_PIPE_FLUX_W_M2: f64 = 2.0;

/// The water a planet keeps.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Water {
    /// The insolation exceeds the runaway greenhouse limit.
    pub runaway: bool,
    /// Water kept, kg.
    pub kept_kg: f64,
    /// Water kept, Earth oceans ([`OCEAN_KG`] each).
    pub oceans: f64,
}

/// The water a planet keeps after its star's light.
///
/// A runaway greenhouse starts when the insolation exceeds the runaway
/// limit of [`habitable_fluxes`] (Kopparapu et al. 2014,
/// doi:10.1088/2041-8205/787/2/L29, the fit for 1 M-earth; their mass
/// correction is not applied, not checked). In a runaway the water is in
/// the air, where the XUV splits it and the hydrogen escapes at the
/// energy-limited rate (Owen & Wu 2017, [`energy_limited_loss_kg`]) with
/// 9 kg of water for each kg of hydrogen: `kept = max(0, water - 9 H)`.
/// Without a runaway the cold trap keeps the water.
pub fn water(
    context: &Context,
    inputs: &PlanetInputs,
    interior: &Interior,
    fluence_rel: f64,
) -> Water {
    let initial = inputs.water_fraction * inputs.mass_earth * EARTH_KG;
    let (runaway_limit, _) = habitable_fluxes(context.teff_k as f32);
    let runaway = context.insolation > f64::from(runaway_limit);
    let kept_kg = if runaway {
        let hydrogen = energy_limited_loss_kg(interior, inputs.mass_earth, fluence_rel);
        (initial - WATER_PER_HYDROGEN * hydrogen).max(0.0)
    } else {
        initial
    };
    Water {
        runaway,
        kept_kg,
        oceans: kept_kg / OCEAN_KG,
    }
}

/// The tectonic regime of a body. A labelled calibration: the literature
/// disagrees on the mass dependence (see [`crate::geology::regime_of`]).
///
/// - heat pipe above [`HEAT_PIPE_FLUX_W_M2`] of internal heat (Io);
/// - plates from [`PLATES_MIN_MASS_EARTH`] when water is kept (Earth);
/// - episodic when the mass allows plates but the water is gone (Venus,
///   Weller et al. 2023, doi:10.1038/s41550-023-02102-w);
/// - stagnant otherwise (Mars, Mercury, the Moon).
///
/// Calibration, no source, for the mass threshold and the water rule:
/// Earth, Venus, Mars and Mercury.
pub fn regime(
    inputs: &PlanetInputs,
    heat: &InternalHeat,
    interior: &Interior,
    water: &Water,
) -> Regime {
    if heat.flux_w_m2(interior.radius_earth) > HEAT_PIPE_FLUX_W_M2 {
        Regime::HeatPipe
    } else if inputs.mass_earth < f64::from(PLATES_MIN_MASS_EARTH) {
        Regime::Stagnant
    } else if water.kept_kg > 0.0 {
        Regime::Plates
    } else {
        Regime::Episodic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::heat::{EARTH_SILICATE_KG, radiogenic_w};
    use crate::planet::interior::interior;
    use crate::planet::reference::{exposure_at, inputs, inputs_of, reference};
    use crate::planet::xuv::xuv_fluence;
    use crate::star::{StarInfo, parse_type};

    /// The Sun's context at `orbit_au`, 4.6 Gyr, with the insolation of
    /// exactly `1/a²` (the table's G2V is slightly brighter).
    fn sun(orbit_au: f64) -> Context {
        let star = StarInfo::new(parse_type("G2V").unwrap(), orbit_au as f32, 0.0, 4.6);
        let mut c = Context::of(&star, 0.0, 2.7);
        c.insolation = 1.0 / (orbit_au * orbit_au);
        c.star_mass = 1.0;
        c
    }

    /// Water, heat and regime of a body at `orbit_au` with `tidal_w`.
    fn derive(inputs: &PlanetInputs, orbit_au: f64, tidal_w: f64) -> (Water, Regime) {
        let c = sun(orbit_au);
        let int = interior(inputs, &exposure_at(orbit_au));
        let w = water(&c, inputs, &int, xuv_fluence(&c));
        let silicate = inputs.mass_earth * (1.0 - inputs.water_fraction) * 5.972e24;
        let heat = InternalHeat {
            radiogenic_w: radiogenic_w(silicate.min(EARTH_SILICATE_KG * 10.0), 1.0, 4.6),
            tidal_w,
        };
        (w, regime(inputs, &heat, &int, &w))
    }

    #[test]
    fn venus_ran_away_and_dried() {
        let v = reference("Venus");
        let (w, _) = derive(&inputs_of(v), v.orbit_au, 0.0);
        assert!(w.runaway);
        assert_eq!(w.kept_kg, 0.0);
        // Given an Earth ocean, it would have lost it too.
        let wet = PlanetInputs {
            water_fraction: 0.00023,
            ..inputs_of(v)
        };
        let (w, _) = derive(&wet, v.orbit_au, 0.0);
        assert!(w.runaway);
        assert_eq!(w.kept_kg, 0.0);
    }

    #[test]
    fn earth_keeps_its_ocean() {
        let e = reference("Earth");
        let (w, _) = derive(&inputs_of(e), e.orbit_au, 0.0);
        assert!(!w.runaway);
        assert!((0.9..=1.1).contains(&w.oceans), "{}", w.oceans);
    }

    #[test]
    fn the_solar_system_has_its_regimes() {
        let regime_of_body = |name: &str, tidal: f64| {
            let b = reference(name);
            derive(&inputs_of(b), b.orbit_au, tidal).1
        };
        assert_eq!(regime_of_body("Earth", 0.0), Regime::Plates);
        assert_eq!(regime_of_body("Venus", 0.0), Regime::Episodic);
        assert_eq!(regime_of_body("Mars", 0.0), Regime::Stagnant);
        assert_eq!(regime_of_body("Mercury", 0.0), Regime::Stagnant);
        assert_eq!(regime_of_body("Io", 1.0e14), Regime::HeatPipe);
    }

    #[test]
    fn a_dry_super_earth_is_not_plated() {
        let dry = inputs(5.0, 0.325, 0.0);
        let (w, r) = derive(&dry, 1.5, 0.0);
        assert!(!w.runaway);
        assert_ne!(r, Regime::Plates);
    }
}
