# RFMIP clear-sky shortwave field

`rfmip-shortwave.tsv` holds the clear-sky field of RFMIP-IRF (Pincus et
al. 2016) the shortwave is tested on (`science::radiation::rfmip_tests`),
and the sunlight its atmosphere absorbs by the line-by-line reference,
LBLRTM-12-8, and by RTE-RRTMGP-181204. `tools/rfmip-shortwave` regenerates
it (see its README). This file says where every number comes from and how
it was derived. Study 35 (Wiki `35-stellar-spectra-audit`, issue #655)
chose the tests; the total comes with #715.

## Format

`#` comment lines, then rows led by their kind, fields separated by
tabs:

- `E expt co2_ppmv lblrtm rrtmgp`: an experiment of the rad-irf set (its
  index in the files, named by their `expt_label`), its CO₂, and the
  sunlight the atmosphere absorbs, W/m², weighted over the 100 sites by
  `profile_weight` (night sites absorb nothing). Kept: present day (0),
  0.5x (4), 2x (5), 4x (2) and 8x CO₂ (7), pre-industrial CO₂ (8), +4 K
  (13) and +4 K at constant relative humidity (14).
- `S site weight insolation albedo`: one of the 51 sunlit sites, its
  weight over the whole field (the 100 weights sum to 1), the sunlight at
  its top (`total_solar_irradiance` times the cosine of
  `solar_zenith_angle`), W/m², and its `surface_albedo`.
- `L p t x t4 x4 x4rh`: the site's 61 levels from the surface up:
  pressure, Pa; present-day temperature, K, and water vapour, mol per mol
  of dry air; the same at +4 K; the water at +4 K at constant relative
  humidity. The CO₂ experiments keep the present-day profiles.

| Experiment | CO₂, ppmv | LBLRTM, W/m² | RRTMGP, W/m² |
| --- | --- | --- | --- |
| Present day | 397.5 | 72.0515 | 71.8394 |
| 0.5x CO₂ | 142.2 | 71.5164 | 71.3922 |
| 8x CO₂ | 2274.5 | 73.5936 | 73.5316 |
| Pre-industrial CO₂ | 284.3 | 71.8506 | 71.6715 |
| +4 K | 397.5 | 72.1493 | 71.9867 |
| +4 K, constant RH | 397.5 | 76.3591 | 76.1520 |

## Sources and terms

- **Profiles**: Pincus, R., Forster, P. M., Stevens, B. 2016, *Geosci.
  Model Dev.* 9, 3447, doi:10.5194/gmd-9-3447-2016, the RFMIP-IRF input
  file `multiple_input4MIPs_radiation_RFMIP_UColorado-RFMIP-1-2_none.nc`
  (input4MIPs, v20190401): 100 ERA-Interim profiles of 2014 and their
  weights, 18 experiments.
- **Line-by-line fluxes**: `rsd_Efx_LBLRTM-12-8_rad-irf_r1i1p1f1_gn.nc`
  and `rsu_…` (AER, CMIP6 RFMIP rad-irf, v20190514): LBLRTM 12.8 with
  its radiative transfer, on every level, for each experiment.
- **RRTMGP fluxes**: `rsd_Efx_RTE-RRTMGP-181204_rad-irf_r1i1p1f1_gn.nc`
  and `rsu_…` (RTE-RRTMGP Consortium, v20191007); Pincus, R., Mlawer,
  E. J., Delamere, J. S. 2019, *J. Adv. Model. Earth Syst.* 11, 3074,
  doi:10.1029/2019MS001621.
- **Terms**: CMIP6 data are shared under the CMIP6 terms of use (cite the
  data and the models' papers). The table ships derived numbers only, the
  profiles of the sunlit sites and the weighted absorbed fluxes; the
  netCDF files are not committed.

## Files

`tools/rfmip-shortwave/SHA256SUMS` lists the SHA-256 of the five netCDF
files, equal to the checksums ESGF publishes for them, and of the five
OPeNDAP text views the tool reads (the same variables, served by the
same ESGF node), as downloaded on 2026-11-25 from
`https://esgf-node.ornl.gov/thredds/`. `fetch.sh` checks them. Study 35
read the netCDF files with h5py and found the same weighted fluxes to
four decimals.

## Method

- **Absorbed sunlight**: at each site, what enters the top less what
  leaves it, less what the surface absorbs: `rsd − rsu` at the top level
  less `rsd − rsu` at the surface level.
- **Water at the levels** (`Definition:`): the files give water on the
  60 layers and temperature on the 61 levels; the water of a level is
  interpolated linearly in `ln p` between the layers around it, held at
  the end layers' values outside them.

## How far it can be trusted

- The tests compare differences of experiments that change only CO₂ or
  only water, so that the gases the column lacks (ozone, O₂, CH₄, N₂O)
  cancel. The total cannot be compared until the column holds ozone and
  O₂ (#653, then #715).
- The column takes its fixed slant (diffusivity 1.66) and the table's
  Sun, not each site's zenith angle and the reference's solar spectrum.
- Weighted, not per site: the column's error at each site is not tested.
