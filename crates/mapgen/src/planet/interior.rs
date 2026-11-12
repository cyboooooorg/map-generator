//! The rocky and icy interior: iron core, rock mantle and water shell,
//! their radii, and the radius, bulk density and surface gravity they give
//! (spec "Derived, in order", item 1). The hydrogen and helium envelope is
//! added on top by the envelope model.
//!
//! The refractory part (iron and rock) follows the mass–radius relation of
//! Zeng, Sasselov & Jacobsen (2016) where it applies, and its uncompressed
//! size below it; the water shell is compressed along with it. Units:
//! masses in Earth masses, radii in Earth radii, densities relative to
//! Earth's, so that a sphere of mass `M` and relative density `ρ` has
//! `R³ = M / ρ`.
use super::{Interior, Layer, Material, PlanetInputs};
use crate::world::UNCOMPRESSED_ROCK_DENSITY;

/// Earth's bulk density, g/cm³ (NASA Earth fact sheet: 5.513).
pub const EARTH_DENSITY: f64 = 5.513;

/// Uncompressed density of the iron core, g/cm³: an Fe–FeS alloy, lighter
/// than pure iron (7.87). Calibration, no source: with
/// [`UNCOMPRESSED_ROCK_DENSITY`] it gives Mercury 0.387 R⊕ (0.383
/// observed) and Mars 0.536 (0.532).
const IRON_DENSITY: f64 = 7.0;

/// Mean density of the water and ice shell before compression, g/cm³: a
/// mix of ice I and high-pressure ices. Calibration, no source: it gives
/// Ganymede 0.409 R⊕ (0.413 observed), Callisto 0.376 (0.378) and Titan
/// 0.398 (0.404).
const WATER_DENSITY: f64 = 1.25;

/// How much more the water shell compresses than the refractory part: the
/// shell's density grows as `k^1.5` where the rock's grows as `k` (ices
/// are softer than rock). Calibration, no source: it brings a 5 M⊕
/// planet with half its mass in water to 1.954 R⊕, between the 300 K
/// (1.911) and 700 K (1.981) curves of Zeng et al. 2019
/// (doi:10.1073/pnas.1812905116), and stays within −6 % to +6 % of them
/// from 1 to 20 M⊕ (sharing the rock's compression, `k^1`, gave 2.074,
/// 8.5 % above the 300 K curve). Bodies too small to compress (the icy
/// moons, `k = 1`) are not affected.
const WATER_COMPRESSION_EXPONENT: f64 = 1.5;

/// Coefficient of the mass–radius relation of rocky planets at core mass
/// fraction `cmf`: `R/R⊕ = (1.07 − 0.21 CMF) (M/M⊕)^(1/3.7)` (Zeng,
/// Sasselov & Jacobsen 2016, doi:10.3847/0004-637X/819/2/127,
/// fitted to Earth-like models scaled from PREM, 1 to 8 M⊕).
fn zeng_radius(mass_earth: f64, cmf: f64) -> f64 {
    (1.07 - 0.21 * cmf) * mass_earth.powf(1.0 / 3.7)
}

