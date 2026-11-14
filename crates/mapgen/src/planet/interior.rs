//! The rocky and icy interior: iron core, rock mantle and water shell,
//! their radii, and the radius, bulk density and surface gravity they give
//! (spec "Derived, in order", item 1), then the hydrogen and helium
//! envelope on top of them ([`interior`]).
//!
//! The refractory part (iron and rock) follows the mass–radius relation of
//! Zeng, Sasselov & Jacobsen (2016) where it applies, and its uncompressed
//! size below it; the water shell is compressed along with it. Units:
//! masses in Earth masses, radii in Earth radii, densities relative to
//! Earth's, so that a sphere of mass `M` and relative density `ρ` has
//! `R³ = M / ρ`.
use super::{Exposure, Interior, Layer, Material, PlanetInputs};
use crate::world::UNCOMPRESSED_ROCK_DENSITY;
use std::sync::LazyLock;

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

/// Least mass of a giant, Earth masses: from about 10 M⊕ a core accretes
/// gas faster than solids and runs away into a giant (Pollack et al. 1996,
/// doi:10.1006/icar.1996.0190). With [`GIANT_MIN_ENVELOPE`], the line
/// between a planet with an envelope and a giant; the classifier uses it
/// too.
pub const GIANT_MIN_MASS_EARTH: f64 = 10.0;

/// Least hydrogen and helium share of a giant. Calibration, no source: it
/// keeps Uranus and Neptune (about 10 to 15 %) giants and the sub-Neptunes
/// of a few percent on the envelope fit of Lopez & Fortney (2014).
pub const GIANT_MIN_ENVELOPE: f64 = 0.05;

/// Thickness of a hydrogen and helium envelope of solar make on a rocky
/// core, Earth radii, from the planet's mass (Earth masses), the
/// envelope's share of it, the insolation (Earth's = 1) and the age (Gyr).
///
/// Lopez & Fortney (2014, doi:10.1088/0004-637X/792/1/1), their fit:
/// `R_env = 2.06 R⊕ M^−0.21 (f/5 %)^0.59 S^0.044 (age/5 Gyr)^−0.11`, with
/// the age exponent of their solar-metallicity models (the printed fit,
/// made on enhanced-opacity models, has −0.18). The fit is within about
/// 0.1 dex of their full models. The mass, insolation and age are clamped
/// to the grid it was fitted on (1–20 M⊕, 0.1–1000 S⊕, 0.1–10 Gyr). The
/// envelope share is extrapolated below the grid's smallest envelope
/// (0.01 % of the mass), so that the envelope thins to nothing as its
/// share goes to zero instead of jumping at `f = 0`, and above its largest
/// (20 %) up to the largest share the model draws under 30 M⊕
/// ([`ENVELOPE_MAX_SMALL`], 30 %), so that more gas makes a larger planet
/// over every drawn share; a larger share, which only a request asks for,
/// is sized as 30 % (a giant from [`GAS_GIANT_MIN_ENVELOPE`] takes the gas
/// giants' law instead, see [`crate::giants::giant_radius_of`]). The thin
/// radiative atmosphere above it (their `R_atm`, about 0.1 R⊕) is left
/// out.
///
/// [`ENVELOPE_MAX_SMALL`]: super::inputs::ENVELOPE_MAX_SMALL
/// [`GAS_GIANT_MIN_ENVELOPE`]: super::classify::GAS_GIANT_MIN_ENVELOPE
fn lopez_fortney_thickness(mass_earth: f64, envelope: f64, insolation: f64, age_gyr: f64) -> f64 {
    let m = mass_earth.clamp(1.0, 20.0);
    let f = envelope.clamp(0.0, super::inputs::ENVELOPE_MAX_SMALL);
    let s = insolation.clamp(0.1, 1000.0);
    let age = age_gyr.clamp(0.1, 10.0);
    2.06 * m.powf(-0.21) * (f / 0.05).powf(0.59) * s.powf(0.044) * (age / 5.0).powf(-0.11)
}

/// Light Neptune gets, Earth's = 1 (`1 / 30.07²`): under the grid of
/// [`lopez_fortney_thickness`], which takes it as 0.1.
const NEPTUNE_INSOLATION: f64 = 1.0 / (30.07 * 30.07);

