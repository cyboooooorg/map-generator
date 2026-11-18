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
//! the radius, the bulk density and the surface gravity;
//! [`differentiation::partly_differentiated`] leaves an icy moon whose ice
//! did not all separate from its rock a mixed rock-and-ice layer, and
//! [`differentiation::moment_of_inertia`] reads its moment of inertia off
//! the layers.
//! [`heat::InternalHeat`] holds the heat it makes inside: radiogenic
//! ([`heat::radiogenic_w`]), tidal ([`heat::tidal_power_w`], with the
//! response [`heat::k2_over_q`] of a rocky, icy or enveloped body) and
//! the secular cooling of its mantle and core ([`heat::secular_w`]).
//! [`core::core`] derives whether its iron core is liquid and drives a
//! dynamo, and the field at its surface.
//! [`atmosphere::outgassed`], [`atmosphere::atmosphere`],
//! [`atmosphere::methane_photolysis`], [`atmosphere::cold_trap`] and
//! [`atmosphere::photochemistry`] derive its air, as [`Gases`], from what
//! it outgasses, what escape leaves, what its star's ultraviolet destroys
//! and what its cold freezes out.
//! [`climate::climate`] sets its surface temperature through a grey
//! radiative–convective column (`column`) and the carbonate thermostat,
//! under the Bond albedo [`albedo::bond_albedo`] derives from what covers
//! it ([`albedo::cover`]), the two solved together, and the
//! [`SurfaceLiquid`] on its surface. Where the ice–albedo feedback gives
//! it both a temperate and a frozen state, its snowball draw picks one
//! ([`climate::snowball_chance`]); a cold planet's CO₂ builds up from its
//! volcanoes up to its carbon inventory.
//! [`ice_shell::ice_shell`] gives a frozen body with a water layer the
//! ice shell its internal heat sustains, conductive or convecting, thinned
//! at the poles by the tides, and the ocean beneath, its melting point
//! lowered by the ammonia its ice carries.
//! [`life::life`] decides whether life arose where water lasted, and
//! [`life::oxygenate`] gives the air its oxygen, and
//! [`atmosphere::wash_out`] takes the SO₂ out of a wet or oxidising air.
//! [`classify::classify`] reads its [`PlanetClass`], including the surface
//! classes, off the result, and
//! [`PlanetModel`] holds it all: [`planet_model`] derives it from a
//! context and inputs, [`derive_model`] for a planet of a system as it is
//! drawn, which stores it on the body ([`body_model`] reads it back).
//! [`interim::base_of`] maps a model onto today's looks until #424, and
//! [`interim::params_for`] generates its world from what it derived.
//! [`explain::explain`] unfolds what it derived into cause trees for a
//! body's card.
//! [`reference::SOLAR_BODIES`] lists the Solar System bodies the relations
//! are checked against.
use serde::{Deserialize, Serialize};

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::exomoon::{EARTH_KG, EARTH_M};
use crate::system::{Body, System, body_seed};

pub mod albedo;
pub mod atmosphere;
pub mod classify;
pub mod climate;
pub(crate) mod column;
pub mod core;
pub mod differentiation;
pub mod escape;
pub mod explain;
pub mod geysers;
pub mod heat;
pub mod ice_shell;
pub mod inputs;
pub mod interim;
pub mod interior;
pub mod life;
pub mod mass;
pub mod moon;
pub mod reference;
pub mod regime;
pub mod resonance;
pub mod surface_age;
pub mod xuv;

pub use atmosphere::{
    EXOSPHERE_BAR, Gases, HostPlasma, LossContext, atmosphere, cold_trap, escape_velocity_km_s,
    methane_photolysis, non_thermal_loss, outgassed, photochemistry, wash_out,
};
pub use classify::{PLANET_CLASSES, PlanetClass, classify};
pub use climate::{
    CarbonCycle, Climate, SurfaceLiquid, climate, methane_freezes_k, methane_sea_vapour_bar,
    snowball_chance, surface_temperature_k,
};
pub use core::{Core, CoreState, core};
pub use escape::{energy_limited_loss_kg, envelope_after_escape};
pub use explain::{Cause, Explanation, InputField, Quantity, explain};
pub use geysers::{Fracture, Geysers, geysers};
pub use heat::{
    EARTH_SILICATE_KG, InternalHeat, k2_over_q, radiogenic_w, secular_w, tidal_power_w,
};
pub use ice_shell::{IceShell, ice_shell};
pub use inputs::{Context, HostTide, InputOverrides, draw_inputs, draw_inputs_with};
pub use interior::{
    GIANT_MIN_ENVELOPE, GIANT_MIN_MASS_EARTH, interior, solid_interior, steam_interior,
};
pub use life::{Life, life, oxygenate};
pub use mass::{Origin, SMALL_MASS, draw_mass};
pub use moon::{MoonBirth, MoonFormation, derive_moon_model};
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
    /// Uniform in `[0, 1)`, drawn just before `snowball_draw`: life
    /// arises where it is below [`life::LIFE_CHANCE`] (and the world
    /// allows it).
    pub life_draw: f64,
    /// Uniform in `[0, 1)`, drawn last, after `life_draw`: a planet whose
    /// climate has both a temperate and a frozen state is frozen where it
    /// is below [`climate::snowball_chance`] (see [`climate::climate`]). 1
    /// keeps the temperate state, 0 takes the frozen one wherever it
    /// exists.
    pub snowball_draw: f64,
}

