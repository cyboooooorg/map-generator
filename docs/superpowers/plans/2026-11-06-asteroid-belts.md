# Asteroid belts, trojans and Lagrange points: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax
> for tracking.

**Goal:** draw a system's small bodies (inner belt, outer belt, scattered
disk, trojan swarms), let them set the crater rate, and show them, with
the selected planet's Lagrange points, in the star view and in the
planet view's sky.

**Architecture:** a new module `crates/mapgen/src/belts.rs` draws the
belts and swarms from the planets once they are placed, from streams of
their own (`BELTS` = 32, `TROJANS` = 33), so the planets and moons of a
seed stay as they are. `system.json` gains optional fields, read as empty
on older systems. The web side reads them through `lib/systemStar.ts`
and draws them in `StarView.ts`, and in the planet view's sky.

**Tech stack:** Rust (mapgen, ts-rs types), TypeScript, Svelte 5,
Three.js.

**Spec:** `docs/analysis/13-asteroids.md`, with the user's decisions of
2026-11-06 (backlog item 17).

## Global constraints

- Realism first: a belt the known physics and observations rule out is
  not generated.
- Existing systems keep every body: new `system.json` fields are optional
  (`#[serde(default, skip_serializing_if)]`), and `SYSTEM_FORMAT_VERSION`
  stays 1.
- Only systems drawn from now on change: their rocky planets keep out of
  the first cold giant's belt zone, and their bodies get a crater rate
  from the belts (`WorldParams::impact_factor`, `None` on stored bodies,
  which keep today's rate).
- English in the repository; the French UI strings in `lib/i18n/fr.ts`.
- Docs in the same change: `docs/system-spec.md`, the main `README.md`,
  `web/README.md`.

## User decisions (2026-11-06)

1. Inner belt, outer belt and scattered disk, each only where physics and
   observations allow it.
2. Trojan swarms drawn in the star view. In the planet view, where they
   are invisible, an option circles them or highlights them from afar,
   with a label.
3. All five Lagrange points marked for the selected planet.
4. The crater rate reads the belts' masses.
5. New systems keep the first cold giant's belt zone (0.40–0.63 of its
   distance) free of rocky planets.

## Rules (calibration choices, stated in the docs)

