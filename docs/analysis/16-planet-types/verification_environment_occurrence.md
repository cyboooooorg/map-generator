# Verification of the environment and occurrence claims

Checked on 2026-11-15 against primary sources actually opened in this session:
arXiv abstract pages (`arxiv.org/abs/<id>`), the arXiv API, ar5iv full text,
Crossref, and the NASA Exoplanet Archive TAP service (`pscomppars` and `ps`
tables, queried live). Verdicts: confirmed, corrected, not found. Where a
claim rests only on a secondary source, the row says so and the verdict is
"not found" or "partly" rather than confirmed.

The report was not edited. Section "Corrections to apply" lists what to change.

## Corrections to apply (summary)

1. Wrong arXiv ids in the notes (the id points to an unrelated paper):
   TOI-1338 b is `2004.07783` (not 2004.07773); WD 1145+017 is `1510.06387`
   (not 1509.07516); KIC 12557548 b is `1201.2662` (not 1201.2189);
   Leconte et al. 2013 (Nature 504, 268) is `1312.3337` (not 1303.7094);
   Barnes 2017 is `1708.02981` (not 1608.06919). Pierrehumbert 2011 has no
   arXiv id found (1008.3911 is an unrelated sociophysics paper): cite DOI
   `10.1088/2041-8205/726/1/L8`, title "A Palette of Climates for Gliese 581g".
   Laughlin et al. 2009 has no arXiv id found (0812.1583 is an unrelated
   Tevatron paper): cite DOI `10.1038/nature07649`.
2. Dressing and Charbonneau 2015: the 2.5 +/- 0.2 planets per M dwarf is for
   **1-4 R_earth** with P < 200 d, not 0.5-4 R_earth.
3. Petigura 2013: the notes' "about 22 % of Sun-like stars have a 1-2 R_earth
   planet with P of 20-200 d" is wrong. The paper gives 22 +/- 8 % for
   1-2 R_earth planets in the **habitable zone** (flux 0.25-4 F_earth), and
   11 +/- 4 % for Earth-size planets at 1-4 F_earth. The report's "26 % for
   1-2 R_earth, P 5-100 d" was not found in the paper text: do not use it.
4. Cumming 2008: 10.5 % is for 0.3-10 M_Jup and **P = 2-2000 d** (about < 3 au
   around a solar-mass star), not "< 5 au".
5. Fischer and Valenti 2005 versus Johnson 2010: Johnson 2010 itself finds
   f proportional to 10^(1.2 [Fe/H]), not 10^(2 [Fe/H]). The 10^(2.0 [Fe/H])
   form (0.03 x 10^(2.0 [Fe/H]) in secondary sources) is Fischer and Valenti,
   and the exponent was not read in the paper itself.
6. Wright 2012 hot Jupiter rate is 1.2 +/- 0.38 % (not "0.5-1 %"); Petigura
   2018 is 0.57 +0.14/-0.12 %. The two differ by about 1.5 sigma.
7. Transiting circumbinary planets: 14 planets in 12 systems in the NASA
   Exoplanet Archive on 2026-11-15, **12 from Kepler (10 systems) and 2 from
   TESS** (TOI-1338 b, TIC 172900988 b). The report's "~14 transitantes dans
   Kepler" is wrong on the attribution.
8. Mróz 2020 (OGLE-2016-BLG-1928): the lens is "Mars- to Earth-mass, Mars
   favoured", not "about 0.3 Earth masses". Companions are excluded to 8 au;
   the "unbound at 1 sigma" wording is not in the abstract.
9. KIC 12557548 b: fiducial mass is 0.1 M_earth (evaporation time about
   0.2 Gyr), not 0.01 M_earth. Period 15.685 h and depth 0.2-1.3 % are right.
10. Sumi 2023: the two normalisations in the report are the same fit with
    different pivot masses (check: 2.18 x (8/38)^0.96 = 0.49). No conflict.
11. Pulsar planets: the archive lists 7 entries (PSR B1257+12 b, c, d;
    B1620-26 b; J1719-1438 b; J2322-2650 b; B0329+54 b), so "5-6" is low.

## Table

