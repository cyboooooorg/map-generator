//! The Bond albedo of a rocky or icy body, derived from what covers it
//! (climate pass lot 2b, issue #510).
//!
//! [`cover`] splits the disc into open ocean, bare land, water ice and
//! snow, frosts of N₂, CH₄ and CO₂ and water clouds, as planet-wide
//! fractions (the model is
//! zero-dimensional); [`bond_albedo`] stacks them under the Rayleigh
//! scattering of the air's column, a methane haze and, when there is one,
//! a sulphuric acid deck. [`super::climate::climate`] solves the albedo
//! and the temperature together as a fixed point, and
//! [`super::Context::with_albedo`] turns an albedo into the equilibrium
//! temperature the climate absorbs at.
use std::sync::LazyLock;

use super::{Gases, Water};

/// The Bond albedo behind a context's equilibrium temperature: Earth's,
/// 0.3, which [`crate::star::equilibrium_k`] assumes (255 K at 1 S⊕).
/// [`super::Context::with_albedo`] rescales from it.
pub const REFERENCE_ALBEDO: f64 = 0.3;

/// What covers a body's disc, as fractions of its area (0 to 1): open
/// ocean, bare land, water ice and snow, frosts of other volatiles, the
/// first four summing to 1; and the water clouds over them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cover {
    pub ocean: f64,
    pub land: f64,
    pub ice: f64,
    /// N₂, CH₄ or CO₂ frost ([`FROST_LAYER_BAR`], issue #464).
    pub frost: f64,
    pub cloud: f64,
}

/// Albedo of open water, averaged over the sun's angles: about 0.06
/// (Payne 1972, doi:10.1175/1520-0469(1972)029<0959:AOTSS>2.0.CO;2,
/// monthly means 0.06 to 0.07 in the tropics; from memory, not checked).
const OCEAN_ALBEDO: f64 = 0.06;

/// Albedo of the weathered soils, sands and dusts of a body with an air:
/// 0.25. Calibration on Mars, whose thin air leaves its Bond albedo (0.25,
/// the NASA Mars fact sheet) to its dust, within Earth's land range (0.1
/// for forests to 0.4 for deserts; Henderson-Sellers & Wilson 1983,
/// doi:10.1029/RG021i008p01743, from memory, not checked).
const SOIL_ALBEDO: f64 = 0.25;

/// Albedo of an airless body's regolith, darkened by space weathering
/// (micrometeorites and the solar wind make nanophase iron in its grains;
/// Hapke 2001, doi:10.1029/2000JE001338): 0.11. Calibration on the Moon
/// (Bond 0.11) and Mercury (0.09), the NASA fact sheets.
const REGOLITH_ALBEDO: f64 = 0.11;

/// Surface pressure, bar, that shields a surface from space weathering:
/// the land's albedo goes from [`REGOLITH_ALBEDO`] to [`SOIL_ALBEDO`] as
/// `1 − exp(−p / SHIELD_BAR)`. Calibration, no source: any value between
/// the exospheres of Mercury and the Moon (under 1e-12 bar) and Mars's
/// 6 mbar gives both their albedos; 1e-4 bar stops the solar wind and the
/// small micrometeorites under a column of about a kilogram per square
/// metre.
const SHIELD_BAR: f64 = 1.0e-4;

/// Albedo of water ice and snow, sea ice and land snow alike: 0.6.
/// Calibration, within the spec's 0.5 to 0.8: bare sea ice is about 0.5
/// and snow-covered sea ice or land snow 0.7 to 0.85 (Perovich et al.
/// 2002, doi:10.1029/2000JC000438; Warren 1982,
/// doi:10.1029/RG020i001p00067; from memory, not checked); a
/// snowball Earth, 71 % frozen sea under 29 % snowy land, averages
/// about 0.6 over its disc, and the planet-wide cover does not tell the
/// sea ice from the snow.
const ICE_ALBEDO: f64 = 0.6;

