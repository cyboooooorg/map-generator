# How much ozone does an oxygen air hold, and what light does it take?

Seen from: the Bond albedo and the outgoing heat of a rocky planet whose
air holds oxygen (the `Oxidising` and `Modern` biospheres of planet PR
2d-4). Nothing of this page is in the app yet: it is the study behind
issues #653 (ozone, O₂ and aerosols in the shortwave) and #704 (ozone in
the longwave), [Wiki study 34](https://github.com/cyboooooorg/map-generator/wiki/34-ozone-oxygen-aerosols-audit).

## Short answer

Sunlight below 242 nm splits O₂ molecules; the freed atoms join other
O₂ molecules into ozone, O₃, mostly 15 to 35 km up. Earth holds about
300 Dobson units of it, a layer 3 mm thick if brought to the ground.
Ozone swallows nearly all the Sun's light below 300 nm and a few per cent
of its visible light, about 11 W/m² of Earth's 340, and it adds a band
of absorption at 9.6 µm in the infrared. How much ozone a planet holds
depends on its oxygen, but not in proportion: a hundredth of Earth's
oxygen still gives a fifth to a half of its ozone. It also depends on the
star: a hotter star, with more ultraviolet, makes more. O₂ itself
absorbs a little red and near-infrared light (its A and B bands), about
1 % of the sunlight. Dust, sea salt and sulphate droplets (aerosols)
reflect about 2 % more of the clear-sky light and absorb 1.4 %.

## The science

### Where ozone comes from

Ozone is made by the Chapman cycle: an O₂ molecule split by ultraviolet
light gives two oxygen atoms, each of which joins an O₂ molecule with a
third body to carry the energy away (O + O₂ + M → O₃ + M); ozone is split
again by ultraviolet and destroyed by atoms and by catalytic cycles of
hydrogen, nitrogen and chlorine oxides. The layer sits where the light
that splits O₂ meets enough air for the three-body reaction. When O₂
falls, that light reaches deeper, where the air is denser and the
reaction faster, so the layer moves down and its column falls more
slowly than the O₂ (Kozakis et al. 2022, §4.1).

**Earth's column and profile.** NASA's Ozone Watch puts Earth's ozone
at "roughly 300 Dobson Units" and 90 % of it in the stratosphere; Kim &
Ramanathan (2008, Table 2) take 300 ± 30 DU as the input of their
global run. One Dobson unit is $2.687\times10^{16}$ molecules per cm²
(a layer 10 µm thick at 273.15 K and 1013.25 hPa; Cooke et al. 2022,
§3.1). The AFGL profiles (Anderson et al. 1986), integrated here in
libRadtran's files, give:

| Profile | Column | Above 100 hPa | Densest at | Ozone-weighted temperature |
| --- | --- | --- | --- | --- |
| U.S. Standard 1976 (`afglus.dat`) | 346 DU | 79 % | 22 km, 40 hPa | 225 K |
| Mid-latitude summer (`afglms.dat`) | 336 DU | 78 % | 25 km, 28 hPa | 232 K |
| Tropical (`afglt.dat`) | 284 DU | 88 % | 25 km, 26 hPa | 230 K |

The mixing ratio peaks higher, near 35 km and 4 to 9 hPa in the same
files (WMO 2014, chapter 2: about 5 hPa). For the column's optics, the
number density matters: ozone lies mostly between 10 and 100 hPa, above
80 to 90 % of the air's mass, at 225 to 232 K.

### Ozone against oxygen

The published models disagree by a factor of about two at low oxygen,
and only for the Sun:

| O₂ (PAL) | Cooke et al. 2022 (3-D, WACCM6) | Kozakis et al. 2022 (1-D, Sun) | Rugheimer et al. 2015b (1-D, Sun) |
| --- | --- | --- | --- |
| 1 | 279 DU | 5.15e18 cm⁻² (192 DU) | 5.29e18 cm⁻² (197 DU) |
| 0.25 | | 5.63e18 (210 DU, its maximum) | |
| 0.1 | 169 DU | 5.36e18 (199 DU) | 4.23e18 (157 DU) |
| 0.01 | 66 DU | 2.90e18 (108 DU) | 6.01e17 (22 DU) |
| 0.001 | 18 DU | 6.5e17 (24 DU) | |

Notes on the columns:

- **Cooke et al. 2022** (§3.1, p. 9; runs at 150, 50, 5 and 0.5 % PAL
  are plotted only, Fig. 6) is a three-dimensional model whose
  preindustrial run recovers the observed column "in both magnitude and
  latitudinal variation". It finds 1-D and older 3-D models 1.46
  (Way et al. 2017), 1.57 (Segura et al. 2003), 1.76 (Kasting & Donahue
  1980) and 2.43 (Levine et al. 1979) times its column at 10 % PAL, and
  1.83 to 2.89 times at 1 % PAL. It rises monotonically with O₂; the
  older 1-D models peak between 10 % and 100 % PAL.
- **Kozakis et al. 2022** give a fifth-degree polynomial of the column
  in $\log_{10}$ of the O₂ *mixing ratio* (not PAL), per host, valid
  from 0.01 % to 150 % PAL (Table 3); the column values above are that
  fit, evaluated by the study and checked against their Table 2 (the
  maxima) and their G0V values in the text. Their Sun at present O₂, 192
  DU, is two thirds of the observed 300.
- **Rugheimer et al. 2015b** (Table 1) change O₂ together with the
  epoch's other gases (1 % PAL at 2.0 Ga, 10 % at 0.8 Ga), so their
  column is not a function of O₂ alone.
- **Gregory et al. 2021** (§3, Fig. 1a) find the column rising with a
  fixed ground O₂ and levelling off above a mixing ratio of 1e-2 (about
  5 % PAL), and, when O₂ is set by its fluxes rather than fixed, few
  stable states between a low-ozone and a high-ozone one (§4.5, Fig. 6);
  they tabulate no column.
- Segura et al. 2003 and Kasting & Donahue 1980 were not reached; their
  columns are known here only through Cooke et al.'s ratios.

### Ozone against the star

The ultraviolet that makes ozone (below 242 nm) and the ultraviolet that
destroys it (200 to 320 nm) come in different shares from different
stars, so the column at a given O₂ depends on the host.

At present O₂, for a planet given the Sun's total light (Rugheimer et
al. 2013, Table 2; Atmos 1-D model; DU converted here):

