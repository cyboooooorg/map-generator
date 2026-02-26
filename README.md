# Map Generator

This project is a map generator that creates random maps using Perlin noise. The generated maps can be used for various applications such as games, simulations, or any other project that requires a random terrain.

## Usage

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run [-- [OPTIONS]]
```

All parameters are optional. Any omitted value is chosen **randomly** at startup, and the chosen values are printed so every world is reproducible.

```text
Parameters → planet=Frozen  sea_level=0.12  volcanic_intensity=0.61
World generated → worlds/2590618090/
```

### Options

| Flag                | Values                                                  | Default                   |
| ------------------- | ------------------------------------------------------- | ------------------------- |
| `--planet <type>`   | `terran` · `volcanic` · `frozen` · `caustic` · `barren` | random                    |
| `--sea-level <f32>` | float in `[-1.0, 1.0]`                                  | random in `[-0.30, 0.50)` |
| `--volcanic <f32>`  | float in `[0.0, 1.0]`                                   | random in `[0.0, 1.0)`    |

**sea-level** — shifts the waterline. `0.0` is the default; positive values raise it (more ocean), negative values lower it (more land).

**volcanic** — controls how much of the mountain chains become volcanic. `0.0` = no volcanoes, `1.0` = most mountain chains erupt.

### Examples

```bash
# Fully random world
cargo run

# Frozen planet, everything else random
cargo run -- --planet frozen

# Earth-like ocean world, moderate volcanic activity
cargo run -- --planet terran --sea-level 0.3 --volcanic 0.4

# Barren dry rock, no volcanoes
cargo run -- --planet barren --sea-level -0.5 --volcanic 0.0

# Volcanic hell
cargo run -- --planet volcanic --sea-level -0.2 --volcanic 1.0
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

Each run writes two files inside `worlds/<seed>/`:

| File         | Description                                              |
| ------------ | -------------------------------------------------------- |
| `world.png`  | 1920 × 1080 PNG with biome colours and contour lines     |
| `world.json` | Full tile data (elevation, moisture, temperature, biome) |
