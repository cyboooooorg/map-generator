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
- **Grid** shows a 30° graticule.
- Drag to rotate the globe or pan the map, scroll to zoom.
- Click a point to see its coordinates, biome, elevation above the coastline, temperature, moisture and river flow.

**New world** opens the generation form. Each parameter is either random (left to the planet preset, then to the generator defaults) or fixed with its slider. Progress is streamed live from the server, and the new world opens when it is ready.

For everyday use, run the whole app in Docker instead: see "Web app" in the repository README.

## Checks

```bash
mise run web:check   # svelte-check (strict TypeScript) and the unit tests
mise run web:build   # production build into web/dist/
```

## Configuration

There is nothing to configure here: the server address is fixed in `vite.config.ts` (development) and the page is served by `mapgen-server` in production.

## Code layout

| Path                    | Content                                                                        |
| ----------------------- | ------------------------------------------------------------------------------ |
| `src/lib/api/client.ts` | Typed API calls and the job progress stream                                    |
| `src/lib/api/types/`    | API types generated from the Rust server by `cargo test` (ts-rs): do not edit  |
| `src/lib/geo.ts`        | Coordinate conventions (texture, globe, flat map) and ray intersections        |
| `src/lib/layers/`       | 16-bit layer decoding in a Web Worker (browsers decode PNGs to 8 bits)         |
| `src/lib/viewer/`       | three.js view: one mesh morphing between plane and sphere in its vertex shader |
| `src/components/`       | Svelte components: viewer, world list, generation form, point information      |

TypeScript runs in its strictest settings (`strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`), and `npm run check` fails on any warning.
