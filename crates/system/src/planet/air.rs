//! The air of a planet with a surface: what its interior outgasses (its
//! radiogenic argon too, study 29), what the cosmic shoreline keeps of
//! it, the water a runaway keeps as vapour, the SO2 its wash-out leaves,
//! what the stars' ultraviolet leaves of its
//! methane, whether the air is reducing (and then outgasses no CO2), what
//! the cold of its equilibrium temperature traps as gas and what the
//! photochemistry makes of it. Ported from
//! `legacy-final:crates/mapgen/src/planet/atmosphere.rs` (`outgassed`,
//! `atmosphere`, `wash_out`, `methane_photolysis`, `reducing`,
//! `without_co2_if_reducing`, `cold_trap`, `photochemistry`),
//! `planet/mod.rs` (`bare_air`, `Water::keeping` in a runaway) and
//! `planet/climate.rs` (`washed`); the laws are in `science::outgassing`,
//! `science::volatiles`, `science::escape` and `science::photochemistry`.
//!
//! The climate's condensation (`condense`, the N2, SO2 and argon frost
//! curves, the methane of the seas) and the reducing-air fallback, which
//! read the surface temperature, are the climate's (`super::climate`, chained by
//! `super::settled`). The non-thermal
//! loss is not in the rewrite (#658): the photolysis reads the air the
//! shoreline kept.

use std::collections::BTreeMap;

use super::codes::{self, AIR_GASES};
use super::core::CoreRecord;
use super::draws::Drawn;
use super::enveloped::{Body, Exposed};
use super::interior::InteriorError;
use crate::Trial;
use science::escape::{escape_velocity, shoreline_kept};
use science::outgassing::{
    LID_SHARE, MOBILE_SHARE, NITROGEN_RECYCLED_UNDER_PLATES, RECYCLED_UNDER_PLATES, SO2_PER_S,
    SULPHUR_IN_AIR, argon_degassed_share, argon_mass, column_pressure, decomposed,
    radiogenic_argon, released_under_ocean, so2_kept,
};
use science::photochemistry::{
    LymanSource, methane_left, methane_survives, nitrogen_from_ammonia, reducing,
};
use science::radiation::gases::Weights;
use science::surface::cold_trapped;
use science::units::{Bar, Dex, Fraction, Kilogram, Metre, WattPerSquareMetre};
use science::volatiles::{disc_temperature, inventory, traps_nitrogen};
use science::xuv::{History, history};
use timeline::{Atmosphere, Code, Gas, NodeId, Provenance, TectonicRegime, Trace, Traced, Water};

/// The air of a planet with a surface, and what its making leaves.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Air {
    /// The air once the photochemistry has run.
    pub atmosphere: Atmosphere,
    /// The water once the air is made: in a runaway, the vapour the
    /// cosmic shoreline kept.
    pub water: Water,
    /// The CO2 outgassed (`planet.air.outgassed.co2`), before the cosmic
    /// shoreline and the cold trap: the carbon inventory weathering and
    /// volcanoes trade with the air, which the climate's carbon cycle
    /// reads (legacy `CarbonCycle::inventory_bar`).
    pub outgassed_co2: Traced<Bar>,
    /// The CO2 the cold trap froze out (`planet.air.frozen-co2`), which
    /// the climate's carbon cycle reads.
    pub frozen_co2: Traced<Bar>,
    /// Whether the air is reducing, Titan's (`planet.air.reducing`): its
    /// CO2 was then held back.
    pub reducing: Traced<bool>,
}

