# Seeded Sky Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every system a stored galactic place and draw each system's
own sky from it: band brightness and orientation from the place, faint baked
gas and dust clouds from a sky seed, with a three-way sky choice in the
viewer.

**Architecture:** The generator draws a `GalacticPlace` once per system on
its own rng stream and writes it to `system.json` (filled on read when
missing). The web turns it into a `SkyLook`, bakes the clouds once on the
CPU into six cube faces (cached per look), and a shared `SkyBackdrop`
object draws the cube map and the star points in each of the three views,
oriented by the system axes the views already know.

**Tech Stack:** Rust (`crates/mapgen`, serde, ts-rs), TypeScript strict,
Svelte 5, Three.js, vitest.

**Spec:** `docs/superpowers/specs/2026-11-14-seeded-sky-design.md`

## Global Constraints

- No `SYSTEM_FORMAT_VERSION` bump (stays 4 on main; #465 takes it to 5).
- The new rng stream is `stream::GALAXY = 43`, seeded with
  `stream_seed(seed, stream::GALAXY)`, shared with no other draw: every
  other value of a system stays byte-identical.
- Radius: `R = 8.2 + (−0.1 − [Fe/H]) / 0.06 + 1.67·N(0,1)` kpc (scatter
  0.1 dex / 0.06 dex/kpc), clamped to `[2, 16]`. Height: two-sided
  exponential, scale 0.3 kpc, clamped to `[−1.5, 1.5]`. Azimuth uniform.
  Orientation a uniform random rotation.
- Galactocentric frame: centre at the origin, +z the galactic north pole,
  the Sun at `(8.2, 0, 0.02)`; a system at `(R cos φ, R sin φ, z)`.
- The system frame is the star's equatorial frame of `lib/systemSky.ts`
  (y the star's spin axis); in a view, the system's axes are
  `systemSky(...).star.axes`.
- Brightness relative to the Sun's place: `exp(−(R − 8.2) / 2.6)`.
- Clouds stay faint: no cloud texel brighter than 0.08 in linear RGB, and
  under the band's own brightness.
- Tests: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`,
  one full run at a time; web `npx vitest run`, `npm run check`.
- Never `npm ci` in the worktree: `web/node_modules` is a symlink to the
  main checkout. Never port 3000.
- Docs ship with each task that changes behaviour (README, `docs/system-spec.md`).

## Review Focus

1. A standalone world (`world.system` null) shows the fixed sky and no
   error: test in Task 6 (`skyLook(null)` gives the fixed look) and Task 7.
2. A moon's view: the sky stays fixed against the star while the moon is
   locked (`setStarsTurn`): Task 7 checks the backdrop's rotation follows
   `star.axes` and not the globe.
3. Extreme places (R = 2 and R = 16, |z| = 1.5) stay readable: no NaN, the
   band never vanishes nor saturates: Task 3 tests both ends.
4. Opening several systems in a row does not leak cube textures or bake
   again for a cached look: Task 5 tests the cache, Task 7 disposes.
5. A stored `stars: false` display turns into "no sky", not the system's
   sky: Task 8 tests the migration.

---

### Task 1: Galactic place draw (Rust)

**Files:**
- Create: `crates/mapgen/src/galaxy.rs` (+ `pub mod galaxy;` in `lib.rs`)
- Modify: `crates/mapgen/src/noise.rs` (`stream::GALAXY = 43`)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
  #[serde(rename_all = "lowercase")]
  pub enum PlaceSource { Drawn, Observed }
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] // + ts export
  pub struct GalacticPlace {
      pub source: PlaceSource,
      /// Galactocentric position, kpc (see the module doc for the frame).
      pub position_kpc: [f64; 3],
      /// The galactic x, y, z axes in the system's frame (orthonormal, right-handed).
      pub axes: [[f64; 3]; 3],
      /// Seed of the sky's clouds and stars.
      pub sky_seed: u32,
  }
  pub fn draw_place(seed: u32, metallicity: f32) -> GalacticPlace;
  pub const SUN_RADIUS_KPC: f64 = 8.2;
  ```

- [ ] **Step 1: Write failing tests** in `galaxy.rs`:
  `a_place_is_deterministic` (same args, equal); `the_mean_metallicity_sits_at_the_sun`
  (mean R over seeds 0..2000 at `[Fe/H] = −0.1` within 8.2 ± 0.2);
  `metal_rich_stars_sit_closer_in` (mean R at +0.3 < mean R at −0.5 − 5);
  `radius_and_height_stay_in_the_disc` (all R in `[2,16]`, |z| ≤ 1.5 over
  seeds 0..2000 and metallicities −1.0, 0.5); `axes_are_a_rotation`
  (orthonormal to 1e-9, determinant +1); `seeds_give_different_sky_seeds`.
- [ ] **Step 2:** `cargo test -p mapgen --all-features galaxy` → fails (no module).
- [ ] **Step 3:** Implement with the Global Constraints' numbers; uniform
  rotation from a random unit quaternion (four normals, normalised).
  Module doc states the frame and cites Genovali et al. 2014 for the gradient.
- [ ] **Step 4:** Tests pass; `cargo clippy -p mapgen --all-features -- -D warnings`.
- [ ] **Step 5:** Commit `feat(galaxy): draw a system's galactic place`.

### Task 2: Store it in the system

**Files:**
- Modify: `crates/mapgen/src/system.rs` (`System` :136, `assemble` construction :1505, `read_system` :375)
- Regenerate: `web/src/lib/api/types/{System,GalacticPlace,PlaceSource}.ts` (ts-rs, written by `cargo test --all-features`)
- Docs: `docs/system-spec.md` (system.json fields), README system file section (grep `system.json`)

**Interfaces:**
- Consumes: `galaxy::draw_place`.
- Produces: `System.galaxy: Option<GalacticPlace>` with
  `#[serde(default, skip_serializing_if = "Option::is_none")]` and `ts(optional)`;
  always `Some` when drawn; `read_system` fills it with
  `draw_place(system.seed, system.star.metallicity)` when absent.

- [ ] **Step 1: Failing tests** in `system.rs` tests:
  `a_drawn_system_has_its_galactic_place` (whole and around both `Some`,
  equal to `draw_place(seed, star.metallicity)`);
  `a_requested_metallicity_sets_the_place` (an around request with
  `metallicity: 0.3` gets `draw_place(seed, 0.3)`);
  `a_system_without_a_place_gets_it_on_read` (write a system, strip `galaxy`
  from the JSON, `read_system` gives the same place);
  (that no other draw shifts is checked on whole systems in Task 9, Step 1).
- [ ] **Step 2:** Run → fail.
- [ ] **Step 3:** Add the field; draw it in `assemble` after the metallicity
  is known; fill in `read_system` next to the activity fill.
- [ ] **Step 4:** Full throttled test run passes; TS types regenerated; `npm run check` in web passes.
- [ ] **Step 5:** Docs; commit `feat(system): store the galactic place in system.json`.

### Task 3: Sky look (web)

**Files:**
- Create: `web/src/lib/viewer/skyLook.ts`, `skyLook.test.ts`

**Interfaces:**
- Produces:
  ```ts
  export interface SkyLook {
    seed: number;            // sky_seed, or 7 for the fixed sky
    brightness: number;      // exp(−(R − 8.2)/2.6); 1 for the fixed sky
    bandWidth: number;       // radians (std dev); 7° at the Sun's place
    bulge: number;           // 0..1, the bulge's strength toward the centre
    north: Vec3;             // galactic north pole, system frame
    centre: Vec3;            // direction to the galactic centre, system frame
  }
  export const FIXED_SKY: SkyLook; // today's: seed 7, band tilted 60° about x
  export function skyLook(place: GalacticPlace | null): SkyLook;
  ```
  `centre` = `axes` applied to `normalize(−position)`.

- [ ] **Step 1: Failing tests:** `skyLook(null)` is `FIXED_SKY`; the Sun's
  place gives brightness 1 and bandWidth 7°; R = 2 is brighter, wider-bulged
  than R = 16; |z| = 1.5 narrows the band against z = 0; `north` and
  `centre` are unit vectors; no NaN at R = 2, 16 and |z| = 1.5; bulge
  within `[0,1]`.
- [ ] **Steps 2-4:** fail, implement (starting laws in the spec, tuned in
  Task 9), pass.
- [ ] **Step 5:** Commit `feat(web): derive the sky's look from the galactic place`.

### Task 4: Seeded star field (after Task 5: it uses `dustAt`)

**Files:**
- Modify: `web/src/lib/viewer/stars.ts`, `stars.test.ts`

**Interfaces:**
- Consumes: `SkyLook`, `dustAt` (Task 5; land Task 5's `dustAt` first or
  pass a `dust?: (dir: Vec3) => number` parameter defaulting to none).
- Produces: `starField(look: SkyLook = FIXED_SKY, dust?: (dir: Vec3) => number): StarField`.
  Band stars spread about the plane whose normal is `look.north`, count
  scaled by `look.brightness` (sky stars by `sqrt(brightness)`), a bulge
  cluster toward `look.centre` by `look.bulge`; each star's intensity
  multiplied by `1 − dust(dir)`.

- [ ] **Step 1: Failing tests** (keep the existing ones, now with `FIXED_SKY`):
  `FIXED_SKY` reproduces today's field exactly (directions equal to a
  snapshot of `starField(7)` taken before the change); band stars lie
  closer to the plane ⟂ `look.north` than random; a brighter look has
  more band stars; stars toward `centre` are denser with bulge 1 than 0;
  dust 1 everywhere gives zero intensity.
- [ ] **Steps 2-4.** Update the header comment: compared worlds each keep
  their own sky.
- [ ] **Step 5:** Commit `feat(web): seed and orient the star field from the sky look`.

### Task 5: Baked clouds

**Files:**
- Create: `web/src/lib/viewer/skyClouds.ts`, `skyClouds.test.ts`

**Interfaces:**
- Produces:
  ```ts
  export interface SkyFaces { size: number; faces: Uint8Array[] } // 6 RGBA faces, Three's cube order (+x, −x, +y, −y, +z, −z), sRGB-encoded
  export function dustAt(look: SkyLook, dir: Vec3): number;       // 0..1 extinction
  export function bakeSkyClouds(look: SkyLook, size = 256): SkyFaces; // memoised per look (last 4)
  ```
  Clouds: band glow × brightness, emission (H-alpha red, O III blue-green)
  and blue reflection patches from seeded fBm with domain warping,
  concentrated along the band (gaussian in latitude about `north`), dust
  lanes from `dustAt` darkening the glow. Colour mix and patch shapes vary
  with `look.seed`.

- [ ] **Step 1: Failing tests:** deterministic per look; two seeds differ
  (mean absolute texel difference > a threshold); no texel above 0.08
  linear; the band latitude band is brighter on average than the poles;
  `dustAt` is higher in the band than at the poles and in `[0,1]`; the
  cache returns the same object for an equal look; 256 bake under 300 ms
  in vitest (log the time).
- [ ] **Steps 2-4.** If the bake is over 150 ms in a browser in Task 9,
  move it to a worker there.
- [ ] **Step 5:** Commit `feat(web): bake faint gas and dust clouds per sky`.

### Task 6: Sky backdrop object

**Files:**
- Create: `web/src/lib/viewer/skyBackdrop.ts`, `skyBackdrop.test.ts`
- Modify: `web/src/lib/viewer/shaders.ts` (backdrop shader; the stars'
  shader keeps `HIDE_HOST_STAR`)

**Interfaces:**
- Produces:
  ```ts
  export type SkyChoice = 'system' | 'fixed' | 'off';
  export class SkyBackdrop {
    constructor(options: { hideHostStar: boolean; renderOrder: number });
    readonly group: Group;           // cube backdrop + star Points
    readonly stars: Points<BufferGeometry, ShaderMaterial>;
    setLook(look: SkyLook): void;    // rebuilds stars, uploads the baked faces as a CubeTexture
    setAxes(axes: [Vec3, Vec3, Vec3] | null): void; // system axes in the view's frame; null = identity
    setOpacity(o: number): void;
    setPixelRatio(r: number): void;
    dispose(): void;
  }
  ```
  The cube backdrop is drawn at the far plane like the stars
  (`clip.xyww`), render order one below the stars, `depthWrite: false`.

- [ ] **Step 1: Failing tests** (no WebGL needed: inspect objects):
  `setAxes` sets the group's rotation matrix from the three axes;
  `setLook` twice with an equal look keeps the same texture; `dispose`
  disposes geometry, materials and texture; `skyLook(null)` through
  `setLook` gives a star count equal to `starField(FIXED_SKY)`'s.
- [ ] **Steps 2-4.**
- [ ] **Step 5:** Commit `feat(web): add the sky backdrop shared by the views`.

### Task 7: Use it in the three views

**Files:**
- Modify: `web/src/lib/viewer/PlanetView.ts` (stars at :856-900, `setStars` :1892, `setStarsTurn` :1902, visibility :3491, opacity :3539, host uniforms :3408-3444)
- Modify: `web/src/lib/viewer/StarView.ts` (:420-437, dispose :1670, pixel ratio :1789)
- Modify: `web/src/lib/viewer/StarBodyView.ts` (:172-189)

**Interfaces:**
- Consumes: `SkyBackdrop`, `skyLook`, `SkyChoice`.
- Produces on each view: `setSkyLook(look: SkyLook): void`,
  `setSkyChoice(choice: SkyChoice): void` (replaces `PlanetView.setStars`),
  `setSkyAxes(axes: [Vec3, Vec3, Vec3] | null): void`.
  PlanetView: the host-disc uniforms move to `backdrop.stars`; with axes
  set, the backdrop follows the system axes and `setStarsTurn` no longer
  turns it (the axes already turn with the body); without axes
  (standalone world), today's turn behaviour stays.

- [ ] **Step 1: Failing tests** where views are already tested
  (`StarBodyView.test.ts` pattern): `setSkyChoice('off')` hides the
  backdrop; `'fixed'` uses `FIXED_SKY` whatever look was set; disposing
  the view disposes the backdrop.
- [ ] **Steps 2-4**; `npm run check`.
- [ ] **Step 5:** Commit `feat(web): draw the system's sky in every view`.

### Task 8: Viewer wiring and control

**Files:**
- Modify: `web/src/components/Viewer.svelte` (display defaults :168-183, `migrateDisplay` call :248, save :568-578, toggle :1904, effect :901, `sysSky` :1096, StarPane :2568, StarSurfacePane :2602)
- Modify: `web/src/components/StarPane.svelte`, `StarSurfacePane.svelte` (new props `look`, `skyChoice`)
- Modify: `web/src/lib/switches.ts` (`migrateDisplay`), `web/src/lib/display.ts` (linked key), `web/src/lib/shortcuts.ts` (`b` cycles system → fixed → off)
- Modify: `web/src/lib/i18n/en.ts`, `fr.ts` (three labels + help)
- Docs: README viewer section (grep `starsHelp` text / "Stars" toggle), the viewer help.

**Interfaces:**
- Consumes: Task 7's setters; `sys.galaxy`; `sysSky.star.axes`.
- Produces: display key `sky: SkyChoice` (default `'system'`), replacing
  `stars: boolean`.

- [ ] **Step 1: Failing test** in `switches.test.ts` (or the
  `migrateDisplay` test file): stored `{stars: true}` gives `sky: 'system'`,
  `{stars: false}` gives `'off'`, an unknown `sky` falls back to `'system'`.
- [ ] **Steps 2-4:** wire: `look = skyLook(sys?.galaxy ?? null)`, axes
  from `sysSky?.star.axes ?? null` for PlanetView, identity for StarView
  and StarBodyView (check their scene frames against `systemSky`'s and
  pass the matching axes if they differ). The toggle row offers the three
  choices. `npm run check`, `npx vitest run`.
- [ ] **Step 5:** Commit `feat(web): let the viewer choose the sky`.

### Task 9: Tune and check in the browser

- [ ] **Step 1:** `world-coherence-checker`: a few systems drawn by
  `origin/main` and by this branch are byte-identical apart from `galaxy`.
- [ ] **Step 2:** Rebuild develop with this branch (warn mapgen1 and
  mapgen3 first; keep `feat/planet-model-part4` merged), deploy with
  `develop-deployer`.
- [ ] **Step 3:** `viewer-visual-check`: screenshots of the planet, system
  and star views for three systems (R near 3, 8, 14 kpc): different
  clouds per system, brighter band near the core, clouds faint behind the
  planet, photo filter unaffected, host disc hides the backdrop. Tune
  Task 3/5 constants from what it reports; re-run their tests.
- [ ] **Step 4:** Commit tuning `fix(web): tune the sky's clouds and brightness`.

---

## Addendum: galactic core, nebulae, two renderings

Spec: the "Addendum, 2026-11-14" section of the design. Same Global
Constraints, except the cloud ceiling: 0.08 linear holds for the naked-eye
rendering only; long exposure may reach 0.35 linear in nebula cores and
0.25 in the bulge.

### Task 10: Exposure and galactic core

**Files:** `web/src/lib/viewer/skyLook.ts`, `skyClouds.ts` (+ tests)

**Interfaces:**
- Produces: `export type SkyExposure = 'eye' | 'photo'`;
  `bakeSkyClouds(look: SkyLook, exposure: SkyExposure, size?: number)`
  (cache keyed by look and exposure); `dustAt(look, dir)` gains the core's
  dust lane.
- Bulge: elongated glow centred on `look.centre`, axis ratio 0.6 along the
  band, angular radius and brightness from `(8.2 / R)²` (store what is
  needed in `SkyLook`, e.g. `coreSize`, `coreBrightness`), crossed by a
  dark lane along the plane.

- [ ] Tests first: at the Sun's place, the core region (within its radius
  of `centre`) is at least 2× brighter on average than the band 90° away,
  in both exposures; R = 3 core brighter and larger than R = 14; the lane
  makes the plane through the core darker than 3° above and below it;
  'eye' texels ≤ 0.08 linear and mean saturation lower than 'photo';
  'photo' ≤ 0.35; deterministic per (look, exposure).
