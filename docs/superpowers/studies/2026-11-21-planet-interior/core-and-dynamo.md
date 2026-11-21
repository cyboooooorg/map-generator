<!-- markdownlint-disable-next-line MD013 -->
# Literature study: core thermal evolution and dynamo (rocky planets, 0.05-20 Earth masses)

Date: 2026-11-21. Rule followed: a number appears only if it was read in a
source text during this session. Reading
status is stated per source: FULL TEXT (PDF text extracted and read), ABSTRACT
(abstract only, via Europe PMC or a
publisher record), SECONDARY (quoted by another paper or a search snippet, not
checked in the primary source).
"not found" means the value was not in anything I could read.

Caveat on extraction: PDF text extraction dropped minus signs in some exponents
(for example `10 6 m2 sec i` in
Stevenson et al. 1983 Table I, `κ 106` in Driscoll and Barnes Table 1). Where a
sign was lost, I restored it only
when physical consistency makes it unambiguous, and I flag it as "(sign
restored)". Everything flagged should be
checked against the typeset table before it goes into code.

## 0. Sources actually read

| Short name | Reference | DOI / identifier | Status |
| --- | --- | --- | --- |
| SSS83 | Stevenson, Spohn, Schubert 1983, Icarus 54, 466-489, "Magnetism and thermal evolution of the terrestrial planets" | 10.1016/0019-1035(83)90241-5 | FULL TEXT (Glasgow copy of the paper) |
| D&B15 | Driscoll, Barnes 2015, Astrobiology 15(9), 739-760 | 10.1089/ast.2015.1325; arXiv:1509.07452 | FULL TEXT (arXiv, Appendix B and Table 1) |
| Chr09 | Christensen, Holzwarth, Reiners 2009, Nature 457, 167-169 | 10.1038/nature07626 | FULL TEXT |
| Chr10 | Christensen 2010, Space Sci. Rev. 152, 565-590, "Dynamo scaling laws and applications to the planets" | 10.1007/s11214-009-9553-2 | FULL TEXT |
| Gai10 | Gaidos, Conrad, Manga, Hernlund 2010, ApJ 718, 596-609 | arXiv:1005.3523 (journal DOI not read) | FULL TEXT |
| Tac11 | Tachinami, Senshu, Ida 2011, ApJ 726, 70 | 10.1088/0004-637X/726/2/70; arXiv:1010.5317 | FULL TEXT |
| Zul13 | Zuluaga, Bustamante, Cuartas, Hoyos 2013, ApJ 770, 23 | arXiv:1304.2909 (journal DOI not read) | FULL TEXT |
| vSum13 | van Summeren, Gaidos, Conrad 2013, JGR Planets 118, 938-951 | arXiv:1304.2437 | FULL TEXT (abstract and structure read, not every table) |
| Lab15 | Labrosse 2015, Phys. Earth Planet. Inter. 247, 36-55 | 10.1016/j.pepi.2015.02.002 | FULL TEXT (HAL), Table 2 read |
| Dav15 | Davies 2015, Phys. Earth Planet. Inter. 247, 65-79 | not read | FULL TEXT (accepted version, White Rose) |
| And11 | Anderson et al. 2011, Science 333, 1859-1862 | 10.1126/science.1211001 | ABSTRACT only (Europe PMC, PMID 21960627). Paper itself is paywalled. |
| Mit20 | Mittelholz, Johnson, Feinberg, Langlais, Phillips 2020, Science Advances 6, eaba0513 | 10.1126/sciadv.aba0513 | FULL TEXT (Europe PMC PMC7195189) |
| IGRF13 | IGRF-13 coefficient file, NOAA NCEI | <https://www.ngdc.noaa.gov/IAGA/vmod/coeffs/igrf13coeffs.txt> | FULL FILE read (data, not the paper) |
| W&H17 | Wicht, Heyner 2017 (arXiv:1701.05060), "Mercury's magnetic field in the MESSENGER era" | arXiv:1701.05060 | FULL TEXT (review; journal not confirmed) |

Primary sources I could not obtain: Nimmo 2015 Treatise on Geophysics chapter
(paywalled), Lay, Hernlund, Buffett 2008
Nature Geoscience 1, 25 (the only copy found returned HTTP 404), Lillis et al.
2013 JGR Planets 118, 1488 (abstract
only seen via a search snippet), Driscoll and Olson 2011 Icarus 213, 12 (no text
found), Stamenkovic et al. 2012
ApJ 748, 41 (title, authors, DOI and a paraphrase of the abstract only).

## 1. Stevenson, Spohn and Schubert 1983 (SSS83) and Driscoll and Barnes 2015

<!-- markdownlint-disable-next-line MD013 -->
### 1.1 SSS83 Table I ("Parameter values for core evolution models of the terrestrial planets", p. 472)

Values adopted for all planets (Earth values). Quoted from the text extraction.

