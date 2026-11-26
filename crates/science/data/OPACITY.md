# Opacity table

`opacity.bin` holds the absorption of CO₂'s and water's spectral lines that
the climate column integrates (`crates/science/src/radiation/spectrum.rs`,
issue #523; read by `crates/science/src/radiation/ktable.rs`), and MT_CKD's
CO₂ continuum on the same bins. The lines form a correlated-k table: within
each 20 cm⁻¹ bin from 0 to 5000 cm⁻¹
the line-by-line absorption coefficients are sorted and reduced to 8
g-points, on a grid of pressure and temperature.
`opacity-halved.bin` is the same table on 10 cm⁻¹ bins over 0-2500 cm⁻¹ at
200-300 K, read only by the test `halving_bins_keeps_the_forcing`.
`shortwave.bin` is the table of the star's near infrared that the Bond
albedo absorbs (read by `crates/science/src/radiation/shortwave.rs`;
task 7.6 of legacy issue #523): the same lines and sets on the albedo's
own spectral bands from 0.7 to 5 µm, with MT_CKD's water and CO₂
continua (see [Shortwave table](#shortwave-table)).

Both are built by `tools/opacity-table` (see its README) and never by the
generator, as are the text tables of the continua and the
collision-induced pairs (`mt-ckd-water.tsv`, `co2-cia.tsv`,
`pairs-cia.tsv`, see [Continua](#continua)) and of ozone (`ozone.tsv`).

## Provenance

The tables were copied byte for byte from the tag `legacy-final`. The
generator is `tools/opacity-table`, at the commit `ef6ef794` of 2026-11-19
(`feat(planet): add the CO2 far wings from MT_CKD`), the last to touch
`opacity.bin` before the tag; the tables were built from HITRAN2024
(I. E. Gordon et al., 2026, doi:10.1016/j.jqsrt.2026.109807) as the server
answered on 2026-11-18 and 2026-11-19.

SHA-256 of the files:

| File | SHA-256 |
| --- | --- |
| `opacity.bin` | `d5645512b269292e752745056643c0a4faf1e4dd658cb0ddc8c141cdaaee2ae7` |
| `opacity-halved.bin` | `497afef8d6b3fce2b45a73449a766d3958f915de476a1ee3c69ad4644efc6ab1` |
| `shortwave.bin` | `de6ffe0a5069fc6fcbde77badace9377b40095c3352d3ba385b4aea12b1c2380` |

The line lists cannot be fetched again identically: HITRAN serves its
current data, so a new download may differ from the one the tables were
built from. Neither the tool nor `fetch.sh` checks the line lists' or
partition sums' hashes (those recorded under [Sources](#sources) are of
the downloads of that day); `fetch.sh` checks only the inputs of
the continua and the pairs (see [Continua](#continua)) and the ozone file. Rebuilding
gives a table close to these, not equal to them; keep these files as the
reference and compare their hashes before replacing them.

## Regeneration

```bash
tools/opacity-table/fetch.sh
cargo run --release -j 4 -p opacity-table
cargo run --release -j 4 -p opacity-table -- \
  --bin-width 10 --top 2500 --temperatures 200,225,250,275,300 \
  --out crates/science/data/opacity-halved.bin
cargo run --release -j 4 -p opacity-table -- --shortwave
cargo run --release -j 4 -p opacity-table -- --continua
```

The k-tables (`opacity.bin`, `opacity-halved.bin`) were not regenerated
for the port: they are `legacy-final`'s files byte for byte. The two
continuum files were: on 2026-11-22 the three continuum inputs were
downloaded again by hand, with the URLs of `fetch.sh`, into a scratch
`raw/` (all three matched the SHA-256 that `fetch.sh` checks), and

```bash
cargo run --release -j 4 -p opacity-table -- --continua \
  --raw <scratch>/raw --out <scratch>/out/x
cmp <scratch>/out/mt-ckd-water.tsv crates/science/data/mt-ckd-water.tsv
cmp <scratch>/out/co2-cia.tsv crates/science/data/co2-cia.tsv
```

gave both committed files byte for byte. `pairs-cia.tsv` was written on
2026-11-26 by the same command from the five pair files downloaded that
day with the URLs of `fetch.sh` (their SHA-256 equal to those of study
36's downloads of 2026-11-25, recorded below), which also gave
`mt-ckd-water.tsv` and `co2-cia.tsv` byte for byte again; its values
equal, to the last printed digit, those of the scratch binning study 36
measured with.

The raw downloads (about 80 MB) stay in `tools/opacity-table/raw/`, which
is git-ignored.

## Sources

| Input | Version | Where | Downloaded |
| --- | --- | --- | --- |
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
- T. Karman et al., "Update of the HITRAN collision-induced absorption
  section", *Icarus* 328, 160 (2019), doi:10.1016/j.icarus.2019.02.034:
  the CIA files' format and definitions.
- The sources of `pairs-cia.tsv`'s blocks, as HITRAN's
  `CIA_References_2024.pdf` gives them: D. N. Chistikov et al., *J. Chem.
  Phys.* 151 (19) (2019), doi:10.1063/1.5125756 (N₂–N₂
  roto-translational); W. J. Lafferty et al., *Appl. Opt.* 35, 5911
  (1996), doi:10.1364/AO.35.005911, and Y. Baranov, W. Lafferty and
  G. Fraser, *J. Mol. Spectrosc.* 233, 160 (2005),
  doi:10.1016/j.jms.2005.06.008 (the N₂ and O₂ fundamentals);
  J.-M. Hartmann, C. Boulet and G. C. Toon, *J. Geophys. Res. Atmos.*
  122, 2419 (2017), doi:10.1002/2016JD025677 (N₂ near 2.16 µm);
  F. Thibault et al., *Appl. Opt.* 36, 563 (1997),
  doi:10.1364/ao.36.000563, and J. J. Orlando et al., *J. Geophys. Res.*
  96, 20755 (1991), doi:10.1029/91JD02042 (O₂–N₂); A. A. Finenko et al.,
  *Astrophys. J. Suppl.* 258, 33 (2022), doi:10.3847/1538-4365/ac36d3
  (N₂–CH₄); A. Borysow and L. Frommhold, *Astrophys. J.* 318, 940
  (1987), doi:10.1086/165426 (CH₄–CH₄).

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
- The science crate interpolates `ln k` linearly in `ln p` and in `T`. Past the
  grid it clamps the temperature, and scales `k` as `p` above 100 bar and
  below 1e-5 bar.

## Layout

Little-endian:

| Field | Type |
| --- | --- |
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
`crates/science/src/radiation/ktable.rs` decodes and interpolates both this
layout and the shortwave one below.

## Continua

Three text files hold the continua and the collision-induced pairs the
spectrum reads beside the tables. `tools/opacity-table -- --continua`
regenerates them from the raw downloads of `fetch.sh`, into this
directory, and writes nothing else:

- `mt-ckd-water.tsv`: MT_CKD 4.3's water continuum at every 50 cm⁻¹ from
  0 to 5000 cm⁻¹ (101 rows): the file's own points at those wavenumbers,
  not bin means (`self_absco_ref` at 296 K, `self_texp`,
  `for_absco_ref` at 296 K, from `absco-ref_wv-mt-ckd.nc`, SHA-256 above),
  printed to four significant digits.
- `co2-cia.tsv`: HITRAN's CO₂–CO₂ collision-induced absorption,
  `https://hitran.org/data/CIA/main/CO2-CO2_2024.cia` (HITRAN2024,
  documented by Terragni et al. 2025 and `CIA_Readme_2024.pdf`; SHA-256
  `9db0856cb008cb147728e3a945f818c60a02370442d492c38d7f8884c9cfadca`,
  downloaded 2026-11-22). One row per 20 cm⁻¹ cell: the trapezoid mean of
  the file's coefficient over the cell, at the 200, 300, 450 and 800 K
  columns. Each of the file's bands (roto-translational 1-750 cm⁻¹,
  dimers 1120-1850 cm⁻¹, 2510-2850, 2850-3250 and 3995-4500 cm⁻¹) takes
  the set nearest each column's temperature, held at its warmest set
  past it; a cell takes the first band it touches. Cells under
  1e-50 cm⁵/molecule² at every temperature are left out.

How the generator (`tools/opacity-table/src/cia.rs`) chooses the bands
of the CO₂ file: the file stitches measurements and calculations that
cover different bands at different temperatures, so the five regions
above are fixed in the code, with the slack of 1 cm⁻¹ on each side to
match a block to its region by its header's first and last wavenumber.
A cell takes the first region it overlaps (the regions meet at
2840-2860 cm⁻¹, where the earlier one wins). Within the region, at
each of the four temperatures, it takes the block whose temperature is
nearest, so a temperature past the block's warmest set uses that set.
The roto-translational band starts at 1 cm⁻¹ in the file, and its curve
is taken as zero at 0 cm⁻¹; elsewhere the coefficient is zero outside
the block, a cell a block only partly covers counts the rest as zero,
and a negative mean (the file's noise) counts as zero.

- `pairs-cia.tsv`: HITRAN's nitrogen, oxygen and methane pairs (study
  36, issue #703, `docs/science/longwave-absorbers.md`), downloaded on
  2026-11-25 and again on 2026-11-26 (same bytes):

  | File | Where | SHA-256 | Bands below 5000 cm⁻¹ (K) | Sources of the blocks (`CIA_References_2024.pdf`) |
  | --- | --- | --- | --- | --- |
  | `N2-N2_2021.cia` | `https://hitran.org/data/CIA/main/` | `f0525f75c667802e8dbbfd7054a0a7e07863d8a4ffdb449fe80666bc658c849d` | 0-450, 0-550, 0-650 (70-400); 2000-2698 (228-272) and 1850-3000 (301-363); 4300-5000 (200-330) | Chistikov et al. 2019; Lafferty et al. 1996 and Baranov et al. 2005; Hartmann, Boulet & Toon 2017 |
  | `O2-O2_2024.cia` | same | `0146acfe5c1414b8deaa56be9a63159154479d5804d1615855888a2e1efd4961` | 1150-1950 (193-353) | Baranov et al. 2005 |
  | `O2-N2_2024.cia` | same | `90851fbfae226d7ecda93efc0de16105f72966a532adc7f1bc234cb13833799f` | 1300-1850 (193-356); 1850-3000 and 2000-2698 (228-363) | Thibault et al. 1997, Orlando et al. 1991; N₂'s band estimated from N₂–N₂ (Lafferty et al. 1996, eq. 7) |
  | `N2-CH4_2024.cia` | same | `7056ee0bc4a38a4a4d2b215b40312ba5be6029dd6098bf1b1c76d5522eb12523` | 0-800 (70-400) | Finenko et al. 2022 |
  | `CH4-CH4_2011.cia` | `https://hitran.org/data/CIA/alternate/` (the only CH₄–CH₄ file HITRAN serves) | `12de769200685d2bb7faff7550b6276ee20c9e3d3af4f318d2dc46935d39403a` | 0-990 (40-400) | Borysow & Frommhold 1987 |

  One row per pair and 20 cm⁻¹ cell up to 5000 cm⁻¹ (`pair
  cell_start_cm k_70K … k_400K`, cm⁵/molecule², five significant
  digits): the coefficient at every 10 K from 70 to 400 K, the grid of
  the N₂–N₂ roto-translational and N₂–CH₄ blocks. The generator
  (`cia::Cia::interpolated`) groups each file's blocks into bands (a
  block joins the first band one of whose blocks shares wavenumbers
  with it), takes each block's trapezoid mean over the cell (a negative
  point, the file's noise, counted as zero; zero outside the block),
  averages a band's blocks at the same temperature, interpolates the
  band linearly in temperature between its blocks, held at its coldest
  and warmest, and adds the bands that reach the cell. Cells under
  1e-50 cm⁵/molecule² at every temperature are left out: 127 cells of
  N₂–N₂, 41 of O₂–O₂, 86 of O₂–N₂, 40 of N₂–CH₄ and 50 of CH₄–CH₄,
  130 767 bytes. Bands past 5000 cm⁻¹ (O₂'s visible and near-infrared
  ones) are outside the longwave spectrum and not written.
- **Terms of the CIA files:** HITRAN's pages state a citation policy
  (cite the edition, HITRAN2024, Gordon et al. 2026, and the original
  sources of the data used), no licence; the CIA readme and the files
  state none. `co2-cia.tsv` and `pairs-cia.tsv` hold derived numbers
  only (cell means), cited above and in `spectrum.rs`; the raw files are
  not committed (`tools/opacity-table/raw/` is ignored by git).

`fetch.sh` checks the SHA-256 of these inputs (the files above and
`contnm.f90`) and fails on a mismatch; it checks no line list.

## Shortwave table

`shortwave.bin` is built by `opacity-table --shortwave` from the same
line lists, partition sums and line shapes as `opacity.bin`:

- **Bins:** the bands of the Bond albedo's stellar spectrum
  (`crates/science/src/radiation/starlight.rs`, ported from the legacy
  `planet::albedo::spectrum`; 240 bands even in `ln λ` from 0.2 to
  5 µm), from the one that holds 0.7 µm to 5 µm: 147 bins over
  2000-14 364 cm⁻¹, 27 to 193 cm⁻¹ wide. The science crate checks the
  table's edges against its bands when it loads it.
- **Grid:** the same 15 pressures and 4 sets; 13 temperatures, 150, 175,
  200, 225, 250, 275, 300, 325, 350, 400, 500, 650 and 900 K, so that the
  table stays under 2 MB.
- **Continua:** MT_CKD 4.3's CO₂ continuum (`contnm.f90`, to 10 000
  cm⁻¹) as in `opacity.bin`, and its water continuum, the mean over each
  bin of the file's `self_absco_ref` and `for_absco_ref` and the
  coefficient-weighted mean of `self_texp` (read from
  `absco-ref_wv-mt-ckd.nc`, NetCDF classic).
- **Not in it:** CO₂–CO₂ collision-induced absorption, which the science
  crate takes from `co2-cia.tsv` (see Continua); ozone and O₂. The
  shortwave takes the CO₂–CO₂ pairs only, not those of `pairs-cia.tsv`
  (N₂'s fundamental at 4.3 µm and first overtone at 2.2 µm).

Layout, little-endian, as `opacity.bin` but for these fields:

| Field | Type |
| --- | --- |
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

## Ozone cross-sections

`ozone.tsv` is built by `opacity-table --ozone` from one file:

- **Source:** Serdyuchenko, A., Gorshelev, V., Weber, M., Chehade, W.,
  Burrows, J. P. 2014, "High spectral resolution ozone absorption
  cross-sections – Part 2: Temperature dependence", *Atmos. Meas. Tech.*
  7, 625, doi:10.5194/amt-7-625-2014. Data file
  `serdyuchenkogorshelev5digits.dat` (IUP Bremen, molecular spectroscopy
  lab, created 05.03.2012), downloaded on 2026-11-25 from
  `https://www.iup.uni-bremen.de/gruppen/molspec/downloads/serdyuchenkogorshelev5digits.dat`:
  213.33 to 1100 nm on a 0.01 nm grid of vacuum wavelengths, cm² per
  molecule at 293 to 193 K in steps of 10 K, total relative systematic
  uncertainty under 3 % (its header).
- **SHA-256:**
  `4dfbf021b746512c192df5f0d43c54cee6ea3b4365bb490bcf6ed347f0ce7092`
  (checked by `fetch.sh`; recorded in the tool's `ozone::SHA256` and in
  the table's header, which a test of the tool compares).
- **Terms:** the data page and the file state none; the paper is
  published under CC BY 3.0. The table holds derived numbers only (the
  g-point means below), cited; the raw file is not committed
  (`tools/opacity-table/raw/` is ignored by git).
- **Reduction:** each sample goes to the band of the albedo's spectrum
  (240 bands even in `ln λ` from 0.2 to 5 µm, `starlight.rs`) that holds
  its wavelength; a band's samples at one temperature are sorted and
  averaged over the eight g-intervals of the k-tables (edges 0, 0.3,
  0.6, 0.8, 0.9, 0.96, 0.99, 0.998, 1). A negative sample (the file's
  noise around zero) counts as zero. The four bands below 213.33 nm take
  the first band's g-points; the bands past 1100 nm are not written (the
  science crate reads them as zero). Only 223 K (the layer's) and 233 K
  (for a test) are kept.
- **Layout:** comment lines (`#`), a `bands` line (count and outer
  edges, m, checked against the star's bands when the crate loads it), a
  `g` line (the g-intervals' edges), then one row per temperature and
  band, `temperature_k band sigma_1 … sigma_8`, cm² per molecule, band 0
  the bluest. 26.6 kB.
- **Checked by:** `radiation::ozone`'s tests (the Hartley peak
  1.165e-17 cm² at 255.3 nm and the Chappuis peak 5.16e-21 cm² at
  602.5 nm at 223 K, and the file's 233 K peaks, within 1e-3) and the
  tool's (the bands and g lines, the SHA-256).

Regeneration, from the repository root:

```bash
tools/opacity-table/fetch.sh
cargo run --release -j 4 -p opacity-table -- --ozone
```

It reads `raw/serdyuchenkogorshelev5digits.dat` and writes
`crates/science/data/ozone.tsv` in under a second.
