# Planet model part 5a (the card) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split a body's details into themed, detachable cards in a
two-column grid, with the legend as one of them, a host button on moons,
cause trees under derived values, and "Similar to X".

**Architecture:** The generator gains `planet::explain` (cause trees
rebuilt from a stored model and its formation context) and
`archetype::similar`; the server adds both per body to `GET
/api/systems/{id}`. The web dock learns panels made of cards, each card
floatable like a panel; `BodyCard.svelte` and `WorldInfo.svelte` become
themed card components; the Viewer's free legend becomes a card.

**Tech Stack:** Rust (mapgen, mapgen-server, serde, ts-rs), Svelte 5 +
TypeScript (vitest, svelte-check).

**Spec:** `docs/superpowers/specs/2026-11-14-planet-model-part5-design.md`
(part 5a = everything under "Card layout", "Cause chain", "Similar to X";
the form is 5b).

## Global Constraints

- No value shown in two cards of one body (the cards' own lines; cause
  trees may repeat values).
- Cause trees: two or three levels below the value; leaves that are form
  inputs carry their field (`InputField`).
- No prose in Rust: codes and numbers only; every string lands in both
  `web/src/lib/i18n/en.ts` and `fr.ts`.
- A tree whose numbers do not reproduce the stored model is not sent
  (`explain` returns `None`), never a tree that disagrees.
- No format bump: `FORMAT_VERSION`, system format and index schema stay;
  drawn systems stay byte-identical.
- Details panel about 680 px wide; two columns when the panel's content
  is at least 560 px wide, else one. Detaching obeys
  `FLOAT_MIN_WIDTH` (720 px view width, `web/src/lib/dock.ts:87`).
- Worktree `~/Developer/mg-planet-model-p5`, branch
  `feat/planet-model-part5`. Before any web command:
  `ln -s ~/Developer/map-generator/web/node_modules web/node_modules`
  (never `npm ci` in a worktree).
- Rust checks: `cargo fmt --all`,
  `cargo clippy --workspace --all-features --all-targets -- -D warnings`,
  `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`
  (regenerates `web/src/lib/api/types/*.ts`: commit them). Web checks:
  `cd web && npm run check && npm test`. The server test
  `index::tests::two_processes_opening_an_old_file_migrate_it_once` is
  flaky: rerun it alone before believing a failure.
- Commits: Conventional Commits, English, no emoji, with the session's
  `Co-Authored-By` and `Claude-Session` trailers.

## Review Focus

1. **A moon's context.** A round moon's tree must use its host's tide and
   the planet's light, not the star's tide at the moon's own orbit (the
   orbit of a moon is in km around its planet). Test in Task 1 on a
   Titan and an Io.
2. **A body without a model or without a surface** (small moon, giant,
   mini-Neptune with an envelope): no tree for surface values, no crash,
   cards still render. Tests in Tasks 1, 5.
3. **An old system.json fetched through the static fallback**
   (`/systems/{id}/system.json`, `world.svelte.ts:520-532`) carries no
   extras: cards render without trees or "Similar to". Test in Task 3.
4. **Stored layouts from before** (`dock.planet` in localStorage with
   only panel ids, or a floating card id no longer offered): loads
   without error, unknown ids dropped. Test in Task 4.
5. **Narrow view** (< 720 px): no detach buttons, every card in the
   panel, one column; a card stored floating shows in the grid. Test in
   Task 4.

---

### Task 1: Cause trees in the generator (opus, hard: physics)

**Files:**
- Create: `crates/mapgen/src/planet/explain.rs`
- Modify: `crates/mapgen/src/planet/mod.rs` (module, re-exports)
- Modify: `crates/mapgen/src/system.rs` (`body_context`)
- Test: in `explain.rs` and `system.rs` `#[cfg(test)]`

