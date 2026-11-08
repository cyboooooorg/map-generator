# Normal-Map Relief Lighting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Light the relief of every 3D view (planet view, detail tiles, crater epochs, sky bodies, star view) with a normal map instead of the hillshade baked into `raw_data/surface.png`.

**Architecture:** The generator renders each pixel once and finishes it twice: with the hillshade for the flat images (`world.png`, thumbnails, drift previews) and without it for a new `colour.png`. It also writes `normal.png` (tangent-space normals from the elevation). Tiles and epochs come from the server in the same two forms. The viewer's shaders read colour and normals and light them with the sun or the headlight; lava and magma add their glow after the lighting.

**Tech Stack:** Rust (crates/mapgen, crates/server, axum, image, rayon), TypeScript/Svelte 5, three.js 0.187 (GLSL in `web/src/lib/viewer/shaders.ts`).

**Spec:** `docs/superpowers/specs/2026-11-08-normal-map-relief-design.md`

## Global Constraints

- Pure normal map; no hybrid with the fixed north-west light; no cast shadows.
- No legacy path: no code reading `surface.png`, no fallback for worlds without the new layers; such worlds are refused by `MIN_FORMAT_VERSION`.
- Flat images keep their exact current pixels: `world.png`, `thumb.png`, legend, SVG, drift previews.
- Relief exaggeration = the hillshade's: `relief_km(circumference_km)` (`RELIEF_KM` 1500).
- Water (`Biome::is_water`, which includes SeaIce, MagmaSea, frozen lakes) is flat: normal (0, 0, 1). Lava (`is_lava`: Volcano, LavaField) slopes scaled by `LAVA_SHADE` (0.4).
- Normal encoding: RGB8, channel = round((n + 1) / 2 · 255), order east, north, up.
- Lava/magma glow is added after the relief lighting and is never darkened by it; night glow as today (`LAVA_GLOW` 0.9); on the day side a floor of the lava's own light (0.25 of its colour) keeps a shaded flow from going dark.
- Flat ground lit at the zenith = today's flat ground brightness; night side follows `smoothstep(-0.08, 0.15, h) · (0.55 + 0.45 · clamp(h, 0, 1))`; slopes facing away from a low light keep at least 0.12 of the flat light.
- Format: `FORMAT_VERSION` and `MIN_FORMAT_VERSION` both go to the next number after main's (expected 16: #290 = 14, #289 = 15); `web/src/lib/formatVersion.ts` follows (a server test checks it).
- `RENDER_VERSION` (detail.rs) and `TILE_RENDER_VERSION` (TileLayer.ts) both go from 5 to 6.
- Shared machine: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`; never `npm ci` in the worktree; never port 3000.
- Commits: Conventional Commits; PR description lists `Closes #N`/`Refs #N` for issues it resolves (CLAUDE.md, #338).
- Docs in the same change: README (raw_data table, file tree, tile and epoch routes, lighting), `web/README.md` (Relief, Sunlight, Night side), `docs/system-spec.md:489-491`, the `FORMAT_VERSION` doc comment, and the `docs/analysis/*` lines naming `surface.png` (08-geology 419, 436, 541, 803; 10-stars 217; 11-moons 286, 301; 12-gas-giants 471; 15-storage 291).

## Review Focus

1. **Longitude seam and poles:** normals at column 0 / width−1 must match the wrap (central differences wrap), and polar rows must stay bounded (east differences use `Grid::dx_km`, floored at 0.1 of the equator). Test in Task 1.
2. **Tile borders:** the normals of two neighbouring tiles must agree along their shared edge (same bordered elevation as the hillshade used). Test in Task 3.
3. **Lava on a slope turned away from the light, and at night:** it must glow, not go black. Pinned by the shader constants of Task 6 and checked by hand in Task 9.
4. **The flat map (morph = 0) and the headlight mode:** east = +x, north = +y on the map; with the sun off the relief is lit by `uLightDir`. Checked by hand in Task 9.
5. **Giants:** no relief; `normal.png` flat; the viewer has one path. Test in Task 2.

---

### Task 0: Rebase on main

