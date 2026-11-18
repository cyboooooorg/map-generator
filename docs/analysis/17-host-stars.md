# Analysis: other host stars

Study for issue #375, requested on 2026-11-09 and extended on
2026-11-15. Written on 2026-11-18. The sibling study on binary and
multiple systems is #376 (the binary-systems study); what a second star
changes is left to it.

The generator draws one kind of host: a main-sequence dwarf, O to M
(`docs/analysis/10-stars.md`). The issue asks which other objects could
host a system, what a planet would receive from each, what the viewer
would show, and what the code would need. The objects: neutron stars
(pulsars, millisecond pulsars, magnetars), stellar black holes, the
other luminosity classes (subgiants, giants, supergiants), white dwarfs,
brown dwarfs, Wolf-Rayet stars, and quasi-stars.

This document is a study and a proposal. It decides nothing: the last
section is a recommendation to discuss with the user.

Reading convention: a number without a mark comes from the cited source
or from the NASA Exoplanet Archive query named in the text. "Own
arithmetic" is a calculation made for this study from cited inputs.
Claims that could not be verified are marked `[M]` and listed in
"Claims not verified" at the end.

## 1. What the code does today

- **One luminosity class.** `crates/mapgen/src/star.rs:3-6` documents
  that `V` is "the only luminosity class modelled". A type is a float,
  `class × 10 + subtype` (`star.rs:22-25`, `SUN` = 42), interpolated in
  Pecaut & Mamajek's dwarf sequence (`SEQUENCE`, `star.rs:57`, O3V to
  M9.5V; `TYPE_RANGE`, `star.rs:148`). `values(code)` (`star.rs:166`)
  returns temperature, luminosity, radius and mass from the type alone;
  13 call sites outside `star.rs` rely on that shape.
- **Draw.** `draw_type` (`star.rs:247`) picks a class with the
  solar-neighbourhood shares `CLASS_SHARES` (`star.rs:242`: M 76.6 %,
  K 12.1 %, G 7.6 %, F 3 %, and so on, summing to the seven classes);
  white dwarfs, giants, brown dwarfs and remnants are not in it.
  `parse_type` (`star.rs:197`) accepts a class letter, a subtype and an
  optional `V`. `system.rs:1029-1041` draws the star, then its age from
  its own stream (`stream::AGE` = 41, `noise.rs:195`).
- **Age is capped by the main sequence.** `draw_age_gyr`
  (`star.rs:451`) draws between 0.5 Gyr and the star's lifetime, and
  `resolve_age_gyr` (`star.rs:462`) refuses an asked age beyond
  `lifetime_gyr` (`star.rs:356`, from a mass table `LIFETIMES`,
  `star.rs:337`). A system cannot be older than its star's main
  sequence, so no post-main-sequence host can occur today.
- **Light and climate.** `StarInfo` (`star.rs:487`, built by
  `StarInfo::new` / `for_planet`, `star.rs:575`) holds the flux
  `luminosity / a²`, the habitable-zone position and edges, the
  equilibrium temperature with Earth's albedo (`equilibrium_k`,
  `star.rs:318`), the year and the star's apparent diameter
  (`angular_diameter_deg`, `star.rs:329`). `habitable_fluxes`
  (`star.rs:287`) is Kopparapu's fit, valid for 2 600 to 7 200 K and
  clamped outside. `star_temperatures` (`star.rs:764`) scales Earth's
  temperature rows with `relative_flux` (`star.rs:654`): unregulated
  planets in kelvin with `S^(1/4)`, regulated ones through the habitable
  zone. The planet's colours under its star come from `light_colour`, a
  black body (`star.rs:416`).
- **System layout.** `INNER_EDGE_AU = 0.02` (`system.rs:86`) is the
  closest orbit; the ice line is `2.7 √L` (`system.rs:1043`);
  `SystemStar` (`system.rs:190`) stores the type string, the metallicity
  and the activity. `SYSTEM_FORMAT_VERSION` is 9 (`system.rs:82`).
