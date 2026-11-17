# Climate pass lot 3B: geysers on the map and in the viewer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show the geysers part A stores (`PlanetModel.geysers`) as tiger
stripes on the surface map, as a plume above the moon seen in its host's
sky, as jets in the moon's close-up, and make the plume ring (E ring)
visible: forward scattering when back-lit, and dust rings drawn in a
moon's sky.

**Architecture:** One Rust module turns a stored `Fracture` into sphere
geometry; world generation cuts troughs flanked by ridges along it, darkens
the stripes through the albedo layer and removes the craters of the young
polar terrain (base map and detail tiles alike). The viewer shares one
dust phase function (Henyey–Greenstein) between the dust rings, the sky
plume and the close-up jets, all placed from the same stored fractures.

**Tech Stack:** Rust (crate `mapgen`), TypeScript + Three.js + GLSL
(`web/src/lib/viewer`), vitest.

**Spec:** `docs/superpowers/specs/2026-11-16-climate-pass-lot3-design.md`,
section "Part B", acceptance "Part B"; schedule in #427 (comment
6919717434), item 3.

## Global Constraints

- Branch `feat/climate-lot3b-geysers`, worktree `~/Developer/mg-climate-lot3b`, from `main` at 7a26059.
- World `FORMAT_VERSION` 24 → 25 (`crates/mapgen/src/export/raw_data.rs:89`); `MIN_FORMAT_VERSION` stays 24 (format-24 worlds stay readable, only without stripes). `SYSTEM_FORMAT_VERSION` stays 9: no stored model field changes.
- `RENDER_VERSION` 9 → 10 (`crates/mapgen/src/detail.rs:57`) and `TILE_RENDER_VERSION` 9 → 10 (`web/src/lib/viewer/TileLayer.ts:76`): the tiles' craters change in the young terrain.
- No new `WorldParams` field: generation and tiles read `params.model.as_ref().and_then(|m| m.geysers.as_ref())` (`load_layers` already rebuilds `params.model` from `world.json`).
- Tiger stripes (Porco et al. 2006, doi:10.1126/science.1123013): trough about 500 m deep and 2 km wide, flanked on each side by a ridge about 100 m high and 2 km wide; the south polar terrain, bounded near 55° S, is nearly free of craters.
- Every constant carries its source in a doc comment; a look constant (no physical source) says so.
- Worlds of bodies without geysers stay byte-identical.
- Tests: `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features <filter> -- --test-threads=3`; full run `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`, one at a time on this machine; web `cd web && npm run check && npm test`. Never `npm ci` in the worktree, never port 3000, never write under the data checkout's `systems/`, `worlds/`, `db/`.
- CLI checks go to a scratch dir: `cargo run --release -p map-generator -- --system "saturn-like=1,enceladus-like=1" --seed 1 --systems-dir "$SCRATCH/systems" --draw-only` to find the moon id, then the same with `--body <id> --width 1024 --height 512 --no-svg --no-noise-maps`.
- Documentation in the same commit as the behaviour (global rule): README paragraph at line 131 (planet model, geysers), README `### Impact craters` (line 777), `docs/science/planet-interiors.md` ("The ice shell and the ocean beneath"), `web/README.md` (viewer), the `FORMAT_VERSION` doc list (`raw_data.rs:47-88`).

## Review Focus

1. A geyser pole near the map seam (fracture centre at lon ±180, or the fracture through the pole at lat ±90): stripes must be continuous across the seam and around the pole. Tests in Task 1 (`segment_across_the_seam`, `segment_through_the_pole`).
2. Detail tiles and the base map must agree: a crater removed on the base map must not reappear in tiles (`CraterField::near`). Test in Task 2 (`tiles_drop_the_same_craters`).
3. A format-24 world with geysers in `world.json` but no stripes in its elevation: tiles now drop its polar craters (accepted, hence the render bump), and the viewer still draws jets from the model. Test in Task 2 (`format_24_world_still_loads`) and the jets' placement does not depend on the format.
4. A moon with geysers drawn at the 2.5 px floor in the host's sky: the plume must stay visible and not exceed its floor-scaled size; a plume must vanish with the sky bodies (headlight mode, flat map). Test in Task 5.
5. Many dust rings (gossamer + plume) in a moon's sky, the camera inside the plume ring (Enceladus's own sky): fixed uniform array, extras dropped, no shader error. Test in Task 4.

