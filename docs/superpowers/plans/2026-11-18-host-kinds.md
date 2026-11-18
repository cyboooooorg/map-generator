# Host Kinds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the host-kind data model of #569 (kinds, parser, state, inner
edge, quiet activity, draw table, `system.json` field) with every
non-dwarf kind unavailable, so only O/B hosts' inner edge changes worlds.

**Architecture:** A new module `crates/mapgen/src/host.rs` owns `HostKind`,
the extended parser, the state entry point, the inner edge and the draw
table. `star.rs` keeps `values(code)` and delegates `parse_type` to it;
`system.rs` stores the kind on `SystemStar` and threads the inner edge
through the orbit draws; `activity.rs` gets a kind-aware entry.

**Tech Stack:** Rust (crate `mapgen`), serde, ts-rs (bindings in
`web/src/lib/api/types`), rand `StdRng` streams.

**Spec:** `docs/superpowers/specs/2026-11-18-host-kinds-design.md`

## Global Constraints

- Worktree `~/Developer/cyboooooorg/mg-host-kinds`, branch `feat/host-kinds`.
  Never write to the main checkout's `systems/`, `worlds/`, `db/`.
- Tests throttled: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`,
  one full run at a time. Single test:
  `CARGO_PROFILE_TEST_OPT_LEVEL=0 cargo test -p mapgen <name>`.
- No `npm ci` in the worktree.
- Only `HostKind::Dwarf` is available. Every other kind is parsed and
  then refused with its issue number; nothing draws it.
- `habitable_fluxes`, `StarInfo.teff_k`, `StarInfo.luminosity` keep their
  signature and meaning. `StarInfo` gets no new field (so the raw-data
  `FORMAT_VERSION` stays 28).
- `SYSTEM_FORMAT_VERSION` = 12.
- New stream `stream::HOST_KIND = 44`.
- Commits: Conventional Commits, English, footer `Refs: #569`, plus
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. Hot-Jupiter clamp `clamp(INNER_EDGE_AU, 0.1)` (`system.rs:666`) panics
   if the host's inner edge exceeds 0.1 AU (O3V: 2 × 0.062 AU = 0.125).
   Must not panic; test in Task 3.
2. Ambiguous notations: `M` and `B` alone are dwarfs (M5V, B5V), `MAG` is
   a magnetar, `BH` a black hole, `G2` a dwarf. Tests in Task 1.
3. An asked planet orbit band entirely inside the new edge (O star,
   `orbit_au` 0.03) is refused with the existing `impossible()` path, not
   a panic. Test in Task 3.
4. A named non-dwarf type from the CLI, the server or an archetype
   `orbit.star` returns `BadStar` with the "not modelled yet" message
   (server maps it to `UnknownStar`). Test in Task 1 through `draw`.
5. Default draws do not consume `stream::HOST_KIND` while no non-dwarf
   kind is available, so seeds keep their systems. Test in Task 5.

---

### Task 1: `HostKind` and the extended parser

**Files:**
- Create: `crates/mapgen/src/host.rs` (declare `pub mod host;` in `crates/mapgen/src/lib.rs`)
- Modify: `crates/mapgen/src/star.rs:197-226` (`parse_type`)
- Test: `crates/mapgen/src/host.rs` (`mod tests`), `crates/mapgen/src/system.rs` (`mod tests`)

**Interfaces:**
- Produces:
  - `pub enum HostKind { Dwarf, Subgiant, Giant, WhiteDwarf, NeutronStar, Magnetar, BrownDwarf, BlackHole, WolfRayet, QuasiStar }`.
    It derives `Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize`, with `serde(rename_all = "snake_case")` and the ts export like `Envelope`.
  - `impl HostKind`:
    - `pub const ALL: [HostKind; 10]`
    - `pub fn available(self) -> bool` (only `Dwarf`)
    - `pub fn issue(self) -> Option<u32>`: Subgiant/Giant 576, WhiteDwarf 577, NeutronStar 578, Magnetar 579, BrownDwarf 580, BlackHole 581, WolfRayet 582, QuasiStar 583, Dwarf None.
    - `pub fn name(self) -> &'static str` (plural English noun for messages: "white dwarfs").
    - `pub fn has_convective_photosphere(self) -> bool`: true for Dwarf, Subgiant, Giant, BrownDwarf.
  - `pub struct HostType { pub kind: HostKind, pub code: f32 }`.
  - `pub fn parse_host(text: &str) -> Result<HostType, String>`: grammar only, availability not checked.
  - `star::parse_type(text) -> Result<f32, String>` keeps its signature. It calls `parse_host`. It returns an error for an unavailable kind: `spectral type {text:?}: {name} are not modelled yet (see #{issue})`.

