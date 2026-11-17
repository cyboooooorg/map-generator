# Tide Spheres Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Draw four nested tide spheres (Hill, locking, heating, fluid
Roche) round the host in the planet view, their radii computed in Rust
and served at read time.

**Architecture:** A new `crates/mapgen/src/tides.rs` computes each body's
`TideZones` from the stored system with the generator's own formulas. The
server adds a `tides` map to `SystemDetail` (`GET /api/systems/{id}`).
The web app stores it beside the extras, and the viewer only converts km
to radii of the body shown and draws one grid and one limb-lit shell per
sphere, under a `parted` "Tides" toggle.

**Tech Stack:** Rust (mapgen, axum server, ts-rs), Svelte 5 + TypeScript,
Three.js.

**Spec:** `docs/superpowers/specs/2026-11-17-tide-spheres-design.md`

## Global Constraints

- No change to `system.json`, `SYSTEM_FORMAT_VERSION` or world
  `FORMAT_VERSION`, and no change to generated worlds: the
  `world-coherence` byte-identity of generation must hold.
- No tidal physics in TypeScript. TS may divide a km radius by a body
  radius, and nothing more.
- Reuse the generator's formulas and constants by reference
  (`moons::despin_gyr`, `heat::tidal_power_w`, `heat::radiogenic_w`,
  `heat::k2_over_q`, `heat::ICY_K2_OVER_Q`, `heat::ROCKY_K2_OVER_Q`,
  `rings::fluid_roche`). Never copy their values. mapgen1's lot 3C-3
  (#540) will change `ROCKY_K2_OVER_Q` and `tidal_power_w`, and the
  spheres must follow.
- Do not edit `crates/mapgen/src/system.rs` or `crates/mapgen/src/planet/*`.
  mapgen1 is changing them on `feat/climate-lot3c`.
- Tests are throttled on this shared machine:
  `RAYON_NUM_THREADS=4 cargo test -j 4 -p mapgen --features ts -- --test-threads=3`.
  For the web, use `npm test` inside `web/` (no `npm ci` in the worktree,
  and never port 3000).
- Regenerate `web/src/lib/api/types` through `cargo test` and commit it.
  CI checks it with `git diff --exit-code`.
- Commits follow Conventional Commits, scopes `mapgen`, `server` and
  `web`. Each commit ends with the session's attribution lines.

## Review Focus

1. **A system loaded from its file alone** (`fetchSystem`'s fallback to
   `/systems/<id>/system.json`, `web/src/lib/world.svelte.ts:476`) has
   no `tides`. The Tides toggle is then absent and nothing throws. Task 3
   tests it.
2. **A moon whose host's radius exceeds a sphere's radius.** A dense moon
   round a puffy giant can have its Roche limit below the cloud tops.
   That sphere is `None` and its square hidden. Task 1 tests it.
3. **A circular orbit** (`e` below `1e-4`): `heating_km` is `None`, and
   there is no division by zero or infinite radius. Task 1 tests it.
4. **A moon without a model and without `bulk`** (a giant-host-chain
   moon): k₂/Q comes from `body_type`, the rock share defaults to 1. The
   result is finite. Task 1 tests it on a drawn Jupiter's moons.
5. **Switching from a moon to a planet and back**: the spheres follow the
   body shown, and no stale sphere stays drawn. Task 5 checks it in the
   visual check.

---

### Task 1: `mapgen::tides`, the radii (hard: physics, run on opus)

**Files:**
- Create: `crates/mapgen/src/tides.rs`
- Modify: `crates/mapgen/src/lib.rs` (add `pub mod tides;`)
- Modify: `crates/mapgen/src/moons.rs`: make `MOON_INERTIA` `pub(crate)`.
  `despin_gyr` is already `pub(crate)`. Change no formula.

**Interfaces:**
- Produces:

  ```rust
  #[derive(Clone, Debug, PartialEq, Serialize)]
  #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
  pub struct TideZones {
      /// The body the spheres are centred on: the moon's planet, or the planet itself.
      pub host: String,
      /// The body that feels the tide: the moon shown, or a planet's most
      /// massive moon; `None` for a moonless planet's typical satellite.
      pub satellite: Option<String>,
      pub hill_km: f64,
      pub locking_km: Option<f64>,
      pub heating_km: Option<f64>,
      pub roche_km: Option<f64>,
  }
  pub fn hill_km(a_km: f64, eccentricity: f64, mass_kg: f64, primary_kg: f64) -> f64;
  pub fn locking_km(mass_kg: f64, radius_km: f64, density: f64, host_kg: f64, age_gyr: f64) -> f64;
  pub fn heating_km(host_kg: f64, radius_km: f64, eccentricity: f64, k2_over_q: f64, radiogenic_w: f64) -> Option<f64>;
  pub fn roche_km(host_radius_km: f64, host_density: f64, density: f64) -> f64;
  pub fn tide_zones(system: &System, body_id: &str) -> Option<TideZones>;
  pub fn system_tides(system: &System) -> BTreeMap<String, TideZones>;
  ```

  Each doc comment names the formula and its source, as the generator
  does.

Rules for `tide_zones` (from the spec):

- **Moon** (`parent` is `Some`): `host` is its parent. The moon's own
  mass, radius, density and orbit `e` are used against
  `host_kg = parent.physical.mass_earth × EARTH_KG`. Hill is the
  parent's, round its primary: the parent's own parent if any, else the
  star, `star.info.mass × SUN_KG`, with the parent's `orbit`.
- **Moon**: `satellite` is `Some(own id)`.
- **Planet with moons** (bodies whose `parent` is the planet): `host` is
  itself. Every field equals `tide_zones` of its most massive moon
  (largest `physical.mass_earth`), `satellite` naming that moon. The
  moon's Hill is its parent's, so Hill is the planet's own.
- **Planet without moons**: `host` is itself, `satellite` is `None`.
  Hill is its own. Roche uses density `3.0` g/cm³ (the typical rocky
  satellite). Locking uses the Moon: `7.342e22` kg, `1737.4` km,
  `3.344` g/cm³. Heating is `None`.
- `hill_km = a (1 − clamp(e, 0, 0.95)) cbrt(m / 3M)`.
- `locking_km`: `a₀ (age / despin_gyr(m, R, ρ, MOON_INERTIA, M, a₀))^(1/6)`
  with any `a₀` (use `1e5` km).
- `heating_km`: `None` when `e < 1e-4`. Otherwise
  `a₀ (tidal_power_w(M, R_m, a₀_m, e, k2q) / radiogenic_w)^(2/15)`, with
  `a₀` in metres for `tidal_power_w` and the result converted back to km.
  `k2q` is `heat::k2_over_q(model.inputs.envelope_fraction,
  model.inputs.water_fraction)` when there is a model. Without one, it is
  `ICY_K2_OVER_Q` for `body_type == "icy-moon"` and `ROCKY_K2_OVER_Q`
  otherwise. `radiogenic_w` is `model.heat.radiogenic_w` when there is a
  model. Without one, it is
  `heat::radiogenic_w(mass_kg × bulk.rock, 1.0, age)`, with
  `bulk = physical.bulk.unwrap_or_default()`.
- `roche_km = fluid_roche(host_density, density) × host_radius_km`.
- Any sphere whose radius is not finite or not larger than the host's
  `radius_km` becomes `None`. Hill stays a plain `f64`: a body always
  has one.
- `system_tides` maps every body id to `tide_zones` (the bodies that
  return `None` are left out).

- [ ] **Step 1: Write the failing tests** in `tides.rs` `mod tests`.
  Use real numbers and a 10 % tolerance unless stated otherwise:
  - `earths_hill_sphere_is_a_million_and_a_half_km`:
    `hill_km(1.496e8, 0.0167, 5.972e24, 1.989e30)` is within 10 % of
    `1.47e6`.
  - `jupiters_hill_sphere_is_fifty_million_km`:
    `hill_km(7.785e8, 0.0489, 1.898e27, 1.989e30)` is within 10 % of
    `5.03e7`.
  - `the_moon_and_the_galilean_moons_lie_inside_their_locking_sphere`:
    - the Moon, with `locking_km(7.342e22, 1737.4, 3.344, 5.972e24, 4.5)`,
      gives a value above `384_400`;
    - for Io (`8.93e22`, `1821.6`, `3.53`, a = `421_700`), Europa
      (`4.80e22`, `1560.8`, `3.01`, `671_034`), Ganymede (`1.482e23`,
      `2634.1`, `1.94`, `1_070_412`) and Callisto (`1.076e23`, `2410.3`,
      `1.83`, `1_882_709`) round `1.898e27` kg at `4.5` Gyr, the value is
      above each one's `a`.
  - `locking_grows_as_the_sixth_root_of_age`: the ratio of
    `locking_km(…, 4.0)` to `locking_km(…, 4.0 / 64.0)` is `2.0 ± 1e-9`.
  - `io_lies_inside_jupiters_heating_sphere`:
    `heating_km(1.898e27, 1821.6, 0.0041, ROCKY_K2_OVER_Q, rad)`, with
    `rad = radiogenic_w(8.93e22, 1.0, 4.5)`, is `Some(r)` with
    `r > 421_700`.
  - `a_circular_orbit_has_no_heating_sphere`: `heating_km(…, 0.0, …)` and
    `heating_km(…, 5e-5, …)` are `None`.
  - `saturns_ice_roche_limit_is_about_two_saturn_radii`:
    `roche_km(60_268.0, 0.687, 0.93)` over `60_268` lies in `2.0..2.5`.
  - `drawn_systems_give_finite_nested_zones`: for seeds 1 to 4, draw
    `crate::system::draw` with a `WholeSystemRequest` of
    `[("earth-like", 1), ("jupiter-like", 1)]` (build it as
    `system.rs`'s test `whole` does, G2V, 64×32). Then for every body,
    `tide_zones` is `Some`. Every radius present is finite and larger
    than the host's `radius_km`. When `locking_km` and `roche_km` are
    both present, `roche_km < locking_km`. A moon's `host` equals its
    `parent` and its `satellite` is its own id. A planet with moons has
    the zones of its most massive moon (`satellite` is that moon's id,
    every field equal), and a moonless one has `satellite: None` and
    `heating_km: None`.
  - `spheres_inside_the_host_are_dropped`: take a body from such a drawn
    system, set its host's `radius_km` to `1e9` in a clone, and check
    that every `Option` field is `None`.
  - `stored_locking_agrees_with_the_locking_sphere`: on the drawn
    systems, every moon with `spin.locked` has `locking_km` at or above
    its `orbit.semi_major_axis_km` (within 1 %), and every unlocked
    disk-born moon has it below.

- [ ] **Step 2: Run them and check they fail**

  Run: `RAYON_NUM_THREADS=4 cargo test -j 4 -p mapgen --features ts tides:: -- --test-threads=3`

  Expected: compile errors, because `tides` does not exist yet.

- [ ] **Step 3: Implement `tides.rs`** with the interface and rules
  above. `EARTH_KG` comes from `crate::exomoon::EARTH_KG`. `SUN_KG` is a
  local `const SUN_KG: f64 = 1.988_5e30;` (the value of
  `planet/mod.rs:386`).

- [ ] **Step 4: Run the tests and check they pass.** Use the same
  command. Then run the whole mapgen suite, which must stay green with
  no world changed.

- [ ] **Step 5: Commit.** Include the regenerated
  `web/src/lib/api/types/TideZones.ts`.

  ```bash
  git add crates/mapgen/src/tides.rs crates/mapgen/src/lib.rs crates/mapgen/src/moons.rs web/src/lib/api/types/TideZones.ts
  git commit -m "feat(mapgen): compute the tide spheres of a system's bodies"
  ```

### Task 2: serve the tides with the system

**Files:**
- Modify: `crates/server/src/api.rs:497` (`SystemDetail`)
- Modify: `crates/server/src/app.rs:879` (`get_system`)
- Test: `crates/server/src/app.rs` `mod tests`
- Regenerated: `web/src/lib/api/types/SystemDetail.ts`

**Interfaces:**
- Consumes: `mapgen::tides::{TideZones, system_tides}`.
- Produces: `SystemDetail.tides: BTreeMap<String, mapgen::tides::TideZones>`,
  documented as "each body's tide spheres (`mapgen::tides`), by body id,
  derived at read time". `ResolvedSystem` is unchanged: the form's
  preview draws no spheres.

- [ ] **Step 1: Write the failing test**
  `a_system_detail_carries_its_bodies_tides`. Use the helper of the test
  at `app.rs:3038` (`generated_planet`) to draw an `earth-like`, then
  `GET /api/systems/{id}`, and assert the following:
  - `detail["tides"][focus]["host"] == focus`;
  - `detail["tides"][focus]["hill_km"]` is a number above 1e5.

- [ ] **Step 2: Run it and check it fails.** Run
  `cargo test -j 4 -p server a_system_detail_carries -- --test-threads=3`.
  Expected: FAIL, because `tides` is null.

- [ ] **Step 3: Add the field** and fill it with
  `mapgen::tides::system_tides(&system)` in the same `spawn_blocking`.

- [ ] **Step 4: Run the server tests, then `cargo test --all-features`
  throttled as above.** Check that they pass and that `SystemDetail.ts`
  is regenerated.

- [ ] **Step 5: Commit**: `feat(server): serve each body's tide spheres with its system`.

### Task 3: the web app keeps the tides; Hill leaves TypeScript

**Files:**
- Modify: `web/src/lib/world.svelte.ts:100,176-181,450-477`
- Modify: `web/src/lib/systemSky.ts`: delete `hillRadiusKm`, `hillSphere`
  and `EARTH_PER_SUN` if nothing else uses it. Keep `hillReach`.
- Modify: `web/src/lib/systemSky.test.ts:342-382`: drop the deleted
  functions' tests and keep `hillReach`'s.
- Test: `web/src/lib/world.svelte.test.ts`

**Interfaces:**
- Consumes: `TideZones` from `web/src/lib/api/types/TideZones.ts`.
- Produces: `fetchSystem` also returns
  `tides: Record<string, TideZones>` (`{}` when the route is absent or
  omits it); `world.systemTides: Record<string, TideZones>` (`$state`,
  set beside `systemExtras` and cleared where those are);
  `world.shownTides: TideZones | null` (`$derived`, like `shownExtras`).

- [ ] **Step 1: Write the failing tests** in `world.svelte.test.ts`,
  next to `keeps the extras the route sends…` (line 35):
  - `keeps the tides the route sends`: the route answers
    `{ system, tides: { 'd-1': { host: 'd', satellite: 'd-1', hill_km: 1e6, locking_km: 5e5,
    heating_km: null, roche_km: 2e4 } } }`. Then
    `world.systemTides` equals it, and `world.shownTides` is that entry
    once `d-1` is shown.
  - `has no tides from the file alone`: the route fails and the file
    serves. Then `world.systemTides` is `{}` and `world.shownTides` is
    `null`.

- [ ] **Step 2: Run and check they fail.** Run
  `cd web && npx vitest run src/lib/world.svelte.test.ts`.

- [ ] **Step 3: Implement it.** Delete the TS Hill functions. The build
  breaks in `Viewer.svelte` until Task 5, so keep a temporary
  `const hill = null` there if it's needed to type-check, and remove it
  in Task 5. Better: do Tasks 3 to 5 before running `npm run check`.

- [ ] **Step 4: Run the vitest file and `systemSky.test.ts`.** Expected:
  PASS.

- [ ] **Step 5: Commit**: `feat(web): keep each body's tide spheres from the server`.

### Task 4: `PlanetView` draws a list of spheres

**Files:**
- Modify: `web/src/lib/viewer/PlanetView.ts`. Replace `hill`,
  `hillSphereOn` and `setHillSphere` (≈ lines 714-718, 2479-2482,
  2843-2886, 3851-3862).

**Interfaces:**
- Produces:

  ```ts
  export interface TideSphere { centre: Vec3; radius: number; colour: readonly [number, number, number] }
  setTideSpheres(spheres: readonly TideSphere[]): void
  ```

  `radius` is in radii of the body shown, and `centre` is in its frame,
  as `setHillSphere` took them.

- [ ] **Step 1: Generalise the existing code.** Build one `{ grid, shell }`
  pair per list entry, as `setHillSphere` builds its one. The pairs are
  pooled: grow the pool when the list grows, and hide the extra pairs
  when it shrinks. The colour becomes a uniform or material colour per
  pair. The shell's fragment shader takes `uniform vec3 uColour` instead
  of the literal `vec3(0.55, 0.85, 0.8)`.
  - Per-frame visibility, position, scale, `0.3 × morph³` grid opacity
    and `morph³` shell opacity are unchanged, for each pair.
  - Dispose of every pair where `hill` was disposed.
  - Grids: keep `sphereGrid(1, 96, 15)` for the outermost sphere (the
    first in the list). Inner spheres use `sphereGrid(1, 48, 30)`, half
    as dense, so that four nested grids stay readable. Tune the
    density on the visual check.
- [ ] **Step 2: Type-check.** Run `cd web && npx tsc --noEmit -p .`, or
  `npm run check`, after Task 5.
- [ ] **Step 3: Commit**: `refactor(web): draw a list of tide spheres in the planet view`.

### Task 5: the Tides toggle, i18n and docs

**Files:**
- Modify: `web/src/components/Viewer.svelte:58` (imports), `:205` and
  `:330` (state), `:541` (saved), `:1593-1601` (derived and effects),
  `:1940-1955` (toggle).
- Modify: `web/src/lib/i18n/en.ts:993-995` and `fr.ts:939-941`. Put the
  new keys under `viewer`, next to `hillSphere`. mapgen1 is editing other
  i18n blocks, so keep yours contiguous.
- Modify: `web/README.md:47` (the **Hill sphere** entry becomes
  **Tides**).

**Interfaces:**
- Consumes: `world.shownTides`, `setTideSpheres`, `TideSphere`,
  `hillReach`.

- [ ] **Step 1: State.** Keep the saved key `hillSphere` for the Hill
  square, so users' saved choice carries over with no migration. Add the
  saved keys `tideLocking`, `tideHeating` and `tideRoche`, all `false`
  by default, each with a `$state` and saved like `hillOn`.
- [ ] **Step 2: Derived spheres.** Replace `hill` and `hillDrawn` with
  `tideSpheres: TideSphere[]`:
  - the centre is computed exactly as `hillDrawn` computes it today, with
    `tides.host` in place of `hill.id`;
  - `radius = km / sysBody.physical.radius_km`;
  - the list is ordered outside in (Hill, locking, heating, Roche) and
    holds the spheres whose square is on and whose km is not null.

  Colours: Hill `[0.55, 0.85, 0.8]`, locking `[0.45, 0.6, 1.0]`, heating
  `[1.0, 0.6, 0.25]`, Roche `[1.0, 0.35, 0.3]`.

  Effects: `view?.setTideSpheres(tideSpheres)`, and the globe reach is
  the max of `reach` and `hillReach(s.centre, s.radius)` over the list.
- [ ] **Step 3: The toggle.** Replace the `key: 'hill'` entry with
  `parted({ key: 'tides', label: t.viewer.tides, help: t.viewer.tidesHelp,
  icon: 'zone' }, [hill, locking, heating, roche], false)`. Each part's
  `shown` is "its km is not null", and Hill's is "there are tides".
  Icons: Hill `orbit`, locking `clock`, heating `sun`, Roche `belt`. Each
  part's help gives its radius in km and in host radii, with
  `withUnit(formatInt(km), 'km')` and `formatInt(km / host.physical.radius_km)`.
  The host comes from `sys.bodies` by `tides.host`, and its name comes
  from `world.systemNames`, as in `hillSphereHelp` today.
- [ ] **Step 4: i18n**, en and fr, with the same keys:
  - `tides`: "Tides" / "Marées".
  - `tidesHelp`: the nested regimes of the host's tides on the body
    shown.
  - Keep `hillSphere` and `hillSphereHelp`.
  - `tideLocking`, `tideLockingHelp({ host, body, radius, radii })`:
    within it, `host`'s tides have despun `body` over the system's age.
  - `tideHeating`, `tideHeatingHelp(…)`: within it, `body`'s tidal heat
    exceeds its radioactive heat.
  - `tideRoche`, `tideRocheHelp(…)`: within it, a fluid body as dense as
    `body` is torn apart.

  `body` is the name of `tides.satellite` (from `world.systemNames`, else
  its designation). When it is null (a moonless planet), it reads "a
  Moon-like satellite" or "a rocky satellite" (en), and « un satellite
  comme la Lune » or « un satellite rocheux » (fr). Match the style of the strings around them, including
  the `${S}` spacing helper in `fr.ts`.
- [ ] **Step 5: Docs.** In `web/README.md:47`, describe the Tides switch
  with its four squares, what each sphere means, that the radii come
  from the server (`mapgen::tides`), and that a system read from its
  file alone has none.
- [ ] **Step 6: Check.** Run `cd web && npm run check && npm test`.
  Expected: no type errors and every test green.
- [ ] **Step 7: Commit**: `feat(web): replace the Hill sphere switch with tide spheres`.

### Task 6: whole-branch review and visual check

- [ ] `branch-reviewer` reviews the branch against the spec and this
  plan.
- [ ] `viewer-visual-check` looks at the following, with the
  screenshots downscaled:
  - a drawn Jupiter-like system: a Galilean-like moon with all four
    squares on, then the planet itself;
  - an Earth-like planet;
  - switching moon to planet to moon (Review Focus 5);
  - the grids' readability from inside.
- [ ] File a backlog issue: the star view's Hill circles
  (`starSystem.ts:391` `hillRadiusAu`, `StarView.ts:1209`) still compute
  Hill in TS. Its giants are not keyed by body id, so moving them to the
  server's value is left for later.
