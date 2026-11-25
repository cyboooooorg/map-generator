//! Tests of the shortwave budget, ported from `legacy-final`'s
//! `planet/albedo.rs` over a given ground albedo (the surface cover and
//! its albedos come with the global climate), with the checks of Earth's
//! clear and cloudy sky against the satellites and the cache's.

use super::*;
use crate::constants::{SOLAR_EFFECTIVE_TEMPERATURE, STANDARD_GRAVITY};
use crate::radiation::clouds::cloud_cover;
use crate::radiation::column::{Levels, Profile, STEAM_MAX_BAR, levels};
use crate::radiation::gases::{H2O_MOLAR_MASS, PartialPressures};
use crate::radiation::rayleigh::rayleigh_depth_550;
use crate::radiation::shortwave::{Air, Budget, Paths};
use crate::radiation::starlight::{SOLAR_METALLICITY, SPECTRA, star_bands};
use crate::radiation::test_points::{
    EARTH_AIR, EARTH_GROUND_ALBEDO, EARTH_INCOMING_W_M2, EARTH_SURFACE_K, TITAN_GRAVITY,
    TITAN_SURFACE_AIR, TITAN_SURFACE_K, VENUS_GRAVITY, VENUS_PRESSURE_BAR, VENUS_SURFACE_K,
};
use crate::surface::RELATIVE_HUMIDITY;
use crate::units::{Bar, Dex};
use crate::water::vapour_saturation_bar;

/// Source: the low end of the window of Earth's clear-sky albedo at
/// 77 % humidity, decided by the user on 2026-11-23 after the diagnostic
/// study of PR 2b-2: the observed albedo less what the sky holds and the
/// port does not, plus what the port lacks and the sky holds. The observed
/// 0.149 (Stephens et al. 2015, doi:10.1002/2014RG000449, §4) to 0.156
/// (CERES EBAF Ed4.0, Kato et al. 2018, doi:10.1175/JCLI-D-17-0208.1,
/// Table 5); less the aerosols' 0.017 +- 0.003 (5.9 +- 1 of 341.8 W/m²,
/// Kim & Ramanathan 2008, doi:10.1029/2007JD008434, Table 3b and §[52]);
/// plus what the port lacks or adds, each computed with its source: the
/// table's Sun (PHOENIX, study 35) against the measured TSIS-1 HSRS
/// spectrum (Coddington et al. 2021, doi:10.1029/2020GL091709) +0.0023
/// (it was the black body's +0.0147 before issue #655); no ozone,
/// 300 +- 30 DU with the cross-sections of Serdyuchenko et al. 2014
/// (doi:10.5194/amt-7-625-2014) +0.0145 +- 0.0005; no O₂ (Hogan &
/// Matricardi 2020, doi:10.5194/gmd-13-6501-2020, Table 4) +0.0018; and
/// the two-stream closure against exact radiative transfer (checked
/// against Kokhanovsky, Mayer & Rozanov 2005, doi:10.1016/j.atmosres.2004.07.004)
/// −0.0028. The terms sum to 0.1443 at the low end and 0.1583 at the
/// high end. Issue #653 (ozone, O₂) narrows it when it lands. The port's
/// 0.1577 lies near its top edge.
const CLEAR_SKY_ALBEDO_LOW: f64 = 0.1443;

/// Source: the high end of the window of [`CLEAR_SKY_ALBEDO_LOW`], the
/// sum of the same cited terms.
const CLEAR_SKY_ALBEDO_HIGH: f64 = 0.1583;

/// Source: Earth's clear-sky albedo at 77 % humidity over its ground
/// under the measured Sun, TSIS-1 HSRS (Coddington et al. 2021,
/// doi:10.1029/2020GL091709) binned on the bands by study 35 (Wiki
/// `35-stellar-spectra-audit`, section 1), continued past 2730 nm by a
/// 5772 K black body scaled to the data (0.1555 when continued by the
/// PHOENIX Sun): the same shortwave with the measured spectrum.
const TSIS_CLEAR_SKY_ALBEDO: f64 = 0.1554;

/// Calibration: how far Earth's clear-sky albedo under the table's Sun may
/// lie from [`TSIS_CLEAR_SKY_ALBEDO`], 0.003: the PHOENIX Sun's 0.0023,
/// measured by study 35, rounded up; decided by the user on 2026-11-25.
/// No source gives a model library's error against the measured Sun on a
/// planet's albedo.
const TSIS_CLEAR_SKY_TOLERANCE: f64 = 0.003;

