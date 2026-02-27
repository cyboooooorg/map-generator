use crate::world::*;
use font8x8::UnicodeFonts;
use image::{Rgb, RgbImage};

// Contour lines are drawn whenever a tile and a neighbour straddle one of these levels.
const CONTOUR_LEVELS: &[f32] = &[-0.45, -0.15, 0.0, 0.15, 0.30, 0.45, 0.60, 0.75, 0.90];
// Fraction to darken a pixel by when it sits on a contour line (0.0 = no change, 1.0 = black).
const CONTOUR_DARKNESS: f32 = 0.40;

pub fn export_png(world: &World, path: &str) {
    let w = world.width as u32;
    let h = world.height as u32;
    let mut img = RgbImage::new(w, h);

    // Build a fast elevation lookup indexed by [q * height + r].
    let mut elevation: Vec<f32> = vec![0.0; (w * h) as usize];
    for tile in &world.tiles {
        elevation[(tile.q as u32 * h + tile.r as u32) as usize] = tile.elevation;
    }

    let elev_at = |q: i32, r: i32| -> Option<f32> {
        if q < 0 || r < 0 || q >= world.width || r >= world.height {
            return None;
        }
        Some(elevation[(q as u32 * h + r as u32) as usize])
    };

    // Returns true if the edge between elevations `a` and `b` crosses any contour level.
    let crosses_contour =
        |a: f32, b: f32| -> bool { CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl)) };

    for tile in &world.tiles {
        let mut color = biome_color(tile.biome);
        let e = tile.elevation;

        // Check the 4-connected neighbours.
        let is_contour = [
            elev_at(tile.q - 1, tile.r),
            elev_at(tile.q + 1, tile.r),
            elev_at(tile.q, tile.r - 1),
            elev_at(tile.q, tile.r + 1),
        ]
        .iter()
        .filter_map(|n| *n)
        .any(|ne| crosses_contour(e, ne));

        if is_contour {
            // Darken the biome colour proportionally.
            color = color.map(|c| (c as f32 * (1.0 - CONTOUR_DARKNESS)) as u8);
        }

        img.put_pixel(tile.q as u32, tile.r as u32, Rgb(color));
    }

    // ── Overlay equator and tropic reference lines (dotted) ───────────────────
    // Latitude → row: r = height * (0.5 + lat_deg / 180)
    let line_rows: &[(f64, [u8; 3])] = &[
        (h as f64 * 0.5, [220, 50, 50]),                  // equator — red
        (h as f64 * (0.5 + 23.5 / 180.0), [220, 150, 0]), // Tropic of Cancer — amber
        (h as f64 * (0.5 - 23.5 / 180.0), [220, 150, 0]), // Tropic of Capricorn — amber
        (h as f64 * (0.5 + 66.5 / 180.0), [0, 200, 240]), // Arctic Circle — cyan
        (h as f64 * (0.5 - 66.5 / 180.0), [0, 200, 240]), // Antarctic Circle — cyan
    ];
    // Dash pattern: 6 px on, 4 px off
    const DASH_ON: u32 = 6;
    const DASH_OFF: u32 = 4;
    const PERIOD: u32 = DASH_ON + DASH_OFF;

    for &(row_f, color) in line_rows {
        let row = row_f.round() as u32;
        if row >= h {
            continue;
        }
        for x in 0..w {
            if x % PERIOD < DASH_ON {
                img.put_pixel(x, row, Rgb(color));
            }
        }
    }

    img.save(path).unwrap();
}

// ── Legend PNG ────────────────────────────────────────────────────────────────

/// Scale factor for the bitmap font (each logical pixel becomes `SCALE` screen pixels).
const FONT_SCALE: u32 = 2;
/// Width of one character in screen pixels.
const CHAR_W: u32 = 8 * FONT_SCALE;
/// Height of one character in screen pixels.
const CHAR_H: u32 = 8 * FONT_SCALE;

/// Draw a single character at (x, y) using the 8×8 bitmap font.
fn draw_char(img: &mut RgbImage, c: char, x: u32, y: u32, color: [u8; 3]) {
    let Some(glyph) = font8x8::BASIC_FONTS.get(c) else {
        return;
    };
    for (row, &byte) in glyph.iter().enumerate() {
        for col in 0u32..8 {
            if byte & (1 << col) != 0 {
                for dy in 0..FONT_SCALE {
                    for dx in 0..FONT_SCALE {
                        let px = x + col * FONT_SCALE + dx;
                        let py = y + row as u32 * FONT_SCALE + dy;
                        if px < img.width() && py < img.height() {
                            img.put_pixel(px, py, Rgb(color));
                        }
                    }
                }
            }
        }
    }
}

/// Draw a string starting at (x, y).
fn draw_str(img: &mut RgbImage, s: &str, x: u32, y: u32, color: [u8; 3]) {
    for (i, c) in s.chars().enumerate() {
        draw_char(img, c, x + i as u32 * CHAR_W, y, color);
    }
}

/// Fill a rectangular area with `color`.
fn fill_rect(img: &mut RgbImage, x: u32, y: u32, w: u32, h: u32, color: [u8; 3]) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, Rgb(color));
            }
        }
    }
}

/// Draw a 1-pixel border around a rectangle.
fn outline_rect(img: &mut RgbImage, x: u32, y: u32, w: u32, h: u32, color: [u8; 3]) {
    for dx in 0..w {
        img.put_pixel(x + dx, y, Rgb(color));
        img.put_pixel(x + dx, y + h - 1, Rgb(color));
    }
    for dy in 0..h {
        img.put_pixel(x, y + dy, Rgb(color));
        img.put_pixel(x + w - 1, y + dy, Rgb(color));
    }
}

