# `figures`: the science book's figures

Draws the figures of the science book (`docs/science`) as SVG, from the
laws of the `science` crate through its public API. A figure shows what
the code computes, so the page and the engine cannot drift apart: a
change of a law changes the figure, and `mise run docs:check` fails until
the regenerated file is committed.

## Prerequisites

The Rust toolchain of the workspace and `mise`. Nothing else: the SVG is
written by the `plotters` crate, with no bitmap backend and no system
font.

## Usage

```bash
mise run docs:figures                  # write docs/science/figures/*.svg
mise run docs:figures -- --check       # compare only; exit 1 on a stale file
cargo run -q -p figures -- --out DIR   # write somewhere else
```

`--check` writes nothing: it prints each missing or stale figure by name
and exits 1. `mise run docs:check` runs it first.

## Adding a figure

1. Write a function `fn my_figure() -> Result<String, FigureError>` that
   draws through `svg(width, height, |area| …)`. Never build an
   `SVGBackend` directly: `svg` registers the font.
2. Add one `Figure { name, render }` entry to `figures()` in `src/lib.rs`.
3. Read laws only through the public API of `science`. A number that
   belongs to the figure itself (a reference body, say) is a `const`
   whose doc comment starts with `Source:`.
4. Run `mise run docs:figures`, commit the SVG, and link it from its page
   in `docs/science/` with `![…](figures/<name>.svg)`.

Never edit an SVG by hand: the next run overwrites it.

## Why the font is committed

SVG text is laid out from font metrics. With a system font, macOS and the
CI runner would measure the same label differently and the files would
differ byte for byte, so `--check` would fail on one of them. The binary
therefore carries one font and registers it as `sans-serif`:

- Noto Sans Regular 2.015, the `unhinted/ttf` file of the
  [NotoSans-v2.015 release](https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSans-v2.015)
  of `notofonts/latin-greek-cyrillic`;
- licence: SIL Open Font License 1.1, in `fonts/OFL.txt`.

To change the font, replace both files, update this section and
regenerate the figures.
