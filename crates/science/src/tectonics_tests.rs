use super::*;

/// Source: mass of Mars, Earth masses, NASA NSSDCA planetary fact sheet.
const MARS: f64 = 0.107;

/// Source: mass of Venus, Earth masses, NASA NSSDCA planetary fact sheet.
const VENUS: f64 = 0.815;

#[test]
fn plates_start_at_the_geometric_mean_of_mars_and_venus() {
    let mean = (MARS * VENUS).sqrt();
    assert!((PLATES_MIN_MASS.0 - mean).abs() < 0.01, "{mean}");
}

#[test]
fn io_is_a_heat_pipe_even_at_the_low_end_of_its_flux() {
    // Moore & Webb 2013, doi:10.1038/nature12473: Io carries about 2.5
    // W/m2, some 40 times Earth's mean. The threshold is under half of
    // Io's flux and over ten times Earth's.
    /// Source: Io's heat flux, W/m2, Moore & Webb 2013 (checked).
    const IO: f64 = 2.5;
    /// Source: Earth's mean flux, Io's over 40 (Moore & Webb 2013).
    const EARTH: f64 = IO / 40.0;
    let threshold = HEAT_PIPE_FLUX.0;
    assert!(threshold < IO / 2.0);
    assert!(threshold > 10.0 * EARTH);
}
