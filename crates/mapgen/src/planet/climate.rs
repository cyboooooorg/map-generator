//! The surface climate of a rocky or icy body and the liquid on its
//! surface (part 2 of the physical planet model, issue #423).
//!
//! [`surface_temperature_k`] warms the equilibrium temperature through a
//! grey greenhouse of CO₂, H₂O and CH₄; [`climate`] adds the water vapour
//! that an ocean or ice keeps in the air, runs the carbonate thermostat of
//! a planet with plates and water, and reads the liquid at the surface off
//! the phase diagrams of water, hydrocarbons and basalt.
use serde::{Deserialize, Serialize};

use super::{Context, Gases, Water};
use crate::geology::Regime;

/// The liquid that covers part of a surface, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceLiquid {
    None,
    Water,
    Hydrocarbons,
    Lava,
}

/// The climate at a body's surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Climate {
    /// Mean surface temperature, K.
    pub temperature_k: f64,
    pub liquid: SurfaceLiquid,
    /// The carbonate thermostat holds the temperature at
    /// [`THERMOSTAT_K`]: plates, liquid water and enough CO₂ to reach it.
    pub regulated: bool,
}

/// Stefan–Boltzmann constant, W/(m²·K⁴) (CODATA 2018, exact).
const SIGMA: f64 = 5.670_374_419e-8;

/// Grey opacity of CO₂ at 1 bar of CO₂ in 1 bar of air. Calibration, no
/// source: Venus's 737 K under its 85 bar of CO₂ (with
/// [`CO2_EXPONENT`]); it also leaves Mars about 3 K above its
/// equilibrium temperature.
const CO2_OPACITY: f64 = 1.66;

/// Power of the CO₂ partial pressure in its opacity. Calibration, no
/// source: CO₂'s 15 µm band is saturated at its centre, so its forcing
/// grows with the logarithm of its amount (Myhre et al. 1998,
/// doi:10.1029/98GL01908, `ΔF = 5.35 ln(C/C₀)` W/m²); a small power
/// stands in for that logarithm across the six decades from Earth's
/// 3e-4 bar to Venus's 85 bar.
const CO2_EXPONENT: f64 = 0.25;

/// Grey opacity of water vapour at 1 bar in 1 bar of air, with
/// [`LINE_EXPONENT`]. Calibration, no source: with Earth's vapour at
/// 288 K, the thermostat leaves about 3e-4 bar of CO₂ in its air (the
/// pre-industrial 280 ppm) and the vapour makes most of the greenhouse.
const H2O_OPACITY: f64 = 5.9;

/// Grey opacity of methane at 1 bar in 1 bar of air, with
/// [`LINE_EXPONENT`]. Calibration, no source: Titan's 94 K under its
/// 1.5 % of methane in 1.5 bar (2.1 bar in the model). Titan's real
/// greenhouse is mostly collision-induced (N₂–N₂, N₂–CH₄, N₂–H₂; McKay,
/// Pollack & Courtin 1991, doi:10.1126/science.11538492, not checked),
/// and its haze cools it by about 9 K; this opacity carries all of it.
const CH4_OPACITY: f64 = 3.4;

/// Power of a partial pressure, and of the total pressure, in a gas's
/// opacity: the strong-line (square-root) limit, where a pressure-
/// broadened line absorbs as `sqrt(S u α)`, its absorber path `u ∝ p_i`
/// and its width `α ∝ P` (Pierrehumbert 2010, *Principles of Planetary
/// Climate*, §4.4; Goody & Yung 1989, *Atmospheric Radiation*, ch. 4).
const LINE_EXPONENT: f64 = 0.5;

/// Grey infrared optical depth of the air: each absorber's
/// `k_i (p_i / 1 bar)^a_i (P / 1 bar)^½`, the last factor the pressure
/// broadening of its lines by the whole air (see [`LINE_EXPONENT`]).
/// Pressures in bar.
fn optical_depth(gases: &Gases) -> f64 {
    let broadening = gases.total().max(0.0).powf(LINE_EXPONENT);
    let line =
        |opacity: f64, partial: f64, exponent: f64| opacity * partial.max(0.0).powf(exponent);
    broadening
        * (line(CO2_OPACITY, gases.co2, CO2_EXPONENT)
            + line(H2O_OPACITY, gases.h2o, LINE_EXPONENT)
            + line(CH4_OPACITY, gases.ch4, LINE_EXPONENT))
}

