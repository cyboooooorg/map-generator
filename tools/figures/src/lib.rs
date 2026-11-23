//! Figures of the science book (`docs/science`), drawn as SVG from the laws
//! of the `science` crate. See README.md.

use std::error::Error;
use std::fmt;
use std::path::Path;

use std::sync::OnceLock;

use plotters::backend::SVGBackend;
use plotters::coord::Shift;
use plotters::drawing::{DrawingArea, IntoDrawingArea};
use plotters::style::{FontStyle, register_font};

/// Noto Sans Regular 2.015 (SIL OFL 1.1, see fonts/OFL.txt): text is
/// measured with this font on every machine, never with a system one.
const FONT_BYTES: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// One figure: the file stem under `docs/science/figures/` and its drawing.
pub struct Figure {
    pub name: &'static str,
    pub render: fn() -> Result<String, FigureError>,
}

/// What can go wrong while drawing or comparing a figure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FigureError {
    /// The drawing backend refused an element.
    Draw(String),
    /// A figure file could not be read, written or listed.
    Io(String),
}

impl fmt::Display for FigureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draw(m) => write!(f, "drawing failed: {m}"),
            Self::Io(m) => write!(f, "file error: {m}"),
        }
    }
}

impl Error for FigureError {}

/// Every figure of the book. Each one is added here by its own function.
pub fn figures() -> Vec<Figure> {
    Vec::new()
}

/// The figures whose file under `dir` is missing or differs from a fresh
/// drawing, in the order of [`figures`]. A missing `dir` makes all stale.
pub fn stale(dir: &Path) -> Result<Vec<&'static str>, FigureError> {
    stale_of(&figures(), dir)
}

fn stale_of(figures: &[Figure], dir: &Path) -> Result<Vec<&'static str>, FigureError> {
    let mut stale = Vec::new();
    for figure in figures {
        let fresh = (figure.render)()?;
        let path = dir.join(format!("{}.svg", figure.name));
        match std::fs::read_to_string(&path) {
            Ok(on_disk) if on_disk == fresh => {}
            Ok(_) => stale.push(figure.name),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => stale.push(figure.name),
            Err(e) => return Err(FigureError::Io(format!("{}: {e}", path.display()))),
        }
    }
    Ok(stale)
}

/// Writes every figure as `<name>.svg` under `dir`, creating it.
pub fn write(dir: &Path) -> Result<Vec<&'static str>, FigureError> {
    let io = |path: &Path, e: std::io::Error| FigureError::Io(format!("{}: {e}", path.display()));
    std::fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    let mut written = Vec::new();
    for figure in figures() {
        let path = dir.join(format!("{}.svg", figure.name));
        std::fs::write(&path, (figure.render)()?).map_err(|e| io(&path, e))?;
        written.push(figure.name);
    }
    Ok(written)
}

/// Renders a `width` by `height` SVG through `draw`, after registering the
/// committed font once: every figure draws through it, so none can measure
/// text with a system font.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "drawn through by the figures added after this one"
    )
)]
pub(crate) fn svg(
    width: u32,
    height: u32,
    draw: impl FnOnce(DrawingArea<SVGBackend, Shift>) -> Result<(), FigureError>,
) -> Result<String, FigureError> {
    static FONT: OnceLock<Result<(), String>> = OnceLock::new();
    FONT.get_or_init(|| {
        register_font("sans-serif", FontStyle::Normal, FONT_BYTES)
            .map_err(|_| "the committed font is not a valid font file".to_owned())
    })
    .clone()
    .map_err(FigureError::Draw)?;
    let mut out = String::new();
    {
        let area = SVGBackend::with_string(&mut out, (width, height)).into_drawing_area();
        draw(area)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotters::prelude::*;

    /// A test-only figure: one line and a text label.
    fn sample() -> Result<String, FigureError> {
        svg(200, 100, |area| {
            area.fill(&WHITE)
                .map_err(|e| FigureError::Draw(e.to_string()))?;
            let mut chart = ChartBuilder::on(&area)
                .margin(10)
                .build_cartesian_2d(0.0..1.0, 0.0..1.0)
                .map_err(|e| FigureError::Draw(e.to_string()))?;
            chart
                .draw_series(LineSeries::new([(0.0, 0.0), (1.0, 1.0)], &BLACK))
                .map_err(|e| FigureError::Draw(e.to_string()))?;
            area.draw(&Text::new("label", (20, 20), ("sans-serif", 14)))
                .map_err(|e| FigureError::Draw(e.to_string()))?;
            area.present().map_err(|e| FigureError::Draw(e.to_string()))
        })
    }

    #[test]
    fn the_committed_font_measures_text() {
        use plotters::style::{FontTransform, IntoFont};
        sample().unwrap();
        let (width, height) = ("sans-serif", 14)
            .into_font()
            .transform(FontTransform::None)
            .box_size("label")
            .expect("the registered font measures text");
        assert!(width > 10 && height > 5, "{width}x{height}");
    }

    #[test]
    fn rendering_twice_gives_the_same_bytes() {
        assert_eq!(sample().unwrap(), sample().unwrap());
    }

    #[test]
    fn text_renders_without_a_system_font() {
        assert!(sample().unwrap().contains("<text"));
    }

    fn only_figure() -> Vec<Figure> {
        vec![Figure {
            name: "sample",
            render: sample,
        }]
    }

    #[test]
    fn a_missing_file_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(stale_of(&only_figure(), dir.path()).unwrap(), ["sample"]);
        assert_eq!(
            stale_of(&only_figure(), &dir.path().join("absent")).unwrap(),
            ["sample"]
        );
    }

    #[test]
    fn a_changed_file_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sample.svg"), "<svg/>").unwrap();
        assert_eq!(stale_of(&only_figure(), dir.path()).unwrap(), ["sample"]);
    }

    #[test]
    fn a_fresh_file_is_not_stale() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sample.svg"), sample().unwrap()).unwrap();
        assert!(stale_of(&only_figure(), dir.path()).unwrap().is_empty());
    }

    #[test]
    fn figure_names_are_unique() {
        let mut names: Vec<_> = figures().iter().map(|f| f.name).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n);
    }
}
