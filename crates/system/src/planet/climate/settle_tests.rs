//! Tier 1: the settle of the albedo and emissivity, and the condensation
//! fixed point inside it, on Mars, a dry Earth, a hot rock and a
//! Triton-like body; the convergence of their numerical constants. The
//! cold methane world of #443 is tier 3 (about 13 s at opt-level 0).

use super::body::ClimateBody;
use super::fixture::{earth, mars, moved, titan};
use super::settle::{
    AlbedoMap, Bracket, End, Next, Settle, albedo_fixed_point, albedo_for, attracting, settle,
    warm_start,
};
use super::{
    ALBEDO_BRACKET, ALBEDO_ITERATIONS, ALBEDO_TOLERANCE_K, CONDENSE_ITERATIONS, ClimateInputs,
    Light, Numerics,
};
use science::balance::{BRACKET_TOLERANCE, Point, TOLERANCE};
use science::radiation::gases::{PartialPressures, Weights};
use science::radiation::ozone::ozone_column;
use science::surface::Liquid;
use science::units::{Bar, Dex, EarthFlux, Kelvin, Kilogram, WattPerSquareMetre};

/// Source: Mars's air, 6.36 mbar of 95.1 % CO₂, 2.59 % N₂ and 0.16 % O₂
/// (NASA Mars fact sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/
/// marsfact.html): 6.05 mbar of CO₂, 0.165 mbar of N₂ and 0.010 mbar of
/// O₂, imposed (study 25, "Test air and Solar System bounds"), and its
/// 1.93 % of argon-40, 0.123 mbar (Mahaffy et al. 2013, Science 341, 263,
/// abstract, as study 29 reads it). These are partial pressures, shares
/// by volume of the surface pressure; the climate reads their weights
/// ([`PartialPressures::weights`], issue #749).
const MARS_AIR: PartialPressures = PartialPressures {
    n2: 0.165e-3,
    co2: 6.05e-3,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.010e-3,
    ar: 0.123e-3,
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

/// Definition: a test tolerance, the test's own window on Mars's Bond
/// albedo, 0.05, for a model whose ground is one regolith; not a
/// measured uncertainty.
const MARS_BOND_WINDOW: f64 = 0.05;

/// Definition: a test tolerance, the test's own window on the relative
/// agreement of one sun and two half suns, 1e-9: a few roundings
/// through the fixed point (Review Focus 2).
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
/// input, weights: legacy's census drew them with its chain (seed 182,
/// `planet_census`, legacy `census_182_d`), whose gases were column
/// pressures, `m g / 4πR²`, typed as partial pressures (#749). Its census
/// bound of 4 to 5 bar was on that chain's N₂, the 4.379 bar here.
const COLD_AIR: Weights = Weights {
    n2: 4.379,
    co2: 1.14e-5,
    h2o: 0.0,
    ch4: 26.26,
    nh3: 0.0,
    so2: 2.4e-4,
    o2: 0.0,
    ar: 0.0,
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
        air: MARS_AIR.weights(),
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
        air: Weights {
            n2: TRITON_N2.0,
            ..Weights::default()
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

/// Definition: the metallicity of the census case's F star, dex: the
/// legacy case carries none, so the Sun's ([M/H] = 0).
const COLD_METALLICITY: Dex = Dex(0.0);

/// The census case of #443 (legacy `cold_methane_world`): a cold, wet
/// body round an F star, under a thick nitrogen and methane air.
pub(super) fn cold_methane_world() -> ClimateInputs {
    ClimateInputs {
        sources: vec![Light {
            flux: COLD_FLUX,
            effective_temperature: COLD_STAR,
            metallicity: COLD_METALLICITY,
            equilibrium: COLD_EQUILIBRIUM,
            ozone: ozone_column(Bar(COLD_AIR.o2), COLD_STAR),
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
        air: Weights::default(),
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
    assert!(base.converged && !base.cycles, "{base:?}");
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

/// Source: the argon a Triton made into its air, 1.8 Pa, the upper end
/// of study 29b's 1.3 to 1.8 Pa ("Checks on the cold bodies"), in bar.
const TRITON_ARGON: Bar = Bar(1.8e-5);

/// Definition: the test's margin on the share of a Triton-like air its
/// argon may hold, 5 %, twice the 2.5 % that study 29b's check finds the
/// two frost fits allow at activity 1 (the margin study 29b proposed).
const TRITON_ARGON_SHARE: f64 = 0.05;

/// A Triton-like body keeps its argon as frost: the air holds argon's
/// frost pressure at the settled temperature (activity 1), a small share
/// of the N₂ air, and the rest lies on the ground (study 29b).
#[test]
fn a_triton_like_body_keeps_its_argon_as_frost() {
    let bare = triton_like();
    let inputs = ClimateInputs {
        air: Weights {
            ar: TRITON_ARGON.0,
            ..bare.air
        },
        ..bare
    };
    let c = settled(&inputs, Numerics::DEFAULT);
    assert!(c.converged, "{c:?}");
    let frost = science::volatiles::argon_frost_pressure(c.temperature)
        .expect("under argon's triple point")
        .0
        / science::constants::PASCALS_PER_BAR;
    let held = c.air.partial_pressures();
    assert!((held.ar / frost - 1.0).abs() < 1e-9, "{c:?} {frost:e}");
    assert!(c.condensate.ar > 0.0, "{c:?}");
    let share = held.ar / held.total();
    println!(
        "Triton-like: {:.3} K, {:.3e} bar of Ar by weight, {:.3e} bar of partial \
         pressure, {share:.4} of the moles",
        c.temperature.0, c.air.ar, held.ar
    );
    assert!(share < TRITON_ARGON_SHARE, "{share}");
}

/// The end at `albedo` of a synthetic climate: emissivity 1, its radiative
/// equilibrium falling as its albedo rises (`used`, any decreasing map),
/// its `push` and `temperature`.
fn synthetic_end(
    albedo: f64,
    push: &impl Fn(f64) -> f64,
    temperature: &impl Fn(f64) -> f64,
) -> End<()> {
    End {
        albedo,
        emissivity: 1.0,
        used: -albedo,
        push: push(albedo),
        temperature: temperature(albedo),
        climate: (),
    }
}

/// Drives a [`Bracket`] over a synthetic `push` of the albedo (a positive
/// push asks for a warmer, so darker, climate) from the ends at `a` and
/// `b`, as `settle` does once its push turned back: each step takes the
/// albedo it gives, until its push is under `settled` or the bracket
/// closes. The albedo reached, whether it closed, the temperature of its
/// end, and the steps.
fn drive(
    push: impl Fn(f64) -> f64,
    temperature: impl Fn(f64) -> f64,
    a: f64,
    b: f64,
    settled: f64,
) -> (f64, bool, f64, usize) {
    let mut bracket = Bracket::default();
    bracket.take(synthetic_end(a, &push, &temperature));
    bracket.take(synthetic_end(b, &push, &temperature));
    assert!(bracket.attracts(), "the ends push towards each other");
    for steps in 1..=ALBEDO_ITERATIONS {
        match bracket.next(ALBEDO_BRACKET).expect("both ends are known") {
            Next::Closed(cold) => return (cold.albedo, true, cold.temperature, steps),
            Next::Step { albedo, .. } => {
                let here = synthetic_end(albedo, &push, &temperature);
                if here.push.abs() < settled {
                    return (albedo, false, here.temperature, steps);
                }
                bracket.take(here);
            }
        }
    }
    panic!("the bracket neither settled nor closed");
}

#[test]
fn a_bracketed_push_with_a_root_settles_on_it() {
    // A smooth push with its zero at 0.29, darker below it and brighter
    // above, curved so that the line between the ends misses it and the
    // Illinois rule has to halve a weight.
    let root = 0.29;
    let push = |a: f64| (a - root) * (1.0 + 40.0 * (a - root).powi(2)) + 0.5 * (a - root).powi(2);
    // Settled under a push of 1e-4 (in albedo here): `settle`'s 0.1 K over
    // a push of about 100 K per unit of albedo, a hundred times the
    // bracket's width, so a smooth root settles before the bracket closes.
    let (albedo, closed, _, steps) = drive(push, |_| 280.0, 0.279, 0.296, 1e-4);
    assert!(
        !closed && (albedo - root).abs() < 1e-4,
        "{albedo} after {steps}"
    );
    assert!(steps < 20, "{steps} steps");
}

#[test]
fn a_bracketed_jump_closes_on_its_cold_side() {
    // #729's cycle: under 0.29 the climate is warm and derives a brighter
    // albedo (its push is negative), above it the CO₂ collapses and it
    // derives a darker one. No push is ever under the tolerance: the
    // bracket closes on the jump, at the collapsed (colder) end, in about
    // as many steps as a bisection.
    let jump = 0.29;
    let push = |a: f64| if a < jump { -0.02 } else { 0.01 };
    let temperature = |a: f64| if a < jump { 277.0 } else { 188.0 };
    let (albedo, closed, t, steps) = drive(push, temperature, 0.279, 0.296, 1e-12);
    assert!(closed, "{albedo} after {steps}");
    assert_eq!(t, 188.0);
    assert!(albedo >= jump && albedo - jump < ALBEDO_BRACKET, "{albedo}");
    assert!(steps <= 40, "{steps} steps");
}

#[test]
fn a_bracket_whose_ends_push_apart_is_not_kept() {
    // A bistable body's unstable state: darker than 0.29 it warms
    // further, brighter it cools further. Its ends push apart, so
    // `settle` drops the bracket and the plain step leaves the root.
    let push = |a: f64| 0.29 - a;
    let mut bracket = Bracket::default();
    bracket.take(synthetic_end(0.279, &push, &|_| 280.0));
    bracket.take(synthetic_end(0.296, &push, &|_| 280.0));
    assert!(bracket.closed_in() && !bracket.attracts());
}

/// Definition: a synthetic S curve, the push of a bistable body against
/// the radiative equilibrium it is solved under (K): stable roots at 50
/// and 70 K (the push falls through them), an unstable one at 60 K (it
/// rises through it), the shape study 38 measured on a nitrogen world
/// (`docs/science/nitrogen-phases.md`).
fn s_curve(x: f64) -> f64 {
    -(x - 50.0) * (x - 60.0) * (x - 70.0) / 100.0
}

#[test]
fn only_the_branches_of_an_s_curve_attract() {
    let d = ALBEDO_TOLERANCE_K.0;
    let at = |x: f64| Point { x, f: s_curve(x) };
    // `probe_root`'s verdict: the push one tolerance colder against the
    // push at the root.
    let verdict = |root: f64| attracting(at(root - d), at(root));
    assert!(verdict(50.0) && verdict(70.0));
    assert!(!verdict(60.0));
    // Off the root by less than the tolerance, the verdict holds: the
    // probe reads the slope, not the sign of the push.
    assert!(verdict(50.0 + 0.5 * d) && !verdict(60.0 + 0.5 * d));
    assert!(verdict(70.0 - 0.5 * d) && !verdict(60.0 - 0.5 * d));
    // A bracket over the whole curve attracts, so the Illinois rule may
    // stop on any of its three roots, the unstable one included: the
    // ends' condition does not see what lies between them.
    let mut bracket = Bracket::default();
    for x in [45.0, 75.0] {
        bracket.take(End {
            albedo: 0.0,
            emissivity: 1.0,
            used: x,
            push: s_curve(x),
            temperature: x,
            climate: (),
        });
    }
    assert!(bracket.attracts());
    // The colder probe of the unstable root pushes colder, towards the
    // stable branch on the cold side.
    assert!(s_curve(60.0 - d) < 0.0);
}

#[test]
fn the_probe_albedo_puts_the_radiative_equilibrium_where_asked() {
    // T ∝ (1 − A)^¼ at one emissivity.
    let (albedo, used) = (0.75, 55.0);
    for target in [54.9, 55.1] {
        let moved = albedo_for(albedo, used, target);
        let ratio = ((1.0 - moved) / (1.0 - albedo)).powf(0.25);
        assert!((used * ratio - target).abs() < 1e-12, "{moved}");
    }
}

/// Definition: the light of a constructed climate, the radiative
/// equilibrium of a black, unit-emissivity body, 300 K: its radiative
/// equilibrium under albedo `A` and emissivity `ε` is `300 (1 − A)^¼
/// ε^-¼`, as `settle` reads a body's.
const CONSTRUCTED_LIGHT: f64 = 300.0;

/// Definition: where a constructed climate jumps, K of radiative
/// equilibrium: 274.4, between the radiative equilibria of the two
/// albedos its branches derive (273.42 under 0.31, 275.38 under 0.29).
const JUMP_AT: f64 = 274.4;

/// Definition: the albedos a constructed climate's branches derive, the
/// warm one's brighter (`#729`: a warm climate at its maximum greenhouse
/// derives a Rayleigh albedo that collapses its CO₂) and the collapsed
/// one's darker, a swing of 0.02 as on a methane Earth past its edge.
const WARM_DERIVES: f64 = 0.31;
const COLD_DERIVES: f64 = 0.29;

/// Definition: the most albedo steps a climate with no fixed point takes
/// to stop, 40: from the swing of 0.02 to [`ALBEDO_BRACKET`] is about 15
/// halvings, and the Illinois rule may take two steps for one; a test
/// bound, the one the methane Earth's cycling test held before #749.
const CYCLING_STEPS: usize = 40;

/// A constructed climate: its radiative equilibrium `x` under the albedo
/// and emissivity it was solved under, and its surface temperature.
#[derive(Clone, Copy, Debug)]
struct Constructed {
    x: f64,
    temperature: f64,
}

/// A climate that jumps at [`JUMP_AT`]: above it warm (30 K of
/// greenhouse) and deriving [`WARM_DERIVES`], under it collapsed (60 K
/// colder than its radiative equilibrium) and deriving [`COLD_DERIVES`].
/// Each derives an albedo that puts it on the other side: it has no fixed
/// point.
struct Jump;

/// A smooth climate (30 K of greenhouse) whose derived albedo rises with
/// its temperature, 0.004 per K of radiative equilibrium about 0.30 at
/// [`JUMP_AT`]: a stable root.
struct Smooth;

fn radiative(albedo: f64, emissivity: f64) -> f64 {
    CONSTRUCTED_LIGHT * (1.0 - albedo).powf(0.25) / emissivity.powf(0.25)
}

impl AlbedoMap for Jump {
    type Climate = Constructed;
    fn solve(&mut self, albedo: f64, emissivity: f64) -> Constructed {
        let x = radiative(albedo, emissivity);
        let temperature = if x >= JUMP_AT { x + 30.0 } else { x - 60.0 };
        Constructed { x, temperature }
    }
    fn temperature(climate: &Constructed) -> f64 {
        climate.temperature
    }
    fn regulated(_: &Constructed) -> bool {
        false
    }
    fn derived(&mut self, climate: &Constructed) -> (f64, f64) {
        let albedo = if climate.x >= JUMP_AT {
            WARM_DERIVES
        } else {
            COLD_DERIVES
        };
        (albedo, 1.0)
    }
    fn radiative(&mut self, albedo: f64, emissivity: f64) -> f64 {
        radiative(albedo, emissivity)
    }
}

impl AlbedoMap for Smooth {
    type Climate = Constructed;
    fn solve(&mut self, albedo: f64, emissivity: f64) -> Constructed {
        let x = radiative(albedo, emissivity);
        Constructed {
            x,
            temperature: x + 30.0,
        }
    }
    fn temperature(climate: &Constructed) -> f64 {
        climate.temperature
    }
    fn regulated(_: &Constructed) -> bool {
        false
    }
    fn derived(&mut self, climate: &Constructed) -> (f64, f64) {
        (0.30 + 0.004 * (climate.x - JUMP_AT), 1.0)
    }
    fn radiative(&mut self, albedo: f64, emissivity: f64) -> f64 {
        radiative(albedo, emissivity)
    }
}

#[test]
fn a_climate_with_no_fixed_point_stops_on_its_cold_side() {
    // #729, rebuilt on a constructed climate (#767): started warm, the
    // climate derives a brighter albedo that collapses it, and the
    // collapsed one a darker albedo that warms it back. The plain step
    // swings across the jump, the bracket narrows on it, and the fixed
    // point stops there with the colder end, not converged.
    let fixed = albedo_fixed_point(&mut Jump, (COLD_DERIVES, 1.0), &Numerics::DEFAULT);
    println!(
        "albedo {:.8}, {:.3} K, {} steps",
        fixed.albedo, fixed.climate.temperature, fixed.steps
    );
    assert!(fixed.cycles && !fixed.settled, "{fixed:?}");
    assert!(fixed.climate.x < JUMP_AT, "the colder side: {fixed:?}");
    assert!(
        JUMP_AT - fixed.climate.x < 1e-3,
        "on the jump within the bracket: {fixed:?}"
    );
    assert!(fixed.steps <= CYCLING_STEPS, "{} steps", fixed.steps);
}

#[test]
fn a_climate_with_a_stable_root_settles_on_it() {
    // The same loop on a smooth climate: it settles, and its albedo is
    // the one it derives within the tolerance.
    let fixed = albedo_fixed_point(&mut Smooth, (COLD_DERIVES, 1.0), &Numerics::DEFAULT);
    assert!(fixed.settled && !fixed.cycles, "{fixed:?}");
    let derived = Smooth.derived(&fixed.climate).0;
    let off = radiative(derived, 1.0) - fixed.climate.x;
    assert!(off.abs() < ALBEDO_TOLERANCE_K.0, "{off} K: {fixed:?}");
    assert!(fixed.steps < CYCLING_STEPS, "{} steps", fixed.steps);
}
