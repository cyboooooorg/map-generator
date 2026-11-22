//! The crust that floats on the mantle: its thickness, its density, and the
//! height of its columns (Airy isostasy).

use super::spectrum::Analogue;
use crate::constants::DEGREES_PER_HALF_TURN;
use crate::units::{Fraction, KilogramPerCubicMetre, Metre, Radian};

/// A crust of two thicknesses floating on its mantle: the thick
/// (continental, or highland) crust and the thin (oceanic, or lowland) one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crust {
    /// Thickness of the thick crust.
    pub continental_thickness: Metre,
    /// Thickness of the thin crust, the datum of [`floating_height`].
    pub oceanic_thickness: Metre,
    /// Mean density of the crust.
    pub crust_density: KilogramPerCubicMetre,
    /// Density of the mantle at the base of the crust.
    pub mantle_density: KilogramPerCubicMetre,
}

/// Source: Earth's crust. Continental crust 41.1 km and 2830 kg/m³, the
/// means of Christensen & Mooney's (1995) model of the continental crust
/// (doi:10.1029/95JB00259); oceanic crust 7.1 km, its igneous section away
/// from fracture zones and hot spots (White, McKenzie & O'Nions 1992,
/// doi:10.1029/92JB01749); mantle 3380 kg/m³, PREM's LID at the top of
/// the mantle, `2.6910 + 0.6924 r/a` g/cm³ at 24.4 km depth (Dziewonski &
/// Anderson 1981, Table I). The continents stand
/// `34 km × (1 − 2830/3380) = 5.53 km` above the oceanic crust. Ported
/// from `legacy-final:crates/mapgen/src/generation.rs:505-516`, whose
/// mantle of 3300 kg/m³ was assumed.
pub const EARTH_CRUST: Crust = Crust {
    continental_thickness: Metre(41.1e3),
    oceanic_thickness: Metre(7.1e3),
    crust_density: KilogramPerCubicMetre(2830.0),
    mantle_density: KilogramPerCubicMetre(3380.0),
};

/// Source: Mars's crust, its dichotomy. The modal thicknesses of the
/// southern highlands and northern lowlands, 58 and 32 km (Neumann et al.
/// 2004, doi:10.1029/2004JE002262); crust 2900 and mantle 3500 kg/m³
/// (Wieczorek 2007, Fig. 11 caption; Neumann et al.'s 600 kg/m³
/// contrast). The highlands stand `26 km × (1 − 2900/3500) = 4.46 km`
/// above the lowlands. Ported from
/// `legacy-final:crates/mapgen/src/generation.rs:518-533`.
pub const MARS_CRUST: Crust = Crust {
    continental_thickness: Metre(58e3),
    oceanic_thickness: Metre(32e3),
    crust_density: KilogramPerCubicMetre(2900.0),
    mantle_density: KilogramPerCubicMetre(3500.0),
};

/// The crust of an analogue body: [`EARTH_CRUST`] or [`MARS_CRUST`].
pub fn crust(analogue: Analogue) -> Crust {
    match analogue {
        Analogue::Earth => EARTH_CRUST,
        Analogue::Mars => MARS_CRUST,
    }
}

/// Source: share of the surface the thick crust covers. Earth's
/// continental crust 0.41 (Cogley 1984); Mars's southern highlands 0.58,
/// one less the 0.42 of the northern lowlands (Marinova, Aharonson &
/// Asphaug 2008, doi:10.1038/nature07070).
pub fn continental_share(analogue: Analogue) -> Fraction {
    let share = match analogue {
        Analogue::Earth => 0.41,
        Analogue::Mars => 0.58,
    };
    Fraction::new(share).expect("a share within 0 and 1")
}

/// Height of a crust column `thickness` thick above a column of the
/// crust's oceanic thickness: `(t − t_o)(1 − ρc/ρm)` (Airy isostasy, Airy
/// 1855, doi:10.1098/rstl.1855.0003), whatever the gravity. Negative for
/// a column thinner than the oceanic crust. Ported from
/// `legacy-final:crates/mapgen/src/generation.rs` (`Crust::height_km` and
/// the Airy part of `floating`, 605-650).
pub fn floating_height(crust: &Crust, thickness: Metre) -> Metre {
    let buoyancy = 1.0 - crust.crust_density.0 / crust.mantle_density.0;
    Metre((thickness - crust.oceanic_thickness).0 * buoyancy)
}

/// Source: the gradient over which the continent's edge falls to the ocean
/// floor, 2.4°: the mean gradient of ETOPO 2022 (NOAA NCEI,
/// doi:10.25921/fd45-gt74) between 200 and 2500 m deep within 150 km of
/// land, measured by the study's `scripts/etopo_slope.py` (2.2-3.1° over
/// the method's choices); see
/// `docs/superpowers/studies/2026-11-22-relief/README.md`.
pub const CONTINENTAL_SLOPE: Radian = Radian(2.4 * std::f64::consts::PI / DEGREES_PER_HALF_TURN);

#[cfg(test)]
#[path = "crust_tests.rs"]
mod tests;