/// Mean surface temperature, K, of a body with `gases` (bar) under its
/// star's light, plus its internal `heat_flux_w_m2`.
///
/// A grey atmosphere in radiative equilibrium (the Eddington
/// approximation, Pierrehumbert 2010, *Principles of Planetary Climate*,
/// §4.6): `T⁴ = T_eq⁴ (1 + ¾ τ) + q/σ`, with `T_eq` the context's
/// equilibrium temperature (Bond albedo 0.3) and `τ` the infrared optical
/// depth of `optical_depth`, whose opacities are calibrated on Venus,
/// Earth and Titan. Without air, the surface sits at `T_eq` warmed by its
/// internal heat alone.
pub fn surface_temperature_k(context: &Context, gases: &Gases, heat_flux_w_m2: f64) -> f64 {
    let tau = optical_depth(gases);
    let t_eq4 = context.equilibrium_k.max(0.0).powi(4);
    (t_eq4 * (1.0 + 0.75 * tau) + heat_flux_w_m2.max(0.0) / SIGMA).powf(0.25)
}

/// Temperature, K, at which the carbonate thermostat holds a planet with
/// plates and liquid water: weathering draws CO₂ down faster when warm,
/// and volcanoes give it back (Walker, Hays & Kasting 1981,
/// doi:10.1029/JC086iC10p09776). Calibration, no source: Earth's mean
/// surface temperature, 288 K.
pub const THERMOSTAT_K: f64 = 288.0;

/// Relative humidity of the air above an ocean or ice: the fixed 77 % of
/// Manabe & Wetherald 1967 (doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2,
/// their surface value; from memory, not checked).
const RELATIVE_HUMIDITY: f64 = 0.77;

/// Triple point of water, K and bar (IAPWS: 273.16 K, 611.657 Pa).
const WATER_TRIPLE_K: f64 = 273.16;
const WATER_TRIPLE_BAR: f64 = 0.006_116_57;

/// Melting point of ice, K (273.15 K at 1 atm; it barely moves with
/// pressure).
const ICE_MELTS_K: f64 = 273.15;

/// Critical point of water, K (IAPWS-95: 647.096 K, 220.64 bar): above
/// it no liquid exists.
const WATER_CRITICAL_K: f64 = 647.096;

/// Latent heat of vaporisation of water over its gas constant
/// (`L / R_v`), K: `L` 2.4e6 J/kg, between its 2.50e6 at 0 °C and 2.26e6
/// at 100 °C, over `R_v` 461.5 J/(kg·K). This mean puts the boiling point
/// at 1 atm at 373 K and the saturation at 288 K within 5 % of the
/// tables.
const WATER_L_OVER_R: f64 = 2.4e6 / 461.5;

/// Saturation vapour pressure of water at `temperature_k`, bar:
/// Clausius–Clapeyron from the triple point with a constant latent heat
/// (Pierrehumbert 2010, §2.1), capped at the critical point. Below
/// 273 K it stands in for the vapour over ice.
fn water_saturation_bar(temperature_k: f64) -> f64 {
    let t = temperature_k.clamp(1.0, WATER_CRITICAL_K);
    WATER_TRIPLE_BAR * (WATER_L_OVER_R * (1.0 / WATER_TRIPLE_K - 1.0 / t)).exp()
}

/// Boiling point of water under `pressure_bar`, K: the same
/// Clausius–Clapeyron inverted, capped at the critical point.
fn water_boils_k(pressure_bar: f64) -> f64 {
    let p = pressure_bar.max(WATER_TRIPLE_BAR);
    let inverse = 1.0 / WATER_TRIPLE_K - (p / WATER_TRIPLE_BAR).ln() / WATER_L_OVER_R;
    if inverse <= 1.0 / WATER_CRITICAL_K {
        WATER_CRITICAL_K
    } else {
        1.0 / inverse
    }
}

/// Triple point of methane, K: the coldest liquid hydrocarbon surface
/// (NIST Chemistry WebBook, 90.69 K; from memory, not checked).
const METHANE_TRIPLE_K: f64 = 90.7;

/// Normal boiling point of ethane, K, at 1 atm (NIST Chemistry WebBook,
/// 184.6 K; from memory, not checked).
const ETHANE_BOILS_K: f64 = 184.6;

