# Physical planet model, part 5: card and form

Step 5 of #423 (design: `2026-11-11-physical-planet-model-design.md`,
sections "Presets and the form" and "Order of work"), roadmap #427.
Folds in #466 ("Similar to X" on the card).

## Intent

Parts 1 to 4 made every planet and large moon the result of a physical
model: drawn inputs (mass, composition, water, volatiles, internal heat,
life) go through a chain (interior, escape, regime, core, atmosphere,
climate, liquid, life, class). The web app does not show that chain, and
the form cannot set its inputs. The user, on 2026-11-14:

- **The card is a block.** A body's details are two cards that repeat
  each other (`BodyCard.svelte` and `WorldInfo.svelte` both show orbit,
  light, temperature, atmosphere, age). The biome list repeats the
  viewer's legend. From a moon, the host's details sit inside the moon's
  card, which is confusing.
- **The form is blind.** "I don't know what some values act on, so I
  create blindly." An impossible request is only reported when Generate
  is pressed.
- **Learning is the point.** The cause chain is wanted "for educational
  purposes". What would make the user wince: a form that is not
  ergonomic. "It is already a complex scientific field: the UI/UX must
  encourage learning."

Part 5 turns the card into themed cards, shows why each derived value is
what it is, and gives the form every model input with a live preview and
live validation.

## Goals

1. **Themed cards.** A body's details are split into cards, one theme
   each, with no value shown twice.
2. **Detachable cards.** Each themed card can be taken out of the panel
   as a floating card and put back.
3. **The legend is a card of the body's details**, not a separate
   floating card: it describes the world too. It carries the biome
   shares and areas that the card lists today.
4. **The host has its own button.** A moon's card links to its planet's
   card instead of copying the planet's details.
5. **Cause chain.** Each derived value unfolds into its causes, two or
   three levels deep, down to the inputs the form can set.
