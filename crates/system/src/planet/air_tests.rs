//! Tier 1: the air of a planet, its outgassing, the cosmic shoreline, the
//! SO2 wash-out, the methane photolysis, the reducing air, the cold trap
//! and the photochemistry, ported from
//! `legacy-final:crates/mapgen/src/planet/atmosphere.rs` (its planet
//! tests, l.1358-1825) on the reference bodies of `planet/tests.rs`, with
//! the targets of studies 21 and 23.

use super::air::{Air, air, histories, lyman_sources, released};
use super::codes;
use super::core::{CoreRecord, core};
use super::draws::Drawn;
use super::enveloped::Body;
use super::interior::Water as Held;
use super::regime::regime;
use super::tests::{
    CALLISTO, EARTH, EARTH_CORE_MASS_FRACTION, HOT_ORBIT_AU, MARS, MERCURY, ReferenceBody, TITAN,
    VENUS, WATER_WORLD_FRACTION, ancestry, derive_surface, derived_from, reference_trial, trial,
    with_fluence,
};
use crate::Trial;
use crate::sources::{light, sources};
use crate::star::star;
use science::constants::PASCALS_PER_BAR;
use science::escape::{WATER_PER_HYDROGEN, energy_limited_loss, escape_velocity, shoreline_kept};
use science::photochemistry::{
    CH4_MOLECULE_MASS, CH4_PER_LYMAN_ALPHA_PHOTON, LYMAN_ALPHA_PHOTON_ENERGY, methane_destroyed,
};
use science::units::{Bar, Gyr, Kilogram, Metre, SolarMass};
use science::volatiles::{ICE_CH4, ICE_CO2, ROCK_CO2};
use timeline::{
    Atmosphere, Code, Gas, NodeId, Provenance, Seed, TectonicRegime, Trace, Traced, Water,
};

/// Definition: the surface pressure below which a world is airless, no
/// sky and no clouds, 1e-4 bar: legacy's `AIRLESS_BAR`
/// (`legacy-final:crates/mapgen/src/atmosphere.rs` l.25), the class
/// threshold of the climate (#659).
const AIRLESS_BAR: f64 = 1e-4;

/// Source: Venus's surface pressure, 92 bar, about 96.5 % CO2 and 3.5 %
/// N2 (3.2 bar) (NASA Venus fact sheet): legacy's test bands around them,
/// a total of 60 to 120 bar, CO2 above 90 % and 1 to 6 bar of N2.
const VENUS_TOTAL: (f64, f64) = (60.0, 120.0);
/// Source: see [`VENUS_TOTAL`].
const VENUS_CO2_SHARE: f64 = 0.9;
/// Source: see [`VENUS_TOTAL`].
const VENUS_N2: (f64, f64) = (1.0, 6.0);

/// Source: Earth's N2 as a weight, 0.746 bar: dry air of 5.1352e18 kg
/// (Trenberth & Smith 2005, J. Climate 18, 864, abstract), 78.08 % of it
/// by volume at a mean molecular weight of 28.97 (NASA Earth fact sheet),
/// on Earth's surface at 9.80665 m/s^2 (study 23 of the Wiki; the 0.78
/// bar often quoted is the volume share of a bar, not a weight).
const EARTH_N2: f64 = 0.746;

/// Definition: the test's own tolerance round [`EARTH_N2`], 10 %, no
/// source: the chain is fitted on it
/// (`science::outgassing::NITROGEN_RECYCLED_UNDER_PLATES`, 0.7458 bar),
/// and the band lets an input of the reference Earth move without
/// pinning the fit to its digits.
const EARTH_N2_TOLERANCE: f64 = 0.1;

/// Source: an abiotic Earth keeps no CH4 or NH3, and Callisto no CH4:
/// legacy's test bound, 1e-3 bar (`legacy-final` `atmosphere.rs`,
/// `earth_has_no_abiotic_methane_or_ammonia` and
/// `ganymede_and_callisto_lose_their_methane_to_photolysis`), Earth's
/// 1.9 ppm of CH4 being life's.
const NO_METHANE_OR_AMMONIA: f64 = 1e-3;