/// Definition: the coolest and hottest stars of Kopparapu et al. 2013's
/// comparison, K (§4.1: 2600 to 7200 K), between which the albedo falls
/// from F to M hosts.
const KOPPARAPU_K: (f64, f64) = (2600.0, 7200.0);

/// Calibration: how far Earth's clear-sky albedo under a row of the table
/// of spectra may lie from its value under that row rebuilt from its two
/// neighbours, 0.001: a tenth of what the black body moved Earth's
/// cloudy albedo by (0.0104, study 35). Study 35 measured 0.0036 at most
/// over gaps of 600 K (at 2600 K), 0.0001 at 3300 K; the table's steps
/// are 100 to 1000 K.
const INTERPOLATED_ALBEDO: f64 = 0.001;

/// Definition: how much more a row rebuilt over two steps of the table
/// errs than the table between two rows, 4: a linear interpolation errs
/// as the square of its step, and the rebuilt row spans two.
const TWO_STEPS: f64 = 4.0;

/// Source: the precipitable water of Kim & Ramanathan 2008's global run,
/// cm: the NVAP water field (§[22]), whose global mean is 2.45 (heritage
/// NVAP) to 2.53 (NVAP-M, 1988-2009) cm (Forsythe, Vonder Haar & Cronk
/// 2014, NOAA ESRL Global Monitoring Annual Conference, pp. 6 and 18),
/// rounded. Their 3.0 cm of Table 2 is the sensitivity case's input.
const NVAP_WATER_CM: f64 = 2.5;

/// Source: the columns of Kim & Ramanathan 2008's sensitivity to the
/// precipitable water, 3.0 +- 0.3 cm (doi:10.1029/2007JD008434, Table 2).
const SENSITIVITY_WATER_CM: [f64; 2] = [2.7, 3.3];

/// Source: what 0.6 cm more water adds to the clear sky's absorption,
/// W/m²: Kim & Ramanathan 2008, Table 2, +-0.3 cm moves the surface's net
/// flux by 1.3 and the top's by 0.2, so the air's absorption by 1.5 each
/// way.
const MORE_WATER_ABSORBED_W_M2: f64 = 3.0;

/// Calibration: the tolerance of [`MORE_WATER_ABSORBED_W_M2`], W/m², approved
/// by the user on 2026-11-23: the table's rounding to a tenth and the
/// unstated sign of its two terms.
const MORE_WATER_ABSORBED_TOLERANCE_W_M2: f64 = 1.0;

/// Source: what 0.6 cm more water takes from the light reflected at the
/// top, W/m²: Kim & Ramanathan 2008, Table 2, 0.2 for +-0.3 cm.
const MORE_WATER_REFLECTED_W_M2: f64 = 0.4;

/// Calibration: the tolerance of [`MORE_WATER_REFLECTED_W_M2`], W/m², approved
/// by the user on 2026-11-23: the table's rounding to a tenth on each
/// side.
const MORE_WATER_REFLECTED_TOLERANCE_W_M2: f64 = 0.2;

/// Source: the low end of the window of the clear-sky absorption by
/// water vapour and CO₂, W/m² at 340.25, approved by the user on
/// 2026-11-23: 51 +- 3, water about 50 of 341.8 (Kim & Ramanathan 2008,
/// §[51]) plus CO₂ 1.0 to 1.6 (Kiehl & Trenberth 1997,
/// doi:10.1175/1520-0477(1997)078<0197:EAGMEB>2.0.CO;2, Table 4; Hogan &
/// Matricardi 2020, doi:10.5194/gmd-13-6501-2020, Table 4), with the +-3
/// of Kim & Ramanathan's total. See `docs/science/radiative-transfer.md`,
/// "Window of the H₂O and CO₂ absorption".
const CLEAR_SKY_ABSORBED_LOW_W_M2: f64 = 48.0;

/// Source: the high end of the window of [`CLEAR_SKY_ABSORBED_LOW_W_M2`].
const CLEAR_SKY_ABSORBED_HIGH_W_M2: f64 = 54.0;

/// Source: Earth's Bond albedo, 0.293 (CERES, Stephens et al. 2015,
/// doi:10.1002/2014RG000449; Kato et al. 2018 give 0.2915).
const EARTH_BOND_ALBEDO: f64 = 0.293;

/// Calibration: how far Earth's Bond albedo may stray from
/// [`EARTH_BOND_ALBEDO`], half the gap between Stephens et al.'s 0.293 and
/// Kato et al.'s 0.2915: the cloud albedo is fitted on it to three digits.
const EARTH_BOND_TOLERANCE: f64 = 0.0015;

/// Definition: the M dwarf of Kopparapu et al. 2013's comparison, K (their
/// coolest star, 2600 K, is redder still; 3300 K is the legacy test's).
const M_DWARF_K: f64 = 3300.0;

