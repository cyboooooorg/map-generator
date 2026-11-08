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

| Item                                                                                                                 | Status |
| -------------------------------------------------------------------------------------------------------------------- | ------ |
| Depression filling: neighbours by index (4.2 → 2.4 s at 8192 × 4096, same worlds); parallel land masses gave no gain | done   |
| Picking on the globe ignores the relief displacement (error under a degree)                                          | done   |
| Night side lighting: the light the sky's bodies reflect on a log display curve, not a fixed 7 % floor                | done   |
| Volcanoes on non-volcanic presets: quantile threshold, cones after erosion, lowland vents (lava 0 → 0.3–0.5 %)       | done   |
| The default random sea level (−0.30 to 0.50) mostly gives 60–95 % land and supercontinents                           | done   |
| Supercontinent interiors stay desert belts: no moisture transport inland (analysis 02, D)                            | todo   |
| Noise maps kept every debug layer until the export (OOM at 8192 × 4096): now streamed                                | done   |
| Planet view: a switch to draw the planet's Hill sphere (from a moon, its host planet's)                              | done   |
| Planet view: from a moon, the host planet is drawn without its atmosphere and clouds                                 | done   |

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

- **Status:** done
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

- **Status:** done
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
| 7    | Tidally locked worlds: more craters on the leading hemisphere since the locking (ratio about 1.3)              | Change | done   |

## 9. Plate tectonics and continental drift

