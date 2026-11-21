# `adapters`: the outside world

Implementations of the domain ports: filesystem storage with atomic
staging, PNG projections, SVG cross-sections, the English text catalogue
of codes and cause trees, relief files. The only library crate allowed to
read and write files.

## Filesystem store

`FsStore::new(root, budget)` implements `SystemRepository`,
`TimelineCache` and `WorldCache`. Layout under `root`:

```text
systems/<id>/inputs.json
cache/<engine>/<id>/timelines.json
cache/<engine>/<id>/worlds/<designation>/<keyframe>-<grid>.world
cache/uses.json
```

Writes are staged in `<file>.tmp` then renamed. A cache file that cannot
be read or parsed is a miss. `cache/uses.json` holds a logical use
counter (no wall clock); `FsStore::evict(current)` deletes the entries of
other engine versions, then the least recently used ones until the cache
fits `budget` bytes. World and timeline files are both listed. It also
deletes the cache entries of any system whose `inputs.json` no longer
exists, so removing a system frees its caches at the next eviction.

A `.world` file is the binary layout of `encode`/`decode` (see
`src/world_codec.rs`): magic `MGWORLD1`, then little-endian header and
fields. A truncated, corrupt or mismatching file is a cache miss.

PNG projections: `write_png(path, raster, ramp)` colours a `world::Raster`
through a `Ramp` (linear stops, `Ramp::grey` for a grey scale; NaN is
black) and writes an 8-bit RGB PNG with the same staged write as the
store.

## Ports

None of its own: it implements the ports of the domain crates.

## Dependencies

Any domain crate. No domain crate may depend on it.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