/// Definition: the surface of the thick CO₂ test air, K (the legacy's).
const THICK_CO2_SURFACE_K: f64 = 320.0;

/// Definition: the CO₂ of the thick-air test, bar: a trace, 8 bar and
/// Venus's order (the legacy's).
const THICK_CO2_BAR: [f64; 3] = [1e-3, 8.0, 90.0];

/// Definition: the surface of the legacy's cold thick air of the M-star
/// test, K (1 bar of N₂ and 1 of CO₂ at the freezing point).
const COLD_SURFACE_K: f64 = 273.0;

/// Calibration: how much brighter Titan's air is than Earth's over the
/// same ground, the legacy test's 0.05 (a margin of the comparison, not a
/// measure).
const TITAN_BRIGHTER_BY: f64 = 0.05;

/// Definition: a pressure just below zero, bar, as a caller's rounding
/// leaves.
const NEGATIVE_TRACE_BAR: f64 = -1e-12;

/// Definition: the tolerance of the exact checks.
const EXACT: f64 = 1e-12;

/// Definition: the grounds of the bound checks and the no-air check.
const GROUNDS: [f64; 4] = [0.0, 0.122, 0.6, 1.0];

/// Definition: more airs than a [`ShortwaveCache`] remembers.
const MANY_AIRS: usize = 40;

/// Definition: the finer column of the convergence test: levels per decade
/// and top share of the surface pressure (the legacy's).
const FINE_LEVELS_PER_DECADE: f64 = 16.0;

/// Definition: the top of the finer column, as a share of the surface
/// pressure (the legacy's).
const FINE_TOP_SHARE: f64 = 1e-3;

/// Calibration: the shortwave grid's tolerance on Earth's Bond albedo, the
/// legacy's (see `SHORTWAVE_LEVELS_PER_DECADE`).
const CONVERGED_ALBEDO: f64 = 1e-3;

/// Calibration: the shortwave grid's tolerance on Earth's absorption,
/// W/m², the legacy's.
const CONVERGED_W_M2: f64 = 1.0;

/// Definition: kilograms of water per m² in a centimetre of precipitable
/// water (1 kg/m² is 1 mm).
const KG_M2_PER_CM: f64 = 10.0;

/// Definition: how close the bisection must bring the column's water to
/// its target, cm (a thirtieth of Kim & Ramanathan's +-0.3).
const WATER_TOLERANCE_CM: f64 = 0.01;

/// Definition: steps of the bisections, which halve the interval each time.
const BISECTIONS: usize = 60;

fn share(x: f64) -> Fraction {
    Fraction::new(x).expect("a test share")
}

fn under(ground: f64, clouds: f64, deck: f64) -> Underneath {
    Underneath {
        ground: share(ground),
        clouds: share(clouds),
        deck: share(deck),
    }
}

/// Earth's air at [`EARTH_SURFACE_K`] at a surface relative humidity
/// `humidity` of the IAPWS saturation.
fn earth_air_at(humidity: f64) -> PartialPressures {
    PartialPressures {
        h2o: humidity * vapour_saturation_bar(EARTH_SURFACE_K),
        ..EARTH_AIR
    }
}

/// Earth's air at [`RELATIVE_HUMIDITY`], the path the climate takes.
fn earth_air() -> PartialPressures {
    earth_air_at(RELATIVE_HUMIDITY)
}

/// Earth's cloud cover from the law the climate calls.
fn earth_clouds() -> Fraction {
    cloud_cover(Kelvin(EARTH_SURFACE_K), Bar(earth_air().h2o))
}

/// The budget of `gases` over a surface at `surface_k` on an Earth-gravity
/// body under the Sun, with a fresh cache.
fn earth_budget(gases: &PartialPressures, surface_k: f64, below: Underneath) -> ShortwaveBudget {
    shortwave_budget(
        gases,
        Kelvin(surface_k),
        STANDARD_GRAVITY,
        SOLAR_EFFECTIVE_TEMPERATURE,
        Dex(SOLAR_METALLICITY),
        below,
        &mut ShortwaveCache::default(),
    )
}

/// Earth's clear sky over its ground.
fn clear() -> Underneath {
    under(EARTH_GROUND_ALBEDO, 0.0, 0.0)
}

