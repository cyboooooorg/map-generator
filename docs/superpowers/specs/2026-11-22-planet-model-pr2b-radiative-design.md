# Planet model PR 2b: radiative transfer in `science`

Decided with the user on 2026-11-22. Slice 1, #602. Follows the split
[planet model PR 2 split](2026-11-21-planet-model-pr2-split-design.md)
(2a merged as #628). Reference page:
[radiative transfer](../../science/radiative-transfer.md). Full audit:
[Wiki study 19](https://github.com/cyboooooorg/map-generator/wiki/19-radiative-transfer-audit).

## Intent

The global climate (2d) needs the outgoing infrared of a planet's air
and the share of its star's light it reflects. In `legacy-final` these
come from a HITRAN correlated-k column and a two-stream shortwave,
which together make 98 % of the generation time. 2b ports them into
`science` as laws on typed values, with their tables, their generator,
their provenance and their tests, so that 2d only solves the balance.

The audit (Wiki study 19) checked every number and citation of the
legacy code. The port fixes what the audit found wrong (decisions below)
instead of carrying it over.

## Goals

- The longwave column: outgoing radiation of a column of given partial
  pressures over a surface temperature, a skin temperature and a
  gravity, for the profiles the climate uses (climate, saturated,
  plateau, shortwave).
- The shortwave: the Bond albedo and the share of the light the air
  absorbs, for a given ground albedo, cloud cover, sulphuric deck and
  star effective temperature.
- The atmosphere's part of the albedo: cloud cover, sulphuric deck,
  methane haze, Rayleigh scattering.
- The saturation laws the column needs: water (IAPWS), CO₂, methane.
- `opacity.bin` and `shortwave.bin` in `crates/science/data/`, with
  `OPACITY.md` and a generator that rebuilds everything it can.
- The in-column performance levers of the legacy code: the column memo,
  single precision with `fast_math`, the bin cut-off, the opaque stop,
  the thin g-point merge, the shortwave paths memo.

## Non-goals

- The climate solver: `wet_balance`, `settle`, the thermostat, the
  maximum greenhouse, condensation, collapse, the thaw search. They come
  in 2d.
- The #593 levers (the two secant steps of #600, one maximum-greenhouse
  search per body). They live in the climate solver, so they move to 2d.
  The split document is amended to say so.
- The surface cover: ice line, frost, icy age, ocean fraction, ice
  sheets, the albedos of the surface kinds. They move to 2d with their
  own study (option B, chosen by the user).
- Any change to `timeline`, `system` or the draw. Nothing outside
  `science` calls the new laws before 2d, so `ENGINE_VERSION` is
  unchanged.

## Anti-goals

- A number without `Source:`, `Calibration:` or `Definition:`. Legacy
  comments that say "from memory, not checked" are not carried over:
  each takes the citation the audit checked, or becomes a calibration
  with what was searched.
- Physics rewritten from memory. Every law is ported from
  `legacy-final` with its tests, then changed only where a decision
  below says so.
- Hidden state in `science`: no `thread_local!` memo. The caches are
  values the caller owns.
- Regenerating `opacity.bin` from a new HITRAN fetch: HITRAN serves only
  its current data, so the legacy bytes are copied and their hashes
  recorded.

## Approaches

1. **One PR for 2b**: rejected by the user, larger than 2a to review.
2. **Two PRs, longwave then shortwave (chosen)**: the shortwave reads
   the column's levels, the column does not read the shortwave. Each PR
   leaves `main` tested, and the recalibration of the albedo, the part
   hardest to review, gets its own PR. 2d waits for both.
3. **All of `albedo.rs` in 2b** (surface cover included): rejected by
   the user. The cover depends on the climate's state, and it would
   double the study.

## Decisions taken with the user

1. **Water saturation** follows IAPWS: the saturation pressure of Wagner
   & Pruß 1993 (IAPWS SR1-86(1992)) up to the critical point, and the
   sublimation pressure of ice Ih of Wagner et al. 2011 (IAPWS
   R14-08(2011)) below the triple point. The legacy constant latent heat
   overshoots by 58 % at 600 K.
2. **The Bond albedo takes the ground albedo** as one input. Earth's,
   for the tests, is the CERES EBAF Ed4 clear-sky surface albedo, 0.122
   (Kato et al. 2018, Table 5). The cloud albedo is refitted on Earth's
   Bond albedo of 0.293 (Stephens et al. 2015), not 0.30.
3. **Rayleigh** cross-sections of each gas come from its refractive
   index and King factor (He et al. 2021 and the per-gas sources it
   cites), not from the square of a static polarizability.
4. **Values and citations** as the audit corrects them: NH₃ heat
   capacity 35.65 J/(mol·K), clear-sky air absorption 72-73 W/m² (the
   observed value; the test takes the window of "Rulings during 2b-2"),
   the DOIs and sections listed on the reference page.
5. **CO₂ and methane saturation** follow the ancillary vapour-pressure,
   sublimation and melting equations of their reference equations of
   state: Span & Wagner 1996 for CO₂, Setzmann & Wagner 1991 for
   methane (decided 2026-11-22 after the legacy constant-latent-heat
   methane law missed NIST by +5.9 % at 150 K). CO₂'s latent heat in
   the moist adiabat comes from the curve's slope, as water's does.

## PR 2b-1: the longwave column

Branch `feat/science-radiative-transfer` (this one).

### 2b-1 content

- `science::water`: the IAPWS saturation and sublimation pressures
  (decision 1).
- The CO₂ and methane saturation laws and CO₂'s latent heat, ported from
  `legacy-final:crates/mapgen/src/planet/atmosphere.rs` (l.849-960),
  with the audit's corrections (CO₂ curve sourced by its fit against
  NIST/Span-Wagner, not Kopparapu; methane's sublimation enthalpy: the
  audit found no NIST entry at 9.1 kJ/mol, so the port takes a NIST
  value or records a calibration).