/// The air of the planet `trial` of composition `drawn`, both as they were
/// before a runaway's steam loss (`super::escape::Escaped`), of body
/// `body` after it, with its surface water before its stars' light
/// `surface` (`super::water::surface_water`), its water `water`, its
/// tectonic regime `regime` and its core `core`; a reducing air's CO2
/// held back when `hold_back` (the climate's fallback runs it without).
/// In order (legacy ran the escape twice to hold the CO2 back; the
/// shoreline's share and the photolysis do not read the CO2, so one pass
/// gives the same values):
///
/// 1. **Outgassing** (`planet.air.outgassed.<gas>`, one node per gas):
///    the volatiles at formation ([`inventory`] of the refractory mass
///    `(1 - w)(1 - f) M`, its silicate `(1 - cmf)` share and the ice,
///    the surface water `surface`, before the steam loss so that they
///    keep their mass: the volatiles delivered with the water that is at
///    the surface, a dry planet's share of its bulk water, a water world's
///    whole water; legacy fed the ice term its ocean water, and the
///    mantle's water is not ice; the ice's N2 only where it formed cold
///    enough to trap it, `planet.volatiles.ice-traps-n2`, [`traps_nitrogen`]
///    of the disc's temperature at the orbit,
///    `planet.volatiles.disc-temperature`, [`disc_temperature`]),
///    each times `10^scatter`, the share of the NH3 the interior
///    decomposes coming up as N2 ([`decomposed`]), weigh
///    [`column_pressure`] on the body's
///    surface, times the regime's share ([`LID_SHARE`] for a stagnant lid,
///    [`MOBILE_SHARE`] otherwise), the share the water lets through
///    ([`released`]) and, under plates, what subduction leaves
///    ([`NITROGEN_RECYCLED_UNDER_PLATES`] of the N2 and NH3,
///    [`RECYCLED_UNDER_PLATES`] of the rest). Sulphur comes up as SO2
///    ([`SO2_PER_S`]), of which [`SULPHUR_IN_AIR`] stays. Water is vapour
///    only in a runaway, and then the whole inventory: it is the ocean,
///    not a volatile of the mantle, so neither the regime nor the
///    seafloor holds it back. No O2 (imposed: life makes it). Argon is
///    the Ar-40 the mantle's potassium made ([`radiogenic_argon`] of the
///    interior's mantle mass, the radiogenic abundance and the age, the
///    heat's), times its own share ([`argon_degassed_share`], under a
///    stagnant or episodic lid less) instead of the regime's, and the
///    share the water lets through; subduction does not recycle it. The
///    CH4 comes first: the CO2 waits on the reducing air (step 4).
/// 2. **Shoreline** (`planet.air.shoreline-kept`): the share
///    [`shoreline_kept`] at the escape velocity of the whole mass
///    and the radius of the [`Exposed`] view: the bulk's, with the bulk
///    mass plus a water beyond the laws ([`Body::whole_mass`]); for a
///    steam world's pass on its envelope (`settled::steam_world`), the
///    envelope's radius and its whole mass, core and steam; under the
///    XUV fluence of all the stars, with the dynamo. Each partial
///    (`planet.air.kept.<gas>`) is its outgassed one times that share.
///    The escape velocity has no node (ledger ruling 1): the share
///    derives from the whole mass and the radius, the fluences and the
///    dynamo.
/// 3. **Photolysis** (`planet.air.photolysed.ch4`): [`methane_left`] of
///    the kept CH4 on the bulk surface gravity under every star's present
///    flux and Lyman α history ([`history`] at the star's mass and the
///    trial's metallicity and age), over the trial's age.
/// 4. **Reducing air** (`planet.air.reducing`): [`reducing`] of the
///    photolysed CH4 at the stars' equilibrium temperature
///    (`StellarSources::equilibrium_k`, from each star's `equilibrium`
///    node), false without `hold_back`. A reducing air outgasses no CO2:
///    `planet.air.outgassed.co2` is then 0 bar, from the reducing node.
///    The other gases are outgassed and kept as in steps 1 and 2.
/// 5. **Runaway**: the water inventory becomes the vapour kept, the
///    inventory times the shoreline's share (`planet.water.inventory`,
///    from the inventory and the share); the caller derives the body
///    again from it. Out of a runaway the water is returned as given.
/// 6. **Wash-out** (`planet.air.so2-kept`): [`so2_kept`], wet when the
///    water keeps a wet balance (out of a runaway, some water kept),
///    with the O2's partial pressure and the surface pressure (the sum of
///    the weights) of the air as it stands before it: the kept gases,
///    the photolysed CH4 and the SO2 the shoreline kept; the SO2's weight
///    is the outgassed one times the shoreline's share and this one.
/// 7. **Cold trap** (`planet.air.trapped.co2`, `planet.air.trapped.nh3`):
///    the CO2 and the NH3 (the photolysis leaves them as kept) of the
///    same air with its SO2 washed, each capped at
///    the weight whose partial pressure is its frost pressure at the
///    equilibrium temperature ([`cold_trapped`]), uncapped at or above
///    its triple point; they derive from the whole air. `planet.air.frozen-co2`
///    is the photolysed CO2 less the trapped CO2: legacy books all of it
///    as a frost, which overstates it on small icy bodies (#590).
/// 8. **Photochemistry** (`planet.air.photochemistry.<gas>`): the trapped
///    NH3 becomes N2 ([`nitrogen_from_ammonia`], added to the kept N2)
///    and leaves 0 bar; the photolysed CH4 stays where it survives the OH
///    of water vapour ([`methane_survives`] at the equilibrium
///    temperature), 0 bar otherwise. H2O, SO2, O2 and Ar stay as kept:
///    argon's frost is the climate's (`science::surface::condense`).
/// 9. **Pressure** (`planet.air.pressure`): the sum of the final
///    partials.
///
/// # Errors
///
/// The errors of the trace, and [`InteriorError::Stellar`] when a star
/// has no high-energy history at the trial's age.
#[expect(
    clippy::too_many_arguments,
    reason = "one argument per step of the chain the air reads"
)]
pub(crate) fn air(
    trial: &Trial,
    drawn: &Drawn,
    body: &Body,
    exposed: &Exposed,
    surface: Traced<Kilogram>,
    water: &Water,
    regime: &Traced<TectonicRegime>,
    core: &CoreRecord,
    hold_back: bool,
    trace: &mut Trace,
) -> Result<Air, InteriorError> {
    let traps = ice_traps_nitrogen(trial, trace)?;
    let volatiles = mantle(trial, drawn, body, surface, water, regime, traps);
    let ch4 = volatiles.record(Gas::Ch4, trace)?;
    let shoreline = shoreline(trial, exposed, core, trace)?;
    let share = shoreline.value().get();
    let runaway = *water.runaway.value();
    let keep = |gas: Gas, out: Traced<Bar>, trace: &mut Trace| {
        let (.., code) = AIR_GASES
            .into_iter()
            .find(|&(g, ..)| g == gas)
            .expect("every gas of the air has its codes");
        trace.record(
            Code::of(code),
            Bar(out.value().0 * share),
            &[out.node(), shoreline.node()],
        )
    };

    // Steps 1 to 4: the methane, what the light leaves of it, and whether
    // the air is reducing.
    let mut outgassed = BTreeMap::new();
    let mut partial = BTreeMap::new();
    outgassed.insert(Gas::Ch4, ch4);
    partial.insert(Gas::Ch4, keep(Gas::Ch4, ch4, trace)?);
    let photolysed = photolysed(trial, body, partial[&Gas::Ch4], trace)?;
    let equilibrium = trial.sources.equilibrium_k();
    let cold: Vec<NodeId> = trial.sources.iter().map(|s| s.equilibrium.node()).collect();
    let mut from = vec![photolysed.node()];
    from.extend(&cold);
    let reducing = trace.record(
        Code::of(codes::AIR_REDUCING),
        hold_back && reducing(*photolysed.value(), equilibrium),
        &from,
    )?;
    for gas in [Gas::N2, Gas::Co2, Gas::Nh3, Gas::So2, Gas::Ar] {
        let p = if gas == Gas::Co2 && *reducing.value() {
            trace.record(
                Code::of(codes::AIR_OUTGASSED_CO2),
                Bar(0.0),
                &[reducing.node()],
            )?
        } else {
            volatiles.record(gas, trace)?
        };
        outgassed.insert(gas, p);
    }
    outgassed.extend(water_and_oxygen(body, water, trace)?);
    for (gas, out) in &outgassed {
        if *gas != Gas::Ch4 && *gas != Gas::So2 {
            partial.insert(*gas, keep(*gas, *out, trace)?);
        }
    }

    // Step 5: a runaway keeps its water as the vapour the shoreline kept.
    let kept_water = if runaway {
        Water {
            inventory: trace.record(
                Code::of(codes::WATER_INVENTORY),
                Kilogram(water.inventory.value().0 * share),
                &[water.inventory.node(), shoreline.node()],
            )?,
            ..water.clone()
        }
    } else {
        water.clone()
    };

    // Step 6: the wash-out, on the air as it stands: its methane is the
    // photolysed one, and the SO2 the shoreline keeps, before its
    // wash-out, weighs in the column the wash-out reads.
    let so2 = outgassed[&Gas::So2];
    let so2_before = so2.value().0 * share;
    let standing = Weights {
        ch4: photolysed.value().0,
        so2: so2_before,
        ..weights_of(&partial)
    };
    let wet = !runaway && water.inventory.value().0 > 0.0;
    let mut from: Vec<NodeId> = vec![water.runaway.node(), water.inventory.node()];
    from.extend(
        partial
            .iter()
            .filter(|(gas, _)| **gas != Gas::Ch4)
            .map(|(_, p)| p.node()),
    );
    from.extend([photolysed.node(), so2.node(), shoreline.node()]);
    let washed = trace.record(
        Code::of(codes::AIR_SO2_KEPT),
        so2_kept(
            wet,
            Bar(standing.partial_pressures().o2),
            Bar(standing.total()),
        ),
        &from,
    )?;
    let p = trace.record(
        Code::of(codes::AIR_KEPT_SO2),
        Bar(so2_before * washed.value().get()),
        &[so2.node(), shoreline.node(), washed.node()],
    )?;
    partial.insert(Gas::So2, p);

    // Step 7: the cold trap, at the equilibrium temperature, on the same
    // air with its SO2 washed.
    let standing = Weights {
        so2: p.value().0,
        ..standing
    };
    let trapped = cold_trapped(&standing, equilibrium);
    let mut whole: Vec<NodeId> = partial
        .iter()
        .filter(|(gas, _)| **gas != Gas::Ch4)
        .map(|(_, p)| p.node())
        .collect();
    whole.push(photolysed.node());
    whole.extend(&cold);
    let mut trap = |code: &'static str, kept: Traced<Bar>, gas: f64| {
        let mut from = vec![kept.node()];
        from.extend(whole.iter().filter(|n| **n != kept.node()));
        trace.record(Code::of(code), Bar(gas), &from)
    };
    let co2 = partial[&Gas::Co2];
    let trapped_co2 = trap(codes::AIR_TRAPPED_CO2, co2, trapped.co2)?;
    let trapped_nh3 = trap(codes::AIR_TRAPPED_NH3, partial[&Gas::Nh3], trapped.nh3)?;
    let frozen_co2 = trace.record(
        Code::of(codes::AIR_FROZEN_CO2),
        Bar(co2.value().0 - trapped_co2.value().0),
        &[co2.node(), trapped_co2.node()],
    )?;
    partial.insert(Gas::Co2, trapped_co2);

    // Step 8: the photochemistry.
    let n2 = partial[&Gas::N2];
    let n2 = trace.record(
        Code::of(codes::AIR_PHOTOCHEMISTRY_N2),
        Bar(n2.value().0 + nitrogen_from_ammonia(*trapped_nh3.value()).0),
        &[n2.node(), trapped_nh3.node()],
    )?;
    partial.insert(Gas::N2, n2);
    let nh3 = trace.record(
        Code::of(codes::AIR_PHOTOCHEMISTRY_NH3),
        Bar(0.0),
        &[trapped_nh3.node()],
    )?;
    partial.insert(Gas::Nh3, nh3);
    let mut from = vec![photolysed.node()];
    from.extend(&cold);
    let ch4 = trace.record(
        Code::of(codes::AIR_PHOTOCHEMISTRY_CH4),
        if methane_survives(equilibrium) {
            *photolysed.value()
        } else {
            Bar(0.0)
        },
        &from,
    )?;
    partial.insert(Gas::Ch4, ch4);

    // Step 9: the pressure.
    let from: Vec<NodeId> = partial.values().map(Traced::node).collect();
    let surface_pressure = trace.record(
        Code::of(codes::AIR_PRESSURE),
        Bar(partial.values().map(|p| p.value().0).sum()),
        &from,
    )?;
    Ok(Air {
        atmosphere: Atmosphere {
            surface_pressure,
            partial,
        },
        water: kept_water,
        outgassed_co2: outgassed[&Gas::Co2],
        frozen_co2,
        reducing,
    })
}

