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

/// Coefficients of the fit of the isothermal temperature of a runaway
/// steam envelope, Turbet et al. 2020 (A&A 638, A41,
/// doi:10.1051/0004-6361/201937151, arXiv:1911.08878), their Table 2:
/// `α₁…α₆` centre and scale the inputs, `β₁…β₁₀` weigh the terms of
/// their Eq. 3 ([`steam_temperature_k`]).
const TURBET_ALPHA: [f64; 6] = [-3.550, 1.310, 1.099, 4.683e-1, 7.664e-1, 4.224e-1];
const TURBET_BETA: [f64; 10] = [
    2.846, 1.555e-1, 8.777e-2, 6.045e-2, 1.143e-2, 1.736e-2, 1.859e-2, 4.314e-2, 3.393e-2,
    -1.034e-2,
];

/// Pressure of the transit radius over a steam envelope, Pa: about
/// 0.1 Pa over the whole grid of Turbet et al. 2020 (§2.2).
const TURBET_TRANSIT_PA: f64 = 0.1;

/// Range of the fit of Turbet et al. 2020: irradiation 1 to 30 S⊕ and
/// surface gravity 0.2 to 6 g⊕ (§2.2, where Eq. 3 is "valid"), and the
/// steam pressures of the atmospheric structures the fit was made on,
/// 27 bar to 270 kbar (§2.1; in Pa here). §2.2 states Eq. 3 valid to
/// 27 kbar only, but clamped there it takes a steam world's envelope
/// below the same water condensed (the #492 body at 1.61 R⊕ against
/// 1.64), against the paper's own finding; up to the simulated 270 kbar
/// the polynomial is still on its grid.
const TURBET_INSOLATION: (f64, f64) = (1.0, 30.0);
const TURBET_GRAVITY_EARTH: (f64, f64) = (0.2, 6.0);
const TURBET_STEAM_PA: (f64, f64) = (2.7e6, 2.7e10);

/// Standard gravity, m/s² (CGPM 1901), the `g⊕` of Turbet et al.'s range.
const EARTH_GRAVITY_M_S2: f64 = 9.806_65;

/// Molar gas constant, J/(mol·K) (CODATA 2018, exact), and the molar mass
/// of water, kg/mol (18.015 g/mol).
const GAS_CONSTANT: f64 = 8.314_462_618;
const WATER_MOLAR_KG: f64 = 18.015e-3;

/// Temperature of the isothermal steam envelope of a runaway water-rich
/// planet above its photosphere, K: Eq. 3 and Table 2 of Turbet et al.
/// 2020 (doi:10.1051/0004-6361/201937151), `log₁₀ T = β₁ + β₂x + β₃y +
/// β₄z + β₅xy + β₆y² + β₇x³ + β₈x²y + β₉xy² + β₁₀y⁴` with `x = (log₁₀ f −
/// α₁)/α₂` of the water mass fraction `f`, `y = (log₁₀ g − α₃)/α₄` of the
/// gravity at the core in m/s², `z = (log₁₀ S − α₅)/α₆` of the
/// irradiation in S⊕.
///
/// The fit is a polynomial: outside its grid it runs off (its `x³`
/// term grows fast with the water share). Its inputs are clamped to the
/// grid ([`TURBET_STEAM_PA`]): `S` to 1–30 S⊕, `g` to 0.2–6 g⊕, and the
/// water share to the shares whose steam weighs 27 bar to 270 kbar at
/// that gravity (`P = f/(1−f) g²/(4πG)`, their Eq. 2). A steam world
/// deeper than that takes the temperature of the grid's deepest steam.
fn steam_temperature_k(water_share: f64, core_gravity_m_s2: f64, insolation: f64) -> f64 {
    let g = core_gravity_m_s2.clamp(
        TURBET_GRAVITY_EARTH.0 * EARTH_GRAVITY_M_S2,
        TURBET_GRAVITY_EARTH.1 * EARTH_GRAVITY_M_S2,
    );
    // Water share at which the steam weighs `p` at gravity `g`.
    let share_at = |p: f64| {
        let ratio = p * 4.0 * std::f64::consts::PI * crate::exomoon::G / (g * g);
        ratio / (1.0 + ratio)
    };
    let f = water_share.clamp(share_at(TURBET_STEAM_PA.0), share_at(TURBET_STEAM_PA.1));
    let s = insolation.clamp(TURBET_INSOLATION.0, TURBET_INSOLATION.1);
    let [a1, a2, a3, a4, a5, a6] = TURBET_ALPHA;
    let b = TURBET_BETA;
    let x = (f.log10() - a1) / a2;
    let y = (g.log10() - a3) / a4;
    let z = (s.log10() - a5) / a6;
    let log_t = b[0]
        + b[1] * x
        + b[2] * y
        + b[3] * z
        + b[4] * x * y
        + b[5] * y * y
        + b[6] * x.powi(3)
        + b[7] * x * x * y
        + b[8] * x * y * y
        + b[9] * y.powi(4);
    10f64.powf(log_t)
}

