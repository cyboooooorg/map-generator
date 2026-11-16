# What are planets made of, and how big does that make them?

Seen from: the model behind the planet cards, in the generator's code
(`crates/mapgen/src/planet/`). Part 1 of the physical planet model
computes it for every planet of a stored system but shows it nowhere yet,
and does not change any generated world.

## Short answer

A planet is a stack of layers: an iron core, a rock mantle, a shell of
water and ices, and sometimes a hydrogen and helium envelope. How much of
each depends on where the planet formed (inside or beyond the ice line),
around what star (its iron content) and how massive it grew (a heavy core
captures gas). Given the layers, the radius follows from how much each
material is squeezed by the planet's own weight, plus the thickness of the
envelope. The model then adds the heat made inside (radioactive decay and
tides), the ice shell a frozen water layer keeps over an ocean, and reads
a class off the result: rocky, super-Earth, ice world,
mini-Neptune, ice giant, hot Neptune, gas giant or hot Jupiter. No class name goes into
the calculation; it only comes out.

## The science

### What a planet starts with

**The ice line.** Beyond a certain distance from the star, water freezes
in the protoplanetary disc and solid planets form with a large share of
ice. The model reads the system's ice line (see
[How do asteroid belts form?](asteroid-belts.md) and
[giant planet placement](giant-planet-placement.md)): beyond it, ices make
20 to 50 % of the non-gas mass, like Ganymede and Titan (about 40 to 50 %);
inside it, 0.01 to 0.2 % (Earth holds 0.023 %, Marty 2012). One planet in
ten inside the line is drawn as if it had formed beyond it and migrated in.

**Metallicity.** Stars rich in iron form planets rich in iron: Adibekyan et
al. (2021) find the iron fraction of rocky planets rising with their
star's. The model draws the iron core's share of the refractory (iron and
rock) mass around Earth's 0.325, shifted by 0.15 per dex of the star's
`[Fe/H]`, with a spread of 0.05. A few planets close to the star (5 % inside 0.3
of the ice line) are drawn Mercury-like, their mantle stripped by giant
impacts, with a core share of 0.55 to 0.75.

**Core accretion.** A solid core that grows in the gas disc captures
hydrogen and helium. Below about 0.5 Earth masses a body finishes forming
after the disc is gone and keeps none (Mars). Between 0.5 and 5 Earth
masses the chance to have gathered gas rises linearly; those that did hold
a few percent of their mass in it (Lee & Chiang 2016). From about 10 Earth
masses the core accretes gas faster than solids and runs away into a giant
(Pollack et al. 1996): Uranus and Neptune hold 10 to 20 % of their mass in
gas, Jupiter and Saturn 85 to 95 % (Guillot 2005, Helled et al. 2011).

### From layers to radius

A sphere of density $\rho$ and mass $M$ has radius $R = (3M / 4\pi\rho)^{1/3}$.
Rock under pressure is denser than rock at the surface, so a heavier planet
is smaller than its uncompressed size. For the iron and rock, Zeng,
Sasselov & Jacobsen (2016) fit models of Earth-like planets scaled from
PREM, with $M$ in Earth masses and $c$ the core's share of the refractory
mass:

```math
R / R_\oplus = (1.07 - 0.21\,c)\,M^{1/3.7}
```

**In words.** A planet is slightly smaller the more iron it holds, and
grows more slowly than the cube root of its mass.

**Orders of magnitude.** Earth ($c$ = 0.325, 1 $M_\oplus$) gives 1.00
$R_\oplus$; a 5-Earth-mass planet of the same make gives 1.5 times that.
The fit was made for 1 to 8 Earth masses.

The model takes the smaller of this radius and the uncompressed size (so
Mars, which does not compress its rock, is not shrunk below its real
size). The ratio of the two volumes, $k$, measures the compression. The
iron core shrinks by $k$, and the water shell, softer than rock, by
$k^{1.5}$, a calibration against the models of Zeng et al. (2019): a
5-Earth-mass planet with half its mass in water comes out at 1.95 $R_\oplus$,
between their 1.911 (300 K) and 1.981 (700 K), and within 6 % of them from
1 to 20 Earth masses.

**The envelope.** A hydrogen and helium envelope adds to the radius a
thickness that Lopez & Fortney (2014) fit:

```math
R_\mathrm{env} = 2.06\,R_\oplus\, M^{-0.21} \left(\frac{f}{5\,\%}\right)^{0.59} S^{0.044} \left(\frac{t}{5\,\mathrm{Gyr}}\right)^{-0.11}
```

with $M$ the mass in Earth masses, $f$ the envelope's share of the mass,
$S$ the light received (Earth's = 1) and $t$ the age. The age exponent is
that of their solar-metallicity models (the text of the paper gives −0.11
for them and −0.18 for the enhanced-opacity models the printed fit was
made on).

**In words.** A heavier planet squeezes its envelope; a thicker envelope
swells it; more light and a younger age swell it a little. The relation
holds on the grid it was fitted on (1 to 20 Earth masses, 0.1 to 1000
times Earth's light, 0.1 to 10 Gyr) and the model clamps the mass, the
light and the age to it. It extrapolates the envelope share beyond the
grid's 20 % up to the 30 % it draws under 30 Earth masses, so that more
gas makes a larger planet over every drawn share; a larger share, which
only a request asks for, is sized as 30 %.

**Orders of magnitude.** 2 % of a 5-Earth-mass core's mass in gas adds about
0.9 $R_\oplus$, which is why most planets above 1.6 $R_\oplus$ are not
rocky (Rogers 2015).

**Heavy elements.** Their fit is made on envelopes of pure hydrogen and
helium over rocky cores. A water-rich planet mixes heavy elements into
its envelope (Uranus and Neptune hold them up to their outer layers:
Helled et al. 2011; Nettelmann et al. 2013), which makes it denser and
thinner. The model multiplies the thickness of every envelope, from a
mini-Neptune's to an ice giant's, by a factor of the water share $w$
that runs linearly from 1 for a dry planet to $h_N \approx 0.49$ at
Neptune's 65 %:

```math
R = R_\mathrm{solid} + R_\mathrm{env} \left(1 - (1 - h_N)\,\frac{w}{65\,\%}\right)
```

$h_N$ is set so that Neptune's composition (65 % water and ices, 15 %
gas) at Neptune's mass, light and age gives Neptune's radius: its solid
layers come out 2.91 $R_\oplus$, the fit's envelope 1.98 (Neptune would
be 4.89 $R_\oplus$, 27 % too large, and Uranus 22 %), and 0.97 is left.

**In words.** At a fixed composition a Neptune barely grows with its
mass, because a heavier core squeezes its envelope; more gas or more
light makes it larger. More water at the same gas share grows the
solid layers but thins the envelope, and which wins depends on the gas:
at 1 $S_\oplus$, 60 % water instead of none makes a planet with 2 % gas
8 % smaller at 1 Earth mass but 12 % larger from 20; with 10 % gas, 25 %
smaller at 1 Earth mass and 2 % at 20; with 30 % gas, 34 % smaller at 1
Earth mass and still 7 % at 29.

**Orders of magnitude.** Uranus, with Neptune's composition and 15 % less
mass, comes out 3.80 $R_\oplus$ (3.98 observed, −4.5 %); Neptune 3.88
(3.86). A law of the mass alone, $M^{0.59}$ (Chen & Kipping 2017, a fit
to a population whose gas share grows with the mass), gave Uranus 3.52,
11.6 % too small. A dry 12-Earth-mass planet with 5 % gas at 100 times
Earth's light comes out 3.4 $R_\oplus$.

