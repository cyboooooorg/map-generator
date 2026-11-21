# Study: architecture families of a random system

Date: 2026-11-21. Owner: mapgen3. Spec:
[system draw at the present](../../specs/2026-11-21-system-draw-design.md),
section "Planets of a random system". Roadmap #427, slice 1 #602.

This folder holds the numbers the planet draw (system draw PR 2) needs
inside each of the five families. The rule of the spec applies: a
`Definition:` may only choose *what to pick* (which family, how many
slots, a flattened shape where variety asks for it); every physical rule
(spacing, stability, eccentricity range, inner edge, mass range) comes
from a source read here.

- [Small planets and spacing](small-planets-and-spacing.md): compact
  systems, period ratios, mutual Hill radii, stability, small-planet
  eccentricities, the Kepler dichotomy, ultra-short periods.
- [Giant planets](giant-planets.md): hot Jupiters and their companions,
  giant eccentricities, scattering, cold-giant placement, mass function,
  ice line, mass bounds.

Each file states its reading channel per source (full text, abstract,
not reached) and gives the PDF page of every number. This page is the
synthesis: it maps those numbers to the draw's parameters. Section
numbers below point into the two files (`S` small, `G` giant).

## Common rules

### Mutual Hill radius

Every source read uses the same definition (S 2.1):

```text
R_H = ((m1 + m2) / (3 M_star))^(1/3) * (a1 + a2) / 2
Delta = (a2 - a1) / R_H
```

For eccentric pairs, He et al. 2019/2020 use the periapsis of the outer
planet and the apoapsis of the inner one in place of `a2 - a1` (S 2.1).
The draw uses that form for every pair, so one formula covers circular
and eccentric orbits.

### Stability floor between neighbours

- Two planets: `Delta > 2 sqrt(3)` (Gladman 1993, read second-hand in
  Fabrycky 2014 p4, Pu & Wu 2015 p2, Petit 2020 eq. 1; S 2.3).
- Three or more planets: the median-survival spacing of Pu & Wu 2015,
  eq. 12 and 14 (S 2.3):

  ```text
  K50 = 0.7 log10(t / T1) + 2.87 + sigma_e / 0.01 + sigma_inc / 0.04
  ```

  with `t` the star's age, `T1` the innermost period, `sigma_e` and
  `sigma_inc` (rad) the system's Rayleigh scales. About 10 at 1 Gyr for
  circular orbits, 12 for Kepler-like eccentricities.
- Proposal: a pair is accepted when its eccentric `Delta` exceeds `K50`
  (three or more planets) or `2 sqrt(3)` (two planets). `K50` is a
  population median, not a per-system guarantee; using it as a floor is
  a `Definition:` that keeps drawn systems on the stable side.
- Giants (G 3.3): Juric & Tremaine's post-scattering floor is about 4
  Hill radii; the same two rules apply, since giant pairs are few.

### Inner edge

- Physical floor (no orbit inside it): the Roche limit. Rappaport et
  al. 2013 (S 6): `P_min = 12.6 h * rho^(-1/2)` (fluid body, rho in
  g/cm³), independent of the host. For giants, Ford & Rasio 2006: the
  hot-Jupiter edge sits near twice the Roche separation; Dawson &
  Johnson 2018 eq. 15 gives `P_Roche = 0.79 d` for a 1 M_J, 1.3 R_J
  planet, so about 2.2 d at `2 a_Roche` (G 1.2, derived).
- Needs from the planet model: the trial's mean density. `class_of`
  gives the class; the draw needs `rho` too (open point with mapgen1).
- A slot inside the star (orbit below the stellar radius) or inside its
  Roche floor drops the family for this host (spec rule).

### Host scaling

- The innermost-planet break near 10 d stays at a constant *period* across
  FGK and M hosts (Mulders et al. 2015, S 1.2). Millholland 2017 finds a
  weak trend instead (0.069 AU per solar mass). Proposal: constant period
  (one rule, the stronger source), stated as an extrapolation outside
  0.2-1.4 M_sun.
