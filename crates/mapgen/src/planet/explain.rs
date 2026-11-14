//! Cause trees (part 5 of the physical planet model, issue #423): each
//! value the chain derives for a body (its class, surface temperature,
//! pressure, surface liquid, regime, magnetic field and water kept)
//! unfolded into the values it was derived from, down to the inputs a
//! request sets ([`InputField`]), for the body's card.
//!
//! [`explain`] recomputes each decomposed value with the chain's own
//! functions from the model and its formation context, so the tree and
//! the stored model cannot disagree: when the recomputation does not give
//! the stored values back (the code changed without a format bump), it
//! gives no tree. The trees are data only, codes and numbers: the web app
//! owns their wording.
use serde::Serialize;

use super::{
    Bare, Context, Gases, OCEAN_KG, PlanetModel, Surface, Water, bare, internal_heat, steam,
    surface_temperature_k, water,
};
use crate::exomoon::EARTH_KG;

/// The values a body's card unfolds, one tree each.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Explanation {
    /// One root per explained value, in this order, a root left out when
    /// its value does not exist (a body without surface has its class
    /// only): [`Quantity::Class`], [`Quantity::SurfaceTemperature`],
    /// [`Quantity::Pressure`], [`Quantity::Liquid`], [`Quantity::Regime`],
    /// [`Quantity::MagneticField`], [`Quantity::WaterKept`].
    pub trees: Vec<Cause>,
}

/// A value and the values it was derived from. At most three levels lie
/// below a root.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Cause {
    pub quantity: Quantity,
    /// In the unit of `quantity`; 0 when `label` holds the value.
    pub value: f64,
    /// The key of a value that is not a number: a class
    /// (`PlanetClass::key`, `mini-neptune`), a regime (`plates`), a
    /// liquid (`water`), a core state (`liquid`), or a note's key
    /// (`thermostat`, `life`, `runaway`, `boiled`).
    pub label: Option<String>,
    /// The request's field this value comes from, when a request sets it.
    pub input: Option<InputField>,
    pub causes: Vec<Cause>,
}

/// What a [`Cause`] holds, with its unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Quantity {
    /// Mean surface temperature, K.
    SurfaceTemperature,
    /// The surface temperature without air and without internal heat:
    /// the equilibrium temperature, K.
    EquilibriumTemperature,
    /// Light received, Earth's = 1.
    LightReceived,
    /// What the air adds to the temperature, K.
    Greenhouse,
    /// What the internal heat adds to the temperature under the air, K.
    InternalWarming,
    /// Internal heat flux through the surface, W/m².
    HeatFlux,
    /// Radiogenic heat, W.
    RadiogenicHeat,
    /// Tidal heat, W.
    TidalHeat,
    /// Surface pressure, bar.
    Pressure,
    /// CO₂ partial pressure, bar.
    Co2,
    /// N₂ partial pressure, bar.
    N2,
    /// Water vapour partial pressure, bar.
    H2o,
    /// CH₄ partial pressure, bar.
    Ch4,
    /// O₂ partial pressure, bar.
    O2,
    /// NH₃ partial pressure, bar.
    Nh3,
    /// SO₂ partial pressure, bar.
    So2,
    /// A gas as outgassed under the regime, bar.
    Outgassed,
    /// What escape to space took, in its parent's unit (bar of a gas,
    /// Earth oceans of water).
    Escape,
    /// What the star's ultraviolet destroyed of a gas (methane's
    /// photolysis and oxidation, ammonia's photolysis), bar.
    Destroyed,
    /// What the cold froze out of the air onto the ground, bar.
    FrozenOut,
    /// N₂ the photolysis of ammonia made, bar.
    FromAmmonia,
    /// CO₂ the carbonate weathering drew out of the air, bar.
    Weathered,
    /// The liquid on the surface (label: `none`, `water`,
    /// `hydrocarbons`, `lava`).
    Liquid,
    /// The tectonic regime (label: `plates`, `stagnant`, `episodic`,
    /// `heat_pipe`).
    Regime,
    /// Surface equatorial dipole field, µT.
    MagneticField,
    /// The iron core's state (label: `none`, `liquid`, `frozen`).
    CoreState,
    /// Iron share of the refractory mass.
    CoreMassFraction,
    /// The class (label: its key).
    Class,
    /// Mass, Earth masses.
    Mass,
    /// Radius, Earth radii.
    Radius,
    /// Hydrogen and helium share of the mass.
    EnvelopeFraction,
    /// Share of the envelope photoevaporation took, 0 to 1.
    EnvelopeLost,
    /// Water the body formed with, Earth oceans.
    WaterFormed,
    /// Water kept, Earth oceans.
    WaterKept,
    /// Age of the system, Gyr.
    Age,
    /// A condition, in `label`: `thermostat` (the carbonate thermostat
    /// set the CO₂), `life` (its oxygen), `runaway` or `boiled` (the
    /// water ran away by the insolation limit, or the climate boiled it).
    Note,
}

