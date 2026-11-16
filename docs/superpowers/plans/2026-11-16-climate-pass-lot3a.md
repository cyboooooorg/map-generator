# Climate pass lot 3, part A: subsurface oceans Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the oceans under the ice of icy bodies come from resonance heating, partial differentiation, ammonia and convective shells. Decide geysers by physics, and show the new interior on the card.

**Architecture:**

- A new `planet::resonance` module turns a moon system's 2:1 chains into equilibrium tidal heat. The system builder computes that heat from the siblings and hands it to each moon through `HostTide`. `body_context` rebuilds it from a stored system so that `explain` agrees.
- `planet::differentiation` derives how much of a moon's ice separated from its rock, from the impact flux at its orbit.
- `ice_shell.rs` gains the ammonia liquidus, the convective shell, polar thinning and the geysers.
- `rings::dust_rings` takes its plume moon from the model.
- The card reads the new `IceShell` and `Geysers` fields and their cause trees.

**Tech Stack:** Rust (crate `mapgen`, ts-rs bindings), Svelte/TypeScript web app (vitest).

**Spec:** `docs/superpowers/specs/2026-11-16-climate-pass-lot3-design.md` (part A; the "Calls made" section settles the builder-check questions).

## Global Constraints

- Every physical relation carries its source (author, year, DOI) in its doc comment. Every calibrated constant is named and says what it was calibrated on.
- No per-archetype heat, shell or ocean values. Archetypes give orbit and composition only, plus the new resonance partner.
- No geysers drawn at odds. `PLUME_ODDS` is removed.
- No `FORMAT_VERSION` gate for the old shell rule.
- Do not raise `ICY_K2_OVER_Q`.
- Tests on this shared machine run throttled, one full run at a time:
  `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`.
- Never run `npm ci` in the worktree (`web/node_modules` is a symlink). Never use port 3000.
- Web checks: `cd web && npm run check && npm test`.
- Lint: `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Documentation ships in the task that changes behaviour: `docs/science/planet-interiors.md` (§ "The ice shell and the ocean beneath" :250, "heat inside" :155, "Where it departs" :410, sources :475), `docs/science/ring-moons.md` / `ring-extent.md` (plume :181 :245 :299), `docs/system-spec.md` (:323 :430 :586), README (:131 :250 :343 :353).
- Commits follow Conventional Commits (`feat(planet): …`) and end with the session's attribution lines.
- System format becomes 9, to be confirmed with the peer sessions before merge.

## Review Focus

1. **A stored system reopened for `explain`.** `body_context` must rebuild the resonant heat from the sibling bodies. Otherwise `explain` returns None for every resonant moon (explain.rs:240 refuses a tidal heat that differs). Test it in Task 2.
2. **A moon with no neighbour, or with a neighbour far from 2:1.** A moon alone around its giant, or one whose neighbour sits at a period ratio of about 4 (`slots.retain` can drop the slot in between), gets no resonant heat. It keeps `tidal_power_w` with its own e. It must not panic. Test it in Task 2.
3. **A body with no water layer, or with water but no ammonia.** It goes through the liquidus and the convective shell without NaN. Today a body with 0 % NH3 (e.g. stripped of hypervolatiles, `moon.rs:124`) must still get 273 K. Test it in Task 4.
4. **A whole ocean frozen by convection.** `ice_shell` then returns a shell with `ocean_km == 0.0` and no geysers. It must never return a negative ocean. Test it in Task 5.
5. **Lone worlds around a giant** (`host_chain`, siblings with no mass and with a mass derived from the radius). They go through the same resonance code as whole systems. Test it in Task 2.

---

### Task 1: Ocean census (baseline)

Model: sonnet.

**Files:**
- Create: `crates/mapgen/examples/ocean_census.rs`

**Interfaces:**
- Produces: `cargo run --release -p mapgen --example ocean_census -- [SEEDS=20] [SYSTEMS=300]`.
  - It prints one table per moon archetype (`enceladus-like`, `europa-like`, `ganymede-like`, `titan-like`, `callisto-like`, `io-like`), each asked alone in a whole system over the seeds. Columns: min, median and max of shell km, polar shell km, ocean km, tidal W, moment of inertia, convecting share and geyser share. Print `-` for fields that do not exist yet.
  - It also prints the same fields for the drawn icy moons and icy planets of `SYSTEMS` drawn systems, plus the plume-ring count.
  - Reuse the whole-system request path that `system.rs` tests use (`requested(seed, &[(name, 1)])`). If those helpers are `#[cfg(test)]`, call the public request API they wrap.

