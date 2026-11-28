# Readable purity check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A failure of `generation_is_pure_within_a_run` names the first
differing body and field instead of "the timelines differ".

**Architecture:** A pure helper finds the first JSON pointer where two
`serde_json::Value`s differ; a second helper compares two timeline lists
and reports through it; the purity check panics with its message.
Test-only change in `crates/regression/tests/planets.rs`, plus notes in
`crates/regression/tests/cli.rs` and `crates/regression/README.md`.

**Tech Stack:** Rust, `serde_json` (dev-dependency of `regression`),
nextest.

**Spec:** `docs/superpowers/specs/2026-11-28-readable-purity-check-design.md`

## Global Constraints

- No change to `crates/cli/src/report.rs` or `crates/regression/src/lib.rs`.
- Engine output unchanged: no digest or pin changes.
- The new tests never call `references()` (350 s per process in debug)
  and generate no system (CLAUDE.md, Tests): timelines are hand-built
  from `timeline::fixtures::earth_like_fixture`.

## Review Focus

- Arrays of different lengths: the pointer names the array and both
  lengths (test in Step 1).
- A key present on one side only: the missing side shows as `absent`
  (Step 1).
- Unequal under `==` but equal once serialised (NaN, which `serde_json`
  turns into `null`): said as such, not a silent pass (Step 1).
- A different number of timelines: both counts, before any per-body
  search (Step 1).
- A whole object on one side and a scalar on the other: the message
  stays short (each value cut to 80 characters) (Step 1).

---

### Task 1: Name the first difference in the purity check

**Files:**

- Modify: `crates/regression/tests/planets.rs` (two helpers, two tests,
  `generation_is_pure_within_a_run` at lines 114-126)
- Modify: `crates/regression/tests/cli.rs` (header, lines 1-10)
- Modify: `crates/regression/README.md` (purity, lines 14-16; `cli.rs`,
  lines 33-36)

**Interfaces:**

- Produces, private to `planets.rs`:
  - `fn first_difference(a: &Value, b: &Value) -> Option<String>`:
    `"<pointer>: <a> vs <b>"`. The root pointer is written `(root)`.
    Arrays of different lengths give `"<pointer>: length <n> vs <m>"`.
    A key missing on one side gives `absent` for that side. Objects are
    walked in the sorted union of their keys, arrays by index. Each
    value is its compact JSON cut to 80 characters (`…` appended), shown
    from 20 characters before the first difference when two values
    differ late (`…` prepended).
  - `fn timelines_difference(again: &[Timeline], shared: &[Timeline]) -> Option<String>`:
    `"<n> timelines vs <m>"` when the counts differ; otherwise, for the
    first index with `!=`, `"<body>: <first_difference>"`, or
    `"<body>: unequal but serialised the same (a NaN?)"` when
    `first_difference` finds nothing. `<body>` is the timeline's
    `Designation` (`Display`).

- [ ] **Step 1: Write the failing tests**

  `#[test] fn a_difference_names_its_pointer()`, with `serde_json::json!`:

  ```rust
  assert_eq!(first_difference(&json!({"a": [1]}), &json!({"a": [1]})), None);
  assert_eq!(first_difference(&json!({"a": {"b": [1, 2]}}), &json!({"a": {"b": [1, 3]}})).as_deref(), Some("/a/b/1: 2 vs 3"));
  assert_eq!(first_difference(&json!([1]), &json!([1, 2])).as_deref(), Some("(root): length 1 vs 2"));
  assert_eq!(first_difference(&json!({"x": 1}), &json!({})).as_deref(), Some("/x: 1 vs absent"));
  // an object against a scalar: the object's side is cut to 80 characters
  let long = json!({"k": "x".repeat(200)});
  let message = first_difference(&json!({"v": long}), &json!({"v": 1})).unwrap();
  assert!(message.starts_with("/v: {") && message.ends_with("… vs 1"), "{message}");
  ```

  `#[test] fn a_timeline_difference_names_the_body_and_the_field()`:
  build two planet timelines by hand (`earth_like_fixture`, one keyframe
  at 4.57 Gyr, designations ranks 1 and 2 of system 10001), take them as
  `shared`, and check on clones:
  - `timelines_difference(&shared, &shared) == None`;
  - one timeline dropped → `Some("1 timelines vs 2")` (counts taken from
    the slices, not literals);
  - `present` of `again[1]` changed (`Gyr(shared[1].present.0 + 1.0)`) →
    the message starts with `format!("{}: /present: ", shared[1].body)`;
  - `present` set to `Gyr(f64::NAN)` on both sides of index 1 → the
    message is `format!("{}: unequal but serialised the same (a NaN?)", shared[1].body)`.

- [ ] **Step 2: Run them, expect a compile failure (helpers missing)**

  `cargo nextest run -p regression --test planets -E 'test(a_difference_names_its_pointer) | test(a_timeline_difference_names_the_body_and_the_field)'`

- [ ] **Step 3: Implement `first_difference` and `timelines_difference`**

  Pointer segments follow RFC 6901 (`~` becomes `~0`, `/` becomes `~1`).
  Serialise with `serde_json::to_value(..).expect("a timeline serialises")`.

- [ ] **Step 4: Run the two tests, expect PASS**

  Same command as Step 2.

- [ ] **Step 5: Use it in `generation_is_pure_within_a_run`**

  Keep the digest assertion. Replace the bare `assert!` with
  `if let Some(difference) = timelines_difference(&again.timelines, &references()[0].timelines) { panic!("purity: {difference}") }`,
  and say in its doc comment that a failure names the first differing
  body and field.

- [ ] **Step 6: Notes in `cli.rs` and the README**

  - `cli.rs` header: its tests generate reference 1 themselves (0.11 s
    each, measured 2026-11-28) instead of reading `references()`, which
    would cost each process the eight systems (348 s in debug); this is
    why they are separate tests.
  - README: the purity check names the first differing body and field;
    the `cli.rs` paragraph gives the same reason in one sentence.

- [ ] **Step 7: Run tier 2 and the CI**

  `mise run ci`. Expected: everything passes, and tier 2 stays near
  349 s (before: `the_reference_planets` 348.5 s, wall 349 s; the two
  new tests add about 0.1 s each in their own processes). Note the
  after times for the PR.

- [ ] **Step 8: Commit**

  ```bash
  git add crates/regression docs/superpowers/specs/2026-11-28-readable-purity-check-design.md docs/superpowers/plans/2026-11-28-readable-purity-check.md
  git commit
  ```

  Message `test(regression): name the first difference in the purity check`,
  body with the measurement and the choice, footer `Refs: #785` (the PR
  says `Closes #785`).

- [ ] **Step 9: Comment on #785**

  The measurement, the three options and the choice (option 1), and
  that `report_from` is not done, with the reason.
