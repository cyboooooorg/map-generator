//! The surface cover of a body and the albedo of its ground.
//!
//! Ported from `legacy-final` (`planet/albedo.rs`, `cover`,
//! `ocean_fraction`, `ice_line_share`, `frost_cover`, and the ground of
//! `shortwave_budget`). The cloud cover stays out of [`Cover`]: the
//! caller builds [`crate::radiation::albedo::Underneath`] from
//! [`ground_albedo`], `radiation::clouds::cloud_cover` and
//! `radiation::veils::sulphuric_deck`.

use crate::constants::STANDARD_ATMOSPHERE;
use crate::habitable::OCEAN_MASS;
use crate::radiation::gases::Weights;
use crate::units::{Bar, Fraction, Kelvin, Kilogram, Metre, Radian};
use crate::water::{ice_ih_density, ice_ih_melting_temperature, vapour_saturation_bar};

/// Source: the surface relative humidity over the oceans, 77 %: Dai 2006
/// (*J. Climate* 19, 3589, doi:10.1175/JCLI3816.1, abstract), a mean of
/// 75 % to 80 % over most oceans and 70 % to 80 % over most land, from
/// over 15 000 stations and ships (1976 to 2005); Manabe & Wetherald
/// 1967 (*J. Atmos. Sci.* 24, 241, eq. 2, read in the scan GFDL hosts,
/// Wiki study 33) took 0.77, fitted to observed hemispheric means of
/// relative humidity (their Fig. 3), as konrad (`Manabe67`) and climlab
/// (`FixedRelativeHumidity`) take it. Study 24, section 8.
pub const RELATIVE_HUMIDITY: f64 = 0.77;

/// Source: the albedo of the ocean under isotropic light, 0.06: Payne
/// 1972 (*J. Atmos. Sci.* 29, 959), "0.061 +- 0.005 for heavily overcast
/// skies ... indicating isotropic radiance distribution", the albedo
/// averaged over every angle. Study 24, section 4.
pub const OCEAN_ALBEDO: f64 = 0.06;

/// Calibration: the albedo of bare land under an air, 0.25, fitted on
/// Mars's Bond albedo, 0.250 (NASA Mars Fact Sheet). Searched and not
/// found: a broadband value for bare land (Henderson-Sellers & Wilson
/// 1983 give none in their abstract). Study 24, section 4.
pub const SOIL_ALBEDO: f64 = 0.25;

/// Source: the Bond albedo of the Moon, 0.11 (NASA Moon Fact Sheet), the
/// albedo of an airless regolith; Mercury's, 0.068, is darker (a
/// departure). Study 24, section 4.
pub const REGOLITH_ALBEDO: f64 = 0.11;

/// Calibration: the albedo of water ice and snow, 0.6, the middle of
/// NSIDC's "50 to 70 percent of the incoming energy" for sea ice, under
/// snow (Perovich et al. 2002: dry snow 0.8 to 0.9, bare white ice 0.65,
/// 0.4 with melt ponds). Searched and not found: a broadband value in
/// Warren 1982. Study 24, section 4.
pub const ICE_ALBEDO: f64 = 0.6;

/// Calibration: the albedo of the N2, CH4 and CO2 frosts, 0.75, fitted on
/// Triton's N2 ice at 38 +- 1 K (Tryka et al. 1993, *Science* 261, 751,
/// and 1994, *Icarus* 112, 513), given [`N2_ICE_EMISSIVITY`]: at the
/// orbit-mean flux of 1.508 W/m2 (NASA Neptune Fact Sheet) the balance
/// `eps sigma T^4 = (1 - A) F / 4` gives `A` = 0.749 and 37.97 K for
/// 0.75. Hillier et al. 1990 (*Science* 250, 419) give 0.82 +- 0.05 and
/// Hillier et al. 1991 (*J. Geophys. Res.* 96) a bolometric 0.85 +- 0.05
/// with an emissivity of 0.46 +- 0.16 at 37.7 K: the pair misses them by
/// 1.4 to 2 sigma. Pluto's 0.72 +- 0.07 (Buratti et al. 2017, *Icarus*
/// 287, doi:10.1016/j.icarus.2016.11.012) holds the albedo, Triton the
/// temperature. Searched and not found: a bolometric albedo and an
/// emissivity of Triton measured apart from this balance (issue #673;
/// `docs/science/planet-atmospheres.md`, "Calibration of `FROST_ALBEDO`
/// and the N2 ice emissivity").
pub const FROST_ALBEDO: f64 = 0.75;