---

### Task 1: Fracture geometry on the sphere

**Files:**
- Modify: `crates/mapgen/src/planet/geysers.rs` (after `Fracture`, `:76`)
- Test: same file, `mod tests` (`:216`)

**Interfaces:**
- Produces, in `planet::geysers` (grid frame of `Grid::unit_vector_at`: `[cos lat cos lon, cos lat sin lon, sin lat]`, lon in radians east):
  - `pub fn unit_at(lat_deg: f64, lon_deg: f64) -> [f64; 3]`
  - `impl Fracture { pub fn ends(&self, radius_km: f64) -> ([f64; 3], [f64; 3]) }` — great-circle segment of `length_km` centred on (`lat_deg`, `lon_deg`) along `azimuth_deg` (clockwise from north; at a pole, 0 runs along meridian `lon_deg`, as the field doc says).
  - `impl Fracture { pub fn offset_km(&self, p: [f64; 3], radius_km: f64) -> Option<(f64, f64)> }` — `(across_km, along)`: signed distance from `p` to the fracture's great circle, and position along it in `[-0.5, 0.5]` of its length; `None` when `p`'s projection falls beyond the ends or `|across_km|` exceeds `FRACTURE_SPACING_KM`.
  - `pub struct YoungTerrain { pub pole: [f64; 3], pub cap_rad: f64 }` with `pub fn of(geysers: &Geysers, radius_km: f64) -> YoungTerrain` and `pub fn covers(&self, p: [f64; 3]) -> bool`.
  - `pub const YOUNG_CAP_FACTOR: f64 = 2.0;` and `pub const YOUNG_CAP_MAX_DEG: f64 = 40.0;` — the young terrain reaches twice the fractures' farthest end from the pole, at most 40°: Enceladus's fractures fill a ~16° cap and its south polar terrain reaches ~35° from the pole (bounded near 55° S, Porco et al. 2006).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn ends_are_half_a_length_from_the_centre() {
    let f = Fracture { lat_deg: -80.0, lon_deg: 30.0, azimuth_deg: 90.0, length_km: 130.0 };
    let r = 252.1;
    let (a, b) = f.ends(r);
    let c = unit_at(-80.0, 30.0);
    for e in [a, b] {
        assert!((angle(c, e) * r - 65.0).abs() < 0.1);
    }
}

#[test]
fn offset_is_zero_on_the_fracture_and_spacing_away_beside_it() {
    // the centre is on it; a point 10 km across the strike is 10 km off
    let f = Fracture { lat_deg: -80.0, lon_deg: 30.0, azimuth_deg: 90.0, length_km: 130.0 };
    let r = 252.1;
    let (across, along) = f.offset_km(unit_at(-80.0, 30.0), r).unwrap();
    assert!(across.abs() < 1e-6 && along.abs() < 1e-6);
    let north = unit_at(-80.0 + (10.0 / r).to_degrees(), 30.0);
    assert!((f.offset_km(north, r).unwrap().0.abs() - 10.0).abs() < 0.05);
    assert!(f.offset_km(unit_at(-80.0, 30.0 + 90.0), r).is_none());
}

#[test]
fn segment_through_the_pole() {
    let f = Fracture { lat_deg: -90.0, lon_deg: 45.0, azimuth_deg: 0.0, length_km: 130.0 };
    let r = 252.1;
    let (a, b) = f.ends(r);
    // runs along the 45°/225° meridians
    for (e, lon) in [(a, 45.0_f64), (b, 225.0)] { assert!(lon_close(e, lon) || lon_close(e, lon - 180.0)); }
    assert!(f.offset_km(unit_at(-89.0, 45.0), r).unwrap().0.abs() < 0.05);
}

