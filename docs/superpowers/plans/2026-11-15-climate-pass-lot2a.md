# Climate pass lot 2a: faithful archetypes — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On every seed, a Solar System archetype gives a body close to the
real one. Two changes do it:

- the NH₃ and S scatter factors can be pinned;
- an archetype can impose the system age, and the form shows it.

**Architecture:**

- `InputOverrides` gains `nh3` and `s`, carried through every layer that
  carries `ch4` today: archetype, request, CLI, server limits, web model
  fields and `formation()`.
- `ArchetypeInputs` gains `age_gyr`. A pure helper turns the asked
  archetypes' ages into one asked age. That age goes into the existing
  asked-age path (`resolve_age_gyr(Some(_))`, `draw_type_living`) for
  whole systems and lone worlds.
- The Solar System TOMLs then pin real values, checked by per-seed tests.

**Tech stack:** Rust (mapgen, server, cli crates; ts-rs bindings),
Svelte 5 with TypeScript (web, vitest).

**Spec:** `docs/superpowers/specs/2026-11-15-climate-pass-lot2a-design.md`.
Read it first.

## Global constraints

- Work in `~/Developer/mg-climate-archetypes`, branch
  `feat/climate-archetypes`.
- Never run `npm ci` there: `web/node_modules` is a symlink.
- Never use port 3000.
- Rust tests are throttled:
  `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`,
  one full run at a time. Prefer filtered runs (`-p mapgen <name>`)
  during the steps.
- The ts-rs bindings are written to `web/src/lib/api/types/` by
  `cargo test --all-features` (`.cargo/config.toml`). Commit the
  regenerated files.
- Web: `cd web && npx vitest run <file>`, and `npm run check` (svelte-check
  fails on warnings).
- No special case in `climate.rs` or `atmosphere.rs`. Faithfulness comes
  only from pinned inputs.
- An age the user typed always wins over an archetype's age.
- The Solar System age is `4.57` Gyr (Bouvier & Wadhwa 2010,
  doi:10.1038/ngeo941).
- Formats: system `SYSTEM_FORMAT_VERSION` 6 → 7
  (`crates/mapgen/src/system.rs:72`). World `FORMAT_VERSION` and
  `MIN_FORMAT_VERSION` 22 → 23 (`crates/mapgen/src/export/raw_data.rs:79,87`),
  plus `web/src/lib/formatVersion.ts`. The main session confirms these
  numbers with the peer sessions before Task 1 is committed. There is no
  migration.
- Each task updates the docs it touches in the same commit: README section
  "Archetypes and looks" (`grep -n '^### Archetypes and looks' README.md`,
  never read README whole), the doc header of `archetype.rs:11-57`, the
  CLI help, and the format history comments.
- Commits follow Conventional Commits. End each one with:

  ```text
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01NRxWVd48PkMKmKJPLqbXxK
  ```

## Review focus

1. **A whole system that names a star and asks for an age-fixing
   archetype**, for example `star = "F0V"` with `mars-like`. The star is
   honoured and the asked-age rules apply, so it is refused with
   `BadAge` when the star cannot reach 4.57 Gyr. The age is never
   silently dropped. Owned by Task 2.
2. **A drifted or pinned world** (`pinned()`, `request.rs:824`) of an
   archetype keeps nh3, s and its age when generated again. Owned by
   Tasks 1 and 2.
3. **A moon archetype asked in a whole system** (`titan-like`) also fixes
   the age, even though moons are placed after the star is chosen
   (`host_star_of`). Owned by Task 2.
4. **The lone-world form after switching from `mars-like` to
   `hot-jupiter`.** The archetype's age default and its note go away; an
   age the user typed stays. Owned by Task 4.
5. **Systems without archetypes stay byte-identical** apart from the
   format number. The nh3 and s overrides must not shift any draw. Owned
   by Task 1 (draw-order test) and checked in Task 5.

---

### Task 1: NH₃ and S factors can be pinned (sonnet)

**Files:**

- Modify: `crates/mapgen/src/planet/inputs.rs`
  - `InputOverrides` at `:229`.
  - `volatiles()` at `:413-425`.
  - Tests at `:478-545`.
- Modify: `crates/mapgen/src/planet/mod.rs` (`formation()` at `:255-300`).
- Modify: `crates/mapgen/src/archetype.rs`
  - `ArchetypeInputs` at `:342`.
  - `overrides()` at `:658`.
  - Validation, using `VOLATILE_FACTOR`.
