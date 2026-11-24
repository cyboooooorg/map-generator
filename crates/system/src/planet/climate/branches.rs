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
//! stable ice (#671) and the state `Frozen` for a body frozen for good.

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
/// CO₂ in its air.
const THAW_FLOOR: f64 = 1e-12;

/// A frozen branch beside the temperate one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bistable {
    /// The CO₂ the temperate branch holds, with which it also freezes.
    pub froze: Bar,
    /// The most CO₂ known to leave the body frozen: just under the thaw
    /// (within the numerics' `thaw_steps`), or its maximum greenhouse
    /// when that never thaws it. The CO₂ a snowball holds.
    pub still_frozen: Bar,
    /// The least CO₂ known to thaw it; `None` when no CO₂ it can hold
    /// does.
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
/// ([`ClimateBody::greenhouse_co2`]), in the numerics' `thaw_steps`;
/// none when that maximum never thaws it.
pub(crate) fn branches(body: &mut ClimateBody) -> (Settle, Option<Bistable>) {
    let start = warm_start(body);
    let warm = settle(body, start);
    if !body.wet() || warm.liquid != Some(Liquid::Water) {
        return (warm, None);
    }
    let froze = Bar(warm.air.co2);
    let thawed = |body: &mut ClimateBody, co2: Bar| !frozen_over(&frozen_at(body, co2));
    if thawed(body, froze) {
        return (warm, None);
    }
    let inventory = body.inputs.air;
    let most = body.greenhouse_co2(&inventory);
    let (still_frozen, thaw) = if most.0 <= froze.0 || !thawed(body, most) {
        (Bar(most.0.max(froze.0)), None)
    } else {
        // Frozen at `low`, thawed at `high`.
        let (mut low, mut high) = (froze.0.max(most.0 * THAW_FLOOR), most.0);
        for _ in 0..body.numerics.thaw_steps {
            let mid = (low * high).sqrt();
            if thawed(body, Bar(mid)) {
                high = mid;
            } else {
                low = mid;
            }
        }
        (Bar(low), Some(Bar(high)))
    };
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
