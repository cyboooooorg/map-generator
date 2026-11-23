//! The giant-planet radius figure: radius against mass for three core
//! masses, from the Fortney et al. (2007) grid of `science::giants`, with
//! Jupiter and Saturn.

use plotters::prelude::*;
use science::constants::{GRAVITATIONAL_CONSTANT, JUPITER_MASS_PARAMETER};
use science::giants::{JUPITER_RADIUS, giant};
use science::units::{EarthFlux, EarthMass, Gyr, Kilogram, Metre};

use crate::{FigureError, svg};

/// Age of the models: that of the solar system, 4.5 Gyr.
const AGE: Gyr = Gyr(4.5);
/// Distance of the models, Jupiter's 5.2 au from a star of the Sun's
/// luminosity, in au.
const DISTANCE_AU: f64 = 5.2;
/// Cores drawn, in Earth masses.
const CORES: [f64; 3] = [0.0, 10.0, 25.0];
/// Mass range of the curves, in Earth masses.
const MIN_MASS: f64 = 20.0;
const MAX_MASS: f64 = 3000.0;
const POINTS: usize = 120;

/// Source: NASA Planetary Fact Sheet, Jupiter, "Mass (10^24 kg)" 1,898.13
/// (nssdc.gsfc.nasa.gov/planetary/factsheet/jupiterfact.html, read
/// 2026-11-23), in kg.
const JUPITER_MASS: Kilogram = Kilogram(1_898.13e24);
/// Source: NASA Planetary Fact Sheet, Jupiter, "Equatorial radius (1 bar
/// level) (km)" 71,492 (nssdc.gsfc.nasa.gov/planetary/factsheet/jupiterfact.html,
/// read 2026-11-23). The convention of Fortney et al. (2007), whose radii
/// are at 1 bar and in units of this radius.
const JUPITER_OBSERVED_RADIUS: Metre = Metre(71_492e3);
/// Source: NASA Planetary Fact Sheet, Saturn, "Mass (10^24 kg)" 568.32
/// (nssdc.gsfc.nasa.gov/planetary/factsheet/saturnfact.html, read
/// 2026-11-23), in kg.
const SATURN_MASS: Kilogram = Kilogram(568.32e24);
/// Source: NASA Planetary Fact Sheet, Saturn, "Equatorial radius (1 bar
/// level) (km)" 60,268 (nssdc.gsfc.nasa.gov/planetary/factsheet/saturnfact.html,
/// read 2026-11-23).
const SATURN_OBSERVED_RADIUS: Metre = Metre(60_268e3);

const CORE_COLORS: [RGBColor; 3] = [
    RGBColor(0x1f, 0x77, 0xb4),
    RGBColor(0xd6, 0x5f, 0x0e),
    RGBColor(0x2a, 0x9d, 0x4a),
];
const JUPITER_COLOR: RGBColor = RGBColor(0x20, 0x20, 0x20);
const SATURN_COLOR: RGBColor = RGBColor(0x8e, 0x44, 0xad);

/// Jupiter's mass, the unit of the mass axis.
fn jupiter_mass() -> Kilogram {
    Kilogram(JUPITER_MASS_PARAMETER / GRAVITATIONAL_CONSTANT)
}

/// Mass and radius in Jupiter masses and radii of the giants of a given
/// core, log-spaced from 20 to 3 000 M⊕. A mass the tables refuse (below
/// about 13 M⊕ for a core of 25 M⊕ at some age or distance) is skipped.
fn series(core: f64) -> Vec<(f64, f64)> {
    let flux = EarthFlux(1.0 / (DISTANCE_AU * DISTANCE_AU));
    (0..POINTS)
        .filter_map(|i| {
            let mass = MIN_MASS * (MAX_MASS / MIN_MASS).powf(i as f64 / (POINTS - 1) as f64);
            let g = giant(EarthMass(mass), EarthMass(core), AGE, flux).ok()?;
            let radius = Metre::from(g.radius);
            Some((
                Kilogram::from(EarthMass(mass)).0 / jupiter_mass().0,
                radius.0 / JUPITER_RADIUS.0,
            ))
        })
        .collect()
}

