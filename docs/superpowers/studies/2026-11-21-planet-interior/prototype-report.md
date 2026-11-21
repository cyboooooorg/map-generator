# EOS prototype for the planet interior model

Scripts (this folder, Python 3 with numpy 2.5 and scipy 1.18 in `venv/`):

- `eos.py`: every EOS, with its source in comments.
- `solve.py`: the integrator and the shooting.
- `run.py`: bodies, compositions, the run over bodies and candidates (writes
  `results.json`).
- `conv.py`: convergence study.
- `report_tables.py`: the tables below.

Run with `./venv/bin/python -W ignore run.py`.

## Model

- Mass coordinate `t = (m/M)^(1/3)`, so the centre is regular and the layer
  boundaries are exact (layer masses are given, never found).
- State `(r, P, I)`; `I = (2/3)∫r² dm = (8π/3)∫ρ r⁴ dr`; classical RK4,
  `N` steps per layer.
- Shooting on the central pressure (brentq on `ln Pc`) so that `P(M) = 1e5 Pa`.
- EOS are tabulated once as `ρ(P)` on a 40 000-point log-P grid
  (1 kPa to 30 TPa) and read by log-log linear interpolation.
  Table build: 0.1 s once.
- Mantle: one mass layer, EOS switched by pressure at 23.83 GPa
  (upper / lower mantle). Core: one mass layer, iron EOS switched at
  234.4 GPa (B), or inner / outer core switched at the PREM inner-core
  boundary pressure 328.851 GPa (C, D1).
- 300 K, no thermal term (the study gives no complete sourced thermal
  parameter set for the chosen EOS).

## Candidates

| Id | Core | Mantle | Water |
| --- | --- | --- | --- |
| A | Seager 2007 polytrope Fe (Table 3) | polytrope MgSiO3 (Table 3) | polytrope H2O (Table 3) |
| B | Fe Vinet Dewaele 2006 (ρ0 8270, K 163.4, K' 5.38) below 234.4 GPa, Hakim 2018 Holzapfel above | below 23.83 GPa 50/50 (mass) forsterite + enstatite BM3; above, 90 % (Mg0.88,Fe0.12)SiO3 + 10 % MgO BM3 (Amagat mix) | ice VII Vinet fit to DFT (ρ0 1460, K0 14.3771, K0' 6.57972) |
| C | Zeng 2016 BM2: outer core 7.05 g/cc 201 GPa; inner core 7.85 g/cc 255 GPa above 328.851 GPa | upper: linear in P from the olivine/enstatite ρ(0) to PREM 3992.1 at 23.83 GPa; lower: BM2 3.98 g/cc 206 GPa | as B |
| D1 | B with the core replaced by the Valencia 2006 Table 1 PREM-fitted alloy BM3: outer 6800, 165, 4.2; inner 7990, 200, 5.2 (above 328.851 GPa) | as B | as B |
| D2 | B iron with a constant density deficit δ = 15.06 %, calibrated on PREM core-side CMB density 9903.5 at 135.751 GPa | as B | as B |

Values and sources: all from `study-eos.md` (sections 1.1, 2.1, 2.2, 2.3, 3.1,
4),
except the items listed under "Values not in the study".

### Values not in the study

- Fe molar mass 55.845 g/mol (standard atomic weight), needed to turn the
  Hakim 2018 `V0` into a density (13 030 kg/m³). Cross-check: Dewaele
  8270 kg/m³ × 6.75 cm³/mol = 55.8 g/mol. The Dewaele and Hakim branches
  agree at 234.4 GPa within 0.07 %.
- MgSiO3 and FeSiO3 molar masses taken as `ρ0·V0` of the same Unterborn
  Table 1 (100.5 and 132.0 g/mol).
- Mixing proportions of the mantle (90 / 10 and Mg0.88Fe0.12) are from
  Seager 2007 Sect. 4.1 and Table 1 as quoted in the study; the 50/50
  upper mantle is Unterborn 2016 Sect. 3.5. Mass fractions assumed
  (study does not say mass or mole).
- Titan composition (see below) and the 5 M⊕ 50 % water split are my
  assumptions.

## Bodies, masses, radii and compositions

Masses and radii: NASA NSSDC fact sheets (fetched this session):
Earth 5.9722e24 kg; Venus 4.8673e24, 6051.8 km; Mars 0.64169e24, 3389.5 km;
Mercury 0.33010e24, 2439.7 km; Moon 0.07346e24, 1737.4 km; Ganymede
1481.9e20 kg (Jovian satellite sheet); Titan 1345.5e20 kg (Saturnian
satellite sheet). Target radii for Ganymede 2634.1 and Titan 2574.7 km
are the brief's values (NASA sheets give 2631.2 and 2575). Target C/MR²
from study section 4.

