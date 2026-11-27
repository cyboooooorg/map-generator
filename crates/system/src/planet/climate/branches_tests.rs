use super::ClimateInputs;
use super::THAW_STEPS;
use super::branches::{IcyStart, Solved, THAW_FLOOR, frozen_branch, solve, thaw_search};
use super::fixture::{earth, moved, titan};
use science::surface::Liquid;
use science::units::Kilogram;
use timeline::ClimateState;

/// Source: the share of its history Earth spent in dated snowball
/// states, 0.013 to 0.016: the Sturtian and Marinoan glaciations, 63 My
/// over 4.6 Gyr, within the dating bounds of Hoffman et al. 2017
/// (*Sci. Adv.* 3, e1600983, Table 1; derived in
/// `docs/science/planet-atmospheres.md`, "Calibration of
/// `SNOWBALL_WARM_BAR` (refit)").
const HOFFMAN_SHARE: (f64, f64) = (0.013, 0.016);

/// Definition: a test tolerance, the test's own bound on a snowball's
/// Bond albedo, 0.5: an ocean under ice reflects more than it absorbs.
/// Not a measured albedo: the bound of legacy's
/// `a_snowball_keeps_its_ocean_frozen`, kept by the brief.
const SNOWBALL_LEAST_ALBEDO: f64 = 0.5;

/// Definition: a test tolerance, the test's own bound on a snowball's
/// mean surface, 260 K, well under the melting of ice. Not a measured
/// temperature: the bound of legacy's `a_snowball_keeps_its_ocean_frozen`,
/// kept by the brief.
const SNOWBALL_WARMEST_K: f64 = 260.0;

/// Definition: an orbit past Earth's outer edge, 5 AU (legacy's
/// `far_planets_only_freeze`), well beyond Kopparapu et al. 2014's
/// maximum greenhouse at about 1.7 AU.
const FAR_ORBIT_AU: f64 = 5.0;

/// Definition: a farther orbit, 10 AU (legacy's
/// `far_planets_only_freeze`), where Earth's nitrogen condenses.
const FARTHER_ORBIT_AU: f64 = 10.0;

/// Definition: a snowball draw under any chance, 0: frozen wherever the
/// body is bistable.
const COLD_DRAW: f64 = 0.0;

/// Definition: a snowball draw over any chance, 1: never frozen where a
/// temperate branch exists.
const WARM_DRAW: f64 = 1.0;

#[test]
#[ignore = "heavy: two full solves of the bistable Earth, about 65 s each at opt-level 0"]
fn earth_snowballs_as_often_as_hoffman_dates() {
    let inputs = earth();
    assert!(inputs.water.0 > 0.0);
    let warm = solve(&inputs, WARM_DRAW);
    let bistable = warm.bistable.expect("Earth at 1 AU is bistable");
    let thaw = bistable.thaw.expect("a CO₂ Earth can hold thaws it");
    assert!(thaw.0.is_finite() && thaw.0 > bistable.froze.0);
    assert!(bistable.still_frozen.0 < thaw.0);
    let chance = bistable.chance.get();
    println!(
        "froze {:.6} bar, thaw {:.6} bar, d {:.6} bar, volcanism {:.6}, chance {chance:.5}",
        bistable.froze.0,
        thaw.0,
        thaw.0 - bistable.froze.0,
        inputs.volcanism.get()
    );
    assert!(
        (HOFFMAN_SHARE.0..=HOFFMAN_SHARE.1).contains(&chance),
        "{chance}"
    );
    assert_eq!(warm.state, ClimateState::Temperate);
    assert_eq!(warm.settle.liquid, Some(Liquid::Water));

    let cold = solve(&inputs, COLD_DRAW);
    assert_eq!(cold.state, ClimateState::Snowball, "{cold:?}");
    assert_eq!(cold.bistable, warm.bistable);
    assert_eq!(cold.settle.liquid, None, "{cold:?}");
    assert!(
        cold.settle.bond_albedo.get() > SNOWBALL_LEAST_ALBEDO,
        "{cold:?}"
    );
    assert!(cold.settle.temperature.0 < SNOWBALL_WARMEST_K, "{cold:?}");
}

/// Earth's inputs moved to `au`, past its outer edge, solved with the
/// draw under any chance: no frozen branch, so the draw is not read, and
/// frozen for good at a finite, positive temperature (Review Focus 1).
fn only_freezes(au: f64) -> Solved {
    let cold = solve(&moved(&earth(), au), COLD_DRAW);
    assert_eq!(cold.bistable, None, "{au} AU");
    assert_eq!(cold.state, ClimateState::Frozen, "{au} AU: {cold:?}");
    let t = cold.settle.temperature.0;
    assert!(t.is_finite() && t > 0.0, "{au} AU: {t}");
    cold
}