/// A request's field, which a [`Cause`] links to its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum InputField {
    MassEarth,
    CoreMassFraction,
    WaterOceans,
    EnvelopeFraction,
    Co2,
    N2,
    Ch4,
    Radiogenic,
    Life,
    Insolation,
    Eccentricity,
    AgeGyr,
}

impl Cause {
    /// A value with no causes.
    fn of(quantity: Quantity, value: f64) -> Self {
        Self {
            quantity,
            value,
            label: None,
            input: None,
            causes: Vec::new(),
        }
    }

    /// A value that is a key: `value`'s serialised name.
    fn keyed(quantity: Quantity, value: impl Serialize) -> Self {
        let label = serde_json::to_value(value)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned));
        Self {
            label,
            ..Self::of(quantity, 0.0)
        }
    }

    /// A [`Quantity::Note`] of `key`.
    fn note(key: &str) -> Self {
        Self {
            label: Some(key.to_owned()),
            ..Self::of(Quantity::Note, 0.0)
        }
    }

    /// The value as the request's `field` sets it.
    fn set_by(self, field: InputField) -> Self {
        Self {
            input: Some(field),
            ..self
        }
    }

    fn because(self, causes: Vec<Cause>) -> Self {
        Self { causes, ..self }
    }
}

/// Most a recomposed surface temperature may differ from the stored one,
/// K: the water-vapour feedback converges to 1e-3 K (see
/// [`super::climate::climate`]), the rest is exact.
const TEMPERATURE_TOLERANCE_K: f64 = 0.5;

/// Most a recomputed internal heat may differ from the stored one,
/// relative.
const HEAT_TOLERANCE: f64 = 1e-6;

/// The cause trees of `model` in its formation `context` (see
/// [`Explanation`]); `None` when `context` does not reproduce it.
///
/// The internal heat is recomputed ([`super::internal_heat`]) and must be
/// the stored one within [`HEAT_TOLERANCE`]. A bare body's surface steps
/// are run again from the water the chain started them with (its first
/// pass, or the boiled steam it kept, see [`super::planet_model`]), which
/// gives what it outgassed, what escape left and the air its climate was
/// solved with (the stored air is after life's oxygen). The surface
/// temperature is decomposed with [`surface_temperature_k`],
/// `T⁴ = T_eq⁴ (1 + ¾ τ) + q/σ`: its equilibrium temperature (no air, no
/// heat), the greenhouse (with air, minus that) and the internal warming
/// (with air and heat, minus with air only), whose sum must be the stored
/// temperature within [`TEMPERATURE_TOLERANCE_K`].
pub fn explain(context: &Context, model: &PlanetModel) -> Option<Explanation> {
    let heat = internal_heat(context, &model.inputs, &model.interior);
    let close = |a: f64, b: f64| (a - b).abs() <= HEAT_TOLERANCE * a.abs().max(b.abs());
    if !close(heat.tidal_w, model.heat.tidal_w)
        || !close(heat.radiogenic_w, model.heat.radiogenic_w)
    {
        return None;
    }
    let steps = model.surface.as_ref().map(|s| replay(context, model, s));
    let mut trees = vec![class_tree(model)];
    if let (Some(surface), Some(steps)) = (&model.surface, &steps) {
        trees.push(temperature_tree(context, model, surface, steps)?);
        trees.push(pressure_tree(surface, steps));
        trees.push(liquid_tree(surface));
        trees.push(regime_tree(context, model, surface));
        trees.push(field_tree(context, model, surface));
        trees.push(water_tree(model, surface));
    }
    Some(Explanation { trees })
}

