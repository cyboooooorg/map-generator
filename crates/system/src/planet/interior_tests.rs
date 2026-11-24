//! Tier 1: the steam envelope of a steam world, on the composition of
//! issue #492 and on a boiled Earth.

use super::codes;
use super::draws::Drawn;
use super::interior::{Interior, Water, rocky, steam_interior};
use super::tests::{EARTH, ancestry, derived_from, reference_trial, water_world_trial};
use super::water::surface_water;
use crate::Trial;
use science::constants::GRAVITATIONAL_CONSTANT;
use science::units::{EarthMass, EarthRadius, Kilogram, Metre};
use timeline::{Code, Material, NodeId, Provenance, Trace, Traced};

/// Definition: the mass of the body of issue #492, 4.78 M_E.
pub(super) const MASS_492: f64 = 4.78;

/// Definition: its core's share of the refractory mass, 1.06 / 4.22 (issue
/// #492's composition).
pub(super) const CORE_492: f64 = 1.06 / 4.22;

/// Definition: its water's share of the mass, 0.56 / 4.78 (issue #492's
/// composition).
pub(super) const WATER_492: f64 = 0.56 / 4.78;

/// Definition: its distance from the Sun, 1 au, under which its water is a
/// layer (below 355 K; a test choice).
pub(super) const AU_492: f64 = 1.0;

/// Definition: a body inside the structures of Turbet et al. 2020 § 2.1
/// (steam weighing 27 bar to 270 kbar), past § 2.2's 27 kbar, and inside
/// § 2.2's 1 to 30 S_E and 0.2 to 6 g_E: 5 M_E with 3 %
/// water at 1 au (1 S_E, its lower edge). Its water is a layer, which a
/// condensed twin needs; at 10 S_E its water would be beyond the laws and
/// the rewrite's interior builds no twin. Its steam weighs about 10^5 bar
/// at the base. A test choice.
const GRID: (f64, f64, f64) = (5.0, 0.325, 0.03);

/// Definition: the relative tolerance of a sum of masses, 1e-9.
const SUM: f64 = 1e-9;

/// The rock of the #492 body, its trial and the leaf that says its steam
/// hides the surface.
fn body_492(trace: &mut Trace) -> (Trial, Interior, Traced<bool>) {
    let (t, d) = water_world_trial(trace, MASS_492, CORE_492, WATER_492, AU_492);
    let rock = rocky(&t.mass, &d, &t, trace).unwrap();
    let hides = trace
        .leaf(Code::of("test.hides"), true, Provenance::Input)
        .unwrap();
    (t, rock, hides)
}

fn layer_mass(rock: &Interior) -> Traced<Kilogram> {
    match rock.water {
        Water::Layer { mass, .. } => mass,
        other => panic!("{other:?}"),
    }
}

fn kilograms(mass: &Traced<EarthMass>) -> f64 {
    Kilogram::from(*mass.value()).0
}

fn shell_sum(interior: &Interior) -> f64 {
    interior.shells.iter().map(|s| s.mass.value().0).sum()
}

#[test]
fn steam_replaces_the_condensed_water_layer() {
    let mut trace = Trace::new();
    let (t, rock, hides) = body_492(&mut trace);
    assert!(
        rock.shells
            .iter()
            .any(|s| matches!(s.material.value(), Material::Water | Material::Ice))
    );
    let steamy = steam_interior(&rock, layer_mass(&rock), &t, hides, &mut trace).unwrap();
    let (last, core) = steamy.shells.split_last().unwrap();
    assert_eq!(*last.material.value(), Material::Steam);
    assert!(
        core.iter()
            .all(|s| matches!(s.material.value(), Material::Iron | Material::Rock))
    );
    let whole = kilograms(&t.mass);
    assert!((shell_sum(&steamy) / whole - 1.0).abs() < SUM);
    assert!((kilograms(&steamy.bulk.mass) / whole - 1.0).abs() < SUM);
    assert!((kilograms(&steamy.bulk.mass) / shell_sum(&steamy) - 1.0).abs() < SUM);
    assert!(matches!(steamy.water, Water::Steam { .. }));
}

fn check_bulk(steamy: &Interior) {
    let m = kilograms(&steamy.bulk.mass);
    let r = Metre::from(*steamy.bulk.radius.value()).0;
    let g = GRAVITATIONAL_CONSTANT * m / (r * r);
    assert!((steamy.bulk.surface_gravity.value().0 / g - 1.0).abs() < 1e-12);
    let rho = m / (4.0 / 3.0 * std::f64::consts::PI * r.powi(3));
    assert!((steamy.bulk.mean_density.value().0 / rho - 1.0).abs() < 1e-12);
}

#[test]
fn a_steam_world_is_larger_than_its_condensed_twin() {
    // Turbet et al. 2020 § 3.1, inside the structures of its § 2.1
    // (27 bar to 270 kbar), past § 2.2's 27 kbar.
    let mut trace = Trace::new();
    let (mass, core, water) = GRID;
    let (t, d) = water_world_trial(&mut trace, mass, core, water, AU_492);
    let rock = rocky(&t.mass, &d, &t, &mut trace).unwrap();
    let hides = trace
        .leaf(Code::of("test.hides"), true, Provenance::Input)
        .unwrap();
    let steamy = steam_interior(&rock, layer_mass(&rock), &t, hides, &mut trace).unwrap();
    assert!(
        steamy.bulk.radius.value().0 > rock.bulk.radius.value().0,
        "{:?} {:?}",
        steamy.bulk.radius,
        rock.bulk.radius
    );
    check_bulk(&steamy);
}

