# Stellar tables

Three tab-separated tables give a main-sequence star's present values and
interior from 0.08 to 8 M☉ and [Fe/H] −1 to +0.5. `tools/stellar-grid`
regenerates them (see its README); this file says where every number
comes from, how it was derived, and how far it can be trusted.

| File | Range | Source | Size (bytes) |
| --- | --- | --- | --- |
| `stellar-tracks.tsv` | 0.3-8 M☉, 4 [Fe/H] | YREC grid, computed for this repository | 422 584 |
| `stellar-profiles.tsv` | 0.3-8 M☉, 4 [Fe/H] | YREC grid | 1 814 254 |
| `stellar-low-mass.tsv` | 0.08-0.4 M☉ | Baraffe et al. 2015; Dartmouth offsets | 5 764 |

Total 2 242 602 bytes (the budget is about 2.5 MB).

## Common format

Each file starts with `#` comment lines (source, licence, columns), then
one row per line, fields separated by tabs, decimal numbers written with
`%.6g`. [Fe/H] is in dex (−1, −0.5, 0, 0.5). The tracks have 0.1 M☉
steps from 0.3 to 2 M☉, then 0.2 M☉ steps (2, 2.2, … 8). The profiles
have 0.1 M☉ steps from 0.3 to 2.6 M☉, then 3, 4, 5, 6, 7 and 8 M☉. See
[Mass steps above 2 M☉](#mass-steps-above-2-m).

## The YREC grid

312 runs of YREC (`yrec` 2026.2.dev1, data commit `4ede684`), 0.3 to
8.0 M☉ by 0.1 M☉ at four metallicities, with the physics of YREC's
shipped `std25` grid: GS98 mixture, OPAL 2006 and SCV equations of state,
grey Eddington atmosphere, mixing length 1.708, core overshoot 0.2 Hp,
no diffusion, no rotation. Initial compositions (from the run headers):

| [Fe/H] | X | Z |
| --- | --- | --- |
| −1 | 0.750950 | 0.001721 |
| −0.5 | 0.743176 | 0.005386 |
| 0 | 0.719620 | 0.016492 |
| +0.5 | 0.654061 | 0.047401 |

Each run starts on the pre-main sequence and stops at a central hydrogen
mass fraction X_c = 10⁻⁴ or at 20 Gyr. One run, 3.3 M☉ at [Fe/H] −0.5,
stalled at X_c = 0.0044 until it hit the model limit (age 0.285 Gyr,
against 0.268 Gyr for 3.2 M☉); it is cut where X_c first reached that
level and is not in any table (3.3 M☉ is not a tabulated mass).

Definitions used by the extraction:

- **ZAMS** (`Definition:`): the first model whose nuclear luminosity
  (pp chains, CNO, triple alpha) is at least 99 % of L and whose radius
  has stopped shrinking (the next model is not smaller). The two
  conditions together mark a star that has finished its pre-main-sequence
  contraction and lives on nuclear power. Neither suffices alone:
  - Nuclear power alone, first crossing: the out-of-equilibrium burning of
    ¹²C on the approach to the main sequence gives ≥ 99 % for a few models
    while the star still contracts, then drops (to about 60 % above
    2 M☉). That puts the ZAMS on the pre-main sequence for some masses and
    not others.
  - Nuclear power alone, last unbroken run of ≥ 99 % before 10 % of X_c
    is burnt (the first rule of this file): after the ZAMS, ³He and CN
    approach equilibrium for a long time and L_nuc / L wavers around
    0.97-1.05. A late dip below 0.99 moved the ZAMS deep into the main
    sequence: 4.62 Gyr at 0.4 M☉ and +0.5 (0.39 and 0.17 Gyr at 0.3 and
    0.5 M☉), 0.149 Gyr at 1.2 M☉ and +0.5 (0.042 at 1.1 M☉).
  - The radius minimum alone, sought before 10 % of X_c is burnt: the
    stars that build a convective core contract again after the ¹²C
    transient, and the deeper of the two minima switches from one mass to
    the next.

  With this rule the ZAMS age falls with mass at every metallicity, or
  rises by under 0.004 dex (three steps above 4.9 M☉), except at four
  steps of 0.1 M☉. There the ¹²C transient first becomes deep enough to
  stop the contraction early, and the next mass settles after it:

  | [Fe/H] | Masses (M☉) | ZAMS ages (Gyr) |
  | --- | --- | --- |
  | −1 | 1.1 → 1.2 | 0.0200 → 0.0263 |
  | −0.5 | 1.1 → 1.2 | 0.0238 → 0.0361 |
  | 0 | 1.4 → 1.5 | 0.0184 → 0.0547 |
  | 0 | 3.3 → 3.4 | 0.0031 → 0.0036 |

  Each step is 0.2-1.3 % of the star's main-sequence life. At the ZAMS at
  most 0.78 % of the central hydrogen is burnt (1.4 M☉ at 0 burns 0.04 %,
  1.5 M☉ 0.78 %).
- **xc_frac**: X_c divided by X_c at the ZAMS.
- **TAMS**: X_c = 10⁻⁴. Divided by X_c at the ZAMS (0.65 to 0.75), the
  last row's xc_frac is at most 1.52 × 10⁻⁴ (1.1 M☉ at +0.5; the last
  step often overshoots to X_c = 0). A track that ends at 20 Gyr
  instead ends with xc_frac ≥ 1.64 × 10⁻³ (0.9 M☉ at +0.5). A reader
  tells them apart with a threshold between the two, such as xc_frac <
  10⁻³; 1.5 × 10⁻⁴ would miss 1.1 M☉ at +0.5. The tracks that end at
  20 Gyr: 0.3-0.7 M☉ at [Fe/H] −1 and −0.5, 0.3-0.8 M☉ at 0, 0.3-0.9 M☉
  at +0.5.

