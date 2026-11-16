//! The ice shell of a frozen body over its water layer, and the ocean
//! left beneath it ([`ice_shell`]): the shell conducts the internal heat
//! out through ice whose conductivity rises as it gets colder, thinner
//! where the tides heat it most; a shell thick enough convects under a
//! stagnant lid, and when convection carries out more heat than comes in,
//! the ocean freezes until the ammonia it concentrates stops it, or
//! entirely. The water below the shell's base stays liquid down to where
//! the pressure turns it to high-pressure ice.
use serde::{Deserialize, Serialize};

use std::f64::consts::PI;

use super::{Interior, InternalHeat, Material, Surface, SurfaceLiquid};
use crate::exomoon::{EARTH_KG, EARTH_M, G};

/// The conductivity of water ice times its temperature, W/m: `k = 651 /
/// T` W m⁻¹ K⁻¹ (Petrenko & Whitworth 1999, *Physics of Ice*, Oxford
/// University Press; Klinger 1980 gives 567 / T), the
/// law thermal models of icy satellites' shells use (Hussmann, Sohl &
/// Spohn 2006, doi:10.1016/j.icarus.2006.06.005).
pub const ICE_CONDUCTIVITY_W_M: f64 = 651.0;

/// Melting point of pure water ice, K: the temperature at the base of a
/// shell over an ocean without ammonia, and [`liquidus_k`] at none.
/// Calibration, no source, for 273 rather than 273.15. Salts, which
/// would lower it, are left out.
pub const MELT_K: f64 = 273.0;

/// Ammonia mass fraction of the H₂O–NH₃ eutectic, about a third
/// (Leliwa-Kopystyński, Maruyama & Nakajima 2002,
/// doi:10.1006/icar.2001.6762, about 0.32 to 0.33): a liquid richer in
/// ammonia than this freezes no colder, so [`liquidus_k`] holds there.
pub const NH3_EUTECTIC: f64 = 0.33;

/// Coefficients of the liquidus of [`liquidus_k`], K per power of the
/// ammonia mass fraction, from the first.
const LIQUIDUS_NH3: [f64; 5] = [-53.07, -1651.4, 11842.0, -46269.0, 56695.0];

/// Temperature, K, at which ice melts into an ocean holding
/// `nh3_mass_fraction` of ammonia (kg of NH₃ per kg of liquid): the
/// water-rich branch of the H₂O–NH₃ liquidus at low pressure, [`MELT_K`]
/// plus `Σ aₙ Xⁿ` for `n` from 1 to 5 with the coefficients
/// [`LIQUIDUS_NH3`], the fit of Leliwa-Kopystyński, Maruyama & Nakajima
/// 2002 (doi:10.1006/icar.2001.6762) to the measured phase diagram (the
/// coefficients as later thermal models of icy satellites quote them,
/// not checked against the paper's table; the pressure's own lowering of
/// the melting point is left out). It falls from
/// 273 K for pure water through about 259 K at 10 % to about 175 K at
/// the eutectic ([`NH3_EUTECTIC`], 176 K measured), where the fraction
/// is held. A fraction under 0 or not a number counts as pure water.
pub fn liquidus_k(nh3_mass_fraction: f64) -> f64 {
    let x = if nh3_mass_fraction.is_nan() {
        0.0
    } else {
        nh3_mass_fraction.clamp(0.0, NH3_EUTECTIC)
    };
    let depression = LIQUIDUS_NH3.iter().rev().fold(0.0, |acc, a| (acc + a) * x);
    MELT_K + depression
}

/// Pressure, Pa, at which the ocean ends on high-pressure ice: the ice
/// III–ice V–liquid triple point, 350.1 MPa (256.164 K), of the IAPWS
/// melting curves (Wagner, Riethmann, Feistel & Harvey 2011,
/// doi:10.1063/1.3657937). Deeper, liquid water near its melting point
/// freezes into ice V; between the ice Ih–III–liquid triple point
/// (208.566 MPa, 251.165 K) and this one, ice III forms only in an
/// ocean colder than about 256 K, which the model does not resolve.
pub const HIGH_PRESSURE_ICE_PA: f64 = 350.1e6;

/// Density of the ice shell, kg/m³: ice Ih near its melting point
/// (Feistel & Wagner 2006, doi:10.1063/1.2183324, about 917 kg/m³ at
/// 273 K and 1 bar; the shell's compression and cooling are neglected).
const ICE_DENSITY: f64 = 917.0;

/// Thermal expansivity of ice Ih near its melting point, 1/K (Feistel &
/// Wagner 2006, doi:10.1063/1.2183324, about 1.6e-4 at 273 K and 1 bar).
const ICE_EXPANSIVITY: f64 = 1.6e-4;

/// Specific heat of ice Ih near its melting point, J/(kg·K) (Feistel &
/// Wagner 2006, doi:10.1063/1.2183324, about 2097 at 273 K and 1 bar).
const ICE_HEAT_CAPACITY: f64 = 2097.0;

/// Density of the ocean, kg/m³: liquid water, rounded. Calibration, no
/// source: compression and dissolved salts would raise it by a few
/// percent.
const OCEAN_DENSITY: f64 = 1000.0;

/// Depth step, m, of the hydrostatic integration of [`ocean_floor_m`].
const DEPTH_STEP_M: f64 = 100.0;