/// What a layer of a planet is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Material {
    Iron,
    Rock,
    /// Rock and ice still mixed, under the water of a moon whose ice did
    /// not all separate from its rock ([`differentiation`]).
    RockIce,
    Water,
    Envelope,
    /// Water above its critical point over the rock of a steam world: a
    /// supercritical envelope with no liquid–gas boundary
    /// ([`climate::hides_surface`], [`interior::steam_interior`]).
    Steam,
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
    /// The ice shell over its water layer and the ocean beneath
    /// ([`ice_shell::ice_shell`]); `None` without water or a frozen
    /// surface.
    #[serde(default)]
    pub ice_shell: Option<IceShell>,
    /// The geysers its ice shell vents through, if it cracks through at a
    /// pole ([`geysers::geysers`]), drawn by [`derive_model`] and
    /// [`moon::derive_moon_model`] from the body's seed; `None` from
    /// [`planet_model`] alone, which has no seed, and in a model stored
    /// before they existed.
    #[serde(default)]
    pub geysers: Option<Geysers>,
    /// Moment of inertia factor `C / (M R²)` of its layers
    /// ([`differentiation::moment_of_inertia`]); 0 in a model stored
    /// before it existed.
    #[serde(default)]
    pub moment_of_inertia: f64,
    /// The parts of its surface and their ages ([`surface_age::terrain`]);
    /// empty in a model stored before it existed.
    #[serde(default)]
    pub terrain: surface_age::Terrain,
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
    /// of the drawn mass; the CO₂, N₂, CH₄, NH₃ and sulphur factors, the stored volatiles over
    /// the formation rule of the formation shares (1 where the rule is 0);
    /// the core share, the radiogenic abundance, `life: Some(arose)` and
    /// `snowball: Some(frozen)` (a pinned snowball replays the CO₂ it
    /// froze with, not the amount its draw had built up).
    ///
    /// Set in a request with the same context, they derive the same inputs
    /// and class whatever the seed: the scatter of every volatile is
    /// pinned with them. Values read back by division are
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
        let rule = inputs::volatile_rule(refractory, i.water_fraction, i.core_mass_fraction);
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
                ch4: Some(factor(i.volatiles.ch4, rule.ch4)),
                nh3: Some(factor(i.volatiles.nh3, rule.nh3)),
                s: Some(factor(i.volatiles.s, rule.s)),
                radiogenic: Some(i.radiogenic),
                life: Some(self.surface.is_some_and(|s| s.life.arose)),
                // The state, not the draw: a pinned snowball replays its
                // CO₂ from where it froze (the draw at 0), not from where
                // its draw caught the build-up.
                snowball: Some(self.surface.is_some_and(|s| s.climate.snowball)),
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
    /// CO₂ outgassed before escape, bar: the carbon inventory of the
    /// climate ([`climate::CarbonCycle`]).
    pub carbon_bar: f64,
    /// CO₂ the cold trap froze onto the ground, bar
    /// ([`climate::CarbonCycle::frozen_bar`]).
    pub frozen_co2_bar: f64,
    /// SO₂ frost the volcanoes of a heat pipe lay on the ground, bar
    /// ([`climate::CarbonCycle::so2_frost_bar`]).
    pub so2_frost_bar: f64,
}

/// Mass of the Sun, kg: the IAU 2015 nominal solar mass parameter
/// (Resolution B3, 1.327 124 4e20 m³/s²) over the G of `crate::exomoon`
/// (6.674e-11).
const SUN_KG: f64 = 1.988_5e30;

/// The astronomical unit, m (IAU 2012, Resolution B2).
const AU_M: f64 = 1.495_978_707e11;