/// Precipitable water of a column, cm: the water's mass per area, each
/// span's `ln p` times the logarithmic mean of its levels' water mass per
/// `ln p` (the mean the shortwave's pieces take).
fn precipitable_cm(column: &Levels) -> f64 {
    let mass = |i: usize| {
        let level = &column.levels[i];
        level.mass_per_ln_p(level.gases.h2o, H2O_MOLAR_MASS)
    };
    let kg_m2: f64 = column
        .spans
        .iter()
        .enumerate()
        .map(|(i, span)| {
            let (a, b) = (mass(i), mass(i + 1));
            let mean = if a <= 0.0 || b <= 0.0 || (a - b).abs() <= EXACT * a {
                0.5 * (a + b)
            } else {
                (b - a) / (b / a).ln()
            };
            span * mean
        })
        .sum();
    kg_m2 / KG_M2_PER_CM
}

/// Earth's shortwave column at [`EARTH_SURFACE_K`] holding `water_cm` of
/// precipitable water at one relative humidity throughout: the saturated
/// column (3.165 cm) with every level's vapour times one factor, found by bisection
/// (the diagnostic study's column, which leaves the humidity profile to
/// issue #654 and tests the optics at equal water). Returns the surface
/// air and the column.
fn earth_column_holding(water_cm: f64) -> (PartialPressures, Levels) {
    let saturated = earth_air_at(1.0);
    let scaled = |factor: f64| {
        let mut column = levels(
            &saturated,
            EARTH_SURFACE_K,
            STANDARD_GRAVITY.0,
            Profile::SHORTWAVE,
        )
        .expect("Earth has air");
        for level in &mut column.levels {
            level.gases.h2o *= factor;
        }
        column
    };
    // Above 3.165 cm (Kim & Ramanathan's 3.3) the factor passes 1: a
    // column a few per cent over saturation, as the study took it.
    let (mut low, mut high) = (0.0, 2.0);
    assert!(precipitable_cm(&scaled(high)) > water_cm);
    for _ in 0..BISECTIONS {
        let mid = 0.5 * (low + high);
        if precipitable_cm(&scaled(mid)) < water_cm {
            low = mid;
        } else {
            high = mid;
        }
    }
    let factor = 0.5 * (low + high);
    let column = scaled(factor);
    let held = precipitable_cm(&column);
    assert!(
        (held - water_cm).abs() < WATER_TOLERANCE_CM,
        "{held} cm for {water_cm}"
    );
    let air = PartialPressures {
        h2o: saturated.h2o * factor,
        ..saturated
    };
    (air, column)
}

/// The clear sky of Earth over its ground under the Sun, through the
/// column of [`earth_column_holding`] `water_cm`.
fn earth_clear_sky_holding(water_cm: f64) -> Budget {
    let (air, column) = earth_column_holding(water_cm);
    let through = Air {
        haze: (0.0, 1.0),
        rayleigh_550: rayleigh_depth_550(&air, STANDARD_GRAVITY.0),
        column: Some(&column),
    };
    Paths::through(
        &through,
        &star_bands(SOLAR_EFFECTIVE_TEMPERATURE.0, SOLAR_METALLICITY).bands,
    )
    .budget(EARTH_GROUND_ALBEDO, 0.0)
}

#[test]
fn earths_clear_sky_albedo_is_observed_less_what_is_missing() {
    let budget = earth_budget(&earth_air(), EARTH_SURFACE_K, clear());
    let a = budget.bond_albedo.get();
    println!(
        "Earth clear sky at 77 %: albedo {a:.4}, absorbed {:.2} W/m²",
        budget.absorbed_by_air.get() * EARTH_INCOMING_W_M2
    );
    assert!(
        (CLEAR_SKY_ALBEDO_LOW..=CLEAR_SKY_ALBEDO_HIGH).contains(&a),
        "{a}"
    );
}

#[test]
fn earths_clear_sky_air_absorbs_what_satellites_see() {
    let budget = earth_clear_sky_holding(NVAP_WATER_CM);
    let absorbed = budget.air * EARTH_INCOMING_W_M2;
    let manabe = earth_budget(&earth_air(), EARTH_SURFACE_K, clear());
    println!(
        "Earth clear sky at {NVAP_WATER_CM} cm: absorbed {absorbed:.2} W/m², albedo {:.4}; \
         at 77 % {:.2} W/m²",
        budget.albedo,
        manabe.absorbed_by_air.get() * EARTH_INCOMING_W_M2,
    );
    assert!(
        (CLEAR_SKY_ABSORBED_LOW_W_M2..=CLEAR_SKY_ABSORBED_HIGH_W_M2).contains(&absorbed),
        "{absorbed} W/m²"
    );
    // Without vapour or CO₂ the air absorbs nothing, and reflects more.
    let dry = PartialPressures {
        h2o: 0.0,
        co2: 0.0,
        ..earth_air()
    };
    let none = earth_budget(&dry, EARTH_SURFACE_K, clear());
    assert!(none.absorbed_by_air.get() < 1e-6, "{none:?}");
    assert!(
        none.bond_albedo.get() > budget.albedo,
        "{none:?} {budget:?}"
    );
}

