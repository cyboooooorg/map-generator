# System star (PR 1 of the system draw) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the system engine its main-sequence star: drawn or asked
mass, metallicity and present date, the star's present radius,
luminosity and effective temperature, and its interior profile, all
from published stellar models.

**Architecture:** An offline tool (`tools/stellar-grid`) turns the YREC
grid and the Baraffe 2015 and Dartmouth tracks into three text tables
in `crates/science/data/`. `science::stellar` embeds them
(`include_str!`), parses them once, and interpolates plain values.
`system::star` draws the star, refuses what is out of range, and wraps
every value in `Traced` nodes of the system's `Trace`.

**Tech Stack:** Rust 2024 (workspace crates `science`, `timeline`,
`system`), Python 3 with numpy and the `yrec` package for the offline
tool only.

**Spec:** `docs/superpowers/specs/2026-11-21-system-draw-design.md`
(sections The star, Star values at the present, Interior profile,
Stellar data in the repository, Errors and traces, Delivery item 1).

## Global Constraints

- Every numeric `const` in `crates/science/src` has a first doc line
  starting `Source:`, `Calibration:` or `Definition:` (guardrail
  `science_constants_are_sourced`); `let` bound to a bare literal other
  than 0, 0.5, 1, 2 fails it too.
- `science` depends on `serde` only; `system` on `timeline`, `science`,
  `serde`, `rand_core`, `rand_chacha`. No new external dependency.
- No `HashMap`/`HashSet`, no `std::fs`, no `std::env`, no clock in
  domain crates (root `clippy.toml`). Tables are embedded with
  `include_str!`.
- Draw distributions are `Definition:` and live in `system`; physical
  values are sourced and live in `science`.
- Host mass range 0.08–8 M☉, [Fe/H] −1 to +0.5 dex, present date at most
  13.8 Gyr (Planck 2018).
- Class shares one in six for M, K, G, F, A, B; class bounds (Pecaut &
  Mamajek 2013, table 2022.04.16): 0.08, 0.57, 0.88, 1.06, 1.61, 2.18,
  8 M☉; log-uniform mass within the class; [Fe/H] uniform −1 to +0.5;
  present date uniform between ZAMS and min(lifetime, 13.8 Gyr).
- Seeds: `inputs.seed.derive("star")` for every star draw, ChaCha8.
- Cause codes `system.star.*`.
- Tier 1 tests only (no system generation); expected values come from
  sources, not from the code's output.
- Each crate README at most 150 lines; markdownlint clean.
- Commits follow Conventional Commits with the session's attribution
  footer; `mise run ci` passes before the PR.

## Review Focus

- A request at a range edge (0.08 M☉, 8 M☉, −1 and +0.5 dex, a present
  date equal to the lifetime) is accepted and gives finite values, not
  an interpolation error: Task 3 and Task 5 tests at the edges.
- A present date before the star's ZAMS (a 0.08 M☉ star is still
  contracting at 1 Gyr) is refused, not drawn as a pre-main-sequence
  star in the main-sequence phase: Task 5 test.
- The 0.3 M☉ seam between Baraffe and YREC never jumps by more than the
  measured 3 % in T_eff: Task 3 test.
- A star whose lifetime is under the drawn upper bound (a B star lives
  under 0.2 Gyr) draws its date inside that life: Task 5 test.
- Same seed, same star, bit for bit, whatever was asked elsewhere in
  the request: Task 5 determinism test.

---

### Task 1: The stellar tables and their tool

Run by mapgen3: it needs the grid outputs and downloads in mapgen3's
session scratch space
(`/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg-map-generator/2a20d7a0-8075-4482-b1e1-cf5b43dca9aa/scratchpad/`:
`ygrid/`, `dl-bhac15/`, `dl-kiauhoku/`, `dl-b23/`).

**Files:**

- Create: `tools/stellar-grid/README.md` (purpose, prerequisites:
  Python 3, numpy, `pip install yrec`; how to regenerate; licences and
  citations)
- Create: `tools/stellar-grid/namelists/` (the 312 `.nml1`/`.nml2`,
  written by `prep_grid.py` from the shipped YREC grid namelists)
- Create: `tools/stellar-grid/prep_grid.py`, `run_grid.py`,
  `extract.py` (the first two exist in the scratch space)
- Create: `crates/science/data/stellar-tracks.tsv`,
  `crates/science/data/stellar-profiles.tsv`,
  `crates/science/data/stellar-low-mass.tsv`,
  `crates/science/data/STELLAR.md` (columns, units, sources, how each
  number was derived, the B23 comparison)

**Interfaces:**

