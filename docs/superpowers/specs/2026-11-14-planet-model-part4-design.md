# Physical planet model, part 4: large moons

Step 4 of #423 (design: `2026-11-11-physical-planet-model-design.md`,
section "Moons"), roadmap #427.

## Intent

Today a moon is `rocky-moon` or `icy-moon` and nothing more: no interior,
no atmosphere, no climate, no class. A cold moon of 3000 km renders as a
bare icy rock. The user, on 2026-11-14, opening drawn systems:

- around giants they see many small rocky moons and few large ones like
  Titan;
- they never saw a hydrocarbon world in random generation;
- opening a Titan in the viewer, "a bare surface with no orange haze and
  no methane lakes" would make them wince.

Part 4 makes large moons real bodies of the planet model, makes the moon
draw match what is observed, and lets the user ask for a given moon
(a Titan) in the form and get it for sure.

## Measured before the change

`crates/mapgen/examples/moon_census.rs` (on this branch), 500 whole
systems, seeds 1 to 500:

- 67 gas giants, 172 ice giants, 293 mini-Neptunes, 55 hot Neptunes.
- Moons over 2300 km (Titan, Ganymede): 21, all around gas giants, about
  one system in 25. Ice giants and mini-Neptunes top out near 1700 km.
- Mini-Neptunes carry most small moons: 251 of 293 have moons, 457 rocky
  moons under 600 km around them alone.
- No moon has a model; 3 hydrocarbon-world planets in 500 systems.