/// Albedo of the frosts of nitrogen, methane and carbon dioxide: 0.75.
/// Calibration on the two bodies covered in them: Triton's Bond albedo,
/// about 0.75 (Voyager 2; Hillier et al. 1990,
/// doi:10.1126/science.250.4979.419, from memory, not checked), and
/// Pluto's, 0.72 ± 0.07 (New Horizons; Buratti et al. 2017,
/// doi:10.1016/j.icarus.2016.07.012, from memory, not checked); within
/// the spec's "high", brighter than the model's water ice and snow
/// ([`ICE_ALBEDO`]), which mixes in sea ice.
const FROST_ALBEDO: f64 = 0.75;

/// Frost, bar of the body's own surface pressure, that covers a body's
/// whole disc; less covers the share `frost / FROST_LAYER_BAR` of it:
/// a frost gathers into deposits in its cold traps (poles, basins,
/// shadows) and is spread no thinner, as [`ICE_SHEET_KM`] does for water
/// ice. 0.02 bar: about 0.7 m of CO₂ frost on Io's gravity, or 2.5 m of
/// N₂ frost on Triton's. Calibration on the moons, no source: the
/// model's Io freezes 0.013 bar of CO₂ out of its air (the real one's
/// frost is SO₂, which the model keeps far less of), which covers about
/// two-thirds of its disc and gives it a Bond albedo of 0.56 (0.63
/// observed, Simonelli & Veverka 1988, from memory, not checked; its
/// frost-free ground is shielded regolith); Triton's 0.36 bar of N₂
/// frost covers all of it with any value under that; Europa's trace of
/// CO₂ frost (2e-4 bar) leaves it to its water ice.
const FROST_LAYER_BAR: f64 = 0.02;

/// Annual mean temperature, K, under which a surface keeps ice or snow:
/// −5 °C. Calibration: Budyko 1969 (doi:10.3402/tellusa.v21i5.10109)
/// takes −10 °C and other energy-balance models 0 °C; −5 °C puts Earth's
/// ice and snow at about a tenth of its disc (land ice 3 %, sea ice about
/// 5 %, seasonal snow a few percent) with [`ICE_LINE_CONTRAST_K`].
const ICE_LINE_K: f64 = 268.15;

/// Equator-to-mean temperature contrast, K, of an Earth-like planet: the
/// annual mean temperature is `T(x) = T_mean − ΔT (x² − 1/3)` with `x` the
/// sine of the latitude, so `ΔT = 3/2 · 28 K` from North 1975's fit of
/// Earth (`T₂ = −28 K` on the Legendre polynomial `P₂`,
/// doi:10.1175/1520-0469(1975)032<2033:TOSEBC>2.0.CO;2, from memory, not
/// checked). The same for every planet: the transport and the rotation
/// that set it are not modelled.
const ICE_LINE_CONTRAST_K: f64 = 42.0;

/// Mean surface temperature, K, above which a body keeps no ice
/// ([`ice_line_share`]): its poles reach the ice line, `T_mean − 2/3 ΔT`.
pub(crate) const ICE_FREE_K: f64 = ICE_LINE_K + 2.0 / 3.0 * ICE_LINE_CONTRAST_K;

/// Mean surface temperature, K, at and under which a body's water is all
/// ice ([`ice_line_share`]): its equator reaches the ice line,
/// `T_mean + 1/3 ΔT`. The frozen branch of the climate starts there
/// ([`super::climate::climate`]).
pub(crate) const ICE_COVERED_K: f64 = ICE_LINE_K - ICE_LINE_CONTRAST_K / 3.0;

/// Mean thickness, km, of the ice sheets a body's water ice gathers
/// into where it is cold: 1.5 km (Antarctica's averages about 2 km,
/// Greenland's 1.7 km, Fretwell et al. 2013, doi:10.5194/tc-7-375-2013;
/// Mars's polar layered deposits 1 to 3 km over about 1 % of its surface,
/// Plaut et al. 2007, doi:10.1126/science.1139672; from memory, not
/// checked). Ice is spread no thinner, so that a nearly dry body (a Mars
/// with a few thousandths of an ocean) whitens only its poles: its frost
/// sublimates and gathers there instead of covering the disc.
const ICE_SHEET_KM: f64 = 1.5;

