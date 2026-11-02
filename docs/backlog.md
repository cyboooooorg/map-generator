# Backlog

Open work on the generator and the planet viewer, in implementation order.
Each item states the problem, what "done" means, the decision taken and its
status. The analyses behind the decisions are in `docs/analysis/`.

Status values: `todo`, `analysis` (options being compared), `decided`
(solution chosen, not started), `in progress`, `done`.

## Decisions (2026-11-02)

The three analyses agree on one point: the deep-zoom tile renderer must draw
the final biomes (with their soft borders) and the final vector rivers. Items
are therefore implemented in this order, so that the tile renderer is built
once, on top of the final data:

1. Shared groundwork (item 0).
2. Biomes (item 2): changes the climate fields, which the rivers read.
3. Rivers (item 3): reads the new moisture; adds the vector river export.
4. Deep zoom (item 1): renders the final biomes and rivers at any scale.

Each step regenerates different worlds for a given seed; `raw_data` moves to
`format_version` 2 in step 1 and stays readable for v1 worlds where possible.

## 0. Shared groundwork

- **Status:** done
- **Work:**
  - A deserialised `Preset` rebuilds its biome lookup (today `lookup` is
    `serde(skip)`, so a preset read back from `world.json` cannot remap).
  - `raw_data` v2 exports the layers biome selection needs and that are not
    exported today: volcanic zone and coast distance (and the highland
    jitter from item 2), so a world can be reloaded and biomes recomputed.
  - A `BaseWorld::load` (or equivalent) that reloads a world from `raw_data`,
    shared by the server, the hex resampling and the tile renderer.

## 1. Deep zoom in the viewer

- **Status:** decided, after items 2 and 3
- **Analysis:** [01-deep-zoom.md](analysis/01-deep-zoom.md)
- **Problem:** worlds are generated at about 20 km per pixel; zooming in only
  magnifies those pixels.
- **Decision:** detail is synthesised on demand, not generated: the noise has
  no feature below about 500 km, so a larger map adds nothing (8192 × 4096
  takes 26 s and 4.2 GB). The server renders 256 × 256 tiles of an
  equirectangular quadtree (2^(z+1) × 2^z tiles per level, z = 8 is about
  315 m per pixel on an Earth-sized planet) from the base layers: smooth
  interpolation, noise octaves limited to features smaller than two base
  pixels (coastlines move less than one base pixel), climate fields read at
  a warped position for irregular borders, and the generation's own biome
  rules and preset remap. About 6 ms per tile; no disk cache (immutable
  browser caching keyed by the world revision, the last two worlds kept in
  server memory). The viewer replaces its single mesh with a quadtree of
  patches using the same globe/map morph, streams tiles with a priority
  queue (parent shown until children arrive), and adapts controls and near
  plane near the surface.
- **Rejected:** a pre-generated pyramid (about 175 000 tiles, 20 GB and 15 min
  per world); detail in the shader as the main path (it would duplicate the
  biome rules in GLSL and cannot draw rivers), kept for micro-relief later.
  WebAssembly in the browser stays possible later with the same Rust code.
- **Done when:** zoom from the whole planet to a few hundred metres per screen
  pixel on the globe and the map, consistent with the base world, loading
  progressively without freezing the page.

## 2. Biomes form bands and have hard transitions

- **Status:** done
- **Analysis:** [02-biome-banding.md](analysis/02-biome-banding.md)
- **Findings:** latitude explains 99.3 % of land temperature and 76 % of land
  moisture, so every threshold draws an east-west line; the moisture noise is
  tiny. The altitude "rings" are the contour lines drawn on land, not biomes.
  Bug: since erosion, elevations top out near 0.67 while `highland` is 0.70
  above sea level, so Earth-like worlds have no Mountain or Snow at all (and
  few volcanoes, which need highland).
- **Decision:**
  - Generation: large-scale temperature noise (land 0.40, sea 0.20), local
    multi-octave climate noise stored in the exported temperature (0.25) and
    moisture (0.50), moisture noise weight 0.30 → 0.50, and a jittered,
    recalibrated highland line so mountains and snow exist again.
  - Rendering: ecotones by blending each land pixel's colour across nearby
    climate thresholds (σ_t 0.04, σ_m 0.10); contour lines under water only.
    `biomes.png` stays a crisp classification; the SVG stays flat.
- **Rejected:** a continentality model (shifted the bands, grew ice caps);
  warping the circulation band (no measurable effect).
- **Watch:** soft borders must not look blurry (polar areas in the
  prototype); every seed changes; sea ice becomes patchy.
- **Done when:** borders aligned with latitude drop from about 0.49 to 0.25
  or less (metric in the analysis), mountains appear on Earth-like worlds,
  borders show ecotones, and every preset still works.
- **Result:** on the analysis' eight reference worlds, borders aligned with
  parallels went from 0.487 to 0.235 (0.22 is isotropic), mountain borders
  aligned with contours from 0.98 to 0.45, and the chance that a 500 km step
  east keeps the same biome from 0.89 to 0.68. Mountains cover about 2–15 %
  of land depending on the sea level. The ecotone width was halved from the
  analysis (σ 0.02 / 0.05 instead of 0.04 / 0.10): the wider blend washed
  out the new mosaic.

## 3. Rivers are too straight and too thin

- **Status:** decided, second
- **Analysis:** [03-rivers.md](analysis/03-rivers.md)
- **Findings:** the hydrology is sound; the drawing is the problem. All river
  length follows the 8 grid directions, 23–28 % of 160 km windows are
  straight, 41–45 % of river pixels lie on flats raised by the depression
  filling (where the fill order alone sets the direction), meanders are below
  one pixel, and every river is 1 px wide.
- **Decision:**
  - Routing noise on filled flats only, so divides and lakes do not move. The
    prototype applied it everywhere and moved divides: the flats-only
    variant must be measured before it is kept.
  - Vector rivers: polylines traced from the receivers (main stem follows the
    largest tributary), Chaikin smoothing, resampling, noise-driven meanders
    scaled to width, with guards (no uphill offset, bend radius, distance to
    other rivers including the parent, loops cut, tributaries snapped).
  - Drawn anti-aliased with width 0.9 px × (Q / 120 000)^0.5, capped at 4 px
    at base resolution; the deep zoom draws the larger of the real and the
    exaggerated width. New `raw_data/rivers.json`; the flow raster is kept.
  - The River biome is set from the drawn line, so `biomes.png` matches the
    picture.
- **Rejected:** D-infinity and multiple-flow routing (diffuse bands, not
  channels); a finer routing grid (4–16× the hydrology time, still 8
  directions).
- **Done when:** median sinuosity about 1.4, straight windows about 5 %, zero
  river crossings, under 0.5 % of samples going uphill, width growing with
  flow, deterministic per seed.

## 4. Smaller items

| Item                                                                                        | Status |
| ------------------------------------------------------------------------------------------- | ------ |
| Hydrology is sequential: the depression filling takes about 1 s of the 1.8 s run            | todo   |
| Picking on the globe ignores the relief displacement (error under a degree)                 | todo   |
| Night side lighting is a fixed 7 % floor, not physically calibrated                         | todo   |
| Volcanoes are rare on non-volcanic presets: the volcanic zone seldom exceeds its thresholds | todo   |
| The default random sea level (−0.30 to 0.50) mostly gives 60–95 % land and supercontinents  | todo   |
| Supercontinent interiors stay desert belts: no moisture transport inland (analysis 02, D)   | todo   |