/// Source: the emissivity of N2 ice, 0.80: Lellouch et al. 2011 (*Icarus*
/// 214, 701) for large grains, as Meza et al. 2019 (arXiv:1903.02315,
/// section 4) quote it and adopt it for Pluto, whose occultations
/// constrain only `(1 - A) / eps` (Lellouch et al. itself not read; the
/// rewrite takes the value for Triton too). Stansberry, Pisano & Yelle 1996 (*Planet. Space
/// Sci.* 44, 945) give pure beta-N2 0.85 at 1 cm grains and 0.40 at 1 mm.
/// One value for alpha- and beta-N2. Issue #673.
pub const N2_ICE_EMISSIVITY: f64 = 0.80;

/// Calibration: the albedo of the SO2 frost, 0.63, the legacy's fit on
/// Io's Bond albedo (Simonelli & Veverka 1988, from memory, not checked),
/// the albedo of a disc nearly all frost. Giles et al. 2024 fit Io's
/// frost with a Bond albedo of 0.565 (their Fig. 3, study 24, open
/// questions): issue #669 follows it up.
pub const SO2_FROST_ALBEDO: f64 = 0.63;

/// Calibration: the pressure above which an air shields a regolith from
/// the solar wind and the micrometeoroids, 1e-4 bar: any value between
/// the exospheres of Mercury and the Moon (under 1e-12 bar) and Mars's
/// 6 mbar gives both their albedos. Searched and not found: a threshold
/// in the space-weathering studies, which treat airless bodies only.
/// Study 24, section 5.
pub const SHIELD_PRESSURE: Bar = Bar(1e-4);

/// Calibration: the frost that covers a whole disc, 0.02 bar of gas: a
/// column of 2 000 Pa, about 200 kg/m2 under Earth's gravity (0.2 m of
/// ice), 0.7 m of CO2 frost on Io's or 2.5 m of N2 frost on Triton's.
/// The legacy's value, set on the CO2 frost its Io then froze out of its
/// air (0.013 bar, two thirds of its disc). Triton's 0.36 bar covers the
/// whole disc with any value under that, and Europa's 2e-4 bar is left
/// to its water ice, so neither constrains it. Not studied: the legacy's
/// calibration is kept, with no source for a frost's coverage as a
/// function of its amount (`docs/science/planet-atmospheres.md`,
/// "Calibration of `FROST_LAYER`").
pub const FROST_LAYER: Bar = Bar(0.02);

/// Calibration: the temperature under which the ice reaches the equator's
/// edge of the ice line, -5 C (268.15 K), fitted on Earth's ice and
/// seasonal sea ice, a tenth of its disc (Antarctica 2.7 % of the surface,
/// Bedmap2, Fretwell et al. 2013; sea ice 6.7 %, NSIDC), against
/// -10 C for the permanent cap (North et al. 1981, from Budyko's rule). Study 24, section 6.
pub const ICE_LINE_TEMPERATURE: Kelvin = Kelvin(268.15);

/// Source: the pole-to-equator contrast of the temperature, 42 K: North,
/// Cahalan & Coakley 1981 (*Rev. Geophys.* 19, 91), Earth's annual mean
/// `T(x) = T0 + T2 P2(x)` with the observed `T2 = -28 K`, so that
/// `T = T0 - 42 K (x^2 - 1/3)` (derived, `-3/2 T2`). One contrast for
/// every planet. Study 24, section 6.
pub const ICE_LINE_CONTRAST: Kelvin = Kelvin(42.0);

