# Opacity table

`opacity.bin` holds the absorption of CO₂'s and water's spectral lines that
the climate column integrates (`crates/mapgen/src/planet/spectrum.rs`,
issue #523; read by `crates/mapgen/src/planet/ktable.rs`), and MT_CKD's CO₂ continuum on the same bins. The lines form a
correlated-k table: within each 20 cm⁻¹ bin from 0 to 5000 cm⁻¹ the
line-by-line absorption coefficients are sorted and reduced to 8
g-points, on a grid of pressure and temperature.
`opacity-halved.bin` is the same table on 10 cm⁻¹ bins over 0-2500 cm⁻¹ at
200-300 K, read only by the test `halving_bins_keeps_the_forcing`.
`shortwave.bin` is the table of the star's near infrared that the Bond
albedo absorbs (`crates/mapgen/src/planet/shortwave.rs`, task 7.6 of
#523): the same lines and sets on the albedo's own spectral bands from
0.7 to 5 µm, with MT_CKD's water and CO₂ continua (see
[Shortwave table](#shortwave-table)).

Both are built by `tools/opacity-table` (see its README) and never by the
generator.

## Regeneration

```bash
tools/opacity-table/fetch.sh
cargo run --release -j 4 -p opacity-table
cargo run --release -j 4 -p opacity-table -- \
  --bin-width 10 --top 2500 --temperatures 200,225,250,275,300 \
  --out crates/mapgen/data/opacity-halved.bin
cargo run --release -j 4 -p opacity-table -- --shortwave
```

The raw downloads (about 80 MB) stay in `tools/opacity-table/raw/`, which
is git-ignored.

## Sources

| Input | Version | Where | Downloaded |
|---|---|---|---|
| H₂O lines: H₂¹⁶O, H₂¹⁸O, H₂¹⁷O, HD¹⁶O, 0-5025 cm⁻¹, 99 006 lines (shortwave: 1975-14 389 cm⁻¹, 255 613 lines, downloaded 2026-11-19) | HITRAN2024, as served (updates to May 2026) | HITRAN's public line-by-line API, `https://hitran.org/lbl/api` | 2026-11-18 |
| CO₂ lines: ¹²C¹⁶O₂, ¹³C¹⁶O₂, ¹⁶O¹²C¹⁸O, ¹⁶O¹²C¹⁷O, 0-5025 cm⁻¹, 336 165 lines (shortwave: 1975-14 389 cm⁻¹, 313 716 lines, downloaded 2026-11-19) | HITRAN2024, as served | same | 2026-11-18 |
| Partition sums of the same isotopologues | TIPS, as HITRAN serves it | `https://hitran.org/data/Q/q<id>.txt` | 2026-11-18 |
| Water continuum (in `spectrum.rs` for the longwave; binned into `shortwave.bin`) | MT_CKD 4.3 | `absco-ref_wv-mt-ckd.nc`, `https://github.com/AER-RC/MT_CKD_H2O`, tag 4.3 | 2026-11-18 |
| CO₂ continuum (in the table) | MT_CKD 4.3: LBLRTM's `contnm.f90` at commit `f66bb596`, the submodule of `AER-RC/MT_CKD` tag 4.3 | `https://github.com/AER-RC/LBLRTM` | 2026-11-19 |

The design named HITRAN2020 (Gordon et al. 2022); HITRAN's server now
serves only its successor, HITRAN2024, so the table is built from it.

SHA-256 of the inputs of `opacity.bin`, concatenated in file-name order
(the pieces up to 5000 cm⁻¹):
`cat raw/lines/h2o_*` `55b9aaec0f3360debac33d5cd05a030d14d5c1626e0dac6347d4090132921b69`,
`cat raw/lines/co2_*` `cec95004862a0652506141e10915cddf4df065e152d6334c5691f96289360b0a`,
`absco-ref_wv-mt-ckd.nc` `69944eb8b045c268e2daeb2cddf536b99f9e9efe7e91c425067b46a5b287ddb3`,
`contnm.f90` `643f4173063d51759a0ea9e5715d0fcbd4066902452a2a5e56f62d37de69bcf7`.
With the pieces up to 14 500 cm⁻¹ that `shortwave.bin` reads (2026-11-19;
HITRAN answers 404 for the CO₂ pieces without lines, 13 000-13 500 and
14 250-14 500 cm⁻¹, which `fetch.sh` keeps empty), `cat raw/lines/h2o_*.par`
gives `a506a99b60f816e250c85d395cc6845a91e6fab80214cba230ee917dc965b9a3`
and `cat raw/lines/co2_*.par`
`03faa6af0b219c1d9e1b5a62202fde3825d108a1c5672defd1d3cfe4525215d9`.

## Citations

- I. E. Gordon et al., "The HITRAN2024 molecular spectroscopic database",
  *J. Quant. Spectrosc. Radiat. Transf.* 109807 (2026),
  doi:10.1016/j.jqsrt.2026.109807.
- I. E. Gordon et al., "The HITRAN2020 molecular spectroscopic database",
  *J. Quant. Spectrosc. Radiat. Transf.* 277, 107949 (2022),
  doi:10.1016/j.jqsrt.2021.107949.
- E. J. Mlawer et al., "Development and recent evaluation of the MT_CKD
  model of continuum absorption", *Phil. Trans. R. Soc. A* 370, 2520
  (2012), doi:10.1098/rsta.2011.0295; MT_CKD_H2O 4.3, AER (2024); MT_CKD
  4.3's CO₂ continuum (mt_ckd_2.5, built on the line coupling of
  Hartmann's group) in LBLRTM's `contnm.f90`.
- S. A. Clough, F. X. Kneizys and R. W. Davies, "Line shape and the water
  vapor continuum", *Atmos. Res.* 23, 229 (1989): the line shape and
  cutoff LBLRTM and MT_CKD share.
- J. Humlíček, "Optimized computation of the Voigt and complex probability
  functions", *J. Quant. Spectrosc. Radiat. Transf.* 27, 437 (1982).
- A. A. Lacis and V. Oinas, "A description of the correlated k
  distribution method", *J. Geophys. Res.* 96, 9027 (1991).

## How the coefficients are made

- **Lines.** Each line's intensity moves from 296 K to the grid's
  temperature with HITRAN's formula (partition sum, lower-state energy,
  stimulated emission). Its Lorentz half width is
  `(296/T)^n_air γ p`, with `γ` the air-broadened width for the two air
  sets and the self-broadened width (`γ_self`) for the two self sets
  (HITRAN's `.par` records carry only `n_air`, used for both). Its
  centre moves by `δ_air p`. Its Doppler width is the isotopologue's.
- **Shape.** Voigt (Humlíček's W4), cut at 25 cm⁻¹ from the centre, less
  the pedestal LBLRTM removes inside the cutoff, so that MT_CKD's continua
  add exactly what the lines leave out. For water, the profile's value at
  25 cm⁻¹ (the "plinth"). For CO₂, `(2 − d²/B²)` times the Lorentz value
  at the cutoff `B` = 25 cm⁻¹, at the distance `d` from the centre (the
  `fcnt_fn` of LBLRTM's fourth function, `oprop.f90`, which MT_CKD's CO₂
  continuum assumes since mt_ckd_2.0). The line coupling LBLRTM applies
  to CO₂'s lines is not in the `.par` records and is left out.
- **CO₂ continuum.** MT_CKD's CO₂ continuum, `C(ν)` every 2 cm⁻¹ in
  `contnm.f90` (`BLOCK DATA BFCO2`, times `XFACCO2` over 2000-2998
  cm⁻¹): the far wings of CO₂'s lines beyond the cutoff, with their
  sub-Lorentzian shape (the χ factor that Perrin & Hartmann 1989 measured
  is built into it). Each bin keeps the mean of `C` over its points, in
  cm² per molecule per cm⁻¹, and the band-head exponents of `FRNCO2`
  (2386-2434 cm⁻¹, `(T / 246 K)^n`) averaged with `C` as their weight.
  `spectrum.rs` takes the optical depth as `CONTNM` does:
  `C (p / 1013 mbar)(296 K / T) ν tanh(c₂ν / 2T)` per molecule.
- **Sampling.** Each bin is sampled uniformly at half the narrowest line
  half width in it (at most 2e6 samples per bin), the wings summed on
  coarser grids and interpolated (checked against the direct sum to
  0.5 %).
- **g-points.** The sorted coefficients of a bin are split at
  `g` = 0, 0.3, 0.6, 0.8, 0.9, 0.96, 0.99, 0.998, 1; each g-point is the
  mean coefficient of its interval, with the interval's width as its
  weight. On synthetic bands this reproduces the line-by-line mean
  transmission within 0.02 for paths from 1e-4 to 1e2 kg/m² (the builder's
  test).
- **Units.** m²/kg of the gas at natural isotopic abundance (molar masses
  18.015 and 44.009 g/mol).

## Grid

- Pressures: 15, every half decade from 1e-5 to 100 bar (total pressure).
- Temperatures: 16, 100 to 350 K every 25 K, then 400, 500, 600, 800 and
  900 K (450 and 700 K were dropped with the fourth set, to keep the file
  under 4 MB).
- Sets: CO₂ broadened by air; water broadened by air; water broadened by
  itself; CO₂ broadened by itself. The column mixes each gas's two sets by
  its share of the pressure, so that pure CO₂ (Venus, Mars, a thick CO₂
  air at the outer edge of the habitable zone) takes its self-broadened
  lines, some 1.3 times as wide as in air.
- `mapgen` interpolates `ln k` linearly in `ln p` and in `T`. Past the
  grid it clamps the temperature, and scales `k` as `p` above 100 bar and
  below 1e-5 bar.

## Layout

Little-endian:

| Field | Type |
|---|---|
| magic `MGOPACK2` | 8 bytes |
| bins, g-points, pressures, temperatures, sets | 5 × u32 |
| bin width, cm⁻¹ | f64 |
| g-interval edges | (g-points + 1) × f64 |
| `ln p` of the pressures, bar | pressures × f64 |
| temperatures, K | temperatures × f64 |
| `ln k` of code 1, and the step per code | 2 × f64 |
| codes | sets × pressures × temperatures × bins × g-points × u16 |
| CO₂ continuum: temperature of its band-head exponents, K (246) | f64 |
| CO₂ continuum per bin: coefficient (cm² per molecule per cm⁻¹) and exponent | bins × 2 × f64 |

A code `q` ≥ 1 stands for `ln k = −30 + (q − 1) × 48/65534` (k from
9.4e-14 to 6.6e7 m²/kg, a step of 0.07 %); code 0 is a coefficient under
9.4e-14 m²/kg, taken as nothing. `opacity.bin` is 3 844 380 bytes
(250 bins × 8 g-points × 15 pressures × 16 temperatures × 4 sets, then
the continuum's 250 bins); `opacity-halved.bin` 1 204 292 bytes.
`crates/mapgen/src/planet/ktable.rs` decodes and interpolates both this
layout and the shortwave one below.

## Shortwave table

`shortwave.bin` is built by `opacity-table --shortwave` from the same
line lists, partition sums and line shapes as `opacity.bin`:

- **Bins:** the bands of the Bond albedo's stellar spectrum
  (`planet::albedo::spectrum`, 240 bands even in `ln λ` from 0.2 to
  5 µm), from the one that holds 0.7 µm to 5 µm: 147 bins over
  2000-14 364 cm⁻¹, 27 to 193 cm⁻¹ wide. `mapgen` checks the table's
  edges against its bands when it loads it.
- **Grid:** the same 15 pressures and 4 sets; 13 temperatures, 150, 175,
  200, 225, 250, 275, 300, 325, 350, 400, 500, 650 and 900 K, so that the
  table stays under 2 MB.
- **Continua:** MT_CKD 4.3's CO₂ continuum (`contnm.f90`, to 10 000
  cm⁻¹) as in `opacity.bin`, and its water continuum, the mean over each
  bin of the file's `self_absco_ref` and `for_absco_ref` and the
  coefficient-weighted mean of `self_texp` (read from
  `absco-ref_wv-mt-ckd.nc`, NetCDF classic).
- **Not in it:** CO₂–CO₂ collision-induced absorption, which `mapgen`
  takes from its longwave binning of HITRAN's `CO2-CO2_2024.cia`; ozone
  and O₂.

Layout, little-endian, as `opacity.bin` but for these fields:

| Field | Type |
|---|---|
| magic `MGSWCK01` | 8 bytes |
| bins, g-points, pressures, temperatures, sets | 5 × u32 |
| bin edges, cm⁻¹, ascending | (bins + 1) × f64 |
| g-interval edges, `ln p`, temperatures, `ln k` of code 1 and step | as `opacity.bin` |
| codes | sets × pressures × temperatures × bins × g-points × u16 |
| CO₂ continuum: its `T_eff` and per bin coefficient and exponent | as `opacity.bin` |
| water continuum per bin: self coefficient at 296 K, self exponent, foreign coefficient at 296 K (cm² per molecule per cm⁻¹ at 1013 mbar) | bins × 3 × f64 |

`shortwave.bin` is 1 841 972 bytes (147 bins × 8 g-points × 15 pressures
× 13 temperatures × 4 sets, then the continua). The build takes about
four minutes on four threads.