#[test]
fn segment_across_the_seam() {
    let f = Fracture { lat_deg: -82.0, lon_deg: 179.0, azimuth_deg: 90.0, length_km: 130.0 };
    let r = 252.1;
    for lon in [178.0, -179.5] {
        assert!(f.offset_km(unit_at(-82.0, lon), r).is_some());
    }
}

#[test]
fn young_terrain_of_enceladus_reaches_about_55_south() {
    // the fractures part A draws for an Enceladus-like moon (helper already used by the tests at :216)
    let g = enceladus_geysers();
    let y = YoungTerrain::of(&g, 252.1);
    assert!((25.0..=40.0).contains(&y.cap_rad.to_degrees()));
    assert!(y.covers(unit_at(-75.0, 0.0)) && !y.covers(unit_at(-45.0, 0.0)));
}
```

`angle` and `lon_close` are small test helpers; `enceladus_geysers` reuses whatever fixture `geysers.rs`'s tests already build (`:216-268`), or calls `geysers(..)` with the same inputs.

- [ ] **Step 2: Run them, expect compile errors** — `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features geysers -- --test-threads=3`
- [ ] **Step 3: Implement** the interfaces above. `ends` uses the spherical destination formula from the centre at bearing `azimuth_deg` and `azimuth_deg + 180`, angular distance `length_km / 2 / radius_km`; at |lat| = 90 take north as the direction of meridian `lon_deg` (so the formula holds with the bearing measured from it). `offset_km`: normal `n = normalize(a × b)`, `across = asin(p·n) · r`; `along` from the angle of `p`'s projection on the great circle measured from the centre, divided by the angular length. `YoungTerrain::of`: pole = `unit_at(±90, 0)` by `south`; `cap_rad = min(YOUNG_CAP_FACTOR × max angle(pole, end), YOUNG_CAP_MAX_DEG)`.
- [ ] **Step 4: Run the tests, expect PASS.**
- [ ] **Step 5: Commit** — `feat(planet): place geyser fractures on the sphere`

### Task 2: Tiger stripes and young terrain in the world

**Files:**
- Create: `crates/mapgen/src/stripes.rs` (register in `lib.rs`)
- Modify: `crates/mapgen/src/generation.rs` (`generate_with_progress` after the cones `:2262-2265`, before the Dry-era craters `:2270`; same place in `cratered_relief` `:1790`; albedo after `:2275`)
- Modify: `crates/mapgen/src/craters.rs` (`CraterField::new_at` filter `:738`, `CraterField::near` `:789`, `context_at` `:635` keeps the `YoungTerrain`)
- Modify: `crates/mapgen/src/export/raw_data.rs:89` (FORMAT 25 + doc list), `crates/mapgen/src/detail.rs:57` (RENDER 10), `web/src/lib/viewer/TileLayer.ts:76` (10)
- Test: `stripes.rs` `mod tests`, `crates/mapgen/src/system.rs` near `plume_rings_follow_geysers` (`:5445`)

**Interfaces:**
- Consumes: Task 1 (`Fracture::offset_km`, `YoungTerrain`).
- Produces, in `stripes`:
  - `pub fn stripe_profile_km(across_km: f64, along: f64) -> f64` — trough −0.5 km over 2 km width, ridges +0.1 km 2 km wide on each side, tapering to 0 over the last 10 % of the length at each end (`along` beyond ±0.4).
  - `pub fn apply_stripes(elevation: &mut [f32], grid: &Grid, p: &WorldParams)` — no-op without geysers; adds `stripe_profile_km / craters::ELEVATION_KM` at each pixel near a fracture, the profile's widths floored at 1.5 pixels (`grid.dy_km()`) so the base map keeps them, the depths scaled by `2 / floored width` to keep the cross-section area.
  - `pub fn stripe_albedo(albedo: &mut [f32], grid: &Grid, p: &WorldParams)` — adds `STRIPE_ALBEDO = -0.25` (look constant: the stripes read darker than the plains in Cassini's clear-filter images) over the trough's floored width.
  - `CraterField` keeps `young: Option<YoungTerrain>`; `new_at` and `near` drop every crater whose `centre` it covers.

- [ ] **Step 1: Write the failing tests**

```rust
// stripes.rs
#[test]
fn profile_is_a_trough_between_two_ridges() {
    assert!((stripe_profile_km(0.0, 0.0) + 0.5).abs() < 1e-6);
    assert!((stripe_profile_km(2.0, 0.0) - 0.1).abs() < 0.02);
    assert_eq!(stripe_profile_km(6.0, 0.0), 0.0);
    assert!(stripe_profile_km(0.0, 0.49).abs() < 0.1);
}

