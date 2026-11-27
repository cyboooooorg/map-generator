//! The planets of the shared reference systems, modelled by the planet
//! chain (`system::planet`).
//!
//! Also the determinism of the draw: its pinned digests and its purity.
//!
//! One test, several named checks: nextest runs each test in its own
//! process, and every process that calls `regression::references()` draws
//! and models the eight systems again (minutes in debug). Each check
//! panics with its own name, so a failure says which one.

use std::collections::BTreeSet;

use regression::{LoggedRead, Reference, digest, reference_requests, references};
use science::escape::MIN_ENVELOPE;
use system::planet::class_of;
use system::{ArchetypeId, Placement, draw_with};
use timeline::{Biosphere, Code, LiquidKind, NodeId, Origin, PlanetClass, PlanetState, Trace};

/// Definition: the index of the `earth-like` request in
/// `regression::reference_requests()` (seed 7).
const EARTH_LIKE_REFERENCE: usize = 6;

/// Calibration: the tolerance on a pinned surface temperature, K, the
/// albedo's fixed point `ALBEDO_TOLERANCE_K` of
/// `system::planet::climate` (0.1 K, study 25, "Solver constants"): a
/// change of the draw under it is not a change of the planet.
const TEMPERATURE_TOLERANCE_K: f64 = 0.1;

/// Definition: the classes the spin draws with the giant law, as
/// `system::spin::is_giant` (private to the crate): a body of one of them
/// has no surface.
fn is_giant(class: PlanetClass) -> bool {
    matches!(
        class,
        PlanetClass::GasGiant | PlanetClass::HotJupiter | PlanetClass::IceGiant
    )
}

fn name(reference: &Reference, placement: &Placement) -> String {
    format!(
        "{} (seed {})",
        placement.designation,
        reference.inputs.seed.get()
    )
}

/// Every placement of every reference with its state, designation first.
fn planets() -> impl Iterator<
    Item = (
        usize,
        &'static Reference,
        &'static Placement,
        &'static PlanetState,
    ),
> {
    references().iter().enumerate().flat_map(|(i, r)| {
        r.drawn
            .placements
            .iter()
            .zip(&r.planets)
            .map(move |(placement, state)| (i, r, placement, state))
    })
}

/// The logged reads of the circular trial of `placement`: the draw
/// classified it with the same seed, mass and semi-major axis.
fn reads_of<'a>(reference: &'a Reference, placement: &Placement) -> Vec<&'a LoggedRead> {
    let mass = placement.trial.mass.value().0.to_bits();
    let axis = placement.trial.orbit.semi_major_axis.value().0.to_bits();
    reference
        .reads
        .iter()
        .filter(|r| r.seed == placement.trial.seed && r.mass_bits == mass && r.axis_bits == axis)
        .collect()
}

#[test]
fn the_reference_planets() {
    golden_digests();
    the_draw_is_pure_within_a_run();
    asked_archetypes_are_placed();
    every_reference_planet_is_modelled();
    every_trial_seed_is_unique();
    the_earth_like_reference_has_a_surface();
    the_obliquity_class_of_read_is_the_placements();
    class_changes_between_draw_and_model_are_pinned();
    the_reference_planets_are_pinned();
    living_planets_keep_their_stage();
    boiled_off_placements_end_bare();
}

/// The pinned digests of the reference systems, drawn with the planet
/// model's `class_of` (engine 4; before it, the stand-in classifier and
/// spins from the body seed). Pinned from a local macOS run: CI runs on
/// Linux only on pull requests. macOS and Linux gave the same values for
/// the stand-in digests (PR #629); if their libm differs by an ulp and
/// flips a threshold of the draw, split this table per `target_os`.
const EXPECTED: [u64; 8] = [
    7717621872393268112,
    13277582001339794335,
    4485924109708704685,
    15676482809518794994,
    10643121030845601339,
    6233997479138571296,
    7398689851432714755,
    3500269132337718553,
];

/// The digests of the reference systems match the table. A change means
/// every drawn system changes: decide it on purpose, then paste the values
/// the failure prints into the table.
fn golden_digests() {
    let digests: Vec<u64> = references().iter().map(|r| digest(&r.drawn)).collect();
    assert_eq!(digests, EXPECTED, "golden digests: {digests:?}");
}

/// The draw is a pure function of its inputs: drawing the first reference
/// request again gives the digest of the shared draw.
fn the_draw_is_pure_within_a_run() {
    let request = &reference_requests()[0];
    let again = draw_with(request, &mut Trace::new(), class_of).unwrap();
    assert_eq!(digest(&again), digest(&references()[0].drawn), "purity");
}