## `stellar-tracks.tsv`

Columns: `mass_msun feh age_gyr log_l log_r log_teff xc_frac`, with age
in Gyr since the start of the run (pre-main sequence included), log_l =
log₁₀(L/L☉), log_r = log₁₀(R/R☉) at the photosphere, log_teff =
log₁₀(T_eff / K).

Rows per (mass, [Fe/H]), in age order: the ZAMS row; the track rows
nearest 40 values of xc_frac evenly spaced from 1 down to the last
model's; the rows nearest xc_frac 0.01, 0.003 and 0.001 (the hook at the
end of the main sequence, where the star contracts in 1 % of its life);
the last model. A track has 29 to 44 rows (rows nearest two targets are
kept once).

## `stellar-profiles.tsv`

One block per snapshot:

```text
P  mass_msun feh age_gyr xc_frac n_layers n_samples
L  r_outer_frac m_outer_frac transport        (n_layers rows, centre out)
S  r_frac m_frac log_rho_cgs log_p_cgs log_t fusion_share transport
                                               (n_samples rows, centre out)
```

`transport` is `R` (radiative) or `C` (convective). ρ in g cm⁻³, P in
dyn cm⁻², T in K, all log₁₀. Radii are fractions of the photospheric
radius, masses of the star's mass.

- **Snapshots**: the stored model (YREC stores one in 20) nearest each of
  xc_frac 1, 0.9, 0.7, 0.5, 0.3, 0.1 and 0.01 that the run reaches; the
  xc_frac 1 snapshot is the first stored model at or after the ZAMS. A run
  that stops at 20 Gyr before xc_frac 0.01 adds its last model. The `P`
  row carries the snapshot's actual age and xc_frac, so a reader
  interpolates between the bracketing snapshots.
- **Samples**: 50 radii, the same in every snapshot: 0 to 0.875 by 0.025,
  0.9 to 0.97 by 0.01, then 0.98, 0.985, 0.99, 0.995, 0.998 and 1. The
  centre (r = 0) takes the central ρ, P, T of the track; the other radii
  are linear in r between YREC's shells. The structure stops at the
  photosphere: the shells flagged `ATM_PT` (the grey atmosphere, whose
  radii are not monotonic) are cut, and the last shell before them is at
  the track's radius and T_eff.
