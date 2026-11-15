# Climate Pass Lot 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the planet model a moist radiation limit, a physical runaway with a consistent end state, and steam worlds that hide their surface, inflate their radius and render as grey clouds.

**Architecture:** A new radiative-convective grey column (`planet/column.rs`) replaces the Eddington closed form; the surface temperature solves `OLR(T) = absorbed + internal`, and a wet planet with no solution runs away. The chain's boil block uses that test and keeps the steam pass unless its leftover water re-condenses. A deep supercritical steam column makes a `steam-world`: its interior gains a steam layer (Turbet et al. 2020) and it renders through the giant path with a grey `GiantKind::Steam`.

**Tech Stack:** Rust (crate `mapgen`, ts-rs bindings), Svelte/TypeScript (`web/`).

**Spec:** `docs/superpowers/specs/2026-11-15-climate-pass-lot1-design.md`

## Global Constraints

- Worktree `~/Developer/mg-climate-steam`, branch `feat/climate-steam-world`. Never touch `~/Developer/cyboooooorg/map-generator`.
- Tests: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`, one full run at a time. A single test: `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features <name> -- --test-threads=3`.
- Web: `cd web && npm test && npm run check`. Never `npm ci` (node_modules is a symlink). Never port 3000.
- ts-rs bindings regenerate into `web/src/lib/api/types` when `cargo test --all-features` runs; commit them.
- Solar System ranges are hard: Earth 282-294 K regulated with CO2 < 0.01 bar, Venus 687-787 K, Mars 198-222 K, Titan 86-102 K.
- Moist plateau 282 W/m2 within 5 % (268-296), varying under 2 % between 1x and 3x Earth's N2. If both cannot hold with one set of constants: stop, report, do not widen ranges.
- A whole system draw may slow by at most 20 %.
- No format gate or legacy path for the old core.
- `SYSTEM_FORMAT_VERSION` 5 -> 6. World `FORMAT_VERSION`/`MIN_FORMAT_VERSION` and `web/src/lib/formatVersion.ts`: number given by the main session (Task 4 step), never guessed.
- Every new constant carries its source (paper, doi or arXiv, what was checked) in its doc comment, as the existing constants do.
- Commits: Conventional Commits, footer `Refs: #441` / `#442` / `#492` as relevant, plus the session's attribution lines.

## Review Focus

1. A cold airless or thin-air body (Mars, the Moon, Mercury): the column must reduce to `T_eq` warmed by internal heat, no NaN at zero pressure. Test in Task 1.
2. A wet planet with a trace of water (`water_bar` tiny) must not run away: the vapour is capped by `water_bar`, so the plateau never binds. Test in Task 2.
3. Thick N2 on a temperate Earth (2-11 bar) must stay temperate rather than run away through an artefact. Test in Task 1.
4. A steam world's card pressure must be the model's (1e5 bar order), never 1000. Test in Task 4.
5. Moons go through the same chain: a hot icy moon (Io-like heating, Europa-like water) must not crash or become a steam world unless its column qualifies. Test in Task 3.

---

### Task 1: Radiative-convective grey column (#441) — hard, opus

**Files:**
- Create: `crates/mapgen/src/planet/column.rs`
- Modify: `crates/mapgen/src/planet/mod.rs` (declare module), `crates/mapgen/src/planet/climate.rs` (`surface_temperature_k`, `optical_depth`, `broadening_bar`, steam constants and their tests, `wet_climate`, `thermostat`)

**Interfaces:**
- Produces: `pub(crate) fn olr_w_m2(gases: &Gases, surface_k: f64) -> f64` in `column.rs` — `gases` are surface partial pressures (bar), `gases.h2o` the surface vapour; above the level where water saturates the vapour follows the saturation curve on the moist adiabat.
- Produces: `pub fn surface_temperature_k(context: &Context, gases: &Gases, heat_flux_w_m2: f64) -> f64` (same signature, new body): bisection of `olr_w_m2(gases, T) = SIGMA T_eq^4 + q`.
- Produces: `fn wet_balance(context: &Context, gases: Gases, heat_flux_w_m2: f64, water_bar: f64) -> Option<(Gases, f64)>` in `climate.rs`: the coldest `T` with `olr_w_m2(with_vapour(gases, T, water_bar), T) = SIGMA T_eq^4 + q`, `None` when absorbed plus internal flux exceeds every OLR the moist column reaches (scan T upward from the dry temperature to `WATER_CRITICAL_K`, then bisect the first crossing). `wet_climate` becomes a wrapper that Task 2 changes.

Column model (the algorithm the signatures do not fix):

