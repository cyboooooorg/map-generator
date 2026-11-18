# Opacity table

`opacity.bin` holds the absorption of CO₂'s and water's spectral lines that
the climate column integrates (`crates/mapgen/src/planet/spectrum.rs`,
issue #523). It is a correlated-k table: within each 20 cm⁻¹ bin from 0 to
5000 cm⁻¹ the line-by-line absorption coefficients are sorted and reduced
to 8 g-points, on a grid of pressure and temperature.
`opacity-halved.bin` is the same table on 10 cm⁻¹ bins over 0-2500 cm⁻¹ at
200-300 K, read only by the test `halving_bins_keeps_the_forcing`.

Both are built by `tools/opacity-table` (see its README) and never by the
generator.

## Regeneration

```bash
tools/opacity-table/fetch.sh
cargo run --release -j 4 -p opacity-table
cargo run --release -j 4 -p opacity-table -- \
  --bin-width 10 --top 2500 --temperatures 200,225,250,275,300 \
  --out crates/mapgen/data/opacity-halved.bin
```

The raw downloads (about 80 MB) stay in `tools/opacity-table/raw/`, which
is git-ignored.

## Sources

| Input | Version | Where | Downloaded |
|---|---|---|---|
| H₂O lines: H₂¹⁶O, H₂¹⁸O, H₂¹⁷O, HD¹⁶O, 0-5025 cm⁻¹, 99 006 lines | HITRAN2024, as served (updates to May 2026) | HITRAN's public line-by-line API, `https://hitran.org/lbl/api` | 2026-11-18 |
| CO₂ lines: ¹²C¹⁶O₂, ¹³C¹⁶O₂, ¹⁶O¹²C¹⁸O, ¹⁶O¹²C¹⁷O, 0-5025 cm⁻¹, 336 165 lines | HITRAN2024, as served | same | 2026-11-18 |
| Partition sums of the same isotopologues | TIPS, as HITRAN serves it | `https://hitran.org/data/Q/q<id>.txt` | 2026-11-18 |
| Water continuum (in `spectrum.rs`, not in the table) | MT_CKD 4.3 | `absco-ref_wv-mt-ckd.nc`, `https://github.com/AER-RC/MT_CKD_H2O`, tag 4.3 | 2026-11-18 |

The design named HITRAN2020 (Gordon et al. 2022); HITRAN's server now
serves only its successor, HITRAN2024, so the table is built from it.

SHA-256 of the inputs, concatenated in file-name order:
`cat raw/lines/h2o_*` `55b9aaec0f3360debac33d5cd05a030d14d5c1626e0dac6347d4090132921b69`,
`cat raw/lines/co2_*` `cec95004862a0652506141e10915cddf4df065e152d6334c5691f96289360b0a`,
`absco-ref_wv-mt-ckd.nc` `69944eb8b045c268e2daeb2cddf536b99f9e9efe7e91c425067b46a5b287ddb3`.

## Citations

- I. E. Gordon et al., "The HITRAN2024 molecular spectroscopic database",
  *J. Quant. Spectrosc. Radiat. Transf.* 109807 (2026),
  doi:10.1016/j.jqsrt.2026.109807.
- I. E. Gordon et al., "The HITRAN2020 molecular spectroscopic database",
  *J. Quant. Spectrosc. Radiat. Transf.* 277, 107949 (2022),
  doi:10.1016/j.jqsrt.2021.107949.
- E. J. Mlawer et al., "Development and recent evaluation of the MT_CKD
  model of continuum absorption", *Phil. Trans. R. Soc. A* 370, 2520
  (2012), doi:10.1098/rsta.2011.0295; MT_CKD_H2O 4.3, AER (2024).
- J. Humlíček, "Optimized computation of the Voigt and complex probability
  functions", *J. Quant. Spectrosc. Radiat. Transf.* 27, 437 (1982).
- A. A. Lacis and V. Oinas, "A description of the correlated k
  distribution method", *J. Geophys. Res.* 96, 9027 (1991).

## How the coefficients are made

- **Lines.** Each line's intensity moves from 296 K to the grid's
  temperature with HITRAN's formula (partition sum, lower-state energy,
  stimulated emission). Its Lorentz half width is
  `(296/T)^n_air γ p`, with `γ` the air-broadened width for the CO₂ set
  and water's foreign set, and the self-broadened width for water's self
  set (HITRAN's `.par` records carry only `n_air`, used for both). Its
  centre moves by `δ_air p`. Its Doppler width is the isotopologue's.
- **Shape.** Voigt (Humlíček's W4), cut at 25 cm⁻¹ from the centre. For
  water the profile's value at 25 cm⁻¹ is subtracted inside the cutoff
  (the "plinth"), MT_CKD's convention, so that the continuum of
  `spectrum.rs` adds exactly what the lines leave out. CO₂'s lines keep a
  plain Voigt shape to 25 cm⁻¹; the sub-Lorentzian far wings of CO₂
  (Perrin & Hartmann 1989) are not included.
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
- Temperatures: 18, 100 to 350 K every 25 K, then 400, 450, 500, 600, 700,
  800 and 900 K.
- Sets: CO₂ broadened by air; water broadened by air; water broadened by
  itself. The column mixes water's two sets by the vapour's share of the
  pressure.
- `mapgen` interpolates `ln k` linearly in `ln p` and in `T`. Past the
  grid it clamps the temperature, and scales `k` as `p` above 100 bar and
  below 1e-5 bar.

## Layout

Little-endian:

| Field | Type |
|---|---|
| magic `MGOPACK1` | 8 bytes |
| bins, g-points, pressures, temperatures, sets | 5 × u32 |
| bin width, cm⁻¹ | f64 |
| g-interval edges | (g-points + 1) × f64 |
| `ln p` of the pressures, bar | pressures × f64 |
| temperatures, K | temperatures × f64 |
| `ln k` of code 1, and the step per code | 2 × f64 |
| codes | sets × pressures × temperatures × bins × g-points × u16 |

A code `q` ≥ 1 stands for `ln k = −30 + (q − 1) × 48/65534` (k from
9.4e-14 to 6.6e7 m²/kg, a step of 0.07 %); code 0 is a coefficient under
9.4e-14 m²/kg, taken as nothing. `opacity.bin` is 3 240 388 bytes
(250 bins × 8 g-points × 15 pressures × 18 temperatures × 3 sets);
`opacity-halved.bin` 900 284 bytes.
