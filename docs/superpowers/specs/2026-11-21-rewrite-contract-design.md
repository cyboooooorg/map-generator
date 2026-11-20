# Slice 1, PR 2a: the contract between the engines

Part of slice 1 (#602) of the
[architecture rewrite](2026-11-20-architecture-rewrite-design.md). This
document refines that design for one pull request; where they disagree,
say so in review and fix one of them.

## Intent

After PR 1 (#612) the crates are empty. Three sessions are about to work
in parallel: mapgen3 on the system draw, mapgen1 on the planet model,
mapgen2 on the map generator and the catalogue. They need a shared,
reviewed contract first, so that none of them invents its own copy of a
unit, a keyframe field or a cause node. PR 2a is that contract: the
typed units of `science`, the types of `timeline`, and the module layout
of `system` with a fixed seam between the draw and the planet model.
mapgen1 and mapgen3 review it before merge. Any later change to
`timeline` is its own small pull request, announced to the other two
sessions.

## Goals

1. `science`: typed units and the physical constants, each constant
   defined once with its source.
2. `timeline`: a body's timeline for slice 1 (one keyframe, the
   present), the stellar sources as a list (#592), designations, seeds,
   and the cause graph with provenance.
3. `system`: the module layout, the inputs type, and the types of the
   seam between the draw and the planet model. No `todo!()` stubs: a
   function that is not written yet does not exist; its signature is
   fixed in this document and in the module's doc comment.
4. Tier-1 tests for everything with behaviour: unit conversions,
   validation, the cause graph, designations, seed derivation.
5. Every keyframe field left out of slice 1 is listed below with the
   track that adds it, and each of those tracks gets a comment saying so.

## Non-goals

- Any law of physics beyond unit conversions: laws arrive with the pull
  requests that use them (mapgen1, mapgen3).
- Events, epochs, past and future keyframes: the `Timeline` holds a list
  of keyframes so they fit later, but no event type is defined.
- The `world` and `catalogue` ports (`TimelineSource`, repositories):
  mapgen2's next pull request.
- The tier-2 reference systems and the determinism test: they need a
  system to generate (mapgen3's pull request).

## Anti-goals

- A field defined for a feature nobody has designed yet (chosen by the
  user: approach A below).
- A second code path that recomputes a value to explain it.
- The kind of the central object anywhere a planet keyframe can reach
  it.
- A unit carried only in a field name or a doc comment, or a constant
  defined twice.
- Human text in the domain: causes are codes and numbers.

## Constraints

- Dependencies: `timeline ──> science`, `system ──> timeline, science`
  (checked by `tools/guardrails`).
- The strict root `clippy.toml`: no unseeded random, no clock, no file.
- Every number in `science` has a doc comment starting with `Source:`,
  `Calibration:` or `Definition:` (tier-1 test).
- mapgen3's constraints for the stellar sources, posted on #592
  (2026-11-20): one entry per star, primary first with `primary()`,
  helpers `total_flux()`, `equilibrium_k()`, `weighted(f)` that return
  the primary's value untouched when there is one source, at most three
  sources, no circumbinary variability term yet, and a cause for the
  system draw at present.
- The system age stays reachable from the keyframe (XUV and Lyman-alpha
  histories read the source's mass and the age).

## Approaches

### Scope of the keyframe

- **A. Only what slice 1 produces** (chosen by the user, 2026-11-21, on
  condition that the rest is not abandoned): the fields slice 1 computes
  and reads; each later track adds its own fields in its own announced
  pull request. Nothing is guessed before it is designed, at the cost of
  a few more small contract pull requests.
- B. The whole keyframe of the design now, with optional fields for what
  slice 1 does not produce. A stable shape, but structures invented for
  undesigned features, left unused for months. Rejected.

### How a value carries its cause

- A wrapper per value holding its subtree: the compiler enforces it,
  but shared inputs are copied into every tree and signatures grow
  heavy.
- A recording context passed to the computations, values staying
  plain: light, stored once as a graph, but forgetting a record compiles.
- **Both** (chosen by the user, 2026-11-21): a `Trace` holds a flat
  graph of nodes; recording a value returns a `Traced<T>` holding the
  value and its node. Keyframe fields are `Traced<T>`, and only a `Trace`
  creates one, so a forgotten cause does not compile, and each node is
  stored once.

### Units

Hand-written newtypes over `f64` *(my call)*, rather than a units
library: the set is small, serde stays plain, compile times stay short,
and the conversions the project needs (Earth and solar units) are not
in the libraries' defaults anyway.

## Design

### `science::units`

- One newtype per unit, `Copy`, `PartialEq`, `PartialOrd`, serde as a
  bare number: `Kelvin`, `Pascal`, `Kilogram`, `Metre`, `Second`,
  `WattPerSquareMetre`, `MetrePerSecondSquared`, `KilogramPerCubicMetre`,
  `Radian`. Addition and subtraction within a unit, scaling by `f64`;
  no general dimensional algebra (a law that multiplies units unwraps
  them and documents the formula).
- Astronomical and planetary units as their own types with exact
  conversions: `AstronomicalUnit`, `SolarMass`, `SolarRadius`,
  `SolarLuminosity`, `EarthMass`, `EarthRadius`, `EarthFlux` (the
  bolometric flux received, Earth = 1), `Bar`, `Year` and `Gyr` (Julian
  years), `Degree`, `Day`. A field stores the unit the domain thinks in
  (an orbit in `AstronomicalUnit`, a mass in `EarthMass`); conversions
  go through `From`.
- A dimensionless share is `Fraction` (0 to 1, both included).
- Unit constructors are plain `const fn`; finiteness is checked where
  values enter the contract (every `timeline` constructor rejects NaN
  and infinities), not on every arithmetic step *(my call)*.
- `Day` is 86 400 s exactly, `Year` the Julian year (365.25 days),
  `Gyr` 10⁹ Julian years.
- `science::constants`: each constant once, with its source: the IAU
  2012 astronomical unit (B2), the IAU 2015 nominal solar and
  terrestrial values (B3: radii, solar luminosity, total solar
  irradiance, and the mass parameters GM), CODATA 2018 for G and
  Stefan-Boltzmann, the Julian year. Masses in kg are the nominal GM
  divided by CODATA G *(my call, the IAU's recommended route)*, and a
  test checks them against the published masses to 10⁻⁴. `EarthFlux` 1
  is the IAU 2015 nominal total solar irradiance, 1361 W/m². The legacy
  code had up to five copies of each constant, with different values;
  the port uses these and nothing else.

### `timeline`

- `Designation`: the IAU-style identity, assigned at birth, never
  recomputed. A system name (the legacy `MGS nnnnn` catalogue number)
  plus a body path: the star (`A`; later `B`, `C` with T7) or a planet
  letter (`b`, `c`… in order of distance at birth). Ordered,
  serialisable as text (`MGS 12345 b`), parsed back. Moons and
  circumbinary paths are added by T2 and T7.
- `Seed`: a 64-bit value with `derive(label: &str) -> Seed`, defined
  *(my call)* as `splitmix64(parent ^ fnv1a64(label))`, where FNV-1a
  64 uses its published offset basis and prime, and splitmix64 is
  Vigna's finaliser (the legacy `stream_seed` mixed the same way on 32
  bits). A body's seed is `system_seed.derive(&designation.to_string())`.
  The timeline publishes each body's seed; the world derives its
  map-pass seeds from it. It is the only behaviour in `timeline`
  besides validation, because both engines must derive seeds the same
  way and neither depends on the other. `timeline` exposes the seed as
  a `u64` and depends on no random crate. Each engine builds its
  generator from it with `rand_chacha::ChaCha8Rng` *(my call)*, whose
  output is stable across versions, which `rand`'s `StdRng` does not
  promise; the dependency arrives with the first engine that draws.
- Causes:
  - `Trace`: an arena of `Node`s, owned by the engine during a
    computation, then frozen into a `CauseGraph` stored in the
    keyframe.
  - `Node`: a `Code` (the rule or the quantity, a dotted lowercase
    string such as `climate.surface-temperature`, validated), the value
    as `f64` with its unit's code, the ids of the nodes it came from, and
    for a leaf its `Provenance`: `Input` (set by the user), `Observed`
    (a catalogue), `Drawn` (seeded), `Imposed` (forced by an archetype).
  - A node's value is a number with its unit's code, a code (for an
    enum: a climate state, a regime, a liquid kind) or a flag (a
    boolean). Units, `Fraction`, `bool` and the contract's enums
    implement the trait that turns them into a node value.
  - `Traced<T>`: a value and its node id; created only by
    `Trace::record(code, value, from)` and `Trace::leaf(code, value,
    provenance)`. Codes are strings, not an enum, so a new rule does
    not change the contract; each engine keeps its codes as constants,
    and the English catalogue in `adapters` translates them.
  - **One `Trace` per system** *(my call)*: the draw and the planet
    model record into the same one, so a `Traced` source in a
    `Placement` is a valid input of the planet's nodes. Freezing a
    keyframe copies the subgraph reachable from its fields into its own
    `CauseGraph`, renumbered, so each keyframe explains itself alone; a
    node shared by two bodies (a star's luminosity) appears in both
    keyframes, once in each.
  - What is traced *(my call)*: every derived or drawn quantity of the
    state. Identity and bookkeeping are plain: the designation, the
    seed, the engine version, the keyframe's date, the star's phase in
    slice 1. The system age as a forcing is a `Traced` leaf (`Input`:
    the present date). An interior layer is traced; its radial samples
    come from one computation and share that layer's node.
  - The laws in `science` stay unaware of causes: the engine calls a law
    and records its result.
- `StellarSources`: one `StellarSource` per star, primary first, one to
  three, validated. A source: orbit-mean bolometric flux (`EarthFlux`,
  eccentricity factor applied), equilibrium temperature from that star
  alone (`Kelvin`), effective temperature (`Kelvin`), mass (`SolarMass`),
  luminosity (`SolarLuminosity`), each `Traced`. The equilibrium
  temperature keeps the legacy definition, Earth's Bond albedo 0.3
  (255 K at one Earth flux, `legacy-final:crates/mapgen/src/star.rs:322`),
  times the eccentricity factor; the law is ported by mapgen3, 2a only
  holds the value. The primary is the first entry; the draw puts the
  most massive star first (designation `A`). Helpers *(my call)*:
  `primary()`; `total_flux()`, the sum; `equilibrium_k()`,
  (Σ T_eq,i⁴)^(1/4); `weighted(f)` with `f: Fn(&StellarSource) -> f64`,
  Σ flux_i · f(source_i) / Σ flux_i. They return plain numbers (the
  caller records what it keeps), and with one source they return the
  primary's raw value without arithmetic.
- `Timeline`: the body's designation and seed, the engine version, the
  present date, and the keyframes in date order (one in slice 1).
- `Keyframe`: a date (the system's age, `Gyr`), the `CauseGraph`, and the
  body's state, either a star or a planet:
  - star: phase (`MainSequence` only in slice 1), mass, radius,
    luminosity, effective temperature, metallicity, interior profile;
  - planet:
    - forcings: `StellarSources`, the system age;
    - orbit: semi-major axis, eccentricity, period;
    - spin: rotation period, obliquity;
    - bulk: mass, radius, surface gravity, mean density;
    - interior profile (below), tectonic regime, core state and
      magnetic field;
    - envelope fraction (giants);
    - atmosphere: surface pressure, partial pressures per gas;
    - global climate: mean surface temperature, equilibrium temperature,
      Bond albedo, climate state (temperate, snowball, runaway);
    - surface liquid: its kind (none, water, hydrocarbons, nitrogen,
      lava), inventory, and ocean share of the surface (owned by the
      system engine);
    - life: arose, oxygenated.
- `InteriorProfile` (#534, Cross-sections): layers from the centre out
  (outer radius, material, state: solid, liquid, plasma, partly molten,
  porous) and radial samples (radius, density, pressure, temperature;
  for stars also the transport and the share of fusion). Every layer is
  `Traced`.
- Every enum of the contract is `#[non_exhaustive]`, so a track adds a
  variant without breaking a match elsewhere.
- Validation on construction (`Keyframe::new` and friends return a
  `Result`): sources ordered and counted, fractions in range, layers
  ordered by radius, keyframes ordered by date.

The exact field names and the list of gases, materials and codes are
left to the builder, checked against what the legacy planet model
produces (`legacy-final:crates/mapgen/src/planet/mod.rs`). mapgen1 and
mapgen3 review the pull request before merge, as agreed on #602: a
change they ask for goes in 2a, not in a follow-up.

### `system`

- Modules: `inputs` (the request, the present date, the root seed),
  `draw` (mapgen3: star, orbits, designations, sources), `planet`
  (mapgen1: planet model), `star` (computes the star's state and
  profile, whose types are in `timeline`), `engine` (assembles the
  timelines).
- `Inputs` in 2a *(my call)*: a schema version, the root seed, the
  present date, and the request, either a whole system or one body,
  each with optional host, metallicity, and archetype. An archetype is
  an `ArchetypeId` (a code); the targets behind each id, their
  tolerances and the backward search are mapgen1's. Inputs are the
  stored truth, so they derive serde from the start; a change to them
  bumps their schema version once the catalogue stores them (mapgen2's
  next pull request), never `ENGINE_VERSION`.
- The seam, fixed here so both sides work against it:
  - the draw produces, per planet, a `Placement`: designation, body seed,
    mass, orbit, spin, `StellarSources`, system age, metallicity,
    ice line, and the archetype id if any;
  - the planet model exposes
    `planet::model(&Placement, &mut Trace) -> PlanetState`; it draws its
    own inputs (composition, volatiles) from sub-seeds of the body seed,
    and runs the archetype's backward search when there are targets.
- `ENGINE_VERSION`: the version stamped on every timeline.
- 2a writes the types (`Inputs`, `Placement`) and the module doc
  comments that fix the signatures; the functions arrive with mapgen3's
  and mapgen1's pull requests.

## Fields left to later tracks

Not abandoned: each track's tracking issue gets a comment listing the
fields it adds to `timeline`.

| Field or type | Track |
| --- | --- |
| Moon designations, the orbit tree's nesting, tides, tidal lock and substellar longitude | T2 #604 |
| Ice-shell and giant-atmosphere states (shell, bands, winds) | T3 #605 |
| Several keyframes, thresholds, XUV flux received per keyframe | T4 #606 |
| Events with their state before and after | T5 #607 |
| Environment (galactic place, birth cluster), external events | T6 #608 |
| Star phases beyond the main sequence, remnants, companion stars, circumbinary variability | T7 #585 |
| Populations: belts, rings, dust rings | T8 #609 |
| Observed provenance in use, real relief | T9 #421 |
| Atmospheric optical depths, stellar wind and field at the body, spectrum class for colours | T10 #610 |

## Tests

Tier 1 only (unit tests in each crate):

- each constant against its published value; round trips between units;
- `StellarSources`: one source returns the primary's values bit for bit;
  two identical half-flux sources give the same total and equilibrium
  temperature as one;
- `Trace`: a `Traced` value finds its node, a node finds its sources,
  leaves carry provenance, the frozen graph round-trips through JSON;
- `Designation`: text round trip, ordering;
- `Seed`: a derivation pinned against a hand-computed value of the hash
  (a definition, not the code's output), distinct labels give distinct
  seeds;
- validation errors for each rule.

## Left to the builder

- The exact field names, gas and material lists, and cause codes (see
  above).
- The serde layout of the JSON (it is a cache, free to change).
- Whether `Traced<T>` exposes the value by `Deref` or by a getter.
