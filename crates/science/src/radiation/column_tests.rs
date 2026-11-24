//! Tests of the longwave column, ported from `legacy-final`'s
//! `planet/column.rs` with their tolerances, with the helpers they read
//! on the moist plateau of `crate::balance` (legacy `planet/climate.rs`'s
//! `plateau_w_m2` and `plateau_skin_k`), and the cache and edge-case tests
//! of the port.

use super::*;
use crate::balance::{self, PLATEAU_SURFACES, SIMPSON_NAKAJIMA_W_M2, earth_moist_plateau};
use crate::constants::{MARS_SURFACE_GRAVITY, STANDARD_GRAVITY};
use crate::radiation::test_points::{
    EARTH_AIR, TITAN_EFFECTIVE_K, TITAN_GRAVITY, TITAN_SURFACE_AIR, TITAN_SURFACE_K, VENUS_GRAVITY,
};
use crate::surface::surface_vapour;
use crate::units::Bar;

/// Definition: Earth's surface gravity of the test columns, the standard
/// acceleration of gravity (the legacy took 9.81 m/s²).
const EARTH: MetrePerSecondSquared = STANDARD_GRAVITY;

/// Definition: Mars's surface gravity of the test columns, the sourced
/// `MARS_SURFACE_GRAVITY`, 3.73 m/s² (the legacy took 3.71).
const MARS: MetrePerSecondSquared = MARS_SURFACE_GRAVITY;

/// Definition: Venus's gravity of the test columns, the shared sourced
/// value.
const VENUS: MetrePerSecondSquared = VENUS_GRAVITY;

/// Definition: Titan's gravity of the test columns, the shared sourced
/// value.
const TITAN: MetrePerSecondSquared = TITAN_GRAVITY;

/// Calibration: how far the column's moist plateau may stray from
/// [`SIMPSON_NAKAJIMA_W_M2`], W/m², 5 %, the legacy's band: it covers the
/// 280-283 of the three line-by-line models; the width itself is a choice
/// (audit, Wiki study 19).
const PLATEAU_TOLERANCE_W_M2: f64 = 14.0;

/// Calibration: how far the plateau may move with the CO₂ and the
/// nitrogen, a share, 3 %: the legacy's bound on the claim that the moist
/// plateau is water's alone (Goldblatt et al. 2013); no source gives a
/// number.
const PLATEAU_GASES_SHARE: f64 = 0.03;

/// Calibration: how flat the moist column must stay over
/// [`PLATEAU_SURFACES`] around the plateau, a share, 1 %: the legacy's
/// bound; Boukrouche et al. 2021 (§III.1) find it flat from 300 to 1050 K
/// under 10 bar of steam, without a number for the flatness.
const RUNAWAY_FLAT_SHARE: f64 = 0.01;

/// Calibration: how far the maximum of the moist column may stand above
/// the plateau, a share, 6 %: the code's own bound (a regression net;
/// the 350-400 K maximum the legacy attributed to Goldblatt et al. 2013
/// is not in the text the audit read, Wiki study 19).
const RUNAWAY_PEAK_SHARE: f64 = 0.06;

/// Definition: the surfaces, K, over which the runaway test looks for the
/// moist column's maximum: 340 to 500 K every 20 K.
const PEAK_SURFACES_K: [f64; 9] = [
    340.0, 360.0, 380.0, 400.0, 420.0, 440.0, 460.0, 480.0, 500.0,
];

/// Calibration: how far the plateau may move on a grid twice as fine,
/// W/m², the legacy's convergence bound on
/// [`PLATEAU_LEVELS_PER_DECADE`].
const GRID_TOLERANCE_W_M2: f64 = 0.5;

/// Definition: a moist surface, K, past the moist column's maximum, where
/// the plateau test varies the gases (the legacy's 380 K).
const MOIST_SURFACE_K: f64 = 380.0;

/// Definition: the CO₂, bar, and the nitrogen, bar, the plateau test
/// varies: 400 ppm to 1 %, one to three Earth's nitrogen.
const PLATEAU_CO2_BAR: [f64; 3] = [4e-4, 1e-3, 1e-2];

/// Definition: the nitrogen, bar, of [`PLATEAU_CO2_BAR`]'s test.
const PLATEAU_N2_BAR: [f64; 3] = [0.78, 1.56, 2.34];

