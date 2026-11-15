# Climate pass lot 2a: archetypes that reproduce their body

Refs #423 (track A), climate pass lot 2 (see #427). Closes #453 and #454.
Advances #464 (Titan's scatter; Triton's air is lot 2b).

## Intent

When the user asks for a `mars-like`, an `earth-like` or a `titan-like`
world, they want something close to the real Mars, Earth or Titan on every
seed. They do not want a body that a plausible history could have made
from the same start. Today one archetype gives very different airs
depending on the seed:

- `earth-like`: 0.54 to 3.6 bar (#453). The NH3 scatter cannot be pinned,
  and its photolysis feeds N2.
- `mars-like`: 0.003 to 0.033 bar (#454). The system age is drawn, and a
  young Mars has lost less air.
- Titan: 0.8 to 8.7 bar (#464). The NH3, CH4 and radiogenic draws all
  feed it.

Lot 2 was split at the user's request:

- **2a (this document)**: archetypes stay faithful to their body.
- **2b**: the physics of cold atmospheres and of the energy budget: #443,
  #447, #510, and Triton's part of #464.

2a goes first because it fixes the reference values that 2b calibrates
against.

## Goals

1. **Every scatter factor can be pinned.** The NH3 and S factors become
   overridable inputs, like CO2, N2 and CH4 today:
   - in `InputOverrides`, archetype `[inputs]`, `WorldRequest` and the CLI;
   - pinned back by `PlanetModel::formation()`, so that a drifted system
     planet keeps its N2. This is the second case in #453.
2. **The Solar System archetypes carry their system's age.** The user
   chose this: an asked Solar System archetype sets the age of the whole
   system. The planet does not get the air a Mars of the drawn age would
   have. The archetypes concerned are mercury, venus, earth, mars,
   jupiter, saturn, uranus and neptune, plus the moons io, europa,
   ganymede, callisto, titan, enceladus and triton (all with `-like`).
3. **The form says so.** This is the user's requirement. When an archetype
   fixes the age:
   - the age field (lone world) shows it;
   - the system preview's star line names the archetype that set it.
4. **Per-seed tests.** Over seeds 0..20, each faithful archetype lands in
   its body's range on every seed, not only on the median. The table is
   under Acceptance.

## Non-goals

- Variety of drawn (non-archetype) bodies: unchanged. Only archetypes are
  pinned.
- Modelling a young Mars's thicker air: sputtering and ion escape of
  small unmagnetised worlds. The age is pinned instead. It can be filed
  as a backlog issue if it ever matters to drawn bodies.
- Triton's 1.4e-5 bar and 38 K, the CH4 cold trap, SO2, secular cooling,
  the CO2 inventory and the Bond albedo: all in lot 2b.
- Exoplanet-reference archetypes do not get an age, because their star's
  age is poorly known. These are `hot-jupiter`, `hot-neptune`,
  `mini-neptune`, `super-earth`, `ocean-world` and `lava-world`. *My call.*

## Anti-goals

- No per-archetype fudge inside the physics. Faithfulness comes from
  pinning inputs (factors, radiogenic, age, mass, orbit) to the real
  body's values or tight ranges. It never comes from a special case in
  `climate.rs` or `atmosphere.rs`.
- An archetype's age never silently overrides an age the user typed.

## Design

### Inputs

- `InputOverrides` (`crates/mapgen/src/planet/inputs.rs:229`) gains
  `nh3: Option<f64>` and `s: Option<f64>`. `volatiles()` (`:413`) uses
  `overrides.nh3.unwrap_or(scatter.nh3)` and the same for `s`.
- The draw order is unchanged: draw everything, then override. Bodies that
  do not pin these factors keep the draws they have today.
- `ArchetypeInputs`, `WorldRequest::overrides`, the request validation
  (`VOLATILE_FACTOR` limits) and the CLI flags gain the same two fields.
  The form's model fields gain them for a lone world, in the same group as
  `ch4`.
- `PlanetModel::formation()` pins nh3 and s back as factors over the
  formation rule, as it does for co2, n2 and ch4.

### Archetype age

- `[inputs]` gains `age_gyr: ParamDefault` (optional), validated against
  the stars' lifetimes like `WorldRequest.age_gyr`. *My call*: it sits in
  `[inputs]` and not in `[orbit]`, because moon archetypes have no
  `[orbit]`, and the form already shows age among the model fields.
- The Solar System archetypes set `age_gyr = 4.57`, fixed. The Solar
  System's age is 4.568 Gyr (Bouvier & Wadhwa 2010,
  doi:10.1038/ngeo941). *My call*: a fixed value, not a range, since
  close to the real body is the goal.
- **Whole system** (`place_whole`, `crates/mapgen/src/system.rs:960`):
  - Before the star is drawn, collect the `age_gyr` of every asked planet
    and moon archetype.
  - If the request has no `age_gyr`, the system age is drawn inside the
    intersection of those ranges, from the AGE stream. An empty
    intersection is a new `RequestError::ArchetypeAges { … }`, naming the
    archetypes. It maps to an `ErrorDetail` and to EN and FR messages, and
    it shows in the live validation.
  - The age then feeds `draw_type_living` and `resolve_age_gyr` as an
    asked age would. The star stays alive at that age.
  - If the request has an explicit `age_gyr`, it wins, as request values
    win over archetype values everywhere else (`request.rs:487-496`). The
    form shows a warning, not a refusal (see Form).
- An archetype age goes through the same path as an asked age: no AGE
  draw is consumed, as `resolve_age_gyr(Some(_))` does today. A star that
  cannot live that long (an asked F0, say) is refused with the existing
  `BadAge`. *My call.*
- The Solar System ages are all equal today, so `ArchetypeAges` is only
  reachable with a user archetype file (`Archetype::load` takes a path).
  The test uses one. The intersection is kept anyway, because it costs
  only a few lines. *My call.*
- **Lone world** (`resolve_with_archetype`, `request.rs:537`): the age is
  `self.age_gyr`, else the archetype's, else the draw.
- **Drift and pinned requests**: `pinned()` already writes the resolved
  age, so a drifted world keeps it.

### Archetype star

Added after the coherence check. A whole system that named no star drew
one, mostly M dwarfs, whose early XUV stripped the asked Earth to 0.19 bar
and the asked Mars to 1e-5 bar. The user chose that an asked planet
archetype brings its `[orbit] star`, as moon archetypes already bring
their host's star (`host_star_of`).

- **No star named**: the system takes the `[orbit] star` of an asked
  archetype, if that star lives to the system age.
- **Several archetypes with a star**: a moon's host comes first (today's
  rule), then the archetypes that carry an age, then the others. Within
  each group, name order decides. *My call.*
- **A star named in the request**: it always wins.
- **The star dies before the system age, or no asked archetype names a
  star**: the star is drawn as today.

### Faithful archetypes

For each Solar System archetype, pin what the spread needs. The values are
the real body's, cited in the TOML as the existing ones are:

- mass;
- radiogenic (where heat drives outgassing);
- co2, n2, ch4, nh3 and s factors;
- water;
- insolation;
- age.

When the model needs a factor that is not a measured quantity (a
volatile factor), it is a calibration with the real body as its target,
and the TOML comment says so. That is the `titan-like` `ch4` precedent.

### Form (web)

- `ArchetypeInfo.inputs` carries `age_gyr` through ts-rs.
- **Lone world**: choosing an archetype that has an age fills the age
  field's default with it. The field says it comes from the archetype, for
  example "4,57 Ga — fixé par Martienne". If the user types another age,
  the field shows a warning that the archetype's body is 4.57 Gyr old and
  that its air may differ. The request is still accepted.
- **Whole system**: the preview's star line (`SystemPreview.svelte:29`)
  appends the source when an asked archetype fixed the age, for example
  "âge 4,6 Ga (fixé par Martienne)". *My call*: the web derives it from
  the asked archetypes' `inputs.age_gyr`, so no new API field is needed.
  When several asked archetypes fix it, the line names each one once.
  The whole-system form sends no age, so the explicit-age warning only
  exists for a lone world (the CLI can still pass `--age`). The new
  `ArchetypeAges` refusal shows like the other live-validation refusals.
- Strings are added in EN and FR.

### Formats

`Formation` stores `InputOverrides`, so its schema changes:

- system format 6 → 7;
- world `FORMAT_VERSION`, `MIN_FORMAT_VERSION` and
  `web/src/lib/formatVersion.ts` bumped too, because archetype worlds
  change. *My call.*

There is no migration. Older files are set aside as outdated, as at
every bump (generation-changes policy: no legacy format gates).

The numbers are agreed with the peer sessions before the bump, as in
lot 1.

## Acceptance

Per seed, seeds 0..20, through the archetype path and through a whole
system that asks for it:

| Archetype | Surface pressure | Other checks | Real value |
|---|---|---|---|
| earth-like | 0.7–1.3 bar | ocean and life (existing) | 1.013 bar |
| mars-like | 0.004–0.010 bar | barren, oceans < 0.01 | ~0.006 bar (seasonal 4–9 mbar) |
| venus-like | 60–120 bar (existing) | CO2 > 0.9 | 92 bar |
| titan-like | 1.2–1.8 bar | hydrocarbon world, a few % CH4 | 1.47 bar (Fulchignoni et al. 2005, doi:10.1038/nature04314) |

Also:

- A whole system that asks for any Solar System archetype gets
  `age_gyr = 4.57`. One that asks for two archetypes with disjoint ages is
  refused with `ArchetypeAges`. An explicit request age wins.
- The `#453` comment's case: a drifted system planet keeps its N2 within
  2 %.
- Archetype worlds change; that is the point. Drawn systems without
  archetypes stay byte-identical, apart from the format number. world-coherence-checker checks this on the census
  seeds.
- The web tests cover the age default, the warning and the star line.
- Documentation: README sections on archetypes and requests, the archetype
  schema doc header (`archetype.rs:11-57`), the CLI help, and the format
  history comments.

## Left to the builder

- Which inputs each archetype must pin to hold its range. Pin the fewest
  that work, with real values first.
- The exact `ArchetypeAges` payload and wording.
- Whether the moons' faithful ranges need their host giant pinned too.
