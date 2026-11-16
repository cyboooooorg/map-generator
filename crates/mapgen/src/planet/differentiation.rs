//! How much of an icy moon's ice separated from its rock
//! ([`differentiated_share`]), the layers that leaves
//! ([`partly_differentiated`]: an undifferentiated rock-and-ice mixture
//! between the rock and the water), and the moment of inertia of a
//! layered body ([`moment_of_inertia`]).
//!
//! A moon whose ice never melted keeps its rock mixed in it: Callisto's
//! moment of inertia, 0.355 (Anderson et al. 2001,
//! doi:10.1006/icar.2001.6664), and Titan's, 0.34 (Iess et al. 2010,
//! doi:10.1126/science.1182583), are too high for a rock core under
//! clean ice, where Ganymede's, 0.311 (Schubert et al. 2004, *Jupiter*,
//! ch. 13), is that of a body differentiated into an iron core, a rock
//! mantle and an ice shell. Two heats decide it here, after Barr & Canup:
//! the share of its accretion heat a moon kept, which melts a share of
//! its ice that grows with its mass over its radius (Barr & Canup 2008,
//! doi:10.1016/j.icarus.2008.07.004), and the impacts of a late
//! bombardment, which run its differentiation away to completion once
//! they deliver enough energy per unit of its mass (Barr & Canup 2010,
//! doi:10.1038/ngeo746, which explains the Ganymede–Callisto dichotomy
//! this way). Neither paper was read beyond its abstract; the forms
//! below are the simplest that carry their arguments.
//!
//! A planet (no host's tides in its [`Context`]) and a body without water
//! are fully differentiated, as before.
use super::{Context, Interior, Layer, Material, PlanetInputs};
use crate::exomoon::{EARTH_KG, EARTH_M, G};

/// Share of a moon's accretion energy, `(3/5) G M / R` per unit mass,
/// kept to melt its ice rather than radiated while it grew. Barr & Canup
/// 2008 (doi:10.1016/j.icarus.2008.07.004) find that Callisto and Titan
/// stayed partly differentiated only if they accreted slowly enough to
/// radiate most of that energy. Calibration, no source, on the moments
/// of inertia of Callisto (0.355) and Titan (0.34): over the asked
/// `callisto-like` and `titan-like` moons of seeds 1 to 10, any value
/// from about 0.044 to 0.074 keeps both in their bands (0.34 to 0.37
/// and 0.33 to 0.36); this is about the geometric middle.
pub const ACCRETION_HEAT_KEPT: f64 = 0.057;

/// Latent heat of fusion of water ice, J/kg: 333.6 kJ/kg at 273 K
/// (Feistel & Wagner 2006, doi:10.1063/1.2183324).
const ICE_FUSION_J_KG: f64 = 333.6e3;

/// Impact energy per unit mass, against the Moon's over the Solar
/// System's age, from which a moon's differentiation runs away to
/// completion: the melt of the impacts lets its rock sink, whose
/// gravitational energy melts the rest (Barr & Canup 2010,
/// doi:10.1038/ngeo746). Calibration on Ganymede versus Callisto: over
/// the asked moons of seeds 1 to 10, the `callisto-like` moons receive
/// 1.80 to 2.05 times the Moon's and the `ganymede-like` ones 2.50 to
/// 2.87; this is their geometric middle. Barr & Canup find a ratio of
/// about 3.5 between them; here it is about 1.3, because the model's
/// [`crate::impacts::giant_moon_flux`] counts the impacts, not their
/// faster speed deeper in the giant's well.
pub const RUNAWAY_IMPACT_ENERGY: f64 = 2.26;

/// The Moon's mass, Earth masses (0.0123, NASA Moon fact sheet), and
/// mean radius, Earth radii (1737.4 km over 6371 km): what
/// [`differentiated_share`]'s impact energy per unit mass is counted
/// against.
const MOON_MASS_EARTH: f64 = 0.0123;
const MOON_RADIUS_EARTH: f64 = 1737.4 / 6371.0;

/// The Solar System's age, Gyr, over which [`crate::impacts`]'s flux
/// against the Moon's is counted (Bouvier & Wadhwa 2010,
/// doi:10.1038/ngeo941).
const SOLAR_AGE_GYR: f64 = 4.57;

/// Share of the Solar System's bombardment, over its 4.57 Gyr, that a
/// system `age_gyr` old has received since it formed: the lunar
/// chronology of Neukum, Ivanov & Hartmann 2001
/// ([`crate::craters::crater_density`]) read from the formation on,
/// `(N(4.57) − N(4.57 − t)) / N(4.57)`. Nearly all of it comes in the
/// first few hundred Myr; an older system counts as 4.57 Gyr old (the
/// constant rate since then adds a few hundredths of a percent per Gyr).
fn bombardment_share(age_gyr: f64) -> f64 {
    let age = age_gyr.clamp(0.0, SOLAR_AGE_GYR);
    let total = crate::craters::crater_density(SOLAR_AGE_GYR);
    (total - crate::craters::crater_density(SOLAR_AGE_GYR - age)) / total
}