/// Source: Mars's surface air, bar: 6.36 mbar at mean radius, 95.1 % of
/// CO₂, 2.59 % of N₂ and 0.16 % of O₂ (NASA Mars Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html, read for study
/// 25: CO₂ 6.05 mbar, N₂ 0.165 mbar, O₂ 0.010 mbar, derived; argon has
/// no field).
const MARS_SURFACE_AIR: PartialPressures = PartialPressures {
    n2: 6.36e-3 * 0.0259,
    co2: 6.36e-3 * 0.951,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 6.36e-3 * 0.0016,
};

/// Source: Mars's mean surface temperature, K, "Average temperature:
/// ~214 K" (NASA Mars Fact Sheet, read for study 25).
const MARS_SURFACE_K: f64 = 214.0;

/// Source: Mars's black-body temperature, K, 209.8 (NASA Mars Fact Sheet,
/// "Black-body temperature"), the effective temperature its skin is taken
/// from.
const MARS_EFFECTIVE_K: f64 = 209.8;

/// Definition: the relative tolerance of a column under the climate's sky
/// against the clear one when no water clouds form, the sums' rounding.
const CLOUDLESS_TOLERANCE: f64 = 1e-9;

/// Definition: the steam, bar, of the steam plateau test (the legacy's 10
/// bar, the column Boukrouche et al. 2021, §III.1, find flat up to about
/// 1050 K).
const THIN_STEAM_BAR: f64 = 10.0;

/// Definition: a surface, K, on the plateau of 10 bar of steam, under the
/// 1050 K where Boukrouche et al. 2021 (§III.1) see it end (the legacy's
/// 680 K).
const THIN_STEAM_SURFACE_K: f64 = 680.0;

/// Calibration: how far the steam column may stray from
/// [`SIMPSON_NAKAJIMA_W_M2`] on its plateau, a share, 10 %: the legacy's
/// bound; Boukrouche et al. 2021 (§III.1) find the radiation flat without
/// a number for the flatness, and the three line-by-line plateaus span
/// 280 to 283 W/m² (Wiki study 19).
const THIN_STEAM_SHARE: f64 = 0.1;

/// Definition: a Titan-like column of the test cases, near the Huygens
/// surface (93.65 K, 1467 hPa, Fulchignoni et al. 2005, Nature 438, 785,
/// doi:10.1038/nature04314): surface temperature (K), N₂ and CH₄ (bar),
/// and the 82 K equilibrium temperature its skin is taken from (McKay,
/// Pollack & Courtin 1991, Science 253, 1118).
const TITAN_COLUMN: [f64; 4] = [93.7, 1.45, 0.07, 82.0];

/// Definition: a Venus-like column of the test cases (the legacy's):
/// surface temperature (K), CO₂, N₂ and H₂O (bar), and the equilibrium
/// temperature (K) its skin is taken from.
const VENUS_COLUMN: [f64; 5] = [737.0, 92.0, 3.2, 3e-3, 230.0];

/// Definition: a Mars-like column of the test cases (the legacy's):
/// surface temperature (K), CO₂ (bar) and the equilibrium temperature (K)
/// its skin is taken from.
const MARS_COLUMN: [f64; 3] = [210.0, 6e-3, 200.0];

/// Definition: a steam runaway of the test cases (the legacy's): H₂O and
/// CO₂ (bar) over Earth's air, and its surface and skin temperatures (K).
const STEAM_COLUMN: [f64; 4] = [70.0, 6.0, 1500.0, 250.0];

/// Definition: Earth-like cases of the reference test (the legacy's):
/// surface and skin temperatures, K, of a moist Earth, of a cold one,
/// and of a 0.3 bar CO₂ air.
const EARTH_CASES_K: [(f64, f64); 3] = [(288.0, 200.0), (250.0, 180.0), (288.0, 190.0)];

/// Definition: the CO₂, bar, of the reference test's thick-CO₂ Earth.
const THICK_CO2_BAR: f64 = 0.3;

/// Definition: moist surfaces, K, of the reference test on the plateau's
/// skin: near the maximum and on the plateau.
const PLATEAU_CASES_K: [f64; 2] = [380.0, 600.0];

/// Calibration: relative tolerance of the single-precision sum against
/// the double-precision one, and its absolute floor, W/m²: the legacy's
/// bound, about the precision of `f32` sums over a column.
const FAST_TOLERANCE: (f64, f64) = (1e-5, 1e-6);

