//! The air of a planet with a surface: what its interior outgasses, what
//! the cosmic shoreline keeps of it, the water a runaway keeps as vapour
//! and the SO2 its wash-out leaves. Ported from
//! `legacy-final:crates/mapgen/src/planet/atmosphere.rs` (`outgassed`,
//! `atmosphere`, `wash_out`), `planet/mod.rs` (`Water::keeping` in a
//! runaway) and `planet/climate.rs` (`washed`); the laws are in
//! `science::outgassing`, `science::volatiles` and `science::escape`.
//!
//! The methane photolysis, the cold trap, the photochemistry and the
//! climate's condensation come after, with the climate (#659).

use std::collections::BTreeMap;

use super::codes::{self, AIR_GASES};
use super::core::CoreRecord;
use super::draws::Drawn;
use super::enveloped::Body;
use super::interior::InteriorError;
use crate::Trial;
use science::escape::{escape_velocity, shoreline_kept};
use science::outgassing::{
    LID_SHARE, MOBILE_SHARE, NITROGEN_RECYCLED_UNDER_PLATES, RECYCLED_UNDER_PLATES, SO2_PER_S,
    SULPHUR_IN_AIR, column_pressure, released_under_ocean, so2_kept,
};
use science::units::{Bar, Fraction, Kilogram, Metre};
use science::volatiles::inventory;
use timeline::{Atmosphere, Code, Gas, NodeId, Provenance, TectonicRegime, Trace, Traced, Water};

