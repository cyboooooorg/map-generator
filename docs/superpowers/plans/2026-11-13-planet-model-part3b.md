# Physical Planet Model (Part 3b) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace planet presets with archetypes (named sets of model
inputs), so every planet, including the one a request asks for, is
derived by the planet model; remove the direct knobs from requests and
stop embedding a `Preset` in `WorldParams`.

**Architecture:** An `Archetype` (`crates/mapgen/src/archetype.rs`,
files in `archetypes/`) holds model inputs and a preferred placement,
each a fixed value or a range. A request names an archetype (or none)
and overrides inputs; `draw_inputs_with` draws the inputs the formation
context would give and replaces the ones set. The world asked for goes
through `planet_model` like its neighbours, then through
`planet::interim` as in 3a. What the generator still needs from a
preset until #424 (biome rules, remap, climate shift, sky, geology,
giant look) becomes a `Look`, an internal rule set chosen by
`interim` and never named by a user. `WorldParams` carries the class,
the archetype's name (display only), the model and the look.

**Tech Stack:** Rust (crates `mapgen`, `server`, `cli`), serde, toml,
ts-rs, rand; Svelte 5 and TypeScript (`web/`), vitest.

**Spec:** `docs/superpowers/specs/2026-11-11-physical-planet-model-design.md`
("Presets and the form", "Storage and formats", "Order of work" step 3
and "Lone worlds"). Previous plan:
`docs/superpowers/plans/2026-11-13-planet-model-part3a.md` ("Parts of
step 3" defines 3b).

## Decisions taken with the user (2026-11-13)

1. An archetype's inputs are each a fixed value or a `[min, max]`
   range, like today's `ParamDefault`. Solar System archetypes use
   narrow ranges around the real body; #418 may pin exact values later.
2. The request and the CLI gain input overrides in 3b (mass, water in
   Earth oceans, core share, envelope share, CO₂, N₂, radiogenic
   abundance, life yes/no). The web form only offers the archetype in
   3b; its input fields wait for step 5.
3. Large-moon archetypes (Io, Europa, Ganymede, Titan) wait for part 4.
   3b ships the eight Solar System planets and the exoplanet kinds.
4. #452 is folded in: `ice-world` only names planets whose surface
   keeps ice or water.

## Global Constraints

- Scope: only what this plan lists. Moons keep `rocky-moon` and
  `icy-moon` looks and have no model (part 4). No new form fields for
  inputs (step 5). No relief change (3c: #312, #313, #315).
- Anti-goal "types in disguise": no code outside `planet::interim`
  maps a class or an archetype name to a look, a mass range or an
  orbit band. `system::band`, `requested_mass` and `PLANET_TYPES` go
  away; an archetype places itself by its own insolation and mass.
- Formats: `SYSTEM_FORMAT_VERSION` 3 → 4 (`system.rs:62`);
  `FORMAT_VERSION` and `MIN_FORMAT_VERSION` 17 → 18
  (`export/raw_data.rs:55,63`), with `web/src/lib/formatVersion.ts:6`
  in the same commit. Server index `SCHEMA_VERSION` 4 → 5
  (`server/src/index.rs:74`), since stored job requests carry the
  removed fields. The peer sessions stay idle until part 3 is finished
  (the user, 2026-11-13): no warning needed.
- No legacy path and no backward compatibility (the user,
  2026-11-13): old type names (`terran`, `caustic`, `frozen`,
  `volcanic`, `barren`) are removed everywhere, including the i18n
  `bodyTypes` labels; old requests with knob fields are refused by
  `deny_unknown_fields`.
- Units: water is given in Earth oceans (`planet::OCEAN_KG` = 1.4e21
  kg each); CO₂ and N₂ are given as factors on the formation rule of
  `planet::inputs::volatiles` (1 = typical, today's draw is
  `10^N(0, 0.3)`); radiogenic as today (Earth = 1); insolation in
  Earth's (1 = 1361 W/m²).
- Determinism: a request without overrides draws the same inputs as
  `draw_inputs` (same draws, same order); overrides replace values
  after the draws, never skip one.
- Every constant cites its source or says `Calibration, no source.`
  and names what it reproduces. Archetype values come from
  `planet::reference::SOLAR_BODIES` and the sources cited there.
- Shared machine: work in `~/Developer/mg-planet-model-p3b`, branch
  `feat/planet-model-part3b`. Tests:
  `RAYON_NUM_THREADS=4 cargo test -j 4 --workspace --all-features -- --test-threads=3`,
  clippy `cargo clippy -j 4 --workspace --all-targets --all-features -- -D warnings`,
  web `cd web && npm run check && npx vitest run`. Never `npm ci` in
  the worktree (symlink `web/node_modules` from the main checkout if
  missing), never port 3000, never write to the main checkout's
  `systems/`, `worlds/`, `db/`.
- ts-rs bindings regenerate on `cargo test`; commit
  `web/src/lib/api/types/` with the Rust change that alters them, and
  delete the bindings of removed types (`Preset.ts`, `PresetInfo.ts`,
  `Defaults.ts`, `ParamSpec.ts` if unused…).
- Rust tests pass at every commit. The web may fail `npm run check`
  between Task 4 and Task 8; Task 8 ends with it green.
- Documentation ships in the task that changes the behaviour: the
  README sections are found with `grep -n` (the file is 180 KB, never
  read whole): CLI table (around :29-120), "Planet presets" (:304-390),
  repository layout (:204-208), the server environment table.

## Review Focus

1. **Overrides that contradict the mass.** `--mass 0.1 --water 5000`
   asks for more water than the planet weighs. The request is refused
   with a message naming both values, not clamped and not panicking
   (Task 4 test `water_heavier_than_the_planet_is_refused`).
2. **A giant archetype asked alone.** `--planet jupiter-like` must
   take the giant path with the archetype's mass, and `--planet
   hot-jupiter` sits at its period, not at an insolation (Task 4 tests
   `a_giant_archetype_makes_a_giant_world`,
   `a_hot_jupiter_sits_at_its_period`).
3. **No archetype at all.** A lone world with no `planet` is drawn
   from its formation context, is the same for the same seed, and
   never reads a list of names (Task 4 test
   `an_unnamed_world_is_drawn_from_its_context`).
4. **Stored state from before.** Old job rows (requests with
   `sea_level`…) and old form memory in the browser (`planet:
   'terran'`, knob fields) must not break the server or the form: the
   index moves to schema 5 like 3a's move to 4, and form memory drops
   unknown fields and unknown archetypes (Task 7 test
   `an_index_of_schema_4_is_set_aside`; Task 8 test
   `old_form_memory_falls_back_to_earth_like`).
5. **Archetypes in a crowded system.** `--system earth-like=3` or
   two archetypes with the same insolation must still give a valid
   system (spacing respected, each slot keeps its archetype's inputs)
   or a clear error, as `place_whole` does today for crowded bands
   (Task 5 test `three_earths_share_the_habitable_zone`).

---

### Task 1: `ice-world` only for planets that keep their ice (#452)

**Files:**
- Modify: `crates/mapgen/src/planet/classify.rs:151-180`
- Modify: `crates/mapgen/src/planet/interim.rs:57-90` (`base_of`)

**Interfaces:**
- Produces: `classify` unchanged in signature. With a surface,
  `IceWorld` needs `surface.water.oceans >= ICE_WORLD_MIN_OCEANS`
  (new const, 0.01, the threshold `base_of` already uses at :86 — move
  it here and have `base_of` use it). Without a surface (enveloped),
  the bulk rule stays. A dry planet falls through to `Rocky` or
  `SuperEarth` by radius.

- [ ] **Step 1: Write the failing tests** in `classify.rs`:

```rust
#[test]
fn a_planet_that_lost_its_water_is_not_an_ice_world() {
    // mgs-65793-1225983567 b (#452): 0.22 M⊕, 37 % water, 0.75 AU, G2V.
    let model = model_at(0.22, 0.37, 0.75); // helper: G2V, 4.6 Gyr, no envelope
    let s = model.surface.as_ref().unwrap();
    assert!(s.water.oceans < 0.01, "{s:?}");
    assert_ne!(model.class, PlanetClass::IceWorld);
}

#[test]
fn a_cold_icy_planet_stays_an_ice_world() {
    let model = model_at(0.5, 0.3, 4.0);
    assert_eq!(model.class, PlanetClass::IceWorld);
}
```

`model_at(mass, water_fraction, orbit_au)` builds a `Context` with
`Context::of(&StarInfo::new(42.0, orbit_au, 0.0, 4.6), 0.0, ice_line)`
(take the ice line the way `system::assemble` does) and calls
`planet_model` with inputs from `reference("Earth")` but this mass and
water, volatiles from the rule at factor 1. If the first test does not
lose its water with these inputs, use the exact inputs of that body
(regenerate the system with `--system` and its seed in the scratchpad)
rather than loosening the assertion.

- [ ] **Step 2: Run** `cargo test -p mapgen planet::classify` — expect
  the first test to FAIL (`IceWorld`).
- [ ] **Step 3: Implement** the surface check in `classify`.
- [ ] **Step 4: Run** the planet tests and `cargo run --release -p mapgen --example planet_census` (the census of 3a); expect PASS and
  fewer `ice-world` among hot bodies. Note the class mix before/after
  in the commit body.
- [ ] **Step 5: Commit** `fix(planet): name ice worlds by the ice they keep` with footer `Closes: #452`.

### Task 2: Input overrides

**Files:**
- Modify: `crates/mapgen/src/planet/inputs.rs` (`draw_inputs`, `volatiles`)
- Modify: `crates/mapgen/src/planet/mod.rs` (re-exports; `derive_model`)

**Interfaces:**
- Produces:

```rust
/// Inputs set by an archetype or a request instead of drawn. `None`
/// keeps the draw.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct InputOverrides {
    pub core_mass_fraction: Option<f64>,
    /// Water and ices, Earth oceans (`OCEAN_KG` each).
    pub water_oceans: Option<f64>,
    pub envelope_fraction: Option<f64>,
    /// Factor on the CO₂ formation rule (1 typical).
    pub co2: Option<f64>,
    /// Factor on the N₂ formation rule (1 typical).
    pub n2: Option<f64>,
    pub radiogenic: Option<f64>,
    /// `Some(true)`: life arises wherever the world allows it;
    /// `Some(false)`: never.
    pub life: Option<bool>,
}

pub fn draw_inputs_with(context: &Context, mass_earth: f64,
    overrides: &InputOverrides, rng: &mut impl RngExt) -> PlanetInputs;
pub fn derive_model_with(system_seed: u32, id: &str, context: &Context,
    mass_earth: f64, overrides: &InputOverrides) -> PlanetModel;
```

`draw_inputs(c, m, rng)` becomes `draw_inputs_with(c, m,
&InputOverrides::default(), rng)`; `derive_model` likewise. `volatiles`
is split so the five log-normal factors are drawn first and the rule
applied after the water and envelope are final (an overridden water
share changes the ice-borne volatiles). Water: `water_fraction =
oceans × OCEAN_KG / (mass_earth × M_EARTH_KG)`. Life: `life_draw = 0.0`
for `Some(true)`, `1.0` for `Some(false)`.

- [ ] **Step 1: Write the failing tests** in `inputs.rs`:

```rust
#[test]
fn no_override_draws_as_before() {
    for seed in 0..50 {
        let c = earth_context();
        let a = draw_inputs(&c, 1.0, &mut rng(seed));
        let b = draw_inputs_with(&c, 1.0, &InputOverrides::default(), &mut rng(seed));
        assert_eq!(a, b);
    }
}

#[test]
fn one_ocean_on_earth_is_earths_water() {
    let o = InputOverrides { water_oceans: Some(1.0), ..Default::default() };
    let i = draw_inputs_with(&earth_context(), 1.0, &o, &mut rng(1));
    assert!((i.water_fraction - 2.34e-4).abs() < 1e-5, "{}", i.water_fraction);
}

#[test]
fn overrides_do_not_shift_the_other_draws() {
    let o = InputOverrides { co2: Some(3.0), ..Default::default() };
    let a = draw_inputs(&earth_context(), 1.0, &mut rng(7));
    let b = draw_inputs_with(&earth_context(), 1.0, &o, &mut rng(7));
    assert_eq!((a.radiogenic, a.life_draw, a.volatiles.n2), (b.radiogenic, b.life_draw, b.volatiles.n2));
    assert!(b.volatiles.co2 > a.volatiles.co2 || a.volatiles.co2 == 0.0);
}

#[test]
fn life_forced_or_forbidden() { /* Some(true) → 0.0, Some(false) → 1.0 */ }
```

- [ ] **Step 2: Run** `cargo test -p mapgen planet::inputs` — FAIL
  (no `draw_inputs_with`).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the full mapgen tests; drawn systems must stay
  byte-identical (`no_override_draws_as_before` plus the existing
  system snapshot tests).
- [ ] **Step 5: Commit** `feat(planet): let inputs be set instead of drawn`.

### Task 3: Archetypes

**Files:**
- Create: `crates/mapgen/src/archetype.rs`
- Create: `archetypes/{mercury,venus,earth,mars,jupiter,saturn,uranus,neptune}-like.toml`,
  `archetypes/{super-earth,mini-neptune,ocean-world,lava-world,hot-jupiter}.toml`
- Modify: `crates/mapgen/src/lib.rs` (module)

**Interfaces:**
- Consumes: `InputOverrides` (Task 2), `preset::ParamDefault` (move it
  to `archetype.rs` if `preset.rs` disappears in Task 6), `PresetLocale`
  (rename `Locale`, same move).
- Produces:

```rust
pub struct Archetype {
    pub name: String,
    pub description: String,
    pub inputs: ArchetypeInputs,
    pub orbit: ArchetypeOrbit,
    pub locales: BTreeMap<String, Locale>, // skip_serializing as today
}
pub struct ArchetypeInputs {          // TOML table [inputs]
    pub mass_earth: ParamDefault,     // required
    pub core_mass_fraction: Option<ParamDefault>,
    pub water_oceans: Option<ParamDefault>,
    pub envelope_fraction: Option<ParamDefault>,
    pub co2: Option<ParamDefault>,
    pub n2: Option<ParamDefault>,
    pub radiogenic: Option<ParamDefault>,
    pub life: Option<bool>,
}
pub struct ArchetypeOrbit {           // TOML table [orbit]
    pub star: Option<String>,         // spectral type, used alone only
    pub insolation: Option<ParamDefault>,  // Earth's = 1
    pub period_days: Option<ParamDefault>, // instead of insolation (hot Jupiter)
    pub eccentricity: Option<ParamDefault>,
    pub axial_tilt: Option<ParamDefault>,
}
impl Archetype {
    pub fn from_toml(text: &str, origin: &str) -> Result<Self, ArchetypeError>;
    pub fn load(spec: &str) -> Result<Self, ArchetypeError>; // built-in name or *.toml path
    pub fn built_in() -> Vec<Archetype>;
    /// The overrides this archetype sets, ranges drawn from `rng`.
    pub fn overrides(&self, rng: &mut impl RngExt) -> InputOverrides;
    pub fn mass(&self, rng: &mut impl RngExt) -> f64;
}
pub fn built_in_names() -> Vec<&'static str>;
```

Validation in `from_toml` (`deny_unknown_fields`): exactly one of
`insolation` and `period_days` or neither; every range within the
request limits of Task 4; a name of lowercase letters and hyphens.

Values (from `planet::reference` and the sources in its module docs;
ranges ±10 % on mass and insolation for the Solar System bodies; the
star `G2V` for them; tilts as seasons, Venus 2.6°, Uranus 82.2°):

| Archetype | mass M⊕ | cmf | water oceans | envelope | insolation | e | tilt | class expected |
|---|---|---|---|---|---|---|---|---|
| mercury-like | 0.05-0.06 | 0.70 | 0 | 0 | 6.0-7.3 | 0.206 | 0.03 | rocky |
| venus-like | 0.73-0.90 | 0.31 | from the draw | 0 | 1.7-2.1 | 0.0068 | 2.6 | runaway-greenhouse |
| earth-like | 0.9-1.1 | 0.325 | 0.8-1.2 | 0 | 0.9-1.1, life true | 0.017 | 23.44 | rocky (oceans, life) |
| mars-like | 0.096-0.118 | 0.24 | 0.01-0.05 | 0 | 0.39-0.47 | 0.093 | 25.19 | rocky |
| jupiter-like | 286-350 | — | — | 0.93-0.96 | 0.033-0.041 | 0.049 | 3.1 | gas-giant |
| saturn-like | 86-105 | — | — | 0.83-0.87 | 0.010-0.012 | 0.057 | 26.7 | gas-giant |
| uranus-like | 13-16 | — | — | 0.12-0.18 | 0.0024-0.003 | 0.046 | 82.2 | ice-giant |
| neptune-like | 15.5-19 | — | — | 0.12-0.18 | 0.001-0.0012 | 0.009 | 28.3 | ice-giant |
| super-earth | 2.5-6 | — | 0-2 | 0 | 0.5-1.5 | — | — | super-earth |
| mini-neptune | 5-12 | — | — | 0.01-0.05 | 5-50 | — | — | mini-neptune |
| ocean-world | 1-3 | — | 300-2000 | 0 | 0.8-1.1 | — | — | ocean-world |
| lava-world | 1-5 | — | 0 | 0 | 1000-3000 | — | — | lava-world |
| hot-jupiter | 100-1000 | — | — | — | period 1-5 d | — | — | hot-jupiter |

"—" leaves the input to the formation context. Each file has a
`[locales.fr]` table (`name`, `description`), like today's presets.

- [ ] **Step 1: Write the failing tests** in `archetype.rs`:

```rust
#[test]
fn every_archetype_comes_out_as_its_class() {
    // At its own insolation (or period) round a G2V of 4.6 Gyr,
    // metallicity 0, over 20 seeds: the class of the table above.
    for a in Archetype::built_in() {
        for seed in 0..20 {
            let model = model_of(&a, seed); // helper building the Context
            assert_eq!(model.class.key(), expected(&a.name), "{} seed {seed}: {model:?}", a.name);
        }
    }
}

#[test]
fn earth_like_keeps_its_ocean_and_its_life() { /* oceans 0.5..2, life.arose, 0.7..1.3 bar */ }

#[test]
fn venus_like_has_venus_air() { /* 60..120 bar, CO₂ > 90 % */ }

#[test]
fn a_file_with_both_insolation_and_period_is_refused() {}

#[test]
fn built_in_names_load() {
    for n in built_in_names() { Archetype::load(n).unwrap(); }
}
```

When a class does not come out, fix the archetype's inputs within the
real body's uncertainty; if the model cannot produce the body at all
(e.g. Venus's water), write it in the test as a known departure with
an issue number, never widen the expectation silently.

- [ ] **Step 2: Run** `cargo test -p mapgen archetype` — FAIL.
- [ ] **Step 3: Implement** the module and the 13 files (embedded with
  `include_str!` like `preset::BUILT_IN`).
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** `feat(planet): add archetypes, named sets of model inputs`.

### Task 4: A world request names an archetype

**Files:**
- Modify: `crates/mapgen/src/request.rs` (`WorldRequest`, `validate`,
  `resolve`, `resolve_with_preset*`, `pinned`, limits, `RequestError`)
- Modify: `crates/mapgen/src/planet/interim.rs` (`params_for`)
- Modify: `crates/mapgen/src/system.rs` (`draw_around`, `assemble`
  :1149-1221: the focus gets a model)
- Modify: `crates/cli/src/main.rs` (flags, `list_planets`, printout,
  "Reproduce with")
- Modify: `crates/server/src/{app.rs,export.rs}` only as far as needed
  to compile (Task 7 finishes the server)
- Modify: `README.md` CLI table and the paragraphs on the removed knobs

**Interfaces:**
- Consumes: `Archetype` (Task 3), `InputOverrides`,
  `derive_model_with` (Task 2).
- Produces:
  - `WorldRequest` loses `sea_level`, `land_fraction`, `volcanic`,
    `circumference_km`, `erosion`, `pressure_bar`, `surface_age_ga`.
    `planet: Option<String>` now names an archetype (or a `.toml`
    path). `mass_earth` applies to any planet (limits `[0.01, 4131]`).
    New: `core_mass_fraction` `[0.0, 0.9]`, `water_oceans`
    `[0, 1e6]`, `envelope_fraction` `[0, 0.99]`, `co2` and `n2`
    `[0, 100]`, `radiogenic` `[0.1, 10]`, `life: Option<bool>`.
    `axial_tilt_deg` stays.
  - `WorldRequest::overrides(&self, archetype: Option<&Archetype>,
    rng) -> InputOverrides`: the request's value, else the
    archetype's, else `None`.
  - `WorldRequest::resolve(&self, rng) -> Result<Resolved, RequestError>`
    and `resolve_with_archetype(&self, archetype: Option<Archetype>,
    rng)` replace `resolve_with_preset*`. Order: seed; star (request,
    else archetype `orbit.star`, else drawn); age; orbit (request
    `orbit_au` / `hz_position`, else archetype insolation `a =
    sqrt(L/S)`, else archetype period by Kepler, else the drawn HZ
    position as today); metallicity; mass (request, else archetype,
    else `draw_mass(Origin::Small)` on the main stream, or the giant
    mass when `host` makes it a giant's moon as today); eccentricity
    drawn; model by `derive_model_with` under
    `body_seed(seed, "world/model")`; if the star circularises that
    orbit for the model's mass and radius, eccentricity 0 and the
    model is rebuilt; then `interim::params_for`.
  - `Resolved { params, model: PlanetModel, archetype: Option<String>, ignored }`
    (`WorldParams` gains `model` and `archetype` only in Task 6; until
    then the tests read them from `Resolved`).
  - `RequestError::UnknownArchetype(String)` replaces the preset
    error; `RequestError::WaterHeavierThanPlanet { water_oceans,
    mass_earth }`.
  - `interim::params_for(model: &PlanetModel, placed: Placed, rng) ->
    WorldParams` sets circumference, pressure, erosion, volcanic and
    surface age from the model directly (no request knobs); `Placed`
    is the struct the request resolution returns for star, orbit, age,
    metallicity, tilt, seed, size, moons, giants, host, drift.
  - `pinned(&self, params) -> WorldRequest` pins the archetype name,
    the model's inputs as overrides (mass, cmf, water in oceans,
    envelope, CO₂ and N₂ factors, radiogenic, life) and the orbit, so
    "Reproduce with" gives the same world.
  - CLI: `--planet <archetype|file.toml>`, `--list-planets` lists
    archetypes; knob flags removed; new `--mass`, `--core-fraction`,
    `--water <oceans>`, `--envelope`, `--co2`, `--n2`,
    `--radiogenic`, `--life <yes|no>`. The printout shows the class,
    the archetype and the regime from the model.

`draw_around` stops typing the focus by `params.planet.name`: the
focus slot is `Placement::Focus` with the resolved model; `assemble`
keeps it instead of deriving one (`model = None` for the focus goes
away, 3a's gap).

- [ ] **Step 1: Write the failing tests** in `request.rs`:

```rust
#[test] fn earth_like_resolves_through_the_model() {
    // planet: earth-like, seed 3 → resolved.model.class is Rocky,
    // circumference within 10 % of 40 075 km, pressure 0.5..2 bar.
}
#[test] fn an_override_wins_over_the_archetype() {
    // earth-like + water_oceans 0 → no sea (params.sea_level at its minimum).
}
#[test] fn water_heavier_than_the_planet_is_refused() {
    // mass 0.1, water_oceans 5000 → Err(WaterHeavierThanPlanet{..}).
}
#[test] fn a_giant_archetype_makes_a_giant_world() {
    // jupiter-like → params.giant_mass_earth in 286..350.
}
#[test] fn a_hot_jupiter_sits_at_its_period() {
    // hot-jupiter → orbit period 1..5 d from orbit_au and the star's mass.
}
#[test] fn an_unnamed_world_is_drawn_from_its_context() {
    // planet None, seed 9, twice → identical params (serde_json equal);
    // resolved.archetype is None.
}
#[test] fn an_unknown_archetype_is_named() {
    // planet "terran" → Err(UnknownArchetype("terran")).
}
#[test] fn the_pinned_request_reproduces_the_world() {
    // resolve, pin, resolve the pinned request → same params.
}
#[test] fn knob_fields_are_refused() {
    // serde_json::from_str::<WorldRequest>(r#"{"sea_level":0.1}"#) is Err.
}
```

In `system.rs`: `the_planet_asked_for_has_a_model` (an `Around`
request with `earth-like`: the focus body has `model: Some` and is
typed by its class).

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** in `request.rs`, `interim.rs`,
  `system.rs`; keep `WorldParams.planet: Preset` for now
  (`interim::preset_for` still builds it). Fix the server and CLI call
  sites to compile; update the CLI help and the README CLI table.
- [ ] **Step 4: Run** the workspace tests and clippy; then
  `cargo run --release -p cli -- --planet earth-like --seed 1` in the
  scratchpad (`--out` there) and check the printout.
- [ ] **Step 5: Commit** `feat(request)!: name archetypes and set inputs instead of knobs` with a `BREAKING CHANGE:` footer listing the removed fields and flags.

### Task 5: Systems requested by archetype

**Files:**
- Modify: `crates/mapgen/src/system.rs` (`PLANET_TYPES` :77, `band`
  :423, `Placement` :443, `Slot::max_au` :512, `requested_mass` :557,
  `place_whole` :812-881, `slot_mass` :1013, `assemble`)
- Modify: `crates/cli/src/main.rs` (`--system` help, `parse_planets`)
- Modify: `README.md` (`--system` paragraph)

**Interfaces:**
- Consumes: `Archetype`, `derive_model_with`.
- Produces: `WholeSystemRequest.planets` keys are archetype names;
  unknown → `RequestError::UnknownArchetype`. `Placement::Requested`
  holds the loaded `Archetype`. Its slot sits at the archetype's
  insolation (drawn in its range; `a = sqrt(L/S)`) or period, its
  mass is `archetype.mass(rng)` on the system stream where
  `requested_mass` drew it, and its model is `derive_model_with(...,
  &archetype.overrides(rng_model))` on the model stream. The allowed
  band of a requested slot is the orbits of its insolation range,
  widened by the spacing `place_whole` already applies.
  `band`, `requested_mass`, `PLANET_TYPES` are deleted.
  `pub fn archetype_names() -> Vec<String>` (built-in names) replaces
  `PLANET_TYPES` for the server's form.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn a_requested_archetype_sits_at_its_insolation() {
    // star G2V, planets {venus-like: 1} → that body's insolation 1.7..2.1.
}
#[test] fn three_earths_share_the_habitable_zone() {
    // planets {earth-like: 3} → Ok, three bodies with model.inputs.mass 0.9..1.1,
    // spaced as place_whole requires; or Err naming the crowding — not a panic.
}
#[test] fn requested_archetypes_keep_their_inputs() {
    // earth-like's body: core_mass_fraction 0.325, life_draw 0.0.
}
#[test] fn an_old_type_name_is_refused() {
    // planets {terran: 1} → Err(UnknownArchetype("terran")).
}
```

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the workspace tests; draw `--system
  venus-like=1,earth-like=1,mars-like=1 --draw-only` in the
  scratchpad and check the three classes in `system.json`.
- [ ] **Step 5: Commit** `feat(system)!: request planets by archetype`.

### Task 6: `WorldParams` without `Preset`: the look

**Files:**
- Rename: `crates/mapgen/src/preset.rs` → `crates/mapgen/src/look.rs`
- Move: `presets/*.toml` → `looks/*.toml`, `presets/moons/` →
  `looks/moons/`; delete `presets/examples/` (replaced by archetypes)
- Modify: `crates/mapgen/src/world.rs:15-80` (`WorldParams`),
  `export/raw_data.rs` (`WorldMetadata` :71, :591, `load_layers`
  :1007, format constants), `system.rs:62`, `moons.rs:1203-1287`,
  `exomoon.rs:189,258-284`, `world.rs:210`, `hex.rs:224`,
  `export/png.rs:110`, `planet/interim.rs`, every `params.planet.*`
  reader listed in the recon (generation, detail, epoch, carve,
  craters, drift, geology, giant_world, export/*)
- Modify: `web/src/lib/formatVersion.ts`, the web readers of
  `meta.preset` (`sky.ts:36,83`, `sea.ts:10`, `sample.ts:31`,
  `epochs.ts:28`, `drift.ts:22`, `format.ts:242,253`,
  `i18n/names.svelte.ts:36-42`, `WorldInfo.svelte:49,97,105`) and
  their tests and fixtures (`systemSky.fixture.json`)
- Modify: `README.md` "Planet presets" section → "Archetypes and
  looks", layout table, the `world.json` description (:387)

**Interfaces:**
- Produces:

```rust
/// How a world is drawn until #424: biome rules and remap, climate
/// shift, sky, geology, giant look. Chosen by `planet::interim` (or by
/// the moon draw), never named by a request.
pub struct Look { pub rules, pub climate, pub atmosphere, pub geology,
                  pub giant: Option<GiantSpec>, pub remap, lookup }
impl Look {
    pub fn load(name: &str) -> Look;        // built-in look, panics if unknown
    pub fn moon(composition: Composition) -> Look;
    pub fn remap(&self, b: Biome) -> Biome;
    pub fn then_remap(self, extra: &[(Biome, &[Biome])]) -> Self;
}
pub struct WorldParams {
    // `planet: Preset` becomes:
    /// The class key (`PlanetClass::key`), or `rocky-moon` / `icy-moon`.
    pub class: String,
    /// The archetype asked for, for display only.
    pub archetype: Option<String>,
    /// The planet's model; `None` for a moon (part 4).
    pub model: Option<PlanetModel>,
    pub look: Look,
    // other fields unchanged
}
```

`Look` drops `name`, `description`, `defaults`, `star`, `locales` and
`derived` (the regime now always comes from the model or the moon
look). Moon surface ages that came from `defaults.surface_age` become
constants in `moons.rs` with the values of `looks/moons/*.toml`
(`ICY_MOON_SURFACE_AGE_GA` 4.0, rocky likewise). `is_moon_world` and
`gravity_modifier` read `params.class`. `WorldMetadata` stores
`class`, `archetype`, `model` and `look` instead of `planet_type` and
`preset`; `HexSource.planet_type` becomes `class`. Formats bump here:
`SYSTEM_FORMAT_VERSION` 4, `FORMAT_VERSION` and `MIN_FORMAT_VERSION`
18, `formatVersion.ts` 18. The web readers move from `meta.preset.X`
to `meta.look.X`; the title shows the archetype's localized name when
there is one, else the class label (`bodyTypes`). The `terran`,
`barren`, `caustic`, `frozen`, `volcanic` keys leave `bodyTypes` in
`en.ts` and `fr.ts`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn world_json_carries_the_model_and_the_look() {
    // generate earth-like 128×64 into a temp dir; world.json has "class",
    // "archetype":"earth-like", "model", "look", and no "preset".
}
#[test] fn a_world_reloads_from_its_metadata() {
    // load_layers rebuilds WorldParams equal to the generated ones.
}
#[test] fn a_moon_has_a_moon_look_and_no_model() {}
```

Web (`sea.test.ts`, `sky.test.ts`, `drift.test.ts`, `epochs.test.ts`,
`format.test.ts`, `fr.test.ts`): change the fixtures to `look` and
keep the assertions; add `planetClass.test.ts`: no old type key left
in `bodyTypes`.

- [ ] **Step 2: Run** — FAIL (no `Look`).
- [ ] **Step 3: Implement** (mechanical rename, then the readers).
- [ ] **Step 4: Run** the workspace tests, clippy, then `cd web && npx vitest run` for the files above.
- [ ] **Step 5: Commit** `refactor(world)!: carry the model and a look instead of a preset` with `BREAKING CHANGE: world.json format 18, system format 4`.

### Task 7: Server: archetype catalog and form spec

**Files:**
- Modify: `crates/server/src/catalog.rs` (archetypes instead of
  presets), `api.rs:14,44-80` (`ArchetypeInfo`, `FormSpec`),
  `app.rs:470-499,581-623,735` (routes, `create_system`), `export.rs:127-149`,
  `systems.rs:577-630` (`BodyMeta` reads `look.geology.wet_until`),
  `index.rs:74` (schema 5), `main.rs:9,102` (env)
- Modify: `Dockerfile:3,12-13,52`, `compose.yaml:26,52`, `mise.toml:70`
- Modify: `README.md` server environment table and API section;
  `docs/system-spec.md` where it names presets (find with `grep -n preset`)

**Interfaces:**
- Produces: `GET /api/archetypes` → `Vec<ArchetypeInfo { spec, name,
  description, built_in, inputs: ArchetypeInputs, orbit:
  ArchetypeOrbit, locales }>`; `/api/presets` is removed.
  `FormSpec` loses the seven knob `ParamSpec`s and `land_fraction`,
  `planet_types` becomes `archetypes: Vec<String>`, and gains the
  `Limits` of the Task 4 overrides (for step 5). `create_system` with
  `Around` resolves `planet` from the catalog; an unknown one gives
  `ErrorDetail::UnknownArchetype { archetype }`. Env
  `MAPGEN_ARCHETYPES_DIR` (default `archetypes`) replaces
  `MAPGEN_PRESETS_DIR`; a missing directory means built-ins only (the
  data checkout has no `archetypes/` yet). The compose mount becomes
  `./archetypes:/data/archetypes:ro` with the directory created if
  absent (or the mount dropped if compose cannot create it read-only:
  check and say which in the commit).

- [ ] **Step 1: Write the failing tests:** `archetypes_are_listed`
  (13 built-ins, `earth-like` with its fr locale);
  `an_unknown_archetype_is_named` (POST around `terran` → 400
  `unknown_archetype`); `the_form_offers_archetypes`;
  `an_index_of_schema_4_is_set_aside` (as 3a's schema test did for 3);
  `a_missing_archetypes_dir_is_fine`.
- [ ] **Step 2: Run** `cargo test -p server` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the workspace tests and clippy.
- [ ] **Step 5: Commit** `feat(server)!: serve archetypes instead of presets`.

### Task 8: Web: archetype choice, no knobs

**Files:**
- Modify: `web/src/lib/api/client.ts:60`, `App.svelte:211-212,694`,
  `components/GenerateForm.svelte` (FIELDS :86-148, `initialState`
  :156-198, preset lookup :206-276, counts :80-82,362,687-691,
  `buildRequest` :414-455, select :699-712, errors :530-535),
  `lib/starForm.ts:58-101`, `lib/formMemory.ts`, `lib/format.ts:119-211`
  (`requestFromWorld`, `cliCommand`), `lib/errors.ts:19-20,40-42`,
  `lib/i18n/names.svelte.ts`, `BodyCard.svelte:134,163`,
  `SystemCard.svelte:65`, `menuCards.ts:93-98` (comment),
  `i18n/en.ts`, `i18n/fr.ts` (form strings of removed fields, error
  `unknown_archetype`)
- Test: `format.test.ts`, `starForm.test.ts`, `formMemory.test.ts`
  (create if missing)

**Interfaces:**
- Consumes: `/api/archetypes`, `ArchetypeInfo`, `FormSpec.archetypes`
  (Task 7), the new `WorldRequest` (Task 4).
- Produces: the form's planet select lists archetypes plus "drawn from
  the star" (no `planet`); only `axial_tilt_deg` remains of `FIELDS`;
  the sea-level / land controls go. System planet counts are keyed by
  archetype with `earth-like` = 1 by default. `initialStarChoice`
  takes an `ArchetypeInfo` and reads `orbit.star` and
  `orbit.insolation` (converted to an orbit with the star's
  luminosity, as the server does). `requestFromWorld(meta)` sends the
  archetype and pins `meta.model.inputs` as overrides (mass, cmf,
  water in oceans, envelope, life) plus the orbit, so reroll and drift
  keep the planet; `cliCommand` prints the new flags. `presetName` /
  `presetDescription` become `archetypeName` / `archetypeDescription`
  plus `className(key)` for class labels (BodyCard, SystemCard,
  errors, counts).

- [ ] **Step 1: Write the failing tests:**
  `requestFromWorld_pins_the_model_inputs` (no `sea_level`, has
  `water_oceans` from `meta.model`);
  `old_form_memory_falls_back_to_earth_like` (stored `{planet:
  'terran', fields: {sea_level: …}}` loads as `earth-like`, no knob);
  `the_archetype_chooses_the_star` (`initialStarChoice` with
  venus-like → `G2V`).
- [ ] **Step 2: Run** `cd web && npx vitest run` — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `npm run check && npx vitest run` — green; then
  build the worktree and ask `viewer-visual-check` for the form, a
  generated earth-like and a venus-like world page (title, sky,
  atmosphere line, sea).
- [ ] **Step 5: Commit** `feat(web)!: choose an archetype instead of a preset and knobs`.

### Task 9: Whole-branch checks

- [ ] **Step 1:** `world-coherence-checker`: drawn systems of seeds
  101-120 on main and on the branch (the drawn systems should keep the
  same classes except #452's ice worlds) plus `--system
  mercury-like=1,venus-like=1,earth-like=1,mars-like=1,jupiter-like=1,saturn-like=1,uranus-like=1,neptune-like=1`
  on a G2V: report each class, pressure, temperature, oceans.
- [ ] **Step 2:** `branch-reviewer` with this plan and the spec.
- [ ] **Step 3:** fixes on the branch, tests again; then
  `pr-shepherd` opens the PR (`Closes #452`, `Refs #423`), and the
  main session rebuilds `develop` for the user's test.