/// Definition: the mean temperature under which the ice covers the whole
/// disc, `ICE_LINE_TEMPERATURE - ICE_LINE_CONTRAST / 3`, about 254 K: the
/// pole-to-equator profile of [`ice_line_share`] reaches the equator at
/// that mean. The start of the frozen branch of 2d-3.
pub const ICE_COVERED_TEMPERATURE: Kelvin =
    Kelvin(ICE_LINE_TEMPERATURE.0 - ICE_LINE_CONTRAST.0 / 3.0);

/// Definition: the mean surface temperature over which a body keeps no
/// ice, `ICE_LINE_TEMPERATURE + 2/3 ICE_LINE_CONTRAST`, about 296 K: its
/// poles reach the ice line at that mean ([`ice_line_share`] is 0 from
/// there). Legacy's `ICE_FREE_K`.
pub const ICE_FREE_TEMPERATURE: Kelvin =
    Kelvin(ICE_LINE_TEMPERATURE.0 + 2.0 / 3.0 * ICE_LINE_CONTRAST.0);

/// Source: the mean thickness of Antarctica's ice with its shelves, 1937 m
/// (Bedmap2, Fretwell et al. 2013, *The Cryosphere* 7, 375,
/// doi:10.5194/tc-7-375-2013, Table 7), rounded to 1.94 km. Study 24,
/// section 7.
pub const ICE_SHEET_THICKNESS: Metre = Metre(1940.0);

/// Source: the share of Earth's surface under ocean, 0.71: 361.84e6 km2
/// of ocean (Charette & Smith 2010, *Oceanography* 23, 112, Table 1) over
/// Earth's 510.06e6 km2, 0.709. Study 24, section 7.
pub const EARTH_OCEAN_COVER: f64 = 0.71;

/// What covers a body's surface, as shares of its disc. The shares sum
/// to 1. The cloud cover is not here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cover {
    /// Open ocean.
    pub ocean: Fraction,
    /// Bare land.
    pub land: Fraction,
    /// Water ice and snow, by area.
    pub ice: Fraction,
    /// Water ice and snow weighted by the annual-mean sunlight they
    /// receive ([`sunlit_cap`]): the share of the light that meets ice.
    /// The ground albedo counts the ice by this share, the map by `ice`.
    pub ice_sunlit: Fraction,
    /// Frost of N2, CH4, CO2 and SO2 over the rest.
    pub frost: Fraction,
    /// The part of `frost` that is SO2.
    pub so2_frost: Fraction,
}

/// `x` as a share, held within 0 to 1; a NaN reads as none.
pub(super) fn share(x: f64) -> Fraction {
    Fraction::new(if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) })
        .expect("a clamped finite value is a fraction")
}

/// Water vapour pressure of the surface air at `t`: [`RELATIVE_HUMIDITY`]
/// of the saturation, at most the `available` water.
pub fn surface_vapour(t: Kelvin, available: Bar) -> Bar {
    Bar((RELATIVE_HUMIDITY * vapour_saturation_bar(t.0)).min(available.0.max(0.0)))
}

/// Share of a disc under open ocean for `oceans` Earth oceans of liquid
/// water: `1 - exp(-k oceans)`, with `k` putting Earth's one ocean at
/// [`EARTH_OCEAN_COVER`]. A fit: a body's sea level follows its relief,
/// which this law does not have; it rises fast while the water fills the
/// basins and saturates as it drowns the continents.
pub fn ocean_fraction(oceans: f64) -> Fraction {
    let k = -(1.0 - EARTH_OCEAN_COVER).ln();
    share(1.0 - (-k * oceans.max(0.0)).exp())
}

