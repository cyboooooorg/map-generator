# Physical planet model, part 3c: craters and crust from the model

Part 3c of #423 (roadmap #427), after 3a (#451) and 3b (#456). It
complements the main design,
`docs/superpowers/specs/2026-11-11-physical-planet-model-design.md`, and
absorbs #312, #313 and #315.

## Intent

Since 3b, a world's composition, interior, heat and regime are drawn and
stored (`PlanetModel`, `planet/mod.rs:158`), but the relief still ignores
them in three places:

- **Crater transition (#312).** The simple-to-complex transition reads
  gravity only, plus one calibrated factor (`ICE_TRANSITION` = 0.15,
  `craters.rs:106`) when the bulk density says "icy"
  (`generation::icy`, `generation.rs:202`). Earth comes out at 2.5 km
  instead of 3 to 5 km.
- **Impact flux (#313).** `Geology.impact_ratio` and `crater_scale`
  (`geology.rs:128,133`) are fixed at 1 now that archetypes replaced
  presets: Mars lost its sourced 2.6 and 0.751, and no other body ever had
  a value.
- **Isostasy (#315).** Continents float as a fixed platform at the
  waterline (`generation::floating`, `generation.rs:300`), calibrated on
  Earth. There is no crustal thickness, so a world with less water
  exposes platform instead of basins.

The user sees no major problem in the relief today. Success is these
three things following from the model with sourced relations, Earth
keeping its hypsometry, and no visible regression. One visible defect
comes along: some large craters on moons have no rim (the user, 2026-11-14:
"plutôt sur des lunes, des gros cratères"); it is rare.

## Goals

1. **Transition from strength.** The transition diameter follows
   O'Keefe & Ahrens (1993), $D_t = 9.0\,Y/(\rho g)$ (already cited in
   `docs/science/impact-craters.md`), with the crust's effective strength
   $Y$ and density $\rho$ coming from the surface material the model
   holds: rock, or an ice shell where water is the outer layer and frozen.
   Volatile-rich rock may be weaker (Pike's explanation for Mars, recalled
   by Barnouin et al. 2012). Targets for model worlds: Mercury 10 to
   12 km, Mars 7 to 8 km, Earth 3 to 5 km. The Moon (15 km) and
   Ganymede-like ice (near 2.5 km) are moons: they stay on today's path
   (see My calls) and keep passing their current tests.
2. **Impact flux from the orbit.** A general rule replaces the per-world
   `impact_ratio` and `crater_scale`. The flux per unit area relative to
   the Moon follows from the impactor population at the orbit (today's
   belts and giants factor, `belts::impact_factor`) and the body's
   gravitational focusing; the crater size scale follows from the impact
   speed and the gravity (crater scaling laws). Targets: Mars comes back to
   Hartmann's 2.6 and 0.751, and Mercury, Venus and Earth to published
   values (Le Feuvre & Wieczorek 2011, Ivanov 2001, Neukum et al. 2001,
   or others the builder can read). A world drawn without an archetype
   gets its flux from its orbit like any other.
3. **Airy isostasy.** A crustal-thickness field over the world; heights
   follow from it by Airy's balance (crust over a denser mantle,
   `docs/science/mountains-and-gravity.md` § Isostasy). The platform no
   longer sits at the waterline: sea level comes from the water volume
   (`sea_level_for_water`), so a world with less water exposes ocean
   basins, not only platform. Earth keeps its ETOPO 2022 hypsometry
   within the bands of `earth_like_worlds_have_earths_hypsometry`
   (`generation.rs:2439`).
4. **Rimless large craters on moons.** Find the cause. Fix it in this part
   if it lies in the crater code it touches (transition, profiles, basins);
   otherwise file an issue and leave it.

## Non-goals

- Large moons through the model: part 4. Moons keep `model: None` and
  today's ice-or-rock test for the transition.
- #311 (icy simple craters keep rock's bowl), #316 (flexure under
  volcanoes and ranges), #317 (sea-floor age, Parsons & Sclater): they stay
  open, for after.
- Giants, the viewer, the creation form and the API (step 5).
- Recalibrating Earth's hypsometry targets.

## Anti-goals

- **Impact values set per archetype.** An archetype sets causes (mass,
  orbit, composition), never an outcome; per-archetype `impact_ratio`
  would bring presets back, and drawn worlds would keep the Moon's flux.
- Earth leaving its hypsometry bands, or the Solar System archetypes no
  longer looking like themselves.
- A legacy path or a format gate for older worlds
  (`generation-changes-policy`): older worlds are listed as outdated.

## Constraints