| Parameter | Value in SSS83 | Unit | Notes |
| --- | --- | --- | --- |
| α, mantle thermal expansion | 2 x 10^-5 | K^-1 | Table I |
| k, mantle thermal conductivity | 4.0 | W m^-1 K^-1 | Table I. This verifies the legacy 4 W/m/K. |
| κ, mantle thermal diffusivity | 10^-6 (sign restored) | m^2 s^-1 | Table I |
| ρm cm, mantle density x heat capacity | 4.0 x 10^6 | J m^-3 K^-1 | Table I |
| ρm cm / ρc cc | 1 | - | Table I. Core heat capacity per volume equals the mantle's. |
| Q0, initial radiogenic heating | 1.7 x 10^-7 (sign restored) | W m^-3 | Check: with λ below, Q0 exp(-λ t_Earth) = 2.4e-8, matches the text's 2.5e-8 W m^-3 at present |
| λ, mean decay constant | 1.38 x 10^-17 (sign restored) | s^-1 | Table I |
| A, viscosity activation parameter | 5.2 x 10^4 | K | Table I |
| ν0, reference kinematic viscosity | 4.0 x 10^3 | m^2 s^-1 | Table I (extraction shows `4.0 x I0 ~`, exponent as I read it) |
| Ra_cr, critical Rayleigh number (mantle) | 5.0 x 10^2 | - | Table I |
| β, Nu-Ra exponent | 0.3 | - | Table I. Text says see Schubert et al. 1979a for discussion. |
| Local boundary-layer critical Rayleigh number at the CMB | 2 x 10^3 | - | Eq. (20) in the text, from Booker and Stengel 1978, p. 471-472 |

Other SSS83 values read in the text:

| Item | Value | Where |
| --- | --- | --- |
| Core heat capacity cc | not given as a number. Table I only fixes ρm cm = 4.0e6 J m^-3 K^-1 and ρm cm / ρc cc = 1. With the Earth core density 1.3e4 kg m^-3 (Table III text) this implies cc = 4.0e6 / 1.3e4 = 308 J kg^-1 K^-1 (my arithmetic, derived, not a quoted value). | Table I, Sec. IV "Earth" |
| Legacy "core heat capacity 800 J/kg/K" | not found in SSS83 | - |
| Core thermal conductivity used for the dynamo criterion | 40 W m^-1 K^-1 | Sec. IV, before Table III |
| Core heat flux conducted along the adiabat F_cond | 10 to 20 mW m^-2; nominal 15 mW m^-2 for Earth | Sec. IV "Earth" |
| Dynamo criterion | thermal convection: core heat flux must exceed F_cond. Chemical convection: energy release per area from inner-core growth must exceed -eps F_cond, with Carnot efficiency eps about 0.1; "readily satisfied if the inner core is growing". | Sec. IV "Earth" |
| Earth core density ρc | 1.3 x 10^4 kg m^-3 | Table III text |
| Earth outer core radius | 3485 km | Table III text |
| Initial light-element (sulfur) fraction x0 | 0.1 Earth; 1e-3 to 0.1 Venus; 0.1 to 0.25 Mars; 0.01 to 0.05 Mercury | Sec. III |
| Latent heat plus gravitational energy L+EG | 2.5e5 to 2e6 J kg^-1 range explored (extraction shows `2.5 x 10^-~`, exponent unreadable; read from context as 10^5) | Sec. III |
| Initial upper-mantle and CMB temperatures Tu0 and Tc0 (Eq. 24) | not found. Eq. (24) names them but I found no numeric value in the text. The legacy "2800 K initial excess temperature" has no support in SSS83 as far as I could read. | Eq. 22-25 |
| Surface temperature Ts | Mercury 1073, Venus 730, Earth 293, Mars 1073 K | Table II |
| Planet radius Rp | Mercury 2440, Venus 6051, Earth 6371, Mars 3389 km | Table II |
| Earth mantle and core adiabat scalings ηm, ηc | 1.30 and 1.20 | Table II / Sec. III |
| Earth models (Table III): time of inner-core onset | 2.3 to 3.0 Gyr after start (6 models); present Ri 1185 to 1234 km; present CMB heat flux 17 to 24 mW m^-2; present surface heat flux 62.5 to 64 mW m^-2 | Table III |

Conclusion for item 1 (SSS83): the legacy 4 W/m/K mantle conductivity is
verified (Table I). The legacy 800 J/kg/K core
heat capacity is not in SSS83. The legacy Nusselt numbers by regime (50, 10, 5)
are not SSS83 either: SSS83 uses
a boundary-layer scaling with exponent β = 0.3 and Ra_cr = 500.

<!-- markdownlint-disable-next-line MD013 -->
### 1.2 Driscoll and Barnes 2015, Table 1 ("Model constants", pp. 55-57 of the arXiv version)

Reference column of the table did not survive extraction, so the original source
of each constant is not given here.
Earth-calibrated model; many values are the Earth's.