- [ ] **Step 1:** Write the example. Read fields through `Body.model` only, so the later tasks fill in the columns.
- [ ] **Step 2:** Run it on `main`'s code and save the output in the SDD ledger as `baseline.txt`. Expected: Enceladus ocean 0 and tidal power about 2.8e9 W. Ganymede and Callisto shells over 180 km.
- [ ] **Step 3:** Commit: `test(planet): add an ocean census of icy moons`.

### Task 2: Resonant chains and equilibrium heating

Model: opus.

**Files:**
- Create: `crates/mapgen/src/planet/resonance.rs` (registered in `planet/mod.rs`)
- Modify: `planet/inputs.rs:47` (`HostTide`), `planet/mod.rs:530-549` (`internal_heat`), `system.rs:1691` (moon loop), `system.rs:2114` (`body_context`), `system.rs:2536` (`host_chain`), `request.rs:946` (`host_tide`), `planet/heat.rs:281` (test)

**Interfaces:**
- Produces:
  - `pub struct Orbiter { pub mass_kg: f64, pub a_m: f64, pub period_s: f64, pub eccentricity: f64, pub radius_m: f64, pub k2_over_q: f64 }`
  - `pub fn chains(periods_s: &[f64]) -> Vec<std::ops::Range<usize>>`. It takes the moons sorted by distance and returns runs of consecutive moons whose neighbouring period ratio lies in `RESONANCE_BAND = (1.8, 2.2)` (2:1 within the `CHAIN_STEP` jitter of ±5 % in a). Runs of length 1 are omitted.
  - `pub fn host_k2_over_q(host_kg: f64, host_radius_m: f64, inner: &Orbiter, age_gyr: f64) -> f64`. Resonance locking: the host dissipates so that the chain's inner moon migrates outward on `a/ȧ = MIGRATION_AGES × age`, with `MIGRATION_AGES = 1.0`. Calibrated on Lainey et al. 2009 (Io, about 3.6 Gyr) and 2012 (Enceladus, about 4.4 Gyr); cites Fuller, Luan & Quataert 2016, doi:10.1093/mnras/stw609. The host's torque on a moon is `T = 1.5 (k2/Q) G m² R⁵ / a⁶`.
  - `pub fn equilibrium_heat_w(host_kg: f64, host_radius_m: f64, host_k2_over_q: f64, chain: &[Orbiter]) -> Vec<f64>`. The total is `Σ nᵢTᵢ − n̄ ΣTᵢ`, with `n̄ = Σ nᵢLᵢ / Σ Lᵢ` and `Lᵢ = mᵢ nᵢ aᵢ²` (Meyer & Wisdom 2007, doi:10.1016/j.icarus.2007.03.001, generalised to N moons locked together). The total is shared in proportion to each moon's own `heat::tidal_power_w` at its eccentricity. The total is clamped at ≥ 0.
  - `HostTide` gains `pub resonant_heat_w: Option<f64>`. When it is `Some`, `internal_heat` uses it as `tidal_w` in place of `tidal_power_w`.
  - `pub fn resonant_heat(host_kg: f64, host_radius_m: f64, age_gyr: f64, moons: &[Orbiter]) -> Vec<Option<f64>>`. The one entry point used by `system.rs` (generation and `body_context`), `host_chain` and `request::host_tide`.

- [ ] **Step 1: Failing tests in `resonance.rs`.**
  - `enceladus_and_dione_heat_as_observed`: Saturn 5.683e26 kg, R 6.0268e7 m. Enceladus 1.08e20 kg, a 2.3802e8 m, P 1.370 d, R 2.521e5 m, e 0.0047, icy k2/Q. Dione 1.095e21 kg, a 3.774e8 m, P 2.737 d, R 5.61e5 m, e 0.0022. Lainey's k2/Q is 2.3e-4. Assert that Enceladus's share is in `10e9..20e9` (about 14 GW by hand).
  - `locking_gives_saturn_and_jupiter_their_measured_dissipation`: `host_k2_over_q` at 4.57 Gyr. Saturn with Enceladus as the inner moon: `1.5e-4..3.5e-4`. Jupiter (1.898e27 kg, R 7.149e7 m) with Io (8.93e22 kg, a 4.217e8 m, P 1.769 d) as the inner moon: `0.6e-5..1.6e-5`.
  - `the_laplace_chain_heats_io_most`: Io, Europa and Ganymede (1.48e23 kg, a 1.0704e9 m, P 7.155 d) with the locked k2/Q. The total is in `2e13..2e14` and Io's share is over 80 %.
  - `chains_need_a_two_to_one`: periods `[1, 2, 4, 16]` → `[0..3]`; `[1, 4]` → `[]`.
  - `equal_migration_gives_no_heat`: two moons whose torques give equal `ȧ/a` → total `< 1e-6 ×` the `Σ nT` term.
