//! Tests of the radiative balance, ported from `legacy-final`'s
//! `planet/climate.rs` tests (l.3339 `more_greenhouse_gas_is_warmer`,
//! l.3996 the runaway albedo, l.4032 and l.2739 the steam blend), with
//! the dark body of the plan's Review Focus 1 and the root finders.

use super::*;
use crate::constants::SOLAR_EFFECTIVE_TEMPERATURE;
use crate::orbit::EARTH_EQUILIBRIUM;
use crate::radiation::test_points::{
    EARTH_SURFACE_K, TITAN_EFFECTIVE_K, TITAN_GRAVITY, TITAN_SURFACE_AIR, TITAN_SURFACE_K,
};

/// Definition: the thin air of the greenhouse test, bar: 1 of N₂ and
/// 1e-3 of CO₂ (legacy l.3341).
const THIN_AIR: [f64; 2] = [1.0, 1e-3];

/// Definition: the CO₂, bar, of the greenhouse test's thick air.
const THICK_CO2_BAR: f64 = 1.0;

/// Definition: the internal heat, W/m², that warms the airless body of
/// the greenhouse test (legacy l.3356).
const WARM_HEAT: WattPerSquareMetre = WattPerSquareMetre(100.0);

/// Definition: an airless body against its grey-body temperature, K, the
/// rounding of a fourth root.
const AIRLESS_TOLERANCE_K: f64 = 1e-9;

/// Definition: the starlight, W/m², of a body with almost no light
/// (Review Focus 1 of the plan).
const DARK_STARLIGHT: WattPerSquareMetre = WattPerSquareMetre(1e-9);

/// Definition: the internal heat, W/m², of the dark body (Review Focus 1
/// of the plan).
const DARK_HEAT: WattPerSquareMetre = WattPerSquareMetre(0.05);

/// Definition: the nitrogen, bar, of the dark body's air and of the
/// convergence test's.
const DARK_N2_BAR: f64 = 1.0;

/// Definition: the tolerance of the root finder's test.
const ROOT_TOLERANCE: f64 = 1e-12;

/// Definition: the bracket of the cube root of 2, (1, 2).
const CUBE_BRACKET: (f64, f64) = (1.0, 2.0);

/// Definition: the absorbed flux, W/m², whose black-body crossing
/// [`bracket_up`] brackets, and the temperature, K, it starts from.
const BRACKET_CASE: (f64, f64) = (240.0, 100.0);

/// Source: the effective temperatures, K, that bound Kopparapu et al.
/// 2014's fits ("2600 K ≤ Teff ≤ 7200 K"), the stars the runaway albedo
/// is checked at with the Sun's.
const FIT_ENDS_K: (f64, f64) = (2600.0, 7200.0);

/// Definition: the relative tolerance of the plateau absorbed at the
/// limit, the rounding of the products.
const BIND_TOLERANCE: f64 = 1e-9;

/// Definition: the stars of the runaway albedo's sweep, K, every 100 K
/// over the fit.
const SWEEP_STEP_K: f64 = 100.0;

/// Definition: the steams, bar, the blend's test rises through.
const STEAMS_BAR: [f64; 7] = [1e-9, 1e-3, 0.1, 1.0, 3.0, 10.0, 1e4];

/// Definition: the derived albedo the steam blend starts from, an
/// Earth-like 0.3 (a test input).
const DERIVED_ALBEDO: f64 = 0.3;

/// Definition: a trace of steam, bar (legacy l.4060: one kilogram's).
const TRACE_STEAM: Bar = Bar(1e-9);

/// Definition: how far the albedo under a trace of steam may be from the
/// derived one: the trace's share, 1e-9, rounded up.
const TRACE_TOLERANCE: f64 = 1e-8;

/// Source: the uncertainty, K, of Titan's surface temperature, +-0.25 K
/// (Fulchignoni et al. 2005, *Nature* 438, 785,
/// doi:10.1038/nature04314).
const TITAN_SURFACE_TOLERANCE_K: f64 = 0.25;

