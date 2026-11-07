# Rotation Dial Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A rotation dial round the shown body's equator in the planet view: a ring with degree ticks, a pointer reading the body's rotation, sun and host markers, the day figures in the *Light and season* panel and a glossary.

**Architecture:** A pure module (`web/src/lib/rotationDial.ts`) turns the system, the day and the sky already computed (`systemSky`) into the dial's state (ring zero, pointer, markers, all in the body's frame) and its figures. A Three.js module (`web/src/lib/viewer/rotationDial.ts`) draws that state round the globe. `PlanetView` owns the drawing and projects its label anchors; `Viewer.svelte` adds the switch, the figures, the glossary and a light label overlay.

**Tech Stack:** TypeScript (strict), Svelte 5, Three.js, Vitest, in `web/`.

**Spec:** `docs/superpowers/specs/2026-11-06-rotation-dial-design.md` (its "The dial" and later sections; the two "First:" sections shipped in #246, which also exported `libration(orbit, days)` and `Placement.equator` from `lib/systemSky.ts`).

## Global Constraints

- Moving nothing else already drawn; changing no generated data, `system.json` or `SYSTEM_FORMAT_VERSION`; regenerating nothing.
- The dial must agree with the scene: its sun marker, pointer and local time match the lighting and sky drawn at that instant (they are computed from `systemSky`'s output and `placeSystem`, never from separate formulas for positions).
- Globe view of a system body only (`sysBody` set, `mode === 'globe'`); off by default; saved with the display settings.
- Ring radius 1.25 body radii, or 1.25 times the largest radius of an irregular body's shape. A tick every 5°, a longer tick with its number every 30°.
- Colours from the mockups: ring `#9fd3ff`, pointer `#ffcc66`, sun marker `#ffe28a`, host marker `#c89a6a`.
- Angles grow in the body's sense of rotation (right-handed about its spin axis, +z towards +x in the body frame).
- English in code, comments, docs and commits; user strings in en and fr; Conventional Commits with the session's footers; docs in the same change (`web/README.md`); do not run prettier.
- Cheap enough for the light quality profile (lines and a few small meshes; no per-frame allocation).

## Review Focus

- A planet locked to its star: zero and host are both the star (one marker), no solar day (the sun stands still), libration from its own orbit; covered in Task 1.
- The sun along the spin axis (a body seen at a solstice with a large tilt): the sun marker is hidden, not drawn at a degenerate direction; covered in Task 1.
- An irregular body (shape radii): the ring clears its largest radius; covered in Task 3.
- Switching body or leaving the globe while the dial is on: the dial follows the new body or hides; covered in Task 3.
- Local time wraps cleanly past midnight (no negative hours, no 24:00); covered in Task 1.

---

### Task 1: The dial's state and figures

**Files:**
- Create: `web/src/lib/rotationDial.ts`
- Test: `web/src/lib/rotationDial.test.ts` (fixture: `web/src/lib/systemSky.fixture.json`, as `systemSky.test.ts` uses it)

**Interfaces:**
- Produces:

```ts
export interface DialState {
  /** Synchronous: the ring is graduated against the host, the pointer starts from the face turned to it. */
  locked: boolean;
  /** The ring's 0°, a unit vector in the body's equatorial plane (its frame, y the spin axis). */
  zero: Vec3;
  /** Longitude the pointer starts from, radians: 0 for a free body, π for a locked one. */
  pointerLongitude: number;
  /** The angle the pointer reads on the ring, radians: [0, 2π) for a free body, (−π, π] for a locked one. */
  reading: number;
  /** The star's direction projected on the equator (unit), null when within 1° of the spin axis. */
  sun: Vec3 | null;
  /** The planet a moon orbits, projected on the equator (unit), null for a planet. */
  host: Vec3 | null;
}
export interface DialFigures {
  siderealHours: number;
  /** Null when the sun stands still in the body's sky (a planet locked to its star). */
  solarHours: number | null;
  /** Local time at the pointer's meridian, in local hours [0, 24): noon when the sun crosses it. */
  localHours: number;
  /** Synchronous bodies only (null otherwise): the libration now and its amplitude, 2e, radians. */
  libration: number | null;
  librationAmplitude: number | null;
}
export function rotationDial(
  system: System,
  shown: string,
  days: number,
  spinOffset: number,
  sky: { sun: Vec3; bodies: SkyBody[]; ids: string[] },
): { state: DialState; figures: DialFigures } | null;
```

- Free body: `placeSystem(system, days, { id: shown, spinOffset })`; with the shown body's `axes` `[x, y, z]` and `equator` `[ex, , ez]`, `reading = atan2(z·ex, z·ez)` wrapped to [0, 2π); `zero = [−sin reading, 0, cos reading]` (the equinox seen in the body's frame).
- Locked body: `zero` is the host's direction projected on the equator (a moon: `sky.bodies[sky.ids.indexOf(parent)].direction`; a planet: `sky.sun`); `reading` is the signed angle from `zero` to the pointer's direction `[0, 0, −1]` about +y, wrapped to (−π, π].
- `localHours = 12 + 24 · (pointerLongitude − atan2(sun.x, sun.z)) / 2π`, wrapped to [0, 24), using the unprojected `sky.sun`.
- `siderealHours` = `spin.period_hours`, or `orbit.period_days · 24` when locked. The year is the body's orbital period round the star (its planet's for a moon), in hours; `solarHours = 1 / (1/sidereal − 1/year)`, null for a planet locked to its star or when `|1/sidereal − 1/year| < 1e-9`.
- `libration` from `libration(orbit, days)` (`lib/systemSky.ts`); `librationAmplitude = 2 · eccentricity`.
- Returns null when `shown` is not in the system.

