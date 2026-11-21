# Planet model PR 1: interior structure and internal heat — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compute a solid planet's interior (radius, layers, radial density
and pressure, moment of inertia) by integrating hydrostatic structure with
published equations of state, plus its internal heat and the composition
draws they need, ready for the planet model's chain.

**Architecture:** Laws go in `science` (`eos`, `structure`, `heat`), free of
causes and seeds. The planet module of `system` draws the composition from
sub-seeds of the trial's seed, calls the laws, and records every value in
the system's `Trace`. Nothing is public outside `system` yet: `class_of`
and `model` arrive with PR 2 (atmosphere, escape, climate), which also
adds the temperature profile, envelopes and giants.

**Tech Stack:** Rust 2024 workspace, `rand_chacha` (ChaCha8Rng, per the
contract design), nextest tiers via `mise`.

**Spec:** `docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md`
(Science, System engine, Cross-sections, No invented data) and
`docs/superpowers/specs/2026-11-21-rewrite-contract-design.md` (the seam).
Studies behind every value:
`docs/superpowers/studies/2026-11-21-planet-interior/`
(`equations-of-state.md`, `core-and-dynamo.md`, `heat-and-composition.md`,
`prototype-report.md`, `prototype/*.py`). Issue #534. Slice 1, #602.

## Global Constraints

- Every numeric constant in `science` has a doc comment starting with
  `Source:` (citation with table or equation), `Calibration:` (what it was
  fitted against, link to the page section that says what was searched),
  or `Definition:`. `tools/guardrails` fails otherwise. A `let` bound to a
  bare number other than 0, 0.5, 1, 2 fails too.
- Never take a value from memory. A value used here is in one of the
  study files with its source, or the implementer reads it in the source
  and cites it. A missing value stops the task and is reported back.
- Expected values in tests come from sources, not from the code's output.
- `science` knows no body, seed, trace or id. `system` draws only from
  `Seed::derive` sub-seeds through `ChaCha8Rng`.
- Tests on this shared machine: `mise run test:tier1` (throttled by the
  mise task); one full run at a time. Never `npm ci` here.
- Earth radius in code is `EarthRadius` (IAU nominal equatorial,
  6378.1 km). Bodies' mean radii are compared in metres.
- One code path: the radius, the layer masses, the profiles and the
  moment of inertia all come from one structure integration.

## Decisions taken (user, 2026-11-21)

- Interior by hydrostatic integration, not the legacy Zeng fit with
  calibrated uniform densities.
- Study before porting: legacy constants marked "Calibration, no source"
  or "from memory" are sourced or documented as calibrations in this PR.
- Equation-of-state set: candidate D1 of the prototype (Fe-alloy core of
  Valencia et al. 2006 Table 1, fitted to PREM; mantle split at
  23.83 GPa, upper olivine/enstatite and lower bridgmanite/periclase,
  SLB values via Unterborn et al. 2016 Table 1; ice VII Vinet fit of
  Seager et al. 2007 §3.3). Prototype: Earth R −0.29 %, C/MR² −0.25 %;
  Venus, Mars, Mercury (0.346), Moon within 2 %; super-Earths within
  1.1 % of Zeng 2016/2019.
- Cold structure: thermal pressure is left out (radius effect ≤ 1.5 %
  below 1500 K at the surface: Seager 2007, Grasset 2009, Hakim 2018, in
  `equations-of-state.md` §3). Recorded as a departure; hot surfaces
  (lava worlds) are a known limit for PR 2 to revisit.

## Departures from legacy (worlds change on purpose)

| What | Legacy | New | Source |
| --- | --- | --- | --- |
| Radius | Zeng 2016 fit, uniform layers | integrated structure | prototype report |
| Earth C/MR² | 0.354 | about 0.330 | Williams 1994: 0.3307 |
| K-40 half-life | 1.248 Gyr | 1.277 Gyr | Dye 2012 Table 2 |
| Core mass fraction | N(0.325 + 0.15·[Fe/H], 0.05) | split-normal 0.32 (−0.12/+0.14), no metallicity slope | Plotnykov & Valencia 2020 (stars) |
| Water beyond the ice line | U(0.2, 0.5) | U(0.5, 0.67) | Zeng et al. 2019 |
| Radiogenic spread | 0.15 dex | 0.23 dex | Nimmo et al. 2020, derived in the study |
| Rocky tidal k2/Q | 0.015 (Io) | 6e-3 (k2 0.3, Q 50) | Henning et al. 2009 |