- Produces the table formats Task 2 and Task 3 parse. Every file:
  `#` comment lines first (source, licence, columns), then
  tab-separated rows, decimal numbers in `%.6g`.
  - `stellar-tracks.tsv`: one row per (mass, feh, sample); columns
    `mass_msun feh age_gyr log_l log_r log_teff xc_frac`; per track
    the ZAMS row (first model with nuclear luminosity ≥ 99 % of L) and
    about 40 rows evenly spaced in `xc_frac` = X_c/X_c,ZAMS down to
    10⁻⁴/X_c,ZAMS or to 20 Gyr; a last row flagged by `xc_frac` ≤
    1.5e-4 marks TAMS.
  - `stellar-profiles.tsv`: header row per snapshot
    `P mass_msun feh age_gyr xc_frac n_layers n_samples`, then
    `L r_outer_frac m_outer_frac transport` rows (`R`/`C`), then
    `S r_frac m_frac log_rho_cgs log_p_cgs log_t fusion_share
    transport` rows. Snapshots at `xc_frac` 1, 0.9, 0.7, 0.5, 0.3,
    0.1, 0.01 (as reached). Mass step 0.1 M☉ up to 2 M☉; above, the
    widest step whose measured interpolation error on log ρ, log P,
    log T at the mid-point stays under 0.02 dex (report the step and
    error in `STELLAR.md`). File size under 2 MB in total.
  - `stellar-low-mass.tsv`: Baraffe 2015 rows for 0.08, 0.09, 0.1,
    0.11, 0.13, 0.15, 0.17, 0.2, 0.3 M☉ at ages 0.1, 0.2, 0.5, 1, 2,
    5, 10 Gyr (`mass_msun age_gyr teff log_l r_rsun`), then Dartmouth
    offsets rows `D mass_msun feh dlog_teff dlog_l dlog_r` at 5 Gyr
    for 0.15, 0.2, 0.25, 0.3 M☉ and [Fe/H] −1, −0.5, +0.5 relative to
    [Fe/H] 0 (Dartmouth `m0NNfeh{m10,m05,p05}afep0.trk`).

- [ ] **Step 1:** Copy `prep_grid.py` and `run_grid.py` from the
  scratch `scripts/` into `tools/stellar-grid/`, make the output
  directory an argument, and write the namelists into
  `tools/stellar-grid/namelists/` with the start-model path left as
  `{YREC_START}/` (YREC sets it).
- [ ] **Step 2:** Write `extract.py` (reads the `.track` and `.store`
  outputs; stops at the atmosphere: `ATM_PT` column 12 = `T`; transport
  layers from the `CONVECTIVE` column, column 9; fusion share = running
  ∫ E_NUC dm / total, E_NUC column 45 (0-based) of the `.store` header, check
  by name).
- [ ] **Step 3:** Run it; check the 1 M☉ [Fe/H] 0 snapshot nearest
  4.57 Gyr gives R 0.9995 R☉, L 0.998 L☉, T_eff 5771 K, envelope base
  0.727 R (values measured on the run).
- [ ] **Step 4:** Write `STELLAR.md` with the B23 comparison table
  (r/R 0.05 … 0.95: ρ, P, T, m) and the seam numbers (YREC vs Baraffe
  vs Dartmouth at 0.3 M☉, 1 and 5 Gyr).
- [ ] **Step 5:** Commit: `feat(science): add the main-sequence
  stellar tables and their tool`.

### Task 2: Present-day values of a star

**Files:**

- Create: `crates/science/src/stellar/mod.rs`,
  `crates/science/src/stellar/tables.rs` (parsing, `OnceLock`),
  `crates/science/src/stellar/values.rs`
- Modify: `crates/science/src/lib.rs` (`pub mod stellar;`),
  `crates/science/src/constants.rs` (add `SOLAR_EFFECTIVE_TEMPERATURE:
  Kelvin = Kelvin(5772.0)`, Definition: IAU 2015 B3;
  `AGE_OF_UNIVERSE: Gyr = Gyr(13.8)`, Source: Planck 2018 VI)
- Test: inline `#[cfg(test)]` in `values.rs`

**Interfaces:**

- Produces:

