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
//! [`atmosphere::outgassed`], [`atmosphere::atmosphere`],
//! [`atmosphere::methane_photolysis`], [`atmosphere::cold_trap`] and
//! [`atmosphere::photochemistry`] derive its air, as [`Gases`], from what
//! it outgasses, what escape leaves, what its star's ultraviolet destroys
//! and what its cold freezes out.
//! [`climate::climate`] sets its surface temperature through a grey
//! greenhouse and the carbonate thermostat, and the [`SurfaceLiquid`] on
//! its surface.
//! [`life::life`] decides whether life arose where water lasted, and
//! [`life::oxygenate`] gives the air its oxygen.
//! [`classify::classify`] reads its [`PlanetClass`], including the surface
//! classes, off the result, and
//! [`PlanetModel`] holds it all: [`planet_model`] derives it from a
//! context and inputs, [`derive_model`] for a planet of a system as it is
//! drawn, which stores it on the body ([`body_model`] reads it back).
//! [`interim::base_of`] maps a model onto today's looks until #424, and
//! [`interim::params_for`] generates its world from what it derived.
//! [`reference::SOLAR_BODIES`] lists the Solar System bodies the relations
//! are checked against.
use serde::{Deserialize, Serialize};

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::exomoon::{EARTH_KG, EARTH_M};
use crate::system::{Body, System, body_seed};

pub mod atmosphere;
pub mod classify;
pub mod climate;
pub mod core;
pub mod escape;
pub mod heat;
pub mod inputs;
pub mod interim;
pub mod interior;
pub mod life;
pub mod mass;
pub mod reference;
pub mod regime;
pub mod xuv;

pub use atmosphere::{
    Gases, atmosphere, cold_trap, escape_velocity_km_s, methane_photolysis, outgassed,
    photochemistry,
};
pub use classify::{PLANET_CLASSES, PlanetClass, classify};
pub use climate::{Climate, SurfaceLiquid, climate, surface_temperature_k};
pub use core::{Core, CoreState, core};
pub use escape::{energy_limited_loss_kg, envelope_after_escape};
pub use heat::{EARTH_SILICATE_KG, InternalHeat, k2_over_q, radiogenic_w, tidal_power_w};
pub use inputs::{Context, InputOverrides, draw_inputs, draw_inputs_with};
pub use interior::{GIANT_MIN_ENVELOPE, GIANT_MIN_MASS_EARTH, interior, solid_interior};
pub use life::{Life, life, oxygenate};
pub use mass::{Origin, SMALL_MASS, draw_mass};
pub use reference::{ReferenceBody, SOLAR_BODIES, reference};
pub use regime::{HEAT_PIPE_FLUX_W_M2, OCEAN_KG, Water, regime, steam, water};
pub use xuv::{EARTH_XUV_FLUENCE_J_M2, xuv_fluence};

/// Kilograms of each volatile per kilogram of planet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Volatiles {
    pub co2: f64,
    pub n2: f64,
    pub ch4: f64,
    pub nh3: f64,
    pub s: f64,
}

/// What a planet is made of: its mass and the shares of its components.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Material {
    Iron,
    Rock,
    Water,
    Envelope,
}

/// One shell of a planet, from the outer radius of the layer below.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
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

impl Exposure {
    /// What `star` gives a planet on its orbit.
    pub fn of(star: &crate::star::StarInfo) -> Self {
        Self {
            insolation: f64::from(star.insolation),
            equilibrium_k: f64::from(star.equilibrium_k),
            age_gyr: f64::from(star.age_gyr),
        }
    }
}

/// The layered structure of a planet and its bulk quantities.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PlanetModel {
    /// Mass drawn for the planet, Earth masses, before photoevaporation
    /// (`inputs.mass_earth` is what escape leaves): with the planet's
    /// context and id, what [`derive_model`] derives the model from again.
    pub drawn_mass_earth: f64,
    /// What it is made of, after photoevaporation.
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

/// The inputs a planet formed with, as a request sets them: what
/// [`PlanetModel::formation`] gives, and world.json stores beside the
/// model so that a request (a new seed, another epoch, a variation, the
/// CLI's "Reproduce with") makes the same planet again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Formation {
    /// Mass drawn, Earth masses, before photoevaporation.
    pub mass_earth: f64,
    /// Every input set: core share, water in Earth oceans (of the drawn
    /// mass), envelope share at formation, CO₂ and N₂ factors on their
    /// formation rule, radiogenic abundance, and whether life arose.
    pub overrides: InputOverrides,
}

/// Most steps of one ulp [`exact_preimage`] tries on each side.
const PREIMAGE_STEPS: usize = 8;

