//! The coldest balance of a surface whose water vapour follows its
//! temperature, and the collapse of a CO₂ its surface cannot keep.
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`
//! (`wet_balance`, `first_balance`, `wet_climate` and
//! `collapsed_climate`, l.1109-1270).

use super::body::ClimateBody;
use super::{ClimateInputs, Numerics};
use science::balance::{
    LEAST_EMISSIVITY, LEAST_STEP, OLR_FLOOR, OVERSHOOT, Point, absorbed, regula_falsi, skin,
    surface_temperature,
};
use science::constants::STEFAN_BOLTZMANN;
use science::radiation::column::{Profile, outgoing_longwave};
use science::radiation::gases::{CO2_MOLAR_MASS, Weights};
use science::surface::surface_vapour;
use science::units::{Bar, Fraction, Kelvin, WattPerSquareMetre};
use science::volatiles::co2_saturation_pressure;
use science::water::CRITICAL_POINT_TEMPERATURE;

/// `air` with the vapour a surface at `t` holds of `water`
/// ([`surface_vapour`]).
fn with_vapour(air: &Weights, t: f64, water: Bar) -> Weights {
    Weights {
        h2o: surface_vapour(Kelvin(t), water).0,
        ..*air
    }
}

/// The coldest balance of `body`'s surface under `starlight` (W/m²,
/// after its albedo) when its water vapour follows its temperature: the
/// gases with that vapour ([`surface_vapour`] of its
/// [`ClimateBody::water_column`]) and the temperature at which the
/// climate's column ([`Profile::CLIMATE`]) over the surface, times its
/// `emissivity`, sends out what the surface [`absorbed`] plus its
/// internal heat, both on those gases: the vapour dilutes the methane
/// whose stratosphere takes a share of the light, so the light absorbed
/// is that air's, as [`surface_temperature`] reads it (#681; legacy took
/// it once, on `air`). A cooling planet stops at the first such
/// temperature above the grey body of the dry air, which absorbs least
/// (the vapour only lowers the outgoing radiation and raises the light
/// absorbed, so no colder surface balances).
///
/// `None` when the water runs away: the grey body is already at water's
/// critical point, or no surface below it balances with its troposphere
/// saturated ([`Profile::SATURATED`]), the moist limit of Kasting 1988
/// and Kopparapu et al. 2013. The saturated column is tried first at the
/// numerics' probe ([`super::RUNAWAY_PROBE_K`]): a surface that sends out
/// what it absorbs there cannot run away; any other, and every one when
/// the probe is `None`, is scanned in full ([`first_balance`]).
pub(crate) fn wet_balance(
    body: &mut ClimateBody,
    starlight: WattPerSquareMetre,
    air: &Weights,
    emissivity: Fraction,
) -> Option<(Weights, Kelvin)> {
    let numerics = body.numerics;
    let water = body.water_column();
    let (heat, gravity) = (body.inputs.heat_flux, body.inputs.gravity);
    // The dry air absorbs least: its vapour only dilutes the methane whose
    // stratosphere takes a share of the light (#681).
    let dry = Weights { h2o: 0.0, ..*air }.partial_pressures();
    let least = absorbed(starlight, &dry, heat).0;
    let skin = skin(starlight);
    let e = emissivity.get().clamp(LEAST_EMISSIVITY, 1.0);
    let bare = (least / (e * STEFAN_BOLTZMANN)).powf(0.25);
    if bare >= CRITICAL_POINT_TEMPERATURE.0 {
        return None;
    }
    let ozone = body.inputs.longwave_ozone();
    let columns = &mut body.columns;
    let mut excess_of = |t: f64, profile: Profile| {
        let gases = with_vapour(air, t, water).partial_pressures();
        e * outgoing_longwave(&gases, ozone, Kelvin(t), skin, gravity, profile, columns).0
            - absorbed(starlight, &gases, heat).0
    };
    let probe_holds = numerics
        .probe
        .is_some_and(|p| excess_of(p.0, Profile::SATURATED) >= 0.0);
    if !probe_holds {
        first_balance(|t| excess_of(t, Profile::SATURATED), least, bare, &numerics)?;
    }
    let t = first_balance(|t| excess_of(t, Profile::CLIMATE), least, bare, &numerics)?;
    Some((with_vapour(air, t, water), Kelvin(t)))
}

/// The first surface temperature, K, from `bare` up to water's critical
/// point, at which `excess` (outgoing less absorbed, W/m²) reaches
/// zero; `None` if none (legacy `first_balance`, the scan of #600).
/// `absorbed` (W/m², the least the surface absorbs over the scan) only
/// scales its steps.
///
/// Each step goes up by the `T⁴` step of `science::balance::bracket_up`,
/// or by the secant through the last two points where that goes further
/// (the moist column's outgoing radiation rises more slowly than `T⁴`),
/// at least [`LEAST_STEP`] and at most the numerics' `scan_ratio`, capped
/// at [`CRITICAL_POINT_TEMPERATURE`]; the first crossing is solved by
/// [`regula_falsi`] within the numerics' solver.
pub(crate) fn first_balance(
    mut excess: impl FnMut(f64) -> f64,
    absorbed: f64,
    bare: f64,
    numerics: &Numerics,
) -> Option<f64> {
    let critical = CRITICAL_POINT_TEMPERATURE.0;
    let mut low = Point {
        x: bare,
        f: excess(bare),
    };
    if low.f >= 0.0 {
        return Some(bare);
    }
    let mut before: Option<Point> = None;
    loop {
        let olr = (low.f + absorbed).max(OLR_FLOOR * absorbed);
        let t4 = low.x * (absorbed / olr).powf(0.25);
        let guess = match before {
            Some(b) if low.f > b.f => t4.max(low.x - low.f * (low.x - b.x) / (low.f - b.f)),
            _ => t4,
        };
        let x = (guess * OVERSHOOT)
            .clamp(low.x * LEAST_STEP, low.x * numerics.scan_ratio)
            .min(critical);
        let high = Point { x, f: excess(x) };
        if high.f >= 0.0 {
            let solver = numerics.solver;
            return Some(regula_falsi(excess, low, high, solver.tolerance.0, solver.iterations).x);
        }
        if x >= critical {
            return None;
        }
        before = Some(low);
        low = high;
    }
}

/// The air and temperature of `body`'s surface under `starlight` when its
/// water vapour follows its temperature: the coldest balance of
/// [`wet_balance`]. Without one (a moist runaway), all of its water as
/// vapour ([`ClimateBody::water_column`]) and the temperature
/// [`surface_temperature`] gives it. As in legacy's `wet_climate`
/// (l.1212) this is a stand-in: a body without a balance boils
/// ([`boils`]), and `settled` runs the climate again on its steam, so
/// this air is never the one a boiled body keeps.
pub(crate) fn wet_climate(
    body: &mut ClimateBody,
    starlight: WattPerSquareMetre,
    air: &Weights,
    emissivity: Fraction,
) -> (Weights, Kelvin) {
    if let Some(balance) = wet_balance(body, starlight, air, emissivity) {
        return balance;
    }
    let steam = Weights {
        h2o: body.water_column().0.max(0.0),
        ..*air
    };
    let i = body.inputs;
    let t = surface_temperature(
        starlight,
        &steam.partial_pressures(),
        i.longwave_ozone(),
        i.heat_flux,
        emissivity,
        i.gravity,
        body.numerics.solver,
        i.ceiling(),
        &mut body.columns,
    );
    (steam, t)
}

/// Whether a body that keeps water out of a runaway boils it (legacy
/// `climate::boils`): under the light its `bond_albedo` lets in plus its
/// internal heat, with `air` (the air its climate left) and its
/// surface's `emissivity`, no wet balance exists ([`wet_balance`] is
/// `None`).
///
/// Source: Kasting 1988 (Icarus 74, 472, doi:10.1016/0019-1035(88)90116-9,
/// section 3a, a moist atmosphere that cannot radiate what it absorbs
/// runs away, and section 3d, the internal (accretional) heat a surface
/// temperature needs counts with the light, Fig. 13); Goldblatt et al.
/// 2013 (Nature Geosci. 6, 661, doi:10.1038/ngeo1892, a runaway once the
/// absorbed flux exceeds the moist limit); Barnes et al. 2013
/// (Astrobiology 13, 225, doi:10.1089/ast.2012.0851, section 2, the rule
/// for any heat source: the energy from the Sun, impacts or tidal heating
/// past the limit evaporates the entire ocean). There is no temperature
/// condition. The vapour is bounded by the water the body keeps
/// ([`ClimateBody::water_column`]), so a trace of water cannot hold its
/// outgoing radiation at the moist limit and does not boil (Kasting 1988
/// section 3a; Boukrouche et al. 2021 Fig. 9).
///
/// Departures from legacy:
///
/// - the emissivity: legacy passes an emissivity of 1.0; the rewrite passes
///   the surface's own `emissivity`, so the verdict reads the surface
///   the climate radiated under;
/// - a regulated climate is judged too (user decision, 2026-11-24).
///   Legacy skipped it (`mod.rs` l.524-537), taking the thermostat's
///   balance for a wet one, but the thermostat balances on the climate's
///   column at 288 K ([`super::carbon::thermostat`]), not on the
///   saturated one the moist limit reads: past the saturated column's
///   peak a regulated climate runs away by the sources above
///   (`docs/science/planet-atmospheres.md`, study 26, "The boil").
pub(crate) fn boils(
    inputs: &ClimateInputs,
    air: &Weights,
    bond_albedo: Fraction,
    emissivity: Fraction,
) -> bool {
    let mut body = ClimateBody::new(inputs, Numerics::DEFAULT);
    if !body.wet() {
        return false;
    }
    let light = body.starlight(bond_albedo);
    wet_balance(&mut body, light, air, emissivity).is_none()
}

/// A collapsed climate: its air, its temperature, the steps it took, and
/// whether it settled by its tolerance (`held`) rather than stopping at
/// the numerics' `collapse_iterations` on its way down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Collapsed {
    pub air: Weights,
    pub temperature: Kelvin,
    pub iterations: usize,
    pub held: bool,
}

/// What the surface of a climate solved with some CO₂ keeps, bar: `kept`,
/// `φ(c)` ([`co2_saturation_pressure`] of its temperature), and `noise`,
/// by how much `φ` moves within the temperature tolerance of the solve,
/// with the climate solved (`state`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Kept<S> {
    pub kept: f64,
    pub noise: f64,
    pub state: S,
}

/// The CO₂, bar, at which a collapse from `start` settles, with the climate
/// solved there, the evaluations after the first one, and whether it
/// settled (false when it stopped at `most` evaluations first).
///
/// `solve(c)` solves the climate with `c` bar of CO₂ ([`Kept`]). The collapse
/// settles at a `c` that keeps what its air holds within `tolerance`, `g(c) =
/// c − φ(c) ≤ tolerance φ(c)`, after at most `most` evaluations.
///
/// With `widest` zero it steps plainly, `c ← φ(c)`, as legacy did: below the
/// maximum greenhouse `φ` grows with `c`, so from above it falls onto the
/// largest root of `g` below `start`, the warmest balance a cooling planet
/// reaches, never past it. A collapse that starts at the maximum greenhouse
/// can start at the fold of the warm branch, where `g` is about zero
/// (measured on a methane Earth's edge search); there it crawls, `g` growing
/// by about a tenth a step, for up to 58 steps at 17 bar (#731, measured
/// before #734).
///
/// Otherwise (#731) each step is at least the plain one and at most twice the
/// last one. While `g` rises or stays flat, it is also at most `widest` times
/// the CO₂; once `g` falls by more than its noise, it goes to the zero of the
/// secant through the last two points, with no cap but twice the last step (a
/// fall within the noise is a jitter of the solve, not a fall, and keeps the
/// rising cap). Near a methane Earth's
/// edge round a cool star, `g` is a single hump from about zero to its root
/// (0.3 to 3 bar high, over 12 to 44 bar of CO₂): the cap kept the rise
/// short and the secant short of the root, 83 and 44 evaluations at 4500
/// and 3300 K; the two rules take about 22 and 22 (simulated on the traced
/// gaps; #731, measured on main a97c1ca1). Where `g` falls below
/// `−noise` it has passed a root: the search brackets it between that point
/// and the last one above zero, and closes in on it by the Illinois rule
/// (Dowell & Jarratt 1971) to a point with `0 ≤ g ≤ tolerance φ`. A root
/// whose `g` crosses zero by no more than its noise is not one: until `g` has
/// risen above its noise once, the search does not stop where it holds, and
/// steps on by no less than the noise; inside a bracket a point below the
/// root within its noise ends the search, since the solve's tolerance lets
/// `g` jump across a root by up to its noise ("A collapse from the fold
/// (#731)" in
/// `docs/science/planet-atmospheres.md`). A warm balance (a pair of roots)
/// narrower than a rising step, at most `widest` of the CO₂, or hidden before
/// the secant's zero on a falling `g`, can be stepped over: a limit accepted
/// on 2026-11-27 for the speed, and held by the convergence test of
/// [`super::COLLAPSE_STEP`].
///
/// Stepping over a pocket (a warm balance: an upper, stable root and a
/// lower, unstable one, where `g` falls with the CO₂) can land within the
/// noise above its unstable root, or within the tolerance below it, where
/// the search would stop on a balance the plain step never reaches (#755).
/// So a stop reached by a longer step than the plain one, or inside the
/// bracket, is checked (within the evaluation budget `most`) by one more
/// evaluation just above it
/// ([`pocket_above`]): on a pocket the search closes in on its upper root
/// instead, where the plain step stops. On synthetic pockets deeper than
/// twice the noise on the traced 3300 K gap, this takes the stops on an
/// unstable root from 26 of 4287 to none, for one more evaluation a
/// collapse on average.
pub(crate) fn collapse_search<S: Copy>(
    start: f64,
    mut solve: impl FnMut(f64) -> Kept<S>,
    tolerance: f64,
    most: usize,
    widest: f64,
) -> (f64, S, usize, bool) {
    let holds = |c: f64, k: f64| c <= k * (1.0 + tolerance);
    let first = solve(start);
    let (mut c, mut here) = (start, first);
    let mut evaluations = 0;
    if holds(c, here.kept) {
        return (c, here.state, evaluations, true);
    }
    // Whether `g` has risen above its noise since the start.
    let mut left = widest == 0.0 || c - here.kept > here.noise;
    let mut before: Option<(f64, f64)> = None;
    let mut last_step = 0.0_f64;
    let mut above = (c, c - here.kept, here);
    // The point the last step left and whether that step was the plain
    // one ([`pocket_above`] reads it only where `g` is above zero).
    let mut left_from: Option<(f64, f64, Kept<S>)> = None;
    let mut last_plain = true;
    while evaluations < most {
        let g = c - here.kept;
        if left && holds(c, here.kept) {
            // A plain step never passes a root (`φ` grows with `c`); a
            // longer one may have crossed a pocket down to its unstable
            // root (#755).
            if let Some(from) = left_from.filter(|_| !last_plain) {
                evaluations += 1;
                if let Some(pocket) =
                    pocket_above(&mut solve, (c, here.noise), (from.0, from.1), tolerance)
                {
                    return bracketed(
                        from,
                        pocket,
                        &mut solve,
                        &holds,
                        tolerance,
                        most,
                        evaluations,
                    );
                }
            }
            return (c, here.state, evaluations, true);
        }
        let mut step = g;
        let mut plain = true;
        if widest > 0.0 {
            if let Some((c0, g0)) = before {
                let longer = if g0 - g > here.noise {
                    // Falling by more than the noise (a jitter of the
                    // solve is no fall): to the secant's zero, at most
                    // twice the last step.
                    (g * (c0 - c) / (g0 - g)).min(2.0 * last_step)
                } else {
                    // Rising or flat: doubling, at most `widest` of the
                    // CO₂.
                    (2.0 * last_step).min(widest * c)
                };
                if longer > step && longer < c {
                    step = longer;
                    plain = false;
                }
            }
            if !left && step < here.noise {
                // Within the noise no zero is a balance: step on, by no
                // less than the noise, so as not to crawl onto one.
                step = (2.0 * last_step).max(here.noise).min(widest * c);
                plain = false;
            }
        }
        // The plain step lands on what the surface keeps, bit for bit.
        let x = if plain { here.kept } else { c - step };
        let there = solve(x);
        evaluations += 1;
        let gx = x - there.kept;
        if widest > 0.0 && gx < -there.noise {
            return bracketed(
                above,
                (x, gx),
                &mut solve,
                &holds,
                tolerance,
                most,
                evaluations,
            );
        }
        before = Some((c, g));
        left_from = Some((c, g, here));
        last_plain = plain;
        last_step = step;
        (c, here) = (x, there);
        if gx > there.noise {
            left = true;
        }
        if gx > 0.0 {
            above = (x, gx, there);
        }
    }
    let held = left && holds(c, here.kept);
    (c, here.state, evaluations, held)
}

