# Living Star Close-up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Up close, the star view looks like a high-resolution image of the Sun:
- granulation as moving bright cells with dark lanes;
- the supergranule network seen from afar;
- irregular, filamentary spots that change shape over their life;
- no visible frame drop.

**Architecture:** Everything stays in the shared surface chunk `STAR_SURFACE_GLSL` (`web/src/lib/viewer/starSurface.ts`), gated by size on screen as in part 1.
- The simplex granulation is replaced by moving cellular noise.
- A network term is added.
- `starSpot` becomes a shaped spot that reads a per-region seed from a new uniform array.
- The star view draws at the profile's moving pixel ratio while its clock runs, and at the full ratio when paused.

**Tech Stack:** TypeScript (strict), Svelte 5, Three.js 0.187 (GLSL ES 3.0 shaders), Vitest.

**Spec:** `docs/superpowers/specs/2026-11-09-living-star-closeup-design.md`. The look to match is prototype variant D, in `docs/superpowers/specs/2026-11-09-living-star-closeup-prototype.html`.

## Global Constraints

- **Branches and PRs.** Branch per PR from `main`: PR 1 `feat/living-star-closeup` (Tasks 1–4), PR 2 `feat/living-star-spots` (Tasks 5–6). Each PR description says `Refs #391`.
- **Commits.** Conventional Commits, with the footers `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01NRxWVd48PkMKmKJPLqbXxK`.
- **No backend change.** No new dependency, no Rust change, no change to the stored format or `SYSTEM_FORMAT_VERSION`.
- **Determinism.** The surface at a day is a function of `StarActivity`, the day and the view (size on screen, pixel ratio) only. Part 1's "same pixels at the same day" checks must keep passing.
- **One clock.** Phases are reduced in JS before they reach GLSL. Every periodic motion in the shader must be periodic in the phase's wrap: the granule phase wraps at 289 lifetimes (`GRANULE_PHASE_PERIOD`). So an angular rate ω must satisfy ω · 289 = 2π · k, for an integer k.
- **Frame-time bounds.**
  - System and planet views: each median stays within 2× the baseline run-to-run spread.
  - Star view: p95 ≤ 16.7 ms at 2560×1440, measured at device pixel ratio 1 and on a ratio-2 screen, each at the default distance, about 1.1 R and 1.02 R.
  - If the look and the bound conflict, the frame time wins. Cut in this order: cheaper noise, a lower moving ratio at close range (down to 1.0), then weaker detail. Record each cut in the PR.
- **Spot area.** The darkened area drawn for a region stays within ±20 % of part 1's at the same day. This margin is a **calibration**.
- **Sources.** Every science-derived number cites its source in a doc comment and in `docs/science/star-activity.md`. Every invented number is labelled **calibration** in both places.
- **Sky disc.** It keeps `STAR_NO_GRANULATION`, and its detail is gated by `hostStarDetail` as today.
- **Web checks.** From `web/`, run `npm run check` (0 errors, 0 warnings) and `npx vitest run`. Never run `npm ci`, never run prettier, never use port 3000.
- **Shader checks.** GLSL cannot run under Vitest. Shader behaviour is checked by source-string tests in Vitest plus the real-GPU checks named in each task. The controller runs those checks with the `viewer-visual-check` agent, on a local server, never on planets.localhost.

## Review Focus

- **Granule phase wrap.** Every 289 granule lifetimes (about 1.6 days for the Sun) the phase wraps. The surface must not jump at that frame. Task 3 pins it.
- **Clock jumps and reverse play.** Scrubbing the time bar, or playing backwards, must give the same cells and the same spot shapes at a day. Tasks 3 and 5 pin it through the existing same-day pixel diff, extended to 1.02 R.
- **Very small or very young regions** (area near 0 at birth and death). They must not draw speckle, NaN or a ring artefact. Task 5 pins it with ages 0.01 and 0.99 of the lifetime.
- **A star with no activity, or a radiative one.** It keeps a plain limb-darkened disc: there are no granules where `granule_contrast` is 0, and the network term must also be 0 there. Task 4 pins it.
- **Pausing and resuming the star view.** The pixel ratio must switch without a black or stretched frame, and the hidden system view must stay idle. Task 2 pins it.

---

### Task 1: Baseline measurement

**Files:** none in the repo. The results go to the PR 1 description and to a comment on #391.

- [ ] **Step 1: Measure on `main`**, with the protocol of `web/README.md` § "Measuring frame time".
  - Systems: G2V seed 4242, K5V seed 4243, M4V seed 4244.
  - Views: the system view; the planet view; the star view at the default distance, at about 1.1 R and at 1.02 R.
  - Window 2560×1440, at device pixel ratio 1 (GPU Chromium) and on a ratio-2 screen; 1920×1080 at ratio 1.
  - Three runs per case. Record median, p95 and spread.
