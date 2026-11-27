//! Tests of the longwave column, ported from `legacy-final`'s
//! `planet/column.rs` with their tolerances, with the helpers they read
//! on the moist plateau of `crate::balance` (legacy `planet/climate.rs`'s
//! `plateau_w_m2` and `plateau_skin_k`), and the cache and edge-case tests
//! of the port.

use super::*;
use crate::balance::{self, PLATEAU_SURFACES, SIMPSON_NAKAJIMA_W_M2, earth_moist_plateau};
use crate::constants::{MARS_SURFACE_GRAVITY, STANDARD_GRAVITY, STEFAN_BOLTZMANN};
use crate::radiation::ozone::EARTH_OZONE_COLUMN;
use crate::radiation::test_points::{
    EARTH_AIR, EARTH_MEASURED_AIR, EARTH_PRESSURE_BAR, EARTH_SURFACE_K, NO_OZONE, PER_MILLION,
    PREINDUSTRIAL_PPM, TITAN_EFFECTIVE_K, TITAN_GRAVITY, TITAN_SURFACE_AIR, TITAN_SURFACE_K,
    VENUS_GRAVITY,
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
/// 25: CO₂ 6.05 mbar, N₂ 0.165 mbar, O₂ 0.010 mbar, derived), and 1.93 %
/// of argon-40 (Mahaffy et al. 2013, Science 341, 263, abstract, as
/// study 29 reads it: 0.123 mbar).
const MARS_SURFACE_AIR: PartialPressures = PartialPressures {
    n2: 6.36e-3 * 0.0259,
    co2: 6.36e-3 * 0.951,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 6.36e-3 * 0.0016,
    ar: 6.36e-3 * 0.0193,
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

/// Source: Earth's clear-sky outgoing longwave radiation, the cloud-free
/// scenes of CERES EBAF Ed4.0, July 2005 to June 2015: 268.1 W/m² (CERES
/// EBAF Ed4.0 Data Quality Summary, NASA Langley, 2018, Table 6-1;
/// `docs/science/radiative-transfer.md`, study 32).
const CERES_CLEAR_SKY_W_M2: f64 = 268.1;

/// Source: Earth's all-sky outgoing longwave radiation over the same
/// years, 240.1 W/m² (CERES EBAF Ed4.0 Data Quality Summary, Table 6-1):
/// what Earth emits, whose effective temperature sets the test's skin.
const CERES_ALL_SKY_W_M2: f64 = 240.1;

/// Source: the clear sky of the same air with its clouds removed (the
/// "total-region" clear sky a column model computes) minus that of the
/// cloud-free scenes, −2.2 W/m² (Loeb et al. 2020, *J. Climate* 33, 61,
/// doi:10.1175/JCLI-D-19-0381.1, Table 3: 265.9 against 268.1).
const TOTAL_REGION_W_M2: f64 = -2.2;

/// Source: what CFC-11 (with the other halocarbons as its equivalent)
/// and CFC-12 take from Earth's clear-sky outgoing radiation, each
/// removed from a present-day atmosphere: 0.22 and 0.17 W/m² (Hogan &
/// Matricardi 2020, *Geosci. Model Dev.* 13, 6501,
/// doi:10.5194/gmd-13-6501-2020, Table 4, line by line). The column has
/// no lines of the halocarbons, so neither absorbs in the test. Methane,
/// N₂O (1.5 W/m² each in the same table) and ozone absorb in the column
/// since #704: the test air carries CERES's ([`CERES_CH4_PPB`],
/// [`CERES_N2O_PPB`], [`EARTH_OZONE_COLUMN`]). N₂'s and O₂'s pairs (0.15
/// and 0.096 W/m² in the same table) are in the column since study 36
/// ([`earths_nitrogen_and_oxygen_pairs_take_what_line_by_line_codes_find`]).
const MINOR_GASES_W_M2: f64 = 0.22 + 0.17;

/// Source: the methane of CERES's years, July 2005 to June 2015, ppb:
/// its global mean of 2011, the middle of the span, 1803.1 ppb (NOAA;
/// AGAGE 1803.6; IPCC AR6 WG1, Gulev et al. 2021, Table 2.2), rounded.
const CERES_CH4_PPB: f64 = 1803.0;

/// Source: the N₂O of CERES's years, ppb, as [`CERES_CH4_PPB`]: its
/// global mean of 2011, 324.2 ppb (NOAA; AGAGE 324.7, CSIRO 324.0; IPCC
/// AR6 WG1, Gulev et al. 2021, Table 2.2, p. 302). No air of the model
/// holds N₂O (its law is #737); the CERES test's does, as Earth's
/// measured air did, so that methane's band shares its overlap.
const CERES_N2O_PPB: f64 = 324.2;

/// Source: what methane takes from Earth's clear-sky outgoing radiation,
/// removed from a present-day atmosphere, W/m²: 1.5 at 1921 ppb (Hogan &
/// Matricardi 2020, Table 4, LBLRTM on 50 profiles) and 1.87 at 1.8 ppm
/// (Höpfner et al. 2012, Table 1, KOPRA, the cloud-free columns of one
/// day), each widened by the error study 36 allows a k-distribution
/// against the line-by-line codes, RRTMGP's 8.2 % on RFMIP's methane
/// step (`docs/science/longwave-absorbers.md`, "Decisions";
/// [`super::rfmip_tests`]).
const METHANE_LINE_BY_LINE_W_M2: (f64, f64) = (1.5 * (1.0 - 0.082), 1.87 * (1.0 + 0.082));

/// Source: what N₂'s and O₂'s collision-induced pairs take from Earth's
/// clear-sky outgoing radiation, W/m², each gas's absorption removed from
/// a present-day atmosphere and the two summed: 0.15 + 0.096 (Hogan &
/// Matricardi 2020, Table 4, LBLRTM on 50 profiles) and 0.17 + 0.11
/// (Höpfner et al. 2012, *Geophys. Res. Lett.* 39, L10706,
/// doi:10.1029/2012GL051409, Table 1, KOPRA, the cloud-free global mean
/// of one day; "0.28 Wm-2" in their conclusions).
const PAIRS_LINE_BY_LINE_W_M2: (f64, f64) = (0.15 + 0.096, 0.17 + 0.11);

/// Calibration: how much more than the larger of
/// [`PAIRS_LINE_BY_LINE_W_M2`] the column's N₂ and O₂ pairs may take,
/// W/m², 0.15. Study 36 (`docs/science/longwave-absorbers.md`, "In the
/// app") measured 0.406 W/m² on this test's column, 0.126 more than
/// Höpfner et al.'s 0.28: N₂'s pairs take about 0.27 against 0.15 to
/// 0.17, most of the excess in N₂'s roto-translational band below
/// 650 cm⁻¹, which water hides, and O₂'s 0.134 against 0.096 to 0.11.
/// Its cause is not isolated (the column's dry upper air, HITRAN's newer
/// N₂–N₂ far wing, or the N₂–O₂ roto-translational band HITRAN does not
/// give): issue #718. The bound is that excess rounded up, so that the
/// test fails if it grows; #718 brings it to zero, or records the
/// departure. What was searched and not found: a third line-by-line
/// value, or a published error of either source on these pairs.
const PAIRS_EXCESS_W_M2: f64 = 0.15;

/// Source: what N₂ and O₂ take from the outgoing radiation of an air in
/// which they alone absorb, W/m²: 2.8 each, against the 365.7 W/m² of
/// the same air without absorbers (Höpfner et al. 2012, §3, paragraph 15,
/// KOPRA on a mid-latitude standard atmosphere), 5.6 for the two ("would
/// exceed the OLR reduction due to CH4", 4.3 W/m², "by a factor of
/// around 1.3").
const PAIRS_ALONE_W_M2: f64 = 2.8 + 2.8;

/// Calibration: the width of the plausibility band of
/// [`PAIRS_ALONE_W_M2`], a share, 0.26: the spread of two line-by-line
/// codes on N₂'s pairs over the same 42 profiles, ARTS's 6.0 % of
/// methane's effect against KOPRA's 8.1 % (Höpfner et al. 2012, §5,
/// paragraph 20), `1 − 6.0 / 8.1`. Taken as the band's width, a choice:
/// the test's column (a dry adiabat from a 288 K surface, 390.1 W/m²
/// without absorbers) is not Höpfner et al.'s standard atmosphere (a
/// 283 K black body's 365.7 W/m²), and no source gives the removal on it.
const PAIRS_ALONE_SHARE: f64 = 1.0 - 6.0 / 8.1;

/// Source: what ozone takes from Earth's outgoing radiation, W/m², from
/// 2.7 % of the 155 W/m² greenhouse under all skies (Schmidt et al. 2010,
/// *J. Geophys. Res.* 115, D20106, doi:10.1029/2010JD014287, Table 1,
/// removed alone), to 10 W/m² under a clear sky with its overlaps (Kiehl
/// & Trenberth 1997, *BAMS* 78, 197, Table 3). No global-mean
/// line-by-line clear-sky value was found (study 32, study 34):
/// [`earths_ozone_takes_what_the_sources_find`] holds the column's
/// [`EARTH_OZONE_COLUMN`] in this window.
const OZONE_W_M2: (f64, f64) = (0.027 * 155.0, 10.0);

/// Source: the mean global CO₂ of CERES's years, July 2005 to June 2015,
/// about 389 ppm (NOAA Global Monitoring Laboratory, global annual means,
/// `co2_annmean_gl.txt`, 378.98 ppm in 2005 to 399.65 ppm in 2015, read
/// 2026-11-25).
const CERES_CO2_PPM: f64 = 389.0;

/// Source: the CO₂ forcing coefficient, `ΔF = 5.35 ln(C / C₀)` W/m²
/// (Myhre, Highwood, Shine & Stordal 1998, *Geophys. Res. Lett.* 25,
/// 2715, doi:10.1029/98GL01908): the radiation the column gains at
/// 280 ppm over CERES's [`CERES_CO2_PPM`], 1.76 W/m².
const MYHRE_CO2_W_M2: f64 = 5.35;

/// Source: the most a black ground adds over a real one: the clear-sky
/// net longwave at the surface, 398.2 − 317.2 W/m² (Loeb et al. 2020,
/// Table 3, up and down, total region), times the 2 % a surface of
/// emissivity 0.98 (the RFMIP profiles' surfaces, Pincus et al. 2016,
/// *Geosci. Model Dev.* 9, 3447) does not emit; an upper bound, part of
/// it being absorbed on the way up.
const BLACK_GROUND_W_M2: f64 = (1.0 - 0.98) * (398.2 - 317.2);

/// Source: the two terms of CERES's uncertainty, the calibration's 0.75 %
/// (1σ) and the 2.75 W/m² of the scene identification (CERES EBAF Ed4.0
/// Data Quality Summary, §4.4). The summary gives them apart; the test
/// combines them in quadrature itself, as independent errors: 3.4 W/m²
/// on 268.1 W/m².
const CERES_UNCERTAINTY: (f64, f64) = (0.0075, 2.75);

/// Definition: the surface temperatures, K, over which the smoothness
/// test sweeps Earth's column, 284 to 292 K: the climate's thermostat
/// (288 K) and the reference Earth's 285.82 K in planet PR 2d-4 inside,
/// the 273.16 K level sweeping 1.5 km of its troposphere.
const SMOOTH_RANGE_K: (f64, f64) = (284.0, 292.0);

/// Definition: the step, K, of the smoothness test's surfaces, a
/// quarter of a kelvin: a 4 K sawtooth (study 32) spans 16 of them.
const SMOOTH_STEP_K: f64 = 0.25;

/// Calibration: how much the rise of the column's outgoing radiation over
/// one [`SMOOTH_STEP_K`] may differ from the rise over the next, as a
/// share of the larger. It bounds the model's own numerics: no published
/// tolerance on a column model's smoothness was found in the sources
/// read (Koll & Cronin 2018 say only that the radiation rises), so the
/// bound is placed between two measured ends (study 32,
/// `docs/science/radiative-transfer.md`, "The phase step of the moist
/// adiabat"). A smooth emission changes its rise little from one step to
/// the next: as `T⁴`, by `3/T` per kelvin, 0.26 % over 0.25 K at 288 K;
/// the column, with its humidity and clouds, by 1.3 % clear and 1.4 %
/// cloudy after the split. A step whose phase is wrong over part of its
/// span shifts the whole column above by a share of the step's lapse
/// (the latent heat jumps by fusion's 6.01 kJ/mol on 45 kJ/mol), a jump
/// that does not shrink with the sweep's step while the rise does: before
/// the split the rise changed by 32 % clear and 192 % cloudy. 10 % sits a
/// factor 7 above the first and 3 under the second. Not derived from the
/// fusion share: the change of the rise scales as the jump over the
/// sweep's step, so a share of the latent heat alone does not set it.
/// The same bound holds CO₂'s column across its own triple point
/// (`a_co2_column_is_smooth_across_its_triple_point`): 5.6 % since its
/// split, 176 % before (three steps falling), a factor 1.8 under the bound
/// and 18 above it.
const SMOOTH_RISE_CHANGE: f64 = 0.1;

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
    outgoing_longwave(
        gases,
        NO_OZONE,
        Kelvin(surface_k),
        skin,
        gravity,
        profile,
        cache,
    )
    .0
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
fn a_column_without_co2_or_water_radiates_through_its_pairs() {
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
    // Nitrogen alone absorbs only through its own pairs (N₂–N₂): the
    // ground sends out more through it than through the air with methane's
    // pairs, less than its black body.
    let n2 = PartialPressures {
        n2,
        ..PartialPressures::default()
    };
    let nitrogen = olr(&n2, surface, skin(t_eq), TITAN, Profile::CLIMATE, cache);
    assert!(
        titan < nitrogen && nitrogen < black,
        "{titan} {nitrogen} {black}"
    );
    // Without its pairs it is transparent: the ground radiates through it.
    let bare = without_pairs(ALL_PAIRS, || {
        olr(
            &n2,
            surface,
            skin(t_eq),
            TITAN,
            Profile::CLIMATE,
            &mut ColumnCache::default(),
        )
    });
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

/// Argon's heat capacity, 5/2 R (Chase 1998 through the NIST WebBook),
/// sets the dry slope `d ln T / d ln p = R / c_p` of pure argon at 2/5,
/// and Earth's argon, 0.93 % of its dry air's moles, lowers the air's
/// heat capacity from 29.18 to 29.09 J/(mol·K) (study 29, derived from
/// the NIST heat capacities at the fact sheet's volume shares).
#[test]
fn argon_has_a_monatomic_gass_heat_capacity() {
    /// Definition: a monatomic ideal gas, `c_p = 5/2 R`, so `R / c_p`.
    const MONATOMIC_SLOPE: f64 = 0.4;
    let argon = DryAir::of(
        &PartialPressures {
            ar: 1.0,
            ..PartialPressures::default()
        },
        BARE_SURFACE_K,
    );
    let slope = MOLAR_GAS_CONSTANT / argon.cp;
    assert!((slope / MONATOMIC_SLOPE - 1.0).abs() < 1e-4, "{slope}");

    // Source: dry air, NASA Earth fact sheet: 78.08 % N₂, 20.95 % O₂,
    // 9340 ppm Ar, 420 ppm CO₂, by volume.
    let air = PartialPressures {
        n2: 0.7808,
        o2: 0.2095,
        ar: 9340e-6,
        co2: 420e-6,
        ..PartialPressures::default()
    };
    /// Source: study 29's heat capacity of Earth's dry air with its
    /// argon, 29.09 J/(mol·K) (`docs/science/radiative-transfer.md`, on
    /// argon's heat capacity), within 0.1 %: its last printed digit and
    /// the code's heat capacities, rounded at 298.15 K.
    const EARTH_DRY_CP: f64 = 29.09;
    let cp = DryAir::of(&air, BARE_SURFACE_K).cp;
    assert!((cp / EARTH_DRY_CP - 1.0).abs() < 1e-3, "{cp}");
    let without = DryAir::of(&PartialPressures { ar: 0.0, ..air }, BARE_SURFACE_K).cp;
    assert!(without > cp, "{without} {cp}");
}

/// Earth's measured air at `co2_ppm` of its dry air, with the vapour of
/// a surface at `surface_k` ([`surface_vapour`] of an ocean).
fn earth_at(co2_ppm: f64, surface_k: f64) -> PartialPressures {
    PartialPressures {
        co2: co2_ppm * PER_MILLION * EARTH_PRESSURE_BAR,
        ..moist(EARTH_MEASURED_AIR, surface_k)
    }
}

/// [`earth_at`] 280 ppm and 288 K with CERES's methane ([`CERES_CH4_PPB`]
/// of its dry air): the CERES test's air, which [`earth_clear`] gives
/// CERES's N₂O too.
fn ceres_air() -> PartialPressures {
    PartialPressures {
        ch4: CERES_CH4_PPB * 1e-9 * EARTH_PRESSURE_BAR,
        ..earth_at(PREINDUSTRIAL_PPM, EARTH_SURFACE_K)
    }
}

/// The clear-sky outgoing radiation, W/m², of `gases` with the ozone
/// column `ozone` and `n2o_ppb` of N₂O in its dry air over Earth's 288 K
/// surface, under Earth's skin, on Earth's gravity: [`outgoing_longwave`]'s
/// column, with the N₂O no air of the model holds yet.
fn earth_clear(gases: &PartialPressures, ozone: DobsonUnit, n2o_ppb: f64) -> f64 {
    let mut scratch = Scratch::new();
    with_column(
        gases,
        EARTH_SURFACE_K,
        earth_skin().0,
        EARTH.0,
        Profile::CLEAR,
        |c| radiate(&c.with_ozone(ozone).with_n2o(n2o_ppb * 1e-9), &mut scratch),
    )
    .expect("Earth's column")
    .iter()
    .sum()
}

#[test]
fn earths_test_column_is_the_climates_without_n2o() {
    // `earth_clear` stands for `outgoing_longwave` (the climate's path)
    // but for its N₂O: without N₂O the two are the same bits.
    let air = ceres_air();
    let climate = outgoing_longwave(
        &air,
        EARTH_OZONE_COLUMN,
        Kelvin(EARTH_SURFACE_K),
        earth_skin(),
        EARTH,
        Profile::CLEAR,
        &mut ColumnCache::default(),
    );
    assert_eq!(earth_clear(&air, EARTH_OZONE_COLUMN, 0.0), climate.0);
}

#[test]
fn earths_ozone_takes_what_the_sources_find() {
    // Issue #704: Earth's 300 DU laid out as the column lays it
    // (calibration 5 of `docs/science/ozone.md`), removed from the CERES
    // test's air, clear.
    let air = ceres_air();
    let taken = earth_clear(&air, NO_OZONE, CERES_N2O_PPB)
        - earth_clear(&air, EARTH_OZONE_COLUMN, CERES_N2O_PPB);
    let (low, high) = OZONE_W_M2;
    println!("Earth's ozone takes {taken:.3} W/m², the sources {low:.3} to {high}");
    assert!(
        (low..=high).contains(&taken),
        "{taken} against {low} to {high}"
    );
}

#[test]
fn earths_methane_takes_what_line_by_line_codes_find() {
    // Issue #704: CERES's methane removed from the CERES test's air, with
    // Earth's ozone and CERES's N₂O (whose band overlaps methane's, as in
    // the line-by-line codes' air), clear.
    let air = ceres_air();
    let without = PartialPressures { ch4: 0.0, ..air };
    let taken = earth_clear(&without, EARTH_OZONE_COLUMN, CERES_N2O_PPB)
        - earth_clear(&air, EARTH_OZONE_COLUMN, CERES_N2O_PPB);
    let (low, high) = METHANE_LINE_BY_LINE_W_M2;
    println!("Earth's methane takes {taken:.3} W/m², line by line {low:.3} to {high:.3}");
    assert!(
        (low..=high).contains(&taken),
        "{taken} against {low} to {high}"
    );
}

/// The skin of Earth: [`skin_temperature`] (`2^-¼` of it) of the
/// effective temperature of CERES's all-sky outgoing radiation,
/// `(240.1 / σ)^¼` = 255.09 K: 214.51 K, the skin the test columns use.
fn earth_skin() -> Kelvin {
    skin((CERES_ALL_SKY_W_M2 / STEFAN_BOLTZMANN).powf(0.25))
}

#[test]
fn earths_clear_sky_column_is_ceres_like_for_like() {
    // The climate's own clear column on Earth's measured air at 288 K and
    // 280 ppm, with CERES's methane and N₂O and Earth's ozone column, against
    // CERES's clear sky made like for like (study 32):
    // the clear sky of the same air without its clouds, plus what the
    // gases the column lacks take, plus the radiation 280 ppm of CO₂
    // gives back over CERES's 389, plus at most a black ground; within
    // CERES's uncertainty.
    let co2 = MYHRE_CO2_W_M2 * (CERES_CO2_PPM / PREINDUSTRIAL_PPM).ln();
    let base = CERES_CLEAR_SKY_W_M2 + TOTAL_REGION_W_M2 + MINOR_GASES_W_M2 + co2;
    let (calibration, scene) = CERES_UNCERTAINTY;
    let uncertainty = (calibration * CERES_CLEAR_SKY_W_M2).hypot(scene);
    let low = base - uncertainty;
    let high = base + BLACK_GROUND_W_M2 + uncertainty;
    let olr = earth_clear(&ceres_air(), EARTH_OZONE_COLUMN, CERES_N2O_PPB);
    println!("Earth's clear sky {olr:.2} W/m², like for like {low:.2} to {high:.2}");
    assert!((low..=high).contains(&olr), "{olr} against {low} to {high}");
}

/// Definition: every pair of [`spectrum::PAIRS`], as the bits of
/// [`spectrum::PAIRS_LEFT_OUT`].
const ALL_PAIRS: u8 = (1 << spectrum::PAIR_COUNT) - 1;

/// The bits of [`spectrum::PAIRS_LEFT_OUT`] of the pairs made only of
/// `gases`.
fn pairs_of(gases: &[spectrum::Partner]) -> u8 {
    spectrum::PAIRS
        .iter()
        .enumerate()
        .filter(|(_, p)| gases.contains(&p.gases.0) && gases.contains(&p.gases.1))
        .fold(0, |bits, (i, _)| bits | (1 << i))
}

/// Restores [`spectrum::PAIRS_LEFT_OUT`] to none when dropped, so that a
/// panic inside [`without_pairs`] leaves no pair out for the next test on
/// the thread.
struct PairsBack;

impl Drop for PairsBack {
    fn drop(&mut self) {
        spectrum::PAIRS_LEFT_OUT.with(|left| left.set(0));
    }
}

/// `f`, with the pairs of `bits` left out of the spectrum on this thread
/// ([`spectrum::PAIRS_LEFT_OUT`]); `f` takes a fresh cache.
fn without_pairs<R>(bits: u8, f: impl FnOnce() -> R) -> R {
    spectrum::PAIRS_LEFT_OUT.with(|left| left.set(bits));
    let _back = PairsBack;
    f()
}

/// What the pairs of `bits` take from the clear-sky outgoing radiation,
/// W/m², of `gases` over a surface at `surface_k` under Earth's skin, on
/// Earth's gravity: the radiation with them left out less with them in.
fn pairs_take(bits: u8, gases: &PartialPressures, surface_k: f64) -> f64 {
    let clear = || {
        olr(
            gases,
            surface_k,
            earth_skin(),
            EARTH,
            Profile::CLEAR,
            &mut ColumnCache::default(),
        )
    };
    without_pairs(bits, clear) - clear()
}

#[test]
fn earths_nitrogen_and_oxygen_pairs_take_what_line_by_line_codes_find() {
    // Study 36: N₂'s and O₂'s pairs (N₂–N₂, O₂–O₂, O₂–N₂) on Earth's
    // measured air at 288 K and 280 ppm, clear, against the two
    // line-by-line sources, with the excess of issue #718 named: the
    // column takes about 0.4 W/m² where the sources give 0.25 to 0.28.
    use spectrum::Partner::{N2, O2};
    let taken = pairs_take(
        pairs_of(&[N2, O2]),
        &earth_at(PREINDUSTRIAL_PPM, EARTH_SURFACE_K),
        EARTH_SURFACE_K,
    );
    let (low, high) = PAIRS_LINE_BY_LINE_W_M2;
    println!(
        "Earth's N₂ and O₂ pairs take {taken:.3} W/m², line by line {low:.3} to {high:.3} \
         (+{PAIRS_EXCESS_W_M2} of issue #718)"
    );
    assert!(
        (low..=high + PAIRS_EXCESS_W_M2).contains(&taken),
        "{taken} W/m² against {low} to {high} + {PAIRS_EXCESS_W_M2}"
    );
}

#[test]
fn an_air_of_nitrogen_and_oxygen_alone_loses_what_hopfner_finds() {
    // Plausibility (Höpfner et al. 2012, §3): Earth's N₂ and O₂ alone,
    // no water, no CO₂, over 288 K, clear; their pairs' removal.
    use spectrum::Partner::{N2, O2};
    let taken = pairs_take(pairs_of(&[N2, O2]), &EARTH_MEASURED_AIR, EARTH_SURFACE_K);
    println!("N₂ and O₂ alone: their pairs take {taken:.3} W/m², Höpfner {PAIRS_ALONE_W_M2}");
    assert!(
        (taken / PAIRS_ALONE_W_M2 - 1.0).abs() <= PAIRS_ALONE_SHARE,
        "{taken} W/m² against {PAIRS_ALONE_W_M2} ± {PAIRS_ALONE_SHARE:.2}"
    );
}

/// The rises of the outgoing radiation `olr_at(surface_k)`, W/m², over
/// steps of [`SMOOTH_STEP_K`] across `(from, to)`.
fn rises_over((from, to): (f64, f64), mut olr_at: impl FnMut(f64) -> f64) -> Vec<f64> {
    let step = SMOOTH_STEP_K;
    let n = ((to - from) / step).round() as usize;
    let olrs: Vec<f64> = (0..=n).map(|i| olr_at(from + i as f64 * step)).collect();
    olrs.windows(2).map(|w| w[1] - w[0]).collect()
}

/// The most a rise of `rises` changes over the next, as a share of the
/// larger.
fn most_change(rises: &[f64]) -> f64 {
    rises
        .windows(2)
        .map(|w| (w[1] - w[0]).abs() / w[0].abs().max(w[1].abs()))
        .fold(0.0, f64::max)
}

/// The rises of the column's outgoing radiation, W/m², over steps of
/// [`SMOOTH_STEP_K`] across [`SMOOTH_RANGE_K`].
fn rises(profile: Profile, cache: &mut ColumnCache) -> Vec<f64> {
    rises_over(SMOOTH_RANGE_K, |t| {
        olr(
            &earth_at(PREINDUSTRIAL_PPM, t),
            t,
            earth_skin(),
            EARTH,
            profile,
            cache,
        )
    })
}

#[test]
fn earths_column_is_smooth_in_its_surface() {
    // The outgoing radiation rises with the surface (Koll & Cronin 2018,
    // doi:10.1073/pnas.1809868115) and, smooth in it, by about as much
    // over each step as over the next: a step of the moist adiabat whose
    // latent heat switches phase inside its span (study 32) makes the
    // rise jump where the surface moves that span across 273.16 K.
    let share = SMOOTH_RISE_CHANGE;
    let cache = &mut ColumnCache::default();
    for profile in [Profile::CLEAR, Profile::CLIMATE] {
        let rises = rises(profile, cache);
        let worst = most_change(&rises);
        println!("{profile:?}: rises {rises:.3?}, most change {worst:.4}");
        assert!(
            rises.iter().all(|&r| r > 0.0),
            "{profile:?} falls: {rises:?}"
        );
        assert!(worst <= share, "{profile:?}: a rise changes by {worst:.3}");
    }
}

/// Definition: a dry air of 12.5 bar of CO₂ over 0.75 bar of nitrogen,
/// whose CO₂ saturates at about 239 K at the surface (its saturation curve,
/// [`co2_saturation_bar`]): a cold, CO₂-rich air whose column condenses
/// CO₂ from the surface, or from just above it, up across CO₂'s triple
/// point (216.592 K) to its skin, as a methane Earth's collapsed climate
/// past its outer edge does (#703, measured at 1.272 AU).
const CO2_RICH_AIR: PartialPressures = PartialPressures {
    n2: 0.75,
    co2: 12.5,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.0,
    ar: 0.0,
};

/// Definition: the surface temperatures, K, over which the smoothness test
/// sweeps [`CO2_RICH_AIR`]'s column, 232 to 239 K: colder than its
/// surface's CO₂ saturation (about 239.4 K), so that the column condenses
/// from the surface up, and the level where it crosses CO₂'s triple point
/// sweeping two of its layers. Past the saturation the column's outgoing
/// radiation flattens (rises of a few mW/m² per step), whose share of
/// change the test cannot read.
const CO2_SMOOTH_RANGE_K: (f64, f64) = (232.0, 239.0);

/// Definition: the skin of [`CO2_RICH_AIR`]'s column, 190 K, below CO₂'s
/// triple point, so that its CO₂ pseudo-adiabat crosses it (the methane
/// Earth of #703 at 1.272 AU had a skin of 190.68 K).
const CO2_RICH_SKIN: Kelvin = Kelvin(190.0);

#[test]
fn a_co2_column_is_smooth_across_its_triple_point() {
    // CO₂'s latent heat jumps by fusion's at its triple point (sublimation
    // below, vaporisation above): a step of its pseudo-adiabat that takes
    // the phase of its midpoint for its whole span shifts the column above
    // by about 1 K, and its outgoing radiation by about 1.3 W/m², where the
    // surface moves that midpoint across 216.592 K (#703's methane Earth,
    // whose collapse stopped on the jump). The bound is the water column's
    // ([`SMOOTH_RISE_CHANGE`]).
    let cache = &mut ColumnCache::default();
    for profile in [Profile::CLEAR, Profile::CLIMATE] {
        let rises = rises_over(CO2_SMOOTH_RANGE_K, |t| {
            olr(&CO2_RICH_AIR, t, CO2_RICH_SKIN, EARTH, profile, cache)
        });
        let worst = most_change(&rises);
        println!("{profile:?}: rises {rises:.3?}, most change {worst:.4}");
        assert!(
            rises.iter().all(|&r| r > 0.0),
            "{profile:?} falls: {rises:?}"
        );
        assert!(
            worst <= SMOOTH_RISE_CHANGE,
            "{profile:?}: a rise changes by {worst:.3}"
        );
    }
}

/// Definition: the substeps of the reference integration of one step of
/// CO₂'s pseudo-adiabat, 2000: each spans 1/2000 of the step, so that the
/// one that crosses the triple point errs by under a thousandth of the
/// jump a whole step makes.
const CO2_STEP_SUBSTEPS: usize = 2000;

/// Calibration: how far, K, one step of CO₂'s pseudo-adiabat across its
/// triple point may land from the same step cut into
/// [`CO2_STEP_SUBSTEPS`], 0.01 K. It bounds the model's own numerics: no
/// published tolerance on the step of a pseudo-adiabat was found in the
/// sources read (Kasting 1991 and Kopparapu et al. 2013 give the slope,
/// not its integration; no dedicated search was made), so the bound is
/// placed between two measured ends: split at the triple point, a step of a
/// twentieth of a decade from 216.8 to 218 K misses by 1.0e-3 to
/// 3.4e-3 K (3.5e-4 K for a step that does not cross it, at 222 and
/// 226 K); taken over the phase of each of its points, as before the
/// split, by 0.08 to 0.45 K. 0.01 K sits a factor 3 above the first and
/// 8 under the second.
const CO2_STEP_TOLERANCE_K: f64 = 0.01;

#[test]
fn a_co2_step_across_its_triple_point_takes_each_phase_on_its_part() {
    // One step of CO₂'s pseudo-adiabat that crosses its triple point
    // agrees with the same step cut into many, each over the condensate
    // stable where it starts: liquid down to 216.592 K, frost below.
    // Before the split, the step took the phase of each point it read
    // for its whole span, and missed by up to 0.45 K.
    let air = DryAir::of(&CO2_RICH_AIR, CO2_RICH_SKIN.0);
    let h = std::f64::consts::LN_10 / 20.0;
    for start in [216.8, 217.2, 217.6, 218.0] {
        let p = co2_saturation_bar(start) / CO2_RICH_AIR.co2 * CO2_RICH_AIR.total();
        let one = co2_step(&air, start, p, h);
        let (mut t, mut from) = (start, p);
        let small = h / CO2_STEP_SUBSTEPS as f64;
        for _ in 0..CO2_STEP_SUBSTEPS {
            t = co2_step(&air, t, from, small);
            from *= (-small).exp();
        }
        println!("{start} K: one step {one:.5} K, {CO2_STEP_SUBSTEPS} steps {t:.5} K");
        assert!(
            one < CO2_TRIPLE_TEMPERATURE,
            "{start} K: the step ends at {one} K"
        );
        assert!(
            (one - t).abs() < CO2_STEP_TOLERANCE_K,
            "{start} K: {one} against {t}"
        );
    }
}