/// Definition: relative tolerance of nitrogen alone against a black body,
/// the sums' rounding.
const TRANSPARENT_TOLERANCE: f64 = 1e-9;

/// Definition: absolute tolerance, W/m², of an airless surface against
/// `σT⁴`.
const AIRLESS_TOLERANCE_W_M2: f64 = 1e-6;

/// Definition: an airless or frozen surface of the tests, K.
const BARE_SURFACE_K: f64 = 250.0;

/// Definition: the coldest surface the column clamps to, K (Review
/// Focus 1-2 of the plan).
const COLDEST_SURFACE_K: f64 = 1.0;

/// Definition: a surface far above water's critical point, K, for the
/// steam case of the edge tests.
const HOTTEST_SURFACE_K: f64 = 3000.0;

/// Definition: columns the cache test solves before asking again for the
/// first, more than the cache holds.
const DISTINCT_COLUMNS: usize = 1100;

/// Definition: the step, K, between the cache test's distinct surfaces.
const DISTINCT_STEP_K: f64 = 0.01;

/// Definition: a caller's rounding of a vapour pressure, bar.
const ROUNDING_BAR: f64 = -1e-12;

/// The skin temperature of a body of equilibrium temperature `t_eq_k`.
fn skin(t_eq_k: f64) -> Kelvin {
    skin_temperature(Kelvin(t_eq_k))
}

/// `gases` with the vapour of a moist surface at `temperature_k`, with
/// all the water it can hold ([`surface_vapour`]).
fn moist(gases: PartialPressures, temperature_k: f64) -> PartialPressures {
    PartialPressures {
        h2o: surface_vapour(Kelvin(temperature_k), Bar(f64::INFINITY)).0,
        ..gases
    }
}

/// The skin of the moist plateau's own body ([`balance::skin`] of
/// [`earth_moist_plateau`]).
fn plateau_skin() -> Kelvin {
    balance::skin(earth_moist_plateau())
}

/// The outgoing radiation, W/m², of `gases` over `surface_k` under
/// `skin`, on `gravity`, as `profile` builds it, through `cache`.
fn olr(
    gases: &PartialPressures,
    surface_k: f64,
    skin: Kelvin,
    gravity: MetrePerSecondSquared,
    profile: Profile,
    cache: &mut ColumnCache,
) -> f64 {
    outgoing_longwave(gases, Kelvin(surface_k), skin, gravity, profile, cache).0
}

/// Outgoing radiation of `air` over a moist surface at `t`, under the
/// plateau's skin, on Earth.
fn moist_olr(air: PartialPressures, t: f64, cache: &mut ColumnCache) -> f64 {
    olr(
        &moist(air, t),
        t,
        plateau_skin(),
        EARTH,
        Profile::SATURATED,
        cache,
    )
}

/// Outgoing radiation of Earth's moist air over a saturated surface at
/// `t`, under the plateau's skin, on `levels` levels per decade.
fn moist_olr_on(t: f64, levels: f64, cache: &mut ColumnCache) -> f64 {
    olr(
        &moist(EARTH_AIR, t),
        t,
        plateau_skin(),
        EARTH,
        Profile::PLATEAU.on(levels),
        cache,
    )
}

#[test]
fn the_moist_plateau_is_goldblatts() {
    let plateau = earth_moist_plateau().0;
    println!("moist plateau {plateau:.2} W/m²");
    assert!(
        (plateau - SIMPSON_NAKAJIMA_W_M2.0).abs() <= PLATEAU_TOLERANCE_W_M2,
        "{plateau} W/m²"
    );
}

#[test]
fn the_plateau_ignores_co2_and_nitrogen() {
    let cache = &mut ColumnCache::default();
    let base = moist_olr(EARTH_AIR, MOIST_SURFACE_K, cache);
    for co2 in PLATEAU_CO2_BAR {
        for n2 in PLATEAU_N2_BAR {
            let air = PartialPressures {
                co2,
                n2,
                ..EARTH_AIR
            };
            let olr = moist_olr(air, MOIST_SURFACE_K, cache);
            println!("CO₂ {co2} bar, N₂ {n2} bar: {olr:.2} W/m²");
            assert!(
                (olr / base - 1.0).abs() < PLATEAU_GASES_SHARE,
                "CO₂ {co2}, N₂ {n2}: {olr} vs {base}"
            );
        }
    }
}

