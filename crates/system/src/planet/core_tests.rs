//! Tier 1: the core and its dynamo, ported from
//! `legacy-final:crates/mapgen/src/planet/core.rs` (its tests) on the
//! reference bodies of `planet/tests.rs`, with the calibration targets of
//! study 21, part 4.

use super::codes;
use super::core::{CoreRecord, core};
use super::enveloped::Body;
use super::heat::heat;
use super::interior::{Interior, interior};
use super::regime::regime;
use super::tests::{EARTH, MARS, MERCURY, ReferenceBody, VENUS, derived_from, reference_trial};
use super::water::water;
use crate::Trial;
use science::dynamo::{NUSSELT_LID, NUSSELT_PLATES, cooling_time};
use science::units::{Gyr, Kilogram, KilogramPerCubicMetre, Metre};
use timeline::{Code, CoreState, Material, Provenance, TectonicRegime, Trace, Traced};

/// Source: the Moon in legacy's reference table
/// (`legacy-final:crates/mapgen/src/planet/reference.rs` l.78): 0.0123 M_E
/// (NASA planetary fact sheet), a core of 2 % of the mass (Weber et al.
/// 2011, doi:10.1126/science.1199375: 1 to 2 %), dry, at 1 au. A rocky
/// body of the Moon's mass and core, as legacy tested it: the moons come
/// with a later track (#658).
const MOON: ReferenceBody = ReferenceBody {
    mass: 0.0123,
    core_mass_fraction: 0.02,
    water_fraction: 0.0,
    orbit: 1.0,
    eccentricity: 0.0,
};

/// Definition: the Solar System's age, 4.567 Gyr, legacy's `NOW`.
const NOW: Gyr = Gyr(4.567);

/// Source: Mars young, 0.3 Gyr after its formation, inside Nimmo &
/// Stevenson 2000's early field "for the first ~500 Myr" (study 21,
/// part 4): legacy's test age.
const MARS_YOUNG: Gyr = Gyr(0.3);

/// Source: Earth's surface equatorial dipole, 20 to 70 µT, legacy's test
/// band around "typically 20,000 to 60,000 nT" (NOAA NCEI, World Magnetic
/// Model; study 21, part 4).
const EARTH_FIELD: (f64, f64) = (20.0e-6, 70.0e-6);

/// Source: Mercury's equatorial dipole is about 0.2 µT (Anderson et al.
/// 2011, doi:10.1126/science.1211001, p. 1861: 195 ± 10 nT R_M³): above
/// zero and below 2 µT, legacy's test band.
const MERCURY_FIELD_MAX: f64 = 2.0e-6;

/// Source: Earth's inner core nucleated 565 Ma ago (Bono et al. 2019,
/// Nature Geosci. 12, 143, secondhand) and up to about 1.5 Ga ago in
/// other work (Biggin et al. 2015; study 21, part 4): its age today, Gyr.
const EARTH_INNER_CORE_AGE: (f64, f64) = (0.5, 1.5);

/// Source: Mars's dynamo ended 4.0 to 4.1 Ga ago (Lillis et al. 2013,
/// the calibration's target; Mittelholz et al. 2020 argue for a field as
/// late as 3.7 Ga; study 21, part 4): the end after the planet's
/// formation 4.567 Gyr ago, Gyr.
const MARS_DYNAMO_END: (f64, f64) = (0.467, 0.567);

/// Source: the Moon's dynamo ran from at least 4.25 to 3.56 Ga, and a weak
/// field remains 1 to 2.5 Ga ago (Weiss & Tikoo 2014; Tikoo et al. 2017;
/// study 21, part 4): its core freezes between 2.07 and 3.57 Gyr after its
/// formation.
const MOON_FROZEN: (f64, f64) = (2.067, 3.567);

/// The trial, interior and regime of the reference `body`, as the chain
/// derives them.
fn derive(trace: &mut Trace, body: &ReferenceBody) -> (Trial, Interior, Traced<TectonicRegime>) {
    let (t, drawn) = reference_trial(trace, body);
    let b = interior(&t, &drawn, trace).unwrap();
    let h = heat(
        &t,
        &b,
        &drawn.radiogenic_abundance,
        &drawn.envelope_fraction,
        trace,
    )
    .unwrap();
    let w = water(&t, &drawn, &b, trace).unwrap();
    let r = regime(&t, &b, &h, &w, trace).unwrap();
    let Body::Rocky(interior) = b else {
        panic!("a reference body is rocky")
    };
    (t, interior, r)
}

/// The core of the reference `body` at `age`, in its own regime.
fn core_at(body: &ReferenceBody, age: Gyr) -> CoreRecord {
    let mut trace = Trace::new();
    let (mut t, i, r) = derive(&mut trace, body);
    t.age = trace
        .leaf(Code::of("test.age"), age, Provenance::Input)
        .unwrap();
    core(&t, &i, &r, &mut trace).unwrap()
}

