# Planet model PR 2a: draws, envelopes and giants — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the trace gaps of #617, draw the composition uniformly
between published bounds (#618), draw every planet's primordial
hydrogen-helium envelope, and give mini-Neptunes and giants a radius, a
layered cross-section and (for giants) an internal heat from published
tabulated models.

**Architecture:** Tables and laws go in `science` (`giants`, `envelope`,
`hydrogen`), read with `include_str!` like the stellar tables, free of
seeds and causes. The planet module of `system` draws from sub-seeds,
picks the model by mass and envelope, and records every value in the
`Trace`. Everything in `system::planet` stays crate-private; `class_of`
and `model` arrive with PR 2d.

**Tech Stack:** Rust 2024 workspace, `rand_chacha` (ChaCha8Rng), nextest
tiers via `mise`.

**Spec:** `docs/superpowers/specs/2026-11-21-planet-model-pr2a-design.md`
and its parent `docs/superpowers/specs/2026-11-21-planet-model-pr2-split-design.md`.
Studies (with the user's decisions at the end of each):
`docs/superpowers/studies/2026-11-21-planet-model-2a/draw-bounds.md` and
`giants-and-envelopes.md`. Previous plan, for conventions:
`docs/superpowers/plans/2026-11-21-planet-interior.md`. Issues #618
(closed by this PR), #621 (deferred integrated giants), #602.

## Global Constraints

- Every numeric constant in `science` has a doc comment starting with
  `Source:` (citation with table, equation or page), `Calibration:` (what
  it was fitted against, and a link to the page section that says what
  was searched) or `Definition:`. `tools/guardrails` fails otherwise. The
  same rule holds in `system` for draw parameters (`branch-reviewer`
  checks it).
- Never take a value from memory: every value is in one of the two
  studies with its source, or is read in the source by the implementer and
  cited. A missing value stops the task and is reported back.
- Expected values in tests come from sources, not from the code's output.
  A test of a calibrated rule says so in its name or comment.
- `science` knows no body, seed, trace or id. `system` draws only from
  `Seed::derive` sub-seeds through `ChaCha8Rng`, one sub-seed per drawn
  value, every draw made whatever the branch (the stream never depends on
  a branch).
- No file, network or process access at run time: tables are
  `include_str!` constants. Fetching the data is a one-off step of Task 1,
  documented in the provenance file.
- Tests on this shared machine: `mise run test:tier1` (throttled); one
  full run at a time. Single-test runs:
  `cargo nextest run --profile tier1 -p <crate> <filter>`.
- Earth radius in code is `EarthRadius` (IAU nominal equatorial,
  6378.1 km); radii at 1 bar in tests from the NASA fact sheet (study
  §1.6): Jupiter 71 492 km, Saturn 60 268 km, Neptune 24 764 km.
- One code path per value: a planet's radius, its layer boundaries and
  (for a giant) its internal heat come from one computation; no value is
  recomputed to explain it.
- The flux a model reads is `trial.sources.total_flux()`; its record
  cites every source's `flux` node.

## Decisions this plan applies

From the studies' decision sections (user, 2026-11-21) and the review of
this plan; the calls marked *(plan call)* are listed in the pull request
for the user to confirm.

| Draw or model | Rule |
| --- | --- |
| Core mass fraction | Uniform 0 to 0.8 |
| Dry water fraction | Log-uniform 1e-5 to 1e-2 |
| Icy water fraction | Uniform 0.1 to 0.67 |
| Water fraction | A share of the planet without its envelope: a mini-Neptune's core is `(1 − f) M` and holds `w (1 − f) M` of water |
| Migration chance | 0.1, calibration (unchanged) |
| Radiogenic abundance | Uniform −0.5 to +0.5 dex |
| Envelope below 0.1 M⊕ | None *(plan call: 0.1 M⊕ is the lowest mass Lammer et al. 2014 model; no sourced threshold exists, so `Calibration:`)* |
| Envelope 0.1 to 5 M⊕ | Log-uniform between the Lammer et al. 2014 min and max at the mass, interpolated log-linearly in mass between the grid masses 0.1, 0.5, 1, 2, 3, 5 M⊕, the max capped at 0.20 |
| Envelope 5 to 10 M⊕ | Log-uniform 8.7e-4 to 0.20 (the 0.35 of `draw-bounds.md` decision 3 is superseded by the cap below) |
| Envelope 10 to 20 M⊕ (ice giants) | Uniform 0.09 to 0.20 |
| Envelope cap below 20 M⊕ | 0.20, the edge of the Lopez & Fortney 2014 tables (user, 2026-11-21); it truncates Lammer's 3 and 5 M⊕ maxima (0.33, 0.35) |
| Envelope from 20 M⊕ (gas giants) | Heavy-element mass log-uniform within a factor 1.82² of Thorngren et al. 2016's `57.9 (M/M_J)^0.61` M⊕ (calibration), intersected with `core_bounds`; envelope `1 − M_z/M` |
| Mini-Neptune radius | Rocky core (the existing structure, water included, at zero surface pressure) plus the Lopez & Fortney 2014 envelope thickness, Tables 2 to 4 (solar metallicity), interpolated; below 1 M⊕ the table is read at 1 M⊕ *(plan call: keeps the radius continuous at 1 M⊕; underestimates a light planet's envelope)* |
| Giant radius | Fortney, Marley & Barnes 2007 `.dat` tables, interpolated, no inflation |
| Giant core radius | Fortney et al. 2007 Eq. 7 at ice mass fraction 0.5 (user: the tables' own core, one model; cores of 10 to 100 M⊕ are beyond `science::structure`'s fitted pressures, #616) |
| Giant internal heat | The table's internal luminosity `L` as the secular heat; radiogenic 0 (`Definition:` included in the table's `L`); tidal from `tidal_kind` with the envelope *(plan call: one model for the radius and the heat)* |
| Metallic hydrogen boundary | 100 GPa (calibration within 50 to 200 GPa), placed only in a giant above 50 M⊕ with a core below 25 % of its mass, by the n = 1 polytrope; every other enveloped planet has one gas envelope layer *(plan call: the study's thin-shell estimate gives a base pressure, not a radius)* |
| Steam worlds | Moved to 2d. 2a records water beyond `science::water`'s domain as such; the bulk then describes the condensed planet (dry mass, dry radius, its gravity and density), the water mass recorded apart |

## Review Focus

1. **A giant whose drawn heavy-element mass has no table row at its mass**
   (e.g. 20 M⊕ with a 25 M⊕ core, or a young, highly irradiated low-mass
   giant whose model the tables exclude): the draw bounds the core to
   `core_bounds`, and a missing (age, distance) cell falls back, per
   interpolation corner, to the nearest older age that has the row; never
   a panic, never an extrapolation. Test in Task 2 and Task 7.
2. **A planet just under and just over 0.1, 1, 5, 10 and 20 M⊕**: every
   regime boundary gives a finite envelope, a finite radius and the right
   model. Known jumps, stated in the science page: the envelope's lower
   bound at 10 M⊕ (8.7e-4 to 0.09), the envelope at 20 M⊕ (accepted by
   the user), and the radius at 20 M⊕ (two models, 20 mbar against
   1 bar, different cores). The radius is continuous at 1 M⊕. Test in
   Task 5 and Task 7.
3. **An envelope below the tables' lowest fraction (1e-4) or a planet
   below 1 M⊕ with an envelope**: the thickness goes linearly to zero
   below 1e-4, and below 1 M⊕ the table is read at 1 M⊕; never a NaN,
   never a negative thickness. Test in Task 3 and Task 7.
4. **A giant whose envelope thickness is zero or negative**
   (`R_tot ≤ R_core`): a typed error, not a clipped radius. Test in
   Task 7.
5. **Determinism**: the same seed gives bit-identical draws, envelope and
   radius; changing one sub-seed label moves only its own draw. Test in
   Task 4 and Task 5.

---

### Task 1: Data tables and their provenance

**Files:**

- Create: `crates/science/data/giants-fortney2007.tsv`,
  `crates/science/data/mini-neptunes-lopez2014.tsv`,
  `crates/science/data/GIANTS.md`
- Modify: `crates/science/README.md` (data section)

- [ ] **Step 1:** Download the 25 `tbl_Evpcore_<core>_<dist>.dat` files
  listed in study §1.2 into the scratchpad; record each file's sha256 and
  the retrieval date.
- [ ] **Step 2:** Convert them to one TSV with columns
  `core_me  dist_au  age_gyr  mass_me  radius_rj  luminosity_erg_s`
  (the `.dat`'s `L`, the internal luminosity), one line per row of every
  block, the `.dat` values kept exactly. Check: 25 files, 9 ages each,
  row counts as in study §1.2 (18 to 25 per block core-free, 21 core 25,
  18 core 50, 15 core 100). The misprint of the paper's Table 4 (study
  §1.2) is not in the `.dat`: the 9.5 AU, core 25, 464.2 M⊕ rows read
  1.013 (3.16 Gyr) and 0.985 (10 Gyr).
- [ ] **Step 3:** Transcribe Lopez & Fortney 2014 Tables 2, 3, 4
  (solar metallicity; 100 Myr, 1 Gyr, 10 Gyr) from the ar5iv HTML of
  arXiv:1311.0329 into a TSV with columns
  `age_gyr  flux_fe  mass_me  envelope_pct  radius_re`: 3 × 264 = 792
  lines. Cross-check five values against the journal PDF.
- [ ] **Step 4:** Write `GIANTS.md`: source of each file (citations,
  URLs as links, retrieval date, sha256 of every source file), the
  conversion command, the misprint note, the licence status (none stated
  by the author; the numbers are also printed in the paper, cited), the
  reference levels (1 bar for Fortney, 20 mbar for Lopez & Fortney) and
  the validity limits (study §1.3, §2.2).
- [ ] **Step 5:** `mise run md:lint` passes. Commit
  `feat(science): add the giant and mini-Neptune radius tables`.

### Task 2: Giant radius and luminosity in `science::giants`

**Files:**

- Create: `crates/science/src/giants.rs`, `crates/science/src/giants_tests.rs`
- Modify: `crates/science/src/lib.rs`, `crates/science/README.md`

**Interfaces:**

- Consumes: `giants-fortney2007.tsv` (Task 1).
- Produces:
  - `pub fn giant(mass: EarthMass, core: EarthMass, age: Gyr, flux:
    EarthFlux) -> Result<Giant, GiantError>`, where
    `pub struct Giant { pub radius: EarthRadius, pub luminosity: Watt,
    pub age_used: Gyr }` (`age_used` differs from `age` only when the
    fallback of Review Focus 1 applied).
  - `pub fn core_bounds(mass: EarthMass) -> (EarthMass, EarthMass)`: the
    lowest and highest tabulated core masses that have rows bracketing
    `mass` at every distance for every age from 0.1 Gyr (for the draw of
    Task 5).
  - `pub fn ice_rock_core_radius(mass: EarthMass) -> EarthRadius`:
    Fortney et al. 2007 Eq. 7 with ice mass fraction 0.5, `log` base 10
    (study §1.4).
  - `pub enum GiantError { OutOfTable { mass: EarthMass, core: EarthMass } }`.
  - `pub const GAS_GIANT_MIN_MASS: EarthMass` = 20 (Source: Thorngren et
    al. 2016's validity bound; `draw-bounds.md` decision 1).

- [ ] **Step 1: Write the failing tests** in `giants_tests.rs`:
  - `jupiter_at_9_5_au_and_4_5_gyr`: 318 M⊕, core 10, 4.5 Gyr, the flux
    of 9.5 AU around the Sun (`1 / 9.5²` F⊕): radius within 3 % of
    71 492 km (study §1.6: table 0.999 R_J).
  - `saturn_needs_a_core_between_10_and_25`: 95.1 M⊕ at 9.5 AU, 4.5 Gyr:
    radius at core 10 above 60 268 km and at core 25 below it (study
    §1.6; Mankovich & Fuller 2021, 19.1 M⊕ of heavy elements).
  - `a_hot_jupiter_matches_table_4`: 1 M_J (317.89 M⊕), core 0,
    0.045 AU around the Sun, 4.5 Gyr: within 1 % of the value printed in
    the paper's Table 4 for that cell (read it in the paper; study §1.6
    quotes 1.107 to 1.113 R_J from 1 to 1.5 M_J).
  - `the_ice_rock_core_matches_table_1`: `ice_rock_core_radius` at 10,
    31.6, 100 M⊕ within 2.5 % of 2.48, 3.23, 4.03 R⊕ (Eq. 7's stated
    accuracy).
  - `a_core_beyond_the_rows_is_refused`: 20 M⊕ with core 25 returns
    `OutOfTable`; `core_bounds(20 M⊕)` is (0, 10).
  - `a_missing_young_cell_falls_back_to_an_older_age`: pick a row
    absent at 1 Myr and 0.02 AU in the TSV; `age_used` is the nearest
    older age that has it.
- [ ] **Step 2:** `cargo nextest run --profile tier1 -p science giants` —
  compile failure.
- [ ] **Step 3:** Implement. Parse the TSV once (`std::sync::LazyLock`),
  a malformed line panics with its line number (as `stellar::tables`).
  Interpolate radius and `log L` linearly in `log mass`, `log age`,
  `core` and `log d_eq`, with `d_eq = 1 AU / sqrt(flux / F⊕)` (study
  §1.3). Clamp age to 0.001 to 10 Gyr and `d_eq` to 0.02 to 9.5 AU;
  refuse mass and core outside the rows; a missing corner takes the
  nearest older age that has its row.
- [ ] **Step 4:** Tests pass; guardrails report no undocumented number.
- [ ] **Step 5:** Commit `feat(science): interpolate giant radii from
  Fortney et al. 2007`.

### Task 3: Mini-Neptune envelope in `science::envelope`

**Files:**

- Create: `crates/science/src/envelope.rs`, `crates/science/src/envelope_tests.rs`
- Modify: `crates/science/src/lib.rs`, `crates/science/README.md`

**Interfaces:**

- Consumes: `mini-neptunes-lopez2014.tsv` (Task 1).
- Produces:
  - `pub fn envelope_thickness(total_mass: EarthMass, envelope: Fraction,
    flux: EarthFlux, age: Gyr) -> Result<EarthRadius, EnvelopeError>`:
    the table radius minus Lopez & Fortney's core radius
    `M_core^0.25` (their Eq. 1, `M_core = (1 − f) M`); the caller adds it
    to its own core. Below `MIN_MASS` the table is read at `MIN_MASS`.
  - `pub const LOWEST_TABULATED: Fraction` = 1e-4 and
    `pub const HIGHEST_TABULATED: Fraction` = 0.20 (Source: the tables'
    first and last columns).
  - `pub const MIN_MASS: EarthMass` = 1 and `pub const MAX_MASS:
    EarthMass` = 20 (Source: the tables' mass columns).
  - `pub enum EnvelopeError { OutOfTable { mass: EarthMass,
    envelope: Fraction } }`.

- [ ] **Step 1: Write the failing tests** in `envelope_tests.rs`:
  - `table_nodes_are_returned_exactly`: at three nodes of the paper's
    Table 3 (1 Gyr), `M_core^0.25 + envelope_thickness` equals the
    printed radius to 0.005 R⊕ (checks the transcription and the
    interpolation at nodes).
  - `between_nodes_the_radius_lies_between_them`: midway in each axis,
    the radius lies between the two printed neighbours.
  - `calibrated_thickness_goes_to_zero_below_the_tables` (a calibrated
    rule): at `f = 5e-5` the thickness is half its value at 1e-4, and 0
    at `f = 0`.
  - `a_light_planet_reads_the_1_earth_mass_row`: 0.5 M⊕ gives the same
    thickness as 1 M⊕ at the same `f`, flux and age.
  - `above_20_percent_is_refused`: `f = 0.25` returns `OutOfTable`.
- [ ] **Step 2:** Run — compile failure.
- [ ] **Step 3:** Implement: interpolate in `log mass`, `log f`,
  `log flux`, `log age`; clamp flux to 0.1 to 1000 F⊕ and age to 0.1 to
  10 Gyr; below `LOWEST_TABULATED`, scale the thickness at 1e-4 by
  `f / 1e-4` (doc comment `Calibration:`, linking the page section of
  Task 8).
- [ ] **Step 4:** Tests pass.
- [ ] **Step 5:** Commit `feat(science): interpolate mini-Neptune
  envelopes from Lopez & Fortney 2014`.

### Task 4: Composition draws and their trace (#618, gaps 1, 4, 5)

**Files:**

- Modify: `crates/system/src/planet/draws.rs`, `crates/system/src/planet/tests.rs`,
  `crates/system/src/planet/codes.rs`

**Interfaces:**

- Produces (changes to `Drawn`):
  - `water_fraction: Traced<Fraction>` becomes a drawn leaf
    (`trace.leaf(.., Provenance::Drawn)`, no inputs). The two flags stay
    recorded as their own nodes, no longer its inputs (gap 1).
  - `migrated: Option<Traced<bool>>`: `None` beyond the ice line, the
    uniform still consumed (gap 4); `is_water_world` reads the `Option`.
  - `formed_beyond_ice_line` computed once in `draw_with` and passed to
    `record` (gap 5).
  - `Drawn::fixed` (test helper) takes `migrated: Option<bool>`.

- [ ] **Step 1: Write the failing tests** in `tests.rs`:
  - `draws_stay_within_their_published_bounds`: over 2 000 sub-seeds,
    `core_mass_fraction` in [0, 0.8], dry water in [1e-5, 1e-2], icy water
    in [0.1, 0.67], radiogenic in [−0.5, 0.5].
  - `draws_reach_both_ends`: over the same seeds, each draw has values in
    its lowest and highest tenth of the range (log range for the dry
    water).
  - `the_water_fraction_is_a_drawn_leaf` and
    `no_migration_is_recorded_beyond_the_ice_line`.
  - `changing_one_label_moves_only_its_draw` stays green.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement: replace the normals with `uniform` (on the
  log of the bounds for the dry water); constants with `Source:` lines
  from `draw-bounds.md` §1 to §3; remove the Plotnykov median and scales,
  Marty's median and sigma and Nimmo's sigma from the code, and keep
  their citations in the science page (Task 8) for the galactic-map
  mode.
- [ ] **Step 4:** `mise run test:tier1` PASS.
- [ ] **Step 5:** Commit `feat(system): draw the composition between
  published bounds` with footer `Refs: #618`.

### Task 5: Envelope draw

**Files:**

- Create: `crates/system/src/planet/envelope.rs`
- Modify: `crates/system/src/planet/draws.rs` (`Drawn`, `LABELS`,
  `Drawn::fixed`), `crates/system/src/planet/codes.rs`,
  `crates/system/src/planet/mod.rs`, `crates/system/src/planet/tests.rs`

**Interfaces:**

- Consumes: `science::giants::{core_bounds, GAS_GIANT_MIN_MASS}`,
  `science::envelope::HIGHEST_TABULATED`.
- Produces:
  - `pub(crate) fn envelope_fraction(seed: Seed, mass: EarthMass) -> EnvelopeDraw`
    with `pub(crate) struct EnvelopeDraw { pub fraction: f64,
    pub heavy_mass: Option<EarthMass> }` (`Some` for a gas giant).
  - `Drawn` gains `envelope_fraction: Traced<Fraction>` (drawn leaf) and
    `heavy_mass: Option<Traced<EarthMass>>` (drawn leaf); label
    `planet/envelope`; `Drawn::fixed` takes the envelope fraction and the
    heavy mass.

- [ ] **Step 1: Write the failing tests:**
  - `envelope_bounds_by_mass`: for masses 0.05, 0.1, 1, 3, 5, 7, 10, 15,
    19.9, 20, 318 M⊕ over 500 seeds each, the fraction lies within the
    rule of the Decisions table (0 at 0.05; Lammer nodes 0.1 → [4.4e-8,
    1e-4], 1 → [8.1e-6, 0.025], 3 → [2e-4, 0.20], 5 → [8.7e-4, 0.20]).
  - `a_gas_giant_core_stays_in_the_tables`: for 20, 50, 318, 3 000 M⊕,
    `heavy_mass` within `core_bounds(mass)` and within the 1.82² band.
  - `every_regime_draws_the_same_number_of_uniforms` (determinism).
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Check the unit of `M` in Thorngren et al. 2016's fit on
  the paper (study §7 lists it as unchecked); a unit other than Jupiter
  masses stops the task. Then implement, each draw parameter with a
  `Source:` or `Calibration:` line (`draw-bounds.md` §4 and Decisions).
- [ ] **Step 4:** Tests pass.
- [ ] **Step 5:** Commit `feat(system): draw the primordial envelope`.

### Task 6: Interior trace gaps (2, 3, 6, 7)

**Files:**

- Modify: `crates/system/src/planet/interior.rs`, `crates/system/src/planet/heat.rs`,
  `crates/system/src/planet/codes.rs`, `crates/system/src/planet/tests.rs`

**Interfaces:**

- Produces:
  - `Water::Inventory` renamed `Water::BeyondLaw`, code
    `planet.water-beyond-law` (the existing code style).
  - `heat(trial, interior, radiogenic, envelope: &Traced<Fraction>,
    trace)`: the envelope fraction replaces the hard-coded `0.0` passed
    to `tidal_kind` (heat.rs).

- [ ] **Step 1: Write the failing tests:**
  - `central_pressure_cites_every_input`: its node's inputs include the
    mass, the core mass fraction, the water fraction, every layer mass
    and, when the water is beyond the law, its temperature (gap 2).
  - `the_mantle_state_is_a_constant`: its node is a leaf with
    `Provenance::Imposed`, no inputs (gap 3).
  - `water_beyond_the_law_is_not_icy`: a water world at 400 K gets the
    rocky k2/Q, and an enveloped planet the enveloped one (gap 6).
  - `water_beyond_the_law_bulk_is_the_condensed_planet`: `Bulk.mass` is
    the dry mass `(1 − w) M`, the radius, gravity and density are the
    dry planet's, and the water mass is recorded apart under
    `planet.water-beyond-law` (gap 7; the steam radius is 2d's).
- [ ] **Step 2:** Run — FAIL (the bulk test fails today: `Bulk.mass`
  holds the whole mass).
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `mise run test:tier1` PASS.
- [ ] **Step 5:** Commit `fix(system): record the interior's causes as
  they are`.

### Task 7: Enveloped interiors and their layers

**Files:**

- Create: `crates/science/src/hydrogen.rs` (with tests),
  `crates/system/src/planet/enveloped.rs`
- Modify: `crates/system/src/planet/interior.rs`,
  `crates/system/src/planet/heat.rs`, `crates/system/src/planet/mod.rs`,
  `crates/system/src/planet/tests.rs`, `crates/science/src/lib.rs`

**Interfaces:**

- Consumes: `science::giants::{giant, ice_rock_core_radius,
  GAS_GIANT_MIN_MASS}`, `science::envelope::envelope_thickness`, the
  `Drawn` fields of Tasks 4 and 5.
- Produces in `science::hydrogen`:
  - `pub const METALLIC_PRESSURE: Pascal` = 100 GPa (`Calibration:`
    within 50 to 200 GPa, study §5.1, the page section of Task 8).
  - `pub fn polytrope_radius_at(mass: Kilogram, radius: Metre,
    pressure: Pascal) -> Option<Metre>`: the n = 1 polytrope of study
    §5.2 (Hubbard 2013 Eqs. 61-62), bisection on `xi` in (0, π); `None`
    when the central pressure is below `pressure`.
- Produces in `system::planet`:
  - `interior.rs` splits out `pub(crate) fn rocky(mass: &Traced<EarthMass>,
    drawn: &Drawn, trial: &Trial, trace: &mut Trace) -> Result<Interior,
    InteriorError>`: today's body of `interior`, reading the mass from
    its argument instead of `trial.mass`. `interior` becomes the
    dispatcher: no envelope → `rocky(&trial.mass, ..)`; an envelope →
    `enveloped`.
  - `pub(crate) enum Body { Rocky(Interior), MiniNeptune { core:
    Interior, envelope: EnvelopeRecord }, Giant(GiantRecord) }`, each
    with `bulk()` and `layers()`; `EnvelopeRecord { mass, thickness }`
    and `GiantRecord { bulk, core_radius, layers, luminosity }` hold
    `Traced` values. `interior` returns `Body`; PR 2d's chain consumes
    it.
  - `pub(crate) fn enveloped(trial: &Trial, drawn: &Drawn, trace: &mut
    Trace) -> Result<Body, InteriorError>`:
    - below `GAS_GIANT_MIN_MASS`: a recorded core mass node
      `(1 − f) M`, then `rocky` on it (water included as a share of it),
      then `envelope_thickness`; total radius core plus thickness, bulk
      from the total mass and that radius;
    - from `GAS_GIANT_MIN_MASS`: one call of `giant` with `heavy_mass`,
      one of `ice_rock_core_radius`; `R_tot ≤ R_core` →
      `InteriorError::NoEnvelope`.
  - Layers: the core's shells (mini-Neptune) or one `RockIce` core of
    the heavy mass (giant); for a giant above 50 M⊕ with a core below
    25 % of its mass, an `Envelope` `Liquid` layer up to
    `polytrope_radius_at(.., METALLIC_PRESSURE)` (when `Some`) then an
    `Envelope` `Gas` layer; otherwise one `Envelope` `Gas` layer (50 M⊕
    and 25 % are `Calibration:` from study §5.2).
  - `heat` for a `Giant`: secular = the table's luminosity, radiogenic
    = 0 (`Definition:`), tidal from `tidal_kind` with the envelope; for a
    `MiniNeptune`, today's heat of the core.
  - `InteriorError` gains `NoEnvelope`, `Giant(GiantError)` and
    `Envelope(EnvelopeError)` with `From` impls.

- [ ] **Step 1: Write the failing tests:**
  - `jupiter_1_mbar_lies_in_debras_and_chabrier_region`:
    `polytrope_radius_at(Jupiter, 71 492 km, 100 GPa)` between 0.80 and
    0.93 of the radius (study §5.3).
  - `a_jupiter_trial`: 318 M⊕ at 5.2 AU around the Sun, 4.5 Gyr, heavy
    mass 10: radius within 3 % of 71 492 km; three layers in order
    (`RockIce`, `Envelope` `Liquid`, `Envelope` `Gas`); secular heat
    equals the table's luminosity at those inputs.
  - `a_neptune_trial`: 17.1 M⊕, envelope 0.15, at 30 AU, a fixed dry
    composition (`Drawn::fixed` with water 2.7e-3, Earth's core mass
    fraction 0.325): radius within −5 % to +15 % of 24 764 km (study §4);
    two layers groups (core shells, one gas envelope).
  - `earth_is_unchanged_without_an_envelope`: the PR 1 Earth test values
    hold with `envelope_fraction = 0`.
  - `a_negative_envelope_thickness_is_an_error` (Review Focus 4) and
    `regime_edges_are_finite_and_1_earth_mass_is_continuous`: trials at
    0.099, 0.101, 0.99, 1.01, 19.9, 20.1 M⊕ give finite radii, and the
    radii at 0.99 and 1.01 M⊕ differ by less than 2 % at equal
    composition and envelope (Review Focus 2 and 3).
  - `the_flux_record_cites_every_source`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement. Record the radius with the envelope
  fraction, heavy mass (giant), age and every source's flux node as
  inputs.
- [ ] **Step 4:** `mise run test:tier1` PASS.
- [ ] **Step 5:** Commit `feat(system): build the interiors of
  mini-Neptunes and giants`.

### Task 8: Science page, READMEs, checks and pull request (main session)

**Files:**

- Modify: `docs/science/planet-interiors.md`, `crates/science/README.md`,
  `crates/system/README.md`

- [ ] **Step 1:** In `docs/science/planet-interiors.md` (find sections
  with `grep -n '^#'`):
  - composition draws: the bounds, why uniform, the observed
    distributions kept for the galactic-map mode with their sources;
  - envelopes: the Lammer grid, the regimes, the caps, the jumps of
    Review Focus 2;
  - giants and mini-Neptunes: the tables, the interpolation, no
    inflation (#621), the hydrogen boundary, the internal heat;
  - under "Where it departs from the science": the core of a
    mini-Neptune integrated at zero surface pressure, a water-rich core
    under a thickness computed for a dry core, the 1 M⊕ floor of the
    tables, the steam worlds left to 2d;
  - a calibration section for each `Calibration:` constant of this plan,
    saying what was searched and not found.
- [ ] **Step 2:** `mise run ci` passes.
- [ ] **Step 3:** `branch-reviewer` and `test-reviewer` review the whole
  branch against the spec and this plan; fix, then re-run `mise run ci`.
- [ ] **Step 4:** Pull request (main session, `pr-shepherd`): title
  `feat(system): widen the planet draws and build envelopes and giants`,
  `Closes #618`, `Refs #602`, `Refs #621`; the departures-from-legacy
  table; calibrations under their own heading; the plan calls of the
  Decisions section for the user to confirm.
