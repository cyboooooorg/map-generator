# Science reference site and generated figures

Short design. Implements the part of the
[architecture rewrite](2026-11-20-architecture-rewrite-design.md) that
publishes `docs/science/` as a site, which slice 1 planned with its PR 2
and did not deliver, and starts the generated figures (`docs:figures`).

## Intent

Read the science articles as a site, with equations, diagrams and
figures rendered, rebuilt on every merge into `main`. Figures of laws
are drawn from the code's own output, so they follow the code.

## Goals

- `docs/science/` built with mdBook and deployed to GitHub Pages
  (source: GitHub Actions) on every merge into `main`; built, not
  deployed, on every pull request.
- Equations (`$…$`, `$$…$$`) and Mermaid diagrams render in the site
  and still render on GitHub.
- Links resolve: links leaving `docs/science/` point to GitHub.
- A Rust tool draws figures from the `science` crate's public laws; the
  figures are committed and CI fails when they are stale.
- Two first figures, each with sourced reference points.
- The pull request documents the site, the figures, how to add one.

## Non-goals

- Rewriting the legacy pages to the new writing rules (done as laws are
  ported).
- The CI check that every page has a non-empty References section: an
  issue for later.
- Figures for laws not yet ported.

## Anti-goals

- A figure drawn by hand, or from values recomputed outside `science`.
- A reference point without a source.
- Figures whose bytes depend on the machine (system fonts).

## Approaches

- **A, chosen: a Rust tool `tools/figures` with `plotters` (SVG
  backend).** It calls the public laws directly, stays in typed Rust
  like the other tools, and needs no other runtime in CI.
- B: a Rust binary exporting JSON plotted by Python/matplotlib. Nicer
  plots for less effort, but two languages and a Python environment in
  CI; rejected.
- C: figures generated only when the site is built. GitHub would show
  the pages without their figures; rejected.

## Decided

- **Book**: `docs/science/book.toml` with the pages in place
  (`src = "."`), a `SUMMARY.md` mirroring the README's topics, output in
  `target/book` (ignored by git).
- **Equations**: `mdbook-katex`. The ` ```math ` blocks are converted once
  to `$$…$$`, which GitHub renders too.
- **Diagrams**: `mdbook-mermaid`.
- **Outbound links**: links leaving `docs/science/` become absolute
  `https://github.com/cyboooooorg/map-generator/blob/main/…` links; a link
  checker runs in CI.
- **Tasks**: `mise run docs:build` builds the book, `mise run
  docs:figures` regenerates the figures; both run in CI.
- **Workflow**: `.github/workflows/pages.yml`, build on pull requests,
  build and deploy on `main`.
- **Pages**: enabled with `gh api` (build type `workflow`). Site URL:
  `https://cyboooooorg.github.io/map-generator/`.
- **Figures**: `tools/figures`, one function per figure, written to
  `docs/science/figures/<name>.svg`; text laid out with a font committed
  in the repository, so the SVG is byte-identical on macOS and in CI.
- **First figures**:
  1. Main-sequence mass–luminosity and mass–radius from the YREC grid
     (`stellar::main_sequence`), the Sun marked; used by
     `star-types.md` and `star-structure.md`.
  2. Giant-planet radius against mass from Fortney et al. (2007)
     (`giants::giant`), Jupiter and Saturn marked; used by
     `planet-interiors.md`.

## Left to the builder

- Exact tool versions (mdBook and its preprocessors must agree on one
  mdBook version), the link checker (`mdbook-linkcheck` or `lychee`),
  the font and its licence (an OFL font).
- The ages and metallicity of the main-sequence curves, the core masses
  of the giant curves, and the sources of the reference points (each
  cited in its page's references).
- Plot styling.