// system.rs, next to plume_rings_follow_geysers
#[test]
fn enceladus_world_has_its_stripes_at_the_stored_fractures() {
    // draw requested(1, &[("enceladus-like", 1)]), generation_params(moon), generate at 1024×512
    // for each stored fracture: elevation at its centre pixel < mean elevation of the two pixels
    // FRACTURE_SPACING_KM / 2 away across it, by at least 0.2 km / ELEVATION_KM
}

#[test]
fn the_young_terrain_has_no_craters() {
    // same world: CraterField::new(&params, ..).craters has none whose centre YoungTerrain covers,
    // and the same cap round the other pole holds at least one
}

#[test]
fn tiles_drop_the_same_craters() {
    // CraterField::context_at(&params, epoch).near(pole, cap_rad, 0.1, 50.0) is empty for the young pole
}

#[test]
fn fractures_only_touch_the_young_terrain() {
    // same params with model.geysers = None: elevation and albedo equal outside YoungTerrain::covers,
    // and fully equal for a moon with no geysers (a_rocky_moon_has_no_geysers's draw)
}
```

and in `raw_data.rs` tests: `format_24_world_still_loads` — a world written with `format_version` 24 (edit the JSON of a freshly written world) loads through `load_layers`.

- [ ] **Step 2: Run, expect FAIL** — `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features stripe enceladus_world young_terrain tiles_drop fractures_only format_24 -- --test-threads=3` (one filter per run if cargo refuses several).
- [ ] **Step 3: Implement** `stripes.rs`, call `apply_stripes` in both relief pipelines (same place, so the sea-level searches see the same relief), `stripe_albedo` after the rays' albedo, and the crater filter in `CraterField`. Bump the three versions; the server test `viewer_and_server_agree_on_the_tile_renderer_version` ties the two render versions.
- [ ] **Step 4: Run the tests above, then `same_seed_produces_same_world` and the server's format tests, expect PASS.**
- [ ] **Step 5: Look at it** — generate the CLI Enceladus (Global Constraints) and downscale `raw_data` relief/colour PNG with `sips -Z 800` into the scratchpad; the stripes and the uncratered cap must be visible round the stored pole. Report the path, do not judge further.
- [ ] **Step 6: Docs** — README line 131 (stripes, young terrain, world format 25), `### Impact craters` (young terrain drops craters), `docs/science/planet-interiors.md` (profile and cap, Porco 2006), `FORMAT_VERSION` doc list.
- [ ] **Step 7: Commit** — `feat(world)!: cut tiger stripes at the stored geyser fractures` with footer `BREAKING CHANGE: world format 25; tiles render version 10.`

### Task 3: Dust phase function and back-lit dust rings

**Files:**
- Create: `web/src/lib/viewer/dustPhase.ts`, `web/src/lib/viewer/dustPhase.test.ts`
- Modify: `web/src/lib/viewer/ringShaders.ts` (`dustRingFragment` `:124-152`, vertex `:109` passes the world position), `web/src/lib/viewer/PlanetView.ts` (`setDustRings` `:1315`: camera position uniform)