#[test]
fn the_492_steam_world_stands_on_its_core() {
    // Outside Turbet's grid (12 % water, about 5.6e5 bar of steam, issue
    // #684): only the property against the dry core holds, the twin's ice
    // layer being thicker than the steam.
    let mut trace = Trace::new();
    let (t, rock, hides) = body_492(&mut trace);
    let steamy = steam_interior(&rock, layer_mass(&rock), &t, hides, &mut trace).unwrap();
    let core = &steamy.shells[steamy.shells.len() - 2];
    assert!(steamy.bulk.radius.value().0 > EarthRadius::from(*core.outer_radius.value()).0);
    check_bulk(&steamy);
}

#[test]
fn the_steam_envelope_records_its_causes() {
    let mut trace = Trace::new();
    let (t, rock, hides) = body_492(&mut trace);
    let steam = layer_mass(&rock);
    let steamy = steam_interior(&rock, steam, &t, hides, &mut trace).unwrap();
    let radius = ancestry(&trace, steamy.bulk.radius.node());
    let code_of = |id: NodeId| trace.node(id).unwrap().code.as_str().to_owned();
    let from = derived_from(&trace, steamy.bulk.radius.node());
    assert_eq!(code_of(steamy.bulk.radius.node()), codes::RADIUS_STEAM);
    let thickness = *from
        .iter()
        .find(|id| code_of(**id) == codes::STEAM_THICKNESS)
        .expect("the radius derives from the thickness");
    let causes = ancestry(&trace, thickness);
    assert!(causes.contains(&steam.node()));
    for source in t.sources.iter() {
        assert!(causes.contains(&source.flux.node()));
    }
    assert!(radius.contains(&thickness));
    let top = steamy.shells.last().unwrap();
    assert!(ancestry(&trace, top.state.node()).contains(&hides.node()));
}

#[test]
fn no_steam_leaves_the_rock_as_it_is() {
    let mut trace = Trace::new();
    let (t, rock, hides) = body_492(&mut trace);
    let none = trace
        .leaf(Code::of("test.no-steam"), Kilogram(0.0), Provenance::Input)
        .unwrap();
    assert_eq!(
        steam_interior(&rock, none, &t, hides, &mut trace).unwrap(),
        rock
    );
}

#[test]
fn a_dry_planets_steam_comes_out_of_its_mantle() {
    let mut trace = Trace::new();
    let (t, d): (Trial, Drawn) = reference_trial(&mut trace, &EARTH);
    let rock = rocky(&t.mass, &d, &t, &mut trace).unwrap();
    assert_eq!(rock.water, Water::InMantle);
    let body = super::enveloped::Body::Rocky(rock.clone());
    let steam = surface_water(&t, &d, &body, &mut trace).unwrap();
    let hides = trace
        .leaf(Code::of("test.hides"), true, Provenance::Input)
        .unwrap();
    let steamy = steam_interior(&rock, steam, &t, hides, &mut trace).unwrap();
    let (last, core) = steamy.shells.split_last().unwrap();
    assert_eq!(*last.material.value(), Material::Steam);
    assert!(
        core.iter()
            .all(|s| matches!(s.material.value(), Material::Iron | Material::Rock))
    );
    let whole = kilograms(&t.mass);
    let core_sum: f64 = core.iter().map(|s| s.mass.value().0).sum();
    assert!((core_sum / (whole - steam.value().0) - 1.0).abs() < SUM);
    assert!((shell_sum(&steamy) / whole - 1.0).abs() < SUM);
    assert!((kilograms(&steamy.bulk.mass) / shell_sum(&steamy) - 1.0).abs() < SUM);
    let mantle: f64 = core
        .iter()
        .filter(|s| *s.material.value() == Material::Rock)
        .map(|s| s.mass.value().0)
        .sum();
    assert!((steamy.mantle_mass.value().0 / mantle - 1.0).abs() < SUM);
    assert!(steamy.mantle_mass.value().0 < rock.mantle_mass.value().0);
    assert!(steamy.bulk.radius.value().0 > rock.bulk.radius.value().0);
}

#[test]
fn a_partial_steam_closes_the_mass_sum() {
    // The water the runaway kept is half of the layer: what escape took is
    // gone, the shells add up to the whole.
    let mut trace = Trace::new();
    let (t, rock, hides) = body_492(&mut trace);
    let half = trace
        .record(
            Code::of("test.half-steam"),
            Kilogram(layer_mass(&rock).value().0 / 2.0),
            &[layer_mass(&rock).node()],
        )
        .unwrap();
    let steamy = steam_interior(&rock, half, &t, hides, &mut trace).unwrap();
    assert!((kilograms(&steamy.bulk.mass) / shell_sum(&steamy) - 1.0).abs() < SUM);
}
