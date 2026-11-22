//! Volcanism and the shape of volcanoes: how much of a surface the
//! planet's heat makes volcanic, and the size of its cones.
//!
//! Values and their citations: the relief study,
//! `docs/superpowers/studies/2026-11-22-relief/volcanoes.md`, and its
//! calibration list (`README.md#final-calibration-list`).

use crate::constants::{MARS_SURFACE_GRAVITY, STANDARD_GRAVITY};
use crate::units::{Fraction, Metre, Watt, WattPerSquareMetre};
use std::f64::consts::PI;

/// Source: Earth's mean surface heat flux, 0.087 W/m², from 44.2 TW over
/// its surface (Pollack, Hurter & Johnson 1993, doi:10.1029/93RG01249).
pub const EARTH_HEAT_FLUX: WattPerSquareMetre = WattPerSquareMetre(0.087);

/// Source: heat flux of a heat-pipe planet, 2.5 W/m², Io's, "about 40
/// times Earth's heat flux" carried through volcanism (Moore & Webb 2013,
/// p. 501, doi:10.1038/nature12473): the flux at which volcanism is
/// total.
pub const HEAT_PIPE_FLUX: WattPerSquareMetre = WattPerSquareMetre(2.5);

/// Calibration: volcanic intensity at [`EARTH_HEAT_FLUX`], 0.5, the
/// legacy's (the middle of the range its worlds drew from). Searched and
/// not found: a law from heat flux to volcanic surface share or cone
/// density (Kite et al. 2009, Crisp 1984, Moore & Webb 2013, Kankanamge &
/// Moore 2019; relief study, calibration 2,
/// `docs/superpowers/studies/2026-11-22-relief/volcanoes.md#heat-flux-to-volcanic-intensity-no-law`).
pub const EARTH_VOLCANIC_INTENSITY: f64 = 0.5;

/// Mean heat flux through the surface of a body of radius `radius` that
/// loses `total`: `total / 4πR²`.
pub fn heat_flux(total: Watt, radius: Metre) -> WattPerSquareMetre {
    WattPerSquareMetre(total.0 / (4.0 * PI * radius.0 * radius.0))
}

/// Volcanic intensity, from 0 (no volcanism) to 1 (a heat pipe), of a
/// surface losing `flux`. Calibration (relief study, calibration 2): a
/// power law `I = 0.5 (F / F⊕)^k` through Earth's flux at
/// [`EARTH_VOLCANIC_INTENSITY`] and [`HEAT_PIPE_FLUX`] at 1,
/// `k = ln 2 / ln(2.5 / 0.087) = 0.206`, capped at 1. The legacy's square
/// root reached 1 at four times Earth's flux, below the heat pipe; no law
/// is published (see [`EARTH_VOLCANIC_INTENSITY`]).
pub fn volcanic_intensity(flux: WattPerSquareMetre) -> Fraction {
    if flux.0 <= 0.0 {
        return Fraction::new(0.0).expect("zero is a fraction");
    }
    let exponent =
        (1.0 / EARTH_VOLCANIC_INTENSITY).ln() / (HEAT_PIPE_FLUX.0 / EARTH_HEAT_FLUX.0).ln();
    let intensity = EARTH_VOLCANIC_INTENSITY * (flux.0 / EARTH_HEAT_FLUX.0).powf(exponent);
    Fraction::new(intensity.min(1.0)).expect("an intensity within 0 and 1")
}

/// Calibration: share of the surface in volcanic provinces at a volcanic
/// intensity of 1, 0.3, the legacy's; proportional to the intensity.
/// Bounded by the Moon's maria (16-17 %), Mercury's volcanic plains (about
/// 18 %) and Venus's (a half to three quarters); searched and not found:
/// Mars and Venus volcanic unit areas against a flux (relief study,
/// calibration 3,
/// `docs/superpowers/studies/2026-11-22-relief/volcanoes.md#volcanic-provinces`).
pub const VOLCANIC_PROVINCE_SHARE: f64 = 0.3;

