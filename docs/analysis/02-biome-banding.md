# Analysis: biome bands and hard transitions

Backlog item 2. Goal: on Earth-like worlds, irregular and varied biome
regions at continental scale, no contour-like altitude rings, ecotones at
borders, and a crisp, valid exported biome layer. Every preset keeps working.

Measurements: release build, `--planet terran --volcanic 0.3
--circumference 40000`, 1920 × 1080, eight worlds: seeds 3, 8, 21, 11
(Earth-like, sea level 0.2–0.3), 42 and 7 (sea level 0.1 and 0), 5 and 9
(supercontinents, sea level −0.25 and −0.1); tilts 15°, 23.44°, 28°, 35°.

## 1. Method

A script reads `raw_data/` (biome ids, elevation, temperature, moisture)
and measures, on land borders between two different land biomes
(ice cap to snow, below 75° of latitude):

- **Latitude alignment**: share of border pixels whose normal (gradient of
  the smoothed biome indicator, in km) lies within ±20° of north-south, i.e.
  borders that follow a parallel. An isotropic mosaic gives 0.22. Measured
  on the local border (2 px smoothing) and at continental scale (400 km).
- **Contour alignment**: same, against the elevation gradient.
- **Driver**: for each pair of neighbours, which input crosses a threshold
  (swap the neighbour's temperature, moisture or elevation into
  `BiomeRules::choose`): T-, M- or E-driven border.
- **Anisotropy**: probability that a step of 500 km east, or north, lands
  in a different biome.
- **Zonal climate**: share of the land temperature and moisture variance
  explained by latitude alone (R² of the row means), and the standard
  deviation of temperature at a fixed latitude.
- **Render**: share of land pixels drawn as contour lines; colour step
  across a border in `world.png` relative to the flat palette step.

## 2. Root causes

Baseline, mean of the eight worlds (range in brackets):

| Measure                                | Baseline              |
| -------------------------------------- | --------------------- |
| Temperature R² by latitude (land)      | 0.993 (0.984–0.999)   |
| Temperature std at a fixed latitude    | 0.026 (all lapse)     |
| Moisture R² by latitude (land)         | 0.76 (0.61–0.93)      |
| Borders aligned with parallels         | 0.49 (0.33–0.69)      |
| Same, at 400 km scale                  | 0.55 (0.37–0.77)      |
| Borders aligned with contours          | 0.25 (0.20–0.40)      |
| T-driven borders, aligned w. parallels | 40 %, 75 % aligned    |
| M-driven borders, aligned w. parallels | 57 %, 22–40 % aligned |
| E-driven borders, aligned w. contours  | 3 %, 98 % aligned     |
| Biome change, 500 km north / east      | 27 % / 11 % (2.5×)    |
| Land pixels on a contour line          | 2.1 %                 |
| Land pixels on a biome border          | 1.6 %                 |
| Colour step at a border (flat = 1)     | 0.91                  |

(a) **Temperature is a function of latitude.** Latitude explains 99 % of
land temperature; at a given latitude it varies by 0.026 (about 1 °C if
1 unit ≈ 50 °C), all from the weak lapse rate. Any threshold on it
(`polar`, `boreal`, `temperate`) is a straight parallel: 75 % of T-driven
borders follow one. Earth's zonal anomalies (continentality, currents,
monsoons) reach 3–6 °C of standard deviation, i.e. 0.06–0.12.

(b) **Moisture is zonal on large continents.** The rain term saturates at
−1 in dry interiors, so what is left is the `cos(6·lat)` band (±0.25) plus a
noise of only 0.06 std (`0.30 × fbm`, fbm std ≈ 0.20). On Earth-like worlds
M-driven borders are close to isotropic (22–35 %), but supercontinents get
R² 0.93 and pure stripes (`base_s5_s7.png`). Thresholds sit inside the wet
half of the range, so on large land masses the dry classes take whole
belts (desert 35–40 % of land on seeds 5, 7, 9).

(c) **Hard thresholds** turn every smooth field into a 1-pixel, perfectly
regular line. Altitude bands are a small part of it: `highland` (0.70) is
measured on `elevation − sea_level`, and raw elevations top out near 0.67,
so Mountain and Snow **never appear** with a sea level ≥ 0 (seeds 3, 8, 11,
21, 42, 7 have none). When they do (seeds 5, 9) their borders are 98 %
contour lines. Temperature borders do not follow contours (0.22, isotropic
level): the lapse rate is too weak to draw rings.

(d) **Rendering draws the rings.** Nine contour levels are darkened by 40 %
on land: 2.1 % of land pixels, more than all biome borders (1.6 %). On
Earth-like worlds these lines, not biomes, are the visible altitude rings.
Flat colours give a full-strength step at every border (0.91; the rest is
hillshade), so no ecotone can exist, whatever the generation.

Minor: the 100 km box blur of precipitation leaves rectangular edges
(straight vertical borders in `crop_s3_base_render_dither_combined.png`).

## 3. Options

All were prototyped in a scratchpad copy behind environment variables
(`prototype.diff`); "K" is the combination recommended below.

