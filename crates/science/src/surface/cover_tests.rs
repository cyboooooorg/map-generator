//! Tests of the surface cover and the ground albedo, ported from
//! `legacy-final`'s `planet/albedo.rs`, with the observations they hold
//! (Earth's Bond albedo and ice, Mars's caps, Triton's frost).

use super::*;
use crate::constants::{EARTH_MEAN_RADIUS, SOLAR_EFFECTIVE_TEMPERATURE, STANDARD_GRAVITY};
use crate::radiation::albedo::{ShortwaveCache, Underneath, shortwave_budget};
use crate::radiation::clouds::cloud_cover;
use crate::radiation::gases::PartialPressures;
use crate::radiation::ozone::ozone_column;
use crate::radiation::starlight::SOLAR_METALLICITY;
use crate::radiation::test_points::{EARTH_AIR, EARTH_SURFACE_K};
use crate::radiation::veils::sulphuric_deck;
use crate::units::{Degree, Dex};

/// Source: Earth's mean obliquity at J2000, 84 381.406 arcseconds, the
/// IAU 2006 value (Capitaine, Wallace & Chapront 2003,
/// doi:10.1051/0004-6361:20031539): 23.439 281 degrees.
const EARTH_OBLIQUITY_DEGREES: f64 = 23.439_281;

fn earth_obliquity() -> Radian {
    Radian::from(Degree(EARTH_OBLIQUITY_DEGREES))
}

/// Definition: the tolerance of an identity.
const TIGHT: f64 = 1e-12;

/// Definition: a surface air thin enough to leave a regolith almost
/// unshielded, 1e-5 bar (a tenth of `SHIELD_PRESSURE`): the shield is
/// `1 - e^-0.1`, about 9.5 %.
const THIN_AIR: Bar = Bar(1e-5);

/// Definition: the temperature of a cold airless body of the tests, K.
const COLD_BODY_K: f64 = 100.0;

/// Definition: the edge of Earth's ice at 288 K, `x_s = 0.898` (study 24,
/// section 6: `1 - ice_line_share`, rounded).
const EARTH_ICE_EDGE: f64 = 0.898;

fn no_frost() -> Weights {
    Weights::default()
}