- A `science::radiation` module, from `ktable.rs`, `fast_math.rs`,
  `spectrum.rs` (with `spectrum/reference.rs` as a test-only module),
  `column.rs`, and from `climate.rs` the items the column calls: the
  methane grey stand-in (`methane_depth`, `CH4_OPACITY`,
  `CH4_SATURATION_BAR`), `STEAM_MAX_BAR`, the water critical point (from
  `science::water`), `stratospheric_methane_bar`.
- A type of partial pressures for the seven gases the column knows (N₂,
  CO₂, H₂O, CH₄, NH₃, SO₂, O₂), owned by `science` (it cannot depend on
  `timeline`, which has no atmosphere yet; 2c adds the timeline's and
  converts).
- An explicit column cache the caller owns and passes in, with the
  legacy behaviour (1 024 entries, keyed on the exact bits of the
  inputs, oldest replaced first). Same results with or without it.
- Data, in `crates/science/data/`: `opacity.bin` and the test-only
  `opacity-halved.bin` copied byte for byte from `legacy-final`
  (SHA-256 recorded), the water continuum nodes (`MT_CKD`) and the CO₂
  CIA table as TSV files loaded like the crate's other tables,
  `OPACITY.md` moved and updated.
- `tools/opacity-table`: it also writes the `MT_CKD` and CIA TSV files
  (reproducing the legacy tables: the audit gives the binning rule and
  the expected agreement, 109 of 139 CIA rows to 0.2 %); `fetch.sh`
  downloads the CIA file and checks the SHA-256 it records; the README
  and `OPACITY.md` lose their `crates/mapgen` paths and gain the output
  hashes, the tool's commit and the HITRAN release.

### 2b-1 tests

- Every legacy test of the ported modules, with its source in the
  test's doc. The plateau anchors (Goldblatt et al. 2013, 282 W/m²;
  Boukrouche et al. 2021; Hamano et al. 2015) run with the IAPWS
  saturation. If a plateau test leaves its tolerance, the work stops
  and the result goes to the user: it is a physics ruling, not a
  tolerance to widen.
- Earth's clear-sky OLR takes a source (CERES EBAF; the audit found
  264 to 268 W/m²).
- The CO₂ doubling test: the audit found Myhre's 3.7 W/m² is not like
  for like. The builder looks for a published instantaneous clear-sky
  top-of-atmosphere forcing for a doubling. If none is found, the
  test's doc says it is a plausibility band, and the PR description
  lists it under its calibrations.
- New: the IAPWS curves against the IAPWS tables' published values.
- The generator reproduces the TSV tables from the files `fetch.sh`
  downloads (a test of the tool, run where the inputs are present).
- Tier 1, within its budget; a column test too slow for it goes to
  tier 3 with its reason.

## PR 2b-2: the shortwave and the atmosphere's albedo

Its own `feat/` branch from `main` once 2b-1 is merged.

### 2b-2 content

- From `shortwave.rs`: the two-stream layers (Meador & Weaver 1980,
  hemispheric mean), the stacking, the paths and the budget.
- From `albedo.rs`, lines 272-460 and 560-760: the cloud cover, the
  cloud albedo, the deck, the haze, the Rayleigh depth (decision 3), the
  star's spectrum bands, the budget over a given ground albedo, with an
  explicit paths cache like the column's.
- `shortwave.bin` in `crates/science/data/`, copied byte for byte.

### 2b-2 calibration

Superseded on 2026-11-23 for the clear-sky window: see "Rulings during
2b-2 (2026-11-23)".