/// The surface steps of `model` run again (see [`bare`]), from the water
/// the chain started the kept pass with: the boiled steam when `surface`
/// boiled, else the water of its first pass.
fn replay(context: &Context, model: &PlanetModel, surface: &Surface) -> Bare {
    let (i, interior, fluence) = (&model.inputs, &model.interior, model.xuv_fluence);
    let start = if surface.water.boiled {
        Water {
            boiled: true,
            ..steam(i, interior, fluence)
        }
    } else {
        water(context, i, interior, fluence)
    };
    bare(context, i, interior, &model.heat, fluence, start)
}

/// The mass kept. It is the request's own when escape took nothing of the
/// envelope; otherwise it is derived from the mass drawn, which is the input.
fn mass(model: &PlanetModel) -> Cause {
    let kept = Cause::of(Quantity::Mass, model.inputs.mass_earth);
    if model.drawn_mass_earth == model.inputs.mass_earth {
        kept.set_by(InputField::MassEarth)
    } else {
        kept.because(vec![
            Cause::of(Quantity::Mass, model.drawn_mass_earth).set_by(InputField::MassEarth),
        ])
    }
}

fn water_kept(surface: &Surface) -> Cause {
    Cause::of(Quantity::WaterKept, surface.water.oceans).set_by(InputField::WaterOceans)
}

fn heat_flux(context: &Context, model: &PlanetModel) -> Cause {
    let tidal = Cause::of(Quantity::TidalHeat, model.heat.tidal_w);
    // The form's eccentricity is the orbit round the star: a moon's
    // tides come from its planet.
    let tidal = if context.tide.is_none() {
        tidal.set_by(InputField::Eccentricity)
    } else {
        tidal
    };
    Cause::of(
        Quantity::HeatFlux,
        model.heat.flux_w_m2(model.interior.radius_earth),
    )
    .because(vec![
        Cause::of(Quantity::RadiogenicHeat, model.heat.radiogenic_w).set_by(InputField::Radiogenic),
        tidal,
    ])
}

/// What [`super::classify`] reads: the mass, the envelope left (and what
/// escape took of it), the radius, and on a bare body the water kept,
/// the pressure, the temperature and the liquid.
fn class_tree(model: &PlanetModel) -> Cause {
    let envelope = Cause::of(Quantity::EnvelopeFraction, model.inputs.envelope_fraction)
        .set_by(InputField::EnvelopeFraction);
    let envelope = if model.envelope_lost > 0.0 {
        envelope.because(vec![Cause::of(Quantity::EnvelopeLost, model.envelope_lost)])
    } else {
        envelope
    };
    let mut causes = vec![
        mass(model),
        envelope,
        Cause::of(Quantity::Radius, model.interior.radius_earth),
    ];
    if let Some(s) = &model.surface {
        causes.extend([
            water_kept(s),
            Cause::of(Quantity::Pressure, s.gases.total()),
            Cause::of(Quantity::SurfaceTemperature, s.climate.temperature_k),
            Cause::keyed(Quantity::Liquid, s.climate.liquid),
        ]);
    }
    Cause {
        label: Some(model.class.key().to_owned()),
        ..Cause::of(Quantity::Class, 0.0)
    }
    .because(causes)
}

