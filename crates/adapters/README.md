# `adapters`: the outside world

Implementations of the domain ports: filesystem storage with atomic
staging, PNG projections, SVG cross-sections, the English text catalogue
of codes and cause trees, relief files. The only library crate allowed to
read and write files.

## Filesystem store

`FsStore::new(root, budget)` (`DEFAULT_BUDGET` is 2 GiB) implements
`SystemRepository`, `TimelineCache` and `WorldCache`. Layout under `root`:

```text
systems/<id>/inputs.json
cache/<engine>/<id>/timelines.json
cache/<engine>/<id>/worlds/<slug>/<keyframe>-<grid>-w<world>.world
cache/uses.json
```

`<slug>` is the body's designation with `-` for spaces (`MGS-12345-b`),
`<keyframe>` its index in the timeline and `<grid>` the cells per face
side, `<world>` the map generator version (`world::ENGINE_VERSION`) that
made it. Never write under `root` by hand: inputs are the only truth, the
rest is rebuilt.

Writes are staged in `<file>.<pid>-<n>.tmp`, synced, then renamed; there
is no lock, so two writers of one file race and the last rename wins.
`evict` also deletes leftover `*.tmp` files under `cache/`. A cache hit
never fails because `uses.json` could not be updated. `WorldCache::put`
refuses a world that does not match its key (`StoreError::KeyMismatch`).
A cache file that cannot be read or parsed is a miss. `cache/uses.json`
holds a logical use counter (no wall clock); `FsStore::evict(current)`
(a `catalogue::Engines`) deletes the entries of other engine versions
(system, and world for worlds), then the least recently used ones until
the cache fits `budget` bytes. World and timeline files are both listed.
It also deletes the world files whose name is not of the form above
(older layout, with their `uses.json` keys and the directories this
leaves empty) and the cache entries of any system whose `inputs.json`
no longer exists, so removing a system frees its caches at the next
eviction.

A `.world` file is the binary layout of `encode`/`decode` (see
`src/world_codec.rs`): magic `MGWORLD2` (an `MGWORLD1` file is a miss),
then the system and world engine versions, then the little-endian header
and fields. A truncated, corrupt or mismatching file is a cache miss.
A grid above `world::MAX_N` is refused on both sides: `put` errors and
`decode` returns `Invalid`.

PNG projections: `write_png(path, raster, ramp)` colours a `world::Raster`
through a `Ramp` (linear stops, `Ramp::grey` for a grey scale; NaN is
black) and writes an 8-bit RGB PNG with the same staged write as the
store.

Export: `export(out, world, field, width, ramp)` projects `field` first
(a missing field is `ExportError::World` and nothing is created), then
writes `<out>/<slug>-<keyframe>.world` (the codec bytes) and
`<out>/<slug>-<keyframe>-<field code>.png` (`width` by `width / 2` pixels),
creating `out` when needed. `Ramp::relief()` is the hypsometric tint for
heights in metres: blues under water, green at the coast, then ochre,
brown and white at 6000 m. Errors display their code first
(`export.io(..)` or the world error's own code). `export` panics on a
zero or odd `width`: callers check it (the CLI refuses it with
`cli.world.width`).

## Ports

None of its own: it implements the ports of the domain crates.

## Dependencies

Any domain crate. No domain crate may depend on it.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