- **Layers**: the runs of YREC's `CONVECTIVE` flag from the centre to
  the photosphere. A boundary is halfway between the last shell of one
  run and the first of the next. A run thinner than 0.001 R, below the
  table's radial resolution, is merged into its inner neighbour (the
  innermost run into its outer one; in the Sun, a radiative skin of a
  few shells within 10⁻⁴ R of the photosphere). Thin convection
  zones of the hot stars' envelopes (ionisation zones of H and He, a few
  thousandths of R) are kept. A sample's transport is that of the layer
  that contains it.
- **fusion_share**: the running integral of YREC's `E_NUC` over mass
  from the centre, divided by its total, kept non-decreasing and ending
  at 1. Its total matches the track's luminosity within 0.1 % at 1 M☉.
- **Precision**: with six significant digits, m_frac reads 1 where less
  than 5 × 10⁻⁷ of the mass lies outside: from r = 0.995 R up to
  1.2 M☉, from 0.96 R above 1.6 M☉. Masses are therefore non-decreasing,
  not strictly increasing, along a snapshot; so is fusion_share, which
  reaches 1 well inside.

## Mass steps above 2 M☉

Measured with `extract.py`. For an interval whose mid-point is a grid
mass, the mid-point mass is interpolated linearly in log mass from the
interval's ends and compared with the computed one, for every
metallicity (the failed 3.3 M☉ run excluded).

- Profiles: at each snapshot xc_frac (each mass first brought to that
  exact xc_frac between its stored models), at equal r_frac; worst
  |error| over log ρ, log P, log T.
- Tracks: at equal fractional main-sequence age τ = 0 to 1 by 0.01, as
  the tracks are read; worst |error| of log L, log R, log T_eff.

### Profiles

The grid of the table, worst |error| in dex:

| Interval (M☉) | All radii | r ≤ 0.9 R | Worst at (mid, [Fe/H], xc_frac, r/R, variable) |
| --- | --- | --- | --- |
| 2.0-2.6 by 0.2 (bound for the 0.1 step) | 0.136 | 0.078 | 2.1, +0.5, 0.3, 0.98, ρ |
| 2.6-3 | 0.055 | 0.008 | 2.8, 0, 0.5, 1.0, ρ |
| 3-4 | 0.221 | 0.031 | 3.5, +0.5, 0.3, 1.0, ρ |
| 4-5 | 0.054 | 0.014 | 4.5, +0.5, 0.1, 1.0, ρ |
| 5-6 | 0.027 | 0.027 | 5.5, −0.5, 0.01, 0.775, P |
| 6-7 | 0.024 | 0.024 | 6.5, 0, 0.01, 0.775, P |
| 7-8 | 0.022 | 0.022 | 7.5, +0.5, 0.01, 0.775, P |

The 0.1 M☉ steps up to 2.6 M☉ have no grid mid-point to test against.
The 0.2 M☉ step over the same range bounds their error from above: 0.136
dex at the worst radius, 0.078 dex inside 0.9 R. Over 2.0-2.5 M☉ at
[Fe/H] +0.5 and 0, the envelope convection zone retreats to the surface.
Its base moves from 0.957 to 0.985 R between 2.0 and 2.1 M☉ at +0.5, so
ρ and P near 0.9-0.98 R change by up to 0.4 dex across 0.2 M☉. The
0.1 M☉ steps keep that change in the table, as far as the grid resolves
it.

Above 2.6 M☉ the 1 M☉ steps reach 0.2 dex only at the photosphere
(r = 1, density). Elsewhere the worst cases are at xc_frac 0.01, where
X_c changes fast between stored models, and they stay under 0.031 dex.

Uniform steps over all of 2-8 M☉, for comparison:

| Step (M☉) | All radii | r ≤ 0.9 R |
| --- | --- | --- |
| 0.2 | 0.136 | 0.078 |
| 0.4 | 0.270 | 0.235 |
| 0.6 | 0.407 | 0.370 |
| 1.0 | 0.507 | 0.408 |
| 2.0 | 0.607 | 0.401 |

