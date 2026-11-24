//! The radiative balance of one column: the surface temperature at which
//! the outgoing longwave radiation of a radiative-convective column
//! ([`crate::radiation::column`]) carries away what the surface absorbs,
//! the root finders that solve it, the moist plateau of a saturated
//! column (the Simpson-Nakajima limit), and the Bond albedo under which
//! a water runaway absorbs that plateau at its star's runaway limit.
//!
//! The balance of one column is a law: radiative-convective equilibrium
//! (Manabe & Wetherald 1967; Pierrehumbert 2010, *Principles of Planetary
//! Climate*, ch. 4). The solver that settles a body on it (its albedo,
//! its condensing air, its thermostat and its branches) lives in
//! `system::planet::climate`.
//!
//! Ported from the tag `legacy-final` (`planet/climate.rs` l.291-297
//! `absorbed_w_m2`, l.339-394 `solve` and `bracket`, l.396-472 the moist
//! plateau, l.475-560 the runaway albedo, l.624-697 `moist_plateau_of`
//! and `surface_temperature_k`), with the values of study
//! [25](https://github.com/cyboooooorg/map-generator/wiki/25-climate-solver-audit);
//! see `docs/science/planet-atmospheres.md`, "The climate solver of the
//! rewrite (study 25)".

use std::sync::OnceLock;

use crate::constants::{STANDARD_GRAVITY, STEFAN_BOLTZMANN};
use crate::habitable::runaway_flux;
use crate::radiation::column::{ColumnCache, Profile, outgoing_longwave, skin_temperature};
use crate::radiation::gases::PartialPressures;
use crate::radiation::methane::stratospheric_share;
use crate::surface::surface_vapour;
use crate::units::{Bar, Fraction, Kelvin, MetrePerSecondSquared, Pascal, WattPerSquareMetre};
use crate::water::{TRIPLE_POINT_PRESSURE, boiling_temperature};

/// The settings of a balance solve: the width under which a root's
/// bracket is narrow enough, the most steps of a root search, and the
/// finer width of the solves nested in another search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solver {
    /// The width, K, under which a surface temperature is solved.
    pub tolerance: Kelvin,
    /// The most steps of one root search.
    pub iterations: usize,
    /// The width, K, of the solves nested inside another search.
    pub bracket_tolerance: Kelvin,
}

/// Calibration: the absolute tolerance of the surface temperatures solved
/// for, 0.01 K (legacy `TOLERANCE_K`). Study 25 ("Solver constants"): the
/// reference climates solved at a tenth of it move by under 0.01 K, a
/// convergence test in tier 1 (`system::planet::climate`). No source sets
/// a numerical tolerance.
pub const TOLERANCE: Kelvin = Kelvin(0.01);

/// Definition: the most steps of one root search, 60 (legacy
/// `ITERATIONS`), a cap the solves stop under by [`TOLERANCE`] (study 25,
/// "Solver constants"; `iterations_is_converged`).
pub const ITERATIONS: usize = 60;

/// Calibration: the tolerance of the solves nested inside another search,
/// a hundredth of [`TOLERANCE`] (legacy `BRACKET_TOLERANCE_K`), held with
/// it (study 25, "Solver constants").
pub const BRACKET_TOLERANCE: Kelvin = Kelvin(TOLERANCE.0 / 100.0);

/// The settings of the balance solves: [`TOLERANCE`], [`ITERATIONS`] and
/// [`BRACKET_TOLERANCE`].
pub const SOLVER: Solver = Solver {
    tolerance: TOLERANCE,
    iterations: ITERATIONS,
    bracket_tolerance: BRACKET_TOLERANCE,
};

/// Calibration: the temperature at which the carbonate thermostat holds a
/// planet with plates and liquid water, 288 K: weathering draws CO₂ down
/// faster when warm and volcanoes give it back (Walker, Hays & Kasting
/// 1981, doi:10.1029/JC086iC10p09776). Fitted on Earth's "Average
/// temperature: 288 K" (NASA Earth Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html, read for study
/// 25). Searched and not found: a law for the temperature at which
/// weathering regulates another planet (study 25, "The runaway branch").
pub const THERMOSTAT_K: Kelvin = Kelvin(288.0);

/// Definition: the least outgoing radiation a bracket step divides by, a
/// share of the absorbed flux, 1e-3 (legacy `climate.rs` l.383 and 1170):
/// a guard against a step of `(absorbed / 0)^¼`; it does not change the
/// root bracketed.
pub const OLR_FLOOR: f64 = 1e-3;