/// A value `x` near `start` that `f` maps back exactly onto `target`
/// (stepping one ulp at a time on either side), else `start`: a value read
/// back from the model by division, which the request path multiplies
/// again, then gives the stored value to the last bit.
fn exact_preimage(target: f64, start: f64, f: impl Fn(f64) -> f64) -> f64 {
    if !start.is_finite() || f(start) == target {
        return start;
    }
    let (mut up, mut down) = (start, start);
    for _ in 0..PREIMAGE_STEPS {
        up = up.next_up();
        down = down.next_down();
        if f(up) == target {
            return up;
        }
        if f(down) == target {
            return down;
        }
    }
    start
}

impl PlanetModel {
    /// The inputs this planet formed with, as a request sets them
    /// ([`Formation`]): the drawn mass (before photoevaporation); the
    /// envelope share at formation, `(M e + M₀ − M) / M₀` with `M₀` the
    /// drawn mass and `M`, `e` what escape left; the water in Earth oceans
    /// of the drawn mass; the CO₂ and N₂ factors, the stored volatiles over
    /// the formation rule of the formation shares (1 where the rule is 0);
    /// the core share, the radiogenic abundance and `life: Some(arose)`.
    ///
    /// Set in a request with the same context, they derive the same inputs
    /// and class whatever the seed; the CH₄, NH₃ and sulphur scatter, which
    /// no request sets, follows the seed. Values read back by division are
    /// nudged by an ulp or two so that the request's arithmetic gives the
    /// stored value exactly.
    pub fn formation(&self) -> Formation {
        let i = &self.inputs;
        let drawn = self.drawn_mass_earth;
        let envelope = if i.mass_earth == drawn {
            i.envelope_fraction
        } else {
            (i.mass_earth * i.envelope_fraction + drawn - i.mass_earth) / drawn
        };
        let per_ocean = OCEAN_KG / (drawn * EARTH_KG);
        let water_oceans = exact_preimage(i.water_fraction, i.water_fraction / per_ocean, |o| {
            o * OCEAN_KG / (drawn * EARTH_KG)
        });
        let refractory = (1.0 - i.water_fraction - envelope).max(0.0);
        let rule = inputs::volatile_rule(refractory, i.water_fraction);
        let factor = |stored: f64, rule: f64| {
            if rule > 0.0 {
                exact_preimage(stored, stored / rule, |f| rule * f)
            } else {
                1.0
            }
        };
        Formation {
            mass_earth: drawn,
            overrides: InputOverrides {
                core_mass_fraction: Some(i.core_mass_fraction),
                water_oceans: Some(water_oceans),
                envelope_fraction: Some(envelope),
                co2: Some(factor(i.volatiles.co2, rule.co2)),
                n2: Some(factor(i.volatiles.n2, rule.n2)),
                radiogenic: Some(i.radiogenic),
                life: Some(self.surface.is_some_and(|s| s.life.arose)),
            },
        }
    }
}

/// A bare body's surface: the water it kept, how it loses its heat, its
/// core, its air and climate, and its life.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
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
/// ([`outgassed`], [`atmosphere`], [`methane_photolysis`], [`cold_trap`],
/// [`photochemistry`], in that order: the photochemistry reads the cold
/// trap's air), its
/// [`climate`] and its [`life`] with the oxygen it makes. A body whose
/// climate carries its water past the critical point
/// ([`climate::boils`]) has run away: it goes through those steps again
/// from its [`steam`], flagged `boiled`, and keeps that second pass only
/// if its steam holds the surface past the critical point too
/// ([`climate::past_critical`]). In a runaway the water kept is what the
/// escape of [`atmosphere`] leaves of the steam in the air, so the
/// [`Water`] and the air's H₂O agree.
pub fn planet_model(context: &Context, inputs: PlanetInputs) -> PlanetModel {
    chain(context, inputs).0
}