/// Calibration: how far under Titan's measured surface (less its
/// uncertainty) the column may settle, K, for the warming it does not
/// hold: 1.9. Its greenhouse is HITRAN's N₂–N₂, N₂–CH₄ and CH₄–CH₄ pairs
/// (study 36, `docs/science/longwave-absorbers.md`), which settled the
/// sourced column at 92.30 K, 1.35 K under the measured 93.65 K, with no
/// fitted constant; the user accepted that on 2026-11-25 (the bound was
/// then 1.4 K), retiring the grey stand-in fitted on Titan
/// (`CH4_OPACITY`, `CH4_SATURATION_BAR`). Since the column condenses N₂
/// where its air saturates (issue #741), its dry adiabat saturates from
/// about 0.65 bar up to the skin, and the column settles at 91.809 K,
/// 1.841 K under the measurement: the user kept the condensation and
/// refitted the bound by its rule on 2026-11-27 (study 39,
/// `docs/science/nitrogen-phases.md`, "Titan's column against Huygens").
/// The column misses N₂–H₂ pairs (Titan's H₂ is not a gas of the model,
/// issue #720), methane's own lines (its rotational lines at 70 to
/// 160 cm⁻¹, Bézard & Vinatier 2019), and the haze's infrared opacity;
/// and it holds too little methane low down: it applies the cold trap's
/// mixing ratio (`radiating_methane_bar`, `column.rs`) at every level, so
/// its lower troposphere holds about 40 % of the sourced surface CH₄ (a
/// review's estimate, derived, issue #744). N₂–CH₄ absorption is linear
/// in CH₄, so that is likely a large share of the gap. Each adds
/// absorption, which can only warm a column whose temperature falls with
/// height. The gap is not those absorbers alone: the column's
/// troposphere is 4 to 8 K colder than Huygens's profile between 0.3 and
/// 0.9 bar (no radiative upper troposphere, no methane latent heat in
/// its adiabat, a skin 1.4 K under the measured tropopause; issue #750),
/// and Huygens's profile, shifted on the same column until it balances,
/// settles 3.1 to 4.2 K under the measured 93.65 K (2.9 to 4.0 K under
/// the profile's own lowest point, 93.50 K; study 39): against the
/// column's 1.841 K, the cold troposphere hides 1.2 to 2.3 K of the
/// missing warming, and would cool the column once fixed. So the test is
/// one-sided only for the missing absorbers (#720, #744, the haze): no
/// warmer than the measurement, no colder than 93.65 − 0.25 − 1.9 =
/// 91.50 K, a margin of 0.31 K under the column's 91.809 K. The bound is
/// the measured gap rounded up, and is re-fitted by the same rule (not
/// only re-checked) when #750 lands, since that fix moves the column
/// colder. What was searched and not found: a split of Titan's
/// greenhouse that gives each missing term in kelvin (study 36, "What we
/// do not know").
const TITAN_MISSING_WARMING_K: f64 = 1.9;

/// Definition: the tolerance of `steam_share(1 bar)` against `1 − e⁻¹`,
/// the rounding of an exponential.
const SHARE_TOLERANCE: f64 = 1e-12;

/// The starlight, W/m², a body of equilibrium temperature `t` absorbs.
fn starlight_of(t: Kelvin) -> WattPerSquareMetre {
    WattPerSquareMetre(STEFAN_BOLTZMANN * t.0.powi(4))
}

/// Earth's starlight, that of a body at [`EARTH_EQUILIBRIUM`].
fn earth_light() -> WattPerSquareMetre {
    starlight_of(EARTH_EQUILIBRIUM)
}

/// A black body's emissivity.
fn black() -> Fraction {
    Fraction::new(1.0).expect("one is a fraction")
}

/// [`surface_temperature`] on Earth's gravity with no heat, a black body,
/// the default solver and a fresh cache.
fn earth_surface(gases: &PartialPressures, heat: WattPerSquareMetre) -> f64 {
    surface_temperature(
        earth_light(),
        gases,
        heat,
        black(),
        STANDARD_GRAVITY,
        SOLVER,
        &mut ColumnCache::default(),
    )
    .0
}

