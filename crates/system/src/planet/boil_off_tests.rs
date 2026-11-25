//! Tier 1: the boil-off of an envelope (Tang, Fortney & Murray-Clay
//! 2024's ceiling) and its place in the chain, before the XUV escape.

use super::bare::bare_with;
use super::boil_off::boiled_off;
use super::codes;
use super::draws::Drawn;
use super::enveloped::Body;
use super::interior::interior;
use super::tests::{
    EARTH_CORE_MASS_FRACTION, EARTH_WATER_FRACTION, ancestry, derived_from, jupiter_at, trial,
    with_fluence,
};
use crate::Trial;
use science::escape::{MIN_ENVELOPE, boil_off_ceiling};
use science::units::{EarthFlux, EarthMass};
use timeline::{Code, Trace};

/// Source: Tang, Fortney & Murray-Clay 2024 (arXiv:2410.08577), Table 2:
/// a core of 1.92 M⊕ under 802 F⊕ lies between the 1.5 and 2.4 M⊕
/// columns and the 300 and 1 000 F⊕ rows, four cells that keep nothing
/// ("0", or starred: the whole envelope lost). The drawn envelope, 2.74 %,
/// is study 30's reference placement ref 8 d, a planet of an A star.
const A_STAR_PLANET: (f64, f64, f64) = (1.92, 802.0, 2.74e-2);

/// Source: Lammer et al. 2014 (MNRAS, arXiv:1401.2765), Tables 1 to 3:
/// the largest envelope a 1 M⊕ core captures in a nebula at 1 au, 2.5 %
/// (`planet::envelope`'s grid).
const LAMMER_EARTH_TOP: f64 = 2.5e-2;

/// Source: Tang et al. 2024, Table 2, a "−" cell ("zero or very slight
/// mass loss"): a core of 8.5 M⊕ at 10 F⊕, about 0.32 au round the Sun.
/// With a 20 % envelope, Lopez & Fortney 2014's last column.
const TEMPERATE_NEPTUNE: (f64, f64, f64) = (8.5, 10.0, 0.20);

/// Definition: a numerical tolerance on a mass computed two ways,
/// relative.
const ROUNDING: f64 = 1e-12;

/// Definition: a numerical tolerance on a mass lost, the difference of two
/// masses computed two ways, relative: the round-off of the masses
/// (`ROUNDING`) grows by the ratio of the mass to the mass lost, up to
/// 1e3 for the envelopes here.
const DIFFERENCE_ROUNDING: f64 = 1e-9;

/// A dry planet whose core (its mass without the envelope) is `core` M⊕,
/// under `flux` F⊕ from the trial's Sun-like source, with the envelope
/// fraction `envelope` (Earth's core and water fractions).
fn planet(trace: &mut Trace, core: f64, flux: f64, envelope: f64) -> (Trial, Drawn) {
    let t = trial(trace, 1, core / (1.0 - envelope), flux.powf(-0.5), 0.0);
    let drawn = Drawn::fixed(
        &t,
        trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        Some(false),
        envelope,
        None,
        None,
    );
    (t, drawn)
}

#[test]
fn a_close_planet_of_an_a_star_boils_bare() {
    // No XUV: an A star has no corona. Before the boil-off, nothing took
    // this envelope and the planet kept its 2.74 %.
    let mut trace = Trace::new();
    let (core, flux, envelope) = A_STAR_PLANET;
    let (lit, drawn) = planet(&mut trace, core, flux, envelope);
    let dark = with_fluence(&mut trace, &lit, 0.0);
    let b = bare_with(&dark, &drawn, &mut trace).unwrap();
    assert_eq!(b.drawn.envelope_fraction.value().get(), 0.0);
    assert!(b.surface.is_some());
    assert!(matches!(b.body, Body::Rocky(_)), "{:?}", b.body);
    let kept = b.trial.mass.value().0;
    assert!(
        (kept / core - 1.0).abs() < ROUNDING,
        "{kept} against {core}"
    );
    // The envelope fraction's cause reaches the ceiling.
    let ceiling = ancestry(&trace, b.drawn.envelope_fraction.node())
        .into_iter()
        .any(|id| trace.node(id).unwrap().code == Code::of(codes::BOIL_OFF_CEILING));
    assert!(ceiling, "the stripped fraction derives from the ceiling");
    assert!(b.envelope_lost.is_none(), "no XUV, no XUV loss");

    // The planet carries the envelope boiled off: the drawn mass less the
    // mass left under the ceiling, `M − M_c / (1 − c)`, with its cause.
    let boiled = b.envelope_boiled.expect("the boil-off cut the envelope");
    let c = boil_off_ceiling(EarthMass(core), EarthFlux(flux)).get();
    let expected = lit.mass.value().0 - core / (1.0 - c);
    let recorded = EarthMass::from(*boiled.value()).0;
    assert!(
        (recorded / expected - 1.0).abs() < DIFFERENCE_ROUNDING,
        "{recorded} against {expected}"
    );
    assert_eq!(
        trace.node(boiled.node()).unwrap().code,
        Code::of(codes::ENVELOPE_BOILED_OFF)
    );
    let causes = ancestry(&trace, boiled.node());
    for (cause, what) in [
        (lit.mass.node(), "the drawn mass"),
        (drawn.envelope_fraction.node(), "the drawn envelope"),
    ] {
        assert!(
            causes.contains(&cause),
            "the envelope boiled off cites {what}"
        );
    }
    assert!(
        causes
            .iter()
            .any(|&id| trace.node(id).unwrap().code == Code::of(codes::BOIL_OFF_CEILING)),
        "the envelope boiled off cites the ceiling"
    );
}

