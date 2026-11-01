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

All parameters are optional. Any omitted value is chosen **randomly** at startup. The chosen values are printed, along with the exact options needed to regenerate the same world:

```text
Parameters → planet=frozen seed=2590618090 sea_level=0.12 volcanic=0.61 circumference=51823 km gravity≈1.29g
World generated → worlds/frozen-2590618090/
Reproduce with: --planet frozen --seed 2590618090 --sea-level 0.1213 --volcanic 0.6107 --circumference 51823.4
```

### Options

| Flag                    | Values                                                  | Default                      |
| ----------------------- | ------------------------------------------------------- | ---------------------------- |
| `--planet <type>`       | `terran` · `volcanic` · `frozen` · `caustic` · `barren` | random                       |
| `--seed <u32>`          | any 32-bit unsigned integer                             | random                       |
| `--sea-level <f32>`     | float in `[-1.0, 1.0]`                                  | random in `[-0.30, 0.50)`    |
| `--volcanic <f32>`      | float in `[0.0, 1.0]`                                   | random in `[0.0, 1.0)`       |
| `--circumference <f32>` | planet equatorial circumference in km (`>= 1`)          | random in `[20 000, 80 000)` |
| `--width <px>`          | map width, `2`–`16384`                                  | `1920`                       |
| `--height <px>`         | map height, `2`–`16384`                                 | `1080`                       |
| `--out-dir <path>`      | directory receiving the generated worlds                | `worlds`                     |
| `--no-json`             | skip `raw_data/world.json` (~230 MB at 1920 × 1080)     | off                          |
| `--no-noise-maps`       | skip the `noise_maps/` diagnostic images                | off                          |

Invalid or unknown options are rejected with an error. Run `target/release/map-generator --help` for the full reference.

**seed** — drives every noise layer. The same seed with the same other options always produces the same world.

**sea-level** — shifts the waterline. `0.0` is the default; positive values raise it (more ocean), negative values lower it (more land).

**volcanic** — controls how much of the mountain chains become volcanic. `0.0` = no volcanoes, `1.0` = most mountain chains erupt.

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
mise run gen -- --width 480 --height 270 --no-json --no-noise-maps
```

### Development tasks

| Task                    | Description                                     |
| ----------------------- | ----------------------------------------------- |
| `mise run build`        | Build the optimized binary                      |
| `mise run gen`          | Build, then generate a world                    |
| `mise run test`         | Run the test suite                              |
| `mise run lint`         | Check formatting and run clippy (`-D warnings`) |
| `mise run fmt`          | Format the code                                 |
| `mise run clean-worlds` | Delete every generated world under `worlds/`    |

### Planet types

| Type       | Description                                       | Exclusive biomes                            |
| ---------- | ------------------------------------------------- | ------------------------------------------- |
| `terran`   | Earth-like — full biome spectrum                  | —                                           |
| `volcanic` | Fire world — extreme heat, near-zero moisture     | `MagmaSea`, `ScorchedWaste`                 |
| `frozen`   | Ice world — perpetually frozen                    | `FrozenOcean`, `GlacialPlain`               |
| `caustic`  | Acid world — corrosive atmosphere, toxic wetlands | `CausticLake`, `ToxicSwamp`, `AcidFlatland` |
| `barren`   | Dead rock — arid and lifeless, no water           | `RockyWaste`, `DustPlain`                   |

### Output

Each run creates a `worlds/<planet>-<seed>/` directory with the following layout:

```text
worlds/<planet>-<seed>/
├── world.png          ← biome map with contour lines and reference lines
├── world.svg          ← equivalent vector image
├── legend.png         ← biome legend with planet metadata
├── raw_data/
│   └── world.json     ← full tile data (elevation, moisture, temperature, biome, …)
└── noise_maps/
    ├── noise_warp_x.png
    ├── noise_warp_y.png
    ├── noise_continent.png
    ├── noise_mountain.png
    ├── noise_mountain_wt.png
    ├── noise_elevation.png
    ├── noise_biome_elev.png
    ├── noise_moisture.png
    ├── noise_temperature.png
    ├── noise_volcanic_raw.png
    └── noise_volcanic_zone.png
```

| File / folder | Description                                                                                                                                                                                               |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `world.png`   | 1920 × 1080 PNG with biome colours, contour lines, and geographic reference lines                                                                                                                         |
| `world.svg`   | Equivalent vector image (run-length encoded `<rect>` rows); suitable for web embedding and scaling                                                                                                        |
| `legend.png`  | Auto-sized PNG listing every biome present on the map (colour swatch + name) plus planet metadata (type, seed, sea level, volcanic intensity, circumference, gravity)                                     |
| `raw_data/`   | Raw world data (compact JSON, one entry per tile). `elevation` is the terrain height before the sea-level shift; `moisture` and `temperature` include the planet-type offsets, as seen by biome selection |
| `noise_maps/` | False-colour PNG for each intermediate noise layer (see below)                                                                                                                                            |

#### Noise maps (false colour)

11 intermediate noise maps are exported into `noise_maps/` for study and debugging. They share a **jet colour ramp**:

> blue (low) → cyan → green → yellow → red (high)

Signed maps (range `[-1, 1]`) are centred so that `0.0` maps to green, negative values to blue/cyan, and positive values to yellow/red. Unsigned maps (range `[0, 1]`) run the full blue-to-red range.

| File                      | Range     | Description                                                     |
| ------------------------- | --------- | --------------------------------------------------------------- |
| `noise_warp_x.png`        | `[-1, 1]` | Domain-warp field, X axis                                       |
| `noise_warp_y.png`        | `[-1, 1]` | Domain-warp field, Y axis                                       |
| `noise_continent.png`     | `[-1, 1]` | Low-frequency FBM continent shape                               |
| `noise_mountain.png`      | `[0, 1]`  | Ridged noise (mountain peaks)                                   |
| `noise_mountain_wt.png`   | `[0, 1]`  | Mountain blend weight (derived from continent height)           |
| `noise_elevation.png`     | `[-1, 1]` | Final elevation = continent + mountain × weight × blend         |
| `noise_biome_elev.png`    | `[-1, 1]` | Elevation shifted by `sea_level` — what biome selection sees    |
| `noise_moisture.png`      | `[-1, 1]` | Moisture FBM                                                    |
| `noise_temperature.png`   | `[0, 1]`  | Temperature (latitude gradient + cooling of land with altitude) |
| `noise_volcanic_raw.png`  | `[-1, 1]` | Raw volcanic-zone FBM before threshold                          |
| `noise_volcanic_zone.png` | `[0, 1]`  | Processed volcanic zone (threshold applied)                     |

#### Geographic reference lines

Both `world.png` and `world.svg` overlay five dotted latitude lines for orientation:

| Line                | Latitude | Colour |
| ------------------- | -------- | ------ |
| Equator             | 0°       | Red    |
| Tropic of Cancer    | +23.5°   | Amber  |
| Tropic of Capricorn | −23.5°   | Amber  |
| Arctic Circle       | +66.5°   | Cyan   |
| Antarctic Circle    | −66.5°   | Cyan   |

The row position for each line is derived from the equirectangular projection used by the map: `row = height × (0.5 − latitude_deg / 180)`, so north is at the top of the image.

## Calculations

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

Both values are written into `world.json` as `circumference_km` and `gravity_modifier` for downstream use.