/// Definition: how far past the `T⁴` estimate a bracket step goes, a
/// ratio, 1.0005 (legacy `climate.rs` l.384 and 1176), so that a column
/// radiating as `T⁴` is crossed in one step; it does not change the root.
pub const OVERSHOOT: f64 = 1.0005;

/// Definition: the least ratio of a bracket step, 1.001 (legacy
/// `climate.rs` l.384 and 1177), so that a search always moves up; it
/// does not change the root.
pub const LEAST_STEP: f64 = 1.001;

/// Definition: the hottest surface, K, [`bracket_up`] searches to, 1e6 K
/// (legacy `bracket`): a guard against a column that never radiates
/// enough, far above any surface the model makes.
pub const HOTTEST: Kelvin = Kelvin(1.0e6);

/// Definition: the least emissivity a surface radiates with, 0.01, a
/// guard against a division by zero; every surface the model makes
/// radiates with more (frost emissivity, `science::surface`).
pub const LEAST_EMISSIVITY: f64 = 0.01;

/// A point of a function solved for: its argument and its value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// The argument.
    pub x: f64,
    /// The function's value there.
    pub f: f64,
}

/// A root found by [`regula_falsi`]: the upper end of the last bracket,
/// where the function is at least zero, and the steps it took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Root {
    /// The root.
    pub x: f64,
    /// The function evaluations made; under the cap when the search
    /// stopped by its tolerance.
    pub iterations: usize,
}

/// The `x` in `[low.x, high.x]` at which `f` (rising with `x`) crosses
/// zero, given `low.f < 0 <= high.f`: regula falsi with the Illinois rule
/// (Dowell & Jarratt 1971, *BIT* 11, 168, doi:10.1007/BF01934364), until
/// the bracket is narrower than `tolerance` or after `iterations`
/// evaluations. Returns the upper end, where `f >= 0`. A step that falls
/// outside the bracket (a flat or NaN chord) bisects instead.
pub fn regula_falsi(
    mut f: impl FnMut(f64) -> f64,
    mut low: Point,
    mut high: Point,
    tolerance: f64,
    iterations: usize,
) -> Root {
    let mut side = 0;
    for step in 0..iterations {
        if high.x - low.x < tolerance {
            return Root {
                x: high.x,
                iterations: step,
            };
        }
        let mut x = (low.x * high.f - high.x * low.f) / (high.f - low.f);
        if !(x > low.x && x < high.x) {
            x = 0.5 * (low.x + high.x);
        }
        let value = f(x);
        if value < 0.0 {
            low = Point { x, f: value };
            if side == -1 {
                high.f *= 0.5;
            }
            side = -1;
        } else {
            high = Point { x, f: value };
            if side == 1 {
                low.f *= 0.5;
            }
            side = 1;
        }
    }
    Root {
        x: high.x,
        iterations,
    }
}

/// Below and above the temperature, K, at which `excess = OLR − absorbed`
/// crosses zero going up from `start` (where it is below zero): steps of
/// `T ← T (absorbed / OLR(T))^¼ ·` [`OVERSHOOT`] (exact for an outgoing
/// radiation that grows as `T⁴`), at least [`LEAST_STEP`] each, the
/// outgoing radiation floored at [`OLR_FLOOR`] of `absorbed`. Stops past
/// [`HOTTEST`] with the last point as the upper end.
pub fn bracket_up(
    mut excess: impl FnMut(f64) -> f64,
    absorbed: f64,
    start: Point,
) -> (Point, Point) {
    let mut low = start;
    loop {
        let olr = (low.f + absorbed).max(OLR_FLOOR * absorbed);
        let x = (low.x * (absorbed / olr).powf(0.25) * OVERSHOOT).max(low.x * LEAST_STEP);
        let f = excess(x);
        let point = Point { x, f };
        if f >= 0.0 || x > HOTTEST.0 {
            return (low, point);
        }
        low = point;
    }
}

/// The skin temperature of the column of a body that absorbs `starlight`
/// (W/m², after its albedo): [`skin_temperature`] of the effective
/// temperature `(starlight / σ)^¼` (legacy `plateau_skin_k`, and the
/// `tropopause_k(T_eq)` of `surface_temperature_k`). A negative starlight
/// reads as none.
pub fn skin(starlight: WattPerSquareMetre) -> Kelvin {
    skin_temperature(Kelvin((starlight.0.max(0.0) / STEFAN_BOLTZMANN).powf(0.25)))
}

