# System view shared model (PR 0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Draw every planet of a system alike, on one scale and one clock
that do not depend on the selected planet, behind the interfaces that PRs
A and B build on.

**Architecture:** `systemStarView` returns every planet, the selected one
included, as a `SystemPlanet` list plus the existing `Giant` list in the
same order. `StarView.setStar` takes the selected index. With one, it runs
in *system mode*: a selection-independent scale and clock, no separate
world marker, and the selected planet's extras (orbit highlight, apsides,
moons) on its own marker.

**Tech Stack:** Svelte 5, TypeScript, Three.js, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-05-system-view-design.md`
(goals 5, 6, 7; the "Split" section, PR 0).

## Global Constraints

- English in the repo; every user-facing string in `lib/i18n/en.ts` and
  `fr.ts`.
- `npm run check` (svelte-check, warnings fail) and `npm test` pass.
- A lone world's star view (no `selected`) behaves exactly as today.
- Docs in the same change: `web/README.md`, and `docs/system-spec.md`
  where it describes the system view.
- Conventional Commits, ending with the `Co-Authored-By` line.

## Review Focus

- Selecting another planet leaves the camera, the zoom, the orbits, the
  zone and every planet's position exactly where they were.
- A 15-planet system from 0.03 to 40 AU fits on screen, and the inner
  orbits stay apart.
- A system whose planets all lie inside the habitable zone's outer edge
  is drawn linearly, with no compression boundary.
- A click on the selected planet's own marker picks it, as on any other.
- A lone world's star view: same scale, same blue marker, same clock as
  before.

---

### Task 1: The system scale and the planet list

**Files:**

- Modify: `web/src/lib/viewer/starSystem.ts`
- Modify: `web/src/lib/systemStar.ts`
- Test: `web/src/lib/viewer/starSystem.test.ts`, `web/src/lib/systemStar.test.ts`

**Interfaces:**

- Produces:
  - `systemCompressedFrom(star: Pick<StarInfo, 'hz_outer_au'>): number`,
    which is `hz_outer_au * 1.15`;
  - `systemScale(star, giants, span = 6, from = compressedFrom(star))`,
    which keeps today's behaviour when `from` is not given;
  - `SystemPlanet` = `{ id, name, type, typeLabel, radius_km, mass_earth,
    orbit_au, eccentricity, year_days, equilibrium_k, moons, generated }`;
  - `systemStarView(system, planet, names?, typeLabels?)`, which also
    returns `planets: SystemPlanet[]` and `selected: number`, with
    `giants.giants` and `ids` in the same order and including the
    selected planet.
  - `typeLabels` is `Record<string, string>`, keyed by body type.

- [ ] Test `systemStarView` puts the selected planet in `ids`, `giants`
  and `planets` at `selected`, in orbit order (fixture: `['b', 'c']`,
  `selected` 0 for b and 1 for c).
- [ ] Test that `planets[i].moons` counts the bodies whose parent is that
  planet.
- [ ] Test that `systemScale(..., from)` with the same `from` gives the
  same mapping whichever planet's `StarInfo` is passed.
- [ ] Test that all planets inside `from` map linearly: no log part.
- [ ] Implement, run `npm test`, then commit.

### Task 2: System mode in StarView

**Files:**

- Modify: `web/src/lib/viewer/StarView.ts`

**Interfaces:**

- Consumes the Task 1 outputs.
- Produces:
  - `setStar(info, omega, giants, selected: number | null = null)`;
  - public `pickPlanet(clientX, clientY): number | null`;
  - `SystemLabels.giants[]` entries `{ x, y, name, au, index }`, covering
    every planet in system mode;
  - `onClock(yearFraction, days)`, where `days` is the simulated Earth
    days since the start;
  - exported `SYSTEM_INNER_YEAR_S = 10`.

In system mode:

- The scale is `systemScale(info, giants, 6, systemCompressedFrom(info))`.
- The world marker (`this.planet`) is hidden. Its position follows
  `giantMarkers[selected]`, so the moons, the link and the labels sit on
  the selected planet.
- The giants' clock: days per second is
  `min(year_days) / SYSTEM_INNER_YEAR_S`. A planet's mean anomaly is
  `2π·phase + 2π·time·daysPerSecond / year_days`. The selected planet's
  position, `planetAu`, and `onClock` use its own giant entry.
- The time resets only when the planet set changes (a key of the star type
  and every orbit), never when the selection changes.
- The `self` label is null, since the selected planet's giant label
  replaces it.

The lone world mode keeps today's code path.

- [ ] Implement, then run `npm run check`.
- [ ] Commit.

### Task 3: Wire the panel and the viewer

**Files:**

- Modify: `web/src/components/StarPane.svelte`
- Modify: `web/src/components/Viewer.svelte`
- Modify: `web/src/lib/i18n/en.ts`, `web/src/lib/i18n/fr.ts`
- Modify: `web/README.md`, `docs/system-spec.md`

The changes:

- StarPane takes a `selected: number | null` prop and passes it to
  `setStar`.
- The planet list heading reads "Planets" in system mode, and the selected
  planet's line is bold.
- The compression note uses `systemCompressedFrom` in system mode.
- The clock text in system mode is a new
  `t.starView.systemClock({ days, day })`. It reads
  `1 s = {days} days · day {day}`.
- Viewer passes `selected` and `typeLabels` (from `t.viewer.bodyTypes`),
  and builds `kinds` from `planets`.

Steps:

- [ ] Implement, then run `npm run check` and `npm test`.
- [ ] Check in the browser on a multi-planet system:
  - selecting b, then c, then d leaves the orbits, the zone and the zoom
    still;
  - clicking the selected planet's marker opens it.
- [ ] Update the docs and run markdownlint.
- [ ] Commit, push and open the PR.
