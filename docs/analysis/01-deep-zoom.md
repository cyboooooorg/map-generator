# Analysis: deep zoom in the viewer

Backlog item 1. Goal: zoom from the whole planet down to a few hundred metres
per screen pixel, on the globe and the flat map, with detail consistent with
the base world and progressive loading.

Measurements below were taken on an Apple M1 Pro (10 cores), release build,
`--planet terran --seed 3` (circumference 41 263 km).

## 1. Constraints found in the current code

### Where detail comes from today

The noise in `Sampler::sample` (`generation.rs`) is continuous on the unit
sphere, but it is **band-limited far above the pixel size**:

| Source                     | Smallest feature (Earth-sized planet)    |
| -------------------------- | ---------------------------------------- |
| Continent fbm, 5 octaves   | lattice of R / 12.8 ≈ 500 km             |
| Ridged mountains, 1 octave | R / 5 ≈ 1 300 km                         |
| Domain warp                | R / 2 ≈ 3 200 km                         |
| Moisture fbm, 4 octaves    | R / 12 ≈ 530 km                          |
| Volcanic fbm, 3 octaves    | R / 4 ≈ 1 600 km                         |
| Erosion delta              | 512 × 256 grid, ≈ 80 km cells, bilinear  |

At 1920 × 1080 (≈ 21 km per pixel) the noise is already oversampled about
20 times. The only pixel-scale features are produced by global passes:
coastline thresholds, the 40 km shore band, rivers and lakes, biome borders.
Consequence: **regenerating at a higher resolution adds no terrain detail**,
only sharper thresholds. Detail below ~20 km must be synthesised.

### Global vs local passes

| Pass           | Scope                                     | On a tile?     |
| -------------- | ----------------------------------------- | -------------- |
| Noise          | per pixel; `(x, y)` tied to the grid size | after refactor |
| Erosion        | global droplets on 512 × 256, upsampled   | interpolate    |
| Temperature    | per pixel: row profile + land lapse rate  | yes            |
| Coast distance | global Dijkstra, capped at 2 000 km       | interpolate    |
| Moisture       | global: winds walk whole rows twice       | interpolate    |
| Hydrology      | global priority-flood, sequential (~1 s)  | no             |
| Biomes         | pure function of 7 inputs + preset remap  | yes            |

"Interpolate" means the field is smooth at the base resolution and can be
sampled between pixels; the coast distance only matters up to the 40 km
shore band.

### Data available after generation

`raw_data/` holds elevation, temperature, moisture, log flow (16-bit) and biome
ids (8-bit, rivers and lakes are ids). Missing for a tile renderer: the
volcanic zone, the coast distance, a lake/river mask separate from the biome.
`WorldMetadata` carries every `WorldParams` field, so a world can be reloaded,
but a deserialised `Preset` has an empty `lookup` (`serde(skip)`): `remap`
would panic until the lookup is rebuilt.

### Measured costs

| Measure                                              | Result               |
| ---------------------------------------------------- | -------------------- |
| Generate 1920 × 1080, with raw data                  | 2.9 s, 0.4 GB, 11 MB |
| Generate 4096 × 2048                                 | 9.9 s, 1.5 GB, 37 MB |
| Generate 8192 × 4096 (server maximum)                | 26 s, 4.2 GB, 124 MB |
| `Sampler::sample`, one thread                        | 260–700 ns/px        |
| One Perlin octave in `fbm`, one thread               | ~25 ns/px            |
| 256² tile: sample + 10 octaves + warp, 10 threads    | 6 ms                 |
| PNG encode of a 256² RGBA tile                       | 0.4–0.6 ms           |
| Decode the 5 raw layers, 1920 / 8192                 | 20 ms / 280 ms       |
| 256² data tile (6 B/px, 393 KB raw), gzip -6         | 105–125 KB           |

Generation sizes are wall time, peak RSS and `raw_data/` size. The tile
benchmark used the full `Sampler` plus 10 detail octaves and two 4-octave
warp fields per pixel, an upper bound for the recipe below.

Target resolution: 300 m per pixel on a 41 000 km planet is a virtual
equirectangular map of ~137 000 × 68 000 pixels; even 8192 px is 5 km/px.

### Viewer

`PlanetView` draws one 720 × 360 plane (0.5° per vertex, ~55 km) morphed in
the vertex shader, sampling one base-resolution data texture and `world.png`
(which has hillshade and contours baked in). The camera itself already gets
close: at the globe `minDistance` 1.15 with a 40° FOV the view spans ~700 km,
about 230–460 m per device pixel; the limit is the data (one texel spans
40–90 screen pixels), the 55 km mesh, and `OrbitControls` whose dolly and
rotate speeds do not scale with altitude. The near plane is fixed at 0.01.

## 2. Options

All options share a tile scheme: equirectangular quadtree, level `z` has
`2^(z+1) × 2^z` tiles of 256² pixels, so the virtual width is `512 · 2^z`.
z = 2 ≈ the default base (2048 px), z = 8 ≈ 315 m/px on Earth size. Columns
wrap in longitude.