- Ice line: Ida & Lin 2005, quoted by Kennedy & Kenyon 2008 (Fig. 1,
  p4): `a_snow = 2.7 AU * M_star / M_sun` (G 3.4). **Decided** (user,
  2026-11-21): this linear form, with its overshoot above 3 M_sun
  stated in the code's `Source:` text (21.6 AU at 8 M_sun, where
  Kennedy & Kenyon's model finds 10-15 AU).
- Why not legacy's `2.7 sqrt(L)`: it has no source in the code, and it
  takes the present main-sequence luminosity, while the ice line that
  shapes the architecture is the disk's, during formation, set by the
  young star and the disk's own heating. With `L` about `M^4` it grows
  as `M^2`: about 150 AU at 8 M_sun, and too close in for M dwarfs,
  which were much brighter when young. Kennedy & Kenyon reject that
  steep form and find a weak dependence on the host's mass.
- Giant slots scale their semi-major axes with the ice line (Fulton's
  break sits near it for Sun-like hosts, G 3.1).

## The families

Every family is drawn with probability 1/5 (`Definition:`, spec). Counts
drawn uniformly in a sourced range are `Definition:` flattenings of a
source's shape (variety); the range stays the source's.

**Decided** (user, 2026-11-21): the counts below are accepted, and every
count range (planets, rocky planets, giants, inner planets, companions)
lives in one data table of the system crate, not in the code, so it can
be raised or lowered later without touching the draw. A per-request
override (an `Inputs` field and a `mapgen system` option) may follow in
its own pull request.

**Decided** (user, 2026-11-21): giant eccentricities split one source's
law into halves to keep the families distinct (solar-like low,
eccentric giants high), without forbidding an eccentric giant in a
solar-like system: one solar-like system in ten (`Definition:`, kept in
the same data table) draws its giants from the upper half instead. The
draw is per system, not per giant, since scattering stirs the whole
system.

### 1. Compact

| Parameter | Proposal | Source |
| --- | --- | --- |
| Planet count | uniform 4-7 | He 2019 lambda 3.86 per cluster, EPOS I 7+ (S 1.1); flattening is a Definition |
| Innermost period | broken power law, peak 12 d, index +1.6 below, -0.9 above, floor at Roche | EPOS I Table 2 (S 1.2) |
| Adjacent period ratio | log-normal in D: log D = -0.39, sigma 0.18 (P about 1.8) | EPOS I (S 1.5) |
| Near resonance | with probability 0.29 per pair, a ratio within 5 % exterior to 3:2 or 2:1 | He 2019 f_mmr and its 5 % window (model; per planet, applied per pair as a Definition), Fabrycky 2014 peaks (S 1.5) |
| Ratio floor | 1.2 | Weiss 2018 "wall" (S 1.5) |
| Masses | one system mass from Neil & Rogers 2020, each planet's radius scattered by sigma_R 0.31 around the cluster radius | Planet-interior study B9; He 2019 (S 1.4) |
| Eccentricity | Rayleigh sigma 0.02-0.035 | He 2019, Mills 2019 (S 3.1) |
| Inclination | Rayleigh sigma 1.4 deg | He 2019, Xie 2016 (S 3.1) |
| Outer extent | none fixed: follows from count and ratios (35-640 d) | spec's 0.3 AU has no source (S 1.3) |

The masses row needs mapgen1's mass-radius relation to turn the radius
scatter into a mass scatter; no source gives the scatter in mass.

### 2. Solar-like

| Parameter | Proposal | Source |
| --- | --- | --- |
| Inner rocky planets | uniform 2-4, 0.4-2.6 Earth masses, inside the ice line | Raymond 2006 simulations (S 5) |
| Their spacing | Fang & Margot shifted Rayleigh (offset 3.5, sigma 14.5), floor as above | Fang & Margot 2013 (S 2.2); Solar System 26-63, consistent (S 5) |
| Their eccentricity | about 0.05 | Raymond 2006 (S 5) |
| Giant count | uniform 1-4 | Definition (Solar System 4); no count source |
| First giant | Fulton broken power law in a (index +0.72 inside the break, -0.86 beyond), break 3.6 AU and support up to 30 AU for the Sun, both scaled by a_snow / 2.7 AU; truncated below the ice line | Fulton 2021 (G 3.1); the scaling is a Definition |
| Next giants | a2/a1 peaked near 3, below 10 | Rosenthal 2023 (G 3.3); Solar System 1.6-2.0 |
| Giant mass | log-uniform 30-900 Earth masses (0.1-2.8 M_J) | Fulton 2021 flat in ln M (G 3.2) |
| Giant eccentricity | lower half (below its median, about 0.2) of the multi-giant Beta(1.06, 3.68); upper half in one system of ten | Rosenthal 2023 (G 2.1); the halves are a Definition |

