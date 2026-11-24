//! Tier 1: the settle of the albedo and emissivity, and the condensation
//! fixed point inside it, on Mars, a dry Earth, a hot rock and a
//! Triton-like body; the convergence of their numerical constants. The
//! cold methane world of #443 is tier 3 (about 13 s at opt-level 0).

use super::body::ClimateBody;
use super::fixture::{earth, mars, moved, titan};
use super::settle::{Settle, settle, warm_start};
use super::{
    ALBEDO_ITERATIONS, ALBEDO_TOLERANCE_K, CONDENSE_ITERATIONS, ClimateInputs, Light, Numerics,
};
use science::balance::{BRACKET_TOLERANCE, TOLERANCE};
use science::radiation::gases::PartialPressures;
use science::surface::Liquid;
use science::units::{Bar, EarthFlux, Kelvin, Kilogram, WattPerSquareMetre};

/// Source: Mars's air, 6.36 mbar of 95.1 % CO₂, 2.59 % N₂ and 0.16 % O₂
/// (NASA Mars fact sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/
/// marsfact.html): 6.05 mbar of CO₂, 0.165 mbar of N₂ and 0.010 mbar of
/// O₂, imposed (study 25, "Test air and Solar System bounds"; argon has
/// no field).
const MARS_AIR: PartialPressures = PartialPressures {
    n2: 0.165e-3,
    co2: 6.05e-3,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.010e-3,
};

/// Source: Mars's average surface temperature, ~214 K (NASA Mars fact
/// sheet).
const MARS_SURFACE: Kelvin = Kelvin(214.0);

/// Source: the window on Mars's temperature, 5 K (study 25, "Test air
/// and Solar System bounds": 214 ± 5 K, decided with the user on
/// 2026-11-24, over the fact sheet's "~").
const MARS_WINDOW: Kelvin = Kelvin(5.0);

/// Source: Mars's Bond albedo, 0.250 (NASA Mars fact sheet).
const MARS_BOND: f64 = 0.250;

/// Definition: the window on Mars's Bond albedo, 0.05, a test tolerance
/// for a model whose ground is one regolith.
const MARS_BOND_WINDOW: f64 = 0.05;

/// Definition: the relative agreement of one sun and two half suns,
/// 1e-9, a few roundings through the fixed point (Review Focus 2).
const SAME_LIGHT: f64 = 1e-9;

/// Definition: the census case of issue #443 (seed 182's planet d,
/// legacy `cold_methane_world`, `climate.rs` l.2776-2808): its F star's
/// effective temperature, K, a test input.
const COLD_STAR: Kelvin = Kelvin(6710.0);

/// Definition: the census case of #443: its flux, S⊕, at 10.5 au, a test
/// input.
const COLD_FLUX: EarthFlux = EarthFlux(0.0401);

/// Definition: the census case of #443: its equilibrium temperature at
/// the reference albedo, K, a test input.
const COLD_EQUILIBRIUM: Kelvin = Kelvin(114.07);

/// Definition: the census case of #443: its air, 4.379 bar of N₂,
/// 26.26 bar of CH₄, 1.14e-5 bar of CO₂ and 2.4e-4 bar of SO₂, a test
/// input.
const COLD_AIR: PartialPressures = PartialPressures {
    n2: 4.379,
    co2: 1.14e-5,
    h2o: 0.0,
    ch4: 26.26,
    nh3: 0.0,
    so2: 2.4e-4,
    o2: 0.0,
};

/// Definition: the census case of #443: the water it keeps, kg, a test
/// input.
const COLD_WATER: Kilogram = Kilogram(1.29e23);

/// Definition: the census case of #443: its internal heat, W/m², a test
/// input.
const COLD_HEAT: WattPerSquareMetre = WattPerSquareMetre(0.0283);

/// Definition: the bound of legacy's `a_methane_greenhouse_cannot_warm_a_
/// cold_world` (`climate.rs` l.2810), 150 K: a methane greenhouse
/// saturates, so a world of 114 K stays far under it.
const COLD_BOUND: Kelvin = Kelvin(150.0);

/// Definition: the orbit of the hot rock, 0.035 au, a test input where a
/// bare Earth under a regolith's albedo sits near 1445 K, under
/// `LAVA_TEMPERATURE` (1673 K), and its substellar point, √2 hotter,
/// over it.
const HOT_ORBIT: f64 = 0.035;

/// Definition: a bare rock's temperature at [`HOT_ORBIT`], 1445 K,
/// worked by hand from `(S₀ (1 − 0.11) / (4 · 0.035²) / σ)^¼` with
/// `REGOLITH_ALBEDO` 0.11.
const HOT_ROCK: Kelvin = Kelvin(1445.0);

/// Definition: the window on [`HOT_ROCK`], 5 K: Earth's orbit-mean flux
/// and internal heat move it by under 1 K.
const HOT_WINDOW: Kelvin = Kelvin(5.0);

