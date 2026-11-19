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
use super::climate::{
    EARTH_GRAVITY_M_S2, Planet, WATER_LATENT_J_KG, water_boils_k, water_saturation_bar,
};
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
    /// N₂, CH₄, CO₂ or SO₂ frost ([`FROST_LAYER_BAR`], issue #464).
    pub frost: f64,
    /// The share of the disc of that frost that is SO₂ frost
    /// ([`SO2_FROST_ALBEDO`]), part of `frost`.
    pub so2_frost: f64,
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
/// N₂ frost on Triton's. Calibration on the moons, no source: it was
/// set on the CO₂ frost the model's Io then froze out of its air (0.013
/// bar, two-thirds of its disc, a Bond albedo of 0.56); that air now goes
/// to Jupiter's plasma and Io's frost is the SO₂ of its volcanoes, whose
/// amount [`super::atmosphere::volcanic_so2_frost_bar`] is calibrated
/// against this layer. Triton's 0.36 bar of N₂ frost covers all of it
/// with any value under that; Europa's trace of CO₂ frost (2e-4 bar)
/// leaves it to its water ice.
pub(crate) const FROST_LAYER_BAR: f64 = 0.02;

/// Albedo of the SO₂ frost of a volcanic body: 0.63, Io's Bond albedo
/// (Simonelli & Veverka 1988, from memory, not checked), the albedo of a
/// disc that frost covers nearly all of (Douté et al. 2001, Icarus 149,
/// 107, doi:10.1006/icar.2000.6513, map it over most of Io; from memory,
/// not checked). Calibration on Io: darker than [`FROST_ALBEDO`], since
/// Io's frost lies mixed with its sulphur and silicate deposits. With it
/// the frost's dayside lies at Io's 106 to 117 K
/// ([`super::atmosphere::volcanic_so2_vapour`]); under [`FROST_ALBEDO`] it would
/// lie near 103 K, and its vapour at 4e-11 bar, under Io's nanobar.
const SO2_FROST_ALBEDO: f64 = 0.63;

/// Bond albedo of clean, fine-grained water ice: 0.85. Calibration, the
/// fresh end of the icy-age law ([`icy_unit_albedo`]): brighter than
/// Enceladus (0.81 ± 0.04; Howett et al. 2010, Icarus 206, 573,
/// doi:10.1016/j.icarus.2009.07.016, from memory, not checked), whose ice
/// holds some dust, and than the model's sea ice and snow
/// ([`ICE_ALBEDO`]); pure water frost reflects about 0.9 of the visible
/// but absorbs its near infrared, about half of sunlight.
const PURE_ICE_ALBEDO: f64 = 0.85;

/// Rock dust in a fresh ice surface per unit of the body's rock-to-ice
/// mass ratio, as a share of its grains: 0.006 ([`icy_unit_albedo`]). A
/// body's renewed ice carries the rock its ocean and its crust were in
/// contact with (Europa's hydrated salts; McCord et al. 1998, Science
/// 280, 1242, from memory, not checked): a mostly-rock body's ice is
/// dirtier. Calibration on Europa: its rock-to-ice ratio of 9 to 16 puts
/// 5 to 10 % of dust in its young ice, a Bond albedo of 0.59 to 0.66
/// (observed about 0.62 to 0.68, from memory).
const DUST_PER_ROCK_TO_ICE: f64 = 0.006;

/// Most of an old ice surface a dark lag covers on a body of
/// [`LAG_ROCK_SHARE`] rock or more: 0.88 ([`icy_unit_albedo`]).
/// Sublimation and impact gardening leave the rock of an old ice-rock
/// crust as a dark lag, with the ice redeposited as bright frost on
/// cold crests and poleward slopes (Spencer 1987, Icarus 69, 297; Moore
/// et al. 1999, Icarus 140, 294, doi:10.1006/icar.1999.6132, Callisto
/// mantled by dark debris but for its bright crests; both from memory,
/// not checked). Calibration on Callisto's Bond albedo, about 0.2.
const LAG_MAX_SHARE: f64 = 0.88;

/// Rock share of a body, as a fraction of its mass, from which its lag
/// reaches [`LAG_MAX_SHARE`]; with less rock it covers less, in
/// proportion (a crust of 94 % ice has little rock to leave): 0.5,
/// Callisto's, the body [`LAG_MAX_SHARE`] is calibrated on (about half
/// rock, Anderson et al. 2001, doi:10.1006/icar.2001.6664, from memory,
/// not checked; the `callisto-like` archetype draws 45 to 55 % ice).
/// A reference, not a free constant.
const LAG_ROCK_SHARE: f64 = 0.5;

/// Craters of 1 km or more per km² over which a lag covers `1 − 1/e` of
/// [`LAG_MAX_SHARE`]: 0.004, the count of the lunar chronology on a
/// surface about 3.45 Gyr old ([`crate::craters::crater_density`]). The
/// lag follows the impacts a surface has taken, which garden it and
/// bring its dust: slowly at today's rate, fast under the late heavy
/// bombardment. Calibration on Ganymede: its 2 Gyr renewed terrain
/// keeps 70 % of its fresh ice uncovered, its 4.5 Gyr dark terrain is
/// Callisto's.
const LAG_CRATERS_PER_KM2: f64 = 0.004;

/// Share of a body's ice that the fallout of its plume keeps under fresh
/// frost ([`PURE_ICE_ALBEDO`]) when it vents at full strength
/// (`geysers::vent_strength`): 0.6. Enceladus's plume
/// feeds Saturn's E ring and falls back over its whole surface, fastest
/// near its south pole, so that its surface stays as bright as fresh
/// frost whatever its craters' age (Verbiscer et al. 2007, Science 315,
/// 815, doi:10.1126/science.1134681; Kempf et al. 2010, Icarus 206, 446,
/// doi:10.1016/j.icarus.2009.09.016; both from memory, not checked).
/// Calibration on Enceladus's Bond albedo, 0.81 (Howett et al. 2010).
const FALLOUT_SHARE: f64 = 0.6;