/// Molar gas constant, J/(mol·K).
const GAS_CONSTANT: f64 = 8.314_462_618;

/// Activation energy of volume diffusion in ice Ih, J/mol, the
/// diffusion creep of [`ice_viscosity_pa_s`] (Goldsby & Kohlstedt 2001,
/// doi:10.1029/2000JB900336, table 3: 59.4 kJ/mol).
const ICE_ACTIVATION_J_MOL: f64 = 59.4e3;

/// Pre-exponential factor of volume diffusion in ice Ih, m²/s (Goldsby &
/// Kohlstedt 2001, doi:10.1029/2000JB900336, table 3).
const ICE_DIFFUSION_M2_S: f64 = 9.10e-4;

/// Molar volume of ice Ih, m³/mol (Goldsby & Kohlstedt 2001,
/// doi:10.1029/2000JB900336, table 3).
const ICE_MOLAR_VOLUME_M3: f64 = 1.97e-5;

/// Grain size of the ice of a shell, m: 30 mm, the low end of the 30 to
/// 80 mm to which dynamic recrystallisation brings the grains of a
/// convecting ice shell of a large icy satellite (Barr & McKinnon 2007,
/// doi:10.1029/2006JE002781, their plausible upper limit). Calibration
/// within that range: the viscosity goes as its square. At 1 mm (about
/// 10¹⁴ Pa s at the melting point, the value of many Europa models), a
/// Ganymede's and a Europa's shells would convect from about 30 km and,
/// without ammonia, freeze their oceans, which both moons keep.
const GRAIN_SIZE_M: f64 = 0.03;

/// Coefficient of the onset of stagnant-lid convection: a layer whose
/// basal Rayleigh number exceeds `20.9 θ⁴` convects, `θ` its
/// Frank-Kamenetskii parameter (Solomatov 1995, doi:10.1063/1.868624, for
/// a Newtonian fluid whose viscosity falls exponentially with
/// temperature; the criterion Barr & Pappalardo 2005,
/// doi:10.1029/2004JE002371, apply to the shells of the icy Galilean
/// satellites, not checked).
const ONSET_COEFFICIENT: f64 = 20.9;

/// Coefficient of the heat flux of stagnant-lid convection in a Newtonian
/// fluid (Solomatov & Moresi 2000, doi:10.1029/2000JE001245, about 0.53),
/// in [`convective_flux_w_m2`].
const CONVECTIVE_COEFFICIENT: f64 = 0.53;

/// Temperature drop across the rheological sublayer at the base of a
/// stagnant lid, in units of `R T_i² / Q` (Solomatov & Moresi 2000,
/// doi:10.1029/2000JE001245, about 2.4 for a Newtonian fluid, not
/// checked): the lid conducts down to `T_i − 2.4 R T_i² / Q`.
const RHEOLOGICAL_DROP: f64 = 2.4;

/// Latitude bands, from the equator to a pole at equal area, over which
/// [`ice_shell`] averages the shell thickness.
const BANDS: usize = 32;

/// Coefficients `a₀₀`, `a₂₀`, `a₄₀` (times e²) of the zonal parts of the
/// eccentricity tide's angular functions `Ψ₀`, `Ψ₂`, `Ψ₄` for a
/// synchronous moon (Beuthe 2013, doi:10.1016/j.icarus.2012.11.020,
/// table 1: 21/5, −33/7, 387/140).
const ECCENTRICITY_TIDE: [f64; 3] = [21.0 / 5.0, -33.0 / 7.0, 387.0 / 140.0];

/// Weight of Beuthe's pattern C against pattern A in a thin shell over an
/// ocean (membrane limit, pattern B vanishing): `f_C / f_A = 24 / (16/3)
/// = 9/2` (Beuthe 2013, doi:10.1016/j.icarus.2012.11.020, eqs. 48-49).
const MEMBRANE_C_OVER_A: f64 = 4.5;

/// An ice shell over a body's water layer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct IceShell {
    /// Mean thickness of the ice shell, km (averaged over the surface
    /// when the tides thin it at the poles); the depth of the ocean's
    /// floor when convection froze the ocean.
    pub shell_km: f64,
    /// Thickness of the liquid ocean under the shell, km, down to the
    /// bottom of the water layer or to [`HIGH_PRESSURE_ICE_PA`],
    /// whichever comes first; 0 when the shell reaches either.
    pub ocean_km: f64,
    /// Whether the shell convects under a stagnant lid
    /// ([`convects`]); false in a model stored before it existed.
    #[serde(default)]
    pub convecting: bool,
    /// Thickness of the conductive lid, km: the stagnant lid of a
    /// convecting shell, the whole shell otherwise; 0 in a model stored
    /// before it existed.
    #[serde(default)]
    pub lid_km: f64,
    /// Thickness of the shell at the poles, km, where the tides heat a
    /// thin shell over an ocean most ([`tidal_pattern`]); the mean
    /// thickness when it convects or has no ocean; 0 in a model stored
    /// before it existed.
    #[serde(default)]
    pub polar_shell_km: f64,
    /// Temperature at the base of the shell, K: the [`liquidus_k`] of the
    /// ocean's ammonia, concentrated as the ocean froze; 0 in a model
    /// stored before it existed.
    #[serde(default)]
    pub liquidus_k: f64,
}