/// Source: Triton's orbit-mean solar flux, 1.508 W/m² (NASA Neptune fact
/// sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/neptunefact.html), as
/// `science::surface`'s cover tests take it.
const TRITON_FLUX: WattPerSquareMetre = WattPerSquareMetre(1.508);

/// Definition: a Triton-like inventory of nitrogen, 10 bar, far more than
/// its frost keeps in the air (as `science::surface`'s condensation
/// tests).
const TRITON_N2: Bar = Bar(10.0);

/// Source: Triton's surface temperature, 37 to 39 K (38 ± 1 K, Tryka et
/// al. 1993, *Science* 261, 751, and 1994, *Icarus* 112, 513).
pub(super) const TRITON_RANGE: (Kelvin, Kelvin) = (Kelvin(37.0), Kelvin(39.0));

/// Definition: the finer tolerances of the convergence tests, a tenth of
/// [`TOLERANCE`] and of [`BRACKET_TOLERANCE`] and [`ALBEDO_TOLERANCE_K`]
/// at 0.01 K (study 25, "Solver constants").
const TENTH: f64 = 0.1;

/// Definition: the finer albedo tolerance, 0.01 K (study 25, "Solver
/// constants").
const FINER_ALBEDO: Kelvin = Kelvin(0.01);

/// The climate of `inputs` settled from its warm start with `numerics`.
pub(super) fn settled(inputs: &ClimateInputs, numerics: Numerics) -> Settle {
    let mut body = ClimateBody::new(inputs, numerics);
    let start = warm_start(&mut body);
    settle(&mut body, start)
}

/// Mars's inputs under the imposed air of [`MARS_AIR`].
pub(super) fn imposed_mars() -> ClimateInputs {
    ClimateInputs {
        air: MARS_AIR,
        ..mars()
    }
}

/// A Triton-like body: Earth's inputs under [`TRITON_FLUX`] from one Sun,
/// its equilibrium temperature scaled as `flux^¼` (a `Definition:`), dry,
/// with an air of [`TRITON_N2`] of N₂ alone, no carbon and no internal
/// heat (Triton's, a few mW/m², would not move a frost balanced on
/// 0.1 W/m² of light; none keeps the case on its light alone).
pub(super) fn triton_like() -> ClimateInputs {
    let earth = earth();
    let sun = earth.sources[0];
    let flux = EarthFlux::from(TRITON_FLUX);
    ClimateInputs {
        sources: vec![Light {
            flux,
            equilibrium: Kelvin(sun.equilibrium.0 * (flux.0 / sun.flux.0).powf(0.25)),
            ..sun
        }],
        air: PartialPressures {
            n2: TRITON_N2.0,
            ..PartialPressures::default()
        },
        water: Kilogram(0.0),
        heat_flux: WattPerSquareMetre(0.0),
        carbon: Bar(0.0),
        frozen_co2: Bar(0.0),
        ..earth
    }
}

/// Whether `a` and `b` agree within `relative` of `a`.
fn close(a: f64, b: f64, relative: f64) -> bool {
    (a - b).abs() <= relative * a.abs()
}

#[test]
fn mars_sits_at_its_measured_temperature() {
    let c = settled(&imposed_mars(), Numerics::DEFAULT);
    assert!(c.converged, "{c:?}");
    assert!(
        (c.temperature.0 - MARS_SURFACE.0).abs() <= MARS_WINDOW.0,
        "{c:?}"
    );
    assert_eq!(c.liquid, None, "{c:?}");
    assert!(
        (c.bond_albedo.get() - MARS_BOND).abs() <= MARS_BOND_WINDOW,
        "{c:?}"
    );
    assert!(!c.regulated, "{c:?}");
}

#[test]
fn two_half_suns_are_one_sun() {
    let one = imposed_mars();
    let sun = one.sources[0];
    let half = Light {
        flux: EarthFlux(sun.flux.0 / 2.0),
        // Definition: half the flux, `2^-¼` of the equilibrium
        // temperature.
        equilibrium: Kelvin(sun.equilibrium.0 * 0.5f64.powf(0.25)),
        ..sun
    };
    let two = ClimateInputs {
        sources: vec![half, half],
        ..one.clone()
    };
    let (a, b) = (
        settled(&one, Numerics::DEFAULT),
        settled(&two, Numerics::DEFAULT),
    );
    assert!(
        close(a.temperature.0, b.temperature.0, SAME_LIGHT),
        "{a:?} {b:?}"
    );
    assert!(
        close(a.equilibrium.0, b.equilibrium.0, SAME_LIGHT),
        "{a:?} {b:?}"
    );
    assert!(
        close(a.bond_albedo.get(), b.bond_albedo.get(), SAME_LIGHT),
        "{a:?} {b:?}"
    );
}

#[test]
fn no_water_no_ocean() {
    let dry = ClimateInputs {
        water: Kilogram(0.0),
        ..earth()
    };
    let c = settled(&dry, Numerics::DEFAULT);
    assert_ne!(c.liquid, Some(Liquid::Water), "{c:?}");
    assert!(!c.regulated, "{c:?}");
}

