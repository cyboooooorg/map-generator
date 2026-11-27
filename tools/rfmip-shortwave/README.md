# rfmip-shortwave

Builds `crates/science/data/rfmip-profiles.tsv`, the RFMIP-IRF
clear-sky field (CC BY-SA 4.0, the file's data only), which the science
crate's tests read (`science::radiation::rfmip_tests`), and
`crates/science/data/rfmip-longwave.tsv`, the field of all 100 sites
the infrared column's tier-3 test reads (methane's and N₂O's steps,
`science::radiation::column::rfmip_tests`), and prints the
six weighted line-by-line and RRTMGP shortwave values the tests quote. Nothing
derived from the flux files (CC BY-NC-SA 4.0) is written. The generator
never runs this tool: the table is committed, and the tool only
regenerates it. Provenance, terms and method:
`crates/science/data/RFMIP.md`.

## Purpose

RFMIP-IRF (Pincus et al. 2016) computed the clear-sky fluxes of 100
real profiles with a line-by-line model, LBLRTM, for 18 experiments. The
differences between experiments that change only CO₂ or only water test
how the shortwave absorbs them, whatever the gases it lacks; the
experiments that change only methane or only N₂O test the infrared's
methane and N₂O.

## Prerequisites

- Rust (the workspace toolchain, see `mise.toml`).
- `curl` and `shasum`, for `fetch.sh`.

## Download

`fetch.sh` downloads into `tools/rfmip-shortwave/raw/` (git-ignored,
about 11 MB) the RFMIP input file and the `rsd` and `rsu` files of
LBLRTM-12-8 and RTE-RRTMGP-181204, and the OPeNDAP text views of the
same variables (and a second view of the input file for the longwave
field: ozone, the surface, methane, N₂, O₂ and N₂O), from the ESGF node of ORNL, then checks every file
against `SHA256SUMS`. Anonymous requests: no account, no personal data.

```bash
tools/rfmip-shortwave/fetch.sh
```

## Usage

```bash
tools/rfmip-shortwave/fetch.sh
cargo run --release -p rfmip-shortwave
```

It writes the two tables and prints the six values; compare them with
the `Source:` constants of `crates/science/src/radiation/rfmip_tests.rs`.

Options: `--raw DIR` (default `tools/rfmip-shortwave/raw`), `--out FILE`
(default `crates/science/data/rfmip-profiles.tsv`), `--longwave-out
FILE` (default `crates/science/data/rfmip-longwave.tsv`) and
`--no-fluxes` (write the tables without reading the flux files).

## Configuration

None beyond the options. The experiments kept are fixed in
`src/main.rs` (`EXPERIMENTS`, `LONGWAVE_EXPERIMENTS`).