/// Depth, km, of a global layer of one Earth ocean of water over Earth's
/// surface: 1.4e21 kg at 1000 kg/m³ over 5.1e14 m². The model takes
/// every body's area as Earth's here (the cover is given no radius).
const OCEAN_LAYER_KM: f64 = 2.745;

/// Earth's share of its disc under oceans, 0.71, for its one ocean
/// ([`ocean_fraction`]).
const EARTH_OCEAN_FRACTION: f64 = 0.71;

/// Water vapour, bar, over which the water cloud cover grows as
/// `1 − exp(−p_H₂O / CLOUD_H2O_BAR)`: 1.15e-2 bar, which puts Earth's
/// 1.26e-2 bar of vapour under its cover of about two-thirds (0.67, ISCCP
/// and MODIS; Stubenrauch et al. 2013, doi:10.1175/BAMS-D-12-00117.1,
/// from memory, not checked). Calibration on Earth alone, its form a
/// choice: the air of a frozen planet (about 1e-4 bar at 230 K) holds
/// about 1 % of cloud cover, Mars's under 0.2 %; a warmer, moister world
/// is cloudier towards an overcast sky, as the general circulation
/// models of moist greenhouses find their clouds thickening with the
/// light (Wolf & Toon 2015, doi:10.1002/2015JD023302, from memory, not
/// checked), which keeps an all-ocean world on Earth's side of the
/// runaway limit.
const CLOUD_H2O_BAR: f64 = 1.15e-2;

/// Earth's cloud cover, `0.67`, the one [`CLOUD_H2O_BAR`] is calibrated
/// on: the look draws it as no shift of the cover.
pub(crate) const EARTH_CLOUD_COVER: f64 = 0.67;

/// Albedo of a water cloud layer, as the reflectance of the cloud cover's
/// share of the disc: 0.19. Calibration on Earth's Bond albedo, 0.30
/// (CERES; Stephens et al. 2015, doi:10.1002/2014RG000449), with its
/// ocean, land, ice and Rayleigh scattering, as the model derives them
/// at 288 K. Lower than a thick cloud's 0.5 to 0.7: Earth's cloud cover
/// counts thin clouds too, and the model's air absorbs no sunlight (the
/// real one takes about a fifth of it, water vapour and ozone), so its
/// clear sky (about 0.2) is brighter than Earth's (about 0.15) and its
/// clouds carry less of the albedo.
const CLOUD_ALBEDO: f64 = 0.19;

/// Albedo of a sulphuric acid deck, which hides all below it: Venus's
/// Bond albedo, 0.76 (Haus et al. 2016, doi:10.1016/j.icarus.2015.11.011,
/// from memory, not checked). Its only body.
const DECK_ALBEDO: f64 = 0.76;

/// Least SO₂, bar, that makes a sulphuric deck. The look reads it too
/// (`interim::sky_of` asks [`sulphuric_deck`]), so that the albedo and
/// the look agree.
pub(crate) const DECK_SO2_BAR: f64 = 1.0e-3;

/// Least surface temperature, K, at which a sulphuric deck forms: the
/// acid condenses into a deck in a warm air (Venus's, 230 to 350 K at its
/// cloud levels over a 737 K surface); a frozen world's SO₂ makes none.
/// Calibration, no source: the melting point of ice.
const DECK_MIN_K: f64 = 273.15;

/// Methane, bar, at which a methane haze reaches `1 − 1/e` of its
/// strength. Calibration, no source: Titan's 1.5 to 5 % of 1.5 bar (0.02
/// to 0.07 bar) make a thick haze, an Earth's trace none.
const HAZE_CH4_BAR: f64 = 0.01;

/// Least ratio of methane to CO₂ for an organic haze: about 0.2 (Trainer
/// et al. 2006, doi:10.1073/pnas.0608561103, hazes from CH₄/CO₂ above
/// 0.1 to 0.2, from memory, not checked); the haze grows in proportion to
/// the ratio below it.
const HAZE_CH4_PER_CO2: f64 = 0.2;