/// [`planet_model`] and the air it had before the climate (see [`Air`]).
pub(crate) fn chain(context: &Context, inputs: PlanetInputs) -> (PlanetModel, Option<Air>) {
    let exposure = context.exposure();
    let fluence = xuv_fluence(context);
    let drawn_mass_earth = inputs.mass_earth;
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
        let bar_per_kg = atmosphere::bar_per_kg(inputs.mass_earth, &interior);
        let bare = |w: Water| {
            let r = regime(&inputs, &heat, &interior, &w);
            let k = core(&inputs, &interior, r, context.age_gyr);
            let out = outgassed(&inputs, &w, r, &interior);
            let atm = atmosphere(out, &interior, inputs.mass_earth, &k, fluence);
            // A runaway's water is the steam escape has left in its air.
            let w = if w.runaway {
                w.keeping(atm.h2o / bar_per_kg)
            } else {
                w
            };
            let atm = methane_photolysis(atm, context, inputs.mass_earth, &interior);
            let gases =
                photochemistry(cold_trap(atm, context.equilibrium_k), context.equilibrium_k);
            let flux = heat.flux_w_m2(interior.radius_earth);
            let water_bar = w.kept_kg * bar_per_kg;
            let (after, climate) = self::climate(context, gases, &w, r, flux, water_bar);
            let life = life(&inputs, &climate, context.age_gyr);
            let surface = Surface {
                water: w,
                regime: r,
                core: k,
                gases: oxygenate(after, &life),
                climate,
                life,
            };
            let before = Air {
                water: w,
                regime: r,
                gases,
            };
            (surface, before)
        };
        let w = water(context, &inputs, &interior, fluence);
        let (mut surface, mut before) = bare(w);
        if climate::boils(&w, &surface.climate) {
            let boiled = Water {
                boiled: true,
                ..steam(&inputs, &interior, fluence)
            };
            let (hot, air) = bare(boiled);
            // Kept only if the steam holds the surface past the critical
            // point itself: a boil whose steam escape leaves too little
            // to stay hot has no consistent runaway, and the first pass,
            // wet and bounded by its own water, stands.
            if climate::past_critical(&hot.climate) {
                (surface, before) = (hot, air);
            }
        }
        air = Some(before);
        Some(surface)
    };
    let class = classify(&inputs, &interior, context.period_days(), surface.as_ref());
    let model = PlanetModel {
        drawn_mass_earth,
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

/// The model of the planet `id` of the system of `system_seed`, in its
/// formation `context`, drawn `mass_earth` Earth masses (before escape).
///
/// Its context is its own star seen from its orbit (its star, orbit,
/// eccentricity and age), its system's metallicity and ice line. Its
/// inputs are drawn from their own stream, seeded by the system's seed and
/// `<id>/model`, so they depend on no other body and shift no other draw.
pub fn derive_model(system_seed: u32, id: &str, context: &Context, mass_earth: f64) -> PlanetModel {
    derive_model_with(
        system_seed,
        id,
        context,
        mass_earth,
        &InputOverrides::default(),
    )
}

/// [`derive_model`] with some inputs set instead of drawn (`overrides`,
/// see [`draw_inputs_with`]); the draws are the same.
pub fn derive_model_with(
    system_seed: u32,
    id: &str,
    context: &Context,
    mass_earth: f64,
    overrides: &InputOverrides,
) -> PlanetModel {
    let seed = body_seed(system_seed, &format!("{id}/model"));
    let mut rng = StdRng::seed_from_u64(u64::from(seed));
    let inputs = draw_inputs_with(context, mass_earth, overrides, &mut rng);
    planet_model(context, inputs)
}

/// The model a stored system holds for `body`: the one [`derive_model`]
/// gave its planet when the system was drawn, or the one the resolution of
/// its request gave the world a system was drawn around (a giant's moon
/// among them); `None` for any other moon (part 4).
pub fn body_model(_system: &System, body: &Body) -> Option<PlanetModel> {
    body.model.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::star::StarInfo;
    use crate::system::{BodyKind, SystemRequest, WholeSystemRequest, draw};

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
    fn derive_model_is_deterministic_and_independent() {
        let system = drawn();
        let planets: Vec<&Body> = system
            .bodies
            .iter()
            .filter(|b| b.kind == BodyKind::Planet)
            .collect();
        assert!(planets.len() >= 2, "{} planets", planets.len());
        for body in &planets {
            let model = body_model(&system, body).expect("a planet has a model");
            let p = &body.params;
            let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
            let context = Context::of(&star, p.metallicity, system.star.ice_line_au);
            // From values alone: the same model again, whatever the other
            // bodies of the system.
            assert_eq!(
                derive_model(system.seed, &body.id, &context, model.drawn_mass_earth),
                model,
                "{}",
                body.id
            );
            // Photoevaporation can only lower the mass, by the envelope lost.
            let lost = model.drawn_mass_earth - model.inputs.mass_earth;
            assert!(lost >= 0.0, "{}", body.id);
            assert!(lost <= 1.0e-3 + model.envelope_lost * model.drawn_mass_earth);
        }
        // Another id draws other inputs.
        let b = planets[0];
        let model = b.model.clone().unwrap();
        let p = &b.params;
        let star = StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr);
        let context = Context::of(&star, p.metallicity, system.star.ice_line_au);
        let other = derive_model(system.seed, "z", &context, model.drawn_mass_earth);
        assert_ne!(other.inputs, model.inputs);
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
        for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
            assert_eq!(body_model(&system, body), None, "{}", body.id);
        }
    }
}
