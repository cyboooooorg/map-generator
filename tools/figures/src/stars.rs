//! The main-sequence figure: luminosity and radius against mass, from the
//! stellar grid of `science::stellar`.

use plotters::prelude::*;
use science::stellar::{life, main_sequence};
use science::units::{Dex, Gyr, SolarMass};

use plotters::backend::SVGBackend;
use plotters::coord::Shift;

use crate::{FigureError, svg};

/// Where in its main-sequence life a star is drawn.
#[derive(Clone, Copy)]
enum Stage {
    /// Zero-age main sequence.
    Zams,
    /// Halfway between the ZAMS and the end of the main sequence.
    MidLife,
}

const MIN_MASS: f64 = 0.1;
const MAX_MASS: f64 = 8.0;
const POINTS: usize = 120;

/// Mass, luminosity and radius (solar units) of solar-metallicity stars from
/// 0.1 to 8 M☉, log-spaced. A mass for which the tables have no answer is
/// skipped.
fn series(stage: Stage) -> Vec<(f64, f64, f64)> {
    let metallicity = Dex(0.0);
    (0..POINTS)
        .filter_map(|i| {
            let mass = MIN_MASS * (MAX_MASS / MIN_MASS).powf(i as f64 / (POINTS - 1) as f64);
            let life = life(SolarMass(mass), metallicity).ok()?;
            let (zams, end) = (life.zams.0, life.end.age().0);
            let age = match stage {
                Stage::Zams => zams,
                Stage::MidLife => zams + 0.5 * (end - zams),
            };
            let star = main_sequence(SolarMass(mass), metallicity, Gyr(age)).ok()?;
            Some((mass, star.luminosity.0, star.radius.0))
        })
        .collect()
}

fn draw_error(e: impl std::fmt::Display) -> FigureError {
    FigureError::Draw(e.to_string())
}

const ZAMS_COLOR: RGBColor = RGBColor(0x1f, 0x77, 0xb4);
const MID_COLOR: RGBColor = RGBColor(0xd6, 0x5f, 0x0e);
const SUN_COLOR: RGBColor = RGBColor(0x20, 0x20, 0x20);

/// The smallest range of whole decades holding every value.
fn decades(values: impl Iterator<Item = f64> + Clone) -> std::ops::Range<f64> {
    let lo = values.clone().fold(f64::INFINITY, f64::min);
    let hi = values.fold(f64::NEG_INFINITY, f64::max);
    10f64.powf(lo.log10().floor())..10f64.powf(hi.log10().ceil())
}

/// A tick label without a spurious decimal: `0.1`, `1`, `2`, `10000`.
fn tick(value: &f64) -> String {
    format!("{value}")
}

/// One log-log panel: `pick` reads the plotted quantity of a sample.
fn panel(
    area: &DrawingArea<SVGBackend, Shift>,
    title: &str,
    y_label: &str,
    zams: &[(f64, f64, f64)],
    mid: &[(f64, f64, f64)],
    pick: fn(&(f64, f64, f64)) -> f64,
) -> Result<(), FigureError> {
    // Padded so that the end ticks (0.1, 10, ...) fall inside the axes:
    // plotters drops a label that sits exactly on the boundary.
    let decades = decades(zams.iter().chain(mid).map(pick));
    let y_range = decades.start * 0.8..decades.end * 1.25;
    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 18))
        .margin(12)
        .x_label_area_size(42)
        .y_label_area_size(62)
        .build_cartesian_2d(
            (MIN_MASS * 0.8..MAX_MASS * 1.25).log_scale(),
            y_range.log_scale(),
        )
        .map_err(draw_error)?;
    chart
        .configure_mesh()
        .x_desc("mass M / M☉")
        .y_desc(y_label)
        .x_labels(12)
        .y_labels(12)
        .x_label_formatter(&tick)
        .y_label_formatter(&tick)
        .label_style(("sans-serif", 13))
        .axis_desc_style(("sans-serif", 14))
        .light_line_style(RGBColor(0xee, 0xee, 0xee))
        .bold_line_style(RGBColor(0xcc, 0xcc, 0xcc))
        .draw()
        .map_err(draw_error)?;
    for (data, color, label) in [(zams, ZAMS_COLOR, "ZAMS"), (mid, MID_COLOR, "mid-life")] {
        chart
            .draw_series(LineSeries::new(
                data.iter().map(|p| (p.0, pick(p))),
                color.stroke_width(2),
            ))
            .map_err(draw_error)?
            .label(label)
            .legend(move |(x, y)| PathElement::new([(x, y), (x + 18, y)], color.stroke_width(2)));
    }
    chart
        .draw_series(PointSeries::of_element(
            [(1.0, 1.0)],
            5,
            SUN_COLOR,
            &|c, s, st| EmptyElement::at(c) + Circle::new((0, 0), s, st.filled()),
        ))
        .map_err(draw_error)?
        .label("Sun")
        .legend(|(x, y)| Circle::new((x + 9, y), 4, SUN_COLOR.filled()));
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperLeft)
        .background_style(WHITE.mix(0.85))
        .border_style(RGBColor(0xaa, 0xaa, 0xaa))
        .label_font(("sans-serif", 13))
        .draw()
        .map_err(draw_error)?;
    Ok(())
}

pub(crate) fn main_sequence_figure() -> Result<String, FigureError> {
    let zams = series(Stage::Zams);
    let mid = series(Stage::MidLife);
    svg(900, 420, |area| {
        area.fill(&WHITE).map_err(draw_error)?;
        let (left, right) = area.split_horizontally(450);
        panel(
            &left,
            "Luminosity of main-sequence stars",
            "luminosity L / L☉",
            &zams,
            &mid,
            |p| p.1,
        )?;
        panel(
            &right,
            "Radius of main-sequence stars",
            "radius R / R☉",
            &zams,
            &mid,
            |p| p.2,
        )?;
        area.present().map_err(draw_error)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAGES: [Stage; 2] = [Stage::Zams, Stage::MidLife];

    #[test]
    fn the_curves_cover_the_mass_range() {
        for stage in STAGES {
            let s = series(stage);
            assert!(s.len() >= 60, "{} points", s.len());
            let (first, last) = (s[0].0, s[s.len() - 1].0);
            assert!((first - MIN_MASS).abs() < 1e-9, "starts at {first}");
            assert!((last - MAX_MASS).abs() < 1e-9, "ends at {last}");
            let ratios: Vec<f64> = s.windows(2).map(|w| w[1].0 / w[0].0).collect();
            let first_ratio = ratios[0];
            assert!(
                ratios.iter().all(|r| (r / first_ratio - 1.0).abs() < 1e-9),
                "not log-spaced"
            );
        }
    }

    #[test]
    fn luminosity_rises_with_mass() {
        for stage in STAGES {
            for w in series(stage).windows(2) {
                assert!(
                    w[1].1 > w[0].1,
                    "L({}) = {} does not exceed L({}) = {}",
                    w[1].0,
                    w[1].1,
                    w[0].0,
                    w[0].1
                );
            }
        }
    }

    #[test]
    fn the_figure_renders() {
        let svg = main_sequence_figure().unwrap();
        assert!(svg.contains("Sun"));
        assert!(!svg.contains("NaN"));
    }
}