/// How much thinner than [`lopez_fortney_thickness`] Neptune's envelope
/// is: what puts its composition ([`GiantComposition::NEPTUNE`]) at
/// Neptune's mass, light and age at Neptune's radius
/// ([`crate::giants::NEPTUNE_RADIUS_EARTH`]). About 0.49: its solid
/// layers come out 2.91 R⊕, the fit gives an envelope of 1.98 R⊕ (Neptune
/// would come out 4.89 R⊕, 27 % too large, and Uranus 22 %), and 0.97 is
/// left for it.
///
/// [`GiantComposition::NEPTUNE`]: crate::giants::GiantComposition::NEPTUNE
static NEPTUNE_ENVELOPE_SHARE: LazyLock<f64> = LazyLock::new(|| {
    let neptune = crate::giants::GiantComposition::NEPTUNE;
    let mass = crate::giants::NEPTUNE_MASS_EARTH;
    let solid = solid_interior(&neptune.inputs(mass)).radius_earth;
    let fit = lopez_fortney_thickness(
        mass,
        neptune.envelope_fraction,
        NEPTUNE_INSOLATION,
        f64::from(crate::star::DEFAULT_AGE_GYR),
    );
    (crate::giants::NEPTUNE_RADIUS_EARTH - solid) / fit
});

/// Thickness of a planet's hydrogen and helium envelope, Earth radii:
/// [`lopez_fortney_thickness`] for its mass, envelope share, insolation
/// and age, times a factor of its water share `water` that runs linearly
/// from 1 for a dry planet to [`NEPTUNE_ENVELOPE_SHARE`] (about 0.49) at
/// Neptune's 65 %.
///
/// The fit of Lopez & Fortney is made on envelopes of pure hydrogen and
/// helium over rocky cores. A water-rich planet mixes heavy elements into
/// its envelope (Uranus's and Neptune's hold them up to their outer
/// layers: Helled et al. 2011, doi:10.1088/0004-637X/726/1/15; Nettelmann
/// et al. 2013, doi:10.1016/j.pss.2012.06.019), which makes it denser and
/// thinner. The linear factor is a calibration, no source, set on Neptune
/// alone: Uranus, with Neptune's composition and 15 % less mass, comes out
/// 3.80 R⊕ (3.98 observed, −4.5 %).
pub(crate) fn envelope_thickness(
    mass_earth: f64,
    envelope: f64,
    water: f64,
    exposure: &Exposure,
) -> f64 {
    let neptune = crate::giants::GiantComposition::NEPTUNE;
    let wet = (water / neptune.water_fraction).clamp(0.0, 1.0 / neptune.water_fraction);
    let factor = 1.0 - (1.0 - *NEPTUNE_ENVELOPE_SHARE) * wet;
    lopez_fortney_thickness(mass_earth, envelope, exposure.insolation, exposure.age_gyr) * factor
}