| Symbol | Value | Unit | Meaning |
| --- | --- | --- | --- |
| cm | 1265 | J kg^-1 K^-1 | mantle specific heat |
| cc | 840 | J kg^-1 K^-1 | core specific heat |
| α | 3 x 10^-5 | K^-1 | mantle thermal expansivity |
| αc | 1 x 10^-5 | K^-1 | core thermal expansivity |
| kUM, kLM | 4.2, 10 | W m^-1 K^-1 | upper and lower mantle thermal conductivity |
| κ | 10^-6 (sign restored) | m^2 s^-1 | mantle thermal diffusivity |
| ρm, ρc, ρic | 4800, 11900, 13000 | kg m^-3 | mantle, core, inner core density |
| Mm, Mc | 4.06e24, 1.95e24 | kg | mantle and core mass |
| Rc, R | 3480, 6371 | km | core and planet radius |
| Rac | 660 | - | critical Rayleigh number |
| β | 1/3 | - | convective cooling exponent, Eq. (25) |
| γc | 1.3 | - | core Grueneisen parameter |
| LFe, EG | 750 kJ kg^-1, 3e5 J kg^-1 | - | latent heat and gravitational energy at the inner-core boundary |
| γdip (γd) | 0.2 | - | dipole saturation constant in Eq. (35) |
| σc | 10 x 10^5 | S m^-1 | core electrical conductivity (extraction; exponent as read) |
| Qrad,0, τrad | 60 TW, 2.94 Gyr | - | mantle radiogenic heat and decay time |
| τrad,c | 1.2 Gyr | - | core radioactive decay time |
| Tm(0), Tc(0) | 2400 K, 6000 K | K | initial mantle and core temperature, Sec. 4 (p. 12-13). "Results independent of initial mantle and core temperatures up to approximately +/-500 K." Note: initial Tc - Tm = 3600 K, versus the legacy 2800 K. |

D&B15 core dynamo law (Appendix B.4, Eq. 35, Olson and Christensen 2006 form): M
= 4π R^3 c γd sqrt(ρ / 2µ0) (Fc Dc)^(1/3),
with Dc = Rc - Ric the dynamo shell thickness, Fc the core buoyancy flux
(thermal plus compositional, Eq. 36-37),
Fth = αc gc ρc cc q_c,conv / ... (Eq. 37 as read; the denominator was lost in
extraction). Their text states a positive
buoyancy flux implies dynamo action, and that the law "likely provides an upper
limit on the dynamo lifetime".
Earth's present magnetic moment quoted in their Figure 4 caption: about 80
ZAm^2.

## 2. Sourced values

### 2.1 Core specific heat

| Value (J kg^-1 K^-1) | Source | Where |
| --- | --- | --- |
| 750 | Labrosse 2015, PEPI 247, 36, doi 10.1016/j.pepi.2015.02.002 | Table 2, "Heat capacity CP" (footnote e, Gubbins et al. 2003) |
| 715 / 840 / 715 | Davies 2015 PEPI 247, 65, columns "this study" / "Nimmo (2014)" / "Pozzo et al. (2012)" | Table 1 |
| 840 | Driscoll and Barnes 2015 | Table 1 (cc) |
| 840 | Christensen et al. 2009 Nature 457, 167 | p. 168, for Earth's core ("cp = 840 in SI units", ref 12 = Nimmo, Treatise on Geophysics vol. 8) |
| 840 (outer core) | Tachinami et al. 2011 | Table 4 (citing Yukutake 2000) |
| 850 (Fe), 1250 (MgSiO3-pv) | Gaidos et al. 2010 | Table 2 (Fe from Wang et al. 2002; pv from Akaogi and Ito 1993) |
| 850 (outer core), 1250 (lower mantle and upper mantle) | Zuluaga et al. 2013 | Table 1 |
| about 308 (derived) | SSS83 | see 1.1 |

Range found: 715 to 850. The legacy 800 sits inside the range but has no direct
citation; 840 (Nimmo 2015 via Chr09 and
D&B15) or 750 (Labrosse 2015) can be cited. I did not read Nimmo's chapter
itself.

### 2.2 Mantle thermal conductivity

| Value (W m^-1 K^-1) | Source | Where |
| --- | --- | --- |
| 4.0 (average mantle) | SSS83 | Table I |
| 4.2 upper mantle, 10 lower mantle | D&B15 | Table 1 |
| 5 upper mantle, 10 lower mantle | Tachinami et al. 2011 | Table 4 (Yukutake 2000) |
| about 6 (MgSiO3 perovskite) | Gaidos et al. 2010 | Table 2 (Goncharov et al. 2009) |
| 6 (km, lower and upper mantle entry) | Zuluaga et al. 2013 | Table 1 |

Core thermal conductivity: 40 (SSS83 text; Zuluaga Table 1; Tachinami Table 4),
30 to 100 (Gaidos Table 2, Stacey and
Loper 2007), 163 at the centre with depth gradient (Labrosse 2015 Table 2, from
Gomi et al. 2013). Dav15 quotes
Pozzo et al. 2012: 63 W/m/K gives 9 TW conducted down the adiabat; 100 W/m/K
gives 15 TW.

### 2.3 CMB heat flow of Earth today

| Value | Source | Status |
| --- | --- | --- |
| 13.25 TW "isentropic value" (heat conducted down the core adiabat, high conductivity) | Labrosse 2015 abstract | FULL TEXT |
| 7 to 17 TW, as used by Davies from "Lay et al. (2009) and Nimmo (2014)" | Davies 2015, p. 3 | FULL TEXT of Davies (note he writes 2009; Lay et al. is 2008 in Nature Geoscience, I could not open it) |
| 9 TW (k = 63) and 15 TW (k = 100) conducted down the adiabat | Davies 2015 quoting Pozzo et al. 2012 | FULL TEXT of Davies |
| 5 to 15 TW | Lay, Hernlund, Buffett 2008 Nature Geosci. 1, 25 | SECONDARY (search snippet of the paper) |
| 12 +/- 5 TW attributed to Nimmo 2015 | Li et al. 2020 arXiv:2010.05423 (not peer-checked) | SECONDARY |
| 13 to 17 TW (Driscoll and Bercovici 2014, PEPI 236, 36) | lab web page summary | SECONDARY |
| 15 TW (Frost et al. 2022, multidisciplinary constraints) | search snippet | SECONDARY |
| Earth models of SSS83: 17 to 24 mW m^-2 at CMB (about 8 to 11.5 TW over 4π(3485 km)^2, my arithmetic) | SSS83 Table III | FULL TEXT (derived conversion mine) |