Each row is checked in the source by the task that ports it; a row that
does not hold up is reported back, not silently changed.

## Review Focus

1. A body with no water or no core (`water = 0`, `cmf = 0`): the
   integration must skip empty layers, not divide by zero.
2. A tiny body (0.01 M⊕, the Moon's scale) and a heavy one (20 M⊕): the
   shooting on central pressure must converge in both, with the pressure
   above the Holzapfel iron branch (234.4 GPa) at 20 M⊕.
3. A phase transition (23.83 GPa, the inner-core boundary 328.851 GPa,
   the water phases) falling between two steps: transitions are layer
   boundaries, so convergence stays fourth order.
4. Non-finite or negative inputs reaching the laws (mass, age): a typed
   error, never a NaN in a recorded value.
5. Determinism: the same trial seed gives bit-identical draws and
   structure on every run and thread count.

---

### Task 0: Commit the studies

**Files:**

- Create: `docs/superpowers/studies/2026-11-21-planet-interior/*` (already
  copied in the worktree)

- [ ] **Step 1:** Run `mise run md:lint`. Fix the study files until it
  passes (headings, blank lines, code-fence languages, bare URLs); never
  change a number or a citation.
- [ ] **Step 2:** Commit `docs: add the planet interior studies`.

### Task 1: Equations of state in `science`

**Files:**

- Create: `crates/science/src/eos.rs`
- Modify: `crates/science/src/lib.rs`, `crates/science/src/constants.rs`,
  `crates/science/README.md`

**Interfaces:**

- Produces:
  - `constants::EARTH_MEAN_RADIUS: Metre`, the IUGG mean radius R1 of
    GRS80 (Moritz 2000, J. Geodesy 74, 128); read the value there.
  - `pub enum Eos { Vinet { rho0, k0, k0_prime }, BirchMurnaghan3 { … },
    BirchMurnaghan2 { rho0, k0 }, Holzapfel { v0, k0, c0, c2, p0, molar_mass }
    }`
    with `pub fn pressure(&self, rho: KilogramPerCubicMetre) -> Pascal`
    and `pub fn density(&self, p: Pascal) -> Result<KilogramPerCubicMetre,
    EosError>`
    (inverse by safeguarded Newton on `pressure`, bracketed between ρ0
    and 100 ρ0).
  - `pub struct Mix(&'static [(f64 /* mass share */, Eos)])` with
    `density` by additive volumes at equal pressure (Amagat).
  - `pub struct Phase { pub up_to: Pascal, pub density_law: DensityLaw }`
    and `pub struct MaterialEos { pub phases: &'static [Phase] }`; a
    material's phases are ordered by pressure, the last one unbounded.
  - Material constants: `CORE_ALLOY` (outer liquid BM3 6800/165/4.2,
    inner solid BM3 7990/200/5.2, boundary 328.851 GPa), `MANTLE` (upper
    50/50 forsterite 3230/128/4.2 and enstatite 3200/107/7.0 to 23.83 GPa;
    lower 90 % (Mg0.88Fe0.12)SiO3 bridgmanite (MgSiO3 4110/251/4.1,
    FeSiO3 5180/272/4.1) and 10 % periclase 3590/161/3.8), `ICE_VII`
    (Vinet 1460/14.3771/6.57972). Units kg/m³, GPa. Sources: study
    `equations-of-state.md` tables (lines 177-188, 270, 307-313, 349-350,
    461-463).
  - `pub enum EosError { OutOfRange { pressure: Pascal } , NoConvergence }`.

- [ ] **Step 1: Write the failing tests** in `eos.rs`:
  - `each_law_gives_its_reference_density_at_zero_pressure`:
    `density(Pascal(0.0))`
    equals ρ0 to 1e-12 for each law (Definition of ρ0).
  - `density_inverts_pressure`: for each material and P in
    {1e5, 1e9, 1e10, 1e11, 1e12} Pa, `pressure(density(P))` within 1e-9
    relative.
  - `the_core_alloy_matches_prem_at_its_boundaries`: outer phase at
    135.751 GPa within 1 % of PREM 9903.5 kg/m³ (outer side of the CMB);
    inner phase at 328.851 GPa within 1 % of 12763.6 (study line 461).
    Tolerance: state Valencia 2006's own fit quality if the study gives
    it, else 1 %.
  - `the_mantle_jumps_at_the_670_km_boundary`: density just below and
    above 23.83 GPa within 3 % of PREM 3992.1 and 4380.7 (study line 463).
  - `earth_mean_radius_is_iugg`: `EARTH_MEAN_RADIUS` against the source
    value, and `EarthRadius::from(EARTH_MEAN_RADIUS)` below 1.
- [ ] **Step 2:** `mise run test:tier1` — compile failure.
- [ ] **Step 3:** Implement. Vinet, BM3 and BM2 in their standard forms
  (cite the form's equation in each doc comment: Vinet 1987 via Seager
  2007 eq. 8 or the study's reference; BM3 Seager eq. 7). The Holzapfel
  iron branch of Hakim 2018 eq. 5 stays out of `CORE_ALLOY` (Valencia's
  fit already spans the core) but is ported with its test against Hakim
  Table F.7 for PR 2's super-Earth cores only if Task 3 shows the
  Valencia fit leaves its range at 20 M⊕; otherwise drop it.
- [ ] **Step 4:** `mise run test:tier1` — PASS; `cargo run -p guardrails`
  (or the mise task that wraps it) reports no undocumented number.
- [ ] **Step 5:** Commit `feat(science): add equations of state for iron,
  silicate and ice`.

### Task 2: Water at low pressure (study gate, then law)

The prototype's icy bodies miss by about 4.5 % in radius (Ganymede
C/MR² +4 %) because every sourced closed form starts at 1460 kg/m³
(ice VII). Liquid water and ices Ih, III, V, VI need their own laws.

**Files:**

- Modify:
  `docs/superpowers/studies/2026-11-21-planet-interior/equations-of-state.md`
  (new section), `crates/science/src/eos.rs`

**Interfaces:**

- Produces: `WATER: MaterialEos` replacing `ICE_VII` in callers, phases by
  pressure along one isotherm `T` passed in: `pub fn water(t: Kelvin) ->
  MaterialEos`
  (or a fixed set if the study shows the temperature can be fixed with a
  source).

- [ ] **Step 1: Study.** Find sourced laws for liquid water (IAPWS-95 or a
  published fit), ice Ih (Feistel & Wagner 2006, doi:10.1063/1.2183324),
  ices III, V, VI (e.g. Bezacier et al. 2014, Fei et al. 1993, or what
  the literature uses for icy-moon interiors), and the phase boundaries
  in pressure along an isotherm. Write the section with values, units,
  sources and tables. **Gate:** if a needed law is not found in a source
  read, stop and report to the main session.
- [ ] **Step 2: Write the failing tests:**
  `liquid_water_is_a_thousand_kilograms_per_cubic_metre`
  (IAPWS value at 1e5 Pa and the chosen T), `ice_ih_density_at_one_bar`
  (Feistel & Wagner value), each phase's density at a tabulated point of
  its source.
- [ ] **Step 3:** Implement; `mise run test:tier1` PASS.
- [ ] **Step 4:** Commit `feat(science): add liquid water and low-pressure
  ices`.

### Task 3: Hydrostatic structure in `science`

**Files:**

- Create: `crates/science/src/structure.rs`
- Modify: `crates/science/src/lib.rs`, `crates/science/README.md`

**Interfaces:**

- Consumes: Task 1-2 `MaterialEos`, `CORE_ALLOY`, `MANTLE`, `water(t)`.
- Produces:
  - `pub struct LayerSpec { pub mass: Kilogram, pub material: &'static
    MaterialEos }`
    (centre out; zero-mass layers allowed and skipped).
  - `pub struct Sample { pub radius: Metre, pub enclosed_mass: Kilogram, pub
    density: KilogramPerCubicMetre, pub pressure: Pascal }`.
  - `pub struct Shell { pub outer_radius: Metre, pub mass: Kilogram, pub layer:
    usize, pub phase: usize }`
    — one shell per (layer, phase) actually present, so a phase change
    inside a layer is a shell boundary.
  - `pub struct Structure { pub radius: Metre, pub shells: Vec<Shell>, pub
    samples: Vec<Sample>, pub moment_of_inertia: f64 /* C/MR² */ }`.
  - `pub fn integrate(layers: &[LayerSpec], surface_pressure: Pascal) ->
    Result<Structure, StructureError>`.
  - `pub enum StructureError { NoMass, NotFinite, NoConvergence, Eos(EosError)
    }`.
- Method (from the prototype, `prototype/solve.py`): integrate in the
  mass coordinate t = (m/M)^(1/3) with RK4 from the centre, shooting on
  the central pressure (bracketed, Brent or bisection) until
  P(M) = `surface_pressure` to 1e-10 relative; ρ from the EOS at each
  stage; a phase transition pressure ends the step exactly (locate it,
  then restart the step in the new phase) so the error stays fourth
  order. C/MR² = (8π/3)∫ρ r⁴ dr / (M R²) along the same samples
  (Definition). Steps per shell: the smallest number giving 1e-4
  relative on R and C/MR² for Earth and 20 M⊕ (expected about 40, the
  prototype's note); fix it as a `Calibration:` constant with the
  convergence test as its check.

- [ ] **Step 1: Write the failing tests** (`structure.rs`, tier 1):
  - `a_uniform_sphere_has_two_fifths`: a constant-density material
    (test-only `MaterialEos`), C/MR² = 0.4 to 1e-6 and R = (3M/4πρ)^(1/3)
    to 1e-6 (Definition).
  - `earth_has_its_radius_moment_and_core`: Earth mass (`EARTH_MASS`),
    cmf 0.325, no water: R within 1 % of `EARTH_MEAN_RADIUS`, C/MR²
    within 1 % of 0.3307, central pressure within 3 % of 363.852 GPa, CMB
    radius within 1 % of 3480 km (PREM, study §4).
  - `the_terrestrial_planets_and_the_moon_within_two_percent`: Venus,
    Mars (cmf 0.21), Mercury (cmf 0.7136, C/MR² 0.346), Moon (cmf
    0.0168): R and measured C/MR² within 2 %. Masses, radii and cmf from
    the sources the prototype report cites; copy the citation into each
    test constant's comment.
  - `icy_bodies_within_two_percent`: Ganymede, Titan with the Task 2 water
    law: R and C/MR² within 2 %. If Task 2 cannot reach 2 %, the test
    asserts the reached tolerance and the gap is written in the page
    (Task 6) and reported in the PR.
  - `super_earths_follow_the_published_curves`: 5 M⊕ Earth-like within 2 %
    of Zeng 2016 (9860 km), 5 M⊕ 50 % water within 2 % of Zeng 2019
    (1.911 R⊕ at 300 K), 20 M⊕ Earth-like converges with finite values.
  - `empty_layers_are_skipped`: cmf 0 and water 0 give one rock shell;
    all mass zero returns `NoMass`.
  - `the_steps_converge`: doubling the steps changes R and C/MR² by less
    than 1e-4 relative for Earth and 20 M⊕.
  - `samples_are_ordered_and_finite`: radius and enclosed mass strictly
    increasing, pressure decreasing, density non-increasing within a
    shell.
- [ ] **Step 2:** `mise run test:tier1` — compile failure.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `mise run test:tier1` — PASS within the tier-1 budget;
  report the time of one Earth integration (release) in the commit body.
- [ ] **Step 5:** Commit `feat(science): integrate hydrostatic planet
  structure`.

### Task 4: Internal heat in `science`

**Files:**

- Create: `crates/science/src/heat.rs`
- Modify: `crates/science/src/lib.rs`, `crates/science/README.md`

**Interfaces:**

- Produces:
  - `pub fn radiogenic(silicate: Kilogram, abundance: f64 /* Earth = 1 */, age:
    Gyr) -> Watt`
    — Σ shareᵢ · H⊕ · (m/m⊕) · a · exp(λᵢ (t⊕ − t)) (legacy `heat.rs:54`).
  - `pub fn secular(silicate: Kilogram, age: Gyr) -> Watt` (legacy
    `heat.rs:92`).
  - `pub fn tidal(host: Kilogram, radius: Metre, semi_major_axis: Metre,
    eccentricity: f64, k2_over_q: f64) -> Watt`
    — 10.5 (k2/Q) G M_h² R⁵ n e² / a⁶ (Peale, Cassen & Reynolds 1979;
    legacy `heat.rs:108`).
  - Constants with their sources from `heat-and-composition.md` §A:
    half-lives (Dye 2012 Table 2: U-238 4.468, U-235 0.704, Th-232
    14.05, K-40 1.277 Gyr), Earth silicate mass 4.03e24 kg (Dye 2012
    Table 7), Earth's radiogenic power and the isotope shares (read §A1:
    take a sourced abundance set; if none is fully read, `Calibration:`
    with the 11-38 TW range of Dye 2012 Table 7 as the bound), Earth's
    secular cooling as total 46.7 TW (Davies & Davies 2010) minus the
    radiogenic power, the cooling time 10 Gyr (`Calibration:`, nothing
    found, §A3), Earth's age 4.5673 Gyr (Connelly 2012, read the paper or
    mark the abstract), k2/Q: rocky 6e-3 (Henning 2009: k2 0.3, Q 50),
    icy and enveloped from Lainey 2016 Table 1 where the study gives
    them; the thresholds between them `Calibration:` as legacy, with the
    search written.