/// The model of a planet of `inputs` in its formation `context`: its
/// [`interior`] under the star's exposure (a moon's partly
/// differentiated, see [`body_interior`]); the XUV fluence of its star
/// ([`xuv_fluence`]) and the envelope that [`envelope_after_escape`] leaves
/// it, from which the interior is derived again; its radiogenic heat from its
/// silicate mass, `M (1 − water − envelope) (1 − CMF)`, its abundance and
/// the system's age; its tidal heat from the host of `context.tide` on
/// the body's orbit around it (a moon's planet), else from the star at its
/// orbit, with the
/// [`heat::k2_over_q`] of its envelope and water shares (a giant's is
/// over a thousand times smaller than a rock's); then its [`classify`]
/// class, which feeds back into none of them. A body that keeps an
/// envelope of [`escape::MIN_ENVELOPE`] or more has no surface; any other
/// goes on to its [`water`] and [`regime`], its [`core`], its air
/// ([`outgassed`], [`atmosphere`] (a giant's moon unshielded by its own
/// dynamo, see `shield`), [`methane_photolysis`], [`cold_trap`],
/// [`photochemistry`], in that order: the photochemistry reads the cold
/// trap's air), its
/// [`climate`] and its [`life`] with the oxygen it makes. A wet body
/// with no wet balance ([`climate::boils`]: its absorbed light and heat
/// exceed the moist limit) has run away, in one pass with no loop: it
/// goes through those steps again from its [`steam`], flagged `boiled`,
/// whose energy-limited escape leaves it some water or none; if that
/// leftover has a wet balance of its own, it condenses again and a third
/// run with it as an ocean (`boiled`, no runaway) is kept, else the
/// steam run is (Kasting 1988, doi:10.1016/0019-1035(88)90116-9, not
/// checked, for the moist runaway; there is no temperature condition).
/// In a runaway the water kept is what the escape of [`atmosphere`]
/// leaves of the steam in the air, so the
/// [`Water`] and the air's H₂O agree. Last, its [`ice_shell`] from its
/// water layer, its internal heat, its surface's climate and the ammonia
/// per kilogram of its ice (its `nh3` over its water share), and the
/// moment of inertia of its layers.
///
/// It leaves `geysers` at `None`: their pole is drawn from the body's
/// seed, which only [`derive_model`], [`derive_model_with`] and
/// [`moon::derive_moon_model`] know ([`vent`]); a model that needs its
/// geysers goes through those.
pub fn planet_model(context: &Context, inputs: PlanetInputs) -> PlanetModel {
    chain(context, inputs).0
}

/// Whether the body orbits a giant (a host of [`GIANT_MIN_MASS_EARTH`] or
/// more, `context.tide`).
pub fn moon_of_giant(context: &Context) -> bool {
    let giant_kg = GIANT_MIN_MASS_EARTH * EARTH_KG;
    context.tide.is_some_and(|t| t.host_kg >= giant_kg)
}

/// The core whose field shields a body's air from escape: its own, unless
/// it is a moon of a giant (a host of [`GIANT_MIN_MASS_EARTH`] or more,
/// `context.tide`), which orbits inside its giant's magnetosphere: the
/// giant's corotating plasma reaches its upper atmosphere and sputters it
/// whatever its own dynamo (Io, Europa and Ganymede inside Jupiter's,
/// Titan at the edge of Saturn's; Kivelson et al. 2004, "Magnetospheric
/// interactions with satellites", in *Jupiter: The Planet, Satellites
/// and Magnetosphere*, ch. 21, not checked), so it meets the unshielded
/// escape of [`atmosphere`]. Calibration, no source, for taking every
/// giant's magnetosphere as enclosing its moons, and for the giant mass:
/// it keeps Jupiter's moons airless (Io, to which the core model gives a
/// dynamo it does not have, would keep about 0.01 bar of N₂) and leaves
/// the Moon, whose host is the Earth, as a planet.
fn shield(context: &Context, core: Core) -> Core {
    if moon_of_giant(context) {
        Core {
            dynamo: false,
            ..core
        }
    } else {
        core
    }
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
    let interior = body_interior(context, &inputs);
    let heat = internal_heat(context, &inputs, &interior);
    let mut air = None;
    let surface = if inputs.envelope_fraction >= escape::MIN_ENVELOPE {
        None
    } else {
        let w = water(context, &inputs, &interior, fluence);
        let bar_per_kg = atmosphere::bar_per_kg(inputs.mass_earth, &interior);
        let flux = heat.flux_w_m2(interior.radius_earth);
        // A thermostat that reached its temperature found a wet balance:
        // only the others need the moist-limit test, under the Bond albedo
        // their climate settled on.
        let boils = |steps: &Bare| {
            let water = &steps.surface.water;
            !steps.surface.climate.regulated
                && climate::boils(
                    &context.with_albedo(steps.surface.climate.bond_albedo),
                    &steps.climate_gases,
                    water,
                    flux,
                    water.kept_kg * bar_per_kg,
                )
        };
        let mut steps = bare(context, &inputs, &interior, &heat, fluence, w);
        if boils(&steps) {
            let boiled = Water {
                boiled: true,
                ..steam(&inputs, &interior, fluence)
            };
            steps = bare(context, &inputs, &interior, &heat, fluence, boiled);
            let left = steps.surface.water;
            if left.kept_kg > 0.0 {
                // The water escape left, condensed again: kept if it has a
                // wet balance of its own, else the steam stands.
                let condensed = Water {
                    runaway: false,
                    ..left
                };
                let wet = bare(context, &inputs, &interior, &heat, fluence, condensed);
                if !boils(&wet) {
                    steps = wet;
                }
            }
        }
        air = Some(steps.before);
        Some(steps.surface)
    };
    // A deep supercritical steam column hides the surface and inflates
    // the planet (#492): its interior is derived again with the steam on
    // top, once, without re-running the climate under the new gravity.
    let interior = match &surface {
        Some(s) if climate::hides_surface(s) => {
            steam_interior(&interior, inputs.mass_earth, s.water.kept_kg, &exposure)
        }
        _ => interior,
    };
    let class = classify(&inputs, &interior, context.period_days(), surface.as_ref());
    let nh3_ice = if inputs.water_fraction > 0.0 {
        inputs.volatiles.nh3 / inputs.water_fraction
    } else {
        0.0
    };
    let ice_shell = ice_shell(&interior, &heat, surface.as_ref(), nh3_ice);
    let moment_of_inertia = differentiation::moment_of_inertia(&interior);
    let mut model = PlanetModel {
        drawn_mass_earth,
        inputs,
        interior,
        heat,
        xuv_fluence: fluence,
        envelope_lost,
        surface,
        class,
        ice_shell,
        geysers: None,
        moment_of_inertia,
        terrain: surface_age::Terrain::default(),
    };
    // The frost the climate left on the ground at its temperature.
    let frost = match (&air, &model.surface) {
        (Some(air), Some(surface)) => climate::frost(
            &air.gases,
            0.0,
            0.0,
            surface.climate.temperature_k,
            &surface.gases,
        ),
        _ => Gases::default(),
    };
    model.terrain = surface_age::terrain(&model, context.age_gyr, &frost, context.tide.as_ref());
    (model, air)
}