### 2.4 Earth inner-core age

| Value | Source | Status |
| --- | --- | --- |
| less than 700 Myr if CMB heat flow exceeds the isentropic value | Labrosse 2015, abstract | FULL TEXT |
| 600 Myr or less (all superadiabatic models, high conductivity), "about two times younger" than low-conductivity estimates; about 1.5 Gyr if the top of the core is strongly subadiabatic | Davies 2015, abstract | FULL TEXT |
| about 300 Myr (Pozzo et al. 2012, as quoted) | Davies 2015 p. 3 | SECONDARY inside a read paper |
| 0.5 to 1.0 Ga (Driscoll and Bercovici 2014, with 13 to 17 TW) | lab page summary | SECONDARY |
| 0.66 Ga at 12 TW (pure Fe) ; 1.91 Ga for 6 TW (Buffett et al. 1996); 1.00 +/- 0.50 Ga for 3 TW (Labrosse et al. 2001) | Li et al. 2020 arXiv:2010.05423 | SECONDARY |
| SSS83 Earth models: onset 2.3 to 3.0 Gyr after start, i.e. about 1.5 to 2.2 Gyr before present if the start is 4.5 Gyr | SSS83 Table III | FULL TEXT (conversion mine), low-conductivity era |
| Tachinami nominal Earth: inner core appears at 2.2 Gyr, radius 1200 km at 4.5 Gyr | Tac11 Sec. 3.1 | FULL TEXT |
| vSum13 Earth model: inner-core formation at about 4.1 Gyr (so about 0.4 Gyr ago), field lasts about 8 Gyr | vSum13 abstract and Sec. 3 | FULL TEXT |

Spread: 0.3 to about 2 Gyr depending on core conductivity and CMB heat flow. No
single consensus age is claimed.

### 2.5 Mars dynamo cessation

| Value | Source | Status |
| --- | --- | --- |
| "ceased by about 4.13 Ga" (early termination scenario "b", based on crater magnetic signatures) | Mit20 Fig. 1 caption, citing Lillis et al. 2013 JGR Planets 118, 1488 and Vervelidou et al. 2017 JGR Planets 122, 2294 | FULL TEXT of Mit20 |
| 4.12 to 4.14 Ga absolute model age (N(300) retention age 2.5 to 3.2) | Vervelidou et al. 2017 abstract | SECONDARY (aggregator, not the journal) |
| dynamo present before and after the Hellas, Utopia, Argyre and Isidis basins; core field at 4.5 Ga and 3.7 Ga | Mit20 abstract (counter-claim: long-lived dynamo) | FULL TEXT |
| Tachinami's Mars constraint: Martian crust of about 4 Gyr retains paleomagnetic field, so lifetime "may be longer than 0.5 Gyr" | Tac11 Appendix B | FULL TEXT |

The 4.1 Ga value is therefore one of two competing scenarios (Lillis 2013,
Vervelidou 2017 versus Mittelholz 2020).

### 2.6 Mercury

