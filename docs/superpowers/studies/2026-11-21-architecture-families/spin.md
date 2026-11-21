# Literature study: primordial spin of a drawn planet

Date: 2026-11-21. Same rule as the sibling files: a number is listed only if it
was read in a source text during this study, with the reading channel stated:

- **Full text**: the arXiv PDF was downloaded and text-extracted, and the
  passage was read.
- **Abstract (search)**: only a search-engine rendering was read.
- **Second-hand**: quoted by a full-text source that was read.
- **Not reached**: not opened. Nothing is claimed from it.

Page numbers `pN` are PDF pages of the arXiv file that was read. Numbers marked
**(derived)** are my arithmetic on sourced inputs. Scope: the obliquity and the
rotation period a planet is born with, rocky and giant, for the system draw's
`Spin { rotation_period, obliquity }` at the present date. Tidal locking,
despinning and the evolution of the obliquity since birth are out of scope
(track T2, #604): this study gives the starting point only.

| Short name | Reference | Channel |
| --- | --- | --- |
| Kokubo & Genda 2010 | ApJ 714, L21, arXiv:1003.4384 | Full text |
| Miguel & Brunini 2010 | MNRAS, arXiv:1004.1406 | Full text |
| Bryan 2018 | Bryan et al., Nature Astronomy, arXiv:1712.00457 | Full text |
| Bryan 2020 | Bryan et al., ApJ, arXiv:2010.07315 | Full text |
| Kokubo & Ida 2007, Agnor 1999 | giant-impact spins | Second-hand (cited by Kokubo & Genda p11 and Miguel & Brunini p9) |
| Laskar & Robutel 1993 | chaotic obliquity | Second-hand (cited by Miguel & Brunini p10) |

## 1. What `legacy-final` did

Read in `git show legacy-final:<file>`; none of it is carried over, this is only
the inventory of values the new draw replaces.

| Quantity | Legacy value | Source in the code |
| --- | --- | --- |
| Rocky obliquity, drawn | 5 to 40 deg, widened to 0 to 85 deg without a large moon (`moons.rs` `UNSTABLE_TILT_DEG`; `docs/science/axial-tilt-climate.md`, "How it computes them" 1) | Range only. The 0 to 85 deg cites Laskar, Joutel & Robutel 1993 for the chaotic zone of a moonless Earth. No source for 5 to 40 deg or for a shape. |
| Rocky rotation | log-uniform 12 to 36 h (`moons.rs`, `drawn_day`), or the period of the largest moon | None in the code. |
| Giant rotation | gas 9 to 12 h uniform, ice and steam 14 to 19 h (`giant_world.rs` around line 657) | "Jupiter's and Saturn's 10 hours, the ice giants' 16 to 17" (comment). Present-day values, not primordial. |
| Giant obliquity | the same 5 to 40 deg draw (a giant's moon "keeps the giant's tilt") | None. |
| Tides | despun to 0 deg or resonant | `spin.rs`: out of scope here (T2). |

Everything legacy drew was a present-day-looking range with no primordial
source. The new draw starts from the formation physics below.

## 2. Rocky planets: giant-impact spin

### 2.1 Kokubo & Genda 2010 (full text)

N-body runs of protoplanets with an accretion condition fitted to SPH collision
experiments, 50 runs per model, gas-free, standard disk (p2 abstract, p10).
The "realistic" model is the one to use: about half of collisions are
hit-and-run (p10: 49 % on average) and the spins are slower than with perfect
accretion.

- **Obliquity** (p11, p12): "the obliquity ranges from 0 to 180 degrees and
  follows an isotropic distribution `n dε = (1/2) sin ε dε`". K-S probabilities
  0.3 (realistic) and 0.9 (perfect) (p12). Reason given (p12): impacts are
  three-dimensional and add equal random contributions to each component of
  the spin angular momentum. The spin anisotropy parameter is β = 0.40
  (realistic) and 0.31 (perfect), against 1/3 for isotropy (Table 2, p19). The
  paper reads that as consistent with isotropy, citing "β ≃ 1/3" (p12).
- **Spin rate** (p11): each Cartesian component of ω "follows a Gaussian
  distribution" (K-S probability above 0.5 in both models). The average
  angular velocity "is almost independent of mass" (p11, Fig. 3a). Table 2
  (p19), in rad per hour (the unit is written hr⁻¹; ωcr = 3.3 hr⁻¹ at 3 g cm⁻³
  is `sqrt(GM/R³)` of p8, eq. 5, so the unit is radians per hour):

  | Model | mean ωx, ωy, ωz | σx, σy, σz | RMS of ω |
  | --- | --- | --- | --- |
  | Realistic | 0.05, 0.08, 0.07 | 1.32, 1.26, 1.49 | 2.35 |
  | Perfect | 0.42, -0.09, -0.16 | 2.15, 2.17, 2.03 | 3.69 |

  The RMS of the realistic model is "about 30 % smaller" than the perfect one
  (p11), and the perfect one is "as large as" the critical rate ωcr (p11).
- **Critical rate** (p8, eq. 5): `ωcr = (GM/R³)^(1/2) = 3.3 (ρ / 3 g cm⁻³)^(1/2)
  hr⁻¹`.
- **Anchor** (p12): for an Earth-mass planet the RMS 2.35 hr⁻¹ is a spin
  angular momentum 2.7 times that of the Earth-Moon system, "reasonably within
  the distribution of ω".

Derived: the RMS 2.35 rad/h is a period of `2π / 2.35 = 2.7 h` **(derived)**.
With isotropic Gaussian components of equal σ, `σ = 2.35 / √3 = 1.36 rad/h`
**(derived)**, which sits among the three tabulated σ (1.26 to 1.49), and `|ω|`
is then Maxwell distributed, with median `1.538 σ ≈ 2.1 rad/h`, a period of
about 3.0 h **(derived)**. The ratio σ / ωcr(3 g cm⁻³) is `1.36 / 3.3 = 0.41`
**(derived)**.

Caveats stated or visible in the paper: gas-free, one standard disk, about
3 × 10⁸ yr runs, planets of about 0.7 to 1.2 Earth masses (p10, Fig. 2). The
paper itself says the Earth-Moon angular momentum "is not a typical value of
the realistic accretion model" (p12), and the post-impact Moon-forming event is
a single draw of this distribution.

### 2.2 Miguel & Brunini 2010 (full text)

Semi-analytic oligarchic growth plus embryo collisions, 1000 discs, planets
below 10 Earth masses, perfect accretion (p1 abstract, p2).

- **Obliquity** (p1 abstract, p9, p10): "isotropic ... which means that planets
  can rotate in direct or indirect sense, regardless of their mass". It
  confirms Agnor 1999 and Kokubo & Ida 2007 (p9), and notes that the
  near-zero obliquities of the Solar System rocky planets (except Venus) are
  not primordial, since the chaotic zone of their obliquities is large
  (Laskar & Robutel 1993, p10, second-hand).
- **Rotation period** (p1 abstract): "most of the planets have rotation periods
  between 10 and 10000 hours"; p6: most reach periods above about 10 h, with a
  sizeable population between 0.1 and 10 h and "really rare" below 0.1 h
  (above the critical rate); p10: "very rare" below about 0.5 h. The period
  depends on the semi-major axis of formation (p1, p10). A planet that only
  accreted planetesimals would spin at `P_ord ≃ 150 h (ρ / 3 g cm⁻³)^(1/15)
  (a / 1 au)^(1/10) ... (M⋆ / M☉)^(-1/6)` (eq. 16, p4), 150 h at 1 au round
  the Sun. The short periods "need one or more impacts" (p10).
- It gives no closed-form distribution of the period.

### 2.3 Where the two disagree

Kokubo & Genda put the typical rocky spin at about 3 h (median, derived), with
the slow tail cut by the Gaussian. Miguel & Brunini put most planets between
10 and 10 000 h. The difference is the model: Kokubo & Genda follow giant
impacts of Moon-to-Mars-sized protoplanets that deliver most of the angular
momentum, while Miguel & Brunini include slow planetesimal accretion for
embryos that suffer few impacts (p10: those with the longest periods "acquired
them mainly by the accretion of planetesimals"). Miguel & Brunini state that
the planets looking like the Solar System terrestrial planets needed impacts
(p10). Neither is a measurement.

## 3. Giant planets: a gas-accreting spin

### 3.1 Bryan 2018 (full text)

Rotation rates of five young planetary-mass companions (ROXs 42B b,
GSC 6214-210 b, VHS 1256-1257 b, beta Pic b, 2M1207-3932 b) against six brown
dwarfs under 20 Jupiter masses.

- A giant accreting gas with no braking "should spin up to rotation rates
  approaching break-up"; the Solar System gas giants rotate "3-4 times more
  slowly than their primordial break-up velocities", possibly through magnetic
  coupling to a circumplanetary disk (p1).
- Error-weighted mean fraction of the break-up velocity: planetary-mass
  companions 0.137 ± 0.058, brown dwarfs 0.114 ± 0.046, both together
  0.126 ± 0.036 (p3). These errors are uncertainties of the mean, not the
  spread of the population.
- No correlation between the rotation rate and the mass in 1 to 20 Jupiter
  masses (Pearson coefficient -0.0788), Jupiter and Saturn included; the ice
  giants and terrestrial planets were excluded because solids dominate their
  mass and spin (p4).
- No spin evolution seen between 2 and 300 Myr, and Jupiter and Saturn are
  similar, so the rate is set early (p4).

### 3.2 Bryan 2020 (full text)

27 planetary-mass objects. Abstract (p1): spin velocities scale as `v ∝ 1/R`
(angular momentum is conserved as the objects contract); "approximately an
order of magnitude below their break-up values"; "a factor of 4 to 5 variation
in spin velocity that has yet to be understood theoretically". p10: no
significant correlation of `v / v_break` with semi-major axis or with the
mass ratio to the star, with or without Jupiter and Saturn.

### 3.3 What is missing for giants

- No source read gives a **primordial obliquity** distribution for a giant.
  Kokubo & Genda and Miguel & Brunini are rocky, impact-driven, and say
  nothing about gas accretion through a disk. Bryan 2020 reports an obliquity
  constraint for one object (2M0122-2439 b, p5, only the method was read, no
  distribution). The Solar System giants' present obliquities were not read in
  a source here.
- No source gives the **shape** of the spread of `v / v_break`, only its mean
  and the "factor 4 to 5".

## 4. Proposal

One distribution per quantity. The rotation period is drawn from the body's
`body_seed.derive("spin")` stream; the obliquity and the rate use two draws on
that stream, in a fixed order, so that adding a draw never shifts another.

### 4.1 Rocky obliquity

- **Distribution:** isotropic, `p(ε) = (1/2) sin ε` on 0 to 180 deg.
  Sampled as `ε = arccos(1 - 2u)`, `u` uniform in [0, 1).
- **Source:** Kokubo & Genda 2010 p11 and p12; Miguel & Brunini 2010 p1, p10.
  Both read in full text, both agree and cite earlier work.
- **Definition:** none for the shape. The range 0 to 180 deg is the source's.

### 4.2 Rocky rotation period

- **Distribution:** the spin vector has three independent Gaussian components
  of zero mean and `σ = 0.41 ωcr`, with `ωcr = 3.3 (ρ / 3 g cm⁻³)^(1/2) rad/h`
  (`sqrt(GM/R³)` from the planet's own mass and radius); the period is
  `2π / |ω|`.
  This is Kokubo & Genda's realistic model: p11 (Gaussian, mass-independent),
  p19 Table 2 (the σ, `σ / ωcr = 0.41` derived), p8 eq. 5 (ωcr).
- **Consistency:** the drawn obliquity of 4.1 and this vector are the same
  draw in the source (the direction of ω); drawing `|ω|` from the Maxwell law
  and the direction isotropically is the same distribution, so one 3-vector
  draw can give both. Implementation choice, no new number.
- **Definition:** two. (a) Scaling σ with `ωcr ∝ ρ^(1/2)`: the source shows the
  rate is mass-independent and gives `ωcr` for scaling, but its runs are at one
  density; using a planet's own density is the paper's own normalisation
  (p11: "natural outcomes ... as large as ωcr" for perfect accretion), not a
  fit across densities. (b) Capping `|ω|` below ωcr: Miguel & Brunini (p6) find
  planets above ωcr do not survive, and the Gaussian tail does reach it
  (for a 3 g cm⁻³ planet `ωcr / σ = 2.4`, and the Maxwell tail beyond it
  holds about 12 % of the draws, **derived**, not counted in the sources).
  The draw rejects `|ω| ≥ ωcr` and redraws.
- **Not chosen:** Miguel & Brunini's 10 to 10 000 h. They give a range, not a
  shape, and it comes with an accretion model that is only valid below ten
  Earth masses. Offered to the user as an alternative in the open points.

### 4.3 Giant rotation period

- **Distribution:** `P = 2π R / (f v_break)` with
  `v_break = sqrt(GM / R)` from the giant's own mass and radius, and `f` drawn
  log-uniform on 0.07 to 0.35.
- **Source:** Bryan 2018 p1 (Solar System giants 3 to 4 times below
  break-up, i.e. `f` 0.25 to 0.33, derived), p3 (mean 0.126), p4 (no
  dependence on mass); Bryan 2020 p1 (factor 4 to 5 of variation, an order of
  magnitude below break-up), p10 (no dependence on `a` or `M / M⋆`).
- **Definition:** the shape (log-uniform) and the bounds. The sources give a
  mean, a factor 4 to 5 and the 3 to 4 of the Solar System giants, never a
  distribution. 0.07 to 0.35 is a ratio of 5, containing 0.126 and the
  Solar System's 0.25 to 0.33 (derived). It is a flattening of what is read.
- Applies to ice giants too, as a `Definition:`. Bryan 2018 p4 excludes them
  from the correlation test (solids dominate their mass). A follow-up search
  (2026-11-21) found no ice-giant primordial spin: Batygin 2018
  (arXiv:1803.07106) and Ginzburg & Chiang 2020 (arXiv:1910.11885), both
  read through a summarising fetch, stop near Saturn's mass and predict no
  mass trend; Morbidelli 2012 (arXiv:1208.4685) gives no initial spin for
  proto-Uranus; the one exoplanet bound, HIP 41378 f (arXiv:2410.05408,
  P ≥ 15.3 h), allows any f below 0.74 (**derived**). Check, **derived**
  from the measured periods, masses and equatorial radii: Uranus f = 0.17,
  Neptune 0.16, inside the range; Jupiter 0.30 and Saturn 0.40, so the
  upper bound 0.35 leaves Saturn out (open point 6).

### 4.4 Giant obliquity

- **Distribution:** none sourced. **Definition:** the same isotropic law as
  4.1, to keep one obliquity law and to claim nothing the sources do not give.
- Open point 2: whether a giant that accreted through a circumplanetary disk
  should instead start close to its orbit normal. No source for that was read.

### 4.5 Open points for the user

1. **Rocky rate.** Settled by the user (2026-11-21): Kokubo & Genda
   (section 4.2).
2. **Giant obliquity.** Settled by the user (2026-11-21): isotropic, as a
   `Definition:` (section 4.4).
3. **Ice giants.** Settled: the gas-giant law, as a `Definition:` (no
   source found, Uranus and Neptune inside, section 4.3).
4. **Obliquity range.** `ε` above 90 deg means a retrograde spin. Task 9 needs
   to know if `Spin.obliquity` accepts 0 to 180 deg.
5. **Moons and tides.** This is the birth value. The present value of a planet
   close to its star (despun, Cassini state) is T2 (#604); the draw keeps the
   birth spin until then.
6. **Saturn.** Its f of 0.40 lies above the 0.35 bound; raise the bound
   to 0.40 (a ratio of 5.7) or keep 0.35.
