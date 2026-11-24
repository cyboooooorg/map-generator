use super::ClimateInputs;
use super::branches::{Solved, solve};
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

/// Definition: the least Bond albedo of a snowball, 0.5: an ocean under
/// ice reflects more than it absorbs (the brief's bound, legacy's
/// `a_snowball_keeps_its_ocean_frozen`).
const SNOWBALL_LEAST_ALBEDO: f64 = 0.5;

/// Definition: the warmest mean surface of a snowball, 260 K, well under
/// the melting of ice (the brief's bound, legacy's
/// `a_snowball_keeps_its_ocean_frozen`).
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
fn far_planets_only_freeze() {
    only_freezes(FAR_ORBIT_AU);
}

#[test]
fn farther_planets_only_freeze_whatever_the_draw() {
    let cold = only_freezes(FARTHER_ORBIT_AU);
    let warm = solve(&moved(&earth(), FARTHER_ORBIT_AU), WARM_DRAW);
    assert_eq!(cold, warm);
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
