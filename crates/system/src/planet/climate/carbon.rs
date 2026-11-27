//! The carbon cycle of a body with plates: the carbonate thermostat
//! that holds its surface at [`THERMOSTAT_K`] where it can, and the
//! maximum greenhouse that caps the CO₂ that warms it. Ported from
//! `legacy-final:crates/mapgen/src/planet/climate.rs` (`thermostat`,
//! l.2274-2350, and `maximum_greenhouse`, l.2352-2423).

use super::body::ClimateBody;
use super::wet::{collapsed_climate, wet_climate};
use science::balance::{
    LEAST_EMISSIVITY, Point, THERMOSTAT_K, absorbed, regula_falsi, skin, surface_temperature,
};
use science::radiation::column::{Profile, outgoing_longwave};
use science::radiation::gases::Weights;
use science::surface::surface_vapour;
use science::units::{Bar, Fraction, Kelvin, WattPerSquareMetre};

/// Definition: the least CO₂ the thermostat searches from, a share of the
/// most, 2⁻⁶⁰ (legacy `thermostat`, l.2318): the resolution of the
/// 60-step bisection its root search replaced. It bounds the search
/// range, not the root.
const TRACE_CO2_SHARE: f64 = 1.0 / (1u64 << 60) as f64;

/// The air, temperature and regulation of `body` under `starlight` (W/m²,
/// after its albedo), its surface radiating with `emissivity`, its air
/// `air` (its inventory, or what a colder surface leaves of it once its
/// nitrogen and methane condense), when its carbonate thermostat (Walker, Hays & Kasting 1981,
/// doi:10.1029/JC086iC10p09776) sets its CO₂ so that its surface is at
/// [`THERMOSTAT_K`], the vapour taken at that temperature
/// ([`surface_vapour`] of its [`ClimateBody::water_column`]).
///
/// The CO₂ ranges up to its maximum greenhouse
/// ([`ClimateBody::greenhouse_co2`] of `air`: the one searched on the
/// inventory air whatever `air`, #593 lever 2, unless the numerics say
/// `greenhouse_once` is off, when it is searched again on `air` as
/// legacy did):
///
/// - if even that leaves the surface colder than the thermostat, the
///   planet takes it, and on a surface too cold to keep it the CO₂ frosts
///   out ([`collapsed_climate`]): not regulated;
/// - if no CO₂ at all is still too warm, the planet is hotter
///   ([`wet_climate`] without CO₂): not regulated;
/// - otherwise the CO₂ at which a surface at [`THERMOSTAT_K`] sends out
///   what it absorbs, found by [`regula_falsi`] in `ln CO₂` from
///   [`TRACE_CO2_SHARE`] of the most within the numerics'
///   `thermostat_tolerance`, and the temperature [`surface_temperature`]
///   gives that air: regulated.
///
/// Each CO₂ tried, the two ends included, is weighed on its own air: what
/// that air's surface [`absorbed`] (the methane anti-greenhouse takes a
/// share of the light that more CO₂ dilutes) against what its column
/// sends out, the same function [`surface_temperature`] reads. Legacy
/// took the light absorbed once, on `air` (the air's CO₂ before the
/// thermostat, 52 bar for the reference Earth): under methane its
/// regulated surface then fell to the dew point of its vapour, not
/// [`THERMOSTAT_K`] (#681).
pub(crate) fn thermostat(
    body: &mut ClimateBody,
    starlight: WattPerSquareMetre,
    air: &Weights,
    emissivity: Fraction,
) -> (Weights, Kelvin, bool) {
    let most = body.greenhouse_co2(air).0;
    let water = body.water_column();
    let i = body.inputs;
    let numerics = body.numerics;
    let air = *air;
    let at_target = Weights {
        h2o: surface_vapour(THERMOSTAT_K, water).0,
        ..air
    };
    let skin = skin(starlight);
    let e = emissivity.get().clamp(LEAST_EMISSIVITY, 1.0);
    let ozone = i.longwave_ozone();
    let columns = &mut body.columns;
    // What a surface at `THERMOSTAT_K` under `co2` absorbs less what it
    // sends out, both on that air: its methane's stratosphere takes a
    // share of the light that the CO₂ dilutes (#681), and
    // `surface_temperature` reads the same air.
    let mut gain_at = |co2: f64| {
        let gases = Weights { co2, ..at_target }.partial_pressures();
        absorbed(starlight, &gases, i.heat_flux).0
            - e * outgoing_longwave(
                &gases,
                ozone,
                THERMOSTAT_K,
                skin,
                i.gravity,
                Profile::CLIMATE,
                columns,
            )
            .0
    };
    let all = gain_at(most);
    if all <= 0.0 {
        let c = collapsed_climate(body, starlight, &Weights { co2: most, ..air }, emissivity);
        return (c.air, c.temperature, false);
    }
    if gain_at(0.0) > 0.0 {
        let (g, t) = wet_climate(body, starlight, &Weights { co2: 0.0, ..air }, emissivity);
        return (g, t, false);
    }
    let floor = most * TRACE_CO2_SHARE;
    let mut deficit = |ln_co2: f64| gain_at(ln_co2.exp());
    let low = Point {
        x: floor.ln(),
        f: deficit(floor.ln()),
    };
    let co2 = if low.f >= 0.0 {
        floor
    } else {
        let top = Point {
            x: most.ln(),
            f: all,
        };
        regula_falsi(
            deficit,
            low,
            top,
            numerics.thermostat_tolerance,
            numerics.solver.iterations,
        )
        .x
        .exp()
    };
    let regulated = Weights { co2, ..at_target };
    let t = surface_temperature(
        starlight,
        &regulated.partial_pressures(),
        ozone,
        i.heat_flux,
        emissivity,
        i.gravity,
        numerics.solver,
        &mut body.columns,
    );
    (regulated, t, true)
}