- **Activity and the living star.** `star_activity` (`activity.rs:282`)
  derives rotation (gyrochronology), activity (Rossby number), spots and
  granulation for a convective envelope (`Envelope`, `activity.rs:36`;
  radiative above `RADIATIVE_TEFF_K` = 7 200 K, `activity.rs:107`).
  Its draws come from `stream::STAR_ACTIVITY` (`noise.rs:192`). The
  living-star design (`docs/superpowers/specs/2026-11-08-living-star-design.md`,
  issues #343 to #345) adds eruptions and a wind (part 2) and magnetic
  fields of every body (part 3).
- **Tides.** `spin.rs` and `star.rs:371` (`locking_gyr`) take the host's
  mass; `tides.rs:136` (`roche_km`) gives the Roche limit from the host's
  radius and density.
- **Viewer.** The system view draws the star with `StarView.ts`
  (shader `starSurface`, `web/src/lib/viewer/starSurface.ts`); the sky
  seen from a planet draws it with `hostStarSurface.ts` and the sprite
  declared at `PlanetView.ts:511`. Both assume a star with a granulated,
  limb-darkened photosphere and a Sunlight direction.
- **Physical planet model.** The planet's type is derived from its
  composition, interior and regime (issue #423, `docs/analysis/16-planet-types.md`).
  Everything below keeps that model: a new host changes the inputs
  (flux, spectrum, tides, age), not the rules.

## 2. Research

### 2.1 How common each object is

The generator's random draw uses solar-neighbourhood frequencies. These
are the numbers a draw would have to respect.

| Object | Per star (solar neighbourhood) | Source |
|---|---|---|
| M dwarfs | 72.5 % | Mamajek memo |
| White dwarfs | 5.9 % (1 in 17) | Mamajek memo |
| "Evolved stars" (subgiants and giants) | 1.1 % (1 in 93) | Mamajek memo |
| Red giants (G, K, M) | 0.33 % (1 in 300) | Mamajek memo |
| O stars | about 5 × 10⁻⁷ | Mamajek memo |
| L, T, Y brown dwarfs | about 0.16 per star, a lower bound | Kirkpatrick 2021, own arithmetic |
| Neutron stars | about 2 × 10⁸ in the Galaxy | Keane & Kramer 2008, own arithmetic |
| Stellar black holes | about 1.2 × 10⁸ single, 9.3 × 10⁶ in binaries | Olejak 2020 |
| Wolf-Rayet stars | 1 200 (+300, −100) in the Galaxy | Rosslowe & Crowther 2015 |
| Quasi-stars | none observed | Begelman 2008 |

Notes on the rows:

- Mamajek's table counts all stars of the solar vicinity (0.0984 per pc³,
  white dwarfs included, brown dwarfs, neutron stars and black holes
  excluded) and says every density is a lower limit.
  See [the memo](https://www.pas.rochester.edu/~emamajek/memo_star_dens.html).
- Brown dwarfs: [Kirkpatrick et al. 2021](https://arxiv.org/abs/2011.11616)
  list 525 L, T and Y dwarfs within 20 pc, statistically complete except
  for types T8 and later. The volume holds about 3 300 stars (own
  arithmetic: 0.0984 pc⁻³ × 33 510 pc³), hence about 0.16 L, T or Y dwarf
  per star. Late-M brown dwarfs are not in that count, so it is a lower
  bound. Their mass function is a power law, dN/dM ∝ M^−α with
  α = 0.6 ± 0.1.
- Neutron stars: [Keane & Kramer 2008](https://arxiv.org/abs/0810.1512)
  give a core-collapse rate of 1.9 ± 1.1 per century, so about 2 × 10⁸
  neutron stars in 10 Gyr at a constant rate (own arithmetic; the rate
  was not constant). Only a small part is seen: the ATNF catalogue
  listed 4 303 pulsars in version 2.6.4 (2025-08-19, see
  [the catalogue history](https://www.atnf.csiro.au/research/pulsar/psrcat/catalogueHistory.html)),
  and [Faucher-Giguère & Kaspi 2006](https://arxiv.org/abs/astro-ph/0512585)
  estimate about 120 000 potentially observable ordinary pulsars. The
  per-star share cannot be given: I did not verify the Galaxy's total
  star count `[M]`; the order is one in a few hundred to a thousand.
- Black holes: [Olejak et al. 2020](https://arxiv.org/abs/1908.08775)
  (population synthesis) give about 1.2 × 10⁸ single black holes of mean
  mass 14 M☉ and 9.3 × 10⁶ in binaries. [El-Badry et al. 2023](https://arxiv.org/abs/2209.06833)
  quote "of order 10⁸", about 20 dynamically confirmed black holes in
  X-ray binaries and about 50 more suspected.
- Wolf-Rayet stars: [Rosslowe & Crowther 2015](https://arxiv.org/abs/1412.0699)
  estimate 1 200 (+300, −100) in the Galaxy, an average WR phase of
  about 0.25 Myr at the present star formation rate.

Consequence for the draw: apart from white dwarfs (6 %) and evolved
stars (1 %), none of these objects is a plausible share of a random
draw in the sense of "what a randomly chosen star is". Pulsars with
planets are three systems among about 4 000 known pulsars. Any of them
would be an explicit choice of the user, or a rare, named archetype.

### 2.2 White dwarfs

**What and how common.** The cooled cores of stars that have shed their
envelope: Earth-sized (the host of WD 1856+534 b has R = 0.0131 R☉ and
M = 0.518 M☉, NASA Exoplanet Archive), 5.9 % of the stars of the solar
vicinity (table above). They no longer burn anything: they cool, from
tens of thousands of kelvin to below 5 000 K in the older ones.

**Planets: survival.** A white dwarf's planets are either survivors of
the giant phases or second-generation bodies.

- *Confirmed survivors.* [WD 1856+534 b](https://arxiv.org/abs/2009.07282)
  (Vanderburg 2020): archive values R = 10.4 R⊕, P = 1.408 d,
  a = 0.0204 AU around a DC white dwarf of 4 710 K. Its progenitor's giant
  envelope would have covered that orbit, so it arrived after the giant
  phase (the discovery paper's argument, not re-read here `[M]`).
  [Limbach et al. 2025](https://arxiv.org/abs/2504.16982) detected its
  mid-infrared thermal emission with JWST: about 186 K, which rules out
  a brown dwarf, with a mass of at most about 6 M_Jup (read in a
  secondary summary `[M]`).
  [MOA-2010-BLG-477Lb](https://arxiv.org/abs/2110.07934) (Blackman
  2021): a 1.4 M_Jup planet at a projected 2.8 ± 0.5 AU from a 0.53 M☉
  white dwarf, found by microlensing, evidence that wide planets survive
  the giant phases. The archive holds four planets with a host radius
  under 0.03 R☉ (DP Leo b, WD 1856+534 b, ZTF J1230-2655 b,
  ZTF J1828+2308 b; I did not check how the three others were found
  `[M]`), plus imaged wide companions (WD 0806-661 b at 2 500 AU).
- *Debris and second-generation material.* [Koester et al. 2014](https://arxiv.org/abs/1404.2617):
  of 85 DA white dwarfs, 48 (56 %) show heavy elements, and at least
  27 % of white dwarfs with cooling ages of 20 to 200 Myr are currently
  accreting planetary debris. The disintegrating bodies (KIC 12557548 b,
  K2-22 b, WD 1145+017: [16-planet-types.md](16-planet-types.md))
  show that rocky bodies reach orbits of hours to days around them.
- *Theory.* [Veras 2016](https://arxiv.org/abs/1601.05419) reviews the
  dynamics: where a planet survives depends on the maximum giant-branch
  radius, tides and the mass-loss prescription, and the studies it
  compares disagree because they use different stellar models. I did not
  extract a single engulfment distance from it.

**What a planet receives.** The light is small and red-shifting as the
star cools: from the archive values of WD 1856+534 (4 710 K,
0.0131 R☉) the luminosity is 7.6 × 10⁻⁵ L☉ (own arithmetic), so at
0.0204 AU the flux is 0.18 S⊕ and the equilibrium temperature with
Earth's albedo 167 K, close to the measured 186 K. A hot young white
dwarf gives far more ultraviolet; the cool ones give almost none.
[Agol 2011](https://arxiv.org/abs/1103.2791) defines a continuously
habitable zone for white dwarfs of 0.4 to 0.9 M☉ and below about 10⁴ K
from about 0.005 to 0.02 AU for at least 3 Gyr. The year there is hours:
at 0.01 AU around 0.6 M☉, 11 h (own arithmetic). Planets that close to
a body with a density of 4 × 10⁵ g/cm³ (own arithmetic) feel strong
tides; a fluid body of 3 g/cm³ is disrupted inside about 0.0075 AU
(Roche, own arithmetic: 2.44 R_WD (ρ_WD / ρ)^(1/3)), which is what the
debris disks mark. A planet is also eclipsed deeply: the star is the
planet's size.

**What the viewer would show.** A small, intensely bright disk, with no
granulation and no spots (a white dwarf's photosphere is a thin
atmosphere on a degenerate core; the surface shader's granulation and
active regions do not apply). Seen from WD 1856+534 b, the star spans
0.34° (own arithmetic with `angular_diameter_deg`'s 0.5332 × R / a),
about two thirds of the Sun's. Colour follows the temperature
(`light_colour`, `star.rs:416`, already works for any black body).

**What the generator would need.**

- A state, not a type: the white dwarf's temperature and luminosity
  depend on its cooling age, which is the system's age minus the
  progenitor's lifetime. `lifetime_gyr` (`star.rs:356`) already gives
  the lifetime from a mass; the initial-to-final mass relation and a
  cooling track are not sourced yet `[M]`.
- A host-dependent inner edge: `INNER_EDGE_AU = 0.02` (`system.rs:86`)
  excludes the whole white-dwarf habitable zone and WD 1856+534 b's own
  orbit. The limit should come from `tides::roche_km` (`tides.rs:136`).
- A survivor rule: planets drawn for the progenitor, removed inside the
  engulfment distance, orbits widened by the mass loss (orbits expand
  as the host loses mass, see Veras), then optional second-generation
  bodies (debris, disks).
- Climate: `relative_flux` and `star_temperatures` work on bolometric
  flux; `habitable_fluxes` (`star.rs:287`) is clamped to 2 600 to
  7 200 K and Kopparapu's fit is for main-sequence spectra, so the
  habitable zone of a white dwarf needs its own model (Agol 2011), and
  the ultraviolet budget is not represented.
- Spin: tides lock a planet at 0.01 AU within a fraction of a million
  years (`star.rs:371`); the existing locking code applies as it is.
- Activity: none (a white dwarf has no convective envelope worth
  modelling and no cycle), so `star_activity` needs a "quiet" branch.

### 2.3 Neutron stars and pulsars

**What and how common.** The 1.4 M☉, 10 km remnants of core-collapse
supernovae (the archive uses 1.4 M☉ for the pulsar hosts). About 2 × 10⁸
formed in the Galaxy (table above); a few thousand are seen as radio
pulsars. Normal pulsars rotate with P of about 0.5 s; **millisecond
pulsars** have 1.4 ms ≲ P ≲ 30 ms and very small period derivatives,
and are old neutron stars spun up by a companion ([Lorimer 2008](https://arxiv.org/abs/0811.0762),
which counted 83 binary and millisecond pulsars in the disk and 140 in
26 globular clusters at the time of writing; the numbers have grown).
Born with a kick, the mean birth speed of the order of 400 km/s
(Hobbs et al. 2005, [arXiv:astro-ph/0504584](https://arxiv.org/abs/astro-ph/0504584);
the figure comes from a secondary summary, not from the full text
`[M]`).

**Planets: the known examples.** The archive lists 8 planets found by
pulsar timing (queried on 2026-11-18):

| Planet | Orbit | Mass | Note |
|---|---|---|---|
| PSR B1257+12 b | 0.19 AU, 25.26 d | 0.02 M⊕ | |
| PSR B1257+12 c | 0.36 AU, 66.54 d | 4.3 M⊕ | |
| PSR B1257+12 d | 0.46 AU, 98.21 d | 3.9 M⊕ | |
| PSR B1620-26 b | 23 AU | 2.5 M_Jup | in the cluster M4 |
| PSR J1719-1438 b | 0.0044 AU, 2.18 h | 1.2 M_Jup | density above 23 g/cm³ |
| PSR J2322-2650 b | 0.0102 AU, 7.75 h | 0.8 M_Jup | millisecond pulsar |
| PSR B0329+54 b | 10.3 AU, 10 140 d | 2 M⊕ | candidate, disputed `[M]` |
| M62H b | P = 0.133 d | 2.5 M_Jup | cluster M62, not checked `[M]` |

PSR B1257+12 is the confirmed system: planets c and d were announced by
[Wolszczan & Frail 1992](https://doi.org/10.1038/355145a0), and
[Konacki & Wolszczan 2003](https://arxiv.org/abs/astro-ph/0305536)
measured their true masses (4.3 and 3.9 M⊕) from their mutual
perturbations. Planets c and d lie near a 3:2 resonance (own
arithmetic: 98.21 / 66.54 = 1.476).
[Sigurdsson et al. 2003](https://arxiv.org/abs/astro-ph/0307339) found
that PSR B1620-26's planet has a white-dwarf companion of 0.34 ± 0.04 M☉
and 480 Myr; the planet's presence is tied to a dynamical exchange in
the cluster core. [Bailes et al. 2011](https://arxiv.org/abs/1108.5201)
describe PSR J1719-1438's companion as possibly an ultra-low-mass carbon
white dwarf of density above 23 g/cm³ (a "diamond planet"), and
[Spiewak et al. 2018](https://arxiv.org/abs/1712.04445) the 3.5 ms
PSR J2322-2650 with a companion of about 10⁻³ M☉ on a 7.75 h circular
orbit.

**Planets: formation and rarity.** Pulsar planets are rare.
[Kerr et al. 2015](https://arxiv.org/abs/1507.06982) searched 151 young
pulsars and found none; for the older ones they exclude Mercury-like
planets in a third of the sample and planets above 0.4 M⊕ with periods
under 1 yr in all but 5 % of the systems. If planets form from a
supernova fallback disk, such disks do not form, stay under 0.1 AU,
are disrupted, or form planets over more than 2 Myr.
[Starovoit & Rafikov 2017](https://arxiv.org/abs/1609.06409) propose
that pulsar planets need the destruction of a companion star (extreme
mass ratio) and a dead zone in the disk, both unlikely, since the
pulsar's irradiation heats the inner disk; I read only their abstract
and introduction. [Margalit & Metzger 2017](https://arxiv.org/abs/1608.08636)
show that the tidal disruption of a C/O white dwarf by a neutron-star
companion gives a debris disk that matches PSR B1257+12's planets.
Planets that were there before the supernova are not expected to have
survived: I found no source that quantifies it `[M]`.

**What a planet receives.** Almost no starlight: a neutron star is 10 km
across, and a hot one (10⁶ K) radiates about 0.19 L☉ in X-rays (own
arithmetic), nothing in the visible. The planet is heated by the
neutron star's X-rays and by the **pulsar wind**, set by the spin-down
luminosity. [Patruno & Kama 2017](https://arxiv.org/abs/1705.07688) (A&A
608, A147) model this. For radio pulsars the median spin-down luminosity
is about 4 × 10³² erg/s (0.10 L☉, own conversion). For PSR B1257+12 they
use L_X ≈ 5.5 × 10²⁹ erg/s (1.4 × 10⁻⁴ L☉) and L_sd ≈ 1.5 × 10³⁴ erg/s
(3.9 L☉). They define the habitable zone by 175 K < T_eq < 270 K, with

> T_eq = η [ L_NS / (16 π σ D²) ]^(1/4)  (their Eq. 22)

where η is the fraction of the X-ray and gamma flux thermalised in the
atmosphere. Applying it (own arithmetic): with X-rays alone and η = 1
the three planets are at 45 to 70 K, too cold, as the paper finds; with
the whole spin-down power and η of 0.3 to 0.5 they would sit at 270 K.
The authors conclude that super-Earths could keep a warm atmosphere,
that atmospheres survive at least about 10⁸ yr if they hold a large
share of the planet's mass and for billions of years with a moderate
magnetosphere, and that, depending on poorly constrained aspects of the
wind, both super-Earths of B1257+12 could be in its habitable zone. If
the energy arrives as X-rays alone (for example by Bondi-Hoyle
accretion), planets within about 0.01 to 1 AU could be in it. All of
this is a model with a free parameter, not a measurement.

**What the viewer would show.** The star is a point (at 0.36 AU a 10 km
body spans 0.08 arcsecond, own arithmetic): the system view needs a
marker, not a disk. The rest is the pulsar's signature: a lighthouse
beam sweeping at the spin rate (invisible except through its effect on
the wind), an aurora-lit sky from the wind striking the atmosphere, and
a planet whose day is not lit by a Sun. The living-star work already
plans the wind (part 2, #344) and fields (part 3, #345) that a pulsar
would use: a rotating dipole of 10⁸ to 10⁹ G for millisecond pulsars is
the extreme case of the same drawing.

**What the generator would need.**

- A host whose heat is not `L / a²`: an explicit heating channel
  (`W/m²` from spin-down and X-rays, with η), replacing
  `relative_flux` (`star.rs:654`) in `star_temperatures` (`star.rs:764`),
  `equilibrium_k` (`star.rs:318`) and the giants' radius inflation
  (`giants::giant_radius`, which reads `equilibrium_k`). The Earth's
  temperature-profile model of `climate.rs` is built for sunlight with a
  day-night cycle; a planet heated by a wind and no light needs a second
  mode (uniform heating, no sunlit side).
- Planet draws that follow the known systems: rocky and super-Earth
  planets of 0.02 to 4 M⊕ at 0.2 to 0.5 AU, a few in near resonance;
  hot Jupiter-mass companions only for a "black widow" variant (J1719
  and J2322), which is a binary question (#376).
- Age and spin: pulsars are recycled or young; a millisecond pulsar is
  old (Gyr) and spun up by a companion that is usually absent from the
  system. The age model of `draw_age_gyr` does not apply (the planets'
  age is that of the system, the pulsar's characteristic age is
  separate).
- Streams: a new stream of its own, so that existing seeds keep their
  draws (`noise.rs`: `GALAXY` = 43 is the last of the run that starts at
  40; `SYSTEM` = 64 and `ARCHETYPE_AGE` = 65 are the highest in use).

### 2.4 Magnetars

**What and how common.** Neutron stars whose activity is powered by the
decay of an ultra-strong field. [Kaspi & Beloborodov 2017](https://arxiv.org/abs/1703.00068)
count about 30 detected, with dipole fields of 10¹⁴ to 10¹⁵ G, that is
10¹⁰ to 10¹¹ T (1 T = 10⁴ G; the issue's 10⁹ T lower bound is the
10¹³ G end). The birth rates in [Keane & Kramer 2008](https://arxiv.org/abs/0810.1512)
are about 0.1 per century for soft gamma repeaters (a lower limit) and
0.2 ± 0.2 for anomalous X-ray pulsars. Their giant flares release over
10⁴⁴ erg; the 2004 flare of SGR 1806-20 released over 10⁴⁶ erg with a
peak luminosity of 10⁴⁷ erg/s at a distance of 15 kpc (Kaspi &
Beloborodov).

**Planets.** None is known. Everything said for pulsars holds, with a
younger, more violent star. Arithmetic on one giant flare of 10⁴⁶ erg
at 1 AU: a fluence of 3.5 × 10¹⁸ erg/cm², which is the Sun's total
output at that distance over about 80 000 years (own arithmetic); a
planet in such a system would lose its atmosphere unless it is far or
magnetised, and the flare history would be part of the planet's story.
The persistent X-ray luminosity and the active lifetime are not
verified here `[M]`.

**Viewer and generator.** A magnetar is a neutron star with a
different field and a flare model. Part 2 of the living star
(eruptions at a statistical rate) and part 3 (fields) are the right
place: a flag on the neutron-star host, not a host kind of its own.

### 2.5 Stellar-mass black holes

**What and how common.** About 10⁸ in the Galaxy (table above); the
known ones are 20 dynamically confirmed in X-ray binaries, about 50
suspected, and a handful of quiet binaries found by astrometry. The
closest is [Gaia BH1](https://arxiv.org/abs/2209.06833): 9.62 ± 0.18 M☉
with a 0.93 M☉ Sun-like star, P = 185.6 d, e = 0.45, at 480 pc, so
about 1.4 AU (own Kepler arithmetic). Gaia BH3 has a 33 M☉ black hole
([Sbarufatti et al. 2025](https://arxiv.org/abs/2503.01679)).

**Planets: can they form or survive?** No planet is known, and the
published argument is against it. [Menou, Perna & Hernquist 2001](https://arxiv.org/abs/astro-ph/0102478)
model supernova fallback disks: around a black hole, even a 1 M☉
disk spreads only to about 5 × 10¹⁰ cm (2 × 10¹¹ cm if weakly
irradiated), not beyond the tidal disruption radius of about
3 × 10¹¹ cm, so "the immediate formation of planets is equally
unlikely"; the disks become neutral and end their activity at ages of
10³ to 10⁴ yr, compact (10⁹ to 10¹¹ cm). Planets orbiting the
progenitor are not expected to survive its collapse (no quantitative
source found `[M]`). A black hole has no light of its own: an isolated
one is dark, and a planet there would receive nothing; with a companion
star, the planets belong to the companion (as in Gaia BH1), which is the
binary study's subject (#376). A black hole with an accretion disk
(only in an interacting binary) would shine in X-rays; I found no model
of planets receiving that light `[M]`.

**What the viewer would show.** The one object where the viewer would
show something new. A black hole of 10 M☉ has a Schwarzschild radius
of 29.5 km (own arithmetic, 2.953 km per M☉) and an innermost stable
orbit at 88.6 km; the visible shadow is about 2.6 times the horizon
radius. From 1 AU that is 5 × 10⁻⁷ rad, a sub-pixel point, so the
system view shows a dot, and the lensing is visible only in a close-up
"star view". [James et al. 2015](https://arxiv.org/abs/1502.03808) (the
Interstellar renderer) give the method for the lensed sky and disk of a
spinning black hole. With no accretion disk, the close-up is a black
disk with a lensed starfield.

**What the generator would need.** A host with no `luminosity`; with a
disk, an accretion luminosity model; planets only as a fantasy or in a
binary. Nothing here would be a physical outcome of the generator
(the planet model would have no heat source other than internal heat).

### 2.6 Subgiants, giants and supergiants

**What and how common.** Stars that have left the main sequence (IV),
or burn shells with an extended envelope (III), or are very luminous
(I, II). 1.1 % of the solar neighbourhood's stars, 0.33 % red giants
(table above). A K giant of 30 R☉ and 4 300 K has about 280 L☉ (own
arithmetic), which puts a flux of one Earth's at about 17 AU.

**Planets: known and surviving.** Queried in the NASA Exoplanet Archive
on 2026-11-18 (6 470 planets in `pscomppars`):

| Host class | Planets | Hosts |
|---|---|---|
| "III" in the spectral type | 142 | 121 |
| "IV" (not III) | 134 | 103 |
| "II" | 2 | BD+20 2457 b and c |
| radius above 10 R☉ | 103 | 95 |
| radius above 3 R☉ and P < 60 d | many | e.g. Kepler-91 b |

No planet has an Ia or Ib supergiant host. The archive's planets around
giants are, in nearly all cases, found by radial velocity; the
innermost around a host larger than 10 R☉ is 24 Boo b at 0.194 AU
(host 12.2 R☉, 1.05 M☉), then TYC 4282-00605-1 b at 0.42 AU, 8 UMi b
at 0.46 AU and HD 102272 b at 0.51 AU. Close-in planets exist around
subgiants, such as Kepler-91 b (P = 6.25 d, host 6.3 R☉) and
TYC 3667-1280-1 b (0.21 AU, 5.4 M_Jup, host 6.3 R☉). Masses of the
massive giants (HD 240237 at 8.76 M☉ in the archive) are model values
and the heaviest are uncertain `[M]`. [Johnson et al. 2010](https://arxiv.org/abs/1005.3084)
find the occurrence of giant planets rising with host mass (3.5 % at
0.5 M☉, 14 % at 2 M☉ at solar metallicity, as recorded in
16-planet-types.md). Wide giants orbit massive stars: μ² Sco b
(B2 IV, 9.1 M☉, 242 AU, archive) and
b Cen (AB)b (about 10.9 M_Jup at about 556 AU around a 6 to 10 M☉
binary, [Janson et al. 2021](https://doi.org/10.1038/s41586-021-04124-8),
Nature 600, 231; the DOI was checked in Crossref, the mass and
separation come from secondary pages `[M]`).

Survival: the planets inside the giant's maximum radius are engulfed.
[Veras 2016](https://arxiv.org/abs/1601.05419) says the minimum
surviving distance depends mostly on the maximum RGB radius and on the
adopted tides and mass loss, with results that differ between studies;
the other planets move outward as the star loses mass. The archive
shows the effect: no planet within about 0.2 AU of a host above 10 R☉.

**What a planet receives.** More light, from a far larger disk, at a
much redder colour, and a habitable zone that has moved out and does not
last. [Ramirez & Kaltenegger 2016](https://arxiv.org/abs/1605.04924)
find that a planet can stay in the post-main-sequence habitable zone
from about 200 Myr (hottest stars of their grid) to 9 Gyr (coolest) at
solar metallicity, longer at higher metallicity. For the K giant above,
the habitable-zone planet orbits at 17 AU with a period of about 60
years (own arithmetic, 1.3 M☉) and sees a star 0.96° across (own
arithmetic), almost two Suns. Seasons and eccentric-orbit cycles would
last decades.

**What the viewer would show.** The existing views, with other
numbers: a very large orange or red disk, with a granulation reduced to
a few huge cells (the granule size in `activity.rs:501` depends on
radius, and the model's range does not reach giants `[M]`), and a
much larger angular diameter in the sky. This is the cheapest visual
change of the study.

**What the generator would need.**

- Evolution beyond the main sequence: a state `(mass, age)` giving
  temperature, luminosity and radius on the subgiant and giant
  branches. [MIST](https://arxiv.org/abs/1604.08592) (MESA Isochrones
  and Stellar Tracks, Choi et al. 2016) cover 0.1 to 300 M☉ over
  5 ≤ log(age) ≤ 10.3, a candidate source for baked tables.
- Allow the age to pass `lifetime_gyr` (`star.rs:356`, `draw_age_gyr`
  `star.rs:451`, `resolve_age_gyr` `star.rs:462`), with a draw that
  gives the evolved fraction of about 1 % of a random draw, and
  `parse_type` (`star.rs:197`) to accept `III`, `IV`.
- Orbits: the planets are placed from the evolved star's habitable
  zone, with a minimum above the engulfment limit; the ice line
  (`2.7 √L`, `system.rs:1043`) already scales with the luminosity.
- Climate: `habitable_fluxes` holds for the temperatures of giants
  (3 800 to 5 000 K) and `star_temperatures` works from the bolometric
  flux, so the climate follows. Stellar evolution during the planet's
  history (a brightening star) is a separate effect.
- Mass loss widens the surviving orbits; the generator would draw the
  original orbit and widen it, or draw the final one directly (a
  simplification to state).

### 2.7 Brown dwarfs

**What and how common.** Objects between about 13 M_Jup (deuterium
burning) and 0.075 M☉ (the hydrogen-burning edge)
([Burrows et al. 2001](https://arxiv.org/abs/astro-ph/0103383), which
adds that the 13 M_Jup cut is a convention). They are not stars, never
reach the main sequence, and cool with age through L, T and Y types;
the coldest in the 20 pc census is WISE 0855-0714 (the only one in the
150 to 300 K bin, Kirkpatrick et al. 2021). About 0.16 per star at the
least (table above).

**Planets.** The archive holds 29 planets with a host mass under
0.08 M☉ (queried 2026-11-18): microlensing planets such as
OGLE-2012-BLG-0358L b (1.9 ± 0.2 M_Jup at about 0.87 AU from a 0.022 M☉
brown dwarf, [Han et al. 2013](https://arxiv.org/abs/1307.6335)),
imaged companions at tens to hundreds of AU (2MASS J0441+2301 b at
15 AU, CFHTWIR-Oph 98 b at 200 AU), a radial-velocity companion
(CD-35 2722 B b), and an astrometric one (DENIS-P J082303.1-491201 b at
0.36 AU, 28.5 M_Jup, which is itself brown-dwarf mass). No transiting
planet appears around a host below 0.08 M☉ in that list (TRAPPIST-1 is
0.09 M☉ and not a brown dwarf). Disks exist, but
[Pascucci et al. 2016](https://arxiv.org/abs/1608.03621) show that the
dust mass grows faster than linearly with the stellar mass (index about
1.3 to 1.9), so brown-dwarf disks are poor in solids and small planets
are what to expect.

**What a planet receives.** The light falls with age: a brown dwarf is
brightest when young. [Bolmont et al. 2011](https://arxiv.org/abs/1109.2906)
place the habitable zones of brown dwarfs at 0.001 to 0.03 AU (interior
to the Roche limit for the lowest masses), show that the habitable zone
moves inward by tens to hundreds of Myr, and that planets that form
beyond the corotation distance are repelled and most closer ones fall
in; surviving planets can have periods under 10 days. Tidal heating is
the second source of heat.

**What the viewer would show.** A dim red globe of roughly Jupiter's size
(Burrows: two to six times Jupiter's radius during deuterium burning,
which lasts from a few million to 10⁸ yr; the later radius is not
quoted here `[M]`). From 0.01 AU a body of 0.1 R☉ would span 5° (own
arithmetic), ten Suns. The colour of a cold dwarf is not a black body
(molecular bands) and is not sourced here `[M]`. A dark red sky, no activity cycle worth drawing except flares on
the youngest.

**What the generator would need.** The dwarf sequence stops at M9.5V,
2 350 K, 0.078 M☉ (`star.rs:63-65`); extending it to L, T and Y means a
state `(mass, age)` from cooling models, a `parse_type` that accepts
`L`, `T`, `Y` (`star.rs:197`), a habitable zone from Bolmont instead of
Kopparapu (`habitable_fluxes`, `star.rs:287`, clamps at 2 600 K), a
minimum distance from the Roche limit (`system.rs:86`), and small
planets only. It is a modest step because the star code already has the
form of a table of values.

### 2.8 Wolf-Rayet stars

**What and how common.** The helium-burning descendants of the most
massive stars, with radiation-driven winds that hide the surface: 1 200
(+300, −100) in the Galaxy, 10 to 25 M☉, about 10 % of a lifetime of
5 Myr, a WR phase of about 0.25 Myr on average (Rosslowe & Crowther
2015; [Crowther 2007](https://arxiv.org/abs/astro-ph/0610356)). Stellar
temperatures run from about 30 kK (WN10) to 40 kK (WN8) and 100 kK or
more (early WC and WO); mass-loss rates are 10^−5.6 to 10^−4.4 M☉/yr.

**Planets.** None is known and none is expected. A disk around a star
that will become a Wolf-Rayet (initial mass of at least 25 M☉ for WN,
Crowther) would be photoevaporated long before the 5 Myr life has
passed, and the star's mass loss and luminosity are incompatible with
solid bodies in the inner system. The massive hosts that exist are
B stars with giants at hundreds of AU (see 2.6). A planet at 1 AU of a
WR star with 10⁵·⁵ L☉ would receive 3 × 10⁵ S⊕, so 6 000 K at the
Earth's albedo (own arithmetic); one flux of Earth's needs 550 AU.

**Viewer and generator.** A wind-dominated, emission-line, blue-white
object that does not show a photosphere; the viewer's star shader has no
use for it, and a nebula would have to be drawn. Nothing here is cheap
or plausible: stay out of the generator.

### 2.9 Quasi-stars

**What they are.** Hypothetical objects of the early universe:
[Begelman, Rossi & Armitage 2008](https://arxiv.org/abs/0711.4078)
(MNRAS 387, 1649) study "accreting black holes embedded within massive
hydrostatic gaseous envelopes", the stage after the collapse of the
core of a supermassive protostar in a metal-free halo, in the model of
[Begelman, Volonteri & Rees 2006](https://arxiv.org/abs/astro-ph/0602363).
The black hole's accretion luminosity is set to the Eddington limit
for the whole mass, the envelope is convective and radiation-supported,
the photospheric temperature is a few thousand kelvin (with a limiting
value of about 4 000 K for metal-free opacity, "analogous to the Hayashi
track"), the radius 10² to 10³ AU, and the object is dispersed once it
reaches that limit; seeds of 10³ to 10⁴ M☉ could form "in less than a
few Myr". The luminosity scales as 1.4 × 10³⁸ m* erg/s in their units
(m* the envelope mass in M☉; own arithmetic: 3.7 × 10⁹ L☉ for 10⁵ M☉).

**Recent status.** Begelman & Dexter 2025
([arXiv:2507.09085](https://arxiv.org/abs/2507.09085)) argue that the
JWST "Little Red Dots" are late-stage quasi-stars; Cantiello et al. 2025
([arXiv:2512.17997](https://arxiv.org/abs/2512.17997)) study their
pulsational instability; Gentile et al. 2026
([arXiv:2606.06575](https://arxiv.org/abs/2606.06575)) discuss the
model's potential and challenges. I read only the abstracts; the
interpretation is not settled.

**Can a planet host or see one?** No. They form from metal-free gas, so
no solids exist to make planets; they live a few Myr; and a planet
inside the radius (10² to 10³ AU) would be in the envelope, where the
central temperature is of order 10⁵ to 10⁶ K. A planet outside would
have to form at hundreds of AU from nothing but hydrogen and helium.
Nobody could see one from a planet either, because no planet can exist
in the epoch (the redshift of formation is of order 20, the paper's
motivation). **Verdict: study-only.** The only thing a viewer could show
is a card or an encyclopedic object, not a system. Its radius would
exceed the system view's scale.

### 2.10 Summary: what a planet receives

| Host | Flux of starlight | Other heating | Habitable zone | Planets known |
|---|---|---|---|---|
| Giant (K, 30 R☉) | ~280 L☉, 1 S⊕ at ~17 AU | none | ~17 AU, for 0.2 to 9 Gyr | 142 + 134 |
| White dwarf (4 700 K) | 7.6 × 10⁻⁵ L☉, 0.18 S⊕ at 0.02 AU | tides | 0.005 to 0.02 AU for ≥ 3 Gyr | 4 + wide ones |
| Brown dwarf | falls with age | tides | 0.001 to 0.03 AU, moves inward | ~29, none transiting |
| Pulsar | ~none visible | wind (L_sd) and X-rays | model-dependent, 175 to 270 K with η | 8 |
| Magnetar | none | flares of 10⁴⁴ to 10⁴⁶ erg | none known | 0 |
| Black hole | none (dark) | disk X-rays if any | none | 0 |
| Wolf-Rayet | 3 × 10⁵ S⊕ at 1 AU | wind | 550 AU, no planets | 0 |
| Quasi-star | 10⁹ L☉ (hypothetical) | none | none | 0 |

## 3. What changes in the generator and viewer

### Data model

`values(code: f32)` makes the star a function of a spectral code, and
every host above except a giant's fixed snapshot needs a state:
white dwarfs and brown dwarfs cool, evolved stars depend on mass and
age, neutron stars have no luminosity in the sense of `L / a²`. The
smallest change that keeps existing systems identical:

- add a `HostKind` (main sequence, evolved, white dwarf, brown dwarf,
  neutron star) to `SystemStar` (`system.rs:190`) with the numbers the
  rest of the code already reads (`teff_k`, `luminosity`, `radius`,
  `mass` in `StarInfo`, `star.rs:487`), plus an optional `heating`
  field for hosts whose heat is not starlight;
- parse the extended spectral types in `parse_type` (`G2V`, `K3III`,
  `DA`, `L5`, `PSR`);
- keep `values(code)` for the main sequence and add a second entry
  point for a state, so the 13 outside callers keep working.

Existing systems stay unchanged if the new hosts are chosen by the
user or drawn from a new stream after `GALAXY` (`noise.rs:200`), with a
zero share in the default random draw; `draw_type` and its
`CLASS_SHARES` are not touched. A change of the stored `system.json`
shape (a new host field) is a `SYSTEM_FORMAT_VERSION` bump
(`system.rs:82`); worlds may change if new planets stay coherent (the
project's generation-changes policy), so no migration is needed.

### Climate

`star_temperatures` and `relative_flux` stay valid for bolometric
flux (giants, white dwarfs, brown dwarfs). Needed: the host-dependent
habitable zone (Agol, Bolmont, Ramirez & Kaltenegger instead of
Kopparapu's 2 600 to 7 200 K fit), a heating channel for the pulsar,
and the absence of ultraviolet and X-ray photochemistry in the model
(a known gap, whatever the host).

### Orbits and tides

`INNER_EDGE_AU` (`system.rs:86`) becomes a function of the host (Roche
limit with `tides::roche_km`, engulfment distance for evolved hosts).
The locking and circularisation times (`star.rs:371`, `spin.rs`) work
as they are; the planets there are locked.

### Living star

`star_activity` (`activity.rs:282`) needs a "quiet" branch (white
dwarf, brown dwarf above a threshold age, neutron star) and a
giant-granulation variant. The wind and the field of the living-star
parts 2 and 3 (#344, #345) carry the pulsar wind and the neutron-star
dipole if their formulas take the star's luminosity and field as
inputs rather than assuming a Sun-like star.

### Viewer

| Host | System view | Planet's sky |
|---|---|---|
| Giant | existing `StarView.ts`, larger, coarser granulation | huge disk, redder `light_colour` |
| White dwarf | small bright disk, no surface detail | 0.3° disk, blue-white |
| Brown dwarf | dim red-magenta globe | very large dim disk |
| Pulsar | marker, beam and wind overlay | black sky, aurorae |
| Black hole | dot; lensed close-up (new shader) | lensed background |

The star shader (`starSurface.ts`) and the sky sprite
(`hostStarSurface.ts`, `PlanetView.ts:511`) assume a photosphere; a
white dwarf needs a plain disk mode, a pulsar a point mode. The
orrery and the cards (`SystemOrrery.svelte`, `StarPane.svelte`) need the
new type names.

## 4. Recommendation (proposal, to decide with the user)

The order below weighs three criteria for each object: how plausible
planets are (known examples, a model), how interesting the result is in
the viewer, and how much code it costs.

| # | Object | Plausibility | Visual interest | Cost | Verdict |
|---|---|---|---|---|---|
| 0 | Host-kind data model | (enabler) | none | medium | first |
| 1 | Giants and subgiants | very high (276 planets) | high, existing view | medium | add |
| 2 | White dwarfs | high (WD 1856+534 b confirmed, debris) | high, new regime | medium | add |
| 3 | Neutron stars / pulsars | high (8 planets, 3 systems) | very high, new look | high | add |
| 4 | Brown dwarfs | medium (microlensing, imaging) | low | low to medium | add |
| 5 | Magnetar flag | none known | medium | low once 3 exists | with 3, optional |
| 6 | Black holes | none | very high as a showcase | high | study-only, or a lensed close-up |
| 7 | Wolf-Rayet | none | low | very high | out |
| 8 | Quasi-stars | none, hypothetical | none | very high | out |

Reasons:

- **Evolved hosts first**: the physics is already in the code (same
  flux, same climate), the evidence is the largest (hundreds of
  planets), and the viewer already draws a star; the new work is the
  evolution state, the engulfment rule and a sparser orbit layout.
- **White dwarfs second**: a confirmed planet, a unique habitable zone
  and a visibly different sky, at the price of a cooling state and a
  host-dependent inner edge, both of which the giants step prepares.
- **Pulsars third**: they have the most distinctive result (a sunless
  world heated by a wind) and real systems to copy (PSR B1257+12), but
  they need the heating channel and a second climate mode; the
  irradiation model is itself uncertain (Patruno & Kama's η).
- **Brown dwarfs fourth**: low cost, low visual novelty, little
  evidence for planets that a generator could draw.
- **Black holes**: no planet is expected to form (Menou 2001); keep them
  as a viewer showcase only if the user wants a lensed close-up, and
  otherwise as the companion in a binary system (#376).
- **Wolf-Rayet stars and quasi-stars stay out**: no planets can
  exist (disks do not survive, no metals, lifetimes of Myr), and the
  viewer would need new objects (nebulae, a 10² to 10³ AU envelope).
  Quasi-stars are hypothetical and debated (the Little Red Dots
  interpretation is recent); they belong to the study only.

### Follow-up issues the proposal would become

To file after the user decides; none is filed by this study.

1. **Host kinds: a state for stars that are not main-sequence dwarfs.**
   Add `HostKind`, extended `parse_type`, a state `(mass, age)` entry
   point, a host-dependent inner edge, a quiet-star branch in
   `activity.rs`, and a new draw stream; no change to default draws.
2. **Evolved hosts: subgiants and red giants.** Stellar tracks (for
   example MIST) for the post-main-sequence, ages beyond the
   main-sequence lifetime, engulfment limit, widened orbits, viewer
   scale for a 10 to 100 R☉ star.
3. **White-dwarf hosts.** Cooling state, survivors of the giant phase,
   white-dwarf habitable zone (Agol 2011), Roche-limited inner edge,
   debris and disintegrating bodies as a later extension.
4. **Neutron-star hosts: pulsar planets.** A heating channel from
   spin-down and X-rays (Patruno & Kama), a PSR B1257+12-like archetype
   with near-resonant super-Earths, point-source rendering and wind
   aurorae; magnetar flag with giant flares as a part of the living-star
   eruption model.
5. **Brown-dwarf hosts.** Extend the dwarf sequence to L, T and Y with a
   cooling state and Bolmont's habitable zone, small planets only.
6. **Optional: a lensed black-hole view.** A close-up star view with a
   lensing shader (James et al. 2015 method), with no planets; only if
   the user wants the showcase.

## 5. Open points

- Initial-to-final mass relation and a white-dwarf cooling track: to
  source (not done here).
- The engulfment distance for the generator: a number from tracks and a
  tidal prescription, to choose and to cite.
- Whether the draw should include evolved stars and white dwarfs at
  their real shares (1 % and 6 %), or only on request; the default
  shares keep every existing seed unchanged.
- Whether a giant's planets should be drawn as survivors of a
  main-sequence architecture (consistent with #423) or drawn fresh
  around the giant; the former is more faithful.
- A planet heated by a wind has no day: the second climate mode and the
  tectonic and atmosphere models of #423 need a decision on what
  "insolation" means there.
- How a binary changes all of this (a white dwarf or neutron star with a
  companion, recycled pulsars, black widows): the binary-systems study
  (#376).
- The unit of the magnetar field in the issue (10⁹ to 10¹¹ T) against
  the literature's 10¹⁴ to 10¹⁵ G (10¹⁰ to 10¹¹ T): the issue's range
  is the wider one.

## Claims not verified

- The mean pulsar birth speed of about 400 km/s (Hobbs et al. 2005):
  read in a secondary listing of the abstract, not found in the full
  text.
- The Galaxy's total star count, hence the per-star share of neutron
  stars and black holes.
- PSR B0329+54 b (candidate, disputed, I read only the first lines of
  [Starovoit et al. 2017](https://arxiv.org/abs/1710.01153)) and
  M62H b (not checked).
- That planets existing before a supernova do not survive it, and
  that planets receive light from a black hole's accretion disk: no
  source found.
- WD 1856+534 b's mass limit (secondary summary of Limbach 2025); the
  claim that its orbit lay inside the progenitor's envelope (the
  discovery paper not re-read); the initial-to-final mass relation and
  the cooling track.
- The three other white-dwarf planets in the archive (DP Leo b,
  ZTF J1230-2655 b, ZTF J1828+2308 b): method not checked.
- b Cen (AB)b: its mass and separation come from secondary pages (the
  DOI was checked in Crossref).
- The magnetar persistent X-ray luminosity and active lifetime.
- The granulation scale of giants in the activity model of
  `activity.rs`.
- The quasi-star and Little Red Dot papers (2025, 2026): abstracts
  only.
- Starovoit & Rafikov 2017 and Menou et al. 2001 were read in part (the
  abstract and the passage quoted).
- The minimum mass of the heaviest giant hosts (HD 240237: 8.76 M☉ in
  the archive).

## Sources

- NASA Exoplanet Archive, table `pscomppars`, queried on 2026-11-18
  through the [TAP service](https://exoplanetarchive.ipac.caltech.edu/TAP)
  (6 470 planets): counts by discovery method, host spectral type,
  host radius and host mass; planet values quoted in the text.
- Mamajek, "Number densities of stars of different types in the solar
  vicinity": [memo](https://www.pas.rochester.edu/~emamajek/memo_star_dens.html).
- Kirkpatrick et al. 2021, "The field substellar mass function ... 525
  L, T and Y dwarfs within 20 pc": [arXiv:2011.11616](https://arxiv.org/abs/2011.11616).
- Keane & Kramer 2008, "On the birthrates of Galactic neutron stars":
  [arXiv:0810.1512](https://arxiv.org/abs/0810.1512).
- Faucher-Giguère & Kaspi 2006, "Birth and evolution of isolated radio
  pulsars": [arXiv:astro-ph/0512585](https://arxiv.org/abs/astro-ph/0512585).
- Lorimer 2008, "Binary and millisecond pulsars", Living Reviews:
  [arXiv:0811.0762](https://arxiv.org/abs/0811.0762).
- Wolszczan & Frail 1992, Nature 355, 145:
  [doi:10.1038/355145a0](https://doi.org/10.1038/355145a0).
- Konacki & Wolszczan 2003, "Masses and orbital inclinations of planets
  in the PSR B1257+12 system":
  [arXiv:astro-ph/0305536](https://arxiv.org/abs/astro-ph/0305536).
- Sigurdsson et al. 2003, "A young white dwarf companion to pulsar
  1620-26": [arXiv:astro-ph/0307339](https://arxiv.org/abs/astro-ph/0307339).
- Bailes et al. 2011, PSR J1719-1438: [arXiv:1108.5201](https://arxiv.org/abs/1108.5201).
- Spiewak et al. 2018, PSR J2322-2650: [arXiv:1712.04445](https://arxiv.org/abs/1712.04445).
- Kerr et al. 2015, "Limits on planet formation around young pulsars":
  [arXiv:1507.06982](https://arxiv.org/abs/1507.06982).
- Starovoit & Rafikov 2017, "Why are pulsar planets rare?":
  [arXiv:1609.06409](https://arxiv.org/abs/1609.06409).
- Margalit & Metzger 2017, "Merger of a white dwarf-neutron star
  binary ... origin of the pulsar planets":
  [arXiv:1608.08636](https://arxiv.org/abs/1608.08636).
- Patruno & Kama 2017, "Neutron star planets: atmospheric processes and
  habitability", A&A 608, A147: [arXiv:1705.07688](https://arxiv.org/abs/1705.07688).
- Kaspi & Beloborodov 2017, "Magnetars": [arXiv:1703.00068](https://arxiv.org/abs/1703.00068).
- Olejak et al. 2020, "Synthetic catalog of black holes in the Milky
  Way": [arXiv:1908.08775](https://arxiv.org/abs/1908.08775).
- El-Badry et al. 2023, "A Sun-like star orbiting a black hole":
  [arXiv:2209.06833](https://arxiv.org/abs/2209.06833).
- Sbarufatti et al. 2025, "Swift observations of Gaia BH3":
  [arXiv:2503.01679](https://arxiv.org/abs/2503.01679).
- Menou, Perna & Hernquist 2001, "Stability and evolution of supernova
  fallback disks": [arXiv:astro-ph/0102478](https://arxiv.org/abs/astro-ph/0102478).
- James et al. 2015, "Gravitational lensing by spinning black holes in
  astrophysics, and in the movie Interstellar":
  [arXiv:1502.03808](https://arxiv.org/abs/1502.03808).
- Vanderburg et al. 2020, WD 1856+534 b: [arXiv:2009.07282](https://arxiv.org/abs/2009.07282);
  Limbach et al. 2025: [arXiv:2504.16982](https://arxiv.org/abs/2504.16982);
  Blackman et al. 2021, Nature 598, 272: [arXiv:2110.07934](https://arxiv.org/abs/2110.07934).
- Koester, Gänsicke & Farihi 2014, "The frequency of planetary debris
  around young white dwarfs": [arXiv:1404.2617](https://arxiv.org/abs/1404.2617).
- Agol 2011, "Transit surveys for Earths in the habitable zones of
  white dwarfs": [arXiv:1103.2791](https://arxiv.org/abs/1103.2791).
- Veras 2016, "Post-main-sequence planetary system evolution", Royal
  Society Open Science: [arXiv:1601.05419](https://arxiv.org/abs/1601.05419).
- Johnson et al. 2010, "Giant planet occurrence in the stellar
  mass-metallicity plane": [arXiv:1005.3084](https://arxiv.org/abs/1005.3084).
- Ramirez & Kaltenegger 2016, "Habitable zones of post-main sequence
  stars", ApJ 823, 6: [arXiv:1605.04924](https://arxiv.org/abs/1605.04924).
- Choi et al. 2016, MIST I: [arXiv:1604.08592](https://arxiv.org/abs/1604.08592).
- Janson et al. 2021, b Cen (AB)b, Nature 600, 231:
  [doi:10.1038/s41586-021-04124-8](https://doi.org/10.1038/s41586-021-04124-8).
- Burrows et al. 2001, "The theory of brown dwarfs and extrasolar giant
  planets": [arXiv:astro-ph/0103383](https://arxiv.org/abs/astro-ph/0103383).
- Han et al. 2013, OGLE-2012-BLG-0358Lb: [arXiv:1307.6335](https://arxiv.org/abs/1307.6335).
- Pascucci et al. 2016, "A steeper than linear disk mass-stellar mass
  scaling relation": [arXiv:1608.03621](https://arxiv.org/abs/1608.03621).
- Bolmont et al. 2011, "Tidal evolution of planets around brown
  dwarfs": [arXiv:1109.2906](https://arxiv.org/abs/1109.2906).
- Crowther 2007, "Physical properties of Wolf-Rayet stars", ARA&A:
  [arXiv:astro-ph/0610356](https://arxiv.org/abs/astro-ph/0610356).
- Rosslowe & Crowther 2015, "Spatial distribution of Galactic Wolf-Rayet
  stars": [arXiv:1412.0699](https://arxiv.org/abs/1412.0699).
- Begelman, Volonteri & Rees 2006: [arXiv:astro-ph/0602363](https://arxiv.org/abs/astro-ph/0602363);
  Begelman, Rossi & Armitage 2008, "Quasi-stars: accreting black holes
  inside massive envelopes": [arXiv:0711.4078](https://arxiv.org/abs/0711.4078).
- Begelman & Dexter 2025, [arXiv:2507.09085](https://arxiv.org/abs/2507.09085);
  Cantiello et al. 2025, [arXiv:2512.17997](https://arxiv.org/abs/2512.17997);
  Gentile et al. 2026, [arXiv:2606.06575](https://arxiv.org/abs/2606.06575).
- Hobbs et al. 2005, "A statistical study of 233 pulsar proper motions":
  [arXiv:astro-ph/0504584](https://arxiv.org/abs/astro-ph/0504584).
- The repository: `docs/analysis/10-stars.md`,
  `docs/analysis/16-planet-types.md`, the living-star specs in
  `docs/superpowers/specs/`, issues #343 to #345, #376 and #423.
