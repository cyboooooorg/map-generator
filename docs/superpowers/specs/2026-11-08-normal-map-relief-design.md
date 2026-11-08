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

## Goals

1. **Planet view (main target):** the globe and the flat map light their
   relief with a normal map. With the sun on, by the sun. With the sun off,
   by the headlight that follows the camera (the light the app already
   uses in that mode, `uLightDir`).
2. **Detail tiles:** the same lighting at every zoom level, so the relief
   does not change look when tiles replace the base map.
3. **Star view:** the small planet models get a day side and a night side,
   lit by their star, with the normal-map relief. Today they are unlit
   (`MeshBasicMaterial`); the night side is wanted.

## Non-goals

- Flat images keep their baked hillshade: `world.png` (download), the
  library thumbnails (`thumb.png`), the legend, `raw_data/surface.png`
  for the consumers listed below.
- Other epochs (crater epochs, plate drift previews) keep the baked
  surface for now; they move to the normal map in a later change.
- No cast shadows (a slope does not darken the valley next to it).
- No hybrid mix with the fixed north-west light (the user chose the pure
  normal map, knowing relief fades under a sun at the zenith).

## Anti-goals

- The relief must not change look between the base map and the detail
  tiles, nor jump when tiles load.
- Water stays flat: no fake waves on seas and lakes.
- No legacy path for old worlds: worlds generated before this change show
  what they show today (baked surface, current shading) until they are
  generated again. The user does not need old algorithms kept.
- No noticeable cost: the viewer draws a frame in 2–5 ms on the
  workstation (measured 2026-11-08); the change may add a texture fetch or
  two per pixel, not a pass.

## Constraints

- Generation output: new worlds get two more layers in `raw_data/`. The
  format version goes up (16; #289 takes 15, #290 sets 14 and
  `MIN_FORMAT_VERSION` 14; agree the number with the other sessions before
  merging). Nothing gates on it except "does the world have the layers".
- The relief exaggeration is the hillshade's (`RELIEF_KM` 1500 km per
  elevation unit on Earth-sized bodies, smaller on small bodies, see
  `relief_km`), so slopes read the same strength as today at a low sun.
- Tile cache: tiles are cached for good by URL; any change to tile pixels
  bumps `RENDER_VERSION` (Rust) and `TILE_RENDER_VERSION` (viewer).
- Shared machine: tests throttled (`RAYON_NUM_THREADS=4 … --test-threads=3`),
  no `npm ci` in worktrees, never port 3000.
- Documentation ships in the same change: README (raw_data layers, the
  lighting), the format version doc comment, `docs/` where the viewer's
  lighting is described.

## Decided

- **Pure normal map**, lit by the sun (sun on) or the headlight (sun off),
  in the planet view; by the star in the star view.
- **Two new raw_data layers** per generated land body:
  - `colour.png` (key `colour` in `world.json`): the detailed render
    without the hillshade. *(Builder check: `albedo.png` already names the
    crater-ejecta layer, so the new layer takes another name; my call.)*
  - `normal.png` (key `normal`): tangent-space normals (east, north, up),
    8 bits per channel, from the elevation's central differences at the
    hillshade's exaggeration.
- **Tiles in two variants**: the colour without hillshade and the normals,
  both at the tile's resolution and from the same detail elevation the
  tile renderer already builds, so the relief continues the base map's.
- **Star view**: the planet models are lit by the star (day and night
  sides) and use the normal map when the body has one.
- **Old worlds**: without the layers, everything behaves as today.

## Calls made after the builder check

These follow from the decisions above. I settled them; the user can
overrule any of them.

- **Which pixels carry relief.** The colour layer is `render_pixels` with
  the hillshade factor set to 1, every other step unchanged (rivers
  blended over it, fresh-crater albedo, lava). The normals mirror today's
  shading exactly: flat (0, 0, 1) wherever the render skips the hillshade
  (water biomes by `is_water`, sea ice, frozen lakes and rivers drawn as
  water), and tilted by `LAVA_SHADE`'s share on lava. Tests compare the
  colour layer with a render whose hillshade is forced to 1 (exact),
  not by dividing `surface.png` (not invertible: clamps, rivers, lava).
- **Two shading paths in the viewer**, chosen per texture set: a world,
  epoch or body whose metadata lists the `normal` layer uses `colour.png`
  + `normal.png`; anything else (old worlds, other epochs, drift
  previews, sky bodies until they switch) uses `surface.png` as today.
  The signal is the presence of `layers.normal` in `world.json`, nothing
  else. Selecting another epoch switches to the baked path: the relief
  lighting visibly changes there until epochs get normals (a later
  change).
- **Tangent frame**: east and north of the latitude/longitude grid, built
  in the shader from the point on the sphere, as the current elevation
  tilt does; on the flat map, east is +x and north +y. Near the poles the
  east-west differences use the hillshade's floored row width
  (`dx_km`, at least a tenth of the equator's), so normals stay bounded.
- **Tiles: one request, both variants.** A tile URL gains a variant
  (`&lit=normal`, in the cache key with `rev` and `render`); the server
  renders colour and normals in one pass and returns them stacked in one
  PNG (colour on top, normals below, 256 × 512), so they arrive together
  and a tile never shows colour without its normals. Normals use the same
  one-pixel border as the hillshade, so tiles have no seams. Old worlds
  keep the baked tile URL. `RENDER_VERSION` and `TILE_RENDER_VERSION` go
  up.
- **The other layers** (elevation, temperature…) use the normal map too
  when it exists, for one consistent look; the fragment-shader elevation
  tilt stays only as the fallback for worlds without it.
- **Giants and bodies without relief** write no `colour.png` or
  `normal.png`; the viewer treats a body without them as flat, with day
  and night. Moons and airless worlds of the main pipeline get both.
- **Star view textures** come downscaled from the server (a `width`
  query on the two layers, as drift previews already have), not the full
  maps, so a system with many bodies stays light.
- **Format version**: 16 in the doc comment, assuming #289 (15) and #290
  (14) merge first; if the order changes, the number is fixed at merge
  time with the other sessions.

## Left to the builder

- The exact shading formula, as long as: flat ground under the light at
  the zenith shows the same brightness as today's flat ground; the night
  side follows the current day curve (`smoothstep` on the sun's height,
  low sun dimmer); slopes facing away from a low light darken without
  turning pitch black. The mock-up's shader is a starting point.
- How the viewer applies the normals to the other layers (elevation,
  temperature…), which today tilt the normal from the elevation texture in
  the fragment shader: either keep that path or read the normal map; one
  consistent look is what matters.
- The width of the star view's textures (planets there are a few to a
  few dozen pixels wide).
- Whether the bodies seen in the sky of the planet view (`skyBodies.ts`,
  which wrap `surface.png`) also switch; optional.
- Tests: Rust tests that the normals of a flat world are (0, 0, 1), that a
  known slope gives the expected normal, that the colour layer equals the
  render with the hillshade forced to 1, that tiles' normals continue
  across tile borders; viewer tests where the code is testable without
  WebGL.
- Measuring the frame time and the extra memory, and reporting them in the
  PR.