### (a) Pre-generated tile pyramid

Either run the pipeline at high resolution, or run a detail synthesiser
(as in b) over every tile after generation.

- **High-resolution pipeline**: impossible past 8192 px (memory grows
  linearly, 4.2 GB at 8192; 137 k px would be ~250× more), and, as shown
  above, it would not add terrain detail anyway.
- **Pre-synthesised pyramid** to z = 8: ~175 000 tiles, ~15 min of all cores,
  ~20 GB per world at 120 KB per tile (still ~8 GB if ocean tiles above z = 5
  are dropped). Same quality as (b), every world pays for places nobody
  visits, and changing the detail algorithm means regenerating everything.
- Useful only for the low levels (z ≤ 4: 682 tiles, a few seconds).

### (b) On-demand server tiles from the base layers (recommended)

A pure function `render_tile(&BaseWorld, TileId) -> Tile` in `mapgen`, served
by `mapgen-server`. Per pixel at unit vector `p`:

1. **Elevation**: bicubic of the exported elevation (wrapping longitude) plus
   band-limited fractal detail. Octaves start at a wavelength of ~2 base
   pixels and stop at the tile's pixel size, so a coarse tile is the low-pass
   of its children (no popping between levels). Amplitude follows the fbm
   1/f law (≈ 0.006 total, so coastlines move by less than a base pixel),
   scaled up on mountains (ridged octaves where the base mountain weight is
   high) and down on plains.
2. **Climate**: moisture and temperature bilinear at a domain-warped position
   (warp amplitude ~1 base pixel) so biome borders become irregular instead
   of interpolated straight edges; temperature gets the lapse-rate
   correction for the new elevation (`climate::temperature`).
3. **Volcanic zone, coast distance**: read from new raw layers (or
   recomputed at load, see below). The shore band stays based on the base
   coast distance: acceptable since the coast moves less than its 40 km width.
4. **Lakes, rivers**: lakes from the biome layer (nearest + warp); rivers from
   the flow layer until item 3 provides vector rivers, which the client then
   draws as geometry and the tile only uses to carve valleys.
5. **Biome**: `preset.remap(rules.choose(input))`, the exact generation code.

- **Quality**: fractal coasts, ridged mountain detail, irregular biome
  borders; rivers stay coarse until item 3.
- **Consistency**: the biome rules run the exact generation code; the base
  level reproduces the base world; each level is the low-pass of the next.
- **Cost**: ~3–8 ms per tile on 10 cores; ~100 tiles fill a 3000 × 2000
  view in under a second.
- **Storage**: none required; ~120 KB per tile over the wire.
- **Cache**: in the browser, `Cache-Control: immutable` on a URL keyed by
  the world revision (`modified_ms`, already in `WorldSummary`) and the tile
  format version; on the server, an LRU of 2 loaded `BaseWorld`s (40 MB at
  1920, ~340 MB at 8192 as u16). A disk cache is unnecessary at 6 ms a tile.
- **Data changes**: `raw_data` format v2 adds `volcanic_zone` and
  `coast_distance` layers (`Option` in `Layers`, recomputed at load for v1
  worlds); `Preset` rebuilds its lookup on deserialisation.
- **Server API**: `GET /api/worlds/{id}/tiles/{rev}/{z}/{x}/{y}`, a binary
  tile sent gzip-compressed.

### (c) Client-side detail in the shader

The fragment shader adds noise octaves to the interpolated elevation and
evaluates the biome rules in GLSL.

- **Quality**: unlimited smooth zoom, no loading. But non-local inputs (coast
  distance, rivers, lakes) cannot be computed per fragment, and relief needs
  a finer mesh anyway.
- **Consistency**: the biome rules and preset remap would exist twice (Rust
  and GLSL); point inspection and the hex export would report base values
  that differ from what is drawn.
- **Cost**: ~40 noise evaluations per fragment (elevation, normal, warp) on a
  6 Mpx retina canvas, every frame. Fine on an M1 Pro, heavy on integrated
  GPUs; float32 noise coordinates stay precise down to ~100 m features.
- **Best use**: a cheap complement below the deepest tile level (micro relief
  in the normal), not the main source of detail.

### (d) Client tiles computed in WebAssembly

The same `render_tile` compiled to WASM and run in a pool of Web Workers,
fed by the base layers the client already decodes.

- Same quality and consistency as (b), works without the server.
- ~1.5–2× slower per core than native, no shared rayon pool: ~40 ms per tile
  per worker, ~150 tiles/s with 6 workers.
- Adds a wasm-bindgen build step to Vite, the Dockerfile and `mise`, and ships
  all base layers to the browser (already the case at 1920, 124 MB at 8192).
- Worth keeping open by making `render_tile` pure (no I/O, no rayon inside).

### Client side, common to b and d

- **Mesh**: replace the single plane by quadtree patches (e.g. 32 × 32
  vertices plus skirts against cracks), each a `uv` rectangle drawn by the
  same morphing vertex shader with `uTileRect` and its tile texture. The
  morph, globe and map keep working unchanged because positions derive from
  `uv`. This is the shared continuous surface renderer items 2 and 3 can
  build on (biome colours from ids and a palette texture, ecotone blending
  in the fragment shader, vector rivers drawn on top).