#[test]
fn the_runaway_anchor_is_flat() {
    // Past the curve's maximum the outgoing radiation is flat within 1 %
    // of its asymptote, the plateau, over 500-640 K, and the maximum
    // stands at most 6 % above it.
    let cache = &mut ColumnCache::default();
    let plateau = earth_moist_plateau().0;
    for t in PLATEAU_SURFACES.map(|t| t.0) {
        let olr = moist_olr_on(t, PLATEAU_LEVELS_PER_DECADE, cache);
        println!("anchor at {t} K: {olr:.2} W/m²");
        assert!(
            (olr / plateau - 1.0).abs() < RUNAWAY_FLAT_SHARE,
            "{t} K: {olr} vs {plateau}"
        );
    }
    let mut peak = 0.0_f64;
    for t in PEAK_SURFACES_K {
        let olr = moist_olr_on(t, PLATEAU_LEVELS_PER_DECADE, cache);
        println!("moist column at {t} K: {olr:.2} W/m²");
        peak = peak.max(olr);
    }
    println!("peak {peak:.2} W/m² over the plateau {plateau:.2}");
    assert!(
        peak <= (1.0 + RUNAWAY_PEAK_SHARE) * plateau,
        "peak {peak} vs {plateau}"
    );
    assert!(peak > plateau, "no maximum before the plateau: {peak}");
}

#[test]
#[ignore = "heavy: the plateau on two grids, about 10 s in a debug build"]
fn the_plateaus_grid_is_converged() {
    let cache = &mut ColumnCache::default();
    let skin = plateau_skin();
    let mut mean = |levels: f64| {
        PLATEAU_SURFACES
            .iter()
            .map(|t| moist_olr_on(t.0, levels, cache))
            .sum::<f64>()
            / PLATEAU_SURFACES.len() as f64
    };
    let (plateau, fine) = (
        mean(PLATEAU_LEVELS_PER_DECADE),
        mean(2.0 * PLATEAU_LEVELS_PER_DECADE),
    );
    println!(
        "asymptote under skin {:.1} K: {plateau:.2}, twice as fine {fine:.2}",
        skin.0
    );
    assert!(
        (plateau - fine).abs() < GRID_TOLERANCE_W_M2,
        "{plateau} vs {fine}"
    );
}

#[test]
fn a_column_without_co2_or_water_radiates_from_its_methane_or_ground() {
    // Titan: 1.45 bar of N₂ and 5 % methane over 93.7 K, under the skin
    // of its 82 K equilibrium, at its gravity.
    let cache = &mut ColumnCache::default();
    let [surface, n2, ch4, t_eq] = TITAN_COLUMN;
    let air = PartialPressures {
        n2,
        ch4,
        ..PartialPressures::default()
    };
    let black = STEFAN_BOLTZMANN * surface.powi(4);
    let titan = olr(&air, surface, skin(t_eq), TITAN, Profile::CLIMATE, cache);
    println!("Titan-like column: {titan:.3} W/m² against σT⁴ {black:.3}");
    assert!(titan.is_finite() && titan > 0.0 && titan < black, "{titan}");
    // Nitrogen alone is transparent: the ground radiates through it.
    let n2 = PartialPressures {
        n2,
        ..PartialPressures::default()
    };
    let bare = olr(&n2, surface, skin(t_eq), TITAN, Profile::CLIMATE, cache);
    assert!(
        (bare / black - 1.0).abs() < TRANSPARENT_TOLERANCE,
        "{bare} vs {black}"
    );
    // And with no air at all.
    let airless = olr(
        &PartialPressures::default(),
        BARE_SURFACE_K,
        Kelvin(0.0),
        EARTH,
        Profile::SATURATED,
        cache,
    );
    assert!(
        (airless - STEFAN_BOLTZMANN * BARE_SURFACE_K.powi(4)).abs() < AIRLESS_TOLERANCE_W_M2,
        "{airless}"
    );
}

