# Analysis: geology, impact craters and geological time

Backlog items 8 and 9. User request: study how plate tectonics works,
whether every kind of planet has plates, whether the surface of a dead
world changes (impact craters), and whether continental drift could be
shown with a time slider in millions of years.

Today the elevation is a pure function of the position: Perlin octaves
on the sphere (`Sampler::sample` in `generation.rs`), ridged mountains
and volcanic cones. The `PLATE_STEP` cell noise only gives coasts
straight stretches and corners: there are no plates, no plate
boundaries, no craters and no notion of time.

## Decisions (2026-11-03)

Taken with the user:

- **The scientific findings are recorded here** (sections 1 to 3), for
  this work and for the plate tectonics to come.
- **Now:** impact craters, and a time slider that moves the surface of
  dead worlds through their cratering history (sections 4 to 7).
- **Later:** plate tectonics and continental drift (section 8). The
  simulation approach studied is too costly for now; it waits until a
  cheaper way of drifting continents is found.

## 1. Tectonic regimes

Plate tectonics is the exception: among the bodies of the Solar System
only Earth has it, active and confirmed. Its lithosphere (crust and
rigid upper mantle, about 100 km thick) is broken into about fifteen
rigid plates moving over the ductile asthenosphere at 2–15 cm per year.
The main driving force is the pull of the slabs sinking at subduction
zones (*slab pull*), with mantle convection and *ridge push*.

Planetary scientists classify bodies by tectonic regime:

| Regime                    | Mechanism                                                                  | Examples                               |
| ------------------------- | -------------------------------------------------------------------------- | -------------------------------------- |
| Mobile lid (plates)       | Lithosphere broken into plates, recycled into the mantle by subduction     | Earth                                  |
| Stagnant lid              | A single rigid plate; heat escapes by conduction and a few giant volcanoes | Mars, Mercury, the Moon                |
| Episodic lid              | Stagnant for hundreds of millions of years, then overturned globally       | Venus (probably)                       |
| Heat pipe                 | Volcanism so intense that lava resurfaces the planet continuously          | Io; early Earth (Moore & Webb 2013)    |
| Plutonic or "squishy" lid | Weak lithosphere deformed by intrusions, without rigid plates              | Proposed for Venus and early Earth     |
| Ice tectonics             | Plates of ice over a subsurface ocean                                      | Europa (subduction proposed), Ganymede |

What favours plate tectonics, according to the models:

- **Liquid water**, the factor cited most often: it hydrates and weakens
  the lithosphere and lubricates subduction. Venus, almost Earth's size
  but dry, has no plates.
- **Enough internal heat**: small bodies (Moon, Mercury, Mars) cooled
  fast, their lithosphere thickened and froze into a single lid.
- **A surface that is not too hot**: a hot surface heals the damage
  that weakens the lithosphere and pushes towards a stagnant lid
  (Landuyt & Bercovici 2009; Foley, Bercovici & Landuyt 2012), an
  argument for Venus.
- **Size**, debated: for super-Earths, Valencia, O'Connell & Sasselov
  (2007) predict plates, O'Neill & Lenardic (2007) a stagnant lid. The
  question is open, which leaves a generator free to choose.

A cold world can keep its plates: Earth had them during its "snowball"
glaciations. A regime is not fixed either: Venus may have had plates
early on (Weller et al. 2023, from its atmospheric nitrogen), and the
Earth itself probably went through a heat-pipe or squishy-lid stage
before modern plates.

### Bodies of the Solar System

| Body     | Regime                   | Surface                                                                       |
| -------- | ------------------------ | ----------------------------------------------------------------------------- |
| Earth    | Plates                   | Oceans < 200 Ma old, continents up to 4 Ga; about 190 impact structures known |
| Venus    | Episodic or squishy lid  | About 1 000 craters, randomly spread: global resurfacing 300–750 Ma ago       |
| Mars     | Stagnant lid             | Old cratered south, smooth north (dichotomy), Tharsis bulge, Valles Marineris |
| Mercury  | Stagnant lid             | Saturated with craters, lobate scarps from global contraction (radius −7 km)  |
| The Moon | Stagnant lid             | Saturated highlands, maria (impact basins flooded by lava 3–3.9 Ga ago)       |
| Io       | Heat pipe                | No impact crater visible: resurfaced faster than craters form                 |
| Europa   | Ice tectonics (proposed) | Few craters, bands and ridges in the ice shell (Kattenhorn & Prockter 2014)   |

## 2. Plate tectonics and continental drift

Orders of magnitude, used by section 8:

- **Speeds:** 2–15 cm per year, that is 20–150 km per million years
  (1 cm/yr = 10 km/Ma). The Atlantic opens at about 2.5 cm/yr, the
  fastest Pacific ridges at 15 cm/yr; India moved at 15–20 cm/yr before
  hitting Asia.
- **Plate boundaries:**

  | Boundary                        | Relief                                               |
  | ------------------------------- | ---------------------------------------------------- |
  | Divergent (ocean)               | Mid-ocean ridge, new oceanic crust of age 0          |
  | Divergent (continent)           | Rift valley, then a new ocean (East Africa, Red Sea) |
  | Convergent, ocean–continent     | Trench, volcanic arc, coastal range (Andes)          |
  | Convergent, ocean–ocean         | Trench and island arc (Japan, Marianas)              |
  | Convergent, continent–continent | Collision range and plateau (Himalaya, Tibet)        |
  | Transform                       | Strike-slip fault, little relief (San Andreas)       |

- **Hotspots** (mantle plumes) stay roughly fixed while plates move
  over them: chains of volcanoes ageing away from the plume (Hawaii).
- **Sea floor depth** follows the age of the oceanic crust as it cools:
  about 2.5 km + 0.35 km × √(age in Ma) up to about 70 Ma, then
  flattening (Parsons & Sclater 1977). Ridges stand high, old floors are
  abyssal plains.
- **Ages:** the oldest oceanic crust is about 180 Ma old (western
  Pacific), since it is recycled by subduction; continental crust,
  too light to sink, keeps rocks of 4 Ga (Acasta gneiss) and minerals
  of 4.4 Ga (Jack Hills zircons).
- **Supercontinent cycle** (Wilson cycle): about 400–600 Ma. Rodinia
  formed about 1.1 Ga ago and broke up about 750 Ma ago; Pangaea formed
  about 335 Ma ago and broke up from about 175–200 Ma ago. Projections
  of the next one (Pangaea Proxima, Amasia, Novopangaea) look 200–250 Ma
  ahead.
- **Sea level** varies by about ±200 m over these cycles (young, hot
  ridges displace water), so the share of land is not constant in time.

## 3. Impact cratering

