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

use super::climate::{frozen_over, wet_temperature_k};
use super::surface_age::Renewal;
use super::{
    Bare, Context, Gases, OCEAN_KG, PlanetModel, Surface, SurfaceLiquid, Water, bare, ice_crust,
    internal_heat, steam, surface_temperature_k, vent_strength, water,
};
use crate::exomoon::EARTH_KG;
use crate::geology::Regime;

/// The values a body's card unfolds, one tree each.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Explanation {
    /// One root per explained value, in this order, a root left out when
    /// its value does not exist (a body without surface has its class
    /// only): [`Quantity::Class`], [`Quantity::SurfaceTemperature`],
    /// [`Quantity::Pressure`], [`Quantity::Liquid`], [`Quantity::Regime`],
    /// [`Quantity::MagneticField`], [`Quantity::WaterKept`], then
    /// [`Quantity::SurfaceAge`] when the model has terrain units. The
    /// Bond albedo's own tree ([`Quantity::BondAlbedo`]) follows the
    /// surface temperature's, whose equilibrium node holds the same
    /// albedo one level shallower.
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
    /// (`thermostat`, `life`, `runaway`, `boiled`, `snowball`, `frozen`,
    /// `co2_build_up`, `methane_trap`).
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
    /// Secular cooling of the mantle and core, W.
    SecularHeat,
    /// Bond albedo: the share of the starlight the body reflects, 0 to 1.
    BondAlbedo,
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
    /// Age of a surface, Gyr: the root's is its largest unit's; a unit's
    /// own age sits under it. Under a Bond albedo, a unit's age that set
    /// it, labelled with the unit's renewal key.
    SurfaceAge,
    /// A part of the surface renewed the same way: its share of the area,
    /// 0 to 1, labelled with its renewal key (`formation`, `plates`,
    /// `episodic`, `heat-pipe`, `ice-lid`, `liquid-cycle`, `volatile-ice`,
    /// `tidal-episode`, see [`super::surface_age::Renewal`]).
    TerrainUnit,
    /// Emissivity of the surface, 0 to 1: below 1 under nitrogen frost.
    Emissivity,
    /// A part of what the Bond albedo weighs: its share of the disc, 0 to
    /// 1, labelled with its key (`ocean`, `land`, `ice`, `crust`,
    /// `frost_n2`, `frost_ch4`, `frost_co2`, `frost_so2`, `cloud`, `deck`);
    /// for `haze` the value is its strength (0 to 1), not a share of the
    /// disc.
    CoverShare,
    /// Thickness of the ice shell, km (its mean over the surface).
    ShellThickness,
    /// Depth of the liquid ocean under the shell, km.
    OceanDepth,
    /// Thickness of the ice shell at the poles, km.
    PolarShell,
    /// Moment of inertia factor `C / (M R²)`.
    MomentOfInertia,
    /// Equilibrium tidal heat of a moon locked in a 2:1 resonance, W.
    ResonantHeat,
    /// The host's tidal dissipation `k₂/Q`, locked on its innermost
    /// round moon (resonance locking).
    HostDissipation,
    /// A condition, in `label`: `thermostat` (the carbonate thermostat
    /// set the CO₂), `life` (its oxygen), `runaway` or `boiled` (the
    /// water ran away by the insolation limit, or the climate boiled it),
    /// `snowball` (frozen over by the ice–albedo feedback until volcanic
    /// CO₂ thaws it), `frozen` (frozen with no warm state to reach),
    /// `co2_build_up` (the CO₂ at the greenhouse's maximum: volcanoes
    /// outgas it faster than weathering draws it down), `co2_condenses`
    /// (that maximum is set by CO₂ condensing high in the air, short of
    /// the carbon inventory: more of it no longer warms the ground) and
    /// `methane_trap`
    /// (the cold keeps methane out of the air), `convecting` (the ice
    /// shell convects under a stagnant lid), `ammonia` (ammonia lowers
    /// the ocean's freezing point), `geysers` (the thin polar shell
    /// cracks through and vents) and `exosphere` (non-thermal loss
    /// exhausted the gas's air, which is left as an exosphere:
    /// [`super::non_thermal_loss`]), `liquid_cycle` (a liquid cycle
    /// renewed the surface), `volatile_ice` (volatile ice comes and goes
    /// on part of it), `tidal_episode` (a past tidal episode renewed part
    /// of it), `volcanic_frost` (the SO₂ air is the vapour of the
    /// volcanic frost), `fallout` (a plume's fallout brightens the
    /// ice) and `not_converged` (the climate's fixed point did not
    /// settle: the temperature is its last estimate, issue #562).
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

    /// The same cause with `value`.
    fn with_value(self, value: f64) -> Self {
        Self { value, ..self }
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
/// K: the surface temperatures are solved to 1e-2 K (see
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
/// pass, the boiled steam it kept, or the water left after a boil that
/// condensed again, see [`super::planet_model`]), which
/// gives what it outgassed, what escape left and the air its climate was
/// solved with (the stored air is after life's oxygen). The surface
/// temperature is decomposed with [`surface_temperature_k`] (the balance
/// of a radiative–convective column, [`super::column`]): its equilibrium
/// temperature (no air, no heat), the greenhouse (with air, minus that)
/// and the internal warming
/// (with air and heat, minus with air only), whose sum must be the stored
/// temperature within [`TEMPERATURE_TOLERANCE_K`].
pub fn explain(context: &Context, model: &PlanetModel) -> Option<Explanation> {
    let heat = internal_heat(context, &model.inputs, &surface_interior(context, model));
    let close = |a: f64, b: f64| (a - b).abs() <= HEAT_TOLERANCE * a.abs().max(b.abs());
    if !close(heat.tidal_w, model.heat.tidal_w)
        || !close(heat.radiogenic_w, model.heat.radiogenic_w)
        || !close(heat.secular_w, model.heat.secular_w)
    {
        return None;
    }
    let steps = model.surface.as_ref().map(|s| replay(context, model, s));
    let mut trees = vec![class_tree(model)];
    if let (Some(surface), Some(steps)) = (&model.surface, &steps) {
        trees.push(temperature_tree(context, model, surface, steps)?);
        trees.push(albedo_tree(context, model, surface, steps, false));
        trees.push(pressure_tree(surface, steps));
        trees.push(liquid_tree(surface));
        trees.push(regime_tree(context, model, surface));
        trees.push(field_tree(context, model, surface));
        trees.push(water_tree(model, surface));
    }
    trees.extend(surface_age_tree(context, model));
    trees.extend(ocean_tree(context, model));
    trees.extend(polar_tree(context, model));
    if model.ice_shell.is_some() || context.tide.is_some() {
        trees.push(Cause::of(
            Quantity::MomentOfInertia,
            model.moment_of_inertia,
        ));
    }
    Some(Explanation { trees })
}

/// The interior the chain ran the heat and the surface steps on: the
/// model's, except a steam world's, whose interior was derived again
/// with its steam afterwards ([`super::steam_interior`]).
pub(crate) fn surface_interior(context: &Context, model: &PlanetModel) -> super::Interior {
    if model
        .interior
        .layers
        .iter()
        .any(|l| l.material == super::Material::Steam)
    {
        super::body_interior(context, &model.inputs)
    } else {
        model.interior.clone()
    }
}

/// The surface steps of `model` run again (see [`bare`]), from the water
/// the chain started the kept pass with: the boiled steam when `surface`
/// boiled and stayed a runaway, its own water when the water left after
/// the boil condensed again, else the water of its first pass.
pub(crate) fn replay(context: &Context, model: &PlanetModel, surface: &Surface) -> Bare {
    let interior = &surface_interior(context, model);
    let (i, fluence) = (&model.inputs, model.xuv_fluence);
    let start = if surface.water.boiled && surface.water.runaway {
        Water {
            boiled: true,
            ..steam(i, interior, fluence)
        }
    } else if surface.water.boiled {
        surface.water
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

/// The tidal heat: a moon in a 2:1 resonance whose pair pumps more than
/// its own tide dissipates ([`super::resonance::todays_heat_w`]) gets it
/// from the host's dissipation, its locked `k₂/Q`, and the age (the
/// pair's orbits are the system's); else the orbit's eccentricity sets
/// it, as the form's field when the orbit is the planet's round the star.
fn tidal_heat(context: &Context, model: &PlanetModel) -> Cause {
    let tidal = Cause::of(Quantity::TidalHeat, model.heat.tidal_w);
    if let Some(resonance) = context.tide.and_then(|t| t.resonance)
        && resonant(context, model)
    {
        return tidal.because(vec![
            Cause::of(Quantity::ResonantHeat, resonance.heat_w).because(vec![
                Cause::of(Quantity::HostDissipation, resonance.host_k2_over_q),
                Cause::of(Quantity::Age, context.age_gyr).set_by(InputField::AgeGyr),
            ]),
        ]);
    }
    // The form's eccentricity is the orbit round the star: a moon's
    // tides come from its planet.
    if context.tide.is_none() {
        tidal.set_by(InputField::Eccentricity)
    } else {
        tidal
    }
}

/// Whether the body's tidal heat is its resonance's: the heat of its
/// pair is more than its own tide makes.
fn resonant(context: &Context, model: &PlanetModel) -> bool {
    let Some(resonant_w) = context.tide.and_then(|t| t.resonance).map(|r| r.heat_w) else {
        return false;
    };
    let tide = super::tide_of(context);
    let own_w = super::heat::tidal_power_w(
        tide.host_kg,
        surface_interior(context, model).radius_earth * crate::exomoon::EARTH_M,
        tide.a_m,
        tide.eccentricity,
        super::heat::k2_over_q(model.inputs.envelope_fraction, model.inputs.water_fraction),
    );
    resonant_w > own_w
}

/// The ocean under the ice, from its shell and the heat flux that sets
/// the shell; `None` for a body without an ocean under ice. A convecting
/// shell and an ocean whose ammonia lowers its freezing point say so.
fn ocean_tree(context: &Context, model: &PlanetModel) -> Option<Cause> {
    let shell = model.ice_shell.filter(|s| s.ocean_km > 0.0)?;
    let flux = Cause::of(
        Quantity::HeatFlux,
        model
            .heat
            .flux_w_m2(surface_interior(context, model).radius_earth),
    );
    let mut causes = vec![flux];
    if shell.convecting {
        causes.push(Cause::note("convecting"));
    }
    if model.inputs.volatiles.nh3 > 0.0 {
        causes.push(Cause::note("ammonia"));
    }
    Some(
        Cause::of(Quantity::OceanDepth, shell.ocean_km).because(vec![
            Cause::of(Quantity::ShellThickness, shell.shell_km).because(causes),
        ]),
    )
}

/// The shell at the poles, thinned by the tides, with the heat of those
/// tides; the geysers it vents through when it cracks. `None` unless the
/// poles are thinner than the mean.
fn polar_tree(context: &Context, model: &PlanetModel) -> Option<Cause> {
    let shell = model
        .ice_shell
        .filter(|s| s.ocean_km > 0.0 && s.polar_shell_km > 0.0 && s.polar_shell_km < s.shell_km)?;
    let mut causes = vec![tidal_heat(context, model)];
    if model.geysers.is_some() {
        causes.push(Cause::note("geysers"));
    }
    Some(Cause::of(Quantity::PolarShell, shell.polar_shell_km).because(causes))
}

fn heat_flux(context: &Context, model: &PlanetModel) -> Cause {
    Cause::of(
        Quantity::HeatFlux,
        model
            .heat
            .flux_w_m2(surface_interior(context, model).radius_earth),
    )
    .because(vec![
        Cause::of(Quantity::RadiogenicHeat, model.heat.radiogenic_w).set_by(InputField::Radiogenic),
        tidal_heat(context, model),
        Cause::of(Quantity::SecularHeat, model.heat.secular_w),
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
    // The climate's light, under the Bond albedo it settled on.
    let context = &context.with_albedo(surface.climate.bond_albedo);
    let flux = model
        .heat
        .flux_w_m2(surface_interior(context, model).radius_earth);
    // The surface radiates with the emissivity the climate settled on
    // (below 1 under nitrogen frost): the equilibrium temperature here is
    // the surface's, `T_eq ε^-¼`.
    let emissivity = surface.climate.emissivity;
    let bare_k = surface_temperature_k(context, &Gases::default(), 0.0, emissivity);
    let air_k = surface_temperature_k(context, gases, 0.0, emissivity);
    // Water whose vapour followed the temperature was solved with it: its
    // balance is recomputed the same way, since the column with the
    // vapour held fixed can balance at another temperature (it jumps
    // where the vapour starts to condense in it).
    let wet = !surface.water.runaway && surface.water.kept_kg > 0.0;
    let full_k = if wet && !surface.climate.regulated {
        let bar_per_kg = super::atmosphere::bar_per_kg(
            model.inputs.mass_earth,
            &surface_interior(context, model),
        );
        wet_temperature_k(
            context,
            gases,
            flux,
            surface.water.kept_kg * bar_per_kg,
            emissivity,
        )
    } else {
        surface_temperature_k(context, gases, flux, emissivity)
    };
    let stored = surface.climate.temperature_k;
    if (full_k - stored).abs() > TEMPERATURE_TOLERANCE_K {
        return None;
    }
    let co2 = Cause::of(Quantity::Co2, gases.co2).set_by(InputField::Co2);
    let climate = &surface.climate;
    let co2 = if climate.regulated {
        co2.because(vec![Cause::note("thermostat")])
    } else if climate.liquid == SurfaceLiquid::Water
        && surface.regime == Regime::Plates
        && !surface.water.runaway
    {
        // Plates and liquid water but no thermostat: the CO₂ is at the
        // most the volcanoes leave, and the greenhouse at its maximum.
        // Short of the carbon inventory, the maximum is CO₂ condensing
        // high in the air (issue #522), not the carbon running out.
        let mut notes = vec![Cause::note("co2_build_up")];
        if gases.co2 < steps.before.carbon_bar * (1.0 - 1e-9) {
            notes.push(Cause::note("co2_condenses"));
        }
        co2.because(notes)
    } else {
        co2
    };
    let ch4 = Cause::of(Quantity::Ch4, gases.ch4).set_by(InputField::Ch4);
    let ch4 = if steps.condensed.ch4 > 0.0 {
        ch4.because(vec![Cause::note("methane_trap")])
    } else {
        ch4
    };
    let albedo = albedo_tree(context, model, surface, steps, true);
    let absorbers = [
        (gases.n2 > 0.0).then(|| Cause::of(Quantity::N2, gases.n2).set_by(InputField::N2)),
        (gases.co2 > 0.0).then_some(co2),
        (gases.h2o > 0.0)
            .then(|| Cause::of(Quantity::H2o, gases.h2o).set_by(InputField::WaterOceans)),
        (gases.ch4 > 0.0).then_some(ch4),
    ];
    // A climate whose fixed points did not settle shows the last
    // estimate of its cycle (issue #562).
    let unsettled = (!climate.converged).then(|| Cause::note("not_converged"));
    Some(
        Cause::of(Quantity::SurfaceTemperature, stored).because(
            [
                Some(
                    Cause::of(Quantity::EquilibriumTemperature, bare_k).because(
                        [
                            Some(
                                Cause::of(Quantity::LightReceived, context.insolation)
                                    .set_by(InputField::Insolation),
                            ),
                            Some(albedo),
                            (emissivity < 1.0).then(|| Cause::of(Quantity::Emissivity, emissivity)),
                        ]
                        .into_iter()
                        .flatten()
                        .collect(),
                    ),
                ),
                Some(
                    Cause::of(Quantity::Greenhouse, air_k - bare_k)
                        .because(absorbers.into_iter().flatten().collect()),
                ),
                Some(
                    Cause::of(Quantity::InternalWarming, full_k - air_k)
                        .because(vec![heat_flux(context, model)]),
                ),
                unsettled,
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
    )
}

/// Smallest share of the disc, 0 to 1, that the Bond albedo's tree names:
/// a cover under 1 % moves the albedo by less than a hundredth of the
/// difference between two covers' albedos.
const COVER_SHARE_MIN: f64 = 0.01;

/// The Bond albedo and what sets it: the cover the climate weighed
/// ([`super::albedo::cover`], replayed from the water, the air and the
/// frost the climate used, at its surface temperature) as one
/// [`Quantity::CoverShare`] per part with its share of the disc, largest
/// first. The surface's parts (ocean, land, ice, a frost) and the
/// sulphuric acid deck over them sum to 1; the water clouds and the
/// methane haze are layers over those (the cloud's share of the disc, the
/// haze's strength, which is not a share of it), none named under
/// [`COVER_SHARE_MIN`]. The frosts other than SO₂ weigh one albedo
/// together, so they are one part, named for the one lying on top by the
/// layering of the condensates (N₂, then CH₄, then CO₂: the most volatile
/// present); the frosts beneath it are omitted. SO₂ frost is its own
/// part, with its own albedo. An exposed ice crust
/// ([`icy_albedo_causes`]) replaces the land and the ice (`crust`).
///
/// `nested` is the node under the temperature tree's equilibrium
/// temperature, where a part has no room below it (the tree stays three
/// levels deep); the albedo's own tree (`!nested`) puts under the crust
/// the units that set it and the plume's fallout, and under the SO₂ frost
/// the note of the volcanoes that lay it. (The nested crust has no room
/// for a pointer to the Bond albedo row, which holds those units.)
fn albedo_tree(
    context: &Context,
    model: &PlanetModel,
    surface: &Surface,
    steps: &Bare,
    nested: bool,
) -> Cause {
    let climate = &surface.climate;
    let gases = &steps.climate_gases;
    let cover = super::albedo::cover(
        climate.temperature_k,
        gases,
        &surface.water,
        &steps.frost,
        &steps.planet,
    );
    let deck = super::albedo::sulphuric_deck(
        climate.temperature_k,
        super::climate::washed(*gases, &surface.water).so2,
    )
    .clamp(0.0, 1.0);
    let crust = icy_albedo_causes(context, model, surface, steps);
    let part = |key: &str, share: f64, causes: Vec<Cause>| {
        (share >= COVER_SHARE_MIN).then(|| {
            Cause::keyed(Quantity::CoverShare, key)
                .with_value(share)
                .because(if nested { Vec::new() } else { causes })
        })
    };
    let bare = 1.0 - deck;
    // The climate weighs one albedo for the whole non-SO₂ frost
    // ([`super::albedo::FROST_ALBEDO`]), so its share of the disc is the
    // frosts' together; the card names the frost lying on top, the most
    // volatile present (N₂, CH₄, CO₂ in that order: the layering of the
    // condensates, the coldest on top), and omits those beneath it.
    debug_assert!(
        steps.frost.o2 + steps.frost.h2o + steps.frost.nh3 <= 0.0,
        "a frost of a gas the cover does not weigh: {:?}",
        steps.frost
    );
    let other_frost = (cover.frost - cover.so2_frost).max(0.0);
    let top_frost = [
        ("frost_n2", steps.frost.n2),
        ("frost_ch4", steps.frost.ch4),
        ("frost_co2", steps.frost.co2),
    ]
    .into_iter()
    .find(|&(_, bar)| bar > 0.0)
    .map(|(key, _)| key);
    let on_top = |key: &str| {
        if top_frost == Some(key) {
            other_frost * bare
        } else {
            0.0
        }
    };
    let volcanic = if steps.before.so2_frost_bar > 0.0 {
        vec![Cause::note("volcanic_frost")]
    } else {
        Vec::new()
    };
    let (ice_key, ice_share, ice_causes) = match crust {
        Some(causes) => ("crust", cover.land + cover.ice, causes),
        None => ("ice", cover.ice, Vec::new()),
    };
    let mut surface_parts: Vec<Cause> = [
        part("ocean", cover.ocean * bare, Vec::new()),
        part(
            "land",
            if ice_key == "crust" {
                0.0
            } else {
                cover.land * bare
            },
            Vec::new(),
        ),
        part(ice_key, ice_share * bare, ice_causes),
        part("frost_n2", on_top("frost_n2"), Vec::new()),
        part("frost_ch4", on_top("frost_ch4"), Vec::new()),
        part("frost_co2", on_top("frost_co2"), Vec::new()),
        part("frost_so2", cover.so2_frost * bare, volcanic),
        part("deck", deck, Vec::new()),
    ]
    .into_iter()
    .flatten()
    .collect();
    surface_parts.sort_by(|a, b| b.value.total_cmp(&a.value));
    let layers = [
        part("cloud", cover.cloud, Vec::new()),
        part("haze", super::albedo::haze(gases), Vec::new()),
    ];
    let mut causes = surface_parts;
    causes.extend(layers.into_iter().flatten());
    // Bright ice sets the albedo of a frozen world, and what ends it.
    // Only a body with plates could have built CO₂ up to a thaw: without
    // them (Mars, Titan) the cold has another cause.
    let frozen =
        frozen_over(climate) && surface.water.oceans > 0.0 && surface.regime == Regime::Plates;
    if climate.snowball {
        causes.push(Cause::note("snowball"));
    } else if frozen {
        causes.push(Cause::note("frozen"));
    }
    Cause::of(Quantity::BondAlbedo, climate.bond_albedo).because(causes)
}

/// What set the Bond albedo of an exposed ice crust: the ages of the
/// units it was derived from ([`super::surface_age::icy_estimate`], as
/// [`super::crust_albedo`] reads them) and the plume's fallout when the
/// body vents; `None` when the albedo is the cover's, not the crust's
/// (no ice crust, an air that erodes it, or a frost over the whole disc,
/// as Triton's nitrogen: the crust then has no weight in the albedo,
/// [`super::albedo::bond_albedo_over`]). A frost over part of the disc
/// keeps the units: they set the albedo of the share it leaves bare.
fn icy_albedo_causes(
    context: &Context,
    model: &PlanetModel,
    surface: &Surface,
    steps: &Bare,
) -> Option<Vec<Cause>> {
    if !ice_crust(&model.inputs, &surface.water, surface.regime)
        || steps.climate_gases.total() >= super::interim::ERODING_BAR
        || super::albedo::frost_cover(&steps.frost) >= 1.0
    {
        return None;
    }
    let interior = surface_interior(context, model);
    let units = super::surface_age::icy_estimate(
        &model.heat,
        interior.radius_earth,
        context.age_gyr,
        context.tide.as_ref(),
    );
    let mut causes: Vec<Cause> = units
        .units
        .iter()
        .map(|u| Cause::keyed(Quantity::SurfaceAge, u.renewal).with_value(u.age_gyr))
        .collect();
    let vent = vent_strength(
        &model.inputs,
        &interior,
        &model.heat,
        surface.climate.temperature_k,
        surface.climate.liquid,
    );
    if vent > 0.0 {
        causes.push(Cause::note("fallout"));
    }
    Some(causes)
}

/// The tree of the surface's age: its units (their shares and ages), the
/// age of the system they are bounded by and a note for each renewal that
/// needs its own words. `None` for a model stored without terrain.
///
/// The root holds the main unit's model age, not the card's
/// `surface_age_ga`: the card shows the age the generation drew around it
/// (a spread of `interim::params_for`, about 10 % for a tidal episode,
/// more for a lid), which the model does not store and this replay cannot
/// see. The two agree to that spread; the units' ages are the model's.
fn surface_age_tree(context: &Context, model: &PlanetModel) -> Option<Cause> {
    let main = model.terrain.main()?;
    let mut causes: Vec<Cause> = model
        .terrain
        .units
        .iter()
        .map(|u| {
            Cause::keyed(Quantity::TerrainUnit, u.renewal)
                .with_value(u.share)
                .because(vec![Cause::of(Quantity::SurfaceAge, u.age_gyr)])
        })
        .collect();
    causes.push(Cause::of(Quantity::Age, context.age_gyr).set_by(InputField::AgeGyr));
    for (renewal, key) in [
        (Renewal::LiquidCycle, "liquid_cycle"),
        (Renewal::VolatileIce, "volatile_ice"),
        (Renewal::TidalEpisode, "tidal_episode"),
    ] {
        if model.terrain.units.iter().any(|u| u.renewal == renewal) {
            causes.push(Cause::note(key));
        }
    }
    Some(Cause::of(Quantity::SurfaceAge, main.age_gyr).because(causes))
}

/// Each gas of the air (as stored, after life) from its source and what
/// took from it, each step of the chain in its order: what was outgassed
/// under the regime, what escape took, what the ultraviolet destroyed
/// (or, for N₂, made from ammonia), what the cold froze out (before the
/// climate, or condensed by it at the surface temperature), and the CO₂
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
        let condensed = of(&steps.condensed);
        let lost = of(&steps.lost);
        let destroyed = (kept - photolysed) + (trapped - before).max(0.0);
        let steps = [
            (Quantity::Escape, of(&steps.outgassed) - kept),
            (Quantity::Destroyed, destroyed),
            (Quantity::FrozenOut, photolysed - trapped + condensed),
            (Quantity::FromAmmonia, (before - trapped).max(0.0)),
            (
                Quantity::Weathered,
                before - condensed - of(&steps.climate_gases),
            ),
        ];
        let mut causes = vec![outgassed];
        causes.extend(
            steps
                .into_iter()
                .filter(|(_, bar)| *bar > 0.0)
                .map(|(q, bar)| Cause::of(q, bar)),
        );
        // The escape counts the non-thermal loss too; a gas it exhausted
        // is left as an exosphere ([`super::non_thermal_loss`]).
        if lost > 0.0 {
            causes.push(Cause::note("exosphere"));
        }
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
    // A volcanic body's SO₂ air is the dayside vapour of the frost its
    // volcanoes lay ([`super::atmosphere::volcanic_so2_vapour`]), not an
    // outgassed gas the escape left.
    let frost = steps.before.so2_frost_bar;
    let so2 = if frost > 0.0 {
        vec![
            Cause::of(Quantity::FrozenOut, frost),
            Cause::note("volcanic_frost"),
        ]
    } else {
        source(|g| g.so2, None)
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
        (Quantity::So2, g.so2, so2),
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
    /// no envelope, CO₂ and N₂ on their rule, Earth's radiogenic
    /// abundance, life, no snowball), the rest drawn.
    fn earth() -> (Context, PlanetModel) {
        earth_at(1.0, false)
    }

    /// An Earth-like body `au` from the Sun, a snowball when asked.
    fn earth_at(au: f64, snowball: bool) -> (Context, PlanetModel) {
        let context = sun_at(au);
        let overrides = InputOverrides {
            core_mass_fraction: Some(0.325),
            water_oceans: Some(1.0),
            envelope_fraction: Some(0.0),
            co2: Some(1.0),
            n2: Some(1.0),
            ch4: None,
            nh3: None,
            s: None,
            radiogenic: Some(1.0),
            life: Some(true),
            snowball: Some(snowball),
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

    /// A round moon of a giant `host_kg` heavy, `orbit_au` from the Sun,
    /// formed as `birth` says; its context and model, as the system's draw
    /// gives them.
    fn moon_of(
        orbit_au: f64,
        (mass_earth, ice_fraction): (f64, f64),
        birth: crate::planet::moon::MoonBirth,
        tide: crate::planet::HostTide,
    ) -> (Context, PlanetModel) {
        let formation = crate::planet::moon::MoonFormation {
            mass_earth,
            ice_fraction,
            birth,
            tide,
            capture_age_share: None,
        };
        let planet = sun_at(orbit_au);
        let model = crate::planet::moon::derive_moon_model(
            1,
            "p1/m1",
            &planet,
            &formation,
            &InputOverrides::default(),
        );
        (planet.with_tide(tide), model)
    }

    fn tide(host_kg: f64, a_m: f64, eccentricity: f64) -> crate::planet::HostTide {
        crate::planet::HostTide {
            host_kg,
            a_m,
            eccentricity,
            resonance: None,
        }
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
    fn explain_has_an_ocean_tree_for_europa() {
        let (context, model) = solar("Europa");
        let shell = model.ice_shell.expect("Europa has an ice shell");
        assert!(shell.ocean_km > 0.0, "{shell:?}");
        let e = explain(&context, &model).expect("explained");
        let ocean = root(&e, Quantity::OceanDepth);
        assert!((ocean.value - shell.ocean_km).abs() < 1e-9);
        let thick = find(ocean, Quantity::ShellThickness).expect("the shell sets the ocean");
        assert!(find(thick, Quantity::HeatFlux).is_some(), "{ocean:#?}");
        let inertia = root(&e, Quantity::MomentOfInertia);
        assert_eq!(inertia.value, model.moment_of_inertia);
        for tree in &e.trees {
            assert!(depth(tree) <= 3, "{:?}: {tree:#?}", tree.quantity);
        }
    }

    /// Callisto round Jupiter: its CO₂ goes to the plasma, and the
    /// pressure tree says the gas is left as an exosphere.
    #[test]
    fn an_exosphere_says_so() {
        let (context, model) = solar("Callisto");
        let context = context.with_tide(crate::planet::HostTide {
            host_kg: 1.898e27,
            a_m: 1.883e9,
            eccentricity: 0.0074,
            resonance: None,
        });
        let model = planet_model(&context, model.inputs);
        let e = explain(&context, &model).expect("explained");
        let pressure = root(&e, Quantity::Pressure);
        assert!(pressure.value < 1e-10, "{pressure:#?}");
        let co2 = find(pressure, Quantity::Co2).expect("its CO₂");
        assert!(
            co2.causes
                .iter()
                .any(|c| c.quantity == Quantity::Note && c.label.as_deref() == Some("exosphere")),
            "{co2:#?}"
        );
        assert!(find(co2, Quantity::Escape).is_some(), "{co2:#?}");
    }

    /// Ganymede's surface age has a tree: one child per terrain unit with
    /// its renewal as the key, the age of the system, and the tidal
    /// episode's note.
    #[test]
    fn explain_has_a_surface_age_tree_for_ganymede() {
        use crate::planet::moon::MoonBirth;
        let tide = crate::planet::HostTide {
            resonance: Some(crate::planet::resonance::Resonance {
                heat_w: 0.0,
                host_k2_over_q: 8.5e-6,
                laplace_like: true,
                capture_age_share: None,
            }),
            ..tide(1.898e27, 1.0704e9, 0.0013)
        };
        let (context, model) = moon_of(5.2, (0.0248, 0.45), MoonBirth::Disk, tide);
        assert!(model.terrain.units.len() >= 2, "{:?}", model.terrain);
        let e = explain(&context, &model).expect("explained");
        let tree = root(&e, Quantity::SurfaceAge);
        let units: Vec<_> = tree
            .causes
            .iter()
            .filter(|c| c.quantity == Quantity::TerrainUnit)
            .collect();
        assert_eq!(units.len(), model.terrain.units.len(), "{tree:#?}");
        for (cause, unit) in units.iter().zip(&model.terrain.units) {
            assert_eq!(cause.value, unit.share);
            let key = serde_json::to_value(unit.renewal).unwrap();
            assert_eq!(cause.label.as_deref(), key.as_str());
            let age = find(cause, Quantity::SurfaceAge).expect("the unit's age");
            assert_eq!(age.value, unit.age_gyr);
        }
        assert!(find(tree, Quantity::Age).is_some(), "{tree:#?}");
        assert!(has_note(tree, "tidal_episode"), "{tree:#?}");
        assert!(depth(tree) <= 3, "{tree:#?}");
    }

    /// An icy moon's Bond albedo takes its terrain units' ages as causes;
    /// Earth's, set by what covers it, takes none.
    #[test]
    fn an_icy_albedo_names_the_surface_ages_that_set_it() {
        let (context, model) = solar("Europa");
        let e = explain(&context, &model).expect("explained");
        let albedo = root(&e, Quantity::BondAlbedo);
        assert!(find(albedo, Quantity::SurfaceAge).is_some(), "{albedo:#?}");
        let nested = find(root(&e, Quantity::SurfaceTemperature), Quantity::BondAlbedo)
            .expect("the temperature tree's albedo");
        assert_eq!(nested.value, albedo.value);
        let (context, model) = earth();
        let e = explain(&context, &model).unwrap();
        let albedo = root(&e, Quantity::BondAlbedo);
        assert!(find(albedo, Quantity::SurfaceAge).is_none(), "{albedo:#?}");
        assert!(cover_part(albedo, "ocean").is_some(), "{albedo:#?}");
    }

    /// Triton's nitrogen frost lowers its emissivity: the equilibrium
    /// temperature says so.
    #[test]
    fn nitrogen_frost_gives_the_equilibrium_an_emissivity() {
        let (context, model) = moon_of(
            30.07,
            (0.0036, 0.3),
            crate::planet::moon::MoonBirth::Captured,
            tide(1.024e26, 3.5476e8, 0.0),
        );
        let e = explain(&context, &model).expect("explained");
        let t = root(&e, Quantity::SurfaceTemperature);
        let eq = find(t, Quantity::EquilibriumTemperature).unwrap();
        let em = find(eq, Quantity::Emissivity).expect("an emissivity cause");
        assert!(em.value < 1.0 && em.value >= 0.89, "{em:?}");
        let (context, model) = earth();
        let e = explain(&context, &model).unwrap();
        let eq = find(
            root(&e, Quantity::SurfaceTemperature),
            Quantity::EquilibriumTemperature,
        )
        .unwrap();
        assert!(find(eq, Quantity::Emissivity).is_none());
    }

    /// The albedo node names the surface's age only where the age sets
    /// the albedo (spec goal 8): Triton's 0.747 is its nitrogen frost's,
    /// which covers its whole disc, so its node names no terrain unit,
    /// while Ganymede's, an exposed crust, names its units.
    #[test]
    fn a_frosted_disc_takes_no_age_in_its_albedo() {
        use crate::system::{SystemRequest, WholeSystemRequest, body_context, draw};
        use rand::SeedableRng;
        let drawn = |arch: &str, seed: u32| {
            let request = SystemRequest::System(WholeSystemRequest {
                planets: Some(std::collections::BTreeMap::from([(arch.to_string(), 1)])),
                seed: Some(seed),
                ..WholeSystemRequest::default()
            });
            let system = draw(
                &request,
                &mut rand::rngs::StdRng::seed_from_u64(u64::from(seed)),
            )
            .expect("the system draws");
            let body = system
                .bodies
                .iter()
                .find(|b| b.params.archetype.as_deref() == Some(arch))
                .expect("the asked body")
                .clone();
            let context = body_context(&system, &body).expect("a context");
            let model = body.model.expect("a model");
            explain(&context, &model).expect("explained")
        };
        for seed in [1, 2, 3] {
            let e = drawn("triton-like", seed);
            let albedo = root(&e, Quantity::BondAlbedo);
            assert!(
                find(albedo, Quantity::SurfaceAge).is_none(),
                "seed {seed}: {albedo:#?}"
            );
            let e = drawn("ganymede-like", seed);
            let albedo = root(&e, Quantity::BondAlbedo);
            assert!(
                find(albedo, Quantity::SurfaceAge).is_some(),
                "seed {seed}: {albedo:#?}"
            );
        }
    }

    /// A body drawn from `arch` with `seed`, its context, model and
    /// explanation.
    fn drawn_body(arch: &str, seed: u32) -> (Context, PlanetModel, Explanation) {
        use crate::system::{SystemRequest, WholeSystemRequest, body_context, draw};
        use rand::SeedableRng;
        let request = SystemRequest::System(WholeSystemRequest {
            planets: Some(std::collections::BTreeMap::from([(arch.to_string(), 1)])),
            seed: Some(seed),
            ..WholeSystemRequest::default()
        });
        let system = draw(
            &request,
            &mut rand::rngs::StdRng::seed_from_u64(u64::from(seed)),
        )
        .expect("the system draws");
        let body = system
            .bodies
            .iter()
            .find(|b| b.params.archetype.as_deref() == Some(arch))
            .expect("the asked body")
            .clone();
        let context = body_context(&system, &body).expect("a context");
        let model = body.model.expect("a model");
        let e = explain(&context, &model).expect("explained");
        (context, model, e)
    }

    /// The part of an albedo tree under `key`.
    fn cover_part<'a>(albedo: &'a Cause, key: &str) -> Option<&'a Cause> {
        albedo
            .causes
            .iter()
            .find(|c| c.quantity == Quantity::CoverShare && c.label.as_deref() == Some(key))
    }

    /// The surface's parts and the deck: what sums to the whole disc (the
    /// clouds and the haze are layers over it).
    fn disc_sum(albedo: &Cause) -> f64 {
        albedo
            .causes
            .iter()
            .filter(|c| {
                c.quantity == Quantity::CoverShare
                    && !matches!(c.label.as_deref(), Some("cloud" | "haze"))
            })
            .map(|c| c.value)
            .sum()
    }

    /// Triton's whole disc is frost (the volatiles of its cold air, one
    /// part per gas); Io's is SO₂ frost, the
    /// volcanoes' (with its note); neither names land or ice.
    #[test]
    fn a_frosted_disc_names_its_frost() {
        for seed in [1, 2, 3] {
            let (_, model, e) = drawn_body("triton-like", seed);
            let albedo = root(&e, Quantity::BondAlbedo);
            let frost: f64 = ["frost_n2", "frost_ch4", "frost_co2"]
                .iter()
                .filter_map(|k| cover_part(albedo, k))
                .map(|c| c.value)
                .sum();
            assert!(frost > 0.95, "seed {seed}: {albedo:#?}");
            assert!(cover_part(albedo, "land").is_none(), "{albedo:#?}");
            assert_eq!(albedo.value, model.surface.unwrap().climate.bond_albedo);
            let (_, _, e) = drawn_body("io-like", seed);
            let albedo = root(&e, Quantity::BondAlbedo);
            let frost = cover_part(albedo, "frost_so2").expect("SO₂ frost");
            assert!(frost.value > 0.95, "seed {seed}: {albedo:#?}");
            assert!(has_note(frost, "volcanic_frost"), "{frost:#?}");
        }
    }

    /// Titan's methane haze is a layer of its own.
    #[test]
    fn titan_names_its_haze() {
        let (_, _, e) = drawn_body("titan-like", 1);
        let albedo = root(&e, Quantity::BondAlbedo);
        let haze = cover_part(albedo, "haze").expect("the haze");
        assert!(haze.value > 0.1, "{albedo:#?}");
        assert!(depth(albedo) <= 3);
    }

    /// Ganymede's exposed crust keeps its two terrain units and the
    /// plumes' note stays where it was; the temperature tree's albedo
    /// names the same parts without them (it has no room below).
    #[test]
    fn an_icy_crust_names_its_units() {
        let (_, _, e) = drawn_body("ganymede-like", 1);
        let albedo = root(&e, Quantity::BondAlbedo);
        let crust = cover_part(albedo, "crust").expect("the crust");
        let units = crust
            .causes
            .iter()
            .filter(|c| c.quantity == Quantity::SurfaceAge)
            .count();
        assert!(units >= 2, "{albedo:#?}");
        let nested = find(root(&e, Quantity::SurfaceTemperature), Quantity::BondAlbedo).unwrap();
        let nested_crust = cover_part(nested, "crust").expect("the crust, nested");
        assert!(nested_crust.causes.is_empty());
        assert_eq!(nested_crust.value, crust.value);
        assert!(depth(root(&e, Quantity::SurfaceTemperature)) <= 3);
    }

    /// Earth's albedo is its oceans, land, ice and clouds.
    #[test]
    fn earth_names_its_clouds_oceans_and_land() {
        let (_, _, e) = drawn_body("earth-like", 1);
        let albedo = root(&e, Quantity::BondAlbedo);
        for key in ["ocean", "land", "cloud"] {
            assert!(cover_part(albedo, key).is_some(), "{key}: {albedo:#?}");
        }
        assert!(cover_part(albedo, "frost_n2").is_none());
    }

    /// The parts are the cover the climate weighed: they sum to the disc
    /// and the albedo they weigh is the stored one.
    #[test]
    fn the_shares_are_the_cover_the_climate_used() {
        for arch in [
            "earth-like",
            "triton-like",
            "io-like",
            "titan-like",
            "ganymede-like",
        ] {
            for seed in [1, 2] {
                let (context, model, e) = drawn_body(arch, seed);
                let albedo = root(&e, Quantity::BondAlbedo);
                let sum = disc_sum(albedo);
                assert!((0.97..=1.03).contains(&sum), "{arch} {seed}: {sum}");
                let surface = model.surface.as_ref().unwrap();
                let steps = replay(&context, &model, surface);
                let cover = super::super::albedo::cover(
                    surface.climate.temperature_k,
                    &steps.climate_gases,
                    &surface.water,
                    &steps.frost,
                    &steps.planet,
                );
                let deck = super::super::albedo::sulphuric_deck(
                    surface.climate.temperature_k,
                    super::super::climate::washed(steps.climate_gases, &surface.water).so2,
                );
                let crust = cover_part(albedo, "crust").map(|_| {
                    let interior = surface_interior(&context, &model);
                    let vent = vent_strength(
                        &model.inputs,
                        &interior,
                        &model.heat,
                        surface.climate.temperature_k,
                        surface.climate.liquid,
                    );
                    super::super::crust_albedo(
                        &context,
                        &model.inputs,
                        &model.heat,
                        &interior,
                        vent,
                    )
                });
                let weighed = super::super::albedo::bond_albedo_over(
                    &cover,
                    &steps.climate_gases,
                    deck,
                    crust,
                    &steps.planet,
                    context.teff_k,
                );
                // The climate's fixed point stops when its temperature moves
                // under `ALBEDO_TOLERANCE_K` (0.1 K), so its albedo is the
                // cover's (its ice line, its frost's layer) at a temperature
                // that near the stored one: 1e-3 left on Earth, 3e-3 on
                // Triton, whose frost is just short of its whole disc. The
                // replay is exact only to that fixed point.
                assert!(
                    (weighed - surface.climate.bond_albedo).abs() < 5e-3,
                    "{arch} {seed}: {weighed} vs {}",
                    surface.climate.bond_albedo
                );
                if let Some(cloud) = cover_part(albedo, "cloud") {
                    assert!((cloud.value - cover.cloud).abs() < 1e-12);
                }
                for tree in &e.trees {
                    assert!(depth(tree) <= 3, "{arch}: {:?}", tree.quantity);
                }
            }
        }
    }

    /// Mixed frosts name the one lying on top (the most volatile), once,
    /// with the share of all the non-SO₂ frost; the albedo is unchanged.
    #[test]
    fn a_mixed_frost_names_the_one_on_top() {
        for seed in 1..=6 {
            let (context, model, e) = drawn_body("triton-like", seed);
            let albedo = root(&e, Quantity::BondAlbedo);
            let surface = model.surface.as_ref().unwrap();
            let steps = replay(&context, &model, surface);
            let frosts: Vec<&Cause> = albedo
                .causes
                .iter()
                .filter(|c| c.label.as_deref().is_some_and(|l| l.starts_with("frost_")))
                .collect();
            assert_eq!(frosts.len(), 1, "seed {seed}: {albedo:#?}");
            let cover = super::super::albedo::cover(
                surface.climate.temperature_k,
                &steps.climate_gases,
                &surface.water,
                &steps.frost,
                &steps.planet,
            );
            assert!((frosts[0].value - cover.frost).abs() < 1e-9);
            if steps.frost.n2 > 0.0 {
                assert_eq!(frosts[0].label.as_deref(), Some("frost_n2"), "{seed}");
            } else if steps.frost.ch4 > 0.0 {
                assert_eq!(frosts[0].label.as_deref(), Some("frost_ch4"), "{seed}");
            }
            if steps.frost.n2 > 0.0 || steps.frost.ch4 > 0.0 {
                assert_ne!(frosts[0].label.as_deref(), Some("frost_co2"), "{seed}");
            }
        }
    }

    /// Io's SO₂ is the volcanic frost's vapour: its node names the frost,
    /// not the outgassing and the escape.
    #[test]
    fn io_says_its_so2_air_is_the_frosts_vapour() {
        let (context, model) = moon_of(
            5.2,
            (0.015, 0.0),
            crate::planet::moon::MoonBirth::Disk,
            tide(1.898e27, 4.217e8, 0.0041),
        );
        let e = explain(&context, &model).expect("explained");
        let p = root(&e, Quantity::Pressure);
        let so2 = find(p, Quantity::So2).expect("Io's SO₂");
        assert!(has_note(so2, "volcanic_frost"), "{so2:#?}");
        assert!(find(so2, Quantity::Escape).is_none(), "{so2:#?}");
    }

    #[test]
    fn a_resonant_moon_says_where_its_tidal_heat_comes_from() {
        let (context, model) = solar("Europa");
        let context = context.with_tide(crate::planet::HostTide {
            host_kg: 1.898e27,
            a_m: 6.709e8,
            eccentricity: 0.0,
            resonance: Some(crate::planet::resonance::Resonance {
                heat_w: 1e15,
                host_k2_over_q: 8.5e-6,
                laplace_like: true,
                capture_age_share: None,
            }),
        });
        let model = planet_model(&context, model.inputs);
        let e = explain(&context, &model).expect("explained");
        let resonant = e
            .trees
            .iter()
            .find_map(|t| find(t, Quantity::ResonantHeat))
            .expect("the tidal heat comes from the resonance");
        assert_eq!(resonant.value, 1e15);
        // The host's locked k2/Q, as a value (#533).
        let host = find(resonant, Quantity::HostDissipation).expect("the host's dissipation");
        assert_eq!(host.value, 8.5e-6);
        assert_eq!(host.label, None);
        assert!(find(resonant, Quantity::Age).is_some());
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

    /// The first cause of `quantity` anywhere under `c`.
    fn find(c: &Cause, quantity: Quantity) -> Option<&Cause> {
        if c.quantity == quantity {
            return Some(c);
        }
        c.causes.iter().find_map(|c| find(c, quantity))
    }

    /// Whether a note of `key` stands anywhere under `c`.
    fn has_note(c: &Cause, key: &str) -> bool {
        (c.quantity == Quantity::Note && c.label.as_deref() == Some(key))
            || c.causes.iter().any(|c| has_note(c, key))
    }

    #[test]
    fn the_temperature_tree_shows_the_bond_albedo() {
        let (context, model) = earth();
        let e = explain(&context, &model).expect("explained");
        let t = root(&e, Quantity::SurfaceTemperature);
        let albedo = find(t, Quantity::BondAlbedo).expect("a BondAlbedo cause");
        assert_eq!(albedo.value, model.surface.unwrap().climate.bond_albedo);
        assert!(!has_note(t, "snowball"));
    }

    #[test]
    fn a_climate_that_did_not_settle_says_so() {
        let (context, mut model) = earth();
        let e = explain(&context, &model).expect("explained");
        assert!(!has_note(
            root(&e, Quantity::SurfaceTemperature),
            "not_converged"
        ));
        model.surface.as_mut().unwrap().climate.converged = false;
        let e = explain(&context, &model).expect("explained");
        assert!(has_note(
            root(&e, Quantity::SurfaceTemperature),
            "not_converged"
        ));
    }

    #[test]
    fn a_snowball_says_why_it_froze() {
        let (context, model) = earth_at(1.0, true);
        let s = model.surface.as_ref().unwrap();
        assert!(s.climate.snowball, "{:?}", s.climate);
        let e = explain(&context, &model).expect("explained");
        assert!(has_note(root(&e, Quantity::SurfaceTemperature), "snowball"));
    }

    #[test]
    fn a_planet_at_its_maximum_greenhouse_says_its_co2_builds_up() {
        // Just inside the outer edge, past where the thermostat holds:
        // its CO₂ is the maximum greenhouse, which CO₂ condensing aloft
        // sets well short of its carbon (issue #522). The band is narrow,
        // so it is looked for.
        let (context, model) = (0..=60)
            .map(|i| earth_at(1.70 + 0.002 * f64::from(i), false))
            .find(|(_, m)| {
                let c = m.surface.as_ref().unwrap().climate;
                !c.regulated && c.liquid == SurfaceLiquid::Water
            })
            .expect("a liquid planet past the thermostat");
        let e = explain(&context, &model).expect("explained");
        let t = root(&e, Quantity::SurfaceTemperature);
        assert!(has_note(t, "co2_build_up"), "{t:#?}");
        assert!(has_note(t, "co2_condenses"), "{t:#?}");
        let (context, model) = earth();
        let e = explain(&context, &model).unwrap();
        let t = root(&e, Quantity::SurfaceTemperature);
        assert!(!has_note(t, "co2_build_up"));
        assert!(!has_note(t, "co2_condenses"));
    }

    #[test]
    fn a_planet_past_the_outer_edge_says_it_is_frozen_for_good() {
        let (context, model) = earth_at(1.8, false);
        let c = model.surface.as_ref().unwrap().climate;
        assert!(!c.snowball && c.liquid != SurfaceLiquid::Water, "{c:?}");
        let e = explain(&context, &model).expect("explained");
        let t = root(&e, Quantity::SurfaceTemperature);
        assert!(has_note(t, "frozen") && !has_note(t, "snowball"), "{t:#?}");
    }

    #[test]
    fn a_frozen_state_no_co2_thaws_says_frozen_for_good() {
        // Drawn frozen where no CO₂ the planet can hold thaws it: frozen
        // for good, not a snowball waiting for its thaw.
        let (context, model) = earth_at(1.6, true);
        let c = model.surface.as_ref().unwrap().climate;
        assert!(!c.snowball && c.liquid != SurfaceLiquid::Water, "{c:?}");
        let e = explain(&context, &model).expect("explained");
        let t = root(&e, Quantity::SurfaceTemperature);
        assert!(has_note(t, "frozen") && !has_note(t, "snowball"), "{t:#?}");
    }

    #[test]
    fn mars_and_titan_are_not_frozen_for_good() {
        for name in ["Mars", "Titan"] {
            let (context, model) = solar(name);
            let e = explain(&context, &model).expect("explained");
            let t = root(&e, Quantity::SurfaceTemperature);
            assert!(!has_note(t, "frozen"), "{name}: {t:#?}");
        }
    }

    #[test]
    fn titan_says_its_methane_is_cold_trapped() {
        let (context, model) = solar("Titan");
        let e = explain(&context, &model).expect("explained");
        let t = root(&e, Quantity::SurfaceTemperature);
        let ch4 = find(t, Quantity::Ch4).expect("methane in the greenhouse");
        assert!(has_note(ch4, "methane_trap"), "{t:#?}");
    }

    #[test]
    fn trees_are_at_most_three_levels_deep() {
        let bodies = [
            earth(),
            earth_at(1.0, true),
            earth_at(1.6, false),
            earth_at(1.8, false),
            earth_at(1.6, true),
            solar("Venus"),
            solar("Mars"),
            solar("Titan"),
        ];
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