| Host | F0V | F2V | F5V | F7V | G0V | Sun | G2V | G8V | K0V | K2V | K4V | K5V | K7V |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| $T_\text{eff}$ (K) | 7000 | 6750 | 6500 | 6250 | 6000 | 5800 | 5750 | 5500 | 5250 | 5000 | 4750 | 4500 | 4250 |
| O₃ (DU) | 447 | 409 | 357 | 309 | 272 | 197 | 190 | 119 | 153 | 123 | 97 | 97 | 130 |
| Ratio to the Sun | 2.27 | 2.08 | 1.81 | 1.57 | 1.38 | 1 | 0.96 | 0.60 | 0.78 | 0.62 | 0.49 | 0.49 | 0.66 |

The grid is not monotonic below the G stars. For M dwarfs the column
depends on the star's activity more than on its temperature: Segura et
al. (2005, Table 2) find 164 DU around the active AD Leo (3400 K) and
484 DU around GJ 643, but 45 to 119 DU around quiescent model M dwarfs of
3100 to 3650 K (Sun: 313 DU in the same model); Rugheimer et al. (2015a,
Table 3) find 2.3e18 to 4.1e18 cm⁻² around active M0 to M9 dwarfs, and
1.7e18 down to 1.1e16 around inactive ones. Kozakis et al. (2022, Table
2) find that a K5V or M5V host gives the most ozone at the most O₂ (150
% PAL), while the Sun and a G0V give theirs at 25 % PAL.