- [ ] Implement, run `npx vitest run`, commit `feat(web): add the galactic core and the long-exposure rendering`.

### Task 11: Nebula catalogue

**Files:** create `web/src/lib/viewer/skyNebulae.ts` (+ test); modify
`skyClouds.ts` (`dustAt` includes dark nebulae and pillar silhouettes).

**Interfaces:**
- Produces:
  ```ts
  export type NebulaKind = 'planetary' | 'emission' | 'pillars' | 'remnant' | 'reflection' | 'dark';
  export interface Nebula {
    kind: NebulaKind;
    dir: Vec3;          // centre, system frame
    radius: number;     // angular radius, radians
    roll: number;       // orientation about dir, radians
    seed: number;       // its own shape seed
    tint: number;       // 0..1 colour-mix parameter within the kind
  }
  export function skyNebulae(look: SkyLook): Nebula[]; // memoised per look
  ```
  Sizes and placement from the spec table; counts per kind drawn from the
  sky seed, scaled with `look.brightness` (starting point at the Sun:
  planetary 3–8, emission 2–5, pillars 1–3, remnant 1–3, reflection 2–6,
  dark 4–10).

- [ ] Tests first: deterministic; two sky seeds give different catalogues
  (kinds counts or positions); every kind appears in at least 80 % of 200
  seeds; sizes within the table; emission/pillars/reflection/dark lie
  within 15° of the band plane, the band-biased kinds closer on average
  than uniform; more nebulae at R = 3 than R = 14; `dustAt` is higher at
  a dark nebula's centre than just outside it.
