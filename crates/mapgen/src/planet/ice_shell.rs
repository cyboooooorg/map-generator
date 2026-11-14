//! The ice shell of a frozen body over its water layer, and the ocean
//! left beneath it ([`ice_shell`]): the shell conducts the internal heat
//! out through ice whose conductivity rises as it gets colder, and the
//! water below its base stays liquid down to where the pressure turns it
//! to high-pressure ice.
use serde::{Deserialize, Serialize};

use super::{Interior, InternalHeat, Material, Surface, SurfaceLiquid};
use crate::exomoon::{EARTH_KG, EARTH_M, G};

/// The conductivity of water ice times its temperature, W/m: `k = 651 /
/// T` W m⁻¹ K⁻¹ (Klinger 1980, doi:10.1126/science.209.4453.271), the
/// law thermal models of icy satellites' shells use (Hussmann, Sohl &
/// Spohn 2006, doi:10.1016/j.icarus.2006.06.005).
pub const ICE_CONDUCTIVITY_W_M: f64 = 651.0;

/// Temperature at the base of the shell, where the ocean begins, K: the
/// melting point of pure water ice. Calibration, no source: ammonia or
/// salts would lower it and thin the shell, which the model does not
/// carry.
pub const MELT_K: f64 = 273.0;

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

/// Density of the ocean, kg/m³: liquid water, rounded. Calibration, no
/// source: compression and dissolved salts would raise it by a few
/// percent.
const OCEAN_DENSITY: f64 = 1000.0;

/// Depth step, m, of the hydrostatic integration of [`ocean_floor_m`].
const DEPTH_STEP_M: f64 = 100.0;

/// A conductive ice shell over a body's water layer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct IceShell {
    /// Thickness of the ice shell, km; the whole water layer when no
    /// ocean is left beneath it.
    pub shell_km: f64,
    /// Thickness of the liquid ocean under the shell, km, down to the
    /// bottom of the water layer or to [`HIGH_PRESSURE_ICE_PA`],
    /// whichever comes first; 0 when the shell reaches either.
    pub ocean_km: f64,
}

/// Thickness, m, of a conductive ice shell with its surface at
/// `surface_k` and its base at [`MELT_K`], carrying the heat flux
/// `flux_w_m2` out: with `k = c / T`, Fourier's law `q = k dT/dz`
/// integrates to `d = c ln(T_melt / T_s) / q` (Hussmann et al. 2006,
/// doi:10.1016/j.icarus.2006.06.005). Europa, about 100 K under some
/// 45 mW/m², gets about 15 km. Infinite when no heat flows.
pub fn conductive_shell_m(surface_k: f64, flux_w_m2: f64) -> f64 {
    ICE_CONDUCTIVITY_W_M * (MELT_K / surface_k).ln() / flux_w_m2
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

/// The ice shell of a body of `interior`, making `heat`, under its
/// `surface`: a body with a water layer whose surface is frozen (no
/// liquid water on it and a mean temperature under [`MELT_K`]) has a
/// [`conductive_shell_m`] shell for its internal heat flux, and an ocean
/// beneath it down to the floor of the water layer or to the depth at
/// which the hydrostatic pressure reaches [`HIGH_PRESSURE_ICE_PA`]
/// (`ocean_floor_m`), whichever is shallower. A shell whose base lies
/// at or beyond that pressure leaves no ocean. `None` for a body without
/// water, without a surface (an envelope hides it) or whose surface is
/// not frozen.
///
/// Known departures (issue #463): heated by their rock alone, the
/// conductive shells of Ganymede, Titan and Callisto come out 180 to
/// 500 km thick, so whether their base lies above the ice V onset, and
/// they keep an ocean, depends on the radiogenic draw; at equal draws
/// Ganymede and Callisto get the same answer. Enceladus, with a quarter
/// of the tidal heat it radiates, freezes its whole water layer.
pub fn ice_shell(
    interior: &Interior,
    heat: &InternalHeat,
    surface: Option<&Surface>,
) -> Option<IceShell> {
    let climate = surface?.climate;
    if climate.liquid == SurfaceLiquid::Water || climate.temperature_k >= MELT_K {
        return None;
    }
    let water = interior
        .layers
        .iter()
        .position(|l| l.material == Material::Water)?;
    let base_earth = water
        .checked_sub(1)
        .map_or(0.0, |below| interior.layers[below].outer_radius_earth);
    let layer_m = (interior.layers[water].outer_radius_earth - base_earth) * EARTH_M;
    let flux = heat.flux_w_m2(interior.radius_earth);
    let shell_m = conductive_shell_m(climate.temperature_k, flux).min(layer_m);
    let floor_m = ocean_floor_m(interior, water, shell_m, layer_m).unwrap_or(layer_m);
    Some(IceShell {
        shell_km: shell_m / 1e3,
        ocean_km: (floor_m - shell_m).max(0.0) / 1e3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
