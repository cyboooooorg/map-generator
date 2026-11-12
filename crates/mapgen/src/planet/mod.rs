//! The physical planet model (track A, issue #423): what a planet is made
//! of and what follows from it, derived from its mass and composition
//! rather than from a type name (see
//! `docs/superpowers/specs/2026-11-11-physical-planet-model-design.md`).
//!
//! [`PlanetInputs`] holds the drawn quantities, which
//! [`inputs::draw_inputs`] draws from the formation [`inputs::Context`]
//! and the mass; [`interior::solid_interior`]
//! derives the iron, rock and water layers, and [`interior::interior`] adds
//! the hydrogen and helium envelope under the star's [`Exposure`], giving
//! the radius, the bulk density and the surface gravity.
//! [`heat::InternalHeat`] holds the heat it makes inside: radiogenic
//! ([`heat::radiogenic_w`]) and tidal ([`heat::tidal_power_w`], with the
//! response [`heat::k2_over_q`] of a rocky, icy or enveloped body).
//! [`core::core`] derives whether its iron core is liquid and drives a
//! dynamo, and the field at its surface.
//! [`atmosphere::outgassed`], [`atmosphere::atmosphere`] and
//! [`atmosphere::cold_trap`] and [`atmosphere::photochemistry`] derive its air, as [`Gases`], from what it
//! outgasses, what escape leaves and what its cold freezes out.
//! [`climate::climate`] sets its surface temperature through a grey
//! greenhouse and the carbonate thermostat, and the [`SurfaceLiquid`] on
//! its surface.
//! [`life::life`] decides whether life arose where water lasted, and
//! [`life::oxygenate`] gives the air its oxygen.
//! [`classify::classify`] reads its [`PlanetClass`], including the surface
//! classes, off the result, and
//! [`PlanetModel`] holds it all: [`planet_model`] derives it from a
//! context and inputs, [`body_model`] for a planet of a stored system.
//! [`reference::SOLAR_BODIES`] lists the Solar System bodies the relations
//! are checked against.
use serde::{Deserialize, Serialize};

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::exomoon::{EARTH_KG, EARTH_M};
use crate::star::StarInfo;
use crate::system::{Body, BodyKind, System, body_seed};

pub mod atmosphere;
pub mod classify;
pub mod climate;
pub mod core;
pub mod escape;
pub mod heat;
pub mod inputs;
pub mod interior;
pub mod life;
pub mod reference;
pub mod regime;
pub mod xuv;

pub use atmosphere::{
    Gases, atmosphere, cold_trap, escape_velocity_km_s, outgassed, photochemistry,
};
pub use classify::{PlanetClass, classify};
pub use climate::{Climate, SurfaceLiquid, climate, surface_temperature_k};
pub use core::{Core, CoreState, core};
pub use escape::{energy_limited_loss_kg, envelope_after_escape};
pub use heat::{EARTH_SILICATE_KG, InternalHeat, k2_over_q, radiogenic_w, tidal_power_w};
pub use inputs::{Context, draw_inputs};
pub use interior::{GIANT_MIN_ENVELOPE, GIANT_MIN_MASS_EARTH, interior, solid_interior};
pub use life::{Life, life, oxygenate};
pub use reference::{ReferenceBody, SOLAR_BODIES, reference};
pub use regime::{HEAT_PIPE_FLUX_W_M2, OCEAN_KG, Water, regime, water};
pub use xuv::{EARTH_XUV_FLUENCE_J_M2, xuv_fluence};

/// Kilograms of each volatile per kilogram of planet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Volatiles {
    pub co2: f64,
    pub n2: f64,
    pub ch4: f64,
    pub nh3: f64,
    pub s: f64,
}