**Interfaces:**
- Produces, all `#[derive(Clone, Debug, PartialEq, Serialize)]`, ts-rs
  exported under the `ts` feature like `PlanetModel`:
  - `pub struct Explanation { pub trees: Vec<Cause> }` (one root per
    explained value, in the order below).
  - `pub struct Cause { pub quantity: Quantity, pub value: f64, pub label: Option<String>, pub input: Option<InputField>, pub causes: Vec<Cause> }`.
    `label` holds an enum key for non-numeric values (`"plates"`,
    `"water"`, a `PlanetClass::key()`), `value` is then 0.
  - `#[serde(rename_all = "snake_case")] pub enum Quantity` with at
    least: `SurfaceTemperature`, `EquilibriumTemperature`,
    `LightReceived`, `Greenhouse`, `InternalWarming`, `HeatFlux`,
    `RadiogenicHeat`, `TidalHeat`, `Pressure`, `Co2`, `N2`, `H2o`,
    `Ch4`, `O2`, `Escape`, `Liquid`, `Regime`, `MagneticField`,
    `CoreState`, `Class`, `Mass`, `EnvelopeFraction`, `WaterKept`,
    `Age`. Add codes as the trees need; each code gets a web label in
    Task 6.
  - `#[serde(rename_all = "snake_case")] pub enum InputField { MassEarth, CoreMassFraction, WaterOceans, EnvelopeFraction, Co2, N2, Ch4, Radiogenic, Life, Insolation, Eccentricity, AgeGyr }`
    (the request's field names, `request.rs:133`, plus the orbit).
  - `pub fn explain(context: &Context, model: &PlanetModel) -> Option<Explanation>`.
  - In `system.rs`: `pub fn body_context(system: &System, body: &Body) -> Option<Context>`:
    a planet's as in the test at `system.rs:4049`
    (`StarInfo::new(p.star, p.orbit_au, p.eccentricity, p.age_gyr)`,
    `Context::of(&star, p.metallicity, system.star.ice_line_au)`); a
    round moon's from its planet's (`system.rs:1631`) with the host
    tide its model was derived with (`planet/moon.rs:174`,
    `system.rs:2036-2050`); the world drawn around a request as
    `request.rs:862-866` does (with `host_tide`, `request.rs:884`, when
    it orbits a giant). `None` for a body without a model.

Trees (roots, in this order; a root is omitted when its value does not
exist, e.g. no surface):

- `Class` ← mass, envelope kept, water kept, surface temperature,
  liquid: the inputs `classify` (`planet/classify.rs:196`) reads.
- `SurfaceTemperature` ← `EquilibriumTemperature` (← `LightReceived`
  [input `Insolation`]), `Greenhouse` (← each gas's pressure, gases
  from inputs marked `Co2`/`N2`/`Ch4`, `H2o` marked `WaterOceans`),
  `InternalWarming` (← `HeatFlux` ← `RadiogenicHeat` [`Radiogenic`],
  `TidalHeat` [`Eccentricity`]). Decompose with
  `climate::surface_temperature_k` (`climate.rs:238`): the three terms
  are T with no air and no heat, T with air minus that, T with both
  minus T with air. A regulated climate (`Climate.regulated`) adds a
  `label: "thermostat"` note node under `Co2`.
- `Pressure` ← each gas (bar), each ← its source (outgassing under the
  regime, `Escape` for what was lost; `Co2`/`N2` inputs).
- `Liquid` ← surface temperature, pressure.
- `Regime` ← `Mass` [`MassEarth`], `HeatFlux`, `WaterKept`: what
  `regime()` (`planet/regime.rs:122`) reads.
- `MagneticField` ← `CoreState` (← core fraction [`CoreMassFraction`],
  `Age` [`AgeGyr`]), `Regime`.
- `WaterKept` (oceans) ← `WaterOceans` input, runaway or boiled,
  `Escape`.

Depth: at most three levels below a root.

- [ ] **Step 1: Write the failing tests** in `explain.rs`:

```rust
#[test]
fn earth_temperature_tree_reproduces_the_model() {
    // reference::sun_at(1.0) and the earth-like archetype's inputs
    let (context, model) = earth();
    let e = explain(&context, &model).expect("Earth is explained");
    let t = root(&e, Quantity::SurfaceTemperature);
    assert!((t.value - model.surface.unwrap().climate.temperature_k).abs() < 1e-9);
    let sum: f64 = t.causes.iter().map(|c| c.value).sum();
    assert!((sum - t.value).abs() < 0.5, "{sum} vs {}", t.value);
    assert!(t.causes.iter().any(|c| c.quantity == Quantity::Greenhouse && c.value > 20.0));
}

#[test]
fn trees_are_at_most_three_levels_deep() { /* every root of Earth, Venus, Mars, Titan */ }

#[test]
fn every_input_leaf_names_a_field() {
    // Earth: some node carries InputField::WaterOceans, Insolation, Co2
}

#[test]
fn a_body_without_surface_has_class_but_no_temperature_tree() { /* mini-Neptune */ }
```

  And in `system.rs` tests:

```rust
#[test]
fn body_context_rebuilds_the_drawn_contexts() {
    // drawn_g2v(3) and a system with a Titan (titan-like asked):
    // for every body with a model, explain(&body_context(..), model) is Some,
    // and the rebuilt context reproduces the stored tidal heat:
    // tidal_power_w(...) from body_context equals model.heat.tidal_w (rel 1e-9).
}

#[test]
fn explain_refuses_a_model_its_context_does_not_reproduce() {
    // a model whose climate.temperature_k is off by 10 K: explain(..) == None
}
```

- [ ] **Step 2:** Run `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features explain -- --test-threads=3`; expect compile errors (missing items).
- [ ] **Step 3:** Implement `explain`, the types and `body_context`. `explain` recomputes each decomposed value with the model's own functions and returns `None` when the temperature recomposition is more than 0.5 K off the stored one, or the rebuilt tidal heat differs from the stored by more than 1e-6 relative.
- [ ] **Step 4:** Run the tests; expect PASS. Then the full Rust checks (Global Constraints).
- [ ] **Step 5: Commit** `feat(planet): explain derived values as cause trees` (with the regenerated `web/src/lib/api/types/*.ts`).

### Task 2: "Similar to X" in the generator (sonnet)

**Files:**
- Modify: `crates/mapgen/src/archetype.rs`
- Test: `archetype.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `Context`, `PlanetModel`, `Archetype` (`archetype.rs:438`).
- Produces: `pub fn similar<'a>(context: &Context, model: &PlanetModel, moon_of_giant: bool, archetypes: impl IntoIterator<Item = &'a Archetype>) -> Option<&'a str>` (the archetype's `name`).

