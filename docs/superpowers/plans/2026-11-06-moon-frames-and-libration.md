# Moon Frames and Libration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Place moons' orbits in their planet's equatorial frame held fixed against the stars, and turn synchronous bodies at a steady pace so that their host librates in their sky.

**Architecture:** All in `placeSystem` (`web/src/lib/systemSky.ts`). Each `Placement` gains its non-turning equatorial frame (`equator`), which its moons' orbit planes are built from instead of its spinning `axes`. A locked body faces the point its host would be at on a circular orbit at the same mean anomaly; a new exported `libration(orbit, days)` gives true minus mean anomaly, for tests now and the dial later. The star view keeps its own simplified moon model (`StarView.placeSystemMoons`, phase plus time times speed) and does not share the defect: it is not touched.

**Tech Stack:** TypeScript (strict), Vitest, in `web/`.

**Spec:** `docs/superpowers/specs/2026-11-06-rotation-dial-design.md`, sections "First: moons' orbits fixed against the stars" and "First: a steady spin for synchronous bodies".

## Global Constraints

- No change to the generated data, `system.json`, or `SYSTEM_FORMAT_VERSION`; regenerate nothing.
- Planets' placements (orbits, axes, spin) are unchanged: only moons and locked bodies on eccentric orbits move.
- English in code, comments, docs and commit messages; Conventional Commits with the session's footers.
- Docs ship with the change: the head comment of `systemSky.ts`, `web/README.md` and `docs/system-spec.md` where they describe how moons and locked bodies are placed.
- Do not run prettier.

## Review Focus

- A moon of a planet with an axial tilt: its orbit stays in the planet's tilted equator (c is tilted 7.8° in the fixture); covered in Task 1.
- A retrograde moon (inclination over 90°): its orbit normal points against the planet's axis and its locked face still tracks its host; covered in Task 2.
- A planet locked to its star (no parent): it librates with its own eccentricity round the star; covered in Task 2.
- A circular orbit (e = 0): the locked face points exactly at the host, as before; covered in Task 2.
- `spinOffset` (the Sun longitude slider) still turns free bodies only, never locked ones; existing behaviour, kept by Task 2's tests.

---

### Task 1: Moons' orbits in their planet's fixed equatorial frame

**Files:**
- Modify: `web/src/lib/systemSky.ts` (`Placement`, `placeSystem`, head comment)
- Test: `web/src/lib/systemSky.test.ts`
- Docs: `web/README.md`, `docs/system-spec.md` (the sentences placing a moon "in the planet's equatorial plane": state that the plane and its node are fixed against the stars, the node's longitude counted from the planet's equinox direction)

**Interfaces:**
- Produces: `Placement.equator: Axes`, the body's equatorial frame not turning with its day: `[cross(y, start), y, start]`, `y` its spin axis and `start` its equinox direction (its orbit's node line projected on its equator, the direction its longitude 0 faces at day 0 for a free body). Defined for every body, locked ones included.
- `placeSystem` builds a body's orbit plane with `orbitPlane(body.orbit, parentPlace?.equator ?? IDENTITY_AXES)`.

- [ ] **Step 1: Write the failing tests**

