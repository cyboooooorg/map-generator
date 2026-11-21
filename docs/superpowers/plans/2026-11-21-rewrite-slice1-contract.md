# Slice 1, PR 2a: the contract — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Typed units and constants in `science`, the slice-1 contract
types in `timeline`, and the module layout and seam types of `system`.

**Architecture:** `science` gains hand-written unit newtypes and one
sourced constant per physical value. `timeline` holds the published
types: seeds, designations, a cause graph built through `Trace` and
`Traced<T>`, stellar sources, and the keyframe of a star or a planet.
`system` gets its inputs, the `Placement` handed from the draw to the
planet model, and doc-only modules that fix the signatures the next pull
requests write.

**Tech Stack:** Rust 2024 workspace, `serde` 1.0.228 (derive),
`serde_json` 1.0.149 (tests only), `cargo nextest` through `mise`.

**Spec:** `docs/superpowers/specs/2026-11-21-rewrite-contract-design.md`,
refining `docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md`.

## Global Constraints

- Worktree `~/Developer/mg-rewrite-contract`, branch
  `feat/rewrite-contract` (from `origin/main` 7c8d97aa).
- Dependencies: `timeline ──> science`, `system ──> timeline, science`;
  no other internal edge (checked by `tools/guardrails`).
- Root `clippy.toml` is strict: no unseeded random, clock, env or file.
- Every numeric `const` or `static` in `science`, and every `let` bound
  to a bare number other than 0, 0.5, 1, 2, has a doc comment starting
  with `Source:`, `Calibration:` or `Definition:` (tier-1 guardrail).
- Every enum of the contract is `#[non_exhaustive]`.
- No `todo!()`, no `unimplemented!()`: a function not written yet does
  not exist.
- No human text in the domain: causes are codes and numbers.
- Commits follow Conventional Commits, end with the session's
  attribution lines, footer `Refs: #602`.
- Crate READMEs stay under 150 lines.
- Run tests with `mise run test:tier1`; before the PR, `mise run ci`.

## Review Focus

1. A law returning NaN or infinity reaches `Trace::record`: it must be
   refused with an error naming the code, never stored (Task 3).
2. One stellar source: `total_flux`, `equilibrium_k` and `weighted` must
   return the primary's value bit for bit, with no arithmetic (Task 4).
3. A keyframe read back from JSON with a node id missing from its graph
   (a corrupted cache): `Keyframe::validate` must report it (Task 5).
4. Planet ranks past `z` (rank 25 is `bb`) must print and parse back
   (Task 2).
5. Seed labels that are empty or non-ASCII (a designation is ASCII today,
   catalogue names may not be later) must derive stable, distinct seeds
   (Task 2).

---

### Task 1: Units and constants in `science`

**Files:**

- Modify: `Cargo.toml` (workspace dependencies: `serde`, `serde_json`)
- Modify: `crates/science/Cargo.toml`, `crates/science/src/lib.rs`
- Create: `crates/science/src/units.rs` (the newtypes and their
  conversions; split into `units/si.rs`, `units/astro.rs` if it passes
  about 400 lines)
- Create: `crates/science/src/constants.rs`
- Modify: `crates/science/README.md`

**Interfaces:**

- Produces, in `science::units`, each a `#[serde(transparent)]` newtype
  over a public `f64` field, deriving `Clone, Copy, Debug, PartialEq,
  PartialOrd, Serialize, Deserialize`, with `Add`, `Sub` within the type,
  `Mul<f64>`, `Div<f64>`, and a `pub const SYMBOL: &'static str`:
  - SI: `Kelvin` ("K"), `Pascal` ("Pa"), `Kilogram` ("kg"), `Metre`
    ("m"), `Second` ("s"), `Watt` ("W"), `WattPerSquareMetre` ("W/m2"),
    `MetrePerSecondSquared` ("m/s2"), `KilogramPerCubicMetre` ("kg/m3"),
    `Radian` ("rad"), `Tesla` ("T").
  - Domain: `AstronomicalUnit` ("au"), `SolarMass` ("Msun"),
    `SolarRadius` ("Rsun"), `SolarLuminosity` ("Lsun"), `EarthMass`
    ("Mearth"), `EarthRadius` ("Rearth"), `EarthFlux` ("Searth"), `Bar`
    ("bar"), `Day` ("d"), `Year` ("yr"), `Gyr` ("Gyr"), `Degree` ("deg"),
    `Dex` ("dex").
  - `Fraction`: private field, `Fraction::new(f64) -> Option<Fraction>`
    (0 to 1 inclusive, finite), `Fraction::get(self) -> f64`, `SYMBOL` "1";
    serde rejects out-of-range values on deserialize.
  - `From` both ways for: `AstronomicalUnit`/`Metre`, `SolarMass`/
    `Kilogram`, `EarthMass`/`Kilogram`, `SolarRadius`/`Metre`,
    `EarthRadius`/`Metre`, `SolarLuminosity`/`Watt`, `EarthFlux`/
    `WattPerSquareMetre`, `Bar`/`Pascal`, `Day`/`Second`, `Year`/`Second`,
    `Gyr`/`Year`, `Degree`/`Radian`.
  - A `macro_rules!` may generate the newtypes; it must hold no numeric
    literal (the guardrail does not expand macros).