/// What a planet is made of: its mass and the shares of its components.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanetInputs {
    /// Mass, Earth masses.
    pub mass_earth: f64,
    /// Iron core share of the refractory (iron + rock) mass. Earth 0.325.
    pub core_mass_fraction: f64,
    /// Share of the total mass in water and ices.
    pub water_fraction: f64,
    /// Hydrogen and helium share of the total mass, at formation.
    pub envelope_fraction: f64,
    pub volatiles: Volatiles,
    /// Radiogenic abundance at formation, relative to Earth's (1.0).
    pub radiogenic: f64,
    /// Uniform in `[0, 1)`, drawn last: life arises where it is below
    /// [`life::LIFE_CHANCE`] (and the world allows it).
    pub life_draw: f64,
}

/// What a layer of a planet is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Material {
    Iron,
    Rock,
    Water,
    Envelope,
}

/// One shell of a planet, from the outer radius of the layer below.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub material: Material,
    /// Mass of the layer, Earth masses.
    pub mass_earth: f64,
    /// Outer radius of the layer, Earth radii.
    pub outer_radius_earth: f64,
}

/// What a planet's star gives it, which sets the size of a hydrogen and
/// helium envelope.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exposure {
    /// Light received, Earth's = 1.
    pub insolation: f64,
    /// Equilibrium temperature, K.
    pub equilibrium_k: f64,
    /// Age of the system, Gyr.
    pub age_gyr: f64,
}

/// The layered structure of a planet and its bulk quantities.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interior {
    /// Innermost first; the last layer's outer radius is the radius.
    pub layers: Vec<Layer>,
    /// Radius, Earth radii.
    pub radius_earth: f64,
    /// Bulk density, g/cm³.
    pub density: f64,
    /// Surface gravity, Earth's = 1.
    pub gravity: f64,
}

/// A planet as the model derives it: what it is made of, its layers, the
/// heat it makes and its class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanetModel {
    pub inputs: PlanetInputs,
    pub interior: Interior,
    pub heat: InternalHeat,
    /// XUV energy received over the system's age, Earth's = 1.
    pub xuv_fluence: f64,
    /// Share of the initial envelope that photoevaporation took, 0 to 1.
    pub envelope_lost: f64,
    /// What the body's surface is like; `None` when an envelope is left
    /// (at least [`escape::MIN_ENVELOPE`] of the mass), which hides it.
    pub surface: Option<Surface>,
    pub class: PlanetClass,
}

/// A bare body's surface: the water it kept, how it loses its heat, its
/// core, its air and climate, and its life.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    pub water: Water,
    pub regime: crate::geology::Regime,
    pub core: Core,
    /// The air as the climate and the life leave it (oxygenated if so).
    pub gases: Gases,
    pub climate: Climate,
    pub life: Life,
}

/// What the chain derives before the climate, for the tests that vary the
/// climate afterwards.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Air {
    pub water: Water,
    pub regime: crate::geology::Regime,
    pub gases: Gases,
}

/// Mass of the Sun, kg: the IAU 2015 nominal solar mass parameter
/// (Resolution B3, 1.327 124 4e20 m³/s²) over the G of `crate::exomoon`
/// (6.674e-11).
const SUN_KG: f64 = 1.988_5e30;

/// The astronomical unit, m (IAU 2012, Resolution B2).
const AU_M: f64 = 1.495_978_707e11;

/// The model of a planet of `inputs` in its formation `context`: its
/// [`interior`] under the star's exposure; the XUV fluence of its star
/// ([`xuv_fluence`]) and the envelope that [`envelope_after_escape`] leaves
/// it, from which the interior is derived again; its radiogenic heat from its
/// silicate mass, `M (1 − water − envelope) (1 − CMF)`, its abundance and
/// the system's age; its tidal heat from the star at its orbit, with the
/// [`heat::k2_over_q`] of its envelope and water shares (a giant's is
/// over a thousand times smaller than a rock's); then its [`classify`]
/// class, which feeds back into none of them. A body that keeps an
/// envelope of [`escape::MIN_ENVELOPE`] or more has no surface; any other
/// goes on to its [`water`] and [`regime`], its [`core`], its air
/// ([`outgassed`], [`atmosphere`], [`cold_trap`], [`photochemistry`], in
/// that order: the photochemistry reads the cold trap's air), its
/// [`climate`] and its [`life`] with the oxygen it makes.
pub fn planet_model(context: &Context, inputs: PlanetInputs) -> PlanetModel {
    chain(context, inputs).0
}