/// Share of the disc colder than [`ICE_LINE_TEMPERATURE`] on a body whose
/// mean surface temperature is `t`, with the temperature falling to the
/// poles as `T_mean - dT (x^2 - 1/3)` ([`ICE_LINE_CONTRAST`]), `x` the
/// sine of the latitude: an energy-balance ice line in the manner of
/// Budyko and Sellers. None above about 296 K, all of it below
/// [`ICE_COVERED_TEMPERATURE`].
pub fn ice_line_share(t: Kelvin) -> Fraction {
    let x2 = 1.0 / 3.0 + (t.0 - ICE_LINE_TEMPERATURE.0) / ICE_LINE_CONTRAST.0;
    share(1.0 - x2.clamp(0.0, 1.0).sqrt())
}

/// Share of a disc that `frost` (bar, weights, gas by gas, all together)
/// covers: `frost / FROST_LAYER`, all of it from that amount on.
pub fn frost_cover(frost: &Weights) -> Fraction {
    share(frost.total() / FROST_LAYER.0)
}

/// Emissivity of a surface with `frost` (bar, weights, gas by gas) on its
/// ground:
/// [`N2_ICE_EMISSIVITY`] over the share of its disc that its nitrogen
/// frost covers ([`frost_cover`] of N2 alone), 1 elsewhere. Rock, water
/// ice and the other frosts radiate close to a black body; nitrogen, the
/// most volatile frost, condenses last and lies over the others. The
/// result is `1 - (1 - N2_ICE_EMISSIVITY) * share`.
pub fn frost_emissivity(frost: &Weights) -> Fraction {
    let nitrogen = Weights {
        n2: frost.n2.max(0.0),
        ..Weights::default()
    };
    share(1.0 - (1.0 - N2_ICE_EMISSIVITY) * frost_cover(&nitrogen).get())
}

/// Share of the light that falls on the ice of a tidally locked body
/// whose ice covers the area share `area`: `max(0, 2a - 1)^2`. A locked
/// rotator receives `4 mu` of the mean light on its day side (`mu` the
/// cosine of the angle from the substellar point, Lambert's law) and
/// none at night; its ice forms from the antistellar point, so the ice
/// of share `a` covers the night side and `2a - 1` of the day side's
/// area, at a light `(2a - 1)^2` (`Definition:`, derived; see
/// `docs/science/planet-atmospheres.md`, "Tidally locked rotators").
pub fn locked_sunlit_ice(area: Fraction) -> Fraction {
    let beyond = (2.0 * area.get() - 1.0).max(0.0);
    share(beyond * beyond)
}

/// Share of the annual-mean light that falls on the polar caps beyond
/// `x = edge` (`x` the sine of the latitude, the caps' area share being
/// `1 - edge`), on a rapid rotator of `obliquity`: the integral of the
/// insolation `S(x) = 1 + s2 P2(x)`, mean 1 over the disc, that is
/// `(1 - e) + (s2 / 2)(e - e^3)` (`Definition:`, the integral of `P2`),
/// held within 0 to 1.
///
/// Source: `s2 = -(5/8) P2(cos obliquity) = -(5/16)(3 cos^2 obliquity - 1)`,
/// the second term of Nadeau & McGehee 2021 (*J. Math. Anal. Appl.*,
/// 125129, doi:10.1016/j.jmaa.2021.125129, Theorem 1.1 and eq. 39,
/// `A2 = -5/8`). The truncation at `P2` overstates Earth's cap by 4.4 %
/// (0.0026 of the disc; derived by integrating their eq. 1); the shape
/// holds for a spin period much shorter than the year: a tidally locked
/// rotator uses [`locked_sunlit_ice`] instead. See
/// `docs/science/planet-atmospheres.md`, "Ice weighted by sunlight".
pub fn sunlit_cap(edge: f64, obliquity: Radian) -> Fraction {
    let c = obliquity.0.cos();
    let s2 = -(5.0 / 16.0) * (3.0 * c * c - 1.0);
    let e = edge.clamp(0.0, 1.0);
    share((1.0 - e) + (s2 / 2.0) * (e - e * e * e))
}

