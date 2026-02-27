# Map Generator

This project is a map generator that creates random maps using Perlin noise. The generated maps can be used for various applications such as games, simulations, or any other project that requires a random terrain.

## Usage

### Build

```bash
devbox run build
```

### Run

```bash
devbox run prod [-- [OPTIONS]]
```

All parameters are optional. Any omitted value is chosen **randomly** at startup, and the chosen values are printed so every world is reproducible.

```text
Parameters → planet=Frozen  sea_level=0.12  volcanic_intensity=0.61  circumference=51823 km  gravity≈1.29g
World generated → worlds/frozen-2590618090/
```

### Options

| Flag                    | Values                                                  | Default                      |
| ----------------------- | ------------------------------------------------------- | ---------------------------- |
| `--planet <type>`       | `terran` · `volcanic` · `frozen` · `caustic` · `barren` | random                       |
| `--sea-level <f32>`     | float in `[-1.0, 1.0]`                                  | random in `[-0.30, 0.50)`    |
| `--volcanic <f32>`      | float in `[0.0, 1.0]`                                   | random in `[0.0, 1.0)`       |
| `--circumference <f32>` | planet equatorial circumference in km (`> 0`)           | random in `[20 000, 80 000)` |

**sea-level** — shifts the waterline. `0.0` is the default; positive values raise it (more ocean), negative values lower it (more land).

**volcanic** — controls how much of the mountain chains become volcanic. `0.0` = no volcanoes, `1.0` = most mountain chains erupt.

**circumference** — equatorial circumference of the planet in kilometres. Drives two derived quantities (noise scale and gravity) described in the [Calculations](#calculations) section below.

### Examples

```bash
# Fully random world
devbox run prod

# Frozen planet, everything else random
devbox run prod -- --planet frozen

# Earth-like ocean world, moderate volcanic activity
devbox run prod -- --planet terran --sea-level 0.3 --volcanic 0.4

# Barren dry rock, no volcanoes
devbox run prod -- --planet barren --sea-level -0.5 --volcanic 0.0

# Volcanic hell
devbox run prod -- --planet volcanic --sea-level -0.2 --volcanic 1.0
```

### Planet types

| Type       | Description                                       | Exclusive biomes                            |
| ---------- | ------------------------------------------------- | ------------------------------------------- |
| `terran`   | Earth-like — full biome spectrum                  | —                                           |
| `volcanic` | Fire world — extreme heat, near-zero moisture     | `MagmaSea`, `ScorchedWaste`                 |
| `frozen`   | Ice world — perpetually frozen                    | `FrozenOcean`, `GlacialPlain`               |
| `caustic`  | Acid world — corrosive atmosphere, toxic wetlands | `CausticLake`, `ToxicSwamp`, `AcidFlatland` |
| `barren`   | Dead rock — arid and lifeless, no water           | `RockyWaste`, `DustPlain`                   |

### Output

Each run writes three files inside `worlds/<planet>-<seed>/`:

| File         | Description                                                                                        |
| ------------ | -------------------------------------------------------------------------------------------------- |
| `world.png`  | 1920 × 1080 PNG with biome colours, contour lines, and geographic reference lines                  |
| `world.svg`  | Equivalent vector image (run-length encoded `<rect>` rows); suitable for web embedding and scaling |
| `world.json` | Full tile data (elevation, moisture, temperature, biome, circumference, gravity, …)                |

#### Geographic reference lines

Both `world.png` and `world.svg` overlay five dotted latitude lines for orientation:

| Line                | Latitude | Colour |
| ------------------- | -------- | ------ |
| Equator             | 0°       | Red    |
| Tropic of Cancer    | +23.5°   | Amber  |
| Tropic of Capricorn | −23.5°   | Amber  |
| Arctic Circle       | +66.5°   | Cyan   |
| Antarctic Circle    | −66.5°   | Cyan   |

The row position for each line is derived from the equirectangular projection used by the map: `row = height × (0.5 + latitude_deg / 180)`.

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
