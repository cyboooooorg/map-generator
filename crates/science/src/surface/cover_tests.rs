//! Tests of the surface cover and the ground albedo, ported from
//! `legacy-final`'s `planet/albedo.rs`, with the observations they hold
//! (Earth's Bond albedo and ice, Mars's caps, Triton's frost).

use super::*;
use crate::constants::{EARTH_MEAN_RADIUS, SOLAR_EFFECTIVE_TEMPERATURE, STANDARD_GRAVITY};
use crate::radiation::albedo::{ShortwaveCache, Underneath, shortwave_budget};
use crate::radiation::clouds::cloud_cover;
use crate::radiation::test_points::{EARTH_AIR, EARTH_SURFACE_K};
use crate::radiation::veils::sulphuric_deck;
use crate::units::Degree;

/// Source: Earth's mean obliquity at J2000, 84 381.406 arcseconds, the
/// IAU 2006 value (Capitaine, Wallace & Chapront 2003,
/// doi:10.1051/0004-6361:20031539): 23.439 281 degrees.
const EARTH_OBLIQUITY_DEGREES: f64 = 23.439_281;

fn earth_obliquity() -> Radian {
    Radian::from(Degree(EARTH_OBLIQUITY_DEGREES))
}

/// Definition: the tolerance of an identity.
const TIGHT: f64 = 1e-12;

/// Definition: a surface air thin enough to leave a regolith unshielded
/// by half, 1e-5 bar (a tenth of `SHIELD_PRESSURE`).
const THIN_AIR: Bar = Bar(1e-5);

/// Definition: the temperature of a cold airless body of the tests, K.
const COLD_BODY_K: f64 = 100.0;

/// Definition: the edge of Earth's ice at 288 K, `x_s = 0.898` (study 24,
/// section 6: `1 - ice_line_share`, rounded).
const EARTH_ICE_EDGE: f64 = 0.898;

fn no_frost() -> PartialPressures {
    PartialPressures::default()
}