/// Share of what the mantle outgasses that reaches the air through the
/// water of `body`: [`released_under_ocean`] at the pressure of the water
/// inventory on the body's surface, [`column_pressure`] of its mass under
/// the bulk gravity and radius (a runaway's steam weighs on the rock as
/// its ocean did).
pub(crate) fn released(body: &Body, water: &Water) -> Fraction {
    let bulk = body.bulk();
    released_under_ocean(column_pressure(
        *water.inventory.value(),
        *bulk.surface_gravity.value(),
        Metre::from(*bulk.radius.value()),
    ))
}

/// What the mantle outgasses (step 1 of [`air`]), each gas weighed but
/// not yet recorded: the air records them in its own order.
struct Volatiles {
    /// Each gas, its partial pressure and the nodes it derives from.
    gases: Vec<(Gas, Bar, Vec<NodeId>)>,
}

impl Volatiles {
    /// Records the outgassed partial pressure of `gas`
    /// (`planet.air.outgassed.<gas>`).
    fn record(&self, gas: Gas, trace: &mut Trace) -> Result<Traced<Bar>, InteriorError> {
        let (_, p, from) = self
            .gases
            .iter()
            .find(|(g, ..)| *g == gas)
            .expect("a volatile of the mantle");
        let (_, code, _) = AIR_GASES
            .into_iter()
            .find(|&(g, ..)| g == gas)
            .expect("every volatile is a gas of the air");
        Ok(trace.record(Code::of(code), *p, from)?)
    }
}