/// The layers of a body of `inputs` (after escape) in its formation
/// `context`: its [`interior`] under the star's exposure, with only the
/// [`differentiation::share_in`] of its ice separated from its rock
/// ([`differentiation::partly_differentiated`]; all of it for a planet),
/// and for a moon (the tides of a host other than its star in `context`)
/// the core and water of a moon ([`differentiation::moon_core`]).
pub fn body_interior(context: &Context, inputs: &PlanetInputs) -> Interior {
    let full = interior(inputs, &context.exposure());
    let share = differentiation::share_in(context, inputs, &full);
    let layered = differentiation::partly_differentiated(&full, share);
    let moon = context
        .tide
        .is_some_and(|t| t.host_kg < context.star_mass * SUN_KG);
    if moon {
        differentiation::moon_core(&layered)
    } else {
        layered
    }
}

/// The host whose tides heat a body of `context`: a moon's planet
/// (`context.tide`), else its star on the body's orbit.
pub(crate) fn tide_of(context: &Context) -> HostTide {
    context.tide.unwrap_or(HostTide {
        host_kg: context.star_mass * SUN_KG,
        a_m: context.orbit_au * AU_M,
        eccentricity: context.eccentricity,
        resonance: None,
    })
}

/// The heat a body of `inputs` (after escape) and `interior` makes in
/// `context` (see [`planet_model`]): radiogenic and secular cooling from
/// its silicate mass and age, tidal from [`tide_of`]: its own tide with
/// the [`heat::k2_over_q`] of its envelope and water shares, or the
/// resonant heat of a moon in a chain ([`HostTide::resonance`]) when
/// that is larger ([`resonance::todays_heat_w`]).
pub(crate) fn internal_heat(
    context: &Context,
    inputs: &PlanetInputs,
    interior: &Interior,
) -> InternalHeat {
    let silicate_kg = inputs.mass_earth
        * (1.0 - inputs.water_fraction - inputs.envelope_fraction)
        * (1.0 - inputs.core_mass_fraction)
        * EARTH_KG;
    let tide = tide_of(context);
    InternalHeat {
        radiogenic_w: radiogenic_w(silicate_kg, inputs.radiogenic, context.age_gyr),
        secular_w: secular_w(silicate_kg, context.age_gyr),
        tidal_w: resonance::todays_heat_w(
            tidal_power_w(
                tide.host_kg,
                interior.radius_earth * EARTH_M,
                tide.a_m,
                tide.eccentricity,
                k2_over_q(inputs.envelope_fraction, inputs.water_fraction),
            ),
            tide.resonance.map(|r| r.heat_w),
        ),
    }
}

/// Every step of a bare body's chain from the water it starts with (see
/// [`bare`]).
pub(crate) struct Bare {
    pub surface: Surface,
    /// The air before the climate.
    pub before: Air,
    /// What it outgassed, bar ([`outgassed`]).
    pub outgassed: Gases,
    /// What escape to space left of it, bar: the cosmic shoreline's
    /// ([`atmosphere`]), then the non-thermal loss inside a giant's
    /// magnetosphere ([`non_thermal_loss`]).
    pub kept: Gases,
    /// What the non-thermal loss took of the air the shoreline left, bar
    /// ([`non_thermal_loss`]): non-zero for the gases it reduced to an
    /// exosphere.
    pub lost: Gases,
    /// What the star's ultraviolet left of that methane
    /// ([`methane_photolysis`]), bar.
    pub photolysed: Gases,
    /// What the cold left of that in the air ([`cold_trap`]), bar.
    pub trapped: Gases,
    /// The air as the climate left it, before life oxygenated it: the
    /// air `surface.climate` was solved with.
    pub climate_gases: Gases,
    /// What the climate condensed on the ground at its surface
    /// temperature ([`atmosphere::condense`]: the methane over its seas
    /// or frosts, the nitrogen of its frosts), bar.
    pub condensed: Gases,
    /// Frost on the ground at the climate's surface temperature, bar, gas
    /// by gas ([`climate::frost`]).
    #[cfg(test)]
    pub frost: Gases,
    /// Whether the climate took its ice crust under a plume's fallout
    /// ([`plume_fallout`]); `None` without an ice crust ([`crust_albedo`]).
    #[cfg(test)]
    pub fallout: Option<bool>,
}

