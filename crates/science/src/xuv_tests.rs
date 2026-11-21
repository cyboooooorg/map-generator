use super::*;
use crate::constants::SOLAR_IRRADIANCE;
use crate::heat::EARTH_AGE;

/// Definition: the Sun of the tables, 1 M☉ at [Fe/H] 0.
const SUN: (SolarMass, Dex) = (SolarMass(1.0), Dex(0.0));

/// Definition: steps of the midpoint sums the analytic integrals are
/// checked against.
const FINE_STEPS: u32 = 100_000;

/// Definition: age the analytic integral of the share is checked to, Gyr,
/// past every saturation time but the lightest.
const CHECK_AGE: f64 = 6.0;

/// Definition: a 2.5 M☉ star, about A0: no convective envelope.
const A_STAR: SolarMass = SolarMass(2.5);

fn sun_luminosity(age: f64) -> f64 {
    main_sequence(SUN.0, SUN.1, Gyr(age)).unwrap().luminosity.0
}

/// Ribas et al.'s flux at 1 au, W/m², held at its first value before
/// [`RIBAS_START`].
fn ribas((f1, n): (f64, f64), age: f64) -> f64 {
    f1 * age.max(RIBAS_START.0).powf(-n)
}

/// The band flux at 1 au from the Sun of the tables at `age`, W/m²:
/// today's solar constant scaled by its luminosity history.
fn model_sun(band: Band, age: f64) -> f64 {
    let flux = SOLAR_IRRADIANCE.0 * sun_luminosity(age) / sun_luminosity(EARTH_AGE.0);
    exposure(band, SUN.0, SUN.1, Gyr(age))
        .unwrap()
        .flux(WattPerSquareMetre(flux))
        .0
}

#[test]
fn the_sun_follows_ribas() {
    for (band, fit) in [(XUV, RIBAS_XUV), (LYMAN_ALPHA, RIBAS_LYMAN_ALPHA)] {
        // Ribas et al.'s five stars, and the Sun today.
        for age in [0.1, 0.3, 0.65, 1.6, EARTH_AGE.0, 6.7] {
            let r = model_sun(band, age) / ribas(fit, age);
            assert!((r - 1.0).abs() < 2e-3, "{fit:?} at {age}: {r}");
        }
    }
}

#[test]
fn earth_fluence_is_the_ribas_integral() {
    let (f1, n) = RIBAS_XUV;
    let gyr = Second::from(Year::from(Gyr(1.0))).0;
    let held = ribas(RIBAS_XUV, 0.0) * RIBAS_START.0;
    let decayed = f1 * (RIBAS_START.0.powf(1.0 - n) - EARTH_AGE.0.powf(1.0 - n)) / (n - 1.0);
    let j = (held + decayed) * gyr;
    assert!((j / EARTH_XUV_FLUENCE.0 - 1.0).abs() < 2e-3, "{j:e}");
    let e = exposure(XUV, SUN.0, SUN.1, EARTH_AGE).unwrap();
    let model = e.fluence(SOLAR_IRRADIANCE).0;
    assert!(
        (model / EARTH_XUV_FLUENCE.0 - 1.0).abs() < 3e-3,
        "{model:e}"
    );
    // Today's flux, 4.6 mW/m².
    let now = e.flux(SOLAR_IRRADIANCE).0;
    assert!((now / 4.6e-3 - 1.0).abs() < 0.01, "{now}");
}

#[test]
fn lyman_alpha_today_is_about_4e11_photons() {
    // Ribas et al. 2005: about 3.9e11 photons cm⁻² s⁻¹ at 4.6 Gyr; a
    // Lyman α photon carries 1.634e-18 J.
    let photons = model_sun(LYMAN_ALPHA, 4.6) / 1.634e-18 * 1e-4;
    assert!((3.7e11..4.1e11).contains(&photons), "{photons:e}");
}

#[test]
fn the_band_history_integrates_and_inverts() {
    for mass in [1.0, 0.5, 0.2].map(SolarMass) {
        for band in [XUV, LYMAN_ALPHA] {
            let age = CHECK_AGE;
            let dt = age / f64::from(FINE_STEPS);
            let sum: f64 = (0..FINE_STEPS)
                .map(|i| ratio(band, mass, Gyr((f64::from(i) + 0.5) * dt)) * dt)
                .sum();
            let exact = integrated_ratio(band, mass, Gyr(age)).0;
            assert!((sum / exact - 1.0).abs() < 1e-3, "{mass:?}: {sum} {exact}");
            let t = age_below_ratio(band, mass, ratio(band, mass, Gyr(3.0))).0;
            if saturation(mass).0 < 3.0 {
                assert!((t / 3.0 - 1.0).abs() < 1e-9, "{mass:?}: {t}");
            } else {
                assert_eq!(t, 0.0, "{mass:?}");
            }
        }
    }
    assert_eq!(age_below_ratio(LYMAN_ALPHA, SUN.0, 1.0).0, 0.0);
    assert!(age_below_ratio(LYMAN_ALPHA, SUN.0, 0.0).0.is_infinite());
}