/// Whether the ice of `trial` trapped N2 when it formed
/// (`planet.volatiles.ice-traps-n2`): [`traps_nitrogen`] at the disc's
/// temperature at its orbit (`planet.volatiles.disc-temperature`,
/// [`disc_temperature`] of the semi-major axis and the host's
/// luminosity). The host is the primary source, whose mass places the
/// ice line; its luminosity is the present one, the only one the
/// timeline holds, as Hayashi 1981 took the Sun's today's for its young
/// disc (`docs/science/giant-planet-placement.md`).
fn ice_traps_nitrogen(trial: &Trial, trace: &mut Trace) -> Result<Traced<bool>, InteriorError> {
    let a = trial.orbit.semi_major_axis;
    let luminosity = trial.sources.primary().luminosity;
    let disc = trace.record(
        Code::of(codes::VOLATILE_DISC_TEMPERATURE),
        disc_temperature(*a.value(), *luminosity.value()),
        &[a.node(), luminosity.node()],
    )?;
    Ok(trace.record(
        Code::of(codes::VOLATILE_ICE_TRAPS_N2),
        traps_nitrogen(*disc.value()),
        &[disc.node()],
    )?)
}

/// The volatiles of the mantle of `body`: N2, CO2, CH4, NH3 and SO2, the
/// ice's N2 only when `traps`; the NH3 that the interior decomposes
/// ([`decomposed`]) comes up as N2; and its radiogenic argon
/// ([`argon`]).
fn mantle(
    trial: &Trial,
    drawn: &Drawn,
    body: &Body,
    surface: Traced<Kilogram>,
    water: &Water,
    regime: &Traced<TectonicRegime>,
    traps: Traced<bool>,
) -> Volatiles {
    let bulk = body.bulk();
    let gravity = *bulk.surface_gravity.value();
    let radius = Metre::from(*bulk.radius.value());
    let weight = |mass: Kilogram| column_pressure(mass, gravity, radius).0;

    let w = drawn.water_fraction.value().get();
    let f = drawn.envelope_fraction.value().get();
    let cmf = drawn.core_mass_fraction.value().get();
    let planet = Kilogram::from(*trial.mass.value()) * (1.0 - f);
    let refractory = planet * (1.0 - w);
    let rule = inventory(
        refractory,
        refractory * (1.0 - cmf),
        *surface.value(),
        *traps.value(),
    );

    let mobile = match regime.value() {
        TectonicRegime::Stagnant => LID_SHARE,
        // Plates, an episodic lid and heat pipes (the contract's enum is
        // non-exhaustive).
        _ => MOBILE_SHARE,
    };
    let (rest, nitrogen) = if *regime.value() == TectonicRegime::Plates {
        (
            1.0 - RECYCLED_UNDER_PLATES,
            1.0 - NITROGEN_RECYCLED_UNDER_PLATES,
        )
    } else {
        (1.0, 1.0)
    };
    let released = mobile * released(body, water).get();

    // The nodes every volatile of the mantle derives from: the mass and
    // composition of its inventory, the surface water its ice is, the
    // water that holds it back, the regime and the surface it weighs on.
    let base: [NodeId; 9] = [
        trial.mass.node(),
        drawn.envelope_fraction.node(),
        drawn.water_fraction.node(),
        drawn.core_mass_fraction.node(),
        surface.node(),
        water.inventory.node(),
        regime.node(),
        bulk.surface_gravity.node(),
        bulk.radius.node(),
    ];
    let scatter = &drawn.volatiles;
    // Definition: a scatter in dex multiplies by 10^dex.
    let scattered = |mass: Kilogram, dex: Traced<Dex>| mass * 10f64.powf(dex.value().0);
    // The NH3 the interior decomposes comes up as N2, with the N2 itself.
    let ammonia = decomposed(scattered(rule.nh3, scatter.nh3));
    let n2 = scattered(rule.n2, scatter.n2) + ammonia.n2;
    let volatiles = [
        (
            Gas::N2,
            n2,
            vec![scatter.n2.node(), traps.node(), scatter.nh3.node()],
            nitrogen,
        ),
        (
            Gas::Co2,
            scattered(rule.co2, scatter.co2),
            vec![scatter.co2.node()],
            rest,
        ),
        (
            Gas::Ch4,
            scattered(rule.ch4, scatter.ch4),
            vec![scatter.ch4.node()],
            rest,
        ),
        (Gas::Nh3, ammonia.nh3, vec![scatter.nh3.node()], nitrogen),
        (
            Gas::So2,
            scattered(rule.s * (SO2_PER_S * SULPHUR_IN_AIR), scatter.s),
            vec![scatter.s.node()],
            rest,
        ),
    ];
    let mut gases: Vec<(Gas, Bar, Vec<NodeId>)> = volatiles
        .into_iter()
        .map(|(gas, mass, own, kept)| {
            let mut from = base.to_vec();
            from.extend(own);
            (gas, Bar(weight(mass) * released * kept), from)
        })
        .collect();
    gases.push(argon(trial, drawn, body, water, regime));
    Volatiles { gases }
}