/// The layers of a planet with its hydrogen and helium envelope, its
/// radius, bulk density and surface gravity, under its star's `exposure`.
///
/// Without an envelope, the [`solid_interior`]. A giant (at least
/// [`GIANT_MIN_MASS_EARTH`] and [`GIANT_MIN_ENVELOPE`]) takes the radius
/// of [`crate::giants::giant_radius_of`]: under 30 M⊕ the same solid layers
/// and [`envelope_thickness`] as any other planet, so that the radius does
/// not jump across those lines, then the giant law of the gas giants. The
/// solid layers stay inside it and the envelope fills the rest. Any other
/// planet gets the envelope of [`envelope_thickness`] on top of its solid
/// part. Density and gravity count the whole mass.
pub fn interior(inputs: &PlanetInputs, exposure: &Exposure) -> Interior {
    let solid = solid_interior(inputs);
    let mass = inputs.mass_earth;
    let envelope = mass * inputs.envelope_fraction;
    if envelope <= 0.0 {
        return solid;
    }
    let radius_earth =
        if mass >= GIANT_MIN_MASS_EARTH && inputs.envelope_fraction >= GIANT_MIN_ENVELOPE {
            f64::from(crate::giants::giant_radius_of(
                mass as f32,
                &crate::giants::GiantComposition::of(inputs),
                exposure,
            ))
            .max(solid.radius_earth)
        } else {
            solid.radius_earth
                + envelope_thickness(
                    mass,
                    inputs.envelope_fraction,
                    inputs.water_fraction,
                    exposure,
                )
        };
    let mut layers = solid.layers;
    layers.push(Layer {
        material: Material::Envelope,
        mass_earth: envelope,
        outer_radius_earth: radius_earth,
    });
    Interior {
        layers,
        radius_earth,
        density: EARTH_DENSITY * mass / radius_earth.powi(3),
        gravity: mass / radius_earth.powi(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::Exposure;
    use crate::planet::reference::{exposure_at, inputs, inputs_of, reference, with_envelope};

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

    fn assert_giant_size(name: &str, tol: f64) {
        let body = reference(name);
        let r = interior(&inputs_of(body), &exposure_at(body.orbit_au)).radius_earth;
        assert!((r / body.radius_earth - 1.0).abs() < tol, "{name}: {r}");
    }

    #[test]
    fn giants_come_out_their_size() {
        for (name, tol) in [("Jupiter", 0.05), ("Saturn", 0.06), ("Neptune", 0.08)] {
            assert_giant_size(name, tol);
        }
    }

    #[test]
    fn uranus_comes_out_its_size() {
        assert_giant_size("Uranus", 0.08);
    }

    /// Inputs of `mass_earth` with a `water` and an `envelope` share,
    /// Earth-like rock.
    fn made_of(mass_earth: f64, water: f64, envelope: f64) -> PlanetInputs {
        PlanetInputs {
            envelope_fraction: envelope,
            ..inputs(mass_earth, 0.325, water)
        }
    }

    fn lit(insolation: f64) -> Exposure {
        Exposure {
            insolation,
            equilibrium_k: 278.0 * insolation.powf(0.25),
            age_gyr: 4.6,
        }
    }

    #[test]
    fn the_radius_is_continuous_across_the_giant_lines() {
        // The 5 % gas line at 12 M⊕ and the 10 M⊕ line, for dry, wet and
        // Neptune-like planets, cold and hot: no step of more than 1 %.
        for water in [0.0, 0.35, 0.6] {
            for insolation in [0.01, 1.0, 100.0, 1000.0] {
                let at =
                    |m: f64, f: f64| interior(&made_of(m, water, f), &lit(insolation)).radius_earth;
                let (below, above) = (at(12.0, 0.0499), at(12.0, 0.0501));
                assert!(
                    (above / below - 1.0).abs() < 0.01,
                    "5 % line, water {water}, S {insolation}: {below} then {above}"
                );
                for f in [0.06, 0.15, 0.3] {
                    let (below, above) = (at(9.999, f), at(10.001, f));
                    assert!(
                        (above / below - 1.0).abs() < 0.01,
                        "10 M⊕ line, water {water}, gas {f}, S {insolation}: \
                         {below} then {above}"
                    );
                }
            }
        }
    }

    #[test]
    fn more_gas_makes_a_larger_planet() {
        for water in [0.0, 0.35, 0.6] {
            for m in [5.0, 12.0, 20.0, 25.0] {
                for insolation in [0.01, 100.0] {
                    let mut last = 0.0;
                    for f in [0.001, 0.01, 0.03, 0.049, 0.051, 0.1, 0.2, 0.3] {
                        let r = interior(&made_of(m, water, f), &lit(insolation)).radius_earth;
                        assert!(
                            r > last,
                            "{m} M⊕, water {water}, gas {f}, S {insolation}: {r}"
                        );
                        last = r;
                    }
                }
            }
        }
    }

    #[test]
    fn a_giant_of_fixed_composition_barely_grows_up_to_thirty_earth_masses() {
        // A dry planet with 6 % gas and a Neptune-like one: from Neptune's
        // mass to the gas giants' line the radius grows slower than
        // `M^0.35` (a solid body's `M^0.27`, an envelope that thins).
        for (water, gas) in [(0.0, 0.06), (0.6, 0.15)] {
            for insolation in [0.01, 100.0] {
                let at = |m: f64| interior(&made_of(m, water, gas), &lit(insolation)).radius_earth;
                let growth = at(29.9) / at(17.6);
                assert!(
                    growth < (29.9f64 / 17.6).powf(0.35),
                    "water {water}, gas {gas}, S {insolation}: {} then {}",
                    at(17.6),
                    at(29.9)
                );
            }
        }
    }

    #[test]
    fn a_requested_gas_giant_under_thirty_earth_masses_is_sized_as_one() {
        // `jupiter-like` asked at 20 M⊕ keeps about 95 % gas: a gas giant
        // by its class, sized by the gas giants' law, whose radius never
        // falls with the mass across 30 M⊕.
        for insolation in [0.01, 100.0] {
            let exposure = lit(insolation);
            let at = |m: f64| interior(&made_of(m, 0.0, 0.95), &exposure).radius_earth;
            let law = f64::from(crate::giants::gas_giant_radius(
                20.0,
                exposure.equilibrium_k as f32,
            ));
            assert!(
                (at(20.0) - law).abs() < 1e-6,
                "S {insolation}: {}",
                at(20.0)
            );
            let mut last = 0.0;
            for m in [10.0, 12.0, 17.15, 20.0, 25.0, 29.9, 30.0, 31.0, 40.0] {
                let r = at(m);
                assert!(r >= last, "S {insolation}, {m} M⊕: {r} after {last}");
                last = r;
            }
        }
    }

    #[test]
    fn the_envelope_fit_is_not_extrapolated_past_the_drawn_shares() {
        // Under the gas giants' class line, a requested share above the
        // 30 % the model draws is sized as 30 %.
        for (m, water) in [(1.0, 0.0), (5.0, 0.3), (12.0, 0.0), (25.0, 0.3)] {
            let at = |f: f64| interior(&made_of(m, water, f), &lit(1.0)).radius_earth;
            assert!(at(0.45) <= at(0.3) + 1e-9, "{m}: {} {}", at(0.3), at(0.45));
        }
        let puff = interior(&made_of(1.0, 0.0, 0.999), &lit(1.0));
        assert!(puff.radius_earth < 7.0, "{}", puff.radius_earth);
    }

    #[test]
    fn a_few_percent_envelope_puts_a_planet_above_the_radius_valley() {
        // Lopez & Fortney 2014 (doi:10.1088/0004-637X/792/1/1), eq. 3 with
        // the solar-metallicity age term: 5 M⊕, 2 % H/He, 10 S⊕, 5 Gyr give
        // an envelope of 0.95 R⊕ on a 1.54 R⊕ core, 2.49 R⊕ in all.
        let exposure = Exposure {
            insolation: 10.0,
            equilibrium_k: 500.0,
            age_gyr: 5.0,
        };
        let r = interior(&with_envelope(5.0, 0.02), &exposure).radius_earth;
        assert!((2.1..2.8).contains(&r), "{r}");
    }

    #[test]
    fn envelope_radius_vanishes_with_the_envelope() {
        let exposure = Exposure {
            insolation: 10.0,
            equilibrium_k: 500.0,
            age_gyr: 5.0,
        };
        let radius = |f: f64| interior(&with_envelope(5.0, f), &exposure).radius_earth;
        let solid = solid_interior(&with_envelope(5.0, 1e-6)).radius_earth;
        assert!((radius(1e-6) - solid).abs() < 0.01, "{}", radius(1e-6));
        let mut last = 0.0;
        for f in [0.0, 1e-6, 1e-4, 1e-3, 1e-2, 0.1] {
            let r = radius(f);
            assert!(r >= last, "{f}: {r} < {last}");
            last = r;
        }
    }

    #[test]
    fn envelope_radius_is_finite_outside_its_fit() {
        for insolation in [1e-3, 1.0, 1e4] {
            for age_gyr in [0.01, 5.0, 13.0] {
                for (m, f) in [(0.5, 0.001), (9.9, 0.3), (12.0, 0.04), (4000.0, 0.97)] {
                    let exposure = Exposure {
                        insolation,
                        equilibrium_k: 278.0 * insolation.powf(0.25),
                        age_gyr,
                    };
                    let r = interior(&with_envelope(m, f), &exposure).radius_earth;
                    assert!(
                        r.is_finite() && r > 0.0,
                        "{m} {f} {insolation} {age_gyr}: {r}"
                    );
                }
            }
        }
    }

    #[test]
    fn no_envelope_means_the_solid_interior() {
        let earth = inputs_of(reference("Earth"));
        assert_eq!(interior(&earth, &exposure_at(1.0)), solid_interior(&earth));
    }

    #[test]
    fn the_envelope_is_the_outer_layer_and_holds_its_mass() {
        for (m, f, orbit) in [(5.0, 0.02, 0.3), (317.8, 0.95, 5.2), (14.54, 0.15, 19.2)] {
            let planet = with_envelope(m, f);
            let interior = interior(&planet, &exposure_at(orbit));
            let mut last = 0.0;
            let mut mass = 0.0;
            for layer in &interior.layers {
                assert!(layer.outer_radius_earth > last, "{:?}", interior.layers);
                last = layer.outer_radius_earth;
                mass += layer.mass_earth;
            }
            let envelope = interior.layers.last().unwrap();
            assert_eq!(envelope.material, Material::Envelope);
            assert!((envelope.mass_earth - m * f).abs() < 1e-9);
            assert!((mass - m).abs() < 1e-9, "{mass}");
            assert_eq!(last, interior.radius_earth);
            let density = EARTH_DENSITY * m / last.powi(3);
            assert!((interior.density - density).abs() < 1e-9);
        }
    }
}
