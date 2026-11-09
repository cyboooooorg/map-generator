# Planet resolutions: design

Date: 2026-11-08. Status: approved by the user on 2026-11-08; part 1 implemented on feat/planet-resolutions.

## Intent

The user generated a planet they liked at 1920×1080 and wanted to see the
same planet at about 8000 px wide. Today the only way is to generate a
new system with the same seed and a larger size, which replaces or
duplicates the system and is not discoverable.

The goal: a planet that already exists can be shown at another
resolution, keeping its large-scale geography (continents, coasts in
outline, biomes, river networks), and the user switches between the
resolutions it has quickly. Fine detail (coast wiggles, gullies, exact
river traces, the smallest craters) may differ between resolutions: the
user accepted that.

A second, related need: free the disk from a whole system, keeping only
its parameters, since every planet can be generated again from them.

## Why it is feasible

The generator is resolution-invariant at large scale by design (recon of
2026-11-08, not yet asserted by a test):

- all noise is sampled on the unit sphere (`Grid::unit_vector_at`,
  `crates/mapgen/src/geo.rs`), seeded per stream without a consumed RNG;
- plate boundaries are built on a fixed `CubeGrid` whatever the map size
  (`plates.rs`);
- the main erosion pass runs on a grid capped at 1024×512
  (`erosion.rs`), so it is identical for any map at least 1024 wide;
- hydrology thresholds and wavelengths are in km and km²
  (`hydrology.rs`, `carve.rs`);
- land fraction is resolved to a stored `sea_level` on a fixed grid.

What varies with size: coast octaves down to 2 px, the detail erosion
pass, flat-fill noise frequency, the minimum crater size (4 px), river
drawing widths and pixel-step routing. These change detail only.

Every body stores its full `WorldParams` (seed, resolved sea level,
resolved preset, epoch...) in `system.json` and in the index, and
`POST /api/systems/{id}/bodies/{body}/generate` already regenerates a
body from them. What is missing is a size that differs from the stored
one, and a place to keep several sizes.

## Goals

1. **Several resolutions per planet, side by side.** A body has its
   original resolution (the size it was generated at) and any number of
   added ones. The original is never overwritten.
2. **Generate on demand.** Asking for a resolution the body does not
   have generates it from the stored parameters with only `width` and
   `height` changed, then saves it.
3. **Fast switch.** Switching to a resolution that exists loads its files
   without any generation.
4. **Remembered per planet.** The body remembers its last displayed
   resolution on the server (not in the browser): reopening the planet,
   from any browser, shows that resolution.
5. **Delete a resolution.** Any added resolution can be deleted; the
   original cannot.
6. **Reduce a system.** One action on a system deletes every generated
   file of every body in it, originals included, keeping only the
   parameters (`system.json`, the index rows, names, pins) and the small
   thumbnails. Opening a body
   afterwards regenerates it at its remembered displayed resolution.
7. **A test of the invariance.** The same parameters generated at two
   sizes keep the same large-scale geography.

## Non-goals

- Changing the resolution of a whole system's bodies at once. Only the
  planet the user is looking at gets a new resolution.
- Free sizes. The offered sizes are the generate form's presets.
- Making detail identical across resolutions, or upscaling images. Each
  resolution is a real generation at its own size.