- Modify: `crates/mapgen/src/request.rs`
  - `WorldRequest` field next to `ch4` at `:195`.
  - `validate` at `:459`.
  - `overrides` at `:493`.
  - `pinned` at `:839`.
- Modify: `crates/cli/src/main.rs`
  - Args at `:75`.
  - Request at `:503`.
  - Pinned flags at `:673`.
- Modify: `crates/server/src/api.rs` (`InputLimits` at `:71`).
- Modify: `crates/server/src/app.rs`
  - `form()` at `:500`.
  - `pins_model_input` at `:900`.
  - Tests at `:4811` and `:5831`.
- Modify: `crates/mapgen/src/system.rs:72` and
  `crates/mapgen/src/export/raw_data.rs:79,87` (format bumps and history
  comments).
- Modify: `web/src/lib/formatVersion.ts`.
- Modify: the regenerated `web/src/lib/api/types/*`.

**Interfaces:**

- Produces:
  - `InputOverrides { …, nh3: Option<f64>, s: Option<f64> }`. Both use
    `serde(default, skip_serializing_if = "Option::is_none")`, as `ch4`
    does.
  - `ArchetypeInputs { …, nh3: Option<ParamDefault>, s: Option<ParamDefault> }`.
  - `WorldRequest { …, nh3: Option<f64>, s: Option<f64> }`.
  - `InputLimits { …, nh3: Limits, s: Limits }`.
  - CLI flags `--nh3` and `--s`.
  - `Formation.overrides.nh3` and `.s` are always `Some` after
    `formation()`.

Steps:

- [ ] **Step 1: Write the failing tests** in `inputs.rs` tests:
  - `nh3_and_s_overrides_replace_the_scatter`: with
    `InputOverrides { nh3: Some(2.0), s: Some(0.5), ..Default::default() }`,
    the drawn volatiles equal `rule.nh3 * 2.0` and `rule.s * 0.5`.
  - `nh3_and_s_overrides_shift_no_other_draw`: every other input equals
    the no-override draw for the same rng seed. Extend
    `overrides_do_not_shift_the_other_draws` if that is simpler.
- [ ] **Step 2: Write the failing test** in `mod.rs` or `system.rs` tests:
  - `a_drifted_planet_keeps_its_n2` reproduces the #453 comment case:
    system seed 2, planet c, drifted.
  - It asserts the N₂ partial pressure is within 2 % of the original.
  - It asserts `formation().overrides.nh3` and `.s` are `Some`.
  - Use the existing drift helper; grep `drift` in `system.rs` tests.
- [ ] **Step 3: Write the failing test** in `request.rs`:
  `a_request_nh3_and_s_reach_the_overrides`. It mirrors
  `a_request_ch4_reaches_the_overrides` (`:1081`), including the
  `OutOfRange { field: "nh3" }` case at 1000.0.
- [ ] **Step 4: Run** `cargo test -p mapgen nh3` and
  `cargo test -p mapgen drifted_planet_keeps`. Expected: compile errors
  or FAIL.
- [ ] **Step 5: Implement the fields and their plumbing** in every file
  listed above. `volatiles()` uses
  `overrides.nh3.unwrap_or(scatter.nh3)` and
  `overrides.s.unwrap_or(scatter.s)`. `formation()` reads nh3 and s back
  as factors over the formation rule, exactly as it does for ch4 (1 where
  the rule is 0). Update its doc comment at `:255-263`: it now says the
  scatter follows the seed.