#[test]
fn more_greenhouse_gas_is_warmer() {
    let [n2, co2] = THIN_AIR;
    let thin = PartialPressures {
        n2,
        co2,
        ..PartialPressures::default()
    };
    let thick = PartialPressures {
        co2: THICK_CO2_BAR,
        ..thin
    };
    let none = WattPerSquareMetre(0.0);
    let (t_thin, t_thick) = (earth_surface(&thin, none), earth_surface(&thick, none));
    println!("thin air {t_thin:.2} K, thick {t_thick:.2} K");
    assert!(
        t_thin > EARTH_EQUILIBRIUM.0 && t_thick > t_thin,
        "{t_thin} {t_thick}"
    );
    let bare = earth_surface(&PartialPressures::default(), none);
    assert!(
        (bare - EARTH_EQUILIBRIUM.0).abs() < AIRLESS_TOLERANCE_K,
        "{bare}"
    );
    // Internal heat warms an airless surface.
    assert!(earth_surface(&PartialPressures::default(), WARM_HEAT) > bare);
}

#[test]
fn a_dark_body_is_warmed_by_its_heat_alone() {
    // Review Focus 1: almost no light, a little internal heat. Without air
    // and under nitrogen the surface sits at least where the heat alone
    // puts a black body.
    let floor = (DARK_HEAT.0 / STEFAN_BOLTZMANN).powf(0.25);
    let nitrogen = PartialPressures {
        n2: DARK_N2_BAR,
        ..PartialPressures::default()
    };
    for gases in [PartialPressures::default(), nitrogen] {
        let t = surface_temperature(
            DARK_STARLIGHT,
            &gases,
            DARK_HEAT,
            black(),
            STANDARD_GRAVITY,
            SOLVER,
            &mut ColumnCache::default(),
        )
        .0;
        println!("{gases:?}: {t} K against {floor} K");
        assert!(t.is_finite() && t >= floor, "{gases:?}: {t} vs {floor}");
    }
}

#[test]
fn regula_falsi_finds_a_crossing() {
    let cube = |x: f64| x.powi(3) - 2.0;
    let (a, b) = CUBE_BRACKET;
    let root = regula_falsi(
        cube,
        Point { x: a, f: cube(a) },
        Point { x: b, f: cube(b) },
        ROOT_TOLERANCE,
        ITERATIONS,
    );
    let expected = 2.0_f64.cbrt();
    println!("{root:?} against {expected}");
    assert!(root.iterations < ITERATIONS, "{root:?}");
    assert!((root.x - expected).abs() < ROOT_TOLERANCE, "{root:?}");
    assert!(cube(root.x) >= 0.0, "{root:?}");

    let (flux, start) = BRACKET_CASE;
    let excess = |t: f64| STEFAN_BOLTZMANN * t.powi(4) - flux;
    let crossing = (flux / STEFAN_BOLTZMANN).powf(0.25);
    let (low, high) = bracket_up(
        excess,
        flux,
        Point {
            x: start,
            f: excess(start),
        },
    );
    println!("{low:?} {high:?} around {crossing} K");
    assert!(low.f < 0.0 && high.f >= 0.0, "{low:?} {high:?}");
    assert!(low.x < crossing && crossing <= high.x, "{low:?} {high:?}");
}