/// The root of `g` between `high` (`g ≥ 0`) and `low` (`g < 0`) by the
/// Illinois rule, to a point above it that holds or one below it within
/// its noise, where `g` jumps across the root by up to the noise of the
/// solve ([`collapse_search`]).
///
/// The bracket may hold a pocket and the cold root below it: the Illinois
/// rule's points can then come within the tolerance or the noise of the
/// pocket's lower, unstable root. Each such stop is checked by
/// [`pocket_above`] (within the evaluation budget `most`), towards the
/// point above it, with its own `g` rather than the Illinois rule's
/// halved one; on a pocket the search goes on in the bracket between the
/// two, which holds its upper root (#755).
fn bracketed<S: Copy>(
    high: (f64, f64, Kept<S>),
    low: (f64, f64),
    solve: &mut impl FnMut(f64) -> Kept<S>,
    holds: &impl Fn(f64, f64) -> bool,
    tolerance: f64,
    most: usize,
    mut evaluations: usize,
) -> (f64, S, usize, bool) {
    let (mut high, mut low) = (high, low);
    let mut side = 0_i8;
    while evaluations < most && !holds(high.0, high.2.kept) {
        let y = high.0 - high.1 * (high.0 - low.0) / (high.1 - low.1);
        let there = solve(y);
        evaluations += 1;
        let gy = y - there.kept;
        if gy >= 0.0 {
            if holds(y, there.kept) && evaluations < most {
                evaluations += 1;
                if let Some(pocket) = pocket_above(
                    solve,
                    (y, there.noise),
                    (high.0, high.0 - high.2.kept),
                    tolerance,
                ) {
                    (low, side) = (pocket, 0);
                    continue;
                }
            }
            high = (y, gy, there);
            if side == 1 {
                low.1 *= 0.5;
            }
            side = 1;
        } else {
            if gy >= -there.noise {
                if evaluations < most {
                    evaluations += 1;
                    if let Some(pocket) = pocket_above(
                        solve,
                        (y, there.noise),
                        (high.0, high.0 - high.2.kept),
                        tolerance,
                    ) {
                        (low, side) = (pocket, 0);
                        continue;
                    }
                }
                return (y, there.state, evaluations, true);
            }
            low = (y, gy);
            if side == -1 {
                high.1 *= 0.5;
            }
            side = -1;
        }
    }
    let held = holds(high.0, high.2.kept);
    (high.0, high.2.state, evaluations, held)
}