- [ ] **Step 2:** Run `cargo test -p mapgen resonance`. Expected: FAIL (module missing).
- [ ] **Step 3:** Implement the module. Wire `resonant_heat` into `system.rs`'s moon loop: the sorted `moons` are in hand at :1661, and `giant_kg` and the radius at :1659-1660. Wire it into `host_chain` and `request::host_tide` too. In `body_context` (:2114), rebuild the `Orbiter`s from the sibling bodies with the same `parent`, so that `explain` gets the same heat.
- [ ] **Step 4:** Add the integration tests in `system.rs`:
  - `explain_agrees_on_a_resonant_moon`: a seed whose giant has a chain; `explain` is `Some` for each chain moon.
  - `a_moon_without_a_neighbour_keeps_its_own_tide`: `resonant_heat` on one moon, or on two at a ratio of 4 → `[None]`, `[None, None]`.
  - `lone_world_siblings_are_in_resonance`: a lone world on `host_chain` gets `Some` heat.
- [ ] **Step 5:** Update `the_icy_response_heats_europa_and_enceladus_as_observed` (heat.rs:281): the own-tide Enceladus stays 2e9..15.8e9 W, and its doc comment says that the resonance now supplies the rest. Update `docs/science/planet-interiors.md` § heat inside (with sources) and § Where it departs.
- [ ] **Step 6:** Run the full throttled suite. Expected: PASS, apart from archetype or census tests whose numbers moved. Fix them only where a test pinned the old heat on purpose, and list them in the commit body.
- [ ] **Step 7:** Commit: `feat(planet): heat moons in resonance from their host's tides`.

### Task 3: Resonance partners of moon archetypes

Model: sonnet.

**Files:**
- Modify: `archetype.rs:491` (`ArchetypeMoon`), `moons.rs:310` (`AskedMoon`), `moons.rs:1395-1474` (`of_giant`), `archetypes/{enceladus,europa,io}-like.toml`, README :343 :353

**Interfaces:**
- Consumes: `resonance::chains` (Task 2).
- Produces:
  - `ArchetypeMoon.resonance: Option<ArchetypeResonance>`, with `#[serde(default)]` and ts-rs exported.
  - `pub struct ArchetypeResonance { pub partner_mass_ratio: ParamDefault }`, `deny_unknown_fields`. The ratio is the partner's mass over the host's mass. The partner sits outward at a 2:1 period ratio.
  - `AskedMoon.partner_mass_ratio: Option<f32>`.
  - `of_giant` places a regular disk moon of that mass at `a × CHAIN_STEP` (no jitter), with a forced eccentricity, and drops chain slots within the existing `slots.retain` distance of it.
- TOML values (sources in comments):
  - enceladus-like: `[moon.resonance] partner_mass_ratio = [1.85e-6, 2.0e-6]` (Dione 1.095e21 kg / Saturn 5.683e26 kg = 1.93e-6).
  - europa-like: `[7.5e-5, 8.1e-5]` (Ganymede / Jupiter = 7.80e-5).
  - io-like: `[2.4e-5, 2.6e-5]` (Europa / Jupiter = 2.53e-5).

- [ ] **Step 1: Failing tests.**
  - In `archetype.rs`, `moon_resonance_parses_and_defaults`: enceladus-like has `Some` and titan-like has `None`.
  - In `system.rs`, `an_asked_resonant_moon_gets_its_partner`: for seeds 1..=10, `requested(seed, &[("enceladus-like", 1)])` has a sibling with a period ratio in 1.9..2.1 and a mass ratio in the band, and Enceladus's tidal heat is in `5e9..30e9` W.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement. Update every test fixture that builds `ArchetypeMoon` or `AskedMoon` by hand.
- [ ] **Step 4:** Run the full throttled suite and the web checks (the ts-rs types change). Expected: PASS.
- [ ] **Step 5:** Commit: `feat(archetypes): give moon archetypes their resonance partner`.

