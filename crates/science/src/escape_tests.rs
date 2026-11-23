use super::*;
use crate::constants::{EARTH_MASS, EARTH_MEAN_RADIUS, GRAVITATIONAL_CONSTANT};
use crate::xuv::EARTH_XUV_FLUENCE;

/// Source: Earth's escape velocity, 11.186 km/s, NASA NSSDCA Earth Fact
/// Sheet (D. R. Williams, last updated 15 November 2024),
/// <https://nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html>,
/// row "Escape velocity (km/s)".
const FACT_SHEET_ESCAPE_VELOCITY: MetrePerSecond = MetrePerSecond(11_186.0);

/// Definition: relative tolerance of a law checked against its own
/// formula evaluated by hand (round-off only).
const ROUND_OFF: f64 = 1e-12;

/// Definition: relative tolerance of the escape velocity against the fact
/// sheet, whose Earth radius (6371.000 km, volumetric) differs from
/// [`EARTH_MEAN_RADIUS`] by 8 m.
const FACT_SHEET_TOLERANCE: f64 = 0.005;

/// Definition: escape velocities, m/s, at which the shoreline is checked,
/// from an airless dwarf to a super-Earth.
const VELOCITIES: [f64; 4] = [2_000.0, 5_000.0, 11_186.0, 20_000.0];

fn relative(value: f64, expected: f64) -> f64 {
    (value / expected - 1.0).abs()
}

#[test]
fn the_loss_is_owen_and_wus_energy_limit() {
    let loss = energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS, EARTH_XUV_FLUENCE);
    // Owen & Wu 2017, eq. 19 integrated over the fluence, eta = 0.1.
    let by_hand = 0.1 * std::f64::consts::PI * EARTH_MEAN_RADIUS.0.powi(3) * EARTH_XUV_FLUENCE.0
        / (GRAVITATIONAL_CONSTANT * EARTH_MASS.0);
    assert!(
        relative(loss.0, by_hand) < ROUND_OFF,
        "{loss:?} against {by_hand}"
    );
}

#[test]
fn the_loss_scales_as_r_cubed_over_m() {
    let base = energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS, EARTH_XUV_FLUENCE).0;
    let wider = energy_limited_loss(EARTH_MEAN_RADIUS * 2.0, EARTH_MASS, EARTH_XUV_FLUENCE).0;
    let heavier = energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS * 4.0, EARTH_XUV_FLUENCE).0;
    assert!(
        relative(wider, base * 8.0) < ROUND_OFF,
        "{wider} against {base}"
    );
    assert!(
        relative(heavier, base / 4.0) < ROUND_OFF,
        "{heavier} against {base}"
    );
}

#[test]
fn no_fluence_no_loss() {
    let none = JoulePerSquareMetre(0.0);
    assert_eq!(
        energy_limited_loss(EARTH_MEAN_RADIUS, EARTH_MASS, none),
        Kilogram(0.0)
    );
    for v in VELOCITIES {
        for magnetised in [true, false] {
            assert_eq!(
                shoreline_kept(MetrePerSecond(v), none, magnetised).get(),
                1.0,
                "{v} {magnetised}"
            );
        }
    }
}

#[test]
fn earths_escape_velocity_is_11_2_km_s() {
    let v = escape_velocity(EARTH_MASS, EARTH_MEAN_RADIUS);
    assert!(
        relative(v.0, FACT_SHEET_ESCAPE_VELOCITY.0) < FACT_SHEET_TOLERANCE,
        "{v:?}"
    );
}

#[test]
fn the_shoreline_passes_half_at_its_line() {
    let kept = shoreline_kept(SHORELINE_VELOCITY, EARTH_XUV_FLUENCE, true).get();
    assert!(relative(kept, 0.5) < ROUND_OFF, "{kept}");
}

#[test]
fn a_missing_dynamo_doubles_the_fluence() {
    for v in VELOCITIES {
        for phi in [0.1, 1.0, 10.0, 1e3] {
            let fluence = EARTH_XUV_FLUENCE * phi;
            let bare = shoreline_kept(MetrePerSecond(v), fluence, false).get();
            let doubled = shoreline_kept(MetrePerSecond(v), fluence * 2.0, true).get();
            assert!(
                (bare - doubled).abs() <= ROUND_OFF * doubled.max(f64::MIN_POSITIVE),
                "{v} {phi}: {bare} against {doubled}"
            );
        }
    }
}
