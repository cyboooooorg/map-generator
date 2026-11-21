# Planet draw (system draw PR 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `system::draw_with(inputs, trace, classify)` draws a whole system
at its present date: the star (PR 1), a family, its planets on sourced
orbits with their stellar sources (XUV and Lyman α included), spins and
designations, deterministic across platforms.

**Architecture:** Laws without randomness (Kepler, mean flux and
temperature factors, ice line, Roche period, mutual Hill spacing, K50) go
to `science::orbit`. The engine (`crates/system`) gets: a const table of
family numbers, the archetype placement half, per-family slot proposers,
one placement loop that classifies each trial through the injected
`classify` and enforces the class rules (#476), a spin draw, and `Drawn`.
Every value is a `Traced` leaf or node under `system.draw.*`.

**Tech Stack:** Rust 2024, `rand_chacha::ChaCha8Rng` streams from
`Seed::derive`, `timeline` types, `science` laws.

**Spec:** `docs/superpowers/specs/2026-11-21-system-draw-design.md`
(section "Planets of a random system" and after), with the study
`docs/superpowers/studies/2026-11-21-architecture-families/README.md`
(`S` = `small-planets-and-spacing.md`, `G` = `giant-planets.md` in the
same folder). Executors read both.

## Global Constraints

- No invented data: every numeric constant has a doc comment starting
  with `Source:`, `Calibration:` or `Definition:`; a `Definition:` only
  chooses *what to pick* (family, counts, flattenings, budgets), never a
  physical rule. `branch-reviewer` checks the engine too.
- Seeds: `seed.derive("star")` (PR 1), `derive("family")` for the family
  and its counts, `derive("trial/<archetype id>")` or
  `derive("trial/slot-<n>")` per trial (one stream per slot, retries
  continue it), `derive("spin/<designation text>")` for the spin, body
  seed `derive(<designation text>)`. Each draw reads its own stream.
- Cause codes start with `system.draw.`; drawn leaves use
  `Provenance::Drawn`, asked ones `Provenance::Input`, computed ones
  `trace.record(code, value, &[sources])`.
- `classify` is a parameter: `impl Fn(&Trial, &Trace) ->
  Result<TrialOutcome, SystemError>`. No stub in production code; tests
  pass a closure. `planet::class_of` stays mapgen1's.
- No `HashMap`/`HashSet` (clippy); `BTreeMap`. No new dependency.
- Crate README under 150 lines; details go to `docs/science`.
- User decisions (2026-11-21): families evenly; asked archetypes placed
  first and never moved; ice line `2.7 au × M/M☉`; giant eccentricity
  halves with one solar-like system in ten from the upper half (per
  system); counts uniform; every count range and that 1-in-10 in one
  const table module `system::families::table`; compact masses through
  the model's own mass-radius slope (two `classify` calls); spin from a
  short source study, no tidal locking (T2 #604).

## Review Focus

1. A request asking more archetypes than fit (e.g. 5 × `hot-jupiter`):
   expected `SystemError::NoRoom { archetype }`, never a panic or an
   infinite loop. Test in Task 8.
2. An 8 M☉ host or a 0.08 M☉ host with every family: the draw succeeds
   (families dropped only when impossible) and no orbit lies inside the
   star or its Roche floor. Test in Task 8.
3. Asked archetypes whose hosts disagree (`earth-like` G2V with
   `lava-world` K0V), or an asked host mass that contradicts an
   archetype's host: `SystemError::IncompatibleHost`. Test in Task 7.
4. A system with zero planets (sparse family, count 0): a valid `Drawn`
   with an empty placement list. Test in Task 8.
5. The same `Inputs` twice, and the draw with one archetype added:
   identical star and family streams (adding a draw never shifts
   another). Test in Task 10.

---

### Task 1: Spin study (docs only)

**Files:**

- Create: `docs/superpowers/studies/2026-11-21-architecture-families/spin.md`

**Interfaces:**

- Produces: the sourced distributions Task 9 implements: primordial
  obliquity (shape and range, per class if sources differ) and
  primordial rotation period or angular momentum law (rocky and giant).

- [ ] **Step 1:** Read what `legacy-final` did (`git show
  legacy-final:crates/mapgen/src/spin.rs`, `moons.rs` around
  `planet_day_hours`, `system.rs` around line 1650,
  `docs/science/axial-tilt-climate.md`) and list each value with its
  source or lack of one.
- [ ] **Step 2:** Find primary sources for: the obliquity distribution
  of planets formed by accretion (candidates: Miguel & Brunini 2010,
  Kokubo & Genda 2010 isotropic obliquities of giant-impact planets;
  Solar System giants), and the primordial spin rate (Kokubo & Genda
  2010 near critical rotation; Hughes 2003 / Bryan et al. 2018 spin of
  giants versus mass). Same reading-channel and page rules as the two
  sibling files.
- [ ] **Step 3:** End with a "Proposal" section: one distribution per
  quantity with its source, what is a `Definition:` flattening, and the
  open points. Lint: `npx markdownlint-cli2 <file>` → 0 issues.
- [ ] **Step 4:** Commit `docs(system): study the primordial spin of a
  drawn planet`. **Stop: the main session shows the proposal to the
  user before Task 9.**

### Task 2: Orbit laws in `science` (hard: physics)

**Files:**

- Create: `crates/science/src/orbit.rs`, `crates/science/src/orbit_tests.rs`
- Modify: `crates/science/src/lib.rs` (add `pub mod orbit;`),
  `crates/science/README.md` (two lines)

**Interfaces:**

- Produces, all pure:
  - `pub fn period(a: AstronomicalUnit, host: SolarMass) -> Day`
    (Kepler, `365.25 d × sqrt(a³/M)` with the Julian year of
    `constants`; planet mass neglected, stated).
  - `pub fn semi_major_axis(period: Day, host: SolarMass) -> AstronomicalUnit`
  - `pub fn circular_flux(luminosity: SolarLuminosity, a: AstronomicalUnit) -> EarthFlux` (`L/a²`).
  - `pub fn mean_flux_factor(e: Fraction) -> f64` = `1/sqrt(1-e²)`.
  - `pub fn mean_temperature_factor(e: Fraction) -> f64` =
    `1 - e²/16 - 15e⁴/1024` (Méndez & Rivera-Valentín 2017 eq. 16, as
    legacy `star.rs:707`).
  - `pub fn equilibrium(circular: EarthFlux, e: Fraction) -> Kelvin` =
    `255 K × S^¼ × mean_temperature_factor(e)` (`EARTH_EQUILIBRIUM`
    `Calibration:` 255 K at one Earth flux, Bond albedo 0.3, legacy
    `star.rs:322`).
  - `pub fn ice_line(host: SolarMass) -> AstronomicalUnit` = `2.7 au × M`
    (`Source:` Ida & Lin 2005 via Kennedy & Kenyon 2008, with the
    overshoot above 3 M☉ stated, study G 3.4).
  - `pub fn roche_period(density: KilogramPerCubicMetre) -> Day` =
    `12.6 h × (ρ / 1 g cm⁻³)^-½` (Rappaport et al. 2013, study S 6).
  - `pub fn mutual_hill_spacing(inner: Pair, outer: Pair, host: SolarMass) -> f64`
    with `pub struct Pair { pub mass: EarthMass, pub a: AstronomicalUnit, pub e: Fraction }`:
    `(a_out(1-e_out) - a_in(1+e_in)) / R_H`, `R_H` from the study's
    "Mutual Hill radius" block (He et al. eccentric form).
  - `pub const TWO_PLANET_FLOOR: f64` = `2√3` (`Source:` Gladman 1993
    via Fabrycky 2014, S 2.3).
  - `pub fn k50(age: Gyr, innermost: Day, sigma_e: f64, sigma_inc: Radian) -> f64`
    (Pu & Wu 2015 eq. 12 and 14, the study's block).

- [ ] **Step 1: Write the failing tests** in `orbit_tests.rs`:
  `earth_has_a_year` (`period(1 au, 1 M☉)` within 1e-9 of 365.25 d),
  `period_and_axis_invert` (round trip 1e-12 at 0.05, 5, 30 au and 0.3,
  8 M☉), `circular_orbit_factors_are_one`, `factors_match_legacy`
  (e = 0.2: flux factor 1.020621, temperature factor 0.997475),
  `equilibrium_of_earth_is_255`, `ice_line_is_linear` (1 M☉ → 2.7 au,
  8 M☉ → 21.6 au), `roche_period_of_unit_density_is_12_6_h`,
  `an_inflated_jupiter_has_a_roche_period_near_0_68_d` (1 M_J at
  1.3 R_J, ρ = 604 kg/m³ → 0.676 d ± 0.01; Dawson & Johnson quote
  0.79 d with another coefficient, stated in the doc; the hot-Jupiter
  floor at twice the Roche separation is `2^1.5 ×` this period),
  `hill_spacing_of_earth_and_mars_is_about_25` (circular: 1 and 0.107
  M⊕ at 1 and 1.524 au around 1 M☉: between 24 and 27; check the value
  against the formula by hand in the test comment),
  `eccentricity_shrinks_the_spacing`, `k50_is_about_10_at_1_gyr`
  (T1 = 10 d, σ_e = σ_inc = 0 → within 9-11; σ_e 0.02 adds 2).
- [ ] **Step 2:** `cargo test -p science orbit` → fails to compile.
- [ ] **Step 3:** Implement `orbit.rs` with the doc prefixes; units from
  `science::units`; add `KilogramPerCubicMetre` handling via `.0`.
- [ ] **Step 4:** `mise run ci` → all green (guardrails scan `science`).
- [ ] **Step 5:** Commit `feat(science): add the orbit laws of the system draw`.

### Task 3: Stellar sources at an orbit

**Files:**

- Create: `crates/system/src/sources.rs` (+ `#[cfg(test)] mod tests` inline)
- Modify: `crates/system/src/lib.rs` (`mod sources;`)

**Interfaces:**

- Consumes: `star::Star` (PR 1: `state: StarState`, `age: Traced<Gyr>`),
  `science::orbit::{circular_flux, mean_flux_factor, equilibrium}`,
  `science::xuv::{history, History, XUV, LYMAN_ALPHA}`.
- Produces:
  - `pub(crate) struct Light { history: History }` built once per star:
    `pub(crate) fn light(star: &Star) -> Result<Light, SystemError>`
    (calls `xuv::history(mass, metallicity, age)`).
  - `pub(crate) fn sources(star: &Star, light: &Light, a: &Traced<AstronomicalUnit>, e: &Traced<Fraction>, trace: &mut Trace) -> Result<StellarSources, SystemError>`:
    one `StellarSource` whose `flux` (code `system.draw.flux`, from a,
    e, luminosity), `equilibrium` (`system.draw.equilibrium`, from the
    flux node and e), `xuv.flux`/`xuv.fluence`
    (`system.draw.xuv-flux`, `system.draw.xuv-fluence`, from flux,
    mass, metallicity, age) and the Lyman α pair
    (`system.draw.lyman-alpha-flux`, `-fluence`) are recorded nodes;
    `effective_temperature`, `mass`, `luminosity` reuse the star's
    `Traced` values (same nodes).

- [ ] **Step 1: Failing tests:** `sun_at_one_au_gives_earth` (a test
  `Star` built from `star::star(Some(1 M☉), Some(0 dex),
  Some(4.5673 Gyr), ...)`: flux within 1 % of 1 Earth flux ×
  L_table, equilibrium within 1 K of 255 × L^¼, XUV flux within 1 % of
  4.6 mW/m² × L, fluence within 1 % of `EARTH_XUV_FLUENCE` × L),
  `eccentricity_raises_the_flux` (e = 0.3 → factor 1.0483),
  `an_a_star_has_no_xuv_but_lyman_alpha` (2.5 M☉),
  `the_flux_node_cites_the_orbit` (the flux node's `from` holds a's,
  e's and the luminosity's nodes).
- [ ] **Step 2:** run → fails. **Step 3:** implement. **Step 4:**
  `cargo test -p system sources` → pass.
- [ ] **Step 5:** Commit `feat(system): compute a planet's stellar sources`.

### Task 4: Draw skeleton, `TrialOutcome`, errors

**Files:**

- Modify: `crates/system/src/placement.rs` (add `TrialOutcome`),
  `crates/system/src/draw.rs` (replace the doc-only file),
  `crates/system/src/error.rs`, `crates/system/src/lib.rs`,
  `crates/system/src/star.rs` (drop its `expect(dead_code)`).

**Interfaces:**

- Produces:
  - `pub struct TrialOutcome { pub class: PlanetClass, pub bulk: Bulk }`
    in `placement.rs`, plain values: `class_of` runs on a scratch trace,
    so its nodes are not in the draw's trace. The draw records the class
    it keeps as `system.draw.class` with `trace.record` from the trial's
    mass, orbit and source nodes. Tell mapgen1 the final shape when the
    task lands (agreed: class + timeline `Bulk`).
  - `pub struct Drawn { pub designation: Designation, pub seed: Seed, pub star: StarState, pub age: Traced<Gyr>, pub family: Family, pub placements: Vec<Placement> }`.
  - `pub fn draw_with(inputs: &Inputs, trace: &mut Trace, classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>) -> Result<Drawn, SystemError>`
    (this task: star + empty placements; Body requests →
    `SystemError::BodyRequestNotDrawn` until the catalogue needs it).
  - `SystemError` new variants: `NoRoom { archetype: ArchetypeId }`,
    `IncompatibleHost { archetype: ArchetypeId }`,
    `UnknownArchetype { archetype: ArchetypeId }`,
    `BodyRequestNotDrawn`, `TooManyPlanets { count: usize }` (rank 650).
  - `pub enum Family { Compact, SolarLike, HotJupiter, EccentricGiants, Sparse }`
    (code names `compact`, `solar-like`, `hot-jupiter`,
    `eccentric-giants`, `sparse`), defined in Task 5's module, used here.
  - System number: `10_000 + seed.derive("designation").get() % 90_000`
    (`Definition:`; agreed with mapgen2: drawn from the seed, may
    collide, never a key; the catalogue keys on the hash of `Inputs`).

- [ ] **Step 1: Failing tests** `a_drawn_system_carries_its_star`
  (star values equal `star::star` on the same inputs, `placements`
  empty at this task), `the_system_number_is_in_range` (1 000 seeds),
  `a_body_request_is_refused`.
- [ ] **Step 2-4:** implement, run `cargo test -p system`.
- [ ] **Step 5:** Commit `feat(system): add the draw entry point and its errors`.

### Task 5: Family table and family pick

**Files:**

- Create: `crates/system/src/families/mod.rs`,
  `crates/system/src/families/table.rs`,
  `crates/system/src/families/tests.rs`

**Interfaces:**

- Produces:
  - `table.rs`: only documented consts, each with the study row it
    comes from. Counts (all `Definition:` uniform flattenings):
    `COMPACT_PLANETS: (u32, u32) = (4, 7)`,
    `SOLAR_LIKE_ROCKY: (2, 4)`, `SOLAR_LIKE_GIANTS: (1, 4)`,
    `HOT_JUPITER_COMPANION_CHANCE: f64 = 0.70` (`Source:` D&J p32),
    `ECCENTRIC_GIANTS: (1, 2)`, `ECCENTRIC_INNER_SMALL: (0, 2)`,
    `SPARSE_PLANETS: (0, 2)`, `SOLAR_LIKE_STIRRED_CHANCE: f64 = 0.1`
    (one solar-like system in ten draws its giants from the upper
    half), `SLOT_TRIES: u32 = 20` (`Definition:` tries budget per slot).
  - `pub(crate) fn pick_family(rng: &mut ChaCha8Rng, possible: impl Fn(Family) -> bool) -> Family`:
    uniform over the five, redrawn among the rest when impossible
    (the stream reads one uniform per attempt).
  - `pub(crate) fn innermost_possible(family: Family, star: &StarState) -> bool`:
    false when the family's smallest innermost period (Task 6 bounds)
    puts the orbit inside the stellar radius.

- [ ] **Step 1: Failing tests:** `families_are_even` (100 000 picks, each
  within 0.2 ± 0.01), `an_impossible_family_is_drawn_again`,
  `the_table_ranges_are_ordered` (every `(lo, hi)` has lo ≤ hi).
- [ ] **Step 2-4:** implement, `cargo test -p system families`.
- [ ] **Step 5:** Commit `feat(system): draw a system's architecture family`.

### Task 6: Slot proposers per family (hard: distributions)

**Files:**

- Create: `crates/system/src/families/{compact,solar_like,hot_jupiter,eccentric,sparse}.rs`,
  `crates/system/src/families/laws.rs` (shared sampled laws)
- Modify: `crates/system/src/random.rs` (add samplers)

**Interfaces:**

- Consumes: Task 2 laws, Task 5 table, `random::uniform`.
- Produces:
  - `random.rs`: `normal`, `log_uniform(lo, hi)`, `rayleigh(sigma)`,
    `half_gaussian(sigma)`, `gamma(shape)` (Marsaglia-Tsang),
    `beta(a, b)` (from two gammas), `beta_half(a, b, median, upper: bool)`
    (rejection until the draw is on the asked side), `log_normal`,
    `broken_power_law(break, index_below, index_above, lo, hi)` (inverse
    CDF per segment). Moved: `planet/draws.rs`'s private `normal` uses
    the shared one.
  - `laws.rs` (each with `Source:` text from the study): EPOS innermost
    period (peak 12 d, +1.6 / −0.9, S 1.2), EPOS ratio log-normal (log D
    −0.39, σ 0.18, ratio floor 1.2; D as defined in S 1.5, formula
    copied in the doc), near-resonance (0.29 per pair, within 5 %
    exterior to 3:2 or 2:1), Neil & Rogers model 1 mass (μ 0.29,
    σ 1.72 in ln M⊕, kept inside [0.1, 30) M⊕ by redraw: the 30 M⊕ cut
    is a `Definition:` split from giants), Fang & Margot spacing
    (offset 3.5, σ 14.5), Fulton a-law (break 3.6 au, +0.72/−0.86,
    support up to 30 au, scaled by `ice_line/2.7 au`), Rosenthal ratio
    (peak near 3, below 10: as S/G 3.3 states; a log-normal around 3
    truncated at 10 if the study gives no shape, `Definition:`), giant
    mass log-uniform ranges, Rosenthal Betas (single 0.60/1.45, multi
    1.06/3.68) with medians as `Calibration:` consts computed by a test,
    hot-Jupiter period log-uniform from max(2 a_Roche period, Task 2)
    to 10 d, Van Eylen half-Gaussian σ 0.32, compact Rayleigh σ_e
    uniform in [0.02, 0.035] per system.
  - Each family module: `pub(crate) fn propose(ctx: &FamilyContext, slot: u32, rng: &mut ChaCha8Rng, previous: Option<&Placed>) -> Proposal`
    with `pub(crate) struct Proposal { pub mass: EarthMass, pub a: AstronomicalUnit, pub kind: SlotKind }`
    (`SlotKind::{Small, Giant, HotGiant}`) and a per-family
    `pub(crate) fn count(rng) -> Counts`. `FamilyContext` holds the
    host mass, ice line, stellar radius, the system-level draws
    (compact system mass, σ_e, stirred flag) drawn once from the
    `family` stream.
  - Eccentricity, separate from the proposal (Task 8 draws it after
    classifying): `pub(crate) fn eccentricity(family: Family, class: PlanetClass, giants_in_system: u32, stirred: bool, rng) -> f64`.
    Giant classes: `GasGiant`, `HotJupiter`; `HotJupiter` → uniform
    [0, 0.1) (`Definition:`, D&J give no shape); small classes →
    family's small rule.

- [ ] **Step 1: Failing tests** (statistical, 50 000 draws, fixed
  seed): `beta_matches_its_mean` (a/(a+b) ± 0.005),
  `beta_halves_split_at_the_median`, `the_medians_are_the_constants`
  (numeric CDF), `innermost_period_peaks_near_12_days` (mode bin),
  `compact_ratios_stay_above_the_floor`, `neil_rogers_median_is_1_3`
  (± 0.05 before truncation), `fulton_axis_scales_with_the_ice_line`
  (median doubles when the host mass doubles),
  `hot_jupiter_periods_stay_in_their_band`,
  `counts_cover_their_table_ranges`.
- [ ] **Step 2-4:** implement, `cargo test -p system families random`.
- [ ] **Step 5:** Commit `feat(system): propose the slots of each family`.

### Task 7: Archetype placement half

**Files:**

- Modify: `crates/system/src/archetype.rs`
- Test: inline `#[cfg(test)] mod tests`

**Interfaces:**

- Produces:
  - `pub struct ArchetypePlacement { pub band: Band, pub mass: (EarthMass, EarthMass), pub eccentricity: Option<(f64, f64)>, pub obliquity: Option<Degree>, pub host: Option<Host> }`,
    `pub enum Band { Insolation(EarthFlux, EarthFlux), Period(Day, Day) }`,
    `pub struct Host { pub mass: SolarMass, pub age: Gyr }`.
  - `pub fn placement(id: &ArchetypeId) -> Option<&'static ArchetypePlacement>`
    over the 14 planet archetypes of `legacy-final`
    (`archetypes/*.toml`: mercury-like ... hot-neptune; the moons are
    T2): values from each file's `[inputs]` and `[orbit]`
    (`git show legacy-final:archetypes/<id>.toml`), a fixed value as a
    zero-width range. `Source:` per row is the file and, for the
    Solar-System rows, the NASA fact sheet the legacy file cites.
    Host masses of spectral types from the legacy star table
    (`legacy-final:crates/mapgen/src/star.rs`, Pecaut & Mamajek), age
    4.57 Gyr where the file sets it.
  - `pub(crate) fn asked_host(asked: &BTreeMap<ArchetypeId, u32>, host_mass: Option<SolarMass>, present: Option<Gyr>) -> Result<(Option<SolarMass>, Option<Gyr>), SystemError>`:
    merges the archetypes' hosts with the request; disagreement →
    `IncompatibleHost`; unknown id → `UnknownArchetype`.

- [ ] **Step 1: Failing tests:** `every_legacy_planet_archetype_is_placed`
  (14 ids), `earth_like_sits_at_one_earth_flux`,
  `two_archetypes_with_different_hosts_are_refused`,
  `an_asked_host_mass_must_match`, `an_unknown_id_is_refused`.
- [ ] **Step 2-4:** implement, `cargo test -p system archetype`.
- [ ] **Step 5:** Commit `feat(system): place the legacy planet archetypes`.

### Task 8: The placement loop (hard: algorithm)

**Files:**

- Create: `crates/system/src/draw/place.rs` (move `draw.rs` to
  `draw/mod.rs`), `crates/system/src/draw/tests.rs`

**Interfaces:**

- Consumes: Tasks 2-7.
- Produces: `draw_with` fills `placements` (before designations, Task
  10). Algorithm (spec "Order for each slot"):

  ```text
  slots = asked archetypes (sorted by id, then index) ++ family slots (inside out)
  for each slot, stream = derive("trial/<id>" | "trial/slot-<n>"):
    for try in 0..SLOT_TRIES:
      proposal from the band (archetype) or the family proposer
      a, mass leaves (Drawn or Input); e = 0; sources at (a, 0)
      trial -> classify -> outcome
      compact: second classify at 1.1 M -> slope dlnR/dlnM;
               mass = system mass * exp(N(0,1) * 0.31 / slope); rebuild the trial
      reject if a < stellar radius or period < roche_period(outcome.bulk.mean_density)
      e from the archetype or eccentricity(family, class, ...); sources at (a, e)
      reject on spacing against every placed planet: Delta >= TWO_PLANET_FLOOR
        when the system will hold 2 planets, >= k50(age, innermost period,
        family sigma_e, 1.4 deg) from 3; reject on hot-giant clearing:
        no other planet inside 1 au * ice_line / 2.7 au of a HotJupiter
      accept -> push, next slot
    budget spent: drawn slot -> left empty; asked -> drop the latest
      drawn neighbour in its way and retry once more, else NoRoom
  ```

  The rejected tries' trace nodes stay in the trace (the trace only
  grows); only accepted trials end up in a keyframe.

- [ ] **Step 1: Failing tests** with a test classifier (mass < 2 M⊕
  → `Rocky`, < 10 → `SuperEarth`, < 30 → `MiniNeptune`, else
  `GasGiant`, `HotJupiter` when also a < 0.1 au; bulk density 5500
  rocky / 1300 giant): `every_family_on_every_class_draws`
  (6 host masses 0.08, 0.3, 0.8, 1.0, 2.0, 8.0 × 5 families forced,
  200 seeds: no error, no orbit inside the star or its Roche period,
  every pair spaced), `five_hot_jupiters_find_no_room`
  (`NoRoom`), `an_asked_planet_is_never_moved` (its `a` equals its
  band draw), `a_sparse_system_may_be_empty`,
  `a_hot_jupiter_clears_its_neighbourhood`,
  `compact_planets_have_similar_masses` (ln-mass spread within one
  system below the spread across systems).
- [ ] **Step 2-4:** implement, `cargo test -p system draw`.
- [ ] **Step 5:** Commit `feat(system): place the planets of a drawn system`.

### Task 9: Spin draw (after the user approved Task 1)

**Files:**

- Create: `crates/system/src/spin.rs` (+ inline tests)

**Interfaces:**

- Produces: `pub(crate) fn spin(class: PlanetClass, mass: &Traced<EarthMass>, archetype: Option<&ArchetypePlacement>, seed: Seed, trace: &mut Trace) -> Result<Spin, SystemError>`
  with codes `system.draw.rotation-period`, `system.draw.obliquity`,
  drawing from `seed.derive("spin")`; an archetype's obliquity is
  `Provenance::Input`. Distributions and constants exactly as the
  approved Task 1 proposal.

- [ ] **Step 1:** failing tests from the proposal's numbers (mean and
  range of each distribution, archetype obliquity kept).
- [ ] **Step 2-4:** implement, `cargo test -p system spin`.
- [ ] **Step 5:** Commit `feat(system): draw the primordial spin of a planet`.

### Task 10: Designations, body seeds, determinism

**Files:**

- Modify: `crates/system/src/draw/mod.rs`, `crates/system/src/draw/tests.rs`
- Create: `crates/system/tests/determinism.rs`

**Interfaces:**

- Produces: placements sorted by `a`, rank 0 closest
  (`Designation::planet(system, rank)`, `TooManyPlanets` from 650),
  body seed `seed.derive(&designation.to_string())`, spin from Task 9.
  `draw_with` complete.
  - `fn digest(drawn: &Drawn) -> u64`: a canonical text of every value
    (family code, star values, each placement's designation, mass, a,
    e, period, ϖ, spin, flux, XUV) with each `f64` rounded to 9
    significant digits (`format!("{:.8e}")`), hashed with
    `timeline::fnv1a64`.

- [ ] **Step 1: Failing tests:** `designations_follow_distance`,
  `the_same_inputs_give_the_same_digest` (twice in one process),
  `golden_digests` (8 fixed `Inputs`: 3 free seeds, one per forced
  host class, one with `earth-like`, one with 2 × `hot-neptune`;
  goldens written on first run and pasted in), and
  `adding_an_archetype_keeps_the_star_and_family`.
- [ ] **Step 2-4:** implement, `cargo test -p system`; then `mise run ci`.
- [ ] **Step 5:** Commit `feat(system): designate the planets and pin the draw`.

### Task 11: Docs

**Files:**

- Create: `docs/science/system-architectures.md` (the five families,
  spacing, inner edge, ice line, spin, with sources and "Where it
  departs from the science")
- Modify: `crates/system/README.md` (draw section, under 150 lines),
  `docs/science/README.md` (index line),
  `docs/superpowers/specs/2026-11-21-system-draw-design.md` (mark
  delivered decisions; note `TrialOutcome` shape)

- [ ] **Step 1:** write; `npx markdownlint-cli2` on the changed files → 0 issues.
- [ ] **Step 2:** `mise run ci` → green.
- [ ] **Step 3:** Commit `docs(system): document the planet draw`.

---

After Task 11: `branch-reviewer` on the whole branch, fixes, tests, then
`pr-shepherd`. Message mapgen1 with the `TrialOutcome` shape as soon as
Task 4 lands, and mapgen2 if the system number answer changes Task 4.