| Claim | Verdict | Correct reference (what was opened) | Corrected number or wording |
|---|---|---|---|
| Holman and Wiegert 1999, stability limit "~2-4 binary separations" | confirmed (reference); wording refined | Holman and Wiegert, "Long-Term Stability of Planets in Binary Systems", AJ 117, 621 (1999), arXiv:astro-ph/9809315, DOI 10.1086/300695 (arxiv.org/abs/astro-ph/9809315, ar5iv full text) | Eq. 3 of the paper: a_c = 1.60 + 5.10e - 2.22e^2 + 4.12mu - 4.27 e mu - 5.09 mu^2 + 4.61 e^2 mu^2 (units of binary a), so about 1.6 a for a circular, low-mass-ratio binary and up to about 4-5 a for e ~ 0.5-0.7. "2-4" is a fair summary, not a paper number. Test particles, 10^4 binary periods. |
| Kepler-16 b (Doyle 2011), 0.33 M_Jup, 0.75 R_Jup, P ~229 d, stars 0.69 + 0.20 M_sun, binary P 41 d | confirmed (abstract level) | Doyle et al., Science 333, 1602 (2011), arXiv:1109.3432, DOI 10.1126/science.1210923 (arxiv.org/abs/1109.3432) | Abstract: Saturn-like mass and size, 229 d, stars 20 % and 69 % of the Sun, 41 d eccentric binary. The 0.33 and 0.75 values are not in the abstract; NASA archive gives 105.8 M_earth (0.33 M_Jup) and 8.45 R_earth (0.75 R_Jup), consistent. |
| Kepler-34 b, 35 b (Welsh 2012) | confirmed | Welsh et al., Nature 481, 475 (2012), arXiv:1204.3955 (API record) | NASA archive: 34 b P = 288.8 d, 35 b P = 131.5 d. |
| Kepler-47 b, c (Orosz 2012) | confirmed | Orosz et al., "Kepler-47: A Transiting Circumbinary Multi-Planet System", arXiv:1208.5489 (API record; Science 337, 1511 not shown in the record) | Archive also lists Kepler-47 d (2019, P = 187 d): the system has 3 planets. |
| Kepler-1647 b, ~1.5 M_Jup, P ~1100 d | confirmed | Kostov et al. 2016, ApJ 827, 86, arXiv:1512.00189 (arxiv.org/abs/1512.00189) | 1.52 +/- 0.65 M_Jup, R = 1.06 R_Jup, P ~1100 d; longest-period transiting CBP at the time. |
| TOI-1338 b, Kostov 2020, arXiv "2004.07773?" | corrected | Kostov et al., "TOI-1338: TESS' First Transiting Circumbinary Planet", AJ 159, 253 (2020), arXiv:**2004.07783**, DOI 10.3847/1538-3881/ab8a48 (arxiv.org/abs/2004.07783) | id ends 783. Planet 6.9 R_earth, P = 95.2 d, binary 1.1 + 0.3 M_sun, 14.6 d. |
| "~14 transiting circumbinary planets in Kepler" | corrected | NASA Exoplanet Archive TAP query `ps`, `cb_flag=1`, `discoverymethod='Transit'`, `default_flag=1`, run 2026-11-15 | 14 planets, 12 systems: Kepler-16 b, 34 b, 35 b, 38 b, 47 b/c/d, 413 b, 453 b, 1647 b, 1661 b, PH1 b (12 Kepler planets, 10 systems), plus TOI-1338 b and TIC 172900988 b (TESS). Kepler-1660 AB b is eclipse timing, not transit. |
| Armstrong 2014, circumbinary occurrence "~10 % of eclipsing binaries" | corrected (wording) | Armstrong et al., "On the Abundance of Circumbinary Planets", MNRAS 444, 1873 (2014), arXiv:1404.5617, DOI 10.1093/mnras/stu1570 (arxiv.org/abs/1404.5617) | 10.0 +18/-6.5 % (95 % limits) for R_p > 6 R_earth and P_p < 300 d, **if** planets are coplanar with the binary; lower limit 47 % if inclinations are isotropic. Based on the 7 systems known then. |
| Kraus 2016, planets ~3 times less frequent in binaries below ~47 au | confirmed | Kraus et al., "The Impact of Stellar Multiplicity on Planetary Systems, I.", AJ 152, 8 (2016), arXiv:1604.05744 (arxiv.org/abs/1604.05744) | Inside a_cut = 47 +59/-23 au, occurrence is S_bin = 0.34 +0.14/-0.15 of that of wide binaries or singles (about 3 times fewer). Large uncertainty on a_cut. |
| Wolszczan and Frail 1992, PSR B1257+12 | confirmed (reference) | Wolszczan and Frail, Nature 355, 145 (1992), DOI 10.1038/355145a0 (Crossref record) | Archive: planets c and d (P = 66.54 d, 98.21 d) are the 1992 pair; b (P = 25.26 d, 0.0000629 M_Jup) was added in 1994. |
| PSR B1620-26 b (Sigurdsson 2003) | confirmed | Sigurdsson et al., "A Young White Dwarf Companion to Pulsar B1620-26: Evidence for Early Planet Formation", Science 301, 193 (2003), arXiv:astro-ph/0307339, DOI 10.1126/science.1086326 (arxiv.org/abs/astro-ph/0307339) | The paper's WD companion is 0.34 +/- 0.04 M_sun, age 480 +/- 140 Myr (the young WD, not the 12.7 Gyr cluster age; the exchange-encounter scenario is in the abstract). Planet mass 2.5 M_Jup from the NASA archive. |
| PSR J1719-1438 b (Bailes 2011), 23 g/cm^3, 2.2 h, "diamond planet" | confirmed | Bailes et al., Science 333, 1717 (2011), arXiv:1108.5201, DOI 10.1126/science.1208890 (arxiv.org/abs/1108.5201) | Abstract: companion mass near Jupiter, min density 23 g/cm^3, "may be an ultra-low mass carbon white dwarf", P = 2.2 h. The term "diamond" is not in the abstract. |
| Pulsar planets "around 5-6" | corrected | NASA Exoplanet Archive TAP query on `pscomppars` (hostname like PSR%), 2026-11-15 | 7 entries: PSR B1257+12 b, c, d; B1620-26 b; J1719-1438 b; J2322-2650 b; B0329+54 b (the last is a 10 140 d candidate). |
| WD 1856+534 b (Vanderburg 2020), ~10 R_earth, < 14 M_Jup | confirmed | Vanderburg et al., Nature 585, 363 (2020), arXiv:2009.07282 (arxiv.org/abs/2009.07282) | Abstract: "giant planet candidate", roughly Jupiter-size, no more than 14 M_Jup (95 %), P = 1.4 d. NASA archive: R = 10.4 R_earth, 13.8 M_Jup, P = 1.408 d. The paper title says "candidate". |
| WD 1145+017 (Vanderburg 2015), 4.5 h, depth up to 40 % | corrected (id) | Vanderburg et al., "A disintegrating minor planet transiting a white dwarf", Nature 526, 546 (2015), arXiv:**1510.06387**, DOI 10.1038/nature15527 (arxiv.org/abs/1510.06387) | The note's arXiv:1509.07516 is an unrelated maths paper. Periods 4.5-4.9 h, depths up to 40 %. |
| KIC 12557548 b (Rappaport 2012), 15.7 h, depth 0.2-1.3 %, 0.01 M_earth | corrected (id and mass) | Rappaport et al., ApJ 752, 1 (2012), arXiv:**1201.2662**, DOI 10.1088/0004-637X/752/1/1 (arxiv.org/abs/1201.2662) | The note's arXiv:1201.2189 is a different paper (KOI-961). P = 15.685 h, depth up to 1.3 % and below 0.2 %. Fiducial mass **0.1 M_earth**, evaporation time about 0.2 Gyr, mass-loss about 1 M_earth/Gyr. |
| K2-22 b (Sanchis-Ojeda 2015), 9.1 h, depth 0-1.3 % | confirmed | Sanchis-Ojeda et al., ApJ 812, 112 (2015), arXiv:1504.04379 (arxiv.org/abs/1504.04379) | P = 9.1457 h, depth ~0 to 1.3 %, M-dwarf host (T_eff ~3800 K). |
| Disintegrating planets "live ~1 Myr to 1 Gyr" | not found | No primary source opened for the 1 Myr bound | Only KIC 12557548 b's 0.2 Gyr evaporation time (0.1 M_earth fiducial) was verified. |
| Pierrehumbert 2011 "eyeball", arXiv:1008.3911 | corrected (reference) | Pierrehumbert, "A Palette of Climates for Gliese 581g", ApJL 726, L8 (2011), DOI 10.1088/2041-8205/726/1/L8 (Crossref record; web search confirmed the eyeball scenario; no arXiv preprint found) | arXiv:1008.3911 is "External bias in the model of isolation of communities" (unrelated). Drop the id. |
| Kasting 1993 HZ | confirmed | Kasting, Whitmire and Reynolds, "Habitable Zones around Main Sequence Stars", Icarus 101, 108 (1993), DOI 10.1006/icar.1993.1010 (Crossref record) | Kopparapu's abstract quotes Kasting's conservative HZ as 0.95-1.67 au. |
| Kopparapu 2013 HZ, 0.99-1.70 au, 2600-7200 K | confirmed | Kopparapu et al., ApJ 765, 131 (2013), arXiv:1301.6674, DOI 10.1088/0004-637X/765/2/131 (arxiv.org/abs/1301.6674) | Water loss 0.99 au, maximum greenhouse 1.70 au; cloud-free 1-D model. |
| Bryson 2021, eta_earth 0.37-0.60 conservative, 0.58-0.88 optimistic | confirmed | Bryson et al., AJ 161, 36 (2021), arXiv:2010.14812 (arxiv.org/abs/2010.14812) | 0.5-1.5 R_earth, T_eff 4800-6300 K. Conservative HZ: 0.37 +0.48/-0.21 to 0.60 +0.90/-0.36. Optimistic: 0.58 +0.73/-0.33 to 0.88 +1.28/-0.51 per star. 68 % intervals are very wide. |
| HD 80606 b (Naef 2001), e = 0.93 | confirmed | Naef et al., A&A 375, L27 (2001), DOI 10.1051/0004-6361:20010853 (Crossref); NASA archive | Archive: e = 0.932, P = 111.4 d, 4.16 M_Jup. |
| HD 80606 b spin-orbit angle ~42 degrees | confirmed | Hebrard et al., A&A 516, A95 (2010), arXiv:1004.0790 (arxiv.org/abs/1004.0790) | lambda = 42 +/- 8 degrees (sky-projected). |
| Laughlin 2009 periastron heating, arXiv:0812.1583 | corrected (reference) | Laughlin et al., Nature 457, 562 (2009), DOI 10.1038/nature07649 (Crossref; web search for the abstract) | arXiv:0812.1583 is a Tevatron diffraction paper. Brightness temperature 800 K to 1500 K over about 6 h; no arXiv id found. |
| Albrecht, Dawson and Winn 2022 obliquity review | corrected (authors) | Albrecht, Dawson and Winn, "Stellar obliquities in exoplanetary systems", PASP 134, 082001 (2022), arXiv:2203.05460, DOI 10.1088/1538-3873/ac6c09 (arxiv.org/abs/2203.05460) | Three authors, as the notes say; abstract: prograde, polar and retrograde orbits; tidal damping in some cases. |
| Winn 2010 hot stars, T_eff > 6250 K | confirmed | Winn et al., "Hot Stars with Hot Jupiters Have High Obliquities", ApJL 718, L145 (2010), arXiv:1006.4161 (arxiv.org/abs/1006.4161) | Large obliquities preferentially for T_eff > 6250 K. |
| PDS 70 b (Keppler 2018) | confirmed (reference) | Keppler et al., A&A 617, A44 (2018), arXiv:1806.11568 (arxiv.org/abs/1806.11568) | Title: "planetary-mass companion within the gap of the transition disk". Masses and 22 au not checked in the abstract. |
| PDS 70 c (Haffert 2019) | confirmed (reference) | Haffert et al., Nature Astronomy 3, 749 (2019), arXiv:1906.01486, DOI 10.1038/s41550-019-0780-5 (arxiv.org/abs/1906.01486) | Two accreting protoplanets (H-alpha). 34 au and 2:1 resonance not checked. |
| J1407b (Kenworthy and Mamajek 2015) | confirmed | Kenworthy and Mamajek, ApJ 800, 126 (2015), arXiv:1501.05652, DOI 10.1088/0004-637X/800/2/126; Mamajek et al. AJ 143, 72 (2012), DOI 10.1088/0004-6256/143/3/72 | 56-d eclipse, ~16 Myr star in Sco-Cen, 37 rings to 0.6 au, ring mass about 100 lunar masses; companion unseen. The notes' "young K5" is not in the abstract. |
| Kepler-1625 b-i (Teachey and Kipping 2018) | confirmed | Teachey and Kipping, Sci. Adv. 4, eaav1784 (2018), arXiv:1810.02362, DOI 10.1126/sciadv.aav1784 | Neptune-like exomoon candidate, HST epoch. |
| Rebuttals: Heller 2019, Kreidberg 2019 | confirmed | Heller and Rodenbeck, A&A 624, A95 (2019), arXiv:1902.06018; Kreidberg et al., ApJL (2019), arXiv:1904.10618, DOI 10.3847/2041-8213/ab20c8 | Kreidberg's title: "No Evidence for Lunar Transit". The notes cite "ApJL 877, L15"; volume and page were not shown by the record opened, so treat "877, L15" as unchecked. |
| TRAPPIST-1 (Gillon 2017) | confirmed | Gillon et al., Nature 542, 456 (2017), arXiv:1703.01424, DOI 10.1038/nature21360 (arxiv.org/abs/1703.01424) | Seven planets; host is "8 % the mass of the Sun" at 12 pc (notes say 0.09 M_sun). |
| Proxima b (Anglada-Escude 2016) | confirmed | Anglada-Escude et al., Nature 536, 437 (2016), arXiv:1609.03449, DOI 10.1038/nature19106 | Minimum mass 1.3 M_earth, a "planet candidate" in the paper title. |
| Mróz 2017: 2 617 events, 0.25 free-floating Jupiter per star (95 %) | confirmed | Mróz et al., Nature 548, 183 (2017), arXiv:1707.07634 (arxiv.org/abs/1707.07634; ar5iv text) | "Observed distribution of timescales of 2,617 high-quality microlensing events"; 95 % upper limit 0.25 planet per main-sequence star. |
| Sumi 2011, "~1.8 per star" | confirmed | Sumi et al., Nature 473, 349 (2011), arXiv:1105.3544, DOI 10.1038/nature10092 (arxiv.org/abs/1105.3544) | 1.8 +1.7/-0.8 Jupiter-mass objects per main-sequence star. |
| Mróz 2020, OGLE-2016-BLG-1928, "about 0.3 Earth masses" | corrected | Mróz et al., ApJL 903, L11 (2020), arXiv:2009.12377 (arxiv.org/abs/2009.12377) | t_E = 0.0288 d, theta_E = 0.842 +/- 0.064 uas; lens "Mars- to Earth-mass, Mars favoured" (Gaia proper motion); no stellar companion to 8.0 au. |
| Sumi 2023, ~3 500 events, 6 short events, alpha = 0.96 +0.47/-0.27, 19 (+23/-13) | confirmed | Sumi et al., AJ 166, 108 (2023), arXiv:2303.08280 (abstract and ar5iv text; v2) | Text: "more than 3500 events"; six events with t_E < 0.5 d; dN/dlogM = 2.18 +0.52/-1.40 (M/8 M_earth)^-alpha per dex per star, alpha = 0.96 +0.47/-0.27 (M < 0.02 M_sun); 21 +23/-13 FFP per star (0.33-6660 M_earth); FFP are 19 +23/-13 times the wide-orbit planets. |
| Sumi 2023 normalisation "0.49 per star at 38 M_earth in a review" | confirmed (consistent) | Review arXiv:2310.07502 text quotes dN/dlogM = 0.49 +0.12/-0.32 (M/38 M_earth)^-p, p = 0.94 +0.47/-0.27 | Same fit as 2.18 at 8 M_earth: 2.18 x (8/38)^0.96 = 0.49. Not a conflict, only a different pivot mass. The Sumi paper text also quotes an earlier 0.4 +/- 0.2 at 38 M_earth. |
| JuMBOs, Pearson and McCaughrean 2023 (~40 binaries in 540 candidates) | confirmed | Pearson and McCaughrean, arXiv:2310.01231 (abstract and ar5iv text) | 540 planetary-mass candidates (down to 0.6 M_Jup); "40 systems that have a binary companion within 1 arcsec (390 au), and two visual triple systems"; 9 % lower bound on multiplicity; table lists JuMBO 1-42; 3.1 chance alignments expected. |
| Luhman 2024: most JuMBOs absent from his sample | confirmed | Luhman, arXiv:2410.10406 (arxiv.org/abs/2410.10406) | "most of the Jupiter-mass binary objects ... are absent from my sample" because their colours indicate reddened background sources or the photometry has low S/N. |
| Cumming 2008, 10.5 %, 0.3-10 M_Jup, "< 5 au" | corrected | Cumming et al., PASP 120, 531 (2008), arXiv:0803.3357 (arxiv.org/abs/0803.3357) | 10.5 % of solar-type stars have a planet of 0.3-10 M_Jup with **P = 2-2000 d** (about < 3 au). Not "< 5 au". |
| Fischer and Valenti 2005, occurrence proportional to 10^(2 [Fe/H]) | partly (not read in the paper) | Fischer and Valenti, ApJ 622, 1102 (2005), DOI 10.1086/428383 (IOP abstract page opened) | Abstract confirms: sample of 850 stars, K > 30 m/s, P < 4 yr, under 3 % with planets at -0.5 < [Fe/H] < 0, 25 % at [Fe/H] > +0.3, and a power-law fit. The exponent (A = 0.03, gamma = 2.0, i.e. the square of the iron abundance) comes from secondary sources only. Keep marked [M]. |
| Johnson 2010, 3.5 % (M dwarfs) to 14 % (A stars) | confirmed | Johnson et al., "Giant Planet Occurrence in the Stellar Mass-Metallicity Plane", PASP 122, 905 (2010), arXiv:1005.3084 (arxiv.org/abs/1005.3084) | 3.5 % at 0.5 M_sun to 14 % at 2 M_sun, **at solar metallicity**; f proportional to M_star; f proportional to 10^(**1.2** [Fe/H]) (not 2). 1194 stars. |
| Fernandes 2019: 1.44 +/- 0.54 per 100 stars at 3.8-7.1 au | confirmed (via Fulton 2021, and Fernandes id) | Fernandes et al., ApJ 874, 81 (2019), arXiv:1812.05569, DOI 10.3847/1538-4357/ab0300; the number is quoted in Fulton et al. arXiv:2105.11584 (ar5iv text) | "1.44 +/- 0.54 giant planets (0.1-20 M_J) per 100 stars for separations between 3.8 and 7.1 au" (Fernandes' own table not opened). |
| California Legacy Survey ~15.5 per 100 | confirmed | Fulton et al., "California Legacy Survey II", ApJS 255, 14 (2021), arXiv:2105.11584 (abstract and ar5iv text) | 15.5 +3.2/-3.0 per 100 stars at 3.8-7.1 au (same bin as Fernandes); abstract also gives 14.1 +2.0/-1.8 per 100 at 2-8 au and 8.9 +3.0/-2.4 at 8-32 au. The text attributes the gap partly to Fernandes's unclear treatment of partial orbits (Mayor 2011 sample). |
| Petigura 2013, "~26 % with 1-2 R_earth, P 5-100 d" | not found | Petigura et al., PNAS 110, 19273 (2013), arXiv:1311.6806 (abstract and ar5iv text, which is the supplement) | No "26 %" found. Found: 11 +/- 4 % of Sun-like stars with an Earth-size planet at 1-4 F_earth; 22 +/- 8 % with 1-2 R_earth planets in the HZ (0.25-4 F_earth); 5.7 +1.7/-2.2 % at P = 200-400 d; 5.8 +/- 1.6 % in the 50-100 d bin. The note "22 % for P 20-200 d" is a mislabel of the HZ number. |
| Hot Jupiters ~0.5-1 % (Wright 2012; Petigura 2018 ~0.57 %) | corrected | Wright et al., ApJ 753, 160 (2012), arXiv:1205.2273; Petigura et al., AJ 155, 89 (2018), arXiv:1712.04042 (abstract of Wright; Petigura rate seen via a search summary) | Wright: **1.2 +/- 0.38 %** (RV, FGK dwarfs). Petigura 2018: 0.57 +0.14/-0.12 % (8-24 R_earth, 1-10 d), about half the RV rate (1.5 sigma). The 0.57 value is from a secondary summary, not read in the paper. |
| Dressing and Charbonneau 2015, "2.5 +/- 0.2 planets of 0.5-4 R_earth, P < 200 d per M dwarf" | corrected | Dressing and Charbonneau, ApJ 807, 45 (2015), arXiv:1501.01623 (arxiv.org/abs/1501.01623) | 2.5 +/- 0.2 planets per M dwarf with radii **1-4 R_earth** and P < 200 d (0.56 Earth-size 1-1.5 R_earth and 0.46 super-Earths 1.5-2 R_earth for P < 50 d). |
| Millholland 2017 (intra-system uniformity) | confirmed (reference) | Millholland, Wang and Laughlin, ApJL 849, L33 (2017), DOI 10.3847/2041-8213/aa9714 (Crossref) | Title: "Kepler Multi-planet Systems Exhibit Unexpected Intra-system Uniformity in Mass and Radius". Numbers not read. |
| Hsu 2019, Suzuki 2016, Weiss 2018 ids | confirmed (ids and titles only) | arXiv:1902.01417 (Hsu, ApJ/AJ 158, 109); arXiv:1612.03939 (Suzuki, ApJ 833, 145, "break and likely peak at a Neptune mass"); arXiv:1706.06204 (Weiss, "Peas in a Pod", AJ 155, 48) | Titles and first authors match; the numbers quoted in the report were not re-checked here. |
| Hardegree-Ullman 2019; Agol 2021; Faria 2022; Kipping 2022; Quintana 2014; Jenkins 2015; Yang 2013; Zahnle and Catling 2017; Fabrycky and Tremaine 2007 (ids in the notes) | confirmed (ids and titles only) | arXiv:1905.05900; 2010.01074; 2202.05188; 2201.04643; 1404.5667; 1507.06723; 1307.0515; 1702.03386; 0705.4285 (API records) | All match the cited paper. Numbers in the notes for these were not re-checked. |
| Leconte 2013 (Nature), arXiv:1303.7094 | corrected | Leconte et al., "Increased insolation threshold for runaway greenhouse processes on Earth-like planets", Nature 504, 268 (2013), arXiv:**1312.3337** (web search record; ar5iv link) | arXiv:1303.7094 is a maths paper. arXiv:1303.7079 is Leconte's A&A 554, A69 paper (different). |
| Barnes 2017 tidal locking, arXiv:1608.06919 | corrected | Barnes, "Tidal locking of habitable exoplanets", Celest. Mech. Dyn. Astron. 129, 509 (2017), arXiv:**1708.02981**, DOI 10.1007/s10569-017-9783-7 (web search record) | arXiv:1608.06919 is "The Habitability of Proxima Centauri b I" (also Barnes, different paper). |
| Kepler-444 (Campante 2015) | confirmed | Campante et al., ApJ 799, 170 (2015), arXiv:1501.06227 (arxiv.org/abs/1501.06227) | Five transiting planets between Mercury and Venus in size; age 11.2 +/- 1.0 Gyr. |
| BEBOP-1c, gamma Cephei Ab, alpha Cen A JWST candidate, Kepler-1708 b-i claims | not found (not opened) | Kepler-1708 b-i: arXiv:2201.04643 title confirmed only. Gamma Cep: Hatzes et al. ApJ 599, 1383 (2003), arXiv:astro-ph/0305110 exists. BEBOP-1c and alpha Cen A not opened. | Not verified in this pass. |
| Pearson/Orion "~90 % of ejected pairs destroyed in 1 Myr", Wang, Perets and Kouwenhoven 2024; Portegies Zwart and Hochart 2023 | not found (not opened) | Only the White Rose PDF was already [V] in the notes | Out of the time budget; keep [M]. |

## Method notes

- NASA Exoplanet Archive queries used `https://exoplanetarchive.ipac.caltech.edu/TAP/sync` with `pscomppars`
  (one default row per planet) and `ps` (`default_flag=1`, `cb_flag=1`).
  Counts are those of the archive on 2026-11-15 and move with the archive.
- ADS and iopscience abstract pages answered 405 or truncated text, so
  numbers for Fischer and Valenti, Pierrehumbert and Petigura 2018 could not
  be read at the source and are flagged above.
- Journal volume and page numbers in the table that are not accompanied by a DOI
  or an arXiv `journal_ref` shown by a source opened here (for example Sumi 2023,
  Mróz 2017, Petigura 2013, Fulton 2021, Johnson 2010, Weiss 2018) are carried over
  from the notes or from memory and must be treated as unchecked; the arXiv ids,
  authors, years and quoted numbers are the verified part.
- No PDF text extractor was available; full-text checks used ar5iv.