- Worlds change on purpose; they must stay coherent. Measure before and
  after on the same drawn systems (`world-coherence-checker`), including
  the Solar System archetypes.
- `FORMAT_VERSION` (18, `export/raw_data.rs:59`) bumps once in this part,
  with its history line. `SYSTEM_FORMAT_VERSION` (4, `system.rs:64`) bumps
  only if what a system stores changes. No other session needs warning
  before these bumps: the peers wait until part 3 is finished.
- Generation stays deterministic, and resolution-invariant
  (`tests/resolution_invariance.rs`).
- Every relation is checked against its source before use and recorded in
  the code and the science pages (`docs/science/impact-craters.md`,
  `docs/science/mountains-and-gravity.md`); a calibration is labelled as
  one, with the bodies it reproduces.
- One branch, `feat/planet-model-part3c`, one pull request into `main`
  closing #312, #313 and #315.

## Decided with the user

- 3c covers #312, #313 and #315 (2026-11-13), after 3b.
- **A general flux rule, not per-archetype values** (2026-11-14). The user
  asked for my suggestion and approved writing it up; the published
  values for Mercury, Venus, Earth and Mars become test targets.
- The rimless large craters on moons are looked at in this part (goal 4).

## My calls

- **Crust thickness.** Plate worlds: continental crust thick and oceanic
  crust thin (Earth about 35 and 7 km), read from the plates' crust
  (`plates.rs`, `continental`) and the continent noise. Worlds without
  plates: a thickness varying by province, which may give a hemispheric
  dichotomy like Mars. The mean thickness per world is a labelled
  calibration on the rocky bodies of the Solar System (Moon, Mars, Mercury,
  Venus), unless a sourced rule from mass and heat is found.
- **Moons** keep `generation::icy` and `ICE_TRANSITION` until part 4,
  and today's flux (giants and locking, no new rule). The Moon stays the
  reference of the flux ratio and of the crater population; the new rules
  are relative to it and need no Moon world to run.
- **Which strength.** One relation cannot give Mercury and Mars, at the
  same gravity, different transitions: a second rock strength is needed.
  Dry rock (Mercury, Venus) and plate-recycled crust (Earth) take one
  strength; volatile-rich crust is weaker. What makes a crust
  volatile-rich is read from the model, not from a name: a labelled
  calibration such as "no plates and a water inventory above a
  threshold", which separates Mars from Mercury and Earth. Its constants
  are labelled calibrations, like `ICE_TRANSITION` was. Frozen ocean
  worlds with a model take ice's strength.
- **Tolerances.** Transition targets are the bands above, as test
  assertions. Flux targets: Mars within 20 % of 2.6 and of 0.751; Mercury,
  Venus and Earth within 20 % of the values of one source the builder
  reads and cites, chosen where sources disagree and the choice recorded.
  One calibrated constant in the flux rule is allowed, labelled, if the
  rule cannot reach Mars otherwise.
- **Earth's hypsometry bands are hard.** Earth's water is an archetype
  input already; the mean thicknesses and densities may be calibrated on
  Earth (labelled). If no calibration keeps the bands, the builder stops
  and reports instead of loosening them.
- **No required dichotomy.** A smooth field of provinces is enough for
  worlds without plates; a hemispheric dichotomy is optional, seeded, and
  never tied to an archetype name.
- **One pull request, one bump.** Commits in the order #312, #313, #315
  (and the rim fix), `FORMAT_VERSION` bumped once.
- **`impact_ratio` and `crater_scale` leave `Geology`** (`deny_unknown_fields`
  look tables and `web/src/lib/api/types/Geology.ts` follow): no direct
  knob, in line with 3b.
- **Lone worlds** (`alone`) get their flux from the orbit `draw_orbit`
  gives them, with no belts or giants unless their context has them.

## Left to the builder

- The exact relations and constants, each checked against its source:
  strength per material, impact speeds per orbit (encounter speed from the
  orbital speed), gravitational focusing, the crater scaling exponents,
  crust and mantle densities, thickness distributions.
- Where the values are computed and whether any is stored
  (`WorldParams.impact_factor` already exists; the model, the system draw
  or the crater context).
- How the dry-world path (`DRY_PLAINS_KM`, `generation.rs:1413`) and
  frozen ocean worlds fit the floating crust, provided they do not
  regress.
- The test layout: unit tests for each relation against its targets, the
  hypsometry test kept, and coherence measurements before and after.
- The documentation: science pages, `docs/analysis/08-geology.md` outcome
  steps, the README where it describes presets' `[geology]` values, and
  the stale comment at `generation.rs:2762`.