#[test]
fn iterations_is_converged() {
    // `ITERATIONS` is a cap: the reference solves stop by their tolerance
    // under it. Earth's air with its vapour, Titan's column at its
    // gravity, and nitrogen under a dark starlight.
    let earth = PartialPressures {
        h2o: surface_vapour(Kelvin(EARTH_SURFACE_K), Bar(f64::INFINITY)).0,
        ..EARTH_DRY_AIR
    };
    let dark = PartialPressures {
        n2: DARK_N2_BAR,
        ..PartialPressures::default()
    };
    let none = WattPerSquareMetre(0.0);
    for (name, light, gases, heat, gravity) in [
        ("Earth", earth_light(), earth, none, STANDARD_GRAVITY),
        (
            "Titan",
            starlight_of(Kelvin(TITAN_EFFECTIVE_K)),
            TITAN_SURFACE_AIR,
            none,
            TITAN_GRAVITY,
        ),
        ("dark", DARK_STARLIGHT, dark, DARK_HEAT, STANDARD_GRAVITY),
    ] {
        let root = surface_root(
            light,
            &gases,
            heat,
            black(),
            gravity,
            SOLVER,
            &mut ColumnCache::default(),
        );
        println!("{name}: {:.3} K in {} steps", root.x, root.iterations);
        assert!(root.x.is_finite(), "{name}: {root:?}");
        assert!(root.iterations < ITERATIONS, "{name}: {root:?}");
    }
}

#[test]
fn titans_pairs_hold_its_column_just_under_its_measured_surface() {
    // Issue #646, study 36: the sourced column of `test_points` (1.467
    // bar, 5.65 % of methane, an effective temperature of 82 K, Titan's
    // gravity, no heat, a black surface) under the anti-greenhouse of its
    // stratosphere, warmed by its nitrogen and methane pairs alone, sits
    // at most `TITAN_MISSING_WARMING_K` under Titan's measured surface:
    // what it lacks (N₂–H₂ pairs, #720; methane's lines; the haze's
    // infrared) would warm it. Its troposphere is too cold aloft (#750),
    // which hides 1.2 to 2.3 K more of the missing warming: fixing it
    // cools the column, and the bound is re-fitted then.
    let t = surface_temperature(
        starlight_of(Kelvin(TITAN_EFFECTIVE_K)),
        &TITAN_SURFACE_AIR,
        WattPerSquareMetre(0.0),
        black(),
        TITAN_GRAVITY,
        SOLVER,
        &mut ColumnCache::default(),
    )
    .0;
    println!("Titan's column: {t:.3} K against {TITAN_SURFACE_K} K");
    let coldest = TITAN_SURFACE_K - TITAN_SURFACE_TOLERANCE_K - TITAN_MISSING_WARMING_K;
    let warmest = TITAN_SURFACE_K + TITAN_SURFACE_TOLERANCE_K;
    assert!(
        (coldest..=warmest).contains(&t),
        "{t} K against {coldest} to {warmest} K"
    );
}

#[test]
fn the_runaway_albedo_binds_the_plateau_at_the_limit() {
    let plateau = earth_moist_plateau();
    let (cool, hot) = FIT_ENDS_K;
    for star in [cool, SOLAR_EFFECTIVE_TEMPERATURE.0, hot] {
        let albedo = runaway_albedo(Kelvin(star), plateau).get();
        let limit = WattPerSquareMetre::from(runaway_flux(Kelvin(star))).0;
        let taken = (1.0 - albedo) * limit / 4.0;
        println!("{star} K: A_rg {albedo:.4}, absorbs {taken:.3} W/m²");
        assert!(
            (taken / plateau.0 - 1.0).abs() < BIND_TOLERANCE,
            "{star} K: {taken} vs {}",
            plateau.0
        );
    }
    // A_rg stays inside its guard over the fit's range.
    let (least, greatest) = RUNAWAY_ALBEDO;
    let mut star = cool;
    while star <= hot {
        let albedo = runaway_albedo(Kelvin(star), plateau).get();
        assert!(albedo > least && albedo < greatest, "{star} K: {albedo}");
        star += SWEEP_STEP_K;
    }
}