**Interfaces:**
- Produces:
  - `export const DUST_G = 0.7;` — Henyey–Greenstein asymmetry of micron-sized ice grains; Saturn's E ring and Enceladus's plume brighten by tens of times from 90° to high phase (Hedman et al. 2012, Icarus 217, "The three-dimensional structure of Saturn's E ring"; Porco et al. 2006).
  - `export const DUST_GAIN_MAX = 80;`
  - `export function dustGain(cosScatter: number, g = DUST_G): number` — `hg(cosScatter, g) / hg(0, g)`, capped at `DUST_GAIN_MAX`; `cosScatter` is the cosine of the scattering angle (1 = light passing straight on to the eye, back-lit).
  - `export function cosScatter(sunDir: Vec3, toEye: Vec3): number` — `-dot(sunDir, toEye)` for unit vectors (`sunDir` points to the sun, `toEye` from the dust to the eye).
  - `export const dustPhaseGlsl: string` — GLSL `float dustGain(float cosScatter)` with the same constants, injected in each dust shader.

- [ ] **Step 1: Failing tests**

```ts
it('is 1 at 90° and grows tens of times back-lit', () => {
  expect(dustGain(0)).toBeCloseTo(1, 6);
  expect(dustGain(1)).toBeGreaterThan(30);
  expect(dustGain(-1)).toBeLessThan(0.5);
});
it('is back-lit when the sun is behind the dust', () => {
  expect(cosScatter([0, 0, 1], [0, 0, -1])).toBe(1);
});
it('the GLSL carries the same constants', () => {
  expect(dustPhaseGlsl).toContain(String(DUST_G));
});
```

- [ ] **Step 2: Run** `cd web && npx vitest run src/lib/viewer/dustPhase.test.ts`, expect FAIL.
- [ ] **Step 3: Implement**, then in `dustRingFragment`: `gl_FragColor = vec4(uColour * light, min(density * dustGain(cosScatter) * uFade, 1.0))` with the scattering cosine from the fragment's world position, `uCameraPos` and `uSunDir`; in the planet's shadow (`light` 0.08) the gain is not applied (no light to scatter).
- [ ] **Step 4: Run** `cd web && npm run check && npm test`, expect PASS.
- [ ] **Step 5: Docs** — `web/README.md`, rings section: dust rings scatter forward.
- [ ] **Step 6: Commit** — `feat(web): brighten dust rings when back-lit`

### Task 4: Dust rings in a moon's sky

**Files:**
- Modify: `web/src/lib/viewer/ringShaders.ts` (`skyRingFragment` `:213`, `skyRingDensityGlsl` `:158`), `web/src/lib/viewer/PlanetView.ts` (`drawSkyRings` `:3303`, `setRingUniforms` `:3878`), `web/src/components/Viewer.svelte` (`:1471-1548`: pass `dustOf(host)` with `hostRings`), `web/src/lib/ringMoons.ts` (export a pure packer)
- Test: `web/src/lib/ringMoons.test.ts` (create if absent)

**Interfaces:**
- Consumes: Task 3 (`dustPhaseGlsl`, `cosScatter` convention), `dustOf(body)` (`ringMoons.ts:15`), `FaintRing` (`:20`).
- Produces:
  - `export const SKY_DUST_MAX = 4;`
  - `export function skyDustUniforms(rings: readonly FaintRing[]): { radii: Float32Array /* 3·N: inner, peak, outer */; opacity: Float32Array /* N */; colour: Float32Array /* 3·N, linear */; count: number }` — the `SKY_DUST_MAX` rings of highest `opacity × (outer − inner)` kept, padded with zeros.
  - The sky ring material gets `uDustRadii`, `uDustOpacity`, `uDustColour`, `uDustCount`; `skyRingDensityGlsl` adds each dust ring's profile (the same triangular profile as `dustRingFragment`) times `dustGain`, in the host's radii.

- [ ] **Step 1: Failing tests**

