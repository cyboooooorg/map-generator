# Is a star's light a black body?

Seen from: the planet card (the Bond albedo and the surface temperature
of every body lit by a star), through the shortwave half of the climate
([How does a planet's air set its outgoing heat and its
albedo?](radiative-transfer.md)).

## Short answer

Not quite. A star's surface glows roughly like a black body at its
effective temperature, but its own atmosphere takes light out where its
atoms and molecules absorb. The measured Sun gives a third less light
below 400 nm than a black body of its temperature. In an M dwarf,
molecular bands carve the red and near infrared into peaks and
troughs. A planet's air cares about both: Rayleigh
scattering reflects mostly the blue, and water vapour absorbs mostly in
the near-infrared bands. Swapping the black body for a real spectrum
lowers Earth's Bond albedo by about 0.01. Once the Sun's error is absorbed
by the cloud fit, what is left matters most for the coolest M dwarfs,
whose planets a black body made too dark by 0.02 to 0.03. The app now
reads a table of model spectra by temperature and metallicity.

## The science

### What a black body gets wrong

A star's effective temperature $T_\text{eff}$ is defined by its
bolometric flux, $\sigma T_\text{eff}^4$; nothing makes its spectrum a
Planck curve. The Sun, measured from orbit by TSIS-1 (the TSIS-1 Hybrid
Solar Reference Spectrum, HSRS, 202 to 2730 nm; Coddington et al. 2021),
puts 8.2 % of its 0.2-5 µm light below 400 nm and 1.0 % below 300 nm. A
black body at the Sun's 5772 K (IAU 2015 Resolution B3) puts 12.3 % and
3.0 % there (study 35, measured on the bands of the app below). The
deficit is the absorption of the star's own atmosphere: the lines of a
star depend first on its temperature, and in M stars below 3500 K the
molecular bands of titanium oxide and other molecules are very strong
(Fraknoi et al. 2022, §17.3; see [What does a G-type star
mean?](star-types.md)).

The weight that matters for a planet's albedo is the light's Rayleigh
weight, the band-averaged $(550\,\text{nm}/\lambda)^4$. On the bands of
the app (study 35):

| Star | Black body | Spectrum | Spectrum used |
| --- | --- | --- | --- |
| Sun, 5772 K | 1.728 | 1.186 | TSIS-1 HSRS, continued past 2730 nm |
| Sun, 5772 K | 1.728 | 1.267 | PHOENIX, interpolated between 5700 and 5800 K |
| K dwarf, 4500 K | 0.683 | 0.429 | PHOENIX, log g 4.5 |
| M dwarf, 3300 K | 0.202 | 0.139 | PHOENIX, log g 5.0 |
| Late M dwarf, 2600 K | 0.080 | 0.048 | PHOENIX, log g 5.0 |
| A star, 10000 K | 7.72 | 6.96 | PHOENIX, log g 4.0 |
| B star, 27000 K | 19.0 | 20.0 | ATLAS9, log g 4.0 |

A real star of every type below about 15 000 K is redder than its black
body for scattering; the hottest are slightly bluer. The PHOENIX model of
the Sun closes 85 % of the gap between the black body and the measured
Sun (1.267 against 1.186).

### Where the near infrared falls