/// What the surface of a body under `gases` absorbs, W/m², from
/// `starlight` (after its albedo) and `heat` of internal heat: the light
/// less the share the stratosphere's methane takes and radiates to space
/// ([`stratospheric_share`] under the [`skin`] of the light, the
/// anti-greenhouse of McKay, Pollack & Courtin 1991,
/// doi:10.1126/science.11538492), plus the heat. A negative light or
/// heat reads as none.
pub fn absorbed(
    starlight: WattPerSquareMetre,
    gases: &PartialPressures,
    heat: WattPerSquareMetre,
) -> WattPerSquareMetre {
    let light = starlight.0.max(0.0);
    let share = stratospheric_share(gases, skin(starlight)).get();
    WattPerSquareMetre(light * (1.0 - share) + heat.0.max(0.0))
}

/// Mean surface temperature of a body with `gases` under `starlight`
/// (W/m² absorbed, after its albedo), plus its internal `heat`, whose
/// surface radiates with `emissivity` (1 for a black body, held within
/// [`LEAST_EMISSIVITY`] to 1), on `gravity`.
///
/// The temperature at which the outgoing radiation of the climate's
/// column ([`Profile::CLIMATE`]), times the emissivity, balances what the
/// surface [`absorbed`]; the column's stratosphere sits at the [`skin`]
/// of the light. The column radiates no more than its surface, so the
/// grey body `(absorbed / ε σ)^¼` is a lower bound, and so is the
/// temperature at which the vapour of `gases` boils (a runaway's steam is
/// all in its air: its surface cannot be colder). Without air the surface
/// sits at that lower bound: the light and the heat alone. The root is
/// bracketed by [`bracket_up`] and found by [`regula_falsi`] within
/// `solver.tolerance`.
pub fn surface_temperature(
    starlight: WattPerSquareMetre,
    gases: &PartialPressures,
    heat: WattPerSquareMetre,
    emissivity: Fraction,
    gravity: MetrePerSecondSquared,
    solver: Solver,
    cache: &mut ColumnCache,
) -> Kelvin {
    Kelvin(surface_root(starlight, gases, heat, emissivity, gravity, solver, cache).x)
}

/// [`surface_temperature`] with the steps of its root search (none when
/// the lower bound is the answer).
pub(crate) fn surface_root(
    starlight: WattPerSquareMetre,
    gases: &PartialPressures,
    heat: WattPerSquareMetre,
    emissivity: Fraction,
    gravity: MetrePerSecondSquared,
    solver: Solver,
    cache: &mut ColumnCache,
) -> Root {
    let absorbed = absorbed(starlight, gases, heat).0;
    let e = emissivity.get().clamp(LEAST_EMISSIVITY, 1.0);
    let bare = (absorbed / (e * STEFAN_BOLTZMANN)).powf(0.25);
    if gases.total() <= 0.0 || bare <= 0.0 {
        return Root {
            x: bare,
            iterations: 0,
        };
    }
    let skin = skin(starlight);
    let mut excess = |t: f64| {
        e * outgoing_longwave(gases, Kelvin(t), skin, gravity, Profile::CLIMATE, cache).0 - absorbed
    };
    let vapour = Pascal::from(Bar(gases.h2o));
    let condenses = if vapour.0 > TRIPLE_POINT_PRESSURE.0 {
        boiling_temperature(vapour).map_or(0.0, |t| t.0)
    } else {
        0.0
    };
    let lowest = bare.max(condenses);
    let start = Point {
        x: lowest,
        f: excess(lowest),
    };
    if start.f >= 0.0 {
        return Root {
            x: lowest,
            iterations: 0,
        };
    }
    let (low, high) = bracket_up(&mut excess, absorbed, start);
    regula_falsi(excess, low, high, solver.tolerance.0, solver.iterations)
}

