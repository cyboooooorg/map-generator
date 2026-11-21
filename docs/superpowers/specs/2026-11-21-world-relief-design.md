# World relief (W2): design

Part of the world and catalogue track
([world and catalogue design](2026-11-21-world-catalogue-design.md), W2),
slice 1 (#602) of the architecture rewrite (roadmap #427). Owner: mapgen2.

## Intent

W1 laid out the grid, the field container, the projection and the
catalogue. W2 gives a solid-surface planet its relief on the cube-sphere
grid, the sea level that the timeline's ocean share implies, and the
first CLI command that exports a world. The relief of `legacy-final` is
the starting point, but about sixty of its numbers carry no source. The
user asked for literature studies first, so that as few as possible stay
calibrations (decided 2026-11-21).

## Goals

- A relief field (`relief.height`, metres) for every planet whose
  keyframe has a `Surface`, built from the timeline's physical forcings:
  radius, surface gravity, tectonic regime, heat, surface liquid.
- The sea level as the area-weighted quantile of that relief which
  leaves the timeline's `ocean_share` under the liquid.
- Every number the relief uses is sourced, a definition, or one of a
  short list of declared calibrations.
- A CLI command that generates (or reads from the cache) a world and
  exports its fields and a 2:1 PNG projection, with `--cache-budget`.

## Non-goals

- Impact craters: their own pull request after W2 (the study of
  `legacy-final:crates/mapgen/src/craters.rs`, 30 constants, and a
  rewrite of its row-window drawing as a neighbourhood pass).
- Erosion: it reads the temperature by row in legacy, so it waits for
  W3's spatialized climate; no page covers it yet.
- Rivers, deltas, hydrology, geysers, drift (`drift_ma`), past and
  future keyframes, small-body flattening, ice shells.
- Matching legacy worlds pixel for pixel: the oracle compares
  distributions.

## Anti-goals

- `world` reads the central object's kind or feeds anything back.
- A second water budget: the sea level is a quantile of the relief at
  `ocean_share`, never a volume of water spread over it.
- A number without `Source:`, `Calibration:` or `Definition:`.
- A hypsometric curve imposed on the relief (rank mapping onto Earth's
  or Mars's curve): it would erase what the crust and the plates
  produce. Published curves are test targets only.

## Constraints

- `world` depends on `timeline` and `science` only; laws and constants
  live in `science`, the passes in `world`.
- Draws come from `Seed::derive` on the body's seed, one sub-seed per
  pass (`relief.continents`, `relief.plates`, `relief.volcanoes`).
- No row walking: every pass is a function of a cell's 3D point, or a
  neighbourhood pass over `Grid::neighbours`.
- Area-weighted statistics use `Grid::area`.
- Tests: no world generated outside tier 3 (CLAUDE.md, Tests). The
  shared reference systems carry no `Surface` until mapgen1's planet
  PR 2d, so whole-world relief tests run in tier 3 on hand-built
  planet states for now, and move to tier 2 on the reference systems
  once 2d lands (follow-up issue).

## The study

Three literature studies ran on 2026-11-21 (plates and boundary relief,
volcanoes, statistical shape of topography). Their findings, in short:

- **Spectrum.** The legacy continent noise has persistence 0.5 and
  lacunarity 2, a Hurst exponent H = 1: too smooth, the cause of the
  rounded coasts of wiki study 04. Measured H above 10 km: Earth 0.479,
  Mars 0.53, Moon 0.226, Mercury 0.248 (Landais, Schmidt & Lovejoy 2018,
  arXiv:1805.11249, Table 2), consistent with Balmino 1993 and
  Aharonson et al. 2001 (doi:10.1029/2000JE001403).
- **Coast.** The level set of a surface of Hurst exponent H has a
  fractal dimension 2 − H; the coast warps of legacy
  (`COAST_WARP_DECAY` 0.8) give no dimension above 1 and become
  unnecessary once the spectrum is right.
- **Plates.** Count and sizes: Bird 2003 (14 large plates),
  Morra et al. 2013 (8 large, 31 in all), Sornette & Pisarenko 2003
  (Pareto exponent 0.25). Speeds: Zahirovic et al. 2015 (median RMS
  2.6 cm/yr without subducting edge, 7.9 cm/yr above 25 % subducting).
  Trenches: relief 0.9-4.7 km, width 31-93 km (Zhang et al. 2018).
  Arcs: above the slab at 72-173 km depth, mean 105 km (Syracuse &
  Abers 2006), arc-trench gap = depth / tan(dip) (England & Katz 2010).
  Rifts: under about 100 km wide (Hopper & Buck 1996; Buck 2007).
  Orogens: Tibet 5 km over 3 million km² (Royden et al. 2008),
  Altiplano 350-400 km wide, mean 3.65 km (Allmendinger et al. 1997).
- **Volcanoes.** Height/width 0.01-0.1 (Grosse & Kervyn 2018); arc
  volcano spacing random, Gamma-distributed, 50-70 km in Alaska
  (de Bremond d'Ars et al. 1995). No published law links heat flux to
  volcanic output: the anchors are Earth's 0.087 W/m² (Pollack et al.
  1993) and a heat pipe's 2.5 W/m² (Moore & Webb 2013).
- **Hypsometry.** ETOPO 2022 (Earth), MOLA (Mars, bimodal, Smith et al.
  1999; Aharonson et al. 2001), Venus unimodal with 80 % within 1 km of
  the mean (Ford & Pettengill 1992).

Several figures came from secondary sources (publisher PDFs refused).
The first task of the plan verifies each value retained below on its
source and writes it into `docs/science/` with its citation; a value
that cannot be verified goes back to the calibration list. The three
studies become one wiki study, which the pages cite. Once verified, the
final list of calibrations goes to the user before any relief code is
written *(my call: a checkpoint, so the count is never a surprise in
review)*.

## Approaches

Decided with the user on 2026-11-21:

1. **Faithful port.** Port the legacy sampler to the cube, source what
   agrees, declare about 25 calibrations. Fast, easy oracle; keeps the
   H = 1 noise and its rounded coasts. Rejected.
2. **Relief rebuilt on measured statistics.** Noise at the measured H,
   rank-mapped onto a published hypsometric curve. Fewest
   calibrations, but imposes Earth's or Mars's curve on any planet and
   erases the physics (crust, plates). Rejected.
3. **Hybrid** (chosen). The structure is ported from legacy (Airy
   floating crust, continental share, plates, volcanoes) with the
   sourced geometry above; the noise is rebuilt on the measured
   spectrum; published hypsometry and spectra are test targets. About
   ten declared calibrations remain.

## Design

### Inputs read from the keyframe

| Input | From `PlanetState` |
| --- | --- |
| Radius, gravity | `bulk.radius`, `bulk.surface_gravity` |
| Regime | `surface.regime` |
| Heat flux | sum of `heat.radiogenic`, `heat.tidal`, `heat.secular` over `4πR²` |
| Ocean share, liquid kind | `surface.liquid` (`None`: a dry world) |

A planet without `Surface` has no relief (an error the CLI reports).

### Passes (in order, each per cell)

1. **Continents.** Fractal Brownian motion over the cell's 3D point:
   lacunarity 2 (Definition), persistence `2^-H`, H from the regime's
   analogue body (Plates: Earth; Stagnant, Episodic, HeatPipe: Mars;
   Venus has no measured H; the regime-to-body table is a declared
   calibration), octaves from the planet's circumference down to two
   grid cells (Definition, the grid's Nyquist limit). One power law:
   the break Landais et al. measure near 10 km lies below the
   shortest wavelength of a 15 km grid (30 km). The continental share
   (Cogley 1984 for plates, Marinova et al. 2008 for the Mars highland
   share of the other regimes) sets the crust line as an area-weighted
   quantile of the noise. *(My call, as in legacy: a constant per
   regime. `surface.terrain` carries ages and renewal, not crust; a
   share driven by the planet's history is a follow-up issue.)*
2. **Floating crust.** Airy isostasy of the continental and oceanic
   crusts, ported with `EARTH_CRUST`, `MARS_CRUST`, `ABYSS_KM`
   (sources already in `mountains-and-gravity.md`), chosen by regime as
   in legacy, not scaled with the planet's mass *(my call; a departure
   the page states)*. The shelf and
   platform shaping (`PLATFORM_*`, `SHELF_*`) is not ported unless the
   Earth hypsometry test needs it; if it does, as calibrations.
3. **Plates** (Plates regime only). The legacy plate map ported onto
   `world::Grid` (same cube layout): plate count and sizes, Euler poles
   and speeds from the sources above; boundary relief rebuilt from the
   sourced geometry: trench depth and width, arc at the slab-depth
   distance, rift width, orogen width and height. Each profile is a
   function of the distance to the boundary, computed by a
   neighbourhood distance pass on the grid. Amplitudes without a source
   (island-arc and Andean uplifts, rift shoulders and axis, the
   convergence thresholds `RELIEF_SPEED` and `ARC_SPEED`) are declared
   calibrations.
4. **Mountains.** Ridged noise on the same spectrum, weighted by
   orogeny (plates) or highlands (other regimes: the legacy
   `HIGHLANDS` noise and, on a world without liquid, the dry plains
   `DRY_PLAINS_KM`, both calibrations already documented, ported as
   they are), scaled by the relief scale `min(1/g, 2)` (Weisskopf
   1975, Plescia 2004).
   `MOUNTAIN_ONSET` and `MOUNTAIN_BLEND` stay calibrations against
   ETOPO 2022 unless the spectrum makes them unnecessary.
5. **Volcanoes.** Volcanic intensity from the heat flux (declared
   calibration anchored on Earth and a heat pipe; the flux drives
   nothing else in W2); cone height/width in
   the published range; isolated cone spacing (calibration); arc cones
   along the arcs with Gamma-distributed spacing (sourced).
6. **Sea level.** The height `s` such that the area of cells below `s`
   is `ocean_share` of the sphere (exact area-weighted quantile).
   Heights are stored relative to it; on a dry world, relative to the
   area-weighted mean (Definition, the Mars and Venus convention).
   Every liquid kind (water, hydrocarbons, nitrogen, lava) is treated
   alike *(my call)*. An `ocean_share` of 0 takes the dry convention;
   of 1, the sea level sits above the highest cell.

The legacy `tanh` range compression and the `[-1, 1]` unit of 25 km are
dropped: heights are metres. Storage: `U16` over ±25 km (Definition;
the tallest known relief, Olympus Mons, stands about 22 km, Plescia
2004), a step under 1 m; values beyond saturate (the tier-3 cases
assert that none does).

### Calibrations left (each with its doc comment, page section and PR line)

Regime-to-analogue-body table; volcanic intensity curve; volcanic
province share; isolated cone spacing and presence; island-arc and
Andean uplifts; rift shoulder and axis amplitudes; `RELIEF_SPEED` and
`ARC_SPEED`; `PLATE_WEIGHT` (fitted to PB2002 areas); `STAGE_MA`;
`MOUNTAIN_ONSET` and `MOUNTAIN_BLEND` if kept.

### CLI

A second pull request with its own plan, once mapgen3's `system` command is on `main`
(it shares its options). Proposed skeleton, to agree with mapgen3
before writing it:

- `clap` (derive) as the argument parser.
- Global options: `--root <DIR>` (the store) and
  `--cache-budget <SIZE>` (bytes, with an optional `KiB`, `MiB`, `GiB`
  suffix; default 2 GiB, from the parent design).
- The system options of `map-generator system` (`--seed`, `--present`,
  `--host-mass`, `--metallicity`, `--archetype`) in one shared
  `SystemArgs` struct.
- `map-generator world <SystemArgs> --body <designation>
  [--cell-km 15] [--field relief.height] [--width 2048] --out <DIR>`:
  stores the inputs, takes the timeline and the world from the cache or
  generates them, evicts over the budget, writes the world file and the
  field's PNG projection.

### Tests

- Tier 1: the fBm persistence and octave count against their
  definitions; the crust line quantile and the sea-level quantile on
  hand-built fields; Airy isostasy (ported tests); boundary profiles
  against the sourced widths and depths; arc distance against
  depth/tan(dip); the volcanic intensity at its anchors; `--cache-budget`
  parsing.
- Tier 3 (hand-built Earth-, Mars- and Venus-like states, 60 km grid):
  measured H of the relief within the published values; land
  hypsometry against ETOPO 2022 (land 29 %, shares above 2 and 4 km);
  Mars highland share; Venus 80 % within 1 km; sea level leaving
  `ocean_share` under water; same seed, same world.
- Carried from #381 (with the CLI PR): two export resolutions agree on
  area-weighted statistics.

## Left to the builder

- The noise implementation (a crate such as `noise`, as in legacy, or
  a port), provided it is seeded from `Seed::derive` only.
- Module names, error enums, the PNG colour ramp of the relief.
- The neighbourhood distance algorithm for plate boundaries.
- Where the hand-built planet states of the tier-3 tests live (the
  structural `earth_like_fixture` of `crates/timeline/src/keyframe_tests.rs`
  is a starting point), without making `world` depend on more than
  `timeline` and `science`.

## Documentation

- `docs/science/`: `mountains-and-gravity.md` (spectrum, crust,
  mountains, hypsometry targets), `plate-tectonics.md` (boundary
  geometry), a volcano section; each "In the app" and calibration
  section updated.
- A wiki study gathering the three studies, cited by the pages.
- `crates/world/README.md` (relief field, sea level convention), the
  CLI README and usage text with the CLI PR.

## Follow-up issues

- Craters on the cube grid (track world and catalogue, after W2).
- Erosion as a neighbourhood pass (after W3).
- A continental share driven by the planet's history rather than a
  constant per regime.
- Move the tier-3 relief tests to tier 2 when the reference systems
  carry a `Surface` (after mapgen1's PR 2d).
