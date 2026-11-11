# Physical planet model: design

Date: 2026-11-11. Status: approved by the user. Tracked in
#423, first track of the roadmap (#427).

## Intent

Today a planet's type is drawn, and everything else follows from the type.
`drawn_types` (`crates/mapgen/src/system.rs:509`) picks a preset name from
the planet's distance to the star plus a coin flip: between half the
habitable zone's inner edge and that edge, a small planet is `caustic` or
`barren` with even odds. The preset then fixes the pressure, the climate
shift, the greenhouse, the tectonic regime and the biome remap
(`Preset`, `crates/mapgen/src/preset.rs:166`). Two neighbours on the same
orbit differ by chance, not by what they are made of or what happened to
them. Drawn caustic worlds have equilibrium temperatures of 250 to 375 K,
overlapping the temperate ones (176 to 273 K, 400 systems drawn on
2026-11-11).

The user wants a planet's nature to follow from what the planet is: its
mass, what it is made of, what it kept of its water and gases, the heat it
gets from its star and from inside, its age. The type becomes a name put
on the result, the way astronomers classify real planets. A Venus is a
runaway greenhouse because it got there, not because a draw said so.

This track also feeds the others: the magnetic fields of #345 read the
core and dynamo it derives, the real Solar System (#418) and the catalogue
systems (#419) fill its inputs from measured values, and the surface
layers (#424) read its surface liquid, atmosphere and life.

## Goals

- **Physical inputs, drawn or set.** Every planet and every large moon has
  a small set of inputs: mass, bulk composition, volatile inventory,
  internal heat sources, age (the system's), life. The system draw fixes
  them; the creation form can set any of them.
- **Derived state.** From the inputs and the star, the generator derives,
  in order: radius and interior structure, core state and magnetic field,
  tectonic regime and outgassing, atmosphere (composition and pressure,
  after escape), surface temperature and surface liquid, habitability and
  life. Each derivation follows a published relation, cited in the code
  and in a `docs/science/` page, and says when it is a calibration.
- **Derived type.** The type shown on cards and labels is a classification
  of the derived state: rocky, super-Earth, mini-Neptune, ocean world,
  hydrocarbon world, runaway greenhouse, lava world, ice world, gas giant,
  ice giant, hot Jupiter… The classification never feeds back into the
  physics.
- **Interior layers kept in full.** The interior is stored as layers
  (core, mantle, ice shells, liquid water layers, envelope) with their
  radii and state, including **subsurface oceans** (Europa, Enceladus,
  Ganymede, Titan), even though nothing draws them yet: a cross-section
  view will (see Later).
- **Moons in the same model.** A large moon (round under its own gravity)
  goes through the same model as a planet, with tidal heating from its
  planet as an internal heat source: Io comes out volcanic, Europa and
  Enceladus with an ocean under the ice, Titan with a thick nitrogen
  atmosphere and methane lakes.
- **Presets as archetypes.** A preset ("like Earth", "like Venus", "like
  Titan", "mini-Neptune"…) fills the inputs with known values; the user
  can then change any of them, and the result follows the physics: an
  Earth moved away from its star freezes.
- **The form keeps as many knobs as possible.** The star's type, the
  system's age, and every input of every planet stay settable; left
  empty, a field is drawn from the seed as today.
- **Life is drawn.** A world where liquid water lasted can have life or
  not; a temperate world without life keeps its oceans and bare rock.

## Non-goals

- **Changing the surface rendering.** Köppen climate classes, continuous
  surface fields, derived biome labels and the removal of the hexagonal
  grid are #424. This track keeps the current biome rules (see
  Interim surface below).
- **Changing the simulation grid or resolutions.** That is #425.
- **Drawing the interior.** The cross-section view is for later.
- **Fantasy planets** that bypass the physics, for role-playing: later,
  see below. The design must not close that door.
- **Migrating existing worlds.** The user decided (2026-11-11) that
  existing worlds and systems may be deleted: no migration, no legacy
  algorithm, no format gate kept for old files.

## Anti-goals

What would make this a failure even if it works:

- **Types in disguise.** A model whose inputs are in practice chosen per
  type (a "caustic" branch drawing a caustic composition) is the same
  arbitrary list with more steps. Inputs are drawn from the formation
  context (star, metallicity, distance to the ice line), never from a
  target type.
- **A black box.** A planet whose card says "runaway greenhouse" must be
  able to say why: the derived quantities that led there are stored and
  shown.
- **Unsourced physics passed off as science.** Where no published relation
  exists (life, parts of the tectonic regime), the rule is a labelled
  calibration, documented as such, as `docs/analysis/*` already does.
- **A form that only experts can use.** All inputs are reachable, but a
  user who picks a preset and a star and presses Generate gets a sensible
  world without touching anything else.

## Decisions taken with the user

All on 2026-11-11, in the design discussion that opened #421 to #427.

1. The type is derived from physical inputs, not drawn (the caustic coin
   flip prompted it).
2. Existing worlds may be deleted; no migration.
3. A habitable world can be lifeless; life is drawn.
4. The interior model includes the core (active or not, composition),
   because #345 needs it, and the subsurface oceans, because a later
   cross-section view will show them.
5. Life colours the surface: vegetation colours stay (and keep following
   the star, `docs/analysis/10-stars.md` step 6).
6. Presets stay, as known starting points; the star's type is chosen; as
   many parameters as possible stay settable.
7. Large moons go through the model; shepherd moons and the other small
   moons keep their current draw.
8. Later, not in this track: fantasy planets for role-playing, a
   cross-section view, a hexagonal grid for gaming (redone after #424).

## The model

The relations below are the intended science, from memory of the
literature; the builder checks each source before relying on it and
records it in the code and the science page.

### Inputs

Per planet or large moon:

| Input | Meaning | Drawn from |
|---|---|---|
| Mass | Earth masses | a mass function by orbit and star (replaces `draw_mass` per type) |
| Iron fraction | core share of the rocky part | the star's metallicity and refractory ratios (Fe/Mg), with scatter |
| Ice and water fraction | share of the mass in water and ices | where the body formed relative to the ice line, with migration scatter |
| Envelope fraction | hydrogen and helium share, at formation | the core mass (core accretion) and the formation time |
| Volatile inventory | CO₂, N₂, CH₄, NH₃, S per unit mass | formation place and composition |
| Internal heat | radiogenic (age), tidal (orbit, host) | derived, with radiogenic abundance as an input |
| Life | whether life arose | drawn where habitable long enough |

Per system: the **star** (type, as today, with metallicity), and the
**age**. Today every star is 4.6 Gyr old (`AGE_GYR`, `star.rs:407`); the
age becomes a system input, drawn within the star's lifetime, and shared
with the star's activity (`activity.rs`) and the crater chronology.

### Derived, in order

1. **Radius and interior.** From mass and composition: mass–radius
   relations for iron, rock and water mixtures (Zeng et al. 2016, 2019;
   Seager et al. 2007), giving layer radii: iron core, silicate mantle,
   high-pressure ice, liquid water layers, ice shell, envelope. Replaces
   today's single `rocky_gravity` relation (`world.rs:158`, M ∝ R^3.7) and
   the `icy-moon` name test (`world.rs:203`).
2. **Core and magnetic field.** Whether the core is still liquid and
   convecting at the system's age, from a parametrised thermal evolution
   (Driscoll & Barnes 2015 style), and the dipole strength from an
   energy-flux scaling (Christensen 2009). Stored for #345.
3. **Tectonic regime.** Plates, stagnant lid, episodic, heat-pipe (the
   existing `Regime`, `geology.rs:76`), from mass, internal heat, water
   and age instead of the preset's `[geology]` table and `regime_of`
   (`geology.rs:60`). The literature disagrees on the mass dependence;
   the rule is a labelled calibration that reproduces Earth (plates),
   Venus (episodic or stagnant), Mars and Mercury (stagnant), Io
   (heat-pipe).
4. **Atmosphere.** Outgassed volatiles plus the kept primordial envelope,
   minus escape: the cosmic shoreline (Zahnle & Catling 2017) for
   whether a body keeps an atmosphere at all, from escape velocity and the
   star's cumulative XUV (which `activity.rs` already models); the
   radius valley (photoevaporation, Owen & Wu 2017) for whether a small
   planet keeps its hydrogen envelope, which separates super-Earths from
   mini-Neptunes. Without a magnetic field, the stellar wind strips
   more (a labelled calibration; it ties to #344). The result is a
   composition and a surface pressure, replacing the preset's
   `Defaults.pressure`.
5. **Climate and surface liquid.** The surface temperature from the
   star, the greenhouse of that composition and pressure, and internal
   heat; the carbonate–silicate regulation where there are plates and
   liquid water (today's `carbon_cycle`, `generation.rs:405`, kept);
   the runaway and moist greenhouse limits (Kopparapu et al. 2013, as
   the habitable zone already uses). The liquid on the surface, if any,
   from the phase diagram at that temperature and pressure: water,
   methane and ethane, molten rock, or none. Replaces the preset's
   `ClimateShift` and `Atmosphere.greenhouse`.
6. **Life.** Possible only where liquid water stayed on the surface for
   long enough (a labelled threshold, order of a billion years); then
   drawn with a probability set in one constant, documented as a game
   choice since no measured rate exists. Life adds oxygen to the
   atmosphere above an age threshold and colours the surface.
7. **Type.** A classification of the result, in a single function, with
   thresholds from the literature where they exist (radius valley near
   1.5–2 Earth radii, giant masses as `docs/analysis/12-gas-giants.md` §2
   lists them).

### Giants

Giants keep their own model (`giant_world.rs`, `giants.rs`): bands,
storms, rings, winds. What changes is how they come in: a body is a giant
because its envelope fraction says so, not because its type was drawn
`gas-giant`. Mini-Neptunes are new: a rocky or icy core under a hydrogen
envelope, drawn like a small ice giant (cloud decks, bands). Ice giant
decks (#298) should be done alongside, since mini-Neptunes need the same
methane and water decks.

### Moons

The draw of moons does not change: giant-impact moons, captured moons,
debris (`moons.rs`), ring moons and shepherds (`rings.rs`), the
Earth-sized moon of a giant (`exomoon.rs`). What changes is a large
moon's nature: instead of `rocky-moon` or `icy-moon` from its composition
(`system.rs:1446`), a moon round under its own gravity goes through the
model above, with its planet's tides as a heat source (`HostGiant`'s
tidal heat, and #304's tidal heating of the plume moon). Small moons
(shepherds, debris, irregular captured moons) keep a composition and a
shape, without interior or atmosphere.

## Presets and the form

- A preset becomes an **archetype**: a named set of inputs (mass,
  composition, volatiles, life, and a preferred insolation for placing it
  in a system), with its locales. It no longer carries `ClimateShift`,
  `Atmosphere`, `[geology]` or a biome `remap`. Built-in archetypes cover
  at least the Solar System's planets and large moons (Mercury, Venus,
  Earth, Mars, Io, Europa, Ganymede, Titan…) and the exoplanet kinds
  (super-Earth, mini-Neptune, ocean world, lava world, hot Jupiter…). The
  real Solar System of #418 will reuse them.
- **Archetypes are starting points.** Choosing one fills the form; any
  input can then change. A request names an archetype and overrides.
- **System requests by archetype.** `--system` and the form's planet
  counts name archetypes instead of types (`venus-like=1,earth-like=1`);
  each is placed at its preferred insolation, then the physics runs. A
  drawn system (`auto`) uses no archetype at all: inputs come from the
  formation context.
- **The star** is chosen by type as today, plus the **system age**.
- **Every input is a field** of the form, grouped (orbit, composition,
  interior heat, volatiles, life) and folded by default.
- **Derived values are not inputs.** Pressure, liquid, magnetic field,
  regime and type are shown, not set. Forcing them against the physics is
  the fantasy-planet feature (Later).
- **Today's direct knobs are set through their causes** (decided with the
  user, 2026-11-11). The request and form fields `sea_level`,
  `land_fraction`, `volcanic`, `surface_age_ga`, `erosion`,
  `pressure_bar` and `circumference_km` (`WorldRequest`,
  `request.rs:132`; `FIELDS` in `GenerateForm.svelte`) go away:
  - the **sea level** follows from the water inventory (an input, given
    in Earth oceans) and the relief's hypsometry;
  - the **volcanic intensity** and the **surface age** follow from the
    tectonic regime and the internal heat;
  - the **erosion** follows from the atmosphere and the surface liquid;
  - the **pressure** from the atmosphere (above);
  - the **circumference** from mass and composition.

  The form shows each of them as a preview of the result. Setting them
  directly, at the cost of consistency, belongs to the fantasy-planet
  feature (Later).
- *Builder's call:* a preview of the derived state before the maps are
  generated, from a cheap resolve call (the system draw already writes a
  `system.json` without maps, `--draw-only`).

## One global state, one grid

The model above is zero-dimensional: one surface temperature, one
pressure, one liquid per body. It is computed when the system is drawn,
before any map exists, and it decides the type. The grid climate
(`climate.rs`, an Earth-fitted profile plus an offset, `generation.rs`
around :1350) then spreads it over the map. *My call:* the global state is
authoritative; it feeds the grid its greenhouse offset and internal heat,
and the grid is tested to agree with it (area-mean temperature within a
stated tolerance on the archetypes), so a world typed "ocean world" does
not freeze over on its map.

## Interim surface and sky, until #424

#424 replaces biome ids, preset remaps and the invented biomes. Until it
lands, worlds still need the current biome rules. *My call:* the derived
state chooses one of the existing biome rule sets and remaps (the closest
existing look: a runaway greenhouse renders with today's barren or
volcanic rules, an ocean world with terran rules at full sea level, a
hydrocarbon world with frozen rules and its lakes), held in one function
that #424 deletes. No new biome is added for this track.

- **Life is visible already.** *My call:* a lifeless world remaps the
  vegetation biomes (forest, jungle, taiga, savanna, plain, shrubland,
  wetland) onto existing bare ones (desert, dust plain, rocky waste,
  tundra by temperature), through the same remap mechanism.
- **The sky follows the atmosphere.** *My call:* the renderer's inputs
  stay as they are (`Atmosphere`: Rayleigh colour, haze, cloud kind and
  cover, `atmosphere.rs:79`), but they are computed from the derived
  composition and pressure instead of read from a preset: Rayleigh from
  the column of gas, cloud kind from what condenses (water, sulfuric acid
  where SO₂ meets water, methane, dust on thin dry air). No viewer change
  is needed for this.

## Storage and formats

- `WorldParams` (`world.rs:15`) stops embedding the whole `Preset`
  (today `planet: Preset`, stored in `world.json`) and carries the inputs
  and the derived state instead; the archetype's name is kept for display
  only.
- The system file stores, per body, inputs and derived state, each
  derived quantity with what it came from, so the card can explain it.
- `SYSTEM_FORMAT_VERSION` (`system.rs:58`) and `FORMAT_VERSION`
  (`export/raw_data.rs:52`) are bumped; `MIN_FORMAT_VERSION` follows, and
  older systems are no longer read. Agreeing on the numbers with the
  other sessions is the main session's job (CLAUDE.md).
- *My call:* the server lists older systems as outdated and offers to
  delete them; it never deletes the user's stored data by itself.

## Where the type name is used today

From a code survey (2026-11-11), all of it changes:

- `system.rs`: `PLANET_TYPES` (:73), `band` (:404), `fits` (:671),
  `draw_mass` (:455), `is_giant` (:476, about 20 call sites),
  `drawn_types` (:509), `planet_params` (:1256, sets the preset name as
  the world's planet), `preview_of` (:1786, colours per type),
  `moon_parts` (:1444).
- `request.rs:389` (draws one of the five built-in names),
  `resolve_with_preset` (:398–714).
- `world.rs:203`, `moons.rs:1204`, `exomoon.rs:280`, exports copying
  `planet.name` into `planet_type` (`hex.rs:224`,
  `export/raw_data.rs:588`, `export/png.rs:110`).
- Server: `/api/presets`, `/api/form` (`FormSpec.planet_types`),
  `create_system`, the index's `body_type` column (`index.rs:127`,
  :505–530), `thumbs.rs:179`.
- Web: `menuCards.ts:99` (`HABITABLE_TYPES`), `menuCards.ts:183` and
  `orrery.ts:35` (giant sets), `systemStar.ts:133–167`, the
  `bodyTypes` labels (`i18n/en.ts:718`, `fr.ts:680`),
  `GenerateForm.svelte` (presets from the API, planet counts keyed by
  type), `format.ts:237–248` (reads the preset's clouds and regime).

## Order of work

*Builder's call within these constraints:* each step leaves `main`
working and tested.

1. Inputs, interior and radius; the system age; the type classifier,
   alongside the existing presets (no stored format change).
2. Atmosphere, escape, climate and surface liquid; life.
3. Drawn systems from the formation context; archetypes replace presets;
   the interim surface mapping. `WorldParams` changes here, so the format
   bumps land in this step: from then on, older systems are listed as
   outdated (*my call*: the user accepted losing them, and a
   compatibility path would be a legacy format gate).
4. Large moons through the model.
5. Form and API; card showing the derived state and why.

**Lone worlds** (a world drawn around a star without a whole system,
`{"around": WorldRequest}`, `alone`) go through the model too. *My call:*
their formation context is the star, metallicity and orbit that
`draw_orbit` already draws; with no archetype named, the inputs come from
that context, never from a drawn type name (today `request.rs:389` draws
one of five built-in names).

**Ice giant decks (#298)** are not a prerequisite. *My call:*
mini-Neptunes first ship with today's ice-giant decks, and #298 improves
both after.

## Left to the builder

- The exact relations and their constants, within the sources above,
  each checked and cited; where a relation is a calibration, the target
  bodies it reproduces.
- Measuring the result on drawn systems: the mix of derived types should
  look like the observed exoplanet census where one exists (Kepler
  occurrence rates for radii and periods), and the Solar System
  archetypes must come out as themselves.
- Code layout (new modules versus extending `geology.rs`, `atmosphere.rs`,
  `star.rs`).
- Which of the issues absorbed by #423 (#312, #313, #315) are done inside
  it and which stay for after; the notes on #311, #324 and #345.

## Later

- **Fantasy planets** for role-playing: set derived values directly,
  against the physics. The derived state being stored as data (not
  recomputed on read) keeps this open.
- **A cross-section view** of a body's interior layers.
- **A hexagonal grid for gaming**, redone after #424.
