//! The branches of a body's climate and the state it is found in: the
//! temperate branch (the fixed point started warm), the frozen branch
//! beside it where the same CO₂ also leaves the body frozen from an icy
//! start, the CO₂ that thaws it, the chance that the body is found on
//! it, and the draw that decides.
//!
//! Ported from `legacy-final:crates/mapgen/src/planet/climate.rs`
//! (`climate_over`, l.1475-1525; `frozen_over`, l.1550-1554;
//! `FrozenBranch` and `branches`, l.1861-1935), with the snowball chance
//! of `science::snowball`, "frozen over" read on the melting curve of the
//! stable ice (#671), the state `Frozen` for a body frozen for good, and
//! a thaw search that stops where the icy start first cycles (#731).

use super::ClimateInputs;
use super::Numerics;
use super::body::ClimateBody;
use super::settle::{Settle, frozen_at, settle, warm_start};
use science::constants::PASCALS_PER_BAR;
use science::snowball::snowball_chance;
use science::surface::Liquid;
use science::units::{Bar, Fraction, Pascal};
use science::water::melting_temperature;
use timeline::ClimateState;

/// Definition: the floor of the thaw's bisection in `ln CO₂`, as a share
/// of the most CO₂ the body holds, 1e-12 (legacy `branches`, l.1916): it
/// keeps the logarithm finite where the temperate branch froze with no
/// CO₂ in its air. [`frozen_branch`] checks that its icy start stays
/// frozen there (#758).
pub(crate) const THAW_FLOOR: f64 = 1e-12;

/// A frozen branch beside the temperate one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bistable {
    /// The CO₂ the temperate branch holds, with which it also freezes.
    pub froze: Bar,
    /// The most CO₂ known to leave the body frozen, in a frozen balance:
    /// just under the thaw (within the numerics' `thaw_steps`, or the
    /// last CO₂ that stayed frozen before the first that cycles), `froze`
    /// itself when the search's floor already thaws it (#758), or its
    /// maximum greenhouse when that never thaws it. The CO₂ a snowball
    /// holds.
    pub still_frozen: Bar,
    /// The least CO₂ known to thaw it, or at which its icy start first
    /// cycles ([`thaw_search`]); `None` when no CO₂ it can hold does.
    pub thaw: Option<Bar>,
    /// The share of its history it spends frozen ([`snowball_chance`]
    /// of `thaw − froze` and its volcanism).
    pub chance: Fraction,
}

/// A solved climate: the settled climate the body is found in, its
/// state, and its frozen branch where it has one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Solved {
    /// The climate the body is found in: the temperate branch, or the
    /// frozen one at its `still_frozen` CO₂ when the draw falls under
    /// the chance.
    pub settle: Settle,
    /// Its state.
    pub state: ClimateState,
    /// Its frozen branch, where it is bistable.
    pub bistable: Option<Bistable>,
}

/// What an icy start comes to at a given CO₂ ([`frozen_at`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IcyStart {
    /// It settles frozen over: a frozen balance.
    StaysFrozen,
    /// It has no fixed point and cycles ([`Settle::cycles`]): stored on
    /// its cold side, but no frozen balance holds there.
    Cycles,
    /// It settles thawed.
    Thaws,
}

/// What the icy start of `body` comes to at `co2`.
fn icy_start(body: &mut ClimateBody, co2: Bar) -> IcyStart {
    let climate = frozen_at(body, co2);
    if !frozen_over(&climate) {
        IcyStart::Thaws
    } else if climate.cycles {
        IcyStart::Cycles
    } else {
        IcyStart::StaysFrozen
    }
}

/// The thaw search between `low` bar, where the icy start stays frozen,
/// and `high`, where it does not: `steps` bisections in `ln CO₂` of
/// `icy` (legacy `branches`, l.1916). Returns the most CO₂ known to stay
/// frozen and the least known not to.
///
/// It stops at the first CO₂ whose icy start cycles (#731): a snowball
/// there has no frozen balance and swings to its warm side, so that CO₂
/// is its thaw, and the last CO₂ that stayed frozen is the snowball's.
/// The band where it cycles sits between the frozen and the thawed CO₂,
/// and each call in it costs the most albedo steps; its bottom is known
/// only to the bracket the search had reached (0.8 % of a methane
/// Earth's thaw round a 4500 K star: `docs/science/planet-atmospheres.md`,
/// "A thaw search that meets a cycle").
pub(crate) fn thaw_search(
    low: f64,
    high: f64,
    steps: usize,
    mut icy: impl FnMut(f64) -> IcyStart,
) -> (f64, f64) {
    let (mut low, mut high) = (low, high);
    for _ in 0..steps {
        let mid = (low * high).sqrt();
        match icy(mid) {
            IcyStart::StaysFrozen => low = mid,
            IcyStart::Thaws => high = mid,
            IcyStart::Cycles => return (low, mid),
        }
    }
    (low, high)
}