/// Source: Titan's CH4, about 5 % of its 1.47 bar at the surface
/// (Niemann et al. 2005, Nature 438, 779, Huygens GCMS): legacy's test
/// bound, 0.01 bar (`legacy-final` `atmosphere.rs`,
/// `titan_keeps_its_methane`).
const TITAN_METHANE_MIN: f64 = 0.01;

/// Definition: legacy's test bound on the methane Titan's photolysis
/// leaves, its stock on the ground and in the air before the climate
/// condenses it, 1 bar (`legacy-final` `atmosphere.rs`,
/// `titans_outgassing_outruns_its_photolysis`): a test choice, above the
/// 0.07 bar its air holds today (Niemann et al. 2005), that fails when
/// the onset or the yield drifts by an order of magnitude.
const TITAN_METHANE_STOCK_MIN: f64 = 1.0;

/// Source: Earth's SO2 is a trace, well below a ppm of its bar (NASA
/// Earth fact sheet lists none): legacy's test bound, 1e-6 bar.
const EARTH_SO2_MAX: f64 = 1e-6;

/// Source: Venus's SO2, about 150 ppm of 92 bar (Bertaux et al. 2007,
/// Marcq et al. 2018): legacy's test band, 0.008 to 0.02 bar.
const VENUS_SO2: (f64, f64) = (0.008, 0.02);

/// Source: Mars's SO2 upper limit, about 0.3 ppb (Encrenaz et al. 2011):
/// legacy's test bound, 1e-6 bar.
const MARS_SO2_MAX: f64 = 1e-6;

/// Source: Mars's surface pressure, about 6 mbar (NASA Mars fact sheet):
/// legacy's test band, 2 to 50 mbar.
const MARS_TOTAL: (f64, f64) = (0.002, 0.05);

/// Source: Earth's ocean on its sea floor changes nothing: legacy's test
/// bound on the share released under it.
const EARTH_RELEASED_MIN: f64 = 0.999_99;

/// Source: an Earth of 30 % water lets about a thousandth of its
/// outgassing through, the few bar of CO2 before the thermostat that Kite
/// & Ford 2018 find on waterworlds (`science::outgassing`, study 21,
/// part 5): legacy's test band.
const DEEP_RELEASED: (f64, f64) = (5e-4, 2e-3);

/// Definition: the water fraction of legacy's deep-ocean Earth, 30 %.
const DEEP_WATER_FRACTION: f64 = 0.3;

/// Source: Earth's oceans, 1.4e21 kg (Charette & Smith 2010, the
/// [`EARTH`] fixture's 1.37e21 kg rounded as in legacy's test).
const EARTH_OCEANS_KG: f64 = 1.4e21;

/// Definition: a relative tolerance on a value computed two ways.
const ROUNDING: f64 = 1e-12;

/// Definition: a relative tolerance on a mass solved back from a ratio
/// of two pressures, 1e-9: the subtraction loses a few digits.
const SOLVED: f64 = 1e-9;

/// A planet up to its core, as the chain derives it before the air.
struct Planet {
    trace: Trace,
    trial: Trial,
    drawn: Drawn,
    body: Body,
    surface: Traced<Kilogram>,
    water: Water,
    regime: Traced<TectonicRegime>,
    core: CoreRecord,
}

impl Planet {
    /// The air of the planet, recorded in its trace, a reducing air's CO2
    /// held back as the chain does.
    fn air(&mut self) -> (Atmosphere, Water) {
        let a = self.air_held(true);
        (a.atmosphere, a.water)
    }

    /// The air of the planet, a reducing air's CO2 held back when
    /// `hold_back`.
    fn air_held(&mut self, hold_back: bool) -> Air {
        air(
            &self.trial,
            &self.drawn,
            &self.body,
            self.surface,
            &self.water,
            &self.regime,
            &self.core,
            hold_back,
            &mut self.trace,
        )
        .unwrap()
    }
}

/// `t` of composition `drawn` up to its core, on its formed body: the
/// reference bodies that run away hold no water, so lose no steam; a
/// planet that does is read here before the chain rebuilds it.
fn up_to_core(mut trace: Trace, t: Trial, drawn: Drawn) -> Planet {
    let (body, h, w, surface) = derive_surface(&mut trace, &t, &drawn);
    let r = regime(&t, &body, &h, &w, &mut trace).unwrap();
    let Body::Rocky(rocky) = &body else {
        panic!("a planet with a surface is rocky")
    };
    let k = core(&t, rocky, &r, &mut trace).unwrap();
    Planet {
        trace,
        trial: t,
        drawn,
        body,
        surface,
        water: w,
        regime: r,
        core: k,
    }
}

