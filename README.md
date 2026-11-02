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
Reproduce with: --planet frozen --seed 2590618090 --sea-level 0.1213 --volcanic 0.6107 --circumference 51823.4 --axial-tilt 18.72 --erosion 1 --width 1920 --height 1080
```

### Options

| Flag                    | Values                                                                                                                                                 | Default                      |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- |
| `--planet <preset>`     | built-in preset name (`terran` · `volcanic` · `frozen` · `caustic` · `barren`) or path to a `.toml` preset file, see [Planet presets](#planet-presets) | random built-in              |
| `--list-planets`        | list the built-in presets and exit                                                                                                                     | —                            |
| `--seed <u32>`          | any 32-bit unsigned integer                                                                                                                            | random                       |
| `--sea-level <f32>`     | float in `[-1.0, 1.0]`                                                                                                                                 | random in `[0.10, 0.40)`     |
| `--land-fraction <f32>` | share of the surface that is land, `[0.0, 1.0]`; sets the sea level to match, so it cannot be combined with `--sea-level`                              | —                            |
| `--volcanic <f32>`      | float in `[0.0, 1.0]`                                                                                                                                  | random in `[0.0, 1.0)`       |
| `--circumference <f32>` | planet equatorial circumference in km (`>= 1`)                                                                                                         | random in `[20 000, 80 000)` |
| `--axial-tilt <deg>`    | axial tilt in degrees, `[0, 90]`                                                                                                                       | random in `[5, 40)`          |
| `--erosion <f32>`       | hydraulic erosion strength, `[0, 5]`; `0` disables it                                                                                                  | `1.0`                        |
| `--width <px>`          | map width, `2`–`16384`                                                                                                                                 | `1920`                       |
| `--height <px>`         | map height, `2`–`16384`                                                                                                                                | `1080`                       |
| `--out-dir <path>`      | directory receiving the generated worlds                                                                                                               | `worlds`                     |
| `--no-raw-data`         | skip the `raw_data/` export (~7 MB at 1920 × 1080)                                                                                                     | off                          |
| `--no-noise-maps`       | skip the `noise_maps/` diagnostic images, and the memory they need (they roughly double the peak)                                                      | off                          |

Invalid or unknown options are rejected with an error. Run `target/release/map-generator --help` for the full reference.

**seed** — drives every noise layer. The same seed with the same other options always produces the same world. Each noise layer hashes the seed with its own stream number, so the layers of a world are independent and neighbouring seeds share nothing. A seed noted before that change (November 2026) gives a different world now; a world already generated keeps its files.

**sea-level** — shifts the waterline: positive values raise it (more ocean), negative values lower it (more land). When it is not set (nor by the preset), it is drawn in `[0.10, 0.40)`, which gives about 10–70 % of land, mostly 20–55 % (Earth: 29 %). Mountains and snow are measured above the waterline, so a low sea level also turns much of the land into highland: at `0.0` about a third of the land is mountain, at `-0.3` almost three quarters.

**land-fraction** — sets the share of land instead of the sea level: `--land-fraction 0.29` gives a world with 29 % of its surface (by area, not pixels) above the sea, whatever the relief of the seed, where a fixed sea level gives anywhere from 20 to 50 %. The generator samples the relief on a coarse grid (512 × 256, about 0.1 s) and picks the sea level that leaves that share above it; on test worlds the result is within 0.1 point of the target. The chosen sea level is printed and stored in `world.json` like any other, so `--sea-level` reproduces the world. In the web form, the sea level fieldset switches to it with its "Land share (%)" option. A preset can set `land_fraction` in `[defaults]` instead of `sea_level`, not both.

**volcanic** — controls how much of the planet is volcanic: the share of volcanic provinces, and so the number of volcanoes, not their size. `0.0` = no volcanoes, `1.0` = many volcanic provinces.

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
| `--columns <n>`      | hex columns around the planet, `4`–`1024`; rows follow from the map height | `96`     |
| `--river-flow <km²>` | land hexes whose largest river drains at least this area get a river flag  | `500000` |

It writes `<world>/hex/`:

- `hex.png`: the map painted with one colour per hex, outlined, with a blue dot on river hexes.
- `hex.json`: the grid layout and one entry per hex (row-major, `index = row × columns + col`): majority `biome` id (rivers never win a hex, whatever biome the preset remaps them to, such as `AcidRiver` on caustic worlds or `LavaField` on volcanic ones, where the river polylines tell them apart; they set `river` instead), mean `elevation`, `moisture` and `temperature`, `max_flow_km2` and `river`. The biome table is copied from `world.json`.

The same resampling runs in the web app (hex overlay of the viewer), through the `mapgen::hex` library module: the server resamples the world it keeps for the detail tiles, one grid at a time, and writes no `hex/` files.

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

A browser app to explore the generated worlds as a 3D globe or a flat map, switch between biome, elevation, temperature, moisture, river, volcanic, distance-to-sea and floodplain layers, raise the relief, light the planet with a seasonal sun, overlay the hex grid, inspect any point, its river and its hex, measure distances, compare two worlds side by side, download a world's files, and queue new worlds from a form. See [web/README.md](web/README.md) for the full feature list and the keyboard shortcuts. It is a personal tool served locally behind the machine-wide Traefik, like the other `*.localhost` apps.

```bash
mise run up      # build and start it on http://planets.localhost
mise run logs    # follow its logs
mise run down    # stop it
```

The container reads and writes `worlds/` of this checkout (worlds generated with `mise run gen` show up too, as long as they have `raw_data/`) and reads the preset files of `presets/`: any `.toml` preset added there is offered in the form without a rebuild, once the server restarts (`docker restart map-generator`): it reads the presets at startup. Generated files keep the host owner.

| Piece     | Technology                               | Location         |
| --------- | ---------------------------------------- | ---------------- |
| Server    | Rust, axum; calls the `mapgen` library   | `crates/server/` |
| Front-end | Svelte 5, strict TypeScript, three.js    | `web/`           |
| Container | Multi-stage `Dockerfile`, `compose.yaml` | repository root  |

Generations run one at a time (each one already uses every core): later ones wait in a queue. Their progress streams to the page (`GET /api/jobs/{id}/events`); `GET /api/jobs` lists the recent jobs, so a reloaded page picks up the ones still running, and `DELETE /api/jobs/{id}` cancels one: a queued job never starts, a running one stops when its next stage begins (or, during the export, once the files are written) and leaves nothing on disk: a world is exported into a hidden `worlds/.tmp-<id>/` directory renamed into place once complete, removed if the job is cancelled or fails, and swept at startup if the server was killed mid-export. A world cannot be deleted while its generation is queued or running (409). Zooming in loads [detail tiles](#detail-tiles--deep-zoom) rendered on demand by the server (`GET /api/worlds/{id}/tiles/{z}/{x}/{y}?rev=`), cached by the browser for good since the URL names the world revision. The server keeps the most recently used worlds in memory within `MAPGEN_TILE_CACHE_MB` (default 1024 MiB; a world takes about 23 bytes per pixel, 0.75 GB at 8192 × 4096, and the last one used is always kept). While a generation runs, it frees them and answers tile requests with 503 and `Retry-After` (the viewer retries), so they do not add to the generation peak; tiles render at most four at a time, and a request the viewer abandons before its turn is dropped. The server refuses to overwrite an existing world: delete it first or pick another seed. Maps are limited to `MAPGEN_MAX_PIXELS` pixels and 8192 pixels per side. The server defaults to 4096 × 2048; `compose.yaml` raises it to 8192 × 4096 for the 6 GB Colima VM of the workstation setup. A generation peaks at about 0.2 GB at 1920 × 1080, 0.8 GB at 4096 × 2048 and 2.0 GB at 8192 × 4096 (about 2 s, 8 s and 40 s on 6 cores): keep the limit under the memory the container can use, or the server is killed mid-job. With Colima's default 2 GB VM, lower it (`MAPGEN_MAX_PIXELS: "8388608"`) or give the VM more memory (`colima stop && colima start --cpu 6 --memory 6`). The web app skips the diagnostic maps, which would add about 90 bytes per pixel.

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

| Preset     | Description                                       | Exclusive biomes                                                                                |
| ---------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `terran`   | Earth-like — full biome spectrum                  | —                                                                                               |
| `volcanic` | Fire world — extreme heat, near-zero moisture     | `MagmaSea`, `ScorchedWaste`                                                                     |
| `frozen`   | Ice world — perpetually frozen                    | `FrozenOcean`, `FrozenLake`, `GlacialPlain`                                                     |
| `caustic`  | Acid world — corrosive atmosphere, toxic wetlands | `AcidSea`, `DeepAcidSea`, `CausticLake`, `AcidRiver`, `AcidShore`, `ToxicSwamp`, `AcidFlatland` |
| `barren`   | Dead rock — arid and lifeless, no water           | `RockyWaste`, `DustPlain`                                                                       |

To create a new planet type, write a preset file and pass its path. `presets/examples/` holds one per rocky planet of the Solar System, with its real size and tilt:

| Example        | Look                                                                                                      |
| -------------- | --------------------------------------------------------------------------------------------------------- |
| `mercury-like` | Small and airless, no erosion: grey cratered highlands (`RockyWaste`), dark smooth basins (`AshLand`)     |
| `venus-like`   | Earth-sized furnace, no erosion: basalt plains (`ScorchedWaste`) around highland plateaus with lava flows |
| `earth-like`   | `terran` with Earth's size, tilt and share of land (29 %)                                                 |
| `mars-like`    | Small cold desert: dust basins where the seas were, polar ice caps                                        |

The web app lists them with the built-in types.

```bash
mise run gen -- --planet presets/examples/mars-like.toml
```

```toml
name = "mars-like"                 # lowercase letters, digits and dashes; used in the output directory name
description = "Small cold desert world with polar caps"

