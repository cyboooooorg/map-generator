use super::{DASH_OFF, DASH_ON, Style, latitude_row, reference_lines, render_pixels};
use crate::world::World;
use std::fmt::Write;

/// Writes the biome map as SVG, with the same contour and reference lines as the PNG.
///
/// Without hillshading: shading gives nearly every pixel its own colour, which
/// would defeat the run-length encoding below.
///
/// Each row is run-length encoded: consecutive pixels sharing a colour become a
/// single wider `<rect>`, which keeps the file 10–40× smaller than one rect per pixel.
pub fn export_svg(world: &World, path: &str) -> std::io::Result<()> {
    let (w, h) = (world.width as usize, world.height as usize);
    let pixels = render_pixels(world, Style::Flat);

    let mut out = String::with_capacity(w * h * 8);
    // Writing into a String cannot fail, so the fmt::Result values are ignored.
    let _ = writeln!(out, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
    );

    for (y, row) in pixels.chunks_exact(w).enumerate() {
        let mut x = 0;
        for run in row.chunk_by(|a, b| a == b) {
            let [cr, cg, cb] = run[0];
            let len = run.len();
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{len}" height="1" fill="#{cr:02X}{cg:02X}{cb:02X}"/>"##
            );
            x += len;
        }
    }

    for (latitude, [cr, cg, cb]) in reference_lines(world.params.axial_tilt_deg) {
        let y = latitude_row(h as u32, latitude);
        let _ = writeln!(
            out,
            r##"<line x1="0" y1="{y:.1}" x2="{w}" y2="{y:.1}" stroke="#{cr:02X}{cg:02X}{cb:02X}" stroke-width="1" stroke-dasharray="{DASH_ON},{DASH_OFF}" opacity="0.75"/>"##
        );
    }

    out.push_str("</svg>\n");
    std::fs::write(path, out)
}