- **Status:** done (step 1: static plates shape the relief, on a
  cube-sphere grid that reaches the poles; step 2: `--drift`; step 3:
  the viewer's Drift panel)
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
| 1    | Static plates on `plates` worlds: Voronoi subplates, velocities, boundary features (ranges, arcs)      | Change | done   |
| 2    | Kinematic drift: Euler poles per stage, plate of each pixel at an epoch, boundary relief from geometry | Same   | done   |
| 3    | Time slider for drift: low-resolution preview of the whole pipeline, "generate at this epoch"          | Same   | done   |

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

## 12. The star view

- **Status:** done
- **Problem:** the star of a world is only seen as a disk in the planet's
  sky and as numbers on the world card.
- **Decision (2026-11-04):** a Planet / Star switch in the viewer; the
  star view shows the star as a globe (colour, limb darkening,
  granulation, spots and faculae by type, corona glow, flares on red
  dwarfs) with its system around it (habitable zone, orbit, planet, not
  to scale), drawn procedurally in the browser from `world.json`, and an
  optional animation (off by default).
- **Follow-up (2026-11-04):** the panel is split into *The star*, *The
  planet* and *Display*, names the spectral class (a G2V is a yellow
  dwarf) and explains each value in a tooltip; it gives the planet's
  orbital speed (mean, at periapsis and at apoapsis, from vis-viva), and
  the live distance label gives the speed at that point. The star shows
  prominences (H-alpha arcs, more on cooler stars, none above F) that can
  be hidden, and an eruption every nine seconds on animated M, K and G
  stars.
- **Follow-up (2026-11-04):** the animation runs on the display's clock
  (thirty frames a second at most, steps capped at 0.1 s) instead of a
  ten-a-second timer whose late ticks fired in bursts and made the planet
  jump; Kepler's equation is solved within one turn, so orbits above an
  eccentricity of 0.8 no longer jump either. A clock line gives the time
  factor and the day of the year; a test checks that the animated motion
  and the vis-viva speed agree. Switching worlds no longer flashes the
  Sun's system while the next world loads.
- **Follow-up (2026-11-04):** prominences are bundles of twelve thin
  strands, as in H-alpha images of the Sun, instead of one tube: they
  braid along the arch, carry knots of gas draining down the legs and
  tear apart when they erupt. They are alpha-blended rather than added,
  so the red of hydrogen stays red over the star's near-white glow.
- **Follow-up (2026-11-04):** a prominence seen against the disk is a dark
  filament (its gas absorbs the surface's light), red off the disk. The
  glow and the prominences are drawn in a fixed order: both centred on
  the star, their distance sort was a tie that flipped as the view
  turned, and the glow, drawn second, washed the strands out to white.

## 13. Moons

- **Status:** done
- **Analysis:** [11-moons.md](analysis/11-moons.md)
- **Problem:** worlds have a star and an orbit but no satellites: no
  moon in the star view or the sky, and nothing ties the axial tilt or
  the tides to a moon.
- **Studied (2026-11-04):** moons drawn from the seed with realistic odds
  (about 1 rocky planet in 12 with a large moon, after Elser et al. 2011;
  small debris or captured moons; companions frequent on small worlds),
  kept between the Roche limit and a third of the Hill radius, none
  lasting on planets locked to their star; Pluto–Charon binaries (mass
  ratio up to about 0.15, around the barycentre, locked face to face);
  irregular moons below about 900 km across for rock and 400 km for ice,
  drawn as deformed meshes in the viewer only.
- **Follow-up (2026-11-04):** the number of moons can be chosen in the
  form and requests (1 to 6), the star and orbit drawn again when they
  leave no room; see the analysis.
- **Plan (2026-11-04):** every moon is a body of its world, generated
  with the planet as a small world of its own (`barren` or a new
  `icy-moon` preset, irregular ones with a `shape` layer) under
  `worlds/<id>/moons/<n>/`, and opened in the planet view like the
  planet, from a Planet / Star / Moon switch. See the analysis, section 8.

| Step | Change                                                                                                           | Seeds  | Status |
| ---- | ---------------------------------------------------------------------------------------------------------------- | ------ | ------ |
| 1    | `moons` module: channels and odds, orbits, stability, shapes, migration; `world.json` `moons`; `--moons`         | Same   | done   |
| 2    | Moon surfaces: each moon generated as a world (`barren`, `icy-moon`, `shape` layer), server routes for its tiles | Same   | done   |
| 3    | Moon view: Planet / Star / Moon switch, the planet view on any body, moon panel, moons in the star view          | Same   | done   |
| 4    | Skies: the planet in each moon's sky, the moons in the planet's sky, eclipses (optional)                         | Same   | done   |
| 5    | Effects on the planet: axial tilt stabilised, tidal flats, binary rotation                                       | Change | done   |

## 14. Giant planets

- **Status:** done
- **Analysis:** [12-gas-giants.md](analysis/12-gas-giants.md)
- **Problem:** every world is a rocky or icy body alone around its star:
  no giant planet in its system, no giant world to generate, and no
  world can be the moon of a giant.
- **Studied (2026-11-04):** types (gas and ice giants, hot Jupiters,
  Sudarsky's cloud classes), interiors, formation (core and pebble
  accretion, disk instability, migration), occurrence (by star mass,
  orbit and metallicity) and gravitational influence (Hill spacing,
  forced eccentricity, impact flux, moons, tidal heating, habitable
  exomoons). Three ways in, to choose from: giant neighbours in the
  system, giant worlds with their own pipeline, rocky worlds orbiting a
  giant.
- **Decided (2026-11-04):** all three, in that order, one pull request
  each; giant worlds of every kind (cold Jupiters and Saturns, ice
  giants, hot Jupiters, rings); a star metallicity, drawn and settable;
  Earth-sized moons of giants rare when drawn, free in the form.
- **Step A (2026-11-04):** giant neighbours drawn in each world's system
  (`src/giants.rs`): a star metallicity (drawn, or set), gas and ice
  giants and hot Jupiters at the observed odds, kept clear of the world
  by Hill spacing; they raise a drawn eccentricity to the one they force
  and set the crater rate; shown in the star view (compressed beyond the
  world's system), its panel and the world card; `world.json` format 10.
- **Step B (2026-11-04):** giant worlds of their own: `gas-giant`,
  `ice-giant` and `hot-jupiter` presets generating cloud bands (Rhines
  jets, storms, polar hazes, Sudarsky colours) in the format of a world;
  the viewer flattens the globe, draws rings, drifts the bands with their
  winds and shows winds and cloud-top temperatures on their own ranges.
- **Step C (2026-11-04):** rocky worlds as moons of a giant: rare when
  drawn, or asked for; the giant generated beside the world and hanging
  in its sky with its phases and eclipses; tidal heating warming the
  climate and raising volcanism, watery worlds kept beyond the habitable
  edge.
- **Science fixes (2026-11-07):** wind speed an outcome of the flux and
  the spin, cloud decks deciding the Sudarsky class, class V with
  gravity, internal heat cooling with the system's age, the great spot in
  a southern zone drifting slowly, previews following the generated
  bands (`giant_clouds.rs`, `giant_world.rs`). Left to do:
  - The peak winds are a calibration on the four giants (Rossby number
    against the flux), not a derivation: Showman et al. 2010 call the
    winds of Saturn and Neptune, faster than Jupiter's on less energy,
    not understood. A friction or depth term would need a model of the
    deep flow. With Saturn's real flux the law gives about a third under
    its winds, and Uranus comes out at 285 m/s against 200.
  - The whole wind profile is held under 0.75 of the speed of sound and the
    peak rises with the heating (calibrations, no source); a cold light
    giant would otherwise reach 2 000 m/s. A giant that tidal locking
    stops (the rule locks a Jupiter-mass giant at 0.5 AU of a G2V star,
    which is doubtful) loses its Rossby wind and keeps the irradiation
    floor (about 120 m/s at 360 K): a drop of several hundred m/s at the
    step. A super-Jupiter (10 MJ at 40 AU) blows at only 20 to 30 m/s,
    because its large internal heat flux gives a tiny Rossby number: an
    extrapolation of the calibration. A Saturn-like giant's westward
    flank (-130 to -170 m/s) is stronger than the real one (about
    -50 to -100): the jets away from the equator take either sign at 0.3 to
    0.7 of the peak. Uranus has Neptune's wind profile; its real
    eastward peak is the larger and its westward flow weaker.
  - The great spot drifts slowly but the hole that hides the map's copy of
    it in `giantColour` (`shaders.ts`) still follows the band wind, a
    patch of plain band riding along the zone. Check it in a browser with
    Band drift on; making the hole follow the spot would freeze the band.
  - Saturn's internal heat is above the cooling law (helium rain); ice
    giants keep a fixed 40 K instead of Neptune's and Uranus's very
    different heat.
  - The decks use the Sun's composition; an ice giant's methane and
    water enrichment (a methane ice deck on Neptune, water near 40 bar)
    and the deep dry adiabat of its troposphere are not modelled.
  - The great spot has the size it is drawn with; the real one shrinks
    by 0.19° of longitude a year (Simon et al. 2018), which needs a
    date in the viewer.
  - Cloud tops only: no view of the depths where Juno sees the belt and
    zone contrast change sign (Fletcher et al. 2021).

## 15. Names

- **Status:** done
- **Problem:** worlds were known by their id (`terran-42`), giants by a
  bare letter counted among the giants only, moons by a bare numeral,
  the star by its spectral type; nothing could be named.
- **Done (2026-11-04):** scientific designations after the IAU's
  conventions for exoplanets (`mapgen::naming`): the star's catalogue
  number in a fictional survey (`MGS 46433`), a letter for each planet,
  the world included, by its distance to the star, a Roman numeral for
  each moon by its distance to its planet. The server gives them in each
  world's summary, computed from `world.json` (older worlds included),
  and `PUT /api/worlds/{id}/names` names the world and its moons
  (`names.json`), edited from the world card.
- **Later:** naming the star and the giants too; keeping given names
  across a regeneration with `replace`.

## 16. Planetary systems

- **Status:** in progress
- **Specification:** [system-spec.md](system-spec.md)
- **Problem:** the generator made one world, its moons, its giant
  neighbours described in its `world.json` and the giant it orbits; the
  other bodies of its system had no card and no map, and several of
  their properties were drawn in the viewer from seeds.
- **Decided (2026-11-05):** a whole system (a star and planets by type,
  15 at most) or one around a planet asked as before; every body drawn
  at once with its card and a preview, its maps generated on demand with
  its neighbourhood; the map size chosen at creation; the worlds made
  before deleted rather than migrated.
- **Done (2026-11-05):**
  - phase 1, the draw (`src/system.rs`, `system.json`);
  - phase 2, generating a body and its neighbourhood (`bodies/<id>/`);
  - phase 3, the server's system routes and jobs;
  - phase 4, the system library and the two-mode form;
  - phase 5, the viewer (spheres from previews, generation on demand
    with its progress, skies and the star view from `system.json`);
  - the CLI writing systems (`systems/<id>/`) and the deployment
    mounting `systems/`.
- **Next:** phase 6, a card for every body and renaming any of them;
  phase 7, the clean-up of the world layout (`/api/worlds`, the moon and
  host exports, `worlds/`).

## 17. Asteroid belts and Lagrange points

- **Status:** in progress
- **Plan:** [2026-11-06-asteroid-belts.md](superpowers/plans/2026-11-06-asteroid-belts.md)
- **Analysis:** [13-asteroids.md](analysis/13-asteroids.md)
- **Problem:** a system has a star, planets and moons, but no small
  bodies: no asteroid belt, no Kuiper belt, no trojans, and its Lagrange
  points are not shown.
- **Studied (2026-11-06):**
  - belts: what they are, their masses and depletion, the giants'
    resonances that set their edges and gaps, sizes and spacing (no dense
    "asteroid fields"), and belts round other stars;
  - Lagrange points: their stability (Routh's criterion), the tadpole
    and horseshoe orbits of trojans, the Solar System's swarms and their
    origin, co-orbital moons, and the PDS 70 b candidate.
- **Proposed:** belts and trojan swarms drawn in `system.json` from a
  stream of their own, so existing systems keep their bodies; shown in
  the star view with the selected planet's Lagrange points. Five choices
  are left to the user (section 5 of the analysis).
- **Decided (2026-11-06):**
  - an inner belt, an outer belt and a scattered disk, only where physics
    and observations allow;
  - trojan swarms drawn in the star view, and ringed and labelled from
    afar in the planet view, where they are invisible;
  - all five Lagrange points of the selected planet;
  - a crater rate weighed by the belts;
  - new systems keep the first cold giant's belt zone free of rocky
    planets.
- **Step 1 (2026-11-06):** the draw (`src/belts.rs`, `system.json`),
  the belt zone kept clear, and the crater rate.

## 18. Rings and ring moons

- **Status:** in progress
- **Analysis:** [analysis/14-rings.md](analysis/14-rings.md)
- **Problem:** giants' rings are drawn twice with different odds
  (`giant_world.rs` for giant worlds, `giants.rs` for a rocky world's
  giant neighbours), not before a giant of a system is generated, and
  with gaps at random places tied to no moon; no moon ever sits in or
  next to the rings, and an ice giant with a captured Triton can get a
  regular moon inside its Roche limit.
- **Findings (2026-11-06):** bright rings end at the fluid Roche limit
  (about 2.0–2.5 radii round a gas giant, 2.27 for Saturn), dust rings
  wherever a moon feeds them (Saturn's Phoebe ring reaches about 270
  radii); every giant of the Solar System has 2 to about 10 moons in or
  at the edge of its rings (shepherds, gap-clearing moonlets, resonance
  edges, dust sources).
- **Proposed:** one ring draw per giant at the system level; the bright
  rings' edge from the Roche limit; gaps cleared by embedded moonlets
  (7–17 of their Hill radii) or set by a moon's 2:1 resonance; edge
  moons and shepherds as `ring-moon` bodies; dust rings fed by moons
  later; first, the Roche check missing from the moons of giant worlds.
- **Decided (2026-11-06):** the four steps now, one pull request each:
  the Roche fix, one draw per giant at the system level, the ring moons
  with their gaps and the Roche edge, then the dust rings.
- **Done (2026-11-06):** one ring draw per giant at the system level
  (`given_rings` in a giant's `WorldParams`, stream `RINGS`), shown in
  the star view before generation; a rocky world's giant neighbours draw
  their rings with the presets' odds.
- **Done (2026-11-06):** ring moons (`rings.rs`, stream `RING_MOONS`):
  bright rings ending at the Roche limit of their particles, gaps
  cleared by embedded moonlets or opened by a moon's 2:1 resonance, edge
  moons and shepherd pairs as `icy-moon` bodies with a `ring_role`, shown
  in the body menu and on the cards.
- **Done (2026-11-06):** dust rings fed by moons (`dust_rings`, stream
  `DUST_RINGS`): gossamer rings of impact dust out to the edge moons of
  bright rings, and plume rings round an active icy moon; drawn faint in
  the planet view of the giant and listed on the cards. A Phoebe-type
  ring waits for the systems' irregular moons.
- **Done (2026-11-07):** the rings follow the science more closely: the
  Roche limit alone sets a bright ring's edge (no cap at 2.5 radii), the
  giant asked for is shaped as the others, the 2:1 resonance cuts an
  edge with a division outward of it, placed with the giant's `J2`,
  moonlets grow from cores until they fill their Roche lobe, edge moons
  grow with distance, and giants with faint rings get small dust-source
  moons and dust rings (Jupiter's gossamer rings, Uranus's μ ring).
- **Remaining:** a plume moon chosen by its tidal heating (an icy moon in
  resonance, its eccentricity forced: Peale et al. 1979 for Io, Meyer &
  Wisdom 2007 for Enceladus), not by drawn odds; a Phoebe-type ring, once
  systems have irregular moons; the A ring's outer edge set by an outer
  moon's 7:6 resonance (Janus); a faint D ring inside bright rings; the
  equatorial ridges of Pan and Atlas.

## 19. Tides, spin and moons: what the science fixes left

- **Problem:** the science fixes of 2026-11-07 (`spin.rs`, `moons.rs`)
  model the 3:2 spin–orbit resonance, circularisation, light moons of
  despun planets and giants, and tidal shapes. Second round (2026-11-08):
  - done: capture into the 3:2 and 2:1 resonances is drawn from the
    world's seed with odds that rise linearly with the eccentricity
    (`spin::capture`: Ribas et al. 2016, Makarov 2012), `mercury-like`
    pins 3:2 (`[star] resonance`), instead of a sharp line at e = 0.06;
  - done: a despun giant on an eccentric orbit turns at the
    pseudo-synchronous rate (Hut 1981), locked only within 5 % of one turn
    per orbit;
  - done: a rocky planet's locking and circularisation times scale with
    its own mass and radius (`m / R³`, `m / R⁵`);
  - done: rocky planets draw their eccentricity from Van Eylen et al.
    2019 (Beta(1.58, 4.4) single-transiting, Beta(1.52, 29)
    multi-transiting), giants
    keep Kipping 2013;
  - remains: resonances above 2:1 (5:2, 3:1 and over: Makarov gives no
    threshold for them) and the Makarov
    triaxiality dependence of the odds (the ramps are Mercury's, a more
    symmetric planet is caught at lower eccentricities);
  - remains: a 3:2 world takes the rotating climate whatever its solar
    day (two years: 176 days for Mercury), not a slow rotator's (the
    climate track);
  - remains, not done for lack of a model: a moon's tilt is the Moon's
    6.68° for a giant-impact moon and 0 for the others, not its own
    Cassini state. The closed form needs the node's precession rate
    (the star's and the other moons' perturbations), the moon's moments
    of inertia and its `C22`, none of which the app draws;
  - remains, decided to leave: inclinations of moons are to the
    planet's equator, the stability limits to its orbit (the Laplace plane
    would need the same precession model); Elser et al. 2011's odds of a
    large moon apply round every star (the despun planets of cool stars
    already lose their moons by `lasting_moon_kg`, and the paper's grid
    does not extend the odds to other masses).
- **Done when:** the remaining items are done: higher resonances, slow
  rotators get a climate of their own, moons get their Cassini obliquity.
- **Status:** in progress

## 20. Relief science, what remains

- **Status:** todo
- **Problem:** the science articles on craters, plates and mountains
  (2026-11-07) list departures of the generator from the science. The
  fixes of 2026-11-07 (rocky gravity, `1/g` relief, crater depths, peak
  rings, diffusion, size law, saturation) leave these, each needing a
  model the app does not have yet.
- **Craters:**
  - The transition from simple to complex craters follows gravity only:
    a target's strength would move it (Mercury about 12 km where the app
    gives 6.6, Earth 3 to 5 km against 2.5, Barnouin et al. 2012). Icy
    moons turn complex far sooner than rock at the same gravity (2 to
    3 km on Ganymede, Schenk 1991, 2002, a number read only in secondary
    sources), so `icy-moon` worlds get a transition several times too
    large. Done when the transition reads a strength per crust (rock,
    volatile-rich rock, ice) from a checked source.
  - No multi-ring basins (Orientale): their onset does not scale as
    `1/g` and depends on a weak layer under the surface (Melosh &
    Ivanov 1999), with no rule to scale it from the Moon.
  - The lunar chronology on most worlds: `[geology] impact_ratio` and
    `crater_scale` carry a body's flux and crater size against the
    Moon's, set for `mars-like` from Hartmann's isochrons (2.6 and
    0.751). Mercury, Venus and Earth keep 1: Le Feuvre & Wieczorek 2011
    (doi:10.1016/j.icarus.2011.03.010) could not be read (paywall, HAL
    behind a bot wall), and Marchi et al. 2009 give Mercury's production
    function only in figures. A world of another system keeps the
    Moon's flux, scaled by its giants and belts and by locking; a
    general rule would need the flux of impactors and their speeds per
    orbit (gravitational focusing included).
- **Plates:**
  - Slab pull sets the present speed of each plate from its sinking
    edge (2026-11-07), on one pass over the present boundary cells; at
    another epoch that pass is extra, about 5 % of a plate map (0.24 s
    at −100 Ma). The drift's other stages keep drawn speeds: setting
    theirs too means finding each stage's boundaries, a pass per stage.
    The plate count does not follow the planet's size: no source scales
    it (Bird 2003 counts Earth's).
  - Isostasy: continents float as a platform and the sea floor is
    bounded between the shelf and Parsons & Sclater's old floor
    (`generation::floating`, 2026-11-07), calibrated on Earth's
    hypsometry (ETOPO 2022). What remains: no crustal-thickness field
    (the platform's height is a calibration rather than Airy's balance
    of a thickness), no flexure (volcanoes and ranges do not bend the
    lithosphere into moats), and the platform floats at the waterline,
    so a sea level lowered on a world exposes platform, not old basins.
  - The age of old ocean floor is unknown: floor older than about 18 Ma
    lies between 4 and 6.4 km by the noise (`generation::floating`), not
    by its age.
- **Small bodies:** their noise relief is capped (8 % of the radius;
  9 km on airless stagnant bodies, ice 0.4 of rock), but an icy body's
  simple craters are still rock's bowls, under rock's transition (see
  the craters' first point).

## 21. Climate: what the model leaves out

- **Status:** todo
- **Source:** the science articles "How does axial tilt make climate zones
  and seasons?" and "Why is one side of a mountain range dry?" (their
  "Where it departs from the science" sections), after the climate fixes
  of 2026-11-07 (exact insolation, seasons, Clausius-Clapeyron rain,
  meridional winds, foehn).
- **Cold eastern-boundary currents** (Humboldt, Benguela, California): the
  sea off the west coasts of the subtropics is several degrees colder,
  which dries the coast (the Atacama, the Namib). A rule cooling the sea
  pixels west of a continent between about 10° and 40° would need a
  sourced magnitude and width (SST climatologies), the ocean basins found
  row by row, and the moisture pass to read the cooled sea; the
  temperature anomaly noise stands in for it today.
- **Monsoons:** the seasonal reversal of the wind between land and sea.
  The seasons now set summer and winter temperatures, but the wind and
  the rain are annual means: a monsoon needs a summer and a winter run of
  the moisture pass with winds driven by the land-sea contrast.
- **Uneven seasons on eccentric orbits:** the annual mean keeps the
  `1/sqrt(1 − e²)` factor, but the seasonal swing ignores the distance to
  the star at the solstices (Mars: 46 % more light at perihelion). It
  needs a longitude of perihelion, drawn or given, in `WorldParams`.
- **Atmospheric mass and the wind belts:** Kaspi & Showman (2015) find
  narrower Hadley cells under heavier atmospheres, without a scaling the
  app could use; the belts scale with rotation and size only.
- **Winter-rain climates and the season of the rain:** Köppen's dryness
  threshold depends on the season of the rain (Kottek et al. 2006). The
  app has annual rain only and takes the summer share from the air's
  capacity at the half-years' temperatures, so rain peaking in winter
  (the Mediterranean, BSk and Csa borders) is not modelled; deserts are
  7.5 % of the land and steppes 22 %, where Earth's are about half and
  half. A monthly moisture pass (see Monsoons) would give both.
- **Absolute rain is calibrated, not derived:** the rain rate is scaled
  to GPCP's land mean and zonal means with constants that have no source
  (`MM_PER_RATE`, `COLD_RAIN_EXPONENT`, `CIRCULATION_OCTAVES`,
  `ITCZ_OCTAVES`, `SUBSIDENCE_HUMIDITY`). Done when the rain comes from a
  moisture budget (evaporation, convergence) in physical units.
- **Circulation on tilted worlds:** the wind belts, the circulation
  factor and the ITCZ follow the geographic latitude, whatever the
  tilt. At 90° the poles are the warm annual-mean regions and the rain
  there reaches 4 000 mm a year, from warm air over a nearby sea, while
  the cells stay where a vertical axis would put them. A circulation
  following the thermal equator (the latitude of the annual-mean
  insolation maximum) would need a source for the Hadley cell's response
  to the tilt (Kaspi & Showman 2015 vary obliquity). A world with no
  tilt has ice on half of its 45-60° land: with no seasons the warmest
  month is the annual mean, below 0 °C there (Köppen's EF), which is
  physical.
- **Storms, fronts and ENSO:** moisture is the annual mean of a steady
  wind. Weather, interannual variability (El Niño, La Niña) and the storm
  rain that makes most orographic precipitation are model limits, not
  planned.

## 22. Storage

- **Status:** decided (2026-11-08)
- **Analysis:** [analysis/15-storage.md](analysis/15-storage.md)
- **Problem:** everything is files on one host's disk: listing systems
  opens every `system.json`, jobs are lost on a restart, source data,
  derived files and diagnostics share one directory (about 30 % of the
  3.9 GB could be rebuilt or dropped), and the user's names sit next to
  generated files.
- **Decided:** tens to hundreds of systems, one user, one Docker host
  (perhaps a server later); no object store. The steps, each its own pull
  request:

| Step | Change                                                                                                   | Status |
| ---- | -------------------------------------------------------------------------------------------------------- | ------ |
| 1    | SQLite index owned by the server: systems, bodies, seeds and requests, names, jobs, last opened, generator version; rebuilt from the tree when missing; its file on a host bind mount (`./db:/data/db`) so it survives restarts | todo   |
| 2    | `world.svg` and `noise_maps/` produced only on request: an "Export SVG" action and a debug option        | todo   |
| 3    | Eviction: a body not opened for 14 days loses its generated files; opening it again regenerates it, with the current generator (an older version draws differently) | todo   |
| 4    | Export: the seeds and requests (JSON) and the images (PNG) of a system, from the app                     | todo   |