- Produces, in `science::constants` (values exact as written; each doc
  comment starts with the prefix shown):
  - `ASTRONOMICAL_UNIT: Metre = Metre(149_597_870_700.0)`, `Definition:`
    IAU 2012 Resolution B2.
  - `SOLAR_RADIUS: Metre = Metre(6.957e8)`, `SOLAR_LUMINOSITY: Watt =
    Watt(3.828e26)`, `SOLAR_IRRADIANCE: WattPerSquareMetre =
    WattPerSquareMetre(1361.0)`, `SOLAR_MASS_PARAMETER: f64 =
    1.327_124_4e20` (m³/s²), `EARTH_EQUATORIAL_RADIUS: Metre =
    Metre(6.378_1e6)`, `EARTH_MASS_PARAMETER: f64 = 3.986_004e14`
    (m³/s²): `Definition:` IAU 2015 Resolution B3 nominal values (Prša et
    al. 2016, AJ 152, 41).
  - `GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11` and
    `STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8`: `Source:` CODATA 2018
    (Tiesinga et al. 2021, RMP 93, 025010).
  - `SOLAR_MASS: Kilogram` and `EARTH_MASS: Kilogram`: `Definition:` the
    nominal mass parameter divided by `GRAVITATIONAL_CONSTANT` (computed
    in a `const`).
  - `DAY: Second = Second(86_400.0)`, `JULIAN_YEAR_DAYS: f64 = 365.25`,
    `YEARS_PER_GYR: f64 = 1e9`, `PASCALS_PER_BAR: f64 = 1e5`:
    `Definition:` (IAU and SI).
- `EarthRadius` is the IAU nominal equatorial radius. Say so in its doc:
  a law fitted against the mean radius (6371 km) converts explicitly.

- [ ] **Step 1: Add the dependencies.** Workspace: `serde = { version =
  "1.0.228", features = ["derive"] }`, `serde_json = "1.0.149"`. `science`
  depends on `serde`, dev-depends on `serde_json`.
- [ ] **Step 2: Write the failing tests** in `constants.rs` and
  `units.rs`:

  ```rust
  #[test]
  fn masses_from_mass_parameters_match_the_published_masses() {
      // Published: Sun 1.98847e30 kg (CODATA/IAU), Earth 5.9722e24 kg.
      assert!((SOLAR_MASS.0 / 1.988_47e30 - 1.0).abs() < 1e-4);
      assert!((EARTH_MASS.0 / 5.972_2e24 - 1.0).abs() < 1e-4);
  }

  #[test]
  fn one_earth_flux_is_the_solar_luminosity_spread_over_one_au() {
      let l = SOLAR_LUMINOSITY.0;
      let a = ASTRONOMICAL_UNIT.0;
      let s = l / (4.0 * std::f64::consts::PI * a * a);
      assert!((s / SOLAR_IRRADIANCE.0 - 1.0).abs() < 1e-3);
  }

  #[test]
  fn conversions_round_trip() {
      let a = AstronomicalUnit(1.523);
      assert_eq!(AstronomicalUnit::from(Metre::from(a)), a);
      // same for every From pair listed above, within 1e-12 relative
  }

  #[test]
  fn a_gyr_is_a_billion_julian_years_of_86400_s_days() {
      let s = Second::from(Year::from(Gyr(1.0))).0;
      assert!((s / 3.155_76e16 - 1.0).abs() < 1e-15);
  }

  #[test]
  fn fraction_accepts_its_bounds_and_rejects_the_rest() {
      assert!(Fraction::new(0.0).is_some() && Fraction::new(1.0).is_some());
      assert!(Fraction::new(-1e-9).is_none() && Fraction::new(1.000_001).is_none());
      assert!(Fraction::new(f64::NAN).is_none());
      assert!(serde_json::from_str::<Fraction>("1.5").is_err());
  }

  #[test]
  fn units_serialise_as_bare_numbers() {
      assert_eq!(serde_json::to_string(&Kelvin(288.0)).unwrap(), "288.0");
  }
  ```

  Test literals are exempt from the guardrail only inside `#[cfg(test)]`
  modules; check that `mise run test:tier1` passes the constants
  guardrail, and name the published values as test constants with a
  `Source:` comment if it does not.
- [ ] **Step 3: Run** `mise run test:tier1`. Expected: the new tests fail
  to compile.
- [ ] **Step 4: Implement** the types, conversions and constants listed
  in Interfaces.
- [ ] **Step 5: Run** `mise run test:tier1`. Expected: PASS, including
  the guardrails.
