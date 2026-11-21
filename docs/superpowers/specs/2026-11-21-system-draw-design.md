# System draw at the present (slice 1)

Date: 2026-11-21. Owner: mapgen3. Track T0, slice 1 (#602), roadmap #427.
Parent designs: [architecture rewrite](2026-11-20-architecture-rewrite-design.md)
and [contract between the engines](2026-11-21-rewrite-contract-design.md)
(merged as #613). Legacy reference: tag `legacy-final`.

## Intent

The system engine draws a system at its present date: one
main-sequence star, its planets on their orbits, the stellar sources
each planet receives (#592), designations, and a `cli system` command
that prints the timeline as JSON with an `--explain` view of the cause
graph. This is the input every other engine reads in slice 1: the
planet model (mapgen1) runs on each placement, the world and the
catalogue (mapgen2) store and render the result.

**Why the random draw aims for variety.** When the request does not fix
the host or the planets, the user wants "places worth the detour", and
what makes a system worth it is variety from one draw to the next (user,
2026-11-21). The real stellar and planetary frequencies (three quarters
M dwarfs, most planets small) belong to a later galactic-map mode, not
to this draw. So every distribution the random draw uses for *what* it
picks is a design choice, documented with `Definition:`; every physical
value of *what was picked* is sourced (`Source:` or `Calibration:`).

## Goals

- A request with nothing fixed gives systems that differ in host class,
  age, metallicity and architecture, and stay physically plausible.
- A request that fixes the host mass, the metallicity, the present date
  or archetype counts gets exactly that, or a refusal that says why.
- The star's present state (mass, radius, luminosity, effective
  temperature, metallicity, lifetime) and its interior profile come
  from published stellar models, never from invented numbers.
- Same `Inputs` gives the same `Drawn`, byte for byte, on every
  platform (determinism test, tier 1).
- `cli system` prints the timeline JSON and explains any traced value.

## Non-goals (this slice)

- Hosts above 8 M☉ (early B, O): #615. Hosts that are not on the main
  sequence, companions, circumbinary variability: track T7 (#585).
- Moons, rings, belts, the orbit tree's nesting: T2 (#604).
- Any keyframe other than the present: T4 (#606).
- The planet model itself, its archetype targets and the backward
  search: mapgen1. The world and the catalogue: mapgen2.

## Anti-goals

- A random draw that mostly shows M dwarfs, or systems that all look
  alike: that is the representative draw the user ruled out.
- Implausible neighbourhoods produced by drawing planets independently
  (a hot Jupiter inside a compact chain of Earths).
- Any number without a source or a stated definition (CLAUDE.md, No
  invented data).
- Moving an asked planet to make room for the drawn ones.

## Constraints

- The contract (#613) fixes the types and signatures: `Inputs`,
  `Request::{System, Body}`, `Trial`, `Placement`, `StellarSources`,
  `StarState`, `InteriorProfile`, seeds by `derive(label)`, ChaCha8,
  designations `MGS nnnnn A` / `b`, `system::draw(inputs, trace) ->
  Result<Drawn, SystemError>`.
- `planet::class_of` and `planet::model` stay crate-private until
  mapgen1's PR 2 (atmosphere, escape, climate). The draw takes the
  classifier as a parameter (`draw_with(inputs, trace, classify)` with
  `classify: impl Fn(&Trial, &Trace) -> Result<PlanetClass,
  SystemError>`); tests pass a test-only closure; the public `draw`
  wires `planet::class_of` in once it is public. No stub in production
  code (agreed with mapgen1).
- `SystemError` does not exist yet: this work creates it in
  `crates/system/src/error.rs`, `#[non_exhaustive]`; mapgen1 adds its
  variants after.
- Cause codes start with `system.draw.` (star values `system.star.`).
- Any change to `timeline` types is its own small PR, announced to
  mapgen1 and mapgen2.

## Approaches considered

### Random host distribution

1. **Representative** (stellar frequencies of the solar neighbourhood,
   as legacy `star::draw_type`): faithful, but three draws in four are
   M dwarfs. Rejected by the user: real frequencies go to the galactic
   map.
2. **Variety, chosen**: equal share per spectral class, then a mass
   within the class. Uniform in mass or in temperature was rejected
   because class B (2.2-8 M☉) would take half the range.

### Star values and interior

1. Mean dwarf sequence (legacy, Pecaut & Mamajek): no age or
   metallicity dependence, no interior. Rejected.
2. Polytropes: a single or composite polytrope is off by a factor of
   10-100 beyond 0.6 R for the Sun (study measured against B23). Kept
   only where the star is fully convective.
3. MIST tracks: good globals (Sun within 6 % in central density) but no
   licence ("All rights reserved") and no profiles.
4. **Offline grid computed with YREC, chosen** (user, "Calculer une
   grille"): full structures, all four metallicities, reproducible
   from the inlists. YREC fails below 0.3 M☉ (rescaled runs from the
   0.3 M☉ start model end in NaN), so the low-mass end uses Baraffe et
   al. 2015 plus an n = 1.5 polytrope (user, 2026-11-21).

### Planets of a random system

1. **Architecture families, chosen** (user, 2026-11-21): draw a family
   evenly, then fill its slots with archetypes under sourced spacing,
   period-ratio and eccentricity rules.
2. Independent draws (count, then each archetype): simple, but
   implausible neighbourhoods. Rejected.
3. A formation model (legacy population synthesis): plausible but
   follows the real frequencies, little variety. Rejected.

## Decisions

### The star

- **Asked**: host mass and metallicity taken as given; a mass outside
  0.08-8 M☉ or a metallicity outside −1 to +0.5 dex is refused
  (`SystemError`, naming #615 above 8 M☉).
- **Drawn class**: M, K, G, F, A, B, one chance in six each
  (`Definition:`). Class mass bounds from Pecaut & Mamajek (2013, table
  version 2022.04.16, mass at the start of each class; legacy
  `star.rs:57`): M 0.08-0.57, K 0.57-0.88, G 0.88-1.06, F 1.06-1.61,
  A 1.61-2.18, B 2.18-8 M☉. The class is only a bucket for the draw:
  the star's temperature comes from the models, so a metal-poor 1 M☉
  star is hotter than G2.
- **Drawn mass**: log-uniform within the class (`Definition:`).
- **Drawn metallicity**: uniform in [Fe/H] from −1 to +0.5
  (`Definition:`, the range of the grid).
- **Present date** when absent: uniform in the star's main-sequence
  life, capped at 13.8 Gyr (Planck 2018 age of the universe), a
  `Drawn` leaf, as legacy `star::draw_age_gyr`. An asked date beyond
  the star's main-sequence life is refused (giants are T7).
- **Asked archetypes that name a host** (placement part): the host is
  drawn inside the archetype's host band, as legacy `asked_star_of`.
  When several asked archetypes name hosts that do not overlap, the
  request is refused. An asked host mass always wins over an
  archetype's host band, as legacy (`system.rs`, "a star named is
  honoured, whatever it does to the planets asked"): the archetype is
  still placed in its insolation or period band, and a target its
  planet cannot reach there is an outcome of mapgen1's backward search,
  not a refusal *(my call)*.
- **Present date limits** *(my call)*: an asked date beyond 13.8 Gyr or
  beyond the end of the star's main sequence (central hydrogen
  10⁻⁴, where the YREC runs stop) is refused. Below about 0.8 M☉ the
  main sequence outlasts 13.8 Gyr, so the table only needs to know that
  the lifetime exceeds the cap.

### Star values at the present

- 0.3-8 M☉: the YREC grid, interpolated in mass, [Fe/H] and the
  fraction of main-sequence life elapsed (central hydrogen), for R, L,
  T_eff and the main-sequence lifetime.
- 0.08-0.3 M☉: Baraffe et al. (2015, A&A 577, A42) at solar
  metallicity for R, L, T_eff by age; the metallicity shift from
  Dartmouth (Dotter et al. 2008, via the Kiauhoku grids, CC-BY 4.0,
  Zenodo 14908017) as a relative offset from 0.15 to 0.3 M☉, held at
  its 0.15 M☉ value below. Only derived numbers enter the repository,
  with the citation; no file of theirs is redistributed (user,
  2026-11-21). At 0.15-0.3 M☉ the two agree within about 2 %; at
  0.3 M☉ YREC is about 60 K hotter than Baraffe and 115 K hotter than
  Dartmouth (grey atmosphere). Values are blended over 0.30-0.35 M☉
  (Baraffe tabulates 0.3 and 0.4 M☉; YREC starts at 0.3 M☉, where it
  is still fully convective, and has a radiative zone at 0.4 M☉); the
  interior profile switches at 0.30 M☉. Both are documented.

### Interior profile

- Above 0.3 M☉: the YREC structure at the nearest grid points,
  interpolated, as `InteriorProfile` layers by transport (convective
  or radiative core, radiative zone, convective envelope), the
  outermost ending at the photosphere, and radial samples with density, pressure, temperature,
  transport and the cumulative share of nuclear power inside the
  radius.
- 0.08-0.3 M☉: fully convective (Chabrier & Baraffe 1997), an n = 1.5
  polytrope scaled to the star's mass and radius. Measured: within 4 %
  of YREC's density profile to 0.8 R at 0.3 M☉; central density within
  5 % (0.3 M☉) and 9 % (0.08 M☉) of Baraffe 2015.
- Validation kept as a regression test: the 1 M☉, 4.57 Gyr profile
  against the B23 standard solar model (Zenodo 10174172, CC-BY 4.0).
  Measured on the grid run: R 0.9995 R☉, ρ_c 145 vs 150 g cm⁻³, T_c
  1.534 vs 1.558 × 10⁷ K, envelope base 0.727 vs 0.713 R, density,
  pressure, temperature and enclosed mass within about 6 % from 0.05 to
  0.95 R.

### Stellar data in the repository

The grid has been run (2026-11-21, 312 runs, all ended at central
hydrogen 10⁻⁴ or 20 Gyr) and the Baraffe 2015 and Dartmouth files are
downloaded, in mapgen3's session scratch space. mapgen3 extracts the
table as part of PR 1; the raw outputs (21 GB) are not kept, since the
tools regenerate them.

- A compact table in the `science` crate (format and size measured at
  extraction; target well under 2 MB): per (mass, [Fe/H]) a few
  snapshots chosen by central hydrogen (ZAMS, then 90, 70, 50, 30, 10
  and 1 % of the initial value; low masses stop at 20 Gyr), each with
  the globals, the transport layers with their exact bounds and about
  50 radial points. If it is too large, the mass step above 2 M☉ is
  widened where the measured interpolation error allows.
- `tools/stellar-grid/`: the YREC namelists (312, of which 44 derived
  from the 3.0 M☉ one: 2.0-2.9 M☉, and 1.8-1.9 M☉ at [Fe/H] −1 and
  −0.5), the preparation, run and extraction scripts, and a README to
  regenerate the table (YREC from pip `yrec`, Pinsonneault 2025,
  Zenodo 17981745; about 35 minutes on 8 cores).
- `docs/science/` page on star structure (what is inside a star, why
  the Sun has a convective envelope and an M dwarf is convective
  throughout), with the comparisons above; the study goes to the Wiki.

### Planets of a random system

- **Family**, one of five, evenly (`Definition:`):
  1. compact: 4-7 small planets inside about 0.3 AU, similar sizes
     ("peas in a pod", Weiss et al. 2018), near resonances;
  2. solar-like: rocky planets inside the ice line, giants beyond;
  3. hot Jupiter: one giant at a few days, no close companions, maybe a
     distant one;
  4. eccentric giants: one or two giants on eccentric orbits, few inner
     rocky planets;
  5. sparse: zero to two planets.
- Every family is open to every host class (variety). A family is
  dropped for a host only when it is physically impossible there (its
  innermost slot would sit inside the star's inner edge), and the
  family is drawn again among the rest.
- **No guaranteed remarkable planet**: a system of barren rocks is a
  valid draw (user, 2026-11-21).
- **Asked archetypes first**: they are placed in their own bands, then
  the family fills the rest around them without moving them.
- **Physical rules on the class, not the slot** (#476): eccentricity,
  hot-giant clearing and Hill spacing read the trial's class from
  `classify`, one code path. Order for each slot, asked archetypes
  first, then the family's slots from the inside out:
  1. draw the mass and the orbit's semi-major axis from the slot (or
     the archetype's placement: insolation or period band, mass draw);
  2. build the `Trial` (period by Kepler, eccentricity zero, sources at
     that distance) and classify it;
  3. draw the eccentricity from the class's rule, then check spacing
     and hot-giant clearing against the planets already placed;
  4. on a violation, redraw the slot from its own stream (a drawn slot
     only; an asked archetype is never moved, an earlier drawn
     neighbour may be dropped instead), within the tries budget; when
     the budget runs out, a drawn slot is left empty and an asked
     archetype is refused with `SystemError`.
  The archetype table, with its placement half, is
  `system::archetype` (contract); this work fills the placement half
  for the legacy archetypes (`legacy-final:crates/mapgen/src/archetype.rs`).
- The numbers inside each family (slot counts, period ratios, spacing
  in mutual Hill radii, eccentricity ranges, stability criterion) need
  a short study first (No invented data), written by mapgen3 before PR
  2's draw code; a `Definition:` may stand only for a choice of what to
  pick, never for a physical rule. Candidate sources: Weiss et
  al. 2018, Fabrycky et al. 2014 (period ratios), Van Eylen et al. 2019
  (small-planet eccentricity), Kipping 2013 (giant eccentricity),
  Chambers et al. 1996 / Pu & Wu 2015 (spacing), Neil & Rogers 2020
  (small-planet masses, found by mapgen1 in #617's study). Where a
  source gives a population shape and variety argues for flattening
  it, the flattening is a `Definition:` and the source's range stays.

### Stellar sources

One `StellarSource` per star (one in this slice), as fixed by the
contract: flux at the planet (with the eccentricity factor), its
equilibrium temperature, the star's effective temperature, mass and
luminosity. No spectrum: the planet model derives what it needs from
T_eff.

### Orbits and spin

- The draw computes the period from Kepler's law and the host mass;
  perihelion longitude uniform in [0°, 360°) (`Definition:`).
- Eccentricity from the class and family rules above; obliquity and
  rotation period from the archetype placement or, when drawn, from
  sources carried over from legacy (`docs/science/axial-tilt-climate.md`,
  `eccentric-orbits.md`) after checking each has a source.
- Designations follow distance once every orbit is placed: `b` the
  closest, `bb` after `z`, ranks from 650 refused (contract).

### Seeds and determinism

- From the system seed: `derive("star")` for class, mass, metallicity
  and age; `derive("family")` for the family and its slots; trials
  `derive("trial/<archetype>")` or `derive("trial/slot-<n>")`; body
  seeds `derive(<designation text>)` (contract). Each draw reads its
  own stream, so adding a draw never shifts another.
- Determinism test: a fixed set of `Inputs` hashed to golden values of
  the serialised `Drawn`, tier 1; floating-point paths avoid
  platform-dependent functions in the hashed values or round them
  before hashing.

### Errors and traces

- `SystemError` (non-exhaustive): host out of range, present date out
  of the star's life, incompatible asked hosts, no room for the asked
  archetypes after the allowed tries, and the classifier's errors.
- Every drawn or computed value is a `Traced` leaf or node; codes
  `system.draw.*` and `system.star.*`; `--explain <code or path>` walks
  the cause graph.

### `cli system`

- `mapgen system [--seed N] [--present GYR] [--host-mass M]
  [--metallicity DEX] [--archetype ID=COUNT ...] [--explain PATH]`:
  prints the timeline JSON (one keyframe, the present). With
  `--explain`, prints the cause chain of the named value instead. Until
  mapgen1's model is public, the command prints the draw (star,
  orbits, sources, designations) without planet states.

## Delivery

Pull requests, each with its docs and tests, in this order:

1. **Star**: the stellar table and its tools, `system::star` (values,
   age, interior), the `docs/science` page, `SystemError`.
2. **Planet architecture study** then **draw**: families, orbits,
   spacing and stability, designations, stellar sources, the archetype
   placement half, determinism test.
3. **`cli system`** with the timeline JSON and `--explain`.

## Left to the builder

- The table's binary or text format and the interpolation scheme,
  provided the regression against B23 and the 0.3 M☉ seam test pass.
- The exact radial sampling (about 50 points; denser near transport
  boundaries is fine).
- The tries budget for placing slots.
