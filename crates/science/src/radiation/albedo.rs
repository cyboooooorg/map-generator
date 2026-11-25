//! The shortwave budget: where the star's light goes on a body whose
//! ground albedo, water cloud cover and sulphuric deck are given (ported
//! from `legacy-final`'s `planet/albedo.rs`, `paths_through` and the part
//! of `shortwave_budget` after the surface cover, which comes with the
//! global climate).
//!
//! The air alone (its haze, its Rayleigh depth and its column) sets the
//! light's paths ([`Paths`]); the ground and the clouds under them are
//! one cheap sum over the paths. The climate asks for the albedo of the
//! same air many times while its cover settles, so the caller keeps the
//! last airs' paths in a [`ShortwaveCache`].

use super::column::{Profile, levels};
use super::gases::PartialPressures;
use super::rayleigh::rayleigh_depth_550;
use super::shortwave::{Air, Paths};
use super::starlight::{StarBands, star_bands};
use super::veils::{DECK_ALBEDO, HAZE_ABSORPTION, HAZE_ALBEDO, haze};
use crate::units::{Fraction, Kelvin, MetrePerSecondSquared};

#[cfg(test)]
#[path = "albedo_tests.rs"]
mod tests;

/// Calibration: albedo of the tops of a water cloud layer, as the
/// reflectance of the cloud cover's share of the disc, 0.276. Fitted on
/// Earth's Bond albedo, 0.293 (CERES; Stephens et al. 2015,
/// doi:10.1002/2014RG000449; Kato et al. 2018,
/// doi:10.1175/JCLI-D-17-0523.1, give 0.2915), by bisection to three
/// digits (0.2758 before rounding; 0.29308 with 0.276) over Earth's
/// inputs: a ground of 0.122 (CERES EBAF Ed4.0 clear-sky surface, Kato et
/// al. 2018, Table 5), its dry air with 400 ppm of CO₂ at 77 % relative
/// humidity over 288 K (Manabe & Wetherald 1967), the cloud cover the law
/// gives it, 0.682 ([`super::clouds::cloud_cover`]), standard gravity and
/// the Sun at 5772 K. What was searched and not found: a single cloud
/// reflectance for a one-column model, which no observation gives (a
/// thick cloud reflects 0.5 to 0.7; Earth's cover counts thin clouds
/// too). The legacy took 0.252, fitted on 0.30 over its own cover.
///
/// The fit absorbs what the port lacks. In the clear sky, the port is
/// brighter than the observed aerosol-free sky by its black-body Sun,
/// its missing ozone and O₂ (issues #653 and #655, about +0.03 together)
/// and darker by the aerosols' reflection it does not carry (about
/// 0.017); and Earth's column at 77 % holds 1.735 cm of water, what one
/// column needs for the observed field's outgoing longwave, where its
/// absorbed sunlight would need about 2.4 cm: about 0.0025 of Bond
/// albedo (Wiki study 33, issue #654). The clouds' reflectance makes up
/// the rest: refit it when any of these lands. See
/// `docs/science/radiative-transfer.md`, "The cloud's water, the deck and
/// the haze".
pub(crate) const CLOUD_ALBEDO: f64 = 0.276;

/// Definition: airs a [`ShortwaveCache`] remembers, the legacy memo's size
/// (a numerical choice with no source: audit, Wiki study 19).
const REMEMBERED_AIRS: usize = 32;

/// The inputs of an air's paths, as bits: the seven partial pressures,
/// the surface temperature, the gravity and the star's temperature.
type AirKey = [u64; 10];

/// What lies under the air: the ground's albedo, the share of the sky
/// under water clouds, and the share of the disc under a sulphuric deck
/// ([`super::veils::sulphuric_deck`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Underneath {
    /// Albedo of the ground (its ocean, land, ice and frost together).
    pub ground: Fraction,
    /// Share of the sky under water clouds
    /// ([`super::clouds::cloud_cover`]).
    pub clouds: Fraction,
    /// Share of the disc under a sulphuric acid deck.
    pub deck: Fraction,
}

/// Where the star's light goes, as shares of it: reflected to space (the
/// Bond albedo) and absorbed by the air; the ground absorbs the rest.
/// What the air absorbs is absorbed by the planet: the climate takes only
/// the reflected light out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShortwaveBudget {
    /// Share of the light reflected to space.
    pub bond_albedo: Fraction,
    /// Share of the light absorbed by the air (its gases and haze).
    pub absorbed_by_air: Fraction,
}

/// What [`shortwave_budget`] keeps between calls, owned by the caller:
/// the last [`REMEMBERED_AIRS`] airs and their light's paths, the oldest
/// replaced first (allocated on first use), and the last star's bands.
/// One cache per thread of work; a hit returns the bits a fresh air
/// gives.
#[derive(Default)]
pub struct ShortwaveCache {
    airs: Vec<(AirKey, Paths)>,
    next: usize,
    star: Option<(u64, StarBands)>,
}