/// Share, 0 to 1, of the ice of a moon of `mass_earth` and
/// `radius_earth` that separated from its rock, under an `impact_flux`
/// against the Moon's ([`crate::impacts::giant_moon_flux`]) over
/// `age_gyr`.
///
/// The impact energy per unit mass the moon has received, against the
/// Moon's over the Solar System's age, is its flux times its
/// [`bombardment_share`] times its `R² / M` over the Moon's (impacts per
/// unit area, area over mass). At [`RUNAWAY_IMPACT_ENERGY`] or more the
/// moon differentiates completely (Barr & Canup 2010,
/// doi:10.1038/ngeo746). Below it, the share is what its accretion heat
/// melted: the [`ACCRETION_HEAT_KEPT`] share of `(3/5) G M / R` over the
/// latent heat of ice, up to 1 (Barr & Canup 2008,
/// doi:10.1016/j.icarus.2008.07.004). A body without mass or radius
/// counts as differentiated.
pub fn differentiated_share(
    mass_earth: f64,
    radius_earth: f64,
    impact_flux: f64,
    age_gyr: f64,
) -> f64 {
    if !(mass_earth > 0.0 && radius_earth > 0.0) {
        return 1.0;
    }
    let per_mass = |m: f64, r: f64| r * r / m;
    let impact_energy =
        impact_flux.max(0.0) * bombardment_share(age_gyr) * per_mass(mass_earth, radius_earth)
            / per_mass(MOON_MASS_EARTH, MOON_RADIUS_EARTH);
    if impact_energy >= RUNAWAY_IMPACT_ENERGY {
        return 1.0;
    }
    let accretion_j_kg = 0.6 * G * mass_earth * EARTH_KG / (radius_earth * EARTH_M);
    (ACCRETION_HEAT_KEPT * accretion_j_kg / ICE_FUSION_J_KG).min(1.0)
}

/// The impact flux, against the Moon's, at the orbit of a body of
/// `context` around its host: [`crate::impacts::giant_moon_flux`] of its
/// star, its host's orbit and mass and its own distance from it; `None`
/// for a planet, whose `context` has no host's tides.
pub fn impact_flux(context: &Context) -> Option<f64> {
    let tide = context.tide?;
    Some(crate::impacts::giant_moon_flux(
        context.star_mass,
        context.luminosity,
        context.orbit_au,
        tide.host_kg / EARTH_KG,
        tide.a_m / 1000.0,
    ))
}

/// The [`differentiated_share`] of a body of `inputs` and `interior` in
/// its formation `context`: a moon's from the [`impact_flux`] at its
/// orbit and its system's age; 1 for a planet (no host's tides) and for a
/// body without water.
pub fn share_in(context: &Context, inputs: &PlanetInputs, interior: &Interior) -> f64 {
    match impact_flux(context) {
        Some(flux) if inputs.water_fraction > 0.0 => differentiated_share(
            inputs.mass_earth,
            interior.radius_earth,
            flux,
            context.age_gyr,
        ),
        _ => 1.0,
    }
}

/// `interior` with only the `share` of its ice separated from its rock:
/// the layers under the water (its iron and rock) keep `share` of their
/// mass, the water layer keeps `share` of its own, and the rest, the
/// rock and the ice in the proportions of the whole, is a
/// [`Material::RockIce`] layer between them. Each part keeps its volume,
/// so the inner layers shrink as `share^(1/3)`, the mixture's density is
/// the mixture's, and the radius, density and gravity do not change. The
/// layers above the water (none on an icy moon) stay as they are.
/// Unchanged at a share of 1 or more, or without a water layer.
pub fn partly_differentiated(interior: &Interior, share: f64) -> Interior {
    let water = interior
        .layers
        .iter()
        .position(|l| l.material == Material::Water);
    let Some(water) = water.filter(|&w| w > 0 && share < 1.0) else {
        return interior.clone();
    };
    let share = share.max(0.0);
    let below = &interior.layers[..water];
    let top = &interior.layers[water];
    let base = below.last().map_or(0.0, |l| l.outer_radius_earth);
    let mixed_mass =
        (1.0 - share) * (below.iter().map(|l| l.mass_earth).sum::<f64>() + top.mass_earth);
    let mixed_volume =
        base.powi(3) + (1.0 - share) * (top.outer_radius_earth.powi(3) - base.powi(3));
    let layers = below
        .iter()
        .map(|l| Layer {
            mass_earth: share * l.mass_earth,
            outer_radius_earth: share.cbrt() * l.outer_radius_earth,
            ..*l
        })
        .chain([
            Layer {
                material: Material::RockIce,
                mass_earth: mixed_mass,
                outer_radius_earth: mixed_volume.cbrt(),
            },
            Layer {
                mass_earth: share * top.mass_earth,
                ..*top
            },
        ])
        .chain(interior.layers[water + 1..].iter().cloned())
        .filter(|l| l.mass_earth > 0.0)
        .collect();
    Interior {
        layers,
        ..interior.clone()
    }
}

