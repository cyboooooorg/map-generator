# Planet draw (system draw PR 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `system::draw_with(inputs, trace, classify)` draws a whole system
at its present date: the star (PR 1), a family, its planets on sourced
orbits with their stellar sources (XUV and Lyman α included), spins and
designations, deterministic for given inputs.

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
same folder; `B9` = section B9 of
`docs/superpowers/studies/2026-11-21-planet-interior/heat-and-composition.md`).
Executors read both.

## Global Constraints

- No invented data: every numeric constant has a doc comment starting
  with `Source:`, `Calibration:` or `Definition:`; a `Definition:` only
  chooses *what to pick* (family, counts, flattenings, budgets, a shape
  where the source gives only a range), never a physical rule.
  `branch-reviewer` checks the engine too.
- Seeds, each draw on its own stream, so adding a draw never shifts
  another:
  - `seed.derive("star")` (PR 1), `derive("family")` for the family, its
    counts and the system-level draws;
  - trials: the first copy of an asked archetype `derive("trial/<id>")`
    (contract), copy k ≥ 2 `derive("trial/<id>/<k>")`; family slot n
    (0-based, counting family slots only, never asked archetypes)
    `derive("trial/slot-<n>")`; retries continue the slot's stream;
  - `derive("orbit/<trial label>")` for the perihelion longitude;
  - body seed `derive(<designation text>)`, spin from
    `body_seed.derive("spin")`.
- Cause codes start with `system.draw.`; drawn leaves use
  `Provenance::Drawn`, asked ones `Provenance::Input`, computed ones
  `trace.record(code, value, &[sources])`.
- `classify` is a parameter: `impl Fn(&Trial, &Trace) ->
  Result<TrialOutcome, SystemError>`. No stub in production code; tests
  pass a closure. `planet::class_of` stays mapgen1's.
- No `HashMap`/`HashSet` (clippy); `BTreeMap`. No new dependency. Items
  unused until a later task carry
  `#[cfg_attr(not(test), expect(dead_code, reason = "..."))]` as in
  `crates/system/src/planet/mod.rs`, removed when used.