| Variant        | Parallels | 400 km | North / east | Ice cap | Colour step |
| -------------- | --------- | ------ | ------------ | ------- | ----------- |
| Baseline       | 0.49      | 0.55   | 27 % / 11 %  | 19 %    | 0.91        |
| Continentality | 0.54      | 0.59   | 30 % / 11 %  | 25 %    | 0.91        |
| T anomaly (A)  | 0.39      | 0.45   | 28 % / 14 %  | 19 %    | 0.93        |
| Dither (B)     | 0.25      | 0.29   | 31 % / 24 %  | 19 %    | 0.95        |
| Soft (C)       | 0.49      | 0.55   | 27 % / 11 %  | 19 %    | 0.34        |
| A + B + C (K)  | 0.25      | 0.29   | 33 % / 25 %  | 19 %    | 0.30        |

Isotropic reference for the alignment columns: 0.22. Ice cap is its share
of land. Images: section 6.

### A. Large-scale temperature anomalies

Add `0.40 × fbm(p·1.2, 3 octaves)` (std 0.08, features of ~3 000 km) to the
land temperature, half on the sea. It stands for currents, monsoons and
continentality without modelling them.

- **Visual**: bands meander at continental scale; zones shift by several
  degrees of latitude (up to ~15°) between continents. Alone it leaves
  the local borders regular (0.39 aligned).
- **Plausibility**: amplitude matches Earth's zonal anomalies. Zero-mean,
  so ice-cap share is unchanged (19 %).
- **Data**: stored in `temperature`, which still explains every biome.
- **Cost**: one fbm per pixel; not measurable (1.85 s per world either way).
- **Presets**: shifts every preset the same way; the frozen world gains
  taiga/glacial mosaics. Sea ice becomes patchy at the polar edge.
- **Variant tested and rejected for now: continentality** carried by the
  zonal wind (maritime index decaying 2 500 km inland, cold interiors at
  high latitude). Physically better, but interiors stay uniformly
  "continental", so it shifts bands without breaking them (0.54 aligned,
  worse), streaks along rows, and grows ice caps by a third. Worth
  revisiting with an ice cap that needs snowfall (moisture), not just cold.

### B. Regional climate variability ("dither") and a ragged highland line

Add a multi-octave fbm (`p·6`, 5 octaves: features from ~1 000 km down to
~60 km) to the stored climate before biome selection: `0.25×` on
temperature (std 0.047, ~2 °C) and an independent sample `0.50×` on
moisture (std 0.09); raise the moisture noise weight from 0.30 to 0.50.
Jitter the `highland` test by `0.50 × fbm(p·4)` (std 0.09).

- **Visual**: the largest gain. Borders become irregular and interleaved
  (patches of each biome on both sides, an interleaved ecotone), at the
  local and continental scale: 0.25 aligned, a 500 km step east changes
  biome 24 % of the time instead of 11 %. Highland borders drop from 98 %
  to 33–63 % contour-aligned.
- **Plausibility**: amplitudes of local climate variability (soils, aspect,
  regional circulation); forest–tundra ecotones span hundreds of km.
- **Data**: the dither lives in the exported `temperature` and `moisture`,
  so the crisp biome layer is still explained by the exported climate. The
  highland jitter is not in any layer: export it (raw layer or noise map),
  or fold it into the biome elevation used by `choose`.
- **Cost**: two fbm per pixel; not measurable.
- **Presets**: works through their offsets and remaps; on barren worlds it
  changes almost nothing (no climate-driven classes).
- **Interaction**: hydrology reads `moisture`, so rivers move slightly.
  The noise is on the unit sphere, so a deep-zoom tile can add finer
  octaves of the same fbm and stay consistent with the base world.

Two moisture tweaks tested and dropped: shifting the circulation band by
±8° × fbm and adding a 0.15 regional octave changed nothing measurable.

### C. Render-side ecotones and contour lines

Colour each land pixel by the mean palette colour of the biomes reached by
small offsets of its own climate (σ_t = 0.04, σ_m = 0.10, 5 × 5 Gaussian
weights through `rules.choose` and `remap`). Pixels far from a threshold
keep their flat colour; pixels near one blend. Drop contour lines on land
(keep them under water as bathymetry); hillshade already shows relief.

- **Visual**: gradual ecotones (colour step 0.91 → 0.34), no rings; coasts,
  rivers, shores and volcanic classes stay crisp (not blended).
- **Data**: render only; `biomes.png` is untouched (checked byte for byte).
- **Cost**: 25 rule evaluations per land pixel, not measurable.
- **Presets**: uses the preset remap, so palettes stay preset-specific.
- **Caveat**: 5 discrete offsets show faint terraces; use an analytic
  membership instead (per threshold, `smoothstep` of the distance to it
  over σ, combined along the decision tree).
- **Rejected alternative**: blurring biome colours over a pixel radius
  (radius 3 tested). It smears every border equally, looks out of focus,
  and its width is in pixels, so it would change with every zoom level. The
  climate-space blend has a physical width and suits a shader or tile
  renderer (item 1).
