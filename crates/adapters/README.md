# `adapters`: the outside world

Implementations of the domain ports: filesystem storage with atomic
staging, PNG projections, SVG cross-sections, the English text catalogue
of codes and cause trees, relief files. The only library crate allowed to
read and write files.

## Filesystem store

`FsStore::new(root, budget)` implements `SystemRepository` and
`TimelineCache`. Layout under `root`:

```text
systems/<id>/inputs.json
cache/<engine>/<id>/timelines.json
cache/uses.json
```

Writes are staged in `<file>.tmp` then renamed. A cache file that cannot
be read or parsed is a miss. `cache/uses.json` holds a logical use
counter (no wall clock); `FsStore::evict(current)` deletes the entries of
other engine versions, then the least recently used ones until the cache
fits `budget` bytes. World files come with the world cache.

## Ports

None of its own: it implements the ports of the domain crates.

## Dependencies

Any domain crate. No domain crate may depend on it.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Crates.