fn dry(t: f64) -> Cover {
    cover(
        Kelvin(t),
        Kilogram(0.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
        false,
    )
}

fn oceans(n: f64) -> Kilogram {
    Kilogram(n * OCEAN_MASS.0)
}

fn nitrogen(bar: f64) -> Weights {
    Weights {
        n2: bar,
        ..Weights::default()
    }
}

#[test]
fn earth_has_seventy_one_percent_of_ocean() {
    assert!((ocean_fraction(1.0).get() - EARTH_OCEAN_COVER).abs() < TIGHT);
    assert_eq!(ocean_fraction(0.0).get(), 0.0);
    // Legacy's two bounds: the cover rises with the water and a drowned
    // world is all ocean.
    assert!(ocean_fraction(0.1).get() < ocean_fraction(1.0).get());
    assert!(ocean_fraction(10.0).get() > 0.99999);
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
            false,
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
    // Observation: Earth's ice and snow, about a tenth of its disc:
    // Antarctica 2.7 % (Bedmap2, Fretwell et al. 2013) and sea ice 6.7 %
    // (NSIDC), study 24, section 6. The band is the test's tolerance.
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
        ozone_column(Bar(air.o2), SOLAR_EFFECTIVE_TEMPERATURE),
        Kelvin(t),
        STANDARD_GRAVITY,
        SOLAR_EFFECTIVE_TEMPERATURE,
        Dex(SOLAR_METALLICITY),
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
    // not a white disc; a snowball Earth is white all over. The cap
    // bound (`ice < 0.02`) is the test's own definition of a cap: the
    // page has no measured share for it. The Bond albedo band is centred
    // on Mars's 0.250 (NASA Mars Fact Sheet), its width the test's
    // tolerance.
    // Mars's 6 mbar of CO2, NASA Mars Fact Sheet.
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
        false,
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
        false,
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
            false,
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
    let at = |frost: &Weights| {
        cover(
            Kelvin(40.0),
            Kilogram(0.0),
            EARTH_MEAN_RADIUS,
            frost,
            earth_obliquity(),
            false,
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
    let sulphur = at(&Weights {
        so2: 10.0,
        ..Weights::default()
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
        false,
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
        false,
    );
    println!(
        "Earth: ice {:.4}, sunlit ice {:.4}, ground {:.4}",
        c.ice.get(),
        c.ice_sunlit.get(),
        ground_albedo(&c, Bar(air.total())).get()
    );
    let bond = shortwave_budget(
        &air,
        ozone_column(Bar(air.o2), SOLAR_EFFECTIVE_TEMPERATURE),
        t,
        STANDARD_GRAVITY,
        SOLAR_EFFECTIVE_TEMPERATURE,
        Dex(SOLAR_METALLICITY),
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
                let mix = Weights {
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
                    false,
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
        false,
    );
    assert_eq!(wet.so2_frost.get(), 0.0);
    assert!(ground_albedo(&wet, Bar(1.0)).get().is_finite());
    // A fully iced disc divides nothing either.
    let iced = cover(
        Kelvin(200.0),
        oceans(5.0),
        EARTH_MEAN_RADIUS,
        &no_frost(),
        earth_obliquity(),
        false,
    );
    assert!(ground_albedo(&iced, Bar(1.0)).get().is_finite());
}

#[test]
fn surface_vapour_is_capped_by_the_water() {
    let t = Kelvin(EARTH_SURFACE_K);
    let saturation = vapour_saturation_bar(EARTH_SURFACE_K);
    let got = surface_vapour(t, Bar(10.0)).0;
    assert!(
        (got - RELATIVE_HUMIDITY * saturation).abs() < TIGHT * saturation,
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
        false,
    );
    assert!(c.ice_sunlit.get() < c.ice.get(), "{c:?}");
}

/// Source: Triton's orbit-mean solar flux, 1.508 W/m2 (NASA Neptune Fact
/// Sheet).
const TRITON_FLUX: f64 = 1.508;

/// Source: the temperature of Triton's N2 ice, 38 +- 1 K (Tryka et al.
/// 1993, *Science* 261, 751).
const TRITON_ICE_K: f64 = 38.0;

/// Source: the tolerance on Triton's ice temperature, 1 K (Tryka et al.
/// 1993).
const TRITON_ICE_SIGMA_K: f64 = 1.0;

/// Definition: the temperature of the locked-rotator test, 270 K, over
/// `ICE_COVERED_TEMPERATURE` and under `ICE_FREE_TEMPERATURE`.
const COLD_EARTH_K: f64 = 270.0;

/// Definition: the 270 K ice line of Earth, worked by hand from
/// `x^2 = 1/3 + (270 - 268.15) / 42`: `1 - sqrt(x^2) = 0.385687`.
const COLD_EARTH_ICE_LINE: f64 = 0.385_687;

/// Definition: the zonal sunlit ice share of Earth at 270 K, worked by
/// hand from `(1 - e) + (s2 / 2)(e - e^3)` with `e = 0.614313`, `s2 =
/// -(5/16)(3 cos^2(23.439281 deg) - 1) = -0.476662`: 0.294529.
const COLD_EARTH_ZONAL_SUNLIT: f64 = 0.294_529;

/// Definition: the tolerance of the hand-worked values, 1e-5.
const HAND_WORKED: f64 = 1e-5;

#[test]
fn nitrogen_frost_radiates_less_than_a_black_body() {
    assert_eq!(frost_emissivity(&no_frost()).get(), 1.0);
    assert!((frost_emissivity(&nitrogen(FROST_LAYER.0)).get() - N2_ICE_EMISSIVITY).abs() < TIGHT);
    assert!((N2_ICE_EMISSIVITY - 0.80).abs() < TIGHT);
    // Half the layer covers half the disc: 1 - 0.2 / 2.
    assert!((frost_emissivity(&nitrogen(FROST_LAYER.0 / 2.0)).get() - 0.90).abs() < TIGHT);
    let co2 = Weights {
        co2: 10.0,
        ..Weights::default()
    };
    assert_eq!(frost_emissivity(&co2).get(), 1.0);
}

#[test]
fn triton_frost_balances_at_38_k() {
    // eps sigma T^4 = (1 - A) F / 4 for the orbit-mean flux.
    let t = ((1.0 - FROST_ALBEDO) * TRITON_FLUX
        / (4.0 * N2_ICE_EMISSIVITY * crate::constants::STEFAN_BOLTZMANN))
        .powf(0.25);
    assert!((t - TRITON_ICE_K).abs() <= TRITON_ICE_SIGMA_K, "{t}");
}

#[test]
fn a_locked_body_freezes_from_its_night_side() {
    let f = |a: f64| locked_sunlit_ice(Fraction::new(a).unwrap()).get();
    assert_eq!(f(0.0), 0.0);
    assert_eq!(f(0.5), 0.0);
    assert!((f(0.75) - 0.25).abs() < TIGHT);
    assert_eq!(f(1.0), 1.0);
    let at = |t: f64, locked: bool| {
        cover(
            Kelvin(t),
            oceans(1.0),
            EARTH_MEAN_RADIUS,
            &no_frost(),
            earth_obliquity(),
            locked,
        )
    };
    let zonal = at(COLD_EARTH_K, false);
    let locked = at(COLD_EARTH_K, true);
    assert!(
        (zonal.ice.get() - COLD_EARTH_ICE_LINE).abs() < HAND_WORKED,
        "{zonal:?}"
    );
    assert_eq!(locked.ice, zonal.ice);
    assert_eq!(locked.ice_sunlit.get(), 0.0);
    assert!(
        (zonal.ice_sunlit.get() - COLD_EARTH_ZONAL_SUNLIT).abs() < HAND_WORKED,
        "{zonal:?}"
    );
    // Under the ice-covered temperature both are white.
    let white = ICE_COVERED_TEMPERATURE.0 - 4.0;
    assert_eq!(at(white, true).ice_sunlit.get(), 1.0);
    assert_eq!(at(white, false).ice_sunlit.get(), 1.0);
}

#[test]
fn the_ice_free_temperature_is_where_the_ice_line_vanishes() {
    assert!((ICE_FREE_TEMPERATURE.0 - 296.15).abs() < TIGHT);
    assert_eq!(ice_line_share(ICE_FREE_TEMPERATURE).get(), 0.0);
}