No published formula gives the column against the star's ultraviolet.

### What ozone absorbs

**The shortwave.** Ozone's absorption is a smooth cross-section, not
lines: the Hartley band (200 to 310 nm), the Huggins bands (310 to 350
nm), and the weak, broad Chappuis band (450 to 750 nm). In the
measurements of Serdyuchenko et al. (2014), at 213 to 1100 nm and 193
to 293 K (data file of IUP Bremen's molecular spectroscopy lab, read by
the study):

| Wavelength | 223 K | 293 K |
| --- | --- | --- |
| Hartley peak, 255.3 nm | 1.165e-17 cm² | 1.151e-17 cm² |
| Huggins, 320 nm | 2.70e-20 cm² | 3.20e-20 cm² |
| Chappuis peak, 602.5 nm | 5.16e-21 cm² | 5.10e-21 cm² |

Under 300 DU ($8.06\times10^{18}$ cm⁻²) the Hartley band's vertical depth
is about 94: no light below 290 nm reaches the ground. The Chappuis
band's depth is 0.042: it takes 4 % of the orange light on a vertical
pass, 13 % on the way down and up again along the diffuse path
($D$ = 1.66). The Hartley maximum moves by 0.5 to 1 % and the Chappuis
band by under 1 % from 193 to 293 K; the Huggins band, by up to 70 %
(Serdyuchenko et al. 2014, p. 626). The stated uncertainty is 1.2 to
2.6 % systematic and under 1 to 2.2 % statistical (their Table 2), up to
30 % in the weak absorption of 350 to 450 nm and past 900 nm. HITRAN
holds no ozone line above about 7000 cm⁻¹ (1.4 µm): its ultraviolet and
visible bands exist only as cross-sections. The MPI-Mainz atlas
(Keller-Rudek et al. 2013) lists Serdyuchenko et al.'s set beside
Bogumil et al. (2003) and the JPL recommendations to 2010; the JPL
evaluation 19-5 (Burkholder et al. 2019) was not reached.

**The longwave.** Ozone's ν₃ and ν₁ bands, at 1042 and 1103 cm⁻¹,
make the 9.6 µm band in the window where water absorbs least; its ν₂
band lies at 14 µm under CO₂'s, and its rotation lines below 300 cm⁻¹.
HITRAN lists them for five isotopologues (global ids 16 to 20: 666, 668,
686, 667, 676): 73 494 lines from 950 to 1200 cm⁻¹, 32 299 from 600 to
800, 110 714 below 300 and 478 384 below 6000 cm⁻¹ (counts of the
line-by-line API, 2026-11-25). Removing ozone raises Earth's outgoing
longwave by 4.2 W/m² under all skies (Schmidt et al. 2010, Table 1:
2.7 % of the 155 W/m² greenhouse) and by 8 W/m² clear, 10 with its
overlaps, on one profile (Kiehl & Trenberth 1997, Table 3). The band
absorbs the light of the warm troposphere and sends out its own at the
temperature of the ozone layer, 225 to 232 K (the profiles above): a
colder layer would make the greenhouse larger. At 1042 cm⁻¹ a black body
at 214 K emits 0.167 of what one at 288 K does, at 225 K 0.232 and at
232 K 0.284 (computed here).

### O₂'s own bands

O₂ absorbs sunlight in weak, magnetic-dipole bands: the A band at 762
nm, the B band at 688 nm, the γ band at 628 nm and the a¹Δ band at
1.27 µm. HITRAN lists them for three isotopologues (global ids 36 to 38:
66, 68, 67): 470 lines in 12 900-13 300 cm⁻¹ (A), 356 in 14 300-14 700
(B, ending near 14 558 cm⁻¹), 163 in 15 600-16 200 (γ) and 1169 in
7700-8100 cm⁻¹ (counts of the API). Pairs of colliding molecules add
broad bands (collision-induced absorption): HITRAN2024's `O2-O2_2024.cia`
covers 1150 to 33 670 cm⁻¹ (the 1.27 µm, A and B bands and the double
transitions at 630, 577 and 477 nm and bluer), `O2-N2_2024.cia` and
`O2-air_2024.cia` 1300 to 13 840 cm⁻¹ (CIA readme 2024; Karman et al.
2019).