/// An asked archetype is placed: reference 7 asks for an earth-like and
/// one of its placements carries it (the tier-3 CLI test checks the
/// report; this keeps the coverage on the shared draw).
fn asked_archetypes_are_placed() {
    let earth_like = ArchetypeId(Code::of("earth-like"));
    assert!(
        references()[EARTH_LIKE_REFERENCE]
            .drawn
            .placements
            .iter()
            .any(|p| p.trial.archetype.as_ref() == Some(&earth_like)),
        "reference 7 has no earth-like placement"
    );
}

fn every_reference_planet_is_modelled() {
    let mut count = 0;
    for r in references() {
        assert_eq!(
            r.planets.len(),
            r.drawn.placements.len(),
            "{}",
            r.drawn.designation
        );
        for state in &r.planets {
            state.validate().unwrap();
            count += 1;
        }
    }
    assert!(count > 0);
}

fn every_trial_seed_is_unique() {
    for r in references() {
        let seeds: BTreeSet<u64> = r
            .drawn
            .placements
            .iter()
            .map(|p| p.trial.seed.get())
            .collect();
        assert_eq!(
            seeds.len(),
            r.drawn.placements.len(),
            "trial seeds repeat in {}",
            r.drawn.designation
        );
    }
}

fn the_earth_like_reference_has_a_surface() {
    // The archetype's promise (a definition, not a pin): an asked
    // earth-like is a planet with a surface, whatever its envelope draw
    // would have given (#695). Folded into the shared build (ruling of
    // 2025-11-25: its own draw alone took 138 s of tier 2).
    let earth_like = ArchetypeId(Code::of("earth-like"));
    let r = &references()[EARTH_LIKE_REFERENCE];
    let (placement, state) = r
        .drawn
        .placements
        .iter()
        .zip(&r.planets)
        .find(|(p, _)| p.trial.archetype.as_ref() == Some(&earth_like))
        .expect("reference 7 asks an earth-like");
    assert!(
        state.surface.is_some(),
        "{} {:?}",
        placement.designation,
        state.class
    );
    assert_eq!(state.envelope_fraction.value().get(), 0.0);
}

fn the_obliquity_class_of_read_is_the_placements() {
    // mapgen3's condition (B2): a body with a surface has a class without
    // the giant law, and the obliquity `class_of` read of its trial is the
    // placement's, bit for bit (it does not depend on the radius); a
    // giant's reads have none.
    let mut checked = 0;
    for (_, r, placement, state) in planets() {
        let reads = reads_of(r, placement);
        assert!(
            !reads.is_empty(),
            "{}: no read of its trial",
            name(r, placement)
        );
        if state.surface.is_some() {
            assert!(!is_giant(*state.class.value()), "{}", name(r, placement));
            for read in reads {
                let obliquity = read.read.obliquity.unwrap_or_else(|| {
                    panic!(
                        "{}: a body with a surface read no obliquity",
                        name(r, placement)
                    )
                });
                assert_eq!(
                    obliquity.0.to_bits(),
                    placement.spin.obliquity.value().0.to_bits(),
                    "{}",
                    name(r, placement)
                );
                checked += 1;
            }
        } else if is_giant(*state.class.value()) {
            for read in reads {
                assert!(read.read.obliquity.is_none(), "{}", name(r, placement));
            }
        }
    }
    assert!(checked > 0);
}

/// The draw's class of `placement`: what the planet model read of its
/// circular trial.
fn draw_class(reference: &Reference, placement: &Placement) -> PlanetClass {
    let reads = reads_of(reference, placement);
    let first = reads.first().expect("the draw classified the trial");
    assert!(
        reads
            .iter()
            .all(|r| r.read.outcome.class == first.read.outcome.class),
        "{}: two reads of one trial differ",
        name(reference, placement)
    );
    first.read.outcome.class
}

/// Pin (first run, 2026-11-27, macOS): the placements whose class changes
/// between the draw (circular trial) and the model (drawn eccentric
/// orbit, higher mean flux, K1): system, designation, eccentricity, draw
/// class, model class.
const CLASS_CHANGES: &[(&str, &str, f64, &str, &str)] = &[];

