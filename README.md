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
Parameters → planet=frozen seed=2590618090 sea_level=0.12 volcanic=0.61 circumference=51823 km tilt=18.7° gravity≈1.29g pressure=0.412 bar surface_age=1.24 Ga (plate tectonics) star=K2V orbit=1.411 AU (2.03 in the habitable zone, 0.19× Earth's light)
World generated → worlds/frozen-2590618090/
Reproduce with: --planet frozen --seed 2590618090 --sea-level 0.1213 --volcanic 0.6107 --circumference 51823.4 --axial-tilt 18.72 --erosion 1 --pressure 0.41237 --surface-age 1.2381 --star K2V --orbit-au 1.4107 --width 1920 --height 1080
```

### Options

| Flag                    | Values                                                                                                                                                 | Default                                           |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| `--planet <preset>`     | built-in preset name (`terran` · `volcanic` · `frozen` · `caustic` · `barren`) or path to a `.toml` preset file, see [Planet presets](#planet-presets) | random built-in                                   |
| `--list-planets`        | list the built-in presets and exit                                                                                                                     | —                                                 |
| `--seed <u32>`          | any 32-bit unsigned integer                                                                                                                            | random                                            |
| `--sea-level <f32>`     | float in `[-1.0, 1.0]`                                                                                                                                 | random in `[0.10, 0.40)`                          |
| `--land-fraction <f32>` | share of the surface that is land, `[0.0, 1.0]`; sets the sea level to match, so it cannot be combined with `--sea-level`                              | —                                                 |
| `--volcanic <f32>`      | float in `[0.0, 1.0]`                                                                                                                                  | random in `[0.0, 1.0)`                            |
| `--circumference <f32>` | planet equatorial circumference in km (`>= 1`)                                                                                                         | random in `[20 000, 80 000)`                      |
| `--axial-tilt <deg>`    | axial tilt in degrees, `[0, 90]`                                                                                                                       | random in `[5, 40)`                               |
| `--erosion <f32>`       | hydraulic erosion strength, `[0, 5]`; `0` disables it                                                                                                  | `1.0`                                             |
| `--pressure <bar>`      | surface pressure in bar, `[0, 1000]`; `0` is airless                                                                                                   | from the preset, else random in `[0.7, 1.5)`      |
| `--surface-age <Ga>`    | age of the oldest surviving surface in Ga, `[0, 4.6]`: how long impacts have cratered it                                                               | from the preset, else random in `[0.2, 0.6)`      |
| `--star <type>`         | spectral type of the host star, `O3V` to `M9.5V` (the Sun: `G2V`)                                                                                      | from the preset, else drawn like the stars nearby |
| `--orbit-au <AU>`       | semi-major axis of the orbit in AU, `[0.005, 1000]`; cannot be combined with `--hz-position`                                                           | from the preset                                   |
| `--hz-position <f32>`   | position of the orbit in the star's habitable zone, `[-2, 6]`: `0` its inner edge, `1` its outer edge                                                  | from the preset, else random in `[0.1, 0.9)`      |
| `--width <px>`          | map width, `2`–`16384`                                                                                                                                 | `1920`                                            |
| `--height <px>`         | map height, `2`–`16384`                                                                                                                                | `1080`                                            |
| `--out-dir <path>`      | directory receiving the generated worlds                                                                                                               | `worlds`                                          |
| `--no-raw-data`         | skip the `raw_data/` export (about 15 MB at 1920 × 1080)                                                                                               | off                                               |
| `--no-noise-maps`       | skip the `noise_maps/` diagnostic images, and the time they take; also set by `MAPGEN_NO_NOISE_MAPS` (see [Environment](#environment))                 | off                                               |

Invalid or unknown options are rejected with an error. Run `target/release/map-generator --help` for the full reference.

**seed** — drives every noise layer. The same seed with the same other options always produces the same world. Each noise layer hashes the seed with its own stream number, so the layers of a world are independent and neighbouring seeds share nothing. A seed noted before that change (November 2026), or before impact craters were added (the same month), gives a different world now; a world already generated keeps its files.

**sea-level** — shifts the waterline: positive values raise it (more ocean), negative values lower it (more land). When it is not set (nor by the preset), it is drawn in `[0.10, 0.40)`, which gives about 10–70 % of land, mostly 20–55 % (Earth: 29 %). Mountains and snow are measured above the waterline, so a low sea level also turns much of the land into highland: at `0.0` about a third of the land is mountain, at `-0.3` almost three quarters.

**land-fraction** — sets the share of land instead of the sea level: `--land-fraction 0.29` gives a world with 29 % of its surface (by area, not pixels) above the sea, whatever the relief of the seed, where a fixed sea level gives anywhere from 20 to 50 %. The generator samples the relief on a coarse grid (512 × 256, about 0.1 s) and picks the sea level that leaves that share above it; on test worlds the result is within 0.1 point of the target. The chosen sea level is printed and stored in `world.json` like any other, so `--sea-level` reproduces the world. In the web form, the sea level fieldset switches to it with its "Land share (%)" option. A preset can set `land_fraction` in `[defaults]` instead of `sea_level`, not both.

**volcanic** — controls how much of the planet is volcanic: the share of volcanic provinces, and so the number of volcanoes, not their size. `0.0` = no volcanoes, `1.0` = many volcanic provinces.

**axial-tilt** — tilt of the rotation axis. Shapes the temperature profile from equator to poles and places the tropics and polar circles, see [Temperature](#temperature--axial-tilt).

**erosion** — how much rain carves the terrain: droplets per cell of the erosion grid, see [Erosion](#hydraulic-erosion).

**pressure** — surface pressure of the atmosphere, in bar (Earth: 1, Mars: 0.006, Venus: 92, 0 for an airless world). Presets give their own range (`[defaults] pressure`); the sky colour, haze and clouds come from the preset's `[atmosphere]` table. It is stored in `world.json` (`pressure_bar`; worlds made before it had Earth's 1 bar). Dense air evens temperatures out between the equator and the poles, and below 0.01 bar no water stays liquid: no rain, rivers or lakes, and no hydraulic erosion unless the request or the preset sets it (see [Temperature](#temperature--axial-tilt)). The viewer draws the sky from it.

**surface-age** — age of the oldest surviving surface, in Ga (billions of years): how long impacts have been cratering it since it was last renewed. Earth keeps few craters older than about 0.4 Ga (erosion and subduction erase them), Venus was resurfaced about 0.5 Ga ago, the Moon and Mars keep surfaces of 4 Ga or more. Presets give their own range (`[defaults] surface_age`), along with their tectonic regime and the end of their wet era (`[geology]`, see [Planet presets](#planet-presets)). It is stored in `world.json` (`surface_age_ga`; worlds made before it have none). It sets how cratered the world is, see [Impact craters](#impact-craters).

**star** and **orbit** — the host star, a main-sequence dwarf from `O3V` to `M9.5V` (the Sun is `G2V`), and the distance of the planet from it. Without `--star`, the preset's star is taken, else one is drawn with the frequencies of the solar neighbourhood (about three quarters red dwarfs, M; 12 % K, 8 % G, 3 % F, under 1 % A, B and O). The orbit is given either in AU (`--orbit-au`) or as a position in the star's habitable zone (`--hz-position`, Kopparapu et al. 2014: `0` at its inner edge, `1` at its outer edge, below 0 closer in, above 1 farther out), so the same position gives the same light around any star; presets place each planet type where it makes sense (terran inside the zone, frozen just beyond it, volcanic anywhere). The star's temperature, luminosity, mass and radius come from the mean dwarf sequence of Pecaut & Mamajek (2013). `world.json` stores them under `star`, with what they imply: the light the planet gets (`insolation`, relative to Earth's), the habitable zone, the equilibrium temperature, the year, the apparent size of the star, its lifetime, whether the planet is tidally locked and the colour of its light (worlds made before stars have none and orbit the Sun at 1 AU). It sets the planet's temperatures, see [Temperature](#temperature--axial-tilt). The research behind it is in [docs/analysis/10-stars.md](docs/analysis/10-stars.md); see `src/star.rs`.

**circumference** — equatorial circumference of the planet in kilometres. Drives two derived quantities (noise scale and gravity) described in the [Calculations](#calculations) section below.

### Environment

| Variable               | Effect                                                                                                                          | Default |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------- |
| `MAPGEN_NO_NOISE_MAPS` | `true` (or `1`, `yes`, `on`) skips the noise maps like `--no-noise-maps`; `false` (or `0`, `no`, `off`) keeps them, the default | `false` |

The flag wins when both are given: `--no-noise-maps` always skips the maps. For example, to skip them for every run of a shell session:

```bash
export MAPGEN_NO_NOISE_MAPS=true
mise run gen -- --planet terran
```

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

The same resampling runs in the web app (hex overlay of the viewer), through the `mapgen::hex` library module: the server resamples the world it keeps for the detail tiles, one grid at a time, and writes no `hex/` files. While a generation runs it refuses hex grids (503), as loading a world would add to the generation's peak.

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

A browser app to explore the generated worlds as a 3D globe or a flat map, switch between biome, elevation, temperature, moisture, river, volcanic, distance-to-sea and floodplain layers, raise the relief, light the planet with a seasonal sun, overlay the hex grid, inspect any point, its river and its hex, measure distances, compare two worlds side by side, download a world's files, and queue new worlds from a form, in English or French. See [web/README.md](web/README.md) for the full feature list and the keyboard shortcuts. It is a personal tool served locally behind the machine-wide Traefik, like the other `*.localhost` apps.

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

Generations run one at a time (each one already uses every core): later ones wait in a queue. Their progress streams to the page (`GET /api/jobs/{id}/events`); `GET /api/jobs` lists the recent jobs, so a reloaded page picks up the ones still running, and `DELETE /api/jobs/{id}` cancels one: a queued job never starts, a running one stops when its next stage begins (or, during the export, once the files are written) and leaves nothing on disk: a world is exported into a hidden `worlds/.tmp-<id>/` directory renamed into place once complete, removed if the job is cancelled or fails, and swept at startup if the server was killed mid-export. A world cannot be deleted while its generation is queued or running (409). Zooming in loads [detail tiles](#detail-tiles--deep-zoom) rendered on demand by the server (`GET /api/worlds/{id}/tiles/{z}/{x}/{y}?rev=&render=`), cached by the browser for good since the URL names the world revision and the renderer version (`RENDER_VERSION` in `src/detail.rs`, mirrored by `TILE_RENDER_VERSION` in the viewer and checked by a test): bump both whenever tile colours or shading change, or browsers keep tiles of the previous renderer. A world generated before a renderer change keeps its old `world.png` under new tiles until it is regenerated. The server keeps the most recently used worlds in memory within `MAPGEN_TILE_CACHE_MB` (default 1024 MiB; a world takes about 23 bytes per pixel, 0.8 GB at 8192 × 4096, and the last one used is always kept). It loads one world at a time and evicts the least recently used ones before loading, so the budget holds during the load too. A generation waits for a load in progress to finish before it starts. While a generation runs, it frees them and answers tile and hex grid requests with 503 and `Retry-After` (the viewer retries both), so they do not add to the generation peak; tiles render at most four at a time, and a request the viewer abandons before its turn is dropped. The world list shows each world's `thumb.png` (`GET /api/worlds/{id}/thumb?rev=`, cached for good like the tiles); for a world made before thumbnails, the server makes it on the first request from `world.png`, decoded one row at a time (about 4 MB and 0.4 s at 8192 × 4096, two at a time, during generations too), and writes it into the world directory. The server refuses to overwrite an existing world (409) unless the request asks to replace it (`POST /api/worlds?replace=true`): the new world is then generated into its temporary directory as usual and swapped in once complete (the old one is renamed to `worlds/.old-<id>/`, the new one renamed into place, then the old one removed; the startup sweep puts the old one back if the server was killed between the two renames), so a generation that fails or is cancelled keeps the old world, and its tiles are served from the new revision afterwards. Meanwhile the old world stays listed and viewable, but cannot be deleted (409), and a second generation of the same id is refused (409). Maps are limited to `MAPGEN_MAX_PIXELS` pixels and 8192 pixels per side. The server defaults to 4096 × 2048; `compose.yaml` raises it to 8192 × 4096 for the 6 GB Colima VM of the workstation setup. A generation peaks at about 0.16 GiB at 1920 × 1080, 0.55 GiB at 4096 × 2048 and 2.0 GiB at 8192 × 4096 (about 70 bytes per pixel, measured in Docker; about 2 s, 8 s and 40 s on 6 cores): keep the limit under the memory the container can use, or the server is killed mid-job. With Colima's default 2 GB VM, lower it (`MAPGEN_MAX_PIXELS: "8388608"`) or give the VM more memory (`colima stop && colima start --cpu 6 --memory 6`). The web app skips the diagnostic maps, which would add about 10 bytes per pixel. Between generations the server holds little more than the worlds loaded for tiles: the Docker image limits glibc to two malloc arenas (`MALLOC_ARENA_MAX=2`), and the server returns freed memory to the system after each generation and cache eviction (`malloc_trim`); after a 4096 × 2048 generation it holds about 50 MB instead of 170–400 MB. A failed API call answers `{"error": "..."}` with an English message; the errors a user can fix also carry `detail`, the reason as data (`{"code": "out_of_range", "field": "sea_level", "min": -1, "max": 1}`; the codes are those of `ErrorDetail` in `crates/server/src/api.rs`), which the web app words in its own language. World files (`/worlds/...`) and the page are sent with `Cache-Control: no-cache`, so browsers revalidate them (a world id is reused when a world is deleted and generated again), while the content-hashed bundles under `/assets/` are cached for a year; JSON, JavaScript, CSS and SVG answers above 1 KiB are gzipped when the browser accepts it, PNG images are sent as they are.

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

[locales.fr]                       # name and description in another language, for the web app
name = "Type Mars"
description = "Petit désert froid aux calottes polaires"

[defaults]                         # a fixed value or a [min, max] range drawn at random
sea_level = [0.10, 0.30]           # sea_level or land_fraction, volcanic, circumference, axial_tilt, erosion, pressure, surface_age
circumference = 21300
pressure = 0.006                   # bar
surface_age = 4.0                  # Ga

[climate]                          # added to every pixel before biome selection
temperature = -10.0                # °C, after the star (see Temperature)
moisture = -0.7                    # [-1, 1] scale
volcanic = 0.0                     # [0, 1] scale
internal_heat = 0.0                # K: the planet's own heat (285 on volcanic worlds)

[rules]                            # override any biome selection threshold
polar = -70.0                      # temperatures in °C
highland_snow_temperature = -70.0

[atmosphere]                       # sky and clouds; Earth's when left out
haze = 0.4                         # aerosol density, 0 (clear) to 1 (opaque)
haze_colour = [0.85, 0.6, 0.4]     # aerosol tint (butterscotch dust)
clouds = "dust"                    # water | sulfuric | dust | none

[geology]                          # tectonic regime and wet era; Earth's when left out
regime = "stagnant"                # plates | stagnant | episodic | heat_pipe
wet_until = 3.5                    # Ga: end of the era of liquid water
maria = [3.7, 3.0]                 # Ga: when lava flooded large basins

[remap]                            # target = [biomes replaced by it]
DustPlain = ["Ocean", "DeepOcean", "Lake", "River", "Plain", "Desert"]
IceCap = ["SeaIce"]
RockyWaste = ["Mountain"]
```

Every table is optional.

To make a dry world, remap its oceans rather than lowering `sea_level`: biome elevations are measured from the sea level, so a world with a sea level of −0.5 has nearly all its land above the snow line (0.3 above the sea) and turns white. The `mars-like` example keeps the usual sea level and turns its seas into dust basins.

- **`[locales.<language>]`**: `name` and `description` in another language (`fr`, two or three lowercase letters), each optional. The web app shows them when its interface is in that language, and the English ones otherwise. They are not copied into `world.json`, so a world does not change when a translation does.
- **`[defaults]`**: command-line options always win; parameters set neither on the command line nor here use the generator defaults listed under [Options](#options).
- **`[rules]`**: accepts any field of `BiomeRules` in `src/biome.rs` (`waterline`, `polar`, `forest_moisture`, `shore_width_km`…), each documented there. Fields left out keep their default.
- **`[atmosphere]`**: `rayleigh` (relative scattering per red, green, blue channel: the clear-sky colour; Earth's air is about `[0.17, 0.41, 1.0]`), `haze` and `haze_colour` (aerosols), `greenhouse` (°C added to every temperature), `clouds` (`water`, `sulfuric`, `dust` or `none`) and `cloud_cover` (−1 to 1). Each field is optional and defaults to Earth's. See `src/atmosphere.rs`.
- **`[geology]`**: `regime` (`plates`, `stagnant`, `episodic` or `heat_pipe`: how the planet loses its heat, from Earth's plates to the single rigid lid of Mars and the Moon, Venus's global overturns and Io's lava) and `wet_until` (Ga before present when liquid water disappeared: `0` for a world still wet, a value at or above the surface age for one never wet, like Mercury; a world with liquid water today counts as still wet whatever the preset says). Both default to Earth's (`plates`, `0`). `maria` (`[start, end]` in Ga, start the older) is when lava flooded large impact basins, as the Moon's maria from 3.9 to 3.1 Ga; without it, none are. `wet_until` sets which [impact craters](#impact-craters) are worn by water, `maria` where they are buried by lava; `regime = "plates"` gives a world tectonic plates whose boundaries shape its relief (see [Elevation](#elevation--continents-coasts-and-mountains)); the other regimes are recorded and shown. See `src/geology.rs`.
- **`[star]`**: `type` (a spectral type such as `G2V`; left out, drawn like the stars nearby) and either `position` (in the habitable zone, a value or a `[min, max]` range) or `orbit_au` (a distance). The built-in types: `terran` at `[0.05, 0.75]`, `frozen` at `[1.0, 1.35]`, just beyond the zone (its cold comes from the distance), `volcanic` at `[-0.5, 4.0]` (its heat comes from within), `caustic` at `[-0.25, 0.1]`, `barren` at `[-1.0, 4.0]`; the examples orbit the Sun at the distances of Earth, Mars, Venus and Mercury. Without the table, the star is drawn and the planet sits at `[0.1, 0.9]`. See `src/star.rs`.
- **`[remap]`**: runs after biome selection; biome names are the keys of the biome table (`DeepOcean`, `SeaIce`, `LavaField`…). A biome listed under two targets is an error, and so is an unknown field or biome name.

`raw_data/world.json` stores the full resolved preset under `preset`, so a world can be regenerated even if its preset file changes.

### Output

Each run creates a `worlds/<planet>-<seed>/` directory with the following layout:

```text
worlds/<planet>-<seed>/
├── world.png          ← hillshaded biome map with soft borders, bathymetry and reference lines
├── thumb.png          ← world.png scaled down to 512 pixels wide, for lists
├── world.svg          ← equivalent vector image
├── legend.png         ← biome legend with planet metadata
├── raw_data/
│   ├── world.json     ← planet parameters, layer encodings, biome table
│   ├── elevation.png  ← 16-bit greyscale layer
│   ├── elevation_base.png ← 16-bit greyscale layer, without the young craters
│   ├── moisture.png   ← 16-bit greyscale layer
│   ├── temperature.png ← 16-bit greyscale layer
│   ├── flow.png       ← 16-bit greyscale layer, log scale
│   ├── volcanic_zone.png  ← 16-bit greyscale layer
│   ├── coast_distance.png ← 16-bit greyscale layer
│   ├── highland_offset.png ← 16-bit greyscale layer
│   ├── floodplain.png ← 16-bit greyscale layer
│   ├── rivers.json    ← river polylines with their flow
│   ├── biomes.png     ← 8-bit greyscale, pixel = biome id
│   ├── clouds.png     ← 8-bit greyscale cloud cover (worlds with clouds)
│   ├── albedo.png     ← 8-bit greyscale brightness of fresh craters' ejecta and rays
│   └── surface.png    ← world.png without its lines, for the viewer
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
    ├── noise_tectonic.png
    ├── erosion.png
    ├── elevation.png
    ├── temperature.png
    ├── coast_distance.png
    ├── precipitation.png
    ├── clouds.png
    ├── moisture.png
    └── flow.png
```

| File / folder | Description                                                                                                                                                                                                                                                                  |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `world.png`   | PNG (1920 × 1080 by default) with biome surface colours (satellite-like palette), soft biome borders (ecotones), hillshaded land (light from the north-west), meandering rivers whose width grows with their flow, depth contours under water and geographic reference lines |
| `thumb.png`   | `world.png` scaled down to 512 pixels wide (narrower maps keep their size), same aspect ratio, by area averaging in linear light: 0.1–0.2 MB instead of a full-size image for the web app's world list. Worlds made before it get one from the server on first request       |
| `world.svg`   | Same map as a vector image (run-length encoded `<rect>` rows), without hillshading so the encoding stays compact; suitable for web embedding and scaling                                                                                                                     |
| `legend.png`  | Auto-sized PNG listing every biome present on the map (surface and classification swatches + name) plus planet metadata (type, seed, sea level, volcanic intensity, circumference, gravity)                                                                                  |
| `raw_data/`   | Machine-readable world data, see [Raw data](#raw-data)                                                                                                                                                                                                                       |
| `noise_maps/` | False-colour PNG for each intermediate noise layer (see below)                                                                                                                                                                                                               |

#### Soft biome borders

`world.png` and the viewer's detail tiles use a **surface palette** (`surface_color` in `src/biome.rs`): colours as seen from orbit, with dark olive vegetation, pale tan deserts and dark open water, while exotic biomes keep a muted hue of their own. The **classification palette** (`biome_color`), saturated and far apart, stays on `world.svg` (seas included, with no per-pixel variation so its run-length encoding stays compact), the hex grid and the `color` of the biome table. `legend.png` shows both swatches for each biome: surface first, classification second.

Land colours vary within a biome: two smooth noise fields on the sphere (wavelengths from 400 km down to 50 km, a noise stream of their own) shift the brightness by up to 18 % and the warmth, red against blue, by up to 8 %, so large forests and deserts are no longer flat plates. The fields depend on the position alone, so the base map and the detail tiles agree; lava varies too, so no step appears through the soft border around it. This is rendering only.

`world.png` draws biome borders as ecotones. Each land pixel takes the mean colour of the biomes its inputs select when nudged by 128 small offsets (a 4D Gaussian, σ 0.9 °C on temperature, 0.05 on moisture, 0.003 on the highland line and 0.004 on the volcanic zone): far from any threshold the colour is flat (8 probes tell, so most pixels skip the sampling), near one the colours mix, averaged in linear light (an sRGB average darkens every border). The width of a transition is set in input units, not pixels, so it suits any zoom level; mountain, snow and volcanic borders soften too, volcanoes less than climate borders. The same offsets serve every pixel, so transitions show neither grain nor terraces. Coastlines, shores, rivers and lakes keep their exact inputs and stay crisp. This is rendering only: `raw_data/biomes.png` and `world.svg` keep crisp, flat classes.

Clear liquids take their colour from their depth (Beer-Lambert absorption in linear light): the bed shows through shallow water and fades with depth, faster in the channels the liquid absorbs most, towards the colour of deep water. Water absorbs red first, so shelves turn turquoise and the open sea dark blue past the shelf break; acid absorbs blue first, so its shallows turn yellow-green. The colour depends on depth alone, so there is no step where Ocean becomes Deep Ocean. A lake's depth is measured from its own level (its highest pixel), not from the sea, and counts four times, as lakes are far shallower than seas. Frozen seas and lakes whiten into fast ice along the shore (up to 25 % over the last `0.05` elevation units), magma seas darken into a cooled crust, and rivers stay flat. Every sea also carries a pattern of its own kind, mixed in linear light and the same on the base map and the detail tiles: water and acid show swirls drawn by currents (a domain-warped noise from 900 km down to 56 km; teal plankton up to 30 % on water, sulphur yellow up to 14 % on acid), ice shows bluish leads of open water between floes (crests of a ridged noise, 700 km), magma shows rafts of dark cooled crust and yellow-hot gaps (1200 km down to 75 km, up to 60 % crust and 30 % hot colour); all seas vary in brightness by up to 8 %. Rivers stay flat. Contour lines are drawn under water only, as bathymetry: on land the hillshade shows the relief, and contours drew rings that read as altitude bands. The hillshade follows a soft curve (`1 + 0.4 × tanh((lit − 1) / 0.4)`, relative to flat ground) rather than a hard clamp, so steep slopes stay distinct; detail tiles use the same curve.

#### Raw data

`raw_data/` holds one lossless PNG per layer, pixel-aligned with `world.png` (row 0 = north pole, column 0 = longitude 0), and a `world.json` describing how to decode them.

| File                  | Encoding         | Content                                                                                                                                                                                                         |
| --------------------- | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `elevation.png`       | 16-bit grey      | Terrain height in `[-1, 1]`, before the sea-level shift (biomes use `elevation − sea_level`)                                                                                                                    |
| `elevation_base.png`  | 16-bit grey      | The elevation without the impact craters younger than the wet era (those drawn after erosion), same range: the surface the time slider will draw them on (format version 7 and later)                           |
| `moisture.png`        | 16-bit grey      | Moisture in `[-1, 1]`, planet-type offset included                                                                                                                                                              |
| `temperature.png`     | 16-bit grey      | Annual mean temperature in °C (−200 to 600), planet-type offset included                                                                                                                                        |
| `flow.png`            | 16-bit grey, log | River flow: `log10(1 + km²)` in `[0, 8]`, the wet-equivalent area draining through the pixel                                                                                                                    |
| `volcanic_zone.png`   | 16-bit grey      | Volcanic zone in `[0, 1]` as biome selection sees it (planet shift included)                                                                                                                                    |
| `coast_distance.png`  | 16-bit grey      | Distance to the nearest open water in `[0, 2000]` km (0 on water, capped)                                                                                                                                       |
| `highland_offset.png` | 16-bit grey      | Local shift of the mountain line in `[-1, 1]`: a pixel is highland when `elevation − sea_level + offset` exceeds the preset's `highland`                                                                        |
| `floodplain.png`      | 16-bit grey      | Floodplain weight in `[0, 1]`: 1 on the flat floor of a river valley, 0 away from rivers                                                                                                                        |
| `biomes.png`          | 8-bit grey       | Biome id of each pixel                                                                                                                                                                                          |
| `clouds.png`          | 8-bit grey       | Cloud cover in `[0, 1]` (`pixel / 255`), from the climate and the cloud kind; absent when the planet has no clouds                                                                                              |
| `albedo.png`          | 8-bit grey       | Brightness shift of the ground in `[-1, 1]` (`pixel / 255 × 2 − 1`, 0 at 128): the bright ejecta and rays of fresh impact craters (zero on worlds with air enough for liquid water) and the dark lava of the maria (format version 8 and later) |
| `surface.png`         | 8-bit RGB        | The render of `world.png` without its reference lines and depth contours: the viewer draws them itself, and can hide them                                                                                       |

A 16-bit pixel `p` decodes to `min + p / 65535 × (max − min)`, with `min` and `max` given per layer in `world.json`. For `flow` (encoding `gray16-log10`) this gives `log10(1 + km²)`: the area is `10^value − 1`. The precision is about `3 × 10⁻⁵` of the range (0.012 °C for temperature).

`world.json` contains the generation parameters, the `layers` description and the `biomes` table mapping each id to its key, display name, classification colour (`color`), surface colour (`surface_color`, empty in worlds written before it existed) and whether it is a water surface:

```json
{
  "format_version": 4,
  "width": 1920,
  "height": 1080,
  "seed": 42,
  "planet_type": "terran",
  "pressure_bar": 0.93,
  "surface_age_ga": 0.41,
  "star": { "type": "M3V", "teff_k": 3430.0, "luminosity": 0.0162, "orbit_au": 0.172, "insolation": 0.55, "hz_position": 0.4, "year_days": 43.1, "tidally_locked": true, "...": "..." },
  "layers": {
    "elevation": { "file": "elevation.png", "encoding": "gray16", "min": -1.0, "max": 1.0 },
    "biomes": { "file": "biomes.png", "encoding": "gray8-biome-id" }
  },
  "biomes": [
    { "id": 0, "key": "DeepOcean", "name": "Deep Ocean", "color": "#0A148C", "surface_color": "#081630", "water": true }
  ]
}
```

The example is abridged. Biome ids are stable: new biomes only get appended. `format_version` changes when the layout of `raw_data/` changes: version 2 added `volcanic_zone`, `coast_distance` and `highland_offset`, so that every input of biome selection is exported, and `rivers.json`; version 3 added `floodplain`; version 4 stores temperatures in °C instead of the former `[0, 1]` scale; version 5 adds `clouds`; version 6 adds `surface`; version 7 adds `elevation_base`, and its worlds have impact craters; version 8 adds `albedo`. Older worlds still load: missing layers are recomputed, or zero for `floodplain`, and the temperatures of versions 1–3 (layer and preset thresholds) are converted to °C through the Earth-fitted curve, which keeps their biomes.

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

Every intermediate layer of the generation is exported into `noise_maps/` for study and debugging (skip them with `--no-noise-maps` or `MAPGEN_NO_NOISE_MAPS=true`). Each map is written as soon as its layer is computed, during the generation, and the layer freed, so they are never all held in memory: they add about 10 bytes per pixel to the peak memory (0.6 GiB instead of 0.55 GiB at 4096 × 2048, 2.3 GiB instead of 2.0 GiB at 8192 × 4096) and little time, as they are encoded on the side. They share a **jet colour ramp**:

> blue (low) → cyan → green → yellow → red (high)

Signed maps (range `[-1, 1]`) are centred so that `0.0` maps to green, negative values to blue/cyan, and positive values to yellow/red. Unsigned maps (range `[0, 1]`) run the full blue-to-red range.

| File                            | Range     | Description                                                                                                                          |
| ------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `noise_warp_x.png`              | `[-1, 1]` | Domain-warp field, X axis                                                                                                            |
| `noise_warp_y.png`              | `[-1, 1]` | Domain-warp field, Y axis                                                                                                            |
| `noise_continent.png`           | `[-1, 1]` | Continent shape: FBM, fractal outline near the coast, smoothed plains inland                                                         |
| `noise_mountain.png`            | `[0, 1]`  | Ridged noise (mountain ridges, with two finer ridged octaves)                                                                        |
| `noise_mountain_wt.png`         | `[0, 1]`  | Mountain blend weight (derived from continent height)                                                                                |
| `noise_elevation.png`           | `[-1, 1]` | Elevation before erosion = continent + mountain × weight × blend                                                                     |
| `noise_biome_elev.png`          | `[-1, 1]` | Elevation shifted by `sea_level` — what biome selection sees                                                                         |
| `noise_moisture.png`            | `[-1, 1]` | Moisture noise (local variation added to the climate model)                                                                          |
| `noise_volcanic_raw.png`        | `[-1, 1]` | Raw volcanic-zone FBM before threshold                                                                                               |
| `noise_volcanic_zone.png`       | `[0, 1]`  | Volcanic zone: threshold applied, peaking on the volcanic cones                                                                      |
| `noise_temperature_anomaly.png` | `[-1, 1]` | Large-scale temperature anomaly noise (features of ~3 000 km)                                                                        |
| `noise_variability_t.png`       | `[-1, 1]` | Regional variability noise added to temperature                                                                                      |
| `noise_variability_m.png`       | `[-1, 1]` | Regional variability noise added to moisture                                                                                         |
| `noise_highland_jitter.png`     | `[-1, 1]` | Noise shifting the mountain line (× 0.5 gives `highland_offset`)                                                                     |
| `noise_tectonic.png`            | `[-1, 1]` | Relief of plate boundaries, × 4 (red: ranges, arcs and ridges; blue: trenches and rifts)                                             |
| `erosion.png`                   | `[-1, 1]` | Height change from erosion, ×10 (blue = carved, red = deposited)                                                                     |
| `craters.png`                   | `[-1, 1]` | Height change from impact craters, ×5 (blue = cavities, red = rims and ejecta), before and after erosion                             |
| `plates.png`                    | `[0, 1]`  | The tectonic plates (one value per plate), not yet used by the relief (see `src/plates.rs`)                                          |
| `plate_boundaries.png`          | `[-1, 1]` | Convergence of the nearest plate boundary (10 cm/yr full scale), fading 400 km from it: red where plates close, blue where they part |
| `elevation.png`                 | `[-1, 1]` | Final elevation, after erosion                                                                                                       |
| `temperature.png`               | `[0, 1]`  | Temperature before the planet-type offset, −30 °C to 40 °C (see [Temperature](#temperature--axial-tilt))                             |
| `coast_distance.png`            | `[0, 1]`  | Distance to the nearest open water, `1.0` = 2 000 km or more                                                                         |
| `precipitation.png`             | `[-1, 1]` | Rain brought by the winds, smoothed (`0` = neutral rain rate)                                                                        |
| `clouds.png`                    | `[0, 1]`  | Cloud cover (see [Clouds](#clouds))                                                                                                  |
| `moisture.png`                  | `[-1, 1]` | Final moisture before the planet-type offset (see [Moisture](#moisture--winds-and-rain-shadows))                                     |
| `flow.png`                      | `[0, 1]`  | River flow: `log10(1 + drainage km²) / 7` (see [Rivers and lakes](#rivers-and-lakes))                                                |

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

`S` is then rescaled so that an Earth-like tilt (23.44°) gives `t = 0` at the poles and `t = 1` at the equator (kept within `[−0.5, 1.5]` for other tilts), and turned into an annual mean temperature in °C by a curve fitted to Earth's zonal mean:

```text
T = −18 + 66.2 t − 21.2 t²     (−18 °C at the poles, 12 °C at 42°, 27 °C at the equator)
```

Temperature is not linear in sunlight: the tropics are flatter, heat being carried polewards, so a straight line would leave the mid-latitudes too cold. Land then cools by `13.5 °C` per unit of elevation above the waterline; the sea surface does not.

A large-scale anomaly is added on top: `18 °C × fbm` on land and `9 °C × fbm` on the sea (features of about 3 000 km, standard deviation about 3.6 °C on land). It stands for ocean currents, monsoons and continentality without modelling them, so climate zones shift by up to ~15° of latitude between continents instead of following parallels. It is zero-mean: the share of cold and hot climates does not change.

| Axial tilt | Effect                                                   |
| ---------- | -------------------------------------------------------- |
| 0°         | Strongest equator-to-pole contrast, large ice caps       |
| 23.44°     | Earth baseline                                           |
| ~54°       | Nearly uniform temperature across latitudes              |
| 70°+       | Poles receive more sunlight over a year than the equator |

The atmosphere then acts on these rows. **Heat transport:** above 1 bar, dense air carries heat to the poles, so each row is pulled towards the area-weighted mean by `1 / (1 + 0.1 × (pressure − 1))` (0.45 at 13 bar, 0.1 on Venus at 92 bar); the global mean does not move, and thinner atmospheres keep Earth's contrast. **Star:** the rows are Earth's around the Sun; the host star and the orbit then move them, through the flux the planet gets relative to Earth's (`S`):

- a planet with plate tectonics and liquid water (≥ 0.01 bar) has a carbonate-silicate cycle that regulates its greenhouse: across the habitable zone it stays temperate, from +30 °C at the inner edge through Earth's climate at Earth's place (position 0.07) to −15 °C at the outer edge; beyond the zone it follows the flux like the others;
- any other planet follows the flux: temperatures in kelvin are multiplied by `S^(1/4)` (Mars at 1.52 AU: ×0.81; Venus at 0.72 AU: ×1.18), which also narrows the equator-to-pole difference in degrees around a dim star;
- `[climate] internal_heat` (K) adds the planet's own heat as radiated power, `T = (T⁴ + T_int⁴)^(1/4)`: volcanic worlds (285 K) stay scorching far from their star.

The Sun at 1 AU leaves the rows unchanged, so `earth-like` worlds are exactly as before.

**Tidal locking:** a planet whose tidal locking time (Gladman et al. 1996, about `166 Gyr × a⁶ / M²` for an Earth-like planet) is shorter than its star's age (4.6 Gyr, or the star's whole life if shorter) always shows the same face to its star: the planets in the habitable zone of M dwarfs, and of the coolest K dwarfs. Their temperature follows the angle from the point under the star, at the centre of the map (longitude 180°, on the equator), instead of the latitude: the light is `4 cos θ` times the mean on the day side and none on the night side, turned into °C like the rows (34 °C under the star, −56 °C on the night side around the Sun's light), then evened out by the atmosphere and moved by the star as above. Winds, rain and clouds keep their latitude bands. **Greenhouse:** the preset's `[atmosphere] greenhouse` is added to every pixel (395 °C on `venus-like`, 20 °C on `volcanic`). **Liquid water:** below 0.01 bar (water's triple point is 0.006 bar) moisture is −1 everywhere, so there is no rain, no river and no lake, and erosion is off unless the request or the preset sets it (`mars-like` and `barren` keep `erosion = 1` for valleys carved when they were wetter).

The `[climate]` table of the planet preset then shifts the whole temperature field (for example `+4.5 °C` on caustic worlds, `−10 °C` on `mars-like`, about −50 °C on average with its distance, whose ice thresholds are lowered to `−70 °C` to keep its caps; frozen worlds get their cold from their distance alone). Biomes use annual mean thresholds: sea ice below −13.5 °C, ice cap below −11.6 °C, boreal (tundra, taiga) below 0 °C, temperate below 12 °C and tropical above; highland is snow below 2.6 °C. They were converted from the former `[0, 1]` scale through the same curve, so each sits at the latitude it had.

### Moisture — winds and rain shadows

Moisture comes from winds carrying ocean evaporation over land:

- **Wind belts.** The zonal wind at each latitude is `u = −cos(4 · latitude)`: easterly trade winds near the equator, westerlies at mid-latitudes, polar easterlies. Each row is simulated with the wind blowing east and blowing west, and both results are blended by how much the wind blows each way, so moisture stays continuous across the belts.
- **Transport.** Along a row, air over the sea picks up humidity towards what it can hold (warm air holds more). Over land it rains out steadily, much more where it climbs a slope, and immediately where it cools below what it holds. 60 % of the rain over land evaporates again (soil and plants), so continental interiors are not starved. Mountains therefore get wet windward slopes and dry rain shadows behind them.
- **North-south supply.** Rows with little ocean of their own still receive background humidity, set by the ocean fraction within ±20° of latitude and of the whole planet.
- **Circulation bands.** A `cos(6 · latitude)` term adds the wet equator and 60° belts and the dry 30° desert belts and poles.

The rain rate is smoothed over 100 km, mapped to `[-1, 1]` and combined as `0.75 × rain + 0.25 × band + 0.50 × noise`, then clamped. The `[climate]` table of the planet preset then shifts the whole field (for example `−0.55` on volcanic worlds).

### Clouds

The cloud cover of each pixel, in `[0, 1]`, depends on the atmosphere's cloud kind (`[atmosphere] clouds`):

- **water**: `0.12 × equatorial convergence` (a band about 10° wide) `+ 0.15 × circulation band + 0.25 × rain`, plus polar stratus (`+0.3` towards the poles, where the rain term fades out: cold air rains little but stays overcast), plus `1.5 ×` the cloud systems (4-octave noise) and `0.6 ×` finer cells (3 octaves, five times smaller; stronger in the tropics, where clouds are convective clusters), through `smoothstep(0, 0.9)` so cloud fields have thick cores and thin edges. Clouds gather where air rises (the equatorial convergence and the storm tracks around 60°) and where it rains (windward slopes), and thin out over the subtropical highs. Earth-like worlds come out with a mean cover of 0.42–0.49: about 0.67 at the equator, 0.29 around 30°, 0.45 along the storm tracks, 0.4 at the poles. In the viewer, where thin clouds are translucent, they hide about 30 % of the ground.
- **sulfuric**: a deck in zonal bands, `smoothstep(−0.3, 0.6, 0.2 + 0.7 × cover + noise)`: total on `venus-like`, patchy on `volcanic` worlds.
- **dust**: low, patchy and thin (`0.8 × smoothstep(0.15, 0.6, …)`), about 10 % of `mars-like` worlds.
- **none**: no cloud layer.

`[atmosphere] cloud_cover` shifts the cover (−0.3 on frozen worlds). The viewer draws the layer over the globe and the flat map (**Clouds**, `C`), and can let it drift slowly around the planet (**Drift**, `W`). The noise has features of about 3 000 km. For water clouds it is warped by a larger noise, which curls cloud systems into swirls and fronts, and squeezed north-south only at mid-latitudes (up to ×2 at 45°), where the westerlies draw fronts into bands; tropical and polar clouds are not stretched. Sulfuric decks (×6) and dust (×1.5) are squeezed everywhere but near the poles. Airless worlds have no clouds.

### Regional variability

Before biome selection, both climate fields get a regional variability term standing for local differences of soil, aspect and circulation: `11.25 °C × fbm` on temperature (about 2 °C of standard deviation) and an independent `0.50 × fbm` on moisture (about 0.09), with features from about 1 000 km down to 60 km. Biome borders then become irregular and interleaved instead of following isotherms and isohyets. The term is part of the exported `temperature` and `moisture` layers, so they still explain every biome.

### Rivers and lakes

1. **Depression filling.** Starting from the sea, the terrain is flooded upwards (priority-flood, Barnes et al. 2014) so every land pixel gets a path down to the sea; pits are filled to their spill level. Water crosses the filled flats along a tiny slope whose cost per step varies with a noise, so paths across flats wander instead of running straight to the spill point; real slopes keep their height, so divides and lakes do not move. The flood stays sequential (it pops the pixels one by one in height order); it reads the neighbours by index only, without their distances, and takes about 0.2 s at 1920 × 1080 and 2.4 s at 8192 × 4096. Flooding the land masses in parallel (they are independent, the sea never changes) was tried and gave nothing: one continent usually holds most of the land.
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

- **Craters**: the [impact craters](#impact-craters) too small for the base map (from 4 tile pixels up to its smallest, 4 base pixels) are drawn on each tile from the same crater field, so zooming in reveals smaller and smaller ones. They are added to the relief rather than replacing it (see the craters section), scaled down with their size from the base map's relief at its smallest crater to a tenth of it, since the hillshade's 60-fold exaggeration would make small crater walls vertical. Biomes follow the ground without them: a small crater changes the relief, not the surface it is dug into.

A tile takes about 4 ms on 10 cores; on a heavily cratered world (a 4 Ga `mercury-like`) about 10–12 ms, against 5 ms without craters. A world prepared for tiles keeps its layers as stored in `raw_data/` (16 bits per value, decoded as they are sampled), a byte of lake and river flags and the water depth per pixel, and a byte of albedo: about 23 bytes per base pixel (0.2 GB at 4096 × 2048, 0.8 GB at 8192 × 4096), loaded in about 0.1 s at 4096 × 2048 since the layer files are decoded in parallel. Coast-related biomes (beaches, wetlands) still use the base coast distance, so their 40 km band is smoother than the detailed coastline at the deepest levels.

### Elevation — continents, coasts and mountains

Elevation before erosion is `continent + plates + mountain × mountain_weight × mountain_blend`, `plates` being the relief of plate boundaries (zero without plate tectonics).

- **Continents.** A 5-octave FBM on the unit sphere. Its 2 coarse octaves shape continents and basins; its 3 fine ones add hills. Away from the coast (more than about 0.03–0.2 above or below the waterline) only 30 % of the fine octaves remain, so plains, deserts and the abyssal plains of the sea floor are smooth (visible where seas have dried up); coasts keep them all, and so does high ground, with the mountain weight.
- **Coastlines.** Near the waterline the FBM is sampled at a position displaced sideways by every octave finer than its own, down to about two map pixels, each by half its wavelength and 0.8× less at each finer octave. A low "plate" step is added there too: a Voronoi cell noise (±0.06, borders softened over about 1/20 of a cell) that gives coasts straight stretches and angular corners. Both fade out between 0.02 and 0.08 from the waterline, where they would only wrinkle the relief. Coasts are fractal instead of smooth contours. The finest outline octaves follow the map resolution, so a larger map adds detail to the same continents.
- **Mountains.** A ridged noise on a domain-warped position, with two finer ridged octaves concentrated on its ridges (35 % of its height), weighted from continent height 0.2 upwards.
- **Plates.** Worlds with plate tectonics (`[geology] regime = "plates"`: `terran`, `frozen`, `earth-like`) have 10 to 15 plates made of jagged Voronoi subplates, each turning about an Euler pole at 2 to 10 cm/yr (see `src/plates.rs`). On a fixed 1024 × 512 grid, whatever the map's size, each point gets the relief of its nearest boundary, from the boundary's convergence (full from 5 cm/yr) and the crust on each side (continental above the edge of the shelf, 0.05 under the waterline, from the continents' shape alone), interpolated between the grid's pixels:

  | Boundary                                 | Relief                                                                      |
  | ---------------------------------------- | --------------------------------------------------------------------------- |
  | Continent–continent, closing             | A range on both sides, 350 km wide: 0.12 of uplift and full mountain ridges |
  | Continent over ocean, closing            | A coastal range 150 km inland (0.08) and a volcanic arc 220 km inland       |
  | Ocean under a continent or another ocean | A trench along the boundary, 0.16 deep                                      |
  | Ocean over ocean, closing                | An island arc 120 km from the boundary (0.12)                               |
  | Ocean–ocean, parting                     | A mid-ocean ridge, 450 km wide (0.1)                                        |
  | Continent–continent, parting             | A rift valley and its shoulders                                             |

  Relief scales with gravity as the mountains do, and fades out between 70° and 85° of latitude, where the grid stretches boundaries into streaks. Arcs need subduction more than its speed (full from 2 cm/yr) and carry volcanoes of their own, one per 200 km cell in 80 % of the cells. The noise's mountains keep half their weight: the other ranges follow the boundaries. About 0.13 s per world to build the plates.
- **Volcanoes.** The volcanic noise, above the threshold set by `--volcanic`, outlines broad volcanic provinces. Volcanoes are cones within them: at most one per 900 km cell (half the cells), 50–120 km in radius whatever the planet's size, rising up to `0.25` above the terrain (volcanic islands at sea). The volcanic zone peaks on each cone and falls to 12 % of the province away from it, below the ash threshold, so a province is ordinary land dotted with volcanoes, each with its vent (about 1.2 radii), lava flows (1.8, lobed by a noise) and an ash halo (2.6). Presets that shift the volcanic zone (`volcanic` worlds, +0.5) still cover whole provinces with lava.

### Hydraulic erosion

Rain droplets are dropped at random on land. Each one rolls downhill with a little inertia, picks up sediment while it speeds up on steep ground, and drops it when it slows down or climbs (Beyer, 2015); what it still carries when it reaches the sea is carried offshore. Thousands of droplets carve valleys into the mountains and leave fans at their feet.

Erosion runs in two passes, on grids coarser than the map. The main pass always uses 1024 × 512 cells (or the map size if smaller), whatever the resolution: it carves the valleys and shapes the basins, so a world has the same lakes at 1024 and at 8192 pixels of width (with one grid per resolution, the lake area of a world varied from 0.3 % to 0.6 % of its land between sizes). Neither pass fills the basins of the terrain before erosion (cells more than `0.001` below their spill level): droplets and the blur of flat ground may carve them deeper but never raise them, so lakes survive erosion (it used to leave only 0.01–0.4 % of the land under lakes, against 0.6–1.8 % without erosion; now 0.4–1.4 %, Earth: about 1.5 %). Maps wider than 4096 pixels add a detail pass on a grid of one cell per 4 × 4 pixels (2048 × 1024 at 8192 × 4096) and keep only its change finer than a main-grid cell: gullies on the slopes, without new dams or basins. `--erosion 1` drops one droplet per grid cell. Droplets run in parallel: the grid is cut into tiles at least 104 cells wide (twice the farthest a droplet reaches), run in four checkerboard phases so that tiles running together never touch the same cells; each tile gets droplets in proportion to its land and its own random stream, so a world does not depend on the number of cores.

- **Real slopes.** Beyer's constants assume heights of about one per cell; this terrain is about 100 times flatter. Droplets run on heights exaggerated 10× (on a 1024-cell grid, scaled with the cell size), so their capacity follows the slope instead of its minimum everywhere.
- **Plains stay smooth.** Droplets only carve where the slope (exaggerated, per cell) exceeds 0.04, fully from 0.1. On flatter cells the height change is blurred (three box passes of 2 cells): valleys continue as wide, shallow swales, so they still drain and leave no basin at the foot of the mountains, and fans no longer roughen plains and deserts. Fine-scale roughness on deserts is about 5× lower than with uniform erosion, and mountains are about 3.5× rougher than plains.
- **Coasts keep their profile.** Erosion fades out within 0.05 of the waterline, and the change spread back onto the full-resolution map (cubic B-spline, smooth across cell borders) never moves a pixel across the waterline: land is never dug below the sea and no new land rises from it.
- **Poles.** Erosion fades out between 60° and 75° of latitude: droplets move in pixel steps and the projection stretches pixels east-west towards the poles, which would otherwise carve artificial north-south grooves.
- **Frozen ground.** Ice carves no river valleys: erosion fades out on rows whose sea-level temperature (latitude and the preset's climate shift) stays below the preset's polar threshold, rising to full strength 4.5 °C above it. Frozen worlds keep smooth ice sheets, and only their thawed belts are eroded.

### Impact craters

Craters follow the planet's surface age (`--surface-age`), gravity, atmosphere and wet era (`[geology] wet_until`); the research behind them is in [docs/analysis/08-geology.md](docs/analysis/08-geology.md). See `src/craters.rs`.

- **How many.** The lunar chronology of Neukum gives the density of craters of 1 km or more on a surface `T` Ga old: `N(1) = 5.44e-14 (exp(6.93 T) − 1) + 8.38e-4 T` per km², a steep decline until about 3.5 Ga, then a constant flux (0.063 per km² at 4 Ga). Sizes follow `N(>D) ∝ D⁻²` and the density is capped at 0.115 per km² (about a tenth of geometric saturation, the equilibrium of the lunar highlands). On a Moon-sized 4 Ga surface this gives about 6 000 craters of 20 km or more (5 185 are counted on the Moon). Ages are drawn from the same curve, so most craters of an old surface are very old.
- **Where.** Diameters are split into octaves (`[2^k, 2^(k+1)]` km), each with a grid on the faces of a cube projected on the sphere, every cell holding a random number of craters expected from its area. The grids depend on the seed and the planet, not on the map size. The base map draws the craters of 4 pixels or more (80 km at 1920 pixels on an Earth-sized planet, 32 km on a Mercury-sized one); smaller ones read as dots at that scale.
- **Shape.** Simple craters are bowls `D/5` deep with a rim of `0.036 D`. Above a transition diameter of `15 km × 1.62 / g` (the Moon's 15 km, about 2.5 km at Earth's gravity) they get a flat floor and a central peak, then a peak ring above about seven times the transition, and grow shallower (Pike). Ejecta thin out as `(r/R)⁻³` and stop at 3 radii; rims and ejecta are irregular. Elevations are converted with `ELEVATION_KM` = 25 km per elevation unit, calibrated on Earth-like worlds (median sea floor about 4 km deep, highest peaks 8–10 km).
- **Superposition.** Craters are drawn in age order, oldest first: inside a crater the previous relief is replaced (15 % of it kept), outside, ejecta bury it.
- **Water.** Craters older than the end of the wet era are drawn before the hydraulic erosion, worn by their exposure to water (a 20 km crater keeps `1/e` of its relief after 0.1 Ga at erosion 1, larger ones last longer in proportion), then eroded; younger ones are drawn after it, fresh. A world with liquid water today (0.01 bar or more) is wet to the present, so its craters are worn: Earth-like worlds keep only a few large, faint ones. Craters on the floor of a sea that still fills the basins (the preset's ocean is still water, ice-sealed sea, acid or magma) leave nothing. Craters make closed basins, which the hydrology fills into lakes on wet worlds.
- **Diffusion.** Small impacts slowly smooth every crater (lunar topographic diffusivity, 5.5 m² per Ma): a crater of radius `R` loses `1/e` of its relief once `5.5 m²/Ma × age` reaches 5 % of `R²`, so old surfaces lose their sub-kilometre craters (a 500 m crater is subdued in about 0.6 Ga) while the craters of the base map are untouched.
- **Detail tiles.** Craters too small for the base map appear in the [detail tiles](#detail-tiles--deep-zoom). The grids are searched only around each tile, giving exactly the craters of the whole sphere. These craters are added to the relief instead of replacing it, which keeps neighbouring tiles identical at their borders: at saturation small cavities cover only about 7 % of the ground per octave, so adding them digs no pits.
- **Rays.** Fresh craters are bright: their floor, their ejecta (thinning as `(r/R)⁻³`) and 6 to 14 rays of their own, 8 to 30 crater radii long (Tycho's reach about 35), narrowing with distance. Space weathering darkens them, `1/e` every 250 Ma, so that they are all but gone after 1 Ga, like the Moon's Copernican craters. They are drawn on worlds without liquid water today (air, wind and rain erase them elsewhere), from craters down to one pixel wide, into `raw_data/albedo.png`, a brightness shift that takes the ground part of the way to white (`ALBEDO_GAIN`, 60 % at a shift of 1). The detail tiles read it at the base resolution; at another epoch, the rays are those of the craters fresh then. About 0.1 s at 1920 × 1080 on a 4 Ga `mercury-like` (74 000 craters to look at).
- **Maria.** On presets with a window of maria (`[geology] maria`: `barren` 3.9–3.1 Ga, `mars-like` 3.7–3.0, `mercury-like` 4.0–3.6), lava floods a share of the basins of 100 km or more (and six times the simple-to-complex transition) formed before the window ends: 30 % without volcanism up to 90 % at `--volcanic 1`, each at a date of the window after its basin formed. The lava fills the floor flat out to 0.9 basin radii, up to 40–75 % of the way from the lowest of the floor to the mean of the rim as they are then, and darkens it (`MARE_ALBEDO`, an albedo shift of −0.35). Floods are events among the impacts, in age order: older craters inside are buried, or show only their rims; younger ones and their rays lie over the lava. They are all drawn after the erosion, lava plains wearing little, so that the time slider can show them come and go.
- **Tidally locked planets.** A planet locked to its star keeps one hemisphere facing its orbital motion, the apex, on the equator 90° west of the point under its star (longitude 90° on the map, the substellar point being at 180°), as the Moon's apex lies 90° west of the point under the Earth. That hemisphere sweeps up more impactors: since the locking (`star age − locking time` ago), craters are drawn more densely towards the apex, `(1 + k cos γ) / (1 + k)` of a raised density with `k` for an apex-to-antapex ratio of 1.3 (about 1.14 from one hemisphere to the other; models of the Moon give 1.14 to 1.32, observations more). Older craters, formed while the planet still rotated, are spread evenly; the mean density does not change. Unlocked worlds keep their craters.
- **Air.** A thick atmosphere breaks up small impactors: no crater below `0.1 km × p^0.75` (3 km on Venus), fewer below ten times that.
- **Cost.** Craters are drawn in waves of craters that share no pixel, each crater after every older one it overlaps, so a wave runs in parallel and the result is the same as drawing them one at a time. On a 4 Ga Mercury-sized world, the most cratered of the examples, generations with and without craters take the same time within run-to-run variation (about 1 s at 1920 × 1080, 18 s at 8192 × 4096).

### Epochs — a surface through time

A world without liquid water today can be shown at another epoch: its young impact craters (those of the dry era, drawn after the erosion) are drawn again on `raw_data/elevation_base.png` as they were then, or will be. See `src/epoch.rs` and [docs/analysis/08-geology.md](docs/analysis/08-geology.md).

- **Range.** Epochs are in Ma before present (negative in the future): from 1 Ga ahead back to the surface age, or to the end of the wet era if more recent, since erosion reshaped the craters before (`mercury-like`: −1 000 to 4 000 Ma, `mars-like`: −1 000 to 3 500 Ma). Worlds with liquid water today offer none: their lakes, rivers and erosion would move with the relief, and are not recomputed. Neither do worlds made before format version 7, which have no `elevation_base`. The server lists the range of each world (`epochs_ma` in `GET /api/worlds`).
- **Craters at an epoch.** A crater exists at epoch `t` when it is at least `t` old. It is worn for the time between its formation and `t` (diffusion), so going back in time freshens it. Craters still to come form at today's constant flux (8.38 × 10⁻⁴ craters of 1 km or more per km² and per Ga), from a lattice of their own: the present craters never move. Epoch 0 is the world as generated, within two 16-bit steps of `elevation.png`.
- **What follows.** The biomes, the hillshading and the temperature follow the relief through the lapse rate; moisture and the other layers keep their present values.
- **Serving.** The server computes a world at an epoch once and keeps it in the tile cache like a world of its own (same memory, same budget). It serves `GET /api/worlds/{id}/epoch/{ma}/surface.png`, `elevation.png` and `biomes.png` at the base resolution, optionally reduced (`scale=2` or `4`), and the detail tiles at that epoch (`epoch=` on the tile URL). About 0.17 s to compute an epoch and 0.11 s to render its surface at 1920 × 1080; 2.9 s and 1.8 s at 8192 × 4096.

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