- [ ] **Step 1:** `git fetch origin && git rebase origin/main` in `~/Developer/cyboooooorg/mg-normal` (main must contain #290; #289 if merged). Resolve conflicts in the spec/plan only.
- [ ] **Step 2:** `cargo build -j 4 -p mapgen` passes. Read the current `FORMAT_VERSION`/`MIN_FORMAT_VERSION` in `crates/mapgen/src/export/raw_data.rs`; the new value is `FORMAT_VERSION + 1` (write it into this plan's Global Constraints if not 16).

### Task 1: Relief normals and the unshaded render (mapgen)

**Files:**
- Create: `crates/mapgen/src/export/relief.rs` (normals; one responsibility)
- Modify: `crates/mapgen/src/export/mod.rs` (`render_pixels`, `hillshade`, `export_world`), `crates/mapgen/src/drift.rs:91`, `crates/server/src/tiles.rs:360-376` (callers)

**Interfaces:**
- Produces:
  - `pub fn relief_normal(world: &World, grid: &Grid, x: u32, y: u32) -> [f32; 3]` — unit normal (east, north, up) from the same central differences and exaggeration as today's `hillshade`.
  - `pub fn relief_normals(world: &World) -> Vec<[u8; 3]>` — encoded per pixel; (128, 128, 255) on water; lava slopes scaled by `LAVA_SHADE` before normalising.
  - `pub fn encode_normal(n: [f32; 3]) -> [u8; 3]`.
  - `pub struct Rendered { pub shaded: Vec<[u8; 3]>, pub colour: Vec<[u8; 3]> }` and `pub fn render_both(world: &World, layers: &MapLayers) -> Rendered` (Detailed style): one pass per pixel computing everything up to the hillshade once, then finishing twice (shade factor f, and f = 1).
  - `render_pixels(world, layers, style)` keeps its signature and output (it returns `render_both(..).shaded` for `Detailed`).
  - `hillshade` becomes `shade(dot(relief_normal, LIGHT) / LIGHT[2])` — same values as today.

- [ ] **Step 1: Write failing tests in `relief.rs`**
  - `flat_world_has_up_normals`: a world whose elevation is constant → every `relief_normals` pixel == `[128, 128, 255]`.
  - `east_slope_tilts_west`: elevation rising eastward (`e = k·x`) on land → decoded normal has `n.x < 0`, `n.y ≈ 0` (|n.y| < 0.01), `n.z > 0`, unit length within 1e-3.
  - `normals_wrap_at_the_seam`: elevation `sin(2π·x/w)` → normal at x = 0 equals the one computed with the wrapped neighbours (compare with x = w on a doubled grid, or assert |n(0) − n(w−1)| small and symmetric).
  - `polar_rows_stay_bounded`: any world → every normal's z ≥ 0.05 on rows 0 and h−1.
  - `water_is_flat`: a pixel with a water biome → `[128, 128, 255]`.
- [ ] **Step 2: Write failing tests in `export/mod.rs`**
  - `render_both_keeps_the_shaded_render`: on a small generated terran world, `render_both(..).shaded == ` the output of the pre-change `render_pixels` (keep a copy of the old function as `#[cfg(test)] fn render_pixels_reference` for this test only, deleted at the end of the task once the test passes and is replaced by a golden hash of `world.png` for one seed: `shaded_render_is_unchanged` asserting a fixed hash computed on `main`).
  - `colour_equals_shaded_where_flat`: on water pixels `colour == shaded`.
  - `hillshade_is_unchanged`: `hillshade(x, y)` equals the old formula on 100 sampled pixels.
- [ ] **Step 3:** Run `RAYON_NUM_THREADS=4 cargo test -j 4 -p mapgen --lib relief render_both hillshade -- --test-threads=3` → FAIL (not defined).
- [ ] **Step 4:** Implement the interfaces above.
- [ ] **Step 5:** Run the same tests → PASS; run the whole mapgen suite (throttled) → PASS.
- [ ] **Step 6:** Commit `feat(mapgen): render the surface without hillshade and its relief normals`.

### Task 2: raw_data layers, format version, giants (mapgen)

**Files:**
- Modify: `crates/mapgen/src/export/raw_data.rs` (module table 7-17, `Layers` 152-197, `export_raw_data` 298-302 and 437-463, metadata 478-527, `FORMAT_VERSION`/`MIN_FORMAT_VERSION` + doc), `crates/mapgen/src/export/mod.rs` (`export_world` 58-113), `crates/mapgen/src/giant_world.rs:45-46` (doc), `web/src/lib/api/types/Layers.ts` (regenerated by ts-rs), `web/src/lib/formatVersion.ts`
- Delete: the `format10` fixture and `crates/mapgen/tests/legacy_format.rs` if #290 has not already removed them.

**Interfaces:**
- Consumes: `render_both`, `relief_normals` (Task 1).
- Produces:
  - `Layers` fields `colour: Option<ImageLayerInfo>` (file `colour.png`, encoding `rgb8`), `normal: Option<ImageLayerInfo>` (file `normal.png`, encoding `rgb8-normal`), `colour_small`, `normal_small` (files `colour_small.png`, `normal_small.png`, 512 × 256, encodings as above); the `surface` field is removed.
  - `export_raw_data(world: &World, dir: &Path, maps: Option<&SurfaceMaps>) -> Result<(), Box<dyn Error>>` with `pub struct SurfaceMaps<'a> { pub colour: &'a [[u8; 3]], pub normal: &'a [[u8; 3]] }`.
  - Giants: `colour` = `world.surface`, `normal` = all `[128, 128, 255]`.
  - Small copies: box-average downscale of colour; normals downscaled by averaging the decoded vectors and renormalising, then re-encoded.

- [ ] **Step 1: Failing tests** (in `export/mod.rs` tests, beside `moons_are_generated_as_bodies_of_their_own`)
  - `exports_colour_and_normal_layers`: export a small terran world → `raw_data/colour.png` and `normal.png` exist at the map size, `colour_small.png`/`normal_small.png` at 512 × 256, `surface.png` does not exist, `world.json` `layers.colour.file == "colour.png"`, `layers.normal.encoding == "rgb8-normal"`.
  - `giant_normals_are_flat`: export a gas giant → every `normal.png` pixel is (128, 128, 255) and `colour.png` equals `world.surface`.
  - `refuses_the_previous_format`: a `world.json` with `format_version = FORMAT_VERSION - 1` is refused by `load_layers`.
- [ ] **Step 2:** Run them → FAIL.
- [ ] **Step 3:** Implement; bump both versions; extend the doc comment ("Version N replaces `surface.png` with `colour.png` and `normal.png` (and their small copies): the relief is lit by the viewer"); update `formatVersion.ts`.
- [ ] **Step 4:** Regenerate TS bindings: `RAYON_NUM_THREADS=4 cargo test -j 4 -p mapgen --features ts -- --test-threads=3` and check `git diff web/src/lib/api/types` shows `colour`, `normal`, `colour_small`, `normal_small` and no `surface`.
- [ ] **Step 5:** Run the mapgen and server suites (throttled) → PASS (the server's format-version test included).
- [ ] **Step 6:** Commit `feat(mapgen)!: store the colour and the normals instead of surface.png` with a `BREAKING CHANGE:` footer (older worlds are refused).

### Task 3: Detail tiles in two halves (mapgen + server)

**Files:**
- Modify: `crates/mapgen/src/detail.rs` (`RENDER_VERSION` 57, `render_tile` 542-642, private `hillshade` 645-663, tests 896-991), `crates/server/src/tiles.rs:383-390` (`render_png`), `web/src/lib/viewer/TileLayer.ts:75` (`TILE_RENDER_VERSION`)

**Interfaces:**
- Produces: `render_tile(&self, tile: TileId) -> RgbImage` returns **256 × 512**: rows 0-255 the colour without hillshade, rows 256-511 the encoded normals (water flat, lava scaled by `LAVA_SHADE`), from the bordered tile elevation already built (`tile_elevation`, `px + 1, py + 1`), with the tile's own `dx_km = pixel_km · cos(lat).max(0.05)`.
- `RENDER_VERSION = 6`, `TILE_RENDER_VERSION = 6` (the server test `viewer_and_server_agree_on_the_tile_renderer_version` checks they match).

- [ ] **Step 1: Failing tests** (detail.rs)
  - `tiles_stack_colour_over_normals`: `render_tile(..)` is 256 × 512.
  - `tile_normals_agree_across_borders`: for two horizontally adjacent tiles at level 3, the normals of the last column of the left tile and the first column of the right tile differ by at most 2 per channel on average (the same elevation field, sampled a pixel apart).
  - Update `rendering_is_deterministic`, `coarse_tiles_agree_with_the_base_world` (compare the colour half with `colour.png`'s downsample instead of `world.png`), `longitude_seam_is_continuous` (both halves).
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** Run detail and server tests → PASS.
- [ ] **Step 5:** Commit `feat(tiles): serve the colour and the normals of a tile together`.

### Task 4: Epoch layers (server)

**Files:**
- Modify: `crates/server/src/tiles.rs:300-380` (`EpochLayer`, `from_file`, `epoch_png`), `crates/server/src/app.rs:1046-1105` (`body_epoch_layer`, 404 message), `app.rs:3-24` (route table doc), `app.rs:1665-1740` (`serves_bodies_at_other_epochs`), `web/src/lib/epochs.ts:72-82` + `epochs.test.ts:56-57`

**Interfaces:**
- Consumes: `render_both`, `relief_normals` on the decoded epoch world (`w.to_world()`).
- Produces: `pub enum EpochLayer { Colour, Normal, Elevation, Biomes }`; files `colour.png`, `normal.png`, `elevation.png`, `biomes.png`; `scale` 1/2/4 as today (normals box-averaged as vectors and renormalised); `Colour` and `Normal` share the `epoch_surfaces` semaphore. `epochFile(id, ma, file: 'colour.png' | 'normal.png' | 'elevation.png' | 'biomes.png', revision, scale)`.

- [ ] **Step 1: Failing tests:** in `serves_bodies_at_other_epochs` loop over `["colour.png", "normal.png", "elevation.png", "biomes.png"]` (64 × 32 at `scale=4`), and `surface.png` → 404 with the message listing the four files; `epochs.test.ts` expects `…/epoch/2000/colour.png?rev=17&render=6&scale=4`.
- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement. **Step 4:** `RAYON_NUM_THREADS=4 cargo test -j 4 -p mapgen-server -- --test-threads=3` and `cd web && npx vitest run src/lib/epochs.test.ts` → PASS.
- [ ] **Step 5:** Commit `feat(server): serve epoch colour and normals instead of surface.png`.

### Task 5: Planet view shader and textures (viewer)

**Files:**
- Modify: `web/src/lib/viewer/shaders.ts` (`surfaceFragment` 202-722: uniforms, surface read 521-523, lava glow 605-617, relief 624-642, lighting 644-685), `web/src/lib/viewer/PlanetView.ts` (`this.u` 516-555, `setTexture` 1551-1575, `disposeTexture` 1578, patch materials 979-1000), `web/src/lib/viewer/TileLayer.ts` (`place` 416-436)

**Interfaces:**
- Produces:
  - `PlanetView.setMaps(colourUrl: string, normalUrl: string, signal?: AbortSignal, onProgress?: (share: number) => void): Promise<void>` replacing `setTexture`; loads both (colour sRGB, normal `NoColorSpace`, both `RepeatWrapping`, no mipmap bleed concerns on the base map) and swaps them in together.
  - Uniform `uNormalMap: sampler2D` beside `uMap`. Tiles: the colour is read from the top half of `uTile` (`v · 0.5`), the normals from the bottom half (`0.5 + v · 0.5`), with `uTexRect` applied inside each half; clamp the v coordinate half a texel away from 0.5 in each half so mipmaps do not bleed across.
  - GLSL `vec3 reliefNormal(vec3 tangentNormal)` = `normalize(t.x · vEast + t.y · vNorth + t.z · vNormal)`, used on every layer (the elevation-tilt block 624-642 is deleted).
  - Lighting: `light = uLightMode == 1 ? uSunDir : uLightDir`; `h = dot(normalize(vSphere), light)` in sun mode; `ratio = clamp(max(dot(n, light), 0) / max(dot(vNormal, light), 0.04), 0.12, 2.2)`; sun mode `shade = (night + (1 − night) · day · strength · sunlit) · ratio` (today's terms, `relief` replaced by `ratio`); headlight mode `shade = mix(ratio, (0.35 + 0.75 · lambert) , uMorph)` with `lambert = max(dot(n, uLightDir), 0)`.
  - Lava: `emitted = colour · max(LAVA_GLOW · lava · night, LAVA_DAY_GLOW · lava)` with `LAVA_DAY_GLOW = 0.25`, added after `colour *= shade · uLightTint`; the glow test keeps today's colour detection, read from the unshaded colour.
- Remove the `world.png` fallback in `Viewer.svelte:652-655` (Task 6 owns the file).

- [ ] **Step 1:** `cd web && npm run check` passes before changes (baseline).
- [ ] **Step 2:** Implement; `npm run check` and `npx vitest run` → PASS.
- [ ] **Step 3:** Commit `feat(viewer): light the relief with the normal map`.

### Task 6: Wire the views to the new files (viewer)

**Files:**
- Modify: `web/src/components/Viewer.svelte` (617-666 base map, 688-691 `epochSurface` → `epochMaps`, 695-729 epoch slider, 912-916 `sysMaps`, 944-960 `sysTextures`), `web/src/lib/systemSky.ts:211-260`, `web/src/lib/skyBodies.ts:34-82` (`texture` → `colour: string | null; normal: string | null`)

**Interfaces:**
- Consumes: `setMaps` (Task 5), `epochFile` (Task 4), `worldFile`.
- Produces: base map `setMaps(worldFile(id,'raw_data/colour.png',rev), worldFile(id,'raw_data/normal.png',rev))`, epochs `setMaps(epochFile(..,'colour.png',..), epochFile(..,'normal.png',..))`; `systemSky(..., maps: (id) => { colour: string; normal: string } | null)`; `sysMaps` returns `{ colour: worldFile(..,'raw_data/colour_small.png'), normal: worldFile(..,'raw_data/normal_small.png') }`.

- [ ] **Step 1:** Update `systemSky` tests (`starSystem.test.ts`/`systemSky` tests that build bodies) to expect `colour`/`normal` URLs → FAIL. **Step 2:** Implement → `npm run check`, `npx vitest run` PASS.
- [ ] **Step 3:** Commit `feat(viewer): load the colour and the normals in every planet view`.

### Task 7: Sky bodies lit by their normals (viewer)

**Files:**
- Modify: `web/src/lib/viewer/shaders.ts` (`skyBodyFragment` 914-1156), `web/src/lib/viewer/PlanetView.ts` (`setSkyBodies` 2325-2437: uniforms `uHasMap`/`uMap` + new `uNormalMap`, loader, the same-bodies check 2329-2334)

**Interfaces:**
- Produces: in `skyBodyFragment`, east/north from the body frame (`b` and `uAxisX/Y/Z`: east = normalize(cross(axis, n)), north = cross(n, east)); `sunCos` computed with the perturbed normal for the Lambert term (the limb and twilight terms keep the sphere normal); lava glow from the colour as in Task 5 when `uHasMap > 0.5`.

- [ ] **Step 1:** Implement; `npm run check`, `npx vitest run` PASS. **Step 2:** Commit `feat(viewer): light the bodies in the sky with their normals`.

### Task 8: Star view lit by the star (viewer)

**Files:**
- Modify: `web/src/lib/viewer/StarView.ts` (giant markers 876-918, `setPlanetMaps` 940-962, `applyMaps` 982-999, `maps` cache 332), `web/src/components/StarPane.svelte:107-108, 212`
- Create: `web/src/lib/viewer/markerShaders.ts` (the marker material's GLSL)

**Interfaces:**
- Consumes: `sysMaps` entries `{ colour, normal } | null` (Task 6).
- Produces: `StarView.setPlanetMaps(maps: readonly ({ colour: string; normal: string } | null)[]): void`; giant markers use a `ShaderMaterial` with `uMap`, `uNormalMap`, `uHasMap`, `uColour` (fallback colour), lit by the star at the scene origin (`light = normalize(-markerWorldPosition)`, passed per marker as `uLightDir`, updated in `placeGiants`), day curve and lava glow as Task 5 (`LAVA_GLOW` on the night side, `LAVA_DAY_GLOW` floor); previews (`setPlanetPreviews`) stay as the colour with flat normals.

- [ ] **Step 1:** Implement; `npm run check`, `npx vitest run` PASS. **Step 2:** Commit `feat(viewer): light the planets of the star view by their star`.

### Task 9: Docs, full check, and the look in the app

**Files:** the documentation files listed in Global Constraints.

- [ ] **Step 1:** Update every doc line naming `surface.png` or the hillshade lighting of the viewer; README raw_data table rows for `colour.png`, `normal.png`, `colour_small.png`, `normal_small.png`; tile route doc (256 × 512, two halves); epoch route files.
- [ ] **Step 2:** `cargo fmt --check`; `cargo clippy -j 4 --workspace --all-targets --all-features -- -D warnings`; `RAYON_NUM_THREADS=4 cargo test -j 4 --workspace --all-features -- --test-threads=3`; `cd web && npm run check && npx vitest run` → all PASS.
- [ ] **Step 3:** Build and run a local server on a free port other than 3000 with a scratch `MAPGEN_SYSTEMS_DIR`, generate a terran and a volcanic world, and check by hand (screenshots with the playwright scripts in the session scratchpad): sun on at noon and at a low sun; sun off (headlight); the flat map; zoom to detail tiles (no seam, no change of look); an older epoch; the sky from a moon; the star view's day/night; lava glowing on a shaded slope and at night. Measure the frame time with `measure.mjs` against the numbers of 2026-11-08 (2–5 ms).
- [ ] **Step 4:** Commit `docs: describe the normal-map relief lighting`; push; open the PR into main with the measurements and screenshots, `Refs`/`Closes` for any related issue (check `gh issue list` for relief or lighting items); ask mapgen2 for a slot on develop.