- [ ] Implement, run tests, commit `feat(web): draw a catalogue of nebulae per sky`.

### Task 12: Nebula textures

**Files:** create `web/src/lib/viewer/nebulaTextures.ts` (+ test)

**Interfaces:**
- Produces: `bakeNebula(n: Nebula, exposure: SkyExposure): { size: number; rgba: Uint8Array; blend: 'add' | 'darken' }`
  (size 128–512 by angular size; sRGB bytes; premultiplied alpha), memoised.
  Per kind: planetary — ring (or bipolar lobes when `tint` > 0.6) with an
  O III core and H-alpha rim, a central star; emission — fBm glow with
  domain warping and dark lanes; pillars — emission background with 1–3
  dark tapering columns with bright rims; remnant — thin filamentary
  shell; reflection — blue haze round a centre; dark — opaque soft-edged
  patch (`darken`).

- [ ] Tests first: deterministic; planetary ring: mean of the inner 20 %
  radius darker than the rim annulus, rim redder than the core in 'photo';
  pillars: a dark column (a vertical strip far darker than its
  surroundings); dark kind uses `darken`; 'eye' bytes ≤ the 0.08 linear
  ceiling and less saturated than 'photo'; edges fade to zero (no visible
  square border: outer 5 % rows and columns near zero alpha).