- Pressure grid from the surface total `P_s` up to `1e-4 P_s` (log spaced, about 100 levels; the builder may change the count).
- Temperature: dry adiabat `T ∝ p^(R/cp)` of the mixture until the vapour partial pressure reaches saturation, then the moist pseudo-adiabat (Pierrehumbert 2010, §2.7). Past the critical point the vapour is a supercritical gas on the dry adiabat. A skin temperature `T_eq / 2^(1/4)` floors the column (stratosphere).
- Optical depth from the top: integrate `dτ` per level from the current per-gas laws, rewritten per unit pressure so that a uniform column reproduces today's `line(k, p_i, a, P_b)` totals; then recalibrate `CO2_OPACITY`, `H2O_OPACITY`, `CH4_OPACITY`, `THICK_AIR_EXPONENT` against the hard ranges and the plateau.
- `T_emit` = temperature where `τ = 2/3` (surface temperature if the whole column is thinner); `OLR = SIGMA T_emit^4`.
- Remove `STEAM_MAX_TAU`, `STEAM_MAX_BAR`, `STEAM_BREAKDOWN_K` and the steam cap in `broadening_bar` if the column makes them unnecessary; keep `SIMPSON_NAKAJIMA_W_M2` as the test target.

- [ ] **Step 1: Write the failing tests** in `climate.rs` tests (keep `the_solar_system_has_its_climates` unchanged):

```rust
#[test]
fn the_moist_plateau_is_the_simpson_nakajima_limit() {
    // Earth air, saturated column, surface 320..400 K: OLR flat at 282 ± 5 %.
    for t in [320.0, 340.0, 360.0, 380.0, 400.0] {
        let olr = column::olr_w_m2(&with_vapour(earth_air(), t, f64::INFINITY), t);
        assert!((268.0..=296.0).contains(&olr), "{t} K: {olr}");
    }
}

#[test]
fn the_plateau_does_not_depend_on_nitrogen() {
    let plateau = |n2: f64| column::olr_w_m2(&with_vapour(Gases { n2, ..earth_air() }, 380.0, f64::INFINITY), 380.0);
    let (one, three) = (plateau(0.78), plateau(2.34));
    assert!((one - three).abs() / one < 0.02, "{one} vs {three}");
}

#[test]
fn an_airless_column_emits_from_the_surface() {
    assert!((column::olr_w_m2(&Gases::default(), 250.0) - SIGMA * 250f64.powi(4)).abs() < 1e-6);
}

#[test]
fn a_thick_nitrogen_earth_stays_temperate() {
    // 1 AU, modern CO2 (4e-4 bar), N2 at 2, 5 and 11 bar: wet_balance exists, T < 330 K.
}

#[test]
fn thin_steam_follows_the_plateau_past_the_old_grey_law() {
    // 10 bar of pure steam: OLR at 680 K is within 10 % of 282 W/m2;
    // print the temperature at which OLR leaves the plateau (Boukrouche: ~1050 K) for the report.
}
```

`earth_air()` is a test helper: N2 0.78, O2 0.21, Ar 0.009, CO2 4e-4 bar (use the `Gases` fields as they exist).

- [ ] **Step 2: Run them, expect FAIL** (`column` does not exist).
- [ ] **Step 3: Implement `column.rs`, the new `surface_temperature_k` and `wet_balance`; recalibrate constants.** Replace the steam-cap tests (`a_steam_runaway_settles_past_the_end_of_the_radiation_limit`, `thick_steam_sends_out_the_simpson_nakajima_limit_at_the_breakdown`, the equal-τ test near :955) by: a 30 %-water Earth at 0.7 AU settles between 1500 and 2500 K. Update the doc comments of every changed constant and function.
- [ ] **Step 4: Run the full `mapgen` tests.** Expected: all pass, Solar System ranges unchanged. If they cannot hold together with the plateau: stop and report.
- [ ] **Step 5: Measure** `cargo run --release -p mapgen --example planet_census -- 500` wall time before (on `origin/main`) and after; over +20 %: tabulate or coarsen, then re-measure. Write both times in the commit body.
- [ ] **Step 6: Commit** `feat(planet): replace the grey core with a radiative-convective column` (`Refs: #441`).

### Task 2: Runaway from the plateau, end state from the leftover water (#442) — hard, opus

**Files:**
- Modify: `crates/mapgen/src/planet/climate.rs` (`climate`, `boils`, `past_critical`), `crates/mapgen/src/planet/mod.rs` (boil block in `chain`, ~:404-420), `crates/mapgen/src/planet/regime.rs` (doc of `water`, `Water::boiled`), `crates/mapgen/src/planet/classify.rs` tests

**Interfaces:**
- Consumes: `wet_balance` (Task 1).
- Produces: `pub fn boils(context: &Context, before: &Gases, water: &Water, heat_flux_w_m2: f64, water_bar: f64) -> bool` — `!water.runaway && water.kept_kg > 0.0 && wet_balance(...).is_none()`. `past_critical` is removed.
- `climate(...)` keeps its signature; for a wet body with no balance it returns the state with all `water_bar` as vapour, solved by `surface_temperature_k` (a placeholder the chain replaces).