- [ ] **Step 6: Bump the formats.** System format 7 and world format 23,
  each with a history comment line ("Version 7 stores the NH₃ and S
  factors of a formation"). Do not commit before the main session has
  confirmed the numbers with the peer sessions.
- [ ] **Step 7: Run the tests and update the server tests.** Run
  `cargo test -p mapgen` and `cargo test -p server`. Server tests:
  `form["inputs"]["nh3"]` and `["s"]` exist, and the `:5831` loop
  covers `nh3` and `s`. Expected: PASS.
- [ ] **Step 8: Regenerate the bindings and check the web.** Run the full
  throttled workspace test to regenerate the bindings. Then run
  `cd web && npm run check`. Expected: PASS. Web code that switches over
  `ModelFieldKey` may need the new keys; that is Task 4's work.
- [ ] **Step 9: Commit.** `feat(planet)!: let archetypes and requests pin
  the NH3 and S factors`, with footer `BREAKING CHANGE: system format 7,
  world format 23.` and `Refs: #453`.

### Task 2: Archetypes impose the system age (sonnet)

**Files:**

- Modify: `crates/mapgen/src/archetype.rs`
  - `ArchetypeInputs.age_gyr`.
  - Validation.
  - A new `age_range` helper.
  - The doc header.
- Modify: `crates/mapgen/src/system.rs`: `place_whole` at `:960-990`, and
  `host_star_of` at `:849`.
- Modify: `crates/mapgen/src/request.rs`
  - `resolve_with_archetype` at `:537-588`.
  - `RequestError`.
- Modify: `crates/server/src/api.rs`: `ErrorDetail` at `:209` and
  `of_request` at `:277`.
- Modify: `web/src/lib/i18n/en.ts` and `fr.ts`: the refusal text next to
  `badAge` (`en.ts:1621`).

**Interfaces:**

- Consumes: nothing from Task 1.
- Produces:
  - `ArchetypeInputs { …, age_gyr: Option<ParamDefault> }`, validated
    in `(0, star::lifetime_gyr(0.0)]`.
  - `pub fn asked_age_gyr(archetypes: &[&Archetype], seed: u32) -> Result<Option<f32>, RequestError>`
    in `archetype.rs`. It intersects the asked archetypes' age ranges
    (`Fixed(x)` is `[x, x]`) and returns `None` when no archetype has an
    age. Otherwise it returns `Some`, drawn uniformly in the intersection
    from `stream_seed(seed, noise::stream::ARCHETYPE_AGE)`. Add that new
    stream constant next to `AGE = 41`. A fixed value draws nothing.
  - `RequestError::ArchetypeAges { archetypes: Vec<String> }`, returned
    when the intersection is empty.
  - `ErrorDetail::ArchetypeAges { archetypes }`, with EN and FR messages.

Steps:

- [ ] **Step 1: Write the failing tests** in `archetype.rs`. They use
  inline TOML archetypes through `Archetype::from_toml`, with
  `mass_earth = 1.0` and the age as shown:
  - `no_age_asked_gives_none`: archetypes without `age_gyr` return
    `Ok(None)`.
  - `fixed_ages_agree`: two archetypes with `age_gyr = 4.57` return
    `Ok(Some(4.57))`.
  - `ranges_intersect`: `[3.0, 5.0]` and `[4.0, 6.0]` return a value in
    `[4.0, 5.0]`, the same for the same seed.
  - `disjoint_ages_are_refused`: `4.57` and `[1.0, 2.0]` return
    `Err(ArchetypeAges { archetypes })`, naming both.
  - `an_age_out_of_the_stars_lives_is_refused_at_load`: `age_gyr = 0.0`
    or `1e5` gives `BadArchetype`.
- [ ] **Step 2: Write the failing tests** in `system.rs` tests (whole
  system). Each builds a `WholeSystemRequest` with `planets` and seeds
  0..5:
  - `an_asked_mars_sets_the_system_age`: with no `age_gyr`, the stored
    `star.info.age_gyr == 4.57`.
  - `an_asked_titan_sets_the_system_age`: the same with only
    `titan-like` (Review focus 3). Adapt `solar_aged_moon` (`:4942`)
    once the archetype carries the age.
  - `a_typed_age_wins_over_the_archetype`: `age_gyr: Some(2.0)` with
    `mars-like` gives 2.0.
  - `a_named_star_too_short_lived_is_refused`: `star: Some("O9V")`
    (choose one whose `lifetime_gyr < 4.57`) with `mars-like` gives
    `Err(BadAge { .. })` (Review focus 1).
  - `systems_without_archetypes_keep_their_age`: a drawn system's age is
    unchanged from before. Compare with the value read on main for seeds
    1..3, hard-coded.
- [ ] **Step 3: Write the failing tests** in `request.rs` tests (lone
  world):
  - `a_lone_mars_takes_its_archetype_age`: `planet: "mars-like"` with no
    age resolves to `age_gyr == 4.57`.
  - `a_typed_lone_age_wins`: `age_gyr: Some(3.0)` gives 3.0.
  - `a_pinned_lone_world_keeps_its_age` (Review focus 2): `pinned()` of
    the resolved world carries `age_gyr: Some(4.57)`.
- [ ] **Step 4: Run** `cargo test -p mapgen age`. Expected: FAIL.
- [ ] **Step 5: Implement `asked_age_gyr`** and the field.
- [ ] **Step 6: Wire `asked_age_gyr` in.**
  - In `place_whole`, collect the archetypes of `whole.planets`, moons
    included (load them as `host_star_of` and the `asked` loop do), then
    compute `let age = whole.age_gyr.or(asked_age_gyr(&archetypes, seed)?);`.
  - Use `age` wherever `whole.age_gyr` is read: `host_star_of`,
    `draw_type_living` and `resolve_age_gyr`.
  - In `resolve_with_archetype`, use
    `self.age_gyr.or(asked_age_gyr(&[archetype], seed)?)`.
  - Map the error in `api.rs` and add the EN and FR messages.
- [ ] **Step 7: Run** `cargo test -p mapgen` and `cargo test -p server`.
  Expected: PASS. The server test at `app.rs:5820` (earth age 4.6) is
  updated in Task 3, when earth-like gets its age.
- [ ] **Step 8: Document the change.** Update the doc header schema
  example (`archetype.rs:11-57`) and README "Archetypes and looks": an
  archetype may set the system's age, and a typed age wins.
- [ ] **Step 9: Commit.** `feat(planet): let an archetype set the system
  age`, with `Refs: #454`.

### Task 3: Solar System archetypes pin their body (opus: calibration)

**Files:**

- Modify: `archetypes/{mercury,venus,earth,mars,jupiter,saturn,uranus,neptune,io,europa,ganymede,callisto,titan,enceladus,triton}-like.toml`.
- Modify: `crates/mapgen/src/archetype.rs` tests
  - `AGE_GYR` and `model_of` at `:844-870`.
  - The tests at `:943-1170`.
- Modify: `crates/mapgen/src/planet/moon.rs` tests (`titan` at `:202`).
- Modify: `crates/server/src/app.rs:5820` (earth age).

**Interfaces:**

- Consumes: `nh3`, `s` (Task 1) and `[inputs] age_gyr` (Task 2).
- Produces: nothing new. TOML values only.

Steps:

- [ ] **Step 1: Make the test helpers use the archetype's age.**
  `model_of` and `moon_model_of` use
  `asked_age_gyr(&[a], seed)?.unwrap_or(AGE_GYR)`.
- [ ] **Step 2: Turn the median tests into per-seed tests**, all over
  seeds 0..20:
  - `earth_like_keeps_its_ocean_and_its_life`: every seed has
    `gases.total()` in `0.7..=1.3` bar. Delete the comment at
    `:1057-1063`.
  - `mars_like_is_a_barren_desert_under_a_thin_air`: every seed is in
    `0.004..=0.010` bar, `base_of == "barren"` and oceans < 0.01.
  - `venus_like_has_venus_air`: unchanged, 60..120 bar with CO₂ > 0.9.
  - New `titan_like_has_titan_air` (archetype path and
    `moon.rs::titan`): every seed is in `1.2..=1.8` bar, is a
    hydrocarbon world, and has a CH₄ mole fraction in `0.01..=0.10`.
  - New `solar_system_archetypes_carry_the_solar_age`: for the 15 names
    above, `inputs.age_gyr == Some(Fixed(4.57))`. For `hot-jupiter`,
    `hot-neptune`, `mini-neptune`, `super-earth`, `ocean-world` and
    `lava-world` it is `None`.
- [ ] **Step 3: Run** `cargo test -p mapgen archetype::` and
  `cargo test -p mapgen moon::`. Expected: the new tests FAIL.
- [ ] **Step 4: Pin each TOML.**
  - Add `age_gyr = 4.57` to the 15 Solar System archetypes, with the
    citation.
  - For earth, mars and titan, pin the fewest inputs that hold the range:
    real values first (mass, insolation, water, radiogenic), then volatile
    factors.
  - A volatile factor is a calibration with the real body as its target.
    The TOML comment says so, as `titan-like`'s `ch4` comment already
    does.
  - No change to `climate.rs` or `atmosphere.rs`.
- [ ] **Step 5: Run** `cargo test -p mapgen` (throttled). Expected: PASS,
  including `every_archetype_comes_out_as_its_class`,
  `solar_system_archetypes_are_recognised` and the `system.rs` Titan
  tests. Update `app.rs:5820` to 4.57 and run `cargo test -p server`.
- [ ] **Step 6: Commit.** `feat(planet): pin the Solar System archetypes
  to their body`, with `Closes: #453`, `Closes: #454` and `Refs: #464`.

### Task 4: Form shows NH₃, S and the archetype's age (sonnet)

**Files:**

- Modify: `web/src/lib/modelFields.ts`
  - `ModelFieldKey`.
  - The `volatiles` group: `['co2', 'n2', 'ch4', 'nh3', 's']`.
  - `LOG_FIELDS`.
- Modify: `web/src/lib/modelRequest.ts`
  - `PIN_DEFAULTS` `nh3: 1, s: 1`.
  - The loop at `:84`.
- Create: `web/src/lib/archetypeAge.ts` and `web/src/lib/archetypeAge.test.ts`.
- Modify: `web/src/components/GenerateForm.svelte` (`chooseArchetype` at
  `:235`).
- Modify: `web/src/components/form/ModelFields.svelte`.
- Modify: `web/src/components/form/SystemPreview.svelte:29`.
- Modify: `web/src/lib/i18n/en.ts` and `fr.ts`
  - Labels and units next to `ch4` at `:411`, `:450`, `:490`, `:803` and
    `:1640`.
  - `starLine`.
  - The age note and the warning.

**Interfaces:**

- Consumes: `ArchetypeInfo.inputs.age_gyr: ParamDefault | null`,
  `InputLimits.nh3` and `.s`, and the `ArchetypeAges` refusal (Tasks 1
  and 2).
- Produces:
  - `export function archetypeAge(a: ArchetypeInfo): number | null` returns
    the fixed value, or the range's midpoint for display.
  - `export function ageSources(archetypes: ArchetypeInfo[], asked: Record<string, number>): string[]`
    returns the localised names of the asked archetypes (count > 0) that
    carry an age, each once, in name order.

Steps:

- [ ] **Step 1: Write the failing tests** in `archetypeAge.test.ts`:
  - `archetypeAge` of a Fixed 4.57 is 4.57.
  - `archetypeAge` of a range [3, 5] is 4.
  - `archetypeAge` of an archetype without an age is null.
  - `ageSources` with `{ 'mars-like': 1, 'titan-like': 1, 'hot-jupiter': 2 }`
    lists Martienne and Titanienne once each.
  - `ageSources` with `{ 'mars-like': 0 }` is empty.
- [ ] **Step 2: Write the failing component tests**, next to
  `GenerateForm.test.ts` and `ModelFields.test.ts`:
  - Choosing `mars-like` shows the age default 4.57 with the note
    "fixé par Martienne" (FR) or "set by Mars-like" (EN).
  - Typing 2 in the age field shows the warning.
  - Switching to `hot-jupiter` removes the note but keeps a typed age
    (Review focus 4).
  - The `nh3` and `s` sliders render in the volatiles group.
  - `SystemPreview`: the star line appends the archetype names when
    `ageSources` is non-empty.
- [ ] **Step 3: Run** `cd web && npx vitest run src/lib/archetypeAge.test.ts src/components`.
  Expected: FAIL.
- [ ] **Step 4: Implement** the functions and the components. The FR
  copy:
  - note: `fixé par {nom}`;
  - warning: `{nom} a {âge} Ga : un autre âge change son air`;
  - star line suffix: `(fixé par {noms})`.

  The EN equivalents:

  - note: `set by {name}`;
  - warning: `{name} is {age} Gyr old: another age changes its air`;
  - star line suffix: `(set by {names})`.

  Show the `ArchetypeAges` refusal like `badAge`.
- [ ] **Step 5: Run** `cd web && npx vitest run && npm run check`.
  Expected: PASS with no warnings.
- [ ] **Step 6: Commit.** `feat(web): show the NH3 and S factors and the
  age an archetype sets`, with `Refs: #453` and `Refs: #454`.

### Task 5: Verification and coherence (main session)

- [ ] **Step 1: Run the full suite.**
  `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`,
  then `cd web && npx vitest run && npm run check`. Expected: all pass.
- [ ] **Step 2: Check coherence.** Ask `world-coherence-checker` to
  compare main against the branch on census seeds 1..50. Drawn systems
  without archetypes must be byte-identical, apart from the format
  number (Review focus 5). Archetype systems for earth, mars and titan
  report pressures per seed.
- [ ] **Step 3: Review the branch** with `branch-reviewer`, against the
  spec and this plan.
- [ ] **Step 4: Ship.** Open the PR with `pr-shepherd`. Its description
  says `Closes #453`, `Closes #454` and `Refs #464`. Then rebuild
  develop (warn the peers first) and wait for the user's test.
