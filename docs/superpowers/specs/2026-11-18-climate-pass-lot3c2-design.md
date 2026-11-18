# Climate pass lot 3C-2: a right and readable climate

Track A of #423, roadmap #427. Lot 3C of the climate pass is split into
3C-1 surfaces and thin airs (merged, #567), 3C-2 climate (this document)
and 3C-3 oceans (#535 #536 #537 #540).

## Intent

The user's criterion for this lot: **the physical explanation is right and
understandable in the interface.**

- *Right*: every value a card shows (temperature, CO₂, albedo, water,
  mass) comes from faithful physics, and the cause tree replays exactly
  what the model computed. No refusal, no silent gap, no constant standing
  in for a process.
- *Understandable*: a reader who does not know the physics can follow why a
  world is frozen, temperate or boiling. Causes are named by a real process
  (CO₂ condensing high in the air), not by a tuned constant. Chemical
  formulas and uncommon units explain themselves on hover.

The project-wide criterion still holds: maximum physical realism, and every
archetype comes out as its real body on every seed.

## Delivery: two pull requests

1. **PR 1, interface** (`feat/web-unit-glossary`, from `main`): tooltips on
   formulas and units, and a dedicated "Units" button and panel. Web only,
   worlds untouched, so the user can test it first.
2. **PR 2, climate physics and explanations** (`feat/climate-lot3c2`, from
   `main`, this branch): #522 #523 #557 #562 #563 #565 #513 #517 #520.
   Worlds change.

They are independent and may merge in either order. This document lives on
the PR 2 branch; PR 1 refers to it.

## Goals

### PR 1: formulas and units explain themselves

**Tooltips, name only.** Hovering (or tapping, on touch screens) a chemical
formula or a uncommon unit shows its name in full, in the interface's
language, and nothing else:

- formulas: `CO₂` → "carbon dioxide" / "dioxyde de carbone"; also N₂, O₂,
  CH₄, C₂H₆, H₂O, NH₃, SO₂, H₂, He, H₂/He, CO, Ar, and any other formula the
  interface prints (the builder inventories the strings);
- units: `bar` → "bar: pressure"; M⊕ → "Earth mass"; also R⊕, M♃, AU, L☉,
  S⊕, K, W/m², g, Ga, Ma, µbar, and every other unit the interface prints.

The user's decision: the tooltip carries **the name only**. Reference values
("Mars: 0.006 · Earth: 1 · Venus: 92") would overload it; they go to the
panel. Today `unitTooltip` (`web/src/lib/units.ts`) puts name and examples
together in the `title` of some card rows: that becomes the name only.

**Everywhere.** Cards, cause trees, form (labels, help, chips), legend, info
panel, system card, star pane: wherever a formula or unit appears.

**Units panel.** A new button in the viewer's top bar
(`web/src/components/Viewer.svelte`, next to the `?` button), labelled
"Units" / "Unités", opens a reference panel of **units only**. The user's
decision: chemistry has no place there; molecules only get their tooltip.

- The "Units" section moves out of the `?` panel, which keeps the keyboard
  shortcuts only.
- The panel lists every unit the interface uses, each with its name, what it
  means and familiar reference values, in the style of today's six entries
  (`t.units` in `web/src/lib/i18n/fr.ts` / `en.ts`): Ga, Ma, bar, g, AU, L☉
  plus M⊕, R⊕, M♃, S⊕, K (and °C next to it), W/m², µbar, and the others found
  by the inventory. Example references: 1 S⊕ = the light Earth gets; 255 K =
  Earth without greenhouse; 1 M♃ = 318 M⊕.
- The panel behaves like the `?` panel (dialog, close button, Escape). A
  keyboard shortcut is welcome if one is free.

### PR 2: the climate physics

**#522, outer edge of the habitable zone from the column.** Replace the CO₂
cap taken at the equilibrium temperature under albedo 0.3
(`co2_frost_bar(context.equilibrium_k)` in
`crates/mapgen/src/planet/climate.rs`) by CO₂ condensing in the column
itself: saturation at the actual column's upper-air (tropopause) temperature,
together with Rayleigh scattering, as Kopparapu et al. 2013
(doi:10.1088/0004-637X/765/2/131). The maximum greenhouse then comes from the
physics, and the outer edge round the Sun should land near Kopparapu's
maximum greenhouse (about 1.67 AU, 0.36 S⊕) within what a 0-D model can
claim.

- `SNOWBALL_BUILT_UP` goes: a snowball holds the CO₂ the thermostat actually
  reaches.
- The band of orbits "frozen for good" at chance 1 (about 1.5-1.65 AU round
  the Sun) is re-checked; whatever remains must follow from the physics.

**#523, albedo and CO₂ law.**

- Rayleigh column counts gravity (column mass = p/g), so Titan's thick, low-g
  column scatters as it should.
- The air absorbs part of the sunlight (shortwave absorption term), so the
  cloud/air albedo is no longer the bare 0.19.
- The ice reach scales with the planet's area, not Earth's.
- Cloud cover depends on more than vapour (the builder picks a cited
  dependence).
- The albedo is continuous across the runaway edge: no 4-5 K step in the
  equilibrium temperature.
- Refit `CO2_OPACITY` and `CO2_EXPONENT` afterwards so Earth's thermostat
  sits near 2.8e-4 bar (today 1.2e-3).

**#557, CO₂ on hazy hydrocarbon worlds.** Compare the CO₂ inventory of
drawn hazy hydrocarbon worlds with Titan (CO₂ in its air and haze; no CO₂
frost layer of any scale on its surface). Cap or rework the supply if it is
unrealistic, re-check census seed 387 planet d, and decide on physical
grounds whether a hydrocarbon liquid outranks a CO₂ frost in the surface
class (the main session expects it to).

**#562, phase-change jumps.** A probe sweeps the stellar flux across the
methane freezing point (CH₄ activity 0.5 → 1) and N₂ ice's 63.14 K
(emissivity 0.89 → 1). Smooth any jump that makes the fixed point cycle, and
the condensation loop (`CONDENSE_ITERATIONS`) must no longer stop silently:
it either converges or reports non-convergence.

**#563, methane anti-greenhouse anchored.** Anchor one of
`CH4_NEAR_IR_MAX` / `CH4_NEAR_IR_HALF_BAR` on an independent measurement (the
methane-only share in McKay et al. 1991, or the Huygens DISR surface flux);
the other follows from Titan's 93.7 K.

**#565, Titan's pressure.** Run titan-like over seeds 1-500 after the
changes above, then retune `n2` or set the test range to what the model can
honestly claim (still around Titan's 1.47 bar).

### PR 2: right explanations

**#513, mass after a steam runaway.** After the steam pass, subtract the
escaped water from the mass and the water fraction, as
`envelope_after_escape` does for H/He, and derive the interior (radius,
density, gravity) from the water kept. The main session's call: the card
shows the **current** mass; the formation mass and the water lost appear in
the cause tree.

**#517, a boil that re-condensed.** A cause-tree node when
`water.boiled && !water.runaway`: the ocean boiled once, escape removed so
much water, the rest condensed again. EN and FR strings.

**#520, tidally heated heat-pipe moons.** `planet::explain` must replay the
path the chain takes for a steaming heat-pipe `VolcanicWorld` (surface
interior, steam layer, vapour or absorbing context) so its temperature
matches the stored one. The main session's call: replay it faithfully, no
"accept with a note". Add a test over several seeds of the failing request
shape (earth-like asked as a giant's moon; seeds 11, 18, 25, 30 and the 13
of 40 listed in the issue).

**Wording.** Every new or changed cause names the process in plain words
(e.g. "CO₂ condenses high in the air: more of it no longer warms the
ground"), FR and EN, and uses the formulas and units PR 1 explains.

## Non-goals

- Oceans under ice (#535 #536 #537 #540): lot 3C-3.
- Planet names and new classes (#498 #499, ultra-hot part of #501): lot 4.
- Nitrogen seas on the map, the 50× plasma calibration, crusts without a
  stagnant lid (#558 rest, #559 rest, #560).
- Molecules in the Units panel.
- A 1-D radiative-convective model: the climate stays 0-D with a column
  treatment of CO₂ condensation and Rayleigh scattering.

## Anti-goals

- A cause tree that disagrees with the card, or refuses a body.
- A new tuned constant standing in for a process the issue asks to model.
- An archetype that stops looking like its real body on some seed (census).
- Tooltips that repeat the reference values the panel holds.
- A tooltip that only works with a mouse.

## Constraints

- Worlds may change if planets stay coherent; no legacy algorithm, no format
  gate (generation changes policy).
- PR 2 changes the stored model: `SYSTEM_FORMAT_VERSION` 10 → 11
  (`crates/mapgen/src/system.rs`). If world output changes for the same
  inputs, `FORMAT_VERSION` 28 → 29 (`crates/mapgen/src/export/raw_data.rs`).
  Agree on both numbers with the peer sessions before pushing.
- Census examples (`archetype_census`, `planet_census`, `climate_sweep`) are
  the measuring tools; archetype tests keep passing on seeds 1-40 at least.
- Shared machine: throttled tests, no `npm ci` in worktrees, no port 3000.
- Strict TypeScript and Svelte 5 in the web app; i18n strings in both
  `en.ts` and `fr.ts`.

## Design calls

- **Automatic recognition (PR 1).** The main session's call: formulas and
  units are written inline in the translated strings, including cause trees.
  Rather than hand-marking every string, one shared display component
  recognises the known formulas and units in a text and wraps them with
  their tooltip. A new string that writes "CO₂" gets it for free. It must
  not match inside words (the "g" of "grams", "K" in a name) — a unit is
  recognised only next to a number or in a unit position the formatter
  produces; the builder chooses how (e.g. the formatter emits a marked
  unit, the scanner handles formulas).
- **Touch.** A tap shows the tooltip; the builder picks the mechanism
  (`title` alone does not work on touch).
- **Mass on the card (#513).** Current mass on the card, formation mass in
  the tree.
- **Heat-pipe moons (#520).** Faithful replay.
- **Hydrocarbon vs CO₂ frost (#557).** Decided on Titan's evidence; the main
  session expects liquid to outrank the frost.

- **PR order (main session's call).** PR 2 merged alone shows its new
  strings without tooltips; that is acceptable.
- **Outer-edge tolerance (main session's call).** Round the Sun, the outer
  edge lands within ±0.1 AU of 1.67 AU. Round K and M stars it follows from
  the same column physics and is compared with Kopparapu's S_eff fits
  (within ±15 %); no star-dependent tuning constant.
- **Order and bounds of the refits (main session's call).** #523 albedo
  terms first, then #522 condensation, then #563 methane anchor, then the
  CO₂ law refit, then #565. Bounds: Earth's surface 288 ± 2 K with thermostat
  CO₂ within a factor 1.5 of 2.8e-4 bar; Titan 93.7 ± 1 K; the equilibrium
  temperature moves by less than 0.5 K across the runaway edge for a 1 %
  change of flux.
- **Conflicts (main session's call).** If faithful physics and an
  archetype's real body cannot both hold, stop and report to the main
  session; do not add a constant to force either.
- **Format numbers.** Assume system 11 and world 29; the main session
  confirms them with the peers before the push.
- **Steam runaway's interior (#513).** The chain already re-runs once after a
  steam runaway; that re-run uses the interior (gravity, radius) of the water
  kept. No further loop.
- **Non-convergence (#562).** A field on the climate result, surfaced by
  `explain` as a note, and a census counter; no log line only.

## Acceptance

PR 1:

- Every formula and every unit visible in the cards, cause trees, form and
  legend shows its full name on hover and on tap, in FR and EN.
- The "Units" button opens the panel; the `?` panel has shortcuts only.
- Unit tests for the recognition (no match inside words) and the strings.
- A visual check (viewer-visual-check) of a card, a cause tree and the panel.

PR 2:

- Round the Sun, an Earth-like planet's outer edge comes from CO₂
  condensation and lands near Kopparapu's maximum greenhouse; the explain
  tree names that cause.
- Earth's thermostat CO₂ near 2.8e-4 bar; no temperature step at the runaway
  edge.
- Census: every archetype still matches its real body on seeds 1-40; Titan's
  pressure test range justified by seeds 1-500.
- No `explain` refusal over the #520 request shape, seeds 1-40.
- A boiled-and-re-condensed planet shows the boil in its tree; a steam
  runaway's mass and interior count only the water kept.
- The flux probe of #562 finds no cycle, or the cycle is reported.
- `cargo test` and the web tests pass.

## Left to the builder

- The exact column treatment of CO₂ condensation (tropopause temperature
  from the 0-D model's skin temperature or a lapse-rate estimate) as long as
  it is cited and the outer edge lands where Kopparapu does.
- The shortwave absorption and cloud-cover dependences, cited.
- Which measurement anchors the methane anti-greenhouse.
- The inventory of formulas and units, and the reference values of each
  unit in the panel.
- The tooltip mechanism and the recognition method.