/// Single-scattering albedo of an isotropic scatterer whose diffusive
/// reflectance is `albedo` (the inverse of [`diffusive_albedo`]).
fn single_scattering(albedo: f64) -> f64 {
    let gamma = (1.0 - albedo) / (1.0 + albedo);
    1.0 - gamma * gamma
}

/// Diffusive (bihemispherical) reflectance of a half-space of isotropic
/// scatterers of single-scattering albedo `w`: `(1 − γ) / (1 + γ)` with
/// `γ = √(1 − w)` (Hapke 1981, JGR 86, 3039,
/// doi:10.1029/JB086iB04p03039, from memory, not checked), taken as
/// approximately the Bond albedo of a surface whose grains scatter alike
/// in every direction (Hapke's spherical albedo is slightly lower for
/// dark grains).
fn diffusive_albedo(w: f64) -> f64 {
    let gamma = (1.0 - w).clamp(0.0, 1.0).sqrt();
    (1.0 - gamma) / (1.0 + gamma)
}

/// Bond albedo of a unit of exposed water ice on an ice crust, last
/// renewed `age_gyr` Gyr ago, on a body whose rock is `rock_share` of
/// its mass, venting with the strength `vent`, 0 to 1
/// (`geysers::vent_strength`).
///
/// - Its fresh ice is an intimate mix of [`PURE_ICE_ALBEDO`] ice and
///   rock dust at [`REGOLITH_ALBEDO`], the dust `DUST_PER_ROCK_TO_ICE`
///   times the body's rock-to-ice ratio: their single-scattering
///   albedos mix by grain and the mix reflects as Hapke's isotropic
///   half-space ([`diffusive_albedo`]). A little dust darkens ice much.
/// - A dark lag at [`REGOLITH_ALBEDO`] covers patches of it, a share
///   `LAG_MAX_SHARE · min(1, rock / LAG_ROCK_SHARE) · (1 − exp(−N /
///   LAG_CRATERS_PER_KM2))` of the surface, `N` the crater count of the
///   lunar chronology on a surface that old
///   ([`crate::craters::crater_density`]); the patches mix by area.
/// - A plume's fallout covers `vent ·` [`FALLOUT_SHARE`] of it with
///   fresh frost.
pub fn icy_unit_albedo(age_gyr: f64, rock_share: f64, vent: f64) -> f64 {
    let rock = rock_share.clamp(0.0, 1.0);
    let ice = 1.0 - rock;
    let dust = if DUST_PER_ROCK_TO_ICE * rock >= ice {
        1.0
    } else {
        DUST_PER_ROCK_TO_ICE * rock / ice
    };
    let w = (1.0 - dust) * single_scattering(PURE_ICE_ALBEDO)
        + dust * single_scattering(REGOLITH_ALBEDO);
    let fresh = diffusive_albedo(w);
    let craters = crate::craters::crater_density(age_gyr.max(0.0));
    let ceiling = LAG_MAX_SHARE * (rock / LAG_ROCK_SHARE).min(1.0);
    let lag = ceiling * (1.0 - (-craters / LAG_CRATERS_PER_KM2).exp());
    let aged = (1.0 - lag) * fresh + lag * REGOLITH_ALBEDO;
    let fallout = FALLOUT_SHARE * vent.clamp(0.0, 1.0);
    (1.0 - fallout) * aged + fallout * PURE_ICE_ALBEDO
}

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
/// surface: 1.4e21 kg at 1000 kg/m³ over 5.1e14 m². Over a body of
/// radius `R` (Earth radii) the same water makes a layer `1 / R²` as
/// deep ([`cover`], issue #523).
const OCEAN_LAYER_KM: f64 = 2.745;

/// Earth's share of its disc under oceans, 0.71, for its one ocean
/// ([`ocean_fraction`]).
const EARTH_OCEAN_FRACTION: f64 = 0.71;

/// Water vapour, bar, over which the water cloud cover grows as
/// `1 − exp(−p_H₂O / CLOUD_H2O_BAR)` at saturation (see
/// [`CLOUD_HUMIDITY_EXPONENT`] for the humidity): 1.0e-2 bar.
/// Calibration on Earth alone, its form a choice: Earth's 1.26e-2 bar of
/// vapour at 77 % relative humidity then makes its cover of about
/// two-thirds (0.67, ISCCP and MODIS; Stubenrauch et al. 2013,
/// doi:10.1175/BAMS-D-12-00117.1, from memory, not checked). The vapour stands in for the cloud
/// water the model does not carry: the air of a frozen planet (about
/// 1e-4 bar at 230 K) holds about 1 % of cloud cover, Mars's under
/// 0.2 %; a warmer, moister world is cloudier towards an overcast sky,
/// as the general circulation models of moist greenhouses find their
/// clouds thickening with the light (Wolf & Toon 2015,
/// doi:10.1002/2015JD023302, from memory, not checked), which keeps an all-ocean world on Earth's
/// side of the runaway limit.
const CLOUD_H2O_BAR: f64 = 1.0e-2;

/// Exponent of the relative humidity in the cloud cover, 0.25: the
/// factor `RH^0.25` of the semi-empirical cloudiness of Xu & Randall
/// 1996 (*J. Atmos. Sci.* 53, 3084, doi:10.1175/1520-0469(1996)053<3084:ASCPFU>2.0.CO;2),
/// fitted to cloud-resolving simulations. Their other factor, the cloud
/// water over the saturation deficit, is the vapour term of
/// [`CLOUD_H2O_BAR`] here. The same vapour in a warmer air is further
/// from saturation and covers less of the sky: a dry world whose air
/// holds all of its little water is clearer than an ocean world with the
/// same vapour (issue #523).
const CLOUD_HUMIDITY_EXPONENT: f64 = 0.25;