#[test]
fn far_planets_only_freeze_whatever_the_draw() {
    // `solve_with` reads the draw only for a bistable body: with no
    // frozen branch (`only_freezes` asserts it), the warm draw gives the
    // same climate, so one solve per orbit holds both (legacy
    // `far_planets_only_freeze` solved both draws).
    for au in [FAR_ORBIT_AU, FARTHER_ORBIT_AU] {
        only_freezes(au);
    }
}

#[test]
fn a_dry_cold_body_is_temperate() {
    let inputs = ClimateInputs {
        water: Kilogram(0.0),
        ..moved(&earth(), FARTHER_ORBIT_AU)
    };
    let solved = solve(&inputs, COLD_DRAW);
    assert_eq!(solved.bistable, None);
    assert_eq!(solved.state, ClimateState::Temperate, "{solved:?}");
}

#[test]
fn a_hydrocarbon_sea_world_is_temperate() {
    // Titan: water ice under a methane sea and no plates, frozen over
    // but not frozen for good: no carbonate thermostat to build its CO₂.
    let solved = solve(&titan(), COLD_DRAW);
    assert_eq!(solved.bistable, None);
    assert_eq!(
        solved.settle.liquid,
        Some(Liquid::Hydrocarbons),
        "{solved:?}"
    );
    assert_eq!(solved.state, ClimateState::Temperate, "{solved:?}");
}

/// Definition: test inputs, a synthetic thaw search in `ln CO₂` from 1
/// to 100 bar, its icy start thawing from 30 bar up and, in the cycling
/// case, cycling from 20 to 30 bar. Not a body's CO₂.
const SEARCH: (f64, f64) = (1.0, 100.0);

/// Definition: the synthetic thaw and the bottom of its cycling band,
/// bar ([`SEARCH`]).
const SYNTHETIC_THAW: f64 = 30.0;
const SYNTHETIC_BAND: f64 = 20.0;

/// The synthetic search's steps, the solver's.
const SEARCH_STEPS: usize = THAW_STEPS;

/// The synthetic icy start at `co2`, counting its calls in `calls`.
fn synthetic(co2: f64, band: bool, calls: &mut usize) -> IcyStart {
    *calls += 1;
    if co2 >= SYNTHETIC_THAW {
        IcyStart::Thaws
    } else if band && co2 >= SYNTHETIC_BAND {
        IcyStart::Cycles
    } else {
        IcyStart::StaysFrozen
    }
}

#[test]
fn a_thaw_search_with_no_cycle_bisects_every_step() {
    let mut calls = 0;
    let (still_frozen, thaw) = thaw_search(SEARCH.0, SEARCH.1, SEARCH_STEPS, |c| {
        synthetic(c, false, &mut calls)
    });
    assert_eq!(calls, SEARCH_STEPS);
    assert!(still_frozen < SYNTHETIC_THAW && SYNTHETIC_THAW <= thaw);
    // 13 halvings of ln(100): the bracket is 100^(2^-13) wide.
    let width = (SEARCH.1 / SEARCH.0).ln() / f64::from(1u32 << SEARCH_STEPS);
    assert!(
        (thaw / still_frozen).ln() <= width * (1.0 + 1e-9),
        "{still_frozen} {thaw}"
    );
}

#[test]
fn a_thaw_search_stops_at_its_first_cycling_call() {
    // An icy start that cycles has no frozen balance: the snowball stops
    // holding there, so that CO₂ is its thaw, and the last CO₂ that stayed
    // frozen is the snowball's.
    let mut seen = Vec::new();
    let (still_frozen, thaw) = thaw_search(SEARCH.0, SEARCH.1, SEARCH_STEPS, |c| {
        let mut calls = 0;
        let icy = synthetic(c, true, &mut calls);
        seen.push((c, icy));
        icy
    });
    assert!(seen.len() < SEARCH_STEPS, "{seen:?}");
    // The search stops at its first cycling call: it is the last one.
    let cycling: Vec<_> = seen.iter().filter(|s| s.1 == IcyStart::Cycles).collect();
    assert_eq!(cycling.len(), 1, "{seen:?}");
    assert_eq!(seen.last(), Some(&(thaw, IcyStart::Cycles)), "{seen:?}");
    // The snowball's CO₂ is the most the search saw stay frozen.
    let frozen = seen
        .iter()
        .filter(|s| s.1 == IcyStart::StaysFrozen)
        .map(|s| s.0)
        .fold(f64::NEG_INFINITY, f64::max);
    assert_eq!(still_frozen, frozen, "{seen:?}");
}

