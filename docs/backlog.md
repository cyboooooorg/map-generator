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

- **Status:** done
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
- **Result:** the server renders colour tiles (about 4–5 ms each) with the
  Rust renderer itself (biome rules, soft borders, hillshading, vector
  rivers redrawn at the tile's scale, widening with the square root of the
  zoom), instead of data tiles coloured in the shader: no biome rule or
  river drawing duplicated in GLSL. The viewer keeps its base surface and
  draws detail patches over it where tiles are loaded (closest loaded
  ancestor while a tile loads). Data layers and the 3D relief stay at the
  base resolution; picking reads the base layers. Zoom to the cursor came
  later (globe and map).

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

- **Status:** done
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
- **Result:** on seeds 3, 7 and 42 (1920 × 1080), median sinuosity 1.33–1.36
  (p90 1.64–1.68, was 1.22) and 5.3–5.6 % straight 160 km windows (was 23 %);
  zero crossings is a test on three seeds. The flats-only routing noise kept
  the largest basin of seed 3 unchanged (6.11 M km²; noise everywhere grew it
  to 7.6–8.3 M km²) and moved seed 7's by 7 %. Sinuosity is slightly under the
  prototype's 1.45, which needed noise everywhere. Generation takes about
  2.6 s instead of 2.0 s. The uphill share was not re-measured: the valley
  guard of the prototype is kept unchanged.

## 4. Smaller items

| Item                                                                                        | Status |
| ------------------------------------------------------------------------------------------- | ------ |
| Hydrology is sequential: the depression filling takes about 1 s of the 1.8 s run            | todo   |
| Picking on the globe ignores the relief displacement (error under a degree)                 | todo   |
| Night side lighting is a fixed 7 % floor, not physically calibrated                         | todo   |
| Volcanoes are rare on non-volcanic presets: the volcanic zone seldom exceeds its thresholds | todo   |
| The default random sea level (−0.30 to 0.50) mostly gives 60–95 % land and supercontinents  | done   |
| Supercontinent interiors stay desert belts: no moisture transport inland (analysis 02, D)   | todo   |
| Noise maps kept every debug layer until the export (OOM at 8192 × 4096): now streamed       | done   |

## 5. Terrain realism, second round