- [ ] Implement, run tests, commit `feat(web): bake nebula textures by kind`.

### Task 13: Draw them and add the switch

**Files:** `web/src/lib/viewer/skyBackdrop.ts`, `shaders.ts`, the three
views, `web/src/components/Viewer.svelte`, `StarPane.svelte`,
`StarSurfacePane.svelte`, `web/src/lib/i18n/en.ts`, `fr.ts`,
`web/src/lib/display.ts`, `web/README.md`.

**Interfaces:**
- Consumes: Tasks 10-12.
- Produces: `SkyBackdrop.setExposure(e: SkyExposure)`; each view
  `setSkyExposure(e)`; display key `skyExposure: SkyExposure`
  (default `'photo'`), linked display.
- Nebulae: one mesh per nebula, a patch of the far sphere (drawn at the
  far plane like the stars), oriented by `dir` and `roll`, render order
  between the cloud cube and the stars; `add` blending or darkening;
  hidden with the sky; the fixed sky shows none; disposed with the
  backdrop; textures baked lazily when the system's sky is shown.

- [ ] Tests first (object inspection, no WebGL): the system sky creates
  one mesh per nebula; the fixed sky and 'off' create none; changing
  exposure swaps textures without rebuilding the catalogue; dispose frees
  every nebula texture; a stored display without `skyExposure` gets
  `'photo'`.
- [ ] Wire the "Long exposure" toggle next to the sky switch (i18n
  « Pose longue » / "Long exposure", help line), a free shortcut if one
  fits; docs in web/README.md.
- [ ] `npx vitest run`, `npm run check`, commit `feat(web): draw the nebulae and let the viewer pick the exposure`.

### Task 14: Look again

- [ ] Redeploy develop (warn mapgen1, mapgen3); `viewer-visual-check` on
  3 systems at about 3, 8 and 14 kpc in both exposures; tune; commit.