/// [`planet_model`] and the air it had before the climate (see [`Air`]).
pub(crate) fn chain(context: &Context, inputs: PlanetInputs) -> (PlanetModel, Option<Air>) {
    let exposure = context.exposure();
    let fluence = xuv_fluence(context);
    let initial_envelope = inputs.mass_earth * inputs.envelope_fraction;
    let drawn = interior(&inputs, &exposure);
    let inputs = envelope_after_escape(&inputs, &drawn, fluence);
    let envelope_lost = if initial_envelope > 0.0 {
        1.0 - inputs.mass_earth * inputs.envelope_fraction / initial_envelope
    } else {
        0.0
    };
    let interior = interior(&inputs, &exposure);
    let silicate_kg = inputs.mass_earth
        * (1.0 - inputs.water_fraction - inputs.envelope_fraction)
        * (1.0 - inputs.core_mass_fraction)
        * EARTH_KG;
    let heat = InternalHeat {
        radiogenic_w: radiogenic_w(silicate_kg, inputs.radiogenic, context.age_gyr),
        tidal_w: tidal_power_w(
            context.star_mass * SUN_KG,
            interior.radius_earth * EARTH_M,
            context.orbit_au * AU_M,
            context.eccentricity,
            k2_over_q(inputs.envelope_fraction, inputs.water_fraction),
        ),
    };
    let mut air = None;
    let surface = if inputs.envelope_fraction >= escape::MIN_ENVELOPE {
        None
    } else {
        let w = water(context, &inputs, &interior, fluence);
        let r = regime(&inputs, &heat, &interior, &w);
        let k = core(&inputs, &interior, r, context.age_gyr);
        let out = outgassed(&inputs, &w, r, &interior);
        let atm = atmosphere(out, &interior, inputs.mass_earth, &k, fluence);
        let gases = photochemistry(cold_trap(atm, context.equilibrium_k), context.equilibrium_k);
        air = Some(Air {
            water: w,
            regime: r,
            gases,
        });
        let flux = heat.flux_w_m2(interior.radius_earth);
        let (gases, climate) = self::climate(context, gases, &w, r, flux);
        let life = life(&inputs, &climate, context.age_gyr);
        Some(Surface {
            water: w,
            regime: r,
            core: k,
            gases: oxygenate(gases, &life),
            climate,
            life,
        })
    };
    let class = classify(&inputs, &interior, context.period_days(), surface.as_ref());
    let model = PlanetModel {
        inputs,
        interior,
        heat,
        xuv_fluence: fluence,
        envelope_lost,
        surface,
        class,
    };
    (model, air)
}