```rust
pub struct MainSequence {
    pub radius: SolarRadius,
    pub luminosity: SolarLuminosity,
    pub effective_temperature: Kelvin,
}
pub enum Lifetime { Ends(Gyr), Beyond(Gyr) } // Beyond(20.0): track stops before TAMS
pub struct Life { pub zams: Gyr, pub end: Lifetime }
pub enum StellarError { MassOutOfRange(SolarMass), MetallicityOutOfRange(Dex),
                        BeforeZams { zams: Gyr }, AfterTams { tams: Gyr } }
pub fn life(mass: SolarMass, metallicity: Dex) -> Result<Life, StellarError>;
pub fn main_sequence(mass: SolarMass, metallicity: Dex, age: Gyr)
    -> Result<MainSequence, StellarError>;
pub const MIN_MASS: SolarMass; pub const MAX_MASS: SolarMass;   // 0.08, 8 (Definition: scope of the tables)
pub const MIN_METALLICITY: Dex; pub const MAX_METALLICITY: Dex; // −1, +0.5
```

  `StellarError` implements `Display` and `std::error::Error`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn the_sun_at_its_age() {            // Source: IAU 2015 B3, Bahcall 2005 age 4.57 Gyr
    let s = main_sequence(SolarMass(1.0), Dex(0.0), Gyr(4.57)).unwrap();
    assert!((s.radius.0 - 1.0).abs() < 0.01);
    assert!((s.luminosity.0 - 1.0).abs() < 0.02);
    assert!((s.effective_temperature.0 - 5772.0).abs() < 30.0);
}
#[test] fn the_sun_leaves_the_main_sequence_near_ten_gyr() // life(1, 0).end == Ends(t), 9 < t < 11
#[test] fn a_late_m_dwarf_matches_baraffe() // 0.1 M☉, 0 dex, 5 Gyr: T_eff 2811 ± 1 %, R 0.124 ± 1 %, log L −3.061 ± 0.01
#[test] fn a_late_m_dwarf_contracts_before_its_zams() // life(0.08, 0).zams > 0.5 Gyr; main_sequence(0.08, 0, 0.1) == Err(BeforeZams)
#[test] fn the_seam_at_three_tenths_is_smooth() // T_eff at 0.29 and 0.31 M☉, 5 Gyr, differ < 3 %; L increases with mass
#[test] fn metal_poor_stars_are_hotter() // 1 M☉ at 1 Gyr: T_eff(−1) > T_eff(0) > T_eff(+0.5)
#[test] fn the_range_edges_are_finite() // all four corners (0.08/8 × −1/+0.5) at their ZAMS: Ok, finite
#[test] fn out_of_range_is_refused() // 0.07 and 8.1 M☉, −1.1 and +0.6 dex: the matching Err
#[test] fn a_b_star_lives_briefly() // life(8, 0).end == Ends(t), t < 0.06 Gyr
```

- [ ] **Step 2:** Run `cargo nextest run -p science stellar` — FAIL
  (module missing).
- [ ] **Step 3:** Implement. Rules the tests do not fix:
  - Mass interpolation linear in log mass between the two bracketing
    tracks, then linear in [Fe/H] between the bracketing metallicities;
    values interpolated in log L, log R, log T_eff.
  - Along a track, at equal fractional main-sequence age
    τ = (t − t_ZAMS)/(t_TAMS − t_ZAMS) when both bracketing tracks
    reach TAMS; at equal absolute age otherwise. t_TAMS and t_ZAMS of
    the star interpolated in log t.
  - Below 0.3 M☉: Baraffe rows, bilinear in log mass and log age; the
    Dartmouth offset added in dex, interpolated in mass from 0.15 to
    0.3 M☉ and held at its 0.15 M☉ value below. ZAMS for these masses:
    the first Baraffe age at which |Δ log R / Δ log t| < 0.01
    (`Definition:` in `STELLAR.md`). Lifetime `Beyond(Gyr(20.0))`.
  - Seam: between 0.28 and 0.32 M☉ blend the two in log, weight linear
    in mass.
  - Tables parsed once into `OnceLock<Tables>`; a parse failure is a
    `panic!` with the line number (the tables are part of the build,
    covered by tests).
- [ ] **Step 4:** Run `cargo nextest run -p science stellar` — PASS;
  `cargo test -p guardrails` — PASS.
- [ ] **Step 5:** Commit: `feat(science): interpolate a main-sequence
  star's present values`.

### Task 3: The star's interior profile

**Files:**

- Create: `crates/science/src/stellar/profile.rs`,
  `crates/science/src/stellar/polytrope.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**

- Consumes: Task 2 tables and `main_sequence`.
- Produces:

```rust
pub struct StellarLayer { pub outer_radius: Fraction, pub outer_mass: Fraction, pub convective: bool }
pub struct StellarSample { pub radius: Fraction, pub mass: Fraction,
    pub density: KilogramPerCubicMetre, pub pressure: Pascal, pub temperature: Kelvin,
    pub convective: bool, pub fusion_share: Fraction }
