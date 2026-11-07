# Why are ring moons so small?

Seen from: the planet view of a giant with bright rings, and the card of
one of its ring moons ("In the rings").

## Short answer

Close to a giant planet, its tides pull loose ring particles apart faster
than their own gravity can gather them: that is why rings stay rings. A moon
can only grow there around a dense solid core. It stops growing once it fills
the small region its gravity controls, its Roche lobe. By then it is two to
three times the size of its core: a few to a few tens of kilometres. Farther
out, past the Roche limit, moons form freely from the material the rings
spread out, and they get larger the farther they are from the rings.

## The science

![Saturn's rings, its Roche limits and the small moons at the rings' edge, distances to scale](images/ring-moons-saturn.svg)

### The Roche limit: where rings end and moons begin

A body held together only by its own gravity is torn apart by the planet's
tides inside a distance called the Roche limit. For a planet of radius $R$
and density $\rho_p$, and a body of density $\rho_m$ (Murray & Dermott 1999):

```math
d_\text{fluid} \approx 2.44\,R\left(\frac{\rho_p}{\rho_m}\right)^{1/3}
\qquad
d_\text{rigid} \approx 1.26\,R\left(\frac{\rho_p}{\rho_m}\right)^{1/3}
```

**In words.** The *fluid* limit is for a loose pile with no strength, like a
clump of ring particles. The *rigid* limit is for a solid body that its
material strength also holds together. Between the two lies the *Roche
zone*: a solid body survives there, but loose material cannot gather into a
new one.

**Orders of magnitude.** For Saturn (mean density 0.69 g/cm³) and ring ice
(0.5 to 0.9 g/cm³, porous), the fluid limit lies 130,000 to 158,000 km from
Saturn's centre: 2.16 to 2.62 equatorial radii. (The density is the mean one,
so $R$ is the mean radius, 58,232 km; the equatorial radius is 60,268 km.)
Saturn's main rings end at 2.27 equatorial radii, inside that band. A solid
ice body would hold down to 1.11 equatorial radii, inside the rings.

### The Roche lobe: how much a moon can hold on to

A moon of mass $m$ at a distance $a$ from a planet of mass $M$ holds on to
particles only within its Roche lobe. Its long semi-axis, pointing at the
planet, is the Hill radius (Porco et al. 2007):

```math
r_H = a\left(\frac{m}{3M}\right)^{1/3}
```

**In words.** The closer to the planet, the smaller the region a moon of a
given mass controls. A particle that settles on the moon but sticks out of
that region is pulled off by the planet.

**Orders of magnitude.** Pan, 5 × 10¹⁵ kg at 133,584 km from Saturn, has a
Hill radius of 19 km. Its long semi-axis is 17.4 km: it fills 91 % of its
Roche lobe.

A moon fills its lobe exactly when its mean density equals the *critical
density* at its distance (Porco et al. 2007, eq. 1 and 2):

```math
\rho_\text{crit} = \frac{3M}{\gamma\,a^3},
\qquad
\gamma = \frac{2\pi\ln\!\left(2+\sqrt{3}\right)}{3\sqrt{3}} \approx 1.59
```

$\gamma a^3$ is the volume of the lobe, a shape with axes in the ratio
3 : 2 : 2.

**In words.** The critical density depends only on the distance, not on the
moon's size: a bigger moon has a bigger lobe in proportion. Below it, the
moon's own gravity cannot keep what it gathers.

**Orders of magnitude.** Round Saturn it is 0.45 g/cm³ at Pan's distance and
0.38 at Pandora's. The small moons there weigh 0.34 to 0.50 g/cm³: half the
density of solid ice, and very close to the critical density at their
distance (Porco et al. 2007, Table 1).

### Why they stop at a few tens of kilometres

Porco et al. (2007) simulated ring particles, with the porosity and the
spread of sizes of Saturn's A ring, settling on a solid core. The core
gathered particles until its mean density fell to the critical density.
After that, every new particle knocked off as many as stuck. The moonlet
had grown to fill its Roche lobe and could grow no further.

```mermaid
flowchart LR
  A["A dense icy shard<br/>in the rings"]
  A --> B["It sweeps up<br/>ring particles"]
  B --> C{"Mean density above<br/>the critical density?"}
  C -- yes --> B
  D["It fills its Roche lobe<br/>and stops: 2 to 3 times<br/>the size of the core"]
  C -- no --> D
```

The final long semi-axis $a$ of a moon grown from a core of radius $R_c$ and
density $\rho_c$, with a mantle of accreted particles of density $\rho_m$,
follows from its mass (Porco et al. 2007, eq. 3):

```math
\frac{a}{R_c} = \left(\frac{4\pi}{3\gamma}\,
\frac{\rho_c - \rho_m}{\rho_\text{crit} - \rho_m}\right)^{1/3}
\qquad (\rho_m < \rho_\text{crit})
```