/// Earth's cloud cover, `0.67`, the one [`CLOUD_H2O_BAR`] is calibrated
/// on: the look draws it as no shift of the cover.
pub(crate) const EARTH_CLOUD_COVER: f64 = 0.67;

/// Albedo of the tops of a water cloud layer, before the vapour above
/// them absorbs any of the light ([`vapour_transmission`]), as the
/// reflectance of the cloud cover's share of the disc: 0.297.
/// Calibration on Earth's Bond albedo, 0.30 (CERES; Stephens et al. 2015,
/// doi:10.1002/2014RG000449), with its ocean, land, ice, Rayleigh
/// scattering and water vapour, as the model derives them at 288 K
/// (0.19 while the air absorbed no light, issue #523). Lower than a
/// thick cloud's 0.5 to 0.7: Earth's cloud cover counts thin clouds
/// too.
const CLOUD_ALBEDO: f64 = 0.297;

/// Albedo of a sulphuric acid deck, which hides all below it: Venus's
/// Bond albedo, 0.76 (Haus et al. 2016, doi:10.1016/j.icarus.2015.11.011,
/// from memory, not checked). Its only body.
pub(crate) const DECK_ALBEDO: f64 = 0.76;

/// Least SO₂, bar, that makes a sulphuric deck. The look reads it too
/// (`interim::sky_of` asks [`sulphuric_deck`]), so that the albedo and
/// the look agree.
pub(crate) const DECK_SO2_BAR: f64 = 1.0e-3;

/// Least surface temperature, K, at which a sulphuric deck forms: the
/// acid condenses into a deck in a warm air (Venus's, 230 to 350 K at its
/// cloud levels over a 737 K surface); a frozen world's SO₂ makes none.
/// Calibration, no source: the melting point of ice.
const DECK_MIN_K: f64 = 273.15;

/// Warming, K, over which a sulphuric deck grows from none at
/// [`DECK_MIN_K`] to whole: 40 K. Calibration, no source: a deck that
/// appeared whole at one temperature would leave the albedo's fixed
/// point no balance between a warm clear sky and a cold decked one (the
/// deck's 0.76 against a bare ground's 0.2 to 0.3 moves the surface by
/// some 60 K); spread over 40 K, the deck moves the temperature it
/// derives by less than the step it takes, so that the damped fixed
/// point of `climate::settle` converges. Venus's 737 K is far past it.
const DECK_ONSET_K: f64 = 40.0;

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

/// Rayleigh optical depth at 550 nm of a bar of nitrogen under Earth's
/// gravity: 0.0965 (Earth's 1013 hPa of air give 0.097; Bucholtz 1995,
/// doi:10.1364/AO.34.002765, from memory, not checked). The depth grows
/// with the column's mass, `p / g`: a bar weighs a column
/// `g⊕ / g` times Earth's on a body of gravity `g` ([`rayleigh_550`]).
const RAYLEIGH_N2_550: f64 = 0.0965;

/// Rayleigh scattering of a CO₂ molecule against an N₂ molecule: 2.5.
/// Kopparapu et al. 2013 (doi:10.1088/0004-637X/765/2/131, §2.2, read
/// on arXiv:1301.6674v2): CO₂ "is 2.5 times better than air" at
/// Rayleigh scattering. Sneep & Ubachs 2005 (JQSRT 92, 293,
/// doi:10.1016/j.jqsrt.2004.07.025) measured the cross-sections at
/// 532 nm in a ratio of about 2.4 (from memory, not checked).
const CO2_RAYLEIGH_PER_N2: f64 = 2.5;

/// Rayleigh depth per bar of each gas, relative to nitrogen: its
/// molecules scatter as the square of their polarizability and a bar of
/// it holds `28 / μ` times as many molecules (CRC Handbook
/// polarizabilities: N₂ 1.74, CH₄ 2.59, NH₃ 2.26, SO₂ 3.72, O₂ 1.58 Å³;
/// King factors ignored). CO₂ takes its measured ratio instead,
/// [`CO2_RAYLEIGH_PER_N2`]: its static polarizability (2.91 Å³) would
/// give 2.80, about 12 % too much, since what scatters visible light is
/// the optical polarizability and CO₂'s King factor is larger than
/// N₂'s (issue #523). Water vapour is left
/// out: it scatters, but absorbs more of the near infrared, so that a
/// moist atmosphere's albedo stays near 0.2 to 0.3 (Kopparapu et al.
/// 2013); the climate blends a steam column towards that moist albedo
/// (`climate::runaway_albedo`); it absorbs instead
/// ([`vapour_transmission`]).
///
/// The column is the pressure over `planet`'s gravity: a low-gravity
/// moon (Titan, 1.35 m/s²) holds over seven times the molecules Earth
/// does under the same pressure, a super-Earth fewer (issue #523).
fn rayleigh_550(gases: &Gases, planet: &Planet) -> f64 {
    let per = |alpha: f64, mu: f64| (alpha / 1.74).powi(2) * 28.0 / mu;
    RAYLEIGH_N2_550 * EARTH_GRAVITY_M_S2 / planet.gravity_m_s2
        * (gases.n2.max(0.0)
            + gases.co2.max(0.0) * CO2_RAYLEIGH_PER_N2 * 28.0 / 44.0
            + gases.ch4.max(0.0) * per(2.59, 16.0)
            + gases.nh3.max(0.0) * per(2.26, 17.0)
            + gases.so2.max(0.0) * per(3.72, 64.0)
            + gases.o2.max(0.0) * per(1.58, 32.0))
}

/// Effective temperature, K, of the light the Rayleigh scattering is
/// averaged over when the star's is unknown (not positive), and for
/// [`bond_albedo`]: the Sun's nominal 5772 K (IAU 2015 Resolution B3;
/// from memory, not checked).
pub(crate) const SUN_TEFF_K: f64 = 5772.0;