Grammar of `parse_host`, checked in this order, case-insensitive, trimmed:

| Text | Kind | `code` |
|---|---|---|
| `PSR` | NeutronStar | 0 |
| `MAG` | Magnetar | 0 |
| `BH` | BlackHole | 0 |
| `WR`, `WN<n>`, `WC<n>`, `WO<n>` | WolfRayet | n or 0 |
| `QS` | QuasiStar | 0 |
| `D` + one of `A B O Q Z C` (+ optional number) | WhiteDwarf | number or 0 |
| `L`/`T`/`Y` + subtype 0 to 9.5 (default 5) | BrownDwarf | 70/80/90 + subtype |
| class `OBAFGKM` + subtype + suffix: none or `V` → Dwarf, `IV` → Subgiant, `I`/`Ia`/`Ib`/`II`/`III` → Giant | as listed | class × 10 + subtype |

Dwarf parsing keeps today's three error messages and `TYPE_RANGE` check
unchanged. Any other text keeps the first message
(`starts with one of O B A F G K M`).

- [ ] **Step 1: Write the failing tests** in `host.rs`:

```rust
#[test]
fn dwarf_notations_keep_their_codes() {
    for (t, c) in [("G2V", 42.0), ("g2", 42.0), ("m4.5", 64.5), ("K", 55.0), ("M", 65.0), ("B", 15.0)] {
        assert_eq!(parse_host(t).unwrap(), HostType { kind: HostKind::Dwarf, code: c }, "{t}");
    }
}

#[test]
fn other_kinds_are_recognised() {
    use HostKind::*;
    for (t, k) in [("G8IV", Subgiant), ("K3III", Giant), ("M2Iab", Giant), ("B1Ia", Giant),
                   ("DA", WhiteDwarf), ("DA2.4", WhiteDwarf), ("DZ", WhiteDwarf),
                   ("L5", BrownDwarf), ("T7", BrownDwarf), ("Y0", BrownDwarf),
                   ("PSR", NeutronStar), ("MAG", Magnetar), ("BH", BlackHole),
                   ("WR", WolfRayet), ("WN7", WolfRayet), ("QS", QuasiStar)] {
        assert_eq!(parse_host(t).unwrap().kind, k, "{t}");
    }
}

#[test]
fn malformed_types_are_refused() {
    for t in ["X2", "G12", "O1", "D", "L12", "G2VI", ""] {
        assert!(parse_host(t).is_err(), "{t}");
    }
}

#[test]
fn only_dwarfs_are_available() {
    for k in HostKind::ALL {
        assert_eq!(k.available(), k == HostKind::Dwarf);
        assert_eq!(k.issue().is_none(), k == HostKind::Dwarf);
    }
}
```

In `star.rs` tests, add:

```rust
#[test]
fn unavailable_kinds_name_their_issue() {
    let e = parse_type("DA").unwrap_err();
    assert!(e.contains("white dwarfs are not modelled yet") && e.contains("#577"), "{e}");
    assert!(parse_type("K3III").unwrap_err().contains("#576"));
}
```

In `system.rs` tests, add `a_named_white_dwarf_is_a_bad_star`. It calls
`draw(&SystemRequest::System(WholeSystemRequest { star: Some("DA".into()), seed: Some(1), ..Default::default() }), &mut StdRng::seed_from_u64(1))`
and asserts `Err(RequestError::BadStar(m))` with `m` containing `#577`.