- **Level selection**: split a node while its texel size, projected at its
  nearest point to the camera, exceeds ~1.5 device pixels, up to z = 8.
  Frustum culling, and horizon culling on the globe. Polar tiles cover little
  ground on the sphere, so they split less by construction.
- **Loading**: a priority queue by screen-space error, 6–8 concurrent
  fetches, `AbortController` when a node leaves the view, parent tile drawn
  with a sub-rectangle until children arrive. GPU LRU of ~256 tiles
  (2 textures per tile: RGBA16F data + R8 biome, ~200 MB worst case; budget
  configurable).
- **Seams**: tile x wraps modulo `2^(z+1)`; the server samples with wrapped
  longitude and a 1-pixel border (258² with overlap) for filtering and
  normals. Poles: rows clamp, the noise is 3D so the detail is continuous.
- **Controls**: dolly and rotate speeds scaled by altitude (`distance − 1` on
  the globe, camera z on the map), zoom towards the cursor, `minDistance`
  down to ~1.01 above the displaced surface (globe) and ~0.02 (map), near
  plane set each frame to ~altitude / 10.

## 3. Recommendation

**Option (b)**: on-demand server tiles computed from the base layers by a
pure Rust function, with a quadtree surface renderer in three.js. It reuses
the biome rules verbatim, needs no storage, costs a few milliseconds per tile
on a local server, and leaves (d) and (c) as later additions.

### Implementation plan

1. `fix(mapgen): rebuild the preset lookup when deserialising` —
   `#[serde(try_from = "PresetFile")]` so no `Preset` can exist without a
   valid lookup; round-trip test through `WorldMetadata`.
2. `refactor(mapgen): sample the noise at a unit vector` —
   `Sampler::sample_dir(p)`, `sample(x, y)` delegates; no output change
   (test: identical world before and after).
3. `feat(mapgen): export volcanic zone and coast distance layers` —
   `raw_data` format v2, optional fields in `Layers`; README table and
   `raw_data` module docs.
4. `feat(mapgen): reload a world from its raw data` — `BaseWorld::load(dir)`
   with u16 layers, `WorldParams` reconstruction, bilinear/bicubic sampling
   with longitude wrap and pole clamp, recomputing missing v2 layers;
   `hex::resample` switches to it.
5. `feat(mapgen): synthesise detail tiles` — `detail` module: `TileId`,
   `Tile` (typed, ts-rs layout), `render_tile`, `Tile::encode`; a CLI
   `tiles` command writing a PNG mosaic of a region for visual tuning.
6. `feat(server): serve detail tiles` — route, `spawn_blocking`, LRU of
   loaded worlds, gzip (`tower-http` `compression-gzip`), immutable caching;
   route table in `app.rs` docs and README.
7. `refactor(web): draw the surface as quadtree patches` — level 0 only,
   biome colour from ids + palette; no other visual change intended
   (contours from `world.png` disappear, the shader already shades relief).
8. `feat(web): load detail tiles progressively` — tile client, priority
   queue, abort, GPU LRU, parent fallback, skirts.
9. `feat(web): fly close to the surface` — altitude-scaled controls,
   dynamic near plane, zoom to cursor; picking reads the drawn tile so the
   info panel matches the screen.
10. `docs: document deep zoom` — README, `web/README.md`, backlog status.

Effort: about 6–9 days. Steps 1–6 (Rust) ~3 days, 7–9 (renderer and
controls) ~3–5 days, tuning the detail spectrum ~1 day.

### Risks

- **Detail tuning**: too much amplitude moves coastlines and creates islands
  absent from the base map; too little looks blurry. Mitigate with the
  amplitude cap and the agreement test below.
- **Rivers** remain blocky until item 3; the tile layout must leave room for a
  carved-valley input and the renderer for line geometry.
- **Coordination with items 2 and 3**: all three touch the fragment shader.
  Land step 7 first so the others build on the patch renderer.
- **GPU memory and draw calls** on weak machines: cap tiles and patch count.
- **CPU contention** with a running generation (both use all cores): tiles
  stay responsive at a few ms, but give the tile pool fewer threads.
- `OrbitControls` may not be enough near the surface; a small custom control
  may be needed.

### What to test

- Rust: `render_tile` is deterministic; a mosaic of z = 2 tiles downsampled to
  the base size matches the base biome layer on ≥ 95 % of pixels and the
  land/water mask on ≥ 99 %; children averaged equal their parent within a
  tolerance (low-pass property); tiles at x = 0 and x = max are continuous
  across longitude 0; v1 worlds load with recomputed layers.
- Server: route validation (z, x, y bounds, unknown world, stale revision),
  `Cache-Control` and gzip headers, concurrent requests.
- Web: unit tests for level selection, wrap, and LRU eviction; manual checks
  on both views at the poles, at longitude 0, during the morph, and that the
  page stays at 60 fps while tiles stream in.
