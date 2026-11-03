# Planet viewer (web front-end)

Browser front-end of the map generator: a library of generated worlds, a 3D globe and a flat map that morph into each other, point inspection, measurements and side-by-side comparison, and a form that queues new generations on the server.

## Prerequisites

- Node 24 or later (`mise install` at the repository root installs the pinned version).
- The `mapgen-server` API, from `crates/server/`.

## Installation

```bash
mise run web:install
```

## Usage

Development, with hot reload: run the server and the Vite dev server together, then open <http://localhost:5173>. Vite forwards `/api`, `/worlds` and `/healthz` to the server on port 3000.

```bash
mise run dev
```

In the viewer:

- **Globe / Map** switches between the globe and the flat map, with an animated morph.
- Drag to rotate the globe or pan the map, scroll to zoom towards the point under the cursor. Zooming goes from the whole planet down to about 300 m per pixel: on the biome layer, detail tiles rendered by the server stream in as you get closer (the coarser picture stays until they arrive, and a tile with nothing loaded above it also fetches its ancestor two levels up, so fast zooming shows something quickly). A tile that fails to load is retried after 2 s, then less and less often (up to once a minute); one the server refuses (a 4xx error) is not retried. On the globe, rotation slows down near the surface.
- Hovering shows the coordinates, biome and elevation under the pointer at the bottom of a wide viewer.
- Click a point to see its coordinates, biome, elevation above the coastline, annual mean temperature in °C (worlds of format version 3 and older stored a `[0, 1]` scale, converted on load with the generator's Earth-fitted curve), moisture, drainage area and, on worlds with `raw_data` format version 2 or later, the distance to the sea, the volcanic zone, the floodplain weight and the shift of the mountain line. Near a river it also shows the river (flow there and at its mouth, length, where it ends, estuary or delta branch); with the hex grid on, the hex under the point. **Copy** copies the latitude and longitude in degrees.
- The tools in the top-left corner: **Reset view** (whole planet, north up), **Measure** (click points to measure a path of great-circle legs, in km on this planet), **Screenshot** (the view as a PNG) and **?** (the keyboard shortcuts).
- **Display** (folded by default on narrow screens):
  - **Layer**: biomes (the generated map), or elevation, temperature (−30 °C to 40 °C), moisture, river flow, volcanic zone, distance to the sea and floodplains coloured on the GPU, with a legend. The last three need a world of format version 2 (3 for floodplains). On the biome layer, the legend lists the biomes of the planet with their share of its surface, in their surface colours (classification colours while the hex grid is shown): hover one to highlight it on the planet, click it to keep it highlighted.
  - **Relief**: raises land above the coastline and shades its slopes; seas stay flat, except on worlds whose seas have dried up (barren), where basins sink below the old coastline and are shaded like land. The sun glints only on liquid seas, as a broad, faint sheen, and barely on magma. The biome layer and the detail tiles are hillshaded already, so on them it only raises the land.
  - **Sunlight**: lights the planet from a sun whose latitude follows the season and the world's axial tilt, with day, night and a terminator on both the globe and the map. Lava and magma glow on the night side (on the biome layer and the detail tiles, not on the hex grid); they are told by their surface colour, red with almost no green. **Play** runs through a year in 12 seconds; the latitude under the sun is shown below the slider. Off, a headlight follows the camera.
  - **Atmosphere**: the world's sky, from its pressure and its preset's `[atmosphere]` table: single scattering of sunlight (Rayleigh for the gas, Mie for the haze) through a shell around the globe, which draws the halo, the twilight band at the terminator and the veil over distant and grazing ground (an opaque atmosphere such as Venus's hides the surface). The shell is far thicker than a real atmosphere so the sky reads at planet scale, and rises with the **Relief** exaggeration (keeping the same optical depth) so displaced mountains stay under the sky and hide the halo behind them; airless worlds have none, and worlds made before atmospheres get Earth's. **Clouds**: the world's cloud layer (`raw_data/clouds.png`, worlds of format version 5 with clouds), on the globe and the flat map: thin clouds are translucent veils, thick ones hide the ground, tinted by kind (white water, pale yellow sulfuric acid, ochre dust), lit by the sun and casting shadows on the globe; they fade out when zooming close to the ground. **Drift** (under Clouds, off by default) turns the cloud layer slowly eastwards, once around the planet in five minutes; while it drifts the viewer redraws ten times a second instead of only when something moves, and stops when the clouds are hidden or faded out. **Grid**: 30° graticule. **Tropics**: the equator, the tropics and the polar circles of the world's axial tilt, dashed in the colours of `world.png`, 3 screen pixels wide with a dark edge at every zoom, drawn over the clouds and the detail tiles. **Depth contours**: the depth contours (isobaths) under water at the levels of `world.png`, in pale blue. **Elevation contours**: contour lines of elevation on land, in topographic brown, every 0.05 elevation units above the coastline, every fifth one stronger (index contours). Contours are drawn about one screen pixel wide at every zoom, detail tiles included, from the elevation data, so the world's data must be loaded. All three are on by default. Worlds of format version 6 and later show `raw_data/surface.png`, which has no lines, so they can be hidden; older worlds show `world.png`, whose tropics and depth contours are baked in.
  - **Hexes**: the hex grid of the world (majority biome per hex, outlines, a blue dot on river hexes), computed by the server for the chosen number of columns and river threshold. **Download hex JSON** saves the grid shown. While a generation runs the server refuses hex grids: the viewer shows its message, and moving a slider or turning the grid off and on again once the generation ends asks again.
- The display settings (except the hex grid itself) are remembered by the browser.

Keyboard shortcuts, when no form field has the focus:

| Key       | Action                                |
| --------- | ------------------------------------- |
| `G` / `M` | globe / flat map                      |
| `1` – `8` | layer                                 |
| `R`       | reset the view                        |
| `H`       | hexes                                 |
| `L`       | sunlight                              |
| `S`       | play the seasons (with sunlight)      |
| `A`       | atmosphere                            |
| `C`       | clouds                                |
| `W`       | cloud drift (with clouds)             |
| `N`       | grid                                  |
| `T`       | tropics and polar circles             |
| `K`       | depth contours                        |
| `J`       | elevation contours                    |
| `D`       | measure distances                     |
| `P`       | screenshot                            |
| `Esc`     | close the point panel, stop measuring |
| `?`       | shortcut help                         |

In the sidebar:

- The selected world's card shows every parameter, its gravity, its atmosphere (surface pressure and cloud kind; worlds made before atmospheres show Earth's 1 bar), surface, land and water shares, river count and age, and how much of the planet each biome covers. Its actions: **Vary…** opens the form filled with the world's parameters, **Copy command** copies the `mise run gen` command that regenerates it, **Copy link** copies a link that opens it, **Compare with** shows a second world beside it, and **Download** offers the map PNG, the SVG, the legend and a ZIP of `raw_data/` built in the browser.
- The world list can be searched by id or seed, filtered by planet and sorted; the sort and filter are remembered. **Select** picks several worlds, to delete them or to compare two of them.
- Deleting a world hides it at once and offers **Undo** for six seconds; the files are deleted afterwards (or when the page closes).
- The address names the worlds shown (`#world=<id>&compare=<id>`): reloading keeps them, and the link can be shared.

**New world** opens the generation form (sea level or land share, volcanism, circumference, axial tilt, erosion and surface pressure). Each parameter is either random (left to the planet preset, then to the generator defaults) or fixed with its slider; **Roll** draws a seed. A world is named after its planet and seed, and the server refuses to overwrite one: when the planet and seed match an existing world, the form says so and offers to replace it. **Generate** sends the job and closes the form: generations run in the background, one at a time, the others waiting in a queue. The **Generations** list shows each one with its stage, elapsed time, an estimate of the time left (learnt from earlier generations in this browser) and **Cancel**. A reloaded page picks up the generations still running. When one finishes, the new world opens if you have not picked another one meanwhile; otherwise a notification offers to open it. A world replaced in place reloads in the panes that show it: its map URLs carry the world revision (`?rev=`), and its raw data is revalidated with the server, so the browser never shows files of the replaced world.

**Compare** shows two worlds side by side (stacked on narrow screens). **Link cameras** moves both views together; **Swap** exchanges them.

On narrow screens (760 px wide or less), the world list and the viewer take the whole screen one at a time: **Worlds** and **Viewer** in the header switch between them. Picking a world opens the viewer, and **New world** opens the form, whose buttons stay pinned to the bottom of the screen while the parameters scroll.

The viewer draws a frame only when something changes (the camera and its damping, the morph between globe and map, a loaded tile, a display setting, a resize), so an idle page costs no GPU time. A decoded world takes about 11 bytes per pixel in the page (19 with the layers of format version 2 and later), about 0.6 GB for 8192 × 4096. Worlds wider than the GPU's texture limit (8192 × 4096 on a GPU limited to 4096 pixels) are shown at a lower resolution; the point readout still reads every pixel.

For everyday use, run the whole app in Docker instead: see "Web app" in the repository README.

## Checks

```bash
mise run web:check   # svelte-check (strict TypeScript) and the unit tests
mise run web:build   # production build into web/dist/
```

## Configuration

There is nothing to configure here: the server address is fixed in `vite.config.ts` (development) and the page is served by `mapgen-server` in production.

## Code layout

| Path                                      | Content                                                                                                                                                                                                                                                             |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib/api/client.ts`                   | Typed API calls and the job progress stream                                                                                                                                                                                                                         |
| `src/lib/api/types/`                      | API types generated from the Rust server by `cargo test` (ts-rs): do not edit                                                                                                                                                                                       |
| `src/lib/geo.ts`                          | Coordinate conventions (texture, globe, flat map) and ray intersections                                                                                                                                                                                             |
| `src/lib/layers/`                         | 16-bit layer decoding and biome statistics in a Web Worker (browsers decode PNGs to 8 bits); layers stay packed as half floats, as the GPU takes them, and are converted on read                                                                                    |
| `src/lib/viewer/`                         | three.js view (`PlanetView.ts`): one mesh morphing between plane and sphere in its vertex shader, drawn on demand; GLSL in `shaders.ts`, colour ramps shared by the shader and the legend in `colormaps.ts`, textures fitted to the GPU size limit in `textures.ts` |
| `src/lib/viewer/TileLayer.ts`, `tiles.ts` | Deep zoom: quadtree level selection (screen-space error, frustum and horizon culling), tile loading (priority queue, coarse fallbacks, abort with hysteresis, retry backoff, GPU cache) and detail patches                                                          |
| `src/lib/world.svelte.ts`                 | Loading of one shown world: metadata, decoded layers, biome statistics (`stats.ts`) and the river index (`rivers.ts`)                                                                                                                                               |
| `src/lib/jobs.svelte.ts`                  | Generation jobs followed by the page: progress, cancellation, time estimates, jobs picked up after a reload                                                                                                                                                         |
| `src/lib/sample.ts`, `hex.ts`             | Values of a world, a river and a hex at one point                                                                                                                                                                                                                   |
| `src/lib/format.ts`, `download.ts`        | Number and date formatting, the request and command that reproduce a world, file downloads and the raw data ZIP                                                                                                                                                     |
| `src/lib/settings.ts`                     | Preferences in `localStorage`, guarded so the app works without storage                                                                                                                                                                                             |
| `src/components/`                         | Svelte components: world pane (viewer, point information, pointer readout, measurement), world card, world list, generation jobs, generation form, notifications                                                                                                    |

TypeScript runs in its strictest settings (`strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`), and `npm run check` fails on any warning.