### Task 4: Partial differentiation and the ammonia liquidus

Model: opus.

**Files:**
- Create: `crates/mapgen/src/planet/differentiation.rs`
- Modify: `planet/mod.rs:143` (`Material`), `planet/interior.rs:60` (`solid_interior`), `planet/moon.rs:45` (`MoonFormation`), `system.rs:2085` (`moon_formation`) and the moon loop, `planet/ice_shell.rs` (`MELT_K` use), `planet/reference.rs:70`

**Interfaces:**
- Produces:
  - `MoonFormation.impact_flux: f64`, from `impacts::giant_moon_flux(star_mass, luminosity, giant_orbit_au, giant_mass_earth, moon_a_km)` (impacts.rs:166) in the system loop and in `host_chain`.
  - `pub fn differentiated_share(mass_earth: f64, radius_earth: f64, impact_flux: f64, age_gyr: f64) -> f64` (0..1), the share of the ice that separated from the rock. Cite Barr & Canup 2008, doi:10.1016/j.icarus.2008.07.004 (impact heating during late bombardment). Any calibrated threshold is named and calibrated on Ganymede versus Callisto.
  - `Material::RockIce`: the undifferentiated mixture, between the rock and the water layers.
  - `pub fn moment_of_inertia(interior: &Interior) -> f64` (C/MR²), stored in a new field `PlanetModel.moment_of_inertia: f64`.
  - `pub fn liquidus_k(nh3_mass_fraction: f64) -> f64` in `ice_shell.rs`. It follows the H2O–NH3 liquidus at low pressure (cite e.g. Leliwa-Kopystyński et al. 2002, doi:10.1006/icar.2001.6762), down to the eutectic of 176 K at about 0.33. The ocean's NH3 fraction is the body's NH3 (`Volatiles.nh3`, as a share of the ice) over the liquid mass, so it rises as the shell thickens. Solve for it with the shell.
  - `SOLAR_BODIES` gains `body("Enceladus", 1.81e-5, 0.0, 0.40, 0.0, 9.58, 0.0395)`, and the reference test list gains its name.

- [ ] **Step 1: Failing tests.**
  - `liquidus_runs_from_pure_water_to_the_eutectic`: `liquidus_k(0.0) == 273.0`; `liquidus_k(0.33)` within 176 ± 3; `liquidus_k(0.10)` in 250..265; monotonic decreasing on 0..0.33.
  - `ganymede_is_differentiated_and_callisto_is_not` in `system.rs`: asked ganymede-like has moment of inertia 0.30..0.32; callisto-like 0.34..0.37; titan-like 0.33..0.36 (Iess et al. 2010, doi:10.1126/science.1182583); europa-like ≤ 0.35. Use seeds 1..=10.
  - `a_body_without_ammonia_melts_at_273` (Review Focus 3).
  - `enceladus_is_a_reference_body`: `reference("Enceladus")` exists.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement. The `RockIce` layer's density is the mixture's, and its radiogenic heat is its rock share's. The ice shell sits on the pure water layer only.
- [ ] **Step 4:** Update `docs/science/planet-interiors.md` (interior layers, ice shell, sources) and system-spec :586 (the stored fields).
- [ ] **Step 5:** Run the full throttled suite. Expected: PASS.
- [ ] **Step 6:** Commit: `feat(planet): leave cold moons partly differentiated, melt ice with ammonia`.

### Task 5: Convective ice shells and polar thinning

Model: opus.

**Files:**
- Modify: `planet/ice_shell.rs` (all), `planet/mod.rs:501`

**Interfaces:**
- Consumes: `liquidus_k` and the `RockIce` interior (Task 4); `InternalHeat.tidal_w` (Task 2).
- Produces:
  - `IceShell` gains `pub convecting: bool`, `pub lid_km: f64` (the conductive lid; equal to `shell_km` when not convecting), `pub polar_shell_km: f64` and `pub liquidus_k: f64`.
  - `pub fn ice_shell(interior: &Interior, heat: &InternalHeat, surface: Option<&Surface>, nh3_ice_fraction: f64) -> Option<IceShell>`.