/// The reference `body` up to its core.
fn planet(body: &ReferenceBody) -> Planet {
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, body);
    up_to_core(trace, t, drawn)
}

/// The partial pressure of `gas`, bar.
fn partial(a: &Atmosphere, gas: Gas) -> f64 {
    a.partial[&gas].value().0
}

/// The surface pressure, bar.
fn total(a: &Atmosphere) -> f64 {
    a.surface_pressure.value().0
}

/// The number a node of `trace` holds.
fn number(trace: &Trace, node: timeline::NodeId) -> f64 {
    match &trace.node(node).unwrap().value {
        timeline::NodeValue::Number { value, .. } => *value,
        value => panic!("{value:?} is not a number"),
    }
}

/// The node of code `code` that `node` is or derives from; the latest
/// recorded when there are several.
fn ancestor(trace: &Trace, node: NodeId, code: &'static str) -> NodeId {
    ancestry(trace, node)
        .into_iter()
        .rev()
        .find(|&n| trace.node(n).unwrap().code == Code::of(code))
        .unwrap_or_else(|| panic!("{node:?} does not derive from {code}"))
}

/// The codes of the outgassed and kept partial pressures of `gas`.
fn codes_of(gas: Gas) -> (&'static str, &'static str) {
    let (_, outgassed, kept) = codes::AIR_GASES
        .into_iter()
        .find(|(g, ..)| *g == gas)
        .unwrap();
    (outgassed, kept)
}

/// The outgassed partial pressure of `gas` the final `partial` derives
/// from, bar.
fn outgassed(p: &Planet, gas: Gas, partial: &Traced<Bar>) -> f64 {
    number(
        &p.trace,
        ancestor(&p.trace, partial.node(), codes_of(gas).0),
    )
}

/// The partial pressure of `gas` the shoreline kept, which the final
/// `partial` derives from.
fn kept(p: &Planet, gas: Gas, partial: &Traced<Bar>) -> NodeId {
    ancestor(&p.trace, partial.node(), codes_of(gas).1)
}

#[test]
fn venus_has_ninety_bars_of_co2() {
    let a = planet(&VENUS).air().0;
    let t = total(&a);
    assert!((VENUS_TOTAL.0..=VENUS_TOTAL.1).contains(&t), "{a:?}");
    assert!(partial(&a, Gas::Co2) / t > VENUS_CO2_SHARE, "{a:?}");
    let n2 = partial(&a, Gas::N2);
    assert!((VENUS_N2.0..=VENUS_N2.1).contains(&n2), "{a:?}");
}

#[test]
fn earth_has_a_bar_of_nitrogen() {
    let a = planet(&EARTH).air().0;
    let n2 = partial(&a, Gas::N2);
    assert!((n2 / EARTH_N2 - 1.0).abs() < EARTH_N2_TOLERANCE, "{a:?}");
}

#[test]
fn earths_air_keeps_no_sulphur_dioxide() {
    let a = planet(&EARTH).air().0;
    assert!(partial(&a, Gas::So2) < EARTH_SO2_MAX, "{a:?}");
}

#[test]
fn venus_keeps_its_sulphur_dioxide() {
    let a = planet(&VENUS).air().0;
    let so2 = partial(&a, Gas::So2);
    assert!((VENUS_SO2.0..=VENUS_SO2.1).contains(&so2), "{a:?}");
}

#[test]
fn mars_keeps_little_sulphur_dioxide() {
    let a = planet(&MARS).air().0;
    assert!(partial(&a, Gas::So2) < MARS_SO2_MAX, "{a:?}");
}

#[test]
fn mars_keeps_a_trace() {
    let a = planet(&MARS).air().0;
    let t = total(&a);
    assert!((MARS_TOTAL.0..=MARS_TOTAL.1).contains(&t), "{a:?}");
}