#[test]
fn the_fast_radiation_is_the_reference() {
    // The single-precision sum over the open bins against the
    // double-precision one over every band, on columns from Titan to
    // Venus and a steam runaway.
    let moist = |t: f64| moist(EARTH_AIR, t);
    let plateau_skin = plateau_skin().0;
    let [(t0, s0), (t1, s1), (t2, s2)] = EARTH_CASES_K;
    let [p0, p1] = PLATEAU_CASES_K;
    let [steam, steam_co2, steam_k, steam_skin] = STEAM_COLUMN;
    let [venus_k, venus_co2, venus_n2, venus_h2o, venus_eq] = VENUS_COLUMN;
    let [mars_k, mars_co2, mars_eq] = MARS_COLUMN;
    let [titan_k, titan_n2, titan_ch4, titan_eq] = TITAN_COLUMN;
    let cases = [
        (moist(t0), t0, s0, EARTH),
        (moist(t1), t1, s1, EARTH),
        (moist(p0), p0, plateau_skin, EARTH),
        (moist(p1), p1, plateau_skin, EARTH),
        (
            PartialPressures {
                h2o: steam,
                co2: steam_co2,
                ..EARTH_AIR
            },
            steam_k,
            steam_skin,
            EARTH,
        ),
        (
            PartialPressures {
                co2: venus_co2,
                n2: venus_n2,
                h2o: venus_h2o,
                ..PartialPressures::default()
            },
            venus_k,
            skin(venus_eq).0,
            VENUS,
        ),
        (
            PartialPressures {
                co2: mars_co2,
                ..PartialPressures::default()
            },
            mars_k,
            skin(mars_eq).0,
            MARS,
        ),
        (
            PartialPressures {
                n2: titan_n2,
                ch4: titan_ch4,
                ..PartialPressures::default()
            },
            titan_k,
            skin(titan_eq).0,
            TITAN,
        ),
        (
            PartialPressures {
                co2: THICK_CO2_BAR,
                ..EARTH_AIR
            },
            t2,
            s2,
            EARTH,
        ),
    ];
    let mut scratch = Scratch::new();
    for (gases, t, skin, gravity) in cases {
        let fast = with_column(&gases, t, skin, gravity.0, Profile::CLIMATE, |c| {
            radiate(c, &mut scratch)
        })
        .expect("air")
        .iter()
        .sum::<f64>();
        let reference = with_column(
            &gases,
            t,
            skin,
            gravity.0,
            Profile::CLIMATE,
            radiate_reference,
        )
        .expect("air")
        .iter()
        .sum::<f64>();
        println!("{t} K: fast {fast:.5}, reference {reference:.5} W/m²");
        let (relative, floor) = FAST_TOLERANCE;
        assert!(
            (fast - reference).abs() < relative * reference + floor,
            "{gases:?} at {t} K: {fast} vs {reference}"
        );
    }
}

#[test]
fn a_cached_column_is_the_fresh_one() {
    let gases = moist(EARTH_AIR, MOIST_SURFACE_K);
    let skin = balance::skin(SIMPSON_NAKAJIMA_W_M2);
    let column = |surface_k: f64, profile: Profile, cache: &mut ColumnCache| {
        olr(&gases, surface_k, skin, EARTH, profile, cache).to_bits()
    };
    let cache = &mut ColumnCache::default();
    let first = column(MOIST_SURFACE_K, Profile::CLIMATE, cache);
    assert_eq!(column(MOIST_SURFACE_K, Profile::CLIMATE, cache), first);
    let fresh = column(
        MOIST_SURFACE_K,
        Profile::CLIMATE,
        &mut ColumnCache::default(),
    );
    assert_eq!(fresh, first);
    // The sky, the grid and the top are in the key: each profile is its
    // own column, the same through the cache as fresh.
    for profile in [
        Profile::CLEAR,
        Profile::SATURATED,
        Profile::CLIMATE.on(2.0 * LEVELS_PER_DECADE),
        Profile::CLIMATE.up_to(TOP_SHARE / 100.0),
    ] {
        let cached = column(MOIST_SURFACE_K, profile, cache);
        let alone = column(MOIST_SURFACE_K, profile, &mut ColumnCache::default());
        assert_eq!(cached, alone, "{profile:?}");
        assert_ne!(cached, first, "{profile:?} answered as the climate's");
    }
    // Past a full cache the first column is gone, and recomputed the same.
    // The columns that fill it are airless, which cost a black body each,
    // and are kept like any other.
    for i in 1..=DISTINCT_COLUMNS {
        let surface_k = BARE_SURFACE_K + i as f64 * DISTINCT_STEP_K;
        let bare = olr(
            &PartialPressures::default(),
            surface_k,
            skin,
            EARTH,
            Profile::CLIMATE,
            cache,
        );
        assert_eq!(
            bare.to_bits(),
            (STEFAN_BOLTZMANN * surface_k.powi(4)).to_bits()
        );
    }
    assert_eq!(column(MOIST_SURFACE_K, Profile::CLIMATE, cache), first);
}

