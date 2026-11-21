use super::*;
use crate::units::Metre;

/// Source: Jupiter's equatorial radius at 1 bar, 71 492 km (NASA Jupiter
/// Fact Sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/, read 2026-11-21;
/// study `giants-and-envelopes.md` §1.6).
const JUPITER_RADIUS_KM: f64 = 71_492.0;

/// Source: Saturn's equatorial radius at 1 bar, 60 268 km (NASA Saturn
/// Fact Sheet, read 2026-11-21; study §1.6).
const SATURN_RADIUS_KM: f64 = 60_268.0;

/// Source: Jupiter's mass in Earth masses, 318 (1898e24 kg over
/// 5.97e24 kg, NASA fact sheets; study §1.6).
const JUPITER_MASS: EarthMass = EarthMass(318.0);

/// Source: Saturn's mass in Earth masses, 95.1 (568e24 kg over 5.97e24 kg,
/// NASA fact sheets; study §1.6).
const SATURN_MASS: EarthMass = EarthMass(95.1);

/// Source: Jupiter and Saturn orbit near 9.5 au, the coldest distance of
/// Fortney et al. 2007 Tables 2 to 4 (study §1.6).
const OUTER_DISTANCE_AU: f64 = 9.5;

/// Source: age of the solar system as Fortney et al. 2007 Table 4 takes
/// it, 4.5 Gyr.
const TABLE_4_AGE: Gyr = Gyr(4.5);

/// Source: one Jupiter mass in the tables, 317.89 M⊕ (the `Me` column of
/// the `.dat` row with `Mj` 1.000; study §1.2).
const ONE_JUPITER_MASS: EarthMass = EarthMass(317.89);

/// Source: the hottest-but-one distance of Fortney et al. 2007 Table 4,
/// 0.045 au.
const HOT_DISTANCE_AU: f64 = 0.045;

/// Source: Fortney, Marley & Barnes 2007, ApJ 659, 1661, Table 4 ("Giant
/// Planet Radii at 4.5 Gyr"), row 0.045 AU, core 0, column 1.0 M_J
/// (318 M⊕): 1.107 R_J. Read on 2026-11-21 in the ar5iv rendering of
/// arXiv:astro-ph/0612671.
const TABLE_4_HOT_JUPITER_RJ: f64 = 1.107;

/// Source: Fortney et al. 2007 Table 1, first "50/50" (ice-rock) row:
/// 2.48, 3.23 and 4.03 R⊕ at 10, 31.6 and 100 M⊕ (read 2026-11-21 in the
/// ar5iv rendering of arXiv:astro-ph/0612671).
const TABLE_1_ICE_ROCK: [(f64, f64); 3] = [(10.0, 2.48), (31.6, 3.23), (100.0, 4.03)];

/// Source: Fortney et al. 2007, the text after Eq. 7: the fit reproduces
/// Table 1 to 2.5 % on average.
const EQ_7_ACCURACY: f64 = 0.025;

/// Source: the 3 % tolerance of the study's test proposal (§1.6: the model
/// must reproduce Jupiter "to within a few percent"); the table gives
/// 0.999 R_J for Jupiter with a 10 M⊕ core.
const JUPITER_TOLERANCE: f64 = 0.03;

/// Definition: the 1 % tolerance the brief asks of a printed cell, the
/// interpolation between the 3.16 and 10 Gyr blocks standing in for the
/// paper's 4.5 Gyr run.
const TABLE_TOLERANCE: f64 = 0.01;

/// Source: the tabulated heavy-element cores of Fortney et al. 2007,
/// 0, 10 and 25 M⊕.
const CORES: [EarthMass; 3] = [EarthMass(0.0), EarthMass(10.0), EarthMass(25.0)];

/// Definition: a mass every tabulated core of 25 M⊕ or more exceeds.
const LIGHT_GIANT: EarthMass = EarthMass(20.0);

/// Definition: a mass of the coreless 0.02 au file absent at 1 Myr (its
/// rows start at 46.42 M⊕) and present at 3.2 Myr (from 35.94 M⊕).
const YOUNG_MISSING_MASS: EarthMass = EarthMass(40.0);

/// Source: the first two ages of the `.dat` files, 1 Myr and 3.2 Myr
/// (`t (Gyr) = 0.0010`, `0.0032`; study §1.2).
const YOUNGEST_AGES: [Gyr; 2] = [Gyr(0.0010), Gyr(0.0032)];

/// Source: the closest distance of the tables, 0.02 au.
const CLOSEST_AU: f64 = 0.02;

/// Source: the largest mass of the tables, 3 593.81 M⊕ (the last `Me` of
/// every block but the coreless 0.02 au one; study §1.2).
const TOP_MASS: EarthMass = EarthMass(3593.81);

/// Definition: about 13 Jupiter masses (13 x 317.89 M⊕, rounded), the top
/// of the giants the system engine draws.
const HEAVIEST_GIANT: EarthMass = EarthMass(4130.0);