/// The iron, rock and water layers of a planet, its radius, bulk density
/// and surface gravity. The envelope share is ignored here: the
/// refractory mass is `M (1 − water − envelope)`.
///
/// The refractory part takes the smaller of the relation of Zeng et al.
/// (2016) and its uncompressed size (a mix of [`IRON_DENSITY`] and
/// [`UNCOMPRESSED_ROCK_DENSITY`]): small bodies do not compress their
/// rock. Its compression `k = (R_unc / R)³ ≥ 1` shrinks the core alike,
/// and the water shell as `k^1.5` ([`WATER_COMPRESSION_EXPONENT`]).
/// Layers of zero mass are left out.
pub fn solid_interior(inputs: &PlanetInputs) -> Interior {
    let mass = inputs.mass_earth;
    let cmf = inputs.core_mass_fraction.clamp(0.0, 1.0);
    let water = mass * inputs.water_fraction;
    let refractory = mass * (1.0 - inputs.water_fraction - inputs.envelope_fraction);
    let iron = cmf * refractory;
    let rock = refractory - iron;

    let iron_rel = IRON_DENSITY / EARTH_DENSITY;
    let rock_rel = f64::from(UNCOMPRESSED_ROCK_DENSITY);
    let water_rel = WATER_DENSITY / EARTH_DENSITY;
    let refractory_rel = 1.0 / (cmf / iron_rel + (1.0 - cmf) / rock_rel);

    let uncompressed = (refractory / refractory_rel).cbrt();
    let refractory_radius = zeng_radius(refractory, cmf).min(uncompressed);
    let k = if refractory_radius > 0.0 {
        (uncompressed / refractory_radius).powi(3)
    } else {
        1.0
    };

    let core_radius = (iron / (iron_rel * k)).cbrt();
    let water_volume = water / (water_rel * k.powf(WATER_COMPRESSION_EXPONENT));
    let radius = (refractory_radius.powi(3) + water_volume).cbrt();

    let layers: Vec<Layer> = [
        (Material::Iron, iron, core_radius),
        (Material::Rock, rock, refractory_radius),
        (Material::Water, water, radius),
    ]
    .into_iter()
    .filter(|&(_, m, _)| m > 0.0)
    .map(|(material, mass_earth, outer_radius_earth)| Layer {
        material,
        mass_earth,
        outer_radius_earth,
    })
    .collect();
    let radius_earth = layers.last().map_or(0.0, |l| l.outer_radius_earth);
    let solid_mass = iron + rock + water;
    Interior {
        layers,
        radius_earth,
        density: EARTH_DENSITY * solid_mass / radius_earth.powi(3),
        gravity: solid_mass / radius_earth.powi(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::reference::{inputs, inputs_of, reference};

    #[test]
    fn rocky_and_icy_bodies_come_out_their_size() {
        let tolerance = [
            ("Mercury", 0.03),
            ("Venus", 0.03),
            ("Earth", 0.02),
            ("Mars", 0.03),
            ("Moon", 0.03),
            ("Io", 0.03),
            ("Europa", 0.06),
            ("Ganymede", 0.05),
            ("Callisto", 0.05),
            ("Titan", 0.05),
        ];
        for (name, tol) in tolerance {
            let body = reference(name);
            let r = solid_interior(&inputs_of(body)).radius_earth;
            assert!((r / body.radius_earth - 1.0).abs() < tol, "{name}: {r}");
        }
    }

    #[test]
    fn a_half_water_super_earth_matches_the_published_curve() {
        // 5 M⊕, Earth-like rock, 50 % water: Zeng et al. 2019
        // (doi:10.1073/pnas.1812905116, tables at
        // lweb.cfa.harvard.edu/~lzeng/planetmodels.html) give 1.911 R⊕ at
        // 300 K and 1.981 R⊕ at 700 K; the range spans both curves with
        // 2.5 % on each side.
        let r = solid_interior(&inputs(5.0, 0.325, 0.5)).radius_earth;
        assert!((1.86..2.03).contains(&r), "{r}");
    }

    fn assert_nested(inputs: &PlanetInputs) {
        let interior = solid_interior(inputs);
        let mut last = 0.0;
        let mut mass = 0.0;
        for layer in &interior.layers {
            assert!(layer.mass_earth > 0.0, "{layer:?}");
            assert!(layer.outer_radius_earth > last, "{:?}", interior.layers);
            last = layer.outer_radius_earth;
            mass += layer.mass_earth;
        }
        assert!((mass - inputs.mass_earth).abs() < 1e-9, "{mass}");
        assert_eq!(last, interior.radius_earth);
    }

    #[test]
    fn layers_nest_and_add_up() {
        let ganymede = inputs_of(reference("Ganymede"));
        assert_nested(&ganymede);
        let earth = inputs_of(reference("Earth"));
        assert_nested(&earth);
        let materials = |i: &PlanetInputs| -> Vec<Material> {
            solid_interior(i)
                .layers
                .iter()
                .map(|l| l.material)
                .collect()
        };
        assert_eq!(
            materials(&ganymede),
            [Material::Iron, Material::Rock, Material::Water]
        );
        let dry = inputs(1.0, 0.325, 0.0);
        assert_nested(&dry);
        assert_eq!(materials(&dry), [Material::Iron, Material::Rock]);
        let coreless = inputs(1.0, 0.0, 0.0);
        assert_nested(&coreless);
        assert_eq!(materials(&coreless), [Material::Rock]);
    }

    #[test]
    fn earth_has_its_density_and_gravity() {
        let earth = solid_interior(&inputs_of(reference("Earth")));
        assert!(
            (earth.density / 5.513 - 1.0).abs() < 0.03,
            "{}",
            earth.density
        );
        assert!((earth.gravity - 1.0).abs() < 0.03, "{}", earth.gravity);
    }

    #[test]
    fn radius_is_finite_and_grows_with_mass() {
        for (cmf, water) in [(0.325, 0.0), (0.7, 0.0), (0.3, 0.5), (0.0, 0.0), (1.0, 0.0)] {
            let mut last = 0.0;
            for m in [0.001, 0.01, 0.1, 1.0, 5.0, 20.0] {
                let r = solid_interior(&inputs(m, cmf, water)).radius_earth;
                assert!(r.is_finite() && r > last, "{cmf} {water} {m}: {r}");
                last = r;
            }
        }
    }
}