With the 0.1 M☉ grid everywhere the profiles would be 5.0 MB.

### Tracks

Worst |error| in dex over all intervals and metallicities:

| Step (M☉) | log L | log R | log T_eff |
| --- | --- | --- | --- |
| 0.2 (the table) | 0.0080 | 0.0094 | 0.0060 |
| 0.4 | 0.0113 | 0.0139 | 0.0090 |
| 0.6 | 0.0099 | 0.0162 | 0.0091 |
| 1.0 | 0.0155 | 0.0180 | 0.0093 |
| 2.0 | 0.0221 | 0.0223 | 0.0100 |

At 0.2 M☉ steps all three stay under 0.01 dex. The worst log L errors
are at τ = 1, the end of the main sequence.

## The Sun: YREC against B23

1 M☉, [Fe/H] 0, the stored model nearest 4.57 Gyr (model 480, 4.5728
Gyr, xc_frac 0.5290): R = 0.9995 R☉ (IAU nominal radius), L =
0.9982 L☉, T_eff = 5770.9 K, ρ_c = 145.4 g cm⁻³, T_c = 1.535 × 10⁷ K.
Base of the convective envelope: 0.7262 R as a layer bound (halfway
between shells), 0.7272 R as the first convective shell; helioseismology
gives 0.713 (Basu & Antia 1997, MNRAS 287, 189).

Reference: B23 standard solar model `struct+nu_SF3_MB22m.dat`
(Herrera & Serenelli 2023, Zenodo 10174172, CC-BY 4.0): ρ_c = 150.0
g cm⁻³, T_c = 1.558 × 10⁷ K. The YREC column is what a reader of the
table gets: the two snapshots that bracket xc_frac 0.5290 (0.688 and
0.476), interpolated linearly in xc_frac, then linear in r. It differs
from the stored model's own shells by at most 0.6 % in ρ, P and T. The
last two columns compare the fusion share with B23's L(r) divided by its
surface value.

| r/R | ρ YREC | ρ B23 | P YREC | P B23 | T YREC | T B23 | m/M YREC | m/M B23 | fusion share | B23 L(r)/L |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 146.1 | 150.0 (−2.6 %) | 2.283e17 | 2.297e17 (−0.6 %) | 1.536e7 | 1.558e7 (−1.4 %) | 0 | 0 | 0 | 0 |
| 0.05 | 124.2 | 126.3 (−1.7 %) | 1.961e17 | 1.965e17 (−0.2 %) | 1.462e7 | 1.482e7 (−1.3 %) | 0.0119 | 0.0120 | 0.0924 | 0.0934 |
| 0.10 | 86.01 | 86.35 (−0.4 %) | 1.339e17 | 1.332e17 (+0.5 %) | 1.292e7 | 1.307e7 (−1.1 %) | 0.0750 | 0.0753 | 0.4415 | 0.4457 |
| 0.20 | 35.20 | 34.82 (+1.1 %) | 4.380e16 | 4.308e16 (+1.7 %) | 9.295e6 | 9.359e6 (−0.7 %) | 0.3347 | 0.3342 | 0.9356 | 0.9374 |
| 0.25 | 21.13 | 20.83 (+1.5 %) | 2.244e16 | 2.206e16 (+1.7 %) | 7.863e6 | 7.922e6 (−0.8 %) | 0.4817 | 0.4797 | 0.9868 | 0.9874 |
| 0.30 | 12.21 | 12.04 (+1.4 %) | 1.109e16 | 1.094e16 (+1.4 %) | 6.715e6 | 6.775e6 (−0.9 %) | 0.6113 | 0.6077 | 0.9983 | 0.9986 |
| 0.40 | 3.942 | 3.922 (+0.5 %) | 2.696e15 | 2.701e15 (−0.2 %) | 5.056e6 | 5.121e6 (−1.3 %) | 0.7934 | 0.7883 | 0.9999 | 1.0000 |
| 0.50 | 1.337 | 1.353 (−1.1 %) | 7.106e14 | 7.279e14 (−2.4 %) | 3.929e6 | 3.993e6 (−1.6 %) | 0.8932 | 0.8884 | 1.0000 | 1.0001 |
| 0.60 | 0.4941 | 0.5105 (−3.2 %) | 2.060e14 | 2.163e14 (−4.8 %) | 3.081e6 | 3.138e6 (−1.8 %) | 0.9460 | 0.9423 | 1.0000 | 1.0000 |
| 0.70 | 0.1991 | 0.2118 (−6.0 %) | 6.250e13 | 6.696e13 (−6.7 %) | 2.320e6 | 2.315e6 (+0.2 %) | 0.9744 | 0.9721 | 1.0000 | 1.0000 |
| 0.80 | 0.08637 | 0.09293 (−7.1 %) | 1.609e13 | 1.722e13 (−6.5 %) | 1.383e6 | 1.356e6 (+2.0 %) | 0.9905 | 0.9894 | 1.0000 | 1.0000 |
| 0.90 | 0.02471 | 0.02639 (−6.3 %) | 2.001e12 | 2.113e12 (−5.3 %) | 6.061e5 | 5.918e5 (+2.4 %) | 0.9984 | 0.9979 | 1.0000 | 1.0000 |
| 0.95 | 0.007442 | 0.00781 (−4.7 %) | 2.709e11 | 2.778e11 (−2.5 %) | 2.766e5 | 2.673e5 (+3.5 %) | 0.9997 | 0.9994 | 1.0000 | 1.0000 |