/// The air of the planet `trial` of composition `drawn`, both as they were
/// before a runaway's steam loss (`super::escape::Escaped`), of body
/// `body` after it, with its water `water`, its tectonic regime `regime`
/// and its core `core`; and its water once the air is made. In order:
///
/// 1. **Outgassing** (`planet.air.outgassed.<gas>`, one node per gas):
///    the volatiles at formation ([`inventory`] of the refractory mass
///    `(1 - w)(1 - f) M`, its silicate `(1 - cmf)` share and the ice
///    `w (1 - f) M`, before the steam loss so that they keep their mass),
///    each times `10^scatter`, weigh [`column_pressure`] on the body's
///    surface, times the regime's share ([`LID_SHARE`] for a stagnant lid,
///    [`MOBILE_SHARE`] otherwise), the share the water lets through
///    ([`released`]) and, under plates, what subduction leaves
///    ([`NITROGEN_RECYCLED_UNDER_PLATES`] of the N2 and NH3,
///    [`RECYCLED_UNDER_PLATES`] of the rest). Sulphur comes up as SO2
///    ([`SO2_PER_S`]), of which [`SULPHUR_IN_AIR`] stays. Water is vapour
///    only in a runaway, and then the whole inventory: it is the ocean,
///    not a volatile of the mantle, so neither the regime nor the
///    seafloor holds it back. No O2 (imposed: life makes it).
/// 2. **Shoreline** (`planet.air.shoreline-kept`): the share
///    [`shoreline_kept`] at the escape velocity of the bulk mass and
///    radius, under the XUV fluence of all the stars, with the dynamo;
///    each partial (`planet.air.kept.<gas>`) is its outgassed one times
///    that share. The escape velocity has no node (ledger ruling 1): the
///    share derives from the bulk mass and radius, the fluences and the
///    dynamo.
/// 3. **Runaway**: the water inventory becomes the vapour kept, the
///    inventory times the shoreline's share (`planet.water.inventory`,
///    from the inventory and the share); the caller derives the body
///    again from it. Out of a runaway the water is returned as given.
/// 4. **Wash-out** (`planet.air.so2-kept`): [`so2_kept`], wet when the
///    water keeps a wet balance (out of a runaway, some water kept),
///    with the O2 and the surface pressure of the air before it; the SO2
///    partial is the outgassed one times the shoreline's share and this
///    one.
/// 5. **Pressure** (`planet.air.pressure`): the sum of the partials.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn air(
    trial: &Trial,
    drawn: &Drawn,
    body: &Body,
    water: &Water,
    regime: &Traced<TectonicRegime>,
    core: &CoreRecord,
    trace: &mut Trace,
) -> Result<(Atmosphere, Water), InteriorError> {
    let outgassed = outgassed(trial, drawn, body, water, regime, trace)?;
    let shoreline = shoreline(trial, body, core, trace)?;
    let share = shoreline.value().get();
    let runaway = *water.runaway.value();

    let mut partial = BTreeMap::new();
    for (gas, _, kept) in AIR_GASES {
        if gas == Gas::So2 {
            continue;
        }
        let out = outgassed[&gas];
        let p = trace.record(
            Code::of(kept),
            Bar(out.value().0 * share),
            &[out.node(), shoreline.node()],
        )?;
        partial.insert(gas, p);
    }

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

    // The SO2 the shoreline keeps, before its wash-out, weighs in the
    // column the wash-out reads.
    let so2 = outgassed[&Gas::So2];
    let so2_before = so2.value().0 * share;
    let before: f64 = partial.values().map(|p| p.value().0).sum::<f64>() + so2_before;
    let wet = !runaway && water.inventory.value().0 > 0.0;
    let mut from: Vec<NodeId> = vec![water.runaway.node(), water.inventory.node()];
    from.extend(partial.values().map(Traced::node));
    from.extend([so2.node(), shoreline.node()]);
    let washed = trace.record(
        Code::of(codes::AIR_SO2_KEPT),
        so2_kept(wet, *partial[&Gas::O2].value(), Bar(before)),
        &from,
    )?;
    let p = trace.record(
        Code::of(codes::AIR_KEPT_SO2),
        Bar(so2_before * washed.value().get()),
        &[so2.node(), shoreline.node(), washed.node()],
    )?;
    partial.insert(Gas::So2, p);

    let from: Vec<NodeId> = partial.values().map(Traced::node).collect();
    let surface_pressure = trace.record(
        Code::of(codes::AIR_PRESSURE),
        Bar(partial.values().map(|p| p.value().0).sum()),
        &from,
    )?;
    Ok((
        Atmosphere {
            surface_pressure,
            partial,
        },
        kept_water,
    ))
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

/// The outgassed partial pressure of each gas (step 1 of [`air`]).
fn outgassed(
    trial: &Trial,
    drawn: &Drawn,
    body: &Body,
    water: &Water,
    regime: &Traced<TectonicRegime>,
    trace: &mut Trace,
) -> Result<BTreeMap<Gas, Traced<Bar>>, InteriorError> {
    let bulk = body.bulk();
    let gravity = *bulk.surface_gravity.value();
    let radius = Metre::from(*bulk.radius.value());
    let weight = |mass: Kilogram| column_pressure(mass, gravity, radius).0;

    let w = drawn.water_fraction.value().get();
    let f = drawn.envelope_fraction.value().get();
    let cmf = drawn.core_mass_fraction.value().get();
    let planet = Kilogram::from(*trial.mass.value()) * (1.0 - f);
    let refractory = planet * (1.0 - w);
    let rule = inventory(refractory, refractory * (1.0 - cmf), planet * w);

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
    // composition of its inventory, the water that holds it back, the
    // regime and the surface it weighs on.
    let base: [NodeId; 8] = [
        trial.mass.node(),
        drawn.envelope_fraction.node(),
        drawn.water_fraction.node(),
        drawn.core_mass_fraction.node(),
        water.inventory.node(),
        regime.node(),
        bulk.surface_gravity.node(),
        bulk.radius.node(),
    ];
    let scatter = &drawn.volatiles;
    let volatiles = [
        (Gas::N2, rule.n2, scatter.n2, nitrogen),
        (Gas::Co2, rule.co2, scatter.co2, rest),
        (Gas::Ch4, rule.ch4, scatter.ch4, rest),
        (Gas::Nh3, rule.nh3, scatter.nh3, nitrogen),
        (
            Gas::So2,
            rule.s * (SO2_PER_S * SULPHUR_IN_AIR),
            scatter.s,
            rest,
        ),
    ];
    let mut out = BTreeMap::new();
    for (gas, mass, dex, kept) in volatiles {
        let code = AIR_GASES
            .iter()
            .find_map(|&(g, code, _)| (g == gas).then_some(code))
            .expect("every volatile is a gas of the air");
        // Definition: a scatter in dex multiplies by 10^dex.
        let mass = mass * 10f64.powf(dex.value().0);
        let mut from = base.to_vec();
        from.push(dex.node());
        let p = trace.record(Code::of(code), Bar(weight(mass) * released * kept), &from)?;
        out.insert(gas, p);
    }

    let h2o = if *water.runaway.value() {
        trace.record(
            Code::of(codes::AIR_OUTGASSED_H2O),
            Bar(weight(*water.inventory.value())),
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
    out.insert(Gas::H2o, h2o);
    let o2 = trace.leaf(
        Code::of(codes::AIR_OUTGASSED_O2),
        Bar(0.0),
        Provenance::Imposed,
    )?;
    out.insert(Gas::O2, o2);
    Ok(out)
}

/// The share of its air the planet keeps on the cosmic shoreline (step 2
/// of [`air`]).
fn shoreline(
    trial: &Trial,
    body: &Body,
    core: &CoreRecord,
    trace: &mut Trace,
) -> Result<Traced<Fraction>, InteriorError> {
    let bulk = body.bulk();
    let velocity = escape_velocity(
        Kilogram::from(*bulk.mass.value()),
        Metre::from(*bulk.radius.value()),
    );
    let (_, fluence) = trial.sources.band_total(|s| &s.xuv);
    let mut from: Vec<NodeId> = vec![bulk.mass.node(), bulk.radius.node()];
    from.extend(trial.sources.iter().map(|s| s.xuv.fluence.node()));
    from.push(core.dynamo.node());
    Ok(trace.record(
        Code::of(codes::AIR_SHORELINE_KEPT),
        shoreline_kept(velocity, fluence, *core.dynamo.value()),
        &from,
    )?)
}