/// The radiogenic argon of `body` (step 1 of [`air`]), weighed: the
/// Ar-40 its mantle made over the trial's age ([`radiogenic_argon`] of the
/// interior's mantle mass, the mass the heat's potassium is in, and the
/// drawn radiogenic abundance, `10^dex`), times the share that reaches
/// the air under the regime ([`argon_degassed_share`]: a stagnant lid
/// degasses less, and so, by a documented choice, does an episodic lid,
/// though its other volatiles take the plates' share in `mantle`) and the
/// share the water lets through ([`released`]). A giant, which has no
/// mantle of its own, makes none.
fn argon(
    trial: &Trial,
    drawn: &Drawn,
    body: &Body,
    water: &Water,
    regime: &Traced<TectonicRegime>,
) -> (Gas, Bar, Vec<NodeId>) {
    let bulk = body.bulk();
    let abundance = drawn.radiogenic_abundance;
    let mut from = vec![
        abundance.node(),
        trial.age.node(),
        regime.node(),
        water.inventory.node(),
        bulk.surface_gravity.node(),
        bulk.radius.node(),
    ];
    let mantle = match body {
        Body::Rocky(interior) | Body::MiniNeptune { core: interior, .. } => interior.mantle_mass,
        Body::Giant(_) => return (Gas::Ar, Bar(0.0), from),
    };
    from.insert(0, mantle.node());
    // An episodic lid degasses its argon as a stagnant one (Venus's Ar-40
    // came out through the lid of its later Gyr), not as its CO2 and N2:
    // see `argon_degassed_share`.
    let lid = matches!(
        regime.value(),
        TectonicRegime::Stagnant | TectonicRegime::Episodic
    );
    let made = radiogenic_argon(
        *mantle.value(),
        10f64.powf(abundance.value().0),
        *trial.age.value(),
    );
    let share = argon_degassed_share(lid).get() * released(body, water).get();
    let weight = column_pressure(
        argon_mass(made) * share,
        *bulk.surface_gravity.value(),
        Metre::from(*bulk.radius.value()),
    );
    (Gas::Ar, weight, from)
}