#[test]
fn mercury_is_airless() {
    let a = planet(&MERCURY).air().0;
    assert!(total(&a) < AIRLESS_BAR, "{a:?}");
}

#[test]
fn a_dynamo_shields_the_air() {
    let mut mars = planet(&MARS);
    let shielded = |dynamo: bool, p: &mut Planet| {
        p.core.dynamo = p
            .trace
            .leaf(Code::of("test.dynamo"), dynamo, Provenance::Input)
            .unwrap();
        p.air().0
    };
    let bare = shielded(false, &mut mars);
    let kept = shielded(true, &mut mars);
    let co2 = &kept.partial[&Gas::Co2];
    let out = outgassed(&mars, Gas::Co2, co2);
    let (a, b) = (total(&bare), total(&kept));
    assert!(out > 0.0);
    assert!(b > a && a > 0.0, "{a} {b}");
    assert!(co2.value().0 <= out);
}

#[test]
fn a_deep_ocean_holds_back_what_the_mantle_outgasses() {
    let earth = planet(&EARTH);
    let share = released(&earth.body, &earth.water).get();
    assert!(share > EARTH_RELEASED_MIN, "{share}");
    // An Earth of 30 % water, a water world, lets about a thousandth
    // through.
    let mut trace = Trace::new();
    let (t, _) = reference_trial(&mut trace, &EARTH);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        DEEP_WATER_FRACTION,
        0.0,
        Some(true),
        0.0,
        None,
        None,
    );
    let deep = up_to_core(trace, t, drawn);
    let share = released(&deep.body, &deep.water).get();
    assert!(
        (DEEP_RELEASED.0..=DEEP_RELEASED.1).contains(&share),
        "{share}"
    );
}

#[test]
fn a_runaway_keeps_its_water_as_vapour() {
    let mut earth = planet(&EARTH);
    let steam = |runaway: bool, kept: f64, p: &mut Planet| {
        p.water = Water {
            inventory: p
                .trace
                .leaf(Code::of("test.water"), Kilogram(kept), Provenance::Input)
                .unwrap(),
            runaway: p
                .trace
                .leaf(Code::of("test.runaway"), runaway, Provenance::Input)
                .unwrap(),
            ..p.water
        };
        p.air()
    };
    let (a, w) = steam(true, EARTH_OCEANS_KG, &mut earth);
    let vapour = &a.partial[&Gas::H2o];
    assert!(vapour.value().0 > 1.0, "{a:?}");
    // The water left is the vapour the shoreline kept, back in kilograms:
    // the oceans times the shoreline's share, derived from them.
    let from = derived_from(&earth.trace, w.inventory.node());
    assert_eq!(
        earth.trace.node(w.inventory.node()).unwrap().code,
        Code::of(codes::WATER_INVENTORY)
    );
    assert_eq!(from[0], earth.water.inventory.node());
    let shoreline = derived_from(&earth.trace, vapour.node())[1];
    assert_eq!(from[1], shoreline);
    let share = vapour.value().0 / outgassed(&earth, Gas::H2o, vapour);
    let kept = w.inventory.value().0;
    assert!((kept / (EARTH_OCEANS_KG * share) - 1.0).abs() < ROUNDING);
    assert!(kept <= EARTH_OCEANS_KG);
    assert_eq!(w.runaway, earth.water.runaway);

    let (a, _) = steam(true, 0.0, &mut earth);
    assert_eq!(partial(&a, Gas::H2o), 0.0);
    // Out of a runaway the water is in the oceans, and stays as it was.
    let (a, w) = steam(false, EARTH_OCEANS_KG, &mut earth);
    assert_eq!(partial(&a, Gas::H2o), 0.0);
    assert_eq!(w, earth.water);
}

#[test]
fn no_xuv_keeps_the_whole_air() {
    let mut mars = planet(&MARS);
    mars.trial = with_fluence(&mut mars.trace, &mars.trial, 0.0);
    let a = mars.air().0;
    for (gas, p) in &a.partial {
        let out = outgassed(&mars, *gas, p);
        let kept = number(&mars.trace, kept(&mars, *gas, p));
        assert!(out.is_finite() && kept.is_finite(), "{gas:?}");
        if *gas != Gas::So2 {
            assert_eq!(kept, out, "{gas:?}");
        }
    }
    assert!(partial(&a, Gas::Co2) > 0.0);
    assert!(total(&a).is_finite());
}