/// The surface temperature as the equilibrium temperature, the
/// greenhouse of the air the climate was solved with (each absorber's
/// partial pressure, and the N₂ that broadens their lines) and the
/// internal warming (see [`explain`]); `None` when they do not sum to the
/// stored temperature.
fn temperature_tree(
    context: &Context,
    model: &PlanetModel,
    surface: &Surface,
    steps: &Bare,
) -> Option<Cause> {
    let gases = &steps.climate_gases;
    let flux = model.heat.flux_w_m2(model.interior.radius_earth);
    let bare_k = surface_temperature_k(context, &Gases::default(), 0.0);
    let air_k = surface_temperature_k(context, gases, 0.0);
    let full_k = surface_temperature_k(context, gases, flux);
    let stored = surface.climate.temperature_k;
    if (full_k - stored).abs() > TEMPERATURE_TOLERANCE_K {
        return None;
    }
    let co2 = Cause::of(Quantity::Co2, gases.co2).set_by(InputField::Co2);
    let co2 = if surface.climate.regulated {
        co2.because(vec![Cause::note("thermostat")])
    } else {
        co2
    };
    let absorbers = [
        (gases.n2 > 0.0).then(|| Cause::of(Quantity::N2, gases.n2).set_by(InputField::N2)),
        (gases.co2 > 0.0).then_some(co2),
        (gases.h2o > 0.0)
            .then(|| Cause::of(Quantity::H2o, gases.h2o).set_by(InputField::WaterOceans)),
        (gases.ch4 > 0.0).then(|| Cause::of(Quantity::Ch4, gases.ch4).set_by(InputField::Ch4)),
    ];
    Some(
        Cause::of(Quantity::SurfaceTemperature, stored).because(vec![
            Cause::of(Quantity::EquilibriumTemperature, bare_k).because(vec![
                Cause::of(Quantity::LightReceived, context.insolation)
                    .set_by(InputField::Insolation),
            ]),
            Cause::of(Quantity::Greenhouse, air_k - bare_k)
                .because(absorbers.into_iter().flatten().collect()),
            Cause::of(Quantity::InternalWarming, full_k - air_k)
                .because(vec![heat_flux(context, model)]),
        ]),
    )
}

/// Each gas of the air (as stored, after life) from its source and what
/// took from it, each step of the chain in its order: what was outgassed
/// under the regime, what escape took, what the ultraviolet destroyed
/// (or, for N₂, made from ammonia), what the cold froze out, and the CO₂
/// the weathering drew down; the water vapour from the water kept and
/// the temperature (or, in a runaway, the steam and its escape); the
/// oxygen from life.
fn pressure_tree(surface: &Surface, steps: &Bare) -> Cause {
    let g = &surface.gases;
    let regime = || Cause::keyed(Quantity::Regime, surface.regime);
    // The steps of one gas, `of` reading it from a `Gases`.
    let source = |of: fn(&Gases) -> f64, input: Option<InputField>| {
        let outgassed =
            Cause::of(Quantity::Outgassed, of(&steps.outgassed)).because(vec![regime()]);
        let outgassed = match input {
            Some(field) => outgassed.set_by(field),
            None => outgassed,
        };
        let (kept, photolysed, trapped) =
            (of(&steps.kept), of(&steps.photolysed), of(&steps.trapped));
        let before = of(&steps.before.gases);
        let destroyed = (kept - photolysed) + (trapped - before).max(0.0);
        let steps = [
            (Quantity::Escape, of(&steps.outgassed) - kept),
            (Quantity::Destroyed, destroyed),
            (Quantity::FrozenOut, photolysed - trapped),
            (Quantity::FromAmmonia, (before - trapped).max(0.0)),
            (Quantity::Weathered, before - of(&steps.climate_gases)),
        ];
        let mut causes = vec![outgassed];
        causes.extend(
            steps
                .into_iter()
                .filter(|(_, bar)| *bar > 0.0)
                .map(|(q, bar)| Cause::of(q, bar)),
        );
        causes
    };
    let mut co2 = source(|g| g.co2, Some(InputField::Co2));
    if surface.climate.regulated {
        co2.push(Cause::note("thermostat"));
    }
    let h2o = if surface.water.runaway {
        source(|g| g.h2o, Some(InputField::WaterOceans))
    } else {
        vec![
            water_kept(surface),
            Cause::of(Quantity::SurfaceTemperature, surface.climate.temperature_k),
        ]
    };
    let gases = [
        (Quantity::N2, g.n2, source(|g| g.n2, Some(InputField::N2))),
        (Quantity::Co2, g.co2, co2),
        (Quantity::H2o, g.h2o, h2o),
        (
            Quantity::Ch4,
            g.ch4,
            source(|g| g.ch4, Some(InputField::Ch4)),
        ),
        (Quantity::Nh3, g.nh3, source(|g| g.nh3, None)),
        (Quantity::So2, g.so2, source(|g| g.so2, None)),
        (
            Quantity::O2,
            g.o2,
            vec![Cause::note("life").set_by(InputField::Life)],
        ),
    ];
    Cause::of(Quantity::Pressure, g.total()).because(
        gases
            .into_iter()
            .filter(|(_, bar, _)| *bar > 0.0)
            .map(|(q, bar, causes)| Cause::of(q, bar).because(causes))
            .collect(),
    )
}