[defaults]                         # a fixed value or a [min, max] range drawn at random
sea_level = [0.10, 0.30]           # sea_level or land_fraction, volcanic, circumference, axial_tilt, erosion
circumference = 21300

[climate]                          # added to every pixel before biome selection
temperature = -0.08                # [0, 1] scale
moisture = -0.7                    # [-1, 1] scale
volcanic = 0.0                     # [0, 1] scale

[rules]                            # override any biome selection threshold
polar = 0.04
highland_snow_temperature = 0.04

[remap]                            # target = [biomes replaced by it]
DustPlain = ["Ocean", "DeepOcean", "Lake", "River", "Plain", "Desert"]
IceCap = ["SeaIce"]
RockyWaste = ["Mountain"]
```

Every table is optional.

To make a dry world, remap its oceans rather than lowering `sea_level`: biome elevations are measured from the sea level, so a world with a sea level of −0.5 has nearly all its land above the snow line (0.3 above the sea) and turns white. The `mars-like` example keeps the usual sea level and turns its seas into dust basins.

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
│   ├── floodplain.png ← 16-bit greyscale layer
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

`world.png` draws biome borders as ecotones. Each land pixel takes the mean colour of the biomes its inputs select when nudged by 128 small offsets (a 4D Gaussian, σ 0.02 on temperature, 0.05 on moisture, 0.003 on the highland line and 0.004 on the volcanic zone): far from any threshold the colour is flat (8 probes tell, so most pixels skip the sampling), near one the colours mix. The width of a transition is set in input units, not pixels, so it suits any zoom level; mountain, snow and volcanic borders soften too, volcanoes less than climate borders. The same offsets serve every pixel, so transitions show neither grain nor terraces. Coastlines, shores, rivers and lakes keep their exact inputs and stay crisp. This is rendering only: `raw_data/biomes.png` and `world.svg` keep crisp, flat classes.

Water lightens towards its shore, up to 25 % towards white over the last `0.05` elevation units of depth, so seas and lakes show the rim of their basin whatever their colour; a lake's depth is measured from its own level (its highest pixel), not from the sea. Frozen seas and lakes lighten too, as whiter fast ice along the shore, and magma seas darken into a cooled crust; rivers stay flat. Contour lines are drawn under water only, as bathymetry: on land the hillshade shows the relief, and contours drew rings that read as altitude bands. The hillshade follows a soft curve (`1 + 0.4 × tanh((lit − 1) / 0.4)`, relative to flat ground) rather than a hard clamp, so steep slopes stay distinct; detail tiles use the same curve.

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
| `floodplain.png`      | 16-bit grey      | Floodplain weight in `[0, 1]`: 1 on the flat floor of a river valley, 0 away from rivers                                                 |
| `biomes.png`          | 8-bit grey       | Biome id of each pixel                                                                                                                   |

A 16-bit pixel `p` decodes to `min + p / 65535 × (max − min)`, with `min` and `max` given per layer in `world.json`. For `flow` (encoding `gray16-log10`) this gives `log10(1 + km²)`: the area is `10^value − 1`. The precision is about `3 × 10⁻⁵`.

`world.json` contains the generation parameters, the `layers` description and the `biomes` table mapping each id to its key, display name, colour and whether it is a water surface:

```json
{
  "format_version": 3,
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

The example is abridged. Biome ids are stable: new biomes only get appended. `format_version` changes when the layout of `raw_data/` changes: version 2 added `volcanic_zone`, `coast_distance` and `highland_offset`, so that every input of biome selection is exported, and `rivers.json`; version 3 added `floodplain`. Older worlds still load: missing layers are recomputed, or zero for `floodplain`.

`rivers.json` holds the drawn river polylines (version 2 and later; its file name is in `world.json` under `rivers`):

```json
{
  "version": 3,
  "width_law": { "w0_ref_px": 0.9, "q0_km2": 120000, "exponent": 0.5, "max_ref_px": 4, "reference_width_px": 1920, "ink": 0.5, "raster_exponent": 0.6666667, "real_km_per_sqrt_km2": 0.002 },
  "rivers": [
    { "id": 0, "parent": null, "mouth": "sea", "source": null, "estuary": false, "points": [[129.656, -62.275, 158147], [129.802, -62.306, 158175]] }
  ]
}
```

Each river lists `[longitude°, latitude°, flow km²]` from head to mouth; longitudes are continuous along a river, so they may leave `[0, 360)` where it crosses the seam. `mouth` is `sea`, `lake` or `river` (then `parent` is the river it joins). `source` is the river a delta distributary branches from (its first point lies on it), and `estuary` is `true` when the river widens into an estuary towards its mouth (version 3). The nominal width is `w0_ref_px × (flow / q0_km2)^exponent`, capped at `max_ref_px`, in reference pixels (1/`reference_width_px` of the circumference); on a raster `W` pixels around the planet a river is drawn `ink × nominal × (W / reference_width_px)^raster_exponent` pixels wide, and never thinner than `real_km_per_sqrt_km2 × √flow` km. Version 1 files (before `ink`) drew the nominal width. About 0.5 MB per world (250 KB gzipped).

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
| `noise_volcanic_zone.png`       | `[0, 1]`  | Volcanic zone: threshold applied, peaking on the volcanic cones                                  |
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
2. **Routing.** Each land pixel drains to a lower neighbour on the filled surface, scored by its slope (the steepest scores 1) plus the descent of a large-scale routing noise (wavelength 1 200 km, same weight). Always downhill, so the filled surface stays valid, but on even slopes paths converge into the noise's valleys and join: river networks branch like trees instead of running parallel down mountain flanks. The wavelength is in km, so networks do not depend on the map size.
3. **Accumulation.** Every pixel adds its runoff — its area in km² × `((moisture + 1) / 2)²` — and passes the total downstream. The flow of a pixel is the wet-equivalent area draining through it.
4. **Lakes** are filled pits deeper than `0.004` that receive at least 40 000 km² of flow; the depth is measured from the spill level, without the slope added across flats, which grows with the number of pixels crossed and turned wide flats of high-resolution maps into lakes; **rivers** are pixels carrying at least 120 000 km². Under the polar temperature threshold, rivers stay ice and lakes freeze into ice cap.
5. **Vector rivers.** River pixels are chained along their receivers into polylines (the main stem follows the largest tributary), smoothed (a moving average over 5 points removes the pixel staircase, which small maps showed as zigzags, then Chaikin corner cutting), resampled and displaced sideways by noise into meanders whose wavelength (`7 ×` width, 6–40 px) and amplitude (`2.5 ×` width, 1–3 px) grow with the river, plus a slower wander. Guards keep them sound: no offset climbing the valley side, no bend tighter than half its radius, a gap to other rivers, loops cut, tributaries snapped onto their displaced parent, and every river ended at the first other river it crosses, which it then joins (downstream of its own tributaries' mouths). Lengths are in reference pixels, 1/1920 of the circumference, so the shapes do not depend on the map size.
6. **Drawing.** Rivers are drawn anti-aliased over the map. Their nominal width is `0.9 × √(flow / 120 000 km²)` reference pixels (capped at 4, tapered at the head); on a map `W` pixels wide they are drawn `0.5 × nominal × (W / 1920)^(2/3)` pixels wide, never thinner than their real width (`0.002 × √flow` km). The largest rivers are 2 px wide at 1920 × 1080, 3.3 px at 4096 and 5.3 px at 8192, about 8–14× their real width (instead of 21–27× before), so they read at map scale, and each doubling of resolution narrows it by 1.26× in km. Rivers thinner than a pixel are drawn with exact (box-filtered) coverage, so small ones fade instead of keeping a solid one-pixel line. `world.svg` draws them as stroked paths. The biome layer marks a pixel `River` where the drawn river covers at least half of it, so `biomes.png` matches the picture; frozen rivers (below the polar temperature) are not drawn, nor rivers across a lava field or a volcano: lava overrides water, so lakes there become lava too, and rivers start at the edge of the flow.

7. **Valleys.** The terrain around each river is lowered into a valley (`mapgen::carve`). Its axis is the river averaged over one meander wavelength, so walls follow the river's course, not each bend. Its bed is the lowest of the filled surface under the river and of the ground across the floodplain, never rising downstream, incised by `0.0006 × (flow / 120 000 km²)^0.25`, smoothed along the river, and kept `0.002` above the sea (or above the lake the river ends in). The cross-section is a flat floodplain as wide as the meander belt and at least `14 km × √(flow / 120 000 km²)` (about 80 km for the largest rivers), then walls rising `0.00005` per km, blended back into the terrain between 60 % and 100 % of the valley half-width (4 × the floodplain half-width + 60 km), and faded in from the river's head. Each pixel takes the profile of its nearest stretch of valley, blended over 5 km. The new elevation is the minimum of the terrain and the valleys, so carving never raises ground, never creates water and keeps the divides. Floodplains get `+0.15` moisture: rivers draw green strips across dry land. Their weight is exported as `floodplain.png`, and detail tiles keep floodplains flat (80 % less height detail). Carving takes about 2 s at 8192 × 4096.

8. **Deltas and estuaries.** Rivers reaching the sea with at least 800 000 km² of flow get an outlet. Where the land is low (under `0.012` above the waterline) at the apex, `120 km × √(flow / 1 M km²)` upstream of the mouth (60–300 km), 80 % of them get a **delta**: one distributary (two above 3 M km²) leaves the river at the apex 40° off the seaward direction, bends back towards the sea with a slow wander, and takes 20 % of the flow; a branch that does not reach the sea or would cross another river is dropped. The delta plain, within `0.35 ×` the delta length of its channels, is flattened towards `0.002` above the waterline and gets `+0.3` moisture (wetlands near the coast); it never moves the coastline. Elsewhere 60 % of them get an **estuary**: the drawn river widens to 3× towards its mouth over `3 × √(flow / 1 M km²)` reference pixels (about 60 km), on the map, in `world.svg` and in the detail tiles. The choice hashes the seed and the river, so it is deterministic.

Thresholds are in km², so the river network does not depend on the map resolution. Planets with nothing liquid to flow (whose preset remaps `River` to dry ground, like `barren`; lava rivers of `volcanic` worlds still flow) have no rivers at all: no drawn lines, valleys or deltas, and an empty `rivers.json`; only the drainage flow is exported. On non-terran planets, rivers and lakes are remapped like seas: lava and magma on volcanic worlds, ice on frozen worlds, acid on caustic worlds, dry rock on barren worlds.

### Detail tiles — deep zoom

The generated layers stop at the base resolution (about 20 km per pixel at 1920 pixels on an Earth-sized planet), and the generation noise has no feature smaller than that: a larger map adds no detail. The `mapgen::detail` module synthesises detail on demand, consistently with the base world, as 256 × 256 tiles of an equirectangular quadtree (level `z` has `2^(z+1)` × `2^z` tiles; level 8 is about 300 m per pixel):

- **Elevation**: the base elevation, bicubic-interpolated, plus fractal octaves of features smaller than two base pixels and larger than six tile pixels (finer ones only read as grain), so each tile is the low-pass of its children. The first octave is `0.0015` elevation units on plains and up to four times more on high ground, scaled by the surface of the base biome (blended across borders): 10 % on snow and ice, 5 % on lava (a fluid surface), 40 % on sand and ash, all of it on rock and vegetation. Each octave is 0.4 of the previous one on high ground and 0.3 on plains, so slopes get gentler at every zoom level instead of turning into grain. Lava's hillshade is also kept at 40 %, on the base map and in the tiles. Height detail fades out within `0.006` of the waterline, so it never turns the shore into islets and ponds. Near the coast (within `0.01`–`0.04` of the waterline) the base elevation is instead read at a position displaced sideways by every detail octave (0.35 of its wavelength, 0.8× less at each finer octave): coastlines get capes and bays at every zoom level and stay one connected line.
- **Climate**: temperature, moisture and the other biome inputs are read at a position warped by about one base pixel, so biome borders become irregular; temperature follows the detailed elevation through the lapse rate.
- **Channels**: each river is cut into the detailed elevation along the same curve as its drawing, `0.0005 × (flow / 120 000 km²)^0.3` elevation units deep (exaggerated like the rest of the relief), with banks rising from a quarter of the drawn width to the full width from its middle, so the hillshade shows them; never below the sea.
- **Picture**: the generation's own biome rules and preset remap, soft borders, hillshading, and the vector rivers redrawn at the tile's scale (Catmull-Rom smoothed) with the base map's width law at the level's resolution (`W` = columns × 256): no jump in width when tiles replace the base map, 1.26× thinner in km at each level, down to their real width (`0.002 × √flow` km).

A tile takes about 4 ms on 10 cores. A world prepared for tiles keeps its layers as stored in `raw_data/` (16 bits per value, decoded as they are sampled), a byte of lake and river flags and the water depth per pixel: about 23 bytes per base pixel (0.2 GB at 4096 × 2048, 0.75 GB at 8192 × 4096), loaded in about 0.1 s at 4096 × 2048 since the layer files are decoded in parallel. Coast-related biomes (beaches, wetlands) still use the base coast distance, so their 40 km band is smoother than the detailed coastline at the deepest levels.

### Elevation — continents, coasts and mountains

Elevation before erosion is `continent + mountain × mountain_weight × mountain_blend`.

- **Continents.** A 5-octave FBM on the unit sphere. Its 2 coarse octaves shape continents and basins; its 3 fine ones add hills. Away from the coast (more than about 0.03–0.2 above or below the waterline) only 30 % of the fine octaves remain, so plains, deserts and the abyssal plains of the sea floor are smooth (visible where seas have dried up); coasts keep them all, and so does high ground, with the mountain weight.
- **Coastlines.** Near the waterline the FBM is sampled at a position displaced sideways by every octave finer than its own, down to about two map pixels, each by half its wavelength and 0.8× less at each finer octave. A low "plate" step is added there too: a Voronoi cell noise (±0.06, borders softened over about 1/20 of a cell) that gives coasts straight stretches and angular corners. Both fade out between 0.02 and 0.08 from the waterline, where they would only wrinkle the relief. Coasts are fractal instead of smooth contours. The finest outline octaves follow the map resolution, so a larger map adds detail to the same continents.
- **Mountains.** A ridged noise on a domain-warped position, with two finer ridged octaves concentrated on its ridges (35 % of its height), weighted from continent height 0.2 upwards.
- **Volcanoes.** The volcanic noise, above the threshold set by `--volcanic`, outlines broad volcanic provinces. Volcanoes are cones within them: at most one per 900 km cell (half the cells), 50–120 km in radius whatever the planet's size, rising up to `0.25` above the terrain (volcanic islands at sea). The volcanic zone peaks on each cone and falls to 12 % of the province away from it, below the ash threshold, so a province is ordinary land dotted with volcanoes, each with its vent (about 1.2 radii), lava flows (1.8, lobed by a noise) and an ash halo (2.6). Presets that shift the volcanic zone (`volcanic` worlds, +0.5) still cover whole provinces with lava.

### Hydraulic erosion

Rain droplets are dropped at random on land. Each one rolls downhill with a little inertia, picks up sediment while it speeds up on steep ground, and drops it when it slows down or climbs (Beyer, 2015); what it still carries when it reaches the sea is carried offshore. Thousands of droplets carve valleys into the mountains and leave fans at their feet.

The simulation runs on a grid coarser than the map: one cell per 4 × 4 pixels, between 512 × 256 and 2048 × 1024 cells (smaller maps use their own size). A 1920 × 1080 map erodes on 512 × 270 cells in a fraction of a second; 8192 × 4096 uses 2048 × 1024 cells, which takes about 3 s on 6 cores but keeps the valleys fine. `--erosion 1` drops one droplet per grid cell. Droplets run in parallel: the grid is cut into tiles at least 104 cells wide (twice the farthest a droplet reaches), run in four checkerboard phases so that tiles running together never touch the same cells; each tile gets droplets in proportion to its land and its own random stream, so a world does not depend on the number of cores.

- **Real slopes.** Beyer's constants assume heights of about one per cell; this terrain is about 100 times flatter. Droplets run on heights exaggerated 10× (on a 1024-cell grid, scaled with the cell size), so their capacity follows the slope instead of its minimum everywhere.
- **Plains stay smooth.** Droplets only carve where the slope (exaggerated, per cell) exceeds 0.04, fully from 0.1. On flatter cells the height change is blurred (three box passes of 2 cells): valleys continue as wide, shallow swales, so they still drain and leave no basin at the foot of the mountains, and fans no longer roughen plains and deserts. Fine-scale roughness on deserts is about 5× lower than with uniform erosion, and mountains are about 3.5× rougher than plains.
- **Coasts keep their profile.** Erosion fades out within 0.05 of the waterline, and the change spread back onto the full-resolution map (cubic B-spline, smooth across cell borders) never moves a pixel across the waterline: land is never dug below the sea and no new land rises from it.
- **Poles.** Erosion fades out between 60° and 75° of latitude: droplets move in pixel steps and the projection stretches pixels east-west towards the poles, which would otherwise carve artificial north-south grooves.
- **Frozen ground.** Ice carves no river valleys: erosion fades out on rows whose sea-level temperature (latitude and the preset's climate shift) stays below the preset's polar threshold, rising to full strength 0.1 above it. Frozen worlds keep smooth ice sheets, and only their thawed belts are eroded.

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