#[test]
fn each_partial_cites_its_outgassing_and_the_shoreline() {
    let mut venus = planet(&VENUS);
    let a = venus.air().0;
    let trace = &venus.trace;
    let code = |id| trace.node(id).unwrap().code.clone();
    let co2 = kept(&venus, Gas::Co2, &a.partial[&Gas::Co2]);
    let shoreline = derived_from(trace, co2)[1];
    assert_eq!(code(shoreline), Code::of(codes::AIR_SHORELINE_KEPT));
    // The shoreline's share derives from the bulk mass and radius, the
    // XUV fluence and the dynamo (ledger ruling 1: no escape velocity
    // node).
    let bulk = venus.body.bulk();
    assert_eq!(
        derived_from(trace, shoreline),
        vec![
            bulk.mass.node(),
            bulk.radius.node(),
            venus.trial.sources.primary().xuv.fluence.node(),
            venus.core.dynamo.node(),
        ]
    );
    assert_eq!(a.partial.len(), codes::AIR_GASES.len());
    for (gas, p) in &a.partial {
        let (outgassed, _) = codes_of(*gas);
        let kept = kept(&venus, *gas, p);
        let from = derived_from(trace, kept);
        assert_eq!(code(from[0]), Code::of(outgassed), "{gas:?}");
        assert_eq!(from[1], shoreline, "{gas:?}");
        if *gas == Gas::So2 {
            assert_eq!(from.len(), 3);
            assert_eq!(code(from[2]), Code::of(codes::AIR_SO2_KEPT));
        } else {
            assert_eq!(from.len(), 2, "{gas:?}");
        }
    }
    // The outgassing derives from the regime, and from the surface water
    // whose volatiles its ice term carries.
    let co2 = derived_from(trace, co2)[0];
    assert!(derived_from(trace, co2).contains(&venus.regime.node()));
    assert!(derived_from(trace, co2).contains(&venus.surface.node()));
    // The pressure is the sum of the partials, from them.
    let partials: Vec<_> = a.partial.values().map(Traced::node).collect();
    assert_eq!(derived_from(trace, a.surface_pressure.node()), partials);
    let sum: f64 = a.partial.values().map(|p| p.value().0).sum();
    assert_eq!(total(&a), sum);
}

// Review M1: a hot water world, its water beyond the laws and recorded
// apart from its condensed bulk, holds its steam and its air with its
// whole mass, the bulk mass plus that water, at the condensed radius.
#[test]
fn a_hot_water_world_holds_its_air_with_its_whole_mass() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, HOT_ORBIT_AU, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        WATER_WORLD_FRACTION,
        0.0,
        Some(true),
        0.0,
        None,
        None,
    );
    let mut p = up_to_core(trace, t, drawn);
    let Body::Rocky(rocky) = &p.body else {
        panic!("a planet with a surface is rocky")
    };
    let Held::BeyondLaw { mass: beyond, .. } = rocky.water else {
        panic!("{:?}", rocky.water)
    };
    let bulk = p.body.bulk().clone();
    let whole = Kilogram(Kilogram::from(*bulk.mass.value()).0 + beyond.value().0);
    let radius = Metre::from(*bulk.radius.value());
    let (_, fluence) = p.trial.sources.band_total(|s| &s.xuv);
    let xuv = p.trial.sources.primary().xuv.fluence.node();

    // The steam lost: 9 kg of water per kg of hydrogen lost at the whole
    // mass, capped by the bulk water.
    assert!(*p.water.runaway.value());
    let lost = *derived_from(&p.trace, p.water.inventory.node())
        .last()
        .unwrap();
    assert_eq!(
        p.trace.node(lost).unwrap().code,
        Code::of(codes::WATER_STEAM_LOST)
    );
    let water = WATER_WORLD_FRACTION * Kilogram::from(*p.trial.mass.value()).0;
    let steam = (WATER_PER_HYDROGEN * energy_limited_loss(radius, whole, fluence).0).min(water);
    let value = number(&p.trace, lost);
    assert!((value / steam - 1.0).abs() < ROUNDING, "{value} {steam}");
    let from = derived_from(&p.trace, lost);
    assert!(from.contains(&bulk.mass.node()) && from.contains(&beyond.node()));

    // The shoreline: the escape velocity of the whole mass.
    let a = p.air().0;
    let shoreline = ancestor(
        &p.trace,
        a.partial[&Gas::Co2].node(),
        codes::AIR_SHORELINE_KEPT,
    );
    let share = shoreline_kept(
        escape_velocity(whole, radius),
        fluence,
        *p.core.dynamo.value(),
    )
    .get();
    let value = number(&p.trace, shoreline);
    assert!((value / share - 1.0).abs() < ROUNDING, "{value} {share}");
    assert_eq!(
        derived_from(&p.trace, shoreline),
        vec![
            bulk.mass.node(),
            beyond.node(),
            bulk.radius.node(),
            xuv,
            p.core.dynamo.node(),
        ]
    );
}