- [ ] **Step 2: Post the table** as a comment on #391, under the title "Close-up baseline".

### Task 2: Star view pixel ratio while animating

**Files:**
- Modify: `web/src/lib/viewer/pixelRatio.ts`, `web/src/lib/viewer/StarBodyView.ts` (constructor `setPixelRatio` at about line 128, `setAnimated`, the frame loop)
- Test: `web/src/lib/viewer/pixelRatio.test.ts` (create it if it does not exist)
- Docs: `web/README.md`, the star-view bullet: it renders softer while animated and sharp when paused.

**Interfaces:**
- Produces: `export function animatedPixelRatio(deviceRatio: number, animated: boolean, caps?: PixelRatioCaps): number`. It returns `min(full, caps.movingPixelRatio)` when animated, else `fullPixelRatio(deviceRatio, caps)`.

- [ ] **Step 1: Write the failing tests**

```ts
it('draws an animated star at the moving ratio', () => {
  expect(animatedPixelRatio(2, true, { maxPixelRatio: 2, movingPixelRatio: 1.25 })).toBe(1.25);
  expect(animatedPixelRatio(2, false, { maxPixelRatio: 2, movingPixelRatio: 1.25 })).toBe(2);
  expect(animatedPixelRatio(1, true, { maxPixelRatio: 2, movingPixelRatio: 1.25 })).toBe(1);
});
```

- [ ] **Step 2: Run it and see it fail**: `cd web && npx vitest run src/lib/viewer/pixelRatio.test.ts`.
- [ ] **Step 3: Implement it.**
  - StarBodyView calls `renderer.setPixelRatio(animatedPixelRatio(window.devicePixelRatio, animated, quality))` when `setAnimated` changes the state, followed by `setSize` and a render, so the paused frame is drawn sharp at once. It does not call it per frame.
  - Camera motion keeps using the existing moving rule.
- [ ] **Step 4: Run the tests, `npm run check` and the full `npx vitest run`.**
- [ ] **Step 5: GPU check (controller).** Pause and resume the star view 5 times. Each switch must show no black, stretched or blank frame. The system view's 'system' frame count must not grow while the star view shows.
- [ ] **Step 6: Commit**: `feat(web): draw the animated star view at the moving pixel ratio`.

### Task 3: Granulation as moving cells

**Files:**
- Modify: `web/src/lib/viewer/starSurface.ts`. Replace `starGranules` and its simplex helpers. Update the module doc comment and the `STAR_SURFACE_GLSL` doc.
- Test: `web/src/lib/viewer/starSurface.test.ts`
- Docs: `docs/science/star-activity.md` § granulation (cells, lanes, bright points; sources and calibrations).

**Interfaces:**
- Consumes: `uGranule` = (size in star radii, contrast, phase in granule lifetimes reduced mod `GRANULE_PHASE_PERIOD`, fade), unchanged.
- Produces, in GLSL inside `#ifndef STAR_NO_GRANULATION`:
  - `vec2 starCells(vec3 p, float phase)`: F1 and F2 distances to cell centres that wander with `phase`;
  - `float starGranules(vec3 n)`: zero-mean, rms about 1, same name and contract as today, so `starBrightness` is unchanged.

- [ ] **Step 1: Write the failing tests** (source-string tests)

```ts
it('draws granules as cells', () => {
  expect(STAR_SURFACE_GLSL).toMatch(/vec2 starCells\(vec3 p, float phase\)/);
  expect(STAR_SURFACE_GLSL).not.toMatch(/starNoise\(p \+ mod\(step3/); // the simplex cross-fade is gone
});
it('keeps every cell motion periodic in the phase wrap', () => {
  // Each angular rate on the phase is 2π·k/GRANULE_PHASE_PERIOD for an integer k.
  for (const k of granuleMotionHarmonics()) expect(Number.isInteger(k)).toBe(true);
});
```

`granuleMotionHarmonics(): number[]` is a small exported constant list, used both to build the GLSL rates (`2π·k/289` written into the chunk) and in this test.

- [ ] **Step 2: Run it and see it fail.**
- [ ] **Step 3: Implement it.** Follow prototype D's `worley` and live path:
  - cell centres move by `sin(ω·phase + 2π·h)`, with ω taken from `granuleMotionHarmonics`;
  - a domain warp of 2 octaves (`fbm2`);
  - cell brightness `smoothstep(0.02, 0.30, F2 − F1) · (1.15 − 0.55·F1)`;
  - bright points in the lanes.

  Normalise to rms ≈ 1 so that `uGranule.y` (`granule_contrast`) keeps its meaning. Any extra gain needed to match D's look is a single named **calibration** constant. Drop the simplex functions if nothing else uses them.
