# Unit and formula glossary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every chemical formula and uncommon unit in the web app shows its full name on hover and on tap, and a dedicated "Units" button opens a reference panel of units only.

**Architecture:** A pure recogniser (`lib/glossary.ts`) splits any display string into plain and glossed segments; one Svelte component (`Glossed.svelte`) renders them with a name-only tooltip that also opens on tap (focusable `abbr` + CSS popover). Cards, cause trees, form, legend and panes render their text through it. The units section leaves the `?` dialog for its own `UnitsPanel.svelte`, opened by a new top-bar button and a shortcut.

**Tech Stack:** Svelte 5 (runes), strict TypeScript, vitest with `svelte/server` render.

**Spec:** `docs/superpowers/specs/2026-11-18-climate-pass-lot3c2-design.md` (section "PR 1: formulas and units explain themselves", design call "Automatic recognition", "Touch").

## Global Constraints

- Tooltip carries **the name only**, in the interface language ("carbon dioxide" / "dioxyde de carbone"; "bar: pressure" / "bar : pression"). No reference values in a tooltip.
- The Units panel holds **units only**, never molecules.
- The `?` panel keeps keyboard shortcuts only.
- Works on touch: a tap shows the name (`title` alone is not enough).
- Every string in both `web/src/lib/i18n/en.ts` and `fr.ts`; FR uses `S` (narrow no-break space) before `:` as the file already does.
- No `{@html}`: the app has none; keep it that way.
- Worktree `~/Developer/mg-unit-glossary`; `web/node_modules` is a symlink to the main checkout: never `npm ci`. Run from `web/`: `npm test`, `npm run check`.
- Worlds untouched: no Rust change, no format bump.

## Review Focus

1. A unit letter inside an ordinary word or name ("K" in "K-type star", "g" in "giant", "Ma" in "Mars", "bar" in "barrier") must not be glossed — test in Task 1.
2. A formula next to punctuation or at the end of a string ("CO₂.", "(N₂)", "CH₄/N₂") is glossed — test in Task 1.
3. A glossed term inside a `<button>` (cause-tree input buttons) must not nest a focusable element in the button — test in Task 2.
4. French unit spellings ("UA", "Ga", decimal comma, narrow no-break space between number and unit) are recognised like the English ones — test in Task 1.
5. Opening the Units panel while the `?` panel is open (and Escape) leaves one dialog open and focus back on the page — test in Task 3.

---

### Task 1: Recogniser and names

**Files:**
- Create: `web/src/lib/glossary.ts`, `web/src/lib/glossary.test.ts`
- Modify: `web/src/lib/i18n/en.ts`, `web/src/lib/i18n/fr.ts` (new `glossary` block; the catalog type in `web/src/lib/i18n/catalog.ts` if keys are typed there)

**Interfaces:**
- Produces:
  - `export type GlossKind = 'formula' | 'unit'`
  - `export type Segment = { text: string } | { text: string; kind: GlossKind; name: string }`
  - `export function glossSegments(text: string): Segment[]` — concatenating every `text` gives back the input exactly.
  - `export function glossName(kind: GlossKind, term: string): string | null`
  - i18n: `t.glossary.formulas: Record<Formula, string>` and `t.glossary.units: Record<GlossUnit, string>`; `Formula`, `GlossUnit` exported from `glossary.ts`.

Recognition rules (decided here):
- **Formulas**, matched anywhere between non-letter boundaries, subscript forms only: `CO₂ N₂ O₂ CH₄ C₂H₆ H₂O NH₃ SO₂ H₂ H₂/He`. `H₂/He` is one term ("hydrogen and helium"). ASCII `CO2`, and `He`, `Ar`, `CO` alone, are never matched ("He" is an English word). `k₂/Q` is not a molecule: not glossed. Longest match first (`H₂O` before `H₂`).
- **Symbol units**, unambiguous, matched anywhere: `M⊕ R⊕ M♃ L☉ R☉ S⊕ W/m² µT m/s² mm/s² °C`.
- **Letter units**, matched only right after a number and a space (regular, no-break U+00A0 or narrow no-break U+202F): `bar K g Ga Ma Gyr AU UA km km² TW GW MW W` and `M km²`. The number may use `.` or `,` decimals and `×10ⁿ` superscripts. `%` and time units (s, min, h, d, j) are not glossed.
- Builder inventories the remaining units/formulas the interface prints (grep `en.ts`, `fr.ts`, `format.ts`, `causes.ts`, `planetCard.ts`, `massFormat.ts`, components) and adds any missing one to the lists and names.

