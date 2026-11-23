//! What a star's ultraviolet does to a planet's air: the photolysis of
//! methane by every star a planet sees.
//!
//! The destruction is photon-limited: any column worth counting absorbs
//! all the Lyman α a star sends, so the rate is set by the light, not by
//! the methane. A planet outgasses its methane at a steady rate over its
//! age; while the summed destruction outruns that supply nothing builds
//! up, and from the age at which the falling destruction meets it methane
//! gathers. Ported from the tag `legacy-final` (`planet/atmosphere.rs`,
//! `methane_photolysis`), which had one star; the law, its calibration and
//! its departures are on the page `docs/science/planet-atmospheres.md`,
//! *Methane photolysis*.

use crate::constants::{AVOGADRO, PASCALS_PER_BAR, PLANCK, SPEED_OF_LIGHT};
use crate::radiation::gases::CH4_MOLAR_MASS;
use crate::units::{
    Bar, Gyr, Kilogram, Metre, MetrePerSecondSquared, Second, WattPerSquareMetre, Year,
};
use crate::xuv::{History, LYMAN_ALPHA};

/// Calibration: methane molecules destroyed for each Lyman α photon a star
/// sends a body, averaged over its sphere, fitted on Titan's present loss.
/// Yung, Allen & Pinto 1984 (ApJS 55, 465, doi:10.1086/190963, Table 5B;
/// full text read) give a net loss of 1.5e10 molecules cm⁻² s⁻¹; the
/// rewrite's Sun (1 M☉, [Fe/H] 0, 4.6 Gyr, [`crate::xuv::history`]) sends
/// 1.064e9 Lyman α photons cm⁻² s⁻¹ over the sphere of a body at 9.58 AU,
/// hence 14.1 (`legacy-final` had 13.9 from the abstract's 1.48e10). Over
/// one because Lyman α stands for all the light that destroys methane:
/// Lyman α's own photolysis is 2.9e9 of the 1.5e10, and most of the rest
/// is the dissociation photosensitised by acetylene and diacetylene at
/// 1600 to 2300 Å (their Table 2A, § VI e). That light comes from the
/// photosphere and fades far less with age than Lyman α, so the law
/// overstates the destruction round a young star. No yield for any other
/// body or spectrum was found: see the calibration of `Y` on the page
/// `docs/science/planet-atmospheres.md` and study 23 of the Wiki.
pub const CH4_PER_LYMAN_ALPHA_PHOTON: f64 = 14.1;

/// Source: vacuum wavelength of the H I Lyman α multiplet, 1215.6701 Å
/// (NIST Atomic Spectra Database, H I lines; read).
pub const LYMAN_ALPHA_WAVELENGTH: Metre = Metre(1215.6701e-10);

/// Definition: energy of a Lyman α photon, J: `h c / λ` (1.634e-18 J).
pub const LYMAN_ALPHA_PHOTON_ENERGY: f64 = PLANCK * SPEED_OF_LIGHT / LYMAN_ALPHA_WAVELENGTH.0;

/// Definition: mass of a methane molecule, its molar mass over the
/// Avogadro constant (2.664e-26 kg).
pub const CH4_MOLECULE_MASS: Kilogram = Kilogram(CH4_MOLAR_MASS / AVOGADRO);

/// Definition: bisection steps that place the onset of the build-up, each
/// halving the bracket `[0, age]`: 60 reach the f64 resolution (as
/// [`crate::xuv`]'s corona boundary).
const ONSET_BISECTION_STEPS: u32 = 60;

/// One star a planet sees in Lyman α: its present bolometric flux at the
/// planet and its high-energy history.
#[derive(Clone, Copy, Debug)]
pub struct LymanSource<'a> {
    /// The star's present bolometric flux at the planet.
    pub flux: WattPerSquareMetre,
    /// The star's history, which gives its Lyman α share at every age.
    pub history: &'a History,
}