6. **"Similar to X"** (#466): the built-in archetype a body resembles,
   when close enough.
7. **Every model input is a field of the form**, grouped and folded,
   each saying what it drives.
8. **Live preview**: the derived state, as the card would show it,
   recomputed on every change, without generating maps.
9. **Live validation**: an impossible request is reported as soon as it
   is, with the reason and what to change; Generate is disabled until
   it is fixed.

## Non-goals

- New biomes, Köppen climate, surface layers: #424.
- Setting derived values directly (pressure, sea level, class): the
  fantasy-planet feature (Later in the main design).
- The climate and atmosphere clean-up pass (#441, #442, #443, #447,
  #453, #454, #463, #464): after part 5, before #424 (triage comment on
  #423).
- An interior cross-section view (Later).
- Changing the physics. Part 5 explains and exposes the model: drawn
  systems stay byte-identical and no format bumps (see "pinning a field"
  below for requests with overrides).

## Anti-goals

- **An unergonomic form.** A wall of sliders with physics jargon, or a
  form where changing one value makes unrelated values jump, fails
  even if every field works.
- **Explanations that lie.** A cause chain whose numbers differ from the
  values the model stored, or a "why" written by hand in the web app
  that drifts from the Rust chain.
- **Repetition.** Any value shown in two cards of the same body.
- **Validation that disagrees with Generate**: the form says fine and
  the server refuses, or the reverse.

## Decided with the user (2026-11-14)

### Card layout

Mockups shown during the design: `cards-layout.html`,
`cards-grid-v2.html`, `cause-chain.html` (visual companion, not kept in
the repository).

- The body's details panel widens from 260-380 px (`Dock.svelte:259`)
  to about 680 px. The user accepted that it covers more of the globe.
- **Header card, full width:** name (renamable, as today), class,
  "Similar to X", whether the maps exist (Generate button), and for a
  moon a **"↑ host" button** that shows the host's card (the host body
  selected, as picking it in the planet strip does).
- **Themed cards below, in a two-column grid**, one column when the
  panel is too narrow. Themes, as shown to the user:
  - Orbit and light: orbit, period, eccentricity, spin and tilt, light
    received, the star's size in the sky; for a moon, the host's size in
    its sky and the tidal heating (these describe the moon, so they stay).
  - Interior: mass, radius, gravity, composition, layers (ocean under
    ice), core and magnetic field, regime, internal heat.
  - Atmosphere and climate: pressure, gases, surface temperature,
    surface liquid, life.
  - Surface: land and water, rivers, surface age (once the maps exist).
  - Legend: follows the layer shown; for the biome layer, each biome's
    swatch, share and area (today in `WorldInfo.svelte:287-309`), with
    the hover and pin behaviour of today's biome legend
    (`Viewer.svelte:2769-2798`).
  - Moons and rings.
  - Generated map: seed, reroll, vary, copy command and link, downloads,
    delete.
- The system summary above (star and planet strip, `BodyCard.svelte:113`)
  stays as it is.
- **Detaching**: each themed card has a button that takes it out as a
  floating card, draggable, its place remembered like the dock's
  floating panels (`lib/dock.ts`); a button puts it back, and the grid
  closes the gap.
- The free-floating legend (`legendOpen`, `legendAt`, `dragLegend` in
  `Viewer.svelte:388-449`) goes away: the detached legend card replaces
  it.

### Cause chain

- A derived value unfolds (click) into a tree of two or three levels,
  as in the mockup: surface temperature 288 K ← equilibrium without air
  255 K (← light received, albedo) + greenhouse 33 K (← CO₂, water
  vapour). The user approved that depth.
- Inputs the form can set are marked in the tree (the mockup colours
  them), linking what is seen to what can be set.

### Form

- Every model input is a field, grouped (orbit, composition, internal
  heat, volatiles, life) and folded by default, as the main design says.
- Each field says what it drives, in one line ("→ sea level, land,
  ice, water vapour").
- A live preview next to the fields shows the derived state, laid out
  as the card's themed cards, and briefly highlights what the last
  change moved.
- An impossible request is flagged as soon as the form holds it, with
  the reason and what to change; Generate stays disabled meanwhile.
- The UX is a goal in itself: the form must encourage learning.
- **Scope: a lone world only.** The model fields and the live preview
  apply to a planet asked on its own (`{around: WorldRequest}`). A whole
  system keeps today's archetype counts (`WholeSystemRequest.planets`);
  its request gets no per-planet inputs. It does get the live
  validation (too many planets, a zone full, an age past the star's
  lifetime) through the same resolve route.

## My calls (not asked to the user)

- **One computation for everything.** The Rust chain (`planet_model`,
  `planet/mod.rs:347`) produces the cause tree as data while it derives
  the model: nodes of a quantity code, a value, a unit, child nodes, and
  for an input the form field it comes from. No prose in Rust: the web
  owns the wording (`lib/i18n/en.ts`, `fr.ts`), as it does for
  `ErrorDetail` (`lib/i18n/errors.ts`). The card, the preview and the
  validation all read the server's output, so they cannot disagree.
- **Preview and validation through one new route** that resolves a
  `SystemRequest` without writing anything or queueing a job: the same
  code path as `POST /api/systems` (`create_system`, `app.rs:586`) up
  to the draw, stopping before `write_system`. It returns either the
  drawn bodies (with their models and cause trees) or the same
  `ErrorDetail` that Generate would return. The draw does no map work
  (`--draw-only` in the CLI shows it), so it can run on each change,
  debounced (a few hundred ms).
- **The preview holds its randomness still.** While the user edits, the
  preview draws with a fixed seed (the form's, or one held for the
  editing session), so that changing one input moves only what that
  input causes. Otherwise random fields would jump on every change and
  the preview would teach nothing.
- **Generate makes what was previewed.** The preview's seed is the
  request's `seed` (`WorldRequest.seed`, `WholeSystemRequest.seed`), so
  Generate sends the same request and draws the same bodies; a refusal
  that depends on the draw is then the same in both.
- **Pinning a field does not shift the other draws.** Drawing an input
  consumes the random stream whether the request overrides it or not
  (`draw_inputs_with`, `planet/inputs.rs`), so fixing the mass leaves the
  drawn water and volatiles where they were. If the code skips draws
  for overridden inputs today, it stops: requests with overrides may
  then draw differently than before, drawn systems (no overrides) stay
  byte-identical.
- **The cause tree is recomputed, not stored.** The server rebuilds it
  from the stored inputs and formation context with the same function
  that derives the model, and sends it in its answers only: no format
  bump, existing systems get trees. A test checks that the recomputed
  model equals the stored one on freshly drawn systems; where it does
  not (code changed without a bump), the card shows the stored values
  and no tree rather than a tree that disagrees.
- **"Similar to X" is computed by the server** from the stored inputs and
  the built-in archetype catalog, and sent with the body; not stored.
- **"No value twice" applies to the cards' own lines**, not to the
  contents of cause trees, which repeat values by nature.
- **Every string in English and French.** The app's interface is
  bilingual (`lib/i18n/en.ts`, `fr.ts`); new strings land in both in
  the same change.
- **Every refusal gets a machine-readable detail.** `BadArchetype` and
  `BadAge` have only text today (`api.rs:277`); they get `ErrorDetail`
  variants. A refusal tied to one field is shown at that field.
- **Fields teach their scale.** Each slider carries reference marks of
  the Solar System bodies where they apply (Earth, Mars, Venus, Titan),
  and a value shows its unit with an Earth comparison where the unit is
  unfamiliar (water in Earth oceans, radiogenic heat relative to Earth).
- **The chain and the form link both ways.** In the preview, an input
  marked in a cause tree jumps to its field; a field's "drives" line
  names the derived values it moves.
- **"Similar to X" compares causes**, not outcomes: a distance in the
  archetypes' input space (log mass, core fraction, water or ice, light
  received, and for a moon the host kind and the distance in host radii),
  shown only under a threshold. A body asked by archetype shows its own.
  The class already names the outcome.
- **Two steps, one pull request each.** 5a the card (cause tree in the
  generator, themed and detachable cards, legend in the card, host
  button, "Similar to X"); 5b the form (resolve route, all fields, live
  preview, live validation). 5a comes first: the preview reuses its
  cards and cause trees.

## Left to the builder

- The exact cause tree per derived value, and its quantity codes,
  within the chain's steps (`planet/mod.rs:321-346`); which values get
  a tree (at least class, surface temperature, pressure, surface liquid,
  regime, magnetic field, sea level).
- How the dock lets a sub-part of a panel float (the dock floats whole
  panels only, `Dock.svelte:60`): a nested panel notion in the dock or a
  light floating layer that reuses `clampToView` and the saved layout.
  The view-width rule `FLOAT_MIN_WIDTH` (720 px, `dock.ts:87`) applies
  to detached cards too.
- Where the legend card stands when the details panel is not the
  dock's open panel (for example, detached by default, as the legend
  floats today).
- The themes for giants (clouds, winds, internal heat, rings, poles,
  `WorldInfo.svelte:113-157`) and for bodies without a model (small
  moons, the star view's selected body).
- The similarity distance's weights and threshold, measured on drawn
  systems so that Solar System analogues are named and ordinary bodies
  are not.
- The form's layout on a narrow screen (preview below the fields,
  sticky or folded).
- Field wording, in English and French.

## Testing

- Rust: each cause tree's leaves and values equal the model's stored
  values on the Solar System archetypes; the resolve route returns the
  same refusal as `POST /api/systems` for every `RequestError` variant.
- Web: no value appears in two cards of one body; detaching and
  docking back a card keeps it; the legend card shows the biome shares.
- In a GPU browser (`viewer-visual-check`): the card on a planet, a
  moon (host button) and a giant; the form editing an Earth into a Mars
  with the preview following.
