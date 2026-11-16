# Climate pass lot 2b Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the planet model a Bond albedo derived from the body, snowball states, CO2 build-up on cold planets, realistic sulphur and internal heat, a methane cold trap and Triton's thin air, and show them in the look and the card.

**Architecture:** Everything stays in the zero-dimensional planet model (`crates/mapgen/src/planet/`). A new `albedo` module turns planet-wide cover fractions into a Bond albedo; `climate::climate` solves albedo and temperature as a fixed point, started warm and started frozen, and picks the branch with a new drawn input. Look (`interim.rs`), card causes (`explain.rs`) and the web labels follow the stored `Climate`.

**Tech Stack:** Rust (mapgen crate, ts-rs exports), TypeScript/Svelte web (labels, viewer cloud kind).

**Spec:** `docs/superpowers/specs/2026-11-16-climate-pass-lot2b-design.md`

## Global Constraints

- Paths below are relative to the worktree `~/Developer/mg-climate-lot2b`, branch `feat/climate-lot2b`. `P` = `crates/mapgen/src/planet`.
- Tests: `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features -- --test-threads=3 <filter>`; full suite `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`, one full run at a time on this machine. Web: `cd web && npm test` (never `npm ci`: `web/node_modules` is a symlink).
- No per-body or per-archetype tuning of physics: every constant is calibrated on several bodies at once and carries its source in its doc comment (spec, Anti-goals).
- Archetype `[inputs]` may pin a drawn input (as `life = true` pins `life_draw`); that is how `earth-like` stays on the warm branch.
- No legacy algorithm, no format gate: worlds change, old ones are marked outdated.
- Reference values (spec, Acceptance): Earth 288 K ± 2, Bond 0.28-0.32, SO2 < 1e-6 bar, flux 0.087-0.092 W/m², volcanism 0.5; Venus Bond 0.7-0.8, 700-760 K, about 90 bar; Mars Bond 0.2-0.3, about 210 K; Titan 90-97 K, 1.3-1.6 bar, CH4 3-6 %, lakes; Triton 35-41 K, N2 1e-5 to 3e-5 bar; Sun outer edge (warm branch) 1.6-1.7 AU; runaway from about 0.95 AU.
- Commit messages: Conventional Commits, scope `planet` (or `web`), footer `Refs: #423` plus the issue the task closes, and the attribution lines of the session.
- Formats: world `FORMAT_VERSION` 23 → next free number, `SYSTEM_FORMAT_VERSION` 7 → next free number, agreed with the peer sessions by the main session (Task 9).

## Review Focus