- [ ] **Step 4: Run the tests, `npm run check` and the full `npx vitest run`.**
- [ ] **Step 5: GPU check (controller)**, G2V star view, at 1.02 R and about 1.1 R:
  - cells with dark lanes, matching prototype D;
  - no visible tiling or grid over a 3×3 pan of the view;
  - the pixel diff at the same day (day 400 → 50 → 400) is 0 px;
  - frames taken just before and after a phase wrap differ no more than two consecutive frames elsewhere;
  - over 10 s at the default star speed, cells visibly deform and split.
- [ ] **Step 6: Commit**: `feat(web): draw the star's granulation as moving cells`.

### Task 4: Supergranule network and mottle

**Files:**
- Modify: `web/src/lib/viewer/starSurface.ts`: a uniform, a GLSL term in `starBrightness`, `StarSurfaceState.update` (its fade), `surfaceDetail` (a `network` field)
- Test: `web/src/lib/viewer/starSurface.test.ts`
- Docs: `docs/science/star-activity.md` § network (size about 30 Mm, Leighton, Noyes & Simon 1962; faculae brighter at the limb; the slight exaggeration labelled **calibration**). Also `web/README.md`, the star-view bullet.

**Interfaces:**
- Produces:
  - `uniform vec4 uNetwork` = (cell size in star radii, contrast, phase in supergranule lifetimes, fade);
  - `surfaceDetail(radiusPx, granuleSize)` returns `{ granulation, regions, network }`, with `network = ramp(radiusPx · networkSize, 3, 8)`;
  - `export const SUPERGRANULE_SIZE_MM = 30` and `SUPERGRANULE_LIFETIME_DAYS = 1`, both with sources;
  - the network contrast is `min(1, granuleContrast / 0.15) · NETWORK_CONTRAST` (**calibration** 0.07). It is 0 when `granule_contrast` is 0, so a radiative star stays plain.

- [ ] **Step 1: Write the failing tests**

```ts
it('shows the network from afar', () => {
  expect(surfaceDetail(500, 0.0019).network).toBe(1);   // whole disc in a 1080p view
  expect(surfaceDetail(20, 0.0019).network).toBe(0);    // a small star: off
});
it('keeps a radiative star plain', () => {
  // the test file's radiative fixture (granule_contrast 0)
  const u = starSurfaceUniforms(8); const s = new StarSurfaceState(u, 8);
  s.setStar(starLook(HOT_INFO, HOT), HOT);
  s.update(100, { radiusPx: 500, daysPerSecond: 1 });
  expect((u.uNetwork!.value as Vector4).y).toBe(0);
});
```

- [ ] **Step 2: Run them and see them fail.**
- [ ] **Step 3: Implement it.**
  - The network term is `contrast · net · (0.35 + 1.4·(1 − μ))`, with `net = 1 − smoothstep(0, 0.10, F2 − F1)` of `starCells(n / size, phase)`.
  - The mottle is `0.5 · contrast · (fbm2(n / (0.35·size)) − 0.5)`.
  - Both sit behind `if (uNetwork.w > 0.0)`.
  - The network is not under `STAR_NO_GRANULATION`, but the sky disc gives it fade 0 below `hostStarDetail`.
- [ ] **Step 4: Run the tests, `npm run check` and the full `npx vitest run`.**
- [ ] **Step 5: GPU check (controller).**
  - The G2V whole disc in the star view and in the system view zoomed: a faint network is visible, stronger towards the limb.
  - A0V: still plain.
  - Re-measure the cases of Task 1 against the Task 1 bounds. If one fails, cut cost as the Global Constraints say, and record the cut.
- [ ] **Step 6: Commit** `feat(web): show the supergranule network on the star`. Then open PR 1 with the Task 1 baseline and an after-table.

### Task 5: Shaped, evolving spots

**Branch:** `feat/living-star-spots`, from `main` after PR 1 merges (or stacked locally until then, opened only after).

**Files:**
- Modify: `web/src/lib/viewer/starSurface.ts`:
  - `starSpot` becomes the shaped spot;
  - a new `uRegionC` array;
  - the region packing in `StarSurfaceState`;
  - the module doc comment.