/// The census case of #443 (legacy `cold_methane_world`): a cold, wet
/// body round an F star, under a thick nitrogen and methane air.
pub(super) fn cold_methane_world() -> ClimateInputs {
    ClimateInputs {
        sources: vec![Light {
            flux: COLD_FLUX,
            effective_temperature: COLD_STAR,
            equilibrium: COLD_EQUILIBRIUM,
        }],
        air: COLD_AIR,
        water: COLD_WATER,
        runaway: false,
        plates: false,
        heat_flux: COLD_HEAT,
        frozen_co2: Bar(0.0),
        ..earth()
    }
}

#[test]
#[ignore = "heavy: the cold methane world of #443 settled, about 13 s at opt-level 0"]
fn a_methane_greenhouse_cannot_warm_a_cold_world() {
    let cold = cold_methane_world();
    let c = settled(&cold, Numerics::DEFAULT);
    assert!(c.temperature.0 < COLD_BOUND.0, "{c:?}");
    assert_ne!(c.liquid, Some(Liquid::Water), "{c:?}");
    // Most of its methane condenses on the ground.
    assert!(c.air.ch4 < 0.5 * COLD_AIR.ch4, "{c:?}");
    assert!(c.condensate.ch4 > 0.0, "{c:?}");
}

#[test]
fn a_hot_locked_earth_has_lava() {
    let bare = ClimateInputs {
        air: PartialPressures::default(),
        water: Kilogram(0.0),
        ..moved(&earth(), HOT_ORBIT)
    };
    let spinning = settled(&bare, Numerics::DEFAULT);
    assert_eq!(spinning.liquid, None, "{spinning:?}");
    assert!(
        (spinning.temperature.0 - HOT_ROCK.0).abs() <= HOT_WINDOW.0,
        "{spinning:?}"
    );
    let locked = ClimateInputs {
        locked: true,
        ..bare
    };
    let c = settled(&locked, Numerics::DEFAULT);
    assert_eq!(c.liquid, Some(Liquid::Lava), "{c:?}");
}

#[test]
fn a_triton_like_body_settles() {
    let c = settled(&triton_like(), Numerics::DEFAULT);
    assert!(c.converged, "{c:?}");
    let (low, high) = TRITON_RANGE;
    assert!(
        c.temperature.0 >= low.0 && c.temperature.0 <= high.0,
        "{c:?}"
    );
    assert!(c.condensate.n2 > 0.0, "{c:?}");
    assert!(c.emissivity.get() < 1.0, "{c:?}");
}

/// The moves of `inputs`'s temperature under the finer tolerances of
/// study 25 ("Solver constants"), and the check that doubling the caps
/// changes nothing; the climate settled with the solver's constants.
fn assert_converged(inputs: &ClimateInputs) -> Settle {
    let base = settled(inputs, Numerics::DEFAULT);
    assert!(base.converged, "{base:?}");
    let d = Numerics::DEFAULT;
    let finer = Numerics {
        solver: science::balance::Solver {
            tolerance: Kelvin(TOLERANCE.0 * TENTH),
            ..d.solver
        },
        ..d
    };
    let t = settled(inputs, finer).temperature;
    assert!(
        (t.0 - base.temperature.0).abs() < TOLERANCE.0,
        "{t:?} {base:?}"
    );
    let albedo = Numerics {
        albedo_tolerance: FINER_ALBEDO,
        ..d
    };
    let t = settled(inputs, albedo).temperature;
    assert!(
        (t.0 - base.temperature.0).abs() < ALBEDO_TOLERANCE_K.0,
        "{t:?} {base:?}"
    );
    let bracket = Numerics {
        solver: science::balance::Solver {
            bracket_tolerance: Kelvin(BRACKET_TOLERANCE.0 * TENTH),
            ..d.solver
        },
        ..d
    };
    let t = settled(inputs, bracket).temperature;
    assert!(
        (t.0 - base.temperature.0).abs() < TOLERANCE.0,
        "{t:?} {base:?}"
    );
    let doubled = Numerics {
        albedo_iterations: 2 * ALBEDO_ITERATIONS,
        condense_iterations: 2 * CONDENSE_ITERATIONS,
        ..d
    };
    assert_eq!(settled(inputs, doubled), base);
    base
}

#[test]
fn the_fixed_points_are_converged() {
    assert_converged(&imposed_mars());
    // The condensation loop of a Triton-like body runs and stops by its
    // tolerance, under its cap (study 25, "Solver constants").
    let triton = assert_converged(&triton_like());
    assert!(
        triton.iterations > 0 && triton.iterations < CONDENSE_ITERATIONS,
        "{triton:?}"
    );
    let titan = settled(&titan(), Numerics::DEFAULT);
    assert!(titan.converged, "{titan:?}");
    assert!(titan.iterations < CONDENSE_ITERATIONS, "{titan:?}");
}