Causes in `moons.rs` (`MoonSystem::of_giant`): the regular moons share a
fixed 10⁻⁴ of the giant's mass (Jupiter's and Saturn's are 2.1 and
2.5 × 10⁻⁴), split with weights 0.3 to 1.7, so one dominant moon (Titan
holds 96 % of Saturn's) never happens; rock inside 8 to 11 giant radii,
ice beyond or under 60 K.

## Goals

1. **Realistic random draw.** The moons of a giant follow the observed
   satellite systems and the circumplanetary disk models: total mass and
   its split per Canup & Ward (2006, doi:10.1038/nature04860), including
   systems with one dominant moon; the rock/ice boundary from the disk's
   temperature (the giant's distance to the star), not a fixed radius
   band. Frequencies are not inflated for spectacle: a Titan or a
   Ganymede shows up in a minority of systems.
2. **Large moons through the model.** A moon round under its own gravity
   gets inputs (mass, iron, ice and water, volatiles, radiogenic heat),
   then interior, core, regime, atmosphere, climate, surface liquid, life
   and class, exactly as a planet, with its planet's tides as a heat
   source (`planet::heat::tidal_power_w` with the host's mass, the moon's
   orbit and eccentricity; forced eccentricity of a resonant chain, as
   Io's) and its star's light at the planet's orbit. A cold Titan-mass
   moon with nitrogen and methane comes out as `hydrocarbon-world`; an Io
   as volcanic (heat-pipe); a Europa with a subsurface ocean.
3. **Moon archetypes**, asked for by name: `titan-like`, `europa-like`,
   `ganymede-like`, `callisto-like`, `io-like`, `triton-like`,
   `enceladus-like` (*my call* for the list), as `archetypes/*.toml` like
   the planets' (inputs as ranges, French locale), each setting causes
   (mass, ice fraction, volatiles, host and orbit, cold), never the
   outcome.
4. **The form** (`web/src/components/GenerateForm.svelte`, step
   `planets`): the archetype counts gain a "moons" group. The request
   (`WholeSystemRequest.planets`, or a sibling map, builder's call) and
   the CLI accept the same names.
5. **Small moons** (not round: shepherds, debris, irregular captured,
   small regular) keep their shape and stay without a model, but get a
   drawn bulk composition (shares of rock, ice and carbonaceous matter,
   from where they formed or were captured from), shown on their card.
6. **Cards**: a large moon's card shows what a planet's shows (class,
   atmosphere, temperature, subsurface ocean, regime).

## How a moon asked for finds its giant

Decided with the user:

- A moon asked for in a whole system is part of a whole system; the rest
  is drawn as usual.
- A giant asked for in the same request that can carry the moon keeps it
  ("saturn-like × 1, titan-like × 1" gives one Saturn with its Titan).
  Otherwise a giant is made for the moon: a giant whose mass and orbit
  (cold enough, Hill sphere wide enough) let the archetype's moon exist.
- The asked moon is guaranteed: the host's other regular moons are drawn
  around it (the asked moon takes its share of the disk mass, the others
  share the rest).
- *My call:* "can carry" means the giant's moon draw (goal 1) could have
  made that moon: its disk mass holds the moon's mass, its orbit puts the
  moon on the right side of the disk's rock/ice line (goal 1's
  temperature rule, so a Titan's host is cold enough for ice), and the
  moon's orbit lies within the Hill sphere's stable part. A `neptune-like`
  cannot carry a Titan (1.3 × 10⁻³ of Neptune's mass, over its disk's
  budget), so "neptune-like + titan-like" adds a gas giant. A captured
  archetype (`triton-like`) needs a giant with a captured-moon path (ice
  giants today).
- *My call:* a giant made for a moon counts toward `MAX_PLANETS`
  (`system.rs:66`, 15); a request that cannot fit fails like a request
  for too many planets.

## Existing draw paths the model builds on

Answered from the code, for the builder:

- **Captured large moons**: `MoonSystem::of_giant` already captures a
  retrograde Triton (35 % of ice giants, 1.5 to 3 × 10⁻⁴ of the giant's
  mass, 12 to 16 radii, inclination 157°). It is round, so it goes
  through the model; its composition comes from where it was captured
  (*my call*: a Kuiper-belt body's, rock and ice about 2:1, N₂ and CH₄
  ices), not from the giant's disk. No new capture path in random draws.
- **Resonant chains**: the regular moons already sit on a chain of
  period ratio 2 (`1.587` in semi-major axis). *My call:* no resonance
  dynamics; a moon in the chain with an inner and an outer neighbour
  gets a drawn forced eccentricity within a cited range (Io 0.004,
  Europa 0.009, Ganymede 0.0013), which feeds its tidal heat. The free
  eccentricity drawn today (0 to 0.01) is replaced by it.

## Acceptance

- **Titan in the viewer** (the user's wince test): a `titan-like` moon
  asked for renders with Titan's orange haze (`planet::interim` sky,
  `METHANE_HAZE`) and visible methane lakes (`HYDROCARBON_LAND_FRACTION`,
  about 1 % of the surface) clustered toward the poles. Checked with
  `viewer-visual-check`.
- **Polar lakes on every hydrocarbon world** (decided with the user,
  2026-11-14): a climate rule, not a Titan exception. Titan's lakes and
  seas cover about 1 % of it and about 97 % of that area lies north of
  55° N, while the low latitudes are dry dune fields (Hayes 2016,
  doi:10.1146/annurev-earth-060115-012247; Lorenz et al. 2006,
  doi:10.1126/science.1123257): its circulation carries methane poleward,
  so evaporation exceeds rain at low latitudes and liquid gathers near
  the poles (Mitchell 2008, doi:10.1029/2007JE003017; Schneider et al.
  2012, doi:10.1038/nature10666). On a world whose surface liquid is
  hydrocarbons, the interim surface puts its liquid poleward of a
  latitude of about 55 to 60° and keeps the low latitudes dry: a labelled
  calibration with Titan as its target, for planets and moons alike.
  Left for later: the north–south asymmetry from the orbit's
  eccentricity (Aharonson et al. 2009, doi:10.1038/ngeo698), to attach
  to #322; equatorial dunes, to #424.
- Each moon archetype, asked for, comes out as itself: Titan
  hydrocarbon-world with about 1.5 bar of N₂ and a few % CH₄; Europa,
  Ganymede and Enceladus with a subsurface ocean under an ice shell; Io
  heat-pipe and airless (or SO₂-thin); Callisto airless, no ocean or a
  thin one; Triton with a thin N₂ air.
- The census (`moon_census.rs`, kept as an example like
  `planet_census.rs`) before and after: moon radius distribution per host
  kind compared with the Solar System's giants and the disk models; the
  class mix of large moons; the share of systems with a Titan-like or
  Ganymede-like moon reported, not tuned to a target. *My call* for what
  the tests assert: the regular moons' total mass ratio stays within the
  cited range (the Solar System's giants span 1 to 2.5 × 10⁻⁴), and both
  shapes occur over many draws, several comparable moons (Jupiter) and
  one dominant moon (Saturn), at the rates the cited source gives
  (e.g. Sasaki, Stewart & Ida 2010, doi:10.1088/0004-637X/714/2/1052,
  ties the shape to whether the giant opened a gap in the disk). The
  mini-Neptunes' moon count is measured and reported; it gates nothing.
- Solar System archetypes keep their outcome (planets unchanged by the
  moon work, except where they host moons).

## Non-goals

- Ring and shepherd moons' draw (`rings.rs`) and giant-impact moons of
  rocky planets (`moons.rs`): unchanged draw; only their composition (goal
  5) and, when round, the model (goal 2).
- The rest of the form (fine knobs per input, the card explaining why):
  step 5 of #423.
- Ice giant decks (#298), magnetic fields shown (#345).
- Mini-Neptune radius (#446). Whether mini-Neptunes should carry so many
  moons is measured and reported; a change there, if any, is the
  builder's call grounded in a source, otherwise an issue.

## Anti-goals

- A Titan-like that comes out bare: no haze, no lakes.
- An archetype that forces the outcome (sets "hydrocarbon-world" or a
  haze) instead of the causes.
- Inflating large-moon frequencies in random draws to make them visible.
- A legacy path for systems stored before the change (no format gates;
  older systems are listed as outdated, `generation-changes-policy`).

## Constraints

- Each step leaves `main` working and tested; `cargo test`, the web tests
  and CI green.
- Moons get bodies through `system::moon_parts` / `drawn_moons`
  (`system.rs:1695`); the body type becomes the model's class for a
  large moon (`Body.body_type`), `rocky-moon`/`icy-moon` stay for small
  ones (*my call*). `moons::is_moon_world` and every reader of the class
  must follow.
- The model is stored as data (`WorldParams.model`), not recomputed on
  read. Format bumps: `SYSTEM_FORMAT_VERSION` 4 → 5 (`system.rs:64`) and
  world `FORMAT_VERSION` 19 → 20 (`export/raw_data.rs:66`), once each,
  numbers confirmed with the peer sessions (mapgen1 to 3) before merging
  since part 3 is done and they are active again; warn them before
  rebuilding `develop`.
- *My call:* one branch `feat/planet-model-part4` and one PR, as parts
  3a to 3c, so the bumps land once; commits inside it may be ordered
  draw, model, archetypes and form, small-moon composition, cards.
- The generator is the source of the classes; the web app follows its
  types (`web/src/lib/api/types/*.ts`, generated) and adds labels to its
  i18n tables (`web/src/lib/i18n/en.ts`, `fr.ts`, `names.svelte.ts`),
  which name `rocky-moon` and `icy-moon` today. Titan's haze and lakes
  come from the model's data through `planet::interim`, not from the
  class name.
- Every relation and constant cites its source or is labelled a
  calibration with the bodies it reproduces.
- The Earth-sized moon of a giant (`exomoon.rs`, `HostMode::Giant`) and a
  lone moon asked around a giant go through the model too.

## Left to the builder

- The round threshold for entering the model (today `ROUND_ROCK_KM` 600,
  `ROUND_ICE_KM` 400; Mimas, 198 km, is round): check it, since
  `enceladus-like` (252 km) must go through the model.
- Which planet-model relations need a moon variant (the cosmic shoreline
  and XUV at the planet's orbit, escape from a moon inside a giant's
  magnetosphere, the moon's formation context instead of the star's ice
  line for its ice fraction and volatiles).
- How the request carries moon archetypes (inside `planets` or a
  sibling map) and how the host is chosen when several asked giants
  could carry the moon.
- The small bodies' composition classes and their sources.
- Code layout.