#[test]
fn earth_with_lammers_top_envelope_keeps_some_at_one_au() {
    // Tang et al.'s ceiling at 1 M⊕ and 1 F⊕ (0.48 %, extended below the
    // 1.5 M⊕ column) trims a 2.5 % envelope, and the XUV escape at 1 au
    // leaves it above `MIN_ENVELOPE`: Earth's bareness comes from
    // capturing little (Lammer et al. 2014), not from the boil-off.
    let mut trace = Trace::new();
    let (t, drawn) = planet(&mut trace, 1.0, 1.0, LAMMER_EARTH_TOP);
    let b = bare_with(&t, &drawn, &mut trace).unwrap();
    let kept = b.drawn.envelope_fraction.value().get();
    let ceiling = boil_off_ceiling(EarthMass(1.0), EarthFlux(1.0)).get();
    assert!(kept >= MIN_ENVELOPE, "{kept}");
    assert!(kept <= ceiling, "{kept} above the ceiling {ceiling}");
    assert!(b.surface.is_none());
}

#[test]
fn the_cut_envelope_derives_from_the_core_and_the_flux() {
    let mut trace = Trace::new();
    let (t, drawn) = planet(&mut trace, 1.0, 1.0, LAMMER_EARTH_TOP);
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let Body::MiniNeptune { envelope, .. } = &body else {
        panic!("{body:?}")
    };
    let (core, flux) = (envelope.mass_without, envelope.flux);
    let left = boiled_off(&t, &drawn, body.clone(), &mut trace).unwrap();
    let ceiling = boil_off_ceiling(*core.value(), *flux.value());

    // The fraction left is the ceiling; the mass is the core under it.
    let fraction = left.drawn.envelope_fraction;
    assert_eq!(*fraction.value(), ceiling);
    let expected = core.value().0 / (1.0 - ceiling.get());
    let mass = left.trial.mass.value().0;
    assert!(
        (mass / expected - 1.0).abs() < ROUNDING,
        "{mass} against {expected}"
    );

    let boiled = left.boiled.expect("the envelope was cut");
    assert_eq!(
        trace.node(boiled.node()).unwrap().code,
        Code::of(codes::ENVELOPE_BOILED_OFF)
    );
    let lost = t.mass.value().0 - mass;
    let recorded = EarthMass::from(*boiled.value()).0;
    assert!(
        (recorded / lost - 1.0).abs() < DIFFERENCE_ROUNDING,
        "{recorded} against {lost}"
    );

    assert_eq!(
        trace.node(fraction.node()).unwrap().code,
        Code::of(codes::ENVELOPE_AFTER_BOIL_OFF)
    );
    assert_eq!(
        trace.node(left.trial.mass.node()).unwrap().code,
        Code::of(codes::MASS_AFTER_BOIL_OFF)
    );
    let from = derived_from(&trace, fraction.node());
    assert!(from.contains(&drawn.envelope_fraction.node()));
    let ceiling_node = from
        .iter()
        .copied()
        .find(|&id| trace.node(id).unwrap().code == Code::of(codes::BOIL_OFF_CEILING))
        .expect("the fraction left cites the ceiling");
    let from = derived_from(&trace, ceiling_node);
    assert!(from.contains(&core.node()) && from.contains(&flux.node()));
    assert_eq!(
        trace.node(core.node()).unwrap().code,
        Code::of(codes::MASS_WITHOUT_ENVELOPE)
    );
    assert_eq!(
        trace.node(flux.node()).unwrap().code,
        Code::of(codes::BOLOMETRIC_FLUX)
    );
    // The body is built again on the envelope left.
    assert!(
        left.body.bulk().radius.value().0 < body.bulk().radius.value().0,
        "a thinner envelope, a smaller planet"
    );
    assert_eq!(left.body.bulk().mass, left.trial.mass);
}

#[test]
fn an_envelope_under_its_ceiling_is_untouched() {
    // A "−" cell never cuts a draw: nothing is recorded.
    let mut trace = Trace::new();
    let (core, flux, envelope) = TEMPERATE_NEPTUNE;
    let (t, drawn) = planet(&mut trace, core, flux, envelope);
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let nodes = trace.len();
    let left = boiled_off(&t, &drawn, body.clone(), &mut trace).unwrap();
    assert_eq!(left.body, body);
    assert_eq!(left.trial, t);
    assert_eq!(left.drawn, drawn);
    assert_eq!(left.boiled, None);
    assert_eq!(trace.len(), nodes);
}

#[test]
fn a_rocky_planet_and_a_giant_are_not_read() {
    // Rocky: no envelope to boil. A giant, from 20 M⊕: off Tang et al.'s
    // grid, whose heaviest core is 20 M⊕; Owen & Wu 2016 find planets
    // above 10 M⊕ little affected (conclusion 5). Both close in, at
    // 0.05 au.
    let mut trace = Trace::new();
    let (rocky, rocky_drawn) = planet(&mut trace, 1.0, 400.0, 0.0);
    let (giant, giant_drawn) = jupiter_at(&mut trace, 0.05);
    for (t, drawn) in [(rocky, rocky_drawn), (giant, giant_drawn)] {
        let body = interior(&t, &drawn, &mut trace).unwrap();
        let nodes = trace.len();
        let left = boiled_off(&t, &drawn, body.clone(), &mut trace).unwrap();
        assert_eq!(left.body, body);
        assert_eq!(left.boiled, None);
        assert_eq!(trace.len(), nodes);
    }
}
