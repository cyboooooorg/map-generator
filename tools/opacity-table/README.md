# opacity-table

Builds `crates/science/data/opacity.bin`, and with `--shortwave`
`crates/science/data/shortwave.bin`. The second table covers the star's
near infrared on the Bond albedo's bands from 0.7 to 5 µm, with MT_CKD's
water continuum binned in. With `--continua` it writes instead the three
text files of the continua and the collision-induced pairs,
`mt-ckd-water.tsv`, `co2-cia.tsv` and `pairs-cia.tsv`, and
with `--ozone` ozone's shortwave cross-sections on the same bands,
`ozone.tsv`.

`opacity.bin` is the correlated-k table of the line absorption of CO₂,
water, methane, ozone and N₂O, with MT_CKD's CO₂ continuum, that the climate
column of the
`science` crate reads. The generator never runs this tool: the tables are
committed, and this tool only regenerates them. Their provenance, layout
and citations are in `crates/science/data/OPACITY.md`.

## Purpose

For each absorber set (CO₂ broadened by air, water broadened by air, water
broadened by itself, CO₂ broadened by itself, methane broadened by air,
methane broadened by itself, ozone broadened by air, N₂O broadened by
air; the shortwave table keeps the first four), each pressure and each
temperature of the grid, the tool sums the HITRAN lines of every 20 cm⁻¹
bin from 0 to 5000 cm⁻¹ line by line (Voigt profiles cut at 25 cm⁻¹, less
the pedestal LBLRTM removes: water's, methane's, ozone's and N₂O's flat one at
the cutoff, CO₂'s quadratic one), sorts the absorption coefficients of the bin
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
  250 cm⁻¹ from 0 to 14 500 cm⁻¹ for water and CO₂ and from 0 to
  5250 cm⁻¹ for ozone, methane and N₂O (a piece without lines is answered 404
  and kept as an empty file), for the main isotopologues (global ids 1-4:
  H₂¹⁶O, H₂¹⁸O, H₂¹⁷O, HD¹⁶O; 7-10: ¹²C¹⁶O₂, ¹³C¹⁶O₂, ¹⁶O¹²C¹⁸O,
  ¹⁶O¹²C¹⁷O; 16-20: ¹⁶O₃ and its four ¹⁸O and ¹⁷O isotopologues
  (666, 668, 686, 667, 676); 21-25: N₂O 446, 456, 546, 448, 447;
  32-35: ¹²CH₄, ¹³CH₄, ¹²CH₃D, ¹³CH₃D), as
  160-character `.par` records:
  `https://hitran.org/lbl/api?iso_ids_list=1,2,3,4&numin=0&numax=250&head=False&fixwidth=0&sep=[comma]&request_params=par_line`
  (and so on for each piece, `iso_ids_list=7,8,9,10` for CO₂,
  `16,17,18,19,20` for ozone, `21,22,23,24,25` for N₂O,
  `32,33,34,35` for methane). The
  server's `Content-Length` overstates each body by one byte per line, so
  `curl` reports exit code 18 on complete files; the script accepts it.
  Check the pieces before building: on 2026-11-27 three water pieces came
  back mixed with another molecule's records (CO₂ and CO lines, some cut
  short); the tool refuses a malformed record but keeps going on a
  well-formed one of another molecule, which it drops. A piece whose
  records are not all 160 characters, or whose first two characters are
  not its molecule's number, is to be deleted and fetched again
  (`awk 'length($0) != 160'`, `cut -c1-2 | sort | uniq -c`).
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
- HITRAN's nitrogen, oxygen and methane pairs (read by `--continua`):
  `https://hitran.org/data/CIA/main/N2-N2_2021.cia`, `O2-O2_2024.cia`,
  `O2-N2_2024.cia` and `N2-CH4_2024.cia` from the same directory, and
  `https://hitran.org/data/CIA/alternate/CH4-CH4_2011.cia` (the only
  CH₄–CH₄ file HITRAN serves).
- Serdyuchenko et al. 2014's ozone cross-sections (IUP Bremen),
  `https://www.iup.uni-bremen.de/gruppen/molspec/downloads/serdyuchenkogorshelev5digits.dat`
  (read by `--ozone`).

The script ends by checking the SHA-256 of `absco-ref_wv-mt-ckd.nc`,
`contnm.f90`, the six CIA files and
`serdyuchenkogorshelev5digits.dat` against the values it records
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
`halving_bins_keeps_the_forcing` of `spectrum.rs` reads. With methane's,
ozone's and N₂O's four sets they take about twelve and three minutes on four
threads.

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

It needs only `absco-ref_wv-mt-ckd.nc` and the six CIA files and takes
a second. `crates/science/data/mt-ckd-water.tsv` holds MT_CKD's water
continuum at every 50 cm⁻¹ from 0 to 5000 cm⁻¹ (the file's points, not
bin means: `wavenumber_cm self_296K exponent foreign_296K`).
`crates/science/data/co2-cia.tsv` holds the CO₂–CO₂ absorption, cm⁵ per
molecule², averaged (trapezoid) over each 20 cm⁻¹ cell at 200, 300, 450
and 800 K (`cell_start_cm k_200K k_300K k_450K k_800K`): each band of the
file takes the set nearest each temperature, and cells under
1e-50 cm⁵ at every temperature are left out.
`crates/science/data/pairs-cia.tsv` holds the N₂–N₂, O₂–O₂, O₂–N₂,
N₂–CH₄ and CH₄–CH₄ pairs on the same cells up to 5000 cm⁻¹, at every
10 K from 70 to 400 K (`pair cell_start_cm k_70K … k_400K`): each band of
a file (its blocks that share wavenumbers) is averaged over the cell and
interpolated linearly in temperature between its blocks, held at its
ends; the bands that reach a cell add up (provenance and method in
`crates/science/data/OPACITY.md`, "Continua"). `--out FILE` redirects the
three files to the directory of `FILE`.

To write ozone's cross-sections:

```bash
cargo run --release -j 4 -p opacity-table -- --ozone
```

It needs only `serdyuchenkogorshelev5digits.dat` and takes under a
second. `crates/science/data/ozone.tsv` holds, at 223 and 233 K, each
albedo band's cross-sections sorted and averaged over the eight
g-intervals, cm² per molecule (provenance and layout in
`crates/science/data/OPACITY.md`, "Ozone cross-sections").

Then run the spectrum and column tests of `science` (`mise run test:tier1`).

## Configuration

| Option | Default | Meaning |
| --- | --- | --- |
| `--continua` | off | Write `mt-ckd-water.tsv`, `co2-cia.tsv` and `pairs-cia.tsv` instead of a table |
| `--shortwave` | off | Build `shortwave.bin`: the albedo's bands over 0.7-5 µm, 13 temperatures, MT_CKD's water continuum |
| `--ozone` | off | Write `ozone.tsv`: ozone's cross-sections as g-points on the albedo's bands, 223 and 233 K |
| `--raw DIR` | `tools/opacity-table/raw` | Where `fetch.sh` put the downloads |
| `--out FILE` | `crates/science/data/opacity.bin` (`shortwave.bin` with `--shortwave`, `ozone.tsv` with `--ozone`) | The table to write |
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
CO₂ continuum and of the CIA files (nearest set, and the bands
interpolated in temperature), the
layered line sum against the direct sum, and the g-points against the
line-by-line mean transmission over paths of 1e-4 to 1e2 kg/m².