pub struct StellarProfile { pub layers: Vec<StellarLayer>, pub samples: Vec<StellarSample> }
pub fn profile(mass: SolarMass, metallicity: Dex, age: Gyr) -> Result<StellarProfile, StellarError>;
```

  Radii and masses are fractions of the star's; `system` scales them.
  CGS to SI: density × 1000 (g cm⁻³ → kg m⁻³), pressure × 0.1
  (dyn cm⁻² → Pa).

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn the_sun_matches_the_standard_solar_model() {
    // Source: B23 SSM (Zenodo 10174172, CC-BY 4.0); values from STELLAR.md
    // centre: ρ 150.0 g/cm³ (±5 %), T 1.558e7 K (±3 %); at r/R 0.05…0.95
    // ρ, P, T within 8 % of the B23 rows listed in STELLAR.md
}
#[test] fn the_sun_has_a_convective_envelope_from_0_71() // outermost layer convective, inner bound in 0.70..0.74
#[test] fn the_sun_burns_inside_a_quarter_radius() // fusion_share ≥ 0.99 at r/R 0.25 (Source: B23 L(r))
#[test] fn a_red_dwarf_is_convective_throughout() // 0.2 M☉: one layer, convective
#[test] fn the_polytrope_has_its_textbook_shape() {
    // Source: Chandrasekhar 1939, n = 1.5: ξ₁ = 3.65375, ρ_c/ρ̄ = 5.99071
}
#[test] fn a_late_m_dwarf_central_density_matches_baraffe() // 0.08 M☉, 5 Gyr: ρ_c 638 g/cm³ within 10 %
#[test] fn a_hot_star_has_a_convective_core() // 3 M☉ at ZAMS: innermost layer convective, outer radiative
#[test] fn profiles_are_ordered_and_finite() // for masses 0.08, 0.3, 1, 2, 8 and both metallicity edges: radii and masses strictly increasing, fusion_share non-decreasing ending at 1
```

- [ ] **Step 2:** Run `cargo nextest run -p science stellar::profile`
  — FAIL.
- [ ] **Step 3:** Implement. Rules: above 0.3 M☉ interpolate the two
  bracketing snapshots in `xc_frac`, then masses and metallicities as
  in Task 2, sample by sample at equal `r_frac`, in log for ρ, P, T;
  layer bounds interpolated linearly (a layer present in only one
  bracketing profile, such as a thin convective core near 1.1 M☉, is
  kept when the nearer profile has it). Below 0.3 M☉: Lane–Emden n =
  1.5 integrated by RK4 at a fixed step of 1e-3 in ξ (deterministic),
  scaled to the star's M and R; one convective layer; fusion share
  from ε ∝ ρ T⁴ (`Calibration:`, pp chain near 4 × 10⁶ K; cite
  Kippenhahn et al. 2012 §18.5.1) with T from the n = 1.5 relation
  T ∝ θ. Within 0.28–0.32 M☉ use the YREC profile (it is fully
  convective there too).
- [ ] **Step 4:** Run `cargo nextest run -p science` — PASS;
  guardrails — PASS.
- [ ] **Step 5:** Commit: `feat(science): build a main-sequence star's
  interior profile`.

### Task 4: `SystemError`

**Files:**

- Create: `crates/system/src/error.rs`
- Modify: `crates/system/src/lib.rs` (`mod error; pub use
  error::SystemError;`)

**Interfaces:**

- Produces:

```rust
#[non_exhaustive]
pub enum SystemError {
    Contract(timeline::ContractError),
    HostMassOutOfRange { mass: SolarMass },          // message names #615 above 8 M☉
    MetallicityOutOfRange { metallicity: Dex },
    PresentBeforeZams { present: Gyr, zams: Gyr },
    PresentAfterLife { present: Gyr, end: Gyr },     // end = min(life, 13.8)
}
impl From<ContractError> for SystemError; impl From<StellarError> for SystemError;
```

  `Display` gives the code-like message of `ContractError`'s style
  (`system.host-mass-out-of-range: 9 M☉ (0.08-8; above 8 see #615)`).

- [ ] **Step 1:** Test `display_names_the_issue_above_eight` (message
  contains `#615` for 9 M☉) and `stellar_errors_convert` — FAIL, then
  implement, PASS.
- [ ] **Step 2:** Commit: `feat(system): add SystemError`.

### Task 5: Draw the star

**Files:**

- Modify: `crates/system/src/star.rs` (replace the doc stub)
- Create: `crates/system/src/star/codes.rs` if `star.rs` becomes
  `star/mod.rs`
- Modify: `crates/system/README.md` (the star section)