/// Enthalpy of vaporisation of ethane at its boiling point over the gas
/// constant, K: 14.7 kJ/mol over 8.314 J/(mol·K) (NIST Chemistry
/// WebBook; from memory, not checked).
const ETHANE_L_OVER_R: f64 = 14_700.0 / 8.314;

/// Critical temperature of ethane, K (305.3 K, NIST; from memory, not
/// checked).
const ETHANE_CRITICAL_K: f64 = 305.3;

/// One standard atmosphere, bar.
const ATM_BAR: f64 = 1.013_25;

/// Boiling point of ethane under `pressure_bar`, K: Clausius–Clapeyron
/// from its normal boiling point, capped at its critical point.
fn ethane_boils_k(pressure_bar: f64) -> f64 {
    let p = pressure_bar.max(1e-12);
    let inverse = 1.0 / ETHANE_BOILS_K - (p / ATM_BAR).ln() / ETHANE_L_OVER_R;
    if inverse <= 1.0 / ETHANE_CRITICAL_K {
        ETHANE_CRITICAL_K
    } else {
        1.0 / inverse
    }
}

/// Temperature, K, above which a rocky surface is molten: the basalt
/// solidus, about 1400 K, the threshold of Kite, Fegley, Schaefer &
/// Gaidos 2016 (doi:10.3847/0004-637X/828/2/80) for a magma surface on
/// hot rocky exoplanets (value from the plan, not checked against the
/// paper).
pub const LAVA_K: f64 = 1400.0;

/// The liquid at a surface at `temperature_k` under `gases`, from the
/// phase diagrams: lava above [`LAVA_K`]; water between the melting of
/// ice and its boiling point at the surface pressure, above its triple
/// point pressure, when water is kept; hydrocarbons between methane's
/// triple point and ethane's boiling point, with methane in the air;
/// none otherwise.
fn liquid(temperature_k: f64, gases: &Gases, water: &Water) -> SurfaceLiquid {
    let pressure = gases.total();
    if temperature_k >= LAVA_K {
        SurfaceLiquid::Lava
    } else if water.kept_kg > 0.0
        && pressure >= WATER_TRIPLE_BAR
        && temperature_k >= ICE_MELTS_K
        && temperature_k < water_boils_k(pressure)
    {
        SurfaceLiquid::Water
    } else if gases.ch4 > 0.0
        && temperature_k >= METHANE_TRIPLE_K
        && temperature_k < ethane_boils_k(pressure)
    {
        SurfaceLiquid::Hydrocarbons
    } else {
        SurfaceLiquid::None
    }
}

/// `gases` with the water vapour an ocean or ice keeps at
/// `temperature_k`: [`RELATIVE_HUMIDITY`] of saturation.
fn with_vapour(gases: Gases, temperature_k: f64) -> Gases {
    Gases {
        h2o: RELATIVE_HUMIDITY * water_saturation_bar(temperature_k),
        ..gases
    }
}

/// Iterations of the water-vapour feedback and of the thermostat's
/// bisection: enough for 1e-3 K and a CO₂ to 2⁻⁶⁰ of its range.
const ITERATIONS: usize = 60;

/// Temperature, K, and gases of a surface whose water vapour follows its
/// temperature. `T ← T(gases with the vapour at T)` from the dry
/// temperature: the map rises with `T`, so the iteration climbs to the
/// coldest balance (the one a cooling planet reaches); the vapour is
/// capped at water's critical point, so it stays bounded.
fn wet_climate(context: &Context, gases: Gases, heat_flux_w_m2: f64) -> (Gases, f64) {
    let dry = Gases { h2o: 0.0, ..gases };
    let mut t = surface_temperature_k(context, &dry, heat_flux_w_m2);
    for _ in 0..ITERATIONS * 4 {
        let next = surface_temperature_k(context, &with_vapour(gases, t), heat_flux_w_m2);
        let done = (next - t).abs() < 1e-3;
        t = next;
        if done {
            break;
        }
    }
    (with_vapour(gases, t), t)
}

