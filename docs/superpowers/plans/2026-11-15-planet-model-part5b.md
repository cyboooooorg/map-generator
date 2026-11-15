# Planet model part 5b (the form) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the generate form every model input of a lone world, a
live preview laid out as the body's cards with cause trees, and live
validation for every request, all through one server route that resolves
a request the way Generate does.

**Architecture:** The generator draws a lone world's star, orbit, mass
and eccentricity from seed-derived streams, so one request with one seed
always gives the same bodies. The server factors `create_system`'s
checks and draw into one function used by `POST /api/systems` and a new
`POST /api/systems/resolve` that writes nothing and returns the bodies
with their cause trees (`body_extras`, part 5a) or the same refusal.
The web form gains folded model field groups, a debounced preview store,
and a preview panel that reuses the 5a cards through a narrow data
interface instead of `LoadedWorld`.

**Tech Stack:** Rust (mapgen, mapgen-server, axum, serde, ts-rs),
Svelte 5 + TypeScript (vitest, svelte-check).

**Spec:** `docs/superpowers/specs/2026-11-14-planet-model-part5-design.md`
(part 5b = "Form", and the "My calls" items on the resolve route,
randomness, pinning, refusals, field scales and links between the chain
and the form).

## Global Constraints

- Scope of model fields and preview: a lone world (`{around: WorldRequest}`)
  only. A whole system keeps its archetype counts and gets live
  validation only.
- Validation never disagrees with Generate: both routes go through one
  Rust function; the web only shows what the server said.
- Generate makes what was previewed: the preview and Generate send the
  same request with the same `seed`.
- Pinning a field does not shift any other draw.
- Whole systems stay byte-identical; no `FORMAT_VERSION`, system format
  or index schema bump.
- No prose in Rust; every new string in both `web/src/lib/i18n/en.ts`
  and `fr.ts` (`fr` is typed against `en`).
- Preview debounce: 300 ms after the last change.
- Each field shows one "drives" line; a value with an unfamiliar unit
  shows an Earth comparison; sliders carry Solar System reference marks
  (Earth, Mars, Venus, Titan) where they fall in range.
- Worktree `~/Developer/mg-planet-model-p5b`, branch
  `feat/planet-model-part5b`. Before any web command:
  `ln -s ~/Developer/map-generator/web/node_modules web/node_modules`
  (never `npm ci` in a worktree).
- Rust checks: `cargo fmt --all`,
  `cargo clippy --workspace --all-features --all-targets -- -D warnings`,
  `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`
  (regenerates `web/src/lib/api/types/*.ts`: commit them). Web checks:
  `cd web && npm run check && npm test`. The server index test is flaky
  until PR #485 lands: rerun it alone before believing a failure.
- Commits: Conventional Commits, English, no emoji, with the session's
  `Co-Authored-By` and `Claude-Session` trailers.

## Review Focus

1. **Out-of-order answers.** A slow resolve for an older form state that
   arrives after a newer one must not replace it. Test in Task 3.
2. **The server unreachable or failing (5xx, network).** The preview says
   it is unavailable; Generate stays enabled, since only a refusal (400
   with a detail) disables it. Test in Task 3.
3. **Fields the chosen planet ignores** (`ignored` in the answer, e.g.
   water on a giant): shown greyed with "not used by this planet", still
   kept in the form. Test in Task 5.
4. **A giant, a mini-Neptune or a body without trees in the preview:** the
   cards render without surface trees and without a crash. Test in Task 4.
5. **A varied world** (`initial` with pinned inputs from `keepModelInputs`):
   the fields open pinned at those values and the preview matches them.
   Test in Task 5.

---

### Task 1: A lone world drawn from its seed only (sonnet)

**Files:**
- Modify: `crates/mapgen/src/request.rs:518-640` (`resolve_with_archetype`), `:133` (`WorldRequest`), `:458-480` (`overrides`)
- Test: `crates/mapgen/src/request.rs` tests module

