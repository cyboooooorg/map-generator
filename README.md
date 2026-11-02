# Map Generator

This project is a map generator that creates random planet maps using Perlin noise. The generated maps can be used for various applications such as games, simulations, or any other project that requires a random terrain.

## Prerequisites

[mise](https://mise.jdx.dev/) manages the Rust toolchain (pinned in `mise.toml`) and the project tasks.

```bash
mise install
```

Without mise, any Rust toolchain supporting edition 2024 (1.85+) works with plain `cargo` commands.

## Usage

### Build

```bash
mise run build
```

### Run

```bash
mise run gen [-- [OPTIONS]]
```

All parameters are optional. Any omitted value comes from the planet preset when it sets one, and is otherwise chosen **randomly** at startup. The chosen values are printed, along with the exact options needed to regenerate the same world:

```text
Parameters → planet=frozen seed=2590618090 sea_level=0.12 volcanic=0.61 circumference=51823 km tilt=18.7° gravity≈1.29g
World generated → worlds/frozen-2590618090/
Reproduce with: --planet frozen --seed 2590618090 --sea-level 0.1213 --volcanic 0.6107 --circumference 51823.4 --axial-tilt 18.72 --erosion 1
```

### Options

| Flag                    | Values                                                                                                                                                 | Default                      |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- |
| `--planet <preset>`     | built-in preset name (`terran` · `volcanic` · `frozen` · `caustic` · `barren`) or path to a `.toml` preset file, see [Planet presets](#planet-presets) | random built-in              |
| `--list-planets`        | list the built-in presets and exit                                                                                                                     | —                            |
| `--seed <u32>`          | any 32-bit unsigned integer                                                                                                                            | random                       |
| `--sea-level <f32>`     | float in `[-1.0, 1.0]`                                                                                                                                 | random in `[-0.30, 0.50)`    |
| `--volcanic <f32>`      | float in `[0.0, 1.0]`                                                                                                                                  | random in `[0.0, 1.0)`       |
| `--circumference <f32>` | planet equatorial circumference in km (`>= 1`)                                                                                                         | random in `[20 000, 80 000)` |
| `--axial-tilt <deg>`    | axial tilt in degrees, `[0, 90]`                                                                                                                       | random in `[5, 40)`          |
| `--erosion <f32>`       | hydraulic erosion strength, `[0, 5]`; `0` disables it                                                                                                  | `1.0`                        |
| `--width <px>`          | map width, `2`–`16384`                                                                                                                                 | `1920`                       |
| `--height <px>`         | map height, `2`–`16384`                                                                                                                                | `1080`                       |
| `--out-dir <path>`      | directory receiving the generated worlds                                                                                                               | `worlds`                     |
| `--no-raw-data`         | skip the `raw_data/` export (~7 MB at 1920 × 1080)                                                                                                     | off                          |
| `--no-noise-maps`       | skip the `noise_maps/` diagnostic images, and the memory they need (about a third of the peak)                                                         | off                          |

Invalid or unknown options are rejected with an error. Run `target/release/map-generator --help` for the full reference.

**seed** — drives every noise layer. The same seed with the same other options always produces the same world.

**sea-level** — shifts the waterline. `0.0` is the default; positive values raise it (more ocean), negative values lower it (more land).

**volcanic** — controls how much of the mountain chains become volcanic. `0.0` = no volcanoes, `1.0` = most mountain chains erupt.

**axial-tilt** — tilt of the rotation axis. Shapes the temperature profile from equator to poles and places the tropics and polar circles, see [Temperature](#temperature--axial-tilt).

**erosion** — how much rain carves the terrain: droplets per cell of the erosion grid, see [Erosion](#hydraulic-erosion).

**circumference** — equatorial circumference of the planet in kilometres. Drives two derived quantities (noise scale and gravity) described in the [Calculations](#calculations) section below.

### Examples

```bash
# Fully random world
mise run gen

# Frozen planet, everything else random
mise run gen -- --planet frozen

# Earth-like ocean world, moderate volcanic activity
mise run gen -- --planet terran --sea-level 0.3 --volcanic 0.4

# Barren dry rock, no volcanoes
mise run gen -- --planet barren --sea-level -0.5 --volcanic 0.0

# Quick low-resolution preview, images only
mise run gen -- --width 480 --height 270 --no-raw-data --no-noise-maps
```

### Hex grid export

For games and other tile-based uses, an existing world can be resampled onto a hexagonal grid. It is a separate, on-demand step that reads `raw_data/`, so generate the world with raw data first:

```bash
mise run gen -- --seed 42 --planet terran
mise run hex -- worlds/terran-42 --columns 96
```

| Option               | Description                                                                | Default  |
| -------------------- | -------------------------------------------------------------------------- | -------- |
| `<world>`            | world directory produced by `mise run gen`                                 | required |
| `--columns <n>`      | hex columns around the planet, `4`–`4096`; rows follow from the map height | `96`     |
| `--river-flow <km²>` | land hexes whose largest river drains at least this area get a river flag  | `500000` |

It writes `<world>/hex/`:

- `hex.png`: the map painted with one colour per hex, outlined, with a blue dot on river hexes.
- `hex.json`: the grid layout and one entry per hex (row-major, `index = row × columns + col`): majority `biome` id (rivers never win a hex; they set `river` instead), mean `elevation`, `moisture` and `temperature`, `max_flow_km2` and `river`. The biome table is copied from `world.json`.

The same resampling runs in the web app (hex overlay of the viewer), through the `mapgen::hex` library module.

The grid is pointy-top in "odd-r" offset coordinates: odd rows are shifted half a hex east, and columns wrap east-west like the planet. Hexes are regular on the flat map, not on the sphere, so polar hexes cover less real ground than equatorial ones; a true spherical hex grid (geodesic) would be a different, much larger project.

### Project layout

The repository is a Cargo workspace:

| Path                     | Content                                                                                                                                                            |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/mapgen/`         | The `mapgen` library: generation passes, presets, exports, and `WorldRequest` (parameter limits and defaults)                                                      |
| `crates/cli/`            | The `map-generator` and `hexgrid` command-line tools, thin wrappers around the library                                                                             |
| `crates/server/`         | The `mapgen-server` web server: world library, generation jobs with live progress, static files; see its module documentation for routes and environment variables |
| `web/src/lib/api/types/` | TypeScript types of the server API, generated from the Rust types by `cargo test` (ts-rs); do not edit by hand                                                     |
| `presets/`               | Built-in planet presets (embedded in the binaries) and `examples/`                                                                                                 |

The library also reports generation progress: `generate_with_progress` calls back as each stage starts (noise, erosion, temperature, coast, moisture, rivers, biomes). Its `ts` feature derives TypeScript bindings on the types a web front-end needs.

## Web app

A browser app to explore the generated worlds as a 3D globe or a flat map, switch between biome, elevation, temperature, moisture and river layers, raise the relief, light the planet with a seasonal sun, overlay the hex grid, inspect any point, and generate new worlds from a form. It is a personal tool served locally behind the machine-wide Traefik, like the other `*.localhost` apps.

```bash
mise run up      # build and start it on http://planets.localhost
mise run logs    # follow its logs
mise run down    # stop it
```

The container reads and writes `worlds/` of this checkout (worlds generated with `mise run gen` show up too, as long as they have `raw_data/`) and reads the preset files of `presets/`: any `.toml` preset added there is offered in the form, no rebuild needed. Generated files keep the host owner.

| Piece     | Technology                               | Location         |
| --------- | ---------------------------------------- | ---------------- |
| Server    | Rust, axum; calls the `mapgen` library   | `crates/server/` |
| Front-end | Svelte 5, strict TypeScript, three.js    | `web/`           |
| Container | Multi-stage `Dockerfile`, `compose.yaml` | repository root  |

Generations run one at a time (each one already uses every core) and stream their progress to the page. Zooming in loads [detail tiles](#detail-tiles--deep-zoom) rendered on demand by the server (`GET /api/worlds/{id}/tiles/{z}/{x}/{y}?rev=`), cached by the browser for good since the URL names the world revision; the server keeps the two most recently used worlds in memory. The server refuses to overwrite an existing world: delete it first or pick another seed. Maps are limited to `MAPGEN_MAX_PIXELS` pixels and 8192 pixels per side. The server defaults to 4096 × 2048; `compose.yaml` raises it to 8192 × 4096 for the 6 GB Colima VM of the workstation setup. A generation peaks at about 0.3 GB at 1920 × 1080, 1.1 GB at 4096 × 2048 and 3.4 GB at 8192 × 4096 (about 30 s on 6 cores): keep the limit under the memory the container can use, or the server is killed mid-job. With Colima's default 2 GB VM, lower it (`MAPGEN_MAX_PIXELS: "8388608"`) or give the VM more memory (`colima stop && colima start --cpu 6 --memory 6`). The web app skips the diagnostic maps, which would add about 75 bytes per pixel.

For development with hot reload, see [web/README.md](web/README.md) (`mise run dev`).

### Development tasks

| Task                            | Description                                                                         |
| ------------------------------- | ----------------------------------------------------------------------------------- |
| `mise run build`                | Build the optimized binary                                                          |
| `mise run gen`                  | Build, then generate a world                                                        |
| `mise run up` / `down` / `logs` | Start, stop, follow the web app in Docker (see [Web app](#web-app))                 |
| `mise run dev`                  | Run the web server and the front-end with hot reload                                |
| `mise run web:check`            | Type-check and test the front-end                                                   |
| `mise run hex`                  | Resample a generated world onto a hex grid, see [Hex grid export](#hex-grid-export) |
| `mise run test`                 | Run the test suite                                                                  |
| `mise run lint`                 | Check formatting and run clippy (`-D warnings`)                                     |
| `mise run fmt`                  | Format the code                                                                     |
| `mise run clean-worlds`         | Delete every generated world under `worlds/`                                        |

The CI workflow (`.github/workflows/ci.yml`) runs `mise run lint`, `mise run test` (and checks the generated TypeScript types are committed) and `mise run web:check` / `web:build` on every push to `main` and on pull requests.

### Planet presets

A planet type is a TOML preset. The five built-in ones live in `presets/` and are embedded in the binary:

| Preset     | Description                                       | Exclusive biomes                            |
| ---------- | ------------------------------------------------- | ------------------------------------------- |
| `terran`   | Earth-like — full biome spectrum                  | —                                           |
| `volcanic` | Fire world — extreme heat, near-zero moisture     | `MagmaSea`, `ScorchedWaste`                 |
| `frozen`   | Ice world — perpetually frozen                    | `FrozenOcean`, `GlacialPlain`               |
| `caustic`  | Acid world — corrosive atmosphere, toxic wetlands | `CausticLake`, `ToxicSwamp`, `AcidFlatland` |
| `barren`   | Dead rock — arid and lifeless, no water           | `RockyWaste`, `DustPlain`                   |

To create a new planet type, write a preset file and pass its path:

```bash
mise run gen -- --planet presets/examples/mars-like.toml
```

```toml
name = "mars-like"                 # lowercase letters, digits and dashes; used in the output directory name
description = "Small cold desert world with polar caps"

[defaults]                         # a fixed value or a [min, max] range drawn at random
sea_level = [-0.6, -0.4]           # sea_level, volcanic, circumference, axial_tilt, erosion
circumference = 21300

[climate]                          # added to every pixel before biome selection
temperature = -0.08                # [0, 1] scale
moisture = -0.7                    # [-1, 1] scale
volcanic = 0.0                     # [0, 1] scale

[rules]                            # override any biome selection threshold
polar = 0.12

[remap]                            # target = [biomes replaced by it]
DustPlain = ["Ocean", "DeepOcean", "Lake", "River", "Plain", "Desert"]
RockyWaste = ["Mountain"]
```

Every table is optional.

- **`[defaults]`**: command-line options always win; parameters set neither on the command line nor here use the generator defaults listed under [Options](#options).
- **`[rules]`**: accepts any field of `BiomeRules` in `src/biome.rs` (`waterline`, `polar`, `forest_moisture`, `shore_width_km`…), each documented there. Fields left out keep their default.
- **`[remap]`**: runs after biome selection; biome names are the keys of the biome table (`DeepOcean`, `SeaIce`, `LavaField`…). A biome listed under two targets is an error, and so is an unknown field or biome name.

`raw_data/world.json` stores the full resolved preset under `preset`, so a world can be regenerated even if its preset file changes.

### Output

Each run creates a `worlds/<planet>-<seed>/` directory with the following layout:

```text
worlds/<planet>-<seed>/
├── world.png          ← hillshaded biome map with soft borders, bathymetry and reference lines
├── world.svg          ← equivalent vector image
├── legend.png         ← biome legend with planet metadata
├── raw_data/
│   ├── world.json     ← planet parameters, layer encodings, biome table
│   ├── elevation.png  ← 16-bit greyscale layer
│   ├── moisture.png   ← 16-bit greyscale layer
│   ├── temperature.png ← 16-bit greyscale layer
│   ├── flow.png       ← 16-bit greyscale layer, log scale
│   ├── volcanic_zone.png  ← 16-bit greyscale layer
│   ├── coast_distance.png ← 16-bit greyscale layer
│   ├── highland_offset.png ← 16-bit greyscale layer
│   ├── rivers.json    ← river polylines with their flow
│   └── biomes.png     ← 8-bit greyscale, pixel = biome id
└── noise_maps/
    ├── noise_warp_x.png
    ├── noise_warp_y.png
    ├── noise_continent.png
    ├── noise_mountain.png
    ├── noise_mountain_wt.png
    ├── noise_elevation.png
    ├── noise_biome_elev.png
    ├── noise_moisture.png
    ├── noise_volcanic_raw.png
    ├── noise_volcanic_zone.png
    ├── noise_temperature_anomaly.png
    ├── noise_variability_t.png
    ├── noise_variability_m.png
    ├── noise_highland_jitter.png
    ├── erosion.png
    ├── elevation.png
    ├── temperature.png
    ├── coast_distance.png
    ├── precipitation.png
    ├── moisture.png
    └── flow.png
```

| File / folder | Description                                                                                                                                                                                                                                 |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `world.png`   | PNG (1920 × 1080 by default) with biome colours, soft biome borders (ecotones), hillshaded land (light from the north-west), meandering rivers whose width grows with their flow, depth contours under water and geographic reference lines |
| `world.svg`   | Same map as a vector image (run-length encoded `<rect>` rows), without hillshading so the encoding stays compact; suitable for web embedding and scaling                                                                                    |
| `legend.png`  | Auto-sized PNG listing every biome present on the map (colour swatch + name) plus planet metadata (type, seed, sea level, volcanic intensity, circumference, gravity)                                                                       |
| `raw_data/`   | Machine-readable world data, see [Raw data](#raw-data)                                                                                                                                                                                      |
| `noise_maps/` | False-colour PNG for each intermediate noise layer (see below)                                                                                                                                                                              |

#### Soft biome borders

`world.png` draws biome borders as ecotones. Each land pixel takes the mean colour of the biomes its climate selects when nudged by 16 small temperature and moisture offsets (Gaussian, σ 0.02 on temperature and 0.05 on moisture): far from any threshold the colour is flat, near one the colours mix. The width of a transition is set in climate units, not pixels. Coastlines, shores, rivers, lakes and volcanic biomes keep their exact inputs and stay crisp. This is rendering only: `raw_data/biomes.png` and `world.svg` keep crisp, flat classes.

Contour lines are drawn under water only, as bathymetry: on land the hillshade shows the relief, and contours drew rings that read as altitude bands.

#### Raw data

`raw_data/` holds one lossless PNG per layer, pixel-aligned with `world.png` (row 0 = north pole, column 0 = longitude 0), and a `world.json` describing how to decode them.

| File                  | Encoding         | Content                                                                                                                                  |
| --------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `elevation.png`       | 16-bit grey      | Terrain height in `[-1, 1]`, before the sea-level shift (biomes use `elevation − sea_level`)                                             |
| `moisture.png`        | 16-bit grey      | Moisture in `[-1, 1]`, planet-type offset included                                                                                       |
| `temperature.png`     | 16-bit grey      | Temperature in `[0, 1]`, planet-type offset included                                                                                     |
| `flow.png`            | 16-bit grey, log | River flow: `log10(1 + km²)` in `[0, 8]`, the wet-equivalent area draining through the pixel                                             |
| `volcanic_zone.png`   | 16-bit grey      | Volcanic zone in `[0, 1]` as biome selection sees it (planet shift included)                                                             |
| `coast_distance.png`  | 16-bit grey      | Distance to the nearest open water in `[0, 2000]` km (0 on water, capped)                                                                |
| `highland_offset.png` | 16-bit grey      | Local shift of the mountain line in `[-1, 1]`: a pixel is highland when `elevation − sea_level + offset` exceeds the preset's `highland` |
| `biomes.png`          | 8-bit grey       | Biome id of each pixel                                                                                                                   |

A 16-bit pixel `p` decodes to `min + p / 65535 × (max − min)`, with `min` and `max` given per layer in `world.json`. For `flow` (encoding `gray16-log10`) this gives `log10(1 + km²)`: the area is `10^value − 1`. The precision is about `3 × 10⁻⁵`.

`world.json` contains the generation parameters, the `layers` description and the `biomes` table mapping each id to its key, display name, colour and whether it is a water surface:

```json
{
  "format_version": 2,
  "width": 1920,
  "height": 1080,
  "seed": 42,
  "planet_type": "terran",
  "layers": {
    "elevation": { "file": "elevation.png", "encoding": "gray16", "min": -1.0, "max": 1.0 },
    "biomes": { "file": "biomes.png", "encoding": "gray8-biome-id" }
  },
  "biomes": [
    { "id": 0, "key": "DeepOcean", "name": "Deep Ocean", "color": "#0A148C", "water": true }
  ]
}
```

The example is abridged. Biome ids are stable: new biomes only get appended. `format_version` changes when the layout of `raw_data/` changes: version 2 added `volcanic_zone`, `coast_distance` and `highland_offset`, so that every input of biome selection is exported, and `rivers.json`.

`rivers.json` holds the drawn river polylines (version 2 and later; its file name is in `world.json` under `rivers`):

```json
{
  "version": 2,
  "width_law": { "w0_ref_px": 0.9, "q0_km2": 120000, "exponent": 0.5, "max_ref_px": 4, "reference_width_px": 1920, "ink": 0.5, "raster_exponent": 0.6666667, "real_km_per_sqrt_km2": 0.002 },
  "rivers": [
    { "id": 0, "parent": null, "mouth": "sea", "points": [[129.656, -62.275, 158147], [129.802, -62.306, 158175]] }
  ]
}
```

Each river lists `[longitude°, latitude°, flow km²]` from head to mouth; longitudes are continuous along a river, so they may leave `[0, 360)` where it crosses the seam. `mouth` is `sea`, `lake` or `river` (then `parent` is the river it joins). The nominal width is `w0_ref_px × (flow / q0_km2)^exponent`, capped at `max_ref_px`, in reference pixels (1/`reference_width_px` of the circumference); on a raster `W` pixels around the planet a river is drawn `ink × nominal × (W / reference_width_px)^raster_exponent` pixels wide, and never thinner than `real_km_per_sqrt_km2 × √flow` km. Version 1 files (before `ink`) drew the nominal width. About 0.5 MB per world (250 KB gzipped).

The `mapgen` library reloads a world with `export::raw_data::load_world`; for version 1 worlds it recomputes the missing layers from the world parameters and elevation (and has no river polylines).

Reading a layer in Python:

```python
import json
from PIL import Image

meta = json.load(open("raw_data/world.json"))
layer = meta["layers"]["elevation"]
pixels = Image.open("raw_data/elevation.png")
lo, hi = layer["min"], layer["max"]
elevation = [lo + p / 65535 * (hi - lo) for p in pixels.getdata()]
```

#### Noise maps (false colour)

Every intermediate layer of the generation is exported into `noise_maps/` for study and debugging. They share a **jet colour ramp**:

> blue (low) → cyan → green → yellow → red (high)

Signed maps (range `[-1, 1]`) are centred so that `0.0` maps to green, negative values to blue/cyan, and positive values to yellow/red. Unsigned maps (range `[0, 1]`) run the full blue-to-red range.

| File                            | Range     | Description                                                                                      |
| ------------------------------- | --------- | ------------------------------------------------------------------------------------------------ |
| `noise_warp_x.png`              | `[-1, 1]` | Domain-warp field, X axis                                                                        |
| `noise_warp_y.png`              | `[-1, 1]` | Domain-warp field, Y axis                                                                        |
| `noise_continent.png`           | `[-1, 1]` | Continent shape: FBM, fractal outline near the coast, smoothed plains inland                     |
| `noise_mountain.png`            | `[0, 1]`  | Ridged noise (mountain ridges, with two finer ridged octaves)                                    |
| `noise_mountain_wt.png`         | `[0, 1]`  | Mountain blend weight (derived from continent height)                                            |
| `noise_elevation.png`           | `[-1, 1]` | Elevation before erosion = continent + mountain × weight × blend                                 |
| `noise_biome_elev.png`          | `[-1, 1]` | Elevation shifted by `sea_level` — what biome selection sees                                     |
| `noise_moisture.png`            | `[-1, 1]` | Moisture noise (local variation added to the climate model)                                      |
| `noise_volcanic_raw.png`        | `[-1, 1]` | Raw volcanic-zone FBM before threshold                                                           |
| `noise_volcanic_zone.png`       | `[0, 1]`  | Processed volcanic zone (threshold applied)                                                      |
| `noise_temperature_anomaly.png` | `[-1, 1]` | Large-scale temperature anomaly noise (features of ~3 000 km)                                    |
| `noise_variability_t.png`       | `[-1, 1]` | Regional variability noise added to temperature                                                  |
| `noise_variability_m.png`       | `[-1, 1]` | Regional variability noise added to moisture                                                     |
| `noise_highland_jitter.png`     | `[-1, 1]` | Noise shifting the mountain line (× 0.5 gives `highland_offset`)                                 |
| `erosion.png`                   | `[-1, 1]` | Height change from erosion, ×10 (blue = carved, red = deposited)                                 |
| `elevation.png`                 | `[-1, 1]` | Final elevation, after erosion                                                                   |
| `temperature.png`               | `[0, 1]`  | Temperature before the planet-type offset (see [Temperature](#temperature--axial-tilt))          |
| `coast_distance.png`            | `[0, 1]`  | Distance to the nearest open water, `1.0` = 2 000 km or more                                     |
| `precipitation.png`             | `[-1, 1]` | Rain brought by the winds, smoothed (`0` = neutral rain rate)                                    |
| `moisture.png`                  | `[-1, 1]` | Final moisture before the planet-type offset (see [Moisture](#moisture--winds-and-rain-shadows)) |
| `flow.png`                      | `[0, 1]`  | River flow: `log10(1 + drainage km²) / 7` (see [Rivers and lakes](#rivers-and-lakes))            |

#### Geographic reference lines

Both `world.png` and `world.svg` overlay five dotted latitude lines for orientation:

| Line                | Latitude      | Colour |
| ------------------- | ------------- | ------ |
| Equator             | 0°            | Red    |
| Tropic of Cancer    | +tilt         | Amber  |
| Tropic of Capricorn | −tilt         | Amber  |
| Arctic Circle       | +(90° − tilt) | Cyan   |
| Antarctic Circle    | −(90° − tilt) | Cyan   |

`tilt` is the planet's axial tilt (`--axial-tilt`): 23.44° on Earth gives the familiar ±23.4° and ±66.6°.

The row position for each line is derived from the equirectangular projection used by the map: `row = height × (0.5 − latitude_deg / 180)`, so north is at the top of the image.

## Calculations

### Temperature — axial tilt

The sea-level temperature of each latitude comes from the annual-mean sunlight it receives, using the classic two-term approximation (North, 1975):

```text
x  = sin(latitude)
P2(x) = (3x² − 1) / 2
S(x)  = 1 + s2 · P2(x)        with  s2 = −5/8 · P2(cos(tilt))
```

`S` is then rescaled so that an Earth-like tilt (23.44°) gives `0.0` at the poles and `1.0` at the equator, and clamped to `[0, 1]`. Land then cools by `0.3` per unit of elevation above the waterline; the sea surface does not.

A large-scale anomaly is added on top: `0.40 × fbm` on land and `0.20 × fbm` on the sea (features of about 3 000 km, standard deviation about 0.08 on land). It stands for ocean currents, monsoons and continentality without modelling them, so climate zones shift by up to ~15° of latitude between continents instead of following parallels. It is zero-mean: the share of cold and hot climates does not change.

| Axial tilt | Effect                                                   |
| ---------- | -------------------------------------------------------- |
| 0°         | Strongest equator-to-pole contrast, large ice caps       |
| 23.44°     | Earth baseline                                           |
| ~54°       | Nearly uniform temperature across latitudes              |
| 70°+       | Poles receive more sunlight over a year than the equator |

The `[climate]` table of the planet preset then shifts the whole temperature field (for example `−0.55` on frozen worlds).

### Moisture — winds and rain shadows

Moisture comes from winds carrying ocean evaporation over land:

- **Wind belts.** The zonal wind at each latitude is `u = −cos(4 · latitude)`: easterly trade winds near the equator, westerlies at mid-latitudes, polar easterlies. Each row is simulated with the wind blowing east and blowing west, and both results are blended by how much the wind blows each way, so moisture stays continuous across the belts.
- **Transport.** Along a row, air over the sea picks up humidity towards what it can hold (warm air holds more). Over land it rains out steadily, much more where it climbs a slope, and immediately where it cools below what it holds. 60 % of the rain over land evaporates again (soil and plants), so continental interiors are not starved. Mountains therefore get wet windward slopes and dry rain shadows behind them.
- **North-south supply.** Rows with little ocean of their own still receive background humidity, set by the ocean fraction within ±20° of latitude and of the whole planet.
- **Circulation bands.** A `cos(6 · latitude)` term adds the wet equator and 60° belts and the dry 30° desert belts and poles.

The rain rate is smoothed over 100 km, mapped to `[-1, 1]` and combined as `0.75 × rain + 0.25 × band + 0.50 × noise`, then clamped. The `[climate]` table of the planet preset then shifts the whole field (for example `−0.55` on volcanic worlds).

### Regional variability

Before biome selection, both climate fields get a regional variability term standing for local differences of soil, aspect and circulation: `0.25 × fbm` on temperature (about 2 °C of standard deviation) and an independent `0.50 × fbm` on moisture (about 0.09), with features from about 1 000 km down to 60 km. Biome borders then become irregular and interleaved instead of following isotherms and isohyets. The term is part of the exported `temperature` and `moisture` layers, so they still explain every biome.

### Rivers and lakes

1. **Depression filling.** Starting from the sea, the terrain is flooded upwards (priority-flood, Barnes et al. 2014) so every land pixel gets a path down to the sea; pits are filled to their spill level. Water crosses the filled flats along a tiny slope whose cost per step varies with a noise, so paths across flats wander instead of running straight to the spill point; real slopes keep their height, so divides and lakes do not move.
2. **Routing.** Each land pixel drains to its steepest lower neighbour on the filled surface.
3. **Accumulation.** Every pixel adds its runoff — its area in km² × `((moisture + 1) / 2)²` — and passes the total downstream. The flow of a pixel is the wet-equivalent area draining through it.
4. **Lakes** are filled pits deeper than `0.004` that receive at least 40 000 km² of flow; **rivers** are pixels carrying at least 120 000 km². Under the polar temperature threshold, rivers stay ice and lakes freeze into ice cap.
5. **Vector rivers.** River pixels are chained along their receivers into polylines (the main stem follows the largest tributary), smoothed (Chaikin), resampled and displaced sideways by noise into meanders whose wavelength (`7 ×` width, 6–40 px) and amplitude (`2.5 ×` width, 1–3 px) grow with the river, plus a slower wander. Guards keep them sound: no offset climbing the valley side, no bend tighter than half its radius, a gap to other rivers, loops cut, tributaries snapped onto their displaced parent, and every river ended at the first other river it crosses, which it then joins (downstream of its own tributaries' mouths). Lengths are in reference pixels, 1/1920 of the circumference, so the shapes do not depend on the map size.
6. **Drawing.** Rivers are drawn anti-aliased over the map. Their nominal width is `0.9 × √(flow / 120 000 km²)` reference pixels (capped at 4, tapered at the head); on a map `W` pixels wide they are drawn `0.5 × nominal × (W / 1920)^(2/3)` pixels wide, never thinner than their real width (`0.002 × √flow` km). The largest rivers are 2 px wide at 1920 × 1080, 3.3 px at 4096 and 5.3 px at 8192, about 8–14× their real width (instead of 21–27× before), so they read at map scale, and each doubling of resolution narrows it by 1.26× in km. Rivers thinner than a pixel are drawn with exact (box-filtered) coverage, so small ones fade instead of keeping a solid one-pixel line. `world.svg` draws them as stroked paths. The biome layer marks a pixel `River` where the drawn river covers at least half of it, so `biomes.png` matches the picture; frozen rivers (below the polar temperature) are not drawn.

Thresholds are in km², so the river network does not depend on the map resolution. On non-terran planets, rivers and lakes are remapped like seas: lava and magma on volcanic worlds, ice on frozen worlds, acid on caustic worlds, dry rock on barren worlds.

### Detail tiles — deep zoom

The generated layers stop at the base resolution (about 20 km per pixel at 1920 pixels on an Earth-sized planet), and the generation noise has no feature smaller than that: a larger map adds no detail. The `mapgen::detail` module synthesises detail on demand, consistently with the base world, as 256 × 256 tiles of an equirectangular quadtree (level `z` has `2^(z+1)` × `2^z` tiles; level 8 is about 300 m per pixel):

- **Elevation**: the base elevation, bicubic-interpolated, plus fractal octaves of features smaller than two base pixels and larger than two tile pixels. The first octave is `0.004` elevation units on plains, up to three times more on high ground, halving with each octave, so base coastlines move by less than a base pixel and each tile is the low-pass of its children.
- **Climate**: temperature, moisture and the other biome inputs are read at a position warped by about one base pixel, so biome borders become irregular; temperature follows the detailed elevation through the lapse rate.
- **Picture**: the generation's own biome rules and preset remap, soft borders, hillshading, and the vector rivers redrawn at the tile's scale (Catmull-Rom smoothed) with the base map's width law at the level's resolution (`W` = columns × 256): no jump in width when tiles replace the base map, 1.26× thinner in km at each level, down to their real width (`0.002 × √flow` km).

A tile takes about 4 ms on 10 cores. Coast-related biomes (beaches, wetlands) still use the base coast distance, so their 40 km band is smoother than the detailed coastline at the deepest levels.

### Elevation — continents, coasts and mountains

Elevation before erosion is `continent + mountain × mountain_weight × mountain_blend`.

- **Continents.** A 5-octave FBM on the unit sphere. Its 2 coarse octaves shape continents and basins; its 3 fine ones add hills. On low ground inland (more than about 0.03–0.2 above the waterline) only 30 % of the fine octaves remain, so plains and deserts are smooth; coasts keep them all, and so does high ground, with the mountain weight.
- **Coastlines.** Near the waterline the FBM is sampled at a position displaced sideways by every octave finer than its own, down to about two map pixels, each by half its wavelength and 0.8× less at each finer octave. A low "plate" step is added there too: a Voronoi cell noise (±0.06, borders softened over about 1/20 of a cell) that gives coasts straight stretches and angular corners. Both fade out away from the coast, where they would only wrinkle the relief. Coasts are fractal instead of smooth contours. The finest outline octaves follow the map resolution, so a larger map adds detail to the same continents.
- **Mountains.** A ridged noise on a domain-warped position, with two finer ridged octaves concentrated on its ridges (35 % of its height), weighted from continent height 0.2 upwards.

### Hydraulic erosion

Rain droplets are dropped at random on land. Each one rolls downhill with a little inertia, picks up sediment while it speeds up on steep ground, and drops it when it slows down, climbs, evaporates, or reaches the sea (Beyer, 2015). Thousands of droplets carve valleys into the ridges and leave fans and deltas at their feet.

The simulation runs on a grid coarser than the map: one cell per 4 × 4 pixels, between 512 × 256 and 2048 × 1024 cells (smaller maps use their own size). A 1920 × 1080 map erodes on 512 × 270 cells in a fraction of a second; 8192 × 4096 uses 2048 × 1024 cells, which takes a few seconds but keeps the valleys fine. `--erosion 1` drops one droplet per grid cell. The height change is spread back onto the full-resolution map with a cubic B-spline, smooth across cell borders, and never moves a pixel across the waterline: erosion does not dig land below the sea, and sediment dumped at the coast does not raise new land, so the coastlines stay those of the noise. Erosion fades out between 60° and 75° of latitude: droplets move in pixel steps and the projection stretches pixels east-west towards the poles, which would otherwise carve artificial north-south grooves.

The two parameters derived from `--circumference` are computed as follows.

### Noise scale — continent & feature size

The world is projected onto a unit sphere. The Perlin noise samplers are called with coordinates on that sphere, so the spatial frequency of features is independent of image resolution.

For an Earth-sized planet (`C⊕ = 40 075 km`) those frequencies are the baseline. For any other circumference:

```text
noise_scale = C⊕ / circumference_km
```

This multiplies every noise coordinate, effectively zooming in or out on the sphere surface:

| Circumference | noise_scale | Visual effect                             |
| ------------- | ----------- | ----------------------------------------- |
| 20 000 km     | ≈ 2.0       | Many small continents and narrow seas     |
| 40 075 km     | 1.0         | Earth baseline                            |
| 80 000 km     | ≈ 0.5       | Few, sweeping continents with vast oceans |

### Gravity modifier — landscape relief

Assuming a **constant-density rocky planet**, mass scales with volume and surface gravity scales with radius:

```text
M ∝ r³     g = GM/r²  ∝  r  ∝  C
```

So:

```text
gravity_modifier = circumference_km / C⊕
```

Higher gravity means tectonic plates are pressed flatter and mountains cannot stand as tall. This is encoded in the **mountain blend coefficient**:

```text
mountain_blend = 0.35 / √gravity_modifier
```

The square-root softens the curve so extreme planets remain visually interesting:

| Circumference | gravity  | mountain_blend | Relief                  |
| ------------- | -------- | -------------- | ----------------------- |
| 20 000 km     | ≈ 0.50 g | ≈ 0.49         | Dramatic, jagged peaks  |
| 40 075 km     | 1.00 g   | 0.35           | Earth baseline          |
| 80 000 km     | ≈ 2.00 g | ≈ 0.25         | Flat, rounded landscape |

Both values are written into `raw_data/world.json` as `circumference_km` and `gravity_modifier` for downstream use.