/// Thickness, m, of a conductive ice shell with its surface at
/// `surface_k` and its base at [`MELT_K`], carrying the heat flux
/// `flux_w_m2` out, in a plane: with `k = c / T`, Fourier's law
/// `q = k dT/dz` integrates to `d = c ln(T_melt / T_s) / q` (Hussmann et
/// al. 2006, doi:10.1016/j.icarus.2006.06.005). Europa, about 100 K under
/// some 45 mW/m², gets about 15 km. Infinite when no heat flows.
pub fn conductive_shell_m(surface_k: f64, flux_w_m2: f64) -> f64 {
    ICE_CONDUCTIVITY_W_M * (MELT_K / surface_k).ln() / flux_w_m2
}

/// Thickness, m, of a conductive ice shell at the top of a sphere of
/// `radius_m`, its surface at `surface_k` and its base at `base_k`,
/// carrying out the heat flux `flux_w_m2` (at the surface): Fourier's law
/// in a sphere, `Q = −4π r² (c / T) dT/dr`, integrates to
/// `Q (1/r_b − 1/R) = 4π c ln(T_b / T_s)`, so that with the plane
/// thickness `d₀ = c ln(T_b / T_s) / q` of [`conductive_shell_m`] the
/// shell is `d = R d₀ / (R + d₀)`: thinner, as its base is smaller than
/// its surface (Turcotte & Schubert 2014, *Geodynamics*, 3rd ed., §4.4,
/// not checked, for conduction through a spherical shell). An Enceladus
/// of 250 km under a plane 40 km gets 34 km. 0 when the base is no
/// warmer than the surface, `radius_m` when no heat flows.
fn spherical_shell_m(radius_m: f64, base_k: f64, surface_k: f64, flux_w_m2: f64) -> f64 {
    let log = (base_k / surface_k).ln();
    if log.is_nan() || log <= 0.0 {
        return 0.0;
    }
    let plane = ICE_CONDUCTIVITY_W_M * log / flux_w_m2;
    if !plane.is_finite() || plane < 0.0 {
        return radius_m;
    }
    radius_m * plane / (radius_m + plane)
}

/// The tidal heat of a thin ice shell over an ocean at the cosine of the
/// colatitude `mu`, averaged over longitude, relative to its mean over
/// the surface: from Beuthe 2013 (doi:10.1016/j.icarus.2012.11.020), the
/// eccentricity tide of a synchronous moon dissipates in a membrane shell
/// as `Ψ_A + 9/2 Ψ_C` ([`MEMBRANE_C_OVER_A`]), `Ψ_A = Ψ₀ + Ψ₂ + Ψ₄` and
/// `Ψ_C = Ψ₀ − Ψ₂ + Ψ₄/6`, whose zonal parts are `a₀₀`, `a₂₀ P₂(μ)` and
/// `a₄₀ P₄(μ)` ([`ECCENTRICITY_TIDE`]). It peaks at the poles, 1.92 times
/// the mean, and is lowest at the equator, 0.72 times: the polar thinning
/// of Ojakangas & Stevenson 1989 (doi:10.1016/0019-1035(89)90052-3) and
/// Čadek et al. 2016 (doi:10.1002/2016GL068634).
pub fn tidal_pattern(mu: f64) -> f64 {
    let [a0, a2, a4] = ECCENTRICITY_TIDE;
    let c = MEMBRANE_C_OVER_A;
    let p2 = (3.0 * mu * mu - 1.0) / 2.0;
    let p4 = (35.0 * mu.powi(4) - 30.0 * mu * mu + 3.0) / 8.0;
    let mean = (1.0 + c) * a0;
    ((1.0 - c) * a2 * p2 + (1.0 + c / 6.0) * a4 * p4) / mean + 1.0
}

/// Newtonian viscosity of ice Ih at `temperature_k`, Pa s, by diffusion
/// creep: Goldsby & Kohlstedt 2001 (doi:10.1029/2000JB900336) give the
/// strain rate `ε̇ = 42 V_m σ D_v / (R T d²)`, with the volume diffusion
/// `D_v = D₀ exp(−Q / R T)` ([`ICE_DIFFUSION_M2_S`],
/// [`ICE_ACTIVATION_J_MOL`], [`ICE_MOLAR_VOLUME_M3`]) and the grain size
/// [`GRAIN_SIZE_M`]; the viscosity is `σ / (2 ε̇)` (calibration, no
/// source, for the factor 2). Grain-boundary diffusion, which matters only
/// under a millimetre, is left out. About 3·10¹⁷ Pa s at 273 K.
pub fn ice_viscosity_pa_s(temperature_k: f64) -> f64 {
    let diffusion =
        ICE_DIFFUSION_M2_S * (-ICE_ACTIVATION_J_MOL / (GAS_CONSTANT * temperature_k)).exp();
    GAS_CONSTANT * temperature_k * GRAIN_SIZE_M * GRAIN_SIZE_M
        / (84.0 * ICE_MOLAR_VOLUME_M3 * diffusion)
}