/// The climate of a body and its gases once the climate has set its CO₂
/// and water vapour.
///
/// - In a runaway greenhouse, or without water, the gases are as given
///   (a runaway's steam is already in them) and the temperature is
///   [`surface_temperature_k`]'s.
/// - With water kept, its vapour follows the temperature (77 % of saturation),
///   solved together with it.
/// - With plates as well, the carbonate thermostat (Walker, Hays & Kasting
///   1981, doi:10.1029/JC086iC10p09776) sets the CO₂ by bisection between
///   none and all that is outgassed (`gases.co2`, never more) so that the
///   temperature is [`THERMOSTAT_K`], the vapour taken at that
///   temperature. If all the CO₂ is not enough, it stays in the air and
///   the planet is colder (weathering stops on a frozen planet, so the
///   CO₂ accumulates); if none is still too warm, the planet is hotter.
///   Only a temperature reached is `regulated`.
///
/// The liquid follows from the temperature and pressure (see
/// [`LAVA_K`] and the phase diagrams of water and hydrocarbons).
pub fn climate(
    context: &Context,
    gases: Gases,
    water: &Water,
    regime: Regime,
    heat_flux_w_m2: f64,
) -> (Gases, Climate) {
    let wet = !water.runaway && water.kept_kg > 0.0;
    let (gases, temperature_k, regulated) = if !wet {
        let t = surface_temperature_k(context, &gases, heat_flux_w_m2);
        (gases, t, false)
    } else if regime == Regime::Plates {
        thermostat(context, gases, heat_flux_w_m2)
    } else {
        let (g, t) = wet_climate(context, gases, heat_flux_w_m2);
        (g, t, false)
    };
    let liquid = liquid(temperature_k, &gases, water);
    (
        gases,
        Climate {
            temperature_k,
            liquid,
            regulated,
        },
    )
}