- **Problems** (user report after PR #16):
  - continent outlines are too round, with none of the angles, capes and
    straight stretches of real coasts;
  - zoomed coasts are full of tiny islands, and tiny ponds inland;
  - relief looks eroded everywhere: no smooth plains, no fluid deserts;
  - rivers are far too wide on the base render, and narrow suddenly when
    detail tiles appear;
  - rivers are drawn over the terrain instead of flowing in valleys.
- **Analyses:** `docs/analysis/04-coastlines.md`, `05-relief.md`,
  `06-river-width-and-valleys.md`.
- **Decision (2026-11-02):** in this order, so that every seed changes once
  (steps 2 and 3 back to back) and the tiles are tuned on the final terrain:

| Step | Change                                                                                                                                | Seeds  | Status |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------ |
| 1    | One river width law per raster resolution, shared by the base map and the tiles                                                       | Same   | done   |
| 2    | Noise rework: warped, fractal continent outline with a plate step; plains smoothed inland; more ridged mountain octaves               | Change | done   |
| 3    | Erosion on real slopes, tapered near the waterline, sediment carried offshore                                                         | Change | done   |
| 4    | Tiles: coast warped sideways, calmer plains detail, soft shading, no double shading                                                   | Same   | done   |
| 5    | River terrain: valleys and floodplains at generation, riparian moisture, calm floodplains and channels in tiles, deltas and estuaries | Change | done   |

## 6. Atmospheres

- **Status:** decided
- **Analysis:** [07-atmospheres.md](analysis/07-atmospheres.md)
- **Problem:** every planet shows the same blue rim glow, airless worlds
  included, and the climate ignores the atmosphere: Venus keeps Earth's
  equator-to-pole contrast, airless worlds get rain unless their preset
  turns erosion off.
- **Decision (2026-11-03):** an `[atmosphere]` preset table (pressure, sky
  colour, haze, greenhouse, cloud kind) stored in `world.json`, with
  pressure as a generation parameter; physical single scattering in the
  viewer; heat transport, greenhouse and liquid water driven by the
  atmosphere; a cloud layer computed from the climate, exported as
  `raw_data/clouds.png` and shown as an optional layer.

| Step | Change                                                                                    | Seeds  | Status |
| ---- | ----------------------------------------------------------------------------------------- | ------ | ------ |
| 0    | Temperatures in °C: Earth-fitted curve, thresholds and presets in °C, `format_version` 4  | Change | done   |
| 1    | `[atmosphere]` in presets and `world.json`, `--pressure`, form field; no effect yet       | Same   | done   |
| 2    | Viewer: scattering sky shell and aerial perspective from the world's atmosphere           | Same   | done   |
| 3    | Climate: heat transport, greenhouse, no liquid water below 0.01 bar; presets recalibrated | Change | done   |
| 4    | Cloud layer: generation, `raw_data/clouds.png`, `format_version` 5                        | Same   | done   |
| 5    | Viewer: cloud layer, cloud shadows, flat-map overlay, toggle, optional drift              | Same   | done   |

## 7. Surface colours

- **Status:** done
- **Problem:** the hillshaded renders use the classification palette: every
  biome a saturated, arbitrary colour (bright green plains, royal blue seas),
  which reads as an atlas, not as a planet seen from orbit. Hillshading,
  ecotones, the globe, the sky and the clouds already exist (items 1, 2 and
  6); the colours themselves are what is left.
- **Decision (2026-11-03):** a second, satellite-like palette for the
  hillshaded renders only (`world.png`, detail tiles, the viewer's biome
  legend); the classification palette stays on the SVG, the hex grid, the
  biome table's `color` and the legend's second swatch. Colours are mixed in
  linear light. Then, in this order:

| Step | Change                                                                                              | Seeds | Status |
| ---- | --------------------------------------------------------------------------------------------------- | ----- | ------ |
| 1    | Surface palette, ecotones and river edges mixed in linear light, `surface_color` in the biome table | Same  | done   |
| 2    | Water colour from depth (absorption), no visible step between Ocean and Deep Ocean                  | Same  | done   |
| 3    | Small colour variation within a biome, shared by the base map and the tiles                         | Same  | done   |
| 4    | Viewer: lava and magma glow on the night side                                                       | Same  | done   |

- **Rejected:** a 2:1 map size for square pixels (the grid already scales
  slopes by latitude); rock and snow from the slope (later, with the tiles'
  micro-relief).

## 8. Impact craters and geological time

- **Status:** in progress (step 7, added for tidally locked worlds)
- **Analysis:** [08-geology.md](analysis/08-geology.md)
- **Problem:** no world has impact craters, so dead worlds (`barren`,
  `mars-like`, `mercury-like`) look like dry Earths; the surface has no
  history and no notion of time.
- **Decision (2026-11-03):** a `[geology]` preset table (tectonic
  regime, surface age, end of the wet era) stored in `world.json`, with
  the surface age as a generation parameter; a crater field computed as
  a function of the position (lunar chronology, gravity-scaled
  morphology, superposition in age order, atmospheric filtering, worn by
  the erosion of the wet era), shared by the base map and the detail
  tiles; a time slider moving the surface of worlds without liquid water
  through their cratering history, up to 1 Ga into the future.

| Step | Change                                                                                                         | Seeds  | Status |
| ---- | -------------------------------------------------------------------------------------------------------------- | ------ | ------ |
| 1    | `[geology]` in presets and `world.json`, `--surface-age`, form field, regime in the world info; no effect yet  | Same   | done   |
| 2    | Crater field in the generation: chronology, morphology, superposition, atmosphere, wet era; `ELEVATION_KM`     | Change | done   |
| 3    | `elevation_base.png`, `format_version` 7; craters in the detail tiles down to the tile pixel                   | Same   | done   |
| 4    | Epoch: server routes and tile parameter, rendering of a world at an epoch                                      | Same   | done   |
| 5    | Viewer: time slider, reduced layers while dragging, epoch in picking and URL state                             | Same   | done   |
| 6    | Optional: ray systems and ejecta albedo fading with age; lava flooding of basins (maria) at given epochs       | Change | done   |
| 7    | Tidally locked worlds: more craters on the leading hemisphere since the locking (ratio about 1.3)              | Change | todo   |

## 9. Plate tectonics and continental drift

- **Status:** analysis
- **Analysis:** [08-geology.md](analysis/08-geology.md), section 8
- **Problem:** continents come from noise, not from plates: mountains
  and volcanoes are not tied to plate boundaries, and continents cannot
  drift over time.
- **Studied:** a plate simulation on a point mesh of the sphere
  (Cortial et al. 2019) replacing the coarse continent octaves, with a
  time slider over keyframes of the simulation.
- **Deferred (2026-11-03):** too costly for now (a new subsystem, heavy
  tuning, a full climate per epoch).
- **Cheaper approach (research, 2026-11-04):** kinematic drift without
  simulation. Static Voronoi plates (as in frozenfractal's generator)
  turned by Euler poles constant per stage (as in GPlates): each pixel
  reads the present relief at its point of origin, and boundary relief
  (ranges, rifts, ridges, trenches, arcs) follows from the geometry at
  the epoch, with no memory of earlier steps. A few milliseconds to a
  few tens of milliseconds per epoch at 512 × 256 (estimate); bounded
  to about ±200–300 Ma. Not decided yet.
- **Tidally locked worlds (2026-11-04):** their climate depends on the
  longitude around a fixed substellar point, so drift moves continents
  into or out of the warm "eye" and the climate must follow each epoch;
  a day-to-night contrast can organise plates by hemisphere (Meier et
  al. 2021), which the Euler poles of locked worlds could follow. See
  the analysis.

| Step | Change                                                                                                 | Seeds  | Status |
| ---- | ------------------------------------------------------------------------------------------------------ | ------ | ------ |
| 1    | Static plates on `plates` worlds: Voronoi subplates, velocities, boundary features (ranges, arcs)      | Change | todo   |
| 2    | Kinematic drift: Euler poles per stage, plate of each pixel at an epoch, boundary relief from geometry | Same   | todo   |
| 3    | Time slider for drift: low-resolution preview of the whole pipeline, "generate at this epoch"          | Same   | todo   |

## 10. Internationalisation of the web app

- **Status:** done
- **Analysis:** [09-i18n.md](analysis/09-i18n.md)
- **Problem:** every string of the web app is English, written in the
  templates; numbers and dates are formatted for English only.
- **Decision (2026-11-03):** a typed in-house catalog (English source, French typed
  with the same shape), the locale detected from the browser and chosen
  in the header; biome names, stages, request errors and built-in
  presets translated; generated files and the CLI stay English.

| Step | Change                                                                               | Seeds | Status |
| ---- | ------------------------------------------------------------------------------------ | ----- | ------ |
| 1    | i18n module, typed English catalog, `Intl` formatters, locale detection and selector | Same  | done   |
| 2    | Strings moved to the catalog, area by area; formatting localised                     | Same  | done   |
| 3    | Server: structured request errors, `[locales.*]` in presets                          | Same  | done   |
| 4    | French catalog, French built-in presets, layout checked with longer strings          | Same  | done   |
| 5    | Guard against literal text in templates, in CI                                       | Same  | done   |

## 11. The host star

- **Status:** done
- **Analysis:** [10-stars.md](analysis/10-stars.md)
- **Problem:** every world orbits an implicit Sun at 1 AU: temperatures
  come from Earth's profile and preset shifts, the light is white, and
  no star is drawn in the viewer.
- **Decision (2026-11-04):** a star (spectral type, drawn with realistic
  frequencies or chosen) and an orbit given by a position in its
  habitable zone (each planet type in its logical range, volcanic worlds
  heated from within) or a distance in AU; the climate follows the
  insolation; the viewer draws the star at the sunlight direction, with
  the star field, and can light the planet with the star's colour.

| Step | Change                                                                         | Seeds  | Status |
| ---- | ------------------------------------------------------------------------------ | ------ | ------ |
| 0    | Star model: spectral table, black-body colour, habitable zone, derived values  | Same   | done   |
| 1    | `[star]` in presets, `--star` and the orbit, `world.json`, form and world card | Same   | done   |
| 2    | Climate driven by the insolation, internal heat, presets recalibrated          | Change | done   |
| 3    | Viewer: the star at the sunlight direction, with the star field                | Same   | done   |
| 4    | Viewer: "Star light", the planet and its sky lit by the star's colour          | Same   | done   |
| 5    | Tidal locking of close-in planets                                              | Change | done   |
| 6    | Vegetation colours adapted to the star, in the "Star light" view               | Same   | done   |