/// Calibration: how far above a stop [`pocket_above`] looks, in noises:
/// where the secant from the stop to the point above it reaches 5 times
/// the stop's noise. On a stable root `g` there is about 5 noises above
/// zero, beyond the ±1 noise a solve's tolerance lets `g` jump by, so the
/// check does not fire there; on a pocket's unstable root it lands inside
/// the pocket. It sets which pockets are seen, never a result where none
/// is. Fitted on 4287 synthetic pockets deeper than twice the noise, cut
/// into the gap traced round a 3300 K star (#755): the stops left on an
/// unstable root are 5, 3, 1, 0 and 0 at 1.5, 2, 3, 5 and 8 noises (26
/// unchecked), at the same cost. Held by the tier-1 test
/// `a_collapse_does_not_stop_on_a_pockets_unstable_root`
/// (`docs/science/planet-atmospheres.md`, "A collapse from the fold").
pub(crate) const POCKET_PROBE_NOISES: f64 = 5.0;

/// Definition: the farthest [`pocket_above`] looks above a stop, as a
/// share of the way to the point above it, 0.5: the probe stays inside
/// the bracket.
const POCKET_PROBE_REACH: f64 = 0.5;

/// Whether a search that would stop at `stop` (its CO₂ and its noise,
/// bar), with `g` within the tolerance above zero or within its noise
/// below, sits on the lower, unstable root of a pocket: one evaluation a
/// little above it, towards `from` (a point above with `g > 0`, its CO₂
/// and `g`). On a stable root `g` rises above it; on an unstable one it
/// falls into the pocket. The point is where the secant from the stop to
/// `from` reaches [`POCKET_PROBE_NOISES`] times the noise, at least
/// `tolerance` of the CO₂ above the stop, and at most
/// [`POCKET_PROBE_REACH`] of the way to `from`. Returns it, with its `g`,
/// when `g` there is below minus its noise: a pocket the stop sits under
/// (#755).
///
/// A pocket narrower than that point's distance, or shallower than the
/// noise, is not seen: the search then stops where it would have.
fn pocket_above<S>(
    solve: &mut impl FnMut(f64) -> Kept<S>,
    (x, noise): (f64, f64),
    (above, g_above): (f64, f64),
    tolerance: f64,
) -> Option<(f64, f64)> {
    let span = above - x;
    if span <= 0.0 || g_above <= 0.0 {
        return None;
    }
    let h = (POCKET_PROBE_NOISES * noise * span / g_above)
        .max(tolerance * x)
        .min(POCKET_PROBE_REACH * span);
    let u = x + h;
    let there = solve(u);
    let gu = u - there.kept;
    (gu < -there.noise).then_some((u, gu))
}