/// Moment of inertia factor `C / (M R²)` of a body of `interior`, each
/// layer of uniform density: `(2/5) Σ mᵢ (rᵢ⁵ − rᵢ₋₁⁵) / (rᵢ³ − rᵢ₋₁³)
/// / (M R²)`, the sum of the spherical shells. 0.4 for a uniform sphere,
/// less the more the mass sits at the centre (Ganymede 0.311). Layers
/// are uniform: the compression within each, which concentrates a large
/// planet's and above all a giant's mass further, is left out, so it
/// comes out high for them (Earth about 0.354 against 0.3307 measured;
/// Jupiter's is 0.254). 0 for a body without mass.
pub fn moment_of_inertia(interior: &Interior) -> f64 {
    let mut inner = 0.0_f64;
    let (mut sum, mut mass) = (0.0, 0.0);
    for layer in &interior.layers {
        let r = layer.outer_radius_earth;
        let volume = r.powi(3) - inner.powi(3);
        if layer.mass_earth > 0.0 && volume > 0.0 {
            sum += layer.mass_earth * (r.powi(5) - inner.powi(5)) / volume;
            mass += layer.mass_earth;
        }
        inner = r;
    }
    if mass > 0.0 && inner > 0.0 {
        0.4 * sum / (mass * inner * inner)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::reference::{inputs_of, reference};
    use crate::planet::solid_interior;

    #[test]
    fn a_uniform_sphere_is_two_fifths() {
        let interior =
            partly_differentiated(&solid_interior(&inputs_of(reference("Callisto"))), 0.0);
        assert_eq!(interior.layers.len(), 1);
        assert_eq!(interior.layers[0].material, Material::RockIce);
        assert!((moment_of_inertia(&interior) - 0.4).abs() < 1e-9);
    }

    #[test]
    fn earth_concentrates_its_mass() {
        let earth = solid_interior(&inputs_of(reference("Earth")));
        let c = moment_of_inertia(&earth);
        // Earth's is 0.3307; uniform layers, uncompressed within, give
        // about 0.354.
        assert!((0.33..0.36).contains(&c), "{c}");
    }

    #[test]
    fn a_partly_differentiated_body_keeps_its_mass_and_size() {
        let full = solid_interior(&inputs_of(reference("Ganymede")));
        let part = partly_differentiated(&full, 0.4);
        let mass = |i: &Interior| i.layers.iter().map(|l| l.mass_earth).sum::<f64>();
        assert!((mass(&part) - mass(&full)).abs() < 1e-15);
        assert_eq!(part.radius_earth, full.radius_earth);
        assert_eq!(
            part.layers.last().unwrap().outer_radius_earth,
            full.radius_earth
        );
        let materials: Vec<Material> = part.layers.iter().map(|l| l.material).collect();
        assert_eq!(
            materials,
            [
                Material::Iron,
                Material::Rock,
                Material::RockIce,
                Material::Water
            ]
        );
        // The less separated, the higher the moment of inertia.
        let c = |s: f64| moment_of_inertia(&partly_differentiated(&full, s));
        assert!(
            c(0.0) > c(0.4) && c(0.4) > c(1.0),
            "{} {} {}",
            c(0.0),
            c(0.4),
            c(1.0)
        );
        assert_eq!(partly_differentiated(&full, 1.0), full);
    }

    #[test]
    fn impacts_run_ganymede_away_and_leave_callisto_partly_differentiated() {
        // The real pair: Jupiter's 317.8 Earth masses at 5.2 AU round the
        // Sun, Ganymede at 1 070 400 km, Callisto at 1 882 700 km.
        let flux = |a_km: f64| crate::impacts::giant_moon_flux(1.0, 1.0, 5.2, 317.8, a_km);
        let ganymede = reference("Ganymede");
        let callisto = reference("Callisto");
        let share = |b: &crate::planet::ReferenceBody, a_km: f64| {
            differentiated_share(b.mass_earth, b.radius_earth, flux(a_km), SOLAR_AGE_GYR)
        };
        assert_eq!(share(ganymede, 1.0704e6), 1.0);
        let c = share(callisto, 1.8827e6);
        assert!((0.2..0.45).contains(&c), "{c}");
    }

    #[test]
    fn a_young_system_has_had_less_of_its_bombardment() {
        assert_eq!(bombardment_share(SOLAR_AGE_GYR), 1.0);
        assert_eq!(bombardment_share(10.0), 1.0);
        assert_eq!(bombardment_share(0.0), 0.0);
        let young = bombardment_share(0.1);
        assert!(young > 0.0 && young < 0.9, "{young}");
    }

    #[test]
    fn a_body_without_mass_or_water_is_differentiated() {
        assert_eq!(differentiated_share(0.0, 0.3, 1.0, 4.57), 1.0);
        assert_eq!(differentiated_share(f64::NAN, 0.3, 1.0, 4.57), 1.0);
        let io = solid_interior(&inputs_of(reference("Io")));
        assert_eq!(partly_differentiated(&io, 0.2), io);
    }
}