fn dry(t: f64) -> Cover {
    cover(
        Kelvin(t),
        Kilogram(0.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
    )
}

fn oceans(n: f64) -> Kilogram {
    Kilogram(n * OCEAN_MASS.0)
}

fn nitrogen(bar: f64) -> PartialPressures {
    PartialPressures {
        n2: bar,
        ..PartialPressures::default()
    }
}

#[test]
fn earth_has_seventy_one_percent_of_ocean() {
    assert!((ocean_fraction(1.0).get() - EARTH_OCEAN_COVER).abs() < TIGHT);
    assert_eq!(ocean_fraction(0.0).get(), 0.0);
}

#[test]
fn ice_cover_grows_as_the_planet_cools() {
    let ice = |t: f64| {
        cover(
            Kelvin(t),
            oceans(1.0),
            EARTH_MEAN_RADIUS,
            &no_frost(),
            earth_obliquity(),
        )
        .ice
        .get()
    };
    let mut last = f64::INFINITY;
    for i in 0..=100 {
        let t = 200.0 + f64::from(i);
        let now = ice(t);
        assert!(now <= last, "{t}: {now} > {last}");
        last = now;
    }
    let earth = ice(EARTH_SURFACE_K);
    println!("Earth's ice at 288 K: {earth:.4}");
    // Observation: Earth's ice and snow, about a tenth of its disc.
    assert!((0.06..=0.14).contains(&earth), "{earth}");
    for t in [200.0, 230.0, 240.0, 250.0] {
        assert!(ice(t) > 0.999, "{t}: {}", ice(t));
    }
    assert_eq!(ice(300.0), 0.0);
}

/// The Bond albedo of `cover` under `air`, with no cloud and no deck, in
/// the Sun's light on an Earth-gravity body.
fn bond_albedo_clear(cover: &Cover, air: &PartialPressures, t: f64) -> f64 {
    shortwave_budget(
        air,
        Kelvin(t),
        STANDARD_GRAVITY,
        SOLAR_EFFECTIVE_TEMPERATURE,
        Underneath {
            ground: ground_albedo(cover, Bar(air.total())),
            clouds: Fraction::new(0.0).unwrap(),
            deck: Fraction::new(0.0).unwrap(),
        },
        &mut ShortwaveCache::default(),
    )
    .bond_albedo
    .get()
}

#[test]
fn a_nearly_dry_planet_whitens_only_its_poles() {
    // Mars-like: a few thousandths of an ocean at 210 K make polar caps,
    // not a white disc; a snowball Earth is white all over.
    let air = PartialPressures {
        co2: 6e-3,
        ..PartialPressures::default()
    };
    let caps = cover(
        Kelvin(210.0),
        oceans(0.005),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
    );
    println!("0.005 oceans at 210 K: ice {:.5}", caps.ice.get());
    assert!(caps.ice.get() > 0.0 && caps.ice.get() < 0.02, "{caps:?}");
    let a = bond_albedo_clear(&caps, &air, 210.0);
    println!("its Bond albedo: {a:.4}");
    assert!((0.2..=0.3).contains(&a), "{a}");
    let full = cover(
        Kelvin(210.0),
        oceans(1.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
    );
    assert_eq!(full.ice.get(), 1.0);
}

#[test]
fn ice_reach_scales_with_area() {
    let ice = |radius: f64| {
        cover(
            Kelvin(200.0),
            oceans(0.05),
            Metre(radius),
            &no_frost(),
            earth_obliquity(),
        )
        .ice
        .get()
    };
    let one = ice(EARTH_MEAN_RADIUS.0);
    let two = ice(2.0 * EARTH_MEAN_RADIUS.0);
    assert!(one < 1.0, "{one}");
    assert!((two / one - 0.25).abs() < TIGHT, "{}", two / one);
}

#[test]
fn bare_rock_is_all_land() {
    for t in [100.0, 250.0, 440.0, 700.0] {
        let c = dry(t);
        assert_eq!(c.land.get(), 1.0, "{t}: {c:?}");
        assert_eq!(c.ocean.get(), 0.0, "{t}: {c:?}");
        assert_eq!(c.ice.get(), 0.0, "{t}: {c:?}");
        assert_eq!(c.frost.get(), 0.0, "{t}: {c:?}");
        assert_eq!(c.so2_frost.get(), 0.0, "{t}: {c:?}");
        assert!((ground_albedo(&c, Bar(0.0)).get() - REGOLITH_ALBEDO).abs() < TIGHT);
        assert!((ground_albedo(&c, Bar(1.0)).get() - SOIL_ALBEDO).abs() < 1e-3);
    }
}

#[test]
fn frost_brightens_a_body_in_proportion_to_its_amount() {
    let at = |frost: &PartialPressures| {
        cover(
            Kelvin(40.0),
            Kilogram(0.0),
            EARTH_MEAN_RADIUS,
            frost,
            earth_obliquity(),
        )
    };
    let half = at(&nitrogen(FROST_LAYER.0 / 2.0));
    assert!((half.frost.get() - 0.5).abs() < TIGHT, "{half:?}");
    assert!((half.land.get() - 0.5).abs() < TIGHT, "{half:?}");
    let thin = at(&nitrogen(FROST_LAYER.0 / 10.0));
    let air = THIN_AIR;
    assert!(ground_albedo(&half, air).get() > ground_albedo(&thin, air).get());
    assert!(ground_albedo(&thin, air).get() > ground_albedo(&dry(40.0), air).get());
    let full = at(&nitrogen(FROST_LAYER.0));
    let deep = at(&nitrogen(10.0));
    let a_full = ground_albedo(&full, Bar(10.0)).get();
    assert!((a_full - ground_albedo(&deep, Bar(10.0)).get()).abs() < TIGHT);
    assert!((a_full - FROST_ALBEDO).abs() < TIGHT, "{a_full}");
    let sulphur = at(&PartialPressures {
        so2: 10.0,
        ..PartialPressures::default()
    });
    assert_eq!(sulphur.so2_frost.get(), 1.0);
    assert!((ground_albedo(&sulphur, Bar(10.0)).get() - SO2_FROST_ALBEDO).abs() < TIGHT);
    // A quarter of the layer over the ice of a frozen ocean.
    let iced = cover(
        Kelvin(40.0),
        oceans(1.0),
        EARTH_MEAN_RADIUS,
        &nitrogen(FROST_LAYER.0 / 4.0),
        earth_obliquity(),
    );
    assert!((iced.ice.get() - 0.75).abs() < TIGHT, "{iced:?}");
    assert_eq!(iced.ocean.get(), 0.0);
}

#[test]
fn earth_keeps_its_bond_albedo() {
    // Observation: Earth's Bond albedo, 0.30 +- 0.01.
    let t = Kelvin(EARTH_SURFACE_K);
    let h2o = surface_vapour(t, Bar(1.0));
    let air = PartialPressures {
        h2o: h2o.0,
        ..EARTH_AIR
    };
    let c = cover(
        t,
        oceans(1.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
    );
    println!(
        "Earth: ice {:.4}, sunlit ice {:.4}, ground {:.4}",
        c.ice.get(),
        c.ice_sunlit.get(),
        ground_albedo(&c, Bar(air.total())).get()
    );
    let bond = shortwave_budget(
        &air,
        t,
        STANDARD_GRAVITY,
        SOLAR_EFFECTIVE_TEMPERATURE,
        Underneath {
            ground: ground_albedo(&c, Bar(air.total())),
            clouds: cloud_cover(t, h2o),
            deck: sulphuric_deck(t, Bar(air.so2)),
        },
        &mut ShortwaveCache::default(),
    )
    .bond_albedo
    .get();
    println!("Earth's Bond albedo at 288 K: {bond:.4}");
    assert!((0.29..=0.31).contains(&bond), "{bond}");
}

#[test]
fn cover_shares_sum_to_one() {
    for t in [40.0, 200.0, 260.0, 288.0, 400.0] {
        for water in [0.0, 0.005, 1.0, 5.0] {
            for frost in [0.0, 0.005, 0.02, 1.0] {
                let mix = PartialPressures {
                    n2: frost / 2.0,
                    so2: frost / 2.0,
                    ..no_frost()
                };
                let c = cover(
                    Kelvin(t),
                    oceans(water),
                    EARTH_MEAN_RADIUS,
                    &mix,
                    earth_obliquity(),
                );
                let sum = c.ocean.get() + c.land.get() + c.ice.get() + c.frost.get();
                assert!((sum - 1.0).abs() < TIGHT, "{t} K {water} {frost}: {c:?}");
                assert!(c.so2_frost.get() <= c.frost.get() + TIGHT);
            }
        }
    }
}

#[test]
fn no_frost_divides_nothing() {
    let c = dry(COLD_BODY_K);
    assert_eq!(c.so2_frost.get(), 0.0);
    assert_eq!(c.frost.get(), 0.0);
    let wet = cover(
        Kelvin(250.0),
        oceans(1.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
    );
    assert_eq!(wet.so2_frost.get(), 0.0);
    assert!(ground_albedo(&wet, Bar(1.0)).get().is_finite());
}

#[test]
fn surface_vapour_is_capped_by_the_water() {
    let t = Kelvin(EARTH_SURFACE_K);
    let saturation = vapour_saturation_bar(EARTH_SURFACE_K);
    let got = surface_vapour(t, Bar(10.0)).0;
    assert!(
        (got - 0.77 * saturation).abs() < TIGHT * saturation,
        "{got}"
    );
    assert_eq!(surface_vapour(t, Bar(1e-6)).0, 1e-6);
}

#[test]
fn sunlight_weights_the_caps() {
    for degrees in [0.0, 23.4, 60.0, 90.0] {
        let beta = Radian::from(Degree(degrees));
        assert!((sunlit_cap(0.0, beta).get() - 1.0).abs() < TIGHT);
        assert!(sunlit_cap(1.0, beta).get().abs() < TIGHT);
    }
    // Earth's cap gets about 6 % of the light on 10.2 % of the area.
    let earth = sunlit_cap(EARTH_ICE_EDGE, earth_obliquity()).get();
    assert!((0.05..0.07).contains(&earth), "{earth}");
    assert!(earth < 1.0 - EARTH_ICE_EDGE);
    // A pole-on spin lights the caps more than their area.
    let pole_on = sunlit_cap(EARTH_ICE_EDGE, Radian::from(Degree(90.0))).get();
    assert!(pole_on > 1.0 - EARTH_ICE_EDGE, "{pole_on}");
    let c = cover(
        Kelvin(288.0),
        oceans(1.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
    );
    assert!(c.ice_sunlit.get() < c.ice.get(), "{c:?}");
}

#[test]
fn a_fully_iced_disc_has_no_nan() {
    let c = cover(
        Kelvin(200.0),
        oceans(5.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
    );
    assert!(ground_albedo(&c, Bar(1.0)).get().is_finite());
}