- [ ] **Step 2: Run them**: `CARGO_PROFILE_TEST_OPT_LEVEL=0 cargo test -p mapgen host::` should fail to compile (no module).
- [ ] **Step 3: Implement `host.rs` and the delegating `parse_type`.** The existing `star.rs` parser tests (l.844-851) must pass unchanged.
- [ ] **Step 4: Run** `cargo test -p mapgen host:: star:: a_named_white_dwarf`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(mapgen): add host kinds and parse their spectral types`.

### Task 2: State entry point and `StarInfo` from values

**Files:**
- Modify: `crates/mapgen/src/host.rs`, `crates/mapgen/src/star.rs:575-640` (`StarInfo::new`, `for_planet`)
- Test: same files

**Interfaces:**
- Consumes: `HostKind` (Task 1), `star::values`, `star::StarValues`.
- Produces:
  - `pub fn host_state(kind: HostKind, mass: f32, age_gyr: f32) -> Result<StarValues, String>`.
    - For `Dwarf` it looks up the code whose `values(code).mass == mass` on the sequence (mass decreases monotonically with the code): bisection or a row scan with interpolation between rows. It returns `values(code)`.
    - It returns `Err` for a mass outside `[values(69.5).mass, values(3.0).mass]` and for unavailable kinds (same message as Task 1).
    - `age_gyr` is unused for dwarfs; document it.
  - `pub fn dwarf_code_for_mass(mass: f32) -> Option<f32>` (the inverse, public for #570).
  - `StarInfo::from_values(name: String, star: StarValues, orbit_au: f32, eccentricity: f32, mass_earth: f32, radius_earth: f32, age_gyr: f32, seed: u32) -> Self`.
    - It holds today's body of `for_planet`, with `values(code)` replaced by `star` and `type_name(code)` by `name`.
    - `for_planet(code, …)` becomes `from_values(type_name(code), values(code), …)`.
    - `new` is unchanged; it still goes through `for_planet` with the same key.

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn dwarf_state_inverts_the_sequence() {
    for code in [3.0, 15.0, 42.0, 55.5, 64.5, 69.5] {
        let v = values(code);
        let s = host_state(HostKind::Dwarf, v.mass, 4.6).unwrap();
        assert!((s.teff_k - v.teff_k).abs() / v.teff_k < 1e-3, "{code}");
        assert!((s.luminosity - v.luminosity).abs() / v.luminosity < 1e-3, "{code}");
    }
    assert!(host_state(HostKind::Dwarf, 500.0, 4.6).is_err());
    assert!(host_state(HostKind::WhiteDwarf, 0.6, 4.6).unwrap_err().contains("#577"));
}

#[test] // in star.rs
fn star_info_from_values_matches_new() {
    for code in [15.0, 42.0, 64.5] {
        let a = StarInfo::for_planet(code, 0.7, 0.1, 1.0, 1.0, 3.0, 7);
        let b = StarInfo::from_values(type_name(code), values(code), 0.7, 0.1, 1.0, 1.0, 3.0, 7);
        assert_eq!(a, b);
    }
}
```

- [ ] **Step 2: Run them**: they fail (missing functions).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p mapgen host:: star::`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(mapgen): describe a host from its mass and age`.

### Task 3: Host-dependent inner edge

**Files:**
- Modify: `crates/mapgen/src/host.rs`, `crates/mapgen/src/system.rs:92`, and the uses at 666, 693, 710, 829-830, 1093, 1096
- Test: `host.rs`, `system.rs` tests

