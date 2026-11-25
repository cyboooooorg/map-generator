# rfmip-shortwave

Builds `crates/science/data/rfmip-shortwave.tsv`, the RFMIP-IRF
clear-sky field and its line-by-line shortwave fluxes, which the science
crate's tests read (`science::radiation::rfmip_tests`). The generator
never runs this tool: the table is committed, and the tool only
regenerates it. Provenance, terms and method:
`crates/science/data/RFMIP.md`.

## Purpose

RFMIP-IRF (Pincus et al. 2016) computed the clear-sky fluxes of 100
real profiles with a line-by-line model, LBLRTM, for 18 experiments. The
differences between experiments that change only CO₂ or only water test
how the shortwave absorbs them, whatever the gases it lacks.

## Prerequisites

- Rust (the workspace toolchain, see `mise.toml`).
- `curl` and `shasum`, for `fetch.sh`.

## Download

`fetch.sh` downloads into `tools/rfmip-shortwave/raw/` (git-ignored,
about 11 MB) the RFMIP input file and the `rsd` and `rsu` files of
LBLRTM-12-8 and RTE-RRTMGP-181204, and the OPeNDAP text views of the
same variables, from the ESGF node of ORNL, then checks every file
against `SHA256SUMS`. Anonymous requests: no account, no personal data.

```bash
tools/rfmip-shortwave/fetch.sh
```

## Usage

```bash
tools/rfmip-shortwave/fetch.sh
cargo run --release -p rfmip-shortwave
```

Options: `--raw DIR` (default `tools/rfmip-shortwave/raw`) and `--out
FILE` (default `crates/science/data/rfmip-shortwave.tsv`).

## Configuration

None beyond the options. The experiments kept are fixed in
`src/main.rs` (`EXPERIMENTS`).
