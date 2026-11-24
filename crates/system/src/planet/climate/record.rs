//! The climate of a planet with a surface, recorded in the cause trace:
//! every value the solver settles ([`Solved`]) with the nodes it derives
//! from, so that a reader can follow the temperature back to the light,
//! the air, the water, the heat and the body that set it. The values are
//! the solver's, never recomputed here: one code path for each.

use super::Rotation;
use super::branches::Solved;
use crate::planet::bare::Bare;
use crate::planet::codes;
use crate::planet::interior::InteriorError;
use science::radiation::gases::PartialPressures;
use science::steam::hides_surface;
use science::surface::Liquid;
use science::units::{Bar, Fraction, Kelvin};
use std::collections::BTreeMap;
use timeline::{
    Atmosphere, Code, Gas, GlobalClimate, LiquidKind, NodeId, Provenance, Trace, Traced,
};

/// The recorded climate of a planet with a surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Climate {
    /// The temperature, equilibrium temperature, Bond albedo and state.
    pub global: GlobalClimate,
    /// The emissivity the surface radiated under.
    pub emissivity: Traced<Fraction>,
    /// Whether the carbonate thermostat holds the temperature.
    pub regulated: Traced<bool>,
    /// Whether the fixed points settled within the solver's tolerance.
    pub converged: Traced<bool>,
    /// The air the climate leaves.
    pub atmosphere: Atmosphere,
    /// The N2 and CH4 on the ground, recorded when positive.
    pub condensate: BTreeMap<Gas, Traced<Bar>>,
    /// What fills the oceans; `None` without a liquid. The inventory and
    /// the ocean share of `SurfaceLiquid` are 2d-4's.
    pub liquid: Option<Traced<LiquidKind>>,
    /// The snowball branch of a bistable body.
    pub snowball: Option<SnowballNodes>,
}

/// The nodes of a bistable body's frozen branch and of the draw that
/// picks between the branches.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SnowballNodes {
    /// The CO2 the temperate branch froze with.
    pub froze: Traced<Bar>,
    /// Whether some CO2 the body can hold thaws it.
    pub thaws: Traced<bool>,
    /// The least CO2 known to thaw it; `None` when none does.
    pub thaw: Option<Traced<Bar>>,
    /// The share of its history it spends frozen.
    pub chance: Traced<Fraction>,
    /// The draw, uniform in (0, 1).
    pub draw: Traced<Fraction>,
}

/// The liquid of the contract for the liquid of the solver.
fn kind(liquid: Liquid) -> LiquidKind {
    match liquid {
        Liquid::Water => LiquidKind::Water,
        Liquid::Hydrocarbons => LiquidKind::Hydrocarbons,
        Liquid::Nitrogen => LiquidKind::Nitrogen,
        Liquid::Lava => LiquidKind::Lava,
    }
}

/// `air` of the gas `gas`, bar.
fn partial(air: &PartialPressures, gas: Gas) -> f64 {
    match gas {
        Gas::N2 => air.n2,
        Gas::Co2 => air.co2,
        Gas::H2o => air.h2o,
        Gas::Ch4 => air.ch4,
        Gas::Nh3 => air.nh3,
        Gas::So2 => air.so2,
        Gas::O2 => air.o2,
        _ => unreachable!("the air has seven gases"),
    }
}