- [ ] **Step 1: Write the failing tests**

```ts
it('reads a free body’s turn against the stars, 0° at its equinox', () => {
  const day = body('b').spin.period_hours / 24;
  const at = (d: number) => rotationDial(system, 'b', d, 0, systemSky(system, 'b', d, textures))!.state;
  expect(at(0).reading).toBeCloseTo(0, 6);
  expect(at(day / 4).reading).toBeCloseTo(Math.PI / 2, 6);
  const s = at(day / 4);
  [-1, 0, 0].forEach((v, i) => expect(s.zero[i]).toBeCloseTo(v, 6));
  expect(s.pointerLongitude).toBe(0);
  expect(s.locked).toBe(false);
});

it('graduates a locked moon’s ring against its giant, the pointer swinging by the libration', () => {
  const eccentric = structuredClone(system);
  const orbit = eccentric.bodies.find((x) => x.id === 'c-1')!.orbit;
  orbit.eccentricity = 0.2;
  const d = orbit.period_days / 8;
  const dial = rotationDial(eccentric, 'c-1', d, 0, systemSky(eccentric, 'c-1', d, textures))!;
  expect(dial.state.locked).toBe(true);
  expect(dial.state.pointerLongitude).toBeCloseTo(Math.PI, 12);
  expect(Math.abs(dial.state.reading)).toBeCloseTo(Math.abs(libration(orbit, d)), 6);
  expect(dial.figures.libration).toBeCloseTo(libration(orbit, d), 12);
  expect(dial.figures.librationAmplitude).toBeCloseTo(0.4, 12);
  expect(dial.state.host).not.toBeNull();
});

it('gives the local time at the pointer’s meridian, noon under the sun, wrapping past midnight', () => {
  // Over a whole solar day of b the local time runs once through 0..24, never 24.
  const { solarHours } = rotationDial(system, 'b', 0, 0, systemSky(system, 'b', 0, textures))!.figures;
  const hours = Array.from({ length: 48 }, (_, k) => {
    const d = ((k / 48) * solarHours!) / 24;
    return rotationDial(system, 'b', d, 0, systemSky(system, 'b', d, textures))!.figures.localHours;
  });
  hours.forEach((h) => { expect(h).toBeGreaterThanOrEqual(0); expect(h).toBeLessThan(24); });
  // Noon: the sun at the pointer's longitude.
  const noon = hours.findIndex((h, k) => k > 0 && hours[k - 1]! < 12 && h >= 12);
  expect(noon).toBeGreaterThan(-1);
});

it('gives the sidereal and solar days, a moon taking its planet’s year', () => {
  const b = rotationDial(system, 'b', 0, 0, systemSky(system, 'b', 0, textures))!.figures;
  expect(b.siderealHours).toBeCloseTo(body('b').spin.period_hours, 9);
  expect(b.solarHours).toBeCloseTo(1 / (1 / body('b').spin.period_hours - 1 / (body('b').orbit.period_days * 24)), 6);
  const m = rotationDial(system, 'c-1', 0, 0, systemSky(system, 'c-1', 0, textures))!.figures;
  expect(m.siderealHours).toBeCloseTo(body('c-1').orbit.period_days * 24, 9);
  expect(m.solarHours).toBeCloseTo(1 / (1 / m.siderealHours - 1 / (body('c').orbit.period_days * 24)), 6);
});

it('treats a planet locked to its star: one marker, the sun standing still', () => {
  const locked = structuredClone(system);
  const b = locked.bodies.find((x) => x.id === 'b')!;
  b.spin.locked = true;
  b.spin.axial_tilt_deg = 0;
  const dial = rotationDial(locked, 'b', 10, 0, systemSky(locked, 'b', 10, textures))!;
  expect(dial.state.host).toBeNull();
  expect(dial.figures.solarHours).toBeNull();
  expect(Math.abs(dial.state.reading)).toBeCloseTo(Math.abs(libration(b.orbit, 10)), 6);
});

it('hides the sun marker when the sun is along the spin axis', () => {
  const sky = { ...systemSky(system, 'b', 0, textures), sun: [0, 1, 0] as Vec3 };
  expect(rotationDial(system, 'b', 0, 0, sky)!.state.sun).toBeNull();
});
```