On Earth, removing O₂ from a line-by-line clear, aerosol-free model
(with its and N₂'s collision bands) changes the shortwave by +1.25 W/m²
at the top and −4.3 at the surface, of a daytime mean of 680.5 W/m²
(Hogan & Matricardi 2020, Table 4): O₂ absorbs 5.55 W/m² there, 0.82 %,
or 2.8 W/m² of 340.25. Its longwave part is 0.096 W/m². Zender (1999,
abstract) puts the collision pairs alone at 0.75 to 1.2 W/m² of the
global mean, 1 to 2 % of the air's shortwave absorption. Kiehl &
Trenberth (1997, Table 4) give 2 W/m². No source read splits the
absorption between the A band, the B band and the collision pairs.

### Aerosols

Earth's air carries a background of small particles: a mean optical
depth of 0.12 at 550 nm, with a single-scattering albedo of 0.94 (Kinne
2019, MACv2, abstract and Table 2). Its components (Table 1, all sources
together): sulphate 0.037, dust 0.031, sea salt 0.028, organics 0.022 and
soot 0.012. A quarter to a third of the optical depth is anthropogenic
(§4: 0.031 to 0.040); the industrial era added 0.02 to 0.06 (Bellouin
et al. 2020, §4). Under clear skies, Kim & Ramanathan (2008, Table 3b)
find the aerosols reflect 5.9 W/m² more at the top and absorb 4.8 in the
air, of 341.8; under all skies their effect at the top falls to 3.0
W/m², the clouds hiding the aerosols below them. Yu et al. (2006,
abstract) measure −5.5 ± 0.2 W/m² over the oceans and −4.9 ± 0.7 over
land at the top under clear skies.

On another planet each kind has its own source: sea salt from breaking
waves over salt water, dust from bare dry land, sulphate from volcanic
and biological sulphur. ROCKE-3D carries schemes for all three and calls
them "ubiquitous", but writes that volcanic SO₂ sources elsewhere are
"virtually unknown" (Way et al. 2017, §4.6). Boutle et al. (2020) let
dust rise from bare land in the UK Met Office model and find its effect
"very sensitive to the fraction of the surface covered by land". The
sea-salt source functions are fitted to Earth's winds and seas (Jaeglé
et al. 2011). The one-column climate models of the habitable zone carry
no aerosol: Kopparapu et al. (2013, §2) fold Earth's clouds into a
surface albedo raised to reproduce Earth.

### What we do not know

- **The column against the star.** Only one grid of stars (Rugheimer
  et al. 2013) gives the column at present O₂, from one 1-D model whose
  Sun gives 197 DU, two thirds of the observed; nothing gives it against
  both O₂ and the star, except Kozakis et al.'s five hosts.
- **The column at low O₂**, where 3-D and 1-D models differ by a factor
  of two to three (Cooke et al. 2022).
- **M dwarfs**, whose ultraviolet depends on their activity, which the
  app does not draw for the climate.
- **A background aerosol on another planet**: no law ties its optical
  depth to an ocean, a desert or a volcanism a generator knows.

## In the app

### What it does now

