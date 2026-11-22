use super::*;

/// Source: the sidereal year that Kepler's third law gives for 1 au around
/// one nominal solar mass parameter, planet mass neglected: 365.2569 d
/// (2π sqrt(au³ / GM☉), IAU 2012 au and IAU 2015 B3 GM☉).
const KEPLER_YEAR: f64 = 365.256_9;

/// Definition: tolerance on [`KEPLER_YEAR`], days (its last digit).
const YEAR_TOLERANCE: f64 = 1e-3;

/// Definition: relative tolerance of a period and axis round trip.
const ROUND_TRIP: f64 = 1e-12;

/// Definition: the orbits the round trip is checked at, au: a hot orbit,
/// Jupiter's neighbourhood, Neptune's.
const AXES: [f64; 3] = [0.05, 5.0, 30.0];

/// Definition: the hosts the round trip is checked at, solar masses: an M
/// dwarf and the heaviest star of the tables.
const HOSTS: [f64; 2] = [0.3, 8.0];

/// Definition: the eccentricity the factors are checked at.
const ECCENTRICITY: f64 = 0.2;

/// Source: `1 / sqrt(1 - 0.2²)` = 1.020621 (legacy `star.rs`
/// `mean_flux_factor`, computed).
const FLUX_FACTOR_AT_0_2: f64 = 1.020_621;

/// Source: `1 - 0.2²/16 - 15 × 0.2⁴/1024` = 0.9974766 (Méndez &
/// Rivera-Valentín 2017 eq. 16, computed).
const TEMPERATURE_FACTOR_AT_0_2: f64 = 0.997_476_6;

/// Definition: tolerance on [`FLUX_FACTOR_AT_0_2`] (its last digit).
const FLUX_TOLERANCE: f64 = 1e-6;

/// Definition: tolerance on [`TEMPERATURE_FACTOR_AT_0_2`] (its last digit).
const TEMPERATURE_TOLERANCE: f64 = 1e-7;

/// Definition: a host of 8 M☉, the heaviest of the tables.
const HEAVY_HOST: SolarMass = SolarMass(8.0);

/// Source: 2.7 au × 8 (Ida & Lin 2005 via Kennedy & Kenyon 2008, the
/// linear ice line at 8 M☉; architecture study, "Host scaling").
const HEAVY_ICE_LINE: f64 = 21.6;

/// Source: Rappaport et al. 2013 eq. 2: 12.6 h at 1 g cm⁻³, in days.
const UNIT_DENSITY_ROCHE: f64 = 12.6 / 24.0;

/// Source: the mean density of a 1 M_J planet of 1.3 R_J, kg/m³: 1.898e27 kg
/// over (4π/3)(1.3 × 7.1492e7 m)³ = 604 kg/m³ (computed from the IAU 2015
/// B3 nominal Jovian values).
const INFLATED_JUPITER_DENSITY: KilogramPerCubicMetre = KilogramPerCubicMetre(604.0);

/// Source: Rappaport et al. 2013 eq. 2 at [`INFLATED_JUPITER_DENSITY`]:
/// 12.6 h / sqrt(0.604) = 16.21 h = 0.6755 d (computed). Dawson & Johnson
/// 2018 eq. 15 quote 0.79 d for the same planet with the coefficient
/// 2.7 in place of 2.44, (2.7 / 2.44)^(3/2) = 1.16 times longer.
const INFLATED_JUPITER_ROCHE: f64 = 0.675_5;

/// Definition: tolerance on [`INFLATED_JUPITER_ROCHE`], days.
const ROCHE_TOLERANCE: f64 = 0.005;

/// Source: textbook masses, Earth masses, and semi-major axes, au, of
/// Venus, Earth and Mars (small-planets study, section 5).
const VENUS: (f64, f64) = (0.815, 0.723);
/// Source: as [`VENUS`].
const EARTH: (f64, f64) = (1.0, 1.0);
/// Source: as [`VENUS`].
const MARS: (f64, f64) = (0.107, 1.524);

/// Source: Venus-Earth spacing in mutual Hill radii, 26.3 (small-planets
/// study, section 5, derived from Weiss 2018 eq. 3-4).
const VENUS_EARTH_SPACING: f64 = 26.3;

/// Source: Earth-Mars spacing in mutual Hill radii, 40.1 (small-planets
/// study, section 5, derived from Weiss 2018 eq. 3-4).
const EARTH_MARS_SPACING: f64 = 40.1;

/// Definition: tolerance on the spacings, Hill radii (the study rounds
/// to 0.1, the textbook inputs to three digits).
const SPACING_TOLERANCE: f64 = 0.3;

/// Definition: an age of 1 Gyr, where Pu & Wu 2015 quote K50.
const ONE_GYR: Gyr = Gyr(1.0);

/// Definition: an innermost period of 10 days, the break of Mulders et al.
/// 2015 (architecture study, "Host scaling").
const INNERMOST: Day = Day(10.0);

/// Source: Pu & Wu 2015 eq. 12 at 1 Gyr and 10 d: 0.7 log10(3.6525e10)
/// + 2.87 = 10.26 (computed).
const K50_AT_1_GYR: f64 = 10.26;

/// Definition: tolerance on [`K50_AT_1_GYR`] (its rounding).
const K50_TOLERANCE: f64 = 0.05;