/// The model of a planet of a stored system; `None` for a moon (part 4).
///
/// Its context is its own star seen from its orbit (its params' star,
/// orbit, eccentricity and age), its system's metallicity and ice line;
/// its mass the stored one. Its inputs are drawn from their own stream,
/// seeded by the system's seed and `<id>/model`, so they depend on no
/// other body and shift no other draw.
pub fn body_model(system: &System, body: &Body) -> Option<PlanetModel> {
    if body.kind != BodyKind::Planet {
        return None;
    }
    let p = &body.params;
    let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
    let context = Context::of(&star, p.metallicity, system.star.ice_line_au);
    let seed = body_seed(system.seed, &format!("{}/model", body.id));
    let mut rng = StdRng::seed_from_u64(u64::from(seed));
    let inputs = draw_inputs(&context, body.physical.mass_earth, &mut rng);
    Some(planet_model(&context, inputs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::{SystemRequest, WholeSystemRequest, draw};

    /// The whole system seed 2 draws round a Sun-like star.
    fn drawn() -> System {
        let request = SystemRequest::System(WholeSystemRequest {
            age_gyr: None,
            star: Some("G2V".into()),
            metallicity: Some(0.0),
            planets: None,
            seed: Some(2),
            width: Some(64),
            height: Some(32),
        });
        draw(&request, &mut StdRng::seed_from_u64(2)).unwrap()
    }

    #[test]
    fn body_model_is_deterministic_and_independent() {
        let system = drawn();
        let planets: Vec<&Body> = system
            .bodies
            .iter()
            .filter(|b| b.kind == BodyKind::Planet)
            .collect();
        assert!(planets.len() >= 2, "{} planets", planets.len());
        for body in &planets {
            let model = body_model(&system, body).expect("a planet has a model");
            assert_eq!(
                body_model(&system, body),
                Some(model.clone()),
                "{}",
                body.id
            );
            // Photoevaporation can only lower the mass, by the envelope lost.
            let lost = body.physical.mass_earth - model.inputs.mass_earth;
            assert!(lost >= 0.0, "{}", body.id);
            assert!(lost <= 1.0e-3 + model.envelope_lost * body.physical.mass_earth);
        }
        // Removing another planet leaves the model of b as it was.
        let b = planets[0];
        let mut fewer = system.clone();
        let other = planets[1].id.clone();
        fewer.bodies.retain(|x| x.id != other);
        assert_eq!(body_model(&fewer, b), body_model(&system, b));
    }

    /// The context of a body round the Sun at `orbit_au` on an orbit of
    /// `eccentricity`, at 4.6 Gyr.
    fn sun_at(orbit_au: f32, eccentricity: f32) -> Context {
        let sun = crate::star::parse_type("G2V").unwrap();
        let star = StarInfo::new(sun, orbit_au, eccentricity, crate::star::DEFAULT_AGE_GYR);
        Context::of(&star, 0.0, 2.7)
    }

    /// Starlight a body of `radius_earth` takes in, W, black (albedo 0):
    /// its cross-section times the mean flux, 1361 W/m² per unit of
    /// insolation.
    fn absorbed_w(context: &Context, radius_earth: f64) -> f64 {
        let r = radius_earth * EARTH_M;
        std::f64::consts::PI * r * r * 1361.0 * context.insolation
    }

    #[test]
    fn a_close_eccentric_hot_jupiter_makes_far_less_tidal_heat_than_starlight() {
        // A Jupiter (318 M⊕, 95 % hydrogen and helium) at 0.04 AU from
        // the Sun on an orbit of eccentricity 0.1. With Jupiter's k₂/Q
        // its tides make about 4 % of the starlight it would absorb black;
        // with Io's, which every body took before issue #433, they made
        // 60 times that starlight.
        let context = sun_at(0.04, 0.1);
        let model = planet_model(&context, reference::with_envelope(318.0, 0.95));
        assert_eq!(model.class, PlanetClass::HotJupiter);
        let ratio = model.heat.tidal_w / absorbed_w(&context, model.interior.radius_earth);
        assert!(ratio < 0.1, "tidal / absorbed starlight {ratio}");
    }

    #[test]
    fn a_rocky_planet_keeps_io_tidal_response() {
        let context = sun_at(0.05, 0.2);
        let model = planet_model(&context, reference::inputs(1.0, 0.325, 0.0));
        let io = tidal_power_w(
            context.star_mass * SUN_KG,
            model.interior.radius_earth * EARTH_M,
            context.orbit_au * AU_M,
            context.eccentricity,
            0.015,
        );
        assert_eq!(model.heat.tidal_w, io);
    }

    #[test]
    fn moons_have_no_model_yet() {
        let system = drawn();
        let mut moon = system.bodies[0].clone();
        moon.kind = BodyKind::Moon;
        assert_eq!(body_model(&system, &moon), None);
        for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
            assert_eq!(body_model(&system, body), None, "{}", body.id);
        }
    }
}
