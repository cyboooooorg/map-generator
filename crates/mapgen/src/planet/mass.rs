//! The mass a slot of a drawn system is given, from where it was drawn
//! rather than from a type name: a planet's type follows from its mass,
//! its composition and its star (see [`super::classify`]), not the other
//! way round.
use rand::RngExt;

use crate::giants::JUPITER_EARTH_MASSES;

/// The mass range of a small planet, Earth masses. Calibration, no source:
/// the target is the Kepler census, where super-Earths (1 to 1.75 R⊕) and
/// sub-Neptunes (1.75 to 3.5 R⊕) are about equally common within 100 days
/// (Fulton et al. 2017, doi:10.3847/1538-3881/aa80eb).
pub const SMALL_MASS: (f64, f64) = (0.05, 20.0);

/// Where a slot of a drawn system was drawn: used for the spacing and the
/// eccentricity of the slot only, never stored on the body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// An inner or outer small planet, from the Kepler counts.
    Small,
    /// A cold gas giant.
    GasGiant,
    /// A cold ice giant.
    IceGiant,
    /// A giant on an orbit of 1 to 10 days.
    HotJupiter,
}

impl Origin {
    /// Whether the slot holds a giant (all but [`Origin::Small`]).
    pub fn is_giant(self) -> bool {
        self != Origin::Small
    }
}

/// A log-uniform draw between `lo` and `hi`.
fn log_uniform(lo: f64, hi: f64, rng: &mut impl RngExt) -> f64 {
    (lo.ln() + rng.random::<f64>() * (hi / lo).ln()).exp()
}

/// The mass of a slot, Earth masses.
///
/// - Small planets: log-uniform on [`SMALL_MASS`] (calibration, no source).
/// - Gas giants: dN/d ln M ∝ M^−0.31 between 0.1 and 13 Jupiter masses
///   (Cumming et al. 2008, doi:10.1086/588487).
/// - Ice giants: log-uniform from 10 to 50 Earth masses. Calibration, no
///   source.
/// - Hot Jupiters: log-uniform from 0.3 to 3 Jupiter masses. Calibration,
///   no source.
pub fn draw_mass(origin: Origin, rng: &mut impl RngExt) -> f64 {
    match origin {
        Origin::Small => log_uniform(SMALL_MASS.0, SMALL_MASS.1, rng),
        Origin::GasGiant => {
            let (lo, hi) = (0.1f64.powf(-0.31), 13f64.powf(-0.31));
            (lo + rng.random::<f64>() * (hi - lo)).powf(1.0 / -0.31)
                * f64::from(JUPITER_EARTH_MASSES)
        }
        Origin::IceGiant => log_uniform(10.0, 50.0, rng),
        Origin::HotJupiter => log_uniform(0.3, 3.0, rng) * f64::from(JUPITER_EARTH_MASSES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const JUPITER: f64 = JUPITER_EARTH_MASSES as f64;

    #[test]
    fn small_planets_span_the_mass_function() {
        let mut rng = StdRng::seed_from_u64(1);
        let m: Vec<f64> = (0..20_000)
            .map(|_| draw_mass(Origin::Small, &mut rng))
            .collect();
        assert!(
            m.iter()
                .all(|&x| (SMALL_MASS.0..=SMALL_MASS.1).contains(&x))
        );
        let below_2 = m.iter().filter(|&&x| x < 2.0).count() as f64 / m.len() as f64;
        // Log-uniform from 0.05 to 20 M⊕: log(2/0.05)/log(20/0.05) = 0.62.
        assert!((below_2 - 0.62).abs() < 0.02, "{below_2}");
    }

    #[test]
    fn giant_origins_keep_todays_laws() {
        let mut rng = StdRng::seed_from_u64(2);
        for _ in 0..2_000 {
            let gas = draw_mass(Origin::GasGiant, &mut rng);
            assert!((0.1 * JUPITER..=13.0 * JUPITER).contains(&gas));
            let ice = draw_mass(Origin::IceGiant, &mut rng);
            assert!((10.0..=50.0).contains(&ice));
            let hot = draw_mass(Origin::HotJupiter, &mut rng);
            assert!((0.3 * JUPITER..=3.0 * JUPITER).contains(&hot));
        }
    }
}