/// Thermal diffusivity of ice at `temperature_k`, m²/s: its conductivity
/// `651 / T` over its density and specific heat.
fn diffusivity_m2_s(temperature_k: f64) -> f64 {
    ICE_CONDUCTIVITY_W_M / temperature_k / (ICE_DENSITY * ICE_HEAT_CAPACITY)
}

/// The rheological temperature scale `R T² / Q`, K, of ice at `base_k`.
fn rheological_k(base_k: f64) -> f64 {
    GAS_CONSTANT * base_k * base_k / ICE_ACTIVATION_J_MOL
}

/// Whether an ice shell `shell_m` thick, its base at `base_k` and its
/// surface at `surface_k`, under `gravity` m/s², convects: its Rayleigh
/// number with the viscosity at its base, `Ra = ρ g α ΔT D³ / (κ η_b)`,
/// reaches the stagnant-lid onset `20.9 θ⁴` ([`ONSET_COEFFICIENT`]), with
/// `θ = Q ΔT / (R T_b²)` the Frank-Kamenetskii parameter of the
/// viscosity's activation energy (Solomatov 1995, doi:10.1063/1.868624;
/// Barr & McKinnon 2007, doi:10.1029/2006JE002781). With 30 mm grains a
/// Ganymede's shell convects from about 280 km, an Enceladus's never.
pub fn convects(shell_m: f64, base_k: f64, surface_k: f64, gravity: f64) -> bool {
    let drop = base_k - surface_k;
    if !(shell_m > 0.0 && drop > 0.0) {
        return false;
    }
    let rayleigh = ICE_DENSITY * gravity * ICE_EXPANSIVITY * drop * shell_m.powi(3)
        / (diffusivity_m2_s(base_k) * ice_viscosity_pa_s(base_k));
    let theta = drop / rheological_k(base_k);
    rayleigh >= ONSET_COEFFICIENT * theta.powi(4)
}

/// Heat flux, W/m², that a convecting ice shell over an ocean at `base_k`
/// carries up into its stagnant lid under `gravity` m/s²:
/// `q = 0.53 k (R T² / Q)^(4/3) (ρ g α / (κ η))^(1/3)`
/// ([`CONVECTIVE_COEFFICIENT`]), independent of the shell's thickness
/// (Solomatov & Moresi 2000, doi:10.1029/2000JE001245, for a Newtonian
/// fluid; Barr & McKinnon 2007, doi:10.1029/2006JE002781, for ice
/// shells), with the conductivity, the diffusivity and the viscosity
/// taken at the melting point of the ocean (`base_k`) rather than at the
/// slightly colder interior of the convecting layer, which overestimates
/// it a little.
pub fn convective_flux_w_m2(base_k: f64, gravity: f64) -> f64 {
    let k = ICE_CONDUCTIVITY_W_M / base_k;
    let buoyancy = ICE_DENSITY * gravity * ICE_EXPANSIVITY
        / (diffusivity_m2_s(base_k) * ice_viscosity_pa_s(base_k));
    CONVECTIVE_COEFFICIENT * k * rheological_k(base_k).powf(4.0 / 3.0) * buoyancy.cbrt()
}

/// The ammonia mass fraction, kg/kg, whose [`liquidus_k`] is
/// `temperature_k`, from 0 (at 273 K or warmer) to [`NH3_EUTECTIC`] (at
/// the eutectic or colder): the inverse of the liquidus, by bisection.
fn nh3_at_liquidus(temperature_k: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, NH3_EUTECTIC);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if liquidus_k(mid) > temperature_k {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Depth, m, under the surface of a body of `interior` at which the
/// hydrostatic pressure reaches [`HIGH_PRESSURE_ICE_PA`], with
/// [`ICE_DENSITY`] down to `shell_m` and [`OCEAN_DENSITY`] below; `None`
/// when it is not reached above `base_m`, the depth of the water layer's
/// floor. Gravity at radius `r` is `G M(<r) / r²`, with the water layer's
/// mass spread evenly through its volume as in
/// [`super::interior::solid_interior`].
fn ocean_floor_m(interior: &Interior, water: usize, shell_m: f64, base_m: f64) -> Option<f64> {
    let layer = &interior.layers[water];
    let radius = layer.outer_radius_earth * EARTH_M;
    let inner = radius - base_m;
    let layer_kg = layer.mass_earth * EARTH_KG;
    let below_kg: f64 = interior.layers[..water]
        .iter()
        .map(|l| l.mass_earth * EARTH_KG)
        .sum();
    let shell_volume = radius.powi(3) - inner.powi(3);
    let gravity =
        |r: f64| G * (below_kg + layer_kg * (r.powi(3) - inner.powi(3)) / shell_volume) / (r * r);
    let mut pressure = 0.0;
    let mut depth = 0.0;
    while depth < base_m {
        let step = DEPTH_STEP_M.min(base_m - depth);
        let mid = depth + step / 2.0;
        let density = if mid < shell_m {
            ICE_DENSITY
        } else {
            OCEAN_DENSITY
        };
        let next = pressure + density * gravity(radius - mid) * step;
        if next >= HIGH_PRESSURE_ICE_PA {
            return Some(depth + step * (HIGH_PRESSURE_ICE_PA - pressure) / (next - pressure));
        }
        pressure = next;
        depth += step;
    }
    None
}

/// The water layer of a body and what its shell is solved from.
struct Water<'a> {
    interior: &'a Interior,
    /// Index of the water layer in the interior.
    index: usize,
    /// Its outer radius, m.
    radius_m: f64,
    /// Its thickness, m.
    layer_m: f64,
    /// Its mass, kg.
    layer_kg: f64,
    /// Ammonia per kilogram of its ice.
    nh3: f64,
}