```ts
it('keeps the rings that matter most and pads the rest', () => {
  const six = Array.from({ length: 6 }, (_, i) => ({ inner: i, peak: i + 0.5, outer: i + 1, opacity: 0.01 * (i + 1), colour: [1, 1, 1] as [number, number, number] }));
  const u = skyDustUniforms(six);
  expect(u.count).toBe(SKY_DUST_MAX);
  expect(Array.from(u.opacity)).toEqual(expect.arrayContaining([0.06, 0.05, 0.04, 0.03].map(Math.fround)));
  expect(skyDustUniforms([]).count).toBe(0);
});
```

- [ ] **Step 2: Run** `cd web && npx vitest run src/lib/ringMoons.test.ts`, expect FAIL.
- [ ] **Step 3: Implement**; `Viewer.svelte` passes the host's dust rings next to `hostRings` whenever the shown body is a moon of a giant with dust rings, the camera inside a ring included (Enceladus inside its E ring).
- [ ] **Step 4: Run** `cd web && npm run check && npm test`, expect PASS.
- [ ] **Step 5: Docs** — `web/README.md`: a moon's sky shows its host's dust rings.
- [ ] **Step 6: Commit** — `feat(web): draw the host's dust rings in a moon's sky`

### Task 5: Plume above a moon in the sky

**Files:**
- Create: `web/src/lib/viewer/plume.ts`, `web/src/lib/viewer/plume.test.ts`
- Modify: `web/src/lib/skyBodies.ts` (`SkyBody` `:41`), `web/src/lib/systemSky.ts` (`:244-267`), `web/src/lib/viewer/PlanetView.ts` (`setSkyBodies` `:2456`, `drawSkyBodies` `:3211`), `web/src/lib/viewer/shaders.ts` (plume sprite shaders next to `skyBodyVertex` `:1037`)

**Interfaces:**
- Consumes: Task 3 (`dustGain`, `dustPhaseGlsl`), `SkyBody.axes`, `SkyBody.phaseCos` (`skyBodies.ts:86`, 1 full, −1 new).
- Produces:
  - `SkyBody.plume?: { south: boolean }` — set in `systemSky` from `body.model?.geysers`.
  - `export const PLUME_HEIGHT_RADII = 1.5;` (look constant, Cassini images show the plume above one radius), `export const PLUME_ALPHA = 0.12;` (look constant at 90° phase).
  - `export function plumeFrame(axes: SkyBody['axes'], south: boolean, drawnHalf: number): { up: Vec3; height: number }` — `up` along the spin axis (`axes[1]`, negated for a south pole; check `systemSky.ts:259` that `axes[1]` points north), `height = PLUME_HEIGHT_RADII × drawnHalf`, where `drawnHalf` is the half-size the body is actually drawn at (its floor included).
  - `export function plumeAlpha(phaseCos: number): number` — `min(1, PLUME_ALPHA × dustGain(-phaseCos))`.
  - One fan-shaped sprite per plume body, drawn in `drawSkyBodies` from the body's disc, hidden whenever the body is hidden (`uLightMode !== 1` or `morph <= 0.01`).

- [ ] **Step 1: Failing tests**

```ts
it('stands on the venting pole at the drawn size', () => {
  const axes = [[1, 0, 0], [0, 1, 0], [0, 0, 1]] as const;
  expect(plumeFrame(axes, true, 2).up).toEqual([-0, -1, -0]);
  expect(plumeFrame(axes, false, 2).height).toBeCloseTo(3);
});
it('is brightest back-lit and still seen side-lit', () => {
  expect(plumeAlpha(-1)).toBe(1);
  expect(plumeAlpha(0)).toBeCloseTo(PLUME_ALPHA);
  expect(plumeAlpha(1)).toBeGreaterThan(0.02);
});
```