Nothing of this page. The shortwave absorbs in water vapour and CO₂
only, from 0.7 µm redward (`shortwave.bin`); the longwave holds water
and CO₂; O₂ enters only through its Rayleigh scattering and as a
broadener. `CLOUD_ALBEDO` (0.276) is fitted on Earth's Bond albedo with
the absorbers the column has, and so absorbs what it lacks. See
[radiative-transfer.md](radiative-transfer.md#where-it-departs-from-the-science).

### What the study measured

A scratch probe put a pure ozone absorber above the app's shortwave
column (Serdyuchenko et al.'s 223 K cross-sections, averaged per band as
$-\ln\langle e^{-D\tau}\rangle/D$ with $D$ = 1.66), on Earth's air at 77 %
over 288 K, a ground of 0.122 and the law's cloud cover of 0.682, under
the app's 5772 K black body and under the measured Sun (TSIS-1,
Coddington et al. 2021). Measured, 340.25 W/m²:

| Sun | Ozone | Clear-sky albedo | Clear-sky air absorbs | All-sky Bond albedo | `CLOUD_ALBEDO` for 0.293 |
| --- | --- | --- | --- | --- | --- |
| Black body | none | 0.1701 | 48.41 W/m² | 0.2931 | 0.2758 |
| Black body | 300 DU | 0.1412 | 66.96 | 0.2597 | 0.3509 |
| TSIS-1 | none | 0.1554 | 51.06 | 0.2810 | 0.3015 |
| TSIS-1 | 300 DU | 0.1409 | 61.90 | 0.2626 | 0.3425 |