/// The heaviest host whose magnetosphere [`non_thermal_loss`] reads, Earth
/// masses: 13 Jupiter masses, the heaviest giant the generator draws
/// ([`mass::draw_mass`]), at the deuterium-burning line. A heavier host is
/// a star raising its own planet's tides (`context.tide` set to the star),
/// not a giant with a plasma.
const HOST_PLASMA_MAX_MASS_EARTH: f64 = 13.0 * crate::giants::JUPITER_EARTH_MASSES as f64;

/// What [`non_thermal_loss`] reads of a body of `inputs` and `interior`
/// in `context`: its gravity and radius, its star's distance and its age,
/// and, for a moon of a giant ([`moon_of_giant`], up to
/// [`HOST_PLASMA_MAX_MASS_EARTH`]), the giant's mass and the moon's orbit
/// in the giant's radii, that radius from the giant's mass and its
/// equilibrium temperature ([`crate::giants::giant_radius`], as for the
/// host giants of moons).
pub fn loss_context(context: &Context, inputs: &PlanetInputs, interior: &Interior) -> LossContext {
    let radius_m = interior.radius_earth * EARTH_M;
    let gravity = crate::exomoon::G * inputs.mass_earth * EARTH_KG / (radius_m * radius_m);
    let giant =
        |t: &HostTide| moon_of_giant(context) && t.host_kg <= HOST_PLASMA_MAX_MASS_EARTH * EARTH_KG;
    let host = context.tide.filter(giant).map(|t| {
        let host_mass_earth = t.host_kg / EARTH_KG;
        let host_radius_m = f64::from(crate::giants::giant_radius(
            host_mass_earth as f32,
            context.equilibrium_k as f32,
        )) * EARTH_M;
        HostPlasma {
            host_mass_earth,
            semi_major_axis_radii: t.a_m / host_radius_m,
        }
    });
    LossContext {
        gravity,
        radius_m,
        orbit_au: context.orbit_au,
        age_gyr: context.age_gyr,
        host,
    }
}