- [ ] **Step 1: Write the failing tests** in `glossary.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { glossSegments } from './glossary';

const glossed = (s: string) => glossSegments(s).filter((g) => 'kind' in g).map((g) => g.text);

describe('glossSegments', () => {
  it('round-trips the text', () => {
    const s = 'CO₂ 1.2 bar, 288 K and 5 M⊕.';
    expect(glossSegments(s).map((g) => g.text).join('')).toBe(s);
  });
  it('finds formulas next to punctuation', () => {
    expect(glossed('(N₂), CH₄/N₂ and CO₂.')).toEqual(['N₂', 'CH₄', 'N₂', 'CO₂']);
  });
  it('prefers the longest formula', () => expect(glossed('H₂O and H₂/He')).toEqual(['H₂O', 'H₂/He']));
  it('needs a number before letter units', () => {
    expect(glossed('K-type giant near Mars, a barrier of 4 Ga')).toEqual(['Ga']);
    expect(glossed('288 K')).toEqual(['K']);
  });
  it('reads French numbers and spaces', () => {
    expect(glossed('1,52 UA · 0,006 bar')).toEqual(['UA', 'bar']);
  });
  it('finds symbol units anywhere', () => expect(glossed('masse en M⊕')).toEqual(['M⊕']));
  it('ignores ASCII lookalikes', () => expect(glossed('He said CO2')).toEqual([]));
});
```

Plus one test that every `Formula` and `GlossUnit` has a non-empty name in both `en` and `fr` catalogs (import both catalogs directly as `fr.test.ts` does).

- [ ] **Step 2: Run** `npm test -- glossary` — expected FAIL (module missing).
- [ ] **Step 3: Implement** `glossary.ts` (one regex per class built from the sorted term lists, merged by position) and the `glossary` i18n blocks. Names: name only, e.g. EN `CO₂: 'carbon dioxide'`, `bar: 'bar: pressure'`, `'M⊕': 'Earth mass'`, `K: 'kelvin: temperature from absolute zero'`; FR `'dioxyde de carbone'`, `` `bar${S}: pression` ``, `'masse terrestre'`, `` `kelvin${S}: température depuis le zéro absolu` ``.
- [ ] **Step 4: Run** `npm test -- glossary` and `npm run check` — expected PASS, no warnings.
- [ ] **Step 5: Commit** `feat(web): recognise formulas and units in display text`.

### Task 2: `Glossed` component with tap support

**Files:**
- Create: `web/src/components/Glossed.svelte`, `web/src/components/Glossed.test.ts`
- Modify: `web/src/app.css` (replace the `.unit-help` rule at about line 103 by `.gloss` rules)

**Interfaces:**
- Consumes: `glossSegments` (Task 1).
- Produces: `<Glossed text={string} inert?={boolean} />`. Each glossed segment renders as `<abbr class="gloss" title={name} data-name={name} tabindex="0">term</abbr>`; with `inert` (inside a button or a link) it renders `<abbr class="gloss" title={name}>` with no `tabindex`. Plain segments are text nodes.

Tap mechanism (decided here): a focusable `abbr`; CSS shows a popover under it on `:focus-visible`/`:focus` via `::after { content: attr(data-name) }`, positioned absolute, themed with the existing frame variables, `white-space: nowrap`, `z-index` above cards. Mouse hover keeps the native `title`. Tapping elsewhere blurs it.

- [ ] **Step 1: Write the failing tests** (render with `svelte/server` as `CauseTree.test.ts` does):
  - `renders CO₂ as a focusable abbr with its name`: body contains `<abbr class="gloss"`, `title="carbon dioxide"`, `tabindex="0"`, and the text `CO₂`.
  - `inert glossed terms are not focusable`: with `inert: true`, body contains `title="carbon dioxide"` and no `tabindex`.
  - `plain text stays plain`: `text: 'no units here'` renders no `<abbr`.
- [ ] **Step 2: Run** `npm test -- Glossed` — expected FAIL.
- [ ] **Step 3: Implement** `Glossed.svelte` and the `.gloss` CSS (dotted underline and `cursor: help` as `.unit-help` today).
- [ ] **Step 4: Run** `npm test -- Glossed` and `npm run check` — PASS.
- [ ] **Step 5: Commit** `feat(web): show the name of a formula or unit on hover and tap`.

### Task 3: Units panel and button

**Files:**
- Create: `web/src/components/UnitsPanel.svelte`, `web/src/components/UnitsPanel.test.ts`
- Modify: `web/src/lib/units.ts` (extend `UNITS`; delete `unitTooltip`), `web/src/lib/units.test.ts`, `web/src/lib/shortcuts.ts` (+ its test), `web/src/components/Viewer.svelte` (top bar at about :2726, help dialog at :2775-2802, `handleKey` at :2324-2330, CSS `.help .units` :3079), `en.ts`/`fr.ts` (`t.units` entries, `t.viewer.units`, `t.viewer.unitsHelp`, `t.shortcuts.units`)

**Interfaces:**
- Consumes: `t.glossary.units` names are short; `t.units[u]` keeps the long `{ name, examples }` for the panel.
- Produces: `export const UNITS` covering every `GlossUnit` that the panel explains (all of Task 1's units except `°C`, which sits in the `K` entry's text); `<UnitsPanel onClose={() => void} />`; shortcut key `i` → action `'units'` (letters f i q v x y z are free; `u` is taken by `moonView`).

