# Analysis: binary and multiple systems

Issue #376. Request of 2026-11-15: study the systems with more than one
star or with a compact object in a pair, so that the user can decide
which configurations the generator and the viewer should get. This is a
research study, not a decision: the last section is a proposal.

Compact objects on their own (neutron stars, pulsars, magnetars, black
holes, white dwarfs, giants, brown dwarfs) are the subject of the
host-stars study (#375). Here the question is what being in a pair
changes, so a compact object appears only as a companion.

*Sources checked on 2026-11-18 (arXiv abstracts and the NASA Exoplanet
Archive queried through its TAP service). A claim marked [M] could not
be checked against a source and must be rechecked before it goes into
code. Numbers marked "computed here" are my own arithmetic on checked
inputs, not published results.*

## 1. What the code does today

Every system has one star, and the whole pipeline assumes it.

- **One star per system.** `System` holds `star: SystemStar`
  (`crates/mapgen/src/system.rs:158`, `:190`) and a flat list of
  `bodies`; `SystemStar.info` is a [`StarInfo`](../../crates/mapgen/src/star.rs)
  (`star.rs:487`): one temperature, luminosity, mass and radius, "seen
  from 1 AU" (`system.rs:196`). Optional additions already exist as
  `#[serde(default, skip_serializing_if = ...)]` fields (`belts`,
  `galaxy`, `activity`), the precedent for a `companions` list.
- **One star type per world.** `WorldParams.star` is a single type code
  (`f32`), which `spin::rocky_star` hands to `StarInfo::for_planet`
  (`spin.rs:213`, `:237`). A planet's flux is
  `L(code) / a² * mean_flux_factor(e)` (`star.rs:654`, `:662`), and the
  habitable zone is the Kopparapu fit on one `teff_k` (`star.rs:287`).
  The planet model reads it as `Context.insolation` and
  `equilibrium_k` (`planet/inputs.rs:19-33`); the climate works on the
  annual mean (`climate.rs:74`, `:106`), so any variation faster than a
  year is averaged out.
- **One star drawn.** `star::draw_type` draws a spectral type with the
  solar-neighbourhood shares, about 77 % M (`star.rs:242`, `:247`);
  `place_whole` draws it from the system's own stream (`system.rs:1008`,
  `:1010`, `stream::SYSTEM = 64`, `noise.rs:203`) after the request's
  star, age and metallicity. The planet request path
  (`WorldRequest::resolve_with_archetype`) still draws the star and the
  orbit from an unseeded rng (#473): any new draw must not join that
  shared rng.
- **Placement.** Planets are placed from the star out between
  `INNER_EDGE_AU = 0.02` and `OUTER_EDGE_AU = 60` (`system.rs:86-87`),
  spaced in mutual Hill radii (`clear`, `system.rs:620`; `place`,
  `:780`; `hill_spacing`, `giants.rs:82`), with the ice line at
  `2.7 sqrt(L)` (`system.rs:1493` `assemble`).
- **Tides and moons.** The Hill sphere of a planet is computed against
  its parent or `star.info.mass * SUN_KG` (`tides.rs:194`); the tidal
  locking time (`star.rs:371`, `spin.rs:154`) and the circularisation
  (`spin.rs:122`) take one stellar mass and the orbit.
- **Seeds.** Noise streams use `stream_seed(seed, stream)` with numbers
  1 to 43, 64 and 65 in use (`noise.rs:129-206`); `STAR_ACTIVITY = 40`
  seeds the star's surface (`system.rs:434`). The format version is 9
  (`system.rs:82`): `read_system` refuses another version, so adding
  stored fields means a bump.
- **Viewer.** The system frame has the star at the centre
  (`web/src/lib/systemSky.ts:7`), with one `sun` vector and one
  `star: {position, radius, axes}` (`:223-229`); the orrery draws one
  star of radius `STAR_RADIUS = 1` (`lib/viewer/starSystem.ts:152`); the
  living star (`StarView.ts`, `starSurface.ts`; design in
  `docs/superpowers/specs/2026-11-08-living-star-design.md`, issues
  #343-#345) draws one surface; eclipses exist only for a planet or moon
  shadowing the sun (`lib/skyBodies.ts:257`).
- **Documented non-goal.** `10-stars.md` lists multiple stars as "not
  planned".

## 2. Research

### 2.1 How common, and with what orbits

Multiplicity depends strongly on the primary's mass.

| Primary | Result | Source |
|---|---|---|
| Solar-type (F6 to K3, 25 pc) | single 56 %, double 33 %, triple 8 %, higher 3 % | [Raghavan et al. 2010](https://arxiv.org/abs/1007.0414) |
| F and G dwarfs (4,847 stars) | 54:33:8:4:1 for 1 to 5 components, multiplicity 0.46, 2+2 quadruples 4 % | [Tokovinin 2014](https://arxiv.org/abs/1401.6827) |
| M dwarfs (1,120, 25 pc) | multiple 26.8 +/- 1.4 %, companion rate 32.4 +/- 1.4 %, separations peak near 4-20 AU | [Winters et al. 2019](https://arxiv.org/abs/1901.06364) |
| Solar-type to O-type | companions per primary (q > 0.1) rise from 0.50 +/- 0.04 to 2.1 +/- 0.3 | [Moe and Di Stefano 2017](https://arxiv.org/abs/1606.05347) |

The review of Duchene and Kraus 2013
([arXiv:1303.3028](https://arxiv.org/abs/1303.3028)) tabulates the
frequency by mass bin; its numbers were not reread here [M].

Distributions for solar-type primaries:

- **Period.** Raghavan: unimodal, roughly log-normal, peak near 300
  years; the fit quoted by later papers is mean `log10 P[d] = 5.03`,
  `sigma = 2.28` (read in a secondary summary, not in the paper [M]).
  Tokovinin's independent F and G sample gives a median of 100 years and
  a logarithmic dispersion of 2.4 dex. Moe and Di Stefano find the
  companion frequency peaks at `log P = 3.5` days (a of about 10 AU),
  with `log P` of 5.5 to 7.5 days (200 to 5,000 AU) mostly outer
  tertiaries of triples. For M dwarfs the peak is near 4 to 20 AU
  (Winters).
- **Mass ratio.** Nearly uniform and independent of period for solar-type
  primaries (Tokovinin; Raghavan prefers like-mass pairs, more so when
  close). Moe and Di Stefano: `<q> = 0.5` for `P < 20 d`, small values
  (`q` of 0.2 to 0.3) at `log P = 3.5`, random pairing from the mass
  function beyond `log P = 5.5`; a small twin excess (`q > 0.95`) at short
  periods.
- **Eccentricity.** Circularised below about 12 days (Raghavan, flat
  period-eccentricity relation); `e < 0.4` below 20 days and a thermal
  distribution `f(e) = 2e` at intermediate periods (Moe and Di Stefano).
- **Period ratio of triples.** Tokovinin's simulations impose stability
  and fail to reproduce the observed excess of inner periods under 10
  days, attributed to tidal evolution.

A candidate draw (my proposal, to be confirmed, not a published fit):

```text
multiple?   p(M) = 0.27 at 0.3 Msun, 0.45 at 1 Msun, interpolated in
            log M; beyond 1.5 Msun the sources checked give no number [M]
log10 P[d]  ~ N(5.03, 2.28), clamped to a_b >= 0.03 AU and <= 2000 AU
q = M2/M1   ~ U(0.1, 1), with the low end set by the brown dwarf limit
e           = 0 for P < 12 d, else sqrt(U(0,1)) (thermal), at most 0.95
```

For M dwarfs the same form with the period centred two to three decades
lower (peak at 4 to 20 AU) matches Winters; the exact mean is a choice.
Computed here: the log-normal puts `P < 50 d` (`a_b < 0.3 AU` for 1.5
solar masses) at about 7 % of the binaries, so about 3 % of solar-type
stars; the observed close excess (Moe and Di Stefano) and tidal
shrinking make the true share larger, but I found no checked number.

### 2.2 Stable orbits

Two families, as Holman and Wiegert 1999 define them for a test
particle ([arXiv:astro-ph/9809315](https://arxiv.org/abs/astro-ph/9809315),
AJ 117, 621; equations read in the ar5iv version). With `mu = m2 /
(m1 + m2)`, `e` the binary eccentricity and `a_b` its semi-major axis,
the critical semi-major axis of the planet is:

```text
S-type (orbits m1, perturbed by m2), 0 <= e <= 0.8, 0.1 <= mu <= 0.9:
  a_c = a_b (0.464 - 0.380 mu - 0.631 e + 0.586 mu e
             + 0.150 e^2 - 0.198 mu e^2)

P-type (orbits both), 0 <= e <= 0.7, 0.1 <= mu <= 0.5 simulated:
  a_c = a_b (1.60 + 5.10 e - 2.22 e^2 + 4.12 mu - 4.27 e mu
             - 5.09 mu^2 + 4.61 e^2 mu^2)
```

The paper gives fit errors of typically 4 % (11 % at worst) for S-type
and 3 % (6 % at worst) for P-type. Integrations ran for `10^4` binary
periods, so the fit is a necessary, not a sufficient, condition on
longer times. Computed here: circular equal-mass P-type gives
`a_c = 2.39 a_b`; `e = 0.5` gives `3.6 a_b`; `e = 0.7` gives `3.9 a_b`
(the limit `mu -> 0` of the formula is 1.60, the figure quoted in the
issue, and does not apply to a real pair). An S-type planet must be
inside `a_c`, which is 0.2 to 0.46 `a_b` for a circular binary of
comparable masses.

The known circumbinary planets sit just outside this limit, in what is
called a pile-up near `a_c`; Martin and Triaud
([arXiv:1404.5360](https://arxiv.org/abs/1404.5360)) find that
observational biases alone cannot account for it; inward disc
migration is one proposed cause, not a result. Computed here with the masses
and periods read in the abstracts: Kepler-16 (0.69 and 0.20 M_sun, 41 d,
`a_b = 0.224 AU`, planet at 0.705 AU) is at 3.1 `a_b` against an
`a_c` of 2.9 `a_b` for `e = 0.159` (the eccentricity is from memory
[M]); TOI-1338 (1.1 and 0.3 M_sun, 14.6 d, `e = 0.16`) is at 3.5 `a_b`
against 2.9.

Tight binaries are depleted of planets. Munoz and Lai
([arXiv:1505.05514](https://arxiv.org/abs/1505.05514)) note that no
binary shorter than 7 days is known to host one (Kepler-47 at 7.45 d is
the shortest); Fleming et al. 2018
([arXiv:1804.03676](https://arxiv.org/abs/1804.03676)) propose that
tidal evolution of young binaries below about 8 days ejects planets,
and Graham, Fleming and Barnes
([arXiv:2011.10430](https://arxiv.org/abs/2011.10430)) find that for
equal-mass stars at `P = 7.5 d` the whole habitable zone is unstable
when the stars are under 0.12 M_sun (0.17 for `e < 0.5`).

Hierarchical triples (inner pair `a_in`, third body `a_out`) are
stable only for a large enough ratio: Mardling and Aarseth 2001 (MNRAS
321, 398) give `a_out (1 - e_out) / a_in` above about
`2.8 [(1 + q_out)(1 + e_out)/(1 - e_out)^(1/2)]^(2/5)` for a circular
coplanar inner orbit. I only saw this form quoted in secondary papers,
and one reports it performs poorly in the planetary limit [M].

### 2.3 Known planets

Computed here from the NASA Exoplanet Archive composite table
(`pscomppars`, queried through TAP on 2026-11-18: 6,470 planets):

- by catalogued stars in the system (`sy_snum`): 5,891 planets in
  one-star systems, 504 in two, 74 in three, 1 in four, so 8.9 % of the
  planets sit in catalogued multiples (the figure follows how well each
  system was searched for companions);
- `cb_flag = 1` (circumbinary): 53 planets, of which 14 by transit, 17
  by eclipse timing, 12 by imaging, 6 by microlensing, 3 by radial
  velocity and 1 by pulsar timing.

The 14 transiting circumbinary planets are in 12 systems (10 from
Kepler, 2 from TESS), as the issue says:

| System | Planet(s), archive semi-major axis (AU) | Note |
|---|---|---|
| Kepler-16 | b, 0.705 (P 228.8 d, R 8.4 Re) | binary 41 d, planes within 0.5 deg ([Doyle et al. 2011](https://arxiv.org/abs/1109.3432)) |
| Kepler-34, -35 | 1.09, 0.603 | |
| Kepler-38 | 0.463 | |
| Kepler-47 | b 0.288, d 0.699, c 0.964 | binary 7.45 d, c in the classical habitable zone ([Orosz et al. 2012](https://arxiv.org/abs/1208.5489); d: [Orosz et al. 2019](https://arxiv.org/abs/1904.07255)) |
| Kepler-64 (PH1) | 0.652 | a four-star system |
| Kepler-413, -453 | 0.355, 0.790 | |
| Kepler-1647 | b 2.72 (P 1,107.6 d, R 1.06 R_Jup) | binary about 11 d, in the conservative habitable zone ([Kostov et al. 2016](https://arxiv.org/abs/1512.00189)) |
| Kepler-1661 | 0.633 | |
| TOI-1338 | b 0.461 (R 6.9 Re, P 95 d), c 0.794 (radial velocity) | binary 14.6 d, masses 1.1 and 0.3, `e_b = 0.16`, planes within 1 deg ([Kostov et al. 2020](https://arxiv.org/abs/2004.07783)) |
| TIC 172900988 | 0.903 | |

S-type planets are more numerous. The census of Thebault and Bonanni
([arXiv:2506.18759](https://arxiv.org/abs/2506.18759)) lists 759
planet-hosting multiple systems: 728 S-type (651 binaries, 73 triples,
4 quadruples) and 31 circumbinary. About 22.5 % of planet hosts within
500 pc are in binaries, about half the field value, and the deficit
extends to separations of about 500 AU. Gamma Cephei Ab is the
historical shortest-period case: 1.7 M_Jup (as `M sin i`) at 2.13 AU
around the primary of a binary whose period the paper gives as about 57
years ([Hatzes et al. 2003](https://arxiv.org/abs/astro-ph/0305110));
the archive now lists 9.4 M_Jup at 1.98 AU after astrometry. The
separation of the binary was not checked (my estimate from the period
is of order 20 AU, assuming 1.7 M_sun [M]). Alpha Centauri A has an
unconfirmed JWST candidate ("S1", 90 to 150 Earth masses; one epoch,
not recovered later; [Beichman et al. 2025](https://arxiv.org/abs/2508.03814)).

### 2.4 Planets in binaries are rarer, by how much

- Raghavan: the planet-host fraction is not distinguishable between
  single stars and wide binaries or multiples.
- Kraus et al. 2016 ([arXiv:1604.05744](https://arxiv.org/abs/1604.05744)):
  in Kepler hosts, companions within about 50 AU (`q > 0.4`) are 23
  instead of the 58 expected, a 4.6-sigma deficit; planet occurrence
  inside about 47 AU is `0.34 (+0.14/-0.15)` of that of wide binaries
  and singles.
- Moe and Kratter 2021 ([arXiv:1912.01699](https://arxiv.org/abs/1912.01699)):
  binaries closer than 1 AU eliminate S-type planets, at 10 AU they
  occur at about 15 % of the single-star rate, beyond 200 AU the effect
  is small; 43 +/- 7 % of solar-type primaries have no close planets
  because of close companions.
- Circumbinary: Armstrong et al. 2014 ([arXiv:1404.5617](https://arxiv.org/abs/1404.5617),
  MNRAS 444, 1873) find about 10 % (+18 / -6.5) of close binaries host
  planets above 6 Re and under 300 d if coplanar, and at least 47 % if
  the planes are isotropic; giants are rarer than smaller planets.

The generator can read this as an occurrence factor `S(a_b)` applied to
the planet count of a system: 0 under 1 AU, rising through 0.15 at
10 AU and about 0.34 near 47 AU (the two papers do not agree on the
shape in between; this is a rough bridge [M]), to 1 beyond 200 AU.

### 2.5 What a planet receives

**S-type.** The planet gets the primary's flux plus the companion's.
At the planet-companion distance `d` the companion adds
`L2 / (L1) * (a / d)^2` of the primary's flux (computed here: a
Sun-like companion at 20 AU adds 0.25 % at 1 AU, about 6.5 magnitudes
fainter than the primary, so about 1,000 times brighter than the full
Moon, my arithmetic on the standard magnitudes). The effect on the
habitable zone follows from the flux sum; Kaltenegger and
Haghighipour 2013 ([arXiv:1306.2889](https://arxiv.org/abs/1306.2889))
find that a dimmer companion hardly moves the primary's habitable
zone while a brighter one can. The variation over the binary orbit is
slow (years to centuries), so it matters only for eccentric binaries
with the planet well inside `a_c`; the companion's tide on the planet
is smaller than the primary's by `(m2/m1)(a/d)^3`, negligible unless
`a_b / a` is under about 5 (computed here).

**P-type.** The planet gets the sum of both stars. Computed here for
two equal, equally luminous stars on a circular orbit, the peak-to-peak
variation of the flux over the binary period is 11 % at `3 a_b`, 6 % at
`4 a_b`, 2.8 % at `6 a_b` and 1 % at `10 a_b`, and the mean flux is
`L_tot / r^2` to 2.9 % at `3 a_b` and 0.7 % at `4 a_b`. Since the
binary period is days and the planet's climate responds over years, the
climate sees the mean: `L_tot / a^2` is a sufficient flux for it. Popp
and Eggl 2017 (Nature Communications 8, 14957, doi:10.1038/ncomms14957)
ran 3-D simulations of a water-covered Earth-like planet in the
Kepler-35 binary: the forcing leaves signatures at the planet's and the
binary's frequencies in temperature and rain but does not change much
the range of insolation that allows habitable climates. Forgan 2016
([arXiv:1608.05592](https://arxiv.org/abs/1608.05592), MNRAS 463, 2768)
finds rapid Milankovitch cycles (of order 1,000 years) in eccentricity,
obliquity and precession for terrestrial planets in Kepler-47 and in
Alpha Centauri.

**Habitable zone.** Kane and Hinkel 2013
([arXiv:1211.2812](https://arxiv.org/abs/1211.2812))
derive circumbinary habitable zones that depend on the stellar masses,
the separation and time, and show when treating the pair as one star
breaks down; Haghighipour and Kaltenegger 2013
([arXiv:1306.2890](https://arxiv.org/abs/1306.2890)) give a general
formalism for the contribution of each star, with a zone that moves as
the stars orbit. I did not reread the weighting, so the form below is
my reading of the idea, to be checked [M]: the planet is habitable when
`sum_i (L_i / d_i^2) / S_eff,i(T_eff,i)` lies between the values at the
outer and inner edges, with `S_eff` the Kopparapu et al. 2014 edges the
code already has (`star.rs:287`). For a P-type planet beyond `3 a_b` the
sum reduces to `L_tot / a^2` and an effective temperature weighted by
the fluxes.

**Eclipses.** An S-type planet sees the companion as a second sun that
rarely meets the primary. A coplanar P-type planet sees the two stars
pass in front of each other twice per binary period (computed here by
geometry: viewed in the plane, their angular separation goes through
zero twice per orbit), which makes mutual eclipses a regular feature
of the sky, partial or total depending on their angular radii.
Seen from afar, the planet transits one or both stars (Kepler-16 b,
TOI-1338 b). Kepler-16 and TOI-1338 are coplanar to within
0.5 and 1 degree, so the generator can assume near coplanarity.

**Spin.** Tidal locking for a P-type planet is the single-star formula
with the total mass at the planet's orbit (`star::locking_gyr`,
`star.rs:371`), since `a >= 2.4 a_b`; the `a^6` dependence means a
planet at `0.4 AU` locks `4,096` times slower than at `0.1 AU` (computed
here). The binary's own spin matters through the stars: Mason et al.
2015 ([arXiv:1408.5163](https://arxiv.org/abs/1408.5163)) argue that
tidal braking locks close binaries and so lowers their magnetic
activity and the planets' atmospheric loss; this feeds the existing
stellar activity model (`activity.rs`), but the evidence is one paper.

### 2.6 Configurations

**A. Two stars.**

- *Wide (a_b above about 50 AU, S-type).* The commonest case: roughly
  half of the solar-type primaries and a quarter of M dwarfs have a
  companion, and the median separation is tens of AU. Planets around
  the primary are barely disturbed (`a_c` of 20 AU or more), the
  second star is a point-like very bright object in the sky. Also the
  cheapest: the planetary system is unchanged, bounded by `a_c`.
- *Intermediate (3 to 50 AU, S-type).* The main-sequence tail of the
  distribution: planets rarer (section 2.4), the outer part of the system
  truncated at `a_c`, a companion that is a disc of a few arcminutes to
  degrees in the sky. Gamma Cephei Ab is the proof of existence.
- *Close (a_b under 1 AU, P-type).* The 14 transiting planets show it
  works, at `a >= a_c` and with `P_bin >= 7 d`; the planets need two
  fluxes and an orbit pair, the sky has two close stars that eclipse.

**B. A star and a compact object.** Moe and Di Stefano estimate that
30 +/- 10 % of the single-lined spectroscopic binaries of solar-type
primaries have a white dwarf companion instead of a low-mass star, so a
white dwarf companion is common in the close separations. Known
planets: WD 1856+534 b (an archive-listed transiting giant at 0.0204
AU, `R = 10.4 Re`, in a system of three stars; the paper
[Vanderburg et al. 2020](https://arxiv.org/abs/2009.07282) reports a
1.4 d transit period, no more than 14 M_Jup) is around the white
dwarf, not around a main-sequence primary. Post-common-envelope pairs
(NN Ser) carry eclipse-timing planets, which are model-dependent and
whose stability was not rechecked here [M]. A pulsar and white dwarf
with a planet: PSR B1620-26 in M4 (white dwarf 0.34 +/- 0.04 M_sun
of 480 +/- 140 Myr, planet of 1 to 3 M_Jup on a wide low-eccentricity
circumbinary orbit; [Sigurdsson et al. 2003](https://arxiv.org/abs/astro-ph/0307339)),
archive semi-major axis 23 AU. Black-hole companions are rare but
exist: Gaia BH1 is a 9.62 +/- 0.18 M_sun dark companion to a 0.93 M_sun
G dwarf, `P = 185.6 d`, `e = 0.45`, at 480 pc, the nearest known
([El-Badry et al. 2023](https://arxiv.org/abs/2209.06833)); Gaia BH3 is
32.7 M_sun with an old metal-poor halo giant, 11.6 years, 590 pc
([Gaia Collaboration 2024](https://arxiv.org/abs/2404.10486)). Computed
here for BH1 (`a_b = 1.4 AU` from Kepler's third law, as my estimate):
the S-type limit around the G dwarf is `0.095 AU`, so a habitable zone
around it (about 1 AU) would not be stable, and `mu = 0.91` lies
slightly outside the fit's range. What changes with a compact
companion: the stable region is the same formula; the flux from the
companion is small or none (a cool white dwarf adds almost nothing; a
black hole and a neutron star none, except an accretion disc); the tide
can be large (a black hole of ten solar masses at 1.4 AU tides the
star's planets as a ten times heavier star would); the viewer shows a
dark or faint object. The companion's own physics is for #375.

**C. Two compact objects.** No planet is known to orbit such a pair
other than PSR B1620-26 (neutron star plus white dwarf). For a double
neutron star, PSR B1913+16 has `P = 7.75 h`, `e = 0.617` and masses
`1.438` and `1.390 M_sun` ([Weisberg and Huang 2016](https://arxiv.org/abs/1606.02744));
computed here, `a_b = 2.8 R_sun` and, with the Peters formula (my
recollection of it, not checked in a source [M]), a merger in about
0.3 Gyr, a figure that agrees with the usual quoted value. Such pairs
are short-lived on the scale of a system's age, have no light of their
own and nothing for a planet to orbit that has survived two supernovae;
neutron-star and black-hole pairs are visible mainly through
gravitational waves and lensing. A viewer would show two black points
and some lensing.

**D. Triples and higher.** 8 % of solar-type systems are triples and
3 to 4 % are of higher order (Raghavan, Tokovinin), and 73 of the 728
S-type planetary systems of the Thebault and Bonanni census are
triples. A coherent
generator needs the Mardling-Aarseth criterion and 3-D orbits of the
subsystems, and the sky has three suns in nested motion. The count is
low and the cost is high.

## 3. What changes

### Generator

1. **Data model.** `System` gains `companions: Vec<Companion>` (a star
   code, `orbit` as `Orbit`, a `kind` of star or compact object, a
   `relation` of S-type or P-type for the planets), with
   `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
   (`system.rs:158`, as `belts`). `SYSTEM_FORMAT_VERSION` goes from 9
   to 10 (`system.rs:82`), and the CLI/server schema and the `ts_rs`
   types follow. `WorldRequest` and `WorldParams` need a companion
   too, since the planet model reads them (`spin.rs:213`).
2. **Draw.** A new stream (the next free numbers are 44 to 63;
   `noise.rs:129`) draws: whether there is a companion, its mass, its
   period and eccentricity (section 2.1), and its orbit angles. It must
   come after every existing draw and be gated: with the multiplicity
   off, no new random number is read and every existing seed gives the
   same system. If the draw were on by default, about half of the existing
   seeds would change (the generation-changes policy allows it when the
   worlds stay coherent, but it is a decision for the user, not an
   implementation detail). The unseeded path (#473) must not be extended;
   the companion draws from the system's seed only.
3. **Stability.** A function `stable_limit(kind, m1, m2, e_b, a_b)`
   with the Holman-Wiegert polynomials (section 2.2) sets `OUTER_EDGE`
   for S-type (`min(a_c, OUTER_EDGE_AU)`, in `Slot::max_au`,
   `system.rs:569`) or the inner edge for P-type (`max(INNER_EDGE_AU,
   1.1 a_c)`, applied in `place`, `system.rs:780`). `place` spaces from
   the star out, so the P-type case needs the spacing reversed or an
   inner offset. The planet count comes from `S(a_b)` (section 2.4).
4. **Flux and zone.** S-type: add `sum_i L_i * mean(1 / d_i^2)` to
   `StarInfo.insolation` (`star.rs:654`); P-type: use `L_tot` and a
   flux-weighted `T_eff`. Today `StarInfo` is built from a type code
   (`values(code)`, `star.rs:166`), so P-type needs a constructor from
   explicit `L`, `T_eff`, `M`, `R`, the largest change after the format.
   The habitable zone code (`star.rs:287-312`) needs the weighted form
   of section 2.5. `Context` (`planet/inputs.rs:19`) takes the total
   mass for P-type tides and spin (`spin.rs:237`, `tides.rs:194`).
5. **Tides and moons.** For P-type use the total mass wherever the star
   mass appears (`star::locking_gyr`, `spin::circularisation_gyr`,
   `host_hill_km`); the moons' Hill spheres for S-type should also
   include the companion's perturbation, which is small beyond
   `a_b / a > 5`.
6. **Star surface.** A second star needs its own `activity`
   (`system.rs:434`) from a stream of its own, so that the primary's
   stays unchanged.

### Viewer

- **Data.** The generated TypeScript types change with the format; the
  orrery (`SystemOrrery.svelte`, `starSystem.ts`) places a second
  marker, with a Keplerian orbit as the planets already have
  (`keplerPosition`, `starSystem.ts:281`). In P-type the two stars
  circle a barycentre at the centre of the frame (`systemSky.ts:7`).
- **Sky from a planet.** `systemSky.ts:223` builds one `sun` direction
  and one star object; both become lists. The planet's shading takes
  one sun; for S-type a second, much fainter light is enough, while
  for P-type with comparable stars the shaders (`shaders.ts`) need two
  light directions and the shadow code needs two suns
  (`skyBodies.ts:238-270`); mutual star eclipses are a new occluder.
- **Star view.** `StarView.ts` shows one star; a second would reuse it
  with its own activity. The living star work (#343-#345) is one star
  per scene.

## 4. Recommendation (proposal, to decide with the user)

The order is by frequency, plausibility of planets, visual interest
and cost.

1. **Wide and intermediate S-type binaries (a_b of 3 AU to 1,000 AU).**
   The most common configuration (44 to 46 % of solar-type and 27 % of
   M-dwarf systems are multiple), planets are possible with a known
   rule, and the cost is low: the planet pipeline is unchanged except
   for an outer bound, a flux term and the companion's marker and sky
   object. Gamma Cephei Ab and the Kraus / Moe-Kratter statistics give
   checked inputs. It also decides the data model all later steps use.
2. **Circumbinary (P-type) around close binaries (`P_bin` of 7 to
   about 50 d).** Fourteen known transiting planets; the strongest
   visual interest (two suns, mutual eclipses, the Tatooine sky) and
   a clear formula (Holman-Wiegert). The cost is moderate: a combined
   star constructor, a placement that starts at `a_c`, and two suns in
   the viewer. Rare (about 3 % of solar-type stars by the log-normal,
   computed here) and the planets are rarer still.
3. **White dwarf companions (after #375 provides white-dwarf rendering).**
   Common in close binaries (about 30 % of the single-lined
   spectroscopic binaries) and cheap once step 1 exists: only the
   companion's `kind`, its low luminosity and its mass.

Out of scope for now:

- **Black-hole companions:** rare, and the stable zone around the star
  excludes the habitable zone in the only well-measured case; possible
  as a later curiosity after #375.
- **Two compact objects:** no planets, short-lived, nothing to draw but
  a lensing effect.
- **Triples and higher:** 11 to 12 % of systems but a large cost: the
  stability criterion, nested motion in the sky and a second layer
  in the data model. Reconsider after steps 1 and 2.
- **Binaries tighter than 7 days:** no known planets; the generator
  would draw none.

Follow-up issues to file once the user decides (not filed here):

| Title | Scope |
|---|---|
| `feat(mapgen): draw a stellar companion and bound S-type planets` | `companions` in `system.json` (format 10), draw stream, Holman-Wiegert outer bound, occurrence factor, flux sum, gated by an option so existing seeds do not change |
| `feat(web): show the companion star in the orrery and the sky` | second marker and orbit, second sun in the sky of S-type planets, star view of the companion |
| `feat(mapgen): circumbinary planets around close binaries` | P-type draw (`P_bin` of 7 to 50 d), combined star constructor, inner edge at `a_c`, total-mass tides and spin |
| `feat(web): two suns: flux sum, two-light shading and mutual eclipses` | viewer side of P-type: two light directions, binary orbit around the barycentre, eclipse of one star by the other |
| `feat(mapgen): habitable zone of a binary` | flux-weighted zone for S-type and P-type, reused by the form's zone display |
| `feat(mapgen): white dwarf companions` | companion kind for step 1, after #375 |

## 5. Open points

- Whether multiplicity is on by default (a worlds change, the
  generation-changes policy applies) or opt-in per request.
- Which stellar mass range to draw for the companion's mass ratio and
  whether brown dwarfs count (Raghavan's mass-ratio fit extends into
  them).
- The binary period distribution for M dwarfs: the checked source
  (Winters) gives the peak, not a fit.
- The occurrence factor `S(a_b)` between 10 AU and 200 AU: two papers
  bracket it, the shape is a choice.
- The habitable-zone weighting (section 2.5): reread
  Haghighipour and Kaltenegger before coding.
- Whether a companion's orbit should be coplanar with the planets
  (observed in circumbinary systems, 0.5 to 1 degree) or drawn for wide
  S-type pairs, where the evidence checked here is silent.
- Stellar lifetimes: both stars share the age, but the companion's
  lifetime may be shorter (`star::lifetime_gyr`,
  `star.rs:356`), which makes an old system with a white dwarf
  companion a natural outcome.

## 6. Not verified

- The mean and dispersion of Raghavan's log-normal period fit (read
  only in a secondary summary) and the full Raghavan eccentricity law.
- Multiplicity versus mass for K, F, A and B primaries (Duchene and
  Kraus 2013 not reread).
- The Mardling-Aarseth coefficient 2.8 and exponents (secondary
  quotations).
- The weighting of the circumbinary habitable zone (abstracts only).
- The separation and mass of the Gamma Cephei binary, and Kepler-16's
  binary eccentricity (0.159).
- The Peters merger-time formula (recalled, not checked) and the claim
  that eclipse-timing planets around post-common-envelope binaries are
  stable.
- Mason et al. 2015 beyond its abstract; the Moe and Di Stefano numbers
  on white dwarf companions are read from the abstract only.
- The archive counts depend on the date and on how the archive flags
  circumbinary systems (two reviews give 45 and 31 systems for
  different criteria).

## Sources

- Raghavan et al. 2010, "A Survey of Stellar Families", ApJS 190, 1:
  [arXiv:1007.0414](https://arxiv.org/abs/1007.0414).
- Tokovinin 2014, "From binaries to multiples II: hierarchical
  multiplicity of F and G dwarfs":
  [arXiv:1401.6827](https://arxiv.org/abs/1401.6827).
- Winters et al. 2019, "The Solar Neighborhood XLV":
  [arXiv:1901.06364](https://arxiv.org/abs/1901.06364).
- Moe and Di Stefano 2017, mass-dependent binary distributions
  (ApJS):
  [arXiv:1606.05347](https://arxiv.org/abs/1606.05347).
- Moe and Kratter 2021, "Impact of binary stars on planet statistics I":
  [arXiv:1912.01699](https://arxiv.org/abs/1912.01699).
- Kraus et al. 2016, "The impact of stellar multiplicity on planetary
  systems I", AJ 152:
  [arXiv:1604.05744](https://arxiv.org/abs/1604.05744).
- Thebault and Bonanni 2025, "A complete census of planet-hosting
  binaries": [arXiv:2506.18759](https://arxiv.org/abs/2506.18759).
- Holman and Wiegert 1999, "Long-term stability of planets in binary
  systems", AJ 117, 621:
  [arXiv:astro-ph/9809315](https://arxiv.org/abs/astro-ph/9809315).
- Mardling and Aarseth 2001, MNRAS 321, 398 (not read, see section 2.2).
- Armstrong et al. 2014, "On the abundance of circumbinary planets",
  MNRAS 444, 1873: [arXiv:1404.5617](https://arxiv.org/abs/1404.5617).
- Martin and Triaud 2014, "Planets transiting non-eclipsing binaries",
  A&A 570, A91:
  [arXiv:1404.5360](https://arxiv.org/abs/1404.5360).
- Munoz and Lai 2015, PNAS 112, 9264:
  [arXiv:1505.05514](https://arxiv.org/abs/1505.05514).
- Fleming et al. 2018, ApJ 858, 86:
  [arXiv:1804.03676](https://arxiv.org/abs/1804.03676); Graham, Fleming
  and Barnes 2021: [arXiv:2011.10430](https://arxiv.org/abs/2011.10430).
- Doyle et al. 2011, Kepler-16: [arXiv:1109.3432](https://arxiv.org/abs/1109.3432);
  Orosz et al. 2012, Kepler-47: [arXiv:1208.5489](https://arxiv.org/abs/1208.5489);
  Orosz et al. 2019: [arXiv:1904.07255](https://arxiv.org/abs/1904.07255);
  Kostov et al. 2016, Kepler-1647: [arXiv:1512.00189](https://arxiv.org/abs/1512.00189);
  Kostov et al. 2020, TOI-1338: [arXiv:2004.07783](https://arxiv.org/abs/2004.07783).
- Hatzes et al. 2003, Gamma Cephei Ab:
  [arXiv:astro-ph/0305110](https://arxiv.org/abs/astro-ph/0305110);
  Beichman et al. 2025, Alpha Centauri A candidate:
  [arXiv:2508.03814](https://arxiv.org/abs/2508.03814).
- Kaltenegger and Haghighipour 2013, "Calculating the habitable zone of
  binary star systems I: S-type", ApJ 777, 165:
  [arXiv:1306.2889](https://arxiv.org/abs/1306.2889); Haghighipour and
  Kaltenegger 2013, "II: P-type" (as read on the arXiv page):
  [arXiv:1306.2890](https://arxiv.org/abs/1306.2890).
- Kane and Hinkel 2013, circumbinary habitable zones (ApJ; the volume
  differs between the bibliographies found, so it is left out):
  [arXiv:1211.2812](https://arxiv.org/abs/1211.2812).
- Mason et al. 2015, "Circumbinary habitability niches", IJA 14, 391:
  [arXiv:1408.5163](https://arxiv.org/abs/1408.5163).
- Popp and Eggl 2017, "Climate variations on Earth-like circumbinary
  planets", Nature Communications 8, 14957,
  [doi:10.1038/ncomms14957](https://doi.org/10.1038/ncomms14957).
- Forgan 2016, "Milankovitch cycles of terrestrial planets in binary
  star systems", MNRAS 463, 2768:
  [arXiv:1608.05592](https://arxiv.org/abs/1608.05592).
- Vanderburg et al. 2020, WD 1856+534 b:
  [arXiv:2009.07282](https://arxiv.org/abs/2009.07282); Sigurdsson et
  al. 2003, PSR B1620-26: [arXiv:astro-ph/0307339](https://arxiv.org/abs/astro-ph/0307339).
- El-Badry et al. 2023, Gaia BH1:
  [arXiv:2209.06833](https://arxiv.org/abs/2209.06833); Gaia
  Collaboration 2024, Gaia BH3:
  [arXiv:2404.10486](https://arxiv.org/abs/2404.10486).
- Weisberg and Huang 2016, PSR B1913+16:
  [arXiv:1606.02744](https://arxiv.org/abs/1606.02744).
- NASA Exoplanet Archive, composite planet table (`pscomppars`),
  queried through the
  [TAP service](https://exoplanetarchive.ipac.caltech.edu/docs/TAP/usingTAP.html)
  on 2026-11-18 (6,470 planets).