/// A bare body's surface from the water `w` it starts with (the chain's
/// first pass, or its boiled steam): its [`regime`] and [`core`], its
/// air ([`outgassed`], [`atmosphere`] under its [`shield`], [`non_thermal_loss`],
/// [`methane_photolysis`], [`cold_trap`], [`photochemistry`]), its
/// [`climate`] (with the SO₂ frost a heat pipe's volcanoes lay,
/// [`atmosphere::volcanic_so2_frost_bar`], whose dayside vapour,
/// [`atmosphere::volcanic_so2_vapour`], joins the air after it while it is
/// a trace, [`atmosphere::TRACE_SO2_BAR`], and before it otherwise, the
/// climate then solved again; none on a dayside above SO₂'s triple
/// point), its [`life`] and the oxygen it makes. A runaway's water is
/// the steam escape has left in its air.
pub(crate) fn bare(
    context: &Context,
    inputs: &PlanetInputs,
    interior: &Interior,
    heat: &InternalHeat,
    fluence: f64,
    w: Water,
) -> Bare {
    let bar_per_kg = atmosphere::bar_per_kg(inputs.mass_earth, interior);
    let r = regime(inputs, heat, interior, &w);
    let k = core(inputs, interior, r, context.age_gyr);
    let out = outgassed(inputs, &w, r, interior);
    let shoreline = atmosphere(
        out,
        interior,
        inputs.mass_earth,
        &shield(context, k),
        fluence,
    );
    let loss = loss_context(context, inputs, interior);
    let (kept, lost) = non_thermal_loss(shoreline, &loss);
    let w = if w.runaway {
        w.keeping(kept.h2o / bar_per_kg)
    } else {
        w
    };
    let photolysed = methane_photolysis(kept, context, inputs.mass_earth, interior);
    let trapped = cold_trap(photolysed, context.equilibrium_k);
    let gases = photochemistry(trapped, context.equilibrium_k);
    let flux = heat.flux_w_m2(interior.radius_earth);
    let water_bar = w.kept_kg * bar_per_kg;
    // A heat pipe's volcanoes lay SO₂ frost, which the climate sees in
    // its albedo and whose vapour is the air's SO₂.
    let volcanic_bar = if r == crate::geology::Regime::HeatPipe {
        let supply = atmosphere::volcanic_so2_supply(heat, inputs);
        atmosphere::volcanic_so2_frost_bar(supply, &loss, bar_per_kg)
    } else {
        0.0
    };
    // CO₂ the plasma exhausted is gone from the rocks' reach too: no
    // carbon cycle brings it back.
    let carbon = |so2_frost_bar: f64| climate::CarbonCycle {
        inventory_bar: if lost.co2 > 0.0 { kept.co2 } else { out.co2 },
        frozen_bar: photolysed.co2 - trapped.co2,
        so2_frost_bar,
        snowball_draw: inputs.snowball_draw,
    };
    let (mut gases, mut so2_frost_bar) = (gases, volcanic_bar);
    // An ice crust under its plume's fallout, if it vents: guessed on
    // the shell under the equilibrium temperature, then checked on the
    // shell under the climate's, and the climate solved again under the
    // other answer when they differ (the fallout's brightness cools the
    // surface and thickens the shell).
    let icy = ice_crust(inputs, &w, r);
    let crust = |fallout: bool| icy.then(|| crust_albedo(context, inputs, heat, interior, fallout));
    let solve = |gases: Gases, so2_frost_bar: f64, fallout: bool| {
        climate::climate_over(
            context,
            gases,
            &w,
            r,
            flux,
            water_bar,
            carbon(so2_frost_bar),
            crust(fallout),
        )
    };
    let mut fallout = icy && plume_fallout(inputs, interior, heat, context.equilibrium_k, None);
    let (mut after, mut climate) = solve(gases, so2_frost_bar, fallout);
    if icy {
        let vents = plume_fallout(
            inputs,
            interior,
            heat,
            climate.temperature_k,
            Some(climate.liquid),
        );
        if vents != fallout {
            fallout = vents;
            (after, climate) = solve(gases, so2_frost_bar, fallout);
        }
    }
    // The frost's vapour on the dayside, the air's SO₂ wherever the frost
    // holds: more than a trace goes into the air before the climate, out
    // of the frost, and the climate is solved again with it. A surface or
    // dayside above SO₂'s triple point holds no frost: the body keeps the
    // SO₂ its outgassing gave it.
    let mut vapour = 0.0;
    if volcanic_bar > 0.0 {
        let held = atmosphere::volcanic_so2_vapour(
            volcanic_bar,
            climate.equilibrium_k,
            climate.temperature_k,
        );
        let added = match held {
            None => {
                so2_frost_bar = 0.0;
                Some(0.0)
            }
            Some(v) if v > atmosphere::TRACE_SO2_BAR => {
                vapour = v;
                Some(v)
            }
            Some(v) => {
                vapour = v;
                None
            }
        };
        if let Some(v) = added {
            gases.so2 += v;
            so2_frost_bar = (so2_frost_bar - v).max(0.0);
            (after, climate) = solve(gases, so2_frost_bar, fallout);
        }
    }
    let condensed = atmosphere::condense(gases, climate.temperature_k).1;
    let life = life(inputs, &climate, context.age_gyr);
    // Washed as the albedo washed it ([`climate::washed`]), so that the
    // look's deck is the albedo's; but the frost's vapour is not washed
    // out: the frost holds the air at it, whatever takes it. Neither deck
    // sees it, under SO₂'s triple point and so far under the deck's 273 K.
    let washed = climate::washed(oxygenate(after, &life), &w);
    #[cfg(test)]
    let frost = climate::frost(
        &gases,
        photolysed.co2 - trapped.co2,
        so2_frost_bar,
        climate.temperature_k,
        &after,
    );
    let surface_gases = Gases {
        so2: washed.so2.max(vapour),
        ..washed
    };
    Bare {
        surface: Surface {
            water: w,
            regime: r,
            core: k,
            gases: surface_gases,
            climate,
            life,
        },
        before: Air {
            water: w,
            regime: r,
            gases,
            carbon_bar: out.co2,
            frozen_co2_bar: photolysed.co2 - trapped.co2,
            so2_frost_bar,
        },
        outgassed: out,
        kept,
        lost,
        photolysed,
        trapped,
        climate_gases: after,
        condensed,
        #[cfg(test)]
        frost,
        #[cfg(test)]
        fallout: icy.then_some(fallout),
    }
}

/// Whether a body of `inputs` with the water `w` and the regime `regime`
/// it starts its climate with has an ice crust: the test of
/// `generation::model_crust_kind`, read before the model exists. A
/// stagnant lid (the regime `surface_age::terrain` gives ice units),
/// [`classify::ICE_WORLD_MIN_WATER`] of water and
/// [`classify::ICE_WORLD_MIN_OCEANS`] of it kept, out of a runaway. That
/// test's third condition, no liquid water at the surface, is the
/// climate's: the crust's albedo applies to the cover's ice and land
/// ([`albedo::cover`]), none of which is open water, and only under an
/// air thinner than `interim::ERODING_BAR` ([`climate::climate_over`]).
fn ice_crust(inputs: &PlanetInputs, w: &Water, regime: crate::geology::Regime) -> bool {
    use classify::{ICE_WORLD_MIN_OCEANS, ICE_WORLD_MIN_WATER};
    regime == crate::geology::Regime::Stagnant
        && !w.runaway
        && inputs.water_fraction >= ICE_WORLD_MIN_WATER
        && w.oceans >= ICE_WORLD_MIN_OCEANS
}