#[test]
fn extreme_surfaces_stay_finite() {
    let cache = &mut ColumnCache::default();
    let [steam, co2, _, steam_skin] = STEAM_COLUMN;
    let steam_air = PartialPressures {
        h2o: steam,
        co2,
        ..EARTH_AIR
    };
    let [venus_surface, venus_co2, _, _, venus_eq] = VENUS_COLUMN;
    let venus_air = PartialPressures {
        co2: venus_co2,
        ..PartialPressures::default()
    };
    for (gases, surface, skin_k, gravity) in [
        (EARTH_AIR, COLDEST_SURFACE_K, 0.0, EARTH),
        (steam_air, HOTTEST_SURFACE_K, steam_skin, EARTH),
        (venus_air, venus_surface, skin(venus_eq).0, VENUS),
    ] {
        let olr = olr(
            &gases,
            surface,
            Kelvin(skin_k),
            gravity,
            Profile::CLIMATE,
            cache,
        );
        let black = STEFAN_BOLTZMANN * surface.powi(4);
        println!("{surface} K under {skin_k} K: {olr:e} W/m² against σT⁴ {black:e}");
        assert!(
            olr.is_finite() && olr > 0.0 && olr <= black,
            "{surface} K: {olr} vs {black}"
        );
    }
}

#[test]
fn a_negative_rounding_is_no_gas() {
    let cache = &mut ColumnCache::default();
    let skin = skin(BARE_SURFACE_K);
    let clean = olr(
        &EARTH_AIR,
        BARE_SURFACE_K,
        skin,
        EARTH,
        Profile::CLIMATE,
        cache,
    );
    let rounded = PartialPressures {
        h2o: ROUNDING_BAR,
        ch4: f64::NAN,
        ..EARTH_AIR
    };
    let dirty = olr(
        &rounded,
        BARE_SURFACE_K,
        skin,
        EARTH,
        Profile::CLIMATE,
        cache,
    );
    assert_eq!(dirty.to_bits(), clean.to_bits(), "{dirty} vs {clean}");
}

#[test]
fn mars_and_titan_have_no_cloud_greenhouse() {
    // Their airs hold no water, so the climate's column, cloudy where
    // water clouds form, is the clear one (legacy l.2628), at their
    // measured surface temperatures.
    let cache = &mut ColumnCache::default();
    for (name, air, surface, effective, gravity) in [
        (
            "Mars",
            MARS_SURFACE_AIR,
            MARS_SURFACE_K,
            MARS_EFFECTIVE_K,
            MARS,
        ),
        (
            "Titan",
            TITAN_SURFACE_AIR,
            TITAN_SURFACE_K,
            TITAN_EFFECTIVE_K,
            TITAN,
        ),
    ] {
        let skin = skin(effective);
        let clear = olr(&air, surface, skin, gravity, Profile::CLEAR, cache);
        let all = olr(&air, surface, skin, gravity, Profile::CLIMATE, cache);
        println!("{name}: clear {clear:.4}, all sky {all:.4} W/m²");
        assert!(
            (all / clear - 1.0).abs() < CLOUDLESS_TOLERANCE,
            "{name}: {all} vs {clear}"
        );
    }
}

#[test]
fn thin_steam_stays_on_the_plateau_at_680_k() {
    // 10 bar of pure steam: the column is still on the plateau at 680 K
    // (legacy l.4207; its scan for where the plateau ends asserted
    // nothing and is not ported).
    let steam = PartialPressures {
        h2o: THIN_STEAM_BAR,
        ..PartialPressures::default()
    };
    let olr = olr(
        &steam,
        THIN_STEAM_SURFACE_K,
        Kelvin(0.0),
        EARTH,
        Profile::SATURATED,
        &mut ColumnCache::default(),
    );
    println!("10 bar of steam at 680 K: {olr:.2} W/m²");
    assert!(
        (olr / SIMPSON_NAKAJIMA_W_M2.0 - 1.0).abs() < THIN_STEAM_SHARE,
        "{olr} W/m²"
    );
}