/// Thickness of the steam envelope of a runaway water-rich planet, Earth
/// radii, up to its transit radius, on a core of `core_mass_earth` and
/// `core_radius_earth` holding `steam_earth` of water as steam, under
/// `insolation` S⊕.
///
/// Turbet et al. 2020 (doi:10.1051/0004-6361/201937151, arXiv:1911.08878),
/// their Eq. 2 in the form of their Appendix C (an isothermal envelope in
/// hydrostatic balance under a gravity falling as `1/r²`; the printed
/// Eq. 2 has `RT/(Mg)` upside down, which Appendix C settles):
/// `1/z = M g / (R T ln(P_surf / P_transit)) − 1/R_core`, with `P_surf =
/// f/(1−f) g²/(4πG)` the weight of the steam, `f` the water mass
/// fraction, `g` the gravity at the core, `M` the molar mass of water,
/// `P_transit` about 0.1 Pa ([`TURBET_TRANSIT_PA`]) and `T` of their
/// Eq. 3 ([`steam_temperature_k`], clamped to its grid). Eq. 2 itself is
/// hydrostatics and takes the planet's own share, gravity and radius
/// unclamped.
///
/// An envelope so hot or so light that `1/z` reaches 0 is not bound in
/// this isothermal form (the paper warns it does not hold for inflated
/// atmospheres); the thickness is capped at the core's radius, which no
/// steam world of the census approaches. Calibration, no source.
fn turbet_thickness(
    core_mass_earth: f64,
    core_radius_earth: f64,
    steam_earth: f64,
    insolation: f64,
) -> f64 {
    use crate::exomoon::{EARTH_KG, EARTH_M, G};
    if steam_earth <= 0.0 || core_mass_earth <= 0.0 || core_radius_earth <= 0.0 {
        return 0.0;
    }
    let share = steam_earth / (core_mass_earth + steam_earth);
    let core_m = core_radius_earth * EARTH_M;
    let g = G * core_mass_earth * EARTH_KG / (core_m * core_m);
    let surface_pa = share / (1.0 - share) * g * g / (4.0 * std::f64::consts::PI * G);
    let depth = (surface_pa / TURBET_TRANSIT_PA).ln();
    if depth <= 0.0 {
        return 0.0;
    }
    let t = steam_temperature_k(share, g, insolation);
    let inverse = WATER_MOLAR_KG * g / (GAS_CONSTANT * t * depth) - 1.0 / core_m;
    let cap = core_m;
    let z = if inverse * cap <= 1.0 {
        cap
    } else {
        1.0 / inverse
    };
    z / EARTH_M
}