#[test]
fn a_runaways_light_follows_its_steam_down_to_none() {
    assert_eq!(steam_share(Bar(0.0)).get(), 0.0);
    let one = steam_share(STEAM_ALBEDO_BAR).get();
    assert!(
        (one - (1.0 - (-1.0_f64).exp())).abs() < SHARE_TOLERANCE,
        "{one}"
    );
    let mut last = 0.0;
    for steam in STEAMS_BAR {
        let share = steam_share(Bar(steam)).get();
        assert!(share > last, "{steam} bar: {share} after {last}");
        last = share;
    }
    let moist = runaway_albedo(SOLAR_EFFECTIVE_TEMPERATURE, earth_moist_plateau()).get();
    let blend = DERIVED_ALBEDO + (moist - DERIVED_ALBEDO) * steam_share(TRACE_STEAM).get();
    assert!((blend - DERIVED_ALBEDO).abs() < TRACE_TOLERANCE, "{blend}");
}

/// Source: how much Earth's argon may warm it, 0.2 K: study 29's bound,
/// over its +0.07 to +0.11 K scaled from Goldblatt et al. 2009's doubled
/// N₂ (argon, 0.0127 bar, broadens less than N₂). An upper bound only:
/// that window is for a wet climate, and this dry column, without the
/// water's feedback, is about 12 times less sensitive (+0.005 K for the
/// argon, +0.37 K for doubled N₂; the wet Earth at 280 ppm gives +0.125 K
/// for its argon, `docs/science/planet-atmospheres.md`, "Argon").
const EARTH_ARGON_WARMING_K: f64 = 0.2;

/// Source: Earth's argon as a weight, 0.0127 bar (study 29, derived from
/// the NASA Earth fact sheet's 9340 ppm and Trenberth & Smith 2005).
const EARTH_ARGON_BAR: f64 = 0.0127;

#[test]
fn earths_argon_warms_its_column_little() {
    // Earth's dry air under Earth's light, with and without its argon:
    // the argon weighs, broadens as air and steepens the dry slope.
    let without = earth_surface(&EARTH_DRY_AIR, WattPerSquareMetre(0.0));
    let with = earth_surface(
        &PartialPressures {
            ar: EARTH_ARGON_BAR,
            ..EARTH_DRY_AIR
        },
        WattPerSquareMetre(0.0),
    );
    let warming = with - without;
    println!("Earth's dry column: {without:.3} K, with its argon {with:.3} K ({warming:+.3} K)");
    // An upper bound only (see `EARTH_ARGON_WARMING_K`).
    assert!(warming < EARTH_ARGON_WARMING_K, "{warming}");
}

/// Source: Earth's surface pressure, 1.014 bar (NASA Earth fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html).
const EARTH_PRESSURE_BAR: f64 = 1.014;

/// Source: Earth's dry air by volume, 78.08 % N₂ and 20.95 % O₂ (NASA
/// Earth fact sheet), and its preindustrial CO₂, 280 ppm, 2.8e-4 bar
/// (Myhre et al. 1998, doi:10.1029/98GL01908): Earth's measured air at
/// 280 ppm, without its argon (the checks add it), bar.
const EARTH_MEASURED_AIR_NO_AR: PartialPressures = PartialPressures {
    n2: EARTH_PRESSURE_BAR * 0.7808,
    co2: 2.8e-4,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: EARTH_PRESSURE_BAR * 0.2095,
    ar: 0.0,
};

/// Definition: the solver of the wet checks, [`SOLVER`] narrowed to
/// 1e-5 K, so that its width is far under the hundredths of a kelvin the
/// checks compare.
const FINE_SOLVER: Solver = Solver {
    tolerance: Kelvin(1e-5),
    ..SOLVER
};