impl std::fmt::Debug for ShortwaveCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShortwaveCache")
            .field("remembered", &self.airs.len())
            .field("star", &self.star.as_ref().map(|(_, s)| s.star_k))
            .finish()
    }
}

impl ShortwaveCache {
    /// The light's paths through `gases` over a surface at `surface_k` on
    /// a body of gravity `gravity_m_s2` under a star of `star_k`: its
    /// haze, its Rayleigh depth and its column, whatever lies beneath.
    fn paths(
        &mut self,
        gases: &PartialPressures,
        surface_k: f64,
        gravity_m_s2: f64,
        star_k: f64,
    ) -> &Paths {
        let key: AirKey = [
            gases.n2,
            gases.co2,
            gases.h2o,
            gases.ch4,
            gases.nh3,
            gases.so2,
            gases.o2,
            surface_k,
            gravity_m_s2,
            star_k,
        ]
        .map(f64::to_bits);
        if let Some(i) = self.airs.iter().position(|(k, _)| *k == key) {
            return &self.airs[i].1;
        }
        let star = match &mut self.star {
            Some((bits, bands)) if *bits == star_k.to_bits() => bands,
            slot => &slot.insert((star_k.to_bits(), star_bands(star_k))).1,
        };
        let h = haze(gases);
        let column = levels(gases, surface_k, gravity_m_s2, Profile::SHORTWAVE);
        let air = Air {
            haze: (h * HAZE_ALBEDO, 1.0 - h * (HAZE_ALBEDO + HAZE_ABSORPTION)),
            rayleigh_550: rayleigh_depth_550(gases, gravity_m_s2),
            column: column.as_ref(),
        };
        let paths = Paths::through(&air, &star.bands);
        if self.airs.capacity() == 0 {
            self.airs.reserve_exact(REMEMBERED_AIRS);
        }
        let at = self.next;
        if self.airs.len() < REMEMBERED_AIRS {
            self.airs.push((key, paths));
        } else {
            self.airs[at] = (key, paths);
        }
        self.next = (at + 1) % REMEMBERED_AIRS;
        &self.airs[at].1
    }
}

/// Where the light of a star at `star` goes on a body of surface gravity
/// `gravity` whose air `gases` lies over a surface at `surface` with
/// `underneath` below it: the share reflected (its Bond albedo) and the
/// share its air absorbs.
///
/// A whole deck reflects [`DECK_ALBEDO`] and lets nothing through.
/// Otherwise the light crosses the air's haze, its Rayleigh scattering
/// and the near-infrared absorption of its column (built as
/// [`Profile::SHORTWAVE`] says), the water clouds (their cover times
/// [`CLOUD_ALBEDO`]) at their tops inside it, down to the ground; the
/// deck's share of the disc then reflects [`DECK_ALBEDO`] over the rest.
/// A star whose temperature is not positive or not finite (NaN, infinite)
/// is the Sun. A negative or NaN partial pressure counts as none; without
/// air the ground's albedo comes back. A gravity that is not finite or not
/// positive is no air: the budget is that of a body without air, the
/// ground under the clouds and the deck.
///
/// `cache` keeps the last airs' paths, keyed on the exact bits of their
/// inputs: a hit returns the bits a fresh air gives.
pub fn shortwave_budget(
    gases: &PartialPressures,
    surface: Kelvin,
    gravity: MetrePerSecondSquared,
    star: Kelvin,
    underneath: Underneath,
    cache: &mut ShortwaveCache,
) -> ShortwaveBudget {
    let deck = underneath.deck.get();
    if deck >= 1.0 {
        return ShortwaveBudget {
            bond_albedo: fraction(DECK_ALBEDO),
            absorbed_by_air: fraction(0.0),
        };
    }
    let weightless = !(gravity.0 > 0.0 && gravity.0.is_finite());
    let paths = if weightless {
        // No air: its gravity would only make the Rayleigh depth infinite.
        cache.paths(&PartialPressures::default(), surface.0, 1.0, star.0)
    } else {
        cache.paths(gases, surface.0, gravity.0, star.0)
    };
    let below = paths.budget(
        underneath.ground.get(),
        (underneath.clouds.get() * CLOUD_ALBEDO).clamp(0.0, 1.0),
    );
    let albedo = fraction(deck * DECK_ALBEDO + (1.0 - deck) * below.albedo);
    ShortwaveBudget {
        bond_albedo: albedo,
        absorbed_by_air: fraction(((1.0 - deck) * below.air).min(1.0 - albedo.get())),
    }
}

/// `x` as a share, held within 0 to 1 against the rounding of the sums
/// (a path that absorbs nothing leaves `1 - a - ground` a few ulps below
/// zero). A NaN reads as none.
fn fraction(x: f64) -> Fraction {
    Fraction::new(x.clamp(0.0, 1.0))
        .unwrap_or_else(|| Fraction::new(0.0).expect("zero is a fraction"))
}