(`body(id)` is a one-line helper finding a body of the fixture; `textures` and `system` as in `systemSky.test.ts`.)

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npx vitest run src/lib/rotationDial.test.ts`
Expected: FAIL, the module does not exist.

- [ ] **Step 3: Implement `rotationDial` in `web/src/lib/rotationDial.ts` as specified in Interfaces**

- [ ] **Step 4: Run the tests, the suite and the type check**

Run: `cd web && npx vitest run src/lib/rotationDial.test.ts && npm test && npm run check`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/rotationDial.ts web/src/lib/rotationDial.test.ts docs/superpowers/plans/2026-11-07-rotation-dial.md
git commit -m "feat(web): compute a rotation dial's state and day figures"
```

### Task 2: Drawing the dial

**Files:**
- Create: `web/src/lib/viewer/rotationDial.ts`
- Test: `web/src/lib/viewer/rotationDial.test.ts`

**Interfaces:**
- Consumes: `DialState` (Task 1).
- Produces:

```ts
export const DIAL_RADIUS = 1.25;
export class RotationDial {
  /** Added to the scene once; in the body's frame. */
  readonly group: Group;
  /** Places the ring (0° at state.zero), the pointer and the markers; `radius` in body radii. */
  set(state: DialState, radius: number): void;
  /** Shows or hides it, faded with the globe's morph (opacity = fade³, as bodyGuides). */
  update(on: boolean, fade: number): void;
  /** Points to label, in the body's frame: the 12 numbered ticks (0°, 30°, … 330°, in order), the pointer's tip, the sun and host markers (null when absent). */
  anchors(): { ticks: Vector3[]; pointer: Vector3; sun: Vector3 | null; host: Vector3 | null };
  dispose(): void;
}
```

