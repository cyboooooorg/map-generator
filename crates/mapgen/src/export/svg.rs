use super::{DASH_OFF, DASH_ON, MapLayers, Style, latitude_row, reference_lines, render_pixels};
use crate::rivers;
use crate::world::{Biome, World, biome_color};
use std::fs::File;
use std::io::{BufWriter, Write};

/// Writes the biome map as SVG, with the same contour and reference lines as the PNG.
///
/// Without hillshading: shading gives nearly every pixel its own colour, which
/// would defeat the run-length encoding below.
///
/// Each row is run-length encoded: consecutive pixels sharing a colour become a
/// single wider `<rect>`, which keeps the file 10–40× smaller than one rect per pixel.
/// The file is streamed to disk rather than built in memory.
pub fn export_svg(world: &World, layers: &MapLayers, path: &str) -> std::io::Result<()> {
    let (w, h) = (world.width as usize, world.height as usize);
    let pixels = render_pixels(world, layers, Style::Flat);

    let mut out = BufWriter::new(File::create(path)?);
    writeln!(out, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
    )?;

    for (y, row) in pixels.chunks_exact(w).enumerate() {
        let mut x = 0;
        for run in row.chunk_by(|a, b| a == b) {
            let [cr, cg, cb] = run[0];
            let len = run.len();
            writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{len}" height="1" fill="#{cr:02X}{cg:02X}{cb:02X}"/>"##
            )?;
            x += len;
        }
    }
    drop(pixels);

    // Rivers as stroked paths, in short chunks so the width follows the flow.
    let planet = &world.params.planet;
    let [rr, rg, rb] = biome_color(planet.remap(Biome::River));
    let chunk = 8;
    for line in &world.rivers {
        let widths = rivers::widths_px(line, world.width, world.params.circumference_km as f64);
        let mut start = 0;
        while start + 1 < line.points.len() {
            let stop = (start + chunk).min(line.points.len() - 1);
            let points = &line.points[start..=stop];
            let mid = points[points.len() / 2];
            let (mx, my) = (
                (mid[0].rem_euclid(w as f64) as usize).min(w - 1),
                (mid[1] as usize).min(h - 1),
            );
            // Frozen rivers and rivers under lava are not drawn, as in the PNG.
            let i = my * w + mx;
            let lava = planet.rules.choose(world.biome_input(i)).is_lava();
            if world.temperature[i] >= planet.rules.polar && !lava {
                let width = widths[start..=stop].iter().sum::<f64>() / (stop - start + 1) as f64;
                let min_x = points.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
                let max_x = points.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
                // Draw again one map width away where the chunk crosses the seam.
                for shift in [-(w as f64), 0.0, w as f64] {
                    if max_x + shift < -width || min_x + shift > w as f64 + width {
                        continue;
                    }
                    write!(out, r#"<polyline points=""#)?;
                    for (k, p) in points.iter().enumerate() {
                        let sep = if k == 0 { "" } else { " " };
                        write!(out, "{sep}{:.2},{:.2}", p[0] + shift, p[1])?;
                    }
                    writeln!(
                        out,
                        r##"" fill="none" stroke="#{rr:02X}{rg:02X}{rb:02X}" stroke-width="{width:.2}" stroke-linecap="round" stroke-linejoin="round"/>"##
                    )?;
                }
            }
            start = stop;
        }
    }

    for (latitude, [cr, cg, cb]) in reference_lines(world.params.axial_tilt_deg) {
        let y = latitude_row(h as u32, latitude);
        writeln!(
            out,
            r##"<line x1="0" y1="{y:.1}" x2="{w}" y2="{y:.1}" stroke="#{cr:02X}{cg:02X}{cb:02X}" stroke-width="1" stroke-dasharray="{DASH_ON},{DASH_OFF}" opacity="0.75"/>"##
        )?;
    }

    writeln!(out, "</svg>")?;
    out.flush()
}
