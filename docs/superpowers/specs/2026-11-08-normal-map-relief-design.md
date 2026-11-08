# Normal-map relief lighting

Date: 2026-11-08. Status: design, awaiting approval.

## Intent

Today the relief of a planet is shaded once, at generation, into its
surface texture (`export::hillshade`): light from the north-west at 45°,
whatever the sun does. In the viewer the sun only sets day and night. The
user compared three renderings on a mock-up (baked hillshade, normal map,
hybrid; `https://claude.ai/artifact/Y3yRd1kobsFoiX53JwkRgE`) and chose the
**pure normal map**: the colour without shading, plus a texture of the
ground's orientation, lit every frame by the real light. Mountains then
face the sun in the morning and the evening, and the terminator gets
volume. It looks more realistic.

The user wants no legacy left behind: every 3D view switches at once, and
the worlds generated before the change will be deleted, so nothing keeps
the baked path alive for them.

## Goals

1. **Planet view (main target):** the globe and the flat map light their
   relief with a normal map. With the sun on, by the sun. With the sun off,
   by the headlight that follows the camera (the light the app already
   uses in that mode, `uLightDir`).
2. **Detail tiles:** the same lighting at every zoom level, so the relief
   does not change look when tiles replace the base map.
3. **Other epochs:** the crater epochs and the plate-drift previews are lit
   the same way.
4. **Bodies in the sky** of the planet view (`skyBodies.ts`: the planet
   seen from its moon, the moons seen from their planet) use the same
   textures and lighting.
5. **Star view:** the small planet models get a day side and a night side,
   lit by their star, with the normal-map relief. Today they are unlit
   (`MeshBasicMaterial`); the night side is wanted.

## Non-goals

- Flat images keep their baked hillshade: `world.png` (download, with its
  contours and reference lines), the library thumbnails (`thumb.png`, made
  from `world.png`), the legend, the SVG. They have no light to follow.
- No cast shadows (a slope does not darken the valley next to it).
- No hybrid mix with the fixed north-west light (the user chose the pure
  normal map, knowing relief fades under a sun at the zenith).

## Anti-goals

- **No legacy path.** No viewer code for textures with baked shading, no
  fallback for worlds without the new layers, no `surface.png` kept for
  old consumers.
- The relief must not change look between the base map and the detail
  tiles, nor jump when tiles load.
- Water stays flat: no fake waves on seas and lakes.
- No noticeable cost: the viewer draws a frame in 2–5 ms on the
  workstation (measured 2026-11-08); the change may add a texture fetch or
  two per pixel, not a pass.

## Constraints

- **Format version.** New worlds have a new raw_data layout, so
  `FORMAT_VERSION` goes up, and `MIN_FORMAT_VERSION` (added by #290) goes
  up with it: a world without the new layers is refused, as #290 refuses
  older formats. The user will delete the existing worlds. Expected
  number: 16 (#290 takes 14, #289 takes 15); agree it with the other
  sessions at merge time if the order changes.
- The relief exaggeration is the hillshade's (`RELIEF_KM` 1500 km per
  elevation unit on Earth-sized bodies, smaller on small bodies, see
  `relief_km`), so slopes read the same strength as today at a low sun.
- Tile cache: tiles are cached for good by URL; any change to tile pixels
  bumps `RENDER_VERSION` (Rust) and `TILE_RENDER_VERSION` (viewer).
- Shared machine: tests throttled (`RAYON_NUM_THREADS=4 … --test-threads=3`),
  no `npm ci` in worktrees, never port 3000.
- Documentation ships in the same change: README (raw_data layers, the
  lighting, the API routes), the format version doc comment, `docs/`
  where the viewer's lighting is described.

## Decided

- **Pure normal map**, lit by the sun (sun on) or the headlight (sun off)
  in the planet view; by the star in the star view.
- **`raw_data/surface.png` is replaced** by two layers:
  - `colour.png` (key `colour` in `world.json`): the detailed render
    without the hillshade. (`albedo.png` already names the crater-ejecta
    layer, hence another name.)
  - `normal.png` (key `normal`): tangent-space normals (east, north, up),
    8 bits per channel, from the elevation's central differences at the
    hillshade's exaggeration.
- **Every 3D consumer of `surface.png` moves to the pair**: the planet
  view's base map, the detail tiles, the crater epochs, the drift
  previews, the sky bodies, the star view.
- **No fallback**: worlds without the layers are refused by the format
  version.

## Calls made after the builder check

These follow from the decisions above. I settled them; the user can
overrule any of them.

- **Which pixels carry relief.** The colour layer is `render_pixels` with
  the hillshade factor set to 1, every other step unchanged (rivers
  blended over it, fresh-crater albedo, lava). The normals mirror today's
  shading exactly: flat (0, 0, 1) wherever the render skips the hillshade
  (water biomes by `is_water`, sea ice, frozen lakes and rivers drawn as
  water), and tilted by `LAVA_SHADE`'s share on lava. A test checks that
  `world.png`'s render equals the colour layer times today's hillshade
  (the flat images keep exactly their current look).
- **Giants** (their own pipeline, no relief): `colour.png` holds their
  rendered bands (what `surface.png` held) and `normal.png` is flat, or
  is omitted with the viewer treating a missing normal map as flat; the
  builder picks whichever keeps the viewer's code to one path.
- **Epochs and drift previews**: the server renders them from the decoded
  world (`render_pixels` in `crates/server/src/tiles.rs` and
  `crates/mapgen/src/drift.rs`); its epoch and drift routes serve
  `colour.png` and `normal.png` instead of `surface.png` (same `scale` and
  `width` queries), from the epoch's own elevation, so the relief at an
  epoch is lit like the present's.
- **Tangent frame**: east and north of the latitude/longitude grid, built
  in the shader from the point on the sphere, as the current elevation
  tilt does; on the flat map, east is +x and north +y. Near the poles the
  east-west differences use the hillshade's floored row width (`dx_km`,
  at least a tenth of the equator's), so normals stay bounded.
- **Tiles: one request, both variants.** The server renders colour and
  normals in one pass and returns them stacked in one PNG (colour on top,
  normals below, 256 × 512), so they arrive together and a tile never
  shows colour without its normals. Normals use the same one-pixel border
  as the hillshade, so tiles have no seams. `RENDER_VERSION` and
  `TILE_RENDER_VERSION` go up.
- **The other layers** (elevation, temperature…) use the normal map too,
  for one consistent look; the fragment-shader elevation tilt goes away.
- **Star view textures** come downscaled from the server (a `width` query
  on the two layers, as drift previews already have), not the full maps,
  so a system with many bodies stays light.

## Left to the builder

- The exact shading formula, as long as: flat ground under the light at
  the zenith shows the same brightness as today's flat ground; the night
  side follows the current day curve (`smoothstep` on the sun's height,
  low sun dimmer); slopes facing away from a low light darken without
  turning pitch black. The mock-up's shader is a starting point.
- The width of the star view's textures (planets there are a few to a
  few dozen pixels wide).
- Tests: Rust tests that the normals of a flat world are (0, 0, 1), that a
  known slope gives the expected normal, that the flat images are
  unchanged, that tiles' normals continue across tile borders; server
  tests of the new routes; viewer tests where the code is testable
  without WebGL.
- Measuring the frame time and the extra memory, and reporting them in the
  PR.