/// The light of a black body at `teff_k` (a star of that effective
/// temperature), by wavelength: each band's `(550 nm / λ)⁴` and its share
/// of the light, from 0.2 to 5 µm in [`SPECTRUM_BANDS`] bands even in
/// `ln λ`. A redder star's light scatters less: Rayleigh's `λ⁻⁴` weighs
/// the blue (Kopparapu et al. 2013, §4.1, doi:10.1088/0004-637X/765/2/131,
/// read on arXiv:1301.6674v2: the main reason the outer edge of an
/// M dwarf's habitable zone lies at a lower flux; the stronger
/// near-infrared absorption of its light by CO₂ and water, their other
/// reason, is left out). Kept for the last star asked, per thread: a
/// body's climate asks for one star thousands of times.
fn spectrum(teff_k: f64) -> Bands {
    thread_local! {
        static LAST: std::cell::RefCell<Option<(f64, Bands)>> =
            const { std::cell::RefCell::new(None) };
    }
    let teff = if teff_k > 0.0 { teff_k } else { SUN_TEFF_K };
    LAST.with(|last| {
        if let Some((k, bands)) = last.borrow().as_ref()
            && *k == teff
        {
            return bands.clone();
        }
        let (from, to, n) = (0.2e-6_f64, 5.0e-6_f64, SPECTRUM_BANDS);
        let step = (to / from).ln() / f64::from(n);
        // h c / k, m·K.
        let c2 = 1.438_777e-2;
        let bands: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let lambda = from * (step * (f64::from(i) + 0.5)).exp();
                // B_λ dλ with dλ = λ d(ln λ).
                let planck = lambda.powi(-4) / ((c2 / (lambda * teff)).exp() - 1.0);
                ((0.55e-6 / lambda).powi(4), planck)
            })
            .collect();
        let total: f64 = bands.iter().map(|b| b.1).sum();
        let bands = std::rc::Rc::new(
            bands
                .into_iter()
                .map(|(r, w)| (r, w / total))
                .collect::<Vec<_>>(),
        );
        *last.borrow_mut() = Some((teff, bands.clone()));
        bands
    })
}

/// Bands of [`spectrum`].
const SPECTRUM_BANDS: u32 = 240;

/// The bands of a star's light ([`spectrum`]): each band's
/// `(550 nm / λ)⁴` and its share of the light.
type Bands = std::rc::Rc<Vec<(f64, f64)>>;

/// Reflectance of a conservative Rayleigh layer of optical depth `tau`
/// over a black surface, in the two-stream approximation:
/// `(3/4) τ / (1 + (3/4) τ)` (isotropic phase function; Bohren 1987,
/// doi:10.1119/1.15109, from memory, not checked).
fn rayleigh_reflectance(tau: f64) -> f64 {
    let x = 0.75 * tau.max(0.0);
    x / (1.0 + x)
}

/// Earth's mean tropospheric lapse rate, K/m: 6.5e-3 (U.S. Standard
/// Atmosphere 1976). A body's scales with its gravity, as the adiabat
/// `g / c_p` does ([`vapour_column_kg_m2`]).
const EARTH_LAPSE_K_M: f64 = 6.5e-3;

/// Mass of water vapour over a square metre, kg/m², of an air whose
/// vapour at the surface is `h2o_bar` on `planet`.
///
/// The vapour stays near saturation as the air cools with height, so
/// that by Clausius–Clapeyron it thins with the scale height
/// `H_v = R_v T² / (L Γ)` (`Γ` the lapse rate, [`EARTH_LAPSE_K_M`]
/// scaled by gravity), `T` its dew point; its column is the surface
/// density `p / (R_v T)` over that height, `p T / (L Γ)`. Earth's
/// 1.3e-2 bar at a 284 K dew point make 24 kg/m², against a measured
/// mean of about 25 kg/m² of precipitable water (Trenberth, Fasullo &
/// Smith 2005, doi:10.1007/s00382-005-0017-4, from memory, not checked).
/// Never more than the vapour would weigh alone, `p / g` (a steam air).
fn vapour_column_kg_m2(h2o_bar: f64, planet: &Planet) -> f64 {
    let p = h2o_bar.max(0.0) * 1e5;
    if p <= 0.0 {
        return 0.0;
    }
    let dew_k = water_boils_k(h2o_bar);
    let lapse = EARTH_LAPSE_K_M * planet.gravity_m_s2 / EARTH_GRAVITY_M_S2;
    (p * dew_k / (WATER_LATENT_J_KG * lapse)).min(p / planet.gravity_m_s2)
}

/// Share of the sunlight a column of `y` cm of precipitable water,
/// counted along the light's slanted path, absorbs: the near-infrared
/// bands of water vapour, `2.9 y / ((1 + 141.5 y)^0.635 + 5.925 y)`
/// (Lacis & Hansen 1974, doi:10.1175/1520-0469(1974)031<0118:APFTAO>2.0.CO;2,
/// their eq. 20, from memory, not checked). It saturates, as the bands
/// do, at `2.9 / 5.925`, 49 % of the light.
fn water_absorptivity(y_cm: f64) -> f64 {
    let y = y_cm.max(0.0);
    2.9 * y / ((1.0 + 141.5 * y).powf(0.635) + 5.925 * y)
}

/// Lengthening of the light's path through the vapour on its way down,
/// averaged over the disc: Lacis & Hansen's magnification
/// `35 / √(1224 μ₀² + 1)` at the disc's mean `μ₀ = 2/3` (the cosine of
/// the Sun's zenith angle weighted by the light it brings), 1.5.
const PATH_DOWN: f64 = 1.5;