/// The frozen branch of a synthetic body whose icy start is `at_froze` at
/// [`SEARCH`]'s low end and `at_most` at its high end, thawing in between
/// as [`synthetic`] with no cycle; and whether `most` was read.
fn ends(at_froze: IcyStart, at_most: IcyStart) -> (Option<(f64, Option<f64>)>, bool) {
    let mut read = false;
    let branch = frozen_branch(
        &mut read,
        SEARCH.0,
        |read| {
            *read = true;
            SEARCH.1
        },
        |_, co2| {
            if co2 == SEARCH.0 {
                at_froze
            } else if co2 == SEARCH.1 {
                at_most
            } else {
                synthetic(co2, false, &mut 0)
            }
        },
        SEARCH_STEPS,
    );
    (branch, read)
}

#[test]
fn an_icy_start_that_cycles_at_the_frozen_co2_leaves_no_frozen_branch() {
    for at_froze in [IcyStart::Cycles, IcyStart::Thaws] {
        let (branch, read) = ends(at_froze, IcyStart::Thaws);
        assert_eq!(branch, None, "{at_froze:?}");
        assert!(
            !read,
            "{at_froze:?}: the maximum greenhouse is read only past the frozen CO₂"
        );
    }
}

#[test]
fn an_icy_start_that_cycles_at_the_maximum_greenhouse_thaws_below_it() {
    for at_most in [IcyStart::Cycles, IcyStart::Thaws] {
        let (branch, _) = ends(IcyStart::StaysFrozen, at_most);
        let (still_frozen, thaw) = branch.expect("a frozen branch");
        let thaw = thaw.expect("a thaw");
        assert!(
            still_frozen < SYNTHETIC_THAW && SYNTHETIC_THAW <= thaw,
            "{at_most:?}"
        );
    }
    // Frozen at its maximum greenhouse too: frozen for good, at that CO₂.
    let (branch, _) = ends(IcyStart::StaysFrozen, IcyStart::StaysFrozen);
    assert_eq!(branch, Some((SEARCH.1, None)));
}

/// The frozen branch of a synthetic body whose temperate branch froze
/// with no CO₂, so that the thaw search starts from [`THAW_FLOOR`] of its
/// maximum greenhouse ([`SEARCH`]'s high end, where it thaws); its icy
/// start is `at_floor` there, as [`synthetic`] elsewhere. Also the CO₂ of
/// every call.
fn from_the_floor(at_floor: IcyStart) -> (Option<(f64, Option<f64>)>, Vec<f64>) {
    let floor = SEARCH.1 * THAW_FLOOR;
    let mut calls = Vec::new();
    let branch = frozen_branch(
        &mut calls,
        0.0,
        |_| SEARCH.1,
        |calls, co2| {
            calls.push(co2);
            if co2 == floor {
                at_floor
            } else {
                synthetic(co2, false, &mut 0)
            }
        },
        SEARCH_STEPS,
    );
    (branch, calls)
}

#[test]
fn a_thaw_search_from_its_floor_checks_that_the_floor_stays_frozen() {
    // The bisection holds only if its low end stays frozen: above the CO₂
    // the temperate branch froze with, the floor is read once (#758).
    let floor = SEARCH.1 * THAW_FLOOR;
    let (branch, calls) = from_the_floor(IcyStart::StaysFrozen);
    assert_eq!(
        calls.iter().filter(|&&c| c == floor).count(),
        1,
        "{calls:?}"
    );
    let (still_frozen, thaw) = branch.expect("a frozen branch");
    let thaw = thaw.expect("a thaw");
    assert!(still_frozen < SYNTHETIC_THAW && SYNTHETIC_THAW <= thaw);
    // Not frozen at the floor: the thaw lies between the frozen CO₂ and the
    // floor, with no bisection.
    for at_floor in [IcyStart::Thaws, IcyStart::Cycles] {
        let (branch, calls) = from_the_floor(at_floor);
        assert_eq!(branch, Some((0.0, Some(floor))), "{at_floor:?}");
        assert_eq!(calls, vec![0.0, SEARCH.1, floor], "{at_floor:?}");
    }
}

#[test]
fn a_thaw_search_from_its_frozen_co2_reads_no_floor() {
    // Above the floor, the low end is the frozen CO₂, already read.
    let mut calls = Vec::new();
    frozen_branch(
        &mut calls,
        SEARCH.0,
        |_| SEARCH.1,
        |calls, co2| {
            calls.push(co2);
            synthetic(co2, false, &mut 0)
        },
        SEARCH_STEPS,
    );
    assert_eq!(calls.len(), 2 + SEARCH_STEPS, "{calls:?}");
}