impl Water<'_> {
    /// Depth, m, of the ocean's floor under a shell `shell_m` thick.
    fn floor_m(&self, shell_m: f64) -> f64 {
        ocean_floor_m(self.interior, self.index, shell_m, self.layer_m).unwrap_or(self.layer_m)
    }

    /// Thickness, m, of the ocean under a shell `shell_m` thick, never
    /// negative.
    fn ocean_m(&self, shell_m: f64) -> f64 {
        (self.floor_m(shell_m) - shell_m).max(0.0)
    }

    /// Ammonia mass fraction of the ocean under a shell `shell_m` thick:
    /// the ice excludes ammonia as it freezes, so all of the layer's
    /// ammonia is in the liquid left (Grasset & Sotin 1996,
    /// doi:10.1006/icar.1996.0141, for Titan's ocean, not checked), up
    /// to the eutectic; the eutectic when no ocean is left, 0 without
    /// ammonia.
    fn ocean_nh3(&self, shell_m: f64) -> f64 {
        if self.nh3.is_nan() || self.nh3 <= 0.0 {
            return 0.0;
        }
        let r = self.radius_m;
        let top = r - shell_m;
        let bottom = r - self.floor_m(shell_m);
        if top <= bottom {
            return NH3_EUTECTIC;
        }
        let ocean_kg = OCEAN_DENSITY * 4.0 / 3.0 * PI * (top.powi(3) - bottom.powi(3));
        (self.nh3 * self.layer_kg / ocean_kg).min(NH3_EUTECTIC)
    }

    /// The shallowest shell, m, whose ocean holds `nh3` of ammonia
    /// ([`Self::ocean_nh3`] grows with the shell), by bisection.
    fn shell_holding(&self, nh3: f64) -> f64 {
        let (mut lo, mut hi) = (0.0, self.layer_m);
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            if self.ocean_nh3(mid) < nh3 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        hi
    }
}

/// The ice shell of a body of `interior`, making `heat`, under its
/// `surface`, its ice carrying `nh3_ice_fraction` kg of ammonia per kg: a
/// body with a water layer whose surface is frozen (no liquid water on it
/// and a mean temperature under [`MELT_K`]) has one; `None` for a body
/// without water, without a surface (an envelope hides it) or whose
/// surface is not frozen.
///
/// - **Conduction.** The shell is in equilibrium with all of the
///   internal heat, from below and from the tides in the ice alike (all
///   of it leaves through the surface): latitude by latitude
///   ([`BANDS`]), a conductive shell in a sphere ([`spherical_shell_m`])
///   carries out the radiogenic and secular flux, spread evenly, plus
///   the tidal flux in the pattern of a thin shell over an ocean
///   ([`tidal_pattern`], peaking at the poles). Every watt of tidal heat
///   is spread in that pattern, whether it is dissipated in the shell
///   or below it: Beuthe 2019 (doi:10.1016/j.icarus.2019.05.035) finds
///   that Enceladus's shell dissipates too little with standard ice
///   rheology and its core the rest, and a solid core's eccentricity
///   tide also peaks at the poles (Beuthe 2013), where Choblet et al.
///   2017 (doi:10.1038/s41550-017-0289-8) focus its heat. The pattern
///   matters only as far as the tides make the heat; without an ocean
///   the heat is spread evenly. `shell_km` is the mean over the
///   surface, `polar_shell_km` the shell at the poles.
/// - **Ammonia.** The shell's base is at the [`liquidus_k`] of the ocean,
///   whose ammonia concentrates as it freezes ([`Water::ocean_nh3`]): a
///   thicker shell leaves a smaller, richer, colder ocean and a thinner
///   shell, so shell and ocean are solved together by bisection.
/// - **Convection.** A shell that reaches the onset of stagnant-lid
///   convection ([`convects`]) carries [`convective_flux_w_m2`] up into
///   its lid whatever its thickness. If that is more than the internal
///   heat supplies, the ocean freezes: down to where its concentrated
///   ammonia lowers the liquidus enough that convection carries just the
///   heat supplied, or entirely (no ammonia, or more than the eutectic
///   would need), the shell then reaching the ocean's floor. The lid
///   conducts the heat from the surface down to [`RHEOLOGICAL_DROP`]
///   under the base's temperature. A convecting shell is not thinned at
///   the poles.
///
/// The ocean reaches down to the floor of the water layer or to the
/// depth at which the hydrostatic pressure reaches
/// [`HIGH_PRESSURE_ICE_PA`] (`ocean_floor_m`), whichever is shallower; a
/// shell whose base lies at or beyond it leaves no ocean.
///
/// Left out: salts in the liquidus, and the pressure's lowering of the
/// melting point; the surface temperature's fall towards the poles
/// (it would thicken the polar shell); the lateral flow of the ice that
/// evens out the shell, and the concentration of a shell's dissipation
/// where it is thinner (Beuthe 2019), which work against each other.
pub fn ice_shell(
    interior: &Interior,
    heat: &InternalHeat,
    surface: Option<&Surface>,
    nh3_ice_fraction: f64,
) -> Option<IceShell> {
    let climate = surface?.climate;
    if climate.liquid == SurfaceLiquid::Water || climate.temperature_k >= MELT_K {
        return None;
    }
    frozen_shell(interior, heat, climate.temperature_k, nh3_ice_fraction)
}

