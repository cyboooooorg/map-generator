# RFMIP clear-sky fields

`rfmip-profiles.tsv` holds the clear-sky field of RFMIP-IRF (Pincus et
al. 2016) the shortwave is tested on (`science::radiation::rfmip_tests`):
the atmospheric conditions of its 51 sunlit sites. The sunlight the
reference models absorb is not in the repository: the tests quote six
weighted values of it (and four of the longwave going out at the top),
cited like values from a paper (below).
`tools/rfmip-shortwave` regenerates the file and prints the ten values
(see its README). This file says where every number comes from, under
which terms, and how it was derived. Study 35 (Wiki
`35-stellar-spectra-audit`, issue #655) chose the tests; the total comes
with #715.

`rfmip-longwave.tsv` holds the clear-sky field of all 100 sites the
infrared column is tested on (`science::radiation::column::rfmip_tests`):
in tier 3 methane's and N₂O's steps from pre-industrial to present day,
against the line-by-line codes' 0.613 and 0.204 W/m² and RRTMGP's 0.563
and 0.180 (Pincus et al. 2020, quoted from
`docs/science/longwave-absorbers.md`; issue #704), and in tier 1 its
outgoing radiation at the top without the halocarbons against the five
line-by-line codes' "PI HCs" (study 40; RFMIP's experiment 12, present
day with the halocarbons at their pre-industrial amounts, its other
gases, temperatures and ozone present day's).

## Format

`#` comment lines, then rows led by their kind, fields separated by
tabs:

- `X expt co2_ppmv`: an experiment of the rad-irf set the tests take
  (its index in the files, named by their `expt_label`) and its CO₂
  (`carbon_dioxide_GM`): present day (0), 0.5x (4) and 8x CO₂ (7),
  pre-industrial CO₂ (8), +4 K (13) and +4 K at constant relative
  humidity (14).
- `S site weight insolation albedo`: one of the 51 sunlit sites, its
  weight over the whole field (the 100 weights sum to 1), the sunlight at
  its top (`total_solar_irradiance` times the cosine of
  `solar_zenith_angle`), W/m², and its `surface_albedo`.
- `L p t x t4 x4 x4rh`: the site's 61 levels from the surface up:
  pressure, Pa; present-day temperature, K, and water vapour, mol per mol
  of dry air; the same at +4 K; the water at +4 K at constant relative
  humidity. The CO₂ experiments keep the present-day profiles.

`rfmip-longwave.tsv`, the same way:

- `X expt co2_ppmv ch4_ppbv n2 o2 n2o_ppbv`: present day (0),
  pre-industrial methane (9, "PI CH4", 808.249 ppb against 1831.471)
  and pre-industrial N₂O (10, "PI N2O", 273.021 ppb against 326.988),
  with their `carbon_dioxide_GM`, `methane_GM`, `nitrogen_GM`,
  `oxygen_GM` and `nitrous_oxide_GM`.
- `S site weight surface_k emissivity`: each of the 100 sites, its
  weight (the 100 sum to 1), its present-day `surface_temperature` and
  `surface_emissivity` (0.98 everywhere), its ground's.
- `L p t x x_o3`: the site's 61 levels from the surface up: pressure,
  Pa; present-day temperature, K; water vapour and ozone, mol per mol of
  dry air. The methane and N₂O experiments keep the present-day
  profiles.

## The references' values

The sunlight the atmosphere absorbs, W/m², weighted over the 100 sites
by `profile_weight` (night sites absorb nothing), as a difference
between two experiments. The tests hold them as `Source:` constants:

| Difference | LBLRTM-12-8 | RTE-RRTMGP-181204 |
| --- | --- | --- |
| Pre-industrial to present-day CO₂ (8 to 0) | 0.2008 | 0.1679 |
| 0.5x to 8x CO₂ (4 to 7) | 2.0772 | 2.1394 |
| +4 K to +4 K at constant RH (13 to 14) | 4.2098 | 4.1653 |

The clear-sky outgoing longwave at the top, W/m²: the `rlu` of the top
level (the least pressure), weighted the same way over the 100 sites.
The test quotes the five line-by-line codes' values from Pincus et al.
2020's published global means (`lw-lbl-models.nc`, see
`docs/science/radiative-transfer.md`); LBLRTM's equal these:

| Experiment | LBLRTM-12-8 | RTE-RRTMGP-181204 |
| --- | --- | --- |
| Present day (0) | 263.2287 | 263.2301 |
| PI HCs (12) | 263.7138 | 263.6449 |

## Sources and terms

- **Profiles**: Pincus, R., Forster, P. M., Stevens, B. 2016, *Geosci.
  Model Dev.* 9, 3447, doi:10.5194/gmd-9-3447-2016; the RFMIP-IRF input
  file `multiple_input4MIPs_radiation_RFMIP_UColorado-RFMIP-1-2_none.nc`
  (input4MIPs, v20190401, University of Colorado,
  hdl:21.14100/f379c294-f7bb-442d-bca7-64661d60780e): 100 ERA-Interim
  profiles of 2014 and their weights, 18 experiments.
- **Line-by-line fluxes**: Mlawer, E., Pernak, R. 2019, AER LBLRTM model
  output prepared for CMIP6 RFMIP rad-irf, Earth System Grid Federation,
  doi:10.22033/ESGF/CMIP6.2528: `rsd_Efx_LBLRTM-12-8_rad-irf_r1i1p1f1_gn.nc`,
  `rsu_…` and `rlu_…` (v20190514), LBLRTM 12.8 on every level, for each
  experiment.
- **RRTMGP fluxes**: Pincus, R. 2019, RTE-RRTMGP-Consortium RTE+RRTMGP
  (2018-12-04 full-resolution) model output prepared for CMIP6 RFMIP
  rad-irf, Earth System Grid Federation, doi:10.22033/ESGF/CMIP6.10173:
  `rsd_Efx_RTE-RRTMGP-181204_rad-irf_r1i1p1f1_gn.nc`, `rsu_…` and
  `rlu_…` (v20191007); Pincus, R., Mlawer, E. J., Delamere, J. S. 2019, *J. Adv.
  Model. Earth Syst.* 11, 3074, doi:10.1029/2019MS001621.
- **Terms of the profiles**: the input file states that its data are
  "licensed under a Creative Commons Attribution "Share Alike" 4.0
  International License", with a link to CC BY 4.0
  (`http://creativecommons.org/licenses/by/4.0/`). The stricter reading
  is taken: `rfmip-profiles.tsv` is shared under
  [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/), with
  the attribution above. The licence applies to that data file only, not
  to the repository's code. Changes made: the 51 sunlit sites of 100
  only; the sunlight at the top made from `total_solar_irradiance` and
  `solar_zenith_angle`; the weights renormalised over all 100 sites; the
  water vapour moved from the layers to the levels (Method); the values
  rounded. As the source states, no warranty. `rfmip-longwave.tsv`
  takes the same terms and attribution; its changes: present day's
  profiles of the 100 sites, the weights renormalised, water and ozone
  moved from the layers to the levels, the values rounded.
- **Terms of the fluxes**: the LBLRTM and RRTMGP files are licensed under
  CC BY-NC-SA 4.0 (and the CMIP6 terms of use,
  `https://pcmdi.llnl.gov/CMIP6/TermsOfUse`). No data derived from them
  is committed: only the ten weighted values above, quoted with their
  citations, as values read from a paper would be. `fetch.sh` and the
  checksums let anyone recompute them.

## Files

`tools/rfmip-shortwave/SHA256SUMS` lists the SHA-256 of the seven netCDF
files, equal to the checksums ESGF publishes for them, and of the eight
OPeNDAP text views the tool reads (the same variables, served by the
same ESGF node), as downloaded on 2026-11-25 from
`https://esgf-node.ornl.gov/thredds/` (the longwave view of the input
file, `ozone`, `surface_temperature`, `surface_emissivity`,
`methane_GM`, `nitrogen_GM`, `oxygen_GM` and `nitrous_oxide_GM`, on
2026-11-27, fetched again that day with `nitrous_oxide_GM` added, the
other variables equal; the other nine matched again that day; the two
`rlu` files and their views on 2026-11-27, their checksums equal to
ESGF's).
`rfmip-longwave.tsv` has the SHA-256
`c4d118bec7a3d1ca16a5eec5df6b4ee84f83d07e82e7703240a72c8d0ce9cbff`;
rebuilding wrote `rfmip-profiles.tsv` byte for byte again. `fetch.sh`
checks them. Study 35 read the netCDF files with h5py and found the same
weighted fluxes to four decimals.

## Method

- **Absorbed sunlight**: at each site, what enters the top less what
  leaves it, less what the surface absorbs: `rsd − rsu` at the top level
  less `rsd − rsu` at the surface level; weighted over the sites, then
  differenced between the experiments.
- **Water at the levels** (`Definition:`): the files give water on the
  60 layers and temperature on the 61 levels; the water of a level is
  interpolated linearly in `ln p` between the layers around it, held at
  the end layers' values outside them. Ozone the same way.
- **The longwave column of a site** (the test's, not the file's): the
  climate's 20 levels per decade from the surface to the top level,
  temperature and water linear in `ln p` between the file's levels (the
  lowest at the file's lowest level, the air's); the ground grey, at the
  file's surface temperature and emissivity (0.98), reflecting the light
  of an air at its temperature (the column's test-only `Ground`, an
  upper bound 0.0968 W/m² over the light the air sends down on the
  field); N₂ the rest of the dry air, argon in its place;
  the experiment's N₂O as a share of the dry air at every level (the
  column's test-only `with_n2o`, since no air of the model holds N₂O);
  the ozone column integrated from the file and laid out on the app's
  AFGL shape (`docs/science/ozone.md`, calibration 5); weighted over the
  sites.

## How far it can be trusted

- The tests compare differences of experiments that change only CO₂ or
  only water, so that the gases the column lacks (ozone, O₂, CH₄, N₂O)
  cancel. The total cannot be compared until the column holds ozone and
  O₂ (#653, then #715).
- The column takes its fixed slant (diffusivity 1.66) and the table's
  Sun, not each site's zenith angle and the reference's solar spectrum.
- Weighted, not per site: the column's error at each site is not tested.