/// Reflectance and absorption of a thick methane haze, as fractions of
/// the light falling on it: 0.25 and 0.55. Calibration on Titan, the one
/// hazy body (Bond albedo 0.27, Li et al. 2011,
/// doi:10.1029/2011GL047746, from memory, not checked): the orange haze
/// absorbs most of the blue and hides the bright ice below.
const HAZE_ALBEDO: f64 = 0.25;
const HAZE_ABSORPTION: f64 = 0.55;

/// Rayleigh optical depth at 550 nm of a bar of nitrogen: 0.0965 (Earth's
/// 1013 hPa of air give 0.097; Bucholtz 1995, doi:10.1364/AO.34.002765,
/// from memory, not checked). The depth grows with the column's mass,
/// which the model takes from the pressure alone (Earth's gravity).
const RAYLEIGH_N2_550: f64 = 0.0965;

/// Rayleigh depth per bar of each gas, relative to nitrogen: its
/// molecules scatter as the square of their polarizability and a bar of
/// it holds `28 / μ` times as many molecules (CRC Handbook
/// polarizabilities: N₂ 1.74, CO₂ 2.91, CH₄ 2.59, NH₃ 2.26,
/// SO₂ 3.72, O₂ 1.58 Å³; King factors ignored). Water vapour is left
/// out: it scatters, but absorbs more of the near infrared, so that a
/// moist atmosphere's albedo stays near 0.2 to 0.3 (Kopparapu et al.
/// 2013); the climate blends a steam column towards that moist albedo
/// (`climate::runaway_albedo`).
fn rayleigh_550(gases: &Gases) -> f64 {
    let per = |alpha: f64, mu: f64| (alpha / 1.74).powi(2) * 28.0 / mu;
    RAYLEIGH_N2_550
        * (gases.n2.max(0.0)
            + gases.co2.max(0.0) * per(2.91, 44.0)
            + gases.ch4.max(0.0) * per(2.59, 16.0)
            + gases.nh3.max(0.0) * per(2.26, 17.0)
            + gases.so2.max(0.0) * per(3.72, 64.0)
            + gases.o2.max(0.0) * per(1.58, 32.0))
}

/// Effective temperature, K, of the light the Rayleigh scattering is
/// averaged over: the Sun's (the star's spectrum is not passed; a red
/// star's light scatters less).
const RAYLEIGH_SPECTRUM_K: f64 = 5780.0;

/// The light of a black body at [`RAYLEIGH_SPECTRUM_K`], by wavelength:
/// each band's `(550 nm / λ)⁴` and its share of the light, from 0.2 to
/// 5 µm in 240 bands even in ln λ.
static SPECTRUM: LazyLock<Vec<(f64, f64)>> = LazyLock::new(|| {
    let (from, to, n) = (0.2e-6_f64, 5.0e-6_f64, 240);
    let step = (to / from).ln() / f64::from(n);
    // h c / k, m·K.
    let c2 = 1.438_777e-2;
    let bands: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let lambda = from * (step * (f64::from(i) + 0.5)).exp();
            // B_λ dλ with dλ = λ d(ln λ).
            let planck = lambda.powi(-4) / ((c2 / (lambda * RAYLEIGH_SPECTRUM_K)).exp() - 1.0);
            ((0.55e-6 / lambda).powi(4), planck)
        })
        .collect();
    let total: f64 = bands.iter().map(|b| b.1).sum();
    bands.into_iter().map(|(r, w)| (r, w / total)).collect()
});

/// Reflectance of a conservative Rayleigh layer of optical depth `tau`
/// over a black surface, in the two-stream approximation:
/// `(3/4) τ / (1 + (3/4) τ)` (isotropic phase function; Bohren 1987,
/// doi:10.1119/1.15109, from memory, not checked).
fn rayleigh_reflectance(tau: f64) -> f64 {
    let x = 0.75 * tau.max(0.0);
    x / (1.0 + x)
}