/// Methane destroyed per Gyr, bar, on a body of surface `gravity` under a
/// present bolometric flux `flux`, per unit of the Lyman α share of the
/// star's present light: `Y · F / 4 / E_γ` photons per m² and second over
/// the sphere, times the mass of a molecule, times `g` over the pascals
/// in a bar.
fn bar_per_share_gyr(gravity: MetrePerSecondSquared, flux: WattPerSquareMetre) -> f64 {
    let photons = flux.0.max(0.0) / 4.0 / LYMAN_ALPHA_PHOTON_ENERGY;
    let kg_m2_s = CH4_PER_LYMAN_ALPHA_PHOTON * photons * CH4_MOLECULE_MASS.0;
    kg_m2_s * gravity.0 / PASCALS_PER_BAR * Second::from(Year::from(Gyr(1.0))).0
}

/// The summed destruction rate of the `sources` at `age`, bar per Gyr.
fn destruction_rate(gravity: MetrePerSecondSquared, sources: &[LymanSource], age: Gyr) -> f64 {
    sources
        .iter()
        .map(|s| bar_per_share_gyr(gravity, s.flux) * s.history.ratio_at(LYMAN_ALPHA, age))
        .sum()
}

/// The methane the `sources`' Lyman α destroys between ages `from` and
/// `to` on a body of surface `gravity`, bar (as a weight, `m g`),
/// photon-limited: whatever methane there is, each photon destroys
/// [`CH4_PER_LYMAN_ALPHA_PHOTON`] molecules. Each source counts its flux
/// averaged over the sphere (`/ 4`) and its Lyman α history
/// ([`History::integrated_between`]); the sources add up.
pub fn methane_destroyed(
    gravity: MetrePerSecondSquared,
    sources: &[LymanSource],
    from: Gyr,
    to: Gyr,
) -> Bar {
    Bar(sources
        .iter()
        .map(|s| {
            bar_per_share_gyr(gravity, s.flux)
                * s.history.integrated_between(LYMAN_ALPHA, from, to).0
        })
        .sum())
}

/// The methane left, bar, once the `sources`' ultraviolet has worked on
/// the methane a body of surface `gravity` `outgassed` over its `age`.
///
/// The supply is steady, `outgassed / age`. The summed destruction
/// ([`methane_destroyed`]) is constant while the stars are saturated and
/// only falls after, so the supply overtakes it once, at an onset found by
/// bisection on `[0, age]` (the age itself if it never does). Before the
/// onset nothing gathers; after it the stock is `supply · (age − onset)`
/// less what the stars destroy over that span, at least 0. Destroyed
/// methane is gone: its hydrogen escapes and its carbon ends in heavier
/// hydrocarbons on the ground. Returns `outgassed` unchanged when it or
/// `age` is 0.
pub fn methane_left(
    outgassed: Bar,
    gravity: MetrePerSecondSquared,
    sources: &[LymanSource],
    age: Gyr,
) -> Bar {
    if outgassed.0 <= 0.0 || age.0 <= 0.0 {
        return outgassed;
    }
    let supply = outgassed.0 / age.0;
    let onset = onset(gravity, sources, supply, age);
    let destroyed = methane_destroyed(gravity, sources, onset, age).0;
    Bar((supply * (age.0 - onset.0) - destroyed).max(0.0))
}

/// The age from which the summed destruction rate is at most `supply`,
/// by bisection on `[0, age]`: `age` if it stays above it.
fn onset(gravity: MetrePerSecondSquared, sources: &[LymanSource], supply: f64, age: Gyr) -> Gyr {
    if destruction_rate(gravity, sources, age) > supply {
        return age;
    }
    let (mut above, mut below) = (0.0, age.0);
    for _ in 0..ONSET_BISECTION_STEPS {
        let mid = 0.5 * (above + below);
        if destruction_rate(gravity, sources, Gyr(mid)) > supply {
            above = mid;
        } else {
            below = mid;
        }
    }
    Gyr(0.5 * (above + below))
}

#[cfg(test)]
#[path = "photochemistry_tests.rs"]
mod tests;