- [ ] **Step 1: Write the failing tests**:
  - `units.test.ts`: every `UNITS` entry has non-empty `name` and `examples` in EN and FR; `UNITS` includes `'M⊕', 'R⊕', 'M♃', 'S⊕', 'K', 'W/m²'`.
  - `UnitsPanel.test.ts`: render contains `role="dialog"`, the title `t.units.title`, an `<abbr>` per unit, and no formula (`not.toContain('CO₂')`).
  - shortcuts test: `i` maps to `'units'`, and `shortcutHelp()` lists it.
- [ ] **Step 2: Run** `npm test -- units shortcuts UnitsPanel` — FAIL.
- [ ] **Step 3: Implement**:
  - `t.units` entries for the new units, same style as today's six (name with meaning, then `examples` with familiar values). Required references: 1 S⊕ = the light Earth gets (Mars 0.43, Venus 1.9); 1 M⊕ = Earth (Moon 0.012, Mars 0.107, Neptune 17); 1 M♃ = 318 M⊕; 1 R⊕ = 6 371 km; K: 0 K absolute zero, 255 K Earth without greenhouse, 288 K Earth, 93.7 K Titan, with °C = K − 273.15; W/m²: Earth's sunlight 1 361 W/m² at the top of the air.
  - `UnitsPanel.svelte`: the dialog markup, focus handling and Escape of today's help dialog, moved from `Viewer.svelte`.
  - `Viewer.svelte`: button `{t.viewer.units}` with `title={t.viewer.unitsHelp}` and `aria-expanded`, right after the `?` button; the `?` dialog loses the units `<dl>`; opening one dialog closes the other; global Escape closes whichever is open.
- [ ] **Step 4: Run** `npm test` and `npm run check` — PASS.
- [ ] **Step 5: Commit** `feat(web): open the units reference from its own button`.

### Task 4: Gloss every display text

**Files:**
- Modify: `web/src/components/body/CauseBranch.svelte` (lines rendering `formatCause` at about :29 and :33; `inert` inside `<button class="input">`), `web/src/components/body/{InteriorCard,SurfaceCard,AtmosphereCard,OrbitCard,MapCard,MoonsCard,LegendCard,SystemSummary,BodyHeader}.svelte`, `web/src/components/{SystemCard,StarPane,InfoPanel,GenerateForm,PlanetCard}.svelte`, `web/src/components/form/*` (ModelFields), `Viewer.svelte` epoch label (:2550)
- Test: `web/src/components/body/CauseTree.test.ts` and the existing card tests

**Interfaces:**
- Consumes: `Glossed` (Task 2). `unitTooltip` is gone (Task 3): every `<span class="unit-help" title={unitTooltip(...)}>` becomes `<Glossed text={...} />`.

Rule: every text node that can carry a formula or a unit renders through `<Glossed>`: values from `format.ts`, labels and help from `t.*`, `formatCause` output. Attributes (`title=`, `aria-label=`, `<option>` text) stay plain strings — they cannot hold elements.

- [ ] **Step 1: Write the failing tests**:
  - `CauseTree.test.ts`: a tree whose root is the CO₂ quantity renders `<abbr class="gloss"` with `title="carbon dioxide"`; a cause with a `bar` value renders `title="bar: pressure"`; an input button holds its abbr without `tabindex`.
  - One card test (AtmosphereCard or the nearest existing card test): a pressure row renders the `bar` gloss and no longer the long `examples` text in any `title`.
- [ ] **Step 2: Run** `npm test -- CauseTree Card` — FAIL.
- [ ] **Step 3: Implement** the sweep. Grep check afterwards: `grep -rn "unit-help\|unitTooltip" web/src` returns nothing.
- [ ] **Step 4: Run** `npm test` and `npm run check` — PASS.
- [ ] **Step 5: Commit** `feat(web): explain formulas and units in cards, trees and the form`.

### Task 5: Documentation and visual check

**Files:**
- Modify: `web/README.md` (the paragraph on the `?` panel / units, and the keyboard shortcut table: add `i`), `README.md` only if it lists the shortcuts.

- [ ] **Step 1:** Update the docs: tooltips name formulas and units on hover or tap; the **Units** button (shortcut `i`) opens the reference panel; `?` lists shortcuts.
- [ ] **Step 2:** `npm test`, `npm run check` — PASS.
- [ ] **Step 3:** Visual check by `viewer-visual-check` on a local build of this branch: a planet's Atmosphere card with the cause tree unfolded (hover CO₂ and bar), the Units panel, the `?` panel, and the same card at a 390 px wide touch viewport with a tap on `M⊕`. Report: tooltip text, no clipping of the popover, no layout shift.
- [ ] **Step 4: Commit** `docs(web): document the units panel and glossary tooltips`.
