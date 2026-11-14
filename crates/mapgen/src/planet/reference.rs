//! Solar System bodies the planet model is checked against.
//!
//! Masses and mean radii: NASA planetary fact sheets
//! (nssdc.gsfc.nasa.gov/planetary/factsheet, volumetric mean radii) and,
//! for the satellites, the JPL satellite physical parameters
//! (ssd.jpl.nasa.gov/sats/phys_par, GM over Earth's 398 600.4 km³/s²,
//! radii over Earth's 6371 km). Orbits: semi-major axes of the planets
//! (a moon takes its planet's).
//!
//! Compositions are rounded values from the interior literature:
//! - Mercury: core about 70 % of the mass (Hauck et al. 2013,
//!   doi:10.1002/jgre.20091, core radius about 2020 km).
//! - Venus: Earth-like, slightly less iron (Aitta 2012,
//!   *Icarus* 218).
//! - Earth: CMF 0.325 (Zeng, Sasselov & Jacobsen 2016,
//!   doi:10.3847/0004-637X/819/2/127); oceans 0.023 % of the mass.
//! - Mars: core about 24 % (Stähler et al. 2021, doi:10.1126/science.abi7730,
//!   core radius about 1830 km).
//! - Moon: core 1 to 2 % (Weber et al. 2011, doi:10.1126/science.1199375).
//! - Io, Europa, Ganymede, Callisto: Anderson et al. 1996–2001 (Galileo
//!   gravity: Io's Fe–FeS core up to 20 %, Europa's water shell about
//!   8 %, Ganymede and Callisto close to half ice); Schubert et al. 2004
//!   (*Jupiter*, ch. 13).
//! - Titan: about 45 % ice (Iess et al. 2010, doi:10.1126/science.1182583).
//! - Giants: envelope and ice shares of Guillot 2005
//!   (doi:10.1146/annurev.earth.33.092203.122639) and Helled et al. 2011
//!   (doi:10.1088/0004-637X/726/1/15), rounded. Their radii come from
//!   `giants::giant_radius_of` for their composition (see
//!   `interior::interior`).

/// A Solar System body with its composition and its observed radius.
#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceBody {
    pub name: &'static str,
    /// Mass, Earth masses.
    pub mass_earth: f64,
    /// Iron share of the refractory (iron + rock) mass.
    pub core_mass_fraction: f64,
    /// Share of the total mass in water and ices.
    pub water_fraction: f64,
    /// Hydrogen and helium share of the total mass.
    pub envelope_fraction: f64,
    /// Semi-major axis of the orbit around the Sun, AU.
    pub orbit_au: f64,
    /// Observed mean radius, Earth radii.
    pub radius_earth: f64,
}

const fn body(
    name: &'static str,
    mass_earth: f64,
    core_mass_fraction: f64,
    water_fraction: f64,
    envelope_fraction: f64,
    orbit_au: f64,
    radius_earth: f64,
) -> ReferenceBody {
    ReferenceBody {
        name,
        mass_earth,
        core_mass_fraction,
        water_fraction,
        envelope_fraction,
        orbit_au,
        radius_earth,
    }
}

/// The reference bodies (see the module documentation for the sources).
pub const SOLAR_BODIES: &[ReferenceBody] = &[
    body("Mercury", 0.0553, 0.70, 0.0, 0.0, 0.387, 0.383),
    body("Venus", 0.815, 0.31, 0.0, 0.0, 0.723, 0.950),
    body("Earth", 1.0, 0.325, 0.00023, 0.0, 1.0, 1.0),
    body("Mars", 0.107, 0.24, 0.0, 0.0, 1.524, 0.532),
    body("Moon", 0.0123, 0.02, 0.0, 0.0, 1.0, 0.273),
    body("Io", 0.0150, 0.20, 0.0, 0.0, 5.2, 0.286),
    body("Europa", 0.0080, 0.30, 0.08, 0.0, 5.2, 0.245),
    body("Ganymede", 0.0248, 0.30, 0.45, 0.0, 5.2, 0.413),
    body("Callisto", 0.0180, 0.20, 0.50, 0.0, 5.2, 0.378),
    body("Titan", 0.0225, 0.20, 0.45, 0.0, 9.58, 0.404),
    body("Jupiter", 317.8, 0.30, 0.0, 0.95, 5.2, 10.97),
    body("Saturn", 95.16, 0.30, 0.0, 0.85, 9.58, 9.14),
    body("Uranus", 14.54, 0.30, 0.65, 0.15, 19.2, 3.98),
    body("Neptune", 17.15, 0.30, 0.65, 0.15, 30.1, 3.86),
];

/// The reference body called `name`.
///
/// # Panics
///
/// When no reference body has that name.
pub fn reference(name: &str) -> &'static ReferenceBody {
    SOLAR_BODIES
        .iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("no reference body named {name}"))
}