**Interfaces:**
- Consumes: `StarValues`, `tides::roche_km(host_radius_km, host_density, density)` (km, g/cm³).
- Produces:
  - `pub const MIN_INNER_EDGE_AU: f64 = 0.02` (the old `INNER_EDGE_AU`, moved from `system.rs`).
  - `pub const ROCKY_DENSITY: f64 = 5.5`.
  - `pub const STELLAR_RADII_EDGE: f64 = 2.0`.
  - `pub fn inner_edge_au(star: &StarValues) -> f64`: the largest of `MIN_INNER_EDGE_AU`, `roche_km(R_km, ρ_host, ROCKY_DENSITY)` in AU, and `STELLAR_RADII_EDGE × R` in AU.
    - Host density in g/cm³ is `1.41 × mass / radius³` (solar units; use the crate's solar constants if present).
    - Doc comment: why (the closest known planets orbit at 2 to 3 R★), and that dwarfs from B7 to M stay at 0.02 AU.
  - Every function in `system.rs` that used `INNER_EDGE_AU` takes `inner_edge_au: f64`, computed once in `place_whole` from `values` of the star. Pass the value down; do not recompute it per call.
  - The hot-Jupiter clamp becomes `.clamp(inner, 0.1_f64.max(inner))`.

- [ ] **Step 1: Failing tests:**

```rust
#[test]
fn cool_dwarfs_keep_the_old_edge() {
    for code in [17.0, 30.0, 42.0, 69.5] { // B7 to M9.5
        assert_eq!(inner_edge_au(&values(code)), MIN_INNER_EDGE_AU, "{code}");
    }
}

#[test]
fn hot_stars_push_the_edge_outside_themselves() {
    let o3 = values(3.0);
    let r_au = f64::from(o3.radius) * 0.004_650_47;
    assert!(inner_edge_au(&o3) >= 2.0 * r_au - 1e-9);
    assert!(inner_edge_au(&o3) > 0.1); // exercises the hot-Jupiter clamp
}
```

In `system.rs` tests:

- `no_planet_orbits_inside_an_o_star`: draw `WholeSystemRequest { star: Some("O3V"), seed: s }` for seeds 1..=40. Every body's `orbit_au >= inner_edge_au(&values(3.0))`, and no panic.
- `an_asked_orbit_inside_an_o_star_is_impossible`: an asked planet at `orbit_au` 0.03 around `O3V` returns the existing impossible-orbit error. Use the request fields that the existing asked-orbit tests near `system.rs:1093` use.

- [ ] **Step 2: Run them**: they fail.
- [ ] **Step 3: Implement**, then run the seed-pinned O-star tests (`planetless_o_star_seeds`, `a_drawn_o_star_can_end_with_no_planet`, `a_whole_system_with_no_planet_is_a_star_alone`).
  - If their pinned seeds moved, re-pin them to the new first matching seed.
  - Say so in the commit body. This change to the worlds was decided by the user.
- [ ] **Step 4: Run** `cargo test -p mapgen system:: host::`. Expected: PASS.
- [ ] **Step 5: Commit** `fix(mapgen): keep planets outside hot stars with a host inner edge`.

### Task 4: Quiet-star activity

**Files:**
- Modify: `crates/mapgen/src/activity.rs:282-328`
- Test: `activity.rs` tests

**Interfaces:**
- Consumes: `HostKind::has_convective_photosphere` (Task 1).
- Produces:
  - `pub fn host_activity(kind: HostKind, info: &StarInfo, system_seed: u32) -> StarActivity`.
  - `star_activity(info, seed)` stays as `host_activity(HostKind::Dwarf, info, seed)`, so its outputs do not change.
  - For a kind without a convective photosphere, the envelope is forced to `Envelope::Radiative` and the function returns the existing `quiet` value: no spots, no granulation contrast, rotation from the radiative formula.
    - The four stream draws still happen in the same order.
    - Doc: each kind's issue gives its own rotation.

- [ ] **Step 1: Failing test:** `quiet_kinds_have_no_spots_or_granulation`.
  - Build `info = StarInfo::new(42.0, 1.0, 0.0, 4.6)`.
  - For `WhiteDwarf`, `NeutronStar`, `Magnetar`, `BlackHole`, `WolfRayet`, `QuasiStar`: `host_activity(k, &info, 3)` has `envelope == Radiative`, `spot_coverage == 0.0`, `granule_contrast == 0.0`, `activity == 0.0`.
  - `host_activity(HostKind::Dwarf, &info, 3) == star_activity(&info, 3)`.
- [ ] **Step 2: Run**: it fails.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p mapgen activity::`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(mapgen): give hosts without a photosphere a quiet activity`.

### Task 5: Draw table, stored kind, format 12, docs

**Files:**
- Modify:
  - `crates/mapgen/src/host.rs`
  - `crates/mapgen/src/noise.rs:200` (stream) and the test at 237-245
  - `crates/mapgen/src/system.rs`: `SystemStar` l.196, `place_whole` l.1021-1056, the build at l.1933-1940, `SYSTEM_FORMAT_VERSION` l.88, `read_system` l.446
  - `crates/mapgen/tests/old_format.rs` (only if it hardcodes 10 as current)
  - `web/src/lib/api/types/*` (regenerated)
  - `README.md` (l.70 `--star` row, l.276 `system.json` snippet, l.410 output tree if it lists star fields)
- Test: `host.rs`, `system.rs`, `noise.rs`

**Interfaces:**
- Consumes: Tasks 1, 2, 4.
- Produces:
  - `stream::HOST_KIND: u32 = 44` in `noise.rs`, documented like `GALAXY`.
  - `pub const HOST_KIND_SHARES: [(HostKind, f64); 10]`: relative shares per star from the study's section 2.1. Each value carries a comment with its source, and each kind's issue confirms its share before enabling it.
    - Dwarf: 0.93.
    - WhiteDwarf: 0.059 (Mamajek).
    - Subgiant: 0.0077 and Giant: 0.0033 (Mamajek: evolved 1.1 %, red giants 0.33 %).
    - BrownDwarf: 0.16 (Kirkpatrick 2021, lower bound).
    - NeutronStar: 1e-3 (order of one in a thousand, study `[M]`).
    - Magnetar: 1e-5.
    - BlackHole: 1e-3 (Olejak 2020, about 1.3 × 10⁸ against about 10¹¹ stars `[M]`).
    - WolfRayet: 1e-8 (Rosslowe & Crowther 2015).
    - QuasiStar: 0.0 (left to #583).
  - `pub fn draw_host_kind(system_seed: u32) -> HostKind`.
    - It sums the shares of available non-dwarf kinds. If the sum is 0, it returns `Dwarf` without seeding any rng.
    - Otherwise it draws from `StdRng::seed_from_u64(stream_seed(system_seed, stream::HOST_KIND))` over the available kinds.
  - `SystemStar.kind: HostKind` is a new field after `spectral`, serialised as `"kind"`.
  - In `place_whole`: with no named or archetype star, call `draw_host_kind(seed)`. For `Dwarf`, keep the existing `draw_type_living` path untouched (same `rng`, same order). A named star sets the kind from `parse_host`; only dwarfs can get there.
  - `read_system` derives a missing activity with `host_activity(star.kind, …)`.
  - `SYSTEM_FORMAT_VERSION = 12`.

- [ ] **Step 1: Failing tests:**

```rust
#[test] // host.rs
fn no_kind_but_dwarfs_is_drawn_yet() {
    for seed in 0..2_000 { assert_eq!(draw_host_kind(seed), HostKind::Dwarf); }
}

#[test] // host.rs
fn shares_cover_every_kind_once() {
    let mut kinds: Vec<_> = HOST_KIND_SHARES.iter().map(|(k, _)| *k).collect();
    kinds.dedup();
    assert_eq!(kinds.len(), HostKind::ALL.len());
    assert!(HOST_KIND_SHARES.iter().all(|(_, s)| *s >= 0.0));
}
```

In `system.rs`, add `a_drawn_system_stores_a_dwarf_kind`. It draws seed 5 with no star, round-trips through serde JSON, and finds `"kind":"dwarf"` and `star.kind == HostKind::Dwarf`.

In `noise.rs`, extend the uniqueness test to chain `AGE`, `HIGHLANDS`, `GALAXY`, `HOST_KIND`, `ARCHETYPE_AGE`.

- [ ] **Step 2: Run them**: they fail.
- [ ] **Step 3: Implement.** Regenerate the bindings with the throttled full test run (ts-rs writes on `cargo test`), and commit the generated `HostKind.ts` and `SystemStar.ts`.
  - Fix server tests only if they hardcode the version (they use `SYSTEM_FORMAT_VERSION + 1`, so they should not need it).
- [ ] **Step 4: Update the README.**
  - `--star` row: the notations accepted, and that only dwarfs (`O3V` to `M9.5V`) are modelled. The other kinds are refused with their issue number.
  - `system.json` snippet: the `kind` field.
- [ ] **Step 5: Full run** `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`. Then run `mise run lint` and `cd web && npm run check`. Expected: all pass, and `git status` shows no stray generated file. Rerun the flaky server index test alone if it fails.
- [ ] **Step 6: Commit** `feat(mapgen)!: store the host kind in system.json`, with the footer `BREAKING CHANGE: system.json format 12; older systems must be regenerated.`

### Task 6: Proof of unchanged worlds

- [ ] **Step 1:** Dispatch `world-coherence-checker`.
  - Binaries: `origin/main` and `feat/host-kinds`.
  - Inputs: 30 default seeds, plus named stars `B8V`, `G2V`, `M4V` (5 seeds each), plus `O5V` (5 seeds).
  - Expected: worlds byte-identical; systems identical apart from `format_version` and `star.kind`; `O5V` systems may differ (inner edge).
  - If a non-O/B system differs, stop and report.
- [ ] **Step 2:** Dispatch `branch-reviewer` with the branch, the spec and this plan.
- [ ] **Step 3:** Message mapgen1 about format 12 before opening the PR. Then hand off to `pr-shepherd` (PR body `Closes #569`).
