use super::*;

/// Source: the age of Lopez & Fortney 2014 Table 3, 1 Gyr.
const TABLE_3_AGE: Gyr = Gyr(1.0);

/// Source: the age of Lopez & Fortney 2014 Table 4, 10 Gyr.
const TABLE_4_AGE: Gyr = Gyr(10.0);

/// Source: Lopez & Fortney 2014 (ApJ 792, 1), Table 3 ("Low Mass Planet
/// Radii at 1 Gyr, Solar Metallicity"), three printed cells as
/// (flux F⊕, mass M⊕, envelope %, radius R⊕): row 10 F⊕ / 2.4 M⊕, column
/// 0.5 %: 1.86; row 0.1 F⊕ / 13 M⊕, column 5 %: 3.11; row 1000 F⊕ /
/// 1 M⊕, column 20 %: 8.75. Read on 2026-11-21 in the ar5iv rendering of
/// arXiv:1311.0329 (the page whose sha256 `GIANTS.md` records).
const TABLE_3_CELLS: [(f64, f64, f64, f64); 3] = [
    (10.0, 2.4, 0.5, 1.86),
    (0.1, 13.0, 5.0, 3.11),
    (1000.0, 1.0, 20.0, 8.75),
];

/// Definition: half the last printed digit of the table (0.01 R⊕).
const PRINTED_PRECISION: f64 = 0.005;

/// Definition: percent to fraction.
const PER_CENT: f64 = 100.0;

/// Source: Lopez & Fortney 2014 Table 3, row 10 F⊕, columns 2.4 and
/// 3.6 M⊕ at 1 %: 2.08 and 2.12 R⊕ (mass, radius).
const MASS_NEIGHBOURS: [(f64, f64); 2] = [(2.4, 2.08), (3.6, 2.12)];

/// Source: Lopez & Fortney 2014 Table 3, row 10 F⊕ / 5.5 M⊕, columns
/// 1 % and 2 %: 2.19 and 2.45 R⊕ (envelope %, radius).
const ENVELOPE_NEIGHBOURS: [(f64, f64); 2] = [(1.0, 2.19), (2.0, 2.45)];

/// Source: Lopez & Fortney 2014 Table 3, column 1 % at 5.5 M⊕, rows
/// 10 and 1000 F⊕: 2.19 and 2.52 R⊕ (flux, radius).
const FLUX_NEIGHBOURS: [(f64, f64); 2] = [(10.0, 2.19), (1000.0, 2.52)];

/// Source: Lopez & Fortney 2014, row 10 F⊕ / 5.5 M⊕, column 1 %: 2.19 R⊕
/// at 1 Gyr (Table 3) and 2.13 R⊕ at 10 Gyr (Table 4).
const AGE_NEIGHBOURS: [(f64, f64); 2] = [(1.0, 2.19), (10.0, 2.13)];

/// Source: a printed node of Table 3 used as the fixed coordinates when
/// another axis varies: 10 F⊕, 5.5 M⊕, 1 %.
const NODE: (f64, f64, f64) = (10.0, 5.5, 1.0);

/// Definition: half the lowest tabulated fraction, 5e-5.
const HALF_LOWEST: f64 = 5e-5;

/// Definition: a mass below the tables' 1 M⊕.
const LIGHT: EarthMass = EarthMass(0.5);

/// Definition: an envelope above the tables' 20 %.
const TOO_THICK: f64 = 0.25;

/// Definition: a mass above the tables' 20 M⊕.
const TOO_HEAVY: EarthMass = EarthMass(25.0);

fn fraction(percent: f64) -> Fraction {
    Fraction::new(percent / PER_CENT).unwrap()
}

/// Lopez & Fortney's planet radius: their Eq. 1 core plus the thickness.
fn radius(mass: f64, percent: f64, flux: f64, age: Gyr) -> f64 {
    let f = fraction(percent);
    let thickness = envelope_thickness(EarthMass(mass), f, EarthFlux(flux), age).unwrap();
    (mass * (1.0 - f.get())).powf(CORE_RADIUS_EXPONENT) + thickness.0
}

fn geometric_mean([(a, _), (b, _)]: [(f64, f64); 2]) -> f64 {
    (a * b).sqrt()
}