/// Generate a legend PNG listing every biome that actually appears on the map.
pub fn export_legend_png(world: &World, path: &str) {
    // ── Collect biomes present on this map, in canonical order ────────────────
    let mut seen = std::collections::HashSet::new();
    let mut biomes: Vec<Biome> = Vec::new();
    for tile in &world.tiles {
        if seen.insert(tile.biome) {
            biomes.push(tile.biome);
        }
    }
    biomes.sort_by_key(|&b| biome_order(b));

    // ── Planet metadata lines (key, value) ───────────────────────────────────
    // Capitalize the planet type name for display.
    let planet_str = {
        let s = format!("{}", world.planet_type);
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    };
    let meta: &[(&str, String)] = &[
        ("Planet", planet_str),
        ("Seed", format!("{}", world.seed)),
        ("Sea level", format!("{:+.2}", world.sea_level)),
        ("Volcanic", format!("{:.2}", world.volcanic_intensity)),
        ("Circumference", format!("{:.0} km", world.circumference_km)),
        ("Gravity", format!("{:.2} g", world.gravity_modifier)),
    ];

    // ── Layout constants ──────────────────────────────────────────────────────
    const PAD: u32 = 14;
    const SWATCH_W: u32 = 48;
    const SWATCH_H: u32 = CHAR_H;
    const SWATCH_GAP: u32 = 8;
    const ROW_H: u32 = SWATCH_H + 6;
    const META_ROW_H: u32 = CHAR_H + 5;
    // Space added before and after each horizontal divider line.
    const SECTION_GAP: u32 = 8;

    // Width is the maximum of: title, metadata block, biome block.
    let title = "BIOME LEGEND";
    let max_biome_len = biomes
        .iter()
        .map(|&b| biome_name(b).len())
        .max()
        .unwrap_or(10) as u32;
    let biome_col_w = SWATCH_W + SWATCH_GAP + max_biome_len * CHAR_W;

    // For metadata we align values at a fixed column (longest key + ": ").
    let max_key_len = meta.iter().map(|(k, _)| k.len()).max().unwrap_or(0) as u32;
    let key_col_chars = max_key_len + 2; // +2 for ": "
    let max_val_len = meta.iter().map(|(_, v)| v.len()).max().unwrap_or(0) as u32;
    let meta_col_w = (key_col_chars + max_val_len) * CHAR_W;

    let content_w = biome_col_w.max(meta_col_w).max(title.len() as u32 * CHAR_W);
    let img_w = PAD + content_w + PAD;

    // Height = title + meta section (2 dividers + rows) + biome rows.
    let divider_block_h = SECTION_GAP + 1 + SECTION_GAP; // gap · line · gap
    let img_h = PAD
        + CHAR_H                                          // title
        + divider_block_h                                 // divider above meta
        + meta.len() as u32 * META_ROW_H                 // meta rows
        + divider_block_h                                 // divider below meta
        + biomes.len() as u32 * ROW_H                    // biome rows
        + PAD;

    const BG: [u8; 3] = [22, 22, 35];
    const TITLE_COLOR: [u8; 3] = [240, 240, 240];
    const KEY_COLOR: [u8; 3] = [140, 155, 190];
    const VAL_COLOR: [u8; 3] = [220, 225, 240];
    const TEXT_COLOR: [u8; 3] = [210, 210, 210];
    const BORDER_COLOR: [u8; 3] = [80, 80, 100];
    const DIVIDER_COLOR: [u8; 3] = [55, 60, 88];

    let mut img = RgbImage::from_pixel(img_w, img_h, Rgb(BG));

    // ── Title ─────────────────────────────────────────────────────────────────
    let title_x = (img_w.saturating_sub(title.len() as u32 * CHAR_W)) / 2;
    draw_str(&mut img, title, title_x, PAD, TITLE_COLOR);
    let mut y = PAD + CHAR_H;

    // ── Helper: draw a horizontal divider line ────────────────────────────────
    let draw_divider = |img: &mut RgbImage, y: u32| {
        for x in PAD..img_w.saturating_sub(PAD) {
            img.put_pixel(x, y, Rgb(DIVIDER_COLOR));
        }
    };

    // ── Metadata section ──────────────────────────────────────────────────────
    y += SECTION_GAP;
    draw_divider(&mut img, y);
    y += 1 + SECTION_GAP;

    let val_x = PAD + key_col_chars * CHAR_W;
    for (key, val) in meta {
        let label = format!("{}: ", key);
        draw_str(&mut img, &label, PAD, y, KEY_COLOR);
        draw_str(&mut img, val, val_x, y, VAL_COLOR);
        y += META_ROW_H;
    }

    y += SECTION_GAP;
    draw_divider(&mut img, y);
    y += 1 + SECTION_GAP;

    // ── One row per biome ─────────────────────────────────────────────────────
    for &b in &biomes {
        let color = biome_color(b);
        fill_rect(&mut img, PAD, y, SWATCH_W, SWATCH_H, color);
        outline_rect(&mut img, PAD, y, SWATCH_W, SWATCH_H, BORDER_COLOR);
        draw_str(
            &mut img,
            biome_name(b),
            PAD + SWATCH_W + SWATCH_GAP,
            y,
            TEXT_COLOR,
        );
        y += ROW_H;
    }

    img.save(path).unwrap();
}