/// Records the climate `solved` of `bare` under `rotation`, with the
/// snowball draw `draw` (uniform in (0, 1), a drawn leaf recorded only for
/// a bistable body). `I`, the inputs every node derives from, are:
///
/// - each source's flux, effective temperature and equilibrium
///   temperature (the warm start of the fixed point reads it,
///   `settle::warm_start`);
/// - the bare air's partial pressures;
/// - the water inventory and whether it ran away;
/// - the three heats;
/// - the radius, the gravity, the regime and the volcanism;
/// - the CO2 outgassed and the CO2 the cold trap froze;
/// - the obliquity and whether the body is locked.
///
/// The nodes derive as follows:
///
/// - `froze-co2`, `thaws` and `thaw-co2` (when it thaws) from `I`; the
///   chance from the thaws flag, the thaw, the CO2 it froze with and the
///   volcanism; the draw is a drawn leaf;
/// - the state from `I`, plus the chance and the draw on a bistable body;
/// - the temperature from `I` and the state;
/// - the albedo, the emissivity, `regulated` and `converged` from the
///   temperature and `I`;
/// - the equilibrium temperature from the albedo and the fluxes;
/// - each partial pressure from its bare partial and the temperature
///   (the CO2 also from the CO2 outgassed and frozen, the vapour also
///   from the water inventory); the pressure from the partials;
/// - each condensate from its bare partial and the temperature;
/// - the liquid from the temperature, the pressure, the water inventory,
///   whether it ran away and whether the body is locked, and the
///   condensates; for a locked body also from each source's flux and the
///   albedo (the substellar temperature).
///
/// # Errors
///
/// The errors of the trace.
///
/// # Panics
///
/// When `bare` has no surface or `draw` is not a fraction.
pub(crate) fn record(
    solved: &Solved,
    bare: &Bare,
    rotation: &Rotation,
    draw: f64,
    trace: &mut Trace,
) -> Result<Climate, InteriorError> {
    let surface = bare
        .surface
        .as_ref()
        .expect("only a planet with a surface has a climate");
    let settle = &solved.settle;
    let bulk = bare.body.bulk();
    let fluxes: Vec<NodeId> = bare.trial.sources.iter().map(|s| s.flux.node()).collect();
    let mut inputs = Vec::new();
    for s in bare.trial.sources.iter() {
        inputs.extend([
            s.flux.node(),
            s.effective_temperature.node(),
            s.equilibrium.node(),
        ]);
    }
    inputs.extend(surface.atmosphere.partial.values().map(Traced::node));
    inputs.extend([
        surface.water.inventory.node(),
        surface.water.runaway.node(),
        bare.heat.radiogenic.node(),
        bare.heat.tidal.node(),
        bare.heat.secular.node(),
        bulk.radius.node(),
        bulk.surface_gravity.node(),
        surface.regime.node(),
        surface.volcanism.node(),
        surface.outgassed_co2.node(),
        surface.frozen_co2.node(),
        rotation.obliquity.node(),
        rotation.locked.node(),
    ]);

    // The snowball branches, the chance and the draw, then the state.
    let mut state_from = inputs.clone();
    let snowball = if let Some(b) = &solved.bistable {
        let froze = trace.record(Code::of(codes::CLIMATE_FROZE_CO2), b.froze, &inputs)?;
        let thaws = trace.record(Code::of(codes::CLIMATE_THAWS), b.thaw.is_some(), &inputs)?;
        let thaw = b
            .thaw
            .map(|thaw| trace.record(Code::of(codes::CLIMATE_THAW_CO2), thaw, &inputs))
            .transpose()?;
        let mut from = vec![thaws.node()];
        from.extend(thaw.map(|t| t.node()));
        from.extend([froze.node(), surface.volcanism.node()]);
        let chance = trace.record(Code::of(codes::CLIMATE_SNOWBALL_CHANCE), b.chance, &from)?;
        let draw = trace.leaf(
            Code::of(codes::CLIMATE_SNOWBALL_DRAW),
            Fraction::new(draw).expect("the snowball draw is a fraction"),
            Provenance::Drawn,
        )?;
        state_from.extend([chance.node(), draw.node()]);
        Some(SnowballNodes {
            froze,
            thaws,
            thaw,
            chance,
            draw,
        })
    } else {
        None
    };
    let state = trace.record(Code::of(codes::CLIMATE_STATE), solved.state, &state_from)?;

    // The settled values.
    let mut from = inputs.clone();
    from.push(state.node());
    let temperature = trace.record(
        Code::of(codes::CLIMATE_SURFACE_TEMPERATURE),
        settle.temperature,
        &from,
    )?;
    let mut from = inputs.clone();
    from.push(temperature.node());
    let bond_albedo = trace.record(
        Code::of(codes::CLIMATE_BOND_ALBEDO),
        settle.bond_albedo,
        &from,
    )?;
    let emissivity = trace.record(
        Code::of(codes::CLIMATE_EMISSIVITY),
        settle.emissivity,
        &from,
    )?;
    let regulated = trace.record(Code::of(codes::CLIMATE_REGULATED), settle.regulated, &from)?;
    let converged = trace.record(Code::of(codes::CLIMATE_CONVERGED), settle.converged, &from)?;
    let mut from = vec![bond_albedo.node()];
    from.extend(&fluxes);
    let equilibrium = trace.record(
        Code::of(codes::CLIMATE_EQUILIBRIUM),
        settle.equilibrium,
        &from,
    )?;

    // The air it leaves, what condenses of it, and the liquid.
    let mut partials = BTreeMap::new();
    for (gas, code) in codes::CLIMATE_AIR_GASES {
        let mut from: Vec<NodeId> = surface
            .atmosphere
            .partial
            .get(&gas)
            .map(Traced::node)
            .into_iter()
            .collect();
        from.push(temperature.node());
        match gas {
            Gas::Co2 => from.extend([surface.outgassed_co2.node(), surface.frozen_co2.node()]),
            Gas::H2o => from.push(surface.water.inventory.node()),
            _ => {}
        }
        let p = trace.record(Code::of(code), Bar(partial(&settle.air, gas)), &from)?;
        partials.insert(gas, p);
    }
    let from: Vec<NodeId> = partials.values().map(Traced::node).collect();
    let surface_pressure = trace.record(
        Code::of(codes::CLIMATE_AIR_PRESSURE),
        Bar(settle.air.total()),
        &from,
    )?;
    let mut condensate = BTreeMap::new();
    for (gas, code) in [
        (Gas::N2, codes::CLIMATE_CONDENSATE_N2),
        (Gas::Ch4, codes::CLIMATE_CONDENSATE_CH4),
    ] {
        let amount = partial(&settle.condensate, gas);
        if amount > 0.0 {
            let mut from: Vec<NodeId> = surface
                .atmosphere
                .partial
                .get(&gas)
                .map(Traced::node)
                .into_iter()
                .collect();
            from.push(temperature.node());
            condensate.insert(gas, trace.record(Code::of(code), Bar(amount), &from)?);
        }
    }
    let liquid = settle
        .liquid
        .map(|liquid| {
            let mut from = vec![
                temperature.node(),
                surface_pressure.node(),
                surface.water.inventory.node(),
                surface.water.runaway.node(),
                rotation.locked.node(),
            ];
            from.extend(condensate.values().map(Traced::node));
            if *rotation.locked.value() {
                from.extend(&fluxes);
                from.push(bond_albedo.node());
            }
            trace.record(Code::of(codes::SURFACE_LIQUID), kind(liquid), &from)
        })
        .transpose()?;

    Ok(Climate {
        global: GlobalClimate {
            surface_temperature: temperature,
            equilibrium,
            bond_albedo,
            state,
        },
        emissivity,
        regulated,
        converged,
        atmosphere: Atmosphere {
            surface_pressure,
            partial: partials,
        },
        condensate,
        liquid,
        snowball,
    })
}