**In words.** The final size is set by the core, not by how much ring
material lies around. A loose mantle is lighter than the critical density,
so it can only be held because the dense core raises the mean. The more the
core outweighs the mantle, the more mantle the moon can carry. Without a
core, the moon cannot even start: loose particles near the critical density
do not stick together under the tides.

**Orders of magnitude.** With a solid ice core (0.9 g/cm³), a mantle packed
like the simulated one (particles of 0.4 g/cm³ with 65 % voids: 0.14 g/cm³)
and the critical density at Pan's distance (0.45 g/cm³), the moon ends at
1.9 times the size of its core. Porco et al. (2007) find, for the moons
from Pan to Pandora, cores of one third to one half of their present size.
Their sizes run from 4 km (Daphnis) to 43 km (Prometheus) in mean radius.

A moonlet embedded in the rings also cuts off its own supply. It clears a
gap round its orbit (Pan keeps the 325 km Encke gap, Daphnis the Keeler
gap, about 40 km wide), and it stops gathering once the gap is open. Porco et al.
(2007) conclude that both grew to their present size early on, before their
gaps were cleared.

### Beyond the Roche limit: moons grow with distance

Outside the fluid Roche limit, clumps hold together without a core. Rings
spread slowly, as their particles collide. The material that crosses the
Roche limit gathers into new moonlets, and the rings' gravity pushes them
outwards.

```mermaid
flowchart LR
  R["The rings spread"] --> L["Material crosses<br/>the Roche limit"]
  L --> M["It gathers into<br/>a moonlet"]
  M --> O["The rings push<br/>it outwards"]
  O --> G["It merges with others:<br/>larger farther out"]
```

Charnoz, Salmon & Crida (2010) reproduced this way the small moons just
outside Saturn's rings: Atlas, Prometheus, Pandora, Janus and Epimetheus.
In their model these moons formed less than 10 million years ago, and their
mass grows with distance from the rings' edge.
Crida & Charnoz (2012) extended the model: "when the spreading is slow, a
retinue of satellites appear with masses increasing with distance to the
Roche radius", in agreement with the moon systems of Saturn, Uranus and
Neptune. When the spreading is fast, a single large moon forms instead.

**Orders of magnitude.** The material is scarce. Saturn's rings weigh about
1.5 × 10¹⁹ kg (Iess et al. 2019). Pan weighs a three-thousandth of that.
Mimas (198 km in radius, 3.08 Saturn radii out) weighs 3.75 × 10¹⁹ kg:
more than the rings themselves, and over ten times all the small moons from
Pan to Janus together. Porco et al. (2007) suggest that Janus and Epimetheus
ran out of ring material before they reached their critical density.

### What we do not know

- **The rings' age.** The rings may be 10 to a few hundred million years old
  (Iess et al. 2019; Kempf et al. 2023), or much older: Crida et al. (2019)
  argue that their clean ice does not prove they are young. Either way, the
  small moons at their edge are young or have been rebuilt since.
- **The cores.** Nobody has seen a core. Their sizes come from the moons'
  densities and the model above, not from observation.

## In the app

### What you see

A giant's bright rings end at the fluid Roche limit of their particles. Its
ring moons are bodies of the system:

- **Moonlets** inside the rings, each clearing a gap.
- **Edge moons** just beyond the rings' outer edge.
- **Shepherds**: a pair on either side of a faint narrow ring.
- **A resonant moon** outside the rings, which opens a gap at its 2:1
  resonance (like Mimas and the Cassini Division).

Each one's card tells its part ("orbits inside the rings and clears a gap
40 km wide round its orbit"). The rules and their sources are in
[docs/analysis/14-rings.md](../analysis/14-rings.md).

### How it computes them

The system draws a giant's rings, then
[`rings::shape`](../../crates/mapgen/src/rings.rs) shapes them with the
giant's density and mass:

1. **The outer edge** is the fluid Roche limit of the ring particles, their
   density drawn between 0.5 and 0.9 g/cm³. It is kept between 2.0 and
   2.5 giant radii (Saturn's rings end at 2.27).
2. **Moonlets**, zero to two, sit in the outer 40 % of the rings. Each one
   is kept outside the rigid Roche limit for its own density, so a solid
   body could survive there.
3. **Each moonlet's gap** is 7.5 to 17 of its Hill radii wide, the narrowest
   for the lightest moonlet. The Hill radius is the long semi-axis of its
   Roche lobe, from the second equation above.
4. **Edge moons and shepherds** are placed outside the rings or on either
   side of the narrow ring, each outside its own rigid Roche limit.

The moonlet draw, trimmed (from `crates/mapgen/src/rings.rs`):

```rust
// Diameter of a gap-clearing moonlet, km (Daphnis 8, Pan 28).
const MOONLET_KM: (f64, f64) = (4.0, 30.0);

// For each moonlet:
let diameter = (MOONLET_KM.0.ln()
    + rng.random::<f64>() * (MOONLET_KM.1 / MOONLET_KM.0).ln())
.exp(); // log-uniform: as many from 4 to 11 km as from 11 to 30 km
let density = rng.random_range(0.35..0.6); // g/cm³, porous ice
let from = rigid_roche(giant_density, density).max(inner + 0.6 * (outer - inner));
let a = from + at * (outer - 0.03 - from); // giant radii
let mass = 4.0 / 3.0 * PI * (radius_km * 1e3).powi(3) * density * 1e3; // kg
let hill = a * (mass / (3.0 * giant_mass_kg)).cbrt(); // giant radii
// k: the log of the mass, from 0 (Daphnis) to 1 (Pan).
let width = hill * (GAP_HILL.0 + k * (GAP_HILL.1 - GAP_HILL.0));
```

The sizes each kind of ring moon is drawn from:

| Kind | Size | Density (g/cm³) | Solar System examples |
| --- | --- | --- | --- |
| Moonlet | 4 to 30 km across, log-uniform | 0.35–0.6 | Daphnis, Pan |
| Edge moon | 10 to 100 km in radius, log-uniform | 0.4–0.7 | Atlas, Janus |
| Shepherd | 10 to 25 km in radius | 0.5–1.0 | Cordelia, Ophelia |
| Resonant moon | 150 to 250 km in radius | 1.1–1.3 | Mimas |

Small moons get irregular shapes, the more elongated the smaller they are
(`irregular` in the same file).

### Where it departs from the science

- **The sizes are drawn, not grown.** The app draws each ring moon's size
  from the range seen round Saturn and Uranus. It does not model a core or
  the critical density. The result looks right because the ranges come from
  the real moons, not because the app computes the limit.
- **No growth with distance within a kind.** Moonlets are smaller than edge
  moons, and edge moons smaller than the resonant moon. That keeps the trend
  of Charnoz et al. (2010) between kinds. Within a kind, though, size and
  distance are drawn independently: an edge moon far out can be smaller
  than one close in.
- **The density ignores the critical density.** A moonlet's density, 0.35
  to 0.6 g/cm³, matches Saturn's moonlets. Round a Saturn-like giant the
  critical density in the rings is about 0.4 to 0.5, so this is consistent.
  Round a dense giant like Jupiter (1.33 g/cm³) it rises to about 0.75 at
  2.4 radii. A moonlet of 0.35 there would take about twice the volume of
  its Roche lobe: such a moon would lose its outer layers.
- **Shapes are not Roche lobes.** Real moonlets that fill their lobe are
  stretched towards the planet (3 : 2 : 2). Pan and Atlas also wear an
  equatorial ridge of ring material. The app's irregular shapes have neither.

## Sources

1. Murray, C. D. & Dermott, S. F. 1999, *Solar System Dynamics*, Cambridge
   University Press, §4.7 (Roche limits).
2. Porco, C. C., Thomas, P. C., Weiss, J. W. & Richardson, D. C. 2007,
   "Saturn's small inner satellites: clues to their origins", *Science* 318,
   1602–1607, [doi:10.1126/science.1143977](https://doi.org/10.1126/science.1143977).
   Roche lobes, critical density (eq. 1–3), the moons' sizes and densities
   (Table 1), the core simulations.
3. Charnoz, S., Salmon, J. & Crida, A. 2010, "The recent formation of
   Saturn's moonlets from viscous spreading of the main rings", *Nature*
   465, 752–754, [doi:10.1038/nature09096](https://doi.org/10.1038/nature09096).
4. Crida, A. & Charnoz, S. 2012, "Formation of regular satellites from
   ancient massive rings in the Solar System", *Science* 338, 1196–1199,
   [doi:10.1126/science.1226477](https://doi.org/10.1126/science.1226477),
   preprint [arXiv:1301.3808](https://arxiv.org/abs/1301.3808).
5. Iess, L. et al. 2019, "Measurement and implications of Saturn's gravity
   field and ring mass", *Science* 364, eaat2965,
   [doi:10.1126/science.aat2965](https://doi.org/10.1126/science.aat2965).
6. Kempf, S. et al. 2023, "Micrometeoroid infall onto Saturn's rings
   constrains their age to no more than a few hundred million years",
   *Science Advances* 9, eadf8537,
   [doi:10.1126/sciadv.adf8537](https://doi.org/10.1126/sciadv.adf8537).
7. Crida, A. et al. 2019, "Are Saturn's rings actually young?", *Nature
   Astronomy* 3, 967–970,
   [doi:10.1038/s41550-019-0876-y](https://doi.org/10.1038/s41550-019-0876-y).
8. NASA Space Science Data Coordinated Archive, [Saturnian rings fact
   sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/satringfact.html)
   and [Saturnian satellite fact
   sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/saturniansatfact.html)
   (ring edges; Mimas).

## See also

- [docs/analysis/14-rings.md](../analysis/14-rings.md): the study behind the
  app's rings and ring moons, with more sources.
- [docs/system-spec.md](../system-spec.md): ring moons as bodies of a
  system.