#[test]
fn the_saturation_time_goes_through_its_anchors() {
    for (mass, gyr) in SATURATION {
        let t = saturation(SolarMass(mass)).0;
        assert!((t / gyr - 1.0).abs() < 1e-12, "{mass}: {t}");
    }
    let masses = [2.0, 1.0, 0.7, 0.5, 0.3, 0.25, 0.1, 0.05, 0.01];
    for pair in masses.windows(2) {
        assert!(saturation(SolarMass(pair[1])).0 >= saturation(SolarMass(pair[0])).0);
    }
    assert!(saturation(SolarMass(0.01)).0.is_finite());
    assert!(saturation(SolarMass(0.1)).0 > 13.0);
}

#[test]
fn a_star_that_cools_through_the_limit_gains_its_corona_on_the_way() {
    // A 1.6 M☉ star at [Fe/H] 0 crosses the limit during its main
    // sequence. Its exposure counts only the ages below the limit: against
    // a fine midpoint sum.
    let (mass, feh) = (SolarMass(1.6), Dex(0.0));
    let l = life(mass, feh).unwrap();
    let age = l.end.age().0;
    let teff = |t: f64| {
        main_sequence(mass, feh, Gyr(t))
            .unwrap()
            .effective_temperature
    };
    assert!(
        !has_corona(teff(l.zams.0)) && has_corona(teff(age)),
        "{:?} {:?}",
        teff(l.zams.0),
        teff(age)
    );
    let dt = (age - l.zams.0) / f64::from(FINE_STEPS);
    let fine: f64 = (0..FINE_STEPS)
        .map(|i| l.zams.0 + (f64::from(i) + 0.5) * dt)
        .filter(|&t| has_corona(teff(t)))
        .map(|t| ratio(XUV, mass, Gyr(t)) * dt)
        .sum();
    let scale = main_sequence(mass, feh, l.zams).unwrap().luminosity.0
        / main_sequence(mass, feh, Gyr(age)).unwrap().luminosity.0;
    let e = exposure(XUV, mass, feh, Gyr(age)).unwrap();
    assert!(
        (e.integrated.0 / (fine * scale) - 1.0).abs() < 1e-3,
        "{e:?} {}",
        fine * scale
    );
    assert!(e.present_ratio > 0.0);
}

#[test]
fn a_star_without_a_corona_gives_no_xuv() {
    let mass = A_STAR;
    let feh = Dex(0.0);
    let zams = life(mass, feh).unwrap().zams;
    let star = main_sequence(mass, feh, Gyr(0.3)).unwrap();
    assert!(!has_corona(star.effective_temperature), "{star:?}");
    let e = exposure(XUV, mass, feh, Gyr(0.3)).unwrap();
    assert_eq!(
        (e.present_ratio, e.integrated.0),
        (0.0, 0.0),
        "zams {zams:?}"
    );
}

#[test]
fn m_dwarfs_saturate_longer() {
    let sun = exposure(XUV, SUN.0, SUN.1, EARTH_AGE).unwrap().integrated.0;
    let m = exposure(XUV, SolarMass(0.2), Dex(0.0), EARTH_AGE)
        .unwrap()
        .integrated
        .0;
    assert!(m > 10.0 * sun, "{m} {sun}");
}

#[test]
fn older_systems_received_more_but_not_much_more() {
    let at = |age| exposure(XUV, SUN.0, SUN.1, Gyr(age)).unwrap();
    // Received at today's flux of each age; the Sun of the tables leaves
    // the main sequence at 9.8 Gyr.
    let received = |age| at(age).integrated.0 * sun_luminosity(age);
    let r = received(9.0) / received(4.5);
    // The Sun of the tables, at a fixed orbit.
    assert!(r > 1.0 && r < 1.3, "{r}");
}

#[test]
fn the_exposure_is_finite_over_the_tables() {
    for mass in [0.08, 0.15, 0.3, 0.32, 0.5, 1.0, 1.5, 1.6, 2.0, 4.0, 8.0] {
        for feh in [-1.0, 0.0, 0.5] {
            let (mass, feh) = (SolarMass(mass), Dex(feh));
            let l = life(mass, feh).unwrap();
            for age in [l.zams, l.end.age()] {
                let e = exposure(XUV, mass, feh, age).unwrap();
                assert!(
                    e.integrated.0.is_finite() && e.integrated.0 >= 0.0,
                    "{mass:?} {feh:?} {age:?}: {e:?}"
                );
                assert!(e.present_ratio.is_finite() && e.present_ratio >= 0.0);
            }
        }
    }
}

#[test]
fn an_age_before_the_main_sequence_is_an_error() {
    let zams = life(SUN.0, SUN.1).unwrap().zams;
    assert!(matches!(
        exposure(XUV, SUN.0, SUN.1, Gyr(zams.0 / 2.0)),
        Err(StellarError::BeforeZams { .. })
    ));
}