/// The CO₂ of the warmest climate of `body` under `air` within the most
/// its air can hold, its carbon inventory and never less than its
/// inventory air holds (Kasting
/// et al. 1993, doi:10.1006/icar.1993.1010, the maximum greenhouse): the
/// amount at which a surface at [`THERMOSTAT_K`] gains most, the light
/// it absorbs under the albedo that CO₂ derives
/// ([`ClimateBody::derived_albedo`], its Rayleigh scattering) against what
/// its column sends out under the [`skin`] of that light. A golden-section
/// search in `ln CO₂` over the numerics' `greenhouse_decades` under the
/// most, `greenhouse_steps` steps; the end of the range when the gain
/// still grows there. Its other gases are `air`'s, the vapour taken at
/// [`THERMOSTAT_K`] (legacy searched it on the air of the body it was
/// given, a condensed one included, `climate.rs` l.1975-1983). Where CO₂ condenses in the column its greenhouse flattens
/// and its scattering wins: the maximum Kopparapu et al. 2013
/// (doi:10.1088/0004-637X/765/2/131) find near 8 bar.
pub(crate) fn maximum_greenhouse(body: &mut ClimateBody, air: &Weights) -> Bar {
    let i = body.inputs;
    let most = i.carbon.0.max(i.air.co2).max(0.0);
    if most <= 0.0 {
        return Bar(0.0);
    }
    let numerics = body.numerics;
    let water = body.water_column();
    let ozone = i.longwave_ozone();
    let at_target = Weights {
        h2o: surface_vapour(THERMOSTAT_K, water).0,
        ..*air
    };
    let mut gain = |ln_co2: f64| {
        let gases = Weights {
            co2: ln_co2.exp(),
            ..at_target
        };
        let albedo = body.derived_albedo(THERMOSTAT_K, &gases);
        let light = body.starlight(albedo);
        let partial = gases.partial_pressures();
        absorbed(light, &partial, WattPerSquareMetre(0.0)).0
            - outgoing_longwave(
                &partial,
                ozone,
                THERMOSTAT_K,
                skin(light),
                i.gravity,
                Profile::CLIMATE,
                &mut body.columns,
            )
            .0
    };
    // Definition: the golden section's ratio, (√5 − 1) / 2.
    let ratio = (5f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (
        most.ln() - numerics.greenhouse_decades * 10f64.ln(),
        most.ln(),
    );
    let (mut c, mut d) = (b - ratio * (b - a), a + ratio * (b - a));
    let (mut fc, mut fd) = (gain(c), gain(d));
    for _ in 0..numerics.greenhouse_steps {
        if fc < fd {
            a = c;
            (c, fc) = (d, fd);
            d = a + ratio * (b - a);
            fd = gain(d);
        } else {
            b = d;
            (d, fd) = (c, fc);
            c = b - ratio * (b - a);
            fc = gain(c);
        }
    }
    let best = if gain(most.ln()) >= fc.max(fd) {
        most.ln()
    } else {
        (a + b) / 2.0
    };
    Bar(best.exp().min(most))
}