- [ ] **Step 1: Write the failing tests:** half-life decay of each isotope
  over one half-life halves its term (Definition);
  `earth_today_is_inside_the_published_range`
  (radiogenic in 11-38 TW, total in 46.7 ± 2 TW, Davies & Davies 2010);
  `io_is_heated_about_as_observed` (Io's mass, radius, orbit and e from
  a cited source with Lainey's k2/Q 0.015: within the observed heat flow
  range the study cites; if the study has none, find one and cite);
  `heat_scales_with_silicate_mass`.
- [ ] **Step 2:** fail; **Step 3:** implement; **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(science): add radiogenic, secular and tidal
  heat`.

### Task 5: Composition draws, interior and heat in `system::planet`

**Files:**

- Modify: workspace `Cargo.toml` (`rand_chacha` and `rand_core` in
  `[workspace.dependencies]`, exact versions at least two weeks old),
  `crates/system/Cargo.toml`
- Move: `crates/system/src/planet.rs` → `crates/system/src/planet/mod.rs`
  (keep its doc comment)
- Create: `crates/system/src/planet/draws.rs`,
  `crates/system/src/planet/interior.rs`, `crates/system/src/planet/heat.rs`,
  `crates/system/src/planet/codes.rs` (cause codes as `Code::of` constants)

**Interfaces:**

- Consumes: `Trial` (`placement.rs`), `Trace::{leaf, record}`, `Seed::derive`,
  Tasks 1-4.
- Produces (all `pub(crate)`):
  - `draws::Composition` drawn from `trial.seed.derive("planet/cmf")`,
    `…("planet/water")`, `…("planet/radiogenic")`, each its own
    `ChaCha8Rng::seed_from_u64`: core mass fraction (split-normal, mode
    0.32, σ− 0.12, σ+ 0.14, Plotnykov & Valencia 2020, clamped to a sourced
    range or the study's tail), water fraction (inside the ice line
    log-normal around Marty 2012's bulk Earth 2700 ppm with its ±1350 ppm
    as 1σ; beyond U(0.5, 0.67), Zeng 2019; migration chance 0.1 as
    `Calibration:`), radiogenic abundance 10^N(0, 0.23). Recorded as
    `Drawn` leaves with codes `planet.core-mass-fraction`,
    `planet.water-fraction`, `planet.radiogenic-abundance`. Distribution
    constants live in `science` only if they are laws; draw parameters
    live in `system` with the same doc-comment rule.
  - `interior::Interior { bulk: Bulk, composition: Composition /* timeline */,
    shells: Vec<ShellRecord>, structure: Structure }`
    and `interior::interior(trial: &Trial, drawn: &draws::Composition, trace:
    &mut Trace) -> Result<Interior, InteriorError>`:
    layers Fe-alloy = cmf·(1−w)·M, rock = (1−cmf)·(1−w)·M, water = w·M;
    `Bulk` radius, gravity GM/R², mean density recorded from the
    structure node; each shell recorded with `Material` (core shells
    `Iron`, mantle `Rock`, water `Water` or `Ice` by phase) and
    `MatterState` (outer core `Liquid`, inner core `Solid`, …). The
    `InteriorProfile` itself (it needs temperatures) is built in PR 2.
  - `heat::heat(trial: &Trial, interior: &Interior, abundance: &Traced<f64>,
    trace: &mut Trace) -> Result<timeline::Heat, InteriorError>`:
    tidal from the primary's mass (`trial.sources.primary().mass`),
    `trial.orbit`, k2/Q by composition.
  - `InteriorError` (crate-internal until mapgen3's `SystemError` lands;
    then a variant of it).
- [ ] **Step 1: Write the failing tests:**
  `the_same_trial_draws_the_same_composition`
  (bit-identical twice); `different_labels_draw_independently` (changing
  the water draw's label leaves cmf unchanged); `draws_follow_their_sources`
  (10 000 draws: cmf median within 0.01 of 0.32, beyond-ice-line water
  within [0.5, 0.67]); `an_earth_trial_has_earths_bulk` (1 M⊕ at 1 AU
  around a 1 M☉ source, cmf forced through a test helper to 0.325 and
  water to 2.7e-3: radius within 1 % of `EARTH_MEAN_RADIUS`, gravity
  within 1 % of 9.80665 m/s² (CGPM 1901), every recorded node resolves
  in the trace); `heat_nodes_point_at_their_inputs`.
- [ ] **Step 2:** fail; **Step 3:** implement; **Step 4:** `mise run test:tier1`
  PASS.
- [ ] **Step 5:** Commit `feat(system): draw a planet's composition and build
  its interior and heat`.

### Task 6: Science pages and READMEs

**Files:**

- Modify: `docs/science/planet-interiors.md` (find sections with
  `grep -n '^#'`; rewrite "From layers to radius", "The heat inside",
  "How it computes them", "Where it departs from the science",
  "Sources"), `crates/science/README.md`, `crates/system/README.md`

- [ ] **Step 1:** Rewrite the sections for the integrated structure: the
  equations, the EOS table with sources, the validation table (body,
  measured, computed, error), the cold-structure limit, every
  `Calibration:` with what was searched and not found (from the study
  files), and the departures table above. Link the studies.
- [ ] **Step 2:** `mise run md:lint` PASS.
- [ ] **Step 3:** Commit `docs(science): describe the integrated interior and
  its sources`.

### Task 7: Checks and pull request (main session)

- [ ] **Step 1:** `mise run ci` PASS.
- [ ] **Step 2:** `branch-reviewer` and `test-reviewer` on the whole branch,
  given this plan and the two specs; fix, re-run the tests.
- [ ] **Step 3:** Tell mapgen3 the mass-function finding (Neil & Rogers
  2020 log-normal replaces the unsourced log-uniform small-planet draw;
  `heat-and-composition.md` §B9): the mass draw is theirs.
- [ ] **Step 4:** PR into `main` with `Refs #602`, `Closes #534` only if
  the icy bodies reach 2 % (else `Refs #534` and an issue for the gap),
  the departures table, a "Calibrations" section listing each
  `Calibration:` constant, and the Earth integration time.