Density, pressure and temperature agree within 7.1 % from the centre to
0.95 R; enclosed mass within 0.6 %. At 0.25 R the fusion share is 0.987
(B23: 0.987): it passes 0.99 near 0.26 R (B23: 0.256 R), and 0.998 at
0.3 R. A test that wants 99 % of the power inside 0.25 R fails on both
models; 98 % at 0.25 R, or 99 % at 0.3 R, holds for both.

## `stellar-low-mass.tsv`

Below 0.3 M☉ the YREC runs fail (rescaled from the 0.3 M☉ start model,
they end in NaN). Two kinds of rows:

- **Baraffe rows**, `mass_msun age_gyr teff log_l r_rsun`, solar
  metallicity, from Baraffe, Homeier, Allard & Chabrier 2015 (A&A 577,
  A42, file `BHAC15_tracks+structure`) for 0.08, 0.09, 0.1, 0.11, 0.13,
  0.15, 0.17, 0.2, 0.3 and 0.4 M☉. For each mass, the tabulated rows
  nearest 0.05, 0.07, 0.1, 0.15, 0.2, 0.3, 0.5, 0.7, 1, 1.5, 2, 3, 5, 7
  and 10 Gyr, the last being the end of the track (10.0 Gyr): 15 rows per
  mass, values as tabulated (BHAC15 uses L☉ = 3.839 × 10³³ erg s⁻¹ and
  R☉ = 6.96 × 10¹⁰ cm). Beyond the last tabulated age a reader holds the
  last row: these stars change little after 10 Gyr. The file has no
  licence; only these derived numbers are committed, with the citation.
  0.3 and 0.4 M☉ are there for the seam with YREC, blended over
  0.30-0.35 M☉.
- **Dartmouth rows**, `D mass_msun feh dlog_teff dlog_l dlog_r`: the
  metallicity offset of log T_eff, log L and log R, at [Fe/H] −1, −0.5
  and +0.5 minus [Fe/H] 0, at 5 Gyr (linear in age between the track's
  rows), for 0.15, 0.2, 0.25 and 0.3 M☉. Source: Dartmouth tracks
  (Dotter et al. 2008, ApJS 178, 89), [α/Fe] = 0, files
  `m0NNfeh{m10,m05,p00,p05}afep0.trk` of the Kiauhoku grids (Claytor et
  al. 2020; Zenodo 14908017, CC-BY 4.0). A reader adds the offset to the
  Baraffe values, interpolated in mass and held at its 0.15 M☉ value
  below.