Core mass fractions, read this session in Skinner, Pudritz & Cloutier
2026 (arXiv:2604.15304) and Rodriguez-Mozos & Moya 2022
(arXiv:2203.01065), downloaded and extracted with pypdf:

| Body | Fractions used (core / mantle / water) | Source |
| --- | --- | --- |
| Earth | 0.325 / 0.675 / 0 | PREM, study section 4 |
| Venus | 0.30 / 0.70 / 0 | Skinner 2026 Sect. 3.4, citing Shah et al. 2022 |
| Mars | 0.21 and 0.25 (both run) | Khan et al. 2023 (0.21) and Khan 2022 / Le Maistre 2023 (0.25), both as quoted by Skinner 2026 Sect. 3.2 |
| Mercury | 0.7136 | Skinner 2026 (core radius 1975 km, Genova et al. 2019) |
| Moon | 0.0168 | Skinner 2026 (Viswanathan et al. 2019) |
| Ganymede | 0.085 / 0.46 / 0.455 | Rodriguez-Mozos & Moya 2022 Table 5 model column (CMF 0.085, MMF 0.46, IMF 0.455; Kuskov & Belashchenko 2016 gives CMF 0.04 to 0.10) |
| Titan | 0 / 0.545 / 0.455 | ASSUMPTION: Ganymede's ice fraction, no iron (no sourced Titan split was found; Gao & Stevenson 2013 use a rock core). Indicative only |
| 5 M⊕ | 0.325 / 0.675 / 0 | brief |
| 5 M⊕ 50 % water | 0.1625 / 0.3375 / 0.5 | ASSUMPTION: Earth-like rock part with 50 % water on top |
| 10 M⊕ | 0.325 / 0.675 / 0 | brief |

Reference radii without a measured value: 5 M⊕: Zeng 2016
`R = (1.07 - 0.21·cmf)·M^(1/3.7)` gives 9860 km (1.548 R⊕); 10 M⊕: 11 892 km
(extrapolated above the 1 to 8 M⊕ validity of the formula); Seager eq. 23
with the Table 4 mixed row (m1 6.41, r1 3.19, printed labels swapped as
the study suspects) gives 6179 km (1 M⊕), 9765 km (5), 11 672 km (10);
Zeng 2019: 5 M⊕ 50 % water 1.911 R⊕ = 12 175 km at 300 K (1.981 R⊕ at
700 K, 12 621 km).

## Results: radius, km (error vs reference)

| Body | A | B | C | D1 | D2 |
| --- | --- | --- | --- | --- | --- |
| Earth (6371) | 6114 (-4.04 %) | 6272 (-1.56 %) | 6366 (-0.07 %) | 6352 (-0.29 %) | 6359 (-0.18 %) |
| Venus (6052) | 5775 (-4.57 %) | 5955 (-1.61 %) | 6038 (-0.22 %) | 6029 (-0.38 %) | 6030 (-0.35 %) |
| Mars cmf 0.21 (3390) | 3112 (-8.20 %) | 3388 (-0.05 %) | 3384 (-0.15 %) | 3412 (+0.67 %) | 3409 (+0.57 %) |
| Mars cmf 0.25 | 3087 (-8.92 %) | 3354 (-1.05 %) | 3357 (-0.97 %) | 3384 (-0.17 %) | 3379 (-0.30 %) |
| Mercury (2440) | 2240 (-8.20 %) | 2350 (-3.69 %) | 2422 (-0.72 %) | 2435 (-0.19 %) | 2421 (-0.75 %) |
| Moon (1737) | 1595 (-8.22 %) | 1746 (+0.47 %) | 1744 (+0.39 %) | 1746 (+0.52 %) | 1746 (+0.51 %) |
| Ganymede (2634) | 2424 (-7.97 %) | 2508 (-4.79 %) | 2509 (-4.73 %) | 2512 (-4.62 %) | 2512 (-4.65 %) |
| Titan (2575, assumed split) | 2370 (-7.97 %) | 2461 (-4.43 %) | 2458 (-4.52 %) | 2461 (-4.43 %) | 2461 (-4.43 %) |
| 5 M⊕ (Zeng 9860) | 9765 (-0.96 %) | 9728 (-1.33 %) | 9812 (-0.48 %) | 9842 (-0.19 %) | 9884 (+0.25 %) |
| 5 M⊕ 50 % H2O (Zeng 2019 12175) | 12339 (+1.34 %) | 11996 (-1.47 %) | 12026 (-1.22 %) | 12039 (-1.12 %) | 12060 (-0.94 %) |
| 10 M⊕ (Zeng 11892, extrapolated) | 11744 (-1.24 %) | 11567 (-2.73 %) | 11641 (-2.11 %) | 11761 (-1.10 %) | 11769 (-1.03 %) |

