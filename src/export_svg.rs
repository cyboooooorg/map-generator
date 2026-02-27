use crate::world::*;
use std::io::Write;

// Mirror the same contour parameters used by the PNG exporter.
const CONTOUR_LEVELS: &[f32] = &[-0.45, -0.15, 0.0, 0.15, 0.30, 0.45, 0.60, 0.75, 0.90];
const CONTOUR_DARKNESS: f32 = 0.40;

pub fn export_svg(world: &World, path: &str) {
    let w = world.width as usize;
    let h = world.height as usize;

    // ── 1. Pre-compute elevation grid (column-major: index = q * h + r) ──────
    let mut elevation = vec![0.0f32; w * h];
    for tile in &world.tiles {
        elevation[tile.q as usize * h + tile.r as usize] = tile.elevation;
    }

    let elev_at = |q: i32, r: i32| -> Option<f32> {
        if q < 0 || r < 0 || q >= world.width || r >= world.height {
            return None;
        }
        Some(elevation[q as usize * h + r as usize])
    };

    let crosses_contour =
        |a: f32, b: f32| -> bool { CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl)) };

    // ── 2. Pre-compute final pixel colours (biome + contour darkening) ────────
    let mut pixel_color = vec![[0u8; 3]; w * h];
    for tile in &world.tiles {
        let mut color = biome_color(tile.biome);
        let e = tile.elevation;
        let is_contour = [
            elev_at(tile.q - 1, tile.r),
            elev_at(tile.q + 1, tile.r),
            elev_at(tile.q, tile.r - 1),
            elev_at(tile.q, tile.r + 1),
        ]
        .into_iter()
        .flatten()
        .any(|ne| crosses_contour(e, ne));

        if is_contour {
            color = color.map(|c| (c as f32 * (1.0 - CONTOUR_DARKNESS)) as u8);
        }
        pixel_color[tile.q as usize * h + tile.r as usize] = color;
    }

    // ── 3. Build SVG with per-row run-length encoding ─────────────────────────
    // Each row (fixed r, varying q) is scanned left-to-right; consecutive pixels
    // sharing the same colour are merged into a single wider <rect>.  This keeps
    // the file size manageable (~10-40× fewer elements than one rect per pixel).
    let mut out: Vec<u8> = Vec::with_capacity(w * h * 48);

    writeln!(out, r#"<?xml version="1.0" encoding="UTF-8"?>"#).unwrap();
    writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#
    )
    .unwrap();

    for r in 0..h {
        let mut run_start = 0usize;
        let mut run_color = pixel_color[0 * h + r];

        for q in 1..=w {
            // Sentinel colour that can never equal the real last colour so
            // the final run is always flushed without special-casing after the loop.
            let cur = if q < w {
                pixel_color[q * h + r]
            } else {
                [
                    run_color[0] ^ 0xFF,
                    run_color[1] ^ 0xFF,
                    run_color[2] ^ 0xFF,
                ]
            };

            if cur != run_color {
                let run_len = q - run_start;
                let [cr, cg, cb] = run_color;
                writeln!(
                    out,
                    r##"<rect x="{run_start}" y="{r}" width="{run_len}" height="1" fill="#{cr:02X}{cg:02X}{cb:02X}"/>"##,
                )
                .unwrap();
                run_start = q;
                run_color = cur;
            }
        }
    }

    writeln!(out, "</svg>").unwrap();

    std::fs::write(path, &out).expect("failed to write SVG");
}