- [ ] **Step 6: Update** `crates/science/README.md`: the units, the rule
  "a field stores the unit the domain thinks in; conversions go through
  `From`", the constants' sources.
- [ ] **Step 7: Commit** `feat(science): add typed units and the sourced
  physical constants`.

### Task 2: Seeds and designations in `timeline`

**Files:**

- Modify: `crates/timeline/Cargo.toml` (`serde`; dev `serde_json`),
  `crates/timeline/src/lib.rs`
- Create: `crates/timeline/src/seed.rs`, `crates/timeline/src/designation.rs`,
  `crates/timeline/src/error.rs`

**Interfaces:**

- Produces `timeline::ContractError` (`#[non_exhaustive]` enum, derives
  `Debug, Clone, PartialEq`, implements `std::error::Error` and
  `Display` as a code with its parameters, no prose). Variants used by
  every later task: `InvalidDesignation(String)`, `InvalidCode(String)`,
  `NotFinite { code: String }`, `UnknownNode { node: u32 }`,
  `NoSource`, `TooManySources(usize)`, `LayersNotOrdered`,
  `KeyframesNotOrdered`, `NoKeyframe`, `MixedBodyKinds`.
- Produces `Seed`: `#[serde(transparent)]` over `u64`, deriving
  `Clone, Copy, Debug, PartialEq, Eq, Hash`;
  `Seed::new(u64) -> Seed`, `Seed::get(self) -> u64`,
  `Seed::derive(self, label: &str) -> Seed`, defined as
  `splitmix64(parent ^ fnv1a64(label.as_bytes()))`, with private
  `fn fnv1a64(&[u8]) -> u64` (offset basis `0xcbf2_9ce4_8422_2325`, prime
  `0x0000_0100_0000_01b3`) and `fn splitmix64(u64) -> u64` (add
  `0x9e37_79b9_7f4a_7c15`, then the mix with `0xbf58_476d_1ce4_e5b9` and
  `0x94d0_49bb_1331_11eb`, shifts 30, 27, 31).
- Produces `Designation { system: SystemNumber, body: BodyPath }`:
  - `SystemNumber(u32)`, 10 000 to 99 999 (the legacy `MGS` survey
    number range, `legacy-final:crates/mapgen/src/naming.rs`);
  - `#[non_exhaustive] enum BodyPath { Star, Planet { rank: u32 } }`,
    `rank` 0 to 649 (one or two letters);
  - `Display`: `MGS 12345 A` for the star, `MGS 12345 b` for rank 0;
    `FromStr` parses it back (`InvalidDesignation` otherwise);
  - serde as that text; `Ord` by system, then star before planets, then
    rank;
  - `pub fn planet_letters(rank: u32) -> String`, ported from
    `legacy-final:crates/mapgen/src/naming.rs:75` `planet_letter` with
    its test (`letters_and_numerals`, the letter asserts).

- [ ] **Step 1: Write the failing tests:**

  ```rust
  #[test]
  fn fnv1a64_matches_the_published_vectors() {
      assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
      assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
      assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
  }

  #[test]
  fn splitmix64_matches_the_reference_generator_from_zero() {
      // First output of Vigna's splitmix64.c seeded with 0.
      assert_eq!(splitmix64(0), 0xe220_a839_7b1d_cdaf);
  }

  #[test]
  fn derive_mixes_the_parent_with_the_label_hash() {
      let parent = Seed::new(42);
      assert_eq!(parent.derive("b").get(), splitmix64(42 ^ fnv1a64(b"b")));
      assert_ne!(parent.derive("").get(), parent.derive("b").get());
      assert_ne!(parent.derive("é").get(), parent.derive("e").get());
      assert_ne!(Seed::new(43).derive("b"), parent.derive("b"));
  }

  #[test]
  fn letters_follow_the_legacy_scheme() {
      assert_eq!(planet_letters(0), "b");
      assert_eq!(planet_letters(24), "z");
      assert_eq!(planet_letters(25), "bb");
  }

  #[test]
  fn designations_print_parse_and_sort() {
      for text in ["MGS 12345 A", "MGS 12345 b", "MGS 12345 bb", "MGS 99999 zz"] {
          let d: Designation = text.parse().unwrap();
          assert_eq!(d.to_string(), text);
      }
      for bad in ["MGS 123 b", "MGS 12345 a", "XYZ 12345 b", "MGS 12345 B"] {
          assert!(bad.parse::<Designation>().is_err(), "{bad}");
      }
      let star: Designation = "MGS 12345 A".parse().unwrap();
      let b: Designation = "MGS 12345 b".parse().unwrap();
      let bb: Designation = "MGS 12345 bb".parse().unwrap();
      assert!(star < b && b < bb);
  }
  ```

- [ ] **Step 2: Run** `mise run test:tier1`. Expected: compile failure.
- [ ] **Step 3: Implement** `seed.rs`, `designation.rs`, `error.rs` as in
  Interfaces.