/// Source: a Rayleigh eccentricity scale of 0.02, which Pu & Wu 2015
/// (eq. 14) turn into 2 more Hill radii.
const SIGMA_E: f64 = 0.02;

fn fraction(e: f64) -> Fraction {
    Fraction::new(e).unwrap()
}

fn pair((mass, a): (f64, f64), e: f64) -> Pair {
    Pair {
        mass: EarthMass(mass),
        a: AstronomicalUnit(a),
        e: fraction(e),
    }
}

#[test]
fn earth_has_a_year() {
    let year = period(AstronomicalUnit(1.0), SolarMass(1.0));
    assert!((year.0 - KEPLER_YEAR).abs() < YEAR_TOLERANCE, "{}", year.0);
}

#[test]
fn period_and_axis_invert() {
    for a in AXES {
        for m in HOSTS {
            let host = SolarMass(m);
            let back = semi_major_axis(period(AstronomicalUnit(a), host), host);
            assert!((back.0 / a - 1.0).abs() < ROUND_TRIP, "{a} au, {m} Msun");
        }
    }
}

#[test]
fn circular_orbit_factors_are_one() {
    assert_eq!(mean_flux_factor(fraction(0.0)), 1.0);
    assert_eq!(mean_temperature_factor(fraction(0.0)), 1.0);
    let flux = circular_flux(SolarLuminosity(1.0), AstronomicalUnit(1.0));
    assert_eq!(flux.0, 1.0);
}

#[test]
fn factors_match_legacy() {
    let e = fraction(ECCENTRICITY);
    let f = mean_flux_factor(e);
    assert!((f - FLUX_FACTOR_AT_0_2).abs() < FLUX_TOLERANCE, "{f}");
    let t = mean_temperature_factor(e);
    assert!(
        (t - TEMPERATURE_FACTOR_AT_0_2).abs() < TEMPERATURE_TOLERANCE,
        "{t}"
    );
}

#[test]
fn equilibrium_of_earth_is_255() {
    assert_eq!(
        equilibrium(EarthFlux(1.0), fraction(0.0)),
        EARTH_EQUILIBRIUM
    );
    let eccentric = equilibrium(EarthFlux(1.0), fraction(ECCENTRICITY));
    let expected = EARTH_EQUILIBRIUM.0 * TEMPERATURE_FACTOR_AT_0_2;
    assert!((eccentric.0 - expected).abs() < 1e-4, "{}", eccentric.0);
}

#[test]
fn ice_line_is_linear() {
    assert_eq!(ice_line(SolarMass(1.0)), ICE_LINE_OF_THE_SUN);
    let heavy = ice_line(HEAVY_HOST);
    assert!((heavy.0 - HEAVY_ICE_LINE).abs() < 1e-12, "{}", heavy.0);
}

#[test]
fn roche_period_of_unit_density_is_12_6_h() {
    let p = roche_period(ROCHE_REFERENCE_DENSITY);
    assert!((p.0 - UNIT_DENSITY_ROCHE).abs() < 1e-12, "{}", p.0);
}

#[test]
fn an_inflated_jupiter_has_a_roche_period_near_0_68_d() {
    let p = roche_period(INFLATED_JUPITER_DENSITY);
    assert!(
        (p.0 - INFLATED_JUPITER_ROCHE).abs() < ROCHE_TOLERANCE,
        "{}",
        p.0
    );
}

#[test]
fn venus_and_earth_are_26_hill_radii_apart() {
    let d = mutual_hill_spacing(pair(VENUS, 0.0), pair(EARTH, 0.0), SolarMass(1.0));
    assert!((d - VENUS_EARTH_SPACING).abs() < SPACING_TOLERANCE, "{d}");
}

#[test]
fn earth_and_mars_are_40_hill_radii_apart() {
    let d = mutual_hill_spacing(pair(EARTH, 0.0), pair(MARS, 0.0), SolarMass(1.0));
    assert!((d - EARTH_MARS_SPACING).abs() < SPACING_TOLERANCE, "{d}");
}

#[test]
fn eccentricity_shrinks_the_spacing() {
    let host = SolarMass(1.0);
    let circular = mutual_hill_spacing(pair(EARTH, 0.0), pair(MARS, 0.0), host);
    let inner = mutual_hill_spacing(pair(EARTH, ECCENTRICITY), pair(MARS, 0.0), host);
    let outer = mutual_hill_spacing(pair(EARTH, 0.0), pair(MARS, ECCENTRICITY), host);
    let both = mutual_hill_spacing(pair(EARTH, ECCENTRICITY), pair(MARS, ECCENTRICITY), host);
    assert!(
        inner < circular && outer < circular,
        "{inner} {outer} {circular}"
    );
    assert!(both < inner && both < outer, "{both}");
}

#[test]
fn k50_is_about_10_at_1_gyr() {
    let circular = k50(ONE_GYR, INNERMOST, 0.0, Radian(0.0));
    assert!(
        (circular - K50_AT_1_GYR).abs() < K50_TOLERANCE,
        "{circular}"
    );
    let eccentric = k50(ONE_GYR, INNERMOST, SIGMA_E, Radian(0.0));
    assert!((eccentric - circular - 2.0).abs() < 1e-12, "{eccentric}");
}

#[test]
fn two_planet_floor_is_2_sqrt_3() {
    assert!((TWO_PLANET_FLOOR - 2.0 * 3f64.sqrt()).abs() < 1e-15);
}