/// Source: aspect of a volcano, its height over its basal width, "mostly
/// from 0.01 to 0.1" over 158 shield volcanoes (Grosse & Kervyn 2018,
/// doi:10.1016/j.geomorph.2017.12.017).
pub const CONE_ASPECT: (f64, f64) = (0.01, 0.1);

/// Source: volume of a volcano, "from 0.1 to >1000 km³", in cubic metres
/// (Grosse & Kervyn 2018, as [`CONE_ASPECT`]).
pub const CONE_VOLUME: (f64, f64) = (0.1e9, 1000e9);

/// Source: the tallest a volcano stands at standard gravity, 8.4 km:
/// Olympus Mons's relief, "almost 22 km" (Plescia 2004,
/// doi:10.1029/2002JE002031), scaled by Mars's mean surface gravity
/// ([`MARS_SURFACE_GRAVITY`], NASA Mars fact sheet) over standard gravity, relief standing as one over gravity
/// (Weisskopf 1975). Mauna Loa's 9 km from the sea floor agrees.
pub const CONE_HEIGHT_MAX: Metre = Metre(22e3 * MARS_SURFACE_GRAVITY.0 / STANDARD_GRAVITY.0);

/// Calibration: spacing of isolated volcanoes, one at most per cell of
/// 900 km, the legacy's lattice. Searched and not found: a global law of
/// the spacing of isolated volcanoes (ten Brink 1991 and Hahn & Byrne 2023
/// give local densities; relief study, calibration 5,
/// `docs/superpowers/studies/2026-11-22-relief/volcanoes.md#cones`).
pub const CONE_SPACING: Metre = Metre(900e3);

/// Calibration: share of the [`CONE_SPACING`] cells that hold a volcano at
/// Earth's intensity, 0.5, the legacy's (relief study, calibration 5, as
/// [`CONE_SPACING`]).
pub const CONE_PRESENCE: f64 = 0.5;

/// Calibration: volcanoes per metre of arc, one per 250 km, the legacy's
/// (one per 200 km cell in 80 % of the cells); the cones stand at random
/// points of the arc band, as de Bremond d'Ars, Jaupart & Sparks (1995,
/// doi:10.1029/95JB02153) find them. Searched and not found: their Gamma
/// law's mean (full text refused); the arcs of Alaska carry one every
/// 50-70 km, a regional figure (relief study, calibration 6,
/// `docs/superpowers/studies/2026-11-22-relief/volcanoes.md#arc-volcanoes`).
pub const ARC_CONE_DENSITY: f64 = 1.0 / 250e3;

/// A volcano's cone: its height and the radius of its base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    /// Height above the ground around it.
    pub height: Metre,
    /// Radius of its base.
    pub radius: Metre,
}

/// The cone of draws `volume` and `aspect` (uniform in `[0, 1]`), at
/// standard gravity. Calibration (relief study, calibration 4): the volume
/// log-uniform within [`CONE_VOLUME`] and the aspect uniform within
/// [`CONE_ASPECT`], as no global distribution of volcano sizes was found;
/// a cone `V = π r² h / 3` with `h = 2 a r`, capped at
/// [`CONE_HEIGHT_MAX`] at the same aspect.
pub fn cone_size(volume: Fraction, aspect: Fraction) -> Cone {
    let (low, high) = CONE_VOLUME;
    let v = low * (high / low).powf(volume.get());
    let a = CONE_ASPECT.0 + (CONE_ASPECT.1 - CONE_ASPECT.0) * aspect.get();
    let radius = (3.0 * v / (2.0 * PI * a)).cbrt();
    let height = (2.0 * a * radius).min(CONE_HEIGHT_MAX.0);
    Cone {
        height: Metre(height),
        radius: Metre(height / (2.0 * a)),
    }
}

#[cfg(test)]
#[path = "volcanoes_tests.rs"]
mod tests;