/// The frozen branch beside a temperate one that froze with `froze`
/// bar of CO₂, from its icy starts (`icy`, read on `context`): `None`
/// when the start at `froze` does not stay frozen (no frozen branch);
/// otherwise the most CO₂ known to stay frozen and the least known not
/// to, `None` when the start at its maximum greenhouse (`most`, read only
/// then) stays frozen too ([`thaw_search`] in between, from [`THAW_FLOOR`]
/// of that maximum at least). An icy start that cycles counts as not
/// frozen at both ends too: it holds no frozen balance (#731).
///
/// Where the floor lies above `froze` (a temperate branch that froze with
/// almost no CO₂), the start at the floor is read too, since the
/// bisection holds only if its low end stays frozen (#758): if it does
/// not, the floor is the thaw and `froze` the most CO₂ known to stay
/// frozen. At that share of the maximum greenhouse, the floor's CO₂ has
/// no measurable greenhouse, so this is a guard: no body traced so far
/// (the heavy climate tests, an Earth from 0.80 to 1.00 AU) starts its
/// search from the floor.
pub(crate) fn frozen_branch<C>(
    context: &mut C,
    froze: f64,
    most: impl FnOnce(&mut C) -> f64,
    mut icy: impl FnMut(&mut C, f64) -> IcyStart,
    steps: usize,
) -> Option<(f64, Option<f64>)> {
    if icy(context, froze) != IcyStart::StaysFrozen {
        return None;
    }
    let most = most(context);
    if most <= froze || icy(context, most) == IcyStart::StaysFrozen {
        return Some((most.max(froze), None));
    }
    let low = froze.max(most * THAW_FLOOR);
    if low > froze && icy(context, low) != IcyStart::StaysFrozen {
        // The bisection holds only where its low end stays frozen (#758).
        return Some((froze, Some(low)));
    }
    let (low, high) = thaw_search(low, most, steps, |co2| icy(context, co2));
    Some((low, Some(high)))
}

/// Whether `climate` is frozen over: no liquid water, and a surface under
/// the melting temperature of the stable ice at its pressure
/// ([`melting_temperature`], the curve `liquid` reads).
pub(crate) fn frozen_over(climate: &Settle) -> bool {
    let pressure = Pascal(climate.air.total() * PASCALS_PER_BAR);
    climate.liquid != Some(Liquid::Water) && climate.temperature.0 < melting_temperature(pressure).0
}

/// The temperate branch of `body` and, where it keeps liquid water and
/// the same CO₂ also leaves the body frozen from an icy start
/// ([`frozen_at`]), the frozen one with the CO₂ that thaws it: bisected
/// in `ln CO₂` between the CO₂ it froze with and its maximum greenhouse
/// ([`ClimateBody::greenhouse_co2`]), in the numerics' `thaw_steps`
/// ([`thaw_search`]); none when that maximum never thaws it. An icy
/// start that cycles holds no frozen balance and counts as thawed.
pub(crate) fn branches(body: &mut ClimateBody) -> (Settle, Option<Bistable>) {
    let start = warm_start(body);
    let warm = settle(body, start);
    if !body.wet() || warm.liquid != Some(Liquid::Water) {
        return (warm, None);
    }
    let froze = Bar(warm.air.co2);
    let steps = body.numerics.thaw_steps;
    let Some((still_frozen, thaw)) = frozen_branch(
        body,
        froze.0,
        |body| {
            let inventory = body.inputs.air;
            body.greenhouse_co2(&inventory).0
        },
        |body, co2| icy_start(body, Bar(co2)),
        steps,
    ) else {
        return (warm, None);
    };
    let (still_frozen, thaw) = (Bar(still_frozen), thaw.map(Bar));
    let chance = snowball_chance(thaw.map(|t| Bar(t.0 - froze.0)), body.inputs.volcanism);
    let bistable = Bistable {
        froze,
        still_frozen,
        thaw,
        chance,
    };
    (warm, Some(bistable))
}

/// The climate of `inputs` with the snowball draw `draw` (uniform in
/// (0, 1), `draws::snowball_draw`), under the solver's constants
/// ([`solve_with`] of [`Numerics::DEFAULT`]).
pub(crate) fn solve(inputs: &ClimateInputs, draw: f64) -> Solved {
    solve_with(inputs, draw, &Numerics::DEFAULT)
}

/// The climate of `inputs` with the snowball draw `draw` under
/// `numerics` (legacy `climate_over`): the temperate branch, unless the
/// body is bistable and the draw falls under its chance, when it is the
/// frozen branch at the most CO₂ that leaves it frozen (weathering stops
/// under the ice and the volcanoes build the CO₂ up to just short of the
/// thaw, or to the maximum greenhouse where none thaws it). Its state:
///
/// - `Runaway` when the water ran away and the body keeps some (a dry
///   body past its star's limit has no water to run away: it is
///   temperate, as legacy's dry bodies are);
/// - `Snowball` on the frozen branch, frozen over, with a thaw;
/// - `Frozen` for a body with water, frozen over, which no CO₂ it can
///   hold thaws: on the frozen branch with no thaw, or, with plates,
///   frozen on its only branch (past the outer edge of its carbonate
///   thermostat). Without plates no CO₂ builds up to a thaw and the cold
///   has another cause (legacy `explain.rs`, l.860-863: the `frozen`
///   note needs plates);
/// - `Temperate` otherwise: dry bodies, and bodies with no plates such as
///   hydrocarbon or nitrogen sea worlds, included.
pub(crate) fn solve_with(inputs: &ClimateInputs, draw: f64, numerics: &Numerics) -> Solved {
    let mut body = ClimateBody::new(inputs, *numerics);
    let (warm, bistable) = branches(&mut body);
    let (settle, state) = match bistable {
        _ if inputs.runaway && inputs.water.0 > 0.0 => (warm, ClimateState::Runaway),
        Some(b) if draw < b.chance.get() => {
            let frozen = frozen_at(&mut body, b.still_frozen);
            let state = match (frozen_over(&frozen), b.thaw) {
                (true, Some(_)) => ClimateState::Snowball,
                (true, None) => ClimateState::Frozen,
                (false, _) => ClimateState::Temperate,
            };
            (frozen, state)
        }
        Some(_) => (warm, ClimateState::Temperate),
        None if body.wet() && inputs.plates && frozen_over(&warm) => (warm, ClimateState::Frozen),
        None => (warm, ClimateState::Temperate),
    };
    Solved {
        settle,
        state,
        bistable,
    }
}