- [ ] **Step 4: Run** `mise run test:tier1`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(timeline): add seeds and designations`.

### Task 3: The cause graph

**Files:**

- Create: `crates/timeline/src/cause.rs` (split `cause/trace.rs` and
  `cause/graph.rs` if it grows past about 400 lines)

**Interfaces:**

- Consumes: `ContractError` (Task 2), the units and `Fraction` (Task 1).
- Produces:
  - `Code`: `Cow<'static, str>` inside, a dotted lowercase code
    (`^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)*$`); `Code::of(&'static str)
    -> Code` panics on an invalid literal (a programmer error, tested);
    `Code::new(String) -> Result<Code, ContractError>`; serde as the
    string, validated on deserialize.
  - `#[non_exhaustive] enum Provenance { Input, Observed, Drawn, Imposed }`.
  - `#[non_exhaustive] enum NodeValue { Number { value: f64, unit:
    Cow<'static, str> }, Code(Code), Flag(bool) }`.
  - `trait NodeValueOf { fn node_value(&self) -> NodeValue; }`,
    implemented for every unit of Task 1 (`Number` with its `SYMBOL`),
    `Fraction`, `bool` (`Flag`), and by each contract enum (`Code` of
    its kebab-case name).
  - `NodeId(u32)`, `Copy, Ord`, serde transparent.
  - `#[non_exhaustive] enum Origin { Derived { from: Vec<NodeId> },
    Leaf(Provenance) }`; `Node { code: Code, value: NodeValue, origin:
    Origin }`.
  - `Traced<T> { value: T, node: NodeId }`, fields private, derives
    `Clone, Copy (when T: Copy), Debug, PartialEq, Serialize,
    Deserialize`; `value(&self) -> &T`, `node(&self) -> NodeId`. No
    public constructor besides `Trace`'s methods and deserialisation.
  - `Trace`: `Trace::new()`, `leaf<T: NodeValueOf>(&mut self, code:
    Code, value: T, provenance: Provenance) -> Result<Traced<T>,
    ContractError>`, `record<T: NodeValueOf>(&mut self, code: Code,
    value: T, from: &[NodeId]) -> Result<Traced<T>, ContractError>`,
    `node(&self, NodeId) -> Option<&Node>`. Both refuse a non-finite
    number (`NotFinite { code }`) and `record` refuses an id it did not
    issue (`UnknownNode`).
  - `CauseGraph { nodes: Vec<Node> }`, serde; `node(&self, NodeId) ->
    Option<&Node>`; `validate(&self) -> Result<(), ContractError>` (every
    `from` id exists and is smaller than its node's own id: the graph is
    built in order, so it is acyclic).
  - `trait TracedFields { fn for_each_node(&mut self, f: &mut dyn
    FnMut(&mut NodeId)); }`, implemented for `Traced<T>`, `Option<T>`,
    `Vec<T>`, `BTreeMap<K, V>` (values) where the inner type implements
    it. Later tasks implement it for each state struct.
  - `Trace::freeze<S: TracedFields>(&self, state: &mut S) ->
    Result<CauseGraph, ContractError>`: collects the ids `state` holds,
    copies the nodes reachable from them in increasing id order into a
    new graph renumbered from 0, and rewrites the ids in `state`.

- [ ] **Step 1: Write the failing tests:**

  ```rust
  #[test]
  fn a_recorded_value_finds_its_node_and_its_sources() {
      let mut trace = Trace::new();
      let flux = trace.leaf(Code::of("draw.flux"), EarthFlux(1.0), Provenance::Drawn).unwrap();
      let t = trace.record(Code::of("climate.equilibrium"), Kelvin(255.0), &[flux.node()]).unwrap();
      let node = trace.node(t.node()).unwrap();
      assert_eq!(node.origin, Origin::Derived { from: vec![flux.node()] });
      assert_eq!(*t.value(), Kelvin(255.0));
  }

  #[test]
  fn non_finite_values_are_refused_with_their_code() {
      let mut trace = Trace::new();
      let err = trace.record(Code::of("climate.t"), Kelvin(f64::NAN), &[]).unwrap_err();
      assert_eq!(err, ContractError::NotFinite { code: "climate.t".into() });
      assert!(trace.leaf(Code::of("x"), Kelvin(f64::INFINITY), Provenance::Input).is_err());
  }

  #[test]
  fn an_id_from_another_trace_is_refused() {
      let mut other = Trace::new();
      let mut trace = Trace::new();
      let far = { other.leaf(Code::of("a"), Kelvin(1.0), Provenance::Input).unwrap();
                  other.leaf(Code::of("b"), Kelvin(1.0), Provenance::Input).unwrap() };
      assert!(matches!(trace.record(Code::of("c"), Kelvin(1.0), &[far.node()]),
                       Err(ContractError::UnknownNode { .. })));
  }

  #[test]
  fn codes_are_dotted_lowercase() {
      assert!(Code::new("climate.surface-temperature".into()).is_ok());
      for bad in ["", "Climate", "climate.", ".x", "a b", "a..b"] {
          assert!(Code::new(bad.into()).is_err(), "{bad}");
      }
  }

  #[test]
  #[should_panic]
  fn an_invalid_code_literal_panics() { let _ = Code::of("Bad Code"); }

  #[test]
  fn freezing_keeps_only_the_reachable_nodes_renumbered() {
      // leaves a, b, unrelated u; x from a; y from x and b. Freeze a state
      // holding only y: the graph has a, b, x, y (4 nodes, not u), ids 0..4,
      // y's sources resolve, and the state's id now points at y's copy.
  }

  #[test]
  fn a_frozen_graph_round_trips_through_json() { /* serde_json, assert_eq */ }

  #[test]
  fn validate_reports_a_dangling_source() {
      // A CauseGraph deserialised from JSON whose node 1 cites node 7.
  }
  ```

