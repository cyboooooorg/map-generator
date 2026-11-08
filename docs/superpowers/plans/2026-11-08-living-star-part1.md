# Living Star (Part 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each system's star gets a generated activity, and that activity drives a surface that changes deterministically with the system clock (granulation, active regions, coronal loops). The star is shown in a new star view and is the same star in the system view and in a planet's sky.

**Architecture:** Rust derives the star's activity parameters once per system (`crates/mapgen/src/activity.rs`) and stores them on `SystemStar`. They are derived on read for systems drawn before. The browser turns those parameters and the clock's day into the active regions present at that day (`web/src/lib/starActivity.ts`, a pure function of seed and day). One star-surface module (`web/src/lib/viewer/starSurface.ts`: a GLSL chunk, its uniforms and their update) draws that surface in the system view (`StarView`), the new star view (`StarBodyView`) and the sky disc (`PlanetView`'s host star). Coronal loops stand over the regions (`coronalLoops.ts`) and replace the seed-placed prominences.

**Tech Stack:** Rust (mapgen, serde, ts-rs), TypeScript (strict), Svelte 5, Three.js 0.187, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-08-living-star-design.md` (part 1, issue #343. Parts 2 (#344) and 3 (#345) build on the data model fixed here).

## Global Constraints

- Branch per PR from `main` (`feat/living-star-…`). PR descriptions say `Refs #343`, except the last PR of this plan, which says `Closes #343`.
- Conventional Commits. Footers on every commit:
  `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01NRxWVfE7EVNu9UQ4iLcsAp`.
- English in code, comments, docs and commits. Every user string goes in both `web/src/lib/i18n/en.ts` and `fr.ts`.
- Docs ship in the same change as the behaviour they describe (README.md, web/README.md, docs/system-spec.md, docs/science/).
- Rust tests: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`. This run also rewrites the ts-rs bindings in `web/src/lib/api/types/` (`.cargo/config.toml` `TS_RS_EXPORT_DIR`). Commit the regenerated bindings.
- Web checks, from `web/`: `npm run check` and `npx vitest run`. Never run `npm ci` and never run prettier.
- No new dependency, in Rust or in npm. The frame-time measurement is built in, not Playwright.
- `SYSTEM_FORMAT_VERSION` stays 1 and no generated file is regenerated. Activity is a new optional field. When it is absent it is derived, not drawn again.
- **One clock.** The surface uses the system clock in Earth days from the system's t = 0, the same clock the orbits run on (`StarView.systemDay`, `Viewer.svelte`'s `sysDays`). Days are JS numbers (f64). They may be negative or as large as 1e7. Phases are reduced in JS (mod 2π, or relative to a recent epoch) before they reach a shader. GLSL never sees an absolute day.
- **Determinism.** The surface at a given day is a function of `StarActivity` (which carries the seed) and the day only. It never depends on frame history, wall time or the path taken to reach the day.
- **Frame-rate anti-goal.** With the new options off, no existing view's frame time changes measurably. With everything on, each view stays smooth on an Apple-silicon Mac at 1920×1080 and 2560×1440. Regions reach the GPU as a small uniform array, repacked only when the set changes. Granulation is procedural in the shader. Detail falls with the star's size on screen. Anything that is off or out of view costs nothing.
- Every relation cites its source (author, year, journal or doi) in a doc comment and in `docs/science/star-activity.md`. A number with no source is labelled **calibration** in both places, as `star.rs` and `giants.rs` do. Before using a number marked **verify** below, the implementer checks it against the source (the abstract or the paper). If the source gives a different value, the implementer uses the source's value and updates the plan's test bounds in the same commit.
- PR #342 (`feat/normal-map-relief`) edits `PlanetView.ts`, `StarView.ts`, `Viewer.svelte`, `systemSky.ts` and `shaders.ts`. Keep this plan's edits outside its hunks, as listed under "Overlap with PR #342" below. Whichever PR merges second rebases on `main`.

## Review Focus

- **Scrubbing time.** Reach a day going forward, backward, by a jump, at a negative day (−3 650) or at a far day (1e7). The regions and the surface must be identical, finite and in range every time. Task 2's tests cover this.
- **Systems without activity.** A system drawn before this change has no `activity` field, and so does a system loaded through the raw `system.json` fallback in `world.svelte.ts` `fetchSystem`. Such a system must open with the derived activity (Rust), or with a static, limb-darkened star and no crash (web). Task 1's `legacy_system_gets_activity_on_read` and Task 4's `starLook falls back without activity` cover this.
- **Extremes.** A radiative A star must have no regions and no loops. A saturated, fully convective M dwarf at full activity must still draw at most `quality.starRegions` regions, at a bounded cost per frame. Task 1's `hot_star_is_quiet` / `fully_convective_dwarf_is_active` and Task 2's `caps the regions, largest first` cover this.
- **Clock speeds.** At the system view's fastest speed (2¹² days/s) spots and granules must not strobe. They fade, as in a long exposure. At the star view's slowest speed the granules must visibly boil. Task 4's `exposureFade` tests cover this.
- **Switching views.** Switching system view ↔ star view repeatedly, and changing system, must carry the same day both ways. The hidden view must run no animation loop. `#system=<id>&body=star` must open the star view, and links without `body` must still open the system view. Task 6's `starClock` and `world.svelte` tests cover this.

## Decisions This Plan Fixes

These are calls the design left to the builder. Implementers do not reopen them.

1. **The region timeline runs in TypeScript only. Rust supplies the parameters.** The browser needs the region set at an arbitrary day on every frame, at any clock speed. The project has no wasm, and no Rust consumer needs regions in part 1. Part 2's ejections also leave from regions in the browser. Rust therefore stores `StarActivity` (which includes a `seed: u32`, safe as a JS number), and `starActivity.ts` derives the regions from it. Duplicating the timeline in Rust with shared test vectors would double the code for no consumer. If a Rust consumer appears later, it ports the function, and `starActivity.test.ts`'s pinned vectors become the shared vectors.
2. **Convective boundary at 7 200 K.** Stars at or above 7 200 K are `Radiative`. The source is the granulation boundary of Gray & Nagel 1989 (ApJ 341, 421, **verify** the temperature they give for the boundary). X-ray activity also falls off near B−V ≈ 0.3 (Schmitt 1997, A&A 318, 215). Granulation contrast fades from 6 800 K to 7 200 K so neighbouring types do not jump. The fade is a **calibration**. Today's `starSurface` uses 7 500 K, and that value goes away.
3. **Fully convective below 0.35 M☉.** Source: Chabrier & Baraffe 1997, A&A 327, 1039.
4. **"Star view" naming in code.** The design's star view is the class `StarBodyView` (`web/src/lib/viewer/StarBodyView.ts`) inside `StarSurfacePane.svelte`. The existing `StarView`/`StarPane` remain the *system view*. In the UI the new view is labelled "Star" / "Étoile" and the system view "System" / "Système".
5. **Clock hand-off.** `Viewer.svelte` holds `starDays`, the day shown by the system view and the star view, and both panes bind it, so switching between them keeps the day. Opening the system view from a planet sets `starDays` to the planet view's `sysDays`, both counted from the system's epoch (Task 6 verifies this). Going back to the planet view does not move the planet view's season-driven clock. Syncing that clock is out of scope for part 1: open a follow-up issue labelled `backlog` and cite it in the last PR.
6. **Speeds.** There is one `speed` per pane, a power of two of days/s. The system view keeps `SYSTEM_SPEED_MIN..MAX` (0..12). The star view spans −12..6 and opens at `defaultStarSpeed`, chosen so that one granule lifetime lasts about 10 s on screen. Switching between the views clamps the speed into the target view's range.
7. **What leaves in part 1.** The seed-placed prominences (`prominences()`) and the decorative M-dwarf flare (`uFlares`, `StarSurface.flares`) are removed. Flares come back with ejections in part 2 (#344). The periodic blow-off (`animateProminences`'s eruption) stays as an interim effect until part 2, but it now lifts a loop of the largest current region instead of a random arc.
8. **Worlds without a system.** Since #290 every world lives in a system
   and `/api/worlds` is gone, so there should be no lone world left.
   Task 4 checks this on main first (`#world=` links in `link.ts`, the
   `fetchSystem` fallback): if a world can still open without a system,
   it gets a static, limb-darkened star with no regions, no loops and no
   star-view button; if none can, do not write that branch (the user does
   not want code kept for older data).
9. **Star orientation.** The star's spin axis is the system frame's +y, the normal of the reference plane that `placeSystem` uses (`IDENTITY_AXES`). This is also the scene's +y in `StarView` and `StarBodyView`. Regions are placed in that inertial frame. Longitude 0 is the system's +x at day 0.
10. **Data room for parts 2 and 3.** Each region has a stable `id`, a `polarity` (Hale's law: it flips each cycle and is opposite in the two hemispheres), a `tilt` (Joy's law), a `separation` and a `peakArea`. These are what CMEs (part 2) and field lines and loop orientation (part 3) need. `StarActivity` gains fields later only as new optional serde fields, derived when absent, under the same rule as here.

## Science: Relations and What to Verify

These relations are implemented in Task 1, `crates/mapgen/src/activity.rs`, unless a line says otherwise. Each line gives the relation, its source, and the numbers to verify.

- **Age:** `giants::system_age_gyr(info.mass)`, unchanged. The seeded scatter goes on rotation, not on age.
- **Rotation, FGK (convective, M ≥ 0.35, Teff < 6 250 K):** gyrochronology `P = a (B−V − c)^b t^n` (t in Myr), with the coefficients of Mamajek & Hillenbrand 2008 (ApJ 687, 1264, eq. 12–14). **Verify** a = 0.407, b = 0.325, c = 0.495, n = 0.566. B−V comes from Teff by inverting Ballesteros 2012 (EPL 97, 34008), `T = 4600 (1/(0.92 BV + 1.7) + 1/(0.92 BV + 0.62))`, solved by bisection on [−0.3, 2.0]. B−V is clamped to [0.45, 1.6], and values beyond M&H's calibrated 0.5–0.9 are labelled extrapolation. Then multiply by the seeded scatter `exp(N(0, 0.15))`. The 0.15 is a **calibration**, from the 15–20 % gyro-age spread in Barnes 2007 (ApJ 669, 1167).
- **Rotation above the Kraft break (6 250 K ≤ Teff < 7 200 K):** stars here are not magnetically braked (Kraft 1967, ApJ 150, 551). P = 2πR / v, with v ramping linearly from the gyro value at 6 250 K to 100 km/s at 7 200 K. This is a **calibration**.
- **Rotation, radiative (Teff ≥ 7 200 K):** v = 150 km/s, P = 2πR / v. Royer et al. 2007 (A&A 463, 671) give typical A-star v sin i (**verify** that ~100–200 km/s is typical).
- **Rotation, fully convective (M < 0.35):** use Engle & Guinan 2018 (RNAAS 2, 34), the age–rotation relation for M2.5–6.5, inverted for P(age). **Verify** the coefficients, and clamp to [0.2, 150] d.
- **Turnover time:** `log10 τ_c = 2.33 − 1.50 M + 0.31 M²` (days, M in M☉, valid 0.08–1.36). Source: Wright et al. 2018 (MNRAS 479, 2351, eq. 6). **Verify**: τ_c(1 M☉) = 13.8 d. Clamp M into the valid range. Then `rossby = P / τ_c`.
- **Activity:** `activity = min(1, (Ro / 0.13)^−2.70)`, the ratio (Lx/Lbol)/(Lx/Lbol)_sat. Source: Wright et al. 2011 (ApJ 743, 48). **Verify** Ro_sat = 0.13 and β = −2.70 (the unbiased-sample slope). Fully convective stars follow the same relation (Wright et al. 2016, Nature 535, 526; 2018). Radiative stars get 0.
- **Spot coverage at cycle maximum:** `spot_coverage = min(0.3, 0.003 (activity / activity_sun)^0.64)`, where activity_sun is the value of this model for the Sun (1 M☉, 25.4 d). This is a **calibration** anchored on two points. The Sun's maximum spotted area is about 0.3 % of the hemisphere (Hathaway 2015, Living Rev. Sol. Phys. 12, 4; **verify**). Saturated stars reach about 30 % (Strassmeier 2009, A&ARv 17, 251).
- **Facula/spot area ratio:** `facula_ratio = clamp(10 (activity / activity_sun)^−0.32, 1, 15)`. This is a **calibration**. The trend that active stars are spot-dominated is from Shapiro et al. 2014 (A&A 569, A38) and Lockwood et al. 2007 (ApJ 664, 1234).
- **Cycle period:** use the inactive-branch line of Böhm-Vitense 2007 (ApJ 657, 486, Fig. 1) for Ro ≥ 1, and the active branch below. **Verify** both lines' coefficients, and check that the Sun lands at about 11 yr for 25.4 d. Apply a seeded scatter `exp(N(0, 0.15))` (**calibration**). Saturated stars (Ro < 0.13) get `cycle_depth` 0.3, and Ro ≥ 1 gets 0.9, linear in log Ro between them (**calibration**). Radiative stars get `cycle_years = 0`. `cycle_phase` is uniform from the seed.
- **Cycle shape (TS, Task 2):** the shape is `f(x) = x³ / (exp(x²) − 0.71)`, where x is the time since cycle start in units of the rise scale. This is the curve of Hathaway, Wilson & Reichmann 1994 (Sol. Phys. 151, 177). **Verify** c = 0.71. It is normalised to a peak of 1 and stretched to `cycle_years`. Region rate = `region_rate · ((1 − cycle_depth) + cycle_depth · f)`.
- **Active band (Spörer):** regions emerge at `band_deg[0]` at cycle start and drift to `band_deg[1]` by its end, exponentially in time, following the butterfly fit of Hathaway 2011 (Sol. Phys. 273, 221, **verify** the 28°-start, ~8°-end form). The Sun gets [30, 8]. Rapid rotators shift the band poleward by up to 40° as log Ro goes from Ro_sun to Ro_sat (Schüssler & Solanki 1992, A&A 264, L13 give the mechanism, Coriolis deflection; the amount is a **calibration**). Fully convective stars get [75, 0] (**calibration**). Doppler images of M dwarfs show spots at all latitudes (Morin et al. 2008, MNRAS 390, 567, **verify**). The scatter about the band centre is N(0, 4°) (**calibration**).
- **Joy's law:** tilt = 32.1° sin|λ| (Stenflo & Kosovichev 2012, ApJL 745, L129, **verify**). The leader spot is equatorward.
- **Region areas:** the peak spot area follows a lognormal (Bogdan et al. 1988, ApJ 327, 451, **verify** the mean and σ of ln A in MSH). The median is scaled by `(spot_coverage / 0.003)^0.5` (**calibration**). Areas are shares of the visible hemisphere (1 MSH = 1e−6).
- **Lifetimes:** the Gnevyshev–Waldmeier rule, `T = A_peak / W` with W = 10 MSH/day (Petrovay & van Driel-Gesztelyi 1997, Sol. Phys. 176, 249, **verify**). Growth lasts `min(0.1 T, 3 d)` (**calibration**). Area rises linearly over the growth time, then decays linearly. W is the same on every star (**calibration**: Giles et al. 2017, MNRAS 472, 1618, find longer lifetimes for larger spots, consistent with a constant W). The lifetime is capped at `MAX_LIFETIME_DAYS = 120`.
- **Region rate:** `region_rate = spot_coverage · 2W / E[A_peak²]`, so that the mean spotted area at cycle maximum equals `spot_coverage`. With a triangular area history, each region contributes A²/(2W) area-days. Derive this rate, do not set it independently. Task 2 tests it.
- **Bipole separation:** `separation = 4 √(2 A_peak)` radians (**calibration**, scale from Wang & Sheeley 1989, ApJ 347, 529: about 10° for a 1 000 MSH region). Coronal loop height = separation / 2, the semicircular-loop geometry (Aschwanden 2004, *Physics of the Solar Corona*, Springer, ch. 3).
- **Differential rotation:** `Ω(λ) = Ω_eq (1 − shear sin²λ)`. ΔΩ follows Barnes et al. 2005 (MNRAS 357, L1), ΔΩ ∝ Teff^8.92 (**verify** the exponent and the normalisation), with the solar value ΔΩ = 0.055 rad/d (**verify** against Snodgrass & Ulrich 1990, ApJ 351, 309). Then shear = ΔΩ / Ω_eq, clamped to [0, 0.5]. Reinhold et al. 2013 (A&A 560, A4) is the cross-check for the Teff trend.
- **Granulation size:** L = 1.3 Mm · (Teff / 5 772) / (g / g☉), with g / g☉ = M / R². Granules scale with the pressure scale height H_p ∝ T/g (Trampedach et al. 2013, ApJ 769, 18, **verify** both the proportionality and the solar 1.3 Mm). Store it as `granule_size` in star radii: L / (R · 695.7 Mm).
- **Granulation contrast:** 0.15 (Teff / 5 772)², capped at 0.25, with the 6 800–7 200 K fade. This is a **calibration** with the trend of Magic et al. 2013 (A&A 557, A26). **Granule lifetime:** 8 min · (L / L☉), a **calibration** using the solar value from Title et al. 1989 (ApJ 336, 475, **verify**).
- **Spot brightness and structure (shader, Task 4):** umbra 0.3 and penumbra 0.75 of the photosphere, with the penumbra 4× the umbra's area (Solanki 2003, A&ARv 11, 153, **verify**). The leader spot carries 0.6 of the area (**calibration**). Facular contrast is 0.25 · (1 − μ) (**calibration**, with the limb-brightening of faculae from Solanki 2003).
- **Limb darkening:** the quadratic law `I(μ) = 1 − u1 (1 − μ) − u2 (1 − μ)²`. The (u1, u2) pairs come from Claret & Bloemen 2011 (A&A 529, A75, Johnson V, log g = 4.5, [M/H] = 0, ATLAS for Teff ≥ 3 500 K). The table goes in `activity.rs` at Teff nodes 3 500, 4 000, 5 000, 5 750, 6 500, 7 500, 10 000 K, with values read from VizieR J/A+A/529/A75 (**verify** every entry), interpolated linearly in Teff. It replaces today's ad-hoc `0.6 + (5772 − Teff)/5000` and `0.2` in `starSurface`.

## File Map

| File | Change |
| --- | --- |
| `crates/mapgen/src/activity.rs` | Create: `StarActivity`, `Envelope`, `star_activity`, relations, tests |
| `crates/mapgen/src/lib.rs` | `pub mod activity;` |
| `crates/mapgen/src/noise.rs` | `stream::STAR_ACTIVITY = 40` |
| `crates/mapgen/src/system.rs` | `SystemStar.activity`, set in `assemble`, filled in `read_system` |
| `web/src/lib/api/types/{StarActivity,Envelope,SystemStar}.ts` | Regenerated by ts-rs |
| `web/src/lib/starActivity.ts` (+ `.test.ts`) | Create: cycle, rotation, region timeline |
| `web/src/lib/viewer/frameStats.ts` (+ `.test.ts`) | Create: frame-time probe |
| `web/src/lib/viewer/starSurface.ts` (+ `.test.ts`) | Create: shared GLSL chunk, uniforms, `StarSurfaceState`, LOD helpers |
| `web/src/lib/viewer/starShaders.ts` | `starSurfaceFragment` built on the chunk |
| `web/src/lib/viewer/starSystem.ts` (+ test) | Remove `starSurface`, `StarSurface`, `prominences`; keep `Prominence` |
| `web/src/lib/viewer/coronalLoops.ts` (+ `.test.ts`) | Create: loop arcs per region, `CoronalLoops` group |
| `web/src/lib/viewer/StarView.ts` | Shared surface, loops, frame probe, double-click on the star |
| `web/src/lib/viewer/StarBodyView.ts` | Create: the star view |
| `web/src/lib/starClock.ts` (+ `.test.ts`) | Create: speed ranges, clamp, cycle jumps |
| `web/src/components/StarSurfacePane.svelte` | Create: star view pane (dock, toggles, playback) |
| `web/src/components/StarPane.svelte` | `activity`, `clockDays`, `hidden` props; star-view button |
| `web/src/components/Viewer.svelte` | `starDays`, star mode, pane switching, sky surface effect |
| `web/src/lib/world.svelte.ts` (+ test) | `starMode`; `body=star` |
| `web/src/App.svelte` | Hash writes `body=star` in star mode |
| `web/src/lib/systemSky.ts` (+ test) | `star.axes` |
| `web/src/lib/viewer/hostStarSurface.ts` (+ `.test.ts`) | Create: sky disc basis and detail |
| `web/src/lib/viewer/shaders.ts` | `hostStarFragment` includes the chunk |
| `web/src/lib/viewer/PlanetView.ts` | `setHostStarSurface`, basis in `drawHostStar`, frame probe |
| `web/src/lib/viewer/quality.ts` (+ test) | `starRegions`, `loopRegions` |
| `web/src/lib/i18n/en.ts`, `fr.ts` | Star view, loops, activity labels |
| `docs/science/star-activity.md`, `docs/science/README.md` | Create the article; index it under "Star view" |
| `docs/system-spec.md`, `README.md`, `web/README.md` | Model, star view, measurement protocol |

---

### Task 1: Star activity model (Rust) and bindings

**Files:**

- Create: `crates/mapgen/src/activity.rs`
- Modify: `crates/mapgen/src/lib.rs`, `crates/mapgen/src/noise.rs` (after `PERIHELION = 39`), `crates/mapgen/src/system.rs:143-158` (`SystemStar`), `:1184-1190` (`assemble`), `:339-343` (`read_system`)
- Generated: `web/src/lib/api/types/StarActivity.ts`, `Envelope.ts`, `SystemStar.ts`
- Docs: create `docs/science/star-activity.md`, following `docs/science/README.md`'s "Writing an article" outline. It holds every relation of the science section above, with its sources and the calibration labels. Index it in `docs/science/README.md` under "Star view", and add the star's activity to `docs/system-spec.md` § Model (one paragraph: what is stored, that it is derived on read when absent).

**Interfaces:**

- Produces (Rust, serde snake_case, ts-rs exported under `feature = "ts"`):

```rust
pub enum Envelope { Radiative, Convective, FullyConvective }   // serde: "radiative" | "convective" | "fully_convective"

pub struct StarActivity {
    pub envelope: Envelope,
    pub age_gyr: f32,
    /// Equatorial sidereal rotation period, days.
    pub rotation_days: f32,
    /// Ω(λ) = Ω_eq (1 − shear sin²λ).
    pub shear: f32,
    pub turnover_days: f32,     // 0 when radiative
    pub rossby: f32,            // 0 when radiative
    pub activity: f32,          // (Lx/Lbol)/(Lx/Lbol)_sat, (0, 1]; 0 when radiative
    pub cycle_years: f32,       // 0: no cycle
    pub cycle_phase: f32,       // [0, 1) at day 0
    pub cycle_depth: f32,       // [0, 1]
    pub band_deg: [f32; 2],     // emergence latitude at cycle start, at cycle end
    pub spot_coverage: f32,     // share of the hemisphere under spots at cycle maximum
    pub facula_ratio: f32,
    pub region_rate: f32,       // new regions per day at cycle maximum
    pub region_area: f32,       // median peak spot area, share of the hemisphere
    pub region_sigma: f32,      // σ of ln(peak area)
    pub decay_per_day: f32,     // W, share of the hemisphere per day
    pub granule_size: f32,      // star radii
    pub granule_contrast: f32,
    pub granule_lifetime_days: f32,
    pub limb: [f32; 2],         // quadratic u1, u2
    pub seed: u32,              // the region timeline's seed (web)
}

/// The activity of a system's star, deterministic in `system_seed`.
pub fn star_activity(info: &StarInfo, system_seed: u32) -> StarActivity;

// In system.rs, on SystemStar:
#[serde(default, skip_serializing_if = "Option::is_none")]
#[cfg_attr(feature = "ts", ts(optional))]
pub activity: Option<StarActivity>,
```

- `star_activity` seeds `StdRng::seed_from_u64(u64::from(stream_seed(system_seed, stream::STAR_ACTIVITY)))`. The draws go in this order: rotation scatter, cycle scatter, cycle phase, `seed`. The order is fixed so that the values never change across versions.
- Keep the relations as private functions of (teff_k, mass, radius, age_gyr) so they can be tested without a `StarInfo`: `turnover_days(mass)`, `rossby_activity(ro)`, `gyro_period_days(teff_k, age_gyr)`, `bv_from_teff(teff_k)`, `envelope(teff_k, mass)`, `limb_quadratic(teff_k)`, `granule_size(teff_k, mass, radius)`.

- [ ] **Step 1: Write the failing tests** (in `activity.rs` `mod tests`; and in `system.rs` tests for the last two)

```rust
#[test] fn turnover_matches_wright_2018() { assert!((turnover_days(1.0) - 13.8).abs() < 0.3); }
#[test] fn activity_saturates_then_falls() {
    assert_eq!(rossby_activity(0.05), 1.0); assert_eq!(rossby_activity(0.13), 1.0);
    assert!((rossby_activity(1.3) - 10f32.powf(-2.70)).abs() < 1e-4);
}
#[test] fn sun_like_star_rotates_and_cycles_like_the_sun() {
    // G2V at 1 AU (StarInfo::new(parse_type("G2V").unwrap(), 1.0, 0.0)), 201 seeds, medians:
    // rotation_days in 22..30, rossby in 1.4..2.5, cycle_years in 8..14,
    // spot_coverage in 0.002..0.005, band_deg == [30, 8] (±3), envelope == Convective,
    // limb.0 in 0.4..0.6, granule_size in 0.0015..0.0025, region_rate in 0.3..3.
}
#[test] fn hot_star_is_quiet() {
    // A0V: envelope Radiative; activity, spot_coverage, region_rate, granule_contrast, cycle_years all 0; rotation_days < 3.
}
#[test] fn convective_boundary_is_7200_k() { assert_eq!(envelope(7_199.0, 1.6), Envelope::Convective); assert_eq!(envelope(7_200.0, 1.6), Envelope::Radiative); }
#[test] fn fully_convective_dwarf_is_active() {
    // M5V (mass < 0.35): envelope FullyConvective; activity > the G2V median; spot_coverage > G2V's; band_deg == [75, 0].
}
#[test] fn same_seed_same_activity() { /* star_activity(&g2v, 7) == star_activity(&g2v, 7); rotation differs between seeds 7 and 8 */ }
#[test] fn legacy_system_gets_activity_on_read() {
    // draw a system, serialise, strip "activity" from the JSON, write it, read_system:
    // star.activity == Some(star_activity(&star.info, system.seed)).
}
#[test] fn drawn_system_stores_activity() { /* draw(..) → star.activity.is_some(), equal to star_activity(&info, seed) */ }
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `RAYON_NUM_THREADS=4 cargo test -p mapgen --all-features activity -- --test-threads=3`
Expected: FAIL to compile, because `activity` and `star_activity` are not defined.

- [ ] **Step 3: Implement `activity.rs`, the `SystemStar.activity` field, the call in `assemble`, and the fill in `read_system`**

In `read_system`, after deserialising, run `if system.star.activity.is_none() { system.star.activity = Some(star_activity(&system.star.info, system.seed)); }`. The server reads systems through `read_system` (`crates/server/src/systems.rs:175`, `app.rs:676`), so `GET /api/systems/{id}` serves the derived value. Writing the system back is not needed. Use the relations from the science section above, with a doc comment on each one that names its source and marks calibrations.

- [ ] **Step 4: Run all tests and regenerate the bindings**

Run: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`
Expected: PASS. `git status` shows the new or changed `web/src/lib/api/types/StarActivity.ts`, `Envelope.ts` and `SystemStar.ts` (`activity?: StarActivity`).

- [ ] **Step 5: Commit**

```bash
git add crates/mapgen/src web/src/lib/api/types docs/science docs/system-spec.md
git commit -m "feat(mapgen): derive each system star's activity" -m "Rotation from gyrochronology, activity from the Rossby number, cycle, active band, spot and facula coverage, granulation and limb darkening; stored on SystemStar and derived on read for systems drawn before." -m "Refs: #343"
```

(Every commit in this plan ends with the two footers from Global Constraints.)

---

### Task 2: Region timeline, f(seed, day) (TypeScript)

**Files:**

- Create: `web/src/lib/starActivity.ts`, `web/src/lib/starActivity.test.ts`

**Interfaces:**

- Consumes: `StarActivity` (`web/src/lib/api/types/StarActivity.ts`, from Task 1).
- Produces:

```ts
/** A region as born: fixed for its whole life. Angles in radians, areas in shares of the hemisphere, days from the system's t = 0. */
export interface ActiveRegion {
  id: number;            // stable: hash of (seed, slot, k), unique
  born: number;          // day
  growth: number;        // days to its peak
  lifetime: number;      // days from birth to gone
  peakArea: number;
  latitude: number;      // signed
  longitude: number;     // in the star's inertial frame at its birth, (−π, π]
  tilt: number;          // Joy's law, radians
  separation: number;    // bipole footpoints, radians on the sphere
  polarity: 1 | -1;      // leader's sign (Hale)
}
export interface RegionNow { region: ActiveRegion; area: number; longitude: number /* now, (−π, π] */ }

export const MAX_LIFETIME_DAYS = 120;
/** Cycle level at `days`, [0, 1] (1 at maximum); 1 when the star has no cycle. */
export function cycleLevel(a: StarActivity, days: number): number;
/** Active-band centre latitude (degrees, ≥ 0) at `days`. */
export function bandLatitude(a: StarActivity, days: number): number;
/** Angular velocity at `latitude`, rad/day. */
export function angularVelocity(a: StarActivity, latitude: number): number;
/** Equatorial rotation phase at `days`, reduced to [0, 2π). */
export function rotationPhase(a: StarActivity, days: number): number;
/** Regions born in day-slot `slot` (an integer, may be negative): a pure function of (a.seed, slot). */
export function regionsBornIn(a: StarActivity, slot: number): ActiveRegion[];
/** Regions alive at `days`, with their area and longitude now, largest first, at most `max`. */
export function activeRegions(a: StarActivity, days: number, max: number): RegionNow[];
/** Integer hash used by the timeline (and by coronalLoops for per-loop jitter). */
export function hash32(...keys: number[]): number;
```

- Algorithm: time is cut into one-day slots. Slot n's region count is Poisson with mean `rate(n + 0.5)`, where the rate comes from `region_rate` and the cycle shape. The count is drawn with a `hash32(a.seed, n)`-seeded uniform stream, by Knuth's method. For rates above 30 use the normal approximation, capped at 64 per slot. `activeRegions` visits slots `floor(days − MAX_LIFETIME_DAYS) .. floor(days)`. A region's area is `peakArea · min(t/growth, 1)` while it grows, then decays linearly to 0 at `lifetime`. Its longitude is `longitude + angularVelocity(latitude) · (days − born)`, reduced. Ties in area are ordered by `id`.
- Each region draws its hemisphere (±), its latitude around `bandLatitude(born)`, its lognormal peak area, its longitude and its growth jitter from successive values of its own stream `hash32(a.seed, n, k, field)`. Polarity is `(cycleIndex % 2 === 0 ? 1 : −1) · sign(latitude)`, where `cycleIndex = floor(born / P_cyc + a.cycle_phase)`.

- [ ] **Step 1: Write the failing tests** (fixture: a Sun-like `StarActivity` literal from Task 1's G2V output, with seed 1234; and a radiative one)

```ts
it('gives the same regions for the same day, whatever the path', () => {
  const at = activeRegions(SUN, 1234.5, 32);
  activeRegions(SUN, 9000, 32); activeRegions(SUN, -50, 32);
  expect(activeRegions(SUN, 1234.5, 32)).toEqual(at);
});
it('scrubs forward and backward to the same surface', () => {
  // for d of 1200..1300 step 0.37 then back: every pair equal (toEqual)
});
it('stays finite and in range at negative and far days', () => {
  for (const d of [-3650, 0, 1e7]) for (const r of activeRegions(SUN, d, 32)) {
    expect(Number.isFinite(r.area) && r.area > 0).toBe(true);
    expect(Math.abs(r.longitude)).toBeLessThanOrEqual(Math.PI);
  }
});
it('keeps regions in the band of their birth', () => { /* 2000 days: |lat|° within bandLatitude(born) ± 16 (4σ) */ });
it('follows the cycle', () => { /* mean count over 365 d around max > 3× around min (cycle_depth 0.9) */ });
it('covers spot_coverage at cycle maximum on average', () => { /* Σ area mean over 365 d around max within ±30 % of SUN.spot_coverage (max = Infinity) */ });
it('draws nothing on a radiative star', () => expect(activeRegions(HOT, 100, 32)).toEqual([]));
it('turns the equator faster than 60°', () => expect(angularVelocity(SUN, 0)).toBeGreaterThan(angularVelocity(SUN, Math.PI / 3)));
it('caps the regions, largest first', () => { /* saturated fixture: length ≤ 8, areas non-increasing */ });
it('pins a vector', () => { /* activeRegions(SUN, 1000, 4).map(r => r.region.id) equals a literal array recorded on first green run */ });
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cd web && npx vitest run src/lib/starActivity.test.ts`
Expected: FAIL with "Failed to resolve import './starActivity'".

- [ ] **Step 3: Implement `web/src/lib/starActivity.ts`** with the exports above.

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cd web && npx vitest run src/lib/starActivity.test.ts && npm run check`
Expected: PASS, and 0 errors and 0 warnings.

- [ ] **Step 5: Commit** with `feat(web): add the star's region timeline` and the footer `Refs: #343`.

---

### Task 3: Frame-time probe and baseline

**Files:**

- Create: `web/src/lib/viewer/frameStats.ts`, `web/src/lib/viewer/frameStats.test.ts`
- Modify: `web/src/lib/viewer/StarView.ts` (around `this.renderer.render`, at about line 1816), `web/src/lib/viewer/PlanetView.ts` (the `renderer.render` calls at about lines 2119 and 3326, which #342 does not touch)
- Docs: `web/README.md`, a new section "Measuring frame time" with the protocol below.

**Interfaces:**

- Produces:

```ts
/** On when the page URL has `?frames=1` (the search, not the hash: the hash is the app's route). */
export function frameStatsEnabled(search: string): boolean;
export interface FrameSummary { view: string; frames: number; medianMs: number; p95Ms: number }
export class FrameStats {
  constructor(view: string, windowFrames?: number /* 300 */);
  /** Times `draw`; when enabled, forces the GPU to finish (1-pixel readPixels) so the time includes it. */
  measure(gl: WebGLRenderingContext | WebGL2RenderingContext, draw: () => void): void;
  summary(): FrameSummary | null;
}
/** Every view's FrameStats register here; logged with console.table every 5 s and readable as window.__frames. */
export function frameRegistry(): Map<string, FrameStats>;
```

- When it is disabled, `measure` only calls `draw()`: no timer, no readPixels, no allocation.

- [ ] **Step 1: Write the failing tests**

```ts
it('is off unless asked', () => { expect(frameStatsEnabled('')).toBe(false); expect(frameStatsEnabled('?frames=1')).toBe(true); });
it('summarises a known series', () => { /* feed 1..100 ms through a fake clock: median 50.5, p95 95.05, frames 100 */ });
it('keeps only the last window', () => { /* windowFrames 10, 25 samples: frames 10 */ });
```

- [ ] **Step 2: Run the tests and confirm they fail**: `cd web && npx vitest run src/lib/viewer/frameStats.test.ts` fails because the module is missing.
- [ ] **Step 3: Implement it, and wrap the render calls of `StarView` and `PlanetView`** (`this.frames.measure(this.renderer.getContext(), () => this.renderer.render(this.scene, this.camera))`).
- [ ] **Step 4: Run the tests and confirm they pass**: `npx vitest run src/lib/viewer/frameStats.test.ts && npm run check`.
- [ ] **Step 5: Record the baseline on this commit, before any visual change.**
  Use one fixed system: `POST /api/systems` with `{"system":{"star":"G2V","seed":4242}}`, plus a K5V (`seed 4243`) and an M4V (`seed 4244`). Use the quality profile high and a browser window of exactly 1920×1080, then of 2560×1440. Open `/?frames=1#system=<id>`, run the animation for 20 s, and read the summary. Do this for the system view (animated at default speed), and for the planet view globe of the first generated planet with Sunlight on and the star in view. Paste the table (view, resolution, median, p95) into the PR description, and as a comment on #343, so later PRs compare against it.
- [ ] **Step 6: Commit** with `feat(web): add a frame-time probe behind ?frames=1` and the footer `Refs: #343`.

---

### Task 4: Shared star surface, used by the system view

**Files:**

- Create: `web/src/lib/viewer/starSurface.ts`, `web/src/lib/viewer/starSurface.test.ts`
- Modify: `web/src/lib/viewer/starShaders.ts:60-104` (`starSurfaceFragment` becomes the chunk plus a short `main`), `web/src/lib/viewer/StarView.ts` (the star mesh at about lines 431-446, `setStar` at about 567-578, `tick` at about 1426-1441), `web/src/lib/viewer/starSystem.ts` (remove `starSurface`, `StarSurface`) and its test, `web/src/lib/viewer/quality.ts` (+ test), `web/src/components/StarPane.svelte` (new `activity` prop, passed to `view.setStar`), `web/src/components/Viewer.svelte` (the `<StarPane activity={sys?.star.activity ?? null}>` attribute, in the element at about lines 2208-2254)
- Docs: `README.md`, the paragraph that describes the system view's star: the living surface, and its sources (link to the science article).

**Interfaces:**

- Consumes: `activeRegions`, `angularVelocity`, `rotationPhase`, `RegionNow` (Task 2); `StarActivity`, `StarInfo`.
- Produces:

```ts
export interface StarLook {
  colour: [number, number, number];  // info.light
  limb: [number, number];            // activity.limb, or a fallback for worlds without a system: [clamp(0.6 + (5772 − T)/5000, 0.3, 0.85), 0.2]
  granuleSize: number; granuleContrast: number; granuleLifetimeDays: number;
  omegaEq: number;                   // rad/day; 2π / 25 for the fallback
  shear: number;
  faculaRatio: number;
}
export function starLook(info: StarInfo, activity: StarActivity | null): StarLook;
/** The GLSL chunk: uniforms below plus `vec3 starSurfaceColour(vec3 n, float mu)`, n the unit normal in the star's inertial frame (y the spin axis). Defines STAR_REGIONS from the material's defines. Compiles granulation out under #define STAR_NO_GRANULATION. */
export const STAR_SURFACE_GLSL: string;
/** Uniforms: uStarColour, uLimb (vec2), uGranule (vec4: size, contrast, phase, fade), uRegionFade, uRegionCount, uRegionA[STAR_REGIONS], uRegionB[STAR_REGIONS] (vec4 each), uLocalDays, uOmegaEq, uShear, uFacula. */
export function starSurfaceUniforms(maxRegions: number): Record<string, IUniform>;
export class StarSurfaceState {
  constructor(uniforms: Record<string, IUniform>, maxRegions: number);
  setStar(look: StarLook, activity: StarActivity | null): void;
  /** Sets the uniforms for `days`; repacks the region arrays only when the drawn set's ids change or |days − epoch| > 20. Returns whether it repacked. */
  update(days: number, view: { radiusPx: number; daysPerSecond: number }): boolean;
  /** The regions drawn now (for the loops, Task 5). */
  readonly regions: readonly RegionNow[];
}
/** 1 when the frame's time step is shorter than `timescaleDays`, falling as sqrt(timescale/step) beyond: a long exposure's averaging. */
export function exposureFade(daysPerFrame: number, timescaleDays: number): number;
/** Granulation from 1.5 to 4 px per granule, 0 below; regions from 6 to 12 px of star radius, 0 below. */
export function surfaceDetail(radiusPx: number, granuleSize: number): { granulation: number; regions: number };
```

- Packing: `uRegionA[i] = (latitude, longitude at epoch, tilt, separation)`, `uRegionB[i] = (born − epoch, growth, lifetime, peakArea)`. The shader rebuilds the area and the longitude (`+ uOmegaEq (1 − uShear sin²λ) (uLocalDays − bornRel)`), so a repack happens only when the set changes. `uLocalDays = days − epoch` is the only time the shader sees. The granule phase is `(days mod 1e4) / granuleLifetimeDays`, reduced in JS.
- Each region draws a leader and a follower spot (with umbra and penumbra) along its tilt axis, at ±separation/2, and a facular plage 3× the spot radius, brightened by `0.25 (1 − μ)`. The structure and brightness numbers are in the science section above. Brightness multiplies the quadratic limb law and `uStarColour`. The output keeps today's soft exposure: `1 − exp(−1.6 c)`.
- The region fade is `surfaceDetail.regions × exposureFade(daysPerFrame, P_rot / 12)`, where `daysPerFrame = daysPerSecond / 30` is the nominal frame, not the measured one, so the result stays deterministic. The granulation fade is `surfaceDetail.granulation × exposureFade(daysPerFrame, granuleLifetimeDays)`.
- `quality.ts`: `starRegions` (high 32, light 12) and `loopRegions` (high 8, light 3). These become `STAR_REGIONS` through the material's `defines`.
- `StarView`: the star mesh is `new ShaderMaterial({ defines: { STAR_REGIONS }, uniforms: starSurfaceUniforms(n), … })`, plus a `StarSurfaceState`. `setStar(info, …)` takes a new trailing `activity: StarActivity | null = null` argument. It must not move the existing four arguments, because #342 does not change the signature. `tick` calls `surface.update(clockDays, { radiusPx, daysPerSecond: this.speed * this.daysPerSecond })`, where `clockDays` is the `days` already computed in `tick`. `radiusPx = starScale · STAR_RADIUS / (distance · tan(fov/2)) · height/2`. The old `uTime * 0.02` rotation is gone: rotation now comes from the clock. Also call `update` in `setSystemDay`.

- [ ] **Step 1: Write the failing tests**

```ts
it('fades a long exposure', () => {
  expect(exposureFade(0.001, 0.01)).toBe(1);
  expect(exposureFade(4096 / 30, 25 / 12)).toBeLessThan(0.15);   // system view at 2^12 d/s: spots smear out
  expect(exposureFade(2 ** -11 / 30, 8 / 1440)).toBe(1);         // star view at its slowest: granules boil
});
it('drops detail on a small star', () => {
  expect(surfaceDetail(5, 0.002)).toEqual({ granulation: 0, regions: 0 });
  expect(surfaceDetail(4000, 0.002).granulation).toBe(1);
});
it('starLook falls back without activity', () => { /* starLook(SUN_SYSTEM, null): limb[0] ≈ 0.6, granuleContrast 0.12, shear 0 */ });
it('repacks only when the set changes', () => {
  // state.update(1000, big) → true; update(1000.001, big) → false (same ids); update(1000 + 30, big) → true (re-epoch)
});
it('keeps the shader time small', () => { /* after update(1e7, …) uLocalDays.value < 21 */ });
```

- [ ] **Step 2: Run the tests and confirm they fail**: `cd web && npx vitest run src/lib/viewer/starSurface.test.ts` fails because the module is missing.
- [ ] **Step 3: Implement `starSurface.ts`, rebuild `starSurfaceFragment` on it, wire it into `StarView`, `StarPane` and `Viewer`, remove `starSurface`/`StarSurface` and their test, and update `quality.ts`.**
- [ ] **Step 4: Verify**
  Run: `cd web && npx vitest run && npm run check`. Expected: PASS, and 0 warnings.
  Manual check: open the G2V system's system view. The star's regions sit in two bands and turn with the clock. Setting day 400 by the time bar, going elsewhere and setting it again shows the same spots. At 2¹² days/s the spots fade instead of strobing.
- [ ] **Step 5: Commit** with `feat(web): draw the living star surface in the system view` and the footer `Refs: #343`.

---

### Task 5: Coronal loops over the active regions

**Files:**

- Create: `web/src/lib/viewer/coronalLoops.ts`, `web/src/lib/viewer/coronalLoops.test.ts`
- Modify: `web/src/lib/viewer/StarView.ts` (`buildProminences`/`animateProminences`/`arcMesh` at about lines 1638-1705 become loops; `setProminences` keeps its name and toggles the loops), `web/src/lib/viewer/starSystem.ts` (remove `prominences` and `PROMINENCES`; keep the `Prominence` type, now the loop arc) and its test, `web/src/lib/viewer/prominenceGeometry.ts` (unchanged API, reused), `web/src/components/StarPane.svelte` (hide the toggle when `activity` is null or radiative), `web/src/lib/i18n/en.ts` / `fr.ts` (`starView.prominences` → "Coronal loops" / "Boucles coronales", with new help text)
- Docs: `README.md` (loops replace prominences), `docs/science/star-activity.md` § loops.

**Interfaces:**

- Consumes: `ActiveRegion`, `RegionNow`, `hash32` (Task 2); `StarSurfaceState.regions` (Task 4); `prominenceGeometry(arc, radius, seed, strands)`, `prominenceVertex`/`prominenceFragment`.
- Produces:

```ts
/** A region's loops in its own frame (the region centred at longitude 0, at its latitude): n arcs from leader to follower footpoints, heights separation/2 (±20 %), deterministic in region.id. */
export function loopArcs(region: ActiveRegion, n: number): Prominence[];
/** Which region ids to add and to remove, going from `shown` to `wanted`. */
export function diffRegions(shown: readonly number[], wanted: readonly number[]): { add: number[]; remove: number[] };
export class CoronalLoops {
  readonly group: Group;
  constructor(quality: QualityProfile, material: (index: number) => ShaderMaterial);
  /** Keeps one sub-group per region among the `loopRegions` largest; turns each to its longitude now, opacity ∝ sqrt(area / peakArea). */
  update(regions: readonly RegionNow[], days: number): void;
  /** The arc of the largest region's first loop, for the interim eruption; null when none. */
  largestArc(): Prominence | null;
  dispose(): void;
}
```

- The loops per region are 3 in the high profile and 1 in light. The region's sub-group geometry is built once, when the region is added. Per frame only `rotation.y`, the opacity and `uTime` change. The eruption (decision 7) picks `largestArc()` at each eruption cycle instead of `prominences('M', …)`. When `largestArc()` is null, there is no eruption.

- [ ] **Step 1: Write the failing tests**

```ts
it('stands loops on the sphere between the bipole', () => {
  for (const arc of loopArcs(REGION, 3)) {
    expect(Math.hypot(...arc.from)).toBeCloseTo(1, 9); expect(Math.hypot(...arc.to)).toBeCloseTo(1, 9);
    const apart = Math.acos(dot(arc.from, arc.to));
    expect(apart).toBeGreaterThan(0.75 * REGION.separation); expect(apart).toBeLessThan(1.25 * REGION.separation);
    expect(arc.height).toBeGreaterThan(0.4 * REGION.separation); expect(arc.height).toBeLessThan(0.6 * REGION.separation);
  }
});
it('is the same for the same region', () => expect(loopArcs(REGION, 3)).toEqual(loopArcs(REGION, 3)));
it('diffs region sets', () => expect(diffRegions([1, 2, 3], [2, 3, 4])).toEqual({ add: [4], remove: [1] }));
```

- [ ] **Step 2: Run the tests and confirm they fail**: `npx vitest run src/lib/viewer/coronalLoops.test.ts` fails because the module is missing.
- [ ] **Step 3: Implement `coronalLoops.ts` and the `StarView` changes. Remove `prominences` and its test, and update the i18n strings.**
- [ ] **Step 4: Verify**: `cd web && npx vitest run && npm run check` must PASS. Manual check: loops stand over the spotted regions and turn with them. The A0V system's star has no loops and no toggle.
- [ ] **Step 5: Commit** with `feat(web): stand coronal loops over the active regions` and the footer `Refs: #343`. Open PR B (Tasks 3–5) with the Task 3 baseline and an after-table for the system view.

---

### Task 6: The star view, its toggle and routing

**Files:**

- Create: `web/src/lib/viewer/StarBodyView.ts`, `web/src/components/StarSurfacePane.svelte`, `web/src/lib/starClock.ts`, `web/src/lib/starClock.test.ts`
- Modify: `web/src/lib/world.svelte.ts` (+ `world.svelte.test.ts`), `web/src/App.svelte:193-196` and `:261-266` (hash), `web/src/components/Viewer.svelte` (see "Overlap with PR #342": no edit inside lines 899-911), `web/src/components/StarPane.svelte` (props `clockDays` (bindable), `hidden`, `onOpenStar`; a "Star" button in `.star-chrome`), `web/src/lib/viewer/StarView.ts` (dblclick: if no planet is picked and the ray hits the star's sphere, call `onOpenStar`), `en.ts`/`fr.ts`
- Docs: `web/README.md` § Usage (the star view: how to open it, controls, time range); `README.md` (one paragraph).

**Interfaces:**

- Consumes: `StarSurfaceState`, `starSurfaceUniforms`, `starLook`, `STAR_SURFACE_GLSL` (Task 4); `CoronalLoops` (Task 5); `glowVertex`/`glowFragment`, `starField`, `FrameStats` (Task 3); `rotationPhase`, `cycleLevel`, `activeRegions` (Task 2).
- Produces:

```ts
// StarBodyView.ts — the star alone, its spin axis up, camera from 20° above its equator.
export class StarBodyView {
  constructor(canvas: HTMLCanvasElement, quality: QualityProfile);
  setStar(info: StarInfo, activity: StarActivity | null): void;
  setDay(days: number): void;          // any finite day, negative included
  get day(): number;
  setSpeed(daysPerSecond: number): void;
  setAnimated(on: boolean): void;
  setLoops(on: boolean): void;
  resetView(): void;
  onClock: (days: number) => void;
  onOpenSystem: (() => void) | null;   // double-click on the star
  dispose(): void;
}
// starClock.ts
export const STAR_SPEED_MIN = -12;
export const STAR_SPEED_MAX = 6;
export function clampSpeed(speed: number, mode: 'system' | 'star'): number;
/** log2 of days/s so that one granule lifetime lasts about 10 s; SYSTEM default for a radiative star (none boil). */
export function defaultStarSpeed(activity: StarActivity | null): number;
/** The next cycle maximum and minimum after `days` (days), null without a cycle. */
export function cycleJumps(activity: StarActivity, days: number): { maximum: number; minimum: number } | null;
// world.svelte.ts
starMode: 'system' | 'star';          // $state, 'system' by default and on every new system
```

- `StarBodyView` uses OrbitControls with `minDistance = 1.02` star radii (close enough for granules) and `maxDistance = 12`. The star sphere has `quality.starSegments`. Its glow is a corona built from `glowFragment`. It has loops, a star field, the 30 fps cap and the frame probe (`'star'`). Its LOD uses `radiusPx` as in Task 4.
- `StarSurfacePane.svelte` reuses `Dock`, `ToggleBar` and `PlaybackBar`. Toggles: loops. Panels: the star's type and Teff, rotation at the equator and at 60°, age, Rossby number, activity relative to the Sun, cycle period and current phase (rising / maximum / declining / minimum, from `cycleLevel`), regions now, spot coverage now. Playback: speed `STAR_SPEED_MIN..MAX`, a time bar over one cycle (`[0, cycle_years · 365.25]`, or 30 days without a cycle), and the jumps "next maximum" and "next minimum".
- `Viewer.svelte`: add `let starDays = $state(0)`, reset to 0 when `world.systemId` changes. Add an `$effect` that sets `starDays = sysDays` when `world.subject` turns to `'star'` from `'planet'`, placed next to `starAnimated`/`starProminences` (about lines 240-244). `StarPane` stays mounted while `subject === 'star'`, with `hidden={world.starMode === 'star'}` (canvas hidden, `setAnimated(false)`). `StarSurfacePane` is mounted while `subject === 'star' && starMode === 'star'`. Both get `bind:clockDays={starDays}`. On mount, each pane applies `clockDays` once, after `setStar`.
- Routing: `world.loadSystem(id, 'star')` sets `subject = 'star'` and `starMode = 'star'`. `App.svelte` writes `body: subject === 'star' ? (starMode === 'star' ? 'star' : null) : systemBody`. `'star'` is never a body id, because `SYSTEM_NAME_KEY` reserves it. The body picker's "star" option still opens the system view.

- [ ] **Step 1: Write the failing tests**

```ts
// starClock.test.ts
it('clamps speeds into each view', () => { expect(clampSpeed(10, 'star')).toBe(6); expect(clampSpeed(-12, 'system')).toBe(0); });
it('boils granules at the default star speed', () => { /* SUN: 2 ** defaultStarSpeed(SUN) * 10 within ×2 of SUN.granule_lifetime_days */ });
it('jumps to the next maximum and minimum', () => { /* cycleJumps(SUN, d): both > d, cycleLevel at maximum > 0.95, at minimum < 0.1 */ });
// world.svelte.test.ts
it('opens the star view from body=star', () => { /* loadSystem('x', 'star') with a stubbed fetch: subject 'star', starMode 'star', systemBody is not 'star' */ });
it('opens the system view without body', () => { /* loadSystem('x'): subject 'star', starMode 'system' */ });
it('resets the star mode with another system', () => { /* starMode 'star', loadSystem('y'): starMode 'system' */ });
// systemStar.test.ts
it('runs the system view on the planets epoch', () => {
  /* for a fixture planet, systemStarView(...).giants[k].phase · 2π + 2π · days / year_days ≡ anomalies(orbit, days).mean (mod 2π), days = 0 and 1234 */
});
```

- [ ] **Step 2: Run the tests and confirm they fail**: `npx vitest run src/lib/starClock.test.ts src/lib/world.svelte.test.ts src/lib/systemStar.test.ts` fails, because the module and `starMode` are missing. (The epoch test may already pass. If it fails, fix `systemStarView`'s phase before going on.)
- [ ] **Step 3: Implement `starClock.ts`, `StarBodyView`, `StarSurfacePane`, the `world`/`App`/`Viewer`/`StarPane`/`StarView` changes, and the i18n strings** (`starView.openStar` "Star" / "Étoile", `openStarHelp`, `starSurface.*` for the pane, `starSurface.openSystem` "System" / "Système", and the envelope names).
- [ ] **Step 4: Verify**: `cd web && npx vitest run && npm run check` must PASS. Manual check: the body list's star opens the system view. The button and a double-click on the star open the star view. The button and a double-click there come back. The day is the same both ways, and only the visible view animates (with `?frames=1`, only one view logs). `#system=<id>&body=star` opens the star view after a reload.
- [ ] **Step 5: Commit** with `feat(web): add the star view, switched with the system view` and the footer `Refs: #343`.

---

### Task 7: The same star in a planet's sky

**Files:**

- Create: `web/src/lib/viewer/hostStarSurface.ts`, `web/src/lib/viewer/hostStarSurface.test.ts`
- Modify: `web/src/lib/systemSky.ts:223-224, 303-317` (return `star.axes`; the #342 hunks are at 22, 208-214 and 243-263) and `systemSky.test.ts`; `web/src/lib/viewer/shaders.ts:763-795` (`hostStarFragment`: the chunk with `STAR_NO_GRANULATION`); `web/src/lib/viewer/PlanetView.ts` (host star material at about lines 848-870, a new `setHostStarSurface` next to `setHostStar` at about line 1716, `drawHostStar` at about line 3205); `web/src/components/Viewer.svelte` (an `$effect` next to the `setStarPlace` effect at about line 1261)
- Docs: `docs/science/star-activity.md` § "In the app" (the sky disc and its detail threshold).

**Interfaces:**

- Consumes: `StarSurfaceState`, `starSurfaceUniforms`, `starLook`, `STAR_SURFACE_GLSL` (Task 4); `StarActivity`.
- Produces:

```ts
// systemSky.ts: star gains its axes in the frame of the body shown (the system's +x, +y, +z through `local`).
star: { position: Vec3; radius: number; axes: [Vec3, Vec3, Vec3] };
// hostStarSurface.ts
/** Maps the disc's point-sprite coordinates (x right, y up, z towards the viewer, unit disc) to the star's inertial frame: columns = star-frame images of screen right, screen up and the line of sight (star → camera). */
export function discBasis(toCamera: Vec3, cameraUp: Vec3, starAxes: [Vec3, Vec3, Vec3]): number[] /* mat3, column-major */;
/** Detail on the sky disc: regions from 12 to 24 px of disc diameter, none below. */
export function hostStarDetail(diskPx: number): number;
// PlanetView
setHostStarSurface(surface: { info: StarInfo; activity: StarActivity | null; days: number; daysPerSecond: number; axes: [Vec3, Vec3, Vec3] } | null): void;
```

- `hostStarFragment` rebuilds `n = uDiscBasis · (x, y, sqrt(1 − r²))` inside the disc, and takes `mu = z`. It draws `starSurfaceColour(n, mu)` scaled by today's `1.4`, and keeps today's glow and fade outside the disc. When the detail is 0 (the usual case: the Sun from Earth is about 14 px at 1080p), the region loop is skipped behind a uniform branch, and only the quadratic limb law changes from today. With `setHostStarSurface(null)`, as for worlds without a system, it keeps today's `1 − 0.35 r²` look.
- `drawHostStar` computes the disc basis each frame from `starPlace`/`uSunDir` and the camera's up vector, and calls `surface.update(days, { radiusPx: disk / 2, daysPerSecond })`. Viewer passes `daysPerSecond` as the planet view's orbit animation rate while playing, and 0 otherwise.

- [ ] **Step 1: Write the failing tests**

```ts
it('faces the star\'s visible hemisphere', () => {
  // star axes = identity, camera on +z of the star looking at it, up +y: discBasis maps (0,0,1) → [0,0,1], (1,0,0) → [1,0,0]
});
it('turns with the star frame', () => { /* star axes rotated 90° about y: (0,0,1) maps to the rotated z */ });
it('shows no detail on a small disc', () => { expect(hostStarDetail(14)).toBe(0); expect(hostStarDetail(30)).toBe(1); });
// systemSky.test.ts
it('gives the star its axes in the frame of the body shown', () => {
  /* fixture: each axis unit length, mutually orthogonal; for a body with tilt 0 and inclination 0, star.axes[1] ≈ [0, 1, 0] */
});
```

- [ ] **Step 2: Run the tests and confirm they fail**: `npx vitest run src/lib/viewer/hostStarSurface.test.ts src/lib/systemSky.test.ts` fails because the module and `axes` are missing.
- [ ] **Step 3: Implement it.**
- [ ] **Step 4: Verify**: `cd web && npx vitest run && npm run check` must PASS. Manual check: from a close-in planet of the M4V system, or with the camera zoomed so that the disc is wider than 24 px, the sky disc shows the spots the star view shows at the same day. Its limb darkening matches the star view's.
- [ ] **Step 5: Commit** with `feat(web): show the living star in a planet's sky` and the footer `Refs: #343`.

---

### Task 8: After-measurement and closing docs

**Files:**

- Modify: `web/README.md` § "Measuring frame time" (a short line with the outcome and the PR link), `docs/science/star-activity.md` (a "Where it departs from the science" section listing every calibration), `docs/backlog.md` (the decision history line, as the file does for other features)

- [ ] **Step 1: Measure** with Task 3's protocol, on the branch carrying Tasks 6–7 and on `main` at the same commit base. Cover the system view, the planet view globe (Sunlight, star in view) and the star view (loops on, camera at 1.1 radii and at the default distance), at 1920×1080 and 2560×1440, for G2V, K5V and M4V.
- [ ] **Step 2: Check the bounds.** For the system view and the planet view with no new option on, the median must be within the run-to-run noise of the baseline: measure the noise as the spread of three baseline runs, and fail if the median exceeds the baseline by more than 2× that spread. The star view's p95 must be at most 16.7 ms at 2560×1440. If a bound fails, lower `starRegions` or `loopRegions`, or raise the `surfaceDetail` thresholds, and measure again. Record what changed.
- [ ] **Step 3: Write the docs above, and open the follow-up issue from decision 5** with `gh issue create --label backlog`.
- [ ] **Step 4: Run every check**: `RAYON_NUM_THREADS=4 cargo test --workspace --all-features -- --test-threads=3`, then `cd web && npm run check && npx vitest run`. All must PASS.
- [ ] **Step 5: Commit** with `docs: record the living star's frame times and calibrations` and the footer `Refs: #343`. The PR description carries the before/after tables and `Closes #343`.

---

## PR Split

| PR | Tasks | Content | Issue line |
| --- | --- | --- | --- |
| A | 1–2 | Activity model, bindings, region timeline: data and pure code, no visible change | `Refs #343` |
| B | 3–5 | Frame probe and baseline, shared surface in the system view, coronal loops | `Refs #343` |
| C | 6–8 | Star view and toggle, sky disc, after-measurement, closing docs | `Closes #343` |

## Overlap with PR #342 (`feat/normal-map-relief`)

`gh pr diff 342` changes these lines (old numbering):

- `PlanetView.ts`: 99-105, 339-345, 514-591, 990, 1503-1590, 2142, 2330-2460.
- `StarView.ts`: 15-45, 246-252, 325-331, 897-1000.
- `Viewer.svelte`: 9-15, 645-720, 903-960.
- `systemSky.ts`: 22, 208-214, 243-263.
- `shaders.ts`: 199-229, 402-430, 518-547, 608-722, 921-1190 (`skyBodyFragment`).

This plan's edits sit elsewhere:

- `PlanetView.ts`: host star material at about 848-870, `setHostStarSurface` after 1716, `drawHostStar` at 3205, render calls at 2119 and 3326.
- `StarView.ts`: imports from 51, members at 217-221, constructor at 398-446, `setStar` at 545-578, `tick` at 1426, loops at 1638-1705.
- `Viewer.svelte`: state at about 240-244, effects near 1261, `<StarPane>` at 2208-2254. No edit inside 899-960: `toStar` and `sysMaps`/`sysTextures` are #342's.
- `systemSky.ts`: 223-224 and 303-317.
- `shaders.ts`: `hostStarFragment` at 763-795.

No textual conflict is expected, only line shifts on rebase. #342 adds a `setStar`-adjacent `setPlanetMaps` change and a new `markerShaders.ts`, neither of which this plan touches. Keep `StarView.setStar`'s first four parameters as they are.