```ts
it('keeps a moon’s orbit fixed against the stars while its planet turns', () => {
  const day = system.bodies.find((x) => x.id === 'c')!.spin.period_hours / 24;
  const period = system.bodies.find((x) => x.id === 'c-1')!.orbit.period_days;
  const from = (days: number) => {
    const p = placeSystem(system, days);
    const v = [0, 1, 2].map((i) => p.get('c-1')!.centre[i]! - p.get('c')!.centre[i]!) as Vec3;
    return v;
  };
  const a = from(0), b = from(day / 4);
  const angle = Math.acos((a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / (length(a) * length(b))) / (Math.PI / 180);
  // Only its own orbit moves it: about 15.6°, not the 106° it moved when its plane turned with c
  // (within 1.5°: its orbit is slightly eccentric).
  expect(Math.abs(angle - (360 * (day / 4)) / period)).toBeLessThan(1.5);
});

it('lays a moon’s orbit in its planet’s tilted equator, still against the stars', () => {
  const normal = (days: number) => placeSystem(system, days).get('c-1')!.plane.normal;
  const axis = placeSystem(system, 0).get('c')!.axes[1];
  // c-1 is inclined 0.15° to c's equator, which c's 7.8° tilt leans.
  expect(normal(0)[0] * axis[0] + normal(0)[1] * axis[1] + normal(0)[2] * axis[2]).toBeGreaterThan(Math.cos((0.2 * Math.PI) / 180));
  normal(0).forEach((v, i) => expect(v).toBeCloseTo(normal(0.3)[i]!, 9));
});

it('brings the sun round a locked moon once per orbit', () => {
  const period = system.bodies.find((x) => x.id === 'c-1')!.orbit.period_days;
  const lon = (days: number) => { const s = systemSky(system, 'c-1', days, textures).sun; return Math.atan2(s[0], s[2]); };
  let turned = (lon(period / 4) - lon(0)) / (Math.PI / 180);
  turned = ((turned % 360) + 540) % 360 - 180;
  // A quarter turn in a quarter orbit (within 2°: eccentricity and the planet's year).
  expect(Math.abs(Math.abs(turned) - 90)).toBeLessThan(2);
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npx vitest run src/lib/systemSky.test.ts`
Expected: the three new tests FAIL (angle about 106°; normal turning; the sun far past 90°).

- [ ] **Step 3: Add `Placement.equator` and build moons' orbit planes from the parent's `equator` in `placeSystem`**

Compute `start` for every body (the free body's formula, now outside the `else`), keep `axes` as they are, and store `equator`. Update the head comment of `systemSky.ts` ("a moon's round its planet, in the planet's equatorial plane" becomes fixed against the stars, the node counted from the planet's equinox direction).

- [ ] **Step 4: Run the whole file**

Run: `cd web && npx vitest run src/lib/systemSky.test.ts`
Expected: PASS, the existing tests included. If 'keeps a locked moon facing its giant' fails here, stop and report: it should only change in Task 2.

- [ ] **Step 5: Run every test and the type check**

Run: `cd web && npm test && npm run check`
Expected: all pass. Any test that pinned a moon's old position (ring rocks, sky labels) is updated to the new expectation with a comment saying why it moved.

- [ ] **Step 6: Update the docs and commit**

```bash
git add web/src/lib/systemSky.ts web/src/lib/systemSky.test.ts web/README.md docs/system-spec.md docs/superpowers/specs/2026-11-06-rotation-dial-design.md docs/superpowers/plans/2026-11-06-moon-frames-and-libration.md
git commit -m "fix(web): hold moons' orbits fixed against the stars"
```

### Task 2: A steady spin for synchronous bodies

**Files:**
- Modify: `web/src/lib/systemSky.ts` (`orbitPosition` split, `placeSystem` locked branch, new `libration`)
- Test: `web/src/lib/systemSky.test.ts`
- Docs: `web/README.md`, `docs/system-spec.md`, head comment of `systemSky.ts` (a locked body turns at a steady pace, facing its host on average; its host swings by the libration)