/// The inputs of a reference body: its composition, no volatiles, Earth's
/// radiogenic abundance.
#[cfg(test)]
pub(crate) fn inputs_of(body: &ReferenceBody) -> super::PlanetInputs {
    super::PlanetInputs {
        mass_earth: body.mass_earth,
        core_mass_fraction: body.core_mass_fraction,
        water_fraction: body.water_fraction,
        envelope_fraction: body.envelope_fraction,
        volatiles: super::Volatiles::default(),
        radiogenic: 1.0,
        life_draw: 0.0,
    }
}

/// The inputs of a reference body as the model draws them, without the
/// scatter: its composition, the volatiles of the rule of
/// [`super::inputs::draw_inputs`] for its rock and ice, Earth's radiogenic
/// abundance. Every Solar System test from the atmosphere on uses it.
#[cfg(test)]
pub(crate) fn model_inputs(body: &ReferenceBody) -> super::PlanetInputs {
    let refractory = 1.0 - body.water_fraction - body.envelope_fraction;
    super::PlanetInputs {
        volatiles: super::inputs::volatile_rule(refractory, body.water_fraction),
        ..inputs_of(body)
    }
}

/// Inputs without envelope, volatiles at zero, Earth's radiogenic
/// abundance.
#[cfg(test)]
pub(crate) fn inputs(
    mass_earth: f64,
    core_mass_fraction: f64,
    water_fraction: f64,
) -> super::PlanetInputs {
    super::PlanetInputs {
        mass_earth,
        core_mass_fraction,
        water_fraction,
        envelope_fraction: 0.0,
        volatiles: super::Volatiles::default(),
        radiogenic: 1.0,
        life_draw: 0.0,
    }
}

/// Inputs of `mass_earth` with an `envelope` share of hydrogen and helium,
/// Earth-like rock (CMF 0.325), no water, no volatiles.
#[cfg(test)]
pub(crate) fn with_envelope(mass_earth: f64, envelope: f64) -> super::PlanetInputs {
    super::PlanetInputs {
        envelope_fraction: envelope,
        ..inputs(mass_earth, 0.325, 0.0)
    }
}

/// What the Sun gives a body at `orbit_au`, at 4.6 Gyr: insolation `1/a²`
/// and the equilibrium temperature of [`crate::star::equilibrium_k`].
#[cfg(test)]
pub(crate) fn exposure_at(orbit_au: f64) -> super::Exposure {
    let insolation = 1.0 / (orbit_au * orbit_au);
    super::Exposure {
        insolation,
        equilibrium_k: f64::from(crate::star::equilibrium_k(insolation as f32)),
        age_gyr: 4.6,
    }
}

/// What part 2's chain derives for a body before its climate.
#[cfg(test)]
pub(crate) struct Derived {
    pub context: super::Context,
    pub gases: super::Gases,
    pub water: super::Water,
    pub regime: crate::geology::Regime,
    /// Internal heat flux through the surface, W/m².
    pub heat_flux_w_m2: f64,
    /// What all the water kept would weigh as vapour, bar.
    pub water_bar: f64,
}

/// The Sun's context at `orbit_au`, 4.6 Gyr, with the insolation of
/// exactly `1/a²` and its equilibrium temperature.
#[cfg(test)]
pub(crate) fn sun_at(orbit_au: f64) -> super::Context {
    use crate::star::{StarInfo, parse_type};
    let star = StarInfo::new(parse_type("G2V").unwrap(), orbit_au as f32, 0.0, 4.6);
    let mut c = super::Context::of(&star, 0.0, 2.7);
    let exposure = exposure_at(orbit_au);
    c.insolation = exposure.insolation;
    c.equilibrium_k = exposure.equilibrium_k;
    c.star_mass = 1.0;
    c
}

/// Runs [`super::planet_model`]'s chain on `inputs` at `orbit_au` round
/// the Sun and returns what it had before the climate; no tides
/// (circular orbit). `inputs` must leave no envelope.
#[cfg(test)]
pub(crate) fn derive_at(inputs: &super::PlanetInputs, orbit_au: f64) -> Derived {
    let context = sun_at(orbit_au);
    let (model, air) = super::chain(&context, inputs.clone());
    let air = air.expect("a body with an envelope has no surface");
    let bar_per_kg = super::atmosphere::bar_per_kg(model.inputs.mass_earth, &model.interior);
    Derived {
        heat_flux_w_m2: model.heat.flux_w_m2(model.interior.radius_earth),
        water_bar: air.water.kept_kg * bar_per_kg,
        context,
        gases: air.gases,
        water: air.water,
        regime: air.regime,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_body_is_found_by_name() {
        for b in SOLAR_BODIES {
            assert_eq!(reference(b.name), b);
        }
    }

    #[test]
    #[should_panic(expected = "no reference body named Vulcan")]
    fn an_unknown_body_panics() {
        reference("Vulcan");
    }
}
