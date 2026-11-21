# Study: the XUV history of a star

Short study before porting the XUV law of `legacy-final`
(`crates/mapgen/src/planet/xuv.rs`) into `science`. The planet model
(escape, methane photolysis) needs, for each star lighting a planet, its
XUV flux today and the XUV energy received over the system's age.

## The legacy law

The X-ray plus extreme-ultraviolet band (1 to 1180 Å) is a share of the
bolometric luminosity:

- `R_sat = 3.71e-4` while the star is saturated, up to `t_sat(M)`;
- `R_sat (t / t_sat)^-1.23` after;
- `t_sat` interpolated in log-log through (0.25 M☉, 2 Gyr),
  (0.5 M☉, 0.2 Gyr), (1 M☉, 0.1 Gyr), extrapolated along the end
  segments, the mass floored at 0.05 M☉;
- the fluence is the integral of that share from 0 to the age (analytic),
  times the present insolation: the luminosity is taken constant;
- a second band, the H I Lyman α line, follows the same shape with
  `R_sat = 7.40e-5` and exponent 0.72;
- the same law for every mass, from M dwarfs to B stars.

## Sources checked

| Claim | Source | Verdict |
| --- | --- | --- |
| `F = 29.7 τ^-1.23` erg s⁻¹ cm⁻² at 1 au, 1-1180 Å | Ribas et al. 2005, ApJ 622, 680, doi:10.1086/427977, Table 5 | Confirmed. Five stars aged 0.1, 0.3, 0.65, 1.6, 6.7 Gyr: before 0.1 Gyr the law is an extrapolation. `R_sat = 29.7e-3 · 0.1^-1.23 / 1361 = 3.71e-4` recomputed. |
| Lyman α `F = 19.2 τ^-0.72` | Ribas et al. 2005, eq. 2 | Confirmed, flagged "preliminary" by the authors (two solar proxies). `7.40e-5` recomputed. |
| Sun's saturation `log τ_sat = 7.84` | Jackson, Davis & Wheatley 2012, MNRAS 422, 2024, Table 2 (not Table 3) | Confirmed for the 0.565-0.675 B−V bin, which holds the Sun (B−V ≈ 0.65): the paper does not name it. `τ_sat` spans 10^7.8-10^8.3 yr over FGK with no clear trend. |
| Saturation until ~200 Myr at 0.5 M☉, ~2 Gyr at 0.25 M☉ | Johnstone et al. 2021, A&A 649, A96, doi:10.1051/0004-6361/202038407 | Differs in kind: the paper says the slow and fast rotator tracks stay "almost identical until 200 Myr and 2 Gyr respectively". That is when the rotation spread opens, not a published `t_sat`. The two anchors are a reading, hence a calibration. |
| X-ray saturation `log R_X = -3.13` | Wright et al. 2011, ApJ 743, 48, sec. III | Confirmed, "almost independent of spectral type", 0.09-1.36 M☉. Johnstone 2021 eq. 17 gives `R_X,sat = 5.1e-4`. |

Sources: arXiv versions (astro-ph/0412253, 1111.0031, 2009.07695,
1109.4634) read through their HTML renderings.

## What the law leaves out

1. **Luminosity history.** The luminosity is constant at its present
   value. `science::stellar::main_sequence` now gives `L(t)` from the
   zero-age main sequence. The young Sun was about 30 % fainter, which
   Ribas et al.'s absolute fluxes already include; a star whose
   luminosity changed differently gets its history scaled by today's
   luminosity. `main_sequence` gives no values before the ZAMS.
2. **Stars without a convective envelope.** Coronal X-rays need a
   convective dynamo. X-ray emission is "not expected" earlier than A7
   (Schröder & Schmitt 2007, read through a secondary source, and its
   companion arXiv:0712.0173); Vega has `L_X/L_bol < 9e-11` (Pease et
   al. 2006, via a secondary source). O and early B stars emit
   `L_X/L_bol ≈ 1e-7` from wind shocks (Berghöfer et al. 1997, read
   through a secondary source). The legacy solar law gives an A0 star of
   40 L☉ a saturated XUV of 1.5e-2 L☉, three to six orders of magnitude
   above the measured shares. Legacy `activity.rs` already cut the dynamo
   at `T_eff ≥ 7200 K` (Gray & Nagel 1989), but `xuv.rs` did not. The
   system draw now gives A and B stars a third of the hosts, so this
   matters. Not addressed by any source read: the photospheric EUV of hot
   B stars, below 912 Å.
3. **F stars.** Jackson 2012 measures the saturated X-ray share of early
   F stars about 10 times below the Sun's bin (`log` −4.28 and −4.24
   against −3.67). That is X-rays alone, not the whole XUV band.
4. **Initial rotation.** At 1 M☉ saturation lasts from about 6 Myr (slow
   rotators, 10th percentile) to about 230 Myr (fast, 90th) (Tu et al.
   2015, A&A 577, L3). The law follows one median track; no source gives
   the spread of the cumulative fluence as a factor.
5. **M dwarfs.** The saturated X-ray share is about flat with spectral
   type (Wright 2011), which supports one `R_sat` for all convective
   stars. The larger Lyman α share of an M dwarf's light (MUSCLES:
   27-75 % of the 1150-3100 Å flux) is not modelled; no source read gives
   it as a share of the bolometric light.

## X-ray to EUV conversions (not used)

If the law were rebuilt from X-ray luminosities, the published
conversions are, with surface fluxes in erg s⁻¹ cm⁻²: Johnstone 2021
eq. 19, `log F_EUV1 = 2.04 + 0.681 log F_X` (10-36 nm), and King et al.
2018 (arXiv:1804.11124), `F_EUV / F_X = α F_X^γ`, γ = −0.493 for
0.2-2.4 keV. Sanz-Forcada et al. 2011 give
`log L_EUV = 4.80 + 0.860 log L_X` (read through Tu 2015, not in the
paper body). Ribas 2005 measures the whole band directly for the Sun,
so the port keeps it.

## Decisions

Taken with the user on 2026-11-21.

- Port the law, its two bands, `integrated_ratio`, `ratio` and
  `age_below_ratio`, with the citations above (the `t_sat` anchors as
  `Calibration:`).
- Gate the coronal XUV on the envelope: no XUV while the star's effective
  temperature is at or above 7200 K (the legacy dynamo cut), read along
  the star's track, since the measured shares of A and B stars are three
  to six orders of magnitude below the solar law.
- Take the luminosity history into account. A first try made the band a
  share of the present luminosity `L(t)`, refitted on the solar track of
  the tables: the Sun then left Ribas et al.'s fit by −12 % at 1.6 Gyr
  and +16 % at 6.7 Gyr, because the measured XUV history is a power law
  in absolute flux while `L(t)` is not. Kept instead: the band is a share
  of the ZAMS luminosity, `L_band(t) = R_sat L_ZAMS (t / t_sat)^-k`. X-ray
  saturation is measured as `L_X/L_bol` while the star is young, where
  `L ≈ L_ZAMS`, and the decay after follows the spin-down. The Sun of the
  tables then follows the fit at every age (`R_sat = 5.220e-4`, the
  legacy 3.706e-4 times `L_now / L_ZAMS` = 0.998 / 0.709), and the
  fluence stays analytic.
- After review: the Lyman α line is chromospheric (photospheric in hot
  stars), so it is not cut at 7200 K; A and B stars keep the solar law,
  a lower bound. One `History` per star serves both bands and every
  planet.
- Leave the F-star decrease, the rotation spread, the M dwarf Lyman α
  share and the photospheric EUV of B stars out, stated in the docs.