- Automatic eviction after 14 days (issue #328). The reduce action
  shares its mechanism (see Constraints) but the timer stays in #328.

## Anti-goals

- A larger resolution whose continents or biomes visibly differ from the
  original: that is a different planet, and the feature has failed even
  if it runs.
- Losing the original, or the parameters needed to regenerate a body.
- Writing to the user's live data (`systems/`, `db/`) during development
  or tests.

## Decisions made with the user

- **Where the switch lives: on the planet view.** A "px" button in the
  view's icon column (the toggle bar, `web/src/components/dock/`) opens a
  row of chips under the planet, one per offered size:
  - filled chip: generated, click to switch;
  - dashed chip ("8192 +"): not generated, click to generate;
  - highlighted chip: the one displayed.
  The size line ("1920 × 1080 px") leaves the world info panel
  (`WorldInfo.svelte`), since the chips now show it.
- **While generating:** the same loading bar as a body's first
  generation (`LoadingBar.svelte`, the jobs machinery).
- **Deletion:** added resolutions can be deleted, never the original.
- **Reduce a system:** removes all maps including originals; only the
  parameters stay. Regeneration is acceptable because bodies are
  deterministic.
- **2:1 only:** an equirectangular map has square pixels only at 2:1
  (360° by 180°), which is what the sphere wants. The generate form drops
  960×540 and 1920×1080 and offers 2:1 sizes only, and the resolution
  chips offer 2:1 sizes only.
- **No compatibility with existing data.** The systems and planets
  generated so far are deleted when this ships; nothing has to read,
  migrate or display them. The storage layout, the index layout and the
  format versions may change freely, with no migration code.
- **Drift:** the drift slider's previews stay low resolution
  (`PREVIEW_WIDTHS`, 256 and 512) so they stay fast. The full world at the
  chosen epoch is generated at the displayed resolution.

## Builder's calls (made by Claude, not by the user)

- **End of a generation:** when the new resolution is ready, the view
  switches to it, and it becomes the remembered one. The server marks
  it as displayed when the job is done, so that it is remembered even
  when no tab follows the job.
- **After a reduce:** a body regenerates at its remembered displayed
  resolution, not at its original.
- **Offered sizes:** the generate form's `SIZES`
  (`GenerateForm.svelte`), now 2:1 only (see the user's decision on
  aspect ratio). A body made from the CLI at a non-preset size shows
  that size as an extra chip.
- **2:1 presets:** 1024×512, 2048×1024, 4096×2048, 8192×4096. 1024×512
  replaces 960×540 as the quick size; it is also the smallest size at
  which the main erosion grid is the same as at every larger size. The
  default (`request::fallback::WIDTH/HEIGHT`, used by the CLI and
  `GET /api/form`) becomes 2048×1024 instead of 1920×1080. The CLI still
  accepts any size within `MAP_SIZE`; only the presets and the default
  change.
- **Server limit:** `MAPGEN_MAX_PIXELS` defaults to 8192×4096
  (33 554 432) instead of 8 388 608, since the largest offered size is
  refused today. Memory use of an 8192×4096 generation must be measured
  before the default changes (see Open points).
- **Tiles, epochs and drift** use the displayed resolution's files.
- **Bodies in scope:** every body with a generated world map. A body is
  regenerated today from its own `WorldParams` alone
  (`generate_with_progress(&world_params(body), ...)`, `system.rs`), so
  changing only `width` and `height` there is the whole entry point. The
  system-level `width`/`height` (`system.json`, index `systems` table)
  stay the original request's and are not touched.
- **Invariance test first, as a gate.** It is written before the feature.
  If it shows a large-scale difference (for example at 1024 against
  8192 wide), the generator is fixed so large
  scale no longer depends on size; the project accepts worlds that change
  when they stay coherent and keeps no legacy algorithm. Only if that
  fails is the offending size dropped from the chips. Tolerances are the
  builder's to set from measured same-size noise
  (`world-coherence-checker` statistics), for example land mask
  agreement on a 256×128 downsample and per-biome area shares.
- **8192×4096 does not fit in memory:** the chip stays but is disabled,
  its tooltip giving the reason, and `MAPGEN_MAX_PIXELS` is raised only as
  far as measured to be safe.
- **Derived files follow the resolution.** Everything generated for a
  body (`raw_data/`, `world.png`, tiles and epoch layers rendered on the
  fly and their caches) belongs to one resolution; deleting it removes
  them. Drift full worlds are new or replaced systems made through the
  generate form, so they are not a resolution of the old body; the form's
  size defaults to the displayed resolution.
- **Index lost:** sizes present are rediscovered from the tree; the
  displayed one falls back to the original.
- **Reduce keeps thumbnails.** `thumb.png` (a few kB) stays so the library
  and system cards still look right; everything else goes. Only opening a
  body's view queues its regeneration, never a listing or a card.
- **Version numbers:** whatever `FORMAT_VERSION`, `SYSTEM_FORMAT_VERSION`
  or index layout bump the new layout needs is agreed with the other
  sessions when the plan starts; old data is refused, not migrated.
  Step 2's shared eviction function can land before #328 starts; #328
  then reuses it.
- **Generator version:** as decided in `docs/analysis/15-storage.md`
  (decision 4), a body regenerated after the generator changed is drawn by
  the current version and may differ from what was deleted. The index
  already records the version each body was made with.

## Constraints

- **Storage layout.** Today a body's files sit in
  `systems/<id>/bodies/<body>/` (`world.png`, `thumb.png`, `raw_data/`...).
  Since old data is dropped, every resolution, the original included, can
  use the same per-size layout (suggested: `bodies/<body>/sizes/<W>x<H>/`,
  with `thumb.png` beside `sizes/`). The builder picks the layout; the
  CLI, the server and the viewer move to it together.
- **Thumbnails** (`thumb.png`, the system card) come from the original
  and survive a reduce.
- **Index.** The index records, per body, the resolutions present on
  disk and the displayed one (`crates/server/src/index.rs`). The
  reconcile at start discovers per-size directories from the tree. A new
  layout version simply refuses or rebuilds an older database file; no
  migration.
- **`system.json`** keeps the original `width`/`height` in each body's
  params. Whether the displayed resolution is also mirrored in
  `system.json` is the builder's call; the index-only precedent is pins.
- **Wiping the live data** (`systems/`, `db/` of the user's checkout,
  mounted by planets.localhost) happens once, at deployment, with the
  user's go-ahead at that moment. Development and tests never touch it.
- **Reduce and #328.** Reduce is a manual eviction. Its "delete a body's
  generated files, keep its parameters, regenerate on open" path is the
  one #328 needs; build it as a shared function so #328 only adds the
  timer. A body without files must be shown as "not generated" in the
  library and the system view, and opening it queues its generation.
- **Concurrency.** Two requests for the same missing resolution queue one
  job. Deleting a resolution while it is displayed or generating is
  refused (or the view falls back to the original first).
- **Cache invalidation.** Tiles and previews are keyed by body and
  revision (`tiles.forget`, `previews.rs`): the key must also carry the
  resolution, or switching sizes serves stale tiles.
- **Tests and data.** Tests never touch `systems/`, `worlds/` or `db/` of
  the main checkout; use temporary directories. The invariance test runs
  at small 2:1 sizes (for example 1024 and 2048 wide) and compares
  area-weighted statistics and a downsampled land mask with a tolerance.
- **Documentation.** README (viewer and API sections), the API route
  docs at the top of `crates/server/src/app.rs`, and the storage analysis
  get updated with each step.

## API sketch (the builder may refine it)

- `GET /api/systems/{id}` lists, per body, its resolutions and the
  displayed one.
- `POST /api/systems/{id}/bodies/{body}/sizes/{W}x{H}`: generate that
  resolution (a job, like `generate`). 409 if it exists or is queued.
- `DELETE /api/systems/{id}/bodies/{body}/sizes/{W}x{H}`: delete an added
  resolution. 409 for the original.
- `PUT /api/systems/{id}/bodies/{body}/displayed-size`: remember the
  displayed resolution.
- `POST /api/systems/{id}/reduce`: delete every body's generated files.
- File and tile routes take the resolution (a path segment or a query
  parameter), defaulting to the displayed one.

## Steps

1. **Resolutions per planet:** storage, index, API, the chips in the
   view, deletion, the remembered size, the server limit, the invariance
   test, the 2:1-only presets and the 2048×1024 default.
2. **Reduce a system:** the shared eviction function, the "not
   generated" state of a body, regeneration on open, the system action.
   Refs #328.

## Open points left to the builder

- Peak memory and time of an 8192×4096 generation on this machine
  (`generation-profiler`), to set the `MAPGEN_MAX_PIXELS` default and the
  loading bar's expectations.
- Exact layout of the size directories and the route shape.
- Where the reduce action sits in the UI (`SystemActions.svelte` is the
  natural place) and its confirmation wording.