fn class_changes_between_draw_and_model_are_pinned() {
    let mut changes = Vec::new();
    for (_, r, placement, state) in planets() {
        let drawn_class = draw_class(r, placement);
        let e = placement.trial.orbit.eccentricity.value().get();
        if drawn_class != *state.class.value() {
            assert!(
                e > 0.0,
                "{}: a circular trial changed class",
                name(r, placement)
            );
            changes.push((
                r.drawn.designation.to_string(),
                placement.designation.to_string(),
                e,
                drawn_class.code(),
                state.class.value().code(),
            ));
        } else if e == 0.0 {
            // Same trial, same answer: nothing to count.
        }
    }
    let actual: Vec<String> = changes
        .iter()
        .map(|(s, d, e, a, b)| format!("({s:?}, {d:?}, {e:.6}, {a:?}, {b:?}),"))
        .collect();
    let expected: Vec<String> = CLASS_CHANGES
        .iter()
        .map(|(s, d, e, a, b)| format!("({s:?}, {d:?}, {e:.6}, {a:?}, {b:?}),"))
        .collect();
    assert_eq!(
        actual,
        expected,
        "class changes ({}):\n{}",
        changes.len(),
        actual.join("\n")
    );
}

/// Pin (first run, 2026-11-27, macOS; a regression net, not a source):
/// designation, class, liquid, biosphere, surface temperature in K. The
/// temperature is within `TEMPERATURE_TOLERANCE_K`; the rest is exact.
const PLANETS: &[(&str, &str, &str, &str, Option<f64>)] = &[
    ("MGS 17371 b@1", "hot-jupiter", "none", "none", None),
    ("MGS 17371 c@1", "gas-giant", "none", "none", None),
    ("MGS 19340 b@2", "hot-jupiter", "none", "none", None),
    ("MGS 19340 c@2", "gas-giant", "none", "none", None),
    ("MGS 39555 b@3", "gas-giant", "none", "none", None),
    ("MGS 39555 c@3", "gas-giant", "none", "none", None),
    ("MGS 23899 b@4", "hot-jupiter", "none", "none", None),
    ("MGS 84266 b@5", "rocky", "none", "lifeless", Some(821.6)),
    ("MGS 84266 c@5", "gas-giant", "none", "none", None),
    ("MGS 84266 d@5", "gas-giant", "none", "none", None),
    (
        "MGS 43177 b@6",
        "runaway-greenhouse",
        "none",
        "lifeless",
        Some(8274.0),
    ),
    (
        "MGS 43177 c@6",
        "runaway-greenhouse",
        "none",
        "lifeless",
        Some(10333.8),
    ),
    (
        "MGS 43177 d@6",
        "runaway-greenhouse",
        "none",
        "lifeless",
        Some(5316.6),
    ),
    (
        "MGS 43177 e@6",
        "runaway-greenhouse",
        "none",
        "lifeless",
        Some(3517.4),
    ),
    (
        "MGS 43177 f@6",
        "runaway-greenhouse",
        "none",
        "lifeless",
        Some(2297.2),
    ),
    ("MGS 20230 b@7", "rocky", "water", "lifeless", Some(288.0)),
    ("MGS 20230 c@7", "gas-giant", "none", "none", None),
    ("MGS 65909 b@8", "hot-neptune", "none", "none", None),
    ("MGS 65909 c@8", "hot-neptune", "none", "none", None),
    (
        "MGS 65909 d@8",
        "runaway-greenhouse",
        "none",
        "lifeless",
        Some(8760.5),
    ),
    (
        "MGS 65909 e@8",
        "steam-world",
        "none",
        "lifeless",
        Some(5502.6),
    ),
    ("MGS 65909 f@8", "gas-giant", "none", "none", None),
];

fn row(state: &PlanetState, designation: String) -> (String, String, String, String, Option<f64>) {
    let surface = state.surface.as_ref();
    (
        designation,
        state.class.value().code().to_owned(),
        surface
            .and_then(|s| s.liquid.as_ref())
            .map_or("none", |l| l.kind.value().code())
            .to_owned(),
        surface.map_or("none", |s| s.life.value().code()).to_owned(),
        surface.map(|s| s.climate.surface_temperature.value().0),
    )
}