- Ring: a circle of radius `radius` and a fainter one at `1.1 · radius` (`LineLoop`), 72 ticks from `radius` outwards (`0.05 · radius` long, `0.1 · radius` every 30°) as one `LineSegments`, all in the equatorial plane (y = 0). Tick k's direction is `zero` turned by `k · 5°` about +y, right-handed.
- Pointer: a thin tube (as `bodyGuides`' `SPIN_TUBE`) on the equator at `pointerLongitude`, from radius 1 to `radius`, with a small cone at its tip.
- Markers: small spheres (`0.04 · radius`) on the ring at `sun` and `host`.
- Materials `MeshBasicMaterial` / `LineBasicMaterial`, transparent, `depthWrite: false`, `renderOrder` 2 (as the guides), colours from the Global Constraints.

- [ ] **Step 1: Write the failing tests**

```ts
it('lays 72 ticks, 12 of them long, with the 0° tick along the state’s zero', () => {
  const dial = new RotationDial();
  dial.set({ locked: false, zero: [1, 0, 0], pointerLongitude: 0, reading: Math.PI / 2, sun: null, host: null }, 1.25);
  const { ticks } = dial.anchors();
  expect(ticks).toHaveLength(12);
  expect(ticks[0]!.clone().normalize().x).toBeCloseTo(1, 9);
  // 90° on: turned right-handed about +y from +x, towards −z.
  expect(ticks[3]!.clone().normalize().z).toBeCloseTo(-1, 9);
  ticks.forEach((t) => expect(t.y).toBeCloseTo(0, 12));
});

it('runs the pointer to the ring at its longitude and puts the markers on the ring', () => {
  const dial = new RotationDial();
  dial.set({ locked: true, zero: [0, 0, -1], pointerLongitude: Math.PI, reading: 0, sun: [1, 0, 0], host: [0, 0, -1] }, 2);
  const { pointer, sun, host } = dial.anchors();
  [0, 0, -2].forEach((v, i) => expect(pointer.getComponent(i)).toBeCloseTo(v, 6));
  expect(sun!.length()).toBeCloseTo(2, 6);
  expect(host!.length()).toBeCloseTo(2, 6);
});

it('hides and fades with the globe', () => {
  const dial = new RotationDial();
  dial.update(false, 1);
  expect(dial.group.visible).toBe(false);
  dial.update(true, 0.5);
  expect(dial.group.visible).toBe(true);
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npx vitest run src/lib/viewer/rotationDial.test.ts`
Expected: FAIL, the module does not exist.

- [ ] **Step 3: Implement `RotationDial`** (geometry built once in the constructor, reoriented by `set`; no allocation in `update`).

- [ ] **Step 4: Run the tests, the suite and the type check**

Run: `cd web && npx vitest run src/lib/viewer/rotationDial.test.ts && npm test && npm run check`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/viewer/rotationDial.ts web/src/lib/viewer/rotationDial.test.ts
git commit -m "feat(web): draw a rotation dial round the globe"
```

### Task 3: The dial in the planet view

**Files:**
- Modify: `web/src/lib/viewer/PlanetView.ts` (own a `RotationDial`; `setRotationDial`; its fade and size in `render`; its anchors in `skyScreenPositions`; dispose)
- Modify: `web/src/components/Viewer.svelte` (the switch, its saved setting, computing and passing the state)
- Modify: `web/src/lib/icons.ts` (a `dial` icon: a circle with ticks and a radius line, 24×24 stroke path like the others)
- Modify: `web/src/lib/i18n/en.ts`, `web/src/lib/i18n/fr.ts` (switch label and help)

**Interfaces:**
- Consumes: `rotationDial` (Task 1), `RotationDial`, `DIAL_RADIUS` (Task 2).
- Produces:
  - `PlanetView.setRotationDial(state: DialState | null): void` (null hides it).
  - `skyScreenPositions()` gains `dial: { ticks: ({ x: number; y: number } | null)[]; pointer: …; sun: …; host: … } | null`, each point projected with the globe's occlusion as the guides are (`near`), null when the dial is hidden.
  - In `Viewer.svelte`: setting `rotationDial: false` in the saved display settings; state `dialShown`; `const dial = $derived(dialShown && sys && sysBody && sysSky ? rotationDial(sys, sysBody.id, sysDays, skySpin, sysSky) : null)`; `$effect(() => view?.setRotationDial(dial?.state ?? null))`. The switch joins the sky group of `toggleGroups` next to `axes`: `{ key: 'dial', label: t.viewer.rotationDial, help: t.viewer.rotationDialHelp, icon: 'dial', on: dialShown, set: (v) => (dialShown = v) }`.
- Strings: en `rotationDial: 'Rotation dial'`, `rotationDialHelp: 'A ring round the equator, graduated in degrees: its pointer shows how far the body has turned, against the stars, or against what it orbits for a body locked to it'`; fr `rotationDial: 'Boussole de rotation'`, `rotationDialHelp: 'Un anneau autour de l’équateur, gradué en degrés : son trait montre de combien l’astre a tourné, par rapport aux étoiles, ou par rapport à ce qu’il orbite pour un astre en rotation synchrone'` (with the file's `${S}` narrow spaces where fr.ts uses them).

- [ ] **Step 1: Wire it** — the ring radius is `DIAL_RADIUS` times the largest radius of the loaded shape (`this.shapeRadii`, its `radii`), else `DIAL_RADIUS`; the dial is drawn in globe mode only and fades with `this.morph` like the guides; hidden when `setRotationDial(null)`.

- [ ] **Step 2: Check it**

Run: `cd web && npm test && npm run check`
Expected: all pass.

Then in the browser (dev server proxying the production API, `mgs-82899-667`): on planet `d` with the switch on, the ring sits round the equator clear of the globe, its 0° fixed against the stars while playing; on moon `d-1` (camera turning with it), the ring stays still with 0° at the giant; switching to `d-4` moves the dial with it; the map view hides it.

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/viewer/PlanetView.ts web/src/components/Viewer.svelte web/src/lib/icons.ts web/src/lib/i18n/en.ts web/src/lib/i18n/fr.ts
git commit -m "feat(web): show a rotation dial round the shown body"
```

### Task 4: Labels, figures and glossary

**Files:**
- Create: `web/src/components/DialLabels.svelte` (absolutely placed small labels, no leader lines: the 12 numbers, the pointer's reading in a boxed label, "Sun" and the host's name by their markers)
- Modify: `web/src/components/Viewer.svelte` (the overlay; the figures and the `?` glossary in the `lightPanel` snippet while the dial is on)
- Modify: `web/src/lib/i18n/en.ts`, `web/src/lib/i18n/fr.ts`
- Modify: `web/README.md` (the dial, the figures and the glossary text in the planet view's section)

**Interfaces:**
- Consumes: `skyScreenPositions().dial` (Task 3), `dial.figures` and `dial.state` (Task 1).
- Labels: numbers `0°` … `330°` (locked: the same numbers, 0° at the host); pointer label: free `137°` (reading in whole degrees), locked signed with one decimal (`+0.3°`).
- Figures, en (fr alongside): free — `Rotation 137° · Local time 14:20 (meridian 0)` / `Sidereal day 10 h 53 min · Solar day 10 h 54 min`; locked — `Synchronous rotation · Libration +0.3° (±0.4°)` / `Local time 18:05 (face to the host) · Solar day 2.49 d` or, for a planet locked to its star, `Solar day: none, the star stands still`. Local hours as `HH:MM` of the local day; durations under 48 h as `h`/`min`, else days with two decimals.
- Glossary, toggled by a `?` button beside the figures, en (fr given below):
  - Sidereal day: one turn of the body against the stars (Earth: 23 h 56 min).
  - Solar day: from one noon to the next, the day lived there (Earth: 24 h). It differs from the sidereal day because the body moves along its orbit as it turns.
  - Sol: the solar day of Mars (24 h 39 min 35 s), a name kept for Mars; elsewhere, "solar day".
  - Local time: the solar day cut into 24 local hours, noon when the sun crosses the meridian (the convention of Mars missions).
  - Synchronous rotation: the body turns once per orbit and always shows the same face to what it orbits.
  - Libration: on a slightly elliptical orbit the host swings in the sky of a synchronous body, by about ±2e radians.
  - Longitude convention: in this app a synchronous body's face to its host is longitude 180°; the scientific convention puts longitude 0 there.
- fr glossary: Jour sidéral : un tour de l’astre par rapport aux étoiles (Terre : 23 h 56 min). · Jour solaire : d’un midi au suivant, le jour vécu sur place (Terre : 24 h). Il diffère du jour sidéral car l’astre avance sur son orbite en tournant. · Sol : le jour solaire de Mars (24 h 39 min 35 s), nom réservé à Mars ; ailleurs, « jour solaire ». · Heure locale : le jour solaire découpé en 24 heures locales, midi quand le soleil passe au méridien (convention des missions martiennes). · Rotation synchrone : l’astre fait un tour par orbite et montre toujours la même face à ce qu’il orbite. · Libration : sur une orbite un peu elliptique, l’hôte oscille dans le ciel du corps synchrone, d’environ ±2e radians. · Convention de longitude : ici, la face d’un astre synchrone tournée vers son hôte est la longitude 180° ; la convention scientifique y place la longitude 0.

- [ ] **Step 1: Write a failing test for the figure formatting** in `web/src/lib/rotationDial.test.ts`: `formatLocalTime(14.3333)` is `'14:20'`, `formatLocalTime(23.9999)` is `'23:59'` (minutes floored, never `'24:00'`), `formatDayLength(10.883)` is `'10 h 53 min'`, `formatDayLength(59.82)` is `'2.49 d'` — the two helpers exported from `lib/rotationDial.ts`, unit words taken as parameters (`{ hour: 'h', minute: 'min', day: 'd' }`) so that the fr strings reuse them.

- [ ] **Step 2: Run it to verify it fails**

Run: `cd web && npx vitest run src/lib/rotationDial.test.ts`
Expected: FAIL, the helpers do not exist.

- [ ] **Step 3: Implement the helpers, the overlay, the figures, the glossary and the README text**

- [ ] **Step 4: Check it**

Run: `cd web && npm test && npm run check`
Expected: all pass. In the browser, as in Task 3: the numbers ride the ring and hide behind the globe; the pointer's label reads the angle; the panel shows the figures; `?` opens and closes the glossary, in both languages.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/DialLabels.svelte web/src/components/Viewer.svelte web/src/lib/rotationDial.ts web/src/lib/rotationDial.test.ts web/src/lib/i18n/en.ts web/src/lib/i18n/fr.ts web/README.md
git commit -m "feat(web): label the rotation dial and explain its figures"
```
