# stellar-grid

Regenerates the stellar tables of `crates/science/data`:
`stellar-tracks.tsv`, `stellar-profiles.tsv` and `stellar-low-mass.tsv`.
The generator never runs this tool: the tables are committed, and this
tool only rebuilds them. Their formats, sources and the checks they pass
are in `crates/science/data/STELLAR.md`.

## Purpose

The star of a drawn system needs its present radius, luminosity,
effective temperature and main-sequence lifetime, and its interior
(density, pressure, temperature, transport, where the fusion happens).
From 0.3 to 8 M☉ they come from a grid of main-sequence models computed
with YREC, the Yale Rotating Evolution Code, for four metallicities;
below 0.3 M☉, where YREC's runs fail, from Baraffe et al. (2015) and the
Dartmouth tracks. This tool writes the YREC namelists of the grid from
the ones YREC ships, runs them, and extracts the compact tables from the
raw outputs. Neither the namelists nor the outputs (about 21 GB) are
committed: `prep_grid.py` regenerates the namelists.

## Prerequisites

- Python 3 with `numpy` and `yrec`. The grid was run with the pip
  package `yrec` 2026.2.dev1, whose executable reports itself as
  `YREC v202602 (4ede684)`, and its data at commit `4ede684`:

  ```bash
  pip install numpy yrec==2026.2.dev1
  ```

  On first import `yrec` clones its data (opacities, equations of state,
  start models and the shipped grid namelists) from
  [github.com/yreclab/yrec](https://github.com/yreclab/yrec) at the commit
  matching the build, into `~/.config/yrec`, or into `$YREC_DATA` when
  set.
- About 21 GB of disk for the raw outputs, and about 35 minutes on 8
  cores for the 312 runs.
- For `extract.py`, three downloads, kept outside the repository (only
  derived numbers are committed):
  - Baraffe et al. 2015, `BHAC15_tracks+structure` (the tracks with
    central temperature and density), as distributed by the authors.
  - The Dartmouth tracks of the Kiauhoku grids
    ([Zenodo 14908017](https://doi.org/10.5281/zenodo.14908017),
    CC-BY 4.0): `dartmouth_src.tar.gz`, unpacked, for the
    `m0NNfeh{m10,m05,p00,p05}afep0.trk` files.
  - The B23 standard solar models
    ([Zenodo 10174172](https://doi.org/10.5281/zenodo.10174172),
    CC-BY 4.0): `SolarModels/struct+nu_SF3_MB22m.dat`.

## Usage

Run Python with `-I` when it reads downloaded files, and put each
download in its own directory.

1. Write the 312 namelist pairs into `tools/stellar-grid/namelists/`
   (git-ignored) from the grid namelists in `~/.config/yrec/grid`:

   ```bash
   python -I tools/stellar-grid/prep_grid.py
   ```

   It prints `312 namelist pairs, derived: 44`. With YREC data commit
   `4ede684` the files are identical, byte for byte, to the ones the
   committed tables were computed from.

2. Run the grid (writes `.track`, `.store` and the other YREC outputs):

   ```bash
   python -I tools/stellar-grid/run_grid.py /path/to/ygrid-output
   ```

3. Extract the tables into `crates/science/data` and print the checks
   `STELLAR.md` quotes (solar model against B23, mass-step errors, the
   0.3 M☉ seam, file sizes); about one minute on 8 cores:

   ```bash
   python -I tools/stellar-grid/extract.py \
     --grid /path/to/ygrid-output \
     --bhac15 /path/to/bhac15/BHAC15_tracks+structure \
     --dartmouth /path/to/kiauhoku/dartmouth/src \
     --b23 /path/to/b23/ssm/SolarModels/struct+nu_SF3_MB22m.dat
   ```

Then update `STELLAR.md` with the printed numbers and run the `science`
tests.

## Configuration

| Script | Argument | Default | Meaning |
| --- | --- | --- | --- |
| `prep_grid.py` | `OUT_DIR` | `namelists/` | Where to write the namelists |
| `run_grid.py` | `OUT_DIR` | (required) | Where YREC writes its outputs |
| `run_grid.py` | `NAMELIST_DIR` | `namelists/` | The namelists to run |
| `run_grid.py` | `WORKERS` | `8` | Parallel runs |
| `extract.py` | `--data DIR` | `crates/science/data` | Where to write the tables |
| `extract.py` | `--track-step M` | `0.2` | Mass step of the tracks above 2 M☉ |
| `extract.py` | `--workers N` | `8` | Parallel readers |

`YREC_DATA` overrides the YREC data directory (`~/.config/yrec`). The
profile masses (0.1 M☉ steps to 2.6 M☉, then 3 to 8 M☉ by 1 M☉) are
`PROFILE_HIGH` in `extract.py`; `STELLAR.md` says why.

## The grid

312 runs: 0.3 to 8.0 M☉ by 0.1 M☉, at [Fe/H] −1, −0.5, 0 and +0.5, with
the physics of YREC's shipped `std25` grid (GS98 mixture, OPAL 2006 and
SCV equations of state, grey Eddington atmosphere, mixing length
1.708, core overshoot 0.2 Hp, no diffusion, no rotation). The shipped
namelists cover 0.3-1.9 M☉ (`*_RGB`) and 3.0-8.0 M☉ (`*_TAMS`);
`prep_grid.py` derives the 44 missing ones from the 3.0 M☉ namelist of
the same metallicity, with the mass rescaled (`RSCLM`) and the nearest
lower start model of `startmodels/dbl_grid`:

| [Fe/H] | Derived masses (M☉) | Count |
| --- | --- | --- |
| −1 | 1.8, 1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9 | 12 |
| −0.5 | 1.8, 1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9 | 12 |
| 0 | 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9 | 10 |
| +0.5 | 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9 | 10 |

All 312 namelists also get: at most 20 000 models, a stop at central
hydrogen 10⁻⁴ (`END_XCEN(2)`) or at 20 Gyr (`ENDAGE(2)`), and a stored
structure every 20 models (`NPRTMOD`). Every run
stops at a central hydrogen fraction of 10⁻⁴ or at 20 Gyr, and stores
its structure every 20 models. Start models are written
`{YREC_START}/...`, which the YREC executable expands.

One run, 3.3 M☉ at [Fe/H] −0.5, stalls near the end of the main
sequence and hits the model limit instead; `extract.py` cuts it there
and leaves it out of the tables (the tables use other masses above
2 M☉).

## Licences and citations

- YREC: Demarque et al. 2008 (Ap&SS 316, 31); the public release,
  Pinsonneault 2025, "The YREC Stellar Evolution Code: Public Data
  Release" ([Zenodo 17981745](https://doi.org/10.5281/zenodo.17981745),
  published 2025-12-18). Licence as found on 2026-11-21: the Zenodo
  record is CC-BY 4.0; the pip package `yrec` 2026.2.dev1 declares no
  licence (no `License` field or classifier in its metadata); the data
  checkout at `4ede684` holds no licence file. The repository therefore
  commits no YREC file: only `prep_grid.py`, which rewrites the shipped
  namelists locally, and the numbers derived from the runs, with this
  citation.
- Baraffe, Homeier, Allard & Chabrier 2015, A&A 577, A42. The file has
  no licence: only derived numbers enter the repository, with the
  citation.
- Dotter et al. 2008, ApJS 178, 89 (Dartmouth), through Kiauhoku
  (Claytor et al. 2020, ApJ 888, 43), CC-BY 4.0.
- Herrera & Serenelli 2023, Standard Solar Models B23 / SF-III, CC-BY
  4.0; the `MB22m` model uses the meteoritic composition of Magg et al.
  2022.