/// The surface temperature, K, of a wet Earth under `dry` air: the
/// column's balance with its vapour following the surface
/// ([`surface_vapour`] with all the water it can hold), on Earth's
/// starlight ([`earth_light`]), a black surface and the standard gravity,
/// solved from the grey body up by [`bracket_up`] and [`regula_falsi`]
/// with [`FINE_SOLVER`], on a fresh cache. These are fixed inputs, not
/// the settled reference Earth of `system` (its albedo, emissivity and
/// clouds).
fn wet_earth(dry: &PartialPressures) -> f64 {
    let light = earth_light();
    let absorbed = absorbed(light, dry, WattPerSquareMetre(0.0)).0;
    let sky = skin(light);
    let mut cache = ColumnCache::default();
    let mut excess = |t: f64| {
        let gases = PartialPressures {
            h2o: surface_vapour(Kelvin(t), Bar(f64::INFINITY)).0,
            ..*dry
        };
        outgoing_longwave(
            &gases,
            Kelvin(t),
            sky,
            STANDARD_GRAVITY,
            Profile::CLIMATE,
            &mut cache,
        )
        .0 - absorbed
    };
    let bare = (absorbed / STEFAN_BOLTZMANN).powf(0.25);
    let start = Point {
        x: bare,
        f: excess(bare),
    };
    let (low, high) = bracket_up(&mut excess, absorbed, start);
    regula_falsi(
        excess,
        low,
        high,
        FINE_SOLVER.tolerance.0,
        FINE_SOLVER.iterations,
    )
    .x
}

/// The warming, K, of the wet Earth when its air gains `added`, with
/// argon's molar heat capacity `argon_cp` in the column.
fn warming(added: PartialPressures, argon_cp: f64) -> f64 {
    use crate::radiation::column::AR_HEAT_CAPACITY_OVERRIDE;
    AR_HEAT_CAPACITY_OVERRIDE.with(|c| c.set(Some(argon_cp)));
    let base = wet_earth(&EARTH_MEASURED_AIR_NO_AR);
    let more = wet_earth(&PartialPressures {
        n2: EARTH_MEASURED_AIR_NO_AR.n2 + added.n2,
        ar: EARTH_MEASURED_AIR_NO_AR.ar + added.ar,
        ..EARTH_MEASURED_AIR_NO_AR
    });
    AR_HEAT_CAPACITY_OVERRIDE.with(|c| c.set(None));
    more - base
}

/// On a wet Earth at 280 ppm (fixed inputs, see [`wet_earth`]), Earth's
/// 0.0127 bar of argon against as much N₂, with two checks that keep
/// apart what each part of the column does with the argon:
///
/// 1. With argon given N₂'s heat capacity, only the radiation tells the
///    two apart, and argon warms, at most as much as N₂:
///    `0 < ΔT(Ar) <= ΔT(N₂)`. Source: argon broadens CO₂'s and water's
///    lines at most as N₂ does (0.78 of N₂ for CO₂, Reichle 1969; about
///    0.37 of air for water, Claveau & Valentin 2009; study 29). The
///    column counts it as air (a departure), so equality is allowed.
/// 2. With argon's own heat capacity, it warms more than with N₂'s.
///    Definition: the dry adiabatic lapse is `g / c_p` per unit mass
///    (`R / c_p` in `d ln T / d ln p` per mole), and argon's `c_p` is
///    below N₂'s, 20.786 against 29.12 J/(mol·K), 520 against 1040
///    J/(kg·K) (Chase 1998 via the NIST WebBook): the steeper slope warms
///    the surface under the same emission.
#[test]
fn argon_warms_as_air_broadens_and_its_slope_steepens() {
    let argon = PartialPressures {
        ar: EARTH_ARGON_BAR,
        ..PartialPressures::default()
    };
    let nitrogen = PartialPressures {
        n2: EARTH_ARGON_BAR,
        ..PartialPressures::default()
    };
    let n2_cp = crate::radiation::gases::N2_HEAT_CAPACITY;
    let ar_cp = crate::radiation::gases::AR_HEAT_CAPACITY;
    let with_n2 = warming(nitrogen, ar_cp);
    let as_n2 = warming(argon, n2_cp);
    let own = warming(argon, ar_cp);
    println!(
        "wet Earth at 280 ppm, +{EARTH_ARGON_BAR} bar: N₂ {with_n2:+.4} K, \
         argon with N₂'s cp {as_n2:+.4} K, argon {own:+.4} K"
    );
    assert!(as_n2 > 0.0 && as_n2 <= with_n2, "{as_n2} against {with_n2}");
    assert!(own > as_n2, "{own} against {as_n2}");
}