/// The outgassed water vapour and O2 (step 1 of [`air`]): a runaway's
/// whole water inventory, 0 bar otherwise; no O2, imposed.
fn water_and_oxygen(
    body: &Body,
    water: &Water,
    trace: &mut Trace,
) -> Result<[(Gas, Traced<Bar>); 2], InteriorError> {
    let bulk = body.bulk();
    let h2o = if *water.runaway.value() {
        trace.record(
            Code::of(codes::AIR_OUTGASSED_H2O),
            Bar(column_pressure(
                *water.inventory.value(),
                *bulk.surface_gravity.value(),
                Metre::from(*bulk.radius.value()),
            )
            .0),
            &[
                water.runaway.node(),
                water.inventory.node(),
                bulk.surface_gravity.node(),
                bulk.radius.node(),
            ],
        )?
    } else {
        trace.record(
            Code::of(codes::AIR_OUTGASSED_H2O),
            Bar(0.0),
            &[water.runaway.node()],
        )?
    };
    let o2 = trace.leaf(
        Code::of(codes::AIR_OUTGASSED_O2),
        Bar(0.0),
        Provenance::Imposed,
    )?;
    Ok([(Gas::H2o, h2o), (Gas::O2, o2)])
}

/// The weights of the gases of `partial`, bar; a gas it lacks is none.
fn weights_of(partial: &BTreeMap<Gas, Traced<Bar>>) -> Weights {
    let w = |gas: Gas| partial.get(&gas).map_or(0.0, |p| p.value().0);
    Weights {
        n2: w(Gas::N2),
        co2: w(Gas::Co2),
        h2o: w(Gas::H2o),
        ch4: w(Gas::Ch4),
        nh3: w(Gas::Nh3),
        so2: w(Gas::So2),
        o2: w(Gas::O2),
        ar: w(Gas::Ar),
    }
}

