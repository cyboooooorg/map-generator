# Science Reference Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish `docs/science/` as an mdBook site on GitHub Pages and
draw its first figures from the `science` crate with a Rust tool.

**Architecture:** mdBook builds the pages in place (`docs/science/book.toml`,
`src = "."`) with KaTeX and Mermaid preprocessors; a `docs:check` mise task
(figures fresh, book builds, links resolve, summary complete) runs in
`mise run ci` and in a `docs` CI job; `.github/workflows/pages.yml`
deploys on every push to `main`. `tools/figures` is a workspace binary that
calls the public laws of `science` and writes deterministic SVGs with
`plotters` and a committed font.

**Tech Stack:** mdBook, mdbook-katex, mdbook-mermaid, lychee (offline
link check), Rust 2024 with `plotters` (SVG backend, `ab_glyph` text),
mise, GitHub Actions (`actions/upload-pages-artifact`,
`actions/deploy-pages`).

**Spec:** `docs/superpowers/specs/2026-11-23-science-site-design.md`

## Global Constraints

- Work in `/Users/cyborg/Developer/cyboooooorg/map-generator-wt-site`,
  branch `docs/science-site`. Never touch
  `/Users/cyborg/Developer/cyboooooorg/map-generator`. No `npm ci`, no
  `mise run web:check`. Build with `nice` and at most 4 jobs.
- Site URL: `https://cyboooooorg.github.io/map-generator/`.
- Outbound links become `https://github.com/cyboooooorg/map-generator/blob/main/<path>`
  (`tree/main/<path>` for a directory).
- Every tool in `mise.toml` `[tools]` is pinned to an exact version; the
  mdBook preprocessors must support the pinned mdBook version.
- Figures: `docs/science/figures/<name>.svg`, byte-identical on macOS and
  Linux; no system font; no hand-edited SVG.
- Every number added in `tools/figures` (reference bodies) has a doc
  comment starting with `Source:`; a figure reads laws only through the
  public API of `science`.
- Every new or edited Markdown file passes markdownlint (default rules,
  config `.markdownlint-cli2.jsonc`); new Markdown files are added to its
  globs.
- Commits follow Conventional Commits, end with the session's
  `Co-Authored-By` and `Claude-Session` lines, and are not pushed.
- `mise run ci` passes at the end of every task that touches Rust.

## Review Focus

1. A law that errors on part of a figure's range (`GiantError::OutOfTable`
   below the table's lowest mass for the core, about 28 M⊕ for a core of
   25 M⊕ (12.92 M⊕ for any core), `StellarError` at a range edge): the figure draws
   the valid part, never panics — test in Task 3 and Task 4.
2. `figures --check` with a figure missing on disk, or stale: reports it
   by name and exits 1, writes nothing — test in Task 2.
3. A page added to `docs/science/` but not to `SUMMARY.md`: `docs:check`
   fails naming it — check in Task 1.