/// The albedo of a layer of reflectance `r` and transmission `t` over a
/// lower albedo `below`, with the light reflected back and forth between
/// them: `r + t² below / (1 − r below)`.
fn over(r: f64, t: f64, below: f64) -> f64 {
    r + t * t * below / (1.0 - r * below)
}

/// Share of a disc under open ocean for `oceans` Earth oceans of liquid
/// water: `1 − exp(−k oceans)`, with `k` putting Earth's one ocean at
/// 0.71 of its disc. A fit, labelled so: a body's sea level follows its
/// relief, which the model does not have (the world generation fills its
/// drawn relief, `interim::calibrate`); this one rises fast while the
/// water fills the basins and saturates as it drowns the continents.
pub fn ocean_fraction(oceans: f64) -> f64 {
    let k = -(1.0 - EARTH_OCEAN_FRACTION).ln();
    1.0 - (-k * oceans.max(0.0)).exp()
}

/// Share of the disc colder than [`ICE_LINE_K`] on a body whose mean
/// surface temperature is `temperature_k`, with the temperature falling
/// to the poles as `T_mean − ΔT (x² − 1/3)` ([`ICE_LINE_CONTRAST_K`]): an
/// energy-balance ice line in the manner of Budyko and Sellers. None
/// above about 296 K, all of it below about 254 K.
fn ice_line_share(temperature_k: f64) -> f64 {
    let x2 = 1.0 / 3.0 + (temperature_k - ICE_LINE_K) / ICE_LINE_CONTRAST_K;
    1.0 - x2.clamp(0.0, 1.0).sqrt()
}

/// What covers a body at `temperature_k` under `gases` with `water`
/// and `frost_bar` of N₂, CH₄ and CO₂ frost on its ground ([`Cover`]).
///
/// - Its water, out of a runaway, is open ocean ([`ocean_fraction`] of
///   its oceans) where the surface is warmer than the ice line and ice or
///   snow where it is colder ([`ice_line_share`]), the ice limited by
///   sheets the water can make ([`ICE_SHEET_KM`]).
/// - The rest is bare land.
/// - Its frost lies over the ocean's ice and the land, covering the share
///   `frost_bar / FROST_LAYER_BAR` of the disc ([`FROST_LAYER_BAR`]), all
///   of it from that amount on.
/// - Its water clouds follow the vapour in the air ([`CLOUD_H2O_BAR`]):
///   none in a dry or airless sky.
pub fn cover(temperature_k: f64, gases: &Gases, water: &Water, frost_bar: f64) -> Cover {
    let wet = water.is_wet();
    let (sea, ice) = if wet {
        let reach = (water.oceans.max(0.0) * OCEAN_LAYER_KM / ICE_SHEET_KM).min(1.0);
        (
            ocean_fraction(water.oceans),
            ice_line_share(temperature_k) * reach,
        )
    } else {
        (0.0, 0.0)
    };
    let frost = (frost_bar.max(0.0) / FROST_LAYER_BAR).min(1.0);
    let ocean = sea * (1.0 - ice) * (1.0 - frost);
    let ice = ice * (1.0 - frost);
    let land = (1.0 - ocean - ice - frost).max(0.0);
    let cloud = 1.0 - (-gases.h2o.max(0.0) / CLOUD_H2O_BAR).exp();
    Cover {
        ocean,
        land,
        ice,
        frost,
        cloud,
    }
}

/// Whether the air of a body at `temperature_k` makes a sulphuric acid
/// deck: its SO₂, as the washing out leaves it over the surface's liquid
/// ([`super::atmosphere::wash_out`]: none over liquid water), reaches
/// [`DECK_SO2_BAR`], in a warm air ([`DECK_MIN_K`]).
pub(crate) fn sulphuric_deck(temperature_k: f64, gases: &Gases, liquid_water: bool) -> bool {
    use super::climate::SurfaceLiquid;
    let liquid = if liquid_water {
        SurfaceLiquid::Water
    } else {
        SurfaceLiquid::None
    };
    temperature_k >= DECK_MIN_K && super::atmosphere::wash_out(*gases, liquid).so2 >= DECK_SO2_BAR
}