/// Whether the climate's air (`air`, as recorded) is a Titan's, recorded
/// as `planet.climate.titan-like` from the air's nodes and the reducing
/// flag: the test of the reducing-air fallback.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn record_titan_like(
    climate: &Climate,
    air: &PartialPressures,
    reducing: &Traced<bool>,
    trace: &mut Trace,
) -> Result<Traced<bool>, InteriorError> {
    let mut from: Vec<NodeId> = climate
        .atmosphere
        .partial
        .values()
        .map(Traced::node)
        .collect();
    from.push(reducing.node());
    Ok(trace.record(
        Code::of(codes::CLIMATE_TITAN_LIKE),
        science::surface::titan_like_air(air),
        &from,
    )?)
}

/// Whether the water the climate kept boils (`verdict`, from
/// [`super::wet::boils`]), recorded as `planet.climate.boils` from the
/// climate's Bond albedo and emissivity, each partial of its
/// atmosphere, the water's runaway and inventory, the three heats, the
/// bulk's radius and gravity, and each source's flux and effective
/// temperature: everything the verdict reads.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn record_boils(
    climate: &Climate,
    bare: &Bare,
    verdict: bool,
    trace: &mut Trace,
) -> Result<Traced<bool>, InteriorError> {
    let surface = bare
        .surface
        .as_ref()
        .expect("only a planet with a surface has a climate");
    let bulk = bare.body.bulk();
    let mut from = vec![climate.global.bond_albedo.node(), climate.emissivity.node()];
    from.extend(climate.atmosphere.partial.values().map(Traced::node));
    from.extend([
        surface.water.runaway.node(),
        surface.water.inventory.node(),
        bare.heat.radiogenic.node(),
        bare.heat.tidal.node(),
        bare.heat.secular.node(),
        bulk.radius.node(),
        bulk.surface_gravity.node(),
    ]);
    for s in bare.trial.sources.iter() {
        from.extend([s.flux.node(), s.effective_temperature.node()]);
    }
    Ok(trace.record(Code::of(codes::CLIMATE_BOILS), verdict, &from)?)
}

/// Whether a runaway's steam hides its surface
/// ([`science::steam::hides_surface`]), recorded as
/// `planet.climate.hides-surface` from the runaway flag, the H₂O partial
/// and the surface temperature (no partial: 0 bar, from the other two).
/// It takes nodes, not a [`Climate`], so that the steam pass can read the
/// line a second time on the air it re-runs.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn record_hides_surface(
    runaway: Traced<bool>,
    h2o: Option<Traced<Bar>>,
    temperature: Traced<Kelvin>,
    trace: &mut Trace,
) -> Result<Traced<bool>, InteriorError> {
    let steam = h2o.map_or(Bar(0.0), |h| *h.value());
    let mut from = vec![runaway.node()];
    from.extend(h2o.map(|h| h.node()));
    from.push(temperature.node());
    Ok(trace.record(
        Code::of(codes::CLIMATE_HIDES_SURFACE),
        hides_surface(*runaway.value(), steam, *temperature.value()),
        &from,
    )?)
}