**Giants.** A giant (10 Earth masses or more with at least 5 % gas) takes
the generator's giant radius. Under 30 Earth masses (where the model's
gas giants start, with 90 % gas and more) it is the same law as for any
other planet, so the radius does not jump at 10 Earth masses or at 5 %
gas. From 30 Earth masses, whatever the composition, and from half its
mass in gas (a gas giant by its class, which under 30 Earth masses only
a request makes), it goes through Neptune, Saturn and Jupiter (Chen &
Kipping 2017) and is inflated when hot (Thorngren & Fortney 2018). The solid layers stay inside it, and the
envelope fills the rest. A giant whose composition is not known (a
world's giant neighbours, a giant world without a model) is sized as
Neptune's composition at the coldest light of the grid up to Neptune's
mass, then on the law through Neptune, Saturn and Jupiter, which carries
a gas share growing with the mass.

**Partly differentiated moons.** A planet is fully differentiated: iron
core, rock mantle, water on top. An icy moon may not be. Callisto's
moment of inertia, 0.355 (Anderson et al. 2001), and Titan's, 0.34
(Iess et al. 2010), are too high for a rock core under clean ice;
Ganymede's, 0.311 (Schubert et al. 2004), is that of a fully
differentiated body. Only a share $s$ of a moon's ice has separated from
its rock: the iron and rock layers keep the share $s$ of their mass, the
water layer the share $s$ of its own, and the rest, rock and ice in the
proportions of the whole, is a mixed rock-and-ice layer between them.
Each part keeps its volume, so the radius, the density and the gravity
do not change, and neither does the radiogenic heat, which comes from
the rock wherever it lies.

Two heats set $s$, after Barr & Canup. While it grew, a moon kept a
share $h$ of its accretion energy, $\tfrac{3}{5} G M / R$ per unit mass,
which melted that much ice (Barr & Canup 2008 find that Callisto and
Titan stayed partly differentiated only if they grew slowly enough to
radiate most of it):

```math
s = \min\left(1,\ \frac{h\,\tfrac{3}{5}\,G M / R}{L_\mathrm{ice}}\right)
```

with $L_\mathrm{ice}$ = 333.6 kJ/kg the latent heat of ice (Feistel &
Wagner 2006) and $h$ = 0.057, a calibration on Callisto and Titan.
Then the impacts of the late bombardment: a moon that received enough
impact energy per unit of its mass differentiates completely, since the
melt lets the rock sink and the rock's gravitational energy melts the
rest (Barr & Canup 2010, who explain the Ganymede–Callisto dichotomy
this way). That energy is counted against the Moon's: the impact flux at
the moon's orbit against the Moon's (the comets its giant focuses, as
for its craters, Zahnle et al. 2003), times its area over its mass
against the Moon's, times the share of the bombardment its system has
had at its age (the lunar chronology of Neukum et al. 2001, read from
the system's formation). From 2.26 times the Moon's, a calibration
between Callisto and Ganymede, $s$ = 1. A planet, and a body without
water, keeps $s$ = 1.

**In words.** A large moon kept more of the heat it grew with, so
more of its ice melted and let its rock sink; a moon deep in its giant's
well, or a small one, took enough impacts per kilogram to finish the
job.

**Orders of magnitude.** The asked moons of seeds 1 to 10: Callisto
takes 1.8 to 2.05 times the Moon's impact energy per kilogram, Ganymede
2.5 to 2.9, Europa 3.6 to 4.4, Enceladus over 40, Titan about 1.1.
Ganymede, Europa and Enceladus differentiate fully; Callisto keeps
$s \approx 0.3$ and comes out at a moment of inertia of 0.343 to 0.357
(seeds 1 to 20), Titan $s \approx 0.37$ and 0.346 to 0.353.

**A moon's core and water.** A moon's layers differ from a planet's in
three ways (`differentiation::moon_core`, for a body heated by the tides
of a host other than its star). Its iron core is the Fe–FeS eutectic, of
5.15 g/cm³, as the Galileo gravity models take Io's (Anderson et al.
2001: a core of about 20 % of the mass and 950 km under a 3.27 g/cm³
mantle) and Europa's (Anderson et al. 1998): segregated at low pressure,
the core keeps its sulfur, where a planet's takes the denser 7.0 its
radii are calibrated on. A small icy moon whose centre stays under
300 MPa (the threshold that keeps its core unconsolidated, see "The ice
shell and the ocean beneath") separates no iron at all: its rock never
compacts nor reaches the Fe–FeS eutectic, and the water that reaches it
hydrates it into a porous core of 2.4 g/cm³ (Iess et al. 2014; Choblet
et al. 2017). And a water layer whose base stays under the 350 MPa of
ice V holds no high-pressure ice: it takes the density of liquid water,
1.0 g/cm³, over its shell of ice Ih, 0.92 (Feistel & Wagner 2006), not
the 1.25 of a large moon's mix of ices. Each other layer keeps its mass
and volume, so the radius follows: Io comes out 1763 to 1811 km (1821.6
observed), Enceladus 247 to 261 km (252.1), with its archetype's ice
share set by mass balance to 32 to 37 % (its 1.61 g/cm³ over a 2.4 g/cm³
core under water of 0.95 to 1).

**The moment of inertia.** The model stores the factor $C / (M R^2)$ of
its layers, each of uniform density:

```math
\frac{C}{M R^2} = \frac{2}{5\,M R^2} \sum_i m_i \frac{r_i^5 - r_{i-1}^5}{r_i^3 - r_{i-1}^3}
```

0.4 for a uniform sphere, less the more the mass sits at the centre.
Over the asked moons of seeds 1 to 20: Io 0.370 to 0.376 (0.378
measured, Anderson et al. 2001), Europa 0.321 to 0.343 (0.346, Anderson
et al. 1998; a Europa drawn with 10 % water comes out lowest),
Enceladus 0.334 to 0.337 (0.335, Iess et al. 2014), Ganymede 0.306 to
0.315 (0.311). Within each layer the compression is left out, so a large
planet comes out high (Earth 0.354 against 0.3307 measured), and a giant
more so.

### The heat inside

**Radiogenic.** Uranium-238 and -235, thorium-232 and potassium-40 decay
and heat the mantle. Earth makes about 20 TW today (Arevalo et al. 2009).
The model splits that among the four isotopes by their present
abundances and heat per kilogram (McDonough & Sun 1995, Turcotte &
Schubert 2014) and runs each backwards or forwards by its half-life:

```math
H = \sum_i s_i\, H_\oplus\, \frac{m_\mathrm{sil}}{m_{\mathrm{sil},\oplus}}\, a\, e^{\lambda_i (4.567 - t)}
```

where $s_i$ is each isotope's share of Earth's heat, $H_\oplus$ = 20 TW,
$m_\mathrm{sil}$ the silicate mass, $a$ the planet's abundance relative to
Earth's, $\lambda_i = \ln 2 / t_{1/2,i}$ and $t$ the age in Gyr.

**In words.** The younger the planet, the more heat; the isotopes with
short half-lives dominate early.

**Orders of magnitude.** Earth formed with about five times its present
radiogenic heat.

**Secular cooling.** A planet also loses the heat it was born with: its
mantle and core cool. Earth loses about 47 TW through its surface
(Davies & Davies 2010), so some 26 TW beyond its radiogenic 20, a Urey
ratio (radiogenic over total) of 0.43, within the 0.3 to 0.5 of Korenaga
(2008). The model scales Earth's 26 TW by the silicate mass, which holds
the heat, and lets it decay with age over a cooling time that grows as
the square of the body's size, the time heat takes to leave it by
conduction (Turcotte & Schubert 2014, chapter 4):

```math
S = S_\oplus\, \frac{m_\mathrm{sil}}{m_{\mathrm{sil},\oplus}}\, e^{4.567/\tau_\oplus - t/\tau},
\qquad \tau = \tau_\oplus \left(\frac{m_\mathrm{sil}}{m_{\mathrm{sil},\oplus}}\right)^{2/3}
```

with $S_\oplus$ = 26 TW and $\tau_\oplus$ = 10 Gyr, a calibration on Earth
and Mars together: Earth's mantle cooled no faster in the past than its
100 K/Gyr today (150 to 250 K since 3 Ga, Herzberg, Condie & Korenaga
2010), so its loss decays slowly, and Mars, with an eighth of Earth's
silicate, keeps about 0.8 TW over its 2.4 TW of radiogenic heat, 0.022
W/m² in all, within the 0.019 to 0.025 of its thermal histories (Plesa et
al. 2018).

**In words.** Every body holds the same heat per kilogram when it forms;
a small one loses it sooner, so an old Mars or Moon keeps little and a
super-Earth keeps more than its share.

**Orders of magnitude.** Earth's model loses 46 TW, 0.090 W/m², and its
volcanism, which follows the square root of the whole flux, is the
scale's middle, 0.5.

**Tidal.** A host raises tides in a planet on an eccentric orbit, and
friction turns them into heat (Peale, Cassen & Reynolds 1979):

```math
\dot E = \frac{21}{2}\,\frac{k_2}{Q}\,\frac{G M_h^2 R^5 n\, e^2}{a^6}
```

with $M_h$ the host's mass, $R$ the planet's radius, $a$ and $e$ its orbit's
semi-major axis and eccentricity, $n$ its mean motion, and $k_2/Q$ the
tidal response. The host is the star for a planet; for a moon it is its
planet, on the moon's orbit around it (`Context::with_tide`), while the
star's light still arrives at the planet's orbit.

The tidal response $k_2/Q$ depends on what the planet is made of, read
from its composition, never from a class or a type name:

| Body | Rule | $k_2/Q$ | Source |
| --- | --- | --- | --- |
| Enveloped | at least 0.1 % of the mass in gas | $1.1 \times 10^{-5}$ | Jupiter's, measured (Lainey et al. 2009) |
| Icy | otherwise, at least 5 % water and ices | $2.5 \times 10^{-3}$ | calibration (Europa, Enceladus) |
| Rocky | the rest | 0.015 | Io's, measured (Lainey et al. 2009) |

The thresholds are the mini-Neptune and ice world lines of the classes
below. A gas envelope sets the radius, whose fifth power the heat follows,
but dissipates little: Jupiter's $k_2/Q$ is about 1400 times smaller than
Io's. Saturn's, measured at $1.6$ to $2.3 \times 10^{-4}$ (Lainey et al.
2012; Lainey 2016), is 15 to 20 times Jupiter's; the eccentricities of
close-in giants point to a weaker one, a $Q' = 3Q/2k_2$ of $10^{6.5}$, about
$5 \times 10^{-7}$ (Jackson, Greenberg & Barnes 2008). No icy moon has a
measured $k_2/Q$: the icy value is a $k_2$ of about 0.25 for an ice shell
over an ocean (Wahr et al. 2006) over a $Q$ of 100, a calibration.

**Orders of magnitude.** Io, with $k_2/Q$ = 0.015, gets about $10^{14}$ W
from Jupiter (Lainey et al. 2009). Europa, with the icy value, gets about
$1.1 \times 10^{12}$ W, 35 mW/m², within the 10 to 50 mW/m² of tidal
heating that thermal models of its ice shell consider (Kimura 2024), and
Enceladus about 3.7 GW, a quarter of the 15.8 GW it radiates (Howett et
al. 2011). A Jupiter at 0.04 AU from the Sun on an orbit of eccentricity
0.1 gets about 4 % of the starlight it absorbs; with Io's response, which
every body took before, it got 60 times that starlight. Through the
model, Io round Jupiter (`HostTide`, $e$ = 0.0041) gets $8.5 \times
10^{13}$ W, 2.1 W/m², and loses it through a heat pipe.

**Moons in resonance.** Neighbouring moons locked in a 2:1 resonance
form a chain, as Io, Europa and Ganymede, or Enceladus and Dione
(`planet::resonance`). A locked pair sits at the commensurability, off
it only by the precession of the pericentres the resonance forces and
by its libration (Io–Europa 2.007, Europa–Ganymede 2.015,
Enceladus–Dione 1.997): the chains are the runs of neighbours whose
period ratio lies within 1 % of 2. A pair merely near it is not locked:
Dione and Rhea, at 1.67, are not. The host's tides push each moon
outward with a torque

```math
T_i = \frac{3}{2}\,\frac{k_2}{Q}\bigg|_h\,\frac{G m_i^2 R_h^5}{a_i^6}
```

and the resonance locks each pair of neighbours together: they migrate
as one, at the mean motion $\bar n = \sum n_i L_i / \sum L_i$ weighted by
their orbits' angular momenta $L_i = m_i n_i a_i^2$. What the torques
pump into the two orbits and the orbits do not keep is dissipated by the
eccentricity the resonance forces (Meyer & Wisdom 2007):

```math
\dot E = \sum_{i \in \text{pair}} n_i T_i - \bar n \sum_{i \in \text{pair}} T_i \;\ge 0
```

Each pair's heat goes to its inner moon, as Meyer & Wisdom put all of
the Enceladus–Dione pair's heat in Enceladus: the inner moon, closer to
the host, carries the larger forced tide (Io over Europa). The pair, not
the whole chain, so that a moon's heat depends on its own orbit and its
outer neighbour's, not on the drawn moons further out or on any drawn
eccentricity. The outermost moon of a chain, and a moon whose pair pumps
nothing (the outer moon pushed out faster than the inner one), get none.

A moon's heat today is the larger of that equilibrium and its own tide
at its drawn eccentricity: the eccentricity is drawn, not evolved, and a
moon dissipating more than the equilibrium is caught on its way down,
as Io dissipates faster than its resonance resupplies (Lainey et al.
2009), while one dissipating less would have its eccentricity pumped up
until it reaches the equilibrium.

The host's $k_2/Q$ is not one value per planet but follows from
resonance locking (Fuller, Luan & Quataert 2016): the host dissipates so
that its innermost round moon migrates outward over the system's age,
$a/\dot a = t$, which gives

```math
\frac{k_2}{Q}\bigg|_h = \frac{n a^8}{3\,t\,G m R_h^5}
```

for that moon's $m$, $a$ and $n$, at most the $3 \times 10^{-3}$ measured
for Saturn at Titan's frequency (Lainey et al. 2020). One value per host,
taken at its innermost round moon's frequency as the measured ones are
(Jupiter's at Io's, Lainey et al. 2009; Saturn's from its inner moons,
Lainey et al. 2012), so that a moon of the chain does not set the
dissipation that heats it. The round moons of a drawn planet that grew
in its disk, on prograde orbits, form chains among themselves (the moons
too small to be round take no part, nor do captured or retrograde ones,
which keep the heat of their own tide and do not set the host's `k₂/Q`); the
world a request asks around a giant forms one with the giant's round
regular moons, spaced 1:2 around it. A moon asked by archetype brings
its partners at 2:1 (see the archetypes' `[moon.resonance]`), and an
asked moon near another's partner orbit is placed on it (Io, Europa and
Ganymede asked together make the Laplace chain).

**Which drawn moons are locked.** A giant draws its regular moons near
a 2:1 chain, each orbit 5 % either way of its step; that scatter is not
a resonance. The draw decides which neighbours the host's tides drove
into one (`resonance::capture`): each moon migrates out at
$\dot a / a = 2T/L$, which goes as $m\,a^{-13/2}$, so a pair converges
when the inner moon migrates faster than the outer one. A convergent
pair meeting a first-order resonance slowly, at the small eccentricities
of regular moons, is captured (Goldreich 1965; Peale 1976; Murray &
Dermott 1999, §8.12); a divergent one never is. Its period ratio falls
by $\tfrac{3}{2}\,\Delta(\dot a/a)$ times the age: a pair formed wide of
2 by less than that reaches it and locks; one formed inside 2 moves away
from it. The locked moons then migrate together, at $2\sum T / \sum L$,
and may catch the next one out, as Io drove Europa and the two Ganymede
into the Laplace resonance (Yoder 1979). A captured moon is moved onto
the commensurability (period exactly twice its inner neighbour's), one
left free just out of the 1 % band, so that the stored orbits say which
pairs are locked. Over 300 drawn systems, 227 of the 353 neighbouring
pairs of round disk-born moons are locked (64 %; 322, 91 %, when any
pair 1.8 to 2.2 apart counted): inner pairs converge fast and lock in
cascade, outer ones too slowly to close a gap of a few percent. The
heavy neighbour drawn just outside a Dione-mass partner is captured all
the same (the partner, locked with its Enceladus, still migrates fast
enough); a chain drawn near 2:1 at every step locks more often than the
Solar System's moons, most of whose neighbours are not near a 2:1.

**Orders of magnitude.** At 4.57 Gyr, locking gives Jupiter (with Io)
$8.5 \times 10^{-6}$, against the $1.1 \times 10^{-5}$ measured (Lainey et
al. 2009), and Saturn (with Enceladus) $2.2 \times 10^{-4}$, against the
$2.3 \times 10^{-4}$ measured (Lainey et al. 2012): Saturn's 20-fold
stronger dissipation comes out of its moon's closer orbit and smaller
mass, with no value of its own. Enceladus and Dione under Saturn's
measured $k_2/Q$ dissipate 14 GW, all of it in Enceladus, of the
15.8 GW it radiates (Howett et al. 2011); an asked Enceladus gets 11 to
16 GW over ten seeds. In the Laplace chain the Io–Europa pair gives Io
$1.8 \times 10^{13}$ W, under its own tide's $9 \times 10^{13}$, which
stands (an asked Io: 62 to 143 TW, a heat-pipe world); Europa keeps its
own tide (an asked Europa: 27 to 52 mW/m²).

### The ice shell and the ocean beneath

A body with a water layer whose surface is frozen (no liquid water on it,
a mean surface temperature under 273 K) carries its internal heat out
through an ice shell. The conductivity of ice rises as it gets colder,
$k = 651/T$ W m⁻¹ K⁻¹ (Petrenko & Whitworth 1999; Klinger 1980 gives
$567/T$), so Fourier's law integrates to a shell of plane thickness
$d_0$, which the sphere thins to $d$ (its base is smaller than its
surface; Turcotte & Schubert 2014):

```math
d_0 = \frac{651\,\ln(T_b / T_s)}{q}, \qquad d = \frac{R\,d_0}{R + d_0}
```

with $T_s$ the surface temperature, $q$ the internal heat flux (radiogenic,
tidal and secular), $R$ the radius and $T_b$ the melting point at the base
(Hussmann, Sohl & Spohn 2006). Ice melts colder under pressure: the base
follows the IAPWS melting curve of ice Ih (Wagner et al. 2011),

```math
\frac{p}{p_t} = 1 + \sum_{i=1}^{3} a_i \left(1 - \theta^{b_i}\right),
\qquad \theta = \frac{T}{273.16\ \mathrm{K}}
```

with $p_t$ = 611.657 Pa, $a_i$ = 1 195 393.37, 80 818.3159, 3338.2686 and
$b_i$ = 3, 25.75, 103.75, from 273.16 K at no pressure through about
263 K at 100 MPa to 251.165 K at the ice Ih–III–liquid triple point
(208.566 MPa); deeper, the curve of ice III,
$p/p_t = 1 - 0.299948\,(1 - \theta^{60})$ from that point, rises to
256.164 K at 350.1 MPa. The pressure at the base is the weight of the
shell's ice. A deeper base is colder, which thins the shell: Ganymede's
base, near 200 MPa, melts some 20 K below 273 K. Below the shell the
water stays liquid: an ocean, down to the floor of the water layer or
to the depth where the hydrostatic pressure reaches 350.1 MPa,
whichever comes first. That
pressure is the ice III, ice V and liquid triple point of the IAPWS
melting curves (Wagner et al. 2011): deeper, water near its melting point
freezes into ice V. The pressure integrates $\rho\,g(r)$ from the surface,
with ice Ih at 917 kg/m³ in the shell (Feistel & Wagner 2006) and water at
1000 kg/m³ below it (rounded; a calibration), and $g(r) = G M(<r) / r^2$
from the model's layers. When the shell would be thicker than the water
layer, the whole layer is ice; when its base already lies beyond
350.1 MPa, the water below it is high-pressure ice. Either way there is
no ocean.

The shell and its ocean sit on the water layer only: under a partly
differentiated moon's, the mixed rock-and-ice layer is the floor.

**Thinner at the poles.** The shell is in equilibrium with all the heat
that leaves through it, from below and from the tides alike. The
radiogenic and secular heat is spread evenly; the tidal heat follows the
pattern of where it is made, solved latitude by latitude for the flux
$q_\mathrm{even} + q_\mathrm{tidal}\,w$, the shell's thickness being
the mean over the surface and its polar thickness the shell at the pole.

- **In the shell.** The eccentricity tide of a thin shell over an ocean
  dissipates, in the membrane limit, as $\Psi_A + \tfrac{9}{2}\Psi_C$
  (Beuthe 2013), averaged over longitude:

  ```math
  w(\mu) = 1 + \frac{-\tfrac{7}{2}\,a_{20} P_2(\mu) + \tfrac{7}{4}\,a_{40} P_4(\mu)}{\tfrac{11}{2}\,a_{00}}
  ```

  with $\mu$ the cosine of the colatitude and $a_{00} = 21/5$,
  $a_{20} = -33/7$, $a_{40} = 387/140$ (Beuthe 2013, table 1): 1.92
  times the mean at the poles, 0.72 times at the equator, in 32 bands of
  equal area.
- **In an unconsolidated core.** A small moon's rock core stays a porous
  pile of fragments, which water flows through and its tides heat
  (Roberts 2015; Choblet et al. 2017): Beuthe (2019) finds that
  Enceladus's shell, with standard ice rheology, dissipates too little,
  and its core the rest. Choblet et al. find the heated water rising in
  plumes chiefly at the poles, thinning the ice above. A body whose
  centre is under 300 MPa keeps such a core (a calibration: an asked
  Enceladus's centre is at 92 to 108 MPa in the model, Europa's,
  Ganymede's, Titan's and Callisto's at 6 to 13 GPa). Its plumes carry
  14 % of the tidal heat into polar caps 16° in radius, about the size
  of Enceladus's tiger-stripe region (four fractures 130 km long and
  35 km apart, Porco et al. 2006), the rest reaching the shell evenly.
  Both numbers are calibrated on Enceladus: the share so that its polar
  shell is a quarter of its mean or less, as Čadek et al. (2016) find
  under 5 km at the south pole over a mean of 18 to 22 km.

A body heated by its rock alone keeps an even shell; a body without an
ocean too.

**Ammonia.** Ammonia dissolved in the ocean lowers the temperature at
which it freezes, along the liquidus of the water–ammonia phase diagram
at low pressure (Leliwa-Kopystyński et al. 2002), a fit in the ammonia
mass fraction $X$ of the liquid:

```math
T_\mathrm{liq} = 273 - 53.07\,X - 1651.4\,X^2 + 11842\,X^3 - 46269\,X^4 + 56695\,X^5
```

from 273 K for pure water through about 259 K at 10 % to about 175 K at
the eutectic, a third of ammonia (176 K measured), below which no liquid
is left. The ice carries 1 % of ammonia when the moon kept its
hypervolatiles, none when a warm disk stripped them. Ice excludes the
ammonia as it freezes, so all the water layer's ammonia is in the ocean
left (Grasset & Sotin 1996): a thicker shell leaves a smaller, richer
ocean, whose colder liquidus thins the shell. The ammonia lowers the
melting point under pressure by as much as at none,
$T_b = T_\mathrm{melt}(p) - (273 - T_\mathrm{liq}(X))$ (adding the two
lowerings is an approximation, no source). The shell and the ocean are
solved together, by bisection on the shell's thickness. A body whose
shell would fill its water layer without ammonia keeps a thin, cold
ocean with it.

**Convection.** A thick shell convects under a stagnant lid. The ice
creeps by diffusion, a Newtonian viscosity (Goldsby & Kohlstedt 2001):

```math
\eta = \frac{R\,T\,d^2}{84\,V_m\,D_0\,e^{-Q/RT}}
```

with $D_0$ = 9.1 × 10⁻⁴ m²/s, $Q$ = 59.4 kJ/mol, $V_m$ = 1.97 × 10⁻⁵
m³/mol and a grain size $d$ of 30 mm, the low end of the 30 to 80 mm
that dynamic recrystallisation brings a convecting shell's grains to
(Barr & McKinnon 2007): about 3 × 10¹⁷ Pa s at 273 K (the factor 84
takes the viscosity as $\sigma / 2\dot\varepsilon$, a calibration). The
shell convects when its Rayleigh number with the viscosity at its base
reaches the stagnant-lid onset (Solomatov 1995):

```math
Ra = \frac{\rho\,g\,\alpha\,\Delta T\,D^3}{\kappa\,\eta(T_b)} \ge 20.9\,\theta^4,
\qquad \theta = \frac{Q\,\Delta T}{R\,T_b^2}
```

with $\alpha$ = 1.6 × 10⁻⁴ K⁻¹ and $\kappa = k / \rho c_p$, $c_p$ =
2097 J kg⁻¹ K⁻¹ (Feistel & Wagner 2006). It then carries up into its lid
a flux that does not depend on its thickness (Solomatov & Moresi 2000):

```math
q_c = 0.53\,k\left(\frac{R\,T_b^2}{Q}\right)^{4/3}\left(\frac{\rho\,g\,\alpha}{\kappa\,\eta(T_b)}\right)^{1/3}
```

taken at the base's temperature, which overestimates it a little. If
$q_c$ is more than the internal heat supplies, the ocean freezes: down to
where its concentrated ammonia lowers the liquidus enough for $q_c$ to
match the supply, or entirely when there is no ammonia (or not enough),
the shell then reaching the ocean's floor. The lid conducts the heat down
to $T_b - 2\,R T_b^2 / Q$ ("estimates typically give" 2, Foley &
Bercovici 2014 after Solomatov & Moresi 2000). A convecting shell is
not thinned at the poles. With 30 mm grains and its base colder under
pressure, no asked moon's shell convects; with 1 mm grains (about
10¹⁴ Pa s, the value of many Europa models), a Ganymede's would convect
from some 30 km and freeze its ocean.

**In words.** The more heat comes up from inside, the thinner the ice and
the deeper the ocean; a colder surface thickens the ice, but only through
the logarithm. Where the tides make the heat, the poles get more of it
and thinner ice. Ammonia keeps a thin ocean under a shell that would
otherwise freeze through; a shell thick enough to convect sheds its heat
faster than it comes and freezes its ocean, unless ammonia stops it.

**Orders of magnitude** (asked moons, seeds 1 to 20). Europa (8 % water,
its tides from its resonances) gets a shell of 12 to 27 km, 6 to 15 km
at the poles, over an ocean of 80 to 158 km. Enceladus, 11 to 17 GW
from its resonance with Dione, gets 35.8 to 44.6 km of ice, 8 to 11 km at
the poles, over 15 to 29 km of ocean. Ganymede, heated mostly by its
rock, gets 130 to 232 km of ice and 35 to 125 km of ocean above ice V;
Titan, with its ammonia, 136 to 175 km over 55 to 107 km; Callisto,
whose water layer above its mixed rock and ice is thin, mostly freezes
through (204 to 254 km of ice; an ocean on a few seeds, up to 44 km). The Solar System's moon archetypes carry
the chondritic radiogenic abundance of the solar nebula, 0.9 to 1.1
times the Earth's (Lodders 2003), not the 0.15 dex drawn across stellar
systems.

**Geysers.** A shell over an ocean cracks through where it is thin
enough: Rudolph & Manga (2009) model cracks opened by 1 to 3 MPa of
tension against the weight of the ice that closes them, and find that
they cross Europa's shell only when it is under about 2.5 km, and
Enceladus's under about 25 km. The weight at the base of both limits,
$\rho\,g\,h$, is about the same, 3.0 and 2.6 MPa: a body vents when the
shell at a pole is under 2.6 MPa of ice (a calibration on the lower of
the two). The tension itself, from the ocean pressurised as it freezes
(Manga & Wang 2007) or from the tides, is not computed. The body must
also have an ocean, a shell that does not convect, and tides that make
at least half of its heat and thin the shell at the poles. The tides
thin both poles alike; Enceladus vents from its south pole alone, so the
body's seed picks one. Its fractures fill the cap round that pole where
the shell still passes, up to 30° from it, as Enceladus's four "tiger
stripes", about 130 km long and 35 km apart (Porco et al. 2006), and
the body's seed lays them out so that no two moons vent alike: a mean
spacing drawn once per body from 25 to 45 km (35 km on average), each
gap within 20 % of it and in that range, across a common strike, each
fracture turned by up to 15° off it, 60 to 100 % of its edge-to-edge
chord long and shifted along it by up to 15 % of the chord, and on a
third of the bodies a short branch, a third to a half of its parent's
length, splaying off at 20 to 40° near one end. A layout whose drawn
lines (with their meanders) come within 10 km of each other is drawn
again with less turn, so the fractures never cross; at most 12 are
stored, spaced wider to fill a larger cap. They
are stored on the model (`PlanetModel.geysers`: the pole, and each
fracture's centre, strike and length) so that the surface map and the
viewer draw them at the same place, and the innermost moon with geysers
close to its giant feeds its plume ring (see
[ring-extent.md](ring-extent.md)).

The surface map cuts each fracture into the ice (`stripes.rs`): a
trough about 500 m deep and 2 km wide between two ridges about 100 m
high, as the tiger stripes' profile (Porco et al. 2006), tapering to
nothing over the last tenth of its length at each end and darker than
the plains (an albedo shift of −0.25, a look constant: the stripes read
darker than the plains in Cassini's clear-filter images). Narrower than
1.5 pixels of the base map, the trough is drawn that wide and
shallower, keeping the area of its cross-section, so that the map
keeps it. The fractures resurface the cap round their pole, twice as
far from it as their farthest end and at most 40° (Enceladus's young
south polar terrain, bounded near 55° S, reaches about 35° from its
pole): no crater centred there is kept, on the base map, the detail
tiles and at every epoch.

Over seeds 1 to 20, every asked Enceladus vents (a polar shell of 7 to
10 km, at most 1 MPa of ice under its 0.11 m/s², over a cap of 16°:
three to six fractures, four or five on most seeds); Europa's polar shell, 6 to 16 km under 1.3 m/s², never
cracks through, nor do Ganymede's, Titan's or Callisto's. Across 300
drawn systems, 11 % of the icy moons and 3 % of the icy planets vent;
the moons among them are close-in, most of them in a 2:1 chain (some
vent on their own tide).

### Classes

The class is read off the result, in this order; the first rule that holds
wins.

| Class | Rule | Source of the threshold |
| --- | --- | --- |
| Giant: gas giant | at least 10 Earth masses and 5 % gas; at least 50 % of the mass in gas | calibration (parts Jupiter and Saturn from Uranus and Neptune) |
| Giant: hot Jupiter | a gas giant with a period of 10 days or less | Wright et al. 2012 |
| Giant: hot Neptune | an ice giant with a period of 10 days or less (the sparse "Neptune desert") | Wright et al. 2012; Mazeh, Holczer & Faigler 2016, doi:10.1051/0004-6361/201528065 |
| Giant: ice giant | a giant with less than 50 % gas | same calibration |
| Mini-Neptune | at least 0.1 % gas and 1.6 $R_\oplus$ or more | Rogers 2015; the gas share is a calibration |
| Ice world | at least 5 % water and ices | calibration (Europa about 8 %, Earth 0.02 %) |
| Super-Earth | 1.25 $R_\oplus$ or more | Borucki et al. 2011 |
| Rocky | the rest | |

## In the app

### What you see

Nothing yet. Part 1 computes the model for the planets of a stored system
(`planet::body_model`) and the system's age is now drawn, but no card shows
the class, the layers or the heat, and no world changes: the
`body_type` that the user asks for still decides what is generated. Later
parts make the model decide.

### How it computes them

`planet::draw_inputs` draws the iron share, the water share, the envelope,
the volatiles and the radiogenic abundance from the formation context (the
star's metallicity, the orbit over the ice line) and the mass.
`planet::interior` gives the layers and the radius, `planet::heat` the heat
and `planet::classify` the class; `planet::body_model` chains them for one
planet. Each planet's inputs come from their own random stream, seeded by
the system's seed and `<id>/model`, so they depend on no other body.

Water is drawn as a share of the mass that is not gas, then scaled by one
minus the envelope, so that a wet giant keeps its rock.

Sources: [mod.rs](../../crates/mapgen/src/planet/mod.rs),
[inputs.rs](../../crates/mapgen/src/planet/inputs.rs),
[interior.rs](../../crates/mapgen/src/planet/interior.rs),
[differentiation.rs](../../crates/mapgen/src/planet/differentiation.rs),
[heat.rs](../../crates/mapgen/src/planet/heat.rs),
[classify.rs](../../crates/mapgen/src/planet/classify.rs).

### Measuring it

```bash
cargo run --release -p mapgen --example planet_census -- 500
```

draws 500 whole systems (seeds 1 to 500, nothing written) and prints the
base preset each world renders with (`planet::interim::base_of`; the
`body_type` is the class itself), the share of planets per
radius bin of Fulton et al. (2017), and the median age, iron share, water
and envelope per class
([planet_census.rs](../../crates/mapgen/examples/planet_census.rs)).

### From the model to the map

A drawn planet's world is generated from its model, not from a type name
(`planet::interim`, a transitional layer that issue #424 replaces with
archetypes):

- `base_of` picks the built-in preset whose rules, palette and remap
  render the model, the first rule that holds: a giant class renders as
  `gas-giant`, `hot-jupiter` or `ice-giant` (a hot Neptune as a hot
  Jupiter, its methane gone; a mini-Neptune as an ice giant); a lava world or a heat-pipe regime as `volcanic`; a runaway
  greenhouse as `caustic`; a body whose envelope hides its surface as
  `frozen` for an ice world, `barren` otherwise; liquid hydrocarbons as
  `frozen`; liquid water
  as `terran`; no liquid, under 273 K and at least 5 % water, or at
  least 0.01 Earth oceans kept at the surface, as `frozen`; otherwise
  `barren`. The thresholds are the ones of the classes, so the look
  follows the class (Earth is `terran`, Venus `caustic`, Mars and
  Mercury `barren`, Titan `frozen`), except for a cold rock whose bulk is
  dry but whose surface keeps water: its sea level is calibrated on that
  water, which the barren look would draw as rock, so it renders
  `frozen`, a frozen sea in its basins (the 0.01 oceans: a little over
  twice Mars's 0.004, its polar deposits and ground ice of about 34 m of
  global layer, Carr & Head 2015, doi:10.1002/2014GL062464).
- A body that is no giant whose envelope hides its surface (the model
  derives none under it; about 0.3 % of drawn planets) keeps its
  class's preset at the model's radius and gravity, with no ocean, and
  its world aims at a bare surface's temperature, its equilibrium plus
  its internal heat (`calibrate_enveloped`). Part 3b of #423 derives the
  surface under the envelope.
- A giant, and a mini-Neptune rendered as one, takes the giant radius for
  its model's composition (iron, water and gas shares) and light: the
  interior's radius, at the model's mass, which its map and its moons
  take too, however light (issue #446). A giant world without a model
  takes the radius of a giant whose composition is not known.
- The gravity and the tectonic regime are the model's, and the radius is
  the interior's. The world is resolved at that gravity, so the star's
  tides, the moons it draws, the tilt they leave it and its tidal flats
  read the model's mass, not the one the rocky relation gives its size. The relief parameters follow: volcanism from the heat
  flux (Earth's 0.087 W/m2 gives the preset's value), halved when
  stagnant, full under a heat pipe; the age of the surface from the
  regime.
- The grid offset: the temperature shift of the world is found when the
  world is generated, so that the grid's mean matches the model's surface
  temperature (`generation::warming_for`). The sea level is found the
  same way, where the oceans the model keeps fill the relief
  (`sea_level_for_water`), none for a runaway whose water is steam, and
  none for a body whose water lies under its ice shell: a crust of ice
  (an ice world's share of water, no liquid water at its surface) with a
  shell over its water layer (Europa, Ganymede, Callisto, Enceladus) has
  that shell for its surface, land with its regime's relief and craters,
  not a frozen sea over a rocky floor. A frozen sea on a rocky planet (a
  snowball Earth, a cold Mars that kept its water) still fills its
  basins. A world with no water at its surface is dry: no open water anywhere, its
  sea level only the datum of its relief (`DRY_SEA_LEVEL`, see
  [mountains and gravity](mountains-and-gravity.md)). Both
  pass over a coarse grid of the relief (about a second in release), so a
  system's draw leaves them to the generation of the world: the
  parameters stored with the body keep the drawn values, and the
  calibrated ones are in the world's `world.json`.
- Calibrations, no source: the share of water that makes a body frozen
  (5 %) and the water at its surface that does (0.01 oceans), the
  hydrocarbon world's land share (Titan's 99 %), the erosion of methane
  (half of water's).

### Where it departs from the science

- **Calibrations.** Every constant marked "Calibration, no source" in the
  code is a choice, not a measurement: the shares of the iron core and of
  ice, the chance of keeping gas, the envelope share per mass, the iron
  and water densities, the $k^{1.5}$ compression of water, and the class
  thresholds that the table marks. Each names the bodies it reproduces.
- **Heavy elements in the envelope are a calibration.** The factor of
  the water share that thins an envelope is linear and set on Neptune
  alone, and applies to every planet with an envelope. Uranus comes out
  4.5 % small, since the model gives it Neptune's composition. With
  10 % gas or more, a wet planet comes out smaller than a dry one of the
  same mass and gas share (up to a third at 1 Earth mass), where water
  should make it larger.
- **The 30 Earth-mass line.** A giant's radius jumps there, from the
  law of its composition to the law of the gas giants. The drawn
  compositions jump there too (from at most 30 % gas to at least 90 %),
  but the law of the gas giants does not read the composition. Cold,
  the jump is 0.95 to 1.5 times (a dry planet with 5 % gas: 3.5 to
  5.2 $R_\oplus$). A planet hot enough to be inflated (1 500 to 2 000 K)
  also starts its inflation there, up to 1.9 times, so it jumps by
  about 1.5 to 2.5 times; the old law already jumped by about 1.9 times
  there. A requested body of half its mass in gas or more, a gas giant
  by its class, takes the gas giants' law under 30 Earth masses too
  (`M^0.59` below Neptune's mass), so its radius does not fall as its
  mass crosses the line.
- **Mini-Neptunes and the radius valley.** Photoevaporation now strips
  close-in envelopes (see [planet-atmospheres.md](planet-atmospheres.md)),
  and mini-Neptunes make 18.7 % of the planets (16.3 % of those within
  100 days; the census of 500 systems). The census still shows
  no dip between 1.5 and 2 $R_\oplus$ (Fulton et al. 2017), because the
  generator draws few planets that large; the doc reports it as measured.
- **Envelope grid.** The envelope fit stops at 20 % of the mass; the
  model extrapolates its `f^0.59` to the 30 % it draws, and sizes a
  requested envelope of 30 to 50 % as 30 %.
- **Round moons of drawn systems** go through `derive_moon_model` (see
  [planet-atmospheres.md](planet-atmospheres.md) and
  `docs/system-spec.md`, "Moons"); a moon too small to be round has no
  model. Their craters keep the lunar law and the Moon's rate (see
  [impact-craters.md](impact-craters.md)).
- **The ice shell** leaves out salts in the melting point (only
  ammonia and pressure lower it, their lowerings added). Its surface
  temperature is the body's mean, without the colder poles that would
  thicken the polar shell; its ice does not flow sideways to even out
  its thickness, and the shell's own dissipation is not concentrated
  where it is thinner (Beuthe 2019), two effects that work against each
  other. Convection uses one grain size and the viscosity at the base;
  with 30 mm grains and a base colder under pressure, it starts in no
  asked moon and no drawn body of the census, so every shell the model
  makes is conductive. Drawn icy moons and planets, whose radiogenic
  abundance keeps its 0.15 dex scatter, have shells and oceans that
  follow that draw.
- **Polar plumes heat both poles.** An unconsolidated core's plumes thin
  the shell at both poles alike, where Enceladus's south pole is thinner
  than its north. Their share and caps are calibrated on Enceladus
  alone, and the threshold of 300 MPa that keeps a core unconsolidated
  has no source. Enceladus's mean shell, 35.8 to 44.6 km over seeds 1 to
  20 (8 to 11 km at the poles), is about twice the 18 to 22 km observed
  (Čadek et al. 2016): the model's heat for it (11 to 17 GW) over its
  true radius gives a conductive mean shell thicker than the observed
  one.
- **Geysers crack by weight alone.** Whether a shell cracks through is
  read from the weight of the ice at its base against a threshold
  calibrated on the two limits of Rudolph & Manga (2009); the tension
  that opens the cracks, and the fractures' own spacing, are not
  computed: their gaps, turns, lengths and branches are drawn in ranges
  set around Enceladus's. A drawn moon or planet whose shell is under a
  kilometre everywhere (heated by its tides near melting) vents over the
  whole 30° cap. The model stores at most 12 fractures, spaced wider
  than 35 km when the cap would hold more: a storage and rendering limit
  (the map and the viewer draw each one), not physics.
- **Young terrain without craters.** The cap the fractures resurface
  keeps no crater at all, where Enceladus's south polar terrain keeps a
  few small ones (Porco et al. 2006); its size, twice the fractures'
  reach, is a calibration on that one body. The stripes' profile is the
  same on every body, whatever its gravity or shell.
- **Resonances are captured on today's migration rates.** Whether a
  drawn pair locks is judged from the rates its tides give today over
  the whole age, a rough measure of a history the model does not
  evolve, and only 2:1 resonances are looked for (not Mimas–Tethys's
  4:2 or Titan–Hyperion's 4:3). A drawn chain sits near 2:1 at every
  step, closer than the Solar System's spacing, so its neighbours lock
  more often than real moons'. Each pair's heat goes wholly to its
  inner moon, and a moon dissipates the larger of that and its own
  tide at its drawn eccentricity, rather than an eccentricity evolved
  to equilibrium.
  The host's $k_2/Q$ comes from its innermost round moon alone, with a
  migration time of exactly the system's age, capped at
  $3 \times 10^{-3}$, the strongest dissipation measured in a giant
  (Saturn's at Titan's frequency, Lainey et al. 2020): a light or
  distant innermost moon gives its host a strong dissipation, and a
  moon that is itself the innermost sets the dissipation that heats it
  (an asked Europa brings Io inside it for that reason). The heat
  grows with the moons' masses: round a ten-Jupiter giant, a lone
  world's chain dissipates up to about $10^{15}$ W. The giant's regular
  moons beside the world of a request take their pair's heat with the
  $k_2/Q$ of their composition (icy or rocky) and their drawn radius,
  not their model's.
- **Differentiation** follows the two arguments of Barr & Canup in
  their simplest form, not their models: the accretion heat kept is one
  share for every moon, and the impacts count their number, not their
  speed, which is higher deep in a giant's well (Barr & Canup find
  Ganymede receives about 3.5 times Callisto's impact energy; the model
  gives about 1.3). Titan comes out more differentiated than Callisto
  through its mass alone. Fully differentiated into uniform layers, a
  Ganymede with little ice comes out at a moment of inertia as low as
  0.295.
- **Tides** use one $k_2/Q$ per kind of body, not one per planet: a cold
  rocky planet takes Io's, which is partly molten (Mars's is about
  $1.7 \times 10^{-3}$, Lainey 2016), and every enveloped planet takes
  Jupiter's, from a mini-Neptune to a hot Jupiter. They neglect the
  planet's own mass.

## Sources

1. Adibekyan, V. et al. 2021, "A compositional link between rocky exoplanets
   and their host stars", *Science* 374, 330–332,
   [doi:10.1126/science.abg8794](https://doi.org/10.1126/science.abg8794).
2. Anderson, J. D., Jacobson, R. A., Lau, E. L., Moore, W. B. & Schubert,
   G. 2001, "Io's gravity field and interior structure", *Journal of
   Geophysical Research: Planets* 106, 32963–32969,
   [doi:10.1029/2000JE001367](https://doi.org/10.1029/2000JE001367).
3. Anderson, J. D., Jacobson, R. A., McElrath, T. P., Moore, W. B.,
   Schubert, G. & Thomas, P. C. 2001, "Shape, mean radius, gravity field,
   and interior structure of Callisto", *Icarus* 153, 157–161,
   [doi:10.1006/icar.2001.6664](https://doi.org/10.1006/icar.2001.6664).
4. Anderson, J. D., Schubert, G., Jacobson, R. A., Lau, E. L., Moore, W. B.
   & Sjogren, W. L. 1998, "Europa's differentiated internal structure:
   inferences from four Galileo encounters", *Science* 281, 2019–2022,
   [doi:10.1126/science.281.5385.2019](https://doi.org/10.1126/science.281.5385.2019).
5. Arevalo, R., McDonough, W. F. & Luong, M. 2009, "The K/U ratio of the
   silicate Earth", *Earth and Planetary Science Letters* 278, 361–369,
   [doi:10.1016/j.epsl.2008.12.023](https://doi.org/10.1016/j.epsl.2008.12.023).
6. Barr, A. C. & Canup, R. M. 2008, "Constraints on gas giant satellite
   formation from the interior states of partially differentiated
   satellites", *Icarus* 198, 163–177,
   [doi:10.1016/j.icarus.2008.07.004](https://doi.org/10.1016/j.icarus.2008.07.004).
7. Barr, A. C. & Canup, R. M. 2010, "Origin of the Ganymede–Callisto
   dichotomy by impacts during the late heavy bombardment", *Nature
   Geoscience* 3, 164–167,
   [doi:10.1038/ngeo746](https://doi.org/10.1038/ngeo746).
8. Barr, A. C. & McKinnon, W. B. 2007, "Convection in ice I shells and
   mantles with self-consistent grain size", *Journal of Geophysical
   Research: Planets* 112, E02012,
   [doi:10.1029/2006JE002781](https://doi.org/10.1029/2006JE002781).
9. Beuthe, M. 2013, "Spatial patterns of tidal heating", *Icarus* 223,
   308–329,
   [doi:10.1016/j.icarus.2012.11.020](https://doi.org/10.1016/j.icarus.2012.11.020).
10. Beuthe, M. 2019, "Enceladus's crust as a non-uniform thin shell: II
    tidal dissipation", *Icarus* 332, 66–91,
    [doi:10.1016/j.icarus.2019.05.035](https://doi.org/10.1016/j.icarus.2019.05.035).
11. Borucki, W. J. et al. 2011, *The Astrophysical Journal* 736, 19,
    [doi:10.1088/0004-637X/736/1/19](https://doi.org/10.1088/0004-637X/736/1/19).
12. Goldreich, P. 1965, "An explanation of the frequent occurrence of
    commensurable mean motions in the Solar System", *Monthly Notices of the
    Royal Astronomical Society* 130, 159–181,
    [doi:10.1093/mnras/130.3.159](https://doi.org/10.1093/mnras/130.3.159).
13. Murray, C. D. & Dermott, S. F. 1999, *Solar System Dynamics*, Cambridge
    University Press (ch. 8, resonant capture; not checked).
14. Peale, S. J. 1976, "Orbital resonances in the Solar System", *Annual
    Review of Astronomy and Astrophysics* 14, 215–246,
    [doi:10.1146/annurev.aa.14.090176.001243](https://doi.org/10.1146/annurev.aa.14.090176.001243).
15. Yoder, C. F. 1979, "How tidal heating in Io drives the Galilean orbital
    resonance locks", *Nature* 279, 767–770,
    [doi:10.1038/279767a0](https://doi.org/10.1038/279767a0).
16. Čadek, O. et al. 2016, "Enceladus's internal ocean and ice shell
    constrained from Cassini gravity, shape, and libration data",
    *Geophysical Research Letters* 43, 5653–5660,
    [doi:10.1002/2016GL068634](https://doi.org/10.1002/2016GL068634).
17. Chen, J. & Kipping, D. 2017, "Probabilistic forecasting of the masses and
    radii of other worlds", *The Astrophysical Journal* 834, 17,
    [doi:10.3847/1538-4357/834/1/17](https://doi.org/10.3847/1538-4357/834/1/17).
18. Choblet, G. et al. 2017, "Powering prolonged hydrothermal activity
    inside Enceladus", *Nature Astronomy* 1, 841–847,
    [doi:10.1038/s41550-017-0289-8](https://doi.org/10.1038/s41550-017-0289-8).
19. Davies, J. H. & Davies, D. R. 2010, "Earth's surface heat flux",
    *Solid Earth* 1, 5–24,
    [doi:10.5194/se-1-5-2010](https://doi.org/10.5194/se-1-5-2010).
20. Feistel, R. & Wagner, W. 2006, "A new equation of state for H₂O ice
    Ih", *Journal of Physical and Chemical Reference Data* 35, 1021–1047,
    [doi:10.1063/1.2183324](https://doi.org/10.1063/1.2183324).
21. Foley, B. J. & Bercovici, D. 2014, "Scaling laws for convection with
    temperature-dependent viscosity and grain-damage",
    [arXiv:1410.7652](https://arxiv.org/abs/1410.7652).
22. Fuller, J., Luan, J. & Quataert, E. 2016, "Resonance locking as the
    source of rapid tidal migration in the Jupiter and Saturn moon
    systems", *Monthly Notices of the Royal Astronomical Society* 458,
    3867–3879,
    [doi:10.1093/mnras/stw609](https://doi.org/10.1093/mnras/stw609).
23. Fulton, B. J. et al. 2017, "The California-Kepler Survey. III. A gap in
    the radius distribution of small planets", *The Astronomical Journal*
    154, 109,
    [doi:10.3847/1538-3881/aa80eb](https://doi.org/10.3847/1538-3881/aa80eb).
24. Goldsby, D. L. & Kohlstedt, D. L. 2001, "Superplastic deformation of
    ice: experimental observations", *Journal of Geophysical Research*
    106, 11017–11030,
    [doi:10.1029/2000JB900336](https://doi.org/10.1029/2000JB900336).
25. Grasset, O. & Sotin, C. 1996, "The cooling rate of a liquid shell in
    Titan's interior", *Icarus* 123, 101–112,
    [doi:10.1006/icar.1996.0144](https://doi.org/10.1006/icar.1996.0144).
26. Guillot, T. 2005, "The interiors of giant planets: models and
    outstanding questions", *Annual Review of Earth and Planetary
    Sciences* 33, 493–530,
    [doi:10.1146/annurev.earth.33.092203.122639](https://doi.org/10.1146/annurev.earth.33.092203.122639).
27. Helled, R. et al. 2011, *The Astrophysical Journal* 726, 15,
    [doi:10.1088/0004-637X/726/1/15](https://doi.org/10.1088/0004-637X/726/1/15).
28. Herzberg, C., Condie, K. & Korenaga, J. 2010, "Thermal history of the
    Earth and its petrological expression", *Earth and Planetary Science
    Letters* 292, 79–88,
    [doi:10.1016/j.epsl.2010.01.022](https://doi.org/10.1016/j.epsl.2010.01.022).
29. Howett, C. J. A., Spencer, J. R., Pearl, J. & Segura, M. 2011, "High
    heat flow from Enceladus' south polar region measured using 10–600
    cm⁻¹ Cassini/CIRS data", *Journal of Geophysical Research* 116, E03003,
    [doi:10.1029/2010JE003718](https://doi.org/10.1029/2010JE003718).
30. Hussmann, H., Sohl, F. & Spohn, T. 2006, "Subsurface oceans and deep
    interiors of medium-sized outer planet satellites and large
    trans-neptunian objects", *Icarus* 185, 258–273,
    [doi:10.1016/j.icarus.2006.06.005](https://doi.org/10.1016/j.icarus.2006.06.005).
31. Iess, L. et al. 2010, "Gravity field, shape, and moment of inertia of
    Titan", *Science* 327, 1367–1369,
    [doi:10.1126/science.1182583](https://doi.org/10.1126/science.1182583).
32. Iess, L. et al. 2014, "The gravity field and interior structure of
    Enceladus", *Science* 344, 78–80,
    [doi:10.1126/science.1250551](https://doi.org/10.1126/science.1250551).
33. Jackson, B., Greenberg, R. & Barnes, R. 2008, "Tidal evolution of
    close-in extrasolar planets", *The Astrophysical Journal* 678,
    1396–1406, [doi:10.1086/529187](https://doi.org/10.1086/529187).
34. Kimura, J. 2024, "Europa's structural conditions for the existence of
    subsurface ocean and the absence of metallic core-driven magnetic
    field", arXiv:2403.01336,
    [arxiv.org/abs/2403.01336](https://arxiv.org/abs/2403.01336).
35. Korenaga, J. 2008, "Urey ratio and the structure and evolution of
    Earth's mantle", *Reviews of Geophysics* 46, RG2007,
    [doi:10.1029/2007RG000241](https://doi.org/10.1029/2007RG000241).
36. Lainey, V. 2016, "Quantification of tidal parameters from Solar System
    data", *Celestial Mechanics and Dynamical Astronomy* 126, 145–156
    (table 1),
    [doi:10.1007/s10569-016-9695-y](https://doi.org/10.1007/s10569-016-9695-y).
37. Lainey, V. et al. 2012, "Strong tidal dissipation in Saturn and
    constraints on Enceladus' thermal state from astrometry", *The
    Astrophysical Journal* 752, 14,
    [doi:10.1088/0004-637X/752/1/14](https://doi.org/10.1088/0004-637X/752/1/14).
38. Lainey, V. et al. 2020, "Resonance locking in giant planets indicated
    by the rapid orbital expansion of Titan", *Nature Astronomy* 4,
    1053–1058,
    [doi:10.1038/s41550-020-1120-5](https://doi.org/10.1038/s41550-020-1120-5).
39. Lainey, V., Arlot, J.-E., Karatekin, Ö. & Van Hoolst, T. 2009, "Strong
    tidal dissipation in Io and Jupiter from astrometric observations",
    *Nature* 459, 957–959,
    [doi:10.1038/nature08108](https://doi.org/10.1038/nature08108).
40. Lee, E. J. & Chiang, E. 2016, *The Astrophysical Journal* 817, 90,
    [doi:10.3847/0004-637X/817/2/90](https://doi.org/10.3847/0004-637X/817/2/90).
41. Leliwa-Kopystyński, J., Maruyama, M. & Nakajima, T. 2002, "The
    water–ammonia phase diagram up to 300 MPa: application to icy
    satellites", *Icarus* 159, 518–528,
    [doi:10.1006/icar.2002.6932](https://doi.org/10.1006/icar.2002.6932).
42. Lodders, K. 2003, "Solar System abundances and condensation
    temperatures of the elements", *The Astrophysical Journal* 591,
    1220–1247,
    [doi:10.1086/375492](https://doi.org/10.1086/375492).
43. Lopez, E. D. & Fortney, J. J. 2014, "Understanding the mass-radius
    relation for sub-Neptunes: radius as a proxy for composition", *The
    Astrophysical Journal* 792, 1,
    [doi:10.1088/0004-637X/792/1/1](https://doi.org/10.1088/0004-637X/792/1/1).
44. Manga, M. & Wang, C.-Y. 2007, "Pressurized oceans and the eruption of
    liquid water on Europa and Enceladus", *Geophysical Research Letters*
    34, L07202,
    [doi:10.1029/2007GL029297](https://doi.org/10.1029/2007GL029297).
45. Marty, B. 2012, "The origins and concentrations of water, carbon,
    nitrogen and noble gases on Earth", *Earth and Planetary Science
    Letters* 313–314, 56–66,
    [doi:10.1016/j.epsl.2011.10.040](https://doi.org/10.1016/j.epsl.2011.10.040).
46. McDonough, W. F. & Sun, S.-s. 1995, "The composition of the Earth",
    *Chemical Geology* 120, 223–253,
    [doi:10.1016/0009-2541(94)00140-4](https://doi.org/10.1016/0009-2541(94)00140-4).
47. Meyer, J. & Wisdom, J. 2007, "Tidal heating in Enceladus", *Icarus*
    188, 535–539,
    [doi:10.1016/j.icarus.2007.03.001](https://doi.org/10.1016/j.icarus.2007.03.001).
48. Nettelmann, N., Helled, R., Fortney, J. J. & Redmer, R. 2013, "New
    indication for a dichotomy in the interior structure of Uranus and
    Neptune from the application of modified shape and rotation data",
    *Planetary and Space Science* 77, 143–151,
    [doi:10.1016/j.pss.2012.06.019](https://doi.org/10.1016/j.pss.2012.06.019).
49. Neukum, G., Ivanov, B. A. & Hartmann, W. K. 2001, "Cratering records in
    the inner solar system in relation to the lunar reference system",
    *Space Science Reviews* 96, 55–86,
    [doi:10.1023/A:1011989004263](https://doi.org/10.1023/A:1011989004263).
50. Peale, S. J., Cassen, P. & Reynolds, R. T. 1979, "Melting of Io by tidal
    dissipation", *Science* 203, 892–894,
    [doi:10.1126/science.203.4383.892](https://doi.org/10.1126/science.203.4383.892).
51. Petrenko, V. F. & Whitworth, R. W. 1999, *Physics of Ice*, Oxford
    University Press, ISBN 978-0-19-851894-6 (thermal conductivity of ice
    Ih, $k = 651/T$ W m⁻¹ K⁻¹; Klinger 1980, *Science* 209, 271–272,
    gives $567/T$).
52. Plesa, A.-C. et al. 2018, "The thermal state and interior structure of
    Mars", *Geophysical Research Letters* 45, 12198–12209,
    [doi:10.1029/2018GL080728](https://doi.org/10.1029/2018GL080728).
53. Pollack, J. B. et al. 1996, "Formation of the giant planets by
    concurrent accretion of solids and gas", *Icarus* 124, 62–85,
    [doi:10.1006/icar.1996.0190](https://doi.org/10.1006/icar.1996.0190).
54. Porco, C. C. et al. 2006, "Cassini observes the active south pole of
    Enceladus", *Science* 311, 1393–1401,
    [doi:10.1126/science.1123013](https://doi.org/10.1126/science.1123013).
55. Roberts, J. H. 2015, "The fluffy core of Enceladus", *Icarus* 258,
    54–66,
    [doi:10.1016/j.icarus.2015.05.033](https://doi.org/10.1016/j.icarus.2015.05.033).
56. Rudolph, M. L. & Manga, M. 2009, "Fracture penetration in planetary
    ice shells", *Icarus* 199, 536–541,
    [doi:10.1016/j.icarus.2008.10.010](https://doi.org/10.1016/j.icarus.2008.10.010).
57. Rogers, L. A. 2015, "Most 1.6 Earth-radius planets are not rocky", *The
    Astrophysical Journal* 801, 41,
    [doi:10.1088/0004-637X/801/1/41](https://doi.org/10.1088/0004-637X/801/1/41).
58. Schubert, G., Anderson, J. D., Spohn, T. & McKinnon, W. B. 2004,
    "Interior composition, structure and dynamics of the Galilean
    satellites", in *Jupiter: The Planet, Satellites and Magnetosphere*,
    Cambridge University Press, 281–306.
59. Solomatov, V. S. & Moresi, L.-N. 2000, "Scaling of time-dependent
    stagnant lid convection: application to small-scale convection on
    Earth and other terrestrial planets", *Journal of Geophysical
    Research* 105, 21795–21817,
    [doi:10.1029/2000JB900197](https://doi.org/10.1029/2000JB900197).
60. Solomatov, V. S. 1995, "Scaling of temperature- and stress-dependent
    viscosity convection", *Physics of Fluids* 7, 266–274,
    [doi:10.1063/1.868624](https://doi.org/10.1063/1.868624).
61. Thorngren, D. & Fortney, J. J. 2018, "Bayesian analysis of hot-Jupiter
    radius anomalies", *The Astronomical Journal* 155, 214,
    [doi:10.3847/1538-3881/aaba13](https://doi.org/10.3847/1538-3881/aaba13).
62. Turcotte, D. L. & Schubert, G. 2014, *Geodynamics*, 3rd ed., Cambridge
    University Press (table 4.2).
63. Wagner, W., Riethmann, T., Feistel, R. & Harvey, A. H. 2011, "New
    equations for the sublimation pressure and melting pressure of H₂O ice
    Ih", *Journal of Physical and Chemical Reference Data* 40, 043103,
    [doi:10.1063/1.3657937](https://doi.org/10.1063/1.3657937).
64. Wahr, J. M., Zuber, M. T., Smith, D. E. & Lunine, J. I. 2006, "Tides on
    Europa, and the thickness of Europa's icy shell", *Journal of
    Geophysical Research* 111, E12005,
    [doi:10.1029/2006JE002729](https://doi.org/10.1029/2006JE002729).
65. Wright, J. T. et al. 2012, *The Astrophysical Journal* 753, 160,
    [doi:10.1088/0004-637X/753/2/160](https://doi.org/10.1088/0004-637X/753/2/160).
66. Zahnle, K., Schenk, P., Levison, H. & Dones, L. 2003, "Cratering rates
    in the outer Solar System", *Icarus* 163, 263–289,
    [doi:10.1016/S0019-1035(03)00048-4](https://doi.org/10.1016/S0019-1035(03)00048-4).
67. Zeng, L. et al. 2019, "Growth model interpretation of planet size
    distribution", *Proceedings of the National Academy of Sciences* 116,
    9723–9728,
    [doi:10.1073/pnas.1812905116](https://doi.org/10.1073/pnas.1812905116).
    
    The three DOIs above were checked against the publishers' pages; the
    DOIs of the tidal sources (Howett, Jackson, Kimura, Lainey, Wahr) and of
    the ice shell's (Hussmann, and the IAPWS ice papers Feistel &
    Wagner 2006 and Wagner et al. 2011) against their Crossref records; the
    $651/T$ conductivity is quoted as the ice-shell literature cites Petrenko
    & Whitworth, not checked in the book itself. Of the tidal figures, Jupiter's and Io's $k_2/Q$,
    Saturn's $1.59 \times 10^{-4}$ and Mars's $k_2$ and $Q$ match table 1 of
    Lainey 2016; Saturn's $2.3 \times 10^{-4}$, the planetary $Q$ of
    $10^{6.5}$ of Jackson et al. and Kimura's 10 to 50 mW/m² match their
    abstracts; Howett's 15.8 GW and Wahr's $k_2$ of 0.25 were not checked in
    the papers themselves. The resonance sources (Fuller, Luan & Quataert
    2016; Meyer & Wisdom 2007; Lainey et al. 2020) are cited for their method, from their
    abstracts; their DOIs and page ranges were not checked. Rudolph &
    Manga's limits (2.5 km for Europa, 25 km for Enceladus, under 1 to
    3 MPa of tension) match their abstract; Manga & Wang 2007 is cited for
    its mechanism, its DOI and pages not checked. Of the
    figures quoted from papers, Marty's 1000 to 3000 ppm of water and up to
    500 ppm of carbon match the abstract; his 2 ppm of nitrogen (the abstract
    only says nitrogen is depleted by about an order of magnitude) and the
    "5 of 22 rocky planets" of Adibekyan et al. 2021 (22 comes from press
    reports of the paper) were not checked in the papers themselves. The
    DOIs of Goldreich 1965, Peale 1976, Yoder 1979, Anderson et al. 1998
    and 2001 (Io) and Iess et al. 2014 were checked against their
    Crossref records; their figures (Io's 20 % core of 5.15 g/cm³,
    Enceladus's 2.4 g/cm³ core) are quoted as the literature cites them.
68. Zeng, L., Sasselov, D. D. & Jacobsen, S. B. 2016, "Mass-radius
    relation for rocky planets based on PREM", *The Astrophysical Journal*
    819, 127,
    [doi:10.3847/0004-637X/819/2/127](https://doi.org/10.3847/0004-637X/819/2/127).

## See also

- [What is the habitable zone?](habitable-zone.md): the zone whose planets
  this model sizes.
- [Why does a world always show the same face to its
  host?](tidal-locking.md): the system's age, now drawn, sets how far
  locking has gone.
- [docs/superpowers/specs/2026-11-11-physical-planet-model-design.md](../superpowers/specs/2026-11-11-physical-planet-model-design.md):
  the design of the model.

For the atmosphere, climate and XUV history of a planet see
[planet-atmospheres.md](planet-atmospheres.md).
