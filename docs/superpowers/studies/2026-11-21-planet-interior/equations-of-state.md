# Literature study: planet interior structure (EOS, temperature, validation)

Scope: rocky, icy and water-rich planets, 0.01 to 20 Earth masses; giants out of
scope.
Goal: replace "mass-radius fit + calibrated uniform-density layers" by a
hydrostatic
integration (dm/dr = 4 pi r^2 rho, dP/dr = -G m rho / r^2, rho = EOS(P[,T]))
with
iron core / silicate mantle / water-ice layers, giving R, rho(r), P(r) and the
moment of
inertia in one computation.

Method and rules followed: every number below was read in a source text that I
downloaded
and extracted during this session (arXiv PDFs through pypdf, ar5iv HTML, NASA
fact sheets,
publisher abstract pages). Journal/DOI/volume/pages were verified against the
Crossref API
(`api.crossref.org`). Values I could not read are marked **not found**. Values
that I
computed myself are marked **(computed here)**. Where a number reaches me only
through
another paper that quotes it, the column "Read in" says so.

Conventions: GPa = 1e9 Pa; ρ in kg m^-3 unless stated (1 g/cc = 1000 kg m^-3);
C/MR^2 = normalised polar moment of inertia.

---

## 0. Summary of the recommendation

1. The Seager et al. 2007 modified polytrope (ρ = ρ0 + c P^n) is usable as a
   **first
   implementation or fallback**, but it is a fit to a zero-temperature EOS that
   was itself
   spliced from a Vinet/BME branch and Thomas-Fermi-Dirac; its stated accuracy
   is 2 to 5 %
   in density at P < 5 GPa and P > 30 TPa, and 1 to 12 % in between, i.e. just
   in the
   Earth-to-super-Earth pressure range. It has no phase transitions, one single
   silicate
   (perovskite, no upper mantle) and no thermal term.
2. For a coherent, citable, per-layer set I recommend (section 2.6): ε-Fe from
   Dewaele et al. 2006 (Vinet) or Fei et al. 2016 (BM3) at P < 234 GPa joined to
   Hakim et al. 2018 (Holzapfel, closed form) above 234 GPa; silicate from
   Stixrude &
   Lithgow-Bertelloni (SLB) parameters (3rd-order BM at 300 K is the cheapest
   form) with an
   explicit upper-mantle / lower-mantle split at 23.83 GPa (PREM value); H2O
   from
   the Frank et al. 2004 / Seager et al. 2007 / French et al. 2009 chain for ice
   VII/X and a
   tabulated EOS (AQUA, Haldemann et al. 2020) for liquid, low-pressure ices and
   supercritical water.
3. Temperature changes the radius by **about 1 % or less** of R for rocky bodies
   below
   about 1500 K surface temperature (Seager, Valencia, Hakim, Grasset, Dorn all
   agree),
   so an adiabat is a refinement; it matters much more for water layers and when
   a magma
   ocean exists (PALEOS: >1 % above 1500 K, up to 16 % at 4000 K for low-mass
   silicate
   planets). The bigger effect on a pure Fe + MgSiO3 model is chemistry:
   Unterborn et al. 2016 find a 2-layer pure-Fe + bridgmanite model overpredicts
   Earth's
   mass by about 13 % at 1 R_Earth, and about 6 % of that is the missing
   low-pressure
   upper mantle.
4. For the validation of the new model: Earth C/MR^2 = 0.3307 (Williams 1994,
   quoted in
   Margot et al. 2018) or 0.33072 (derived here from Skinner et al. 2026), Mars
   0.3634 to
   0.3638, Moon 0.3927 (solid) to 0.3931, Mercury 0.346 +/- 0.009 or 0.333 +/-
   0.005,
   Venus 0.337 +/- 0.024, Ganymede 0.3105 or 0.3115, Titan 0.3414 +/- 0.0005.
   Details and
   caveats in section 4.

---

## 1. Seager, Kuchner, Hier-Majumder & Militzer 2007

Citation: Seager, S.; Kuchner, M.; Hier-Majumder, C. A.; Militzer, B. 2007,
"Mass-Radius Relationships for Solid Exoplanets", ApJ 669, 1279-1297,
doi:10.1086/521346 (Crossref-verified). Text read: arXiv:0707.2895v1 (full
text).

### 1.1 Modified polytrope ρ(P) = ρ0 + c P^n