1. A body with no water and no atmosphere (Mercury, the Moon): the derived albedo must fall to bare rock (about 0.1), not to the ocean or cloud terms. Test in Task 3.
2. A runaway steam world from lot 1 (`--orbit-au 0.0178`, #492's body): the `absorbing` blend must still apply on top of the derived albedo, no jump at the runaway edge. Test in Task 3.
3. A drawn planet far past the outer edge (5-10 AU): only the frozen branch exists, the draw must not matter, and CO2 build-up must stop at CO2 condensation. Test in Task 4.
4. A moon (Titan, Europa-like icy moon, Io): the moon's context is its planet's, so the albedo rescale must use the moon's own cover. Test in Task 6.
5. A stored system of the old format opened after the bump: it is marked outdated, not crashed on the new `Climate` fields. Test in Task 9.

---

### Task 1: Secular cooling in the internal heat (#447, heat part)

Opus (physics).

**Files:**
- Modify: `P/heat.rs:151-170` (`InternalHeat`, `total_w`), new fn next to `radiogenic_w` (`:53`)
- Modify: `P/mod.rs:497-510` (`internal_heat`)
- Modify: test literals `P/heat.rs:261`, `P/regime.rs:168`, `P/atmosphere.rs:497`, `:589`
- Modify: `P/interim.rs:101` doc of `EARTH_HEAT_FLUX_W_M2` (now compares like with like)

**Interfaces:**
- Produces: `InternalHeat { radiogenic_w: f64, tidal_w: f64, secular_w: f64 }`; `total_w()` sums the three; `pub fn secular_w(silicate_kg: f64, age_gyr: f64) -> f64` in `heat.rs`.

- [ ] **Step 1: Write the failing tests** in `P/heat.rs` tests:

```rust
#[test]
fn earth_loses_forty_six_terawatts_today() {
    // Davies & Davies 2010: 47 ± 2 TW; Urey ratio 0.3-0.5 (Korenaga 2008).
    let r = radiogenic_w(EARTH_SILICATE_KG, 1.0, EARTH_AGE_GYR);
    let s = secular_w(EARTH_SILICATE_KG, EARTH_AGE_GYR);
    let total = r + s;
    assert!((44e12..=47e12).contains(&total), "{total}");
    assert!((0.3..=0.5).contains(&(r / total)));
}

#[test]
fn secular_cooling_fades_with_age_and_size() {
    let s = |kg, age| secular_w(kg, age);
    assert!(s(EARTH_SILICATE_KG, 2.0) > s(EARTH_SILICATE_KG, 4.567));
    assert!(s(EARTH_SILICATE_KG, 9.0) < s(EARTH_SILICATE_KG, 4.567));
    // Mars's silicate is about a tenth of Earth's.
    assert!(s(0.1 * EARTH_SILICATE_KG, 4.567) < 0.15 * s(EARTH_SILICATE_KG, 4.567));
}
```

Update `heat_adds_up_and_spreads_over_the_surface` (`:260`): its literal gets `secular_w`, and the asserted flux is the three-term total. Add to `P/mod.rs` tests: the Solar System `earth` model has `heat.flux_w_m2(1.0)` in `0.087..=0.092`, Mars in `0.015..=0.030` (Plesa et al. 2018 estimate about 0.019-0.025), and `P/interim.rs` test `earths_world_is_earth_sized_wet_and_mild` (`:1160`) asserts Earth volcanism `0.45..=0.55`.

- [ ] **Step 2: Run** `... cargo test -p mapgen --all-features -- --test-threads=3 heat::` — Expected: FAIL, `secular_w` not found.

- [ ] **Step 3: Implement `secular_w`** and the field; `internal_heat` fills it from the same `silicate_kg` and `context.age_gyr`. The law is the builder's (spec, Left to the builder): a thermal-history form decaying with age and growing with silicate mass, calibrated on Earth's 26 ± 3 TW of secular loss, documented with its sources.

- [ ] **Step 4: Run** `heat::`, `regime::`, `interim::`, `mod::` filters — Expected: PASS. Check `regime::the_solar_system_has_its_regimes` still passes (Earth plates; `HEAT_PIPE_FLUX_W_M2` 1.0 untouched).

- [ ] **Step 5: Commit** `feat(planet): add secular cooling to the internal heat`, footer `Refs: #447`.

### Task 2: Sulphur from the silicate, washed out by water and oxygen (#447, sulphur part)

Sonnet.

**Files:**
- Modify: `P/inputs.rs:203` (`ROCK_S`), `:441` (`volatile_rule`) and its callers `P/mod.rs:277`, `P/reference.rs:188`, `P/moon.rs:126`, `P/interim.rs:952`
- Modify: `P/atmosphere.rs:25,105,221`; new fn `wash_out`
- Modify: `P/mod.rs:545-596` (`bare`: call `wash_out` after `oxygenate`)

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `pub(crate) fn volatile_rule(refractory: f64, water: f64, core_mass_fraction: f64) -> Volatiles`; `pub(crate) fn wash_out(gases: Gases, liquid: SurfaceLiquid) -> Gases` in `atmosphere.rs`.

- [ ] **Step 1: Write the failing tests** in `P/atmosphere.rs` tests:
  - `earths_air_keeps_no_sulphur_dioxide`: Solar System Earth model, `gases.so2 < 1e-6`.
  - `venus_keeps_its_sulphur_dioxide`: Venus `so2` in `0.008..=0.02` bar (about 150 ppm of 92 bar, Bertaux et al. 2007 / Marcq et al. 2018).
  - `mars_keeps_little_sulphur_dioxide`: Mars `so2 < 1e-6` (observed upper limit about 0.3 ppb, Encrenaz et al. 2011).
  - `volatile_rule_takes_sulphur_from_the_silicate`: with `core_mass_fraction` 0.325, `s` is `ROCK_S * refractory * (1 - 0.325)`.
- [ ] **Step 2: Run** `atmosphere::` — Expected: FAIL.
- [ ] **Step 3: Implement.** `ROCK_S` becomes the bulk silicate value, about 2.5e-4 kg/kg (McDonough 2003), applied to `refractory * (1 - core_mass_fraction)`. `SULPHUR_IN_AIR` is recalibrated so Venus stays in range. `wash_out` multiplies `so2` by a wet share when the surface liquid is `Water` or `o2 >= 0.01` bar: the ratio of SO2's lifetime in a wet or oxidising air (days, Seinfeld & Pandis) to the dry case; document the number. It runs in `bare` after `oxygenate` (the liquid is known after `climate`, O2 after `oxygenate`).
- [ ] **Step 4: Run** `atmosphere::`, `mod::`, `classify::` — Expected: PASS.
- [ ] **Step 5: Commit** `fix(planet): take sulphur from the silicate and wash it out of wet airs`, footer `Closes: #447`.

### Task 3: Derived Bond albedo and its fixed point

Opus (physics). The largest task.

**Files:**
- Create: `P/albedo.rs` (declared in `P/mod.rs`)
- Modify: `P/inputs.rs:19` (`Context`: new method)
- Modify: `P/climate.rs:30` (`Climate`), `:414-503` (`EQUILIBRIUM_ALBEDO`, `absorbing`), `:908-940` (`climate`), `:53` (`CO2_OPACITY` recalibration)
- Modify: `P/explain.rs:356-373` (`temperature_tree` replays the stored albedo)

**Interfaces:**
- Produces:
  - `pub struct Cover { pub ocean: f64, pub land: f64, pub ice: f64, pub frost: f64, pub cloud: f64 }` (fractions of the disc, 0-1) and `pub fn cover(temperature_k: f64, gases: &Gases, water: &Water) -> Cover` in `albedo.rs`;
  - `pub fn bond_albedo(cover: &Cover, gases: &Gases, deck: bool) -> f64` (surface mix under clouds, plus Rayleigh scattering of the column, plus the sulphuric deck when `deck`);
  - `pub fn ocean_fraction(oceans: f64) -> f64` (Earth's 1 ocean → 0.71; reuse the water-to-sea-level mapping of `system::generation_params` if it is a function of oceans, so look and albedo agree);
  - `impl Context { pub fn with_albedo(self, albedo: f64) -> Context }` rescaling `equilibrium_k` by `((1 - albedo) / (1 - REFERENCE_ALBEDO))^(1/4)`, `REFERENCE_ALBEDO = 0.3` being what the star module assumes;
  - `Climate` gains `pub bond_albedo: f64` (stored, ts-exported).
- Consumes: Task 2's ordering in `bare` is untouched.

- [ ] **Step 1: Write the failing tests** in `P/albedo.rs` and `P/climate.rs`:
  - `the_solar_system_has_its_albedos`: Solar System models' `climate.bond_albedo`: Earth `0.28..=0.32`, Venus `0.70..=0.80`, Mars `0.20..=0.30`, Mercury `0.06..=0.14` (bare rock, Review Focus 1).
  - Update `the_solar_system_has_its_climates` (`P/climate.rs:1036`) to keep its ranges with the derived albedo (Earth 282-294 K regulated, Venus 687-787 K, Mars 198-222 K, Titan 86-102 K).
  - `bare_rock_ignores_ocean_and_cloud`: `cover` of a body with `kept_kg = 0` and `gases.total() < 1e-6` has `ocean == 0` and `cloud == 0`.
  - `a_runaway_blends_from_the_derived_albedo`: lot 1's runaway test (`P/climate.rs:1526` family) still shows no jump: the equilibrium just inside and just outside the runaway edge differ by under 2 K (Review Focus 2).
  - `ice_cover_grows_as_the_planet_cools`: `cover(t, ...)`.ice is non-increasing in `t` from 200 K to 300 K, near Earth's about 0.1 at 288 K and 1.0 below the frozen branch's temperature.
- [ ] **Step 2: Run** `albedo::`, `climate::` — Expected: FAIL.
- [ ] **Step 3: Implement.** Albedo values per cover within the spec's ranges (ocean about 0.06, land 0.1-0.3, ice and snow 0.5-0.8, frosts high, water clouds calibrated so Earth lands at 0.30). Rayleigh grows with the column (what gives the maximum greenhouse near 8 bar CO2, Kopparapu 2013). In `climate()`, iterate: temperature → `cover` → `bond_albedo` → `context.with_albedo` → the existing branches (thermostat, `wet_climate`, `surface_temperature_k`), with damping, until the temperature moves under 0.1 K; `absorbing` applies its runaway blend on top of the derived albedo instead of `EQUILIBRIUM_ALBEDO`. This task solves the warm start only; Task 4 adds the frozen start. Recalibrate `CO2_OPACITY` on Venus at its derived albedo. `temperature_tree` rebuilds its context with `with_albedo(stored bond_albedo)` so `earth_temperature_tree_reproduces_the_model` (`P/explain.rs:591`) holds.
- [ ] **Step 4: Run** `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features -- --test-threads=3` — Expected: PASS. Then `cargo run --release -p mapgen --example climate_sweep -- sweep` and record Earth, Venus, Mars rows in the commit body.
- [ ] **Step 5: Commit** `feat(planet): derive the Bond albedo from the body's cover`, footer `Refs: #510`.

### Task 4: Snowball branch and CO2 build-up on cold planets (#510)

Opus (physics).

**Files:**
- Modify: `P/climate.rs:942` (`thermostat`), `:908` (`climate`), `:30` (`Climate`)
- Modify: `P/mod.rs:117` (`PlanetInputs`), `P/inputs.rs:229` (`InputOverrides`), `:335-350` (`draw_inputs_with`), archetype input parsing in `crates/mapgen/src/archetype.rs`
- Modify: `archetypes/earth-like.toml` (`[inputs] snowball = false`)
- Modify: `crates/mapgen/examples/climate_sweep.rs` (print both branches)

**Interfaces:**
- Consumes: Task 3's `cover`, `bond_albedo`, `with_albedo`, the fixed point.
- Produces: `PlanetInputs.snowball_draw: f64` drawn after `life_draw`; `InputOverrides.snowball: Option<bool>` (`false` → 1.0 never, `true` → 0.0 always), parsed from archetype `[inputs]`, the request and the CLI like `life`; `Climate` gains `pub snowball: bool`; `pub fn snowball_chance(...) -> f64` in `climate.rs` (arguments chosen by the builder).

- [ ] **Step 1: Write the failing tests** in `P/climate.rs`:
  - Replace `an_earth_at_1_6_au_thaws_under_its_best_co2` (`:1402`) by `an_earth_track_planet_stays_temperate_to_the_maximum_greenhouse`: Earth inputs, G2V, warm branch forced, at 1.0, 1.4, 1.6 and 1.65 AU: `temperature_k >= 273.0` and `liquid == SurfaceLiquid::Water`; at 1.8 and 2.5 AU: below 273 K. Temperature non-increasing with distance.
  - `co2_builds_up_past_the_outgassed_snapshot`: at 1.6 AU the warm branch's `co2` exceeds the pre-climate air's CO2, and never exceeds the carbon inventory (pre-escape outgassed) nor the CO2 saturation at the skin temperature.
  - `far_planets_only_freeze`: at 5 and 10 AU, `snowball_draw` 0.0 and 1.0 give identical models (Review Focus 3).
  - `snowballs_are_rare_near_earth_and_common_near_the_edge`: `snowball_chance` at 1.0 AU in `0.03..=0.15` (Earth spent about a tenth of its history in snowball states: Hoffman et al. 2017, doi:10.1126/sciadv.1600983), larger at 1.5 AU, 1.0 past the edge, larger for a stagnant lid than for plates at the same orbit.
  - `a_snowball_keeps_its_ocean_frozen`: frozen branch at 1.2 AU: `snowball == true`, `liquid == SurfaceLiquid::None`, `bond_albedo > 0.5`, `temperature_k < 260`.
  - `earth_like_never_snowballs`: the `earth-like` archetype on seeds 1-20 at 1 AU has `snowball == false`.
  - Update `derive_model_is_deterministic_and_independent` (`P/mod.rs:660`) for the new draw.
- [ ] **Step 2: Run** `climate::`, `mod::`, `archetype::` — Expected: FAIL.
- [ ] **Step 3: Implement.** The thermostat's upper bound becomes the carbon inventory (pre-escape outgassed CO2) capped by CO2 condensation at the skin temperature (`atmosphere::Frost::cap` with `CO2_FROST`); when 288 K is out of reach it takes the CO2 of the warmest climate (maximum greenhouse) and sets `regulated = false`. The frozen start runs the same fixed point from full ice cover; if it converges to a different state, the planet is bistable and `snowball = snowball_draw < snowball_chance(...)`. On the frozen branch the CO2 is the amount built up so far: the fraction `snowball_draw / snowball_chance` of the way from the warm branch's CO2 to the thaw threshold (my call, so a seed gives one state and one CO2). `snowball_chance` follows the spec's "Snowball state": the time a planet spends frozen (CO2 needed to thaw over the outgassing rate, which scales with the volcanism of Task 1) against the warm phase, 1.0 where no warm solution exists; document the rule. `climate_sweep sweep` prints both branches where both exist.
- [ ] **Step 4: Run** the full `mapgen` tests — Expected: PASS. Run `climate_sweep sweep` and `planet_census 500`; record the edge and the census snowball count in the commit body.
- [ ] **Step 5: Commit** `feat(planet): let cold planets build up CO2 and freeze into snowballs`, footer `Closes: #510`.

### Task 5: Methane cold trap, saturated greenhouse and condensation (#443)

Opus (physics).

**Files:**
- Modify: `P/atmosphere.rs:301-320,409` (`CH4_FROST`, `cold_trap`), `P/mod.rs:570` (call site)
- Modify: `P/climate.rs:123,319` (`CH4_OPACITY` law), `:731-745` (`liquid`)
- Modify: `P/moon.rs:210` test bounds if needed (Titan)

**Interfaces:**
- Consumes: Task 3's fixed point (the cold trap now needs the derived temperature).
- Produces: `pub(crate) fn methane_saturation_bar(temperature_k: f64) -> f64` in `atmosphere.rs` (from `CH4_FROST`/its liquid branch); `cold_trap` caps CH4 inside the climate's fixed point at the surface saturation (excess becomes surface condensate) and limits the greenhouse amount by the tropopause (skin, `T_eq · 0.5^(1/4)`) saturation.

- [ ] **Step 1: Write the failing tests**:
  - `P/climate.rs`: `a_methane_greenhouse_cannot_warm_a_cold_world`: inputs giving 26 bar of CH4 at T_eq 114 K (seed 182, planet d of the census, or an equivalent synthetic context) end below 150 K.
  - `P/climate.rs`: `a_trace_of_methane_makes_no_lake`: `liquid` with `ch4` 0.003 bar at 94 K, 1.5 bar N2 returns not `Hydrocarbons`; with Titan's air returns `Hydrocarbons`.
  - `P/moon.rs` `titan_comes_out_a_hydrocarbon_world` (`:210`): bounds become 90-97 K, 1.3-1.6 bar, CH4 share 0.03-0.06, Hydrocarbons.
  - `P/climate.rs`: `methane_greenhouse_saturates`: the surface temperature gain from 1 to 10 bar CH4 at Titan's T_eq is under a third of the gain from 0.01 to 0.1 bar.
- [ ] **Step 2: Run** `climate::`, `moon::` — Expected: FAIL.
- [ ] **Step 3: Implement.** Surface CH4 is capped at its saturation pressure at the surface temperature; a hydrocarbon liquid requires `p_CH4 >= x * methane_saturation_bar(T)`, `x` the methane share of a Titan sea (Raoult, with N2 and ethane: about 0.5-0.7, Tan et al. 2013 / Glein & Shock 2013; document it). The opacity uses the CH4 above the tropopause cold trap and saturates (log or capped law, builder's form); its near-infrared anti-greenhouse is added only if it moves Titan by more than 2 K (spec goal 5). The excess CH4 is a surface condensate that `liquid` can see.
- [ ] **Step 4: Run** full `mapgen` tests and `planet_census 500` — Expected: PASS; census has no planet with T_eq under 150 K above 273 K from methane, and every Hydrocarbons planet satisfies the condensation rule. Record in the commit body.
- [ ] **Step 5: Commit** `fix(planet): cold-trap methane and condense it before it makes a lake`, footer `Closes: #443`.

### Task 6: Triton's nitrogen frost (#464)

Opus (physics).

**Files:**
- Modify: `P/albedo.rs` (frost cover from N2, CH4, CO2 condensing at the surface temperature)
- Modify: `P/atmosphere.rs:409` (N2 trapped at the surface temperature inside the fixed point)
- Modify: `P/moon.rs:296` test

**Interfaces:**
- Consumes: Task 3 `cover`, Task 5's cold trap inside the fixed point.
- Produces: no new public names.

- [ ] **Step 1: Write the failing tests** in `P/moon.rs`:
  - `triton_keeps_a_thin_nitrogen_air` (`:296`): `temperature_k` in `35.0..=41.0`, `n2` in `1e-5..=3e-5`, `bond_albedo >= 0.6` (Voyager: 38 K, 1.4e-5 bar, Tyler et al. 1989).
  - `a_moon_uses_its_own_cover`: Titan, an icy airless moon (Europa-like) and Io from one system get three different `bond_albedo`s, each in its body's range (Titan 0.2-0.35, Europa 0.5-0.75, Io 0.4-0.7) (Review Focus 4).
- [ ] **Step 2: Run** `moon::` — Expected: FAIL.
- [ ] **Step 3: Implement.** N2 is trapped at the surface's frost temperature, the frost cover raises the albedo, the fixed point settles. If 38 K needs N2 ice's low thermal emissivity (about 0.6-0.8), it is a property of every N2-frost surface, not of Triton. If the grey climate cannot reach the range, stop and report why (spec goal 6) instead of tuning.
- [ ] **Step 4: Run** full `mapgen` tests — Expected: PASS.
- [ ] **Step 5: Commit** `fix(planet): let nitrogen frost brighten and chill Triton`, footer `Closes: #464`.

### Task 7: Look: cloud cover, sulphuric deck, methane clouds, snowballs

Sonnet. Needs the viewer for the cloud kind.

**Files:**
- Modify: `crates/mapgen/src/atmosphere.rs:63` (`CloudKind::Methane`), ts export, `web/src/lib/viewer/shaders.ts` (render it: sparse, pale, under the haze)
- Modify: `P/interim.rs:56` (`base_of`), `:279` (`look_for`), `:375-411` (`sky_of`: `cloud_cover` from `Cover.cloud`, cloud kind order)
- Modify: `looks/frozen.toml` only if the viewer check shows relief or the FrozenOcean/land contrast lost

**Interfaces:**
- Consumes: `Climate.bond_albedo`, `Climate.snowball`, `albedo::cover`.
- Produces: `CloudKind::Methane` (serialised `"methane"`).

- [ ] **Step 1: Write the failing tests** in `P/interim.rs`:
  - `titans_clouds_are_methane` (next to `:1114`): `sky_of(titan).clouds == CloudKind::Methane`.
  - `a_frozen_earth_has_no_acid_cloud`: earth inputs at 1.8 AU: `clouds != CloudKind::Sulfuric`.
  - `venus_has_a_sulfuric_deck_and_mars_dust` (`:1142`) gains `venus.cloud_cover > 0.5`.
  - `a_snowball_renders_frozen`: a snowball model (Task 4's test case) has `base_of == "frozen"`, and its world's biomes include both `FrozenOcean` and at least one of `Mountain`, `Snow`, `IceCap`, `GlacialPlain` on land.
  - `earths_sky_is_earths` (`:1097`): Earth's `cloud_cover` within 0.05 of today's 0.0.
- [ ] **Step 2: Run** `interim::` — Expected: FAIL.
- [ ] **Step 3: Implement.** `sky_of` maps the model's cloud fraction to `cloud_cover` around Earth's (Earth → 0.0). Cloud kind order: airless → None; runaway or a warm dry air with `so2 >= SULFURIC_MIN_BAR` and water to make the acid → Sulfuric (the deck forms from H2SO4 condensing in a warm air; a frozen world gets none: write the rule from that, document it); liquid hydrocarbons or CH4 condensing → Methane; wet → Water; thin → Dust. `base_of` sends a snowball to `frozen`.
- [ ] **Step 4: Run** `interim::` and `cd web && npm test` — Expected: PASS.
- [ ] **Step 5: Viewer check.** Dispatch `viewer-visual-check` on a local server of this branch (not port 3000, not the data checkout) with: a snowball at 1.3 AU (use the request override `snowball = true`), a frozen earth-like at 2 AU, `venus-like`, Titan. Expected: snowball shows mountain ranges and readable coastlines; no acid cloud on the frozen one; Venus's deck visible; Titan's clouds not white water clouds. Fix and re-check what fails.
- [ ] **Step 6: Commit** `feat(planet): show snowballs, Venus's deck and Titan's methane clouds`, footer `Refs: #423`.

### Task 8: Card causes and labels

Sonnet.

**Files:**
- Modify: `P/explain.rs` (`temperature_tree :356`, `liquid_tree :479`, `Quantity :55`, notes `:131`)
- Modify: `web/src/lib/i18n/en.ts` (~`:400-481`), `web/src/lib/i18n/fr.ts` (~`:443`), generated `web/src/lib/api/types/Quantity.ts`, `web/src/lib/causes.test.ts`, `web/src/components/body/fixtures/earth-like.json` (regenerate)

**Interfaces:**
- Consumes: `Climate.bond_albedo`, `Climate.snowball`, `regulated`.
- Produces: `Quantity::BondAlbedo`; notes `"snowball"`, `"co2_build_up"`, `"methane_trap"`.

- [ ] **Step 1: Write the failing tests** in `P/explain.rs`: Earth's temperature tree contains a `BondAlbedo` cause; a snowball's carries the `"snowball"` note; a planet at the maximum greenhouse (unregulated, 1.65 AU) carries `"co2_build_up"`; Titan's carries `"methane_trap"`; every tree stays at most 3 levels deep (`:607`).
- [ ] **Step 2: Run** `explain::` — Expected: FAIL.
- [ ] **Step 3: Implement** the causes and the EN and FR labels. The snowball note says why it froze (bright ice reflects the light) and how it ends (volcanic CO2 builds up until it thaws), in the educational tone of the existing notes.
- [ ] **Step 4: Run** `explain::` and `cd web && npm test` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(planet): explain the albedo, snowballs and the CO2 build-up on the card`, footer `Refs: #423`.

### Task 9: Formats, documentation and acceptance

Sonnet.

**Files:**
- Modify: `crates/mapgen/src/export/raw_data.rs:44-88`, `web/src/lib/formatVersion.ts:6`, `crates/mapgen/src/system.rs:60-74`
- Modify: README climate and card sections (find with `grep -n '^#' README.md`), `docs/system-spec.md` climate part, the doc header of `crates/mapgen/examples/climate_sweep.rs`, `CHANGELOG` if present

**Interfaces:**
- Consumes: all earlier tasks.

- [ ] **Step 1: Write the failing test** in `crates/mapgen/tests/old_format.rs`: a system stored at format 7 (without `bond_albedo`, `snowball`, `secular_w`) loads as outdated without panicking (Review Focus 5).
- [ ] **Step 2: Run** `cargo test -p mapgen --test old_format` — Expected: FAIL.
- [ ] **Step 3: Implement.** Ask the main session for the agreed `FORMAT_VERSION` and `SYSTEM_FORMAT_VERSION` numbers (do not pick them), bump them and `MIN_FORMAT_VERSION` with their doc entries. Update the documentation listed above.
- [ ] **Step 4: Run** the full workspace suite (throttled command of Global Constraints) and `cd web && npm test` — Expected: PASS. Run `climate_sweep sweep`, `climate_sweep moons`, `planet_census 500` and paste every Acceptance number of the spec, with its range, in the commit body.
- [ ] **Step 5: Commit** `feat(planet)!: bump the world and system formats for lot 2b`, with a `BREAKING CHANGE: stored worlds and systems of the old formats are outdated.` footer and `Refs: #423`.