- Test: `web/src/lib/viewer/starSurface.test.ts`
- Docs: `docs/science/star-activity.md` § spots. It covers:
  - penumbral filaments (Solanki 2003, A&ARv 11, 153);
  - the Evershed flow, about 1–2 km/s outward (Evershed 1909, MNRAS 69, 454);
  - umbral dots;
  - light bridges;
  - pores in the moat;
  - fragmentation in the decline.

  Every shape number is labelled **calibration**.

**Interfaces:**
- Consumes:
  - `ActiveRegion.id` (32-bit hash) from `starActivity.ts`;
  - `RegionNow`;
  - the area triangle rebuilt in the shader from `uRegionB`, unchanged.
- Produces:
  - `uniform vec4 uRegionC[STAR_REGIONS]` = (seed in [0, 1), 0, 0, 0). The seed is `hash32(region.id, 7) / 2³²`, packed with A and B, only on repack.
  - GLSL `float starSpotShaped(vec2 local, float rs, float seed, float age, float aa)`. `local` is in radians on the tangent plane, centred on the spot. `rs` is the spot radius, as today. `age` is (t / lifetime) in [0, 1]. It returns the brightness factor.
- Behaviour, following prototype D's live path, scaled from `RMAX` to `rs`:
  - a torn umbra edge;
  - filaments at about 40 and 90 per turn, drifting outward;
  - a frayed outer edge;
  - umbral dots, computed only inside the umbra;
  - a light bridge for seeds above 0.5 at ages 0.30–0.80;
  - fragmentation from age 0.45 onwards that moves area into pieces and does not delete it;
  - pores in the moat (1.05–2.6 rs), computed only there.

- [ ] **Step 1: Write the failing tests**

```ts
it('packs a seed per region', () => {
  const u = starSurfaceUniforms(32); const s = new StarSurfaceState(u, 32);
  s.setStar(starLook(SUN_INFO, SUN), SUN); s.update(1000, { radiusPx: 4000, daysPerSecond: 0 });
  const c = u.uRegionC!.value as Vector4[];
  s.regions.forEach((r, i) => expect(c[i]!.x).toBeCloseTo(hash32(r.region.id, 7) / 2 ** 32, 6));
});
it('draws shaped spots', () => expect(STAR_SURFACE_GLSL).toMatch(/float starSpotShaped\(vec2 local, float rs, float seed, float age, float aa\)/));
```

- [ ] **Step 2: Run them and see them fail.**
- [ ] **Step 3: Implement it.** Wire `starSpotShaped` for the leader and the follower in `starBrightness`, with the region's seed (the follower uses `fract(seed + 0.5)`) and `age = t / b.z`. Keep the facular plage as today.
- [ ] **Step 4: Run the tests, `npm run check` and the full `npx vitest run`.**
- [ ] **Step 5: GPU check (controller).**
  - **Look:** M4V and G2V at 1.02 R and about 1.1 R, days chosen so that one spot is young, one mature and one declining. Shapes should be compact, then filamentary, then fragmented; the penumbra fibrous with an outward drift; a light bridge on some spots.
  - **Day scrub:** a pixel diff on day 400 → 50 → 400 gives 0 px.
  - **Ages near 0 and 1:** for a region at ages 0.01 and 0.99, no ring, speckle or NaN.
  - **Area:** render isolated fixture regions at ages 0.1, 0.3, 0.5, 0.7 and 0.9. Count the pixels darker than 0.9 of the limb-corrected quiet surface, against part 1's shader at the same ages. Each ratio must lie within 0.8–1.2. Record the table.
  - **Frame time:** re-measure the cases of Task 1.
- [ ] **Step 6: Commit**: `feat(web): shape the star's spots and let them evolve`.

### Task 6: After-measurement and closing docs

**Files:**
- Modify: `web/README.md` § "Measuring frame time" (the outcome line, a link to the PR), `docs/science/star-activity.md` ("Where it departs from the science": the new calibrations), `docs/backlog.md` (the history line).

- [ ] **Step 1: Measure** every Task 1 case on the PR 2 branch: three runs, against the Task 1 baseline and the Global Constraints bounds. If a bound fails, cut cost in the order given there, then measure again.
- [ ] **Step 2: Decide the system view.** If its star view-zoomed cases exceed the bounds, apply `animatedPixelRatio` to StarView too (spec: left to the measurement); otherwise leave it.
- [ ] **Step 3: Write the docs above.** Post the after-table on #391, and in the PR description.
- [ ] **Step 4: Run every check**: `cd web && npm run check && npx vitest run`.
- [ ] **Step 5: Commit**: `docs: record the close-up photosphere's frame times and calibrations`. Open PR 2 with `Refs #391`.