Equation (11) of the paper. Fit parameters are in **Table 3** of the arXiv text
(the table is captioned "Fits to the merged Vinet/BME and TFD EOS of the form
ρ(P) = ρ0 + c P^n. These fits are valid for the pressure range P < 10^16 Pa"):

| Material (label as printed) | ρ0 [kg m^-3] | c [kg m^-3 Pa^-n] | n | Source |
| --- | --- | --- | --- | --- |
| Fe(α) (see note below, this is the ε-Fe fit) | 8300.00 | 0.00349 | 0.528 | Seager 2007, Table 3 |
| MgSiO3 (perovskite) | 4100.00 | 0.00161 | 0.541 | Table 3 |
| (Mg,Fe)SiO3 | 4260.00 | 0.00127 | 0.549 | Table 3 |
| H2O | 1460.00 | 0.00311 | 0.513 | Table 3 |
| C (graphite) | 2250.00 | 0.00350 | 0.514 | Table 3 |
| SiC | 3220.00 | 0.00172 | 0.537 | Table 3 |

Units: P in pascal, so c P^n has units kg m^-3 (the table header gives c in
"kg m^-3 Pa^-n"). No MgO, FeS or Fe-Si entry exists in Table 3.

Note on the iron label: in the arXiv version, Table 3 and Table 4 label the iron
row
"Fe(α)", but ρ0 = 8300 kg m^-3 is the ε-Fe zero-pressure density of Table 1
(ε-Fe: K0 = 156.2 GPa, K0' = 6.08, ρ0 = 8.30 Mg m^-3, Vinet); α-Fe in Table 1
has
ρ0 = 7.86. The row is therefore the ε-Fe polytrope. Use ε. (The label looks like
a
typesetting slip; the published ApJ table was not read.)

Stated validity and accuracy (Section 5.1, text before and after eq. 11):

- "Table 4 lists the best-fit parameters ρ0, c, and n for some materials over
  the range
  P < 10^16 Pa" (the text says "Table 4", the data are in Table 3; the text and
  captions are inconsistent in the arXiv version).
- "The density given by these approximate EOSs match the density given by the
  more
  detailed EOSs ... to within 2 to 5 % for the pressure ranges P < 5 × 10^9 and
  P > 3 × 10^13 Pa. At intermediate pressures, the discrepancy ranged from less
  than 1 %
  to 12 %."
- The crossover pressure is roughly the bulk modulus, eq. (12): K ≈
  (ρ0/c)^(1/n).
  Check (computed here): Fe 1192 GPa, MgSiO3 693 GPa, H2O 114 GPa, which equals
  the
  P1 column of Table 4 (1192, 693, 114 GPa).
- The abstract/section 5 state that the generic mass-radius relation holds up to
  about
  20 Earth masses.

Consequence for the target mass range (0.01-20 M_Earth): central pressures
of an Earth-mass planet are about 360 GPa (PREM), 20 M_Earth reach several TPa,
i.e.
the range where the fit error is 1-12 %; a density error of 5 % in the core
gives about
1.7 % in radius (R ∝ ρ^(-1/3), the paper's own argument).

### 1.2 Underlying EOS (what the polytrope approximates)

Table 1 of the paper ("Parameters for the Vinet (V) or Birch-Murnaghan (BME) EOS
fits"):

| Material | K0 [GPa] | K0' | ρ0 [Mg m^-3] | Fit | log10(P_V/T) | Ref. in paper |
| --- | --- | --- | --- | --- | --- | --- |
| C (graphite) | 33.8 +/- 3 | 8.9 +/- 1.0 | 2.25 | BME | 11.75 | Ahrens 2000; Hanfland 1989 |
| Fe (α) | 162.5 +/- 5 | 5.5 +/- 0.8 | 7.86 | BME | - | Ahrens 2000; Takahashi & Spain 1989 |
| Fe (ε) | 156.2 +/- 1.8 | 6.08 +/- 0.12 | 8.30 | Vinet | 13.32 | Anderson et al. 2001 |
| FeS | 35 +/- 4 | 5 +/- 2 | 4.77 | BME | 13.23 | Ahrens 2000; King & Prewitt 1989 |
| H2O (ice VII) | 23.7 +/- 0.9 | 4.15 +/- 0.07 | 1.46 | BME | - | Hemley et al. 1987 |
| H2O (liquid, seawater 12 K as printed) | 2.28 | - | - | logarithmic | - | Halliday et al. 2003 |
| MgO | 177.0 +/- 4 | 4.0 +/- 0.1 | 3.56 | BME | 12.8 | Duffy et al. 1995 |
| MgSiO3 (enstatite) | 125 | 5 (assumed) | 3.22 | BME | - | Ahrens 2000; Olinger 1977 |
| MgSiO3 (perovskite) | 247 +/- 4 | 3.97 | 4.10 | 4th-order BME, K0'' = -0.016 GPa^-1 | 13.13 | Karki et al. 2000 |
| (Mg0.88,Fe0.12)SiO3 (pv) | 266 +/- 6 | 3.9 +/- 0.04 | 4.26 | BME | 12.74 | Ahrens 2000; Knittle & Jeanloz 1987 |
| SiC | 227 +/- 3 | 4.1 +/- 0.1 | 3.22 | BME | 11.4 | Ahrens 2000; Alexsandrov 1989 |

Switch points to Thomas-Fermi-Dirac (TFD, Salpeter & Zapolsky 1967 form),
Section 3.3:
Fe(ε) Vinet up to 2.09e4 GPa (fit data only to 330 GPa); MgSiO3-pv 4th-order BME
up to
1.35e4 GPa; H2O: BME of Hemley 1987 up to 44.3 GPa, then density-functional
(DFT)
data for ice VIII/X (Table 2 of the paper, 26 points, 2.32 to 7686 GPa), then
TFD
above 7686 GPa. Their Vinet fit to the combined ice VIII/X DFT data: ρ0 = 1460
kg m^-3,
K0 = 14.3771 GPa, K0' = 6.57972, deviating from the table by <2.5 % in density;
the DFT
data agree with ice VII experiments (Hemley 1987, 6 to 127 GPa) within 3.5 % in
density.
Liquid water, P <= 10 GPa: logarithmic EOS with K0 = 2.28 GPa.
(The silicate EOS is pure lower-mantle perovskite: there is no upper mantle, no
post-perovskite, no iron-alloy core.)

### 1.3 Scaled mass-radius relation (Section 5.2)

Scaled variables (eqs. 13 to 19): ρ_s = ρ/ρ0, P_s = P/P1, r_s = r/r1, m_s = m/m1
with
P1 = (ρ0/c)^(1/n), r1 = G^(-1/2) ρ0^(1/(2n) - 1) c^(-1/(2n)), m1 = r1^3 ρ0;
EOS becomes ρ_s = 1 + P_s^n (eq. 22). For M_s < 4 (eq. 23):

    log10 R_s = k1 + (1/3) log10 M_s - k2 M_s^k3, k1 = -0.20945, k2 = 0.0804, k3
    = 0.394

"this approximation is good to 1 % over this range". Table 4 gives, for the
modified
polytrope, M_s up to about 40 with material-specific k_i, and the conversion
factors:

| Material | m1 [M_E] | r1 [R_E] | P1 [GPa] | k1 | k2 | k3 |
| --- | --- | --- | --- | --- | --- | --- |
| Fe (modified polytrope) | 5.80 | 2.52 | 1192 | -0.209490 | 0.0804 | 0.394 |
| MgSiO3 (modified polytrope) | 10.55 | 3.90 | 693 | -0.209594 | 0.0799 | 0.413 |
| H2O (modified polytrope) | 5.52 | 4.43 | 114 | -0.209396 | 0.0807 | 0.375 |
| Fe (full EOS, effective) | 4.34 | 2.23 | - | - | - | - |
| MgSiO3 (full EOS) | 7.38 | 3.58 | - | - | - | - |
| H2O (full EOS) | 8.16 | 4.73 | - | - | - | - |
| Fe0.675/MgSiO3 0.325 as printed | 6.41 | 3.19 | - | - | - | - |
| Fe0.3/MgSiO3 0.7 as printed | 6.41 | 2.84 | - | - | - | - |
| Fe0.225/MgSiO3 0.525/H2O 0.25 | 6.41 | 3.63 | - | - | - | - |
| Fe0.065/MgSiO3 0.485/H2O 0.45 | 6.88 | 4.02 | - | - | - | - |
| Fe0.03/MgSiO3 0.22/H2O 0.75 | 7.63 | 4.42 | - | - | - | - |

(Table 4 of the arXiv text. The mixed-composition rows are as printed; the first
two
mixed rows carry composition labels that look swapped relative to the Fe
fraction/radius
ordering, so check against the journal before using them. The conversion factors
are
effective values fitted to the full-EOS models, M_s < 4.)

Stated check for Earth: "our simple fiducial model matches the Earth's radius to
within
three percent for a 1 M_Earth planet" (Section 4.1, 32.5 % Fe core, 67.5 %
MgSiO3 mantle).
The paper does not report a moment of inertia.

---

## 2. Better or complementary EOS (parameters read in the sources)

### 2.1 Iron (ε/hcp-Fe)

Table F.7 of Hakim et al. 2018 (Icarus 313, 61-78,
doi:10.1016/j.icarus.2018.05.005;
text read from the open copy at publi2-as.oma.be) collects the isothermal parts:

| Set | Form | ρ0 [kg m^-3] | K_T0 [GPa] | K'_T0 | Θ0 [K] | γ0 | P range of data [GPa] | Origin |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Dewaele et al. 2006 (PRL 97, 215504) | Vinet | 8270 | 163.4 | 5.38 | 417 | 1.875 | 17-197 | quasi-hydrostatic, 300 K |
| Bouchet et al. 2013 (PRB 87, 094102) | Holzapfel | 8878 | 253.8 | 4.719 | - | - | 30-1500 | ab initio |
| Valencia et al. 2007 (from Williams & Knittle 1997) | Vinet | 8300 | 160.2 | 5.82 | 998 | 1.36 [q=0.91] | 136-300 | |
| Seager et al. 2007 (from Anderson et al. 2001) | Vinet | 8300 | 156.2 | 6.08 | - | - | 0-330 | |
| Fei et al. 2016 (GRL 43, 6837) | BM3 | 8269 | 172.7 | 4.79 | 422 | 1.74 [q=0.78] | 18-280 | |
| Dorn et al. 2015 (from Belonoshko et al. 2010) | BM3 | 8341 | 173.98 | 5.297 | - | 2.434 [q=0.489] | 18-360 | |
| Wagner et al. 2011 (from Dewaele 2006) | modified Rydberg | 8269 | 149.4 | 5.65 | 430 | 1.875 | 17-197 | |

(Several cells of the original table, e.g. the Θ and γ columns for Bouchet and
Seager,
are blank or ambiguous in the text extraction; only the numbers I could read
unambiguously are given. Unterborn et al. 2016, Table 1, independently lists
ε-Fe Dewaele 2006: V0 = 6.75 cm3/mol, ρ0 = 8.27 g/cc, K0 = 163.4 GPa, K' = 5.38,
Vinet;
ε-Fe Anderson 2001: 6.72, 8.31, 156.2, 6.08 Vinet; liquid Fe (Anderson & Ahrens
1994):
V0 = 7.96 cm3/mol, ρ0 = 7.019 g/cc, K0 = 109.7 GPa, K' = 4.66, 4th-order BM.)

High-pressure closed form, Hakim et al. 2018, Section 3.1, eq. (5) (Holzapfel,
fitted to
their ab initio hcp-Fe set, 0.234 to 10 TPa, residual in volume below 1.1 %):

    P = P0 + 3 K_T0 x^(-5/3) (1 - x^(1/3)) [1 + c2 x^(1/3) (1 - x^(1/3))] exp[c0
    (1 - x^(1/3))]
    x = V/V0, V0 = 4.28575 cm3/mol, P0 = 234.4 GPa, K_T0 = 1145.7 +/- 3.6 GPa,
    c0 = 3.19 +/- 0.08, c2 = -2.40 +/- 0.05

(valid above 234 GPa; Hakim use the experimental hcp-Fe EOS below 234 GPa and
their DFT
results above it, "a discontinuity in K_T of about 10 %... has a negligible
effect".)
Their conclusions on extrapolation: Vinet and BM3 differ from the DFT results by
more
than 2 % already near 1 TPa and by more than 14 % at 10 TPa (Appendix A); the
density
spread between iron EOS from the literature is up to 4 % at 1 TPa and 20 % at 10
TPa,
which changes the planetary mass by up to 10 % (Earth-like, core radius fraction
0.5)
and 20 % (Mercury-like, 0.8) at fixed radius (abstract).
Hakim et al. also give Holzapfel fits of Sata et al. 2010 alloy data (their
Table D.5):
FeS ρ0 = 6118, K_T0 = 144.3 GPa, K' = 4.79; FeSi 6522, 219.5, 4.29; Fe0.95O
5764,
152.7, 4.13; Fe3C 7980, 289.4, 3.81 (ρ0 in kg m^-3; P_FG0 column also given
there).

Smith et al. 2018, Nature Astronomy 2, 452-458, doi:10.1038/s41550-018-0437-9:
ramp compression of Fe to 1.4 TPa (abstract, via the Princeton and LLNL record
and the
Nature page). The Vinet/K0/K0' values are in the paywalled full text; **not
found**.
I could not read them (the only open page I retrieved was the abstract). Do not
quote
Smith 2018 parameters until read.

PREM-based alternative (Zeng et al. 2016): see 2.3.

### 2.2 Silicates

Stixrude & Lithgow-Bertelloni (SLB) is the reference thermodynamic database:
SLB 2005 GJI 162, 610-632, doi:10.1111/j.1365-246X.2005.02642.x (physical
properties) and
SLB 2011 GJI 184, 1180-1213, doi:10.1111/j.1365-246x.2010.04890.x (phase
equilibria).
I did **not** obtain their parameter tables. The SLB-2005 zero-pressure values
for the
main phases are reproduced in Unterborn et al. 2016 (ApJ 819, 32,
doi:10.3847/0004-637X/819/1/32), Table 1:

| Mineral | V0 [cm3/mol] | ρ0 [g cm^-3] | K0 [GPa] | K' | Form used |
| --- | --- | --- | --- | --- | --- |
| MgSiO3 bridgmanite | 24.45 | 4.11 | 251 | 4.1 | SLB 3rd-order BM at 300 K |
| MgO (periclase) | 11.24 | 3.59 | 161 | 3.8 | idem |
| Mg2SiO4 (forsterite) | 43.6 | 3.23 | 128 | 4.2 | idem |
| MgSiO3 enstatite | 31.4 | 3.2 | 107 | 7.0 | idem |
| FeSiO3 (pv structure) | 25.49 | 5.18 | 272 | 4.1 | idem |
| FeO | 12.26 | 5.86 | 179 | 4.9 | idem |

Other silicate EOS read:

- Seager 2007 (Table 1 above): MgSiO3-pv K0 = 247 GPa, K0' = 3.97, K0'' = -0.016
  GPa^-1,
  ρ0 = 4.10 Mg m^-3 (Karki et al. 2000 DFT).
- Zeng et al. 2016 (ApJ 819, 127, doi:10.3847/0004-637X/819/2/127), Section 2,
  BM2
  (second-order Birch-Murnaghan, K0' = 4) fitted to PREM:
  P = (3/2) K0 [(ρ/ρ0)^(7/3) - (ρ/ρ0)^(5/3)];
  lower mantle ρ0 = 3.98 g/cc, K0 = 206 GPa (error about 1 % in ρ); outer core
  ρ0 = 7.05 g/cc, K0 = 201 GPa (about 1 %); inner core ρ0 = 7.85 g/cc, K0 = 255
  GPa
  (0.01 %). Mantle recipe: 0 to 23.83 GPa linear interpolation of PREM density
  (Stacey & Davis 2008, App. G); 23.83 GPa to 3.5 TPa BM2 above; > 3.5 TPa TFD.
  Core: BM2 outer-core fit to 12 TPa, TFD above. PREM jump at 23.83 GPa: 4 to
  4.4 g/cc
  (10 %).
- Zeng & Sasselov 2013 (PASP 125, 227-239, doi:10.1086/669163): MgSiO3 pv and
  ppv EOS
  from Caracas & Cohen 2008; pv-to-ppv transition at 122 GPa (Spera et al.
  2006),
  ppv about 1.5 % denser; ppv dissociation at 0.90 and 2.10 TPa (Umemoto &
  Wentzcovitch
  2011); TFD (Seager version, rescaled by 1.04437) above 4.90 TPa. The numerical
  parameters of Caracas & Cohen 2008 were **not found** in the text I read.
- Post-perovskite parameters (K0, K0', V0): **not found** (only cited by Zeng
  2013,
  Skinner 2026 uses Sakai et al. 2016 with a Keane/Debye form, no numbers in the
  text).

Why the upper mantle matters (Unterborn et al. 2016, Section 3.5, read): below
about
660 km depth (P < 25 GPa) the mantle is olivine/pyroxene; those phase
transitions give a
net density decrease of 14.8 % against only 8 % compression over that range, and
the upper
mantle is about 17 % of Earth's mass. Replacing the lower-mantle EOS by a 50/50
olivine and
pyroxene mix below 25 GPa lowers the 1-R_Earth, 100 % bridgmanite model from
1.10 to
1.04 M_Earth (a 6 % drop). A pure bridgmanite mantle is a systematic error for
planets of
Mars/Earth size.

### 2.3 Water and ice

| Phase / range | EOS | Parameters read | Source / table |
| --- | --- | --- | --- |
| Ice VII, 6-127 GPa | BME | K0 = 23.7 +/- 0.9 GPa, K0' = 4.15 +/- 0.07, ρ0 = 1.46 Mg m^-3 | Seager 2007 Table 1 (Hemley et al. 1987) |
| Ice VII/VIII/X, up to 7.7 TPa | Vinet fit to DFT | ρ0 = 1460 kg m^-3, K0 = 14.3771 GPa, K0' = 6.57972 (<2.5 % in density) | Seager 2007, Sect. 3.3 |
| Ice VII, 2.216-37.4 GPa (along melting curve) | exponential fit | ρ [mol cm^-3] = 0.0805 + 0.0229 (1 - exp(-0.0743 p)) + 0.1573 (1 - exp(-0.0061 p)), p in GPa | Zeng & Sasselov 2013 eq. (5), Frank, Fei & Hu 2004 (GCA 68, 2781, doi:10.1016/j.gca.2003.12.007) |
| Ice VII (37.4 GPa), X, superionic, to 8.893 TPa | DFT tables | French, Mattsson, Nettelmann & Redmer 2009 Table V (data points on the melting-curve path, e.g. 2.5 g/cm3 at 1000 K, 3.25-4.00 g/cm3 at 2000 K, 5-15 g/cm3 at 4000 K) | Zeng & Sasselov 2013, Sect. 3.3.3 |
| Ice Ih, III, V, VI (< 2.216 GPa) | Chaplin EOS | not tabulated in the text | Zeng & Sasselov 2013 |
| Liquid water, P <= 10 GPa | logarithmic EOS | K0 = 2.28 GPa | Seager 2007 |
| Whole phase diagram | tabular P-T EOS "AQUA" | Haldemann et al. 2020: Feistel & Wagner 2006 (Ih), Journaux et al. 2020 (II-VI), French & Redmer 2015 (VII, VII*, X), Wagner & Pruss 2002 (liquid/gas < 1200 K, < 1 GPa), Brown 2018, Mazevet et al. 2019 (superionic) | listed in Skinner et al. 2026 Sect. 2.4 |

Seager's ice VII Debye temperature: 300 K (Sect. 4.2.2, text under eq. 8).

Phase-boundary data (Zeng & Sasselov 2013 Sect. 3.3.1): Ih/III triple point
209.5 MPa,
251.15 K; III/V 355.0 MPa, 256.43 K; V/VI 618.4 MPa, 272.73 K; VI/VII 2.216 GPa,
355 K;
VII/X about 47 GPa, 1000 K (Goncharov et al. 2005); they follow the melting
curve.

### 2.4 Complementary: "sensitivity" and validation of EOS choices

- Skinner, Pudritz & Cloutier 2026 (arXiv:2604.15304, MNRAS preprint of 29 May
  2026)
  list the EOS forms they use but not the numbers in the text I read: hcp Fe
  Holzapfel (Hakim 2018) above 234.4 GPa and BM3 (Fei et al. 2016) below, Debye
  thermal term; liquid Fe Vinet (Ichikawa & Tsuchiya 2020; Kuwayama et al.
  2020);
  MgSiO3 ppv Keane + Debye (Sakai et al. 2016); FeSiO3 ppv BM3 (SLB 2024); MgO
  B1 Vinet
  (Ye et al. 2017), B2 Holzapfel; SiO2 Fe2P-type Vinet; Mg2SiO4 I-42d tabular
  (Umemoto
  et al. 2017). Their Table 2 to 5 give only forms and citations.
- Attia, Lichtenberg, Jungova & Sastre 2026 (PALEOS, A&A, arXiv:2605.03741)
  provide an
  open-source, phase-aware set of EOS tables (17 phases) for iron, MgSiO3 and
  AQUA water.

### 2.5 Accuracy of simple EOS in the Earth case (what to expect)

- Zeng 2016 BM2 fits: 1 % in density against PREM for lower mantle and outer
  core
  (by construction).
- Seager polytrope vs its own detailed EOS: 2-5 % and up to 12 %, section 1.1.
- Unterborn 2016: Fe + bridgmanite, T = 300 K, 3480 km core overpredicts mass by
  about
  13 % (core radius would have to be about 20 % smaller).

### 2.6 Recommended coherent set

| Layer | Pressure range | Recommended EOS | Parameters (read) | Why |
| --- | --- | --- | --- | --- |
| Core (solid/liquid Fe-rich) | 0 to 234 GPa | Fei et al. 2016 BM3 or Dewaele 2006 Vinet (isothermal 300 K), then thermal term | Dewaele: ρ0 8270, K 163.4, K' 5.38; Fei: ρ0 8269, K 172.7, K' 4.79 (Hakim Table F.7) | experimental, quasi-hydrostatic; Anderson 2001 (Seager) was non-hydrostatic and underestimates density (Unterborn Sect. 3) |
| Core | > 234 GPa | Hakim et al. 2018 Holzapfel | P0 = 234.4 GPa, V0 = 4.28575 cm3/mol, K_T0 = 1145.7 GPa, c0 = 3.19, c2 = -2.40 | the only closed form with correct high-P asymptotics among those compared (Hakim Sect. 3.3); within 3 % up to 10 TPa |
| Core light elements | all | scale ρ (e.g. Fe-S: ρ0 6118, K 144.3, K' 4.79 for pure FeS, Holzapfel fit) or mix with Fe | Hakim Table D.5 | pure Fe in an Earth-size core is too dense: Unterborn 2016, Valencia 2006 (FeO and Fe end members give -200 and +300 kg m^-3 against PREM) |
| Upper mantle | 0 to 23.83 GPa | olivine/pyroxene (SLB BM3) or PREM-linear (Zeng 2016) | Mg2SiO4 3.23 g/cc, 128 GPa, 4.2; enstatite 3.2, 107, 7.0 (Unterborn Table 1) | needed for Mars/Earth/Moon/Mercury-size bodies (6 % mass effect, Unterborn) |
| Lower mantle | 23.83 GPa to about 120 GPa | bridgmanite/periclase mix (SLB BM3) | bridgmanite 4.11 g/cc, 251 GPa, 4.1; MgO 3.59, 161, 3.8 | |
| Lower mantle (high P) | > 122 GPa to TPa | ppv + dissociation (Umemoto & Wentzcovitch 2011) then TFD | transition 122 GPa (Zeng 2013); parameters **not found** | only matters for M > about 2 M_Earth; ppv jump about 1.5 % |
| Ice VII / X | 2.2 GPa to 7.7 TPa | Seager Vinet/DFT: ρ0 1460, K0 14.3771 GPa, K0' 6.57972; or BME 23.7, 4.15 | Seager 2007 | one smooth closed form for the whole high-P range, at 300 K |
| Water < 2.2 GPa | | tabular (AQUA) or IAPWS 95 plus Chaplin / Journaux | not read | strongly T-dependent, the polytrope/Vinet fits do not capture it |

Justification: every row has at least two independent papers that compare it
with the
alternatives (Hakim 2018, Unterborn 2016, Skinner 2026, Attia 2026). The set is
not
fully derived from one paper; mixed provenance is unavoidable because SLB has no
iron and
the iron EOS papers have no silicate. If a single 300 K set is wanted, the
cheapest
defensible choice is Zeng 2016 (PREM-based BM2) for rock and iron plus Seager
ice,
which reproduces Earth by construction but degrades above 1 M_Earth (Skinner
2026,
section 5.2: the Zeng et al. curves predict increasingly smaller radii at high
mass and
pure-core radii several percent too large).

---

## 3. Temperature profile in the literature

### 3.1 Valencia, O'Connell & Sasselov 2006 (Icarus 181, 545-554, doi:10.1016/j.icarus.2005.11.021)

Text read: ar5iv HTML of arXiv:astro-ph/0511150.

- Structure equations (1)-(4): dρ/dr = -ρ g/φ, dg/dr = 4πGρ - 2Gm/r^3, dm/dr =
  4πr²ρ,
  dP/dr = -ρ g, with φ = K_S/ρ. Integration from the surface with 4th-order
  Runge-Kutta;
  the radius is found by iteration until no excess mass remains.
- EOS: 3rd-order Birch-Murnaghan with a Debye thermal correction (eqs. 5-7):
  K_T(ρ,T) = K_T(ρ,300) + ΔK_th, ΔK_th = 3nRγρ [f(T) - f(T0)], f(T) with
  Debye integral, γ = γ0 (ρ0/ρ)^q, θ = θ0 exp[(γ0 - γ)/q], K_S = K_T (1 + αγT).
- Adiabatic gradient in convecting regions (eq. 7): dT/dr = ρ g T γ / K_S.
- Boundary layers at the surface and at the core-mantle boundary (CMB),
  conductive:
  dT/dr = -q/k (eq. 6); thickness δ = a (D/2) (Ra/Ra_c)^(-1/4) (eq. 9) with
  Ra = ρ g α (q_s/k) D^4 /(κ η) (eq. 8); viscosity η(T) = η0 (T/T0)^(-n), n = 30
  (Davies 1980). CMB boundary layer = half of the surface layer thickness
  (stated as
  "chosen ad-hoc"). Heat flux scaled with mass, Q = Q_Earth M/M_Earth (eq. 10).
  Core mass fraction of Earth: 32.59 % (Stacey 1992). γ0 = 2.9 for all mantle
  materials.
- Core entries of their Table 1 (ρ0 [kg m^-3], K0,300 [GPa], K0', γ0, q, θ [K]):
  pure Fe (8171, 135, 6.0, 1.36, 0.91, 998) from Uchida et al. 2001 and Lin et
  al. 2003;
  FeO outer core (6200, 126, 4.8, 2.2, 1.62, 421) from Williams & Knittle 1997
  and
  Lin et al. 2003; Fe + alloy fitted to PREM (outer core 6800, 165, 4.2, 1.9, 1,
  998;
  inner core 7990, 200, 5.2, 1.734, 1, 998; the letter-coded footnotes say
  "assumed in order
  to fit density and bulk compressibility to PREM"); warm-geotherm test rows
  with γ0 = 2.9,
  q = 1. Mantle parameters from Stixrude & Lithgow-Bertelloni 2005 (values not
  printed in
  the text I read).
- Surface temperature: Mercury-like runs 440 K, 1500 K and 2000 K (Table 2 and
  Sect. 4);
  an Earth-run surface temperature is **not found** in the text read. Valencia
  et al.
  2007 (ApJ 665, 1413, arXiv:0704.3454) use T_surf = 500 K for super-Earths,
  citing
  GJ 876 d (Rivera et al. 2005, estimated surface temperature 430-650 K).
- Melting (Lindemann) for the iron alloy: `Tm = Tm0 (ρ0/ρ)^(2/3) exp{(2γ0/q)[1 -
  (ρ0/ρ)^q]}`.
- Reported thermal sensitivity (Sect. 3.2 and 4): exponent of R ∝ M^β for
  Earth-like
  planets 0.267-0.272; for the Mercury-like family at CMF = 65 %: β = 0.3058
  (T_surf =
  440 K), 0.2991 (1500 K), 0.3000 (2000 K) (Table 2). "Choosing different values
  for the
  thickness of the bottom boundary layer would make the core hotter or colder by
  no more
  than ~300 K, not enough to yield different scaling laws". Electronic pressure
  in
  metals is 8 % of the pressure at 350 GPa (Stixrude & Wasserman 1997), so
  omitted.

### 3.2 Sotin, Grasset & Mocquet 2007 (Icarus 191, 337-351, doi:10.1016/j.icarus.2007.04.006)

I could **not** obtain the paper's text (Elsevier redirect only; the arXiv
search hit
astro-ph/0701608 turned out to be Selsis et al. 2007, not this paper). Their
method and
numbers are described in two papers that I did read:

- Grasset, Schneider & Sotin 2009 (ApJ 693, 722-733,
  doi:10.1088/0004-637X/693/1/722;
  arXiv:0902.1640), which extends Sotin 2007 and states that "details of
  computation
  and parameter values can be found in Sotin et al. (2007)". Its **Table 1**
  (reference
  case, allowed random ranges):

| Layer | Quantity | Reference value | Range explored |
| --- | --- | --- | --- |
| Metallic core | upper temperature drop | 800 K | +/- 500 K |
| Metallic core | temperature profile | adiabatic | +/- 2 % |
| Lower silicate mantle | upper temperature drop | 300 K | +/- 200 K |
| Lower silicate mantle | profile | adiabatic | +/- 2 % |
| Upper silicate mantle | upper temperature drop | 1300 K | +/- 500 K |
| Icy mantle | upper temperature drop | 100 K | +/- 100 K |
| Liquid water | surface temperature | 300 K | +/- 200 K |
| All | EOS (core) ANEOS, lower mantle ANEOS above 100 GPa and MGD below, upper mantle 3rd-order BM | | |

  Structure equations (their eq. 2): same hydrostatic set with the adiabat
  dT/dP = γT/(K_S) (Grüneisen γ and seismic parameter Φ).

- Hakim et al. 2018 (Sect. 2.1, read): "temperature jumps of 800-2000 K used by
  Sotin et al. (2007), Valencia et al. (2007) and Wagner et al. (2011)" at the
  CMB.

### 3.3 Other adiabat/temperature choices, with numbers

- Hakim et al. 2018: dT/dP = γ T / K_S (eq. 3), isentrope in the whole mantle or
  isentrope in the upper mantle plus super-adiabatic lower mantle; footing
  temperature
  of the isentrope = surface temperature T_S = 1650 K (potential temperature,
  the
  lithosphere thickness is neglected); CMB temperature either continuous or set
  to the
  MgSiO3 melting temperature T = 1831 K (1 + P/4.6 GPa)^0.33 (Belonoshko et al.
  2005),
  which gives a jump of about 2500-7000 K; core either isentropic or isothermal.
  Einstein-model thermal pressure in the core (their eqs. 6-8) with Bouchet et
  al. 2013
  parameters.
- Dorn et al. 2015 (A&A 577, A83, arXiv:1502.03605): Earth-like mantle gradient
  with
  surface temperature fixed at 1000 K in their Appendix B; for iron the
  Mie-Grüneisen
  form γ = γ0 (ρ/ρ0)^(-q) with ρ0 = 8.334 g/cm3, q = 0.489, γ0 = 2.434; core
  adiabat
  dT/dP = γT/K_S, K_S = (1 + γαT) K_T. Variable mantle temperature changes
  density by
  < 1 % (CMB temperature 2100 to 3500 K).
- Attia et al. 2026 (PALEOS): T_pot = 1600 K (McKenzie & Bickle 1988) at 1 bar,
  adiabat dT/dr = (∇ad T/P) dP/dr, T_CMB = 4370 K as a free parameter ("narrow
  4350-4400 K window" for a dual-phase core).
- Skinner et al. 2026: adiabat below the atmosphere, plus an imposed temperature
  jump to
  the mantle melting temperature at the top and at the bottom of the mantle
  (their
  eqs. 22-23 not read in detail); radiative gradient only in the atmosphere (eq.
  45).
- Seager 2007 (Sect. 4.2.2): thermal pressure linear in T, P_th = α K0 (T - 300
  K)
  (eq. 9); for (Mg,Fe)SiO3 α K0 = 0.00692 GPa/K (Anderson & Masuda 1994); for
  Fe,
  P_th = α K0 (T - 300) + (∂αK0/∂T)_V (T - 300)^2 with α K0 = 0.00121 GPa/K and
  ∂(αK0)/∂T = 7.8e-7 GPa/K^2 (Isaak & Anderson 2003). Debye temperature Θ_D
  "around 700 K for silicate- and Fe-bearing minerals", 300 K for water ice,
  2000 K
  for carbon (text under eq. 8).

### 3.4 Do thermal corrections change the radius enough to matter?

| Statement | Number | Source |
| --- | --- | --- |
| MgSiO3 density decrease from 300 K to 6000 K (> 10 GPa, solid) | < 4 % | Seager 2007, Sect. 4.2.2, Fig. 13 |
| MgSiO3 density decrease below 10 GPa, up to 1200 K | < 2.5 % | idem |
| Fe density decrease at P > 100 GPa | < 4 % | idem |
| Ice VII density change (to 50 GPa, 800 K) | "less than a few percent" | idem |
| Example: mean density overestimated by 3.5 % | radius underestimated by 1.2 % | idem (R ∝ ρ^(-1/3)) |
| Earth-like and Mercury-like core isentropic vs isothermal | mass changes up to 1 % and 2 % | Hakim 2018, Sect. 5.1 |
| CMB temperature jump 2500-7000 K | mass -1 to -2 % (Earth-like), -2 to -5 % (Mercury-like); radius at fixed mass +1 %, +1.5 % | Hakim 2018, Sect. 5.1 |
| Super-adiabatic lower mantle | mass up to -3 % | Hakim 2018 |
| Effect on exponent of M-R relation | Δβ < 0.003 for all T cases | Hakim 2018 |
| Thermal profile variations by about 4000 K | radius difference up to 2 % (quoted from Grasset 2009) | Hakim 2018 |
| 90 random thermal profiles, 10 M_Earth | core and planet radius vary < 1.5 % | Grasset 2009, Sect. III.3 |
| CMB temperature 3500-5000 K | R changes < 0.3 % | Attia 2026, Sect. 4 |
| Surface temperature 1500 K and above, low-mass silicate planets | R > 1 % larger above 1500 K; up to 16 % at 4000 K (continuous solid-to-magma-ocean thermal expansion) | Attia 2026 abstract |
| Mercury-like family, T_surf 440 K to 1500 K | β 0.3058 to 0.2991 (about 2 % in the exponent) | Valencia 2006 Table 2 |

Conclusion: for T_surf < about 1000 K and solid interiors the adiabat is a 0.3-2
%
correction on R, below the composition uncertainties (core light elements, upper
mantle,
section 2.5), and the pressure term matters only through density (adiabatic
profile
affects ρ(P) by < 4 %). It matters for (a) water layers (strongly T-dependent
EOS, phase
changes), (b) hot, magma-ocean planets, and (c) the interior T/phase diagnostic
itself
(liquid vs solid core), not for the bulk radius of a cold rocky planet.

---

## 4. Validation data

All values are normalised C/MR^2 unless stated.

| Body | Quantity | Value | Source read | Notes |
| --- | --- | --- | --- | --- |
| Earth | C/MR^2 | 0.3307 | Williams 1994 as quoted in Margot et al. 2018 (arXiv:1806.02024, Sect. 5.2) | |
| Earth | C/MR^2 | 0.33072 (computed here from Skinner et al. 2026: model 0.33056 is "-0.048 %" from the REM1D value, PREM 0.33090 is "+0.055 %") | Skinner et al. 2026 Sect. 3.1, citing Moulik & Ekstrom 2025 | uncertainty 0.007 % (dominated by G) |
| Earth | C/MR^2 NASA fact sheet | 0.3308 | nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html | rounded |
| Earth | C/MR^2 PREM | 0.33090 | Skinner et al. 2026 Sect. 3.1 | PREM = Dziewonski & Anderson 1981 |
| Earth PREM | central density | 13.0885 g/cm3 | Rodriguez-Mozos & Moya 2022 (arXiv:2203.01065), Table 1 "according to PREM" | secondary reproduction of DA81; the original table was not read |
| Earth PREM | central pressure | 363.852 GPa | idem | |
| Earth PREM | inner-outer core boundary | radius 1221.5 km, pressure 328.851 GPa, density 12.7636 / 12.1663 g/cm3 (inner / outer side) | idem | |
| Earth PREM | core-mantle boundary | radius 3480 km, pressure 135.751 GPa, density 9.9035 / 5.5665 g/cm3 (core / mantle side, as printed), gravity 10.6823 m/s2 | idem | |
| Earth PREM | 670 km boundary | pressure 23.833 GPa, density 4.3807 / 3.9921 g/cm3 | idem | radius 5701 km |
| Earth PREM | core mass fraction | 0.325 | idem (also Zeng 2016 Table 1, 0.325 +/- 0.001) | |
| Earth PREM | mean density | 5.515 g/cm3 | idem | |
| Earth | core radius | 3480 km | idem; Dorn et al. 2015 Table 2 lists 3400 km | |
| Mars | C/MR^2 | 0.3638 +/- 0.0001 | Dehant et al. 2020 (arXiv:1910.03899), quoting Konopliv et al. 2016 | |
| Mars | C/MR^2 | 0.3634 | Skinner et al. 2026 App. E, "following Archinal 2018; Konopliv 2020; Khan 2022" | InSight-era value |
| Mars | C/MR^2 NASA fact sheet | 0.366 | NSSDC | older, rounded |
| Mars | liquid core radius | 1835 +/- 55 km, mean density 5955-6290 kg/m3, density jump at CMB 1690-2110 kg/m3 (entirely solid mantle) | Le Maistre et al. 2023 Nature 619, 733-737, abstract | Stahler 2021 (Science 373, 443) is cited in search results for 1830 +/- 40 km; not read here |
| Mars | core radius | 1794 +/- 65 km | Rodriguez-Mozos & Moya 2022 Table 4 (Rivoldini et al. 2011) | |
| Moon | C/MR^2 (solid Moon) | 0.392728 +/- 0.000012 | Williams et al. 2014, JGR Planets 119, 1546-1578, abstract (via Univ. of Arizona record) | |
| Moon | C/MR^2 (value adopted) | 0.393112 +/- 0.000012 | Skinner et al. 2026 App. E, "from Williams et al. 2014" | differs from the abstract value in the fourth digit; the two come from the same reference, so check the paper's tables |
| Moon | C/MR^2 NASA fact sheet | 0.394 | NSSDC | |
| Moon | C/MR^2 | 0.393 | Williams et al. 1996, quoted in Margot 2018 | |
| Moon | core | fluid outer core radius 200-380 km; inner core 0-280 km (0-1 % of mass) | Williams 2014 abstract | |
| Mercury | C/MR^2 | 0.346 +/- 0.009 (weighted mean of Margot 2012: 0.346 +/- 0.014 and Stark 2015: 0.346 +/- 0.011) | Margot et al. 2018 (arXiv:1806.02024), eq. 19 | Cm+cr/C = 0.425 +/- 0.016 |
| Mercury | C/MR^2 | 0.333 +/- 0.005 | Skinner et al. 2026 App. E, "following Genova et al. 2019" | a newer, lower value; the two disagree at 2 sigma |
| Mercury | outer (liquid) core radius | 2024 +/- 9 km (Table 5 mean/std 2023.66 +/- 9.09); about 2000 km in text | Margot et al. 2018 | total core radius incl. possible inner core; Dorn 2015 Table 2: 2020 km; Margot eq. 21 Rc/R = 0.8209, Rc = 2002 km for an example |
| Venus | C/MR^2 | 0.337 +/- 0.024 | Margot et al. 2021, Nature Astronomy 5, 676-683 (arXiv:2103.01504) Table 2 | bootstrap 0.3373; least squares 0.3350 |
| Venus | core radius | about 3500 km (58 % of R), uncertainty > 500 km; 3508 km for a core density of 10358 kg/m3 | Margot 2021 | two-layer uniform-density estimate; 3609 km (value suggested in the brief): **not found** in the paper |
| Ganymede | C/MR^2 | 0.3105 +/- 0.0028 | Gao & Stevenson 2013 (Icarus 226, 1185, Table 1, citing Anderson et al. 1996) | |
| Ganymede | C/MR^2 | 0.3115 +/- 0.003 | Rodriguez-Mozos & Moya 2022 Table 5, citing Schubert et al. 2004 | the two sources differ by 0.001 |
| Callisto | C/MR^2 | 0.3549 +/- 0.0042 | Gao & Stevenson 2013 Table 1 (Anderson et al. 2001) | |
| Titan | C/MR^2 | 0.3414 +/- 0.0005 | Gao & Stevenson 2013 Table 1, citing Iess et al. 2010 (Science 327, 1367, doi:10.1126/science.1182583) | this is the SOL1 value; the original abstract gives about 0.34 |
| Europa | C/MR^2 | 0.3547 +/- 0.0024 (newer), 0.346 +/- 0.005 (Anderson 1998) | Skinner 2026 App. E (Gomez Casajus et al. 2021) | |

Notes:

- The Venus and Mercury numbers are coarse; do not tune the planet model to
  them.
- The NASA NSSDC fact sheet values (Earth 0.3308, Mars 0.366, Moon 0.394, Venus
  0.337,
  Mercury 0.35) are rounded and appear to mix old sources; use the papers above.
- Core radii of Mars, Venus, Mercury and the Moon are model dependent
  (composition,
  light elements); for validating a 2- or 3-layer model with pure Fe, expect a
  core that
  is too small (see section 5).

---

## 5. Reported results of integrated models for Earth and other bodies

| Model | EOS and thermal treatment | Earth result | Other results | Source |
| --- | --- | --- | --- | --- |
| Seager 2007 | Vinet/BME + TFD, 300 K, Fe + MgSiO3-pv, no upper mantle | radius within 3 % for 1 M_Earth, CMF 32.5 % (also tested an FeS core + 90 % (Mg,Fe)SiO3 + 10 % MgO mantle, which "agrees closely") | no MoI given | Seager 2007 Sect. 4.1 |
| Valencia 2006 | BM3 + Debye, adiabat + boundary layers | pure Fe core: density excess about +300 kg/m3, FeO core: deficit about -200 kg/m3 against PREM; pressure error about 16 % and 10 % | scaling R ∝ M^0.267-0.272 | Valencia 2006 Sect. 3.2 |
| Swift 2009 (arXiv:0908.3294, LLNL-TR-414058) | Fe + basalt EOS, core radius adjusted to fit mass and radius | linear-Grüneisen EOS: C/MR^2 = 0.3290, core radius 3361 km, central pressure 390 GPa; tabular EOS: 0.3118, 3405 km, 448 GPa; reference 0.33 and 3481 km | Mercury 0.3196/0.3140 (ref 0.33), Venus 0.3348/0.3146 (0.33), Moon 0.3647/0.3572 (0.393), Mars 0.3621/0.3316 (0.366); linear-Grüneisen within 3 % except Moon -7 %, tabular 4-5 % low except Moon and Mars (9 %, 10 %) | Swift 2009 Table II and III |
| Unterborn 2016 | Fe + bridgmanite, BurnMan, isothermal 300 K, 3480 km core | mass overpredicted by about 13 % at R = 1 R_Earth; 1.10 M_Earth for pure bridgmanite, 1.04 with 25 GPa upper mantle | core would have to be about 20 % smaller | Unterborn 2016 Sect. 3, 3.5 |
| Zeng 2016 | PREM-based BM2 fits | 1.005 R_Earth at 1 M_Earth for 30 % Fe (Table 2); CMF fit of Earth 0.33 | formula R = (1.07 - 0.21 CMF) M^(1/3.7), 1-8 M_Earth, 0 <= CMF <= 0.4, agrees within about 0.01 R_Earth | Zeng 2016 Table 2, eq. 2 |
| PALEOS (Attia 2026) | Fe + MgSiO3 phase-aware, T_pot = 1600 K, T_CMB = 4370 K, CMF 0.325 | R = 6350 km vs 6371 (-0.33 %); CMB at 3356 km (-124 km); CMB pressure 142.8 GPa (+5 %); central pressure 451 GPa (+24 % vs 364); central T 7055 K | lower-mantle density within 3 %; upper mantle about 20 % off (pyroxene as olivine proxy) | PALEOS Sect. 4 |
| Skinner 2026 | multi-species, phase-aware, adiabat plus melting-temperature jumps, core light elements fitted | R = 1.0015 R_E (+0.15 %); with simple parameters (CMF 0.325, no water, no rotation) 1.0008 R_E (+0.08 %); C/MR^2 = 0.33056 (-0.048 %), simple case 0.33083 (+0.035 %) | Mars R +0.24 % / -0.60 % / +0.95 % / +0.13 % for four composition choices, C = 0.36209 (-0.361 %) to 0.35505 (-2.30 %); Moon R +0.49 to +0.94 %, C 0.39427 to 0.39445 (+0.29 to +0.34 %); Venus R -0.016 %, C = 0.33491 (-0.62 %); Mercury R -0.17 to -1.31 %, C 0.33575 to 0.33744 (+0.28 to +1.33 %); summary "within 0.5 % or 2 sigma for Earth, Mars, Moon, Venus, Mercury; Europa within 1 % or 3 sigma" | Skinner 2026 Sect. 3 |
| Rodriguez-Mozos & Moya 2022 (A&A, arXiv:2203.01065) | PREM-based layer densities, uniform layers with an EOS-derived mean density | Mercury, Venus reproduced with errors "minimal" (mass 0.006 %, density 0.001 %); "errors are greater" for Mars | ice satellites: Ganymede MoI error 0.032 %, Europa 0.003 %, Callisto 0.004 % (model reproduces observed C through the free core radius) | its Sect. 2.5, Table 3-5 (the MoI error is for the model, not an independent test) |

The Skinner et al. 2026 statement "to the best of our knowledge, the closest
replication
of Earth's radius in any interior structure model built for application to
exoplanets
that is not explicitly constructed using Earth's mantle density profile" is the
best
available benchmark: 0.08-0.15 % in R and 0.03-0.05 % in C/MR^2. A simple
Fe + MgSiO3 model reaches R within about 0.3 % (PALEOS) but has central pressure
24 %
too high, a core too small by about 4 % in radius and C/MR^2 several percent too
low
(Swift: 0.31 to 0.33). The MoI is therefore a more sensitive validation target
than R.

### 5.1 Check computed here: Seager-polytrope, two-layer, hydrostatic integration

I integrated dm/dr and dP/dr outward from the centre with RK4 (1 km step) using
the
**Table 3 polytropes of Seager 2007** (ρ = ρ0 + c P^n, Fe, MgSiO3, H2O), layer
masses fixed
by the chosen mass fractions, central pressure found by bisection so that the
total mass
matches, surface at P = 0, and C = (8π/3)∫ρ r^4 dr / (M R^2). The script is
`/private/tmp/claude-501/-Users-cyborg-Developer-map-generator/c435d95e-c8f6-48a9-849d-6888e732f84b/scratchpad/calc/polytrope.py`.
This is my calculation, not a literature value.

| Case (mass, layer mass fractions) | R [km] | Observed R | Central P [GPa] | Core (or Fe) top radius [km] | C/MR^2 | Observed C/MR^2 |
| --- | --- | --- | --- | --- | --- | --- |
| Earth, 5.9722e24 kg, Fe 0.325 / MgSiO3 0.675 | 6112 (-4.1 %) | 6371 | 391 (PREM 363.852, secondary source) | 3398 (PREM 3480) | 0.3396 | 0.3307 to 0.3308 |
| Mars-like, 6.417e23 kg, Fe 0.24 / MgSiO3 0.76 | 3094 (-8.7 %) | 3389.5 | 62 | 1559 (InSight liquid core 1835 +/- 55, but that core has light elements) | 0.3566 | 0.3634 to 0.3638 |
| Mercury-like, 3.301e23 kg, Fe 0.70 / MgSiO3 0.30 | 2249 (-7.8 %) | 2439.4 to 2439.7 | 50 | 1809 | 0.3434 | 0.346 +/- 0.009 or 0.333 +/- 0.005 |
| Ganymede-like, 1.4819e23 kg, Fe 0.065 / MgSiO3 0.485 / H2O 0.45 | 2429 (-7.8 %) | 2632 to 2634 (Table 5 of Rodriguez-Mozos & Moya; Gao & Stevenson) | 14 | Fe 636, silicate top 1609 | 0.3107 | 0.3105 +/- 0.0028 or 0.3115 +/- 0.003 |
| 10 M_Earth, Fe 0.325 / MgSiO3 0.675 | 11742 (1.843 R_E) | not applicable | 3587 | 6208 | 0.3194 | not applicable |
| 0.01 M_Earth, Fe 0.325 / MgSiO3 0.675 | 1406 | not applicable | 12 | 806 | 0.3523 | not applicable |

Reading (all computed here): with the unmodified Seager polytropes (a) Earth's
radius
is 4 % small (Seager quotes "within three percent" with the full EOS), the core
is
too small by 82 km and C/MR^2 is 2.7 % too high, (b) radii of the small bodies
are
8-9 % too small because the polytrope has ρ0 = 4100 (silicate) and 1460 (water)
at zero
pressure, much denser than a real upper mantle (about 3300) or ice Ih (about
920), so
the "ρ0 + c P^n" form cannot reach the low surface densities of small bodies;
(c) the integral C/MR^2 from layered integration comes out within a few percent
of the
observations when the mass fractions are right (Ganymede-like 0.3107 vs
0.3105-0.3115).
The 10 M_Earth case gives R = 1.84 R_E (Fe 0.325 composition), plausible against
Table 2
of Zeng 2016 (rock 1.88 R_E at 8 M_Earth, 2.22 at 16; the 30 % Fe column is 1.75
and 2.06),
but those are different EOS. Bisection tolerance: total masses are within 0.3 %
of the
targets (Mercury-like 3.3053e23, Ganymede-like 1.4864e23 kg).

---

## 6. Items not verified / not found

- Seager 2007: the ApJ-printed Tables 3/4 (I read the arXiv v1 text); the iron
  label
  "Fe(α)" in the arXiv table is, from ρ0, the ε-Fe fit; the text refers to
  "Table 4" where
  Table 3 is meant; mixed-composition rows of Table 4 look mislabeled.
- Smith et al. 2018 (Nature Astronomy 2, 452): Vinet parameters not accessible
  (paywalled; only the abstract). Not found.
- Stixrude & Lithgow-Bertelloni 2005/2011/2024 parameter tables: not read. The
  partial
  values in section 2.2 are quoted from Unterborn et al. 2016 Table 1.
- Post-perovskite K0, K0', V0 (Caracas & Cohen 2008; Sakai et al. 2016): not
  found.
- Sotin, Grasset & Mocquet 2007: paper not read; numbers from Grasset 2009 and
  Hakim 2018.
- Valencia 2006: Earth surface temperature not found; mantle table (SLB 2005
  values)
  not printed in the text read; the data of Table 1 were garbled in places by
  the
  HTML extraction.
- PREM original table (Dziewonski & Anderson 1981,
  doi:10.1016/0031-9201(81)90046-7): not
  read; values are from Rodriguez-Mozos & Moya 2022 Table 1.
- Mars Moment of inertia: Konopliv 2020 (GRL) not read; 0.3638 (via Dehant 2020
  quoting
  Konopliv 2016) and 0.3634 (via Skinner 2026) are secondary. Stahler 2021 core
  radius
  1830 +/- 40 km not read (download blocked, HTTP 429); Le Maistre 2023 abstract
  read.
- Moon C/MR^2: Williams 2014 table not read; abstract 0.392728 (solid Moon) and
  Skinner's
  0.393112 differ.
- Mercury core radius: no single total-core radius with uncertainty was
  extracted;
  outer-core radius 2024 +/- 9 km is from Margot 2018's model ensemble.
- Ganymede 0.3115 requested: found as 0.3115 +/- 0.003 (Schubert 2004 via
  Rodriguez-Mozos
  & Moya) and as 0.3105 +/- 0.0028 (Anderson 1996 via Gao & Stevenson). Titan
  0.341 found
  as 0.3414 +/- 0.0005.
- The Skinner 2026 and Attia 2026 papers are arXiv preprints (2026), not yet
  peer-reviewed
  versions as far as the files I read state.

---

## 7. Sources list (with DOI where verified by Crossref)

| Short name | Full citation | DOI / id |
| --- | --- | --- |
| Seager 2007 | Seager, Kuchner, Hier-Majumder, Militzer, ApJ 669, 1279 | 10.1086/521346; arXiv:0707.2895 |
| Zeng & Sasselov 2013 | PASP 125, 227 | 10.1086/669163; arXiv:1301.0818 |
| Zeng 2016 | Zeng, Sasselov, Jacobsen, ApJ 819, 127 | 10.3847/0004-637X/819/2/127; arXiv:1512.08827 |
| Valencia 2006 | Valencia, O'Connell, Sasselov, Icarus 181, 545 | 10.1016/j.icarus.2005.11.021; arXiv:astro-ph/0511150 |
| Valencia 2007 | ApJ 665, 1413 | arXiv:0704.3454 |
| Sotin 2007 | Sotin, Grasset, Mocquet, Icarus 191, 337 | 10.1016/j.icarus.2007.04.006 (not read) |
| Grasset 2009 | Grasset, Schneider, Sotin, ApJ 693, 722 | 10.1088/0004-637X/693/1/722; arXiv:0902.1640 |
| Hakim 2018 | Hakim et al., Icarus 313, 61 | 10.1016/j.icarus.2018.05.005 |
| Unterborn 2016 | Unterborn, Dismukes, Panero, ApJ 819, 32 | 10.3847/0004-637X/819/1/32; arXiv:1510.07582 |
| Dorn 2015 | Dorn et al., A&A 577, A83 | arXiv:1502.03605 |
| Smith 2018 | Smith et al., Nature Astronomy 2, 452 | 10.1038/s41550-018-0437-9 (abstract only) |
| Dewaele 2006 | PRL 97, 215504 | 10.1103/PhysRevLett.97.215504 (values via Hakim Table F.7) |
| Frank 2004 | Frank, Fei, Hu, GCA 68, 2781 | 10.1016/j.gca.2003.12.007 (formula via Zeng 2013) |
| SLB 2005 / 2011 | GJI 162, 610; GJI 184, 1180 | 10.1111/j.1365-246X.2005.02642.x; 10.1111/j.1365-246x.2010.04890.x (tables not read) |
| Skinner 2026 | Skinner, Pudritz, Cloutier, MNRAS preprint | arXiv:2604.15304 |
| Attia 2026 | Attia, Lichtenberg, Jungova, Sastre, A&A preprint (PALEOS) | arXiv:2605.03741 |
| Swift 2009 | Swift, LLNL-TR-414058 | arXiv:0908.3294; 10.2172/958178 |
| Margot 2018 | Margot, Hauck, Mazarico, Padovan, Peale, in Mercury: The View after MESSENGER | arXiv:1806.02024 |
| Margot 2021 | Nature Astronomy 5, 676 | 10.1038/s41550-021-01339-7; arXiv:2103.01504 |
| Williams 2014 | JGR Planets 119, 1546 | 10.1002/2013JE004559 |
| Le Maistre 2023 | Nature 619, 733 | 10.1038/s41586-023-06150-0 |
| Dehant 2020 | LaRa instrument paper | arXiv:1910.03899 |
| Gao & Stevenson 2013 | Icarus 226, 1185 | 10.1016/j.icarus.2013.07.034; arXiv:1309.1205 |
| Iess 2010 | Science 327, 1367 | 10.1126/science.1182583 |
| Rodriguez-Mozos & Moya 2022 | A&A | arXiv:2203.01065 |
| Dziewonski & Anderson 1981 | PEPI 25, 297 | 10.1016/0031-9201(81)90046-7 (table not read) |
