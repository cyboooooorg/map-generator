# opacity-table

Builds `crates/mapgen/data/opacity.bin`, the correlated-k table of CO₂ and
water line absorption that the climate column of `mapgen` reads
(`crates/mapgen/src/planet/spectrum.rs`). The generator never runs this
tool: the table is committed, and this tool only regenerates it. The
table's provenance, layout and citations are in
`crates/mapgen/data/OPACITY.md`.

## Purpose

For each absorber set (CO₂ broadened by air, water broadened by air, water
broadened by itself), each pressure and each temperature of the grid, the
tool sums the HITRAN lines of every 20 cm⁻¹ bin from 0 to 5000 cm⁻¹ line by
line (Voigt profiles cut at 25 cm⁻¹, water's plinth removed for MT_CKD),
sorts the absorption coefficients of the bin and keeps the mean of each of
8 g-intervals. It writes them, quantised in `ln k`, into one binary file.

## Prerequisites

- Rust (the workspace toolchain, see `mise.toml`).
- `curl`, for `fetch.sh`.
- About 100 MB of disk for the raw downloads, and a few minutes on four
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
  250 cm⁻¹ from 0 to 5000 cm⁻¹, for the main isotopologues (global ids 1-4:
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
  (kept for provenance: `spectrum.rs` holds the continuum, read from this
  file in task 7.1).

## Usage

From the repository root, after the download:

```bash
cargo run --release -j 4 -p opacity-table
cargo run --release -j 4 -p opacity-table -- \
  --bin-width 10 --top 2500 --temperatures 200,225,250,275,300 \
  --out crates/mapgen/data/opacity-halved.bin
```

The first command writes `crates/mapgen/data/opacity.bin`; the second
writes the test-only table on bins half as wide, which the test
`halving_bins_keeps_the_forcing` of `spectrum.rs` reads. Both take about
three minutes on four threads.

Then run `cargo test -p mapgen spectrum` and the column tests.

## Configuration

| Option | Default | Meaning |
|---|---|---|
| `--raw DIR` | `tools/opacity-table/raw` | Where `fetch.sh` put the downloads |
| `--out FILE` | `crates/mapgen/data/opacity.bin` | The table to write |
| `--bin-width CM` | `20` | Width of a spectral bin, cm⁻¹ |
| `--top CM` | `5000` | Upper end of the spectrum, cm⁻¹ |
| `--temperatures K,…` | 18 values from 100 to 900 K | Temperatures of the grid |
| `--pressure-step DECADES` | `0.5` | Spacing of the pressure grid, 1e-5 to 100 bar |
| `--threads N` | `4` | Worker threads |

`mapgen` reads only the default bins (20 cm⁻¹ up to 5000 cm⁻¹) for its
column; the other options exist for the halved test table and for
sensitivity checks.

## Tests

```bash
cargo test -j 4 -p opacity-table -- --test-threads 4
```

They run on embedded or synthetic lines and never download anything: the
HITRAN record format, the temperature scaling of a line strength against
HITRAN's documented formula, the Voigt function against its integral, the
layered line sum against the direct sum, and the g-points against the
line-by-line mean transmission over paths of 1e-4 to 1e2 kg/m².
