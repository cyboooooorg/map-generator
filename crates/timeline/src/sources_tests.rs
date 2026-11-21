use super::*;
use crate::{Code, Provenance, Trace};

fn build(specs: &[(f64, f64, f64)]) -> Vec<StellarSource> {
    let masses = vec![1.0; specs.len()];
    build_with_masses(specs, &masses)
}

fn build_with_masses(specs: &[(f64, f64, f64)], masses: &[f64]) -> Vec<StellarSource> {
    let mut trace = Trace::new();
    specs
        .iter()
        .zip(masses)
        .map(|(&(flux, equilibrium, teff), &mass)| StellarSource {
            flux: trace
                .leaf(Code::of("star.flux"), EarthFlux(flux), Provenance::Input)
                .unwrap(),
            equilibrium: trace
                .leaf(
                    Code::of("star.equilibrium"),
                    Kelvin(equilibrium),
                    Provenance::Input,
                )
                .unwrap(),
            effective_temperature: trace
                .leaf(Code::of("star.teff"), Kelvin(teff), Provenance::Input)
                .unwrap(),
            mass: trace
                .leaf(Code::of("star.mass"), SolarMass(mass), Provenance::Input)
                .unwrap(),
            luminosity: trace
                .leaf(
                    Code::of("star.luminosity"),
                    SolarLuminosity(1.0),
                    Provenance::Input,
                )
                .unwrap(),
            xuv: band(&mut trace, "star.xuv", SUN_XUV, flux),
            lyman_alpha: band(&mut trace, "star.lyman-alpha", SUN_LYMAN_ALPHA, flux),
        })
        .collect()
}

/// The Sun's XUV at 1 au today, W/m², and over Earth's age, J/m²
/// (`science::xuv`).
const SUN_XUV: (f64, f64) = (4.6e-3, 5.64e15);

/// The Sun's Lyman α at 1 au today, W/m² (Ribas et al. 2005), and over
/// Earth's age, J/m² (their eq. 2 integrated, held before 0.1 Gyr).
const SUN_LYMAN_ALPHA: (f64, f64) = (6.4e-3, 2.49e15);

/// A band at one Earth flux `sun` (flux, fluence), scaled by the
/// bolometric flux `flux` (Earth fluxes).
fn band(trace: &mut Trace, code: &'static str, sun: (f64, f64), flux: f64) -> BandLight {
    BandLight {
        flux: trace
            .leaf(
                Code::of(code),
                WattPerSquareMetre(sun.0 * flux),
                Provenance::Input,
            )
            .unwrap(),
        fluence: trace
            .leaf(
                Code::of(code),
                JoulePerSquareMetre(sun.1 * flux),
                Provenance::Input,
            )
            .unwrap(),
    }
}

fn sources(specs: &[(f64, f64, f64)]) -> StellarSources {
    StellarSources::new(build(specs)).unwrap()
}

fn list(n: usize) -> Vec<StellarSource> {
    build(&vec![(1.0, 255.0, 5772.0); n])
}

#[test]
fn one_source_returns_the_primary_bit_for_bit() {
    let s = sources(&[(0.737_1, 236.123_456_789, 5772.0)]);
    assert_eq!(s.total_flux().0.to_bits(), 0.737_1f64.to_bits());
    assert_eq!(s.equilibrium_k().0.to_bits(), 236.123_456_789f64.to_bits());
    assert_eq!(s.weighted(|x| x.effective_temperature.value().0), 5772.0);
    let (flux, fluence) = s.band_total(|x| &x.xuv);
    assert_eq!(flux.0.to_bits(), (SUN_XUV.0 * 0.737_1).to_bits());
    assert_eq!(fluence.0.to_bits(), (SUN_XUV.1 * 0.737_1).to_bits());
}

#[test]
fn band_totals_add_the_stars() {
    let s = sources(&[(0.75, 236.0, 5772.0), (0.25, 180.0, 3500.0)]);
    let (flux, fluence) = s.band_total(|x| &x.lyman_alpha);
    assert!((flux.0 / SUN_LYMAN_ALPHA.0 - 1.0).abs() < 1e-12, "{flux:?}");
    assert!(
        (fluence.0 / SUN_LYMAN_ALPHA.1 - 1.0).abs() < 1e-12,
        "{fluence:?}"
    );
}

#[test]
fn two_half_sources_add_up_to_one() {
    let t = 255.0 / 2f64.powf(0.25);
    let s = sources(&[(0.5, t, 5772.0), (0.5, t, 5772.0)]);
    assert!((s.total_flux().0 - 1.0).abs() < 1e-12);
    assert!((s.equilibrium_k().0 - 255.0).abs() < 1e-9);
}

#[test]
fn weighting_is_by_flux() {
    let s = sources(&[(0.75, 250.0, 6000.0), (0.25, 180.0, 3000.0)]);
    assert!((s.weighted(|x| x.effective_temperature.value().0) - 5250.0).abs() < 1e-9);
}

#[test]
fn the_primary_is_the_first() {
    let s = sources(&[(0.75, 250.0, 6000.0), (0.25, 180.0, 3000.0)]);
    assert_eq!(s.primary().effective_temperature.value().0, 6000.0);
    assert_eq!(s.len(), 2);
    assert_eq!(s.iter().count(), 2);
}

#[test]
fn the_count_is_one_to_three() {
    assert_eq!(
        StellarSources::new(Vec::new()).unwrap_err(),
        ContractError::NoSource
    );
    assert_eq!(StellarSources::new(list(3)).unwrap().len(), 3);
    assert_eq!(
        StellarSources::new(list(4)).unwrap_err(),
        ContractError::TooManySources(4)
    );
}

#[test]
fn serde_round_trips_and_validates_the_count() {
    let s = sources(&[(0.75, 250.0, 6000.0), (0.25, 180.0, 3000.0)]);
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<StellarSources>(&json).unwrap(), s);
    assert!(serde_json::from_str::<StellarSources>("[]").is_err());
    let four = serde_json::to_string(&list(4)).unwrap();
    assert!(serde_json::from_str::<StellarSources>(&four).is_err());
}

#[test]
fn every_node_is_visited_in_order() {
    let mut s = sources(&[(1.0, 255.0, 5772.0), (0.5, 200.0, 4000.0)]);
    let mut count = 0;
    s.for_each_node(&mut |_| count += 1);
    // Five star values and two bands of two values, per star.
    assert_eq!(count, 18);
}

#[test]
fn sources_must_be_in_non_increasing_order_of_mass() {
    let specs = [(0.5, 200.0, 4000.0), (0.5, 200.0, 4000.0)];
    assert_eq!(
        StellarSources::new(build_with_masses(&specs, &[0.5, 1.0])).unwrap_err(),
        ContractError::SourcesNotOrdered
    );
    assert!(StellarSources::new(build_with_masses(&specs, &[1.0, 1.0])).is_ok());
    assert!(StellarSources::new(build_with_masses(&specs, &[1.0, 0.5])).is_ok());
    let json = serde_json::to_string(&build_with_masses(&specs, &[0.5, 1.0])).unwrap();
    assert!(serde_json::from_str::<StellarSources>(&json).is_err());
}