### 3. Hot Jupiter

| Parameter | Proposal | Source |
| --- | --- | --- |
| Period | log-uniform from 2 a_Roche (about 2.2 d) to 10 d | Dawson & Johnson 2018 (G 1.1-1.2); no density shape read |
| Mass | log-uniform 0.25-13 M_J | D&J definition (G 1.1), Spiegel 2011 (G 4.1); the dearth above 3 M_J lies inside 2 a_Roche, already excluded (G 1.3) |
| Eccentricity | below 0.1 | D&J: 31 of 228 at 3-10 d below 0.1, 10 above 0.2, the rest poorly constrained (G 1.3); no shape for the eccentric minority, left out |
| Close companions | none inside the outer companion's band | Huang 2016: 0.52 % vs 37 % (G 1.4) |
| Outer companion | with probability 0.70: 1-13 M_J at 1-20 AU (scaled by a_snow / 2.7 AU), eccentricity from the whole multi-giant Beta(1.06, 3.68) | D&J p32 combining Knutson 2014 and Bryan 2016 (G 1.5); Rosenthal 2023 (G 2.1) |

### 4. Eccentric giants

| Parameter | Proposal | Source |
| --- | --- | --- |
| Giant count | uniform 1-2 | spec (Definition) |
| Semi-major axis | Fulton broken power law as in family 2 | Fulton 2021 (G 3.1) |
| Mass | log-uniform 0.3-13 M_J | Zhu & Wu cut, Spiegel (G 2.4, 4.1) |
| Eccentricity | upper half (above its median, about 0.2) of the single-giant Beta(0.60, 1.45) (one giant) or multi-giant Beta(1.06, 3.68) (two) | Rosenthal 2023 (G 2.1); the half is a Definition |
| Inner small planets | uniform 0-2, placed only where the eccentric Delta floor holds against the giant's periapsis | Zhu & Wu 2018: 1.4 vs 2.5 (G 2.4) |

### 5. Sparse

| Parameter | Proposal | Source |
| --- | --- | --- |
| Planet count | uniform 0-2 | spec (Definition) |
| Kind of each | small or giant with probability 1/2 (Definition) | |
| Small planet | Neil & Rogers mass, eccentricity half-Gaussian sigma 0.32 | Van Eylen 2019 singles (S 4) |
| Giant | mass as family 4, eccentricity from the whole single-giant Beta(0.60, 1.45) | Rosenthal 2023 (G 2.1) |
| Period | log-uniform from the inner floor to 30 AU scaled by a_snow / 2.7 AU | Fulton 2021 survey edge (G 3.1); upper bound is a Definition |

## What this study does not settle

- Primary texts not reached, used second-hand: Gladman 1993, Chambers
  et al. 1996, Hayashi 1981, Udry & Santos 2007 (no hot-Jupiter period
  shape, hence log-uniform).
- No source measures the inner edge or giant placement outside about
  0.2-1.4 M_sun; the draw extrapolates by constant period and the ice
  line, and says so in its `Definition:` text.
- No intra-system mass scatter: derived from the radius scatter through
  the planet model.
- The trial's mean density (Roche floor) must come from mapgen1's model.
- Pu & Wu's `K50` is fitted on systems of seven planets of 3-9 Earth
  masses, innermost at 0.1 AU; Petit 2020's closed form for unequal
  masses was not extracted.
- The moderately eccentric hot Jupiters (e above 0.2, still
  circularising) have no distribution in the sources read and are left
  out.
- He 2019's near-resonant fraction is model-implied and per planet; no
  measured fraction per pair was found.