/// The CH4 the stars' ultraviolet leaves of the `kept` one (step 3 of
/// [`air`]): each star's history is built at its mass and the trial's
/// metallicity and age.
fn photolysed(
    trial: &Trial,
    body: &Body,
    kept: Traced<Bar>,
    trace: &mut Trace,
) -> Result<Traced<Bar>, InteriorError> {
    let gravity = body.bulk().surface_gravity;
    let histories = histories(trial)?;
    let sources = lyman_sources(trial, &histories);
    let mut from = vec![kept.node(), gravity.node()];
    for s in trial.sources.iter() {
        from.extend([s.flux.node(), s.mass.node()]);
    }
    from.extend([trial.metallicity.node(), trial.age.node()]);
    Ok(trace.record(
        Code::of(codes::AIR_PHOTOLYSED_CH4),
        methane_left(
            *kept.value(),
            *gravity.value(),
            &sources,
            *trial.age.value(),
        ),
        &from,
    )?)
}

/// The high-energy history of each star of `trial`, built at its mass and
/// the trial's metallicity and age, in the order of its sources.
pub(super) fn histories(trial: &Trial) -> Result<Vec<History>, InteriorError> {
    let (metallicity, age) = (*trial.metallicity.value(), *trial.age.value());
    Ok(trial
        .sources
        .iter()
        .map(|s| history(*s.mass.value(), metallicity, age))
        .collect::<Result<Vec<History>, _>>()?)
}

/// The stars of `trial` as the photolysis reads them: each one's present
/// flux at the planet and its `histories`.
pub(super) fn lyman_sources<'a>(trial: &Trial, histories: &'a [History]) -> Vec<LymanSource<'a>> {
    trial
        .sources
        .iter()
        .zip(histories)
        .map(|(s, history)| LymanSource {
            flux: WattPerSquareMetre::from(*s.flux.value()),
            history,
        })
        .collect()
}

/// The share of its air the planet keeps on the cosmic shoreline (step 2
/// of [`air`]).
fn shoreline(
    trial: &Trial,
    exposed: &Exposed,
    core: &CoreRecord,
    trace: &mut Trace,
) -> Result<Traced<Fraction>, InteriorError> {
    let radius = exposed.radius;
    let mut from = exposed.mass_from.clone();
    let velocity = escape_velocity(exposed.mass, Metre::from(*radius.value()));
    let (_, fluence) = trial.sources.band_total(|s| &s.xuv);
    from.push(radius.node());
    from.extend(trial.sources.iter().map(|s| s.xuv.fluence.node()));
    from.push(core.dynamo.node());
    Ok(trace.record(
        Code::of(codes::AIR_SHORELINE_KEPT),
        shoreline_kept(velocity, fluence, *core.dynamo.value()),
        &from,
    )?)
}