/// The interior of a steam world (#492): the `solid` layers with their
/// water turned to a supercritical steam envelope of `steam_kg` on top of
/// the rock, the radius raised by its thickness ([`turbet_thickness`],
/// after Turbet et al. 2020, doi:10.1051/0004-6361/201937151, for the
/// refractory layers as the core, the steam's share of core and steam as
/// the water fraction, and the `exposure`'s insolation), the density and
/// gravity of the whole `mass_earth` at that radius.
///
/// The water layer of `solid` (the drawn water, condensed) gives way to
/// the steam, which is the water the runaway kept: what escape took is
/// gone. Any layer above the water (a trace of hydrogen) moves out by the
/// change of radius. Without steam, the `solid` interior as it is.
pub fn steam_interior(
    solid: &Interior,
    mass_earth: f64,
    steam_kg: f64,
    exposure: &Exposure,
) -> Interior {
    let steam_earth = steam_kg / crate::exomoon::EARTH_KG;
    if steam_earth <= 0.0 {
        return solid.clone();
    }
    let water = solid
        .layers
        .iter()
        .position(|l| l.material == Material::Water);
    let below = water.unwrap_or_else(|| {
        solid
            .layers
            .iter()
            .position(|l| l.material == Material::Envelope)
            .unwrap_or(solid.layers.len())
    });
    let core = &solid.layers[..below];
    let core_mass: f64 = core.iter().map(|l| l.mass_earth).sum();
    let core_radius = core.last().map_or(0.0, |l| l.outer_radius_earth);
    let top_before = if below == 0 {
        0.0
    } else {
        solid.layers[water.unwrap_or(below - 1)].outer_radius_earth
    };
    let steam_radius =
        core_radius + turbet_thickness(core_mass, core_radius, steam_earth, exposure.insolation);
    let shift = steam_radius - top_before;
    let mut layers = core.to_vec();
    layers.push(Layer {
        material: Material::Steam,
        mass_earth: steam_earth,
        outer_radius_earth: steam_radius,
    });
    let above = water.map_or(below, |w| w + 1);
    layers.extend(solid.layers[above..].iter().map(|l| Layer {
        outer_radius_earth: l.outer_radius_earth + shift,
        ..*l
    }));
    let radius_earth = layers.last().map_or(0.0, |l| l.outer_radius_earth);
    Interior {
        layers,
        radius_earth,
        density: EARTH_DENSITY * mass_earth / radius_earth.powi(3),
        gravity: mass_earth / radius_earth.powi(2),
    }
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
    use crate::exomoon::EARTH_KG;
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
            // Its ice is ice Ih, lighter than the high-pressure ices
            // `WATER_DENSITY` mixes in: 0.0368 R⊕ against 0.0395.
            ("Enceladus", 0.08),
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
    fn steam_inflates_the_radius_with_the_water_share() {
        // A 5 M⊕ rock with Earth's core under 10 S⊕: no steam leaves the
        // solid as it is, more steam makes a larger planet, and the steam
        // is the outer layer holding its mass.
        let solid = solid_interior(&inputs(5.0, 0.325, 0.0));
        let exposure = lit(10.0);
        assert_eq!(steam_interior(&solid, 5.0, 0.0, &exposure), solid);
        let mut last = solid.radius_earth;
        for share in [1e-5, 1e-4, 1e-3, 0.01, 0.05, 0.1, 0.2] {
            let steam_earth = 5.0 * share / (1.0 - share);
            let mass = 5.0 + steam_earth;
            let steamy = steam_interior(&solid, mass, steam_earth * EARTH_KG, &exposure);
            let r = steamy.radius_earth;
            assert!(r.is_finite() && r > last, "{share}: {r} after {last}");
            last = r;
            let top = steamy.layers.last().unwrap();
            assert_eq!(top.material, Material::Steam);
            assert!((top.mass_earth - steam_earth).abs() < 1e-9 * mass);
            assert_eq!(top.outer_radius_earth, r);
            assert!((steamy.gravity - mass / (r * r)).abs() < 1e-12);
        }
    }

    #[test]
    fn steam_replaces_the_condensed_water_layer() {
        // The water layer of the solid interior turns into the steam: the
        // envelope sits on the rock, not on a condensed ocean.
        let solid = solid_interior(&inputs(4.78, 0.25, 0.12));
        let water = solid.layers.last().unwrap();
        assert_eq!(water.material, Material::Water);
        let steamy = steam_interior(&solid, 4.78, water.mass_earth * EARTH_KG, &lit(5.0));
        let materials: Vec<Material> = steamy.layers.iter().map(|l| l.material).collect();
        assert_eq!(materials, [Material::Iron, Material::Rock, Material::Steam]);
        let mass: f64 = steamy.layers.iter().map(|l| l.mass_earth).sum();
        assert!((mass - 4.78).abs() < 1e-9, "{mass}");
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