/// The liquid from the phase diagrams: the temperature, the pressure and
/// the water kept.
fn liquid_tree(surface: &Surface) -> Cause {
    Cause::keyed(Quantity::Liquid, surface.climate.liquid).because(vec![
        Cause::of(Quantity::SurfaceTemperature, surface.climate.temperature_k),
        Cause::of(Quantity::Pressure, surface.gases.total()),
        water_kept(surface),
    ])
}

/// What [`super::regime`] reads: the mass, the internal heat flux and the
/// water kept (and whether it ran away).
fn regime_tree(context: &Context, model: &PlanetModel, surface: &Surface) -> Cause {
    let water = water_kept(surface);
    let water = if surface.water.runaway {
        water.because(vec![runaway(&surface.water)])
    } else {
        water
    };
    Cause::keyed(Quantity::Regime, surface.regime).because(vec![
        mass(model),
        heat_flux(context, model),
        water,
    ])
}

fn runaway(water: &Water) -> Cause {
    Cause::note(if water.boiled { "boiled" } else { "runaway" })
}

/// The field from the core's state (its iron share and the system's age)
/// and the regime that cools it (see [`super::core`]).
fn field_tree(context: &Context, model: &PlanetModel, surface: &Surface) -> Cause {
    Cause::of(Quantity::MagneticField, surface.core.field_ut).because(vec![
        Cause::keyed(Quantity::CoreState, surface.core.state).because(vec![
            Cause::of(Quantity::CoreMassFraction, model.inputs.core_mass_fraction)
                .set_by(InputField::CoreMassFraction),
            Cause::of(Quantity::Age, context.age_gyr).set_by(InputField::AgeGyr),
        ]),
        Cause::keyed(Quantity::Regime, surface.regime),
    ])
}