#[test]
fn the_ice_is_the_water_at_the_surface() {
    // A dry Earth's ice term carries the volatiles of its ocean, the
    // surface share of its bulk water, not of the water in its mantle
    // (legacy fed it the ocean): its N2 is the rock's and the ocean's.
    let mut earth = planet(&EARTH);
    assert_eq!(
        earth.trace.node(earth.surface.node()).unwrap().code,
        Code::of(codes::WATER_SURFACE)
    );
    let a = earth.air().0;
    let n2 = ancestor(
        &earth.trace,
        a.partial[&Gas::N2].node(),
        codes::AIR_OUTGASSED_N2,
    );
    assert!(derived_from(&earth.trace, n2).contains(&earth.surface.node()));
    // Its magnitude: CO2 and CH4 share every factor but their mass (both
    // recycled alike under plates, no scatter), CH4 coming from the ice
    // alone, so their ratio gives the ice back:
    // `CO2 / CH4 = (ROCK_CO2 r + ICE_CO2 i) / (ICE_CH4 i)`.
    let co2 = outgassed(&earth, Gas::Co2, &a.partial[&Gas::Co2]);
    let ch4 = outgassed(&earth, Gas::Ch4, &a.partial[&Gas::Ch4]);
    let w = earth.drawn.water_fraction.value().get();
    let refractory = (1.0 - w) * Kilogram::from(*earth.trial.mass.value()).0;
    let ice = ROCK_CO2 * refractory / (ICE_CH4 * co2 / ch4 - ICE_CO2);
    let surface = earth.surface.value().0;
    assert!((ice / surface - 1.0).abs() < SOLVED, "{ice} {surface}");
}

#[test]
fn earth_has_no_abiotic_methane_or_ammonia() {
    let a = planet(&EARTH).air().0;
    let (ch4, nh3) = (partial(&a, Gas::Ch4), partial(&a, Gas::Nh3));
    assert!(ch4 < NO_METHANE_OR_AMMONIA, "{a:?}");
    assert!(nh3 < NO_METHANE_OR_AMMONIA, "{a:?}");
}

#[test]
fn titan_keeps_its_methane() {
    // Ported from legacy's `titans_outgassing_outruns_its_photolysis`:
    // the supply overtakes the destruction, so the photolysis leaves a
    // stock of over a bar, yet less than it was given (legacy's
    // "outgassed" is the air past the shoreline, the photolysis's input).
    // Legacy's bound on the air after the climate waits for 2d-3.
    let mut titan = planet(&TITAN);
    let a = titan.air().0;
    let ch4 = &a.partial[&Gas::Ch4];
    assert!(ch4.value().0 > TITAN_METHANE_MIN, "{a:?}");
    let photolysed = ancestor(&titan.trace, ch4.node(), codes::AIR_PHOTOLYSED_CH4);
    let left = number(&titan.trace, photolysed);
    let given = number(&titan.trace, kept(&titan, Gas::Ch4, ch4));
    assert!(
        left > TITAN_METHANE_STOCK_MIN && left < given,
        "{given} {left}"
    );
}