- **The black body doubles ozone's share.** It puts 3.0 % of the light
  below 300 nm where the Sun puts 1.0 % (see radiative-transfer.md), so
  ozone takes 18.6 W/m² under it and 10.8 under the measured Sun.
  Kiehl & Trenberth (1997, Table 4) give 13.9 at 340.25 with older
  cross-sections. Ozone has to come with the measured spectrum (#655),
  or after it.
- **The column matters little.** Under TSIS-1, 150, 270, 330 and 450 DU
  give 7.8, 10.3, 11.4 and 13.5 W/m² of clear-sky absorption; even 3 DU
  takes 1.8 W/m², the Hartley band saturating first. All-sky, halving
  300 DU raises the Bond albedo by 0.0046.
- **Where it sits matters less.** Mixing it into the top 10 % of the
  air's mass instead of above it moves the albedo by 0.0003.
- **A refit hides it on Earth.** Refitted to 0.293 over the ground of
  0.122, as `CLOUD_ALBEDO` is now, the cloud albedo with ozone gives the
  climate's Earth, whose cover makes its ground 0.1445, a Bond albedo of
  0.3025 (black body) to 0.3029 (TSIS-1), against 0.3041 without ozone:
  the cold bias of the reference Earth does not come from the missing
  absorbers but from the two grounds. Without a refit, ozone alone takes
  that Earth from 0.3042 to 0.2703 (black body), 11.5 W/m² more absorbed.
- **The star matters most.** With 300 DU, the black-body host's all-sky
  albedo falls by 0.0033 at 3400 K, 0.0109 at 4500 K, 0.0334 at 5772 K
  and 0.0539 at 6500 K.

### Where it would depart

These are the departures an implementation would carry; none is in the
code yet.

**Decided with the user (2026-11-25).** The measured Sun (#655) lands
before ozone's shortwave; #705 refits `CLOUD_ALBEDO` over the climate's
own ground (0.1445), not 0.122; the column is calibration 1 below; the
`Oxidising` air of planet PR 2d-4 keeps no O₂, so no ozone (an issue
tracks it); the aerosols are a departure (below); the work goes in three
pull requests: ozone's shortwave with the #705 refit, ozone's longwave
with the methane and N₂O of #703 (one rebuild of `opacity.bin`), and
O₂'s bands. The order of the radiation studies' pull requests is 33, 35,
34, 36.

- **The column (calibration 1, decided).** No source gives the column
  against O₂ and the star together for the Sun at the observed 300 DU.
  An oxygen (`Modern`) air holds 300 DU times the ratio of Rugheimer et
  al. 2013's column at the host's temperature to their Sun's,
  interpolated between 4250 and 7000 K; below 4250 K, the ratios of
  Segura et al. 2005's quiescent M dwarfs to their Sun (45, 89 and 119
  of 313 DU at 3100, 3400 and 3650 K); held at the ends of the range.
  An air without O₂ holds none.
- **A pure absorber above the air**, at one temperature (223 K): ozone
  lies above 80 to 90 % of the air's mass at 225 to 232 K. The probe
  bounds the error at 0.0003 of albedo.
- **No ozone heating.** Ozone's absorption warms the stratosphere; the
  column's stratosphere is isothermal at its skin temperature, 214 K for
  Earth, colder than the 225 to 232 K of the ozone layer. Its 9.6 µm
  band would then emit about a quarter to two fifths less than the real
  layer at the band's centre (the Planck ratios above), and overstate
  ozone's longwave greenhouse.
- **O₂ bands outside the table.** `shortwave.bin` starts at 0.7 µm
  (14 364 cm⁻¹): the A band and the 1.27 µm band fall inside it, the B
  and γ bands and the visible collision pairs do not.
- **No background aerosol (a departure, decided 2026-11-25).** Earth's
  aerosols reflect 5.9 W/m² and absorb 4.8 under clear skies (Kim &
  Ramanathan 2008), but no source gives a background aerosol for a
  planet the app draws from what it knows (its oceans, deserts or
  volcanism): the schemes that exist need winds, seas and soils, and
  their sources elsewhere are "virtually unknown" (Way et al. 2017,
  §4.6). The aerosols' effect on Earth stays inside `CLOUD_ALBEDO`.

## Sources

1. Anderson, G. P. et al. 1986, *AFGL Atmospheric Constituent Profiles
   (0-120 km)*, AFGL-TR-86-0110 (read in libRadtran's `afglus.dat`,
   `afglms.dat` and `afglt.dat`, mirror `ashiklom/libradtran` at commit
   `208f5465`; SHA-256 `dab26290…91cfb5`, `875ada62…652b14`,
   `1721b46e…981efe`).
2. Bellouin, N. et al. 2020, *Rev. Geophys.* 58, e2019RG000660,
   doi:10.1029/2019RG000660.
3. Bogumil, K. et al. 2003, *J. Photochem. Photobiol. A* 157, 167,
   doi:10.1016/S1010-6030(03)00062-5 (listed in the MPI-Mainz atlas; not
   read).
4. Boutle, I. A. et al. 2020, *Nat. Commun.* 11, 2731,
   doi:10.1038/s41467-020-16543-8.
5. Burkholder, J. B. et al. 2019, *Chemical Kinetics and Photochemical Data
   for Use in Atmospheric Studies*, JPL Publication 19-5 (not reached).
6. Coddington, O. M. et al. 2021, *Geophys. Res. Lett.* 48, e2020GL091709,
   doi:10.1029/2020GL091709 (TSIS-1 HSRS, `tsis1_hsrs_1nm.csv`).
7. Cooke, G. J. et al. 2022, *R. Soc. Open Sci.* 9, 211165,
   doi:10.1098/rsos.211165, arXiv:2102.11675.
8. Gordon, I. E. et al. 2026, *J. Quant. Spectrosc. Radiat. Transf.* 353,
   109807, doi:10.1016/j.jqsrt.2026.109807 (HITRAN2024; line counts from
   its line-by-line API, <https://hitran.org/lbl/api>, and its CIA files
   and 2024 readme, <https://hitran.org/cia/>, read 2026-11-25).
9. Gorshelev, V. et al. 2014, *Atmos. Meas. Tech.* 7, 609,
   doi:10.5194/amt-7-609-2014.
10. Gregory, B. S., Claire, M. W., Rugheimer, S. 2021, *Earth Planet. Sci.
    Lett.* 561, 116818, doi:10.1016/j.epsl.2021.116818.
11. Hogan, R. J., Matricardi, M. 2020, *Geosci. Model Dev.* 13, 6501,
    doi:10.5194/gmd-13-6501-2020.
12. Jaeglé, L. et al. 2011, *Atmos. Chem. Phys.* 11, 3137,
    doi:10.5194/acp-11-3137-2011.
13. Karman, T. et al. 2019, *Icarus* 328, 160,
    doi:10.1016/j.icarus.2019.02.034 (as the HITRAN CIA readme cites it).
14. Keller-Rudek, H. et al. 2013, *Earth Syst. Sci. Data* 5, 365,
    doi:10.5194/essd-5-365-2013 (the MPI-Mainz UV/VIS Spectral Atlas, its
    O₃ listing read 2026-11-25).
15. Kiehl, J. T., Trenberth, K. E. 1997, *Bull. Am. Meteorol. Soc.* 78,
    197, doi:10.1175/1520-0477(1997)078<0197:EAGMEB>2.0.CO;2 (Tables 3 and
    4, as read for `radiative-transfer.md`).
16. Kim, D., Ramanathan, V. 2008, *J. Geophys. Res.* 113, D02203,
    doi:10.1029/2007JD008434.
17. Kinne, S. 2019, *Tellus B* 71, 1623639,
    doi:10.1080/16000889.2019.1623639 (MACv2).
18. Kopparapu, R. K. et al. 2013, *ApJ* 765, 131,
    doi:10.1088/0004-637X/765/2/131.
19. Kozakis, T., Mendonça, J. M., Buchhave, L. A. 2022, *A&A* 665, A156,
    doi:10.1051/0004-6361/202244164, arXiv:2208.09415.
20. NASA Ozone Watch, "What is a Dobson Unit?" and "Ozone facts",
    <https://ozonewatch.gsfc.nasa.gov/> (read 2026-11-25).
21. Rugheimer, S., Kaltenegger, L., Zsom, A., Segura, A., Sasselov, D.
    2013, *Astrobiology* 13, 251, doi:10.1089/ast.2012.0888,
    arXiv:1212.2638.
22. Rugheimer, S., Kaltenegger, L., Segura, A., Linsky, J., Mohanty, S.
    2015a, *ApJ* 809, 57, doi:10.1088/0004-637X/809/1/57,
    arXiv:1506.07202.
23. Rugheimer, S., Segura, A., Kaltenegger, L., Sasselov, D. 2015b, *ApJ*
    806, 137, doi:10.1088/0004-637X/806/1/137, arXiv:1506.07200.
24. Schmidt, G. A., Ruedy, R. A., Miller, R. L., Lacis, A. A. 2010, *J.
    Geophys. Res.* 115, D20106, doi:10.1029/2010JD014287 (Table 1).
25. Segura, A. et al. 2005, *Astrobiology* 5, 706,
    doi:10.1089/ast.2005.5.706, arXiv:astro-ph/0510224.
26. Serdyuchenko, A., Gorshelev, V., Weber, M., Chehade, W., Burrows, J. P.
    2014, *Atmos. Meas. Tech.* 7, 625, doi:10.5194/amt-7-625-2014 (data:
    `serdyuchenkogorshelev5digits.dat`, IUP Bremen,
    <https://www.iup.uni-bremen.de/gruppen/molspec/databases/referencespectra/o3spectra2011/index.html>,
    SHA-256 `4dfbf021…f0ce7092`; the page states no licence).
27. Way, M. J. et al. 2017, *ApJS* 231, 12, doi:10.3847/1538-4365/aa7a06.
28. WMO 2014, *Scientific Assessment of Ozone Depletion: 2014*, chapter 2.
29. Yu, H. et al. 2006, *Atmos. Chem. Phys.* 6, 613,
    doi:10.5194/acp-6-613-2006.
30. Zender, C. S. 1999, *J. Geophys. Res.* 104, 24471,
    doi:10.1029/1999JD900797 (abstract).

## See also

- [How does a planet's air set its outgoing heat and its albedo?](radiative-transfer.md)
- [Planet atmospheres, climate and surface](planet-atmospheres.md)