fn between(value: f64, [(_, a), (_, b)]: [(f64, f64); 2]) -> bool {
    value > a.min(b) && value < a.max(b)
}

#[test]
fn table_nodes_are_returned_exactly() {
    for (flux, mass, percent, printed) in TABLE_3_CELLS {
        let r = radius(mass, percent, flux, TABLE_3_AGE);
        assert!(
            (r - printed).abs() < PRINTED_PRECISION,
            "{flux} F⊕, {mass} M⊕, {percent} %: {r} against {printed}"
        );
    }
}

#[test]
fn between_nodes_the_radius_lies_between_them() {
    let (flux, mass, percent) = NODE;
    let by_mass = radius(geometric_mean(MASS_NEIGHBOURS), percent, flux, TABLE_3_AGE);
    assert!(between(by_mass, MASS_NEIGHBOURS), "mass: {by_mass}");
    let by_envelope = radius(mass, geometric_mean(ENVELOPE_NEIGHBOURS), flux, TABLE_3_AGE);
    assert!(
        between(by_envelope, ENVELOPE_NEIGHBOURS),
        "envelope: {by_envelope}"
    );
    let by_flux = radius(mass, percent, geometric_mean(FLUX_NEIGHBOURS), TABLE_3_AGE);
    assert!(between(by_flux, FLUX_NEIGHBOURS), "flux: {by_flux}");
    let by_age = radius(mass, percent, flux, Gyr(geometric_mean(AGE_NEIGHBOURS)));
    assert!(between(by_age, AGE_NEIGHBOURS), "age: {by_age}");
}

/// A calibrated rule (below the tables' 1e-4 the thickness scales with
/// the fraction), not a published value.
#[test]
fn calibrated_thickness_goes_to_zero_below_the_tables() {
    let (flux, mass, _) = NODE;
    let at = |f: Fraction| {
        envelope_thickness(EarthMass(mass), f, EarthFlux(flux), TABLE_3_AGE)
            .unwrap()
            .0
    };
    let lowest = at(LOWEST_TABULATED);
    assert!(lowest > 0.0);
    let half = at(Fraction::new(HALF_LOWEST).unwrap());
    assert!((half - lowest / 2.0).abs() < f64::EPSILON * lowest);
    assert_eq!(at(Fraction::new(0.0).unwrap()), 0.0);
}

#[test]
fn a_light_planet_reads_the_1_earth_mass_row() {
    let (flux, _, percent) = NODE;
    let at = |mass| envelope_thickness(mass, fraction(percent), EarthFlux(flux), TABLE_3_AGE);
    assert_eq!(at(LIGHT).unwrap(), at(MIN_MASS).unwrap());
}

#[test]
fn above_20_percent_is_refused() {
    let (flux, mass, _) = NODE;
    let envelope = Fraction::new(TOO_THICK).unwrap();
    let refused = envelope_thickness(EarthMass(mass), envelope, EarthFlux(flux), TABLE_3_AGE);
    assert_eq!(
        refused,
        Err(EnvelopeError::OutOfTable {
            mass: EarthMass(mass),
            envelope
        })
    );
}

#[test]
fn above_20_earth_masses_is_refused() {
    let (flux, _, percent) = NODE;
    let refused = envelope_thickness(TOO_HEAVY, fraction(percent), EarthFlux(flux), TABLE_4_AGE);
    assert!(matches!(refused, Err(EnvelopeError::OutOfTable { .. })));
}

#[test]
fn every_value_is_finite_and_positive_across_the_grid() {
    for mass in [0.1, 1.0, 3.0, 20.0] {
        for envelope in [1e-6, 1e-4, 3e-3, 0.2] {
            for flux in [0.0, 0.1, 50.0, 1e5] {
                for age in [0.0, 0.1, 4.5, 20.0] {
                    let t = envelope_thickness(
                        EarthMass(mass),
                        Fraction::new(envelope).unwrap(),
                        EarthFlux(flux),
                        Gyr(age),
                    )
                    .unwrap();
                    assert!(
                        t.0.is_finite() && t.0 > 0.0,
                        "{mass} {envelope} {flux} {age}"
                    );
                }
            }
        }
    }
}