Distance in the archetypes' input space, each axis divided by a
tolerance, compared to a threshold (both constants, calibrated by the
tests below, each with a comment saying so): `log10` of mass,
core mass fraction, water mass fraction (an archetype's `water_oceans`
× `OCEAN_KG` / (mass × `EARTH_KG`)), `log10` of insolation (an
archetype with `period_days` instead: derive its insolation from the
context's star, or skip that axis). A `Range` counts as its midpoint, a
missing input as the archetype's default draw (skip the axis). Moon
archetypes (`moon.is_some()`) only for a giant's moon, planet
archetypes only for the others. Giants: compare mass and insolation only.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn solar_system_archetypes_are_recognised() {
    // each built-in archetype, resolved with seeds 1..=20
    // (WorldRequest::resolve_with_archetype), is named itself
    // by similar() in at least 16 of 20 draws
}

#[test]
fn ordinary_drawn_bodies_are_mostly_unnamed() {
    // 50 whole systems (seeds 1..=50, G2V): at most 30 % of the
    // bodies with a model get Some(_)
}
```

- [ ] **Step 2:** Run `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features similar -- --test-threads=3`; expect FAIL (missing function).
- [ ] **Step 3:** Implement; tune the tolerances and threshold until both pass, and record the measured shares in the constants' comments.
- [ ] **Step 4:** Run the tests and the full Rust checks; expect PASS.
- [ ] **Step 5: Commit** `feat(archetype): name the archetype a body resembles`.

### Task 3: Extras on the system route, kept by the web (sonnet)

**Files:**
- Modify: `crates/server/src/api.rs:401-411` (`SystemDetail`)
- Modify: `crates/server/src/app.rs:774-796` (`get_system`), `crates/server/src/catalog.rs` (`entries()`)
- Modify: `web/src/lib/world.svelte.ts:70-104, 184-188, 520-532`
- Test: server tests next to `get_system`'s; `web/src/lib/world.test.ts` (create if absent, or the nearest existing world test)

**Interfaces:**
- Consumes: `planet::explain`, `system::body_context` (Task 1), `archetype::similar` (Task 2).
- Produces:
  - Rust: `pub struct BodyExtras { pub explanation: Option<Explanation>, pub similar: Option<String> }` (ts-rs), and `SystemDetail.extras: BTreeMap<String, BodyExtras>` keyed by body id, only for bodies with a model. `similar` is the body's own `params.archetype` when it was asked by archetype (spec: "a body asked by archetype shows its own"), else `similar(..)` over the built-in entries of the catalog. `Catalog::entries(&self) -> impl Iterator<Item = &(ArchetypeInfo, Archetype)>`.
  - Web: `world.systemExtras = $state.raw<Record<string, BodyExtras>>({})`, filled by `loadSystem`, `{}` when the fallback `system.json` is read; `world.shownExtras = $derived(this.systemExtras[this.systemBody ?? ''] ?? null)`.

- [ ] **Step 1: Write the failing tests:** server: GET a drawn system (with a `titan-like` asked) returns `extras` for each body with a model, with a `SurfaceTemperature` tree for the Earth-like body and `similar == Some("titan-like")` for the Titan. Web: `fetchSystem` keeps `extras`; the fallback path gives `{}`.
- [ ] **Step 2:** Run `RAYON_NUM_THREADS=4 cargo test -p mapgen-server --all-features get_system -- --test-threads=3` and `cd web && npx vitest run src/lib/world`; expect FAIL.
- [ ] **Step 3:** Implement (compute inside the existing `spawn_blocking`; clone what the closure needs from `state.catalog` first).
- [ ] **Step 4:** Run the tests, the Rust checks and the web checks; expect PASS.
- [ ] **Step 5: Commit** `feat(server): send cause trees and resemblances with a system`.

### Task 4: Dock panels made of detachable cards (sonnet)

**Files:**
- Modify: `web/src/components/dock/Dock.svelte`
- Modify: `web/src/lib/dock.ts`
- Test: `web/src/lib/dock.test.ts`, `web/src/components/dock/Dock.test.ts` (create, SSR style of `SystemCard.test.ts`)

**Interfaces:**
- Produces:
  - `export interface DockCard { id: string; title: string; content: Snippet; full?: boolean; detachable?: boolean }` (`full`: spans both columns; `detachable` defaults to true).
  - `DockPanel` gains `cards?: DockCard[]` (with `content` then optional) and `wide?: boolean`.
  - `loadLayout(kind, ids, defaultActive)` is called with panel ids and card ids together; a card id in `floating` is a floating card.
- Behaviour: a panel with `cards` renders them in a grid (`grid-template-columns: repeat(auto-fill, minmax(260px, 1fr))` under a container query, two columns from 560 px, `full` cards span the row); each detachable card has a detach button reusing `detach(id)`, `drag`, `front`, `clampToView` and the float layer; a floating card stays shown whatever panel is active (as floating panels do); `dockBack(id)` returns it to the grid. A `wide` panel's body is `width: clamp(260px, 50vw, 680px)`. Under `FLOAT_MIN_WIDTH`, floating cards render in the grid and no detach button shows.

- [ ] **Step 1: Write the failing tests:**

```ts
it('keeps a floating card id offered by a panel', () => {
  storage({ active: 'info', folded: false, floating: { 'info.orbit': { x: 10, y: 20 } } });
  expect(loadLayout('planet', ['info', 'info.orbit'], 'map').floating).toEqual({ 'info.orbit': { x: 10, y: 20 } });
});
it('drops a floating card no longer offered', () => { /* ids without it → floating {} */ });
it('loads a layout stored before cards existed', () => { /* only panel ids → unchanged */ });
```

  and in `Dock.test.ts`: a panel with three cards renders three cards
  in the grid with detach buttons except the one with
  `detachable: false`; a `full` card carries the spanning class.
- [ ] **Step 2:** Run `cd web && npx vitest run src/lib/dock src/components/dock`; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `npm run check && npm test`; expect PASS.
- [ ] **Step 5: Commit** `feat(web): let a dock panel hold detachable cards`.

### Task 5: Themed body cards, host button, legend card (sonnet)

**Files:**
- Create: `web/src/components/body/` with `SystemSummary.svelte`
  (today's `BodyCard.svelte:113-152`), `BodyHeader.svelte`,
  `OrbitCard.svelte`, `InteriorCard.svelte`, `AtmosphereCard.svelte`,
  `SurfaceCard.svelte`, `MoonsCard.svelte`, `MapCard.svelte`,
  `LegendCard.svelte`; `web/src/lib/bodyCards.ts` (which cards a body
  gets).
- Delete: `web/src/components/BodyCard.svelte`, `WorldInfo.svelte`
  (moving their helpers, renaming, generate, reroll/vary/copy/delete,
  downloads into the new cards).
- Modify: `web/src/App.svelte:522-543` (the `info` snippet becomes card
  definitions), `web/src/components/Viewer.svelte` (`info` prop →
  `infoCards: DockCard[]`; the `info` panel is `wide` with those cards
  plus the legend card; remove `legendOpen`, `legendAt`, `dragLegend`,
  `legendClick`, `legendHome` and the legend markup at `:2741-2798`,
  the legend's settings keys, `wideChoice.legendOpen`).
- Modify: `web/src/lib/i18n/en.ts`, `fr.ts` (card titles, host button).
- Test: `web/src/lib/bodyCards.test.ts`, `web/src/components/body/*.test.ts` (SSR).

**Interfaces:**
- Consumes: `DockCard` (Task 4), `world.shownExtras` (Task 3).
- Produces: `export function cardsFor(body: Body | null, meta: WorldMetadata | null, hasStats: boolean): CardId[]` with
  `type CardId = 'header' | 'orbit' | 'interior' | 'atmosphere' | 'surface' | 'legend' | 'moons' | 'map'`;
  card ids in the dock are `info.<CardId>`; `header` is `full`,
  not detachable, and the system summary sits above it, `full`, not
  detachable.

Placement (each value of the mapper's table in exactly one card):

- Header: name and rename, class (`classLabel`), "Similar to X"
  (`shownExtras.similar`, localised with `archetypeName`), generated or
  Generate, and for a moon `↑ <host name>` calling
  `world.showSystemBody(body.parent)`.
- Orbit and light: orbit (a, e, period), spin (period, lock, resonance,
  tilt), light (flux, equilibrium), star or host size in the sky; for a
  moon the host's day and tidal flux (`meta.host`, without the host's
  mass and size, which the host's own card shows).
- Interior: mass, radius or axes, density, gravity, circumference, bulk
  or layers (ocean under ice), regime, core and field, internal heat;
  for a giant `meta.giant` mass, radius, flattening, heat ratio.
- Atmosphere and climate: pressure and gases, surface temperature,
  liquid, life; for a giant clouds, winds, jets, great spot, poles.
- Surface (generated rocky worlds): land and water, sea level,
  volcanism, erosion, surface age, rivers.
- Legend: today's legend for the shown layer; for biomes, swatch, share
  and area per biome with the "show all" toggle and the hover/pin of
  the old legend.
- Moons and rings: moons, rings, D ring, ridge, dust rings and source,
  ring role, `meta.moons` tides.
- Generated map: seed, archetype asked, generated when, reroll, vary,
  copy command and link, delete, downloads.
- The system's age shows once (system summary), not in the cards.

- [ ] **Step 1: Write the failing tests:** `cardsFor` gives no surface card
  before generation and none for a giant; a moon's header renders a
  host button with the host's name; rendering all cards of an
  Earth-like generated planet, a moon and a giant, no label (`<dt>`
  text) appears twice; the legend card for the biome layer lists a
  share and an area per biome.
- [ ] **Step 2:** Run `cd web && npx vitest run src/lib/bodyCards src/components/body`; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `npm run check && npm test`; expect PASS.
- [ ] **Step 5: Commit** `feat(web): split the body details into themed cards`.

### Task 6: Cause trees and resemblance in the cards (sonnet)

**Files:**
- Create: `web/src/components/body/CauseTree.svelte`, `web/src/lib/causes.ts`
- Modify: `BodyHeader.svelte`, `InteriorCard.svelte`, `AtmosphereCard.svelte` (Task 5)
- Modify: `web/src/lib/i18n/en.ts`, `fr.ts` (`causes` section: a label and a unit per `Quantity`, a name per `InputField`, the `thermostat` note)
- Test: `web/src/lib/causes.test.ts`, `CauseTree.test.ts`

**Interfaces:**
- Consumes: `Explanation`, `Cause`, `Quantity`, `InputField` types (Task 1, generated), `world.shownExtras` (Task 3).
- Produces: `export function treeFor(explanation: Explanation | null, quantity: Quantity): Cause | null`;
  `export function formatCause(cause: Cause): string` (value with its
  unit, or the localised `label`).

Behaviour: a value with a tree shows a disclosure mark; clicking
unfolds its causes (closed by default, one open state per card); a node
with `input` is styled as an input (accent colour) and its title names
the form field; class (header), surface temperature, pressure, liquid
(atmosphere card), regime, magnetic field, water kept (interior card).
Without `shownExtras` nothing unfolds.

- [ ] **Step 1: Write the failing tests:** `treeFor` finds a root and gives
  null for a missing one; `formatCause` formats K, bar, W and a label;
  every `Quantity` and `InputField` value of the generated types has an
  `en` and an `fr` string (iterate over a const list typed against the
  generated unions, so a new Rust code fails `npm run check`); the tree
  renders an input node with the input class.
- [ ] **Step 2:** Run `cd web && npx vitest run src/lib/causes src/components/body`; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `npm run check && npm test`; expect PASS.
- [ ] **Step 5: Commit** `feat(web): unfold derived values into their causes`.

### Task 7: Documentation, look in a GPU browser, review

**Files:**
- Modify: `docs/system-spec.md` (section `## Interface`, line 790: the
  card's themes, detaching, legend card, host button, cause trees,
  "Similar to"); `README.md` (find the viewer card section with
  `grep -n '^#' README.md`, update only that range); the API section
  that documents `GET /api/systems/{id}` (`grep -n 'api/systems' README.md docs/*.md`).

- [ ] **Step 1:** Update the documentation (English; markdownlint rules).
- [ ] **Step 2:** Rust and web checks green (Global Constraints).
- [ ] **Step 3:** Commit `docs: describe the themed body cards and cause trees`.
- [ ] **Step 4:** `viewer-visual-check` on a local build: an Earth-like planet, Titan (host button, tree on liquid), a gas giant, a narrow window (< 720 px); detach and dock back the legend card; report overlaps and console errors.
- [ ] **Step 5:** `branch-reviewer` on the whole branch against the spec and this plan; fix, re-run the tests.
