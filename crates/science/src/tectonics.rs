//! The thresholds of a planet's tectonic regime: the heat flux of a heat
//! pipe and the smallest mass with plates. Both are calibrations, since
//! the literature disagrees (see `docs/science/plate-tectonics.md`).
//!
//! Ported from the tag `legacy-final` (`planet/regime.rs`, `geology.rs`),
//! with the sources checked in study
//! [21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit).

use crate::units::{EarthMass, WattPerSquareMetre};

/// Calibration: the regime's onset. Internal heat flux above which a body loses its heat
/// through volcanic heat pipes rather than by plates or conduction. No
/// paper searched sets such a threshold. Moore & Webb 2013 ("Heat-pipe
/// Earth", Nature 501, 501, doi:10.1038/nature12473) take Io as the
/// heat-pipe body of today, carrying about 2.5 W/m², some 40 times
/// Earth's mean (from Veeder et al. 2004). 1 W/m², under half of Io's flux
/// and over ten times Earth's, keeps Io a heat pipe even at the low end
/// of its range; Lainey et al. 2009 (doi:10.1038/nature08108) is not
/// reached beyond its abstract record, so its figure is not used. See
/// `docs/science/planet-atmospheres.md`.
pub const HEAT_PIPE_ONSET_FLUX: WattPerSquareMetre = WattPerSquareMetre(1.0);

/// Calibration: smallest mass of a planet with plate tectonics, the
/// geometric mean of Mars (0.107 M⊕, a stagnant lid) and Venus (0.815
/// M⊕), `√(0.107 × 0.815) = 0.295`, NASA planetary fact sheets. Foley,
/// Bercovici & Landuyt 2012 (doi:10.1016/j.epsl.2012.03.028) favour
/// plates on larger planets, Stern, Gerya & Tackley 2018
/// (doi:10.1016/j.gsf.2017.06.004) tie activity to the vigour of
/// convection, and Korenaga 2010 (doi:10.1088/2041-8205/725/1/L43) finds
/// that size does not matter; none gives a mass.
pub const PLATES_MIN_MASS: EarthMass = EarthMass(0.3);

#[cfg(test)]
#[path = "tectonics_tests.rs"]
mod tests;