fn draw_error(e: impl std::fmt::Display) -> FigureError {
    FigureError::Draw(e.to_string())
}

/// A tick label without a spurious decimal: `0.1`, `1`, `10`.
fn tick(value: &f64) -> String {
    format!("{value}")
}

pub(crate) fn giant_radius_figure() -> Result<String, FigureError> {
    let curves: Vec<Vec<(f64, f64)>> = CORES.iter().map(|&c| series(c)).collect();
    let x_range = 0.05..12.0;
    let (lo, hi) = curves
        .iter()
        .flatten()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.1), hi.max(p.1))
        });
    let y_range = (lo.min(0.8) - 0.05)..(hi.max(1.2) + 0.05);
    let solar_system = [
        (
            "Jupiter",
            JUPITER_COLOR,
            JUPITER_MASS,
            JUPITER_OBSERVED_RADIUS,
        ),
        ("Saturn", SATURN_COLOR, SATURN_MASS, SATURN_OBSERVED_RADIUS),
    ];
    svg(600, 420, |area| {
        area.fill(&WHITE).map_err(draw_error)?;
        let mut chart = ChartBuilder::on(&area)
            .caption(
                "Radius of giant planets at 4.5 Gyr, 5.2 au",
                ("sans-serif", 18),
            )
            .margin(12)
            .x_label_area_size(42)
            .y_label_area_size(62)
            .build_cartesian_2d(x_range.log_scale(), y_range)
            .map_err(draw_error)?;
        chart
            .configure_mesh()
            .x_desc("mass M / M_Jup")
            .y_desc("radius at 1 bar R / R_Jup")
            .x_labels(10)
            .y_labels(10)
            .x_label_formatter(&tick)
            .label_style(("sans-serif", 13))
            .axis_desc_style(("sans-serif", 14))
            .light_line_style(RGBColor(0xee, 0xee, 0xee))
            .bold_line_style(RGBColor(0xcc, 0xcc, 0xcc))
            .draw()
            .map_err(draw_error)?;
        for (data, (core, color)) in curves.iter().zip(CORES.iter().zip(CORE_COLORS)) {
            chart
                .draw_series(LineSeries::new(data.iter().copied(), color.stroke_width(2)))
                .map_err(draw_error)?
                .label(format!("core {core} M⊕"))
                .legend(move |(x, y)| {
                    PathElement::new([(x, y), (x + 18, y)], color.stroke_width(2))
                });
        }
        for (name, color, mass, radius) in solar_system {
            let point = (mass.0 / jupiter_mass().0, radius.0 / JUPITER_RADIUS.0);
            chart
                .draw_series(PointSeries::of_element([point], 5, color, &|c, s, st| {
                    EmptyElement::at(c) + Circle::new((0, 0), s, st.filled())
                }))
                .map_err(draw_error)?
                .label(name)
                .legend(move |(x, y)| Circle::new((x + 9, y), 4, color.filled()));
        }
        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::LowerRight)
            .background_style(WHITE.mix(0.85))
            .border_style(RGBColor(0xaa, 0xaa, 0xaa))
            .label_font(("sans-serif", 13))
            .draw()
            .map_err(draw_error)?;
        area.present().map_err(draw_error)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use science::giants::core_bounds;

    #[test]
    fn out_of_table_masses_are_skipped() {
        let s = series(25.0);
        assert!(!s.is_empty());
        let earth = |jup: f64| jup * jupiter_mass().0 / Kilogram::from(EarthMass(1.0)).0;
        let first = earth(s[0].0);
        assert!(first > MIN_MASS * 1.0001, "starts at {first} M⊕");
        // The first drawn mass is one the tables can serve with that core.
        let (_, highest) = core_bounds(EarthMass(first)).expect("tabulated");
        assert!(highest.0 >= 25.0, "core limit {} at {first}", highest.0);
        for p in &s {
            assert!(p.0.is_finite() && p.1.is_finite() && p.1 > 0.0);
        }
    }

    #[test]
    fn the_figure_renders() {
        let svg = giant_radius_figure().unwrap();
        assert!(svg.contains("Jupiter"));
        assert!(svg.contains("Saturn"));
        assert!(!svg.contains("NaN"));
    }
}