#[test]
fn callisto_loses_its_methane() {
    // Its outgassing cannot keep up with the Sun's ultraviolet at 5.2 au,
    // which destroys methane over three times faster than at Titan.
    let mut callisto = planet(&CALLISTO);
    let a = callisto.air().0;
    let ch4 = &a.partial[&Gas::Ch4];
    assert!(ch4.value().0 < NO_METHANE_OR_AMMONIA, "{a:?}");
    assert!(outgassed(&callisto, Gas::Ch4, ch4) > 0.0, "{a:?}");
}

#[test]
fn titan_outgasses_no_co2() {
    let mut titan = planet(&TITAN);
    let air = titan.air_held(true);
    assert!(*air.reducing.value());
    let co2 = &air.atmosphere.partial[&Gas::Co2];
    assert_eq!(co2.value().0, 0.0);
    assert_eq!(air.frozen_co2.value().0, 0.0);
    let out = ancestor(&titan.trace, co2.node(), codes::AIR_OUTGASSED_CO2);
    assert_eq!(number(&titan.trace, out), 0.0);
    assert_eq!(derived_from(&titan.trace, out), vec![air.reducing.node()]);
    // The reducing air reads the very methane node the final CH4 derives
    // from (one pass; the chain order is `each_partial_reads_in_chain_order`'s)
    // and the equilibrium temperature.
    let from = derived_from(&titan.trace, air.reducing.node());
    let ch4 = air.atmosphere.partial[&Gas::Ch4].node();
    assert_eq!(from[0], derived_from(&titan.trace, ch4)[0]);
    assert!(from.contains(&titan.trial.sources.primary().equilibrium.node()));
}

// Review Focus 5: the hold-back changes the outgassed CO2 alone; every
// other gas, the photolysis and the rest of the chain read the same.
#[test]
fn a_held_back_air_differs_only_by_its_co2() {
    let held = planet(&TITAN).air_held(true);
    let free = planet(&TITAN).air_held(false);
    assert!(*held.reducing.value() && !*free.reducing.value());
    for (gas, p) in &held.atmosphere.partial {
        if *gas != Gas::Co2 {
            assert_eq!(p.value(), free.atmosphere.partial[gas].value(), "{gas:?}");
        }
    }
    assert_eq!(held.atmosphere.partial[&Gas::Co2].value().0, 0.0);
    // Without the hold-back Titan outgasses CO2, and its cold traps it
    // as frost. A structural check, no value: that the hold-back is what
    // keeps the frost away, whatever its amount.
    assert!(free.frozen_co2.value().0 > 0.0, "{:?}", free.frozen_co2);
    assert_eq!(held.water, free.water);
}

#[test]
fn venus_and_mars_keep_their_co2() {
    // Venus is above CO2's triple point, so no cap; Mars is below it,
    // but its CO2 stays under the frost pressure. Their totals are
    // `venus_has_ninety_bars_of_co2`'s and `mars_keeps_a_trace`'s.
    let venus = planet(&VENUS).air_held(true);
    let mars = planet(&MARS).air_held(true);
    for air in [&venus, &mars] {
        assert!(!*air.reducing.value());
        assert_eq!(air.frozen_co2.value().0, 0.0, "{air:?}");
    }
}