/// Lengthening of the path of the light reflected back up, diffuse:
/// 1.9 (Lacis & Hansen 1974, their magnification for reflected light,
/// from memory, not checked).
const PATH_UP: f64 = 1.9;

/// Share of the light that crosses an air's water vapour down to the
/// clouds and the ground and back up to space: `1 − A(y)` of
/// [`water_absorptivity`] along both paths ([`PATH_DOWN`], [`PATH_UP`])
/// through the vapour's column ([`vapour_column_kg_m2`], 1 kg/m² being
/// 0.1 cm of precipitable water).
///
/// Earth's air absorbs about 23 % of the sunlight (78 of 341 W/m²;
/// Trenberth, Fasullo & Kiehl 2009, doi:10.1175/2008BAMS2634.1), most of
/// it in its water vapour: this takes some 14 % on the way down, the
/// rest being its ozone, aerosols and cloud droplets, which the model
/// does not carry. CO₂'s near-infrared bands take about 1 % on Earth and
/// are left out; methane's are Titan's haze's absorption
/// ([`HAZE_ABSORPTION`], calibrated with them). 1 with no vapour.
fn vapour_transmission(gases: &Gases, planet: &Planet) -> f64 {
    let y = 0.1 * vapour_column_kg_m2(gases.h2o, planet);
    1.0 - water_absorptivity(y * (PATH_DOWN + PATH_UP))
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

/// Share of a body's disc, 0 to 1, that `frost` (bar, gas by gas, all
/// together) covers: `frost / FROST_LAYER_BAR`, all of it from that
/// amount on ([`FROST_LAYER_BAR`]).
pub(crate) fn frost_cover(frost: &Gases) -> f64 {
    (frost.total().max(0.0) / FROST_LAYER_BAR).min(1.0)
}

/// What covers a body at `temperature_k` under `gases` with `water`
/// and `frost` (bar, gas by gas) of N₂, CH₄, CO₂ and SO₂ frost on its
/// ground ([`Cover`]).
///
/// - Its water, out of a runaway, is open ocean ([`ocean_fraction`] of
///   its oceans) where the surface is warmer than the ice line and ice or
///   snow where it is colder ([`ice_line_share`]), the ice limited by
///   sheets the water can make ([`ICE_SHEET_KM`]) out of the layer it
///   would spread into over `planet`'s area ([`OCEAN_LAYER_KM`]).
/// - The rest is bare land.
/// - Its frost lies over the ocean's ice and the land, covering the share
///   `frost_bar / FROST_LAYER_BAR` of the disc ([`FROST_LAYER_BAR`]), all
///   of it from that amount on, `frost_bar` all of its gases together;
///   the SO₂ frost takes its part of that share.
/// - Its water clouds follow the vapour in the air ([`CLOUD_H2O_BAR`])
///   and its relative humidity at `temperature_k`
///   ([`CLOUD_HUMIDITY_EXPONENT`]): none in a dry or airless sky.
pub fn cover(
    temperature_k: f64,
    gases: &Gases,
    water: &Water,
    frost: &Gases,
    planet: &Planet,
) -> Cover {
    let frost_bar = frost.total();
    let wet = water.is_wet();
    let (sea, ice) = if wet {
        let layer_km = OCEAN_LAYER_KM / planet.radius_earth.powi(2);
        let reach = (water.oceans.max(0.0) * layer_km / ICE_SHEET_KM).min(1.0);
        (
            ocean_fraction(water.oceans),
            ice_line_share(temperature_k) * reach,
        )
    } else {
        (0.0, 0.0)
    };
    let so2_part = if frost_bar > 0.0 {
        (frost.so2 / frost_bar).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let frost = frost_cover(frost);
    let ocean = sea * (1.0 - ice) * (1.0 - frost);
    let ice = ice * (1.0 - frost);
    let land = (1.0 - ocean - ice - frost).max(0.0);
    let cloud = cloud_cover(temperature_k, gases.h2o);
    Cover {
        ocean,
        land,
        ice,
        frost,
        so2_frost: frost * so2_part,
        cloud,
    }
}

/// Share of the disc, 0 to 1, under water clouds in an air at
/// `temperature_k` that holds `h2o_bar` of vapour: the relative humidity
/// to the power [`CLOUD_HUMIDITY_EXPONENT`] times the vapour's
/// `1 − exp(−p_H₂O / CLOUD_H2O_BAR)`. The humidity is the vapour over
/// its saturation at the surface's temperature, at most 1 (a runaway's
/// steam, past water's critical point, is saturated).
pub(crate) fn cloud_cover(temperature_k: f64, h2o_bar: f64) -> f64 {
    let h2o = h2o_bar.max(0.0);
    if h2o <= 0.0 {
        return 0.0;
    }
    let humidity = (h2o / water_saturation_bar(temperature_k)).min(1.0);
    humidity.powf(CLOUD_HUMIDITY_EXPONENT) * (1.0 - (-h2o / CLOUD_H2O_BAR).exp())
}

/// Share of the disc, 0 to 1, that a sulphuric acid deck covers on a
/// body at `temperature_k` whose air holds `so2_bar` of SO₂ after the
/// washing out ([`super::atmosphere::wash_out`]; the caller washes, once:
/// the climate before the albedo, the model before it stores the air).
/// None under [`DECK_SO2_BAR`] or [`DECK_MIN_K`]; past them it grows
/// linearly over [`DECK_ONSET_K`].
pub(crate) fn sulphuric_deck(temperature_k: f64, so2_bar: f64) -> f64 {
    if so2_bar < DECK_SO2_BAR {
        return 0.0;
    }
    ((temperature_k - DECK_MIN_K) / DECK_ONSET_K).clamp(0.0, 1.0)
}

/// Strength of a methane haze in `gases`, 0 to 1 ([`HAZE_CH4_BAR`],
/// [`HAZE_CH4_PER_CO2`]).
pub(crate) fn haze(gases: &Gases) -> f64 {
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
/// sulphuric acid deck over the share `deck` of its disc
/// ([`sulphuric_deck`]).
///
/// The surface mixes its covers' albedos (ocean [`OCEAN_ALBEDO`], land
/// from [`REGOLITH_ALBEDO`] when airless to [`SOIL_ALBEDO`] under an air,
/// ice and snow [`ICE_ALBEDO`], frost [`FROST_ALBEDO`], SO₂ frost
/// [`SO2_FROST_ALBEDO`]); the water clouds' layer
/// ([`CLOUD_ALBEDO`] over their cover) lies over it, the water vapour
/// over both absorbs on the way down and back up
/// ([`vapour_transmission`]), the Rayleigh
/// scattering of the column over both (averaged over the Sun's spectrum
/// band by band, [`spectrum`]: with CO₂ condensing in the column, what
/// caps CO₂'s warming at the maximum greenhouse, Kopparapu et al. 2013,
/// doi:10.1088/0004-637X/765/2/131), and a methane
/// haze over all ([`HAZE_ALBEDO`], [`HAZE_ABSORPTION`]). A deck hides
/// everything below it ([`DECK_ALBEDO`]) where it lies. The air's
/// column is Earth's gravity's and the light the Sun's
/// ([`bond_albedo_over`] takes the body's and its star's).
pub fn bond_albedo(cover: &Cover, gases: &Gases, deck: f64) -> f64 {
    bond_albedo_over(cover, gases, deck, None, &Planet::EARTH, SUN_TEFF_K)
}

/// [`bond_albedo`] of a body whose ice and land are, where `crust` is
/// given, an exposed ice crust of that albedo (the area-weighted
/// [`icy_unit_albedo`] of its terrain units) instead of snow and rock,
/// on `planet`, whose gravity sets the column of its air
/// ([`rayleigh_550`], [`vapour_transmission`]), under the light of a
/// star of effective temperature `star_k` (K), whose spectrum the
/// Rayleigh scattering is averaged over ([`spectrum`]).
pub fn bond_albedo_over(
    cover: &Cover,
    gases: &Gases,
    deck: f64,
    crust: Option<f64>,
    planet: &Planet,
    star_k: f64,
) -> f64 {
    let deck = deck.clamp(0.0, 1.0);
    if deck >= 1.0 {
        return DECK_ALBEDO;
    }
    let shield = 1.0 - (-gases.total().max(0.0) / SHIELD_BAR).exp();
    let land = REGOLITH_ALBEDO + (SOIL_ALBEDO - REGOLITH_ALBEDO) * shield;
    let (land, ice) = match crust {
        Some(a) => (a, a),
        None => (land, ICE_ALBEDO),
    };
    let surface = cover.ocean * OCEAN_ALBEDO
        + cover.land * land
        + cover.ice * ice
        + (cover.frost - cover.so2_frost) * FROST_ALBEDO
        + cover.so2_frost * SO2_FROST_ALBEDO;
    let cloud = (cover.cloud * CLOUD_ALBEDO).clamp(0.0, 1.0);
    let clouded = over(cloud, 1.0 - cloud, surface) * vapour_transmission(gases, planet);
    let tau = rayleigh_550(gases, planet);
    let h = haze(gases);
    let (hr, ht) = (h * HAZE_ALBEDO, 1.0 - h * (HAZE_ALBEDO + HAZE_ABSORPTION));
    let below: f64 = spectrum(star_k)
        .iter()
        .map(|&(scale, weight)| {
            let r = rayleigh_reflectance(tau * scale);
            weight * over(hr, ht, over(r, 1.0 - r, clouded))
        })
        .sum();
    deck * DECK_ALBEDO + (1.0 - deck) * below
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
            let c = cover(t, &airless, &dry, &Gases::default(), &Planet::EARTH);
            assert_eq!(c.ocean, 0.0, "{t}: {c:?}");
            assert_eq!(c.cloud, 0.0, "{t}: {c:?}");
            assert_eq!(c.ice, 0.0, "{t}: {c:?}");
            assert_eq!(c.land, 1.0, "{t}: {c:?}");
            let a = bond_albedo(&c, &airless, 0.0);
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
            let ice = cover(t, &air, &water, &Gases::default(), &Planet::EARTH).ice;
            assert!(ice <= last, "{t}: {ice} > {last}");
            last = ice;
        }
        let earth = cover(288.0, &air, &water, &Gases::default(), &Planet::EARTH).ice;
        assert!((0.06..=0.14).contains(&earth), "{earth}");
        for t in [200.0, 230.0, 240.0, 250.0] {
            let ice = cover(t, &air, &water, &Gases::default(), &Planet::EARTH).ice;
            assert!(ice > 0.999, "{t}: {ice}");
        }
        assert_eq!(
            cover(300.0, &air, &water, &Gases::default(), &Planet::EARTH).ice,
            0.0
        );
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
        let caps = cover(210.0, &air, &mars, &Gases::default(), &Planet::EARTH);
        assert!(caps.ice > 0.0 && caps.ice < 0.02, "{caps:?}");
        let a = bond_albedo(&caps, &air, 0.0);
        assert!((0.2..=0.3).contains(&a), "{a}");
        assert_eq!(
            cover(
                210.0,
                &air,
                &earth_water(),
                &Gases::default(),
                &Planet::EARTH
            )
            .ice,
            1.0
        );
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
        let n2 = |bar: f64| Gases {
            n2: bar,
            ..Gases::default()
        };
        let a = |frost: f64| {
            bond_albedo(
                &cover(40.0, &thin, &dry, &n2(frost), &Planet::EARTH),
                &thin,
                0.0,
            )
        };
        let half = cover(
            40.0,
            &thin,
            &dry,
            &n2(FROST_LAYER_BAR / 2.0),
            &Planet::EARTH,
        );
        assert!((half.frost - 0.5).abs() < 1e-12, "{half:?}");
        assert!((half.land - 0.5).abs() < 1e-12, "{half:?}");
        assert!(a(0.0) < a(FROST_LAYER_BAR / 2.0));
        assert!(a(FROST_LAYER_BAR / 2.0) < a(FROST_LAYER_BAR));
        assert_eq!(a(FROST_LAYER_BAR), a(10.0));
        assert!((a(10.0) - FROST_ALBEDO).abs() < 0.01, "{}", a(10.0));
        // SO₂ frost, the volcanic frost of a heat pipe, is darker.
        let so2 = Gases {
            so2: 10.0,
            ..Gases::default()
        };
        let volcanic = cover(40.0, &thin, &dry, &so2, &Planet::EARTH);
        assert_eq!(volcanic.so2_frost, 1.0);
        let a_so2 = bond_albedo(&volcanic, &thin, 0.0);
        assert!((a_so2 - SO2_FROST_ALBEDO).abs() < 0.01, "{a_so2}");
        // Over ice, the frost takes its share from the ice.
        let icy = cover(
            40.0,
            &thin,
            &earth_water(),
            &n2(FROST_LAYER_BAR / 4.0),
            &Planet::EARTH,
        );
        assert!(
            (icy.ice - 0.75).abs() < 1e-12 && icy.ocean == 0.0,
            "{icy:?}"
        );
    }

    /// The icy-age law darkens ice with its age and its body's rock,
    /// within fresh ice and the lag, and a plume's fallout brightens it.
    #[test]
    fn old_and_dusty_ice_is_darker() {
        let a = icy_unit_albedo;
        let mut last = f64::INFINITY;
        for i in 0..=50 {
            let age = 0.1 * f64::from(i);
            let now = a(age, 0.5, 0.0);
            assert!(now <= last, "{age}: {now} > {last}");
            assert!((REGOLITH_ALBEDO..=PURE_ICE_ALBEDO).contains(&now), "{now}");
            last = now;
        }
        assert!(a(0.1, 0.9, 0.0) < a(0.1, 0.6, 0.0));
        // The fallout brightens in proportion to the vent's strength.
        assert!(a(0.4, 0.65, 0.0) < a(0.4, 0.65, 0.5));
        assert!(a(0.4, 0.65, 0.5) < a(0.4, 0.65, 1.0));
        assert!(a(4.5, 0.5, 1.0) < PURE_ICE_ALBEDO);
        assert!((a(0.0, 0.0, 0.0) - PURE_ICE_ALBEDO).abs() < 1e-12);
        // All rock: the dust's own albedo, never NaN.
        assert!((a(0.0, 1.0, 0.0) - REGOLITH_ALBEDO).abs() < 1e-9);
        // Callisto-like: old, half rock.
        let callisto = a(4.55, 0.5, 0.0);
        assert!((0.15..=0.3).contains(&callisto), "{callisto}");
        // An old crust of 94 % ice (Tethys-like) has little rock for a
        // lag and stays bright; an old half-rock one (Callisto) does not.
        assert!(a(4.5, 0.06, 0.0) > 0.7, "{}", a(4.5, 0.06, 0.0));
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
            so2_frost: 0.0,
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
                0.0,
            )
        };
        let (thin, eight, ninety) = (a(1e-3), a(8.0), a(90.0));
        println!("albedo under 0, 8, 90 bar of CO2: {thin:.3} {eight:.3} {ninety:.3}");
        assert!(thin < eight && eight < ninety, "{thin} {eight} {ninety}");
        assert!(ninety < 1.0);
    }

    fn bare_land() -> Cover {
        Cover {
            ocean: 0.0,
            land: 1.0,
            ice: 0.0,
            frost: 0.0,
            so2_frost: 0.0,
            cloud: 0.0,
        }
    }

    fn planet(gravity_m_s2: f64, radius_earth: f64) -> Planet {
        Planet {
            gravity_m_s2,
            radius_earth,
        }
    }

    /// A molecule of CO₂ scatters about 2.5 times as much light as one of
    /// air (Kopparapu et al. 2013, §2.2, read on arXiv:1301.6674v2), not
    /// the 2.80 the squared static polarizabilities give (issue #523).
    #[test]
    fn co2_scatters_two_and_a_half_times_nitrogen_per_molecule() {
        let earth = planet(EARTH_GRAVITY_M_S2, 1.0);
        let one = |gases: Gases| rayleigh_550(&gases, &earth);
        let n2 = one(Gases {
            n2: 1.0,
            ..Gases::default()
        });
        let co2 = one(Gases {
            co2: 1.0,
            ..Gases::default()
        });
        // A bar of CO₂ holds 28/44 as many molecules as a bar of N₂.
        let per_molecule = co2 / n2 * 44.0 / 28.0;
        assert!((per_molecule - 2.5).abs() < 1e-9, "{per_molecule}");
    }

    /// Issue #523, Review Focus 2: the same surface pressure weighs a
    /// column `p / g` deep, so a low-gravity moon (Titan, g = 1.35)
    /// scatters as much more than a body with Earth's gravity as its
    /// column is heavier, and a super-Earth less.
    #[test]
    fn rayleigh_counts_the_column_mass() {
        let air = Gases {
            n2: 1.5,
            ..Gases::default()
        };
        let (titan, earth) = (planet(1.35, 0.404), planet(EARTH_GRAVITY_M_S2, 1.0));
        let ratio = rayleigh_550(&air, &titan) / rayleigh_550(&air, &earth);
        let expected = EARTH_GRAVITY_M_S2 / 1.35;
        assert!(
            (ratio / expected - 1.0).abs() < 0.05,
            "{ratio} vs {expected}"
        );
        let heavy = rayleigh_550(&air, &planet(2.0 * EARTH_GRAVITY_M_S2, 1.5));
        assert!((heavy / rayleigh_550(&air, &earth) - 0.5).abs() < 1e-12);
        // So the low-gravity body is brighter over the same ground.
        let a = |p: &Planet| bond_albedo_over(&bare_land(), &air, 0.0, None, p, SUN_TEFF_K);
        assert!(a(&titan) > a(&earth) + 0.05, "{} {}", a(&titan), a(&earth));
    }

    /// Review Focus 3: with no air, neither the scattering nor the
    /// absorption of the air touches the surface's albedo, whatever the
    /// gravity.
    #[test]
    fn an_airless_body_keeps_its_surface_albedo() {
        let none = Gases::default();
        for g in [3.7, EARTH_GRAVITY_M_S2] {
            let p = planet(g, 0.5);
            let a = bond_albedo_over(&bare_land(), &none, 0.0, None, &p, SUN_TEFF_K);
            assert!((a - REGOLITH_ALBEDO).abs() < 1e-12, "{g}: {a}");
            assert_eq!(vapour_transmission(&none, &p), 1.0);
            let crust = bond_albedo_over(&bare_land(), &none, 0.0, Some(0.6), &p, SUN_TEFF_K);
            assert!((crust - 0.6).abs() < 1e-12, "{g}: {crust}");
        }
    }

    /// The same water spread over a planet twice Earth's radius makes a
    /// layer a quarter as deep, which reaches a quarter as far as ice.
    #[test]
    fn ice_reach_scales_with_area() {
        let air = Gases {
            n2: 1.0,
            ..Gases::default()
        };
        let little = Water {
            oceans: 0.05,
            kept_kg: 0.05 * super::super::OCEAN_KG,
            ..earth_water()
        };
        let ice = |radius: f64| {
            cover(
                200.0,
                &air,
                &little,
                &Gases::default(),
                &planet(EARTH_GRAVITY_M_S2, radius),
            )
            .ice
        };
        let (one, two) = (ice(1.0), ice(2.0));
        assert!(one < 1.0, "{one}");
        assert!((two / one - 0.25).abs() < 1e-12, "{two} / {one}");
    }

    /// Earth's air absorbs about a fifth of the sunlight, most of it in
    /// its water vapour: the vapour of 288 K under Earth's gravity takes
    /// some 15 % of the light on its way down and back; none without
    /// vapour.
    #[test]
    fn the_air_absorbs_sunlight_with_its_vapour() {
        let dry = Gases {
            n2: 0.78,
            o2: 0.21,
            co2: 4e-4,
            ..Gases::default()
        };
        let moist = Gases {
            h2o: 0.77 * super::super::climate::water_saturation_bar(288.0),
            ..dry
        };
        assert_eq!(vapour_transmission(&dry, &Planet::EARTH), 1.0);
        let t = vapour_transmission(&moist, &Planet::EARTH);
        println!("Earth's two-way transmission of its vapour: {t:.3}");
        assert!((0.75..=0.9).contains(&t), "{t}");
        let c = bare_land();
        let a_dry = bond_albedo_over(&c, &dry, 0.0, None, &Planet::EARTH, SUN_TEFF_K);
        let a_moist = bond_albedo_over(&c, &moist, 0.0, None, &Planet::EARTH, SUN_TEFF_K);
        assert!(a_moist < a_dry, "{a_moist} {a_dry}");
    }

    /// The cloud albedo is re-derived with the air's absorption, so that
    /// Earth's Bond albedo stays at its measured 0.30 (CERES).
    #[test]
    fn earth_keeps_its_bond_albedo() {
        let air = Gases {
            n2: 0.78,
            o2: 0.21,
            co2: 4e-4,
            h2o: 0.77 * super::super::climate::water_saturation_bar(288.0),
            ..Gases::default()
        };
        let c = cover(
            288.0,
            &air,
            &earth_water(),
            &Gases::default(),
            &Planet::EARTH,
        );
        let a = bond_albedo_over(&c, &air, 0.0, None, &Planet::EARTH, SUN_TEFF_K);
        println!("Earth's Bond albedo at 288 K: {a:.4}");
        assert!((0.29..=0.31).contains(&a), "{a}");
    }

    #[test]
    fn cloud_cover_depends_on_more_than_vapour() {
        // The same vapour in a warmer air is further from saturation, and
        // a drier air makes fewer clouds (issue #523): Earth's vapour at
        // 288 K and at 320 K, and a trace of water that cannot fill a warm
        // air (a desert world whose air holds all its water).
        let air = |h2o: f64| Gases {
            n2: 0.78,
            o2: 0.21,
            co2: 4e-4,
            h2o,
            ..Gases::default()
        };
        let cloud = |t: f64, h2o: f64| {
            cover(
                t,
                &air(h2o),
                &earth_water(),
                &Gases::default(),
                &Planet::EARTH,
            )
            .cloud
        };
        let vapour = 0.77 * super::super::climate::water_saturation_bar(288.0);
        let (cool, warm) = (cloud(288.0, vapour), cloud(320.0, vapour));
        println!("cloud cover at 288 K {cool:.4}, at 320 K {warm:.4}");
        assert!(warm < cool - 0.01, "{warm} vs {cool}");
        // Earth's own air keeps its cover of about two-thirds.
        assert!((cool - EARTH_CLOUD_COVER).abs() < 0.01, "{cool}");
        // A saturated air makes no more cloud than its vapour allows, and
        // more than the same vapour unsaturated.
        let sat = super::super::climate::water_saturation_bar(288.0);
        assert!(cloud(288.0, sat) > cool, "{} vs {cool}", cloud(288.0, sat));
        assert!(cloud(288.0, sat) <= 1.0);
        // No vapour, no cloud, at any temperature.
        assert_eq!(cloud(300.0, 0.0), 0.0);
    }
}