/// Calibration: the surfaces, K, over which the moist plateau averages
/// the outgoing radiation of a saturated column: 500 to 640 K every 20 K,
/// past the curve's maximum near 360 K (legacy `PLATEAU_SURFACES_K`).
/// Study 25 ("Solver constants"): on surfaces every 10 K and up to
/// 700 K the plateau moves by under 0.5 W/m², a convergence test in
/// tier 3. Boukrouche et al. 2021 (*ApJ* 919, 130, §III.1) find the
/// radiation flat over these surfaces, without setting where to sample.
pub const PLATEAU_SURFACES: [Kelvin; 8] = [
    Kelvin(500.0),
    Kelvin(520.0),
    Kelvin(540.0),
    Kelvin(560.0),
    Kelvin(580.0),
    Kelvin(600.0),
    Kelvin(620.0),
    Kelvin(640.0),
];

/// Definition: the most steps of the plateau's fixed point with its skin,
/// 8 (legacy `plateau_w_m2`): the skin moves the plateau by under
/// 0.1 W/m², so a few steps settle it.
pub const PLATEAU_ITERATIONS: usize = 8;

/// Definition: the change of the plateau, W/m², under which its fixed
/// point has settled, 0.01 (legacy `plateau_w_m2`).
pub const PLATEAU_SETTLED: WattPerSquareMetre = WattPerSquareMetre(0.01);

/// Source: the Simpson–Nakajima limit, W/m², 282 line by line (Goldblatt,
/// Robinson, Zahnle & Crisp 2013, *Nature Geosci.* 6, 661,
/// doi:10.1038/ngeo1892, Fig. 2: "the Simpson-Nakajima radiation limit
/// emerges as 282 W m-2"); Boukrouche, Lichtenberg & Pierrehumbert 2021
/// (*ApJ* 919, 130, §III.1) find 283 and Hamano et al. 2015 (*ApJ* 806,
/// 216, §3) 280 (audit, Wiki study 19). The anchor of the moist plateau,
/// and the start of its fixed point.
pub const SIMPSON_NAKAJIMA_W_M2: WattPerSquareMetre = WattPerSquareMetre(282.0);

/// Definition: Earth's dry air, bar: 0.78 of N₂, 0.21 of O₂ and 400 ppm
/// of CO₂ (the legacy `EARTH_AIR`; argon, which neither absorbs nor
/// changes the adiabat much, has no field). The air of Earth's moist
/// plateau ([`earth_moist_plateau`]) and of the radiation tests.
pub const EARTH_DRY_AIR: PartialPressures = PartialPressures {
    n2: 0.78,
    co2: 4e-4,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.21,
};

/// The moist plateau, W/m², of the `dry` air over a saturated surface,
/// its vapour ([`surface_vapour`]) capped by the `water` supply, on
/// `gravity` and the column of `profile`: the mean outgoing radiation over
/// `surfaces` (the [`PLATEAU_SURFACES`]) under the [`skin`] of a body
/// that absorbs it, the two solved together by a fixed-point iteration
/// from `start`, at most [`PLATEAU_ITERATIONS`] steps, until a step moves
/// it by under [`PLATEAU_SETTLED`].
///
/// The asymptote of a moist column, whose emission comes from levels
/// pinned to water's saturation curve: the Simpson–Nakajima limit
/// ([`SIMPSON_NAKAJIMA_W_M2`]). A body's own plateau starts from
/// [`earth_moist_plateau`] (legacy `moist_plateau_of`).
pub fn moist_plateau(
    dry: &PartialPressures,
    water: Bar,
    gravity: MetrePerSecondSquared,
    profile: Profile,
    surfaces: &[Kelvin],
    start: WattPerSquareMetre,
    cache: &mut ColumnCache,
) -> WattPerSquareMetre {
    let mut asymptote = |skin: Kelvin| {
        let sum: f64 = surfaces
            .iter()
            .map(|&t| {
                let gases = PartialPressures {
                    h2o: surface_vapour(t, water).0,
                    ..*dry
                };
                outgoing_longwave(&gases, t, skin, gravity, profile, cache).0
            })
            .sum();
        sum / surfaces.len() as f64
    };
    let mut plateau = start;
    for _ in 0..PLATEAU_ITERATIONS {
        let next = WattPerSquareMetre(asymptote(skin(plateau)));
        let settled = (next.0 - plateau.0).abs() < PLATEAU_SETTLED.0;
        plateau = next;
        if settled {
            break;
        }
    }
    plateau
}

