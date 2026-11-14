# Physical Planet Model (Part 3c) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the relief of model worlds follow the model: the crater
transition from the crust's strength (#312), the impact flux and crater
size from the orbit (#313), and the continents and basins from a
crustal thickness by Airy isostasy (#315); find why some large craters
on moons have no rim.

**Architecture:** `generation::crust_kind` reads a world's surface
material from its `PlanetModel` (moons, with no model, keep the density
test). `craters` takes the transition from `9 Y / (ρ g)` with `Y` and
`ρ` per crust kind. A new module `impacts` derives the flux ratio and
the crater scale against the Moon from the orbit, the star, the
system's impactors and the body's gravity; `Geology.impact_ratio` and
`crater_scale` go. `generation::floating` maps the continents' noise
to a crustal thickness measured from a fixed crust line, not from the
waterline, and turns it into a height by Airy's balance; the sea level
from the water volume then decides the coast.

**Tech Stack:** Rust (crate `mapgen`; `server` for the format checks),
serde, ts-rs; TypeScript bindings in `web/src/lib/api/types/`.

**Spec:** `docs/superpowers/specs/2026-11-14-planet-model-part3c-design.md`
(main design: `docs/superpowers/specs/2026-11-11-physical-planet-model-design.md`).

## Global Constraints

- Worktree `~/Developer/mg-planet-model-p3c`, branch
  `feat/planet-model-part3c` (the spec is its first commit). One pull
  request into `main`, closing #312, #313 and #315.
- Commits in the order #312, #313, #315, then the rim fix; the
  `FORMAT_VERSION` bump (18 → 19, `export/raw_data.rs:59`, with its
  history line, `MIN_FORMAT_VERSION`, `web/src/lib/formatVersion.ts`)
  once, in Task 5. `SYSTEM_FORMAT_VERSION` (4) bumps only if a stored
  system field changes.
- Moons (`model: None`) keep `generation::icy`, `ICE_TRANSITION`,
  `ICE_RELIEF` and today's flux (locking and giants): their crater
  tests pass unchanged.
- Every constant cites its source, read by the implementer, or says
  `Calibration, no source.` and names the bodies it reproduces. No
  impact or crust value is set per archetype or per name.
- Earth's hypsometry bands in `earth_like_worlds_have_earths_hypsometry`
  (`generation.rs:2439`) are not loosened. If no calibration keeps
  them, stop and report.
- Transition targets (model worlds): `earth-like` 3–5 km,
  `mercury-like` 10–12 km, `mars-like` 7–8 km. Flux targets against the
  Moon: `mars-like` within 20 % of 2.6 (flux) and 0.751 (crater scale,
  Hartmann's isochrons, as `docs/science/impact-craters.md` gives them);
  `mercury-like`, `venus-like`, `earth-like` within 20 % of the values
  of one source read and cited (Le Feuvre & Wieczorek 2011, Ivanov 2001,
  Neukum et al. 2001, or another), the choice recorded where sources
  differ. One labelled calibrated constant is allowed in the flux rule.
- Shared machine. Tests:
  `RAYON_NUM_THREADS=4 cargo test -j 4 --workspace --all-features -- --test-threads=3`
  (one full run at a time); clippy
  `cargo clippy -j 4 --workspace --all-targets --all-features -- -D warnings`;
  `cargo fmt --all`. Never write to the main checkout's `systems/`,
  `worlds/`, `db/`; never `npm ci`; never port 3000.
- ts-rs bindings regenerate on `cargo test`; commit
  `web/src/lib/api/types/` with the change (Task 2 changes
  `Geology.ts`).
- Documentation ships in the task that changes the behaviour:
  `docs/science/impact-craters.md` (§ Simple and complex craters :84,
  § In the app :226, Mars's flux :347, departures :369),
  `docs/science/mountains-and-gravity.md` (§ Isostasy :59, § In the app
  :118, floating :155/:184, departures :240), `docs/analysis/08-geology.md`
  (find the outcome steps with `grep -n`), README (find `impact_ratio`
  and `[geology]` with `grep -n`; never read it whole).
- Implementers run on opus for Tasks 1–3 (physics), sonnet for 4–5.

## Review Focus

1. **A model world with a frozen ocean that is not an ice world** (a
   snowball Earth: plates, liquid none, 1 ocean). Its crust is rock
   (the ice sheet is not its crust): Task 1 pins it.
2. **A body with no giants and no belts** (a lone world, `alone`).
   The flux rule must stay finite and above zero (today's floor 0.05):
   Task 2 pins it.
3. **A wet world with little water** (`mars-like`, 0.002–0.008 oceans).
   Its sea must fill the lowest oceanic-crust basins, not stand as a
   ring around a raised platform: Task 3 pins the land share falling
   monotonically with the water.
4. **Resolution invariance** (`tests/resolution_invariance.rs`): the
   crust line is a fixed noise level, never a quantile of the sampled
   grid. Task 3 runs that test.
5. **Dry worlds** (`venus-like`, `mercury-like`, `look.rules.dry`).
   They keep their plains path and their hypsometry tests
   (`generation.rs:2634`, `:2678`): Task 3 runs them.

---

### Task 1: Crater transition from the crust's strength (#312)

**Files:**
- Modify: `crates/mapgen/src/generation.rs:199-205` (`icy`), add
  `CrustKind` and `crust_kind` beside it; fix the stale comment at
  `:2762-2764`.
- Modify: `crates/mapgen/src/craters.rs:93-106` (constants),
  `:578-588` (transition and complex depth).
- Test: `crates/mapgen/src/craters.rs` tests (next to `:1776`).
- Docs: `docs/science/impact-craters.md` § Simple and complex craters
  and § In the app; `docs/analysis/08-geology.md` outcome for craters.

**Interfaces:**
- Produces:
  - `pub(crate) enum CrustKind { Rock, WetRock, Ice }` in
    `generation.rs`.
  - `pub(crate) fn crust_kind(p: &WorldParams) -> CrustKind`: with a
    model and a surface, `Ice` when `inputs.water_fraction >=
    classify::ICE_WORLD_MIN_WATER` and the surface keeps at least
    `classify::ICE_WORLD_MIN_OCEANS` oceans with no liquid water;
    `WetRock` when the regime is not `Plates`, there is no runaway, and
    the water kept is above a labelled threshold that `mars-like`
    passes and `mercury-like` does not; `Rock` otherwise. Without a
    model: `Ice` if `icy(p)`, else `Rock`.
  - `pub(crate) fn icy(p)` returns `crust_kind(p) == CrustKind::Ice`
    (the density test moves inside `crust_kind`'s no-model branch), so
    `relief_share` and the crater depth follow.
  - `pub(crate) fn transition_km(p: &WorldParams) -> f64` in
    `craters.rs`: `9.0 * Y / (ρ * g)` (O'Keefe & Ahrens 1993) for a
    model world, with `Y` (Pa) and `ρ` (kg/m³) per `CrustKind`;
    without a model, today's formula
    (`MOON_TRANSITION_KM * MOON_GRAVITY / g`, times `ICE_TRANSITION`
    if icy). `CraterField::context_at` uses it.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn model_worlds_turn_complex_where_their_crust_gives_way() {
    let t = |name| transition_km(&crate::generation::tests::preset_params(name, 1, 64, 32));
    assert!((3.0..=5.0).contains(&t("earth-like")), "earth {}", t("earth-like"));
    assert!((10.0..=12.0).contains(&t("mercury-like")), "mercury {}", t("mercury-like"));
    assert!((7.0..=8.0).contains(&t("mars-like")), "mars {}", t("mars-like"));
}

#[test]
fn a_frozen_earth_keeps_a_rock_crust() {
    // Review Focus 1: an ice sheet on a plate world is not an icy crust.
    let mut p = crate::generation::tests::preset_params("earth-like", 1, 64, 32);
    // Set the model's climate liquid to none and its temperature to 230 K.
    // (Edit `p.model` in place.)
    assert_eq!(crate::generation::crust_kind(&p), CrustKind::Rock);
}
```

Keep `strength_moves_the_transition_from_simple_to_complex` (`:1776`)
for the no-model path; it must pass unchanged.

- [ ] **Step 2: Run** `cargo test -p mapgen craters::tests::model_worlds_turn` — expect FAIL (`transition_km` not found).
- [ ] **Step 3: Implement** `CrustKind`, `crust_kind`, `transition_km`
  as in Interfaces. Strengths and densities: read Barnouin et al.
  2012 and Melosh & Ivanov 1999 for what they give; the values that
  make the targets are labelled calibrations (rock about 12 MPa at
  2.7–3.0 g/cm³ gives Earth 4 km and Mercury about 10 km; wet rock
  lower so Mars lands at 7–8 km). Ice keeps the Ganymede calibration
  (`ICE_TRANSITION` of the Moon's law expressed as a strength at
  ice's density).
- [ ] **Step 4: Run** the craters and generation tests — expect PASS,
  including every existing moon and crater test.
- [ ] **Step 5: Docs, then commit**

```bash
git commit -am "feat(craters): take the crater transition from the crust's strength" \
  -m "Closes: #312"
```

### Task 2: Impact flux and crater size from the orbit (#313)

**Files:**
- Create: `crates/mapgen/src/impacts.rs` (add `pub mod impacts;` in
  `lib.rs`).
- Modify: `crates/mapgen/src/craters.rs:539-623` (`context_at`).
- Modify: `crates/mapgen/src/geology.rs:106-174`: remove
  `impact_ratio`, `crater_scale`, their defaults and validation.
- Modify: `web/src/lib/api/types/Geology.ts` (regenerated).
- Test: `crates/mapgen/src/impacts.rs`; replace
  `a_mars_like_world_has_marss_crater_production` (`craters.rs:1817`).
- Docs: `docs/science/impact-craters.md` (the "Mars's flux" paragraph
  becomes the rule, § departures), README lines found by
  `grep -n 'impact_ratio\|crater_scale'`, `docs/backlog.md:718-719`
  pointer.

**Interfaces:**
- Consumes: `WorldParams.impact_factor` (system draw, `system.rs:1499`),
  `giants::GiantSystem::draw` (fallback), `spin::star_info(p)` (star
  mass and luminosity), `p.orbit_au`, `p.gravity_modifier()`,
  `p.circumference_km`.
- Produces:
  - `pub struct ImpactRates { pub flux: f64, pub crater_scale: f64 }`
    (both relative to the Moon's).
  - `pub fn rates(p: &WorldParams) -> ImpactRates`. With a model: flux
    = impactors at the orbit (`p.impact_factor`, else the giants'
    draw) × an orbit term (the asteroids' supply falls with distance
    from their source, the belt just inside the ice line, `2.7 √L` AU
    as `request.rs:827` places it; labelled calibration on Mars and
    Mercury) × gravitational focusing `1 + v_esc² / v_∞²` against the
    Moon's; `crater_scale` from π-scaling, `D ∝ g^(-μ/(2+μ))
    v^(2μ/(2+μ))` (Holsapple 1993, μ for competent rock as he gives
    it), with the impact speed `√(v_∞² + v_esc²)` against the Moon's;
    `v_∞` scales with the orbital speed `√(G M★ / a)`, its Earth-orbit
    value from a source (Ivanov 2001 or Le Feuvre & Wieczorek 2011).
    Without a model: `flux` = today's `impact` term (locking excluded),
    `crater_scale` = 1.
  - `context_at` uses `rates(p)`: `raise = locked_raise * rates.flux`.

- [ ] **Step 1: Write the failing tests** in `impacts.rs`:

```rust
#[test]
fn solar_system_look_alikes_have_their_published_crater_production() {
    let r = |name| rates(&crate::generation::tests::preset_params(name, 1, 64, 32));
    let near = |x: f64, target: f64| (x / target - 1.0).abs() <= 0.2;
    let mars = r("mars-like");
    assert!(near(mars.flux, 2.6), "mars flux {}", mars.flux);
    assert!(near(mars.crater_scale, 0.751), "mars scale {}", mars.crater_scale);
    // Mercury, Venus, Earth: the cited source's values, as constants in
    // the test with their citation.
}

#[test]
fn a_lone_world_without_impactors_keeps_a_floor() {
    // Review Focus 2: no belts, no giants.
    let mut p = crate::generation::tests::preset_params("earth-like", 1, 64, 32);
    p.impact_factor = Some(crate::belts::impact_factor(&[], &[], f64::from(p.orbit_au)));
    let r = rates(&p);
    assert!(r.flux.is_finite() && r.flux > 0.0);
}

#[test]
fn moons_keep_the_moons_production() {
    // A params without a model: crater_scale 1, flux as before.
}
```

- [ ] **Step 2: Run** `cargo test -p mapgen impacts` — expect FAIL.
- [ ] **Step 3: Implement** `impacts.rs`, remove the two `Geology`
  fields (the looks under `looks/` do not use them; `deny_unknown_fields`
  stays), switch `context_at`.
- [ ] **Step 4: Run** the full test command and clippy — expect PASS;
  commit the regenerated `Geology.ts`.
- [ ] **Step 5: Docs, then commit**

```bash
git commit -am "feat(craters): derive the impact flux and crater size from the orbit" \
  -m "Closes: #313"
```

### Task 3: Crustal thickness and Airy isostasy (#315)

**Files:**
- Modify: `crates/mapgen/src/generation.rs`: constants `:258-276`,
  `floating` `:300-317`, `Sampler` (`waterline` uses at `:540`,
  `:604`, `:787-820`), `noise_span_km` `:188`; the sea-level searches
  `:1251` and `:1349` (docs, and optionally sampling once since the
  relief no longer depends on the sea level).
- Test: `crates/mapgen/src/generation.rs` tests.
- Docs: `docs/science/mountains-and-gravity.md` § Isostasy, § In the
  app, departures; `docs/analysis/08-geology.md`; `docs/backlog.md:743-756`
  pointer; the `floating` doc comment.

**Interfaces:**
- Produces:
  - `const CRUST_LINE: f32`: the level of the continents' noise
    (coarse + fine) between continental and oceanic crust, a fixed
    noise value; labelled calibration giving Earth's continental-crust
    share, about 41 % of the surface (Cogley 1984, read and cited, or
    another source).
  - `pub(crate) struct Crust { continental_km: f32, oceanic_km: f32,
    crust_density: f32, mantle_density: f32 }` and
    `pub(crate) fn crust_of(p: &WorldParams) -> Crust`: plate worlds
    take Earth's (continental about 35–40 km, Christensen & Mooney
    1995; oceanic 7 km, White et al. 1992; 2.8 and 3.3 g/cm³);
    worlds without plates take a highland/lowland contrast calibrated
    on Mars's dichotomy (Neumann et al. 2004), labelled.
  - `pub(crate) fn floating(x: f32, crust: &Crust) -> f32`: `x` is now
    the noise above (below) `CRUST_LINE`; the result is a height in
    elevation units relative to a fixed datum (the top of oceanic
    crust's column is not the waterline). It maps `x` to a thickness
    (oceanic below the line, rising across a slope to continental
    above it, the platform's inland rise kept) and returns
    `t (1 − ρc/ρm)` minus the oceanic column's, plus the abyss's
    deepening below the slope as today (sea-floor age is #317). The
    shelf, slope and abyss widths stay noise widths.
  - `Sampler`: `continental()` (`:603`), the mountain onset (`:787`),
    the inland band and new crust measure from `CRUST_LINE` instead of
    `self.waterline`; the coast (where land meets sea) still comes
    from the waterline. The dry path keeps its plains, laid from its
    datum, unchanged unless measured better.
  - Sea level: `interim::calibrate` keeps `sea_level_for_water`; Earth's
    1 ocean must land the shore near the shelf (land share 0.29 ± 0.05).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn less_water_exposes_the_basins_too() {
    // Review Focus 3. Earth-like at 1, 0.5 and 0.1 oceans: the land share
    // rises, and at 0.1 some land lies on oceanic crust (noise below
    // CRUST_LINE above the waterline).
}

#[test]
fn the_crust_floats_by_its_thickness() {
    // Replaces the_crust_floats_as_a_platform_and_a_bounded_floor (:2512):
    // continental above oceanic by (tc − to)(1 − ρc/ρm) within the
    // platform's rise; the old floor near plates::OLD_FLOOR_KM below
    // the shelf on Earth's crust; gravity does not enter.
}

#[test]
fn the_crust_line_does_not_move_with_the_sea() {
    // Two sea levels, same world: the continental() mask is identical.
}
```

- [ ] **Step 2: Run** them — expect FAIL.
- [ ] **Step 3: Implement** as in Interfaces; recalibrate only
  `CRUST_LINE`, the slope and platform widths and the mean thicknesses
  within their sources so that `earth_like_worlds_have_earths_hypsometry`
  passes unchanged.
- [ ] **Step 4: Run** the full test command (including
  `tests/resolution_invariance.rs`, the plains and highland
  hypsometry tests at `:2634` and `:2678`) and clippy — expect PASS.
- [ ] **Step 5: Docs, then commit**

```bash
git commit -am "feat(relief): float the crust by its thickness, Airy's balance" \
  -m "Closes: #315"
```

### Task 4: Large craters without a rim on moons

**Files:** found by the investigation; likely `crates/mapgen/src/craters.rs`
(`Profile::new` `:1577`, `Profile::shaped` `:1584`, basins and the
mare threshold `:683`).

- [ ] **Step 1: Reproduce.** Use superpowers:systematic-debugging.
  Generate rocky and icy moons (a drawn system's moons through
  `system::generation_params`, several seeds, 1024 × 512), list every
  crater above twice the transition with the rim height its
  `Profile` gives and the elevation measured on the world around it.
  Report which have no rim (rim ≤ surroundings) and what they share
  (size against the transition or the body's radius, profile class,
  mare flooding, overlap, saturation, epoch).
- [ ] **Step 2:** If the cause lies in the crater code, write the
  failing test that pins it (a crater of that class keeps a rim above
  its surroundings), fix, run the full tests, commit
  `fix(craters): …`. Otherwise stop and report the cause; the main
  session files the issue.

### Task 5: Format bump, coherence and docs

**Files:**
- Modify: `crates/mapgen/src/export/raw_data.rs:21-67` (history line
  for 19: crater transition, impact flux, floating crust),
  `web/src/lib/formatVersion.ts:6`, server tests at
  `crates/server/src/app.rs:2493`.

- [ ] **Step 1:** Bump `FORMAT_VERSION` and `MIN_FORMAT_VERSION` to 19
  and the web's `MIN_FORMAT_VERSION`; run the full test command.
- [ ] **Step 2:** Commit
  `feat(export)!: bump the world format to 19 for the crust and craters`
  with a `BREAKING CHANGE:` footer (older worlds are listed as outdated).
- [ ] **Step 3:** The main session runs `world-coherence-checker`
  (`main` against this branch, the built-in rocky archetypes and one
  drawn system, same seeds) and `branch-reviewer`, then judges the
  change.