#[test]
fn more_water_absorbs_as_kim_and_ramanathan_say() {
    let [less, more] = SENSITIVITY_WATER_CM.map(earth_clear_sky_holding);
    let absorbed = (more.air - less.air) * EARTH_INCOMING_W_M2;
    let reflected = (less.albedo - more.albedo) * EARTH_INCOMING_W_M2;
    println!(
        "from {} to {} cm: absorbed +{absorbed:.3} W/m², reflected −{reflected:.3} W/m²",
        SENSITIVITY_WATER_CM[0], SENSITIVITY_WATER_CM[1]
    );
    assert!(
        (absorbed - MORE_WATER_ABSORBED_W_M2).abs() <= MORE_WATER_ABSORBED_TOLERANCE_W_M2,
        "{absorbed} W/m²"
    );
    assert!(
        (reflected - MORE_WATER_REFLECTED_W_M2).abs() <= MORE_WATER_REFLECTED_TOLERANCE_W_M2,
        "{reflected} W/m²"
    );
}

/// Earth's all-sky Bond albedo stays at CERES's 0.293. `CLOUD_ALBEDO` was
/// fitted on 0.293, so this checks that calibration and is a net if the
/// physics moves, not an independent check; the clear-sky window and the
/// water tests are the independent ones.
#[test]
fn earth_keeps_its_bond_albedo() {
    let clouds = earth_clouds();
    let a = earth_budget(
        &earth_air(),
        EARTH_SURFACE_K,
        Underneath { clouds, ..clear() },
    )
    .bond_albedo
    .get();
    println!(
        "Earth's Bond albedo at 288 K: {a:.4} under {:.4} of cloud",
        clouds.get()
    );
    assert!((a - EARTH_BOND_ALBEDO).abs() <= EARTH_BOND_TOLERANCE, "{a}");
}

/// An M dwarf's light is redder: more of it falls in the near infrared,
/// where CO₂ and water absorb, and less is scattered, so that the same air
/// lowers the albedo of a planet round an M dwarf more than round a G
/// star (Kopparapu et al. 2013, §4.1).
#[test]
fn an_m_stars_albedo_drops_more_than_a_g_stars() {
    let thick = PartialPressures {
        n2: 1.0,
        co2: 1.0,
        h2o: RELATIVE_HUMIDITY * vapour_saturation_bar(COLD_SURFACE_K),
        ..PartialPressures::default()
    };
    for (air, t) in [(earth_air(), EARTH_SURFACE_K), (thick, COLD_SURFACE_K)] {
        let clouds = cloud_cover(Kelvin(t), Bar(air.h2o)).get();
        let column = levels(&air, t, STANDARD_GRAVITY.0, Profile::SHORTWAVE);
        let drop = |star_k: f64| {
            let light = star_bands(star_k, SOLAR_METALLICITY);
            let budget = |column: Option<&Levels>| -> Budget {
                let through = Air {
                    haze: (0.0, 1.0),
                    rayleigh_550: rayleigh_depth_550(&air, STANDARD_GRAVITY.0),
                    column,
                };
                Paths::through(&through, &light.bands)
                    .budget(EARTH_GROUND_ALBEDO, clouds * CLOUD_ALBEDO)
            };
            let scattered = budget(None);
            let absorbed = budget(column.as_ref());
            (scattered.albedo - absorbed.albedo, absorbed)
        };
        let (g, g_budget) = drop(SOLAR_EFFECTIVE_TEMPERATURE.0);
        let (m, m_budget) = drop(M_DWARF_K);
        println!("{air:?}: drop G {g:.4} ({g_budget:?}), M {m:.4} ({m_budget:?})");
        assert!(m > g && g > 0.0, "{m} vs {g}");
        assert!(m_budget.air > g_budget.air);
    }
}

/// Rayleigh scattering grows with the column, and saturates.
#[test]
fn thick_co2_scatters_more_light() {
    let [thin, eight, ninety] = THICK_CO2_BAR.map(|co2| {
        let air = PartialPressures {
            n2: 1.0,
            co2,
            ..PartialPressures::default()
        };
        earth_budget(
            &air,
            THICK_CO2_SURFACE_K,
            under(EARTH_GROUND_ALBEDO, 0.0, 0.0),
        )
        .bond_albedo
        .get()
    });
    println!("albedo under 1e-3, 8, 90 bar of CO2: {thin:.3} {eight:.3} {ninety:.3}");
    assert!(thin < eight && eight < ninety, "{thin} {eight} {ninety}");
    assert!(ninety < 1.0);
}

