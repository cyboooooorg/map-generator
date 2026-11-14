//! The ice shell of a frozen body over its water layer, and the ocean
//! left beneath it ([`ice_shell`]): the shell conducts the internal heat
//! out through ice whose conductivity rises as it gets colder, and the
//! water below its base stays liquid.
use serde::{Deserialize, Serialize};

use super::{Interior, InternalHeat, Material, Surface, SurfaceLiquid};
use crate::exomoon::EARTH_M;

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

/// A conductive ice shell over a body's water layer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct IceShell {
    /// Thickness of the ice shell, km; the whole water layer when no
    /// ocean is left beneath it.
    pub shell_km: f64,
    /// Thickness of the liquid ocean under the shell, km; 0 when the
    /// shell reaches the bottom of the water layer.
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

/// The ice shell of a body of `interior`, making `heat`, under its
/// `surface`: a body with a water layer whose surface is frozen (no
/// liquid water on it and a mean temperature under [`MELT_K`]) has a
/// [`conductive_shell_m`] shell for its internal heat flux, and an ocean
/// beneath it where the shell is thinner than the water layer, the
/// difference. `None` for a body without water, without a surface (an
/// envelope hides it) or whose surface is not frozen.
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
    let layer_km = (interior.layers[water].outer_radius_earth - base_earth) * EARTH_M / 1e3;
    let flux = heat.flux_w_m2(interior.radius_earth);
    let shell_km = (conductive_shell_m(climate.temperature_k, flux) / 1e3).min(layer_km);
    Some(IceShell {
        shell_km,
        ocean_km: layer_km - shell_km,
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
}
