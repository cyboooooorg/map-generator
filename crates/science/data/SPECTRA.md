# Stellar spectra

`stellar-spectra.tsv` gives the share of a star's 0.2-5 µm light in each
of the shortwave's 240 bands, for main-sequence stars of 2300 to
27 000 K at [M/H] −1, −0.5, 0 and +0.5. `science::radiation::starlight`
interpolates it linearly in temperature and metallicity. `tools/stellar-spectra`
regenerates it, with `tsis-sun.tsv`, the measured Sun on the same bands
that only the tests read (see its README); this file says where every number comes
from, how it was derived, and the terms it is shipped under. The study
behind it is Wiki study 35, `35-stellar-spectra-audit` (issue #655).

| Rows | Range | Source | Steps |
| --- | --- | --- | --- |
| PHOENIX | 2300-12 000 K | PHOENIX-ACES-AGSS-COND-2011, HiRes (Husser et al. 2013) | 100 K to 7000 K, 200 K above |
| ATLAS9 | 13 000-27 000 K | `ck04models` (Castelli & Kurucz 2003), log g 4.0 | 1000 K |

352 rows (88 temperatures, 4 metallicities), 868 093 bytes.

## Format

`#` comment lines (sources, terms, columns), then a `bands` line (`bands
240 2e-7 5e-6`: the bands' number and outer edges, m, which
`Spectra::parse` checks against the shortwave's), then one row per
spectrum, fields separated by tabs: `metallicity_dex teff_k logg source
share_1 … share_240`. `source` is the name of the downloaded file. The
shares run from the bluest band, are written to five significant
digits, and sum to 1 within 10⁻⁴ (4.8e-6 at most; the test holds 10⁻⁴;
the science crate renormalises).

`tsis-sun.tsv` has the same comment and `bands` lines, then one row of
240 shares (seven significant digits).

## Sources and terms

- **PHOENIX-ACES-AGSS-COND-2011**: Husser, T.-O., Wende-von Berg, S.,
  Dreizler, S., Homeier, D., Reiners, A., Barman, T., Hauschildt, P. H.
  2013, *A&A* 553, A6, doi:10.1051/0004-6361/201219058, Tables 1 and 2.
  Grid: 2300 to 7000 K by 100 K and 7000 to 12 000 K by 200 K, log g 0 to
  6, [Fe/H] −4 to +1. Wavelengths 500 Å to 5.5 µm, in vacuum, in a
  separate file (`WAVE_PHOENIX-ACES-AGSS-COND-2011.fits`); fluxes in
  erg/s/cm²/cm. Terms: the site asks users to cite the paper ("Please cite
  this paper when using spectra from our spectral library!"), and its FTP
  welcome message says the same; no licence is stated.
- **TSIS-1 HSRS**: Coddington, O. M., Richard, E. C., Harber, D. et al.
  2021, *Geophys. Res. Lett.* 48, e2020GL091709,
  doi:10.1029/2020GL091709; the 1 nm file `tsis1_hsrs_1nm.csv` from
  LISIRD (`https://lasp.colorado.edu/lisird/data/tsis1_hsrs`), 202 to
  2730 nm at 0.1 nm steps, SHA-256
  `1cf3b07e6ac9669c429ad7ce9e92d50dfd741422efcfffa3d1e0eeb5f901616f`.
  Terms: none stated for the data; the paper is CC BY. Binned shares
  only, cited.
- **ATLAS9 `ck04models`**: Castelli, F. & Kurucz, R. L. 2003, *New Grids
  of ATLAS9 Model Atmospheres*, IAU Symp. 210, A20,
  arXiv:astro-ph/0405087, as served by STScI
  (`https://archive.stsci.edu/hlsps/reference-atlases/cdbs/grid/ck04models/`,
  its `AA_README`). Grid: 3500 to 50 000 K, log g 0 to 5, [M/H] −2.5 to
  +0.5; 0.1 to 10 µm (10 Å steps below 0.29 µm, 20 Å to 1 µm, 50 to
  200 Å beyond); fluxes in erg/s/cm²/Å. Terms: none stated in the README
  or on the page.

The table ships derived numbers only, binned shares with their citations,
as the stellar tables do with BHAC15 (`STELLAR.md`). The spectra
themselves are never committed. Decided by the user on 2026-11-25
(study 35).

## Files

`tools/stellar-spectra/SHA256SUMS` lists the SHA-256 of each of the 354
downloaded files (the PHOENIX wavelength grid, 292 PHOENIX and 60 ATLAS9
spectra, and TSIS-1) as downloaded on 2026-11-25. `fetch.sh` checks
them, and `cargo run --release -p stellar-spectra -- --check` checks that
the committed tables are what they give.

## Choices

- **Library** (`Definition:`, decided by the user on 2026-11-25): PHOENIX
  where it exists, which covers the hosts the system engine draws from
  2343 K up; ATLAS9 above 12 000 K, to 27 000 K, the hottest main-sequence
  host (8 M☉ at [Fe/H] −1: 26 900 K in `stellar-tracks.tsv`). Where they
  overlap, the two agree on Earth's cloudy albedo within 0.0014 (10 000
  and 12 000 K, study 35).
- **Surface gravity** (`Definition:`): one per temperature range, a
  choice near the main-sequence log g at mid-life of the stellar tables:
  5.0 below 4000 K (5.35 to 4.89), 4.5 from 4000 to 6300 K (4.61 to
  4.38), 4.0 from 6400 K (4.28 to 4.00); ATLAS9 at 4.0. Not a rounding to
  the nearest 0.5 dex, which would give 1.2 M☉'s 4.28 the 4.5. Changing
  log g by 0.5 moves Earth's albedo by under 0.0005 (study 35).
- **Metallicity**: the stellar tables' four ([Fe/H] −1 to +0.5),
  PHOENIX's and ATLAS9's [M/H] taken as [Fe/H] (solar-scaled, no alpha
  enhancement).
- **Binning** (`Definition:`): each sample weighs its flux density times
  its cell, from the midpoint with the previous sample to the midpoint
  with the next, shared among the bands it straddles by overlap.
- **TSIS-1 past 2730 nm** (`Definition:`): a 5772 K black body scaled to
  the data over 2600-2730 nm, about 2 % of the 0.2-5 µm light (with the
  PHOENIX Sun instead, the shares below 400 and 300 nm move by 1e-4).

## How far it can be trusted

- **The Sun.** The table's 5772 K, [M/H] 0 Sun puts 8.92 % of its light
  below 400 nm and 1.37 % below 300 nm, where TSIS-1 HSRS (Coddington et
  al. 2021) puts 8.19 % and 1.03 % and a black body 12.31 % and 2.99 %:
  it closes 82 % and 83 % of the black body's gap. Earth's clear-sky
  albedo under it is 0.1579, under TSIS-1 0.1556. Both are tests
  (`starlight_tests.rs`, `albedo_tests.rs`), computed from `tsis-sun.tsv`.
- **M dwarfs.** No measured spectrum covering 0.2-5 µm was compared:
  MUSCLES (France et al. 2016) observes only to 0.57 µm and uses PHOENIX
  beyond. ATLAS9 and PHOENIX differ by 0.004 to 0.008 of Earth's cloudy
  albedo at 3500-3750 K.
- **The ultraviolet of cool stars.** Photospheric models carry no
  chromosphere. Below 400 nm an M dwarf sends under 0.5 % of its light:
  this matters for photochemistry, not for the albedo.
- **Interpolation.** Linear in temperature and metallicity. A test
  rebuilds every interior row from its neighbours two steps apart, in
  temperature at every metallicity and in metallicity at every
  temperature: Earth's clear-sky albedo moves by 0.0019 at most (13 000
  K at [M/H] −1, across the seam with ATLAS9; 0.0019 within PHOENIX, at
  7400 K and +0.5; 0.0015 in metallicity), so about a quarter of that
  between neighbouring rows. The log g switches (3900 to 4000 K, 6300 to
  6400 K) and the PHOENIX/ATLAS9 seam (12 000 to 13 000 K) are inside
  that bound.
- **Light outside 0.2-5 µm.** The shares are of the 0.2-5 µm light. A
  27 000 K star sends about 70 % of its light at shorter wavelengths,
  which the bands do not hold (a separate issue).