/// The carbonate thermostat: the CO₂ in `[0, gases.co2]` that puts the
/// surface at [`THERMOSTAT_K`], by bisection (see [`climate`]). Returns
/// the gases, the temperature and whether it was reached.
fn thermostat(context: &Context, gases: Gases, heat_flux_w_m2: f64) -> (Gases, f64, bool) {
    let outgassed = gases.co2.max(0.0);
    let at_target = with_vapour(gases, THERMOSTAT_K);
    let temperature =
        |co2: f64| surface_temperature_k(context, &Gases { co2, ..at_target }, heat_flux_w_m2);
    if temperature(outgassed) < THERMOSTAT_K {
        let (g, t) = wet_climate(
            context,
            Gases {
                co2: outgassed,
                ..gases
            },
            heat_flux_w_m2,
        );
        return (g, t, false);
    }
    if temperature(0.0) > THERMOSTAT_K {
        let (g, t) = wet_climate(context, Gases { co2: 0.0, ..gases }, heat_flux_w_m2);
        return (g, t, false);
    }
    let (mut low, mut high) = (0.0, outgassed);
    for _ in 0..ITERATIONS {
        let mid = 0.5 * (low + high);
        if temperature(mid) < THERMOSTAT_K {
            low = mid;
        } else {
            high = mid;
        }
    }
    let regulated = Gases {
        co2: high,
        ..at_target
    };
    (regulated, temperature(high), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::PlanetInputs;
    use crate::planet::reference::{Derived, derive_at, model_inputs, reference};

    /// The climate of `inputs` at `orbit_au`, with the chain's gases.
    fn climate_at(inputs: &PlanetInputs, orbit_au: f64) -> (Derived, Gases, Climate) {
        let d = derive_at(inputs, orbit_au);
        let (g, c) = climate(&d.context, d.gases, &d.water, d.regime, d.heat_flux_w_m2);
        (d, g, c)
    }

    /// The climate of the reference body `name`.
    fn climate_of(name: &str) -> (Derived, Gases, Climate) {
        let b = reference(name);
        climate_at(&model_inputs(b), b.orbit_au)
    }

    #[test]
    fn the_solar_system_has_its_climates() {
        let (_, g, earth) = climate_of("Earth");
        assert!(
            (282.0..=294.0).contains(&earth.temperature_k),
            "{earth:?} {g:?}"
        );
        assert_eq!(earth.liquid, SurfaceLiquid::Water, "{earth:?}");
        assert!(earth.regulated, "{earth:?}");
        // The thermostat leaves a trace of CO₂, not tens of bars.
        assert!(g.co2 < 0.01, "{g:?}");

        let (_, g, venus) = climate_of("Venus");
        assert!(
            (687.0..=787.0).contains(&venus.temperature_k),
            "{venus:?} {g:?}"
        );
        assert_eq!(venus.liquid, SurfaceLiquid::None, "{venus:?}");

        let (_, g, mars) = climate_of("Mars");
        assert!(
            (198.0..=222.0).contains(&mars.temperature_k),
            "{mars:?} {g:?}"
        );
        assert_eq!(mars.liquid, SurfaceLiquid::None, "{mars:?}");

        let (_, g, titan) = climate_of("Titan");
        assert!(
            (86.0..=102.0).contains(&titan.temperature_k),
            "{titan:?} {g:?}"
        );
        assert_eq!(titan.liquid, SurfaceLiquid::Hydrocarbons, "{titan:?}");
    }

    #[test]
    fn an_airless_body_sits_at_its_equilibrium_temperature() {
        let (d, g, mercury) = climate_of("Mercury");
        let t_eq = d.context.equilibrium_k;
        assert!(d.heat_flux_w_m2 < 1.0, "{}", d.heat_flux_w_m2);
        assert!(
            (mercury.temperature_k - t_eq).abs() < 1.0,
            "{mercury:?} {t_eq} {g:?}"
        );
        assert_eq!(mercury.liquid, SurfaceLiquid::None);
        assert!(!mercury.regulated);
    }

    #[test]
    fn the_thermostat_never_exceeds_the_outgassed_co2() {
        let earth = model_inputs(reference("Earth"));
        let (d, g, c) = climate_at(&earth, 1.6);
        assert!(g.co2 <= d.gases.co2, "{g:?} {:?}", d.gases);
        if !c.regulated {
            // Short of CO₂: all of it is in the air, and the water is
            // liquid only while the surface stays above freezing.
            assert_eq!(g.co2, d.gases.co2, "{g:?}");
            let liquid = c.liquid == SurfaceLiquid::Water;
            assert_eq!(liquid, c.temperature_k >= 273.15, "{c:?}");
        }
        // At 2 AU the CO₂ the cold leaves in the air is not enough.
        let (d, g, c) = climate_at(&earth, 2.0);
        assert_eq!(g.co2, d.gases.co2, "{g:?}");
        assert!(!c.regulated, "{c:?}");
        assert_eq!(c.liquid, SurfaceLiquid::None, "{c:?}");
        // With a ten-thousandth of its CO₂, the thermostat runs out: it puts
        // all of it in the air, and the water freezes.
        let d = derive_at(&earth, 1.6);
        let scarce = Gases {
            co2: d.gases.co2 * 1e-4,
            ..d.gases
        };
        let (g, c) = climate(&d.context, scarce, &d.water, d.regime, d.heat_flux_w_m2);
        assert!(g.co2 <= scarce.co2, "{g:?}");
        assert!(!c.regulated, "{c:?}");
        assert_eq!(c.liquid, SurfaceLiquid::None, "{c:?}");
    }

    #[test]
    fn no_water_no_ocean() {
        let dry = PlanetInputs {
            water_fraction: 0.0,
            ..model_inputs(reference("Earth"))
        };
        let (_, _, c) = climate_at(&dry, 1.0);
        assert_ne!(c.liquid, SurfaceLiquid::Water, "{c:?}");
        assert!(!c.regulated, "{c:?}");
    }

    #[test]
    fn a_close_rock_is_a_lava_world() {
        let (_, _, c) = climate_at(&model_inputs(reference("Earth")), 0.01);
        assert_eq!(c.liquid, SurfaceLiquid::Lava, "{c:?}");
    }

    #[test]
    fn more_greenhouse_gas_is_warmer() {
        let c = crate::planet::reference::sun_at(1.0);
        let thin = Gases {
            n2: 1.0,
            co2: 1e-3,
            ..Gases::default()
        };
        let thick = Gases { co2: 1.0, ..thin };
        let t_thin = surface_temperature_k(&c, &thin, 0.0);
        let t_thick = surface_temperature_k(&c, &thick, 0.0);
        assert!(
            t_thin > c.equilibrium_k && t_thick > t_thin,
            "{t_thin} {t_thick}"
        );
        // Internal heat warms an airless surface.
        let bare = surface_temperature_k(&c, &Gases::default(), 0.0);
        assert!((bare - c.equilibrium_k).abs() < 1e-9);
        assert!(surface_temperature_k(&c, &Gases::default(), 100.0) > bare);
    }

    #[test]
    fn surface_liquid_is_kebab_case() {
        let s = serde_json::to_string(&SurfaceLiquid::Hydrocarbons).unwrap();
        assert_eq!(s, "\"hydrocarbons\"");
    }
}