/// A low-gravity moon with a thick air and a methane haze (Titan) is
/// brighter over the same ground than Earth's clear sky: its column is
/// heavier and its haze reflects.
#[test]
fn titans_air_is_brighter_than_earths() {
    let mut cache = ShortwaveCache::default();
    let titan = shortwave_budget(
        &TITAN_SURFACE_AIR,
        Kelvin(TITAN_SURFACE_K),
        TITAN_GRAVITY,
        SOLAR_EFFECTIVE_TEMPERATURE,
        Dex(SOLAR_METALLICITY),
        clear(),
        &mut cache,
    )
    .bond_albedo
    .get();
    let earth = earth_budget(&earth_air(), EARTH_SURFACE_K, clear())
        .bond_albedo
        .get();
    println!("Titan {titan:.4}, Earth {earth:.4} over the same ground");
    assert!(titan > earth + TITAN_BRIGHTER_BY, "{titan} {earth}");
}

/// Review Focus 2: with no air, the ground's albedo comes back, and a
/// cloud layer over it adds as a non-absorbing layer does.
#[test]
fn no_air_returns_the_ground() {
    for p in [0.0, NEGATIVE_TRACE_BAR] {
        let none = PartialPressures {
            n2: p,
            co2: p,
            h2o: p,
            ch4: p,
            nh3: p,
            so2: p,
            o2: p,
        };
        for g in GROUNDS {
            let bare = earth_budget(&none, EARTH_SURFACE_K, under(g, 0.0, 0.0));
            assert!(
                (bare.bond_albedo.get() - g).abs() < EXACT,
                "{p} {g}: {bare:?}"
            );
            assert!(bare.absorbed_by_air.get() < EXACT, "{p} {g}: {bare:?}");
            let cloudy = earth_budget(&none, EARTH_SURFACE_K, under(g, 0.5, 0.0));
            let c = 0.5 * CLOUD_ALBEDO;
            let adding = c + (1.0 - c) * (1.0 - c) * g / (1.0 - c * g);
            assert!(
                (cloudy.bond_albedo.get() - adding).abs() < EXACT,
                "{p} {g}: {cloudy:?} vs {adding}"
            );
            assert!(cloudy.absorbed_by_air.get() < EXACT, "{p} {g}: {cloudy:?}");
        }
    }
}

/// Review Focus 4: a whole deck reflects its albedo and nothing reaches
/// the air or the ground below it.
#[test]
fn a_full_deck_hides_everything() {
    for g in GROUNDS {
        for clouds in [0.0, 1.0] {
            let b = earth_budget(&earth_air(), EARTH_SURFACE_K, under(g, clouds, 1.0));
            assert_eq!(b.bond_albedo.get(), DECK_ALBEDO, "{g} {clouds}");
            assert_eq!(b.absorbed_by_air.get(), 0.0, "{g} {clouds}");
        }
    }
}

/// Review Focus 5: whatever the ground, the clouds, the deck and the air,
/// the light is shared between space, the air and the ground.
#[test]
fn the_budget_stays_in_bounds() {
    let venus = PartialPressures {
        co2: VENUS_PRESSURE_BAR,
        h2o: STEAM_MAX_BAR,
        ..PartialPressures::default()
    };
    let mut cache = ShortwaveCache::default();
    let airs = [
        (earth_air(), EARTH_SURFACE_K, STANDARD_GRAVITY),
        (venus, VENUS_SURFACE_K, VENUS_GRAVITY),
    ];
    for (air, t, gravity) in airs {
        for g in [0.0, 1.0] {
            for clouds in [0.0, 1.0] {
                for deck in [0.0, 0.5, 1.0] {
                    let b = shortwave_budget(
                        &air,
                        Kelvin(t),
                        gravity,
                        SOLAR_EFFECTIVE_TEMPERATURE,
                        Dex(SOLAR_METALLICITY),
                        under(g, clouds, deck),
                        &mut cache,
                    );
                    let (a, absorbed) = (b.bond_albedo.get(), b.absorbed_by_air.get());
                    assert!(a >= 0.0, "{t} {g} {clouds} {deck}: {b:?}");
                    assert!(
                        (0.0..=1.0 - a).contains(&absorbed),
                        "{t} {g} {clouds} {deck}: {b:?}"
                    );
                }
            }
        }
    }
}