#[test]
fn each_partial_reads_in_chain_order() {
    let mut titan = planet(&TITAN);
    let air = titan.air_held(false);
    let trace = &titan.trace;
    let code = |id| trace.node(id).unwrap().code.clone();
    // Every final partial derives from the one the shoreline kept.
    for (gas, p) in &air.atmosphere.partial {
        kept(&titan, *gas, p);
    }
    let partial = |gas| air.atmosphere.partial[&gas].node();
    // CH4: kept, photolysed, then the photochemistry.
    let ch4 = partial(Gas::Ch4);
    assert_eq!(code(ch4), Code::of(codes::AIR_PHOTOCHEMISTRY_CH4));
    let photolysed = derived_from(trace, ch4)[0];
    assert_eq!(code(photolysed), Code::of(codes::AIR_PHOTOLYSED_CH4));
    assert_eq!(
        code(derived_from(trace, photolysed)[0]),
        Code::of(codes::AIR_KEPT_CH4)
    );
    // CO2: kept, then trapped.
    let co2 = partial(Gas::Co2);
    assert_eq!(code(co2), Code::of(codes::AIR_TRAPPED_CO2));
    let kept_co2 = derived_from(trace, co2)[0];
    assert_eq!(code(kept_co2), Code::of(codes::AIR_KEPT_CO2));
    // NH3: kept, trapped, then the photochemistry, whose N2 it feeds.
    let nh3 = partial(Gas::Nh3);
    assert_eq!(code(nh3), Code::of(codes::AIR_PHOTOCHEMISTRY_NH3));
    let trapped_nh3 = derived_from(trace, nh3)[0];
    assert_eq!(code(trapped_nh3), Code::of(codes::AIR_TRAPPED_NH3));
    assert_eq!(
        code(derived_from(trace, trapped_nh3)[0]),
        Code::of(codes::AIR_KEPT_NH3)
    );
    let n2 = partial(Gas::N2);
    assert_eq!(code(n2), Code::of(codes::AIR_PHOTOCHEMISTRY_N2));
    let from = derived_from(trace, n2);
    assert_eq!(code(from[0]), Code::of(codes::AIR_KEPT_N2));
    assert_eq!(from[1], trapped_nh3);
    // The others stay as the shoreline and the wash-out kept them.
    for (gas, code_of) in [
        (Gas::H2o, codes::AIR_KEPT_H2O),
        (Gas::So2, codes::AIR_KEPT_SO2),
        (Gas::O2, codes::AIR_KEPT_O2),
    ] {
        assert_eq!(code(partial(gas)), Code::of(code_of), "{gas:?}");
    }
    // The frozen CO2 is the photolysed (kept) CO2 less the trapped CO2.
    assert_eq!(
        derived_from(trace, air.frozen_co2.node()),
        vec![kept_co2, co2]
    );
    let frozen = number(trace, kept_co2) - number(trace, co2);
    assert_eq!(air.frozen_co2.value().0, frozen);
}

// One code path: the photolysis reads the light the timeline carries. On
// a Titan lit as the system lights a planet (the Sun's sources and the
// star's age and metallicity on the trial, as `draw::place` builds
// them), its node cites the stars' inputs, and the whole destruction it
// computes from them is the yield times the Lyman α fluence the
// timeline's sources hold.
#[test]
fn the_photolysis_reads_the_timelines_lyman_alpha() {
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &TITAN);
    let sun = star(
        Some(SolarMass(1.0)),
        Some(*t.metallicity.value()),
        Some(*t.age.value()),
        Seed::new(1),
        &mut trace,
    )
    .unwrap();
    let lit = sources(
        &sun,
        &light(&sun).unwrap(),
        &t.orbit.semi_major_axis,
        &t.orbit.eccentricity,
        &mut trace,
    )
    .unwrap();
    let t = Trial {
        sources: lit,
        age: sun.age,
        metallicity: sun.state.metallicity,
        ..t
    };
    let mut titan = up_to_core(trace, t, drawn);
    let a = titan.air().0;
    let photolysed = ancestor(
        &titan.trace,
        a.partial[&Gas::Ch4].node(),
        codes::AIR_PHOTOLYSED_CH4,
    );
    let from = derived_from(&titan.trace, photolysed);
    let gravity = titan.body.bulk().surface_gravity;
    let source = titan.trial.sources.primary();
    for node in [
        gravity.node(),
        source.flux.node(),
        source.mass.node(),
        titan.trial.metallicity.node(),
        titan.trial.age.node(),
    ] {
        assert!(from.contains(&node), "{node:?}");
    }
    let histories = histories(&titan.trial).unwrap();
    let lyman = lyman_sources(&titan.trial, &histories);
    let g = *gravity.value();
    let destroyed = methane_destroyed(g, &lyman, Gyr(0.0), *titan.trial.age.value()).0;
    let (_, fluence) = titan.trial.sources.band_total(|s| &s.lyman_alpha);
    let expected = CH4_PER_LYMAN_ALPHA_PHOTON * fluence.0 / LYMAN_ALPHA_PHOTON_ENERGY
        * CH4_MOLECULE_MASS.0
        * g.0
        / 4.0
        / PASCALS_PER_BAR;
    assert!(
        (destroyed / expected - 1.0).abs() < ROUNDING,
        "{destroyed} {expected}"
    );
}