**Interfaces:**
- Produces: `WorldRequest.ch4: Option<f64>` (factor on the formation rule, limits `archetype::VOLATILE_FACTOR`, passed to `InputOverrides.ch4`; an archetype's own `ch4` is replaced when the request sets it). With `seed` fixed, `resolve_with_archetype(archetype, rng)` returns the same result whatever `rng` is.

The star (`draw_type_living`, `:545`), the orbit pick (`:552-562`), the
mass (`draw_mass`, `:569-572`) and the eccentricity (`:594-602`) stop
drawing from the caller's `rng`. Each gets its own stream,
`seeded(seed, "world/<name>/<attempt>")`, with `<attempt>` the index of
the moon-orbit retry loop (`:591`), and each draws whether or not the
request pins its value, then applies the pin. The caller's `rng` only
draws the seed when the request has none (`:533`).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_seeded_world_ignores_the_callers_rng() {
    let request = WorldRequest { seed: Some(7), ..Default::default() };
    let a = request.resolve_with_archetype(None, &mut StdRng::seed_from_u64(1)).unwrap();
    let b = request.resolve_with_archetype(None, &mut StdRng::seed_from_u64(2)).unwrap();
    assert_eq!(a.params, b.params);
    assert_eq!(a.model(), b.model());
}

#[test]
fn pinning_the_mass_moves_nothing_else() {
    let free = WorldRequest { seed: Some(7), ..Default::default() };
    let pinned = WorldRequest { mass_earth: Some(2.0), ..free.clone() };
    let (a, b) = (resolve(&free), resolve(&pinned));
    assert_eq!(a.params.star, b.params.star);
    assert_eq!(a.params.orbit_au, b.params.orbit_au);
    assert_eq!(a.params.eccentricity, b.params.eccentricity);
    assert_eq!(a.model().inputs.water_mass_fraction_drawn(), b.model().inputs.water_mass_fraction_drawn());
}
// Same for pinning the star, the orbit (orbit_au) and the eccentricity:
// the other three stay. And `ch4: Some(3.0)` reaches InputOverrides.ch4.
```

(`resolve` is a test helper with a fixed caller rng; use the field names
`params` and `model()` actually expose, the intent is "star, orbit,
eccentricity and the drawn inputs are equal".)

- [ ] **Step 2:** Run `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features request:: -- --test-threads=3`; expect FAIL.
- [ ] **Step 3:** Implement the streams and `ch4`.
- [ ] **Step 4:** Run the Rust checks; expect PASS. Goldens of lone worlds that were drawn with a test `StdRng` may move: update them and say so in the commit body. Whole-system goldens must not move.
- [ ] **Step 5: Commit** `feat(mapgen): draw a lone world from its seed only`.

### Task 2: One resolve path for Generate and the preview (sonnet)

**Files:**
- Modify: `crates/server/src/app.rs:587-723` (`create_system`), `:305-352` (router), `:477` (`form`)
- Modify: `crates/server/src/api.rs:53` (`InputLimits`), `:183-280` (`ErrorDetail`, `of_request`), near `:396` (new `ResolvedSystem`)
- Modify: `web/src/lib/i18n/errors.ts`, `en.ts:1452-1490`, `fr.ts` (new error cases)
- Test: server tests next to `create_system`'s; `web/src/lib/i18n/errors.test.ts`

**Interfaces:**
- Consumes: Task 1 (a seeded request resolves the same each time).
- Produces:
  - `async fn resolve_request(state: &Shared, request: SystemRequest) -> Result<(System, Vec<String>), Rejection>`: today's `create_system` steps `:592-649` (size check, archetype lookup, draw, pixel check), returning the system and `ignored`. `create_system` calls it, then goes on from `system_id`.
  - `POST /api/systems/resolve` → `resolve_system`, body `SystemRequest`, answer 200 `Json<ResolvedSystem>` or the same `Rejection` as `create_system`. Writes nothing, holds nothing, queues nothing.
  - `#[derive(TS)] pub struct ResolvedSystem { pub system: mapgen::system::System, pub extras: BTreeMap<String, BodyExtras>, pub ignored: Vec<String> }` (extras from `body_extras`, `app.rs:805`). The asked body is `system.focus`.
  - `ErrorDetail::BadArchetype { archetype: String }` and `ErrorDetail::BadAge { age_gyr: f32, lifetime_gyr: f32 }`; `of_request` maps both (`:277` returns `None` today).
  - `InputLimits` gains `ch4: Limits` (`VOLATILE_FACTOR`) and `age_gyr: Limits` (the bounds `resolve_age_gyr` and `validate` accept; the star's lifetime is checked by the route, not by the limits).
  - Web: `detailText` cases for `bad_archetype` and `bad_age` (age, lifetime, "lower the age or pick a longer-lived star").

- [ ] **Step 1: Write the failing tests:**
  - `resolve_refuses_like_create`: for one request per refusal the HTTP layer can produce (`OutOfRange`, `SizeOutOfRange`, `UnknownArchetype`, `BadArchetype` (a moon archetype asked alone), `BadAge` (age 12 Gyr around an F0), `UnknownStar`, `OrbitAndPosition`, `WaterHeavierThanPlanet`, `TooManyPlanets`, `ZoneFull`, `TypeImpossible`, `NoRoomForMoons`, `HotGiantMoons`, `MoonsOfAMoon`, `TooManyPixels`), both routes answer the same status and the same JSON body.
  - `resolve_then_create_draw_the_same_system`: one `{around: {seed: 11, planet: "earth-like"}}` resolved, then created; the created `system.json` equals the resolved `system`; same for a whole system with a seed.
  - `resolve_writes_nothing`: the systems directory and the index are unchanged after a resolve.
  - `resolve_sends_trees`: the focus body of an Earth-like resolve has a `SurfaceTemperature` tree in `extras`.
  - Web: `detailText` for `bad_age` and `bad_archetype` in both locales.
- [ ] **Step 2:** Run `RAYON_NUM_THREADS=4 cargo test -p mapgen-server --all-features resolve -- --test-threads=3`; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the Rust checks and the web checks; expect PASS.
- [ ] **Step 5: Commit** `feat(server): resolve a system request without writing it`.

### Task 3: Preview store and form helpers (sonnet)

**Files:**
- Modify: `web/src/lib/api/client.ts:99` (next to `createSystem`)
- Create: `web/src/lib/preview.svelte.ts`, `web/src/lib/modelFields.ts`
- Modify: `web/src/lib/causes.ts`, `web/src/lib/i18n/errors.ts`
- Test: `web/src/lib/preview.test.ts`, `modelFields.test.ts`, `causes.test.ts`, `i18n/errors.test.ts`

**Interfaces:**
- Consumes: `ResolvedSystem`, `ErrorDetail` (Task 2).
- Produces:
  - `api.resolveSystem(request: SystemRequest, signal?: AbortSignal): Promise<ResolvedSystem>`.
  - `class Preview` with `$state` fields `result: ResolvedSystem | null`, `refusal: ErrorDetail | null`, `unavailable: boolean`, `pending: boolean`, `changed: ReadonlySet<Quantity>`; `request(next: SystemRequest): void` (debounced 300 ms; aborts the call in flight; an answer for an older request is dropped); `dispose(): void`. A 400 with a detail sets `refusal` and clears `unavailable`; any other failure sets `unavailable` and keeps the last `result`.
  - `changedQuantities(prev: Explanation | null, next: Explanation | null): Set<Quantity>`: roots whose value moved by more than 1 % relative (or appeared, or vanished).
  - `drivenBy(explanation: Explanation | null, field: InputField): Quantity[]`: the roots whose tree has a leaf with `input === field`, in tree order.
  - `errorField(detail: ErrorDetail): ModelFieldKey | 'planet' | 'star' | 'orbit' | 'planets' | 'size' | null`: the field a refusal belongs to (`out_of_range.field`, `water_heavier_than_planet` → `water_oceans`, `bad_age` → `age_gyr`, `bad_archetype`/`unknown_archetype` → `planet`, `orbit_and_position` → `orbit`, `too_many_planets`/`zone_full`/`type_impossible` → `planets`, size and pixel refusals → `size`).
  - `modelFields.ts`: `type ModelFieldKey = 'mass_earth' | 'core_mass_fraction' | 'water_oceans' | 'envelope_fraction' | 'co2' | 'n2' | 'ch4' | 'radiogenic' | 'life' | 'eccentricity' | 'age_gyr'`; `MODEL_GROUPS: { id: 'orbit' | 'composition' | 'heat' | 'volatiles' | 'life'; fields: ModelFieldKey[] }[]` (orbit: eccentricity, age_gyr; composition: mass_earth, core_mass_fraction, water_oceans, envelope_fraction; heat: radiogenic; volatiles: co2, n2, ch4; life: life); `fieldOf(input: InputField): ModelFieldKey | 'orbit'` (`insolation` → `'orbit'`, the star and orbit step); `scaleOf(key)`: `'log'` for mass, water, co2, n2, ch4, radiogenic, `'linear'` otherwise, with water's slider starting at a "none" stop for 0; `marks(key, archetypes: ArchetypeInfo[]): { body: 'earth' | 'mars' | 'venus' | 'titan'; value: number }[]` from the built-in archetypes `earth-like`, `mars-like`, `venus-like`, `titan-like` (a `Fixed` value, or a `Range`'s geometric mean on a log scale and arithmetic mean otherwise), dropped when outside the field's limits; `limitsOf(key, spec: FormSpec): Limits`.

- [ ] **Step 1: Write the failing tests:**
  - `preview.test.ts` (fake timers, mocked `api.resolveSystem`): three `request` calls within 300 ms make one call; an answer resolved after a newer one is dropped; a 400 `{detail: bad_age}` sets `refusal`; a 500 sets `unavailable` and keeps the last `result`; a success after a refusal clears it; `changed` holds `surface_temperature` after a CO2 change moved it.
  - `causes.test.ts`: `drivenBy` on a fixture tree returns the roots that hold a `co2` leaf; `changedQuantities` ignores a 0.5 % move and reports a 2 % move.
  - `modelFields.test.ts`: every `InputField` has a `fieldOf`; `marks('mass_earth', archetypes)` holds Earth at 1 and Mars at about 0.107; a mark outside limits is dropped.
  - `errors.test.ts`: `errorField` for each mapping above.
- [ ] **Step 2:** Run `cd web && npx vitest run src/lib/preview src/lib/modelFields src/lib/causes src/lib/i18n/errors`; expect FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the web checks; expect PASS.
- [ ] **Step 5: Commit** `feat(web): add the preview store and the model field helpers`.

### Task 4: Cards that render from any body (sonnet)

**Files:**
- Create: `web/src/lib/cardSource.ts`
- Modify: `web/src/components/body/{BodyHeader,OrbitCard,InteriorCard,AtmosphereCard,SurfaceCard,MoonsCard}.svelte`, `CauseTree.svelte`, `CauseBranch.svelte`
- Modify: `web/src/App.svelte:564-576` only if the props change shape
- Test: `web/src/components/body/PreviewCards.test.ts` (new), `CauseTree.test.ts`

**Interfaces:**
- Produces:
  - `interface CardSource { readonly shownSystemBody: Body | null; readonly shownExtras: BodyExtras | null; readonly systemBody: string | null; readonly meta: WorldMetadata | null; readonly stats: …; readonly rivers: …; readonly changed?: ReadonlySet<Quantity> }` with the exact types `LoadedWorld` already has (`world.svelte.ts:18-104`); `LoadedWorld` satisfies it unchanged. The six cards take `world: CardSource`.
  - `previewSource(result: ResolvedSystem, changed: ReadonlySet<Quantity>): CardSource` in `cardSource.ts`: the focus body, its extras, `meta`, `stats` and `rivers` empty or null.
  - A line whose tree's root quantity is in `world.changed` gets the class `moved`, a 1.5 s highlight (CSS animation, reduced-motion: none).
  - `CauseBranch` and `CauseTree` take `oninput?: (field: InputField) => void`; when set, an input leaf is a button that calls it (in the loaded world's card it stays unset and nothing changes).

- [ ] **Step 1: Write the failing tests** (render with `svelte/server`, as `CauseTree.test.ts` does): each of the six cards renders from `previewSource` of a fixture Earth-like `ResolvedSystem` with no `meta` and shows its surface temperature once; the same for a fixture hot Jupiter and a mini-Neptune, with no surface tree and no throw; a line in `changed` carries `moved`; an input leaf renders a button when `oninput` is set and plain text otherwise.
- [ ] **Step 2:** Run `cd web && npx vitest run src/components/body`; expect FAIL.
- [ ] **Step 3:** Implement. The existing card tests and `bodyCards.test.ts` must still pass.
- [ ] **Step 4:** Run the web checks; expect PASS.
- [ ] **Step 5: Commit** `refactor(web): let the body cards render from any body`.

### Task 5: Model fields, preview and live validation in the form (sonnet)

**Files:**
- Create: `web/src/components/form/ModelFields.svelte`, `web/src/components/form/PreviewPanel.svelte`
- Modify: `web/src/components/GenerateForm.svelte` (`FIELDS` `:100`, `buildRequest` `:348`, `buildSystemRequest` `:255`, submit `:411`, error `:844`), `web/src/lib/variedWorld.ts` (`keepModelInputs`)
- Modify: `en.ts` `form` (`:676-760`), `fr.ts`
- Test: `web/src/components/form/ModelFields.test.ts`, `PreviewPanel.test.ts`, `web/src/lib/variedWorld.test.ts`

**Interfaces:**
- Consumes: Tasks 2-4.
- Produces:
  - `ModelFields` props `{ spec: FormSpec; archetypes: ArchetypeInfo[]; values: Record<ModelFieldKey, { random: boolean; value: number | boolean }>; drawn: PlanetInputs | null; driven: (key: ModelFieldKey) => Quantity[]; ignored: string[]; refusal: ErrorDetail | null; open: Set<string> }`. One `<details>` per group, folded by default, opened by a jump. Each field: label, a random/pinned switch (random shows the drawn value from the preview, greyed), slider and number box on its scale, unit with an Earth comparison where the unit is unfamiliar (water in Earth oceans, radiogenic heat and volatile factors relative to Earth), reference marks under the slider, the "drives" line (`→ ` + `driven(key)` named through `t.causes.quantities`, or the static `t.form.model.<key>.drives` while no preview exists), "not used by this planet" when in `ignored`, the refusal's text when `errorField(refusal)` is this field. `life` is a three-way choice: drawn, with life, lifeless.
  - `PreviewPanel` props `{ preview: Preview; onjump: (field: InputField) => void; onreroll: () => void }`: the six cards from `previewSource`, the seed with a "new draw" button, a quiet "updating" state, "preview unavailable" when `unavailable`.
  - `GenerateForm`: in around mode, `ModelFields` sits in the planets step and `PreviewPanel` beside the steps (to their right from 900 px of form width, below them and folded under a "Preview" summary on narrower screens); every model field is sent in `buildRequest` when pinned. The seed: when the seed field is random, the form draws one when it opens and sends it in every preview and in Generate; "new draw" replaces it. Every change of the request calls `preview.request(...)`, in both modes. Generate is disabled while `preview.refusal` is set; the refusal's text shows at its field (`errorField`) or above the buttons when it has none. A jump from a tree leaf goes to the planets step (or the star step for `orbit`), opens the group and focuses the field.
  - `keepModelInputs` carries `ch4`, `eccentricity` and `age_gyr` too.

- [ ] **Step 1: Write the failing tests:**
  - `ModelFields`: all groups folded by default; a field in `ignored` shows the "not used" text; an `out_of_range` refusal on `co2` shows at the CO2 field only; Earth and Mars marks under the mass slider; a random field shows the drawn value.
  - `PreviewPanel`: renders the cards of a fixture result; "preview unavailable" when `unavailable`; the seed shown.
  - `variedWorld.test.ts`: a varied world with pinned `co2`, `ch4` and `age_gyr` keeps them.
  - Pure form logic moved into `lib/` where a test needs it (for example a `modelRequest(values): Partial<WorldRequest>` that sends pinned fields only and `life` as a boolean or nothing).
- [ ] **Step 2:** Run `cd web && npx vitest run src/components/form src/lib/variedWorld`; expect FAIL.
- [ ] **Step 3:** Implement, with every string in `en.ts` and `fr.ts`.
- [ ] **Step 4:** Run the web checks; expect PASS.
- [ ] **Step 5: Commit** `feat(web): add the model fields, live preview and validation to the form`.

### Task 6: Documentation, look in a GPU browser, review

- [ ] **Step 1:** Document `POST /api/systems/resolve`, `ResolvedSystem`, the two new error codes and the seed rule for lone worlds in `docs/system-spec.md` (`## API` `:665`, `### Errors` `:447`, `### Around one planet (mode 2)` `:429`); the form in `## Interface` (`:793`); `ch4` in the request options of `README.md` (`### Options` `:44`) and the form in `## Web app` (`:210`). Find each section with `grep -n` first. Commit `docs: document the resolve route and the model form`.
- [ ] **Step 2:** Run the Rust and web checks once more on the whole branch.
- [ ] **Step 3:** `viewer-visual-check` on a local build: the form editing an Earth into a Mars (mass 0.107, water down, CO2 up) with the preview following and the moved lines highlighted; a refusal (age 12 Gyr around an F0) flagged at the age field with Generate disabled; a whole system with too many planets refused live; a narrow view (< 900 px) with the preview below. Then Generate from the Mars state and check the card of the made world matches the last preview.
- [ ] **Step 4:** `branch-reviewer` on the branch against the spec and this plan; fix what it finds, re-run the tests.
