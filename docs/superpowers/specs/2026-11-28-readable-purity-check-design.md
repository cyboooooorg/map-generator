# Readable purity check (#785)

## Intent

Issue #785 asked for two things: let the CLI report tests reuse a
generation already made, and make the failure of the purity check
readable. Only the second is done; the first was measured and dropped.

## Measurement (main at 0b880275, debug, 4 threads)

| Tier 2 test | Time |
|---|---|
| `planets::the_reference_planets` (`references()`, 8 systems) | 348.5 s |
| each of the 9 tests of `cli.rs` (reference 1) | 0.11 s |
| `contracts.rs`, 3 tests | 0.01 s |
| Tier 2 wall time | 349 s (budget 480 s) |

The CLI tests regenerate reference 1 on the three other threads while
`planets` runs: they add about 1 s of CPU and nothing to the wall time.

## Approaches

1. Purity check only (chosen). `cli.rs` stays as it is; its header says
   why it does not read `references()`.
2. One CLI test reading `references()[0]` through a new `report_from`:
   that process would pay the eight systems again (about 350 s of CPU)
   for no gain. Dropped.
3. The report checks inside `planets.rs`'s single test, through
   `report_from(&Reference)`: no extra generation, but at most 0.6 s of
   CPU saved, couples the report layer to the engine checks (the first
   failure stops the rest) and conflicts with #771's Task 6 in
   `report.rs` and `regression/src/lib.rs`. Dropped.

## Design

- `generation_is_pure_within_a_run` keeps the digest comparison of the
  draw, then compares the timelines one by one. A failure names:
  - a different number of bodies, with both counts;
  - otherwise the first body whose timeline differs (its designation),
    and the first field that differs, as a JSON pointer with both
    values, found by comparing `serde_json::to_value` of the two
    timelines;
  - a timeline unequal under `==` but equal once serialised (a NaN),
    said as such.
- The pointer search is a pure helper over `serde_json::Value`; each
  value in the message is cut to 80 characters, from 20 characters
  before the first difference when two values differ late. `serde_json`
  is already a dev-dependency.
- Two cheap tests of `planets.rs` pin the helpers: one on hand-written
  JSON values, one on hand-built timelines (`timeline::fixtures`)
  altered on purpose. Neither generates a system (CLAUDE.md, Tests).
- `cli.rs`'s header and the regression README record the measurement
  and why the CLI tests generate reference 1 themselves.
- #785 gets a comment with the measurement and the choice.

## Non-goals

- No `report_from`, no change to `report.rs` or `regression/src/lib.rs`.
- No change of what is generated: the engine output stays the same.