/// The Bond albedo of the exposed ice crust ([`ice_crust`]) of a body of
/// `inputs`, `heat` and `interior` in `context`: the area-weighted
/// [`albedo::icy_unit_albedo`] of the ice units it has before its
/// climate ([`surface_age::icy_estimate`]), its rock `1 − water_fraction`
/// of its mass, under its plume's fallout if `fallout`
/// ([`plume_fallout`]).
fn crust_albedo(
    context: &Context,
    inputs: &PlanetInputs,
    heat: &InternalHeat,
    interior: &Interior,
    fallout: bool,
) -> f64 {
    let rock = 1.0 - inputs.water_fraction;
    surface_age::icy_estimate(
        heat,
        interior.radius_earth,
        context.age_gyr,
        context.tide.as_ref(),
    )
    .units
    .iter()
    .map(|u| u.share * albedo::icy_unit_albedo(u.age_gyr, rock, fallout))
    .sum()
}

/// Whether a body of `inputs`, `interior` and `heat` vents under a
/// surface at `surface_k`, so that its plume's fallout keeps its ice
/// fresh ([`albedo::icy_unit_albedo`]): the criterion of its geysers
/// ([`geysers::vents`]) on the ice shell [`ice_shell`] gives it there,
/// none over surface liquid water (`liquid`, when known). [`bare`] reads
/// it before the climate under the equilibrium temperature, then under
/// the climate's own, as the stored geysers do ([`vent`]).
pub(crate) fn plume_fallout(
    inputs: &PlanetInputs,
    interior: &Interior,
    heat: &InternalHeat,
    surface_k: f64,
    liquid: Option<SurfaceLiquid>,
) -> bool {
    if liquid == Some(SurfaceLiquid::Water) {
        return false;
    }
    let nh3_ice = if inputs.water_fraction > 0.0 {
        inputs.volatiles.nh3 / inputs.water_fraction
    } else {
        0.0
    };
    ice_shell::shell_under(interior, heat, surface_k, nh3_ice)
        .is_some_and(|shell| geysers::vents(interior, &shell, heat))
}

/// The model of the planet `id` of the system of `system_seed`, in its
/// formation `context`, drawn `mass_earth` Earth masses (before escape).
///
/// Its context is its own star seen from its orbit (its star, orbit,
/// eccentricity and age), its system's metallicity and ice line. Its
/// inputs are drawn from their own stream, seeded by the system's seed and
/// `<id>/model`, so they depend on no other body and shift no other draw;
/// its geysers, if its shell cracks through, on `<id>/geysers` ([`vent`]).
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
    vent(planet_model(context, inputs), system_seed, id)
}

/// `model` with its [`geysers`], the pole and the fractures' strike drawn
/// on the stream of its own `<id>/geysers` of the system of
/// `system_seed`, so that they shift no other draw.
pub(crate) fn vent(model: PlanetModel, system_seed: u32, id: &str) -> PlanetModel {
    let seed = body_seed(system_seed, &format!("{id}/geysers"));
    let geysers = model
        .ice_shell
        .as_ref()
        .and_then(|shell| geysers::geysers(&model.interior, shell, &model.heat, seed));
    PlanetModel { geysers, ..model }
}