/// The water kept from the water the body formed with (its share of the
/// mass escape left, as [`super::water`] takes it) and, in a runaway,
/// what escape took of its steam.
fn water_tree(model: &PlanetModel, surface: &Surface) -> Cause {
    let formed = model.inputs.water_fraction * model.inputs.mass_earth * EARTH_KG / OCEAN_KG;
    let mut causes = vec![Cause::of(Quantity::WaterFormed, formed).set_by(InputField::WaterOceans)];
    if surface.water.runaway {
        causes.push(runaway(&surface.water));
        if formed > surface.water.oceans {
            causes.push(Cause::of(Quantity::Escape, formed - surface.water.oceans));
        }
    }
    Cause::of(Quantity::WaterKept, surface.water.oceans).because(causes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::reference::{model_inputs, reference, sun_at, with_envelope};
    use crate::planet::{InputOverrides, PlanetClass, draw_inputs_with, planet_model};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Earth: the context of `reference::sun_at(1.0)`, the inputs the
    /// `earth-like` archetype sets (one Earth mass, CMF 0.325, an ocean,
    /// no envelope, CO₂ and N₂ on their rule, life), the rest drawn.
    fn earth() -> (Context, PlanetModel) {
        let context = sun_at(1.0);
        let overrides = InputOverrides {
            core_mass_fraction: Some(0.325),
            water_oceans: Some(1.0),
            envelope_fraction: Some(0.0),
            co2: Some(1.0),
            n2: Some(1.0),
            ch4: None,
            radiogenic: None,
            life: Some(true),
        };
        let inputs = draw_inputs_with(&context, 1.0, &overrides, &mut StdRng::seed_from_u64(1));
        let model = planet_model(&context, inputs);
        (context, model)
    }

    /// A Solar System body round the Sun, as `reference` holds it.
    fn solar(name: &str) -> (Context, PlanetModel) {
        let b = reference(name);
        let context = sun_at(b.orbit_au);
        (context, planet_model(&context, model_inputs(b)))
    }

    fn root(e: &Explanation, quantity: Quantity) -> &Cause {
        e.trees
            .iter()
            .find(|c| c.quantity == quantity)
            .unwrap_or_else(|| panic!("no {quantity:?} tree"))
    }

    fn depth(c: &Cause) -> usize {
        c.causes.iter().map(|c| 1 + depth(c)).max().unwrap_or(0)
    }

    fn inputs_named(c: &Cause, out: &mut Vec<InputField>) {
        out.extend(c.input);
        for c in &c.causes {
            inputs_named(c, out);
        }
    }

    #[test]
    fn earth_temperature_tree_reproduces_the_model() {
        let (context, model) = earth();
        let e = explain(&context, &model).expect("Earth is explained");
        let t = root(&e, Quantity::SurfaceTemperature);
        assert!((t.value - model.surface.unwrap().climate.temperature_k).abs() < 1e-9);
        let sum: f64 = t.causes.iter().map(|c| c.value).sum();
        assert!((sum - t.value).abs() < 0.5, "{sum} vs {}", t.value);
        assert!(
            t.causes
                .iter()
                .any(|c| c.quantity == Quantity::Greenhouse && c.value > 20.0),
            "{t:#?}"
        );
    }

    #[test]
    fn trees_are_at_most_three_levels_deep() {
        let bodies = [earth(), solar("Venus"), solar("Mars"), solar("Titan")];
        for (context, model) in &bodies {
            let e = explain(context, model).expect("a Solar System body is explained");
            assert_eq!(e.trees[0].quantity, Quantity::Class);
            assert!(e.trees.len() >= 7, "{:?}", model.class);
            for tree in &e.trees {
                assert!(depth(tree) <= 3, "{:?}: {tree:#?}", tree.quantity);
            }
        }
    }

    #[test]
    fn every_input_leaf_names_a_field() {
        let (context, model) = earth();
        let e = explain(&context, &model).unwrap();
        let mut named = Vec::new();
        for tree in &e.trees {
            inputs_named(tree, &mut named);
        }
        for field in [
            InputField::WaterOceans,
            InputField::Insolation,
            InputField::Co2,
        ] {
            assert!(named.contains(&field), "{field:?} in {named:?}");
        }
    }

    #[test]
    fn a_body_without_surface_has_class_but_no_temperature_tree() {
        let context = sun_at(0.3);
        let model = planet_model(&context, with_envelope(6.0, 0.03));
        assert_eq!(model.class, PlanetClass::MiniNeptune);
        let e = explain(&context, &model).expect("a mini-Neptune is explained");
        let class = root(&e, Quantity::Class);
        assert_eq!(class.label.as_deref(), Some("mini-neptune"));
        assert!(
            e.trees
                .iter()
                .all(|c| c.quantity != Quantity::SurfaceTemperature)
        );
    }

    #[test]
    fn an_escaped_mass_is_derived_from_the_drawn_one() {
        let context = sun_at(0.03);
        let model = planet_model(&context, with_envelope(6.0, 0.05));
        assert!(model.envelope_lost > 0.0, "{}", model.envelope_lost);
        assert!(model.drawn_mass_earth > model.inputs.mass_earth);
        let e = explain(&context, &model).expect("explained");
        let mass = root(&e, Quantity::Class)
            .causes
            .iter()
            .find(|c| c.quantity == Quantity::Mass)
            .expect("the class reads the mass");
        assert_eq!(mass.input, None);
        assert_eq!(mass.value, model.inputs.mass_earth);
        let [drawn] = mass.causes.as_slice() else {
            panic!("one cause: {:?}", mass.causes)
        };
        assert_eq!(drawn.input, Some(InputField::MassEarth));
        assert_eq!(drawn.value, model.drawn_mass_earth);
    }

    #[test]
    fn an_unescaped_mass_is_the_input_itself() {
        let (context, model) = earth();
        let e = explain(&context, &model).expect("explained");
        let mass = root(&e, Quantity::Class)
            .causes
            .iter()
            .find(|c| c.quantity == Quantity::Mass)
            .unwrap();
        assert_eq!(mass.input, Some(InputField::MassEarth));
        assert!(mass.causes.is_empty());
    }
}