fn the_reference_planets_are_pinned() {
    let actual: Vec<_> = planets()
        .map(|(_, r, p, state)| {
            row(
                state,
                p.designation.to_string() + &format!("@{}", r.inputs.seed.get()),
            )
        })
        .collect();
    let printed = actual
        .iter()
        .map(|(d, c, l, b, t)| format!("({d:?}, {c:?}, {l:?}, {b:?}, {t:.1?}),"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(actual.len(), PLANETS.len(), "pinned table:\n{printed}");
    for (got, want) in actual.iter().zip(PLANETS) {
        assert_eq!(
            (
                got.0.as_str(),
                got.1.as_str(),
                got.2.as_str(),
                got.3.as_str()
            ),
            (want.0, want.1, want.2, want.3),
            "pinned table:\n{printed}"
        );
        match (got.4, want.4) {
            (Some(g), Some(w)) => assert!(
                (g - w).abs() <= TEMPERATURE_TOLERANCE_K,
                "{}: {g} K against {w} K",
                got.0
            ),
            (None, None) => {}
            other => panic!("{}: temperature {other:?}\n{printed}", got.0),
        }
    }
}

/// Pin (first run, 2026-11-27, macOS): how many reference planets have a
/// biosphere (none: the one earth-like planet drew `lifeless`), and how
/// many of them have no liquid water in their second climate.
const LIVING: usize = 0;
const LIVING_WITHOUT_WATER_AFTER_OXYGEN: usize = 0;

fn living_planets_keep_their_stage() {
    // Review Focus 1, S8: life needs liquid water in the first climate (the
    // life node's first parent is its liquid). The second climate, solved
    // on the oxygenated air, may lose it; the stage stays, and the number
    // that lost it is pinned.
    let mut living = 0;
    let mut lost = 0;
    for (_, r, placement, state) in planets() {
        let Some(surface) = state.surface.as_ref() else {
            continue;
        };
        let life = *surface.life.value();
        if life == Biosphere::Lifeless {
            continue;
        }
        living += 1;
        let node = r.trace.node(surface.life.node()).unwrap();
        let Origin::Derived { from } = &node.origin else {
            panic!("{}: the life node is a leaf", name(r, placement));
        };
        let first = r.trace.node(from[0]).unwrap();
        assert_eq!(
            first.value,
            timeline::NodeValue::Code(Code::of(LiquidKind::Water.code())),
            "{}: life without liquid water in the first climate ({})",
            name(r, placement),
            first.code.as_str()
        );
        let water_now = surface
            .liquid
            .as_ref()
            .is_some_and(|l| *l.kind.value() == LiquidKind::Water);
        if !water_now {
            lost += 1;
        }
    }
    assert_eq!(living, LIVING, "living reference planets");
    assert_eq!(
        lost, LIVING_WITHOUT_WATER_AFTER_OXYGEN,
        "{lost} of {living} living planets lost their liquid water"
    );
}

/// The codes of the ancestors of `id` (itself included) that contain
/// `needle`.
fn ancestors_coded(trace: &Trace, id: NodeId, needle: &str) -> BTreeSet<String> {
    let mut pending = vec![id];
    let mut seen = BTreeSet::new();
    let mut found = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = trace.node(id).unwrap();
        if node.code.as_str().contains(needle) {
            found.insert(node.code.as_str().to_owned());
        }
        if let Origin::Derived { from } = &node.origin {
            pending.extend(from);
        }
    }
    found
}

fn boiled_off_placements_end_bare() {
    // Study 30 (#697): Tang, Fortney & Murray-Clay 2024's ceiling leaves
    // no envelope (kept fraction under `MIN_ENVELOPE`) to references 6 c
    // and d and 8 d and e. An expected value from the study, not a pin.
    // The boil-off's ceiling and the boiled mass
    // (`planet.envelope.boiled-off`) are in the cause tree of its outputs.
    let mut problems = Vec::new();
    for (reference, letters) in [(5_usize, ["c", "d"]), (7, ["d", "e"])] {
        let r = &references()[reference];
        for letter in letters {
            let (placement, state) = r
                .drawn
                .placements
                .iter()
                .zip(&r.planets)
                .find(|(p, _)| p.designation.to_string().ends_with(letter))
                .unwrap_or_else(|| panic!("reference {} has no {letter}", reference + 1));
            let who = name(r, placement);
            let kept = state.envelope_fraction.value().get();
            if kept >= MIN_ENVELOPE {
                problems.push(format!("{who}: keeps {kept}"));
            }
            let outputs = [
                state.envelope_fraction.node(),
                state.class.node(),
                state.bulk.mass.node(),
            ];
            let mut boil = BTreeSet::new();
            for node in outputs {
                boil.extend(ancestors_coded(&r.trace, node, "boil"));
            }
            for code in [
                "planet.envelope.boil-off-ceiling",
                "planet.envelope.boiled-off",
            ] {
                if !boil.contains(code) {
                    problems.push(format!(
                        "{who}: kept {kept}; no {code} in its causes, boil nodes found: {boil:?}"
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