## Results: C/MR² (error vs measured)

| Body | A | B | C | D1 | D2 |
| --- | --- | --- | --- | --- | --- |
| Earth (0.3307) | 0.3403 (+2.89 %) | 0.3208 (-3.01 %) | 0.3311 (+0.11 %) | 0.3299 (-0.25 %) | 0.3308 (+0.04 %) |
| Venus (0.337 ± 0.024) | 0.3437 | 0.3242 | 0.3347 | 0.3332 | 0.3335 |
| Mars cmf 0.21 (0.3638) | 0.3607 (-0.84 %) | 0.3521 (-3.22 %) | 0.3569 (-1.91 %) | 0.3590 (-1.31 %) | 0.3580 (-1.59 %) |
| Mars cmf 0.25 | 0.3562 (-2.09 %) | 0.3458 (-4.95 %) | 0.3521 (-3.22 %) | 0.3540 (-2.69 %) | 0.3528 (-3.01 %) |
| Mercury (0.346; vs 0.333 in text) | 0.3453 (-0.21 %) | 0.3248 (-6.14 %) | 0.3400 (-1.72 %) | 0.3413 (-1.36 %) | 0.3390 (-2.03 %) |
| Moon (0.3927) | 0.3936 (+0.23 %) | 0.3944 (+0.42 %) | 0.3944 (+0.43 %) | 0.3949 (+0.57 %) | 0.3948 (+0.54 %) |
| Ganymede (0.3115) | 0.3091 (-0.77 %) | 0.3239 (+3.97 %) | 0.3253 (+4.43 %) | 0.3263 (+4.75 %) | 0.3259 (+4.62 %) |
| Titan (0.3414, assumed split) | 0.3211 (-5.95 %) | 0.3399 (-0.44 %) | 0.3392 (-0.65 %) | 0.3399 (-0.44 %) | 0.3399 (-0.44 %) |
| 5 M⊕ | 0.3278 | 0.3097 | 0.3137 | 0.3178 | 0.3207 |
| 5 M⊕ 50 % H2O | 0.2898 | 0.2884 | 0.2905 | 0.2916 | 0.2932 |
| 10 M⊕ | 0.3197 | 0.3018 | 0.3052 | 0.3129 | 0.3131 |

Mercury against the lower value 0.333 (Genova 2019 via Skinner): A +3.7 %,
B -2.5 %, C +2.1 %, D1 +2.5 %, D2 +1.8 %; the two measurements disagree by
4 %, so no candidate can be within 2 % of both.

## Earth structure against PREM (CMB 3480 km, 135.751 GPa; centre 363.852 GPa)

| | A | B | C | D1 | D2 |
| --- | --- | --- | --- | --- | --- |
| CMB radius km | 3395 | 3269 | 3481 | 3480 | 3496 |
| CMB pressure GPa | 150.6 | 156.1 | 135.9 | 138.0 | 136.7 |
| Central pressure GPa | 390.9 | 443.9 | 364.3 | 367.5 | 354.2 |

A reproduces the study's own polytrope integration (6112 km, 0.3396 there,
6114 km and 0.3403 here; the 0.2 % gap is the 1 km versus N=400 grid) and the
5 M⊕ Earth-like point agrees with Seager eq. 23 (9765 km both).

## Run time

Per model on this machine (macOS, Python scalar RK4, N = 400 per layer, about
35 shooting iterations): 0.06 to 0.13 s, mean 0.084 s. The EOS table build
(0.1 s) is paid once. A Rust version with an on-the-fly Newton inversion of
each EOS instead of the table should be of the same order or faster.

## Convergence

Scheme: RK4 in `t = (m/M)^(1/3)`, N equal steps per layer, 3 layers at most.
`conv.py` compares N = 10 to 640 with N = 2560, all bodies x all candidates:

- Smooth cases (small bodies, A, water worlds): 1e-4 on R and C/MR² at
  N = 10 to 20 per layer (errors fall by ~16x per doubling, order 4).
- Earth-size and larger with the 23.83 GPa transition or the inner-core
  switch inside a layer: the error is first order (it halves per doubling of
  N, because RK4 steps over a density jump). 1e-4 needs N = 160 per layer
  for Earth (B, D1, D2), 80 for C, and 640 for 10 M⊕ (B, C, D1, D2),
  i.e. about 2000 steps in total. Everything in the tables used N = 400, so
  numbers for 10 M⊕ carry about 2e-4 numerical error (irrelevant against
  the 2 % goal).