- [ ] **Step 2: Run** `mise run test:tier1`. Expected: compile failure.
- [ ] **Step 3: Implement** `cause.rs` as in Interfaces.
- [ ] **Step 4: Run** `mise run test:tier1`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(timeline): add the cause graph with traced values`.

### Task 4: Stellar sources

**Files:**

- Create: `crates/timeline/src/sources.rs`

**Interfaces:**

- Consumes: `Traced`, `TracedFields`, `ContractError` (Tasks 2-3).
- Produces:
  - `StellarSource { flux: Traced<EarthFlux>, equilibrium: Traced<Kelvin>,
    effective_temperature: Traced<Kelvin>, mass: Traced<SolarMass>,
    luminosity: Traced<SolarLuminosity> }`, public fields. Doc of
    `flux`: orbit-mean bolometric flux, eccentricity factor applied. Doc
    of `equilibrium`: from this star alone, at Earth's Bond albedo 0.3
    (255 K at one Earth flux, as
    `legacy-final:crates/mapgen/src/star.rs:322`), eccentricity factor
    applied.
  - `StellarSources` (private `Vec<StellarSource>`):
    `new(Vec<StellarSource>) -> Result<Self, ContractError>` (1 to 3,
    else `NoSource` or `TooManySources(n)`), `primary(&self) ->
    &StellarSource` (the first), `iter()`, `len()`, `total_flux(&self)
    -> EarthFlux`, `equilibrium_k(&self) -> Kelvin` ((Σ T⁴)^(1/4)),
    `weighted(&self, f: impl Fn(&StellarSource) -> f64) -> f64`
    (Σ flux_i·f_i / Σ flux_i). With one source each helper returns the
    primary's value or `f(primary)` without arithmetic. Serde validates
    the count on deserialize. Implements `TracedFields`.

- [ ] **Step 1: Write the failing tests** (build sources with a `Trace`
  and leaves):

  ```rust
  #[test]
  fn one_source_returns_the_primary_bit_for_bit() {
      // flux 0.7371, equilibrium 236.123456789, teff 5772.0
      let s = sources(&[(0.737_1, 236.123_456_789, 5772.0)]);
      assert_eq!(s.total_flux().0.to_bits(), 0.737_1f64.to_bits());
      assert_eq!(s.equilibrium_k().0.to_bits(), 236.123_456_789f64.to_bits());
      assert_eq!(s.weighted(|x| x.effective_temperature.value().0), 5772.0);
  }

  #[test]
  fn two_half_sources_add_up_to_one() {
      // Two identical stars, each 0.5 Earth flux and T_eq = 255 / 2^(1/4).
      let t = 255.0 / 2f64.powf(0.25);
      let s = sources(&[(0.5, t, 5772.0), (0.5, t, 5772.0)]);
      assert!((s.total_flux().0 - 1.0).abs() < 1e-12);
      assert!((s.equilibrium_k().0 - 255.0).abs() < 1e-9);
  }

  #[test]
  fn weighting_is_by_flux() {
      let s = sources(&[(0.75, 250.0, 6000.0), (0.25, 180.0, 3000.0)]);
      assert!((s.weighted(|x| x.effective_temperature.value().0) - 5250.0).abs() < 1e-9);
  }

  #[test]
  fn the_count_is_one_to_three() {
      // 0 -> NoSource, 4 -> TooManySources(4), also through serde_json.
  }
  ```

- [ ] **Step 2: Run** `mise run test:tier1`. Expected: compile failure.
- [ ] **Step 3: Implement** `sources.rs`.
- [ ] **Step 4: Run** `mise run test:tier1`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(timeline): add the stellar sources as a list (#592)`
  with footer `Refs: #592`.

### Task 5: States, keyframes and the timeline

**Files:**

- Create: `crates/timeline/src/interior.rs`, `crates/timeline/src/state.rs`
  (star and planet states), `crates/timeline/src/keyframe.rs` (keyframe,
  timeline, engine version)