- Physics, each relation cited:
  - The onset of convection and the stagnant-lid convective flux (Solomatov & Moresi 2000; Barr & McKinnon 2007, doi:10.1029/2006JE002781). Use a Newtonian ice viscosity at the melting point, with its activation energy (Goldsby & Kohlstedt 2001, doi:10.1029/2000JB900336).
  - The shell is in equilibrium with the heat from below plus the tidal heat in the ice.
  - If convection carries more than is supplied, the ocean freezes down to the liquidus allowed by the concentrated NH3, or completely.
  - Polar thinning comes from the latitude pattern of tidal heating in a thin shell over an ocean (Beuthe 2013, doi:10.1016/j.icarus.2012.11.020, or Čadek et al. 2016). It applies only when tidal heat is a significant share of the shell's budget.

- [ ] **Step 1: Failing tests (system.rs, asked archetypes, seeds 1..=10).** The bands come from the spec's Acceptance table:
  - `enceladus_shell_and_ocean`: `shell_km` 15..40, `polar_shell_km < 10`, `ocean_km > 0`.
  - `europa_shell_and_ocean`: shell 5..40, ocean 50..200.
  - `ganymede_shell_and_ocean`: shell 80..250, ocean > 0 above high-pressure ice.
  - `titan_shell_and_ocean`: shell 50..200, ocean > 0.
  - `callisto_has_no_ocean_or_a_thin_one`: ocean < 50.
  - Unit test `a_frozen_ocean_is_zero_not_negative` (Review Focus 4).
  - Unit test `a_five_kilometre_shell_does_not_convect`.
- [ ] **Step 2:** Run them. Expected: FAIL on the thick moons.
- [ ] **Step 3:** Implement. Replace the conductive-only docstring departures at ice_shell.rs:119, moon.rs:14 and system.rs:5001. Drop the `// Its ocean: issue #463.` note at system.rs:5047 and assert Enceladus's ocean there.
- [ ] **Step 4:** Run the census (Task 1) and compare with `baseline.txt`. Save it as `after-task5.txt` in the ledger. If an archetype band fails on some seeds, report the seeds and values rather than tuning a constant silently.
- [ ] **Step 5:** Update `docs/science/planet-interiors.md` § The ice shell and the ocean beneath, and § Where it departs: remove "the ice shell is conductive and pure", and add what is still left out (salts, pressure on the liquidus).
- [ ] **Step 6:** Run the full throttled suite. Expected: PASS.
- [ ] **Step 7:** Commit: `feat(planet): convect thick ice shells, thin them at the poles`.

### Task 6: Geysers and the plume ring

Model: opus.

**Files:**
- Create: `crates/mapgen/src/planet/geysers.rs`
- Modify: `planet/mod.rs:227` (`PlanetModel`), `rings.rs:731` (`DustSource`), `rings.rs:760-839`, `system.rs:1773-1784`, `docs/science/ring-extent.md`, `docs/system-spec.md:430`

**Interfaces:**
- Consumes: `IceShell` (Task 5), `InternalHeat`.
- Produces:
  - `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub struct Geysers { pub south: bool, pub fractures: Vec<Fracture> }` (ts-rs).
  - `pub struct Fracture { pub lat_deg: f32, pub lon_deg: f32, pub azimuth_deg: f32, pub length_km: f32 }`: the fracture's centre, its strike and its length.
  - `PlanetModel.geysers: Option<Geysers>`.
  - `pub fn geysers(shell: &IceShell, heat: &InternalHeat, radius_km: f64, gravity: f64, seed: u32) -> Option<Geysers>`. The cited cracking criterion is left to the implementer: a tidal or ocean-pressurisation stress against the tensile strength of ice through the polar shell (Manga & Wang 2007, doi:10.1029/2007GL029297, or Rudolph & Manga 2009, doi:10.1016/j.icarus.2009.01.015). The pole comes from the seed. The fractures are parallel, about 35 km apart, about 130 km long, and fill the region where the shell passes the criterion (Porco et al. 2006, doi:10.1126/science.1123013). They scale with that region's size.
  - `DustSource.icy` becomes `DustSource.geysers: bool`. `dust_rings` picks the innermost source with geysers within `PLUME_REACH`. `PLUME_ODDS` is removed.