Anderson et al. 2011 Science 333, 1859 (abstract, Europe PMC):
"southward-directed, spin-aligned, offset dipole centered
on the spin axis", offset 484 +/- 11 km northward, axis tilt less than 3 degrees
from the rotation axis, best-fit planetary
moment 195 +/- 10 nT R_M^3 (R_M is Mercury's mean radius). Wicht and Heyner 2017
report, from Anderson et al. 2012,
an axial dipole of 190 nT R_M^3 offset 480 km north. The 195 nT R_M^3 figure
confirms the legacy intent: Mercury's
equatorial dipole field at the surface is of order 0.2 µT, roughly 0.0065 of
Earth's (my division of 195 nT by 29.8 µT).
I did not read the paper body (tables, field at the surface including the
quadrupole).

### 2.7 Earth dipole at the surface and at the CMB

| Quantity | Value | Source |
| --- | --- | --- |
| g10, g11, h11 for 2020 | -29404.8, -1450.9, 4652.5 nT | IGRF13 file, rows `g 1 0`, `g 1 1`, `h 1 1`, column 2020.0 |
| Equatorial dipole field at the reference radius (6371.2 km), B0 = sqrt(g10^2 + g11^2 + h11^2) | 29.8 µT | computed from the three coefficients above (my arithmetic). Supports the legacy "Earth about 30 µT". |
| Dipole field at the CMB, rms over the core surface | about 259 µT, i.e. the sqrt(2) x B0 / (3480/6371.2)^3 (my arithmetic) | consistent with Christensen et al. 2009 "dipole field strength at the dynamo surface 0.26 mT at Earth" (p. 168, ref 17 = Bloxham and Jackson) |
| (Rc/R)^3 | 0.163 (my arithmetic with 3480 and 6371.2 km) | - |

Magnetic moment: D&B15 says about 80 ZAm^2 (Fig. 4 caption).

## 3. Christensen scaling (replacement of the calibrated "dipole share")

### 3.1 Exact form (Chr09 Eq. 1-3; Chr10 Eq. 11, 14, 15)

- Local form: B^2 / (2 µ0) ∝ f_ohm ρ^(1/3) (q_c L / H_T)^(2/3) (Chr09 Eq. 1;
  Chr10 Eq. 11).
  Definitions: f_ohm ≤ 1 is the ratio of ohmic to total dissipation; L is the
  length scale of the largest convective
  structures (the shell thickness D = R - ri in the geodynamo); H_T = cp/(α g)
  is the temperature scale height.
- Volume-averaged form: `<B>`^2 / (2 µ0) = c f_ohm <ρ>^(1/3) (F q_o)^(2/3) (Chr09
  Eq. 2; Chr10 Eq. 15).
  q_o is a reference flux (the bolometric flux at the outer boundary for stars;
  for Earth's core q_o = q_i,c (ri/R)^2,
  the effective flux at the inner boundary, convective plus compositional), F is
  the efficiency factor of Eq. 3 (Chr10 Eq. 14).
- Constant of proportionality: c = 0.63 for dipole-dominated dynamos (Chr09 p.
  168, "provides the constant of
  proportionality as c = 0.63"; Chr10 Fig. 2 caption "best-fitting pre-factor is
  c = 0.63", best-fit exponent 0.677);
  c = 0.23 for multipolar dynamos (dipolarity below 0.35; Chr10 Sec. 3.4, Fig.
  7).
- Efficiency factor for Earth's core with ri/R = 0.35 (Chr09 p. 168): F = 0.88 α
  g_o R / cp (constant total flux) or
  0.45 α g_o R / cp (flux zero at the outer boundary). With α = 1.35e-5, g_o =
  10.7, R = 3.48e6, cp = 840 (SI), F = 0.52 or 0.27
  (read from the paper). Chr10 p. 583 repeats "For Earth F = 0.27-0.52 and for
  Jupiter F = 1.1".
- Applicability: rapidly rotating regime (field independent of rotation rate and
  diffusivities, exponents forced to 1/3 and
  2/3 by dimensional analysis, Chr09 p. 168). Also needs the convected part of
  the flux only (Chr10 p. 582: conducted
  fraction along the adiabat must be removed, and compositional buoyancy
  converted to an effective heat flux).
  Chr10 Eq. 7 and 32 (velocity): U = 1.05 (F q_o / ρ)^(2/5) (Ω Rc)^(-1/5).
- Dipolarity criterion: local Rossby number Ro_l = 0.11 (F q*)^(1/2) E^(-1/3)
  Pr^(1/5) Pm^(-1/5) (Chr10 Eq. 28);
  dipole-dominated below about 0.12 (Chr10 Sec. 3.3), reversing dynamos 0.06 to
  0.12.

Comparison with the legacy form B^2/2µ0 = 0.63 ρ^(1/3) q^(2/3): the legacy drops
f_ohm (at most 1) and the efficiency
factor F, and uses the total CMB heat flux rather than the convected,
superadiabatic part. With Earth's F = 0.27 to 0.52,
the effective prefactor c F^(2/3) is about 0.26 to 0.41 (my arithmetic: 0.63 x
0.27^(2/3) and 0.63 x 0.52^(2/3)) before
the f_ohm factor. Tachinami et al. 2011 Eq. 21 writes the same law as B_c ~ 0.5
µ0^(1/2) ρ^(1/6) F_conv^(1/3), with
F_conv = F_CMB - F_cond. Zuluaga et al. 2013 Eq. 28 uses B_rms = C µ0^(1/2)
ρ^(1/6) (D/V)^(1/3) Q_conv^(1/3) with C = 0.24
(dipolar) and 0.18 (multipolar).

### 3.2 Ratio of dipole to rms field (replaces "dipole share 0.067")

| Quantity | Value | Source |
| --- | --- | --- |
| B_rms (inside the dynamo) / B_dip on the outer boundary of the dynamo, dipolar dynamos with fixed ΔT | 3 to 10 | Chr10 Sec. 3.6, from Christensen and Aubert 2006 |
| same ratio "of around seven" for dynamos with an Earth-like magnetic power spectrum | 7 | Chr09 p. 168 ("B/B_dip of around seven"), used to infer the Earth internal field from the 0.26 mT dipole |
| B_rms (internal) / total field at the dynamo surface | about 3.5 | Chr10 Sec. 3.6; Chr09 |
| Larger ratios for different thermal boundary conditions or a stably stratified top layer | not quantified | Chr10 Sec. 3.6 |
| Dipolarity, ratio of mean dipole field at the dynamo surface to the combined field in degrees 1 to 12 | 0.65 for the present geomagnetic field (POMME); likely geodynamo range 0.45-0.75; transition to multipolar at Ro_l = 0.12 | Chr10 Sec. 3.3 and Fig. 4 |
| Surface dipole from CMB dipole | B_dip,p = B_dip,c (Rc/Rp)^3 | Gai10 Eq. 5; Zul13 Eq. 32; Tac11 |
| f_dip^max (upper bound on dipolarity fraction) | 1.0 for dipolar dynamos, 0.35 for multipolar, soft-step in Ro_l about 0.1 | Zul13 Sec. 4 |
| B_dip,c ≲ f_dip^max^(11/10) B_rms / c_bdip, with c_bdip ≈ 2.5 (so about 0.4 B_rms for a dipolar dynamo) | | Zul13 Eq. 30-31 (from Zuluaga and Cuartas 2012, not read) |

Note: Chr10 does not define a symbol called "f_dip". The dipolarity there is the
ratio defined above; the names
f_dip and b_dip come from Zuluaga et al. 2013 (and Zuluaga and Cuartas 2012).
The ratio between Zuluaga's 0.4 upper bound
(1/2.5) and Christensen's 1/7 = 0.14 is a factor of about 2.8; Zuluaga's is
explicitly an upper bound.

Arithmetic with sourced inputs, for comparison with the legacy share 0.067
(derived by me, not published):
(i) rms dipole at the CMB equals B_rms / 7 (Chr09), and the equatorial dipole at
the surface is then
B_rms x (1/7) x (Rc/R)^3 / sqrt(2) when the dipole is pure (the sqrt(2) converts
the sphere rms to the equatorial value, from the
dipole geometry |B|^2 ∝ 1 + 3 cos^2θ whose sphere mean is 2). That gives an
effective "share" of 0.143 / 1.414 = about 0.10 times
(Rc/R)^3; with a surface-rms convention, 0.14 times (Rc/R)^3. The legacy 0.067
is therefore about 0.5 to 0.65 times
the Christensen-based value for Earth. Check this arithmetic before use.
Earth consistency check with the sourced numbers: 0.26 mT x 7 = 1.8 mT internal
rms (Chr09's own inference), surface equatorial
29.8 µT, ratio 0.0165 = 0.163 x 0.101 (matches the formula above to rounding).

## 4. Simple parameterised criteria for dynamo onset and cessation

All equations below are copied from the text I read; symbols as in the papers.

### 4.1 SSS83 (thermal criterion)

Dynamo requires convection in a fluid core. Thermal: core heat flux F_c greater
than the flux conducted along the core adiabat F_cond
(10 to 20 mW m^-2 for Earth, nominal 15). Compositional: inner-core growth
energy per area greater than eps F_cond (eps about 0.1).
Tachinami et al. 2011 Eq. 17 gives the same threshold explicitly: F_crit = k_c
(∂T/∂r)_S = k_c ρ g γ_G T_CMB / K_S.
Lifetime = period during which the core heat flux exceeds F_crit.

### 4.2 Gaidos et al. 2010 (entropy, Reynolds number, inner-core and "snow" criteria)

- Dynamo allowed when dissipation Φ > 0 (entropy balance); field in the core B_c
  = a1 sqrt(µ0 ρ) [φ T (Rc - Ri)]^(1/3)
  with a1 about 1.65 and exponent 1/3 (Aubert et al. 2009); Eq. 2 as read in the
  text (square-root factor garbled in extraction).
- Magnetic Reynolds number Rm = V L / λ greater than 40, λ = 1/(µ0 σ);
  "convective motions as small as 1e-4 m/s suffice for Rm > 40 in Earth-size
  cores" (λ about 2 m^2 s^-1, L about 3e6 m).
  Eq. 4: Rm ≈ a2' (Rc - Ri)/λ [φ T (Rc - Ri)]^(1/3); order-of-magnitude result
  Rm about 16 (B_c / 1 µT), so
  B_c of a few µT (surface at least 1 µT) gives Rm greater than 40, "only very
  weakly dependent on planet mass".
- Surface field: B_p ≈ B_c (Rc/Rp)^3 (Eq. 5).
- Inner-core formation timescale (Eq. 47): t_c ≈ M_c ΔS / (8π k Rc) (1 -
  D^2/Rc^2 ln(τ0(Fe)/τc(pv))); for k = 40, Rc ∝ M_p^(1/4) the first factor is
  about 2 M_p^(3/4) Gyr (units: M_p in Earth masses).
- Complete freezing time after inner-core start (Eq. 48): t_s ≈ c_p ΔS / (2 k α
  G), about 50 Gyr for standard values.
- Inner-core versus "iron snow" criterion: Δ = ratio of solidus gradient to
  adiabat gradient; Δ > 1 inner core forms; Δ < 1 iron snows at the top,
  stratifies the core, no dynamo. Result: inner cores only for planets below
  about 2.5 Earth masses (below 2 for stagnant lids).
- Cessation: after the CMB heat flow falls to the core conduction value, φ = 0
  and the dynamo ceases (Sec. 5.1).

### 4.3 Tachinami, Senshu and Ida 2011

Lifetime = period during which F_CMB > F_crit (Eq. 17). Critical planet mass of
maximum lifetime/flux: Mp,c ≃ (ΔT_CMB / 1000 K) M_earth
(Eq. 18, empirical) and Mp,c ∼ (ΔT_CMB / 1000 K) (V\* / 10^-5 m^3 mol^-1)^-1
M_earth (Eq. 22, V\* = activation volume; the V\* unit in
the text extraction is `10 x 10^-6 m3 mol^-1`). Lifetime scaling for Mp < Mp,c:
τ_life ∼ E_th / (S_CMB F_CMB) ∝ M_p^(1/2) (from
S_CMB ∝ M_p^(1/2), F_CMB ∝ M_p, E_th ∝ M_p^2 at fixed T_CMB, T_CMB ∼ 5 ΔT_CMB).
Surface field B_surf = B_c (rc/rp)^3 with B_c ∼ 0.5 µ0^(1/2) ρ^(1/6)
F_conv^(1/3) (Eq. 21), F_conv = F_CMB - F_cond.
Nominal ΔT_CMB = 1000 K; nominal Earth lifetime 12 Gyr (Sec. 3.1).

### 4.4 Zuluaga et al. 2013 (extends Gaidos 2010 and Tachinami 2011)

Dynamo lifetime t_dyn is the time at which Φ (entropy balance Φ = (E_l + E_s +
E_g - E_k)/T) becomes negative, i.e. Q_conv = Φ T_c ≤ 0
(Eq. 18 and Sec. 3.2). Inner-core growth dRic/dt = - D_c^2 / (2 Ric (Δ - 1))
(1/Tc) dTc/dt (Eq. 17). Dipole moment from the Christensen-type
scalings in 3.1 and 3.2. Initial core temperature: Tc(0) = Tl + ε_adb ΔT_adb
with ε_adb = 0.7 (Eq. 25), potential temperature 1700 K.
Dynamo reaches the end when the core cannot cool (no entropy); inner core forms
before the shutdown for Mp < M_crit = 2.0 Earth masses (Fig. 2 caption).

### 4.5 Driscoll and Barnes 2015

Dynamo exists while the core buoyancy flux is positive (Eq. 35-37); a fully
solid core kills it. Their model predicts, for Earth-mass
planets with a mobile lid, that "all models have a large solid inner core after
10 Gyr" and that dynamos last the full 10 Gyr except
for tidally super-cooled models whose core solidifies completely around 9.8 Gyr.
This is Earth-mass only; no mass scan in the paper.

### 4.6 van Summeren, Gaidos, Conrad 2013 (abstract)

Earth-mass planets with plate-like tectonics: field lasting about 8 Gyr;
elevated surface temperature or pressure-dependent viscosity: about 8 to 9.5
Gyr;
stagnant lid: about 10.5 Gyr, dynamo may cease for several Gyr and restart with
inner-core growth. Sensitive to the initial core temperature, size and solidus.

## 5. Scaling of thermal-evolution quantities with planet mass (super-Earths)

| Result | Source | Status |
| --- | --- | --- |
| Inner core forms only for M < about 2.5 Earth masses (below 2 for stagnant lids); 2-3 M_earth: iron snow shuts the dynamo within 10 Gyr; above 3 M_earth no freezing in 10 Gyr, weak core-cooling dynamo with mobile lid lasting "several Gyr" depending on mass and surface temperature | Gai10 abstract, Sec. 4.2 and 5.1 | FULL TEXT |
| Surface field strength at fixed age decreases with planet mass; dynamo duration decreases with mass for M > 4 M_earth and increases with surface temperature (Fig. 8 bottom right gives lifetime for 4 to 10 M_earth, figure only) | Gai10 Sec. 4.2, Fig. 8 | numbers not extractable (figure) |
| Venus-like stagnant-lid massive planets: no dynamo; for stagnant lid, planets above about 1.5 M_earth do not produce dynamos | Gai10 Sec. 4.2 | FULL TEXT |
| Inner-core time t_c scales slightly faster than M^(3/4); a 2 M_earth planet takes more than twice as long as 1 M_earth | Gai10 Sec. 5.1 | FULL TEXT |
| Lifetime weakly increases as M^(1/2) for M below Mp,c; critical mass about ΔT_CMB/1000 K Earth masses (about 1 to 5 M_earth for ΔT_CMB 1000 to 5000 K); lifetime declines above Mp,c because of pressure-dependent viscosity; dynamo lifetime for 0.1 to 10 M_earth in Fig. 10 (figure only) | Tac11 abstract, Sec. 3.3-3.4, Eq. 18 and 22 | FULL TEXT (numeric lifetimes in Fig. 10 not extractable) |
| Field strength B ∝ M_p^(1/3) at fixed ΔT_CMB (from B ∝ F^(1/3) and F_CMB ∝ M_p) | Tac11 Sec. 3.5 | FULL TEXT |
| Tachinami earlier IAU result: 0.1 to 1.4 M_earth planets sustain the field for 4.5 Gyr | Tachinami et al. IAU Symp. 249 (2008) | SECONDARY (search snippet) |
| Planets of 0.5-6 M_earth: inner core before shutdown when M < 2.0 M_earth; lifetimes in Fig. 2 (figure only); HD 40307g (about 7 M_earth) predicted t_dyn about 4 Gyr | Zul13 Fig. 2 caption, Sec. 5 | FULL TEXT, numbers for other masses not in text |
| Pressure-dependent viscosity makes the usual thermostat regulation fail for massive planets; may influence magnetic field generation | Stamenkovic et al. 2012 ApJ 748, 41, doi 10.1088/0004-637X/748/1/41 | SECONDARY (OSTI record paraphrase) |
| Core radius scaling Rc ∝ M_p^(1/4) used in Gai10 Eq. 47 (their Fig. 2) | Gai10 | FULL TEXT |

No source I read gives a closed-form "dynamo lifetime vs mass" law valid from
0.05 to 20 M_earth. The published results are numerical
curves (Gai10 Fig. 8, Tac11 Fig. 10, Zul13 Fig. 2) with sign-changing mass
dependence: lifetime first increasing with M, then collapsing at a critical mass
of
order 1 to 5 M_earth that depends on the initial core-mantle temperature
contrast, on the activation volume, and on tectonic regime.
Below about 0.1 Earth masses: no published value found in the sources read
(Tac11 covers 0.1 to 10, Zul13 0.5 to 6, Gai10 1 to 10 M_earth).
Mercury and Mars (about 0.055 and 0.107 M_earth) are treated in SSS83 and Tac11
Appendix B; their conditions (ζ ratios, sulfur) are not tabulated here.

## 6. Legacy constants versus literature

| Legacy value | Status |
| --- | --- |
| Core heat capacity 800 J/kg/K | no direct source. Literature range read: 715 to 850 (Dav15 715; Lab15 750; Gai10 850; D&B15, Tac11, Nimmo 2015 via Chr09 840). |
| Mantle conductivity 4 W/m/K, "SSS83" | verified: SSS83 Table I, k = 4.0. Others: 4.2 (D&B15 upper mantle), 5 (Tac11), about 6 (Gai10, Zul13). |
| Initial excess temperature 2800 K | not in SSS83. D&B15: Tc(0) - Tm(0) = 3600 K (6000 vs 2400 K); Tac11: ΔT_CMB 1000 K nominal; Zul13: 0.7 x (Tm - Tl). |
| Nusselt number by regime (50, 10, 5) | not found in any source read. Literature uses Nu ∝ Ra^β with β = 0.3 (SSS83) or 1/3 (D&B15), Ra_cr 500 (SSS83) or 660 (D&B15). Stagnant lid uses a different law (Zul13 Eq. 22 after Nimmo and Stevenson 2000, Q ∝ (ρ g α/(κ η))^(1/3) Γ^(-4/3)). |
| Dynamo end 0.075 τ, inner core start 1.0 τ, frozen 2.25 τ | no source found for these multiples. Gai10 t_c ∝ M^(3/4) and t_s about 50 Gyr give different logic; SSS83 Earth onset 2.3 to 3.0 Gyr. |
| Christensen flux scaling, c = 0.63 | c = 0.63 verified (Chr09, Chr10), but it applies to F q_o with the efficiency factor F (0.27 to 0.52 for Earth) and f_ohm, and q must be the convected flux (core heat flux minus conduction along the adiabat). |
| Dipole share 0.067 (Earth about 30 µT), 3e-4 (Mercury) | Earth 29.8 µT verified (IGRF-13). The share should be derived: Chr09 gives B_rms/B_dip = 7 at the CMB (range 3 to 10, Chr10); times (Rc/R)^3. Mercury 195 nT R_M^3 verified (And11 abstract); the dynamo is not in Christensen's fast-rotation regime for Mercury (Chr09 p. 168, Chr10 p. 583), so a flux scaling is not validated there. |
| Rotation: all dipole fields | Chr10: dipolar only for Ro_l below about 0.12 (and multipolar prefactor c = 0.23); Zul13 gives a soft step in Ro_l around 0.1 for f_dip^max (1.0 to 0.35). |

## 7. Not verified, to flag

1. SSS83 Table I and II exponent signs (extraction dropped minus signs); numbers
   cross-checked only for Q0 and λ. Initial temperatures Tu0, Tc0: not found.
2. SSS83 core heat capacity: only the ratio ρm cm / ρc cc = 1; the 308 J/kg/K
   figure is my derivation.
3. Driscoll and Barnes Table 1 reference column not recovered; κ and σc
   exponents as read.
4. CMB heat flow: Lay et al. 2008, Nimmo 2015, Frost et al. 2022 and Driscoll
   and Bercovici 2014 values are secondary only.
5. Inner-core age ranges: only Labrosse 2015 and Davies 2015 abstracts and
   statements read; Nimmo 2015 and Buffett, Labrosse values secondary.
6. Mars: Lillis 2013 abstract not read; Vervelidou 2017 numbers from an
   aggregator; the 4.13 Ga figure is read in Mittelholz 2020's caption.
7. Mercury: only the Anderson et al. 2011 abstract; surface field value with
   quadrupole not read.
8. IGRF paper DOI not verified (data file only). The 29.8 µT is my arithmetic
   from the file.
9. Dynamo lifetime versus mass: numeric curves are in figures (Gai10 Fig. 8,
   Tac11 Fig. 10, Zul13 Fig. 2) and could not be read from the text; Zuluaga and
   Cuartas 2012 (Icarus 217, 88) and Driscoll and Olson 2011 (Icarus 213, 12)
   were not read.
10. The dipole-share arithmetic in Sec. 3.2 and the Nu or time-multiple
    comparisons in Sec. 6 are my derivations from sourced numbers and need
    review.
11. The Nimmo 2015 Treatise chapter's page range appears as 27-55 in my search
    and 31-65 in the Li et al. reference list; the exact range was not
    confirmed.
12. DOIs of Gai10, Zul13 and vSum13 were not read; I give the arXiv numbers
    instead.