/// Strength of a methane haze in `gases`, 0 to 1 ([`HAZE_CH4_BAR`],
/// [`HAZE_CH4_PER_CO2`]).
fn haze(gases: &Gases) -> f64 {
    let ch4 = gases.ch4.max(0.0);
    if ch4 <= 0.0 {
        return 0.0;
    }
    let ratio = if gases.co2 > 0.0 {
        (ch4 / (HAZE_CH4_PER_CO2 * gases.co2)).min(1.0)
    } else {
        1.0
    };
    (1.0 - (-ch4 / HAZE_CH4_BAR).exp()) * ratio
}

/// The Bond albedo of a body with `cover` under `gases`, with a
/// sulphuric acid deck when `deck`.
///
/// The surface mixes its covers' albedos (ocean [`OCEAN_ALBEDO`], land
/// from [`REGOLITH_ALBEDO`] when airless to [`SOIL_ALBEDO`] under an air,
/// ice and snow [`ICE_ALBEDO`], frost [`FROST_ALBEDO`]); the water clouds' layer
/// ([`CLOUD_ALBEDO`] over their cover) lies over it, the Rayleigh
/// scattering of the column over both (averaged over the Sun's spectrum
/// band by band: what caps CO₂'s warming at the maximum greenhouse,
/// Kopparapu et al. 2013, doi:10.1088/0004-637X/765/2/131), and a methane
/// haze over all ([`HAZE_ALBEDO`], [`HAZE_ABSORPTION`]). A deck hides
/// everything below it ([`DECK_ALBEDO`]).
pub fn bond_albedo(cover: &Cover, gases: &Gases, deck: bool) -> f64 {
    if deck {
        return DECK_ALBEDO;
    }
    let shield = 1.0 - (-gases.total().max(0.0) / SHIELD_BAR).exp();
    let land = REGOLITH_ALBEDO + (SOIL_ALBEDO - REGOLITH_ALBEDO) * shield;
    let surface = cover.ocean * OCEAN_ALBEDO
        + cover.land * land
        + cover.ice * ICE_ALBEDO
        + cover.frost * FROST_ALBEDO;
    let cloud = (cover.cloud * CLOUD_ALBEDO).clamp(0.0, 1.0);
    let clouded = over(cloud, 1.0 - cloud, surface);
    let tau = rayleigh_550(gases);
    let h = haze(gases);
    let (hr, ht) = (h * HAZE_ALBEDO, 1.0 - h * (HAZE_ALBEDO + HAZE_ABSORPTION));
    SPECTRUM
        .iter()
        .map(|&(scale, weight)| {
            let r = rayleigh_reflectance(tau * scale);
            weight * over(hr, ht, over(r, 1.0 - r, clouded))
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn earth_water() -> Water {
        Water {
            runaway: false,
            boiled: false,
            kept_kg: super::super::OCEAN_KG,
            oceans: 1.0,
        }
    }

    #[test]
    fn bare_rock_ignores_ocean_and_cloud() {
        // Review Focus 1: no water, no air (Mercury, the Moon).
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        let airless = Gases::default();
        assert!(airless.total() < 1e-6);
        for t in [100.0, 250.0, 440.0, 700.0] {
            let c = cover(t, &airless, &dry, 0.0);
            assert_eq!(c.ocean, 0.0, "{t}: {c:?}");
            assert_eq!(c.cloud, 0.0, "{t}: {c:?}");
            assert_eq!(c.ice, 0.0, "{t}: {c:?}");
            assert_eq!(c.land, 1.0, "{t}: {c:?}");
            let a = bond_albedo(&c, &airless, false);
            assert!((0.06..=0.14).contains(&a), "{t}: {a}");
        }
    }

    #[test]
    fn ice_cover_grows_as_the_planet_cools() {
        let air = Gases {
            n2: 0.78,
            o2: 0.21,
            co2: 4e-4,
            h2o: 0.01,
            ..Gases::default()
        };
        let water = earth_water();
        let mut last = f64::INFINITY;
        for i in 0..=100 {
            let t = 200.0 + f64::from(i);
            let ice = cover(t, &air, &water, 0.0).ice;
            assert!(ice <= last, "{t}: {ice} > {last}");
            last = ice;
        }
        let earth = cover(288.0, &air, &water, 0.0).ice;
        assert!((0.06..=0.14).contains(&earth), "{earth}");
        for t in [200.0, 230.0, 240.0, 250.0] {
            let ice = cover(t, &air, &water, 0.0).ice;
            assert!(ice > 0.999, "{t}: {ice}");
        }
        assert_eq!(cover(300.0, &air, &water, 0.0).ice, 0.0);
    }

    #[test]
    fn a_nearly_dry_planet_whitens_only_its_poles() {
        // Mars-like: a few thousandths of an ocean at 210 K make polar
        // caps, not a white disc; a snowball Earth is white all over.
        let air = Gases {
            co2: 6e-3,
            ..Gases::default()
        };
        let mars = Water {
            oceans: 0.005,
            kept_kg: 0.005 * super::super::OCEAN_KG,
            ..earth_water()
        };
        let caps = cover(210.0, &air, &mars, 0.0);
        assert!(caps.ice > 0.0 && caps.ice < 0.02, "{caps:?}");
        let a = bond_albedo(&caps, &air, false);
        assert!((0.2..=0.3).contains(&a), "{a}");
        assert_eq!(cover(210.0, &air, &earth_water(), 0.0).ice, 1.0);
    }

    #[test]
    fn frost_brightens_a_body_in_proportion_to_its_amount() {
        // Issue #464: a frost covers `frost / FROST_LAYER_BAR` of the
        // disc over the land and the ice, and all of it from there on.
        let dry = Water {
            runaway: false,
            boiled: false,
            kept_kg: 0.0,
            oceans: 0.0,
        };
        let thin = Gases {
            n2: 1.4e-5,
            ..Gases::default()
        };
        let a = |frost: f64| bond_albedo(&cover(40.0, &thin, &dry, frost), &thin, false);
        let half = cover(40.0, &thin, &dry, FROST_LAYER_BAR / 2.0);
        assert!((half.frost - 0.5).abs() < 1e-12, "{half:?}");
        assert!((half.land - 0.5).abs() < 1e-12, "{half:?}");
        assert!(a(0.0) < a(FROST_LAYER_BAR / 2.0));
        assert!(a(FROST_LAYER_BAR / 2.0) < a(FROST_LAYER_BAR));
        assert_eq!(a(FROST_LAYER_BAR), a(10.0));
        assert!((a(10.0) - FROST_ALBEDO).abs() < 0.01, "{}", a(10.0));
        // Over ice, the frost takes its share from the ice.
        let icy = cover(40.0, &thin, &earth_water(), FROST_LAYER_BAR / 4.0);
        assert!(
            (icy.ice - 0.75).abs() < 1e-12 && icy.ocean == 0.0,
            "{icy:?}"
        );
    }

    #[test]
    fn earth_has_seventy_one_percent_of_ocean() {
        assert!((ocean_fraction(1.0) - 0.71).abs() < 1e-12);
        assert_eq!(ocean_fraction(0.0), 0.0);
        assert!(ocean_fraction(0.1) < ocean_fraction(1.0));
        assert!(ocean_fraction(10.0) > 0.99999);
    }

    #[test]
    fn thick_co2_scatters_more_light() {
        // Rayleigh scattering grows with the column, and saturates.
        let c = Cover {
            ocean: 0.0,
            land: 1.0,
            ice: 0.0,
            frost: 0.0,
            cloud: 0.0,
        };
        let a = |co2: f64| {
            bond_albedo(
                &c,
                &Gases {
                    n2: 1.0,
                    co2,
                    ..Gases::default()
                },
                false,
            )
        };
        let (thin, eight, ninety) = (a(1e-3), a(8.0), a(90.0));
        println!("albedo under 0, 8, 90 bar of CO2: {thin:.3} {eight:.3} {ninety:.3}");
        assert!(thin < eight && eight < ninety, "{thin} {eight} {ninety}");
        assert!(ninety < 1.0);
    }
}