/// [`wet_climate`] of a body whose CO₂ has built up with no thermostat to
/// hold it (a snowball's, or one past its maximum greenhouse): where its
/// surface is colder than the point at which the air's CO₂ condenses
/// ([`co2_saturation_pressure`]), the CO₂ frosts out until the air keeps
/// what its surface allows, the surface cooling with it. Steps from the
/// CO₂ of `air` down to that of its surface ([`collapse_search`], steps of
/// at most the numerics' `collapse_step` of the CO₂), until the air's CO₂
/// exceeds what its surface keeps by under the numerics'
/// `collapse_tolerance`, at most `collapse_iterations` steps: they
/// converge on the warmest balance whose gap crosses zero by more than
/// the solve's noise, the one a cooling planet reaches (the cold trap at
/// the surface the climate solves).
pub(crate) fn collapsed_climate(
    body: &mut ClimateBody,
    starlight: WattPerSquareMetre,
    air: &Weights,
    emissivity: Fraction,
) -> Collapsed {
    let numerics = body.numerics;
    let within = numerics.solver.tolerance;
    let (_, (kept_air, t), iterations, held) = collapse_search(
        air.co2,
        |co2| {
            let next = Weights { co2, ..*air };
            let (kept_air, t) = wet_climate(body, starlight, &next, emissivity);
            // The CO₂ weight whose partial pressure is the saturation,
            // over the air's other gases.
            let others = Weights {
                co2: 0.0,
                ..kept_air
            };
            let holds = |t: Kelvin| {
                Weights::weight_at(
                    &others,
                    CO2_MOLAR_MASS,
                    Bar::from(co2_saturation_pressure(t)),
                )
                .0
            };
            let kept = holds(t);
            let warmer = holds(Kelvin(t.0 + within.0));
            Kept {
                kept,
                noise: (warmer - kept).abs(),
                state: (kept_air, t),
            }
        },
        numerics.collapse_tolerance,
        numerics.collapse_iterations,
        numerics.collapse_step,
    );
    if !held {
        body.collapse_capped = true;
    }
    Collapsed {
        air: kept_air,
        temperature: t,
        iterations,
        held,
    }
}