- [ ] **Step 1: Failing tests.**
  - `enceladus_always_vents`: asked enceladus-like, seeds 1..=10: `geysers` is `Some`, with 3 to 6 fractures, all poleward of 60°, and the plume ring's `source` is its id.
  - `big_moons_never_vent`: ganymede-, titan- and callisto-like over seeds 1..=10: `None`.
  - `europa_may_vent`: no assert on the share; print it for the census.
  - `a_rocky_moon_has_no_geysers`: io-like → `None`. Keep rings.rs:1281.
  - `plume_rings_follow_geysers` (replaces the 60..140 count at rings.rs:1274): over 500 drawn systems, every plume ring's source has geysers, and every system with a geyser moon within reach has exactly one plume ring.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the census. It reports the geyser share of drawn icy moons and the period ratio or e of each geyser moon (spec Acceptance).
- [ ] **Step 5:** Update the docs. Run the full throttled suite. Expected: PASS.
- [ ] **Step 6:** Commit: `feat(planet): vent geysers from thin polar shells, feed plume rings from them`.

### Task 7: System format 9

Model: sonnet.

**Files:**
- Modify: `system.rs:60-79`, `crates/mapgen/tests/old_format.rs:17-21`

- [ ] **Step 1:** Change the old-format test to write version 8 and expect "format version 8". Rename it `a_system_of_format_8_is_outdated_without_panicking`. Run it. Expected: FAIL.
- [ ] **Step 2:** Bump `SYSTEM_FORMAT_VERSION` to 9 with a history line naming lot 3: resonant heat, differentiation, convective shell, geysers.
- [ ] **Step 3:** Run the full throttled suite (server tests included). Expected: PASS.
- [ ] **Step 4:** Commit: `feat(planet)!: bump the system format for lot 3`, with a `BREAKING CHANGE:` footer saying that stored systems are outdated.

### Task 8: Cause trees and the card

Model: sonnet.

**Files:**
- Modify: `planet/explain.rs` (`Quantity` :58, `heat_flux` :312, roots :247-254), `web/src/lib/planetCard.ts:196`, `web/src/components/body/InteriorCard.svelte:35-41`, `web/src/lib/causes.ts`, `web/src/lib/causes.test.ts:15`, `web/src/lib/i18n/{en,fr}.ts`, `web/src/lib/planetCard.test.ts:127`, `web/src/components/body/cards.test.ts:30`, README :250

**Interfaces:**
- Consumes: `IceShell` fields (Task 5), `Geysers` (Task 6), `moment_of_inertia` (Task 4), `HostTide.resonant_heat_w` (Task 2).
- Produces:
  - `Quantity` gains `ShellThickness`, `OceanDepth`, `PolarShell`, `MomentOfInertia`, `ResonantHeat` and `HostDissipation`.
  - `TidalHeat` gets children: `ResonantHeat ← HostDissipation, Age` when resonant, else `Eccentricity` as now.
  - A new root `ocean_tree`: `OceanDepth ← ShellThickness ← (HeatFlux, convecting note), Liquidus (NH3)` and `PolarShell ← TidalHeat`. Cause notes `convecting`, `ammonia` and `geysers` go in `causes.notes`.
- Card lines (FR / EN), in the interior card:
  - "océan de {ocean} sous {shell} de glace ({convective | conductive})" / "ocean of {ocean} under {shell} of ice (…)";
  - "chaleur de marée {W}" with "(résonance 2:1)" when resonant;
  - "moment d'inertie {C/MR²} — {différencié | partiellement différencié}" (threshold 0.33);
  - "geysers actifs au pôle {sud | nord}".

- [ ] **Step 1: Failing tests.**
  - Rust: `explain_has_an_ocean_tree_for_europa`.
  - Web: `planetCard.test.ts` for each new line (fixtures with and without geysers); `causes.test.ts` lists the new quantities.
- [ ] **Step 2:** Run `cargo test -p mapgen explain` and `cd web && npm test`. Expected: FAIL.
- [ ] **Step 3:** Implement. Run `cargo test` first so that ts-rs regenerates the types.
- [ ] **Step 4:** Update README :250 (what the card shows).
- [ ] **Step 5:** Run the web checks, the throttled suite and lint. Expected: PASS.
- [ ] **Step 6:** Commit: `feat(web): show the ocean, the shell and the geysers on the card`.

### Task 9: Final census and branch review

Model: main session.

- [ ] **Step 1:** Run `ocean_census` and put the before/after table in the PR description. The main session judges the drawn icy planets' and moons' coherence.
- [ ] **Step 2:** Have `branch-reviewer` review the whole branch against the spec and this plan.
- [ ] **Step 3:** Fix what it finds, re-run the tests, then rebuild `develop` (after warning the peers) for the user's test.