**Interfaces:**

- Consumes: `science::stellar::{life, main_sequence, profile}`,
  `SystemError`, `Inputs`, `Trace`, `Seed`.
- Produces (for PR 2's draw):

```rust
pub struct Star { pub state: StarState, pub age: Traced<Gyr>, pub lifetime_end: Traced<Gyr> }
pub(crate) fn star(host_mass: Option<SolarMass>, metallicity: Option<Dex>,
                   present: Option<Gyr>, seed: Seed, trace: &mut Trace) -> Result<Star, SystemError>;
pub(crate) const CLASS_BOUNDS: [f64; 7];   // 0.08, 0.57, 0.88, 1.06, 1.61, 2.18, 8.0
```

  `seed` is the system seed; the function derives `"star"` itself.
  Codes: `system.star.class` (drawn class index as a leaf only when
  drawn), `system.star.mass`, `system.star.metallicity`,
  `system.star.age`, `system.star.lifetime-end`, `system.star.radius`,
  `system.star.luminosity`, `system.star.effective-temperature`,
  `system.star.interior` (the `RadialSamples` node), and
  `system.star.layer.N.{outer-radius,mass,material,state,transport}`.
  Provenance `Input` for asked values, `Drawn` for drawn ones; radius,
  luminosity, T_eff, lifetime-end and interior are `record`ed from
  mass, metallicity (and age).
  Layers: material `Plasma`, state `Plasma`, transport from the
  profile; `outer_radius` in metres (× radius × `SOLAR_RADIUS`), `mass`
  in kilograms (layer mass, not cumulative).

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn an_asked_star_is_kept() // 1.0 M☉, 0.2 dex, 3 Gyr asked: same values, provenance Input
#[test] fn classes_are_drawn_evenly() // 6000 seeds: each class count in 1000 ± 100
#[test] fn masses_stay_in_their_class_bounds() // 600 seeds: every drawn mass in [0.08, 8]
#[test] fn metallicity_is_drawn_across_the_range() // 600 seeds: min < −0.9, max > 0.4, all in range
#[test] fn the_drawn_date_lies_in_the_life() // 600 seeds: zams ≤ age ≤ min(end, 13.8)
#[test] fn a_b_star_draws_a_young_date() // asked 6 M☉: drawn age < 0.1 Gyr
#[test] fn a_date_before_zams_is_refused() // 0.08 M☉ at 0.1 Gyr: PresentBeforeZams
#[test] fn a_date_after_the_life_is_refused() // 2 M☉ at 5 Gyr: PresentAfterLife; any star at 14 Gyr: PresentAfterLife
#[test] fn a_host_above_eight_is_refused() // 9 M☉: HostMassOutOfRange
#[test] fn the_same_seed_gives_the_same_star() // two draws of seed 42: equal StarState and age, bit for bit
#[test] fn the_state_validates() // StarState::validate() is Ok for 20 seeds
#[test] fn the_interior_traces_back_to_the_mass() // the interior node's causes include the mass node
```

- [ ] **Step 2:** Run `cargo nextest run -p system star` — FAIL.
- [ ] **Step 3:** Implement. Draw order on one ChaCha8 stream from
  `seed.derive("star")`: class (when mass not asked), mass, metallicity
  (when not asked), age (when not asked). Uniform draws use the open
  (0, 1) 53-bit helper of `planet/draws.rs:199`; move it to a shared
  `crates/system/src/random.rs` (`pub(crate) fn uniform(&mut
  ChaCha8Rng) -> f64`) and make `planet::draws` call it, without
  changing its output.
- [ ] **Step 4:** Run `cargo nextest run -p system` — PASS (the
  planet tests unchanged).
- [ ] **Step 5:** Commit: `feat(system): draw the main-sequence star`.

### Task 6: Docs and CI

**Files:**

- Create: `docs/science/star-structure.md` ("What is inside a star?":
  energy transport, why the Sun has a convective envelope from 0.71 R,
  why an M dwarf is convective throughout and a hot star has a
  convective core; the app's sources and the measured comparisons from
  `STELLAR.md`; one SVG of the three profiles is welcome but optional)
- Modify: `docs/science/README.md` (link under Star view),
  `crates/science/README.md` (the `stellar` module and its data)

- [ ] **Step 1:** Write the pages; `mise run md:lint` — PASS.
- [ ] **Step 2:** `mise run ci` — PASS.
- [ ] **Step 3:** Commit: `docs(science): explain the structure of a
  star`.
- [ ] **Step 4:** `branch-reviewer` on the whole branch against the
  spec; fix, re-run the tests; then the PR (`Refs #602`).