- Crate README under 150 lines; details go to `docs/science`.
- User decisions (2026-11-21): families evenly; asked archetypes placed
  first and never moved; ice line `2.7 au × M/M☉`; giant eccentricity
  Beta halves with one solar-like system in ten from the upper half (per
  system); counts uniform; every count range and that 1-in-10 in one
  const table module `system::families::table`; compact masses through
  the model's own mass-radius slope (two `classify` calls); spin from a
  short source study, no tidal locking (T2 #604); system number drawn
  from the seed (agreed with mapgen2).
- Choices made in this plan (`Definition:` unless a source is named),
  listed in the PR for the user: small-planet masses from Neil & Rogers
  model 2 (B9 recommends model 2 or 3); the compact slope taken once per
  system; K50 applied only to pairs with a drawn planet; per-family σ_e
  for K50; 20 tries per slot; 1:1 odds between 3:2 and 2:1 for a
  near-resonant pair.

## Review Focus

1. A request asking more archetypes than fit (5 × `hot-jupiter`), or a
   huge count (4 000 000 000 × `super-earth`): `NoRoom` or
   `TooManyPlanets` before any allocation, never a panic or a hang.
   Tests in Task 8.
2. Hosts of 0.08 and 8 M☉ with every family forced: the draw succeeds
   (a family dropped only when its whole support lies inside the star),
   no periapsis inside the star or its Roche floor, no e ≥ 1. Tests in
   Task 8.
3. Asked archetypes whose hosts disagree (`earth-like` G2V with
   `lava-world` K0V): `IncompatibleHost`; an asked host mass with an
   archetype that names another host: the asked mass wins. Tests in
   Task 7.
4. A system with zero planets (sparse, count 0): a valid `Drawn` with no
   placement. Test in Task 8.
5. Adding a hostless archetype (`hot-neptune`) to a request keeps the
   star, the family and every drawn slot's stream. Test in Task 10.

---

### Task 1: Spin study (docs only)

**Files:**

- Create: `docs/superpowers/studies/2026-11-21-architecture-families/spin.md`

**Interfaces:**

- Produces: the sourced distributions Task 9 implements: primordial
  obliquity (shape and range, per class if sources differ) and
  primordial rotation period (rocky and giant).

- [ ] **Step 1:** List what `legacy-final` did (`git show
  legacy-final:crates/mapgen/src/spin.rs`, `moons.rs` around
  `planet_day_hours`, `system.rs` around line 1650,
  `docs/science/axial-tilt-climate.md`), each value with its source or
  lack of one.
- [ ] **Step 2:** Find primary sources for the obliquity of planets
  formed by giant impacts (candidates: Kokubo & Genda 2010, Miguel &
  Brunini 2010; the Solar System) and the primordial spin rate (Kokubo
  & Genda 2010; Bryan et al. 2018 for giants versus mass). Same
  reading-channel and page rules as the sibling files.
- [ ] **Step 3:** End with a "Proposal" section: one distribution per
  quantity, its source, what is a `Definition:` flattening, open
  points. `npx markdownlint-cli2 <file>` → 0 issues.
- [ ] **Step 4:** Commit `docs(system): study the primordial spin of a
  drawn planet`. **Stop: the main session shows the proposal to the
  user before Task 9.**

### Task 2: Orbit laws in `science` (hard: physics)

**Files:**

- Create: `crates/science/src/orbit.rs`, `crates/science/src/orbit_tests.rs`
- Modify: `crates/science/src/lib.rs` (`pub mod orbit;`),
  `crates/science/README.md` (two lines)

**Interfaces:**

- Produces, all pure:
  - `pub fn period(a: AstronomicalUnit, host: SolarMass) -> Day`:
    `2π sqrt(a³ / (G M))` with `SOLAR_MASS_PARAMETER` and
    `ASTRONOMICAL_UNIT` (planet mass neglected, stated).
  - `pub fn semi_major_axis(period: Day, host: SolarMass) -> AstronomicalUnit`
  - `pub fn circular_flux(luminosity: SolarLuminosity,
    a: AstronomicalUnit) -> EarthFlux` (`L/a²`).
  - `pub fn mean_flux_factor(e: Fraction) -> f64` = `1/sqrt(1-e²)`.
  - `pub fn mean_temperature_factor(e: Fraction) -> f64` =
    `1 - e²/16 - 15e⁴/1024` (Méndez & Rivera-Valentín 2017 eq. 16, as
    legacy `star.rs:707`).
  - `pub fn equilibrium(circular: EarthFlux, e: Fraction) -> Kelvin` =
    `255 K × S^¼ × mean_temperature_factor(e)`; `S` is the **circular**
    flux (the factor carries the eccentricity once). `EARTH_EQUILIBRIUM`
    is a `Calibration:` (255 K at one Earth flux, Bond albedo 0.3,
    legacy `star.rs:322`).
  - `pub fn ice_line(host: SolarMass) -> AstronomicalUnit` = `2.7 au × M`
    (`Source:` Ida & Lin 2005 via Kennedy & Kenyon 2008, overshoot above
    3 M☉ stated, G 3.4).
  - `pub fn roche_period(density: KilogramPerCubicMetre) -> Day` =
    `12.6 h × (ρ / 1000 kg m⁻³)^-½` (Rappaport et al. 2013, S 6).
  - `pub struct Pair { pub mass: EarthMass, pub a: AstronomicalUnit,
    pub e: Fraction }` and `pub fn mutual_hill_spacing(inner: Pair,
    outer: Pair, host: SolarMass) -> f64`:
    `(a_out(1-e_out) - a_in(1+e_in)) / R_H`, `R_H` as the study's
    "Mutual Hill radius" block.
  - `pub const TWO_PLANET_FLOOR: f64` = `2√3` (`Source:` Gladman 1993
    via Fabrycky 2014, S 2.3).
  - `pub fn k50(age: Gyr, innermost: Day, sigma_e: f64,
    sigma_inc: Radian) -> f64` (Pu & Wu 2015 eq. 12 and 14, the study's
    block; doc states its fit range σ_e ≤ 0.05).

- [ ] **Step 1: Failing tests** in `orbit_tests.rs`:
  `earth_has_a_year` (`period(1 au, 1 M☉)` = 365.2569 d ± 1e-3),
  `period_and_axis_invert` (round trip 1e-12 at 0.05, 5, 30 au and 0.3,
  8 M☉), `circular_orbit_factors_are_one`, `factors_match_legacy`
  (e = 0.2: flux factor 1.020621 ± 1e-6, temperature factor
  0.9974766 ± 1e-7), `equilibrium_of_earth_is_255`,
  `ice_line_is_linear` (1 M☉ → 2.7 au, 8 M☉ → 21.6 au),
  `roche_period_of_unit_density_is_12_6_h`,
  `an_inflated_jupiter_has_a_roche_period_near_0_68_d` (1 M_J at
  1.3 R_J, ρ = 604 kg/m³ → 0.6755 d ± 0.005; Dawson & Johnson quote
  0.79 d with another coefficient, stated in the doc),
  `venus_and_earth_are_26_hill_radii_apart` (circular, 0.815 and 1 M⊕
  at 0.723 and 1 au, 1 M☉: 26.3 ± 0.3) and
  `earth_and_mars_are_40_hill_radii_apart` (1 and 0.107 M⊕ at 1 and
  1.524 au: 40.1 ± 0.3; S 5), `eccentricity_shrinks_the_spacing`,
  `k50_is_about_10_at_1_gyr` (T1 = 10 d, σ_e = σ_inc = 0 → 10.26 ±
  0.05; σ_e 0.02 adds 2).
- [ ] **Step 2:** `cargo test -p science orbit` → fails to compile.
- [ ] **Step 3:** Implement `orbit.rs` with the doc prefixes.
- [ ] **Step 4:** `mise run ci` → green (guardrails scan `science`).
- [ ] **Step 5:** Commit `feat(science): add the orbit laws of the system draw`.

### Task 3: Stellar sources at an orbit

**Files:**

- Create: `crates/system/src/sources.rs` (inline `#[cfg(test)] mod tests`)
- Modify: `crates/system/src/lib.rs` (`mod sources;`)

**Interfaces:**

- Consumes: `star::Star` (`state: StarState`, `age: Traced<Gyr>`),
  `science::orbit`, `science::xuv::{history, History, XUV, LYMAN_ALPHA}`.
- Produces:
  - `pub(crate) struct Light { history: History }`, built once per star:
    `pub(crate) fn light(star: &Star) -> Result<Light, SystemError>`.
  - `pub(crate) fn sources(star: &Star, light: &Light,
    a: &Traced<AstronomicalUnit>, e: &Traced<Fraction>,
    trace: &mut Trace) -> Result<StellarSources, SystemError>`: one
    source; recorded nodes `system.draw.flux` (from a, e, luminosity:
    `circular_flux × mean_flux_factor`), `system.draw.equilibrium` (from
    a, e, luminosity: `equilibrium(circular_flux, e)`),
    `system.draw.xuv-flux`, `system.draw.xuv-fluence`,
    `system.draw.lyman-alpha-flux`, `system.draw.lyman-alpha-fluence`
    (from the flux node, the star's mass, metallicity and age);
    `effective_temperature`, `mass`, `luminosity` reuse the star's
    `Traced` values.

- [ ] **Step 1: Failing tests:** `sun_at_one_au_gives_earth` (star from
  `star::star(Some(1 M☉), Some(0 dex), Some(4.5673 Gyr), ..)`: flux =
  L_table ± 1e-9, equilibrium = 255 × L^¼ ± 0.01 K, XUV flux = 4.6e-3 ×
  L ± 1 %, fluence = `EARTH_XUV_FLUENCE` × L ± 1 %),
  `eccentricity_raises_the_flux` (e = 0.3 → × 1.0483),
  `an_a_star_has_no_xuv_but_lyman_alpha` (2.5 M☉),
  `the_flux_node_cites_the_orbit_and_the_star`.
- [ ] **Step 2-4:** run (fails), implement, `cargo test -p system sources`.
- [ ] **Step 5:** Commit `feat(system): compute a planet's stellar sources`.

### Task 4: Draw skeleton, families enum, `TrialOutcome`, errors

**Files:**

- Create: `crates/system/src/draw/mod.rs` (replaces `draw.rs`),
  `crates/system/src/draw/tests.rs`, `crates/system/src/families/mod.rs`
- Modify: `crates/system/src/placement.rs`, `crates/system/src/error.rs`,
  `crates/system/src/lib.rs`, `crates/system/src/star.rs` (drop its
  `expect(dead_code)` once `draw_with` calls it)

**Interfaces:**

- Produces:
  - `pub struct TrialOutcome { pub class: PlanetClass,
    pub radius: EarthRadius, pub mean_density: KilogramPerCubicMetre }`
    in `placement.rs`: plain values, since `class_of` runs on a scratch
    trace whose node ids mean nothing in the draw's trace. The draw
    records the kept class as `system.draw.class` (from the trial's
    mass, a and flux nodes). Message mapgen1 this shape when the task
    lands (it replaces the agreed `bulk: Bulk`).
  - `pub enum Family { Compact, SolarLike, HotJupiter, EccentricGiants,
    Sparse }` in `families/mod.rs`, codes `compact`, `solar-like`,
    `hot-jupiter`, `eccentric-giants`, `sparse`. Untraced, like the
    star class: the trace has no node type for engine enums; `Drawn`
    carries it.
  - `pub struct Drawn { pub designation: Designation, pub seed: Seed,
    pub star: StarState, pub age: Traced<Gyr>, pub family: Family,
    pub placements: Vec<Placement> }`.
  - `pub fn draw_with(inputs: &Inputs, trace: &mut Trace, classify:
    impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>) ->
    Result<Drawn, SystemError>`: this task draws the star and returns no
    placement; `Request::Body` → `BodyRequestNotDrawn`.
  - `SystemError` new variants: `NoRoom { archetype: ArchetypeId }`,
    `IncompatibleHost { archetype: ArchetypeId }`,
    `UnknownArchetype { archetype: ArchetypeId }` (moons included: they
    are T2), `BodyRequestNotDrawn`, `TooManyPlanets { count: u64 }`.
  - System number: `10_000 + seed.derive("designation").get() % 90_000`
    (`Definition:`; drawn from the seed, may collide, never a key).
  - `lib.rs` re-exports `Drawn`, `draw_with`, `Family`, `TrialOutcome`.

- [ ] **Step 1: Failing tests** `a_drawn_system_carries_its_star` (star
  values equal `star::star` on the same inputs, no placement),
  `the_system_number_is_in_range` (1 000 seeds),
  `a_body_request_is_refused`.
- [ ] **Step 2-4:** implement, `cargo test -p system draw`.
- [ ] **Step 5:** Commit `feat(system): add the draw entry point and its errors`.

### Task 5: Family table and family pick

**Files:**

- Create: `crates/system/src/families/table.rs`,
  `crates/system/src/families/tests.rs`
- Modify: `crates/system/src/families/mod.rs`

**Interfaces:**

- Produces:
  - `table.rs`, documented consts only, each naming its study row:
    `COMPACT_PLANETS: (u32, u32) = (4, 7)`, `SOLAR_LIKE_ROCKY = (2, 4)`,
    `SOLAR_LIKE_GIANTS = (1, 4)`, `ECCENTRIC_GIANTS = (1, 2)`,
    `ECCENTRIC_INNER_SMALL = (0, 2)`, `SPARSE_PLANETS = (0, 2)` (all
    `Definition:` uniform flattenings); `HOT_JUPITER_COMPANION_CHANCE:
    f64 = 0.70` (`Source:` D&J p32, G 1.5);
    `SOLAR_LIKE_STIRRED_CHANCE: f64 = 0.1` (`Definition:`, user);
    `SPARSE_GIANT_CHANCE: f64 = 0.5` (`Definition:`);
    `SLOT_TRIES: u32 = 20` (`Definition:`).
  - `pub(crate) fn pick_family(rng: &mut ChaCha8Rng,
    possible: impl Fn(Family) -> bool) -> Family`: uniform over the
    families still possible; `Sparse` is always possible (it may hold no
    planet), so the loop ends.
  - `pub(crate) fn possible(family: Family, star: &StarState) -> bool`:
    false only when the family's whole period support lies inside the
    star (outermost possible innermost orbit below the stellar radius):
    compact up to 640 d, hot Jupiter up to 10 d, the others up to the
    Fulton support. In practice true for every host of 0.08-8 M☉; the
    test pins it.

- [ ] **Step 1: Failing tests:** `families_are_even` (100 000 picks,
  each 0.2 ± 0.01), `an_impossible_family_is_drawn_again`,
  `sparse_is_always_possible`, `every_family_is_possible_on_every_host`
  (0.08, 0.3, 1, 2, 8 M☉ at their ZAMS and end of life),
  `the_table_ranges_are_ordered`.
- [ ] **Step 2-4:** implement, `cargo test -p system families`.
- [ ] **Step 5:** Commit `feat(system): draw a system's architecture family`.

### Task 6: Slot proposers per family (hard: distributions)

**Files:**

- Create: `crates/system/src/families/{compact,solar_like,hot_jupiter,eccentric,sparse,laws}.rs`
- Modify: `crates/system/src/random.rs` (samplers)

**Interfaces:**

- Consumes: Task 2 laws, Task 5 table, `random::uniform`.
- Produces:
  - `random.rs`: `normal` (Box-Muller, the same arithmetic as
    `planet/draws.rs:197`; leave mapgen1's copy in place, unify later
    with them), `log_uniform(lo, hi)`, `rayleigh(sigma)`,
    `half_gaussian(sigma)`, `gamma(shape)` (Marsaglia-Tsang),
    `beta(a, b)` (two gammas; redraw a result of exactly 0 or 1),
    `beta_half(a, b, median, upper)` (rejection until on the asked
    side), `log_normal(mu, sigma)`, `broken_power_law(break, below,
    above, lo, hi)` (inverse CDF per segment).
  - `laws.rs`, each with its `Source:` text:
    - EPOS innermost period: broken power law, peak 12 d, +1.6 below,
      −0.9 above (S 1.2), truncated below at the period of the stellar
      surface;
    - EPOS spacing: `log10 D` normal, mean −0.39, σ 0.18, period ratio
      `P2/P1 = ((2 + D)/(2 − D))^1.5` (S 1.5), redrawn when D ≥ 2 or the
      ratio < 1.2 (Weiss wall);
    - near resonance: with chance 0.29 per pair (He 2019, S 1.5) the
      ratio is drawn within 5 % exterior to 3:2 or 2:1, each with odds
      1/2 (`Definition:`);
    - small-planet mass: Neil & Rogers model 2, `ln M` normal μ 1.00,
      σ 1.65 (B9, Table 1 p7), redrawn outside [0.1, 30) M⊕; 30 M⊕ =
      0.1 M_J (`Source:` G 4.2, 1 M_J = 317.8 M⊕ NASA fact sheet,
      G line 333);
    - solar-like rocky planets: mass log-uniform 0.4-2.6 M⊕ (range:
      Raymond 2006, S 5; shape `Definition:`); first one at a period
      from the EPOS innermost law; next by Fang & Margot shifted
      Rayleigh in Hill radii (offset 3.5, σ 14.5, S 2.2), kept inside
      the ice line; e Rayleigh σ 0.05 (`Definition:` shape around
      Raymond's "about 0.05");
    - Fulton a-law: break 3.6 au, +0.72 / −0.86, up to 30 au, scaled by
      `ice_line / 2.7 au`, truncated below the ice line (G 3.1);
    - next giant: `ln(a2/a1)` uniform in [0.5, ln 10] (Rosenthal's
      sampling floor exp(0.5) = 1.65 and "less than 10", G 3.3; the
      uniform shape is a `Definition:`, the study gives no width);
    - giant masses log-uniform: solar-like 30-900 M⊕ (Fulton 2021,
      G 3.2), hot Jupiter 0.25-13 M_J (G 1.1), its companion 1-13 M_J
      at 1-20 au × `ice_line / 2.7 au` (G 1.5), eccentric giants 0.3-13
      M_J (G 2.4), sparse giant as eccentric giants;
    - hot-Jupiter period log-uniform from the stellar surface's period
      to 10 d (the Roche floor is checked after classifying, Task 8);
    - sparse: period log-uniform from the stellar surface's period to
      30 au × `ice_line / 2.7 au` (`Definition:` upper bound, G 3.1);
    - eccentric-giant inner small planets: periods from the EPOS
      innermost law, masses from Neil & Rogers, kept only where the
      eccentric Δ floor holds against the giant (G 2.4);
    - Betas: single giant (0.60, 1.45), multi-giant (1.06, 3.68)
      (Rosenthal 2023, G 2.1); their medians 0.220 and 0.183 as
      `Calibration:` consts checked by a numeric-CDF test.
  - Each family module: `pub(crate) fn counts(rng) -> Counts` and
    `pub(crate) fn propose(ctx: &FamilyContext, slot: u32,
    rng: &mut ChaCha8Rng, placed: &[Placed]) -> Proposal` with
    `pub(crate) struct Proposal { pub mass: EarthMass,
    pub a: AstronomicalUnit, pub kind: SlotKind }`
    (`SlotKind::{Small, Giant}`). `Placed` is defined here:
    `pub(crate) struct Placed { pub trial: Trial, pub outcome:
    TrialOutcome, pub label: String, pub asked: Option<ArchetypeId> }`.
    `FamilyContext` holds the host, ice line, stellar radius and the
    system-level draws from the `family` stream (compact system mass
    from Neil & Rogers, compact σ_e uniform in [0.02, 0.035] (He 2019,
    Mills 2019, S 3.1; uniform `Definition:`), the stirred flag, the
    companion flag).
  - Eccentricity, drawn after classifying (Task 8):
    `pub(crate) fn eccentricity(family: Family, class: PlanetClass,
    giants: u32, stirred: bool, rng) -> f64`, redrawn while ≥ 1, by
    class group:

    | Group | Classes | Rule |
    | --- | --- | --- |
    | hot giant | `HotJupiter` | uniform [0, 0.1) (`Definition:`, D&J give no shape, G 1.3) |
    | giant | `GasGiant`, `IceGiant` | the family's giant Beta (half or whole, as the table rows) |
    | small | every other class | compact: Rayleigh σ_e of the system; solar-like rocky: Rayleigh 0.05; sparse and eccentric-family small: half-Gaussian 0.32 (Van Eylen 2019 singles, S 4) |

  - `pub(crate) fn sigma_e(family, ctx) -> f64` for K50: compact the
    system's σ_e; solar-like 0.05; the others 0.05, the upper end of
    Pu & Wu's fit (`Definition:`, clamped to [0, 0.05]).

- [ ] **Step 1: Failing tests** (50 000 draws, fixed seeds, tolerances
  at 4σ): `beta_matches_its_mean` (a/(a+b) ± 0.005),
  `beta_halves_split_at_the_median`, `the_medians_are_the_constants`,
  `innermost_period_peaks_near_12_days`,
  `compact_ratios_stay_above_the_wall`,
  `neil_rogers_median_is_2_7` (exp(1.00) = 2.718 ± 0.06 before
  truncation), `fulton_axis_scales_with_the_ice_line`,
  `hot_jupiter_periods_stay_in_their_band`,
  `eccentricities_stay_below_one`, `counts_cover_their_table_ranges`.
- [ ] **Step 2-4:** implement, `cargo test -p system families random`.
- [ ] **Step 5:** Commit `feat(system): propose the slots of each family`.

### Task 7: Archetype placement half

**Files:**

- Modify: `crates/system/src/archetype.rs` (inline tests)

**Interfaces:**

- Produces:
  - `pub struct ArchetypePlacement { pub band: Band,
    pub mass: (EarthMass, EarthMass), pub eccentricity: Option<(f64, f64)>,
    pub obliquity: Option<Degree>, pub host: Option<Host> }`,
    `pub enum Band { Insolation(EarthFlux, EarthFlux), Period(Day, Day) }`
    (insolation is the **circular** flux `L/a²`, as legacy),
    `pub struct Host { pub mass: SolarMass, pub age: Option<Gyr> }`.
  - `pub fn placement(id: &ArchetypeId) ->
    Option<&'static ArchetypePlacement>` over the 14 planet archetypes
    of `legacy-final` (`git show legacy-final:archetypes/<id>.toml`;
    moons are T2 and stay unknown). A fixed value is a zero-width range.
    Inside a range: mass and insolation log-uniform, eccentricity
    uniform (`Definition:`). Host masses of spectral types from the
    legacy star table (`legacy-final:crates/mapgen/src/star.rs`, Pecaut
    & Mamajek), age 4.57 Gyr where the file sets it.
  - `pub(crate) fn asked_host(asked: &BTreeMap<ArchetypeId, u32>,
    host_mass: Option<SolarMass>, present: Option<Gyr>) ->
    Result<(Option<SolarMass>, Option<Gyr>), SystemError>`: archetypes
    with a count of 0 are skipped; two asked archetypes naming different
    hosts → `IncompatibleHost`; an asked mass or date wins over the
    archetype's host and age (spec, "a star named is honoured"); an
    archetype's age is kept only when the request names no date.

- [ ] **Step 1: Failing tests:** `every_legacy_planet_archetype_is_placed`
  (14 ids), `earth_like_sits_at_one_earth_flux`,
  `two_archetypes_with_different_hosts_are_refused`,
  `an_asked_host_mass_wins`, `an_asked_date_wins`,
  `a_count_of_zero_is_ignored`, `a_moon_or_unknown_id_is_refused`.
- [ ] **Step 2-4:** implement, `cargo test -p system archetype`.
- [ ] **Step 5:** Commit `feat(system): place the legacy planet archetypes`.

### Task 8: The placement loop (hard: algorithm)

**Files:**

- Create: `crates/system/src/draw/place.rs`
- Modify: `crates/system/src/draw/mod.rs`, `crates/system/src/draw/tests.rs`

**Interfaces:**

- Consumes: Tasks 2-7.
- Produces: `pub(crate) fn place(..) -> Result<Vec<Placed>, SystemError>`
  called by `draw_with` (Task 10 turns `Placed` into `Placement`).
  Algorithm (spec "Order for each slot", with the review's fixes):

  ```text
  total = Σ asked counts + family count; total >= 650 -> TooManyPlanets (before any Vec)
  compact only: slope = dlnR/dlnM of the model at the system mass
    (classify a probe trial at M and 1.1 M at the first slot's proposal);
    slope <= 0 or Err -> drop the scatter (every planet at the system mass)
  slots = asked archetypes (id order, copies 1..k) ++ family slots 0..n (inside out)
  for each slot, its own stream:
    for try in 0..SLOT_TRIES:
      proposal (band draw for an archetype, family proposer otherwise)
      compact: mass = system mass × exp(N(0,1) × 0.31 / slope), redrawn outside [0.1, 30)
      leaves: mass, a (Drawn or Input); e = 0; ice line node (once per system)
      trial at e = 0 -> classify -> outcome (Err -> spent try; asked: propagate)
      e from the archetype or eccentricity(); perihelion longitude uniform [0, 360)
      q = a (1 - e)
      reject if q < stellar radius, or q < a_Roche(outcome.mean_density)
        (HotJupiter: q < 2 a_Roche), with a_Roche = semi_major_axis(roche_period(ρ), M)
      sources at (a, e); rebuild the trial
      reject on spacing against every placed neighbour:
        Δ >= TWO_PLANET_FLOOR always;
        Δ >= k50(age, T1, sigma_e, 1.4°) when total >= 3 and the pair has a drawn planet
        (T1 = smallest period among placed planets and the trial)
      reject on clearing: no planet inside 1 au × ice_line / 2.7 au of a HotJupiter
        other than itself (Huang 2016, G 1.4)
      accept -> push, next slot
    budget spent: drawn slot -> left empty; asked -> NoRoom
  after all slots: re-check adjacent pairs with the final T1;
    drop violating drawn planets from the outside in (asked never dropped)
  ```

  Asked archetypes go first, so they never meet a drawn neighbour: the
  spec's "drop a drawn neighbour" step cannot occur; note it in the spec
  (Task 11). Asked-asked pairs face only `2√3`: the Solar System's own
  archetypes (Jupiter-Saturn Δ ≈ 6.5) must stay placeable. Rejected
  tries' nodes stay in the trace; only kept trials reach a keyframe.

- [ ] **Step 1: Failing tests** with a test classifier (mass < 2 M⊕ →
  `Rocky`, < 10 → `SuperEarth`, < 30 → `MiniNeptune`, else `GasGiant`,
  `HotJupiter` when also a < 0.1 au; radius from a fixed density 5 500
  kg/m³ small, 1 300 giant; for the slope, radius ∝ M^0.27 small):
  - `every_family_draws_on_every_host` (0.08, 0.3, 0.8, 1, 2, 8 M☉ ×
    5 families forced × 20 seeds: no error, no periapsis inside the
    star or its Roche floor, every e < 1, every pair ≥ 2√3), plus the
    same grid with 200 seeds under `#[ignore = "slow: full grid"]`;
  - `five_hot_jupiters_find_no_room` (`NoRoom`);
  - `a_huge_count_is_refused_at_once` (`TooManyPlanets`);
  - `an_asked_planet_is_never_moved`;
  - `the_solar_giants_can_be_asked_together` (jupiter-, saturn-,
    uranus-, neptune-like);
  - `a_sparse_system_may_be_empty`;
  - `a_hot_jupiter_clears_its_neighbourhood`;
  - `compact_planets_have_similar_masses` (spread of ln M within a
    system below the spread across systems);
  - `a_classifier_error_on_an_asked_planet_propagates`.
- [ ] **Step 2-4:** implement, `cargo test -p system draw`.
- [ ] **Step 5:** Commit `feat(system): place the planets of a drawn system`.

### Task 9: Spin draw (after the user approved Task 1)

**Files:**

- Create: `crates/system/src/spin.rs` (inline tests)

**Interfaces:**

- Produces: `pub(crate) fn spin(class: PlanetClass,
  mass: &Traced<EarthMass>, archetype: Option<&ArchetypePlacement>,
  body_seed: Seed, trace: &mut Trace) -> Result<Spin, SystemError>`,
  drawing from `body_seed.derive("spin")`, codes
  `system.draw.rotation-period`, `system.draw.obliquity`; an
  archetype's obliquity is `Provenance::Input`. Distributions and
  constants exactly as the approved Task 1 proposal.

- [ ] **Step 1:** failing tests from the proposal's numbers (mean and
  range of each distribution; archetype obliquity kept).
- [ ] **Step 2-4:** implement, `cargo test -p system spin`.
- [ ] **Step 5:** Commit `feat(system): draw the primordial spin of a planet`.

### Task 10: Designations, body seeds, determinism

**Files:**

- Modify: `crates/system/src/draw/mod.rs`, `crates/system/src/draw/tests.rs`
- Create: `crates/system/tests/determinism.rs`

**Interfaces:**

- Produces: `Placed` sorted by `a`, rank 0 closest
  (`Designation::planet(system, rank)`), body seed
  `seed.derive(&designation.to_string())`, spin from Task 9 →
  `Placement`. `draw_with` complete.
  - `pub fn digest(drawn: &Drawn) -> u64` in the test file: a canonical
    text of every value (family code, star values, each placement's
    designation, mass, a, e, period, ϖ, spin, flux, XUV), each `f64`
    as `format!("{:.8e}")`, hashed with `timeline::fnv1a64`.

- [ ] **Step 1: Failing tests:** `designations_follow_distance`,
  `the_same_inputs_give_the_same_digest` (twice in one process),
  `adding_a_hostless_archetype_keeps_the_star_family_and_slots`
  (`hot-neptune` added: same star, family and the same drawn slot
  proposals, by stream), and `golden_digests` (8 fixed `Inputs`: 3 free
  seeds, one per forced host class, one with `earth-like`, one with
  2 × `hot-neptune`). The goldens come from the Linux CI run (CI uses
  `ubuntu-latest`; libm results may differ on macOS by an ulp and flip a
  threshold): commit the test with the digests printed and an
  `#[ignore]`, read them from the CI log, paste them in, drop the
  `#[ignore]`; the test's doc says so.
- [ ] **Step 2-4:** implement, `cargo test -p system`; `mise run ci`.
- [ ] **Step 5:** Commit `feat(system): designate the planets and pin the draw`.

### Task 11: Docs

**Files:**

- Create: `docs/science/system-architectures.md` (the five families,
  spacing, inner edge, ice line, spin, with sources and "Where it
  departs from the science", including: K50 for pairs with a drawn
  planet only, eccentricity counted both in Δ and in K50 (study's own
  rule), K50 of 13-14 removing most near-3:2 pairs of 5 M⊕ or more)
- Modify: `crates/system/README.md` (draw section, under 150 lines),
  `docs/science/README.md` (index line),
  `docs/superpowers/specs/2026-11-21-system-draw-design.md` (mark the
  delivered decisions; `TrialOutcome` shape; the trial label of
  archetype copies; drop the unreachable "drop a drawn neighbour")

- [ ] **Step 1:** write; `npx markdownlint-cli2` on the changed files → 0 issues.
- [ ] **Step 2:** `mise run ci` → green.
- [ ] **Step 3:** Commit `docs(system): document the planet draw`.

---

After Task 11: `branch-reviewer` on the whole branch, fixes, tests, then
`pr-shepherd` (PR body lists the plan's own choices and `Refs #602`).
Message mapgen1 with the `TrialOutcome` shape when Task 4 lands, and
mapgen1 and mapgen2 with the `trial/<id>/<k>` label of archetype copies
(contract note in `placement.rs`).