- Modify: `crates/timeline/src/lib.rs` (re-exports), `crates/timeline/README.md`

**Interfaces:**

- Consumes: Tasks 1-4.
- Produces (every struct derives `Clone, Debug, PartialEq, Serialize,
  Deserialize` and implements `TracedFields`; every enum is
  `#[non_exhaustive]`, implements `NodeValueOf` and serialises
  kebab-case):
  - `interior.rs`:
    - `enum Material { Iron, Rock, RockIce, Water, Ice, Steam, Envelope,
      Plasma }` (legacy `planet/mod.rs:158` plus `Ice` and `Plasma`);
    - `enum MatterState { Solid, Liquid, Gas, Plasma, PartlyMolten,
      Porous }`;
    - `enum Transport { Radiative, Convective }`;
    - `Layer { outer_radius: Traced<Metre>, mass: Traced<Kilogram>,
      material: Material, state: Traced<MatterState> }` (the mass is
      stored: cross-sections and the moment of inertia read it, never
      an integral of the samples);
    - `RadialSample { radius: Metre, density: KilogramPerCubicMetre,
      pressure: Pascal, temperature: Kelvin, transport:
      Option<Transport>, fusion_share: Option<Fraction> }`;
    - `RadialSamples(Vec<RadialSample>)`, `NodeValueOf` as the sample
      count (`Number`, unit "1");
    - `InteriorProfile { layers: Vec<Layer>, samples:
      Traced<RadialSamples> }`, `validate`: layers strictly increasing
      in outer radius, samples increasing in radius (`LayersNotOrdered`).
  - `state.rs`:
    - `enum StarPhase { MainSequence }`;
    - `StarState { phase: StarPhase, mass: Traced<SolarMass>, radius:
      Traced<SolarRadius>, luminosity: Traced<SolarLuminosity>,
      effective_temperature: Traced<Kelvin>, metallicity: Traced<Dex>,
      interior: Option<InteriorProfile> }` (no stellar structure before
      mapgen3's study);
    - `Orbit { semi_major_axis: Traced<AstronomicalUnit>, eccentricity:
      Traced<Fraction>, period: Traced<Day>, perihelion_longitude:
      Traced<Degree> }`; validation refuses an eccentricity of 1 or more
      (`ContractError::EccentricityNotBelowOne`, a new variant);
    - `Spin { rotation_period: Traced<Day>, obliquity: Traced<Degree> }`;
    - `Bulk { mass: Traced<EarthMass>, radius: Traced<EarthRadius>,
      surface_gravity: Traced<MetrePerSecondSquared>, mean_density:
      Traced<KilogramPerCubicMetre> }`;
    - `Composition { core_mass_fraction: Traced<Fraction>,
      water_fraction: Traced<Fraction> }`;
    - `enum PlanetClass { Rocky, SuperEarth, IceWorld, MiniNeptune,
      IceGiant, GasGiant, HotJupiter, HotNeptune, LavaWorld,
      VolcanicWorld, RunawayGreenhouse, HydrocarbonWorld, OceanWorld,
      SteamWorld }` (legacy `planet/classify.rs:14`; the planet's own
      class, also read by the system draw);
    - `Heat { radiogenic: Traced<Watt>, tidal: Traced<Watt>, secular:
      Traced<Watt> }`;
    - `Water { inventory: Traced<Kilogram>, runaway: Traced<bool>,
      boiled: Traced<bool> }` (runaway by insolation, boiled by the
      climate; legacy `planet/regime.rs:37`);
    - `enum Renewal { Formation, Plates, Episodic, HeatPipe, IceLid,
      LiquidCycle, VolatileIce, TidalEpisode }`; `TerrainUnit { age:
      Traced<Gyr>, share: Traced<Fraction>, renewal: Renewal }`;
      `Terrain { units: Vec<TerrainUnit> }` (legacy
      `planet/surface_age.rs:26-58`);
    - `enum TectonicRegime { Plates, Stagnant, Episodic, HeatPipe }`
      (legacy `geology.rs:76`); `enum CoreState { None, Liquid, Frozen }`;
      `Core { state: Traced<CoreState>, field: Traced<Tesla> }`;
    - `enum Gas { N2, Co2, H2o, Ch4, Nh3, So2, O2 }` (legacy
      `planet/atmosphere.rs:151`); `Atmosphere { surface_pressure:
      Traced<Bar>, partial: BTreeMap<Gas, Traced<Bar>> }`;
    - `enum ClimateState { Temperate, Snowball, Runaway }`;
      `GlobalClimate { surface_temperature: Traced<Kelvin>, equilibrium:
      Traced<Kelvin>, bond_albedo: Traced<Fraction>, state:
      Traced<ClimateState> }`;
    - `enum LiquidKind { Water, Hydrocarbons, Nitrogen, Lava }`;
      `SurfaceLiquid { kind: Traced<LiquidKind>, inventory:
      Traced<Kilogram>, ocean_share: Traced<Fraction> }`;
    - `Life { arose: Traced<bool>, oxygenated: Traced<bool> }`;
    - `Surface { regime: Traced<TectonicRegime>, core: Core, water:
      Water, atmosphere: Option<Atmosphere>, climate: GlobalClimate,
      liquid: Option<SurfaceLiquid>, terrain: Terrain, life: Life }`;
    - `PlanetState { sources: StellarSources, age: Traced<Gyr>, orbit:
      Orbit, spin: Spin, bulk: Bulk, composition: Composition, class:
      Traced<PlanetClass>, heat: Heat, interior: InteriorProfile,
      envelope_fraction: Traced<Fraction>, surface: Option<Surface> }`
      (no surface: a giant).
  - `keyframe.rs`:
    - `enum BodyState { Star(StarState), Planet(PlanetState) }`;
    - `EngineVersion(u32)`, serde transparent;
    - `Keyframe { date: Gyr, state: BodyState, causes: CauseGraph }`
      (fields read-only through getters); `Keyframe::new(date: Gyr,
      state: BodyState, trace: &Trace) -> Result<Keyframe,
      ContractError>` validates the date and the profiles, then
      `trace.freeze(&mut state)`; `Keyframe::validate(&self)`: the
      graph validates and every id the state holds exists in it;
    - `Timeline { body: Designation, seed: Seed, engine: EngineVersion,
      present: Gyr, keyframes: Vec<Keyframe> }`, `Timeline::new(...) ->
      Result<Timeline, ContractError>`: at least one keyframe
      (`NoKeyframe`), dates strictly increasing (`KeyframesNotOrdered`),
      all the same body kind (`MixedBodyKinds`), and `validate` on each.

- [ ] **Step 1: Write the failing tests.** A test helper
  `fn earth(trace: &mut Trace) -> PlanetState` builds a complete state
  from leaves (values: legacy Earth, any plausible numbers: the test
  checks structure, not physics). Tests:

  ```rust
  #[test]
  fn a_planet_keyframe_explains_every_field() {
      let mut trace = Trace::new();
      let state = earth(&mut trace);
      let k = Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace).unwrap();
      k.validate().unwrap();
      let mut ids = Vec::new();
      let mut state = k.state().clone();
      state.for_each_node(&mut |id| ids.push(*id));
      assert!(ids.iter().all(|id| k.causes().node(*id).is_some()));
  }

  #[test]
  fn a_keyframe_round_trips_through_json() { /* to_string, from_str, assert_eq */ }

  #[test]
  fn a_corrupted_keyframe_is_reported() {
      // Serialise, replace one node id in the state's JSON by 9999,
      // deserialise: validate() returns UnknownNode { node: 9999 }.
  }

  #[test]
  fn layers_out_of_order_are_refused() { /* LayersNotOrdered */ }

  #[test]
  fn an_eccentricity_of_one_is_refused() { /* EccentricityNotBelowOne */ }

  #[test]
  fn a_timeline_needs_ordered_keyframes_of_one_body_kind() {
      // empty -> NoKeyframe; dates 4.57 then 4.0 -> KeyframesNotOrdered;
      // a star keyframe then a planet keyframe -> MixedBodyKinds.
  }
  ```

- [ ] **Step 2: Run** `mise run test:tier1`. Expected: compile failure.
- [ ] **Step 3: Implement** `interior.rs`, `state.rs`, `keyframe.rs`.
- [ ] **Step 4: Run** `mise run test:tier1`. Expected: PASS.
- [ ] **Step 5: Update** `crates/timeline/README.md`: the types, the
  cause rule (only a `Trace` makes a `Traced`; one trace per system;
  each keyframe freezes its own subgraph), the seed rule, and the
  fields left to later tracks with a link to the design's table.
- [ ] **Step 6: Commit** `feat(timeline): add the star and planet keyframes
  of slice 1`.

### Task 6: The `system` layout and seam

**Files:**

- Modify: `crates/system/Cargo.toml` (`serde`; dev `serde_json`),
  `crates/system/src/lib.rs`, `crates/system/README.md`
- Create: `crates/system/src/inputs.rs`, `crates/system/src/placement.rs`,
  `crates/system/src/archetype.rs`,
  and doc-only `crates/system/src/{draw,planet,star,engine}.rs`

**Interfaces:**

- Consumes: Tasks 1-5.
- Produces:
  - `pub const ENGINE_VERSION: EngineVersion = EngineVersion(1);` in
    `lib.rs`, doc: bumped whenever a timeline or a world could change
    for the same inputs.
  - `archetype.rs`: `ArchetypeId(Code)` serde transparent, `Ord` (add
    `Ord` to `Code` if Task 3 did not derive it); the module doc fixes
    the split: `Archetype { id, placement: ArchetypePlacement, targets:
    ArchetypeTargets }`, placement read by the draw and filled by
    mapgen3 (host, insolation or period band, mass draw, eccentricity,
    tilt, age), targets filled by mapgen1 (inputs, present-day targets,
    tolerances); the backward search moves only the planet's own inputs
    and records an unreachable target as an outcome with its cause.
  - `inputs.rs`: `#[non_exhaustive] enum Request { System(SystemRequest),
    Body(BodyRequest) }`; `SystemRequest { host_mass: Option<SolarMass>,
    metallicity: Option<Dex>, archetypes: BTreeMap<ArchetypeId, u32> }`
    (a count per archetype); `BodyRequest {
    host_mass: Option<SolarMass>, metallicity: Option<Dex>, archetype:
    Option<ArchetypeId> }`; `Inputs { schema: u32, seed: Seed, present:
    Option<Gyr>, request: Request }` (no present: the draw takes the age
    within the star's lifetime) with `pub const INPUTS_SCHEMA: u32 = 1`.
    All serde. Doc: the stored truth; mapgen3 may refine the request
    and mapgen1 the archetypes in their pull requests.
  - `placement.rs`: `Trial { seed: Seed, mass: Traced<EarthMass>,
    orbit: Orbit, sources: StellarSources, age: Traced<Gyr>,
    metallicity: Traced<Dex>, ice_line: Traced<AstronomicalUnit>,
    archetype: Option<ArchetypeId> }` and
    `Placement { designation: Designation, seed: Seed, spin: Spin,
    trial: Trial }`, not serialised. Doc: a trial runs before letters
    exist (they follow distance once every orbit is placed), so it has
    no designation and its own seed (`system_seed.derive("trial/<archetype
    id>")` or `"trial/slot-<n>"`, as legacy `system.rs:896-920`); the
    placement adds the designation, the body seed and the spin, and
    `model` runs again on it. The orbit's period is the draw's (Kepler),
    never recomputed.
  - Doc-only modules, each a `//!` comment fixing its owner and its
    future signature, no items:
    - `draw` (mapgen3): `pub fn draw(inputs: &Inputs, trace: &mut
      Trace) -> Result<Drawn, SystemError>`, `Drawn` holding the star's
      designation, seed and `StarState` and one `Placement` per planet;
    - `planet` (mapgen1): `pub fn class_of(trial: &Trial) ->
      Result<PlanetClass, SystemError>` (runs `model` with a scratch
      `Trace` and returns the class: one classifier) and `pub fn
      model(placement: &Placement, trace: &mut Trace) ->
      Result<PlanetState, SystemError>`; draws its own inputs from
      sub-seeds of the body seed, runs the archetype's backward search;
    - `star` (mapgen3): computes `StarState` (main-sequence values, the
      age draw) and later its profile;
    - `engine`: `pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>,
      SystemError>`: one `Trace` for the system, one timeline per body.

- [ ] **Step 1: Write the failing test** in `inputs.rs`:

  ```rust
  #[test]
  fn inputs_round_trip_through_json() {
      let inputs = Inputs {
          schema: INPUTS_SCHEMA,
          seed: Seed::new(7),
          present: Some(Gyr(4.57)),
          request: Request::Body(BodyRequest {
              host_mass: None,
              metallicity: None,
              archetype: Some(ArchetypeId(Code::of("earth-like"))),
          }),
      };
      let json = serde_json::to_string(&inputs).unwrap();
      assert_eq!(serde_json::from_str::<Inputs>(&json).unwrap(), inputs);
  }
  ```

- [ ] **Step 2: Run** `mise run test:tier1`. Expected: compile failure.
- [ ] **Step 3: Implement** the files listed.
- [ ] **Step 4: Run** `mise run test:tier1`. Expected: PASS.
- [ ] **Step 5: Update** `crates/system/README.md`: the modules and their
  owners, the seam, `ENGINE_VERSION` against `INPUTS_SCHEMA`.
- [ ] **Step 6: Commit** `feat(system): lay out the engine and the seam
  between the draw and the planet model`.

### Task 7: Checks, review, pull request (main session)

- [ ] **Step 1:** `mise run ci` in the worktree. Expected: format,
  clippy, tiers 1 and 2 within budget, markdownlint all pass.
- [ ] **Step 2:** `branch-reviewer` and `test-reviewer` on the branch,
  given the design and this plan. Fix on the branch, re-run `mise run
  ci`.
- [ ] **Step 3:** `pr-shepherd` pushes and opens the PR into `main`
  (`Refs #602`, `Refs #592`; body lists the calls marked *my call* in
  the design for review).
- [ ] **Step 4:** Comment on each of #604, #605, #606, #607, #608, #585,
  #609, #421, #610 with the `timeline` fields it adds (the design's
  table).
- [ ] **Step 5:** Message mapgen1 and mapgen3: the PR is up for review.
  Merge only with the user's approval.