4. A ` ```math ` block left in a page (GitHub shows it, the site shows raw
   LaTeX): `docs:check` fails naming the file — check in Task 1.
5. Text measured with a system font, so CI and macOS disagree: the font
   is registered from bytes in the binary — test in Task 2.

---

### Task 1: The book builds, with equations, diagrams and links

**Files:**

- Create: `docs/science/book.toml`, `docs/science/SUMMARY.md`
- Modify: every `docs/science/*.md` holding ` ```math ` blocks or `../`
  links; `docs/science/README.md` (writing rule for equations)
- Modify: `mise.toml` (`[tools]`, tasks `docs:build`, `docs:check`, `ci`)
- Modify: `.markdownlint-cli2.jsonc` (add `docs/science/SUMMARY.md`)

**Interfaces:**

- Produces: `mise run docs:build` (book in `target/book`),
  `mise run docs:check` (Task 2 puts the figures check first in it).

- [ ] **Step 1: Pin the tools.** Add mdBook, mdbook-katex, mdbook-mermaid
  and lychee to `[tools]` with exact versions (prebuilt binaries through
  mise's `aqua:` or `ubi:` backends; no `cargo install` in CI). Check that
  the preprocessor versions declare support for the mdBook version.
- [ ] **Step 2: Write `book.toml`.** `src = "."`, `build-dir =
  "../../target/book"`, title "The science behind map-generator",
  `site-url = "/map-generator/"`, `git-repository-url` the repo,
  preprocessors `katex` (with `throw-on-error = true`, so a formula
  KaTeX cannot parse fails the build) and `mermaid` (run
  `mdbook-mermaid install` once and commit the files it adds).
- [ ] **Step 3: Write `SUMMARY.md`.** `[Introduction](README.md)`, then
  one part per `###` heading of `README.md` under `## Topics`, same
  order, same link texts. A page missing from the README's topics is
  added to both, under the heading that fits.
- [ ] **Step 4: Convert the equations.** With a throwaway script (not
  committed): each ` ```math ` … ` ``` ` block becomes `$$` … `$$` at the
  same indentation, including the example inside the ` ````markdown `
  fence of `README.md`, and the README's writing rule then says
  equations use `$…$` and `$$…$$`. Check in the built `index.html` that
  this example still shows as code, not as a rendered formula.
- [ ] **Step 5: Rewrite the outbound links.** Every relative link whose
  target lies outside `docs/science/` becomes an absolute GitHub link
  (Global Constraints); anchors are kept. Links between pages stay
  relative.
- [ ] **Step 6: Add the tasks.** `docs:build` runs `mdbook build
  docs/science`. `docs:check` runs, in order: a shell check that fails
  naming any `docs/science/*.md` (except `SUMMARY.md`) missing from
  `SUMMARY.md` and any file still holding a ` ```math ` fence;
  `docs:build`; `lychee --offline --include-fragments
  target/book`. Append `docs:check` to `[tasks.ci]` after `md:lint`.
  Shell code inline in `mise.toml` (no script under `tools/`: every
  directory there is a Cargo workspace member).
- [ ] **Step 7: Verify.** `mise run docs:check` passes. Grep
  `target/book/star-types.html` and `planet-interiors.html`: equations
  are KaTeX spans, no `$$` left; a Mermaid page holds
  `<pre class="mermaid">`; no KaTeX error. Then add a dummy
  `docs/science/zz.md`: `docs:check` fails naming it; remove it.
  `mise run md:lint` passes.
- [ ] **Step 8: Commit** `docs(science): build the science reference
  with mdBook`.

### Task 2: The figures tool

**Files:**

- Create: `tools/figures/Cargo.toml`, `tools/figures/clippy.toml` (copy
  of `tools/opacity-table/clippy.toml`), `tools/figures/src/main.rs`,
  `tools/figures/src/lib.rs`, `tools/figures/fonts/<font>.ttf` and its
  licence file, `tools/figures/README.md`
- Modify: `tools/guardrails/src/deps.rs:23-40` (`("figures",
  set(&["science"]))`), root `Cargo.toml` (`plotters` in
  `[workspace.dependencies]`, `default-features = false`, features
  `svg_backend`, `line_series`, `point_series`, `ab_glyph`), `mise.toml`
  (task `docs:figures`; `docs:check` starts with `docs:figures --check`),
  `.markdownlint-cli2.jsonc`

**Interfaces:**

- Produces in `tools/figures/src/lib.rs`:

  ```rust
  pub struct Figure { pub name: &'static str, pub render: fn() -> Result<String, FigureError> }
  pub enum FigureError { Draw(String), Io(String) } // Display + Error
  pub fn figures() -> Vec<Figure>                    // Tasks 3 and 4 add theirs here
  pub fn stale(dir: &Path) -> Result<Vec<&'static str>, FigureError> // missing or different files
  pub(crate) fn svg(width: u32, height: u32, draw: impl FnOnce(DrawingArea<SVGBackend, Shift>) -> Result<(), FigureError>) -> Result<String, FigureError>
  ```

  `svg` registers the committed font once (`std::sync::Once`) and renders
  into a string; every figure draws through it, so none can forget the
  font.

- CLI: `figures [--out DIR] [--check]`, default `DIR` =
  `docs/science/figures`. Without `--check`, writes every figure. With it,
  writes nothing, prints each stale name, exits 1 if any.

- [ ] **Step 1: Write the failing tests** in `tools/figures/src/lib.rs`
  with a test-only figure `fn sample() -> Result<String, FigureError>`
  drawing one line and a text label:
  - `rendering_twice_gives_the_same_bytes`: `sample()` twice, equal.
  - `text_renders_without_a_system_font`: `sample()` is `Ok` and holds
    `<text` (with `ab_glyph` and no font registered, plotters fails to
    measure text, so this proves the committed font is used).
  - `a_missing_file_is_stale` and `a_changed_file_is_stale` (tempdir):
    `stale` returns the name; `a_fresh_file_is_not_stale`: empty
    (`tempfile` from `[workspace.dependencies]` as a dev-dependency).
  - `figure_names_are_unique`: over `figures()`.
- [ ] **Step 2: Run** `cargo nextest run -p figures`: fails (no code).
- [ ] **Step 3: Implement.** `svg` renders with
  `SVGBackend::with_string` and registers the committed OFL font (bytes
  by `include_bytes!`) under the family `sans-serif` with
  `plotters::style::register_font`. Pick a sans-serif OFL font (e.g.
  Noto Sans or Source Sans 3), regular weight only. `main.rs` parses the
  two flags by hand like `tools/opacity-table`.
- [ ] **Step 4: Run** `cargo nextest run -p figures`: passes;
  `mise run ci` passes (guardrails accepts the new member).
- [ ] **Step 5: Write `tools/figures/README.md`:** purpose, usage
  (`mise run docs:figures`, `--check`), how to add a figure (a function,
  one `figures()` entry, sources as `Source:` doc comments, the image
  link in its page), why the font is committed.
- [ ] **Step 6: Commit** `feat(figures): draw science figures as
  deterministic SVG`.

### Task 3: Main-sequence figure

**Files:**

- Create: `tools/figures/src/stars.rs`
- Modify: `tools/figures/src/lib.rs` (`figures()` entry),
  `docs/science/star-types.md` (section "Mass sets everything", l.108),
  `docs/science/star-structure.md` (section "How the app computes it",
  l.89); `docs/science/figures/main-sequence.svg` (generated)

**Interfaces:**

- Consumes: `science::stellar::{life, main_sequence, Life, MainSequence,
  StellarError}`, units `SolarMass(f64)`, `Dex(f64)`, `Gyr(f64)`
  (`.0` to read).
- Produces: `fn main_sequence_figure() -> Result<String, FigureError>`,
  registered as `"main-sequence"`.

- [ ] **Step 1: Write the failing tests** in `stars.rs`:
  - `the_curves_cover_the_mass_range`: the sampled series (a private
    `fn series(age: Stage) -> Vec<(f64, f64, f64)>` of mass, luminosity,
    radius) spans 0.1 to 8 M☉ with at least 60 points, log-spaced.
  - `luminosity_rises_with_mass`: strictly increasing along each series
    (physics: the main-sequence mass–luminosity relation). If it fails,
    at the 0.30-0.35 M☉ seam for instance, stop and report it: do not
    relax the test.
  - `the_figure_renders`: `main_sequence_figure()` is `Ok` and contains
    the label `Sun`.
- [ ] **Step 2: Run:** fails.
- [ ] **Step 3: Implement.** [Fe/H] = 0. Two stages: ZAMS (`age =
  life.zams`) and mid-life (`zams + 0.5 × (end − zams)`). Two panels side
  by side, log-log: L/L☉ against M/M☉, R/R☉ against M/M☉. Mark the Sun
  at (1, 1) in both panels: by definition of the nominal solar units
  (IAU 2015 B3), cited in the page. A mass whose call errors is skipped.
- [ ] **Step 4: Run** the tests, then `mise run docs:figures`.
- [ ] **Step 5: Embed** `![…](figures/main-sequence.svg)` in both pages
  with a caption saying it is drawn by `tools/figures` from
  `stellar::main_sequence` (YREC grid, `crates/science/data/STELLAR.md`
  as a GitHub link) and regenerated by `mise run docs:figures`.
- [ ] **Step 6: Verify** `mise run ci` passes (includes `docs:check`).
- [ ] **Step 7: Commit** `feat(figures): draw the main sequence from
  the stellar grid`.

### Task 4: Giant-planet radius figure

**Files:**

- Create: `tools/figures/src/giants.rs`
- Modify: `tools/figures/src/lib.rs`, `docs/science/planet-interiors.md`
  (section "Giants and the metallic hydrogen boundary", l.455; Sources
  l.1627 if a source is new); `docs/science/figures/giant-radius.svg`

**Interfaces:**

- Consumes: `science::giants::{giant, Giant, GiantError, JUPITER_RADIUS}`,
  `science::constants::JUPITER_MASS_PARAMETER` and the gravitational
  constant of `science::constants`, units `EarthMass`, `EarthFlux`,
  `Gyr`, `Kilogram`, `Metre`.
- Produces: `fn giant_radius_figure() -> Result<String, FigureError>`,
  registered as `"giant-radius"`.

- [ ] **Step 1: Write the failing tests** in `giants.rs`:
  - `out_of_table_masses_are_skipped`: the series for a 25 M⊕ core
    starts above the core-bounds limit and holds no NaN.
  - `the_figure_renders`: `Ok`, contains `Jupiter` and `Saturn`.
- [ ] **Step 2: Run:** fails.
- [ ] **Step 3: Implement.** Age 4.5 Gyr, flux of 5.2 au (`1 / 5.2²`
  EarthFlux); one curve per core 0, 10 and 25 M⊕; masses log-spaced
  from 20 M⊕ to 3 000 M⊕. Axes in Jupiter units: mass via `Kilogram` and
  `JUPITER_MASS_PARAMETER / G`, radius via `JUPITER_RADIUS`. Jupiter and
  Saturn points: masses and radii from the NASA planetary fact sheets,
  as `const`s with `Source:` doc comments, using the radius convention
  (equatorial or mean) that Fortney et al. (2007) use when they compare
  their models with Jupiter and Saturn. The caption states it, and that
  the curves are at Jupiter's distance (Saturn sits at 9.6 au).
- [ ] **Step 4: Run** the tests, then `mise run docs:figures`.
- [ ] **Step 5: Embed** in `planet-interiors.md` with a caption naming
  `giants::giant`, Fortney et al. (2007) and the fact sheets (add them to
  the page's Sources if missing).
- [ ] **Step 6: Verify** `mise run ci` passes.
- [ ] **Step 7: Commit** `feat(figures): draw giant radii from the
  Fortney grid`.

### Task 5: CI and deployment

**Files:**

- Modify: `.github/workflows/ci.yml` (job `docs`, `changes` filter,
  `ci-ok.needs`)
- Create: `.github/workflows/pages.yml`
- Modify: `docs/science/README.md` (where the site lives, how it is
  built), root `README.md` (link to the site), `CLAUDE.md` (Tests: a law
  change regenerates its figures; `docs:check` in `ci`)

**Interfaces:**

- Consumes: `mise run docs:check`, `mise run docs:build`.

- [ ] **Step 1: `ci.yml`.** A `docs` output in `changes` matching
  `docs/science/**`, `tools/figures/**`, `crates/science/**`,
  `Cargo.toml`, `Cargo.lock`, `mise.toml`, `.github/workflows/**`.
  Job `docs` (if that output is
  true): checkout, `jdx/mise-action@v5`, rust-cache like `tier1`,
  `mise run docs:check`. Add `docs` to `ci-ok.needs`.
- [ ] **Step 2: `pages.yml`.** On push to `main` and
  `workflow_dispatch`; permissions `contents: read`, `pages: write`,
  `id-token: write`; concurrency group `pages`, no cancel. Job `build`:
  mise, `mise run docs:build`, `actions/upload-pages-artifact` with
  `target/book`. Job `deploy`: needs `build`, environment `github-pages`
  with the page URL, `actions/deploy-pages`. Pin action major versions
  like `ci.yml`.
- [ ] **Step 3: Verify** `mise run ci:actions` (actionlint) and
  `mise run ci` pass.
- [ ] **Step 4: Docs.** README pages and `CLAUDE.md` as listed; markdownlint
  passes.
- [ ] **Step 5: Commit** `ci(docs): check the science site and deploy it
  to Pages`.

### Main session, not a task

- Enable Pages before merge: `gh api -X POST repos/{owner}/{repo}/pages
  -f build_type=workflow` (the user asked for `gh api`).
- File an issue (label `backlog`, theme `docs`) for the CI check that
  every page has a non-empty References section.
- After merge: check the first `pages.yml` run and open the site.