Without tectonics the surface still changes: by impacts above all,
and to a lesser degree by early volcanism (lunar maria), global
contraction (Mercury's scarps) and space weathering. Cratering follows
well-established laws.

### How many craters, and how old

- **Size distribution:** a power law, N(>D) ∝ D^−b, with b ≈ 2 from a
  few km to about 100 km on the lunar highlands, steeper below a few km.
  For each 100 km crater there are hundreds of 10 km ones.
- **Chronology:** the crater density dates a surface. The lunar
  chronology of Neukum (1983, Neukum et al. 2001) gives the cumulative
  density of craters of 1 km or more on a surface of age T (Ga):

  ```text
  N(1 km) = 5.44e-14 × (exp(6.93 T) − 1) + 8.38e-4 × T     per km²
  ```

  The flux fell steeply until about 3.5 Ga and has been roughly constant
  since (the linear term). A 4 Ga surface gets about 0.063 craters of
  1 km or more per km²; with b = 2, a Moon-sized body gets about 6 000
  craters of 20 km or more, close to the 5 185 counted on the Moon
  (Head et al. 2010). The shape of the size distribution is assumed not
  to change with time, so the same curve dates every crater size.
  Whether the early flux was a decline or a late spike (the Late Heavy
  Bombardment) is debated (Boehnke & Harrison 2016); for a generator the
  difference does not show.
- **Saturation:** old surfaces such as the lunar highlands reach an
  equilibrium where each new crater erases as many as it adds (Gault
  1970). The density stops growing; the surface stays a jumble of
  overlapping craters.
- **Retention:** on an active world craters disappear faster than they
  form. Earth keeps about 190 known impact structures, most younger than
  500 Ma, erased by erosion, sediment and subduction; Venus none older
  than its resurfacing; Io none at all.

### Shape

- **Simple craters:** bowls about D/5 deep (Pike 1977), with a raised
  rim.
- **Complex craters**, above a transition diameter that scales about as
  1/g (15–20 km on the Moon, 5–8 km on Mars, 2–4 km on Earth): a flat
  floor, terraced walls and a central peak, shallower (on the Moon,
  depth ≈ 1.044 D^0.301 km, Pike 1974).
- **Peak-ring basins** above about 150–200 km on the Moon, and
  **multi-ring basins** above a few hundred km (Orientale, about 930 km
  across its outer ring).
- **Ejecta blanket:** thickness ≈ 0.14 R^0.74 (r/R)^−3 in metres, R the
  crater radius in metres, r the distance from the centre (McGetchin,
  Settle & Head 1973): most of it within one radius of the rim.
- **Rays:** bright streaks of fresh ejecta around young craters
  (Tycho), which fade in about 1 Ga (the lunar Copernican system).

### Degradation

- **Airless bodies:** craters soften by topographic diffusion under
  small impacts, with a lunar diffusivity of about 5.5 m² per Ma
  (Fassett & Thomson 2014): about 120 m of smoothing in 3 Ga, which
  only matters for craters below about a kilometre. Larger craters
  are degraded mostly by younger craters and their ejecta.
- **Worlds with water:** rain and rivers erase small craters within
  millions of years and fill large ones with sediment (Mars kept its
  Noachian craters, but worn).

### Atmosphere

A thick atmosphere breaks up small impactors before they reach the
ground: Venus (92 bar) has no crater smaller than about 1.5–2 km and
few below 30 km; on Earth (1 bar) stony bodies below about 50 m
fragment; on Mars (0.006 bar) craters of a few metres exist. A rough
fit of the smallest crater:

```text
D_min ≈ 0.1 km × (pressure in bar)^0.75     (2 m on Mars, 3 km on Venus)
```

with a deficit up to about ten times `D_min`.

## 4. Geology of a world

A new `[geology]` table in presets, stored in `world.json` with the
resolved preset like `[atmosphere]`:

```toml
[defaults]
surface_age = [3.0, 4.2]     # Ga: age of the oldest surviving surface

[geology]
regime = "stagnant"          # plates | stagnant | episodic | heat_pipe
wet_until = 3.5              # Ga: end of the era of liquid water
```

- `regime` had no effect on the elevation at first: it recorded the
  regime for section 8. It is shown in the world info. Since then
  `plates` shapes the relief (section 8) and the carbon cycle, and a
  plate preset's world loses its plates when it is too small or dry
  (`geology::regime_of`, 2026-11-07).
- `surface_age` sets the crater density through the chronology. Like
  the pressure, it is a generation parameter (`--surface-age`, form
  field, `surface_age_ga` in `world.json`) drawn from the preset's
  `[defaults]`, so it sits there rather than in `[geology]`.
- `wet_until` splits craters in two: those older than it are applied
  before the hydraulic erosion, which wears them (a Noachian Mars);
  those younger are applied after it, fresh. `0` means still wet, and
  a value at or above the surface age means never wet (Mercury, written
  `4.6`, the upper limit): no special value is needed, since no crater
  is then older than it. A world with liquid water today (pressure
  above `LIQUID_WATER_BAR`) is forced to 0 (`Geology::wet_until_ga`).

Presets without the table get Earth's values: `terran` worlds keep a
few large, worn craters.

| Preset         | Regime    | Surface age (Ga) | Wet until (Ga) | Expected look                                      |
| -------------- | --------- | ---------------- | -------------- | -------------------------------------------------- |
| `terran`       | plates    | 0.2–0.6          | 0              | Rare, large, eroded craters, none on the sea floor |
| `earth-like`   | plates    | 0.4              | 0              | Same                                               |
| `frozen`       | plates    | 0.5–2.0          | 0              | Some craters, softened                             |
| `caustic`      | episodic  | 0.3–0.8          | 0              | Few craters, none small (dense air)                |
| `volcanic`     | heat_pipe | 0.001–0.05       | 0              | Almost none: lava resurfaces the planet            |
| `barren`       | stagnant  | 3.0–4.2          | 3.5            | Cratered, old craters worn, young ones fresh       |
| `mars-like`    | stagnant  | 4.0              | 3.5            | Mars: worn southern craters, fresher young ones    |
| `venus-like`   | episodic  | 0.5              | 0              | About a thousand craters, none below 2 km          |
| `mercury-like` | stagnant  | 4.0              | 4.6 (never)    | Saturated, fresh-looking                           |

**Outcome (step 1).** As planned, with the surface age in `[defaults]`
and "never wet" written as a `wet_until` at the upper limit. The
surface age is drawn after every other parameter, the seed included, so
the draws of a seeded request do not move. Worlds made before it have
no `surface_age_ga` in `world.json`: the viewer shows their regime only,
the reproduce command leaves the age to the preset, and a reloaded world
gets an age of 0 (no craters), which is what they have. Worlds do not
change.

## 5. Craters (generation and tiles)

### A crater field as a function of position

Craters follow the pattern of the volcanic cones (`Sampler::cone`, one
point per lattice cell with `nearest_point`): a pure function of the
position on the sphere, deterministic per seed, with a seed stream of
its own. This keeps them consistent between the base map and the detail
tiles, and lets the time slider ask for the surface at any epoch.

- **Octaves of size:** diameters are split into octaves `[D_k, 2 D_k]`,
  from the smallest crater the atmosphere lets through (or the tile's
  pixel) to the largest basin. Each octave has its own 3D lattice, sized
  so that the expected count per cell matches the chronology:
  `n_k = N(1 km) × D_k^−2 × 0.75` per km², capped at the saturation
  density. Each cell holds zero or more craters, with a diameter in the
  octave (power law), an age and a profile jitter drawn from its hash.
- **Ages:** drawn by inverting the chronology between 0 and
  `surface_age`, so most craters of an old surface are very old.
  Craters are also drawn in the future, at the constant late flux, up
  to 1 Ga ahead (negative ages), for the slider; they are absent at the
  present epoch.
- **Profile:** a radial function of the distance in crater radii: a bowl
  (simple) or a flat floor with a central peak (complex) and terraces,
  a rim, and an ejecta blanket in `(r/R)^−3`; depths and rim heights in
  km from Pike's laws; the simple–complex transition at
  `15 km × 1.62 / g` (g in m/s², from the gravity already derived from
  the circumference), 0.15 of that on icy bodies (a calibration with no
  readable source; their simple craters keep rock's bowl); a peak ring
  above about ten times the transition.
  A low-frequency noise makes rims and ejecta irregular.
- **Superposition:** craters are applied in age order, oldest first.
  Inside a crater the previous relief is replaced, not added to: the
  floor is levelled at its own depth below the rim, and ejecta bury the
  older relief in proportion to their thickness. Adding profiles would
  stack bowls into deep pits on saturated surfaces.
- **Degradation:** each crater is smoothed with the diffusion length
  `√(κ × exposure)`, its exposure being its age minus the epoch shown;
  κ = 5.5 m²/Ma on airless worlds. Craters older than `wet_until` are
  also worn by the hydraulic erosion pass. On worlds with liquid water
  today, craters smaller than about 20 km are dropped (erased in a few
  Ma) and, under the `plates` regime, craters on the sea floor too
  (oceanic crust is younger than 200 Ma and buried in sediment).
- **Atmosphere:** craters smaller than `D_min` (section 3) are dropped,
  with a deficit up to ten times `D_min`.
- **Units:** crater depths are in km, while the elevation has no unit
  yet (open point of analysis 07). This step fixes an `ELEVATION_KM`,
  calibrated so that Earth-like worlds' highest peaks and deepest
  trenches read as about 9 and 11 km, and uses it for craters only.

### In the pipeline

```text
noise → craters older than wet_until → erosion → craters younger than wet_until → temperature → …
```

Craters make closed basins: on wet worlds the depression filling turns
them into crater lakes. Biomes follow from the new elevation; crater
interiors and ejecta get no biome of their own in this step.

### At the base resolution and in the tiles

- The base map (about 20 km per pixel at 1920 px on an Earth-sized
  planet) holds the craters of two base pixels or more. Smaller ones
  exist only in the detail tiles, which evaluate the same crater field
  down to two tile pixels: zooming in reveals smaller and smaller
  craters, as on real images.
- `raw_data` moves to `format_version` 7 with a new layer,
  `elevation_base.png`: the elevation after erosion and before the
  young craters. `elevation.png` keeps the present elevation, so the
  tools that read it do not change. The tiles render from the base
  layer plus the young craters at the tile's own resolution, so crater
  rims stay sharp at every zoom level instead of being interpolated.
  Older worlds have no craters and no base layer: their tiles do not
  change.
- **Cost:** a pixel checks the neighbouring cells of about ten octaves.
  Per tile, the craters that overlap the tile are gathered once per
  octave, then each pixel only visits that short list. The target is to
  stay within twice the current 4–6 ms per tile; it is measured in the
  step that adds the tiles.

### Outcome (step 2)

Implemented in `craters.rs`, with these changes from the plan:

- **Lattice:** a cube-sphere grid per octave rather than a 3D lattice:
  each cell holds a Poisson number of craters expected from its exact
  solid angle, so the density is uniform over the sphere. Octaves are
  fixed (`[2^k, 2^(k+1)]` km) and cells depend only on the planet and
  the seed, as step 3 needs.
- **`ELEVATION_KM` = 25**, from Earth-like worlds (seeds 3, 7, 42): their
  median sea floor sits at −0.16 units (4 km on Earth), their highest
  peaks at 0.30–0.39 (8.8 km). The deepest sea floors then read as 13–18
  km, deeper than Earth's trenches: the noise has no trench to calibrate
  on.
- **Smallest crater on the base map: 4 pixels**, not 2. At 2.5 pixels
  small pits read as rings around a bright dot and their floors fell
  into the dark basin biome of `mercury-like`, which made the ground
  look bubbly. Smaller craters are left to the tiles.
- **Ejecta stop at 3 radii**, not 4: beyond, they are thinner than 4 %
  of the rim, and the outer ring cost half of the pixels drawn.
- **Wet worlds:** a single wear law instead of dropping craters under
  20 km: a crater keeps `exp(−exposure / τ)` of its relief, with
  `τ = 0.1 Ga × (D / 20 km) / erosion` and the exposure the time it spent
  while water was liquid (none without erosion). Earth-like worlds keep
  a few large faint craters, Mars-like ones worn Noachian basins.
- **Sea floors:** craters vanish under a sea that still fills the basins,
  decided from the preset (its ocean is still a water biome: water,
  ice-sealed, acid or magma), not from the pressure. With the pressure,
  `venus-like` (92 bar, seas remapped to basalt) lost all its craters.
  The wet era itself still follows the pressure (`Geology::wet_until_ga`).
- **Diffusion** of crater profiles is not applied: it smooths about
  120 m in 3 Ga, far below the base pixel. It belongs to the tiles.
- **Parallel drawing:** craters are grouped in waves of craters that
  share no pixel, each after every older one it overlaps; a wave is
  computed in parallel and written at once, which gives exactly the
  result of drawing one crater at a time (tested). Basins, alone in their
  wave, compute their rows in parallel. Generations with and without
  craters take the same time within run-to-run variation (4 Ga
  `mercury-like`: about 1 s at 1920 × 1080, 18 s at 8192 × 4096), so the
  craters run inside the Erosion stage, without a progress stage of
  their own.
- **Not done:** `--land-fraction` samples the relief without craters, so
  on heavily cratered worlds the land share can drift slightly from the
  target. No preset combines the two today.
- **Debug map:** `noise_maps/craters.png`, the height change of both
  passes.

### Outcome (step 3)

- **The tiles do not redraw the base map's craters.** The plan rendered
  the tiles from `elevation_base.png` plus every young crater at the
  tile's resolution. But a crater replacing the relief needs the ground
  level at its site when it formed, which a tile cannot know: part of the
  crater lies outside it, and its older neighbours too, so two tiles
  would disagree at their border. Craters of 4 base pixels or more stay
  in the interpolated base elevation, as in step 2; tiles add the smaller
  ones (from 4 tile pixels).
- **Small craters are added, not replaced.** An added profile depends
  only on the position: it does not depend on the drawing order or on
  the window, so neighbouring tiles agree exactly (tested). At saturation
  small cavities cover about 7 % of the ground per octave, so adding
  them digs no pits. The sea rule reads the base elevation at the
  crater's centre.
- **Search around a tile.** `CraterField::near` samples the cap around a
  tile (closer than a quarter of the smallest cell), collects the lattice
  cells it touches and draws their craters: exactly the craters of the
  whole sphere (tested), in about 0.6 ms.
- **Relief scaled down with size.** The hillshade exaggerates relief
  about 60 times (`RELIEF_KM`, tuned on the base map's gentle slopes), so
  small crater walls came out vertical and black. A tile crater's relief
  is scaled by `D / D_base`, clamped to `[0.1, 1]`: the base map's look
  at its smallest crater, no jump at that size.
- **Biomes ignore small craters.** Their floors fell below the old sea
  level of `mercury-like` and turned to its dark basin biome. Biomes now
  follow the tile's ground without its small craters; the hillshade
  shows them.
- **Diffusion** (planned for the tiles) is applied to every crater's
  wear: `exp(−κ t / (0.05 R²))` with `κ` = 5.5 m²/Ma. Sub-kilometre
  craters of old surfaces fade; base map craters lose under 1 %.
- **Cost:** 10–12 ms per tile on a 4 Ga `mercury-like` against 5 ms
  without craters (rows in parallel), a little over the target of twice.
  Craters larger than the tile are evaluated at every pixel; a coarser
  grid for them could close the gap if needed.
- **`elevation_base.png`** is exported (format version 7, about 2.5–3 MB
  at 1920 × 1080) but not loaded by the tiles. The time slider (step 4)
  will draw the young craters older than the epoch on it at the base
  resolution, as the generation does, then let the tiles add the small
  ones of that epoch. `RENDER_VERSION` is 3.

### Outcome (step 4)

- **Module `epoch.rs`:** `range_ga` (the epochs a world offers) and
  `loaded_at`, which draws the young craters formed by the epoch on
  `elevation_base` with the generation's own `CraterField::apply`, then
  recomputes the temperature (lapse rate) and the biomes. It works on the
  loaded world (16 bits per value) and consumes it, so computing an epoch
  holds only the elevation and temperature in full precision besides it.
  Epoch 0 gives `elevation.png` back within two 16-bit steps: the base is
  the final elevation minus the young craters' change, in floating point.
- **Craters still to come:** a second population with a seed stream of
  its own (`FUTURE_CRATERS`), at the constant late flux, ages uniform over
  the next 1 Ga. A crater exists at `t` when its age is `t` or more, and
  diffuses for `age − t`.
- **Server:**
  - `GET /api/worlds/{id}/epoch/{ma}/{surface|elevation|biomes}.png`
    with `scale` 1, 2 or 4, and `epoch=` on the tiles. A world at an
    epoch is an entry of its own in the tile cache (keyed by id,
    revision and epoch), within the same budget.
  - `epochs_ma` in the world summaries.
  - Errors: 404 for a world without epochs, 400 out of its range.
- **Rounding is the client's:** the server accepts any whole Ma in the
  range; the slider of step 5 rounds to its steps, so that cached URLs
  repeat.
- **Cost:** 0.17 s to compute an epoch and 0.11 s to render its surface
  at 1920 × 1080 (4 Ga `mercury-like`), 2.9 s and 1.8 s at 8192 × 4096.
  The reduced layers are averaged from the full render, so `scale` saves
  transfer, not time; rendering them directly at the lower resolution
  would, if the slider needs it.
- **Not done:** structured error details (`ErrorDetail`) for the epoch
  errors, for translated messages in step 5.

### Outcome (step 5)

- **Slider** under the tools, in their row (they move and wrap with the
  pane's width; a slider floating at the top centre covered them on
  narrow panes). The past is on the left, the future on the right; the
  label ("3.8 Ga ago", "present", "in 250 Ma") explains Ma and Ga in its
  tooltip. Stops as planned, both ends kept.
- **Dragging** shows the base map of the epoch at a quarter of its
  resolution after a pause of 120 ms, detail tiles hidden; releasing
  loads the full map, the tiles at the epoch, and the elevation and
  biomes (a Web Worker decodes them; the relief, the contours, hovering
  and the point panel follow). The other layers, the biome statistics,
  the hex grid and the comparison stay at the present.
- **The epoch chosen is kept from world to world**, each showing its
  nearest stop. Resetting it on a world change raced with a link
  opening a world at an epoch; keeping it avoids that, and compares
  worlds at the same epoch.
- **Keys:** `,` / `.` one stop older / younger, `<` / `>` ten stops.
  **Link:** `epoch=<Ma>` when not at the present.
- **Checked in a browser** on `mercury-like` and `mars-like` worlds:
  4 Ga ago shows an almost smooth surface, 2 Ga ago nearly the present
  (most craters are older than 3.5 Ga), the future adds a few; no console
  error.

### Outcome (step 6)

- **Rays and ejecta:** a fresh crater lights its floor, its ejecta
  (`(r/R)⁻³`) and 6 to 14 rays of its own, 8 to 30 radii long and
  narrowing with distance, all darkened by space weathering (`1/e` in
  250 Ma, gone in about 1 Ga). Drawn on worlds without liquid water,
  from craters one pixel wide, into a new 8-bit layer `albedo.png`
  (format version 8). Multiplying the shaded colour, as first tried,
  barely showed on the dark ground of `mercury-like`; a positive shift
  now takes the colour part of the way to white, and rays read like the
  Moon's. The tiles read the layer at the base resolution (no rays for
  their own small craters); at an epoch the rays are those of the
  craters fresh then. About 0.1 s at 1920 × 1080 on a 4 Ga
  `mercury-like`.
- **Maria:** `[geology] maria = [start, end]`. A share of the large
  basins (30 % to 90 % with the volcanism) is flooded at a date of the
  window after it formed, the floor filled flat up to a level between
  its lowest point and its rim mean, and darkened (albedo −0.35). Floods
  are events in the age order of the impacts, drawn by the same waves
  (a flood keeps within its basin's reach). They are all drawn in the
  dry pass, after the erosion, even those older than the end of the wet
  era: the mask of flooded pixels then comes out of the same pass at any
  epoch, which a flood baked into `elevation_base` would not give. Lava
  plains wear little, so the cost is small.
- **Presets:** `barren` 3.9–3.1 Ga (the Moon), `mars-like` 3.7–3.0 (the
  Hesperian plains), `mercury-like` 4.0–3.6 (its smooth plains). Their
  worlds change.

### Outcome (transition from the crust, #312, 2026-11-14)

Part 3c of the planet model (#423). A planet's simple–complex transition
now follows O'Keefe & Ahrens (1993), `D_t = 9.0 Y / (ρ g)`, with the
strength and density of its crust read from its model
(`generation::crust_kind`): dry rock (12.5 MPa at 2.8 g/cm³), wet rock
1.5 times weaker (no plates, no runaway, over 0.001 Earth oceans kept),
or ice (an ice world without liquid water, the icy moons' calibration
written as a strength). `earth-like` turns complex at 4.0 km (3–5),
`mercury-like` at 10.9 (10–12), `mars-like` at 7.2 (7–8), `venus-like`
at 4.4. The strengths are calibrations on those bodies; nothing is read
from an archetype's name. Moons have no model yet and keep
`15 km × 1.62 / g` (0.15 of it on ice) until part 4: the planets' rock
strength would put the Moon at 24 km. A frozen Earth keeps a rock crust
under its ice sheet. Worlds with craters change.

## 6. Time slider

### What it shows

The slider moves the **epoch** of the surface: 0 is the world as
generated, positive values go back in time (Ma before present),
negative values go forward, up to 1 Ga. At an epoch `t`:

- craters with an age of `t` or more are shown, the others not yet;
- each crater shows the degradation of its exposure (age − t): going
  back in time freshens them;
- the biomes and the hillshading follow the elevation at `t`.

On a world with liquid water, changing the relief would also change its
lakes, rivers and erosion, which the slider does not recompute: the
slider is only offered on worlds **without liquid water today**
(pressure below `LIQUID_WATER_BAR`), and it stops at `wet_until`, before
which erosion reshaped the craters. In practice it covers the dead
worlds the request was about: `barren`, `mars-like`, `mercury-like`, and
any world of thin enough air.

**Decisions (2026-11-04),** taken with the user before step 4:

- **Future:** up to 1 Ga ahead, craters drawn at the constant flux of
  the last 3 Ga, from a population of their own so the present craters
  do not move.
- **Past:** back to the surface age (4 Ga on `mercury-like`, where the
  oldest craters vanish one by one down to an almost smooth surface),
  still capped at the end of the wet era on worlds that had water.
- **Units:** Ma and Ga, unusual for most users, are explained in the
  interface: a tooltip wherever a unit appears, and a table of units
  with reference values in the help panel (with bar and g), in English
  and French.

### Server

- `GET /api/worlds/{id}/epoch/{ma}/{layer}.png`, `layer` one of
  `world`, `elevation` and `biomes`: the base-resolution layers that
  change with the epoch, rendered from `elevation_base.png` and the
  crater field with the generation's own rules. An optional `scale`
  (1/2, 1/4) renders a reduced version for dragging.
- Tiles take an `epoch` query parameter.
- Epochs are rounded to the slider steps, and URLs name the world
  revision, the render version and the epoch, so browsers cache them for
  good like tiles. The server keeps the last few epochs rendered in
  memory, within the tile cache budget.
- Expected cost at 1920 × 1080: a few hundred milliseconds per epoch
  (base layer plus about ten octaves of craters on 2 M pixels, in
  parallel), about 16 times less at 1/4 scale. Measured in that step.

### Viewer

- A slider below the view, on worlds that allow it, labelled in Ma or
  Ga, with a non-linear scale (fine steps near the present, 1 Ma up to
  100 Ma, 10 Ma up to 1 Ga, 50 Ma beyond), a "present" button and a
  keyboard shortcut.
- While dragging, the 1/4-scale layers replace the base surface and its
  relief; on release, the full layers, then tiles at that epoch. The
  epoch and the slider position are kept in the URL state like the
  other view settings.
- Picking and the info panel read the epoch's elevation and biome. The
  hex overlay, the biome statistics and the comparison mode stay at the
  present epoch in this version.

## 7. Order

Each step is a pull request:

| Step | Change                                                                                                        | Seeds  |
| ---- | ------------------------------------------------------------------------------------------------------------- | ------ |
| 1    | `[geology]` in presets and `world.json`, `--surface-age`, form field, regime in the world info; no effect yet | Same   |
| 2    | Crater field in the generation: chronology, morphology, superposition, atmosphere, wet era; `ELEVATION_KM`    | Change |
| 3    | `elevation_base.png`, `format_version` 7; craters in the detail tiles down to the tile pixel                  | Same   |
| 4    | Epoch: server routes and tile parameter, rendering of a world at an epoch                                     | Same   |
| 5    | Viewer: time slider, reduced layers while dragging, epoch in picking and URL state                            | Same   |
| 6    | Optional: ray systems and ejecta albedo fading with age; lava flooding of basins (maria) at given epochs      | Change |

Step 2 is the only step that changes worlds (every preset, a little on
`terran`, a lot on `barren`).

## 8. Plate tectonics (deferred)

Studied, then deferred by the user until a cheaper way of drifting
continents is found. What was studied:

- **Approach:** a simulation on a point mesh of the sphere (about
  100–150 k points, 60 km apart), following Cortial et al., *Procedural
  Tectonic Planets* (Eurographics 2019). Plates are rigid rotations
  about Euler poles; at each step of 1–5 Ma, points move, subduction,
  collision, rifting and new oceanic crust are resolved, and the crust
  (continental or oceanic), its thickness and age and the orogenic belts
  are updated. The result would replace the coarse continent octaves;
  mountains and volcanic arcs would follow the convergent boundaries,
  and the existing noise would keep the fine detail. Other references:
  PlaTec (Viitanen 2012, on a flat map), Tectonics.js (Davidson), and
  for real reconstructions GPlates and Scotese's PALEOMAP.
- **Time slider:** a deterministic simulation from a random start
  (−300 Ma), the generated world being its state at 0 and the future its
  continuation (+200 Ma). Keyframes every ~10 Ma, a low-resolution
  preview per epoch, and "generate at this epoch" for a full world.
- **Why deferred:** the simulation is a large new subsystem, its tuning
  (credible continents, an emerging supercontinent cycle) is the bulk of
  the work, every `terran` seed changes, and each epoch would need its
  own climate, rivers and biomes.

The `[geology] regime` of section 4 is where this work will hook in.

### Cheaper approaches (research, 2026-11-04)

Asked by the user: how procedural world generators get plates and
drift at a lower cost than a simulation.

**What others do.**

- **Simulations step by step** (Cortial et al. 2019; PlaTec and
  WorldEngine on a flat map; Tectonics.js): the cost of the approach
  above, and hard to control. Frozenfractal's *Around The World*
  simulated about 43 000 Poisson discs on the sphere in 12 plates (move,
  detect overlaps, subduct, slow colliding plates, raise mountains,
  spawn crust at divergent boundaries) and dropped it: "too slow, and
  gave me too little control over the output". WorldEngine reports
  thresholds hard to pick for PlaTec; Tectonics.js reports oceanic crust
  that grows implausibly old, since it is not destroyed.
- **Static plates** (frozenfractal's current generator, Hex Planets on
  Roblox, Almost Hex Planet on Unreal, Fusek 2020): jagged Voronoi
  "subplates" grouped into about 15 plates, each split into continental
  and oceanic parts, one velocity per plate (frozenfractal derives it
  from the neighbours: "oceanic crust is pulled under continental
  crust"), the boundary type from the relative motion of adjacent plates
  (converging or diverging; transform boundaries ignored, since "it's
  always sections of the boundary either converging or diverging"), and
  relief features drawn along the boundaries as curves, with noise on
  their width and height:

  | Boundary                        | Feature                                  |
  | ------------------------------- | ---------------------------------------- |
  | Continent–continent, converging | Mountains and plateaus                   |
  | Continent–continent, diverging  | Rift valley                              |
  | Ocean–ocean, diverging          | Mid-ocean ridge                          |
  | Ocean–ocean, converging         | Island arc                               |
  | Ocean–continent, converging     | Trench on the oceanic side, volcanic arc |

  Cheap and controllable, and it ties mountains to plates, but nothing
  moves: there is no drift.
- **Rigid reconstructions** (GPlates, the geologists' tool): a plate's
  position at any time is a rotation about an Euler pole, constant over
  a stage (stage poles) and changing from one stage to the next (total
  poles chain them back to the present). Any epoch is computed
  directly, with no steps before it.

**Proposal: kinematic drift, without simulation.** The static plates of
the second approach, moved by the rotations of the third. Each epoch is
a function of `t`, as the crater epochs are:

- **Plates fixed at the present:** Voronoi subplates on the sphere,
  grouped into about 12–15 plates, each with an Euler pole and an angular
  speed, constant over stages of 50–100 Ma, smoothed from one stage to
  the next by quaternion interpolation (slerp).
- **Continents at `t`:** each pixel finds the plate whose region,
  rotated to `t`, covers it, and reads the present continental relief
  (today's noise) at its point of origin `R(t)⁻¹ · p`. Continents move
  rigidly; rotations invert, so going back and going ahead cost the
  same.
- **Boundaries without memory:** everything follows from the geometry
  at `t`.
  - Collision: how deep a plate's region reaches into its neighbour's
    gives the convergence accumulated since contact, and so the height
    and width of the range.
  - Opening: the distance to the edge of a gap, divided by the spreading
    speed, gives the age of the new sea floor, and its depth through the
    cooling law of section 2 (about 2.5 km + 0.35 km × √(age in Ma)).
  - Subduction: a trench on the sinking side, an arc on the other.
- **Cost (estimate):** finding the plate of each pixel among a few
  hundred rotated seeds, with a spatial index, at 512 × 256: a few
  milliseconds to a few tens of milliseconds per epoch. Then, as for the
  craters, a preview (the whole pipeline at 512 × 256, about 0.1–0.2 s
  per slider stop) and "generate at this epoch" for a full world.
- **Limits:** continents do not deform; crust is not conserved exactly
  (subducted crust disappears, new crust fills the gaps); the range of
  epochs stays bounded (±200–300 Ma), beyond which constant poles give
  absurd overlaps unless stages are chained.

**Outcome (step 1, first part, 2026-11-04).** `plates.rs` builds the
plates and their boundaries, exported as diagnostic maps only
(`noise_maps/plates.png` and `plate_boundaries.png`): no world changes.

- Subplates: the cells of a jittered 3D lattice (frequency 5 on the unit
  sphere, about 300 cells) looked up at a position displaced by two
  octaves of noise (0.08), so their borders are jagged.
- Plates: 10 to 15 weighted centres (weights up to 0.35 rad, a power
  diagram), each subplate joining the nearest, so plates of varied sizes
  follow the subplates' borders and wrap around the map.
- Motion: one rotation vector per plate, a random Euler pole at 20 to
  100 km/Ma (2 to 10 cm/yr).
- Boundaries, on a grid of at most 1024 × 512: a pixel next to another
  plate is a boundary; its convergence is the relative speed of the two
  plates across the boundary. The direction across is taken over a 7 × 7
  window, towards the other plate's pixels: from one neighbour only it
  took one of 8 directions, which jumped along stepped boundaries and
  combed the map. A multi-source Dijkstra gives every pixel its distance
  to the nearest boundary and that boundary's convergence; far from the
  boundaries this nearest-source value forms facets, which only matter
  within the reach of boundary relief (a few hundred km).

**Outcome (step 1, second part, 2026-11-04).** The boundaries shape the
relief of worlds with plate tectonics (`regime = "plates"`: `terran`,
`frozen`, `earth-like`; their seeds change):

- Each boundary pixel records the crust on its side and across
  (continental above the edge of the shelf, 0.05 under the waterline,
  from the continents' shape alone, so only boundary pixels are sampled);
  the propagation carries the nearest boundary's index, and each coarse
  pixel gets its relief, interpolated at full resolution. Reading the
  nearest coarse pixel's plate and boundary at full resolution switched
  between the overriding and the subducting side from one pixel to the
  next: steps.
- Features as in the README's table: collision ranges (0.12, 350 km),
  coastal ranges and arcs over subducting oceans, trenches (0.16), island
  arcs, mid-ocean ridges (0.1, 450 km), rifts; full from 5 cm/yr, arcs
  from 2 cm/yr. The noise's mountains keep half their weight.
- **Arc volcanoes:** a cone lattice of their own (200 km, 80 %): the
  volcanic provinces' 900 km lattice almost never lands a cone on an arc
  80 km wide. Arcs then reach a volcanic zone of 0.87; but the ash and
  volcano biomes also need an elevation above the sea level reference
  (`ash_elevation = 0`, 0.15 above the waterline, about 3.75 km), which
  coastal arcs seldom reach: only the cones' summits turn volcanic (about
  0.03 % of the land, none before). That threshold is the cause of the
  smaller backlog item "volcanoes are rare"; changing it would also move
  the ash of the volcanic preset, so it is left to that item.
- **Poles:** the latitude-longitude grid stretches boundaries near the
  poles into streaks; relief fades out between 70° and 85°, as erosion
  does between 60° and 75°. A grid even over the sphere (a cube map)
  would lift this (done below).
- **One grid for every map size** (1024 × 512): the boundaries, and so
  the relief, do not depend on the resolution, and the land fraction
  estimate (sampled at 512 × 256) sees the same relief.
- On three `terran` seeds, mountains go from 8.9, 3.4 and 5.3 % of the
  land to 8.4, 5.2 and 5.5 %, the land share gains 1.5 points (arcs and
  ridges). Building the plates takes about 0.13 s per world, which also
  makes the test suite slower (about 10 s instead of 3.5 s for `mapgen`).

**Outcome (uniform grid, 2026-11-04).** The plates and their boundaries
now lie on an equi-angular cube map (`src/cube.rs`): six faces of
256 × 256 cells, about 39 km at a face's centre, cells within 30 % of one
another in size, against the latitude-longitude grid whose cells shrink
to nothing at the poles. Cells across a face edge are found through the
sphere (a cell past the edge is a point of the face's plane extended),
so the 7 × 7 window of the boundary normal, the Dijkstra and the
bilinear interpolation of the relief cross the edges unchanged; the
Dijkstra steps by the great-circle distance between cell centres. The
polar fade is gone: ranges, arcs and ridges reach the poles. 393 216
cells instead of 524 288, generation time unchanged. The new relief
exposed a river case: a tributary crossing its own parent upstream of
its tributaries' mouths was never cut; it now is, and those tributaries
take its course on to its mouth.

**Outcome (step 2, kinematic drift, 2026-11-04).** `--drift <Ma>`
generates a world at an epoch of its plates, within ±300 Ma:

- **Stages:** every 50 Ma each plate gets an Euler pole and a speed
  drawn from the plate and the stage alone (the present motion at stage
  0); the rotation vector moves linearly between stages, and the
  rotations are integrated in 1 Ma steps (Rodrigues' formula), one 3 × 3
  matrix per plate: about 600 compositions per plate at 300 Ma.
- **Cells at the epoch:** each cube cell tests every plate: it belongs to
  plate `k` if `k`'s rotation brings back a point of `k` today.
  Continental crust first, then the lower index; a cell no plate covers
  is new oceanic crust, joined to the nearest covered cell's plate
  (breadth first). The boundaries, their convergence (from the rotation
  vectors at the epoch) and their relief then follow as at the present.
- **Pixels:** each pixel tests the plates of the four cells around it
  exactly (`Plates::plate_at` of its origin), so coasts carried by the
  plates stay as sharp as at the present; the relief noises (continents,
  mountains, volcanic provinces and cones) are sampled at the origin,
  the climate noises, the arcs' cones and the boundary relief at the
  point. New crust is ocean floor 0.3 below the waterline, keeping half
  of the fine octaves.
- At the present the code path is unchanged: `--drift 0` gives the same
  world to the bit. An epoch takes the same time as the present (0.8 s at
  960 × 480).
- Limits: the continents are the present ones carried back or forth, so
  in the past they tear along plate boundaries instead of reassembling
  into an older supercontinent; overlapping crust is hidden, not folded
  into ranges (the collision boundary raises one); a quarter of the
  surface is new crust at 150 Ma, ten random plates moving independently.

**Outcome (step 3, the drift slider, 2026-11-07).** The crater time
slider could not be reused as it was: it redraws craters on a stored
base map, while a drift changes the continents, and with them the
climate, the rivers and the biomes. So the preview runs the whole
pipeline again at a small size instead of reading a stored layer:

- `mapgen::drift::preview` generates the planet from the parameters its
  `system.json` stores, with `drift_ma` set and the map cut to 256 or 512
  pixels wide at the world's aspect ratio, and renders its base map. The
  seed and the epoch give the same pixels. It is not the full world
  downscaled (rivers and small coasts differ), only a way to choose an
  epoch; about 0.4 s at 512 on a 1024 × 512 world.
- `GET /api/systems/{id}/bodies/{body}/drift/{ma}/surface.png` serves
  it, with a small cache in memory, one preview at a time, 503 while a
  generation runs, 404 without plates. Previews never touch the files of
  the world: it is not stored.
- **Generate at this epoch** is the existing `drift_ma` request, as
  *Vary* does for any parameter: the form opens on the same planet with
  the epoch and creates a world of its own (`...-past150ma-<seed>`).
- The slider reads like the request (negative in the past), unlike the
  crater slider (positive in the past); the two never meet, since a
  world with plates has liquid water and so no crater epochs.
- Not done: the preview is a flat map beside the globe, not the globe's
  texture; the climate of a locked world follows the drift as the whole
  pipeline does, but no epoch is kept between the previews.

**Order:**

1. Static plates at the present on `plates` worlds: mountains and
   volcanic arcs follow plate boundaries instead of noise. The best gain
   in realism for the least cost, but every `terran` seed changes.
2. Kinematic drift: Euler poles, the plate of each pixel at `t`, and
   boundary relief without memory.
3. The time slider of the craters, reused, with the low-resolution
   preview and "generate at this epoch". Unlike craters, the climate,
   rivers and biomes are recomputed at each epoch: on a world with water,
   moving the continents changes everything.

**Sources:**

- Cortial, Y., Peytavie, A., Galin, É. & Guérin, É. (2019). [Procedural
  tectonic planets](https://diglib.eg.org/handle/10.1111/cgf13614).
  *Computer Graphics Forum* 38(2).
- Frozenfractal, [Around The World, Part 2: Plate
  tectonics](https://frozenfractal.com/blog/2023/11/13/around-the-world-2-plate-tectonics/)
  (2023) and [Part 29: One year
  in](https://frozenfractal.com/blog/2026/2/14/around-the-world-29-one-year-in/)
  (2026).
- [Ignite the core: plate tectonics in Roblox, Hex Planets dev blog
  II](https://devforum.roblox.com/t/ignite-the-core-plate-tectonics-in-roblox-hex-planets-dev-blog-ii/881011).
- [Almost Hex Planet procedural
  generation](https://forums.unrealengine.com/t/almost-hex-planet-procedural-generation/72326/2),
  Unreal Engine forums.
- r/proceduralgeneration: [first iteration of a tectonic plate
  simulation](https://nyc1.lr.ggtyler.dev/r/proceduralgeneration/comments/1jhiznp/first_iteration_of_my_tectonic_plate_simulation)
  and [reducing a tectonic plate
  simulation](https://reddit.birdcat.cafe/r/proceduralgeneration/comments/1jqese5/reducing_my_tectonic_plate_simulation_algorithms)
  (informal).
- Michel, É., Emilien, A. & Cani, M.-P. [Generation of folded terrains
  from simple vector
  maps](https://diglib7.eg.org/bitstream/handle/10.2312/egsh.20151019.077-080/077-080.pdf?sequence=1).
  Eurographics short papers 2015.
- Fusek, [Procedurální generování
  planet](https://dspace.vut.cz/items/2cb341e2-b6a0-4b4c-a02f-7a7d3d39e836),
  bachelor's thesis, Brno University of Technology, 2020.
- Davidson, [Tectonics.js](https://davidson16807.github.io/tectonics.js/blog).
- Viitanen, L. (2012), PlaTec: [PyPlatec](https://pypi.python.org/pypi/PyPlatec/1.1)
  and [WorldEngine issue #140](https://github.com/Mindwerks/worldengine/issues/140).
- [Plate reconstruction](https://en.wikipedia.org/wiki/Plate_reconstruction),
  Wikipedia, and [GPlates: Euler pole and
  angle](https://gwsdoc.gplates.org/rotation/euler-pole-and-angle).

### Tidally locked worlds (2026-11-04)

Since the host star (backlog item 11, `docs/analysis/10-stars.md`), a
planet whose tidal locking time (`star::locking_gyr`, about
`166 Gyr × a⁶ / M²`) is shorter than its star's age keeps one face to
its star: an "eyeball" climate around the substellar point, at the
centre of the map. What it changes for geology:

- **Impact craters: a leading hemisphere.** A synchronously rotating
  body keeps one hemisphere facing its orbital motion, the apex: it
  sweeps up more impactors, at higher speeds and more head-on, so it
  gets more and larger craters than the trailing hemisphere (Zahnle et
  al. 2001 on the satellites of the giant planets; Le Feuvre & Wieczorek
  2008, 2011 and Wieczorek & Le Feuvre 2009 on the Moon). The Moon's
  apex is at 0° N, 90° W of the sub-Earth point; for a planet locked to
  its star, the substellar point takes that place, so the apex lies on
  the equator 90° west of it (longitude 90° on our map, the substellar
  point being at 180°). The strength depends on the impactors' encounter
  speed against the orbital speed (slower impactors, stronger
  asymmetry): models of near-Earth objects give a leading-to-trailing
  ratio of about 1.14 to 1.32 for the Moon, observations more, an
  unresolved gap. A planet close to an M dwarf orbits fast, which would
  strengthen it, but its impactor population is unknown. Cheap to add:
  a crater density modulated by `1 + k cos γ` (γ the angle to the apex),
  for the craters younger than the locking (the planet locked
  `age − locking_gyr` Ga ago; older craters formed while it rotated),
  with `k` for a leading-to-trailing ratio of about 1.3 by default.
- **Rays.** Space weathering darkens them faster under a stronger
  stellar wind and flares (M dwarfs), and on the day side; no
  quantitative model was found: kept uniform.
- **Plate tectonics: hemispheres.** A strong day-to-night contrast of
  surface temperature can organise mantle convection by hemisphere:
  upwelling under one, subduction under the other ("hemispheric
  tectonics", Meier et al. 2021 for the hot super-Earth LHS 3844b, with
  either direction depending on the lithosphere's strength and the
  internal heating). For the kinematic drift above: on a locked world
  the Euler poles could be drawn so that plates converge towards one
  hemisphere, instead of at random.
- **Drift moves the climate.** A locked world's climate depends on the
  longitude, around a substellar point fixed to the planet's figure by
  its tidal bulge: drifting continents move into or out of the warm
  "eye", so the climate must be recomputed at each epoch (step 3 of the
  order above), and the time slider would show it. A reorientation of
  the whole shell (true polar wander, moving the substellar point
  itself) is out of scope.
- **Tidal heating** comes from an eccentric orbit, flexed at every turn:
  around M dwarfs it can be strong enough for global volcanism, or to
  dry a planet through a runaway greenhouse ("tidal Venuses", Barnes et
  al. 2013), until the orbit circularises. Orbits are circular here,
  so there is none; an eccentricity would be the hook for a heat-pipe
  regime on locked worlds.

**Sources:**

- Le Feuvre, M. & Wieczorek, M. A. (2008, 2011), and Wieczorek, M. A.
  & Le Feuvre, M. (2009), [Did a large impact reorient the
  Moon?](https://www.mso.anu.edu.au/PSI/PSI_Meetings/Entries/2009/4/16_Special_Seminar__Did_a_large_impact_reorient_the_Moon_files/Wieczorek2009.pdf)
  *Icarus* 200; see also [Analytical formulation of lunar cratering
  asymmetries](https://arxiv.org/pdf/1608.03853) and
  [Lunar cratering asymmetries with high orbital obliquity and
  inclination of the Moon](https://arxiv.org/pdf/2101.11776).
- Zahnle, K. et al. (2001). Differential cratering of synchronously
  rotating satellites by ecliptic comets. *Icarus* 153.
- Meier, T. G., Bower, D. J., Lichtenberg, T., Tackley, P. J. &
  Demory, B.-O. (2021). [Hemispheric tectonics on LHS
  3844b](https://boris.unibe.ch/153279/). *ApJL* 908, L48
  ([arXiv:2103.02374](https://arxiv.org/abs/2103.02374)).
- Barnes, R. et al. (2013). [Tidal Venuses: triggering a climate
  catastrophe via tidal heating](https://arxiv.org/pdf/1203.5104).
  *Astrobiology* 13.

**Outcome (step 7 of backlog item 8).** On a locked planet (the star
model's `tidally_locked`), the crater populations are drawn `1 + k`
times denser and each crater is kept with the share
`(1 + k cos γ) / (1 + k)`, γ its angle to the apex (longitude 90°, on the
equator), `k = (1.3 − 1) / (1.3 + 1)`: an apex-to-antapex ratio of 1.3,
about 1.14 from one hemisphere to the other (tested), the mean density
kept. Craters older than the locking (`the star's age −
locking time` ago) are kept with the mean share, wherever
they are. The share is a draw of its own after the crater's others, made
only on locked worlds: unlocked worlds keep the same craters. Larger
craters on the leading side (faster impacts) are not modelled.

## 9. Open points

- Elevation units: `ELEVATION_KM` is introduced for craters only; the
  lapse rate and the relief exaggeration keep their own scales.
- Craters on wet worlds are dropped or worn by a rule, not by the
  erosion of the detail tiles, which has no erosion of its own.
- The slider leaves temperature and moisture at the present epoch: a
  few kilometres of crater relief barely move them on dry worlds.
- A CLI `--epoch` to export a world at a past epoch, if useful.
- Episodic resurfacing (Venus-like) and stagnant-lid landforms (a
  Tharsis-like bulge, a hemispheric dichotomy, contraction scarps) could
  later give dead worlds more than craters.

## References

- Boehnke, P. & Harrison, T. M. (2016). Illusory Late Heavy
  Bombardments. *PNAS* 113, 10802–10806.
- Cortial, Y., Peytavie, A., Galin, É. & Guérin, É. (2019). Procedural
  tectonic planets. *Computer Graphics Forum* 38(2), 1–11.
- Fassett, C. I. & Thomson, B. J. (2014). Crater degradation on the
  lunar maria. *JGR Planets* 119, 2255–2271.
- Foley, B. J., Bercovici, D. & Landuyt, W. (2012). The conditions for
  plate tectonics on super-Earths. *EPSL* 331–332, 281–290.
- Gault, D. E. (1970). Saturation and equilibrium conditions for impact
  cratering on the lunar surface. *Radio Science* 5, 273–291.
- Head, J. W. et al. (2010). Global distribution of large lunar craters.
  *Science* 329, 1504–1507.
- Kattenhorn, S. A. & Prockter, L. M. (2014). Evidence for subduction in
  the ice shell of Europa. *Nature Geoscience* 7, 762–767.
- McGetchin, T. R., Settle, M. & Head, J. W. (1973). Radial thickness
  variation in impact crater ejecta. *EPSL* 20, 226–236.
- Moore, W. B. & Webb, A. A. G. (2013). Heat-pipe Earth. *Nature* 501,
  501–505.
- Neukum, G., Ivanov, B. A. & Hartmann, W. K. (2001). Cratering records
  in the inner Solar System. *Space Science Reviews* 96, 55–86.
- O'Neill, C. & Lenardic, A. (2007). Geological consequences of
  super-sized Earths. *GRL* 34, L19204.
- Parsons, B. & Sclater, J. G. (1977). An analysis of the variation of
  ocean floor bathymetry and heat flow with age. *JGR* 82, 803–827.
- Pike, R. J. (1977). Size-dependence in the shape of fresh impact
  craters on the Moon. In *Impact and Explosion Cratering*, 489–509.
- Valencia, D., O'Connell, R. J. & Sasselov, D. (2007). Inevitability of
  plate tectonics on super-Earths. *ApJ* 670, L45–L48.
- Weller, M. B. et al. (2023). Venus's atmospheric nitrogen explained by
  ancient plate tectonics. *Nature Astronomy* 7, 1436–1444.