**Interfaces:**
- Consumes: `Placement.equator` (Task 1).
- Produces: `export function libration(orbit: Orbit, days: number): number`, radians in (−π, π]: the true anomaly minus the mean anomaly at `days` (Kepler's equation as in `orbitPosition`, eccentricity clamped the same way). Positive when the host is ahead of the face's mean direction in the sense of the orbit.
- A locked body's `z` is the unit vector from its parent towards the point of its orbit at the mean anomaly, `pointOnOrbit(plane, periapsis + mean, 1)`, projected on its equator: its longitude 180° faces that mean point, and the host's real direction is turned from it by `libration`.

- [ ] **Step 1: Write the failing tests**

```ts
it('turns a locked body steadily, its host swinging by the libration', () => {
  const eccentric = structuredClone(system);
  const moon = eccentric.bodies.find((x) => x.id === 'c-1')!;
  moon.orbit.eccentricity = 0.2;
  const period = moon.orbit.period_days;
  for (const days of [0, period / 8, period / 3]) {
    const sky = systemSky(eccentric, 'c-1', days, textures);
    const d = sky.bodies[sky.ids.indexOf('c')]!.direction;
    // The giant's longitude from the face turned to it (−z), in the sense of the orbit.
    const swing = Math.atan2(-d[0], -d[2]);
    expect(Math.abs(swing)).toBeCloseTo(Math.abs(libration(moon.orbit, days)), 6);
    expect(Math.abs(swing)).toBeLessThan(2.5 * 0.2);
  }
});

it('faces its host exactly on a circular orbit, and at periapsis', () => {
  const giantFrom = (s: System, days: number) => { const sky = systemSky(s, 'c-1', days, textures); return sky.bodies[sky.ids.indexOf('c')]!.direction; };
  const circular = structuredClone(system);
  circular.bodies.find((x) => x.id === 'c-1')!.orbit.eccentricity = 0;
  giantFrom(circular, 1.3).forEach((v, i) => expect(v).toBeCloseTo([0, 0, -1][i]!, 6));
  const eccentric = structuredClone(system);
  const orbit = eccentric.bodies.find((x) => x.id === 'c-1')!.orbit;
  orbit.eccentricity = 0.2;
  // The first periapsis after the epoch: mean anomaly 0.
  const periapsis = (((360 - orbit.mean_anomaly_deg) % 360) / 360) * orbit.period_days;
  expect(libration(orbit, periapsis)).toBeCloseTo(0, 6);
  giantFrom(eccentric, periapsis).forEach((v, i) => expect(v).toBeCloseTo([0, 0, -1][i]!, 4));
});

it('librates a retrograde moon and a planet locked to its star too', () => {
  const odd = structuredClone(system);
  odd.bodies.find((x) => x.id === 'c-2')!.orbit.inclination_deg = 150;
  odd.bodies.find((x) => x.id === 'c-2')!.orbit.eccentricity = 0.1;
  const b = odd.bodies.find((x) => x.id === 'b')!;
  b.spin.locked = true;
  for (const [id, host] of [['c-2', 'c'], ['b', null]] as const) {
    const body = odd.bodies.find((x) => x.id === id)!;
    const days = body.orbit.period_days / 5;
    const sky = systemSky(odd, id, days, textures);
    const d = host ? sky.bodies[sky.ids.indexOf(host)]!.direction : sky.sun;
    expect(Math.abs(Math.atan2(-d[0], -d[2]))).toBeCloseTo(Math.abs(libration(body.orbit, days)), 6);
  }
});
```

Update the existing 'keeps a locked moon facing its giant' test: the giant's direction is within `2.5 * e` radians of `[0, 0, -1]` (e = 0.0085 for c-1) instead of equal to six decimals.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd web && npx vitest run src/lib/systemSky.test.ts`
Expected: FAIL, `libration` is not exported.

- [ ] **Step 3: Implement**

Split the mean anomaly and Kepler's solution out of `orbitPosition` (`function anomalies(orbit, days): { mean: number; true: number; eccentric: number }`, private), use it in `orbitPosition` and in `libration`, and set the locked branch's `z` as in Interfaces. Free bodies and `spinOffset` are unchanged.

- [ ] **Step 4: Run the file, then everything**

Run: `cd web && npx vitest run src/lib/systemSky.test.ts && npm test && npm run check`
Expected: all pass.

- [ ] **Step 5: Check in the browser**

On `mgs-82899-667`, show `d-1` and `d-4` under Sunlight and play: the giant stays in place in the moon's sky (d-1's eccentricity is 3e-5); from the giant `d`, its moons now rise and set once per day. Take one screenshot of each and keep them for the PR.

- [ ] **Step 6: Update the docs and commit**

```bash
git add web/src/lib/systemSky.ts web/src/lib/systemSky.test.ts web/README.md docs/system-spec.md
git commit -m "feat(web): turn synchronous bodies steadily so their host librates"
```
