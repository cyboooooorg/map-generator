# Planet classification schemes and demographics

Verification legend: [V] = checked against a fetched/searched source this session; [M] = from memory of the literature, not re-verified (double-check id/numbers before relying on them).

## Official definitions (IAU, Margot, Stern-Levison, Soter)

### Takeaway (section 1)

The IAU 2006 definition is qualitative (orbit-clearing); Margot 2015 and Soter 2006 / Stern and Levison 2002 give quantitative dynamical discriminants. The IAU exoplanet working definition caps planets at 13 Jupiter masses. None of these defines a planetary-mass-moon class.

### Cited Findings (section 1)

- Margot 2015, "A Quantitative Criterion for Defining Planets", AJ 150:185, arXiv:1507.06300. Criterion uses only star mass, planet mass and orbital period to test whether a body can clear its zone within a time such as the main-sequence lifetime. All 8 planets pass; Ceres, Pluto, Eris fail; about 99% of known exoplanets are classifiable and pass. [V] — [arXiv](<https://arxiv.org/abs/1507.06300v4>)
- Margot and Gladman extended the criteria in arXiv:2407.07590, "Quantitative Criteria for Defining Planets". [V, title only] — [ar5iv](<https://ar5iv.labs.arxiv.org/html/2407.07590>)
- IAU 2006 Resolutions B5/B6 (Prague, Aug 2006): a planet orbits the Sun, has enough mass for hydrostatic equilibrium (near round), and has cleared its neighbourhood; a dwarf planet meets the first two but not the third and is not a satellite; other bodies are "Small Solar System Bodies". [M] — official text at iau.org (IAU 2006 GA, Resolution B5); not fetched.
- IAU working definition of exoplanet (2001, amended 2003; reaffirmed 2018 by the IAU Working Group on Extrasolar Planets): objects with true mass below the deuterium-burning limit (about 13 M_Jup) orbiting stars/brown dwarfs/remnants; minimum mass/ratio condition m/M < 2/(25+sqrt(621)) about 0.04 for the host-companion pair. The 0.04 mass-ratio figure and 2018 date are [M]; I did not locate the primary document.
- Soter 2006, "What is a planet?", AJ 132, 2513, arXiv:astro-ph/0608359: discriminant Lambda/Pi style, ratio of body mass to the mass of the rest of its zone; planets exceed it by orders of magnitude (about 1e4 to 1e6), dwarfs fall below. [M]
- Stern and Levison 2002, "Regarding the criteria for planethood and proposed planetary classification schemes", Highlights of Astronomy 12, 205: dynamical discriminant Lambda = k M^2 / P^(3/2)-type; planets have Lambda > 1. [M]

### Inferences (section 1)

- For a generator, a simple rule is "planet if mass above the clearing threshold of its orbit (Margot), dwarf if round but not dominant, else small body", with exoplanet upper cap 13 M_Jup (about 4100 M_earth).

### Gaps (section 1)

- Exact Margot constant (Pi > 1 with k = 807 in his normalisation, I recall) and Soter numeric values unverified.
- IAU 2018 text not retrieved; IAU primary pages not fetched.

## Mass and radius taxonomies

### Takeaway (section 2)

Chen and Kipping 2017 give the standard probabilistic four-class mass-radius split; Kopparapu 2018 gives a size x flux grid. Kepler's informal radius bins are the de facto demographic language.

### Cited Findings (section 2)

- Chen and Kipping 2017, "Probabilistic Forecasting of the Masses and Radii of Other Worlds", ApJ 834, 17, DOI 10.3847/1538-4357/834/1/17, arXiv:1603.08614. [V] — [arXiv](<https://arxiv.org/abs/1603.08614>)
  - Transitions (R proportional to M^S): Terran-Neptunian 2.04 (+0.66/-0.59) M_earth; Neptunian-Jovian 0.414 (+0.057/-0.065) M_Jup (about 131 M_earth); Jovian-Stellar 0.080 (+0.008/-0.007) M_sun. [V] — [ar5iv](<https://ar5iv.labs.arxiv.org/html/1603.08614>)
  - Indices S: Terran 0.279 +/-0.009; Neptunian 0.589 (+0.044/-0.031); Jovian -0.044 (+0.017/-0.019); Stellar 0.881 +/-0.025. [V] (same ar5iv page)
- Kopparapu et al. 2018, "Exoplanet Classification and Yield Estimates for Direct Imaging Missions", ApJ 856, 122 (the page cited vol 856 issue 2; article number [M]), DOI 10.3847/1538-4357/aab205, arXiv:1802.09602. Size bins: rocky, super-Earth, sub-Neptune, sub-Jovian, Jovian; flux bins hot/warm/cold, with boundaries from condensation curves (ZnS, H2O, CO2, CH4). [V] — [arXiv](<https://www.arxiv.org/abs/1802.09602>)
- Borucki et al. 2011, Kepler first-4-months planet candidates, ApJ 736, 19, arXiv:1102.0541: size classes Earth-size (R < 1.25 R_e), super-Earth (1.25-2), Neptune-size (2-6), Jupiter-size (6-15), very large (15-22). Bin edges [M].
- Stevens and Gaudi 2013, "A Posteriori Transit Probabilities" / mass-radius work: I could not recall the correct paper with confidence; the scheme I intended (planet/brown dwarf boundary by formation) is unverified. Treat as a gap.
- NASA Exoplanet Archive/Exoplanet Catalog types: Terrestrial, Super Earth, Neptune-like, Gas Giant, Unknown. [V] — [NASA catalog](<https://science.nasa.gov/exoplanets/exoplanet-catalog>)
- Radius valley near 1.5-2 R_earth (Fulton et al. 2017, AJ 154, 109, arXiv:1703.10375) separates super-Earths and sub-Neptunes. [M]

### Inferences (section 2)

- Use Chen-Kipping mass transitions for mass-driven classes and Fulton valley (about 1.7 R_e) plus Kepler bins for radius; Kopparapu hot/warm/cold for the thermal axis.

### Gaps (section 2)

- Kopparapu flux boundary numbers and the Stevens and Gaudi reference not retrieved.

## PHL classification and Earth Similarity Index

### Takeaway (section 3)

PHL (Univ. Puerto Rico Arecibo) labels planets by thermal zone and size (e.g. mesoplanet, psychroplanet, warm/hot Jovian); ESI is a descriptive index, not a validated habitability measure.

### Cited Findings (section 3)

- Schulze-Makuch et al. 2011, "A Two-Tiered Approach to Assessing the Habitability of Exoplanets", Astrobiology 11, 1041 (ESI = geometric mean of weighted radius, density, escape velocity, surface temperature similarity to Earth, values 0-1). [M] — DOI not verified.
- PHL classes (Mercurian, Subterran, Terran, Superterran, Neptunian, Jovian by mass bands; Hot/Warm/Cold zones): from memory of phl.upr.edu, not fetched. [M]

### Inferences (section 3)

- ESI and PHL are heuristic, ESI is known to be insensitive to atmosphere/interior; use for flavour, not for statistics.

### Gaps (section 3)

- Scientific status criticism papers not located; PHL bin edges not verified.

## Demographics and occurrence rates

### Takeaway (section 4)

Small planets (1-4 R_earth, P < 100 d) are the commonest type, about 30-50% of Sun-like stars per size bin; hot Jupiters about 0.5-1%; cold giants about 10-20%+. Giant occurrence rises with metallicity and stellar mass; small planets are common around M dwarfs.

### Cited Findings (section 4)

- Archive counts: 6,470 confirmed planets (Transit 4,799; RV 1,205; microlensing 294; imaging 98; TTV 29) on the archive counts page; the fetch showed no date, so late-2026 status is inferred. [V] — [NASA Exoplanet Archive](<https://exoplanetarchive.ipac.caltech.edu/docs/counts_detail.html>)
- Per-type counts (terrestrial, super-Earth, Neptune-like, gas giant) not obtained; the catalog only reports a filterable type field. [V] — [NASA catalog](<https://science.nasa.gov/exoplanets/exoplanet-catalog>)
- Zhu and Dong 2021, "Exoplanet Statistics and Theoretical Implications", ARA&A 59, 291, DOI 10.1146/annurev-astro-112420-020055, arXiv:2103.02127. Review focused on the inner 1 au. [V] — [arXiv](<https://arxiv.org/pdf/2103.02127>)
- Gaudi, Christiansen, Meyer, "The Demographics of Exoplanets", arXiv:2011.04703 (ExoFrontiers chapter). [V] — [arXiv](<https://arxiv.org/abs/2011.04703>)
- Hsu et al. 2019, AJ 158, 109, arXiv:1902.01417: occurrence for 0.5-16 R_e, P 0.5-500 d, FGK stars, Kepler DR25 + Gaia DR2; 0.75-1.5 R_e planets at 237-500 d: 0.03-0.40 per star. [V] — [arXiv](<https://arxiv.org/abs/1902.01417>)
- Bryson et al. 2021, AJ 161, 36, DOI 10.3847/1538-3881/abc418, arXiv:2010.14812: eta-Earth (0.5-1.5 R_e, 4800-6300 K hosts): conservative 0.37-0.60, optimistic 0.58-0.88 per star. [V] — [ADS](<https://ui.adsabs.harvard.edu/abs/2021AJ....161...36B>)
- Fernandes et al. 2019, ApJ 874, 81, arXiv:1812.05569: giant-planet occurrence breaks near 2-3 au; 1.44+/-0.54 per 100 stars at 3.8-7.1 au. Contested: California Legacy Survey (Fulton et al. 2021, arXiv:2105.11584) finds about 15.5 per 100 stars at the same separations. [V] — [ADS](<https://ui.adsabs.harvard.edu/abs/2019ApJ...874...81F/abstract>), [CLS II](<https://arxiv.org/pdf/2105.11584>)
- Suzuki et al. 2016, ApJ 833, 145, DOI 10.3847/1538-4357/833/2/145, arXiv:1612.03939: microlensing mass-ratio function broken power law, break near q about 1.7e-4 (about 20 M_e for 0.6 M_sun host), slopes -0.93+/-0.13 above and 0.6 (+0.5/-0.4) below. Revised by Jung 2019 to about 5.5e-5. [V] — [arXiv](<https://arxiv.org/pdf/1612.03939>)
- Weiss et al. 2018, "California-Kepler Survey V: Peas in a Pod", AJ 155, 48, arXiv:1706.06204: 909 planets in 355 multis; adjacent planets similar in size and regularly spaced; essentially none with period ratio below 1.2. Critique: Murchikova and Tremaine 2020, arXiv:2003.02290. [V] — [arXiv](<https://arxiv.org/abs/1706.06204v3>)
- Fressin et al. 2013, ApJ 766, 81, arXiv:1301.0842 (occurrence by size bin; numbers unverified); Petigura et al. 2013, PNAS 110, 19273, arXiv:1311.6806 (26% of Sun-like stars have 1-2 R_e, P 5-100 d; eta-Earth about 22%). [M]
- Cumming et al. 2008, PASP 120, 531, arXiv:0803.3357 (about 10.5% of FGK stars host giants with 0.3-10 M_Jup within 5 au). [M]
- Fischer and Valenti 2005, ApJ 622, 1102 (giant occurrence proportional to roughly 10^(2*[Fe/H])); Johnson et al. 2010, PASP 122, 905, arXiv:1005.3084 (giant occurrence rises with stellar mass, 3.5% M dwarfs to about 14% A stars). [M]
- Millholland et al. 2017, ApJ 849, L33, arXiv:1708.08453 (intra-system mass uniformity). [M]
- Hot Jupiters about 0.5-1% of FGK stars (Wright 2012; Petigura 2018 about 0.57%); not verified. [M]

### Inferences (section 4)

- A generator can sample: per-star P(any small planet P<100 d) about 0.3-0.5; sibling radii correlated within a system; giant probability scaled by 10^(2 [Fe/H]) and stellar mass; dominant planet class is sub-Neptune/super-Earth.

### Gaps (section 4)

- Late-2026 type counts, Fressin/Petigura numbers, stellar-mass scaling of small planets (Dressing and Charbonneau 2015, Hardegree-Ullman 2019) not verified.

## Solar System bodies as types

### Takeaway (section 5)

Solar System yields 4 terrestrials, 2 gas giants, 2 ice giants, 5 IAU dwarf planets, and many large moons that satisfy a geophysical definition (Runyon 2017).

### Cited Findings (section 5)

- Runyon, Stern, Lauer, Grundy, Summers, Singer 2017, "A Geophysical Planet Definition", LPSC XLVIII, abstract 1448: planet = non-stellar sub-fusion body massive enough for hydrostatic equilibrium, independent of orbit; includes satellite planets and gives about 110 planets in the Solar System. [V] — [Wikipedia mirror](<https://ipfs.ssi.eecc.de/ipns/en.wikipedia-on-ipfs.org/wiki/Geophysical_definition_of_'planet'>); coverage at [phys.org](<https://phys.org/news/2017-03-scientists-case-pluto-planet-status.pdf>)
- IAU-recognised dwarf planets: Ceres, Pluto, Haumea, Makemake, Eris. [M]
- Planetary-mass moons (Titan, Ganymede, Callisto, Io, Europa, Triton, Moon) — general knowledge, [M].

### Inferences (section 5)

- Taxonomy for generation: stellar/substellar bins above, then planet-mass moons as a dependent class around giants.

### Gaps (section 5)

- No peer-reviewed full text of Runyon 2017 retrieved; only a news/Wikipedia description.
