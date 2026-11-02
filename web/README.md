# Planet viewer (web front-end)

Browser front-end of the map generator: a library of generated worlds, a 3D globe and a flat map that morph into each other, point inspection, and a form that launches new generations on the server.

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
- Drag to rotate the globe or pan the map, scroll to zoom towards the point under the cursor. Zooming goes from the whole planet down to about 300 m per pixel: on the biome layer, detail tiles rendered by the server stream in as you get closer (the coarser picture stays until they arrive). On the globe, rotation slows down near the surface.
- Click a point to see its coordinates, biome, elevation above the coastline, temperature, moisture and river flow.
- **Display** (folded by default on narrow screens):
  - **Layer**: biomes (the generated map), or elevation, temperature, moisture and river flow coloured on the GPU, with a legend.
  - **Relief**: raises land above the coastline and shades its slopes; seas stay flat. The biome layer and the detail tiles are hillshaded already, so on them it only raises the land.
  - **Sunlight**: lights the planet from a sun whose latitude follows the season and the world's axial tilt, with day, night and a terminator on both the globe and the map. Off, a headlight follows the camera.
  - **Atmosphere**: blue rim around the globe. **Grid**: 30° graticule.
  - **Hexes**: the hex grid of the world (majority biome per hex, outlines, a blue dot on river hexes), computed by the server for the chosen number of columns.

**New world** opens the generation form. Each parameter is either random (left to the planet preset, then to the generator defaults) or fixed with its slider. Progress is streamed live from the server, and the new world opens when it is ready.

On narrow screens (760 px wide or less), the world list and the viewer take the whole screen one at a time: **Worlds** and **Viewer** in the header switch between them. Picking a world opens the viewer, and **New world** opens the form, whose progress bar and **Close** / **Generate** buttons stay pinned to the bottom of the screen while the parameters scroll. A generation keeps running if you switch to the viewer meanwhile, and the new world opens when it is ready.

For everyday use, run the whole app in Docker instead: see "Web app" in the repository README.

## Checks

```bash
mise run web:check   # svelte-check (strict TypeScript) and the unit tests
mise run web:build   # production build into web/dist/
```

## Configuration

There is nothing to configure here: the server address is fixed in `vite.config.ts` (development) and the page is served by `mapgen-server` in production.

## Code layout

| Path                                      | Content                                                                                                                                                                                    |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/lib/api/client.ts`                   | Typed API calls and the job progress stream                                                                                                                                                |
| `src/lib/api/types/`                      | API types generated from the Rust server by `cargo test` (ts-rs): do not edit                                                                                                              |
| `src/lib/geo.ts`                          | Coordinate conventions (texture, globe, flat map) and ray intersections                                                                                                                    |
| `src/lib/layers/`                         | 16-bit layer decoding in a Web Worker (browsers decode PNGs to 8 bits)                                                                                                                     |
| `src/lib/viewer/`                         | three.js view (`PlanetView.ts`): one mesh morphing between plane and sphere in its vertex shader; GLSL in `shaders.ts`, colour ramps shared by the shader and the legend in `colormaps.ts` |
| `src/lib/viewer/TileLayer.ts`, `tiles.ts` | Deep zoom: quadtree level selection (screen-space error, frustum and horizon culling), tile loading (priority queue, abort, GPU cache) and detail patches                                  |
| `src/components/`                         | Svelte components: viewer, world list, generation form, point information                                                                                                                  |

TypeScript runs in its strictest settings (`strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`), and `npm run check` fails on any warning.