/// The model a stored system holds for `body`: the one [`derive_model`]
/// gave its planet when the system was drawn, or the one the resolution of
/// its request gave the world a system was drawn around (a giant's moon
/// among them), or the one [`moon::derive_moon_model`] gave a round moon;
/// `None` for a moon too small to be round.
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
            // The snowball draw is drawn, after the life draw.
            let draw = model.inputs.snowball_draw;
            assert!((0.0..1.0).contains(&draw), "{}: {draw}", body.id);
            assert_ne!(draw, model.inputs.life_draw, "{}", body.id);
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

    /// Mass of Jupiter, kg (NASA fact sheet).
    const JUPITER_KG: f64 = 1.898e27;

    /// The context of a moon of Jupiter on an orbit of semi-major axis
    /// `a_m` and eccentricity `eccentricity`, Jupiter at 5.2 AU of the Sun.
    fn round_jupiter(a_m: f64, eccentricity: f64) -> Context {
        reference::sun_at(5.2).with_tide(HostTide {
            host_kg: JUPITER_KG,
            a_m,
            eccentricity,
            resonance: None,
        })
    }

    /// The model of the reference body `name` round Jupiter.
    fn moon(name: &str, a_m: f64, eccentricity: f64) -> PlanetModel {
        let inputs = reference::model_inputs(reference(name));
        planet_model(&round_jupiter(a_m, eccentricity), inputs)
    }

    #[test]
    fn a_giants_moon_is_unshielded_and_the_moon_is_not() {
        let core = Core {
            state: CoreState::Liquid,
            dynamo: true,
            field_ut: 1.0,
        };
        let around = |host_kg: f64| {
            reference::sun_at(5.2).with_tide(HostTide {
                host_kg,
                a_m: 4.217e8,
                eccentricity: 0.0041,
                resonance: None,
            })
        };
        assert!(!shield(&around(JUPITER_KG), core).dynamo);
        assert_eq!(shield(&around(5.972e24), core), core);
        assert_eq!(shield(&reference::sun_at(1.0), core), core);
    }

    #[test]
    fn io_heats_by_its_host_giant() {
        // Io: about 1e14 W observed (Lainey et al. 2009); the Sun's tides
        // at 5.2 AU on a circular orbit give it none.
        let io = moon("Io", 4.217e8, 0.0041);
        assert!(
            (3e13..3e14).contains(&io.heat.tidal_w),
            "{}",
            io.heat.tidal_w
        );
    }

    #[test]
    fn europa_has_a_thin_shell_over_an_ocean() {
        let europa = moon("Europa", 6.709e8, 0.009);
        let shell = europa.ice_shell.expect("Europa has an ice shell");
        assert!((5.0..40.0).contains(&shell.shell_km), "{shell:?}");
        assert!(shell.ocean_km > 0.0, "{shell:?}");
    }

    #[test]
    fn callisto_shell_is_thick() {
        let callisto = moon("Callisto", 1.883e9, 0.0074);
        let shell = callisto.ice_shell.expect("Callisto has an ice shell");
        assert!(shell.shell_km > 100.0, "{shell:?}");
    }

    #[test]
    fn earth_has_no_ice_shell() {
        let earth = planet_model(
            &reference::sun_at(1.0),
            reference::model_inputs(reference("Earth")),
        );
        assert_eq!(earth.ice_shell, None);
    }

    #[test]
    fn earth_and_mars_lose_their_measured_heat() {
        // Earth: 47 ± 2 TW (Davies & Davies 2010), 0.087 to 0.092 W/m²
        // (Pollack, Hurter & Johnson 1993; Davies & Davies 2010). Mars:
        // about 0.019 to 0.025 W/m² from the thermal histories that fit
        // its crust (Plesa et al. 2018), a range kept wider for the
        // spread of those models.
        let flux = |name: &str| {
            let body = reference(name);
            let model = planet_model(
                &reference::sun_at(body.orbit_au),
                reference::model_inputs(body),
            );
            model.heat.flux_w_m2(model.interior.radius_earth)
        };
        let earth = flux("Earth");
        assert!((0.087..=0.092).contains(&earth), "Earth {earth}");
        let mars = flux("Mars");
        assert!((0.015..=0.030).contains(&mars), "Mars {mars}");
    }

    #[test]
    fn a_planet_without_host_tide_is_unchanged() {
        // `Context::of` leaves the tide to the star: the same model as
        // with the star given as the host, on the planet's own orbit.
        let context = sun_at(0.05, 0.2);
        assert_eq!(context.tide, None);
        let inputs = reference::model_inputs(reference("Earth"));
        let model = planet_model(&context, inputs.clone());
        let star_as_host = context.with_tide(HostTide {
            host_kg: context.star_mass * SUN_KG,
            a_m: context.orbit_au * AU_M,
            eccentricity: context.eccentricity,
            resonance: None,
        });
        assert_eq!(planet_model(&star_as_host, inputs), model);
        assert!(model.heat.tidal_w > 0.0);
        // Earth at its own orbit keeps the climate the climate tests pin.
        let earth = planet_model(
            &reference::sun_at(1.0),
            reference::model_inputs(reference("Earth")),
        );
        let climate = earth.surface.expect("Earth has a surface").climate;
        assert!((282.0..=294.0).contains(&climate.temperature_k));
        assert_eq!(climate.liquid, SurfaceLiquid::Water);
        assert_eq!(earth.heat.tidal_w, 0.0);
    }

    #[test]
    fn only_round_moons_have_a_model() {
        let system = drawn();
        for body in system.bodies.iter().filter(|b| b.kind == BodyKind::Moon) {
            let small = ["rocky-moon", "icy-moon"].contains(&body.body_type.as_str());
            assert_eq!(body_model(&system, body).is_none(), small, "{}", body.id);
        }
    }

    /// A snowball keeps its albedo path (Review Focus 4): an Earth-like
    /// planet pinned frozen at 1.2 AU, built as `climate_sweep` builds
    /// it, has no ice crust (its water is a thin ocean on rock), so the
    /// icy-age law does not touch it: its Bond albedo is the one it had
    /// before that law (0.6224 at bf1f1f2).
    #[test]
    fn a_snowball_keeps_its_albedo() {
        use crate::request::WorldRequest;
        let request = WorldRequest {
            planet: Some("earth-like".into()),
            snowball: Some(true),
            seed: Some(1),
            star: Some("G2V".into()),
            orbit_au: Some(1.2),
            age_gyr: Some(4.57),
            alone: Some(true),
            width: Some(64),
            height: Some(32),
            ..WorldRequest::default()
        };
        let resolved = request
            .resolve(&mut StdRng::seed_from_u64(1))
            .expect("an Earth resolves");
        let surface = resolved.model().surface.expect("a surface");
        assert!(surface.climate.snowball, "{surface:?}");
        let a = surface.climate.bond_albedo;
        println!("snowball Bond albedo {a:.12}");
        assert!((a - SNOWBALL_ALBEDO).abs() < 1e-9, "{a}");
    }

    /// The Bond albedo of [`a_snowball_keeps_its_albedo`]'s snowball
    /// before the icy-age law.
    const SNOWBALL_ALBEDO: f64 = 0.622_385_341_082;
}