(adjust `toEqual` for signed zeros with `toBeCloseTo` per component if needed).
- [ ] **Step 2: Run** `cd web && npx vitest run src/lib/viewer/plume.test.ts`, expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cd web && npm run check && npm test`, expect PASS.
- [ ] **Step 5: Commit** — `feat(web): show a moon's plume in its host's sky`

### Task 6: Jets in the close-up

**Files:**
- Create: `web/src/lib/viewer/jets.ts`, `web/src/lib/viewer/jets.test.ts`
- Modify: `web/src/lib/viewer/PlanetView.ts` (a `setJets(geysers: Geysers | null, radiusKm: number)` next to `setDustRings` `:1315`; drawn in the frame loop), `web/src/components/Viewer.svelte` (pass `world.meta.model?.geysers`)

**Interfaces:**
- Consumes: `Geysers`, `Fracture` (`web/src/lib/api/types`), `lonLatToSphere` (`web/src/lib/geo.ts:32`, radians; Rust `lon_deg` maps to `lon` directly, mod 360), `surfaceRadiusAt` (`PlanetView.ts:1963`), Task 3 (`dustGain`).
- Produces:
  - `export const JETS_PER_FRACTURE = 24;` — about a hundred jets along Enceladus's four stripes (Porco et al. 2014, doi:10.1088/0004-6256/148/3/45).
  - `export const JET_HEIGHT_RADII = 0.3;` (look constant).
  - `export function fracturePoints(f: Fracture, radiusKm: number, n: number): LonLat[]` — `n` points evenly along the segment (same destination formula as Rust `Fracture::ends`, in the viewer's lon/lat), ends excluded.
  - `export function jetSources(g: Geysers, radiusKm: number): LonLat[]`.
  - Jets: one instanced, additively blended, camera-facing column per source, rising along the local normal from the surface (relief included), alpha × `dustGain` from the sun direction and the eye; hidden unless the globe is shown (`morph > 0.99`).

- [ ] **Step 1: Failing tests**

```ts
it('puts the jets on the fracture, between its ends', () => {
  const f = { lat_deg: -80, lon_deg: 30, azimuth_deg: 90, length_km: 130 };
  const pts = fracturePoints(f, 252.1, 24);
  expect(pts).toHaveLength(24);
  const mid = pts[11];
  expect(Math.abs(mid.lat * 180 / Math.PI + 80)).toBeLessThan(0.5);
});
it('handles the fracture through the pole and the seam', () => {
  const pole = fracturePoints({ lat_deg: -90, lon_deg: 45, azimuth_deg: 0, length_km: 130 }, 252.1, 24);
  expect(pole.every(p => p.lat < -1.2)).toBe(true);
  const seam = fracturePoints({ lat_deg: -82, lon_deg: 179, azimuth_deg: 90, length_km: 130 }, 252.1, 24);
  expect(seam.every(p => Number.isFinite(p.lon))).toBe(true);
});
it('gives no jets without geysers', () => {
  expect(jetSources({ south: true, fractures: [] }, 252.1)).toEqual([]);
});
```

- [ ] **Step 2: Run** `cd web && npx vitest run src/lib/viewer/jets.test.ts`, expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cd web && npm run check && npm test`, then the full Rust run (Global Constraints), expect PASS.
- [ ] **Step 5: Docs** — `web/README.md`: plume in the host's sky, jets in the close-up, both from `PlanetModel.geysers`; README line 131 one sentence pointing to them.
- [ ] **Step 6: Commit** — `feat(web): raise jets from the geyser fractures`

## After the tasks

- `branch-reviewer` on the whole branch against this plan and the spec.
- Rebuild `develop` with this branch (`develop-rebuilder`, peers warned), deploy (`develop-deployer`), then `viewer-visual-check`: Enceladus close-up (jets over the stripes, uncratered cap), Saturn view with Enceladus in the sky back-lit and side-lit (plume), Saturn view back-lit (E ring), Enceladus's and Titan's skies (E ring). Spec acceptance: fractures at the stored positions, plume and jets above them.
- The stored systems must be regenerated (world format 25) for the stripes to show.
