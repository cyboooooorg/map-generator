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
use science::radiation::gases::PartialPressures;
use science::surface::surface_vapour;
use science::units::{Bar, Fraction, Kelvin, WattPerSquareMetre};
use science::volatiles::co2_saturation_pressure;
use science::water::CRITICAL_POINT_TEMPERATURE;

/// `air` with the vapour a surface at `t` holds of `water`
/// ([`surface_vapour`]).
fn with_vapour(air: &PartialPressures, t: f64, water: Bar) -> PartialPressures {
    PartialPressures {
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
    air: &PartialPressures,
    emissivity: Fraction,
) -> Option<(PartialPressures, Kelvin)> {
    let numerics = body.numerics;
    let water = body.water_column();
    let (heat, gravity) = (body.inputs.heat_flux, body.inputs.gravity);
    // The dry air absorbs least: its vapour only dilutes the methane whose
    // stratosphere takes a share of the light (#681).
    let dry = PartialPressures { h2o: 0.0, ..*air };
    let least = absorbed(starlight, &dry, heat).0;
    let skin = skin(starlight);
    let e = emissivity.get().clamp(LEAST_EMISSIVITY, 1.0);
    let bare = (least / (e * STEFAN_BOLTZMANN)).powf(0.25);
    if bare >= CRITICAL_POINT_TEMPERATURE.0 {
        return None;
    }
    let columns = &mut body.columns;
    let mut excess_of = |t: f64, profile: Profile| {
        let gases = with_vapour(air, t, water);
        e * outgoing_longwave(&gases, Kelvin(t), skin, gravity, profile, columns).0
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
    air: &PartialPressures,
    emissivity: Fraction,
) -> (PartialPressures, Kelvin) {
    if let Some(balance) = wet_balance(body, starlight, air, emissivity) {
        return balance;
    }
    let steam = PartialPressures {
        h2o: body.water_column().0.max(0.0),
        ..*air
    };
    let i = body.inputs;
    let t = surface_temperature(
        starlight,
        &steam,
        i.heat_flux,
        emissivity,
        i.gravity,
        body.numerics.solver,
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
    air: &PartialPressures,
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

/// A collapsed climate: its air, its temperature, and the steps it took
/// to settle (under the numerics' `collapse_iterations` when it stopped
/// by its tolerance).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Collapsed {
    pub air: PartialPressures,
    pub temperature: Kelvin,
    pub iterations: usize,
}

/// [`wet_climate`] of a body whose CO₂ has built up with no thermostat to
/// hold it (a snowball's, or one past its maximum greenhouse): where its
/// surface is colder than the point at which the air's CO₂ condenses
/// ([`co2_saturation_pressure`]), the CO₂ frosts out until the air keeps
/// what its surface allows, the surface cooling with it. Steps from the
/// CO₂ of `air` down to that of its surface, until the air's CO₂ exceeds
/// what its surface keeps by under the numerics' `collapse_tolerance`, at
/// most `collapse_iterations` steps: they converge on the warmest such
/// balance, the one a cooling planet reaches (the cold trap at the
/// surface the climate solves).
pub(crate) fn collapsed_climate(
    body: &mut ClimateBody,
    starlight: WattPerSquareMetre,
    air: &PartialPressures,
    emissivity: Fraction,
) -> Collapsed {
    let numerics = body.numerics;
    let (mut kept_air, mut t) = wet_climate(body, starlight, air, emissivity);
    let mut iterations = 0;
    while iterations < numerics.collapse_iterations {
        let kept = Bar::from(co2_saturation_pressure(t)).0;
        if kept_air.co2 <= kept * (1.0 + numerics.collapse_tolerance) {
            break;
        }
        let next = PartialPressures { co2: kept, ..*air };
        (kept_air, t) = wet_climate(body, starlight, &next, emissivity);
        iterations += 1;
    }
    Collapsed {
        air: kept_air,
        temperature: t,
        iterations,
    }
}