ZAMS of these stars (`Definition:`): the first Baraffe age at which
|Δ log R / Δ log t| < 0.01 between consecutive rows.

Central densities of BHAC15 at 5 Gyr (log-linear in age between rows),
for the polytrope that models these fully convective stars:

| M (M☉) | 0.08 | 0.09 | 0.1 | 0.11 | 0.13 | 0.15 | 0.17 | 0.2 | 0.3 | 0.4 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ρ_c (g cm⁻³) | 637.8 | 486.5 | 401.0 | 342.9 | 267.4 | 220.2 | 187.4 | 152.8 | 90.3 | 71.0 |

## The seam at 0.3 M☉

[Fe/H] 0. YREC from the track table, Baraffe from its rows (both linear
in age between rows), Dartmouth from `m030fehp00afep0.trk`:

| Mass, age | YREC T_eff, log L, R | Baraffe | Dartmouth |
| --- | --- | --- | --- |
| 0.3 M☉, 1 Gyr | 3475 K, −1.955, 0.2907 R☉ | 3415 K, −1.986, 0.2910 R☉ | 3362 K, −2.005, 0.2934 R☉ |
| 0.3 M☉, 5 Gyr | 3479 K, −1.927, 0.2992 R☉ | 3416 K, −1.961, 0.2990 R☉ | 3363 K, −1.985, 0.3003 R☉ |
| 0.4 M☉, 1 Gyr | 3602 K, −1.686, 0.3688 R☉ | 3520 K, −1.719, 0.3719 R☉ | |
| 0.4 M☉, 5 Gyr | 3604 K, −1.672, 0.3741 R☉ | 3521 K, −1.706, 0.3770 R☉ | |

The radii agree within 1 %. YREC is 60-83 K (1.8-2.3 %) hotter than
Baraffe and 0.03 dex brighter, and about 115 K hotter than Dartmouth: its
grey atmosphere, where Baraffe and Dartmouth use model atmospheres. The
blend over 0.30-0.35 M☉ spreads that step over 0.05 M☉.

## Reading the tables

`science::stellar` (`life`, `main_sequence`) reads the tracks and the
low-mass rows this way:

- **YREC, 0.3-8 M☉**: linear in log mass between the two tracks around
  the mass, at each of the two tabulated [Fe/H] around the metallicity,
  then linear in [Fe/H]; a grid value uses its track alone. log L, log R
  and log T_eff are interpolated, never L, R or T_eff. The star's ZAMS and
  TAMS ages are interpolated the same way, in log t.
- **Along a track**: when every track used reaches the TAMS (last
  xc_frac < 10⁻³), each is read at the star's fractional main-sequence age
  τ = (t − t_ZAMS)/(t_TAMS − t_ZAMS), linear in age between rows. When one
  of them stops at 20 Gyr, all are read at the star's absolute age, each
  held at its first or last row outside its ages. This happens between the
  last mass that stops at 20 Gyr and the next (0.7-0.8 M☉ at [Fe/H] −1 and
  −0.5, 0.8-0.9 at 0, 0.9-1.0 at +0.5).
- **TAMS age**: at each bracketing [Fe/H], when both tracks around the mass
  reach the TAMS, linear in log t against log mass. When only the heavier
  one does (at t_u), log t is extrapolated against log mass from it and the
  next heavier track that reaches the TAMS, and kept at least t_u; when
  neither does, there is none. Then linear in log t against [Fe/H]. A star
  whose TAMS comes out above 20 Gyr, or is missing at either metallicity,
  lives `Beyond(20 Gyr)`. So the lifetime falls continuously with mass: at
  [Fe/H] −1, 19.3 Gyr at 0.72 M☉, 13.6 Gyr at 0.79 M☉, 12.9 Gyr at
  0.8 M☉ (the table), `Beyond` from 0.71 M☉ down. In those pairs a star
  past the heavier track's TAMS reads that track's last row (after the
  hook): at [Fe/H] −1, 0.79 M☉ at 13 Gyr gets R = 1.13 R☉, 6225 K.