- **SVG**: soft colours defeat the run-length encoding (5.1 → 11.1 MB);
  keep `world.svg` flat.

### D. Moisture transport (not prototyped)

Supercontinent interiors stay desert and shrubland belts even with B,
because rain is pinned at its minimum. A meridional or monsoon moisture
source (ocean humidity pulled inland at low latitudes), or moisture
thresholds expressed as quantiles of the land distribution, would fix
those worlds. It is a deeper climate change, best scheduled with the
river work (item 3), which depends on the same precipitation.

## 4. Recommendation

Do **A + B + C**, in that order of value: B first (most of the gain), C
second (ecotones and rings), A third (continental meanders). Defer D and
continentality.

Values found good (fbm std ≈ 0.20 for all sources used):

| Parameter                        | Value                 | Effective std |
| -------------------------------- | --------------------- | ------------- |
| Temperature anomaly, land / sea  | 0.40 / 0.20, `p·1.2`  | 0.08 / 0.04   |
| Climate dither on temperature    | 0.25, `p·6`, 5 oct    | 0.047         |
| Climate dither on moisture       | 0.50, `p·6`, 5 oct    | 0.09          |
| Moisture noise weight            | 0.30 → 0.50           | 0.10          |
| Highland jitter                  | 0.50, `p·4`, 4 oct    | 0.09          |
| Soft render σ_t / σ_m            | 0.04 / 0.10           | n/a           |
| Contour lines on land            | off                   | n/a           |

Dither frequency 3 instead of 6 was worse (0.29 aligned, fewer borders).

### Plan (one commit each, docs included)

1. `feat(climate): add large-scale temperature anomalies` — new Perlin
   (seed + 400) in `Sampler`; anomaly added in pass 3; constants in
   `climate.rs`; `noise_temperature_anomaly` diagnostic map; README
   "Temperature" section.
2. `feat(climate): add regional variability to temperature and moisture` —
   dither noise (seed + 500, two samples), moisture noise weight 0.50,
   dither added to the stored climate; README "Moisture" and noise-map list.
3. `feat(biome): jitter the highland line` — `highland_offset` in
   `BiomeInput`, exported as a layer so the data still explains each biome;
   README raw-data table. Optionally make `highland` reachable at positive
   sea levels (relative to the land relief): a separate, visible decision.
4. `feat(export): blend biome colours across climate thresholds` — analytic
   soft membership in `render_pixels` for PNG; SVG stays flat; README
   "Output".
5. `feat(export): draw contour lines under water only` — README "Output".
6. `docs: record the biome banding decision` — backlog status, changelog
   note that the same seed now gives different biomes.

### Risks

- **Reproducibility**: every seed changes; worlds generated before the
  change cannot be regenerated identically. Say so in the changelog.
- **Rivers**: B and A change moisture and temperature, so river networks
  move; coordinate with item 3.
- **Sea ice** becomes patchy with the sea anomaly; lower it to 0.1 if it
  looks wrong.
- **Preset tuning**: custom presets with tight `[rules]` ranges (e.g.
  `mars-like` polar 0.12) see ragged caps; acceptable, but check visually.
- **Deep zoom**: item 1 interpolates the stored climate; dither octaves
  above the base resolution must continue the same fbm, and the soft blend
  must be the same function in the tile renderer.

### Tests

- Determinism (existing) and `reachable()` preset tests keep passing.
- New: dither and anomaly fields are zero-mean within a tolerance on a
  small world; land ice-cap share changes by less than 2 points.
- New: soft colour equals the flat colour when no offset crosses a
  threshold; `render_pixels` options never change `world.biomes`.
- New regression: on a 256 × 128 Earth-like world, the share of land
  borders aligned with parallels stays below 0.35 (baseline 0.49).
- Visual: the eight worlds above and every preset, before/after.

## 5. Reproducing

`metrics.py` computes every number above from a world directory;
`run.sh <name> ENV=…` generates the eight worlds with the prototype binary
and prints the metrics. Prototype toggles: `MG_TNOISE`, `MG_TNOISE_OCEAN`,
`MG_DITHER_T`, `MG_DITHER_M`, `MG_DITHER_E`, `MG_NOISE_W`, `MG_SOFT_T`,
`MG_SOFT_M`, `MG_CONTOUR_LAND`, `MG_CONT`, `MG_MARITIME_KM`.

## 6. Files

Scratchpad root (prototype copy in `repo/`, worlds in `exp_*/`):

- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/`

Images (left before, right after, unless stated):

- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/view/before_after_K.png`
  (seeds 3, 8, 21)
- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/view/crop_s3_base_render_dither_combined.png`
  (1:1 crop; top: baseline, C only; bottom: B only, K)
- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/view/presets.png`
  (volcanic, frozen, caustic, barren, mars-like)
- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/view/base_s5_s7.png`
  (baseline supercontinents and their moisture)
- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/view/base_s3_diag.png`
  (baseline temperature, moisture, precipitation, moisture noise)

Scripts and prototype:

- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/metrics.py`
- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/run.sh`
- `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/biomes/prototype.diff`