Chain order (spec §2), one pass, no loop:

1. First pass with `water(...)` (the Kopparapu insolation path stays).
2. If `boils(...)`: `steam(...)` with `boiled: true`, re-run `bare`.
3. If the re-run keeps water and `wet_balance` exists for that leftover water, use a re-run with `Water { runaway: false, boiled: true, kept_kg: leftover, .. }` (the ocean that re-condensed after losing water); else keep the steam re-run.

- [ ] **Step 1: Write the failing tests** in `classify.rs`:
  - `a_boil_that_escapes_all_its_water_ends_dry`: a 458 c-like planet (S = 0.906 S_earth, 1.05 bar N2, 93 bar CO2 outgassed, enough water to reach ~280 bar of vapour; pick the inputs with the `reference` helpers) → `water.runaway && water.boiled && water.kept_kg == 0.0`, liquid not `Water`.
  - Rewrite `a_boil_whose_steam_cools_is_not_kept` as `a_cool_small_planet_does_not_boil`: same 0.2 M_earth, 1.5 AU inputs → `!water.runaway`, all water kept, class not `RunawayGreenhouse`.
  - `a_trace_of_water_does_not_boil_a_temperate_planet` stays and passes.
  - `a_boil_whose_leftover_recondenses_keeps_its_ocean`: inputs where escape leaves water and the leftover balances (the builder finds them with a quick scan and states them) → `!water.runaway && water.boiled && kept_kg > 0`, liquid `Water`.
- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement** the chain order above; update the docs of `boils`, `Water::boiled`, `water`.
- [ ] **Step 4: Run the full `mapgen` tests.** Expected: pass.
- [ ] **Step 5: Commit** `fix(planet): run away from the moist limit and keep a consistent end state` (`Closes: #442`).

### Task 3: Steam worlds in the model (#492) — hard, opus

**Files:**
- Modify: `crates/mapgen/src/planet/mod.rs` (`Material::Steam`, chain), `crates/mapgen/src/planet/interior.rs`, `crates/mapgen/src/planet/classify.rs`, `crates/mapgen/src/system.rs:69` (`SYSTEM_FORMAT_VERSION` = 6 and its history line)

**Interfaces:**
- Produces: `pub const STEAM_WORLD_MIN_BAR: f64 = 10.0 * WATER_CRITICAL_BAR` with `pub const WATER_CRITICAL_BAR: f64 = 220.64` in `climate.rs`; `pub fn hides_surface(surface: &Surface) -> bool` in `climate.rs` = `surface.water.runaway && surface.gases.h2o >= STEAM_WORLD_MIN_BAR && surface.climate.temperature_k >= WATER_CRITICAL_K`.
- Produces: `Material::Steam` (ts-exported).
- Produces: `pub fn steam_interior(solid: &Interior, mass_earth: f64, steam_kg: f64, exposure: &Exposure) -> Interior` in `interior.rs`: pushes a `Material::Steam` layer of the steam mass on top, radius = solid radius + envelope thickness from Turbet et al. 2020 (A&A 638, A41, arXiv:1911.08878): read their analytical mass-radius fit for runaway water-rich planets; clamp inputs to their grid and say so; density and gravity from the whole mass.
- Produces: `PlanetClass::SteamWorld`, key `"steam-world"`; `is_giant()` stays false.

- [ ] **Step 1: Write the failing tests:**
  - `classify.rs`: `the_492_super_earth_is_a_steam_world` — 4.78 M_earth, iron 1.06 / rock 3.16 / water 0.56 M_earth, M9V star at 0.018 AU (from `gh issue view 492`) → class `SteamWorld`, `interior.radius_earth > 1.64 * 1.2`, top layer `Material::Steam`, `surface.gases.h2o > 1e5`.
  - `classify.rs`: `thin_steam_over_rock_stays_a_runaway_greenhouse` — the existing 1 M_earth, 1 % water, 1 AU, 20 bar N2 case → `RunawayGreenhouse`.
  - `interior.rs`: `steam_inflates_the_radius_with_the_water_share` — radius grows with steam mass at fixed solid; zero steam returns the solid radius.
  - `classify.rs`: `a_hot_icy_moon_is_no_steam_world_without_a_deep_column` — a Europa-like moon heated past the runaway (pick inputs) is not `SteamWorld` unless `hides_surface`.
- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement**: in `chain`, after the surface is chosen, if `hides_surface` then `interior = steam_interior(&interior, inputs.mass_earth, surface.water.kept_kg, &exposure)`; `surface_class` returns `SteamWorld` before the runaway rule; add the key to `key()` and its list; bump `SYSTEM_FORMAT_VERSION` to 6.
- [ ] **Step 4: Run the full workspace tests** (regenerates `web/src/lib/api/types/PlanetClass.ts`, `Material.ts`). Expected: pass.
- [ ] **Step 5: Commit** `feat(planet): make deep supercritical steam a steam world` (`Refs: #492`), bindings included.

### Task 4: Render steam worlds as grey clouds (#492) — sonnet

**Files:**
- Create: `looks/steam-world.toml`
- Modify: `crates/mapgen/src/giant_world.rs` (`GiantKind::Steam`, `class_palette`, wind/flux laws), `crates/mapgen/src/planet/interim.rs` (`base_of`, `renders_as_giant`, `params_for` giant arm), `crates/mapgen/src/export/raw_data.rs:68,76`, `web/src/lib/formatVersion.ts`

**Interfaces:**
- Consumes: `PlanetClass::SteamWorld`, `Interior::radius_earth` (Task 3).
- Produces: `base_of` returns `"steam-world"` for `SteamWorld`; `renders_as_giant` true for it.

- [ ] **Step 1: Write the failing tests** in `interim.rs`: `a_steam_world_renders_as_grey_clouds` — `base_of == "steam-world"`, `renders_as_giant`, params circumference = `2π R_model` (the model's inflated radius, not `giant_radius_of`), stored class `"steam-world"`, and the model's `surface.gases.total()` still above 1e5 bar after `params_for` (the card reads the model). In `giant_world.rs`: `steam_palette_is_grey` — every palette colour has `max(r,g,b) - min(r,g,b) < 0.04`.
- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement**: `GiantKind::Steam` with ice-giant dynamics and a low-contrast grey palette (zone light grey, belt mid grey, storm and polar near-white); `looks/steam-world.toml` with `kind = "steam"`, `rings = [0.0, 0.0]`, neutral `rayleigh`/`haze_colour`, `clouds = "none"`. In `params_for`, a steam world takes the model's radius for `circumference_km`; the stored model keeps its pressure (no clamp touches the model).
- [ ] **Step 4: Ask the main session for the world `FORMAT_VERSION` number** (it agrees it with the peers); set `FORMAT_VERSION`, `MIN_FORMAT_VERSION` and `web/src/lib/formatVersion.ts` to it.
- [ ] **Step 5: Run the full workspace tests.** Expected: pass.
- [ ] **Step 6: Commit** `feat(worlds): render steam worlds as grey cloud decks` (`Refs: #492`).

### Task 5: Web and docs — sonnet

**Files:**
- Modify: `web/src/lib/planetClass.ts` (+ `.test.ts`), `web/src/lib/i18n/en.ts`, `web/src/lib/i18n/fr.ts`, `web/src/lib/systemStar.ts:138`, the card components that list materials (`InteriorCard.svelte` and its labels), `README.md` (planet model climate and classes sections: find them with `grep -n '^#' README.md`), the census doc if any.

**Interfaces:**
- Consumes: `"steam-world"`, `Material` `"steam"` (Task 3 bindings).

- [ ] **Step 1: Write the failing tests** in `planetClass.test.ts`: `steam-world` is in the class list, is an envelope class (shows no surface tools: follow how `mini-neptune` is treated, except it is not a giant), and has EN and FR labels (`Steam world` / `Monde de vapeur`; moon blocks: `Steam world moon` / `Lune de vapeur`).
- [ ] **Step 2: Run** `cd web && npm test`, expect FAIL.
- [ ] **Step 3: Implement** the labels, the class handling, `systemStar.ts` kind `'ice'` for `steam-world`, the steam layer label (`Steam` / `Vapeur supercritique`) in the interior card; README: the climate section (radiative-convective column, plateau, runaway rule) and the class list (`steam-world`).
- [ ] **Step 4: Run** `npm test && npm run check`, expect pass.
- [ ] **Step 5: Commit** `feat(web): show steam worlds` (`Refs: #492`).

### Task 6: Census, coherence and visual check — main session with agents

- [ ] Run `planet_census -- 500` on `origin/main` and on the branch; report class mix, steam-world count, thin-steam runaway temperatures, radius histogram, wet planets past 647 K (expected 0).
- [ ] `world-coherence-checker`: a temperate Earth-like world, `origin/main` vs branch.
- [ ] `generation-profiler`: a whole-system draw, before and after (≤ +20 %).
- [ ] `branch-reviewer` on the whole branch with the spec and this plan.
- [ ] Deploy on develop (after warning the peers), `viewer-visual-check` on the #492 system (`mgs-81447-3145862552`, body b): whole view and close-up show grey clouds, no relief; card pressure ~4e5 bar.
