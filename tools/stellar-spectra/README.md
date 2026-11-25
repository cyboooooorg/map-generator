# stellar-spectra

Builds `crates/science/data/stellar-spectra.tsv`, the table of stellar
spectra the shortwave of the `science` crate reads
(`science::radiation::starlight::star_bands`). Each spectrum is
integrated over the shortwave's 240 bands (0.2 to 5 µm, even in
`ln lambda`) and normalised to 1. The generator never runs this tool: the
table is committed, and the tool only regenerates it. Its provenance,
terms and method are in `crates/science/data/SPECTRA.md`; the study
behind it is Wiki study 35 (issue #655).

## Purpose

A star is not a black body. Its atmosphere removes light from the blue
and the ultraviolet, where a planet's air scatters, and shapes the near
infrared, where the air absorbs. The table gives the shortwave a model
spectrum for every star the system engine draws: main-sequence stars of
2340 to about 27 000 K, [Fe/H] −1 to +0.5.

## Prerequisites

- Rust (the workspace toolchain, see `mise.toml`).
- `curl` and `shasum`, for `fetch.sh`.
- About 1.9 GB of disk for the raw downloads.

## Download

`fetch.sh` downloads the spectra into `tools/stellar-spectra/raw/`, which
is git-ignored and never committed, then checks each file against
`SHA256SUMS`. The requests are anonymous: no account, no personal data,
a generic User-Agent.

```bash
tools/stellar-spectra/fetch.sh
```

It fetches:

- PHOENIX-ACES-AGSS-COND-2011 HiRes spectra (Husser et al. 2013) and
  their wavelength grid, from
  `https://ftp.gwdg.de/pub/misc/phoenix/HiResFITS/` (where
  `phoenix.astro.physik.uni-goettingen.de/data/` redirects): 2300 to
  7000 K by 100 K and 7200 to 12 000 K by 200 K, at [M/H] −1, −0.5, 0 and
  +0.5 (directories `Z-1.0`, `Z-0.5`, `Z-0.0`, `Z+0.5`).
- ATLAS9 `ck04models` spectra (Castelli & Kurucz 2003) from
  `https://archive.stsci.edu/hlsps/reference-atlases/cdbs/grid/ck04models/`:
  13 000 to 27 000 K by 1000 K, at the same metallicities (`ckm10`,
  `ckm05`, `ckp00`, `ckp05`).

One surface gravity per temperature: the main-sequence log g at
mid-life in the stellar tables of `crates/science/data`, rounded to the
grids' 0.5 dex step. That is 5.0 below 4000 K (0.08 to 0.4 M☉: 5.35 to
4.89), 4.5 from 4000 to 6300 K (0.7 to 1.1 M☉: 4.61 to 4.38), and 4.0
from 6400 K (1.2 to 8 M☉: 4.28 to 4.00). Study 35 found that log g moves
Earth's albedo by under 0.0005.

## Usage

```bash
tools/stellar-spectra/fetch.sh
cargo run --release -p stellar-spectra
```

Options: `--raw DIR` (default `tools/stellar-spectra/raw`) and
`--out FILE` (default `crates/science/data/stellar-spectra.tsv`). It
takes a few seconds.

## Configuration

None beyond the options. The temperatures, metallicities and surface
gravities are fixed in `fetch.sh`: the tool bins whatever spectra it
finds, PHOENIX up to 12 000 K and ATLAS9 above. The science crate
checks that every metallicity has every temperature.

## Method

Each sample of a spectrum weighs its flux density times its cell, half
the distance to each neighbour, and falls in the band holding its
wavelength. Each band's share is its sum over the sum of the bands. The
spectra's units (PHOENIX per cm, ATLAS9 per Å) cancel. A spectrum that
does not span 0.2-5 µm is refused. Shares are written to five
significant digits; the science crate renormalises after interpolating.