/// Definition: a mass below the first row (12.92 M⊕) of every cored
/// block and of the 0.02 au coreless block after 0.1 Gyr.
const BELOW_EVERY_CORE: EarthMass = EarthMass(11.0);

/// The flux, in Earth fluxes, of the Sun at `au`.
fn flux_at(au: f64) -> EarthFlux {
    EarthFlux(1.0 / (au * au))
}

fn km(radius: EarthRadius) -> f64 {
    Metre::from(radius).0 / 1e3
}

#[test]
fn jupiter_at_9_5_au_and_4_5_gyr() {
    let giant = giant(
        JUPITER_MASS,
        CORES[1],
        TABLE_4_AGE,
        flux_at(OUTER_DISTANCE_AU),
    )
    .unwrap();
    let error = km(giant.radius) / JUPITER_RADIUS_KM - 1.0;
    assert!(error.abs() < JUPITER_TOLERANCE, "Jupiter off by {error}");
    assert_eq!(giant.age_used, TABLE_4_AGE);
    assert!(giant.luminosity.0 > 0.0);
}

#[test]
fn saturn_needs_a_core_between_10_and_25() {
    let at = |core| {
        km(
            giant(SATURN_MASS, core, TABLE_4_AGE, flux_at(OUTER_DISTANCE_AU))
                .unwrap()
                .radius,
        )
    };
    assert!(at(CORES[1]) > SATURN_RADIUS_KM, "core 10: {}", at(CORES[1]));
    assert!(at(CORES[2]) < SATURN_RADIUS_KM, "core 25: {}", at(CORES[2]));
}

#[test]
fn a_hot_jupiter_matches_table_4() {
    let giant = giant(
        ONE_JUPITER_MASS,
        CORES[0],
        TABLE_4_AGE,
        flux_at(HOT_DISTANCE_AU),
    )
    .unwrap();
    let rj = Metre::from(giant.radius).0 / JUPITER_RADIUS.0;
    let error = rj / TABLE_4_HOT_JUPITER_RJ - 1.0;
    assert!(error.abs() < TABLE_TOLERANCE, "{rj} R_J against Table 4");
}

#[test]
fn the_ice_rock_core_matches_table_1() {
    for (mass, radius) in TABLE_1_ICE_ROCK {
        let fit = ice_rock_core_radius(EarthMass(mass)).0;
        let error = fit / radius - 1.0;
        assert!(error.abs() < EQ_7_ACCURACY, "{mass} M⊕: {fit} R⊕");
    }
}

#[test]
fn a_core_beyond_the_rows_is_refused() {
    let refused = giant(LIGHT_GIANT, CORES[2], TABLE_4_AGE, flux_at(1.0));
    assert!(matches!(refused, Err(GiantError::OutOfTable { .. })));
    assert_eq!(core_bounds(LIGHT_GIANT), Some((CORES[0], CORES[1])));
    assert_eq!(core_bounds(BELOW_EVERY_CORE), None);
}

#[test]
fn a_missing_young_cell_falls_back_to_an_older_age() {
    let giant = giant(
        YOUNG_MISSING_MASS,
        CORES[0],
        YOUNGEST_AGES[0],
        flux_at(CLOSEST_AU),
    )
    .unwrap();
    assert_eq!(giant.age_used, YOUNGEST_AGES[1]);
    assert!(giant.radius.0.is_finite() && giant.radius.0 > 0.0);
}

#[test]
fn every_value_is_finite_across_the_grid() {
    for mass in [GAS_GIANT_MIN_MASS.0, 50.0, 318.0, 3000.0, 4130.0] {
        let (low, high) = core_bounds(EarthMass(mass)).unwrap();
        for core in [low, high] {
            for age in [0.001, 0.05, 4.5, 20.0] {
                for au in [0.01, 0.03, 0.3, 5.0, 30.0] {
                    let g = giant(EarthMass(mass), core, Gyr(age), flux_at(au)).unwrap();
                    assert!(g.radius.0.is_finite() && g.radius.0 > 0.0);
                    assert!(g.luminosity.0.is_finite() && g.luminosity.0 > 0.0);
                }
            }
        }
    }
}

#[test]
fn a_mass_above_the_table_reads_the_top_row() {
    let core = CORES[1];
    let flux = flux_at(OUTER_DISTANCE_AU);
    let heavy = giant(HEAVIEST_GIANT, core, TABLE_4_AGE, flux).unwrap();
    let top = giant(TOP_MASS, core, TABLE_4_AGE, flux).unwrap();
    assert_eq!(heavy.mass_used, TOP_MASS);
    assert_eq!(heavy.radius, top.radius);
    assert_eq!(heavy.luminosity, top.luminosity);
    assert_eq!(top.mass_used, TOP_MASS);
    assert_eq!(core_bounds(HEAVIEST_GIANT), core_bounds(TOP_MASS));
    assert!(core_bounds(HEAVIEST_GIANT).is_some());
}