/// Review Focus 3: a cached budget is bit for bit the fresh one, and an
/// air the cache has forgotten is solved again to the same bits.
#[test]
fn a_cached_budget_is_the_fresh_one() {
    let below = Underneath {
        clouds: earth_clouds(),
        ..clear()
    };
    let budget = |air: &PartialPressures, t: f64, cache: &mut ShortwaveCache| {
        shortwave_budget(
            air,
            Kelvin(t),
            STANDARD_GRAVITY,
            SOLAR_EFFECTIVE_TEMPERATURE,
            Dex(SOLAR_METALLICITY),
            below,
            cache,
        )
    };
    let bits = |b: ShortwaveBudget| {
        (
            b.bond_albedo.get().to_bits(),
            b.absorbed_by_air.get().to_bits(),
        )
    };
    let air = earth_air();
    let mut cache = ShortwaveCache::default();
    let first = bits(budget(&air, EARTH_SURFACE_K, &mut cache));
    assert_eq!(bits(budget(&air, EARTH_SURFACE_K, &mut cache)), first);
    assert_eq!(
        bits(budget(
            &air,
            EARTH_SURFACE_K,
            &mut ShortwaveCache::default()
        )),
        first
    );
    for i in 0..MANY_AIRS {
        let t = EARTH_SURFACE_K + 1.0 + i as f64;
        budget(&air, t, &mut cache);
    }
    assert!(format!("{cache:?}").contains("remembered: 32"), "{cache:?}");
    assert_eq!(bits(budget(&air, EARTH_SURFACE_K, &mut cache)), first);
}

/// The column's coarse grid gives the near-infrared absorption of a fine
/// one (the legacy test that holds `SHORTWAVE_LEVELS_PER_DECADE`).
#[test]
fn the_shortwave_column_is_converged() {
    let air = earth_air();
    let clouds = earth_clouds().get();
    let light = star_bands(SOLAR_EFFECTIVE_TEMPERATURE.0, SOLAR_METALLICITY);
    let budget = |profile: Profile| {
        let column = levels(&air, EARTH_SURFACE_K, STANDARD_GRAVITY.0, profile);
        let through = Air {
            haze: (0.0, 1.0),
            rayleigh_550: rayleigh_depth_550(&air, STANDARD_GRAVITY.0),
            column: column.as_ref(),
        };
        Paths::through(&through, &light.bands).budget(EARTH_GROUND_ALBEDO, clouds * CLOUD_ALBEDO)
    };
    let coarse = budget(Profile::SHORTWAVE);
    let fine = budget(
        Profile::SHORTWAVE
            .on(FINE_LEVELS_PER_DECADE)
            .up_to(FINE_TOP_SHARE),
    );
    println!("coarse {coarse:?}, fine {fine:?}");
    assert!((coarse.albedo - fine.albedo).abs() < CONVERGED_ALBEDO);
    assert!((coarse.air - fine.air).abs() * EARTH_INCOMING_W_M2 < CONVERGED_W_M2);
}

/// Review Focus 1: a star of 0 K or of no temperature lights the body as
/// the Sun does.
#[test]
fn an_unknown_star_is_the_sun() {
    let below = Underneath {
        clouds: earth_clouds(),
        ..clear()
    };
    let budget = |star: f64| {
        shortwave_budget(
            &earth_air(),
            Kelvin(EARTH_SURFACE_K),
            STANDARD_GRAVITY,
            Kelvin(star),
            Dex(SOLAR_METALLICITY),
            below,
            &mut ShortwaveCache::default(),
        )
    };
    let sun = budget(SOLAR_EFFECTIVE_TEMPERATURE.0);
    for star in [0.0, f64::NAN] {
        assert_eq!(budget(star), sun, "{star}");
    }
}

/// A gravity that is not finite or not positive is no air: the budget is
/// that of a body without air over the same ground, clouds and deck, never
/// the NaN that turned into an albedo of 0 and an absorption of 1.
#[test]
fn a_weightless_air_is_no_air() {
    let below = under(0.3, 0.4, 0.2);
    let airless = earth_budget(&PartialPressures::default(), EARTH_SURFACE_K, below);
    for g in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let b = shortwave_budget(
            &earth_air(),
            Kelvin(EARTH_SURFACE_K),
            MetrePerSecondSquared(g),
            SOLAR_EFFECTIVE_TEMPERATURE,
            Dex(SOLAR_METALLICITY),
            below,
            &mut ShortwaveCache::default(),
        );
        assert!(
            (b.bond_albedo.get() - airless.bond_albedo.get()).abs() < 1e-12
                && (b.absorbed_by_air.get() - airless.absorbed_by_air.get()).abs() < 1e-12,
            "gravity {g}: {b:?} against {airless:?}"
        );
    }
}