At 2600 K PHOENIX puts 86 % of the light beyond 1 µm against 81 % for the
black body (study 35, on the app's bands), but the planet's air absorbs 125 W/m² of Earth's
340.25 W/m² against 136 under the black body: more of the light falls
between the planet's absorption bands. Study 35 did not split the
star's bands by molecule. The black body overstates the near-infrared
absorption of the coolest stars' planets and understates their albedo.
At 3300 K the spectrum is again more absorbed than the black body (113
against 109 W/m²).

### The effect on a planet's albedo

The published climate studies with model or observed spectra all find a
planet's Bond albedo falling from F to M hosts at equal flux: through
Rayleigh scattering, weaker in red light, and through the near-infrared
absorption of water and CO₂ (Kopparapu et al. 2013, §4.1, with the
BT-Settl grid between 2600 and 7200 K; Shields et al. 2013, Table 2,
with the F2V HD 128167, the Sun, the K2V HD 22049 and the M dwarf AD
Leo). Shields et al. find 0.329,
0.319, 0.302 and 0.234 for an Earth-like ocean planet with clouds round
the four stars, and 0.193, 0.142, 0.119 and 0.074 with Rayleigh scattering
alone. None of the papers read quantifies the error of a black body
against the spectrum as a function of $T_\text{eff}$. Wordsworth et al.
(2010, §3.1) say only that AD Leo gives lower albedos than the 3700 K
black body of Kasting et al. (1993), mixing a change of spectrum with one
of temperature.

Study 35 measured that error with the app's own shortwave on Earth's air
(77 % humidity over 288 K, its cloud cover 0.682, a ground of 0.122).
First with the clouds of today, then with the cloud albedo refitted on
Earth's 0.293 under each Sun, so that the Sun's own error is absorbed:

| $T_\text{eff}$ (K) | Black body | Spectrum, clouds refitted | Black body less spectrum |
| --- | --- | --- | --- |
| 2400 | 0.186 | 0.214 | −0.028 |
| 2600 | 0.192 | 0.210 | −0.018 |
| 3000 | 0.205 | 0.214 | −0.009 |
| 3300 | 0.215 | 0.221 | −0.006 |
| 3600 | 0.224 | 0.228 | −0.004 |
| 4000 | 0.237 | 0.235 | +0.001 |
| 4500 | 0.252 | 0.250 | +0.002 |
| 5000 | 0.268 | 0.267 | +0.001 |
| 5772 | 0.293 | 0.293 | 0 (the fit) |
| 6500 | 0.317 | 0.318 | −0.001 |
| 7200 | 0.340 | 0.337 | +0.003 |
| 8000 | 0.365 | 0.360 | +0.006 |
| 10 000 | 0.420 | 0.410 | +0.010 |
| 12 000 | 0.462 | 0.457 | +0.005 |
| 15 000 | 0.505 | 0.505 | +0.001 |
| 20 000 | 0.547 | 0.554 | −0.007 |
| 27 000 | 0.576 | 0.591 | −0.015 |

PHOENIX spectra to 12 000 K, ATLAS9 above, solar metallicity; the clouds
refitted on the PHOENIX Sun (0.296), the black body's on its own (0.274). A Bond albedo 0.02 too low gives a
planet at Earth's flux 6.8 W/m² too much; at the climate's slope of about
2 W/m² per kelvin (study 32), about 3 K.

Metallicity moves the spectrum as much as the mid-M dwarfs' error (0.009
at 3000 K):
at 5800 K, [M/H] = −1 raises the cloudy albedo by 0.009 and +0.5 lowers
it by 0.005; at 3300 K, by 0.002 each way. Surface gravity hardly
matters (log g 4.0 against 4.5 at 5800 K, and 4.5 against 5.0 at 3300 K:
under 0.0005).

### Light outside the bands

The bands run from 0.2 to 5 µm and their weights sum to 1: all the
star's light is put in them. A black body at 5772 K has 99.3 % of its
light there, but one at 27 000 K only 32 %, and at 15 000 K 73 %; the
ATLAS9 B star at 27 000 K has 27 %. The rest is far ultraviolet, which
the upper air absorbs (O₂ and ozone on Earth). A black body keeps 93.2 %
of its light in the bands at 10 000 K and 51.9 % at 20 000 K: above about
20 000 K, for B hosts, most of the light lies outside the bands, and its
share there matters more than its shape inside them.

### Spectral libraries

The app draws main-sequence hosts from 0.08 to 8 M☉ at [Fe/H] −1 to
+0.5: 2340 to about 27 000 K. The libraries read for study 35:

| Library | T_eff (K) | [M/H] | 0.2-5 µm | Terms stated |
| --- | --- | --- | --- | --- |
| PHOENIX-ACES-AGSS-COND-2011 (Husser et al. 2013) | 2300-7000 by 100, 7000-12 000 by 200 | −4 to +1 (by 0.5 from −2) | yes (500 Å to 5.5 µm) | citation only |
| BT-Settl, AGSS2009 grid (Allard et al. 2012), through the SVO server | 400-70 000 (uneven) | −4 to +0.5 | yes | none found |
| ATLAS9 `ck04models` (Castelli & Kurucz 2003), STScI | 3500-50 000 | −2.5 to +0.5 | yes (0.1-10 µm, coarse) | none found |
| MUSCLES, Mega-MUSCLES (France et al. 2016; Wilson et al. 2025) | 23 M and K dwarfs, 2600-5200 | the stars' | observed to 0.57 µm, PHOENIX beyond | CC BY 4.0 (MAST) |
| CALSPEC (Bohlin, Gordon & Tremblay 2014) | standards (G dwarfs P330E, P177D; Vega) | the stars' | observed to about 2.5 µm, model beyond | citation only |
| TSIS-1 HSRS (Coddington et al. 2021) | the Sun | solar | 202-2730 nm, 0.3 % from 460 to 2365 nm, 1.3 % outside | none found; paper CC BY |

