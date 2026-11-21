# Planet model PR 2: split into four pull requests

Decided with the user on 2026-11-21. Slice 1, #602. Follows PR 1 (#617:
interior, bulk and heat).

## Intent

PR 2 of the planet model makes `planet::class_of` and `planet::model`
public and fills the rest of `PlanetState`: atmosphere, escape, the
spectral column (#593), global climate, surface liquids, the class,
envelopes and giants. It also closes the trace gaps listed in #617 and
widens the composition draws (#618).

Ported from `legacy-final`, that is about 15 000 lines (climate 4 400,
atmosphere 1 800, column, spectrum, shortwave and albedo 5 000, class
1 200) and 7 MB of opacity tables: too large for one reviewable pull
request.

## Split with mapgen3 (system draw)

- **XUV belongs to mapgen3.** It ports the XUV activity law of
  `legacy-final:crates/mapgen/src/planet/xuv.rs` into `science` and
  opens a small `timeline` pull request that gives `StellarSource` the
  present XUV flux at the planet (W/m², same eccentricity factor as the
  bolometric flux) and the cumulative XUV fluence at the planet (J/m²,
  up to the source's age). The stellar wind is left for later. The
  planet model only reads those two fields.
- **The seam.** `planet::class_of(&Trial, &Trace) ->
  Result<TrialOutcome, SystemError>` with
  `TrialOutcome { class: Traced<PlanetClass>, bulk: Bulk }`. The draw
  needs the bulk (mean density, radius, mass) for the Roche floor and
  the hot-giant edge. One code path: the chain computes both. The draw
  takes the classifier as a parameter and stubs it in its tests; the
  second pull request to merge wires the real one.
- mapgen3 does not touch `crates/system/src/planet/**`; mapgen1 does
  not touch the draw modules, `placement.rs` or the placement half of
  `archetype.rs`.

## Approaches

1. One pull request: rejected, too large to review.
2. Split by legacy module: rejected, the modules depend on each other
   in a chain (escape before the surface, climate before the class).
3. **Split along the chain's dependencies (chosen):** each pull request
   leaves `main` buildable and tested, and only one waits on mapgen3.

## The four pull requests

| PR | Content | Waits on |
| --- | --- | --- |
| 2a | The seven trace gaps of #617; #618 (wider draws); the envelope fraction draw; giant structure and radius | Nothing |
| 2b | Radiative transfer in `science`: correlated-k spectral column, shortwave, albedo; `opacity.bin` and `shortwave.bin` with their generator and provenance; the #593 levers | Nothing (parallel to 2a) |
| 2c | Envelope escape (energy-limited); cosmic shoreline and non-thermal loss; outgassing and the pressure tree (SO2 wash-out); minimal tectonic regime and core, which outgassing reads | mapgen3's XUV pull request |
| 2d | Global climate (condensation, maximum greenhouse, carbonate thermostat); surface liquids; the class; public `class_of` and `model`, wired into the draw | 2b and 2c |

The model stays crate-private in `system` until 2d. PR 3 is unchanged:
full regime, core and dynamo (Driscoll & Barnes 2015), terrain, life,
archetype targets, cross-field validation.

## Left to each pull request's own design

- 2a: the flattened shape of each draw, whether the observed
  distribution stays behind a setting for the galactic-map mode, how
  the science page states the departure (#618).
- 2a: the sources of the envelope fraction draw and of the giant
  radius (legacy values are checked, not copied).
- 2b to 2d: each law is ported with its source and tests; missing
  science stops the work and gets a study first.