/// A star whose temperature is infinite is the Sun, as a NaN or a
/// temperature that is not positive is.
#[test]
fn an_infinite_star_is_the_sun() {
    let sun = earth_budget(&earth_air(), EARTH_SURFACE_K, clear());
    let infinite = shortwave_budget(
        &earth_air(),
        Kelvin(EARTH_SURFACE_K),
        STANDARD_GRAVITY,
        Kelvin(f64::INFINITY),
        Dex(SOLAR_METALLICITY),
        clear(),
        &mut ShortwaveCache::default(),
    );
    assert_eq!(sun, infinite);
    assert_eq!(
        star_bands(f64::INFINITY, SOLAR_METALLICITY),
        star_bands(SOLAR_EFFECTIVE_TEMPERATURE.0, SOLAR_METALLICITY)
    );
}

/// Earth's clear sky over its ground at 77 % humidity under `light`.
fn earth_clear_sky_under(light: &[(f64, f64)]) -> Budget {
    let air = earth_air();
    let column = levels(
        &air,
        EARTH_SURFACE_K,
        STANDARD_GRAVITY.0,
        Profile::SHORTWAVE,
    );
    let through = Air {
        haze: (0.0, 1.0),
        rayleigh_550: rayleigh_depth_550(&air, STANDARD_GRAVITY.0),
        column: column.as_ref(),
    };
    Paths::through(&through, light).budget(EARTH_GROUND_ALBEDO, 0.0)
}

#[test]
fn the_tables_sun_lights_earth_as_the_measured_sun() {
    let light = star_bands(SOLAR_EFFECTIVE_TEMPERATURE.0, SOLAR_METALLICITY);
    let a = earth_clear_sky_under(&light.bands).albedo;
    println!("Earth's clear sky under the table's Sun: {a:.4} (TSIS-1 {TSIS_CLEAR_SKY_ALBEDO})");
    assert!(
        (a - TSIS_CLEAR_SKY_ALBEDO).abs() <= TSIS_CLEAR_SKY_TOLERANCE,
        "{a}"
    );
}

#[test]
fn the_albedo_falls_from_f_to_m_hosts() {
    // Kopparapu et al. 2013, §4.1: at equal flux, the albedo of an
    // Earth-like planet is higher round an F star and lower round an M
    // star. Every row of the table between their coolest and hottest stars.
    let s = &*SPECTRA;
    let solar = s
        .metallicities
        .iter()
        .position(|m| *m == SOLAR_METALLICITY)
        .expect("a solar row");
    let albedos: Vec<(f64, f64)> = s
        .temperatures
        .iter()
        .filter(|t| (KOPPARAPU_K.0..=KOPPARAPU_K.1).contains(*t))
        .map(|t| {
            (
                *t,
                earth_clear_sky_under(&star_bands(*t, s.metallicities[solar]).bands).albedo,
            )
        })
        .collect();
    for pair in albedos.windows(2) {
        assert!(pair[0].1 < pair[1].1, "{pair:?}");
    }
}

#[test]
fn a_row_rebuilt_from_its_neighbours_lights_earth_alike() {
    // Every interior row at the Sun's metallicity, interpolated linearly
    // from the rows on either side: over two steps, it errs [`TWO_STEPS`]
    // times what the table errs between two rows.
    let s = &*SPECTRA;
    let solar = s
        .metallicities
        .iter()
        .position(|m| *m == SOLAR_METALLICITY)
        .expect("a solar row");
    let rows = &s.shares[solar];
    let t = &s.temperatures;
    let bands = |shares: &[f64]| -> Vec<(f64, f64)> {
        let total: f64 = shares.iter().sum();
        star_bands(SOLAR_EFFECTIVE_TEMPERATURE.0, SOLAR_METALLICITY)
            .bands
            .iter()
            .zip(shares)
            .map(|(b, x)| (b.0, x / total))
            .collect()
    };
    let mut worst = (0.0, 0.0);
    for i in 1..t.len() - 1 {
        let f = (t[i] - t[i - 1]) / (t[i + 1] - t[i - 1]);
        let rebuilt: Vec<f64> = rows[i - 1]
            .iter()
            .zip(&rows[i + 1])
            .map(|(a, b)| (1.0 - f) * a + f * b)
            .collect();
        let gap = (earth_clear_sky_under(&bands(&rows[i])).albedo
            - earth_clear_sky_under(&bands(&rebuilt)).albedo)
            .abs();
        if gap > worst.1 {
            worst = (t[i], gap);
        }
    }
    println!("worst rebuilt row: {} K, {:.5}", worst.0, worst.1);
    assert!(worst.1 <= TWO_STEPS * INTERPOLATED_ALBEDO, "{worst:?}");
}