- **Baraffe, below 0.35 M☉**: bilinear in log mass and log age, held at
  the last row (10 Gyr) beyond it, plus the Dartmouth offset in dex, linear
  in mass between 0.15 and 0.3 M☉ (held outside) and linear in [Fe/H]
  (zero at 0). The ZAMS is the `Definition:` above, interpolated in log t;
  the life is `Beyond(20 Gyr)`.
- **Seam**: from 0.30 to 0.35 M☉, the two above are blended in log, the
  weight of YREC linear in mass from 0 to 1; the ZAMS age likewise, in
  log t.
- **Ages**: a star is answered from its ZAMS to its TAMS, or to 20 Gyr when
  its life is `Beyond`; outside, `BeforeZams` or `AfterTams` (which carries
  the 20 Gyr end in the second case).

`profile` reads the profiles and builds the lightest stars this way:

- **YREC profiles, from 0.30 M☉**: at each profile mass and tabulated
  [Fe/H] around the star, xc_frac is read from the tracks with the clock
  of `main_sequence` (τ or absolute age; linear in log mass between the
  two tracks around a profile mass that has no track, 2.1, 2.3 and
  2.5 M☉). The two snapshots around that xc_frac are interpolated linearly
  in it, the first or last snapshot held outside them. Then linear in log
  mass between the two profile masses around the star, then in [Fe/H].
  Each step goes sample by sample at the same r/R: log ρ, log P, log T,
  m/M and the fusion share linear, so masses and fusion share stay
  non-decreasing.
- **Layers**: at each step the two layer lists are aligned rank by rank. A
  list without the other's convective core gets an empty one at the
  centre; the shorter list gets empty layers at the photosphere,
  alternating, up to the other's length. Bounds (r and m) are linear. A
  layer empty in the nearer profile (weight below or above 0.5) is
  dropped and its neighbours merge, so a layer present in one profile only
  (the transient core of 1 M☉ near the ZAMS, a thin envelope zone of a hot
  star) is kept while that profile is the nearer one. A sample takes the
  transport of the layer that holds it.
- **Below 0.30 M☉**: the n = 1.5 polytrope, integrated by RK4 at a fixed
  step of 10⁻³ in ξ (ξ₁ = 3.653754, ρ_c/ρ̄ = 5.990705; Chandrasekhar 1939:
  3.65375, 5.99071), scaled to the star's mass and to the radius of
  `main_sequence`: ρ_c = ρ̄ ξ₁/(3|θ'₁|), P_c = G M²/(4π (n + 1) θ'₁² R⁴),
  T_c = μ m_u P_c/(k ρ_c) with μ = 0.617, T = T_c θ held at T_eff at the
  photosphere; ρ and P fall to 0 there. One convective layer. Fusion share
  from ε ∝ ρ T⁴ (pp chain near 4 × 10⁶ K, Kippenhahn et al. 2012
  §18.5.1), dL ∝ θ⁷ ξ² dξ. At 0.08 M☉ and 5 Gyr ρ_c is 696 g cm⁻³, 9 %
  above BHAC15 (637.8); at 0.29 M☉, 99 g cm⁻³ and 7.6 × 10⁶ K against
  YREC's 96 g cm⁻³ and 7.5 × 10⁶ K at 0.30 M☉. The switch is sharp: the
  0.30-0.35 M☉ blend of `main_sequence` does not apply to the interior.

## Features of the models

- The YREC 0.3 M☉ snapshots are fully convective (one layer). At
  0.4 M☉ the snapshots have a convective core of 0.18-0.27 R inside a
  radiative zone, except at [Fe/H] 0 at 20 Gyr; the 0.30-0.35 M☉ blend
  crosses that change.
- Near the ZAMS some snapshots carry a small transient convective core
  (1 M☉ at [Fe/H] 0: to 0.11 R at 0.114 Gyr, gone at xc_frac 0.9), from
  burning not yet in equilibrium. It is in the models; a reader that
  interpolates in xc_frac sees it only near xc_frac 1.
