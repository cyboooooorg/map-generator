# Longwave band model (climate pass lot 3C-2, task 7)

Addendum to `2026-11-18-climate-pass-lot3c2-design.md`. Track A of #423,
issue #523 (CO₂ law), with Venus's greenhouse as a measured side effect.

## Intent

The climate column already integrates a radiative-convective profile
(`crates/mapgen/src/planet/column.rs`: dry and moist adiabats, CO₂
condensation, isothermal stratosphere at the skin temperature), but each
gas gives it **one grey optical depth** that is summed over the whole
thermal spectrum. Three refits of that grey law failed (task 7 rounds 1-3,
ledger of the lot): no grey CO₂ law holds Earth at 280 ppm, the outer edge of
Kopparapu, the moist plateau of water, Mars and Venus together, because a
grey CO₂ closes the whole spectrum while real CO₂ only absorbs in its bands,
and those bands widen with the logarithm of the CO₂ amount.

The user's decision (2026-11-18): build a real band model now, in this pull
request. The physical explanation must stay right and readable: the cause
tree keeps naming the same gases and notes.

## Goals

Replace the grey optical depths and the grey two-stream sum of the column by
a **simple spectral model**: a two-stream, non-scattering thermal emission
integrated over spectral bins, on the existing temperature profile.

- **CO₂**: the 15 µm band with exponential wings, its absorption falling off
  as `exp(−|ν − ν₀| / l)` from the band centre, pressure-broadened (Romps,
  Seeley & Edman 2022, J. Climate, "Why the forcing from carbon dioxide
  scales as the logarithm of its concentration"; Jeevanjee, Seeley, Paynter
  & Fueglistaler 2021, J. Climate, "An analytical model for spatially varying
  clear-sky CO₂ forcing"). The opaque part of the band then widens as
  `ln(CO₂)`, which gives the logarithmic forcing.
- **CO₂–CO₂ collision-induced absorption**: from the HITRAN CIA file
  (Karman et al. 2019, already read in round 2), placed in its own spectral
  regions, scaling with the CO₂ density squared.
- **H₂O**: rotation band and vibration-rotation band with exponential
  decay from their centres, plus the self and foreign continuum in the
  window (Jeevanjee & Fueglistaler 2020, J. Atmos. Sci., "Simple spectral
  models for atmospheric radiative cooling"; Koll & Cronin 2018, PNAS, for
  the Earth OLR checks).
- **Windows**: whatever no gas closes stays open, so the moist plateau and
  the 4 µm window of steam atmospheres emerge from the spectrum instead of
  from caps.
- **CH₄ and Titan**: CH₄ keeps its current treatment (one grey depth,
  calibrated on Titan, standing in for Titan's N₂–N₂, CH₄–N₂ and H₂–N₂
  collision-induced absorption), applied across the far infrared where
  those CIA bands lie. A spectral Titan is out of scope (follow-up issue).
- **Gravity**: column masses come from `p / g` through `Planet` (task 2),
  so the absorber paths are right on every body.

Grey constants that the spectrum replaces are removed: `CO2_OPACITY`,
`CO2_EXPONENT`, `CO2_WEAK_BAR`, `H2O_OPACITY`, the broadening knees, the grey
CIA (`CO2_PAIR_*`), and `STEAM_MAX_BAR` if the 4 µm window makes it
redundant. `MOIST_PLATEAU_W_M2` stays a computed value, now from the
spectrum.

## Checks (all in tests or a census, before and after in the report)

Independent physics checks of the radiation alone:

- CO₂ doubling forcing on an Earth column: 3.7 ± 0.6 W/m² (Myhre et al.
  1998), and logarithmic over 70-2000 ppm (forcing per doubling constant
  within 15 %).
- Earth clear-sky OLR at 288 K with 77 % humidity: 265 ± 10 W/m²
  (Kiehl & Trenberth 1997; Trenberth, Fasullo & Kiehl 2009).
- Moist plateau (runaway limit): 282 ± 14 W/m² (Goldblatt et al. 2013),
  independent of CO₂ up to 1e-2 bar and of N₂ from 1 to 3 times Earth's
  within 3 %.
- Runaway anchor flat within 2 % over 380-640 K.

Climate outcomes (the bounds of the lot):

- Earth: thermostat CO₂ in [1.87e-4, 4.2e-4] bar, surface 286-290 K.
- Outer edge round the Sun 1.60-1.80 AU (Kopparapu 1.70); K (4500 K) and
  M (3300 K) within ±15 % of Kopparapu's S_eff.
- Late Archean: 10 mbar CO₂ in 1 bar N₂ near 281 K, doubled N₂ warms by
  a few kelvin.
- Mars 210 ± 5 K; Titan 93.7 ± 1 K; Triton unchanged.
- Venus measured against 737 K; any gap left (H₂SO₄ cloud infrared
  opacity, SO₂, far wings, hot bands) goes to a follow-up issue, and the
  venus-like test bound is set to what the model reaches with a comment.
- Snowball counts reported for 500 systems; `SNOWBALL_WARM_BAR` re-derived
  on its stated calibration with the physical cause of any change.
- Every archetype its real body over 40 seeds (census).

Performance: a whole-system draw costs at most 20 % more than before
(lot 1 budget), measured by `generation-profiler` on the same seeds. If the
spectral column is too slow, tabulate or cache OLR, or coarsen the bins;
the checks above must still hold.

## Non-goals

- Shortwave absorption by CO₂ and H₂O in the stellar near infrared (the
  Kopparapu M-dwarf effect): left out, as today.
- Scattering in the thermal infrared, clouds' infrared effect, H₂SO₄ and
  SO₂ opacity: follow-up with Venus.
- A spectral Titan (N₂, CH₄, H₂ CIA in bins): follow-up issue.
- New stored gases (H₂, He, Ar): no format change for that.

## Anti-goals

- A spectral model tuned constant by constant until it reproduces the old
  grey results: the coefficients come from the cited papers and HITRAN.
- Any new constant fitted on Venus or Mars.
- A cause tree that disagrees with the card (explain must replay the same
  column).

## Design calls (main session)

- Two-stream non-scattering emission with diffusivity 1.66 (Elsasser),
  per bin, on the column's existing profile and levels.
- Bins: a uniform grid in wavenumber fine enough that the CO₂ band edges
  and the H₂O window are resolved (the builder picks the width, at most
  20 cm⁻¹ over 0-3000 cm⁻¹, and checks the doubling forcing changes by
  under 0.1 W/m² when halving it).
- Coefficients are kept in one module (`planet/spectrum.rs` or similar)
  with their sources in doc comments; the column calls it.
- The round-2 uncommitted grey-law work is saved as a diff in the lot's
  workspace and dropped from the worktree; the band model starts from
  commit 3c47979. The HITRAN CIA derivation of round 2 is reused.

## Left to the builder

- Exact Romps/Jeevanjee parameter values (read from the papers), the
  continuum formulation, the bin width, and whether to integrate analytic
  band averages per bin.
- How to keep CH₄'s grey stand-in in the spectral sum (which bins).
- Caching or tabulation if the performance budget needs it.

## Settled after the builder check (main session's calls)

- **What may move.** Band parameters (band-centre strength, wing decay
  length `l`, continuum coefficients) take the papers' values. They may move
  only inside the uncertainty the papers state, and each choice is written
  down. A check missed outside that range is reported with the numbers, and
  the work stops for the user. It is never fixed by a fitted constant.
- **Pressure broadening.** Lorentz scaling of the line absorption,
  `κ ∝ p / p_ref`, applied per level on the column, with the temperature
  dependence the papers give (Jeevanjee & Fueglistaler 2020 form). The same
  law holds at Mars and Venus pressures. CIA scales with the density squared
  and is not broadened.
- **Interfaces.** Internal signatures of `column.rs` and `climate.rs` may
  change. `surface_temperature_k` and `wet_temperature_k` keep their roles,
  because explain replays them. The cause tree needs no per-gas flux split:
  its gas children carry pressures, as today. The builder greps for every
  user of constants that change (`MOIST_PLATEAU_W_M2`, `SNOWBALL_WARM_BAR`,
  …) and updates them.
- **Water at high temperature.** Self continuum ∝ e² and foreign continuum
  ∝ e·(p − e), with the temperature dependence of the cited form, extended
  to steam atmospheres. If the 4 µm window does not make `STEAM_MAX_BAR`
  redundant, it stays, with its doc comment updated.
- **Conflicting bounds.** Priority order:
  1. the radiation checks (doubling forcing, Earth OLR, plateau, anchor);
  2. Earth;
  3. the outer edge and K/M;
  4. Mars and Titan;
  5. Venus, which is measured only.
  A bound that cannot hold stops the work, and the user decides. No partial
  result is committed silently.
- **CH₄ stand-in.** It covers 0-650 cm⁻¹, the far-infrared region of Titan's
  N₂–N₂ and CH₄–N₂ CIA (Courtin, Gautier & McKay 1995). The band model
  applies to every body, with or without CO₂ and H₂O.
- **Spectral range and emission.** 0-5000 cm⁻¹, so that hot columns
  (640 K) are covered. Each bin uses the layer-mean Planck function with the
  diffusivity factor, as the column already does.
- **Formats.** No new numbers: the lot already takes system 11 and world 29
  in its final task.