- The clear sky has no free parameter: with Earth's ground albedo
  0.122, the clear-sky top-of-atmosphere albedo must fall in 0.149 to
  0.156 (Stephens et al. 2015; Kato et al. 2018). Outside it, the work
  stops and the result goes to the user.
- The cloud albedo is then fitted so that Earth's Bond albedo is 0.293.
  The Earth inputs the fit needs (surface 288 K, its relative humidity,
  its cloud cover 0.67 to 0.68) take sources; the audit flagged the 77 %
  humidity as unsourced.
- The deck, the haze and the cloud's water stay calibrations, with what
  was searched (reference page).

### 2b-2 tests

The legacy shortwave and albedo tests that stay in scope, with the
audit's values (Kim & Ramanathan 2008 72 ± 3 W/m²; Earth's Bond albedo
0.293; Kopparapu et al. 2013 §4.1 for the M-star test), and a Rayleigh
test against the refractive-index cross-sections.

## Documentation

- The reference page's "In the app" follows each PR.
- `docs/science/planet-atmospheres.md` loses the paragraphs the
  reference page now holds, with a link instead (the list is in the
  page's commit report).
- `crates/science/README.md`: the new modules and data files.
- Each PR description has its calibrations under their own heading.

## Settled after the builder check

Calls made by the main session, not by the user; open to correction.

- **Tables in the binary.** `opacity.bin`, `shortwave.bin` and the
  test-only `opacity-halved.bin` are embedded with `include_bytes!`, as
  in `legacy-final` (the domain crates cannot read files; macros are not
  denied). They are committed as plain files, like the legacy ones. The
  parser takes a byte slice, so that the halved table, read only by the
  test `halving_bins_keeps_the_forcing`, goes through the same parser.
- **Workspace with the cache.** The legacy column also keeps a
  `thread_local!` scratch of about 120 kB. It becomes part of the
  caller-owned column cache, which the column borrows mutably: no
  allocation per call, no hidden state.
- **Acceptance.** The legacy tests pass at their own tolerances, the
  single-precision path against the double-precision reference
  included. The PR description reports how far the IAPWS saturation
  moves each plateau and runaway value. Speed: a column costs no more
  than 10 % above `legacy-final` on the same inputs, measured with
  `generation-profiler`'s method on the tests' columns.
- **Stops.** A test that a decision above says stops the work (a
  plateau outside its tolerance, a clear sky outside 0.149 to 0.156, a
  constant with no source and no defensible calibration) stops the
  task: the main session brings the result to the user before the
  branch moves on. No test is ignored to get past it.
- **IAPWS in `water.rs`.** The saturation and sublimation laws join
  `science::water`. One implementation, on plain `f64`; the typed
  function wraps it, and the column calls the plain one in its loop.
  That is one code path.
- **Every ported constant** follows the `Source:` / `Calibration:` /
  `Definition:` rule, the gas heat capacities, `STEAM_MAX_BAR`,
  `CH4_OPACITY` and `CH4_SATURATION_BAR` included; the audit gives the
  citation or the calibration record of each. SO₂ is one of the seven
  gases of the column (its heat capacity and Rayleigh depth); its frost
  albedo is surface cover and stays out.

## Rulings during 2b-2 (2026-11-23)

The user's rulings after the diagnostic study of the port:

- **The clear-sky window** is 0.1567 to 0.1707, not 0.149 to 0.156 (which
  stay as the observed values): the observed albedo less the aerosols,
  plus what the faithful port lacks. It supersedes "2b-2 calibration".
- **The absorption test** takes 48 to 54 W/m² on 2.5 cm of water (the
  NVAP mean of Kim & Ramanathan's global run; their 3.0 cm is the
  sensitivity case of Table 2), plus the slope of the response to water.
  It supersedes decision 4's 72-73 W/m², which stays as the observed
  value (it holds the ozone, O₂ and aerosols).
- **A faithful port**: a black-body Sun, no ozone, no O₂, no aerosols.
  What it lacks is tracked in #653 (ozone, O₂) and #655 (the Sun's
  spectrum, the two-stream closure).
- **`CH4_NEAR_IR_HALF_BAR`** is refitted on McKay et al.'s 0.306 with a
  sourced Titan.
- **Rayleigh** has H₂O in the sum, and the King factors of NH₃ and SO₂
  from Chong 2018.
- **The haze** (`HAZE_ALBEDO`, `HAZE_ABSORPTION`) is not refitted: the
  legacy values stay, and the refit is tracked in #656.

## Left to the builder

- The module tree inside `science::radiation` and the names of its
  public items.
- Where the CO₂ and methane saturation laws live (a `volatiles` module
  or inside `radiation`).
- Whether a law takes typed units at its boundary only, keeping `f32`
  and plain numbers inside the hot loops (expected: yes).