- **Inner belt.**
  - Needed: a giant `G` that is not a hot Jupiter, at 0.5 ice lines or
    beyond, the innermost such.
  - Zone `[0.40, 0.63] a_G` (the ν6-like edge to the 2:1), minus each
    other planet's clearing zone (periapsis − 5 Hill radii to
    apoapsis + 5). The widest piece is kept only if outer / inner ≥ 1.15,
    and only beyond `0.05 √L` AU.
  - Mass: log-uniform, 10⁻⁵ to 10⁻² M⊕ (the Sun's is 4 × 10⁻⁴).
  - Gaps: at the 3:1, 5:2 and 7:3 inside it, each `0.012 a_gap √(μ / μ_J)`
    wide.
  - Hildas: a group at the 3:2 (0.763 `a_G`), 0.74–0.79 `a_G`, with
    10⁻³ of the belt's mass, if that range is clear.
  - Composition: rocky, mixed or icy, depending on where it lies against
    the ice line.
- **Outer belt.**
  - Drawn with probability 0.5 round F, G and K stars, 0.25 round M stars
    and 0.3 round hotter ones.
  - Range: beyond the outermost non-hot giant `L`, `[1.31, 1.59] a_L`.
    Without a giant, `[11, 18]` ice lines.
  - Minus the clearing zones of the planets there, kept if the ratio is
    1.15 or more.
  - Mass: log-uniform, 10⁻³ to 1 M⊕. Icy (mixed if inside the ice line).
- **Scattered disk.** Only with an outer belt and a giant `L`. Range
  `[a_L, 4 a_L]`, mass 0.3 to 3 times the belt's, eccentric (0.3) and
  inclined (15°).
- **Trojans**, for each planet `P`. None if:
  - `μ = m / (M + m) ≥ 0.0385` (Routh), or
  - a more massive planet orbits within a period ratio of 3.

  Otherwise, bodies larger than 1 km:

  | Planet                                | Count, log-uniform |
  | ------------------------------------- | ------------------ |
  | gas giant                             | 10⁴ to 10⁶         |
  | ice giant, outermost giant            | 10⁴ to 10⁶         |
  | ice giant, any other                  | 10³ to 10⁵         |
  | hot Jupiter                           | 0 to 10³           |
  | rocky planet                          | 0 to 100           |

  L4 holds `r / (1 + r)` of them, with `r` uniform in 0.7–1.45.
- **Crater rate** of a system body:
  - `curve(innermost giant beyond it) × max(0.05, 0.8 A + 0.2 C)`;
  - `curve` is `giants::impact_factor`;
  - `A` = inner belt mass / 4.0 × 10⁻⁴ M⊕, clamped to `[0, 10]`;
  - `C` = (outer belt + scattered disk) / 0.05 M⊕, clamped to `[0, 10]`;
  - the Solar System gives 1. A moon takes its planet's rate.

## Task 1: `belts.rs`, the draw

**Files:** create `crates/mapgen/src/belts.rs`; modify
`crates/mapgen/src/lib.rs` and `crates/mapgen/src/noise.rs` (streams 32
and 33).

**Interfaces (produces):**

```rust
pub enum BeltKind { Inner, Hildas, Outer, Scattered }
pub enum BeltComposition { Rocky, Mixed, Icy }
pub struct Belt { kind, inner_au: f64, outer_au: f64, mass_earth: f64,
    gaps: Vec<[f64; 2]> /* centre, width, AU */, composition,
    eccentricity: f64, inclination_deg: f64, shaped_by: Option<String> }
pub struct Trojans { l4: u32, l5: u32 }
pub struct Track<'a> { id: &'a str, body_type: &'a str, mass_earth: f64,
    orbit_au: f64, eccentricity: f64 }
pub fn belt_zone(giant_au: f64) -> (f64, f64);
pub fn draw_belts(star: &StarInfo, ice_au: f64, planets: &[Track], seed: u32) -> Vec<Belt>;
pub fn draw_trojans(star_mass: f64, planets: &[Track], seed: u32) -> Vec<Option<Trojans>>;
pub fn impact_factor(belts: &[Belt], planets: &[Track], orbit_au: f64) -> f32;
```

Tests:

- no giant, no inner belt;
- a Solar-like system gives an inner belt within 2.0–3.3 AU, gaps at
  2.50, 2.82 and 2.95 AU, and Hildas;
- a rocky planet in the zone splits it, and a narrow remainder is
  dropped;
- the outer belt sits at `[1.31, 1.59] a_L`;
- Routh and the more-massive-neighbour rule;
- the Solar System's crater rate is 1, with no belts 0.05 × curve;
- determinism.

## Task 2: system integration

**Files:** `crates/mapgen/src/system.rs` and `world.rs`
(`WorldParams::impact_factor`), `craters.rs` (use it when `Some`).

- `System.belts: Vec<Belt>` and `Body.trojans: Option<Trojans>`, both
  optional in the file.
- `assemble` draws them and sets each body's `impact_factor`.
- `space()` keeps non-fixed rocky planets out of the first cold giant's
  zone, iterated as the giants settle; a whole drawn system drops what no
  longer fits, as it already does.
- Tests: no rocky planet in the zone over many seeds, an old
  `system.json` reads, the bodies of a stored system are unchanged.
- Docs: `docs/system-spec.md`, `README.md`.

## Task 3: star view

**Files:**

- `web/src/lib/systemStar.ts`: belts, and each planet's trojans.
- `web/src/lib/viewer/StarView.ts`: `Points` clouds, animated at
  Kepler's pace, the gaps left empty. Swarms round L4 and L5, as tadpoles
  moving with their planet. The five Lagrange points of the selected
  planet.
- `web/src/components/StarPane.svelte`: switches *Belts*, *Trojans* and
  *Lagrange points*; labels; the panels' small-bodies section.
- `lib/i18n/{en,fr}.ts`.
- Tests: placement helpers in `starSystem.ts` (Lagrange positions,
  points out of the gaps).

## Task 4: planet view's sky

**Files:** `web/src/lib/systemSky.ts` and the sky drawing.

- A switch *Small bodies*. It rings each trojan swarm's direction (L4
  and L5 of every planet that has one) and each belt's band across the
  sky, with a label.
- Tests: the swarms' sky directions.