/// The column's own moist plateau on Earth's dry air
/// ([`EARTH_DRY_AIR`]), with all the water it can hold, on the standard
/// gravity and the plateau's clear, saturated, fine column
/// ([`Profile::PLATEAU`]), started from [`SIMPSON_NAKAJIMA_W_M2`]: the
/// plateau a runaway's albedo binds at its star's limit
/// ([`runaway_albedo`]). Solved once per process.
pub fn earth_moist_plateau() -> WattPerSquareMetre {
    static PLATEAU: OnceLock<WattPerSquareMetre> = OnceLock::new();
    *PLATEAU.get_or_init(|| {
        moist_plateau(
            &EARTH_DRY_AIR,
            Bar(f64::INFINITY),
            STANDARD_GRAVITY,
            Profile::PLATEAU,
            &PLATEAU_SURFACES,
            SIMPSON_NAKAJIMA_W_M2,
            &mut ColumnCache::default(),
        )
    })
}

/// Definition: the least and greatest Bond albedo of a body in a water
/// runaway, 0 and 0.9 (legacy `RUNAWAY_ALBEDO`): a guard; over the fit's
/// 2600 to 7200 K, [`runaway_albedo`] runs from about 0.07 to 0.34 on the
/// rewrite's 289.7 W/m² plateau (study 25, "The runaway branch", derived
/// 0.087 to 0.357 on legacy's 283.5 W/m²).
pub const RUNAWAY_ALBEDO: (f64, f64) = (0.0, 0.9);

/// The Bond albedo under which a body in a water runaway, round a star
/// of effective temperature `star`, absorbs its moist `plateau` exactly
/// at the star's runaway limit `S_rg` ([`runaway_flux`]):
/// `A_rg = 1 − 4 F / (S_rg S₀)`, `S₀` the [`SOLAR_IRRADIANCE`](crate::constants::SOLAR_IRRADIANCE)
/// (the unit of [`runaway_flux`]'s flux), held
/// within [`RUNAWAY_ALBEDO`] (a `Definition:` of study 25, "The runaway
/// branch"; legacy `moist_albedo`, whose `REFERENCE_ALBEDO` is not
/// ported). On the rewrite's Earth plateau ([`earth_moist_plateau`],
/// about 289.7 W/m²) it is about 0.23 round the Sun: a consequence of the
/// plateau, not a target (study 25 derived 0.247 from legacy's
/// 283.5 W/m²). A NaN plateau reads as the least albedo.
pub fn runaway_albedo(star: Kelvin, plateau: WattPerSquareMetre) -> Fraction {
    let limit = WattPerSquareMetre::from(runaway_flux(star)).0;
    let albedo = 1.0 - 4.0 * plateau.0 / limit;
    let (least, greatest) = RUNAWAY_ALBEDO;
    let held = if albedo.is_nan() {
        least
    } else {
        albedo.clamp(least, greatest)
    };
    Fraction::new(held).expect("a held albedo is a fraction")
}

/// Calibration: the steam column, bar, over which a runaway's Bond albedo
/// moves from its derived one to the moist `A_rg` ([`runaway_albedo`]),
/// the e-folding of the blend [`steam_share`], 1 bar (legacy
/// `STEAM_ALBEDO_BAR`). Kopparapu et al. 2013 (*ApJ* 765, 131, §III.1)
/// reach the moist limit "at a surface temperature of 340 K" and the
/// albedo's minimum "at a surface temperature of 400 K"; water's
/// saturation there is 0.27188 and 2.4577 bar (NIST Chemistry WebBook):
/// the e-folding sits between. Searched and not found: the vapour column
/// at the runaway limit as a number, in Kopparapu et al. 2013 and 2014
/// and Goldblatt et al. 2013 (study 25, "The runaway branch").
pub const STEAM_ALBEDO_BAR: Bar = Bar(1.0);

/// The weight of the moist albedo in a runaway's Bond albedo for `steam`
/// in its air: `1 − exp(−steam / STEAM_ALBEDO_BAR)`
/// ([`STEAM_ALBEDO_BAR`]). The albedo is `A + (A_rg − A) · share`, so a
/// runaway losing the last of its steam goes back to its derived albedo
/// `A` with no jump. A negative or NaN steam reads as none.
pub fn steam_share(steam: Bar) -> Fraction {
    let share = 1.0 - (-steam.0.max(0.0) / STEAM_ALBEDO_BAR.0).exp();
    Fraction::new(share).expect("1 − exp(−x) of x ≥ 0 is a fraction")
}

#[cfg(test)]
#[path = "balance_tests.rs"]
mod tests;