PHOENIX covers the hosts up to 12 000 K at the stellar tables' four
metallicities; ATLAS9 or BT-Settl the rest. PHOENIX and ATLAS9 agree on
the cloudy albedo within 0.0014 at 10 000 and 12 000 K. Below 4000 K
they part by 0.004 to 0.008, and ATLAS9 has no model below 3500 K.
MUSCLES's spectra beyond 0.57 µm are PHOENIX models, so they cannot
check PHOENIX where an M dwarf's light lies; its observed ultraviolet
matters for photochemistry, not for the albedo. None of the libraries
states a licence beyond asking to be cited.

### What we do not know

- How much of a late M dwarf's albedo error survives in a thick or dry
  air: study 35 measured Earth's air only. Kopparapu et al. (2013, §4.1)
  say both effects grow with a denser atmosphere.
- The model spectra's own error. PHOENIX's Sun puts 8.9 % below 400 nm
  and 1.4 % below 300 nm against TSIS-1's 8.2 % and 1.0 %; for M dwarfs
  no measured spectrum covering 0.2-5 µm was compared in this study.
- The ultraviolet of M dwarfs' chromospheres, absent from photospheric
  models, is under 0.5 % of their 0.2-5 µm light at 3600 K and below: it
  matters for photochemistry, not for the albedo.

## In the app

### What you see

The Bond albedo of the planet card and the temperature it gives: a
planet round a cool M dwarf reflects less than one round the Sun, and one
round an A star more, by the star's spectrum and not only by its
temperature.

### How it computes them

`science::radiation::starlight::star_bands(teff, metallicity)` splits the
star's light over 240 bands, even in $\ln\lambda$ from 0.2 to 5 µm, from
the table `crates/science/data/stellar-spectra.tsv`. The table holds the
band shares of PHOENIX-ACES-AGSS-COND-2011 spectra from 2300 to 12 000 K
and ATLAS9 spectra from 13 000 to 27 000 K, at [M/H] −1, −0.5, 0 and +0.5,
one surface gravity per temperature (the main sequence's).
`tools/stellar-spectra` builds it from the downloaded spectra; the
spectra themselves are not in the repository, and
`crates/science/data/SPECTRA.md` gives their sources, terms, checksums
and method. The shares are interpolated linearly in temperature and
metallicity; a star outside the table takes its edge, and every star the
system engine draws lies inside. The Sun is the table's 5772 K, [M/H] 0
interpolation, like any other star.

The shortwave (`science::radiation::albedo::shortwave_budget`) takes the
star's temperature and metallicity and follows each band through the air.
The climate gives each star the planet's metallicity (the stars of a
system share one) and weights the stars' albedos by their flux (#592).
The cloud albedo (`albedo::CLOUD_ALBEDO`) is fitted on Earth's Bond
albedo under the table's Sun: 0.296 (0.276 under the black body), and
0.322 since #705 fitted it with ozone over the climate's own ground
([ozone.md](ozone.md), "Calibrations").

