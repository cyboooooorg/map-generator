# Analysis: river width and the terrain around rivers

Backlog item 5. User report: rivers are well traced but far too wide on the
base render, and look right only when zoomed in; the terrain around rivers
and streams needs planning, since rivers are drawn over terrain that has no
valleys or floodplains of their own.

Worlds measured: terran, seed 3, sea level 0.2, volcanic 0.3, circumference
40 000 km, tilt 23.44, at 1920, 4096 and 8192 px.

## 1. River width

**Where it is drawn.** The nominal width `0.9 × √(Q / 120 000 km²)`
reference pixels (capped at 4; one reference pixel is 1/1920 of the
circumference) was drawn at `nominal × W / 1920` map pixels: the same width
in km at every map size. `world.png` blends the coverage over the land, the
biome layer marks `River` where coverage reaches 0.5, `world.svg` strokes the
same widths, and the web globe shows `world.png` itself. Detail tiles used
`max(base width × √zoom, 0.002 √Q km)`.

| Raster                      | Drawn width (Q = 0.12 M → 4 M km²) | Times real |
| --------------------------- | ---------------------------------- | ---------- |
| Base 1920                   | 18.8 → 83 km (0.9 → 4 px)          | 21–27×     |
| Base 4096                   | same km (1.9 → 8.5 px)             | 21–27×     |
| Base 8192                   | same km (3.8 → 17 px)              | 21–27×     |
| First tile level, 8192 base | 3.1 → 14 km                        | 3.5–4.5×   |

- Rivers shrank by 3× (4096 base) or 6× (8192 base) when tiles replaced the
  base map, and the same tile drew different widths depending on the base
  size.
- Drawn rivers covered 27× their real area; the River biome took 3.2–3.6 %
  of land against about 0.6 % for real rivers of all sizes.
- Coverage `clamp(half + 0.5 − d, 0, 1)` overstated sub-pixel lines: a 0.1 px
  river was drawn at 0.55 opacity.

**Options.** Shrinking the constants keeps the jump. A single law per raster
resolution removes it: on a raster `W` pixels around the planet, draw
`INK × nominal × (W / 1920)^(2/3)` pixels, never below the real width, with
exact box-filtered coverage. The base map then behaves like a tile level of
its own width, and each doubling of resolution narrows a river by 1.26× in
km. Drawing coarse tile levels on the globe, with a flow-based visibility
floor, would keep more rivers visible from orbit; it is optional.

**Decision (done).** The single law with `INK = 0.5`. The prototype's 0.35
left almost no river visible on the whole-planet view. The jump at the first
tile level falls from 3–6× to 1.26×; the River biome falls to about 1 % of
land. River shapes do not change: meanders still use the nominal width.

## 2. Terrain around rivers

**Measured.** Erosion runs on a coarser grid, before hydrology, so its
valleys are 40–80 km wide. 10–20 km from a river the banks are flat (mean
bank minus river 0 to 2e-4); only 10–20 % of points have both banks higher.
Detail tiles add fractal relief everywhere, floodplains included.

**Design.** All lengths in km, depths in elevation units.

1. **Bed profile:** sample the depression-filled surface along the line,
   keep a running minimum so the bed never rises downstream, subtract an
   incision `d0 × (Q/Q0)^0.25`, clamp above the waterline (by more than the
   detail noise), to the lake surface, and to the parent's bed at
   confluences.
2. **Cross-section:** `e' = min(e, bed + wall × max(0, d − F/2))`, faded back
   to `e` towards the valley edge; floodplain width `F = f0 × √(Q/Q0)`, about
   10–80 km. The minimum never raises terrain, never creates water and keeps
   the divides.
3. **Valley axis** on a smoothed centreline (the line before its meanders),
   the floodplain at least as wide as the meander belt.
4. **Floodplain weight:** a riparian moisture bonus (green strips across
   deserts, like the Nile); an optional `Floodplain` biome later.
5. **Deltas and estuaries**, chosen from flow, coastal slope and a seeded
   hash: distributaries (a `source` field in `rivers.json`), a wetland lobe,
   never moving the coastline.
6. **Tiles:** damp the detail noise in floodplains, floor detailed elevation
   at the waterline where the base is land, carve the channel and banks along
   the same curve as the river coverage.

Pipeline: noise → erosion → climate → hydrology → trace and shape →
**carve** → riparian moisture → coverage → biomes → export. Valleys,
floodplains and moisture go into the raw data (new `floodplain.png`,
`format_version` 4); channels, smaller than a base pixel, go into the tiles.
The prototype took 1.5 s at 8192 px; a careful version costs 35–140 MB of
extra memory, within the 6 GB container.

**What the prototype showed.** Valleys and floodplains read clearly in the
hillshade. Two problems to handle: walls tracing every meander (a ditch
look; hence the smoothed axis), and floodplain floors so close to the
waterline that tile noise turns them into sea near coasts (hence the damping
and the waterline floor).

## 3. Phases

| Phase | Content                                                                    | Size   |
| ----- | -------------------------------------------------------------------------- | ------ |
| 0     | Metric tests: cross-sections, coast unchanged, monotone bed, drainage kept | Small  |
| 1     | `carve.rs` at generation, floodplain layer, load fallback for older worlds | Medium |
| 2     | Tiles: noise damping, waterline floor, channel and banks                   | Medium |
| 3     | Biome polish: riparian bonus tuning, optional Floodplain biome             | Small  |
| 4     | Deltas and estuaries                                                       | Large  |