- Expectation, not tested: locating each pressure-driven transition and
  using it as an extra layer boundary restores order 4 and should bring
  every case to N of about 40 per layer.

## Findings

- A (polytropes) cannot reach small bodies: ρ0 = 4100 (silicate) and 1460
  (water) at zero pressure gives -8 % on R for Mars, Mercury, Moon,
  Ganymede and Titan, and -4 % on Earth. Fallback only.
- B (pure mineral physics, pure Fe core) fails: Earth R -1.56 %, C -3.0 %,
  CMB radius 3269 km (211 km too small), central pressure +22 %; Mercury R
  -3.7 %, C -6.1 %; Mars C -3.2 % (-5.0 % with cmf 0.25). Cause: ε-Fe at
  300 K is 15 % denser than the PREM core at the CMB pressure (11 664 vs
  9 904 kg/m³ at 135.75 GPa). That is light elements plus temperature
  (study section 3.4 says temperature alone is about 4 % at most), and
  matches Unterborn's 13 % mass excess. The mantle side is fine
  (lower mantle +3.5 % over PREM at 23.83 GPa at 300 K).
- C and D1/D2 pass for Earth because they are tied to PREM (C by
  construction, D1 by Valencia's PREM fit, D2 by one calibration at the
  CMB); the useful test is the other rocky bodies. C is within 2 % on R and
  C/MR² for Venus, Mars (cmf 0.21), Mercury (vs 0.346) and Moon.
  D1 is the best of them (Mars R +0.7 %, C -1.3 %; Mercury R -0.2 %,
  C -1.4 %).
- Mars with cmf 0.25 fails on C/MR² for B, C, D1 and D2 (-2.7 to -5 %): the
  assumed pure-mantle plus dense core is too centrally concentrated; the
  measured value needs a lower CMF (0.21 fits) or a less dense core.
- Ganymede and Titan fail for every candidate (R about -4.5 %, Ganymede
  C +4 to +5 %). The cause is the water layer: every sourced simple fit
  (Seager Vinet, BME) starts at 1460 kg/m³ at zero pressure, but the
  ice shell is ice Ih/III/V/VI and liquid, mean density about 1.23 g/cc
  (Rodriguez-Mozos & Moya Table 5). Diagnostic: scaling the water density
  by 0.813 (Ganymede) or 0.82 (Titan) recovers R exactly and Ganymede's
  C/MR² (0.3093 vs 0.3115). The study lists no formula for ice Ih to VI
  (Chaplin tables, IAPWS, AQUA were not read), so this cannot be fixed
  from the study.
- Super-Earths: all candidates are within 1.5 % of Zeng 2016 / Zeng 2019
  at 5 M⊕ except B at 10 M⊕ (-2.7 %, mostly the 300 K pure-Fe core with
  central pressure 5 TPa, plus BM3 mantle extrapolation); D1 and D2 are
  within 1.1 %. Caveats: the 10 M⊕ reference is itself extrapolated, the 50 %
  water composition is assumed, and Zeng 2019 is for a steam-capped 300 K
  planet.

## Recommendation

Rocky bodies and super-Earths: D1 (B, with the Valencia 2006 PREM-fitted
Fe-alloy core, BM3 outer 6800 / 165 / 4.2 and inner 7990 / 200 / 5.2), or
C. D1 keeps every rocky body within 2 % on R and on measured C/MR²
(Mars cmf 0.21, Mercury 0.346), apart from the Mercury 0.333 and Mars cmf
0.25 alternatives. The mantle and water are those of B. Its central pressure
stays
below the validity of the Valencia fit's BM3 only up to a few TPa
(3.7 TPa at 10 M⊕, an extrapolation). D2 (B iron with a 15 % deficit) is a
cruder way to the same result.

No candidate satisfies #534 (2 %) on every body: the two icy moons fail
by about 4.5 % in R and 4 to 5 % in C/MR² because the water layer lacks a
sourced low-pressure EOS (ice Ih to VI, liquid). Next step: read Chaplin /
IAPWS or the AQUA tables (study section 2.3) and add a pressure switch
below about 2.2 GPa. The core light-element content for Mars and Mercury
is set by Earth's PREM fit and is not independent (Mars C -1.3 % at
cmf 0.21, -2.7 % at 0.25).

Not covered: temperature (cold 300 K), post-perovskite and the ppv
transition above 122 GPa (parameters not found), TFD above 3.5 TPa
(Zeng) / 7.7 TPa (Seager water), Mars/Moon mantle compositions (Earth-like
mantle used for all), crust.
