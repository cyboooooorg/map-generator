# opacity-table

Builds `crates/science/data/opacity.bin`, and with `--shortwave`
`crates/science/data/shortwave.bin`. The second table covers the star's
near infrared on the Bond albedo's bands from 0.7 to 5 µm, with MT_CKD's
water continuum binned in. With `--continua` it writes instead the two
text files of the continua, `mt-ckd-water.tsv` and `co2-cia.tsv`.

`opacity.bin` is the correlated-k table of CO₂ and water line
absorption, with MT_CKD's CO₂ continuum, that the climate column of the
`science` crate reads. The generator never runs this tool: the tables are
committed, and this tool only regenerates them. Their provenance, layout
and citations are in `crates/science/data/OPACITY.md`.

## Purpose

For each absorber set (CO₂ broadened by air, water broadened by air, water
broadened by itself, CO₂ broadened by itself), each pressure and each
temperature of the grid, the tool sums the HITRAN lines of every 20 cm⁻¹
bin from 0 to 5000 cm⁻¹ line by line (Voigt profiles cut at 25 cm⁻¹, less
the pedestal MT_CKD's continua assume: water's flat plinth, CO₂'s
quadratic one from LBLRTM), sorts the absorption coefficients of the bin
and keeps the mean of each of 8 g-intervals. It writes them, quantised in
`ln k`, into one binary file, followed by MT_CKD's CO₂ continuum averaged
over the same bins.

## Prerequisites

- Rust (the workspace toolchain, see `mise.toml`).
- `curl`, for `fetch.sh`.
- About 200 MB of disk for the raw downloads, and a few minutes on four
  cores.

## Download

`fetch.sh` downloads the raw inputs into `tools/opacity-table/raw/`, which
is git-ignored and never committed. The requests are anonymous: no account,
no personal data, a generic User-Agent.

```bash
tools/opacity-table/fetch.sh
```

It fetches:

- HITRAN line lists through HITRAN's public line-by-line API, in pieces of
  250 cm⁻¹ from 0 to 14 500 cm⁻¹ (a piece without lines is answered 404
  and kept as an empty file), for the main isotopologues (global ids 1-4:
  H₂¹⁶O, H₂¹⁸O, H₂¹⁷O, HD¹⁶O; 7-10: ¹²C¹⁶O₂, ¹³C¹⁶O₂, ¹⁶O¹²C¹⁸O,
  ¹⁶O¹²C¹⁷O), as 160-character `.par` records:
  `https://hitran.org/lbl/api?iso_ids_list=1,2,3,4&numin=0&numax=250&head=False&fixwidth=0&sep=[comma]&request_params=par_line`
  (and so on for each piece, and `iso_ids_list=7,8,9,10` for CO₂). The
  server's `Content-Length` overstates each body by one byte per line, so
  `curl` reports exit code 18 on complete files; the script accepts it.
  The API serves the current HITRAN edition: HITRAN2024 (Gordon et al.
  2026) since January 2026.
- The HITRAN partition sums (TIPS), `https://hitran.org/data/Q/q<id>.txt`
  for the same ids.
- MT_CKD 4.3's water continuum coefficients,
  `https://raw.githubusercontent.com/AER-RC/MT_CKD_H2O/4.3/data/absco-ref_wv-mt-ckd.nc`
  (read by `--shortwave` and `--continua`).
- MT_CKD 4.3's CO₂ continuum, in LBLRTM's `contnm.f90` at the commit
  MT_CKD 4.3 pins (`f66bb596`, SHA-256 checked by the script):
  `https://raw.githubusercontent.com/AER-RC/LBLRTM/f66bb596e1686e5ff162a6fadc014e91d3ce01d4/src/contnm.f90`.
- HITRAN2024's CO₂–CO₂ collision-induced absorption,
  `https://hitran.org/data/CIA/main/CO2-CO2_2024.cia` (read by
  `--continua`).

The script ends by checking the SHA-256 of `absco-ref_wv-mt-ckd.nc`,
`contnm.f90` and `CO2-CO2_2024.cia` against the values it records
(`shasum -a 256 -c`) and fails on a mismatch. HITRAN serves its current
edition, so a changed CIA file means a new edition to review.

## Usage

From the repository root, after the download:

```bash
cargo run --release -j 4 -p opacity-table
cargo run --release -j 4 -p opacity-table -- \
  --bin-width 10 --top 2500 --temperatures 200,225,250,275,300 \
  --out crates/science/data/opacity-halved.bin
```

The first command writes `crates/science/data/opacity.bin`; the second
writes the test-only table on bins half as wide, which the test
`halving_bins_keeps_the_forcing` of `spectrum.rs` reads. Both take about
three minutes on four threads.

To build the shortwave table:

```bash
cargo run --release -j 4 -p opacity-table -- --shortwave
```

It writes `crates/science/data/shortwave.bin` in about four minutes on
four threads; the albedo tests of `science` check it.

To write the continuum files:

```bash
cargo run --release -j 4 -p opacity-table -- --continua
```

It needs only `absco-ref_wv-mt-ckd.nc` and `CO2-CO2_2024.cia` and takes
a second. `crates/science/data/mt-ckd-water.tsv` holds MT_CKD's water
continuum at every 50 cm⁻¹ from 0 to 5000 cm⁻¹ (the file's points, not
bin means: `wavenumber_cm self_296K exponent foreign_296K`).
`crates/science/data/co2-cia.tsv` holds the CO₂–CO₂ absorption, cm⁵ per
molecule², averaged (trapezoid) over each 20 cm⁻¹ cell at 200, 300, 450
and 800 K (`cell_start_cm k_200K k_300K k_450K k_800K`): each band of the
file takes the set nearest each temperature, and cells under
1e-50 cm⁵ at every temperature are left out. `--out FILE` redirects both
files to the directory of `FILE`.

Then run the spectrum and column tests of `science` (`mise run test:tier1`).

## Configuration

| Option | Default | Meaning |
| --- | --- | --- |
| `--continua` | off | Write `mt-ckd-water.tsv` and `co2-cia.tsv` instead of a table |
| `--shortwave` | off | Build `shortwave.bin`: the albedo's bands over 0.7-5 µm, 13 temperatures, MT_CKD's water continuum |
| `--raw DIR` | `tools/opacity-table/raw` | Where `fetch.sh` put the downloads |
| `--out FILE` | `crates/science/data/opacity.bin` (`shortwave.bin` with `--shortwave`) | The table to write |
| `--bin-width CM` | `20` | Width of a spectral bin, cm⁻¹ |
| `--top CM` | `5000` | Upper end of the spectrum, cm⁻¹ |
| `--temperatures K,…` | 16 values from 100 to 900 K (13 from 150 K with `--shortwave`) | Temperatures of the grid |
| `--pressure-step DECADES` | `0.5` | Spacing of the pressure grid, 1e-5 to 100 bar |
| `--threads N` | `4` | Worker threads |

`science` reads only the default bins (20 cm⁻¹ up to 5000 cm⁻¹) for its
column; the other options exist for the halved test table and for
sensitivity checks.

## Tests

```bash
cargo test -j 4 -p opacity-table -- --test-threads 4
```

They run on embedded or synthetic lines and never download anything: the
HITRAN record format, the temperature scaling of a line strength against
HITRAN's documented formula, the Voigt function against its integral,
the CO₂ pedestal against LBLRTM's formula, the parsing and binning of the
CO₂ continuum, the
layered line sum against the direct sum, and the g-points against the
line-by-line mean transmission over paths of 1e-4 to 1e2 kg/m².