/// The [`ice_shell`] of a body of `interior` making `heat` under a frozen
/// surface at `surface_k`, its ice carrying `nh3_ice_fraction` of
/// ammonia; `None` without a water layer.
fn frozen_shell(
    interior: &Interior,
    heat: &InternalHeat,
    surface_k: f64,
    nh3_ice_fraction: f64,
) -> Option<IceShell> {
    let index = interior
        .layers
        .iter()
        .position(|l| l.material == Material::Water)?;
    let base_earth = index
        .checked_sub(1)
        .map_or(0.0, |below| interior.layers[below].outer_radius_earth);
    let layer = &interior.layers[index];
    let water = Water {
        interior,
        index,
        radius_m: layer.outer_radius_earth * EARTH_M,
        layer_m: (layer.outer_radius_earth - base_earth) * EARTH_M,
        layer_kg: layer.mass_earth * EARTH_KG,
        nh3: nh3_ice_fraction,
    };
    let radius_m = interior.radius_earth * EARTH_M;
    let total_kg: f64 = interior.layers.iter().map(|l| l.mass_earth).sum::<f64>() * EARTH_KG;
    let gravity = G * total_kg / (radius_m * radius_m);
    let flux = heat.flux_w_m2(interior.radius_earth);
    let tidal = flux * heat.tidal_w / heat.total_w().max(f64::MIN_POSITIVE);
    let even = flux - tidal;
    let column = |base_k: f64, mu: f64| {
        spherical_shell_m(
            radius_m,
            base_k,
            surface_k,
            even + tidal * tidal_pattern(mu),
        )
    };
    // The mean shell over the surface for a base at `base_k`: thinned at
    // the poles over an ocean, even without.
    let mean_shell = |base_k: f64, ocean: bool| {
        let shell = if ocean {
            (0..BANDS)
                .map(|i| column(base_k, (i as f64 + 0.5) / BANDS as f64))
                .sum::<f64>()
                / BANDS as f64
        } else {
            spherical_shell_m(radius_m, base_k, surface_k, flux)
        };
        shell.min(water.layer_m)
    };
    // The shell its own ocean's liquidus sustains: the excess falls as
    // the shell thickens, so its root is found by bisection, from the
    // side where it is not positive (a shell that would fill the layer
    // fills it, with no sliver of ocean left).
    let excess = |shell_m: f64| {
        let base_k = liquidus_k(water.ocean_nh3(shell_m));
        mean_shell(base_k, water.ocean_m(shell_m) > 0.0) - shell_m
    };
    let (mut lo, mut hi) = (0.0, water.layer_m);
    for _ in 0..50 {
        let mid = 0.5 * (lo + hi);
        if excess(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut shell_m = hi;
    let mut base_k = liquidus_k(water.ocean_nh3(shell_m));
    let convecting = convects(shell_m, base_k, surface_k, gravity);
    if convecting && convective_flux_w_m2(base_k, gravity) > flux {
        // Convection carries out more than comes in: the ocean freezes
        // until its liquidus brings the convective flux down to the
        // supply, or entirely.
        let eutectic_k = liquidus_k(NH3_EUTECTIC);
        if water.nh3 > 0.0 && convective_flux_w_m2(eutectic_k, gravity) <= flux {
            let (mut cold, mut warm) = (eutectic_k, base_k);
            for _ in 0..50 {
                let mid = 0.5 * (cold + warm);
                if convective_flux_w_m2(mid, gravity) > flux {
                    warm = mid;
                } else {
                    cold = mid;
                }
            }
            shell_m = water.shell_holding(nh3_at_liquidus(0.5 * (cold + warm)));
        } else {
            shell_m = water.floor_m(water.layer_m);
        }
        base_k = liquidus_k(water.ocean_nh3(shell_m));
    }
    let ocean_m = water.ocean_m(shell_m);
    let lid_m = if convecting {
        let lid_base_k = base_k - RHEOLOGICAL_DROP * rheological_k(base_k);
        spherical_shell_m(radius_m, lid_base_k, surface_k, flux).min(shell_m)
    } else {
        shell_m
    };
    let polar_m = if !convecting && ocean_m > 0.0 {
        column(base_k, 1.0).min(shell_m)
    } else {
        shell_m
    };
    Some(IceShell {
        shell_km: shell_m / 1e3,
        ocean_km: ocean_m / 1e3,
        convecting,
        lid_km: lid_m / 1e3,
        polar_shell_km: polar_m / 1e3,
        liquidus_k: base_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The H₂O–NH₃ liquidus at low pressure runs from pure water's 273 K
    /// down to the eutectic, about 176 K at a third of ammonia
    /// (Leliwa-Kopystyński et al. 2002).
    #[test]
    fn liquidus_runs_from_pure_water_to_the_eutectic() {
        assert_eq!(liquidus_k(0.0), 273.0);
        let eutectic = liquidus_k(0.33);
        assert!((eutectic - 176.0).abs() <= 3.0, "{eutectic}");
        let tenth = liquidus_k(0.10);
        assert!((250.0..265.0).contains(&tenth), "{tenth}");
        let mut last = liquidus_k(0.0);
        for i in 1..=330 {
            let t = liquidus_k(f64::from(i) / 1000.0);
            assert!(t < last, "{i}: {t} after {last}");
            last = t;
        }
    }

    #[test]
    fn a_europa_shell_is_ten_to_twenty_kilometres() {
        // Europa: about 100 K at the surface, some 45 mW/m² of tidal and
        // radiogenic heat; thermal models give a conductive lid of 10 to
        // 30 km (Hussmann et al. 2006).
        let d = conductive_shell_m(100.0, 0.045) / 1e3;
        assert!((10.0..20.0).contains(&d), "{d}");
    }

    #[test]
    fn more_heat_or_a_warmer_surface_thins_the_shell() {
        let base = conductive_shell_m(100.0, 0.02);
        assert!(conductive_shell_m(100.0, 0.04) < base);
        assert!(conductive_shell_m(150.0, 0.02) < base);
        assert_eq!(conductive_shell_m(MELT_K, 0.02), 0.0);
        assert_eq!(conductive_shell_m(100.0, 0.0), f64::INFINITY);
    }

    /// An interior of a rock core of `rock_earth` M⊕ and `rock_r` R⊕ under
    /// a water layer of `water_earth` M⊕ up to `radius` R⊕.
    fn layered(rock_earth: f64, rock_r: f64, water_earth: f64, radius: f64) -> Interior {
        use super::super::Layer;
        Interior {
            layers: vec![
                Layer {
                    material: Material::Rock,
                    mass_earth: rock_earth,
                    outer_radius_earth: rock_r,
                },
                Layer {
                    material: Material::Water,
                    mass_earth: water_earth,
                    outer_radius_earth: radius,
                },
            ],
            radius_earth: radius,
            density: 0.0,
            gravity: 0.0,
        }
    }

    #[test]
    fn high_pressure_ice_floors_a_deep_ocean() {
        // A Ganymede: 2605 km, the water layer from 1704 km, 1.46 m/s² at
        // the surface. Under a 100 km shell 350.1 MPa lies some 250 km
        // down, far above the layer's floor at 900 km.
        let int = layered(0.01364, 0.2675, 0.01116, 0.4089);
        let layer_m = (0.4089 - 0.2675) * EARTH_M;
        let floor = ocean_floor_m(&int, 1, 100e3, layer_m).expect("reached");
        assert!((200e3..300e3).contains(&floor), "{floor}");
        // A thicker shell, lighter than water, pushes the floor down.
        let deeper = ocean_floor_m(&int, 1, 200e3, layer_m).unwrap();
        assert!(deeper > floor, "{deeper} {floor}");
        // A Europa's 120 km layer never reaches it.
        let europa = layered(0.00736, 0.2177, 0.00064, 0.2361);
        let layer_m = (0.2361 - 0.2177) * EARTH_M;
        assert_eq!(ocean_floor_m(&europa, 1, 16e3, layer_m), None);
    }

    /// Internal heat of `total_w` watts, `tidal_share` of it tidal.
    fn heat(total_w: f64, tidal_share: f64) -> InternalHeat {
        InternalHeat {
            radiogenic_w: total_w * (1.0 - tidal_share),
            tidal_w: total_w * tidal_share,
            secular_w: 0.0,
        }
    }

    /// A ball of ice 1500 km across over a rock core of 300 km, at
    /// 0.42 m/s²: thick enough a shell to convect, light enough that its
    /// ocean reaches far down before ice V.
    fn ice_ball() -> Interior {
        layered(5.68e-5, 300.0 / 6371.0, 2.345e-3, 1500.0 / 6371.0)
    }

    #[test]
    fn a_five_kilometre_shell_does_not_convect() {
        // Europa's gravity, its surface and a base at the melting point:
        // the onset lies some 280 km down with 30 mm grains.
        assert!(!convects(5e3, MELT_K, 100.0, 1.31));
        assert!(!convects(100e3, MELT_K, 100.0, 1.31));
        assert!(convects(400e3, MELT_K, 100.0, 1.31));
        // No shell, or no temperature drop across it, never convects.
        assert!(!convects(0.0, MELT_K, 100.0, 1.31));
        assert!(!convects(400e3, 100.0, 100.0, 1.31));
    }

    /// The convective flux of a Ganymede's shell at the melting point is a
    /// few mW/m², and falls as the base gets colder (stiffer ice).
    #[test]
    fn colder_ice_carries_less_heat() {
        let warm = convective_flux_w_m2(MELT_K, 1.43);
        assert!((1e-3..5e-3).contains(&warm), "{warm}");
        assert!(convective_flux_w_m2(250.0, 1.43) < warm);
        let eta = ice_viscosity_pa_s(MELT_K);
        assert!((1e17..1e18).contains(&eta), "{eta:e}");
    }

    #[test]
    fn a_frozen_ocean_is_zero_not_negative() {
        // Some 0.8 mW/m²: a conductive shell of about 600 km would leave
        // an ocean above ice V, but it convects and carries out more than
        // that, so without ammonia the whole ocean freezes.
        let ball = ice_ball();
        let shell = frozen_shell(&ball, &heat(2.26e10, 0.0), 80.0, 0.0).unwrap();
        assert!(shell.convecting, "{shell:?}");
        assert_eq!(shell.ocean_km, 0.0, "{shell:?}");
        assert!(
            shell.shell_km > 600.0 && shell.shell_km.is_finite(),
            "{shell:?}"
        );
        assert!(
            shell.lid_km > 0.0 && shell.lid_km < shell.shell_km,
            "{shell:?}"
        );
        assert_eq!(shell.polar_shell_km, shell.shell_km);
        assert_eq!(shell.liquidus_k, MELT_K);
        // With 1 % of ammonia in its ice, it freezes only until the
        // ammonia left in a thin ocean makes the ice too stiff to carry
        // out more than comes in.
        let wet = frozen_shell(&ball, &heat(2.26e10, 0.0), 80.0, 0.01).unwrap();
        assert!(wet.convecting, "{wet:?}");
        assert!(
            wet.ocean_km > 0.0 && wet.shell_km < shell.shell_km,
            "{wet:?}"
        );
        assert!(wet.liquidus_k < MELT_K - 5.0, "{wet:?}");
        let flux = 2.26e10 / (4.0 * PI * 1.5e6_f64.powi(2));
        let carried = convective_flux_w_m2(wet.liquidus_k, 0.425);
        assert!((carried / flux - 1.0).abs() < 0.05, "{carried} {flux}");
    }

    /// Ammonia concentrated in a shrinking ocean lowers its liquidus and
    /// thins the shell: a body whose shell would fill its water layer
    /// keeps a thin, cold, ammonia-rich ocean.
    #[test]
    fn ammonia_keeps_a_thin_ocean_under_a_thick_shell() {
        // A Titan: its 230 km water layer over a rock core, 3 mW/m².
        let titan = layered(0.0177, 0.35, 0.0034, 0.389);
        let area = 4.0 * PI * (0.389 * EARTH_M).powi(2);
        let dry = frozen_shell(&titan, &heat(2.2e-3 * area, 0.0), 93.0, 0.0).unwrap();
        assert_eq!(dry.ocean_km, 0.0, "{dry:?}");
        let wet = frozen_shell(&titan, &heat(2.2e-3 * area, 0.0), 93.0, 0.005).unwrap();
        assert!(wet.ocean_km > 0.0 && wet.shell_km < dry.shell_km, "{wet:?}");
        assert!(wet.liquidus_k < 260.0 && wet.liquidus_k >= liquidus_k(NH3_EUTECTIC));
        assert!(!wet.convecting, "{wet:?}");
    }

    /// The tides thin a shell over an ocean at the poles by heating it
    /// most there (Beuthe 2013): its pattern averages to 1 over the
    /// surface, peaks at the poles, is least at the equator.
    #[test]
    fn the_tides_thin_the_shell_at_the_poles() {
        let n = 10_000;
        let mean = (0..n)
            .map(|i| tidal_pattern((f64::from(i) + 0.5) / f64::from(n)))
            .sum::<f64>()
            / f64::from(n);
        assert!((mean - 1.0).abs() < 1e-6, "{mean}");
        assert!((tidal_pattern(1.0) - 1.92).abs() < 0.01);
        assert!((tidal_pattern(0.0) - 0.72).abs() < 0.01);
        // An Enceladus heated by its tides alone: thinner at the poles,
        // thicker on average than a shell under even heating.
        let enceladus = layered(8.6e-6, 0.0229, 8.4e-6, 0.0366);
        let even = frozen_shell(&enceladus, &heat(12.5e9, 0.0), 86.0, 0.0).unwrap();
        let tidal = frozen_shell(&enceladus, &heat(12.5e9, 1.0), 86.0, 0.0).unwrap();
        assert!(tidal.ocean_km > 0.0, "{tidal:?}");
        assert!((even.polar_shell_km / even.shell_km - 1.0).abs() < 1e-9);
        assert!(tidal.polar_shell_km < 0.6 * tidal.shell_km, "{tidal:?}");
        assert!(tidal.shell_km > even.shell_km, "{tidal:?} {even:?}");
        assert!(!tidal.convecting && tidal.lid_km == tidal.shell_km);
    }

    /// A shell in a sphere is thinner than in a plane, as its base is
    /// smaller than its surface; both agree for a thin one.
    #[test]
    fn a_spherical_shell_is_thinner_than_a_flat_one() {
        let flat = conductive_shell_m(86.0, 0.0188);
        let round = spherical_shell_m(250e3, MELT_K, 86.0, 0.0188);
        assert!((round / 1e3 - 34.0).abs() < 1.5, "{round}");
        assert!(round < flat);
        let europa = spherical_shell_m(1561e3, MELT_K, 100.0, 0.045);
        let plane = conductive_shell_m(100.0, 0.045);
        assert!((europa / plane - 1.0).abs() < 0.02, "{europa} {plane}");
        assert_eq!(spherical_shell_m(250e3, MELT_K, 86.0, 0.0), 250e3);
        assert_eq!(spherical_shell_m(250e3, 200.0, 250.0, 0.01), 0.0);
    }
}