/// What covers a body at mean surface temperature `t` with `water` (kg of
/// liquid and ice; 0 when the planet is dry or in runaway: the caller
/// decides) on a sphere of `radius`, and `frost` bar (weights) of N2,
/// CH4, CO2 and SO2 frost on its ground.
///
/// - The water is open ocean ([`ocean_fraction`] of its oceans) where the
///   surface is warmer than the ice line and ice or snow where it is
///   colder ([`ice_line_share`]); the ice is limited by the sheets the
///   water can make: it spreads as ice of [`ICE_SHEET_THICKNESS`] over
///   the sphere, at the density of ice Ih at one atmosphere.
/// - The sunlit ice, `ice_sunlit`, is the ice weighted by the light its
///   caps receive at `obliquity` ([`sunlit_cap`]).
/// - The frost lies over the ocean's ice and the land, [`frost_cover`] of
///   the disc; the SO2 frost takes its part of that share.
/// - On a tidally `locked` body the ice is weighted by
///   [`locked_sunlit_ice`] of its area share instead.
/// - The rest is bare land.
pub fn cover(
    t: Kelvin,
    water: Kilogram,
    radius: Metre,
    frost: &Weights,
    obliquity: Radian,
    locked: bool,
) -> Cover {
    let (sea, ice, sunlit) = if water.0 > 0.0 {
        let rho = ice_ih_density(
            ice_ih_melting_temperature(STANDARD_ATMOSPHERE),
            STANDARD_ATMOSPHERE,
        );
        let area = 4.0 * std::f64::consts::PI * radius.0 * radius.0;
        let reach = (water.0 / (rho.0 * area * ICE_SHEET_THICKNESS.0)).min(1.0);
        let line = ice_line_share(t).get();
        (
            ocean_fraction(water.0 / OCEAN_MASS.0).get(),
            line * reach,
            if locked {
                locked_sunlit_ice(Fraction::new(line).expect("a share")).get()
            } else {
                sunlit_cap(1.0 - line, obliquity).get()
            } * reach,
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    let total = frost.total();
    let so2_part = if total > 0.0 {
        (frost.so2.max(0.0) / total).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let frost_share = frost_cover(frost).get();
    let ocean = sea * (1.0 - ice) * (1.0 - frost_share);
    let ice = ice * (1.0 - frost_share);
    let sunlit = sunlit * (1.0 - frost_share);
    let land = (1.0 - ocean - ice - frost_share).max(0.0);
    Cover {
        ocean: share(ocean),
        land: share(land),
        ice: share(ice),
        ice_sunlit: share(sunlit),
        frost: share(frost_share),
        so2_frost: share(frost_share * so2_part),
    }
}

/// Albedo of the ground of `cover` under an air of `air`: the frosts
/// and the sunlit ice by their shares (the ice counted by the light its
/// caps receive, not by area), the rest, ocean and land in their area
/// ratio. The land's albedo goes from [`REGOLITH_ALBEDO`] without air to
/// [`SOIL_ALBEDO`] under an air that shields the regolith,
/// `1 - exp(-air / SHIELD_PRESSURE)`.
pub fn ground_albedo(cover: &Cover, air: Bar) -> Fraction {
    let shield = 1.0 - (-air.0.max(0.0) / SHIELD_PRESSURE.0).exp();
    let land = REGOLITH_ALBEDO + (SOIL_ALBEDO - REGOLITH_ALBEDO) * shield;
    let frost = cover.frost.get();
    let sunlit = cover.ice_sunlit.get();
    let bare = cover.ocean.get() + cover.land.get();
    let rest = (1.0 - frost - sunlit).max(0.0);
    let surface = if bare > 0.0 {
        rest * (cover.ocean.get() * OCEAN_ALBEDO + cover.land.get() * land) / bare
    } else {
        0.0
    };
    share(
        surface
            + sunlit * ICE_ALBEDO
            + (frost - cover.so2_frost.get()) * FROST_ALBEDO
            + cover.so2_frost.get() * SO2_FROST_ALBEDO,
    )
}

#[cfg(test)]
#[path = "cover_tests.rs"]
mod tests;