/// The cooling time of the reference `body` with the Nusselt number
/// `nusselt`, from the radius and mass of its iron core.
fn tau_of(body: &ReferenceBody, nusselt: f64) -> Gyr {
    let mut trace = Trace::new();
    let (_, i, _) = derive(&mut trace, body);
    let r_core = i
        .shells
        .iter()
        .filter(|s| *s.material.value() == Material::Iron)
        .map(|s| s.outer_radius.value().0)
        .fold(0.0, f64::max);
    let radius = Metre::from(*i.bulk.radius.value()).0;
    let mass: Kilogram = *i.core_mass.value();
    let density = mass.0 / (4.0 / 3.0 * std::f64::consts::PI * r_core.powi(3));
    cooling_time(
        KilogramPerCubicMetre(density),
        Metre(r_core),
        Metre(radius - r_core),
        nusselt,
    )
}

fn has_dynamo(record: &CoreRecord) -> bool {
    *record.dynamo.value()
}

#[test]
fn the_solar_system_has_its_dynamos() {
    let earth = core_at(&EARTH, NOW);
    assert!(has_dynamo(&earth));
    assert_eq!(*earth.core.state.value(), CoreState::Liquid);
    let field = earth.core.field.value().0;
    assert!(
        (EARTH_FIELD.0..=EARTH_FIELD.1).contains(&field),
        "{field} T"
    );
    let mercury = core_at(&MERCURY, NOW);
    assert!(has_dynamo(&mercury));
    let field = mercury.core.field.value().0;
    assert!(field > 0.0 && field < MERCURY_FIELD_MAX, "{field} T");
    let mars = core_at(&MARS, NOW);
    assert!(!has_dynamo(&mars));
    assert_eq!(*mars.core.state.value(), CoreState::Liquid);
    assert_eq!(mars.core.field.value().0, 0.0);
    assert!(has_dynamo(&core_at(&MARS, MARS_YOUNG)));
    let venus = core_at(&VENUS, NOW);
    assert!(!has_dynamo(&venus));
    assert_eq!(venus.core.field.value().0, 0.0);
    let moon = core_at(&MOON, NOW);
    assert!(!has_dynamo(&moon));
    assert_eq!(*moon.core.state.value(), CoreState::Frozen);
}

#[test]
fn the_phases_meet_their_calibration_targets() {
    use science::dynamo::{CORE_FROZEN, INNER_CORE_START, THERMAL_DYNAMO_END};
    // Earth's inner core started INNER_CORE_START cooling times in.
    let earth = tau_of(&EARTH, NUSSELT_PLATES).0;
    let inner_age = NOW.0 - INNER_CORE_START * earth;
    assert!(
        (EARTH_INNER_CORE_AGE.0..=EARTH_INNER_CORE_AGE.1).contains(&inner_age),
        "Earth's inner core is {inner_age} Gyr old"
    );
    let mars_end = THERMAL_DYNAMO_END * tau_of(&MARS, NUSSELT_LID).0;
    assert!(
        (MARS_DYNAMO_END.0..=MARS_DYNAMO_END.1).contains(&mars_end),
        "Mars's dynamo ends at {mars_end} Gyr"
    );
    let moon_frozen = CORE_FROZEN * tau_of(&MOON, NUSSELT_LID).0;
    assert!(
        (MOON_FROZEN.0..=MOON_FROZEN.1).contains(&moon_frozen),
        "the Moon's core freezes at {moon_frozen} Gyr"
    );
}

#[test]
fn no_iron_no_core() {
    let coreless = ReferenceBody {
        core_mass_fraction: 0.0,
        water_fraction: 0.0,
        ..EARTH
    };
    let record = core_at(&coreless, NOW);
    assert_eq!(*record.core.state.value(), CoreState::None);
    assert!(!has_dynamo(&record));
    assert_eq!(record.core.field.value().0, 0.0);
}

#[test]
fn the_trace_says_why_earth_has_a_field() {
    let mut trace = Trace::new();
    let (t, i, r) = derive(&mut trace, &EARTH);
    let record = core(&t, &i, &r, &mut trace).unwrap();
    let code = |node| trace.node(node).unwrap().code.clone();
    assert_eq!(code(record.core.state.node()), Code::of(codes::CORE_STATE));
    assert_eq!(code(record.dynamo.node()), Code::of(codes::CORE_DYNAMO));
    assert_eq!(code(record.core.field.node()), Code::of(codes::CORE_FIELD));
    // The state derives from the cooling time and the age; the cooling
    // time from the core's mass and radius, the planet's radius and the
    // regime.
    let state_from = derived_from(&trace, record.core.state.node());
    let tau = state_from[0];
    assert_eq!(code(tau), Code::of(codes::CORE_COOLING_TIME));
    assert_eq!(state_from[1], t.age.node());
    let tau_from = derived_from(&trace, tau);
    assert!(tau_from.contains(&i.core_mass.node()));
    assert!(tau_from.contains(&i.bulk.radius.node()));
    assert!(tau_from.contains(&r.node()));
    // The dynamo reads the regime (plates keep it running); the field
    // derives from the dynamo.
    assert!(derived_from(&trace, record.dynamo.node()).contains(&r.node()));
    assert!(derived_from(&trace, record.core.field.node()).contains(&record.dynamo.node()));
}