The tests hold the table's Sun against the measured one, TSIS-1 HSRS
(Coddington et al. 2021), binned on the same bands by the same tool and
committed as `crates/science/data/tsis-sun.tsv` (the tests' reference,
never the generator's): it removes at least 80 % of the black body's
excess below 400 and 300 nm (82 % and 83 % measured), and Earth's
clear-sky albedo under it lies within 0.003 of its value under TSIS-1
(0.1579 against 0.1556), both computed in the tests. Other tests: every
row sums to 1 within 1e-4, the table's bands are the shortwave's, a row
rebuilt from its neighbours in temperature (across PHOENIX's surface
gravity switches and the seam with ATLAS9) or in metallicity lights
Earth's clear sky alike, and the albedo rises from 2600 to 7200 K
(Kopparapu et al. 2013, §4.1).

### Calibrations

Study 35 searched for each of these and found no published value; each
is fitted on its own measurement, decided by the user on 2026-11-25:

- `GAP_CLOSED`, 0.80 (`starlight_tests.rs`): the share of the black
  body's excess over TSIS-1 below 400 and 300 nm the table's Sun must
  remove; measured 0.822 and 0.830. Searched and not found: a published
  accuracy of a model library's solar ultraviolet against TSIS-1 on a
  planet's bands.
- `TSIS_CLEAR_SKY_TOLERANCE`, 0.003 (`albedo_tests.rs`): how far Earth's
  clear-sky albedo under the table's Sun may lie from its value under
  TSIS-1; measured 0.0023. Searched and not found: a model library's
  error against the measured Sun on a planet's albedo.
- `INTERPOLATED_ALBEDO`, 0.001, times `TWO_STEPS` 4 for a row rebuilt
  over two steps (`albedo_tests.rs`): the error of the linear
  interpolation of the shares on Earth's clear-sky albedo; measured
  0.0019 at most over two steps (at 13 000 K, [M/H] −1, and at 7400 K,
  +0.5, within PHOENIX; 0.0015 in metallicity). Searched and not found: a
  published interpolation error of a spectral grid on a planet's albedo.
- `CO2_RANGE_TOLERANCE_W_M2` and `WATER_TOLERANCE_W_M2`
  (`rfmip_tests.rs`): see [How does a planet's air set its outgoing heat
  and its albedo?](radiative-transfer.md), "Calibrations of the RFMIP
  tolerances".
- `CLOUD_ALBEDO`, 0.296 (`albedo.rs`; 0.322 since #705, with ozone,
  see [ozone.md](ozone.md), "Calibrations"): refitted on Earth's Bond
  albedo 0.293 under the table's Sun. The refit was forced: with the black
  body's 0.276 the table's Sun gives Earth 0.2835, outside the 0.0015 of
  the Bond albedo test (`EARTH_BOND_TOLERANCE`). Searched and not found:
  a single cloud reflectance for a one-column model (see
  [radiative-transfer.md](radiative-transfer.md), "The cloud's water,
  the deck and the haze").
- `SNOWBALL_WARM_BAR`, 10.2 bar (`snowball.rs`; 10.1 bar since #705,
  9.80 bar since the pairs of #703):
  refitted on Earth's
  dated snowball share under the new cloud albedo and Earth's argon
  (`d⊕` 0.1468 bar, `W` 10.22) (see [Planet
  atmospheres](planet-atmospheres.md), "Calibration of
  `SNOWBALL_WARM_BAR` (refit)").

### Decisions

Decided by the user on 2026-11-25 (study 35): PHOENIX below 12 000 K and
ATLAS9 above; the Sun from the table, one code path, with TSIS-1 as the
tests' reference; the binned shares shipped with their citations, never
the spectra; metallicity read now. The light outside 0.2-5 µm of hot
hosts has its own issue. Then, for the clear-sky absorption
window: retired, and the column's responses to CO₂ and water tested
against RFMIP's line-by-line fluxes.

### Where it departs from the science

- **Model spectra.** The table's Sun is 0.0023 brighter than the
  measured one in Earth's clear sky. The table interpolates linearly
  across PHOENIX's surface-gravity switches (log g 5.0 to 4.5 between
  3900 and 4000 K, 4.5 to 4.0 between 6300 and 6400 K) and the seam with
  ATLAS9 (12 000 to 13 000 K); the rebuilt-row test bounds what that
  costs Earth's clear sky (0.0019 at most over two steps). No measured M-dwarf spectrum
  covering 0.2-5 µm was compared, and ATLAS9 and PHOENIX differ by 0.004
  to 0.008 of Earth's cloudy albedo at 3500-3750 K.
- **What the black body was.** Study 35 measured it, on main before the
  change:
  - Earth's clear sky (77 %, ground 0.122): 0.1701 under the black body,
    0.1554 under TSIS-1 continued past 2730 nm (by the PHOENIX Sun or by
    the black body, within 0.0001), 0.1577 under PHOENIX; the air absorbs
    48.41, 50.91 and 51.23 W/m². After the binning by overlap and the
    moist adiabat's split at the triple point (#711), TSIS-1 gives 0.1556
    and the table's Sun 0.1579.
  - The cloud albedo that gives Earth's 0.293, re-measured on this
    branch: 0.274 under the black body, 0.300 under TSIS-1, 0.296 under
    the table's Sun (taken). With the black body's 0.276 kept, the
    table's Sun gives 0.2835.
  - The reference Earth (`earth_has_its_climate`, held at 288 K by the
    thermostat) on main with Earth's argon (#694, `6ff2480e`): its Bond
    albedo is 0.2965 and its climate at 280 ppm 287.20 K.
- **After the change**, measured on the same main: the reference Earth's
  Bond albedo is 0.2961 at 288 K and its climate at 280 ppm 287.27 K
  (+0.07 K). Its
  snowball chance fell from 0.0140 to 0.0117 under the old
  `SNOWBALL_WARM_BAR`, refitted to 10.2 bar (0.0140 again; see [Planet
  atmospheres](planet-atmospheres.md), "Calibration of
  `SNOWBALL_WARM_BAR`").
- **Earth's clear-sky absorption at 2.5 cm of water** is 55.80 W/m² under
  the table's Sun, above the 48-54 W/m² window built on Kim & Ramanathan
  (2008)'s rounded "about 50" for water; the black-body Sun passed it.
  The user retired the window on 2026-11-25: no line-by-line reference on
  water and CO₂ alone exists, and the total is compared with LBLRTM on
  RFMIP's field once the column holds O₂ too (ozone's shortwave is in
  since #653's first pull request; #715). Earth's Bond albedo test still guards the budget; the column's responses to
  CO₂ and to water are tested against LBLRTM on RFMIP's field (see [How
  does a planet's air set its outgoing heat and its
  albedo?](radiative-transfer.md), "Window of the H₂O and CO₂
  absorption").
- **All the light in 0.2-5 µm.** For hosts above about 20 000 K most of
  the light lies at shorter wavelengths, and the bands take it all
  (above).

## Sources

1. Allard, F., Homeier, D., Freytag, B. 2012, *Phil. Trans. R. Soc. A*
   370, 2765, doi:10.1098/rsta.2011.0269 (BT-Settl, the AGSS2009 grid
   listed by the SVO theoretical spectra server).
2. Bohlin, R. C., Gordon, K. D., Tremblay, P.-E. 2014, *Publ. Astron.
   Soc. Pac.* 126, 711, doi:10.1086/677655 (CALSPEC).
3. Castelli, F., Kurucz, R. L. 2003, in *Modelling of Stellar
   Atmospheres*, IAU Symp. 210, A20, arXiv:astro-ph/0405087 (the
   `ck04models` grid of the STScI reference atlases, its `AA_README`).
4. Coddington, O. M., Richard, E. C., Harber, D. et al. 2021,
   *Geophys. Res. Lett.* 48, e2020GL091709, doi:10.1029/2020GL091709
   (TSIS-1 HSRS, through LISIRD `tsis1_hsrs_1nm.csv`).
5. Fraknoi, A., Morrison, D., Wolff, S. C. et al. 2022, *Astronomy 2e*,
   OpenStax, §17.3.
6. France, K., Loyd, R. O. P., Youngblood, A. et al. 2016, *Astrophys.
   J.* 820, 89, doi:10.3847/0004-637X/820/2/89 (MUSCLES; HLSP
   doi:10.17909/T9DG6F, CC BY 4.0).
7. Husser, T.-O., Wende-von Berg, S., Dreizler, S. et al. 2013,
   *Astron. Astrophys.* 553, A6, doi:10.1051/0004-6361/201219058
   (PHOENIX-ACES-AGSS-COND-2011, Tables 1 and 2; HiRes files from
   `ftp.gwdg.de/pub/misc/phoenix/HiResFITS`).
8. IAU 2015 Resolution B3, nominal solar effective temperature 5772 K.
9. Kasting, J. F., Whitmire, D. P., Reynolds, R. T. 1993, *Icarus* 101,
   108, doi:10.1006/icar.1993.1010 (as cited by Wordsworth et al. 2010;
   not read).
10. Kopparapu, R. K., Ramirez, R., Kasting, J. F. et al. 2013,
    *Astrophys. J.* 765, 131, doi:10.1088/0004-637X/765/2/131, §4.1
    (read on arXiv:1301.6674v2).
11. Shields, A. L., Meadows, V. S., Bitz, C. M. et al. 2013,
    *Astrobiology* 13, 715, doi:10.1089/ast.2012.0961, §2.4.1 and
    Table 2 (read on arXiv:1305.6926).
12. Wilson, D. J. et al. 2025, *Astrophys. J.* 978, 85,
    doi:10.3847/1538-4357/ad9251 (Mega-MUSCLES).
13. Wordsworth, R. D., Forget, F., Selsis, F. et al. 2010, *Astron.
    Astrophys.* 522, A22, doi:10.1051/0004-6361/201015053, §3.1 (read
    on arXiv:1005.5098).
14. Study 35, *Stellar spectra against the black body*, in the Wiki:
    [35-stellar-spectra-audit](https://github.com/cyboooooorg/map-generator/wiki/35-stellar-spectra-audit).

## See also

- [How does a planet's air set its outgoing heat and its
  albedo?](radiative-transfer.md): the shortwave these spectra feed.
- [What does a G-type star mean?](star-types.md): the temperature scale
  of the hosts.
- [What is the habitable zone?](habitable-zone.md): Kopparapu's edges,
  computed with stellar spectra.
