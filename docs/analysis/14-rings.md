# Analysis: rings of giant planets and the moons inside them

User request (2026-11-06): "Is there a maximum to the extent of the
giants' rings? Normally some moons can sit inside the rings (and clear
what lies on their orbit), but I've never seen that in our generator."

A study first, no code: the physics of how far rings reach, the moons
that live in and next to them, what the generator does today, and a
first proposal for bringing ring moons in. It follows the giant planets
study (`12-gas-giants.md`, which gave the rings a line) and the moons
study (`11-moons.md`, which left the moons of giants aside). Asteroid
belts and Lagrange points are studied in parallel in `13-*`.

Distances below are in radii of the planet (R, equatorial) unless given
in km.

## Short answer

- **Dense, bright rings have a maximum: the Roche limit**, about
  2.15–3.6 R for icy particles round a giant. Beyond it, ring material
  gathers into moons within a few orbits. Saturn's main rings end at
  2.27 R, inside its Roche band for ice (2.15–2.62 R).
- **Faint dust rings have no such limit.** They are not held up by
  anything: they are fed by a source (a geyser moon, impacts on small
  moons) and last only as long as it feeds them. Saturn's E ring spans
  about 3–8 R round Enceladus, and its Phoebe ring reaches about 100 to
  270 R. A dust ring follows its source moon, not the Roche limit.
- **Exceptions are known but rare:** Quaoar, a dwarf planet, holds a
  dense ring at 7.4 of its radii, far outside its classical Roche limit
  (Morgado et al. 2023). Elastic collisions of very cold ice and a
  spin–orbit resonance may keep it from accreting. Nothing like it is
  known round a giant.
- **Every ring system of the Solar System has moons in it or on its
  edges**: 2 to about 9 per giant (section 2). Our generator has none:
  its regular moons start at 4–12 R, and its ring gaps are drawn at
  random places, tied to no moon (section 3).

## 1. How far rings reach

### The Roche limit

A moon held together by its own gravity is pulled apart by the
planet's tides inside the Roche limit:

- **Fluid body** (no strength; a rubble pile, an icy aggregate):
  `d ≈ 2.44 R (ρ_planet / ρ_moon)^(1/3)`.
- **Rigid body** (held by material strength as well):
  `d ≈ 1.26 R (ρ_planet / ρ_moon)^(1/3)`.

Ring particles are porous water ice, about 0.5–0.9 g/cm³. The fluid
limit for them is below, in equatorial radii, the unit ring distances
are given in. The formula's `R` and `ρ_planet` must come from the same
radius: the mean density (column 2) goes with the mean radius, so the
limits here take the planet's density over a sphere of its equatorial
radius instead (from NASA's masses and equatorial radii; Saturn: 0.62
g/cm³ over 60,268 km).

| Planet | Mean ρ (g/cm³) | Fluid, ice 0.9 | Fluid, ice 0.5 | Rigid, ice 0.9 |
| --- | --- | --- | --- | --- |
| Jupiter | 1.33 | 2.72 R | 3.30 R | 1.40 R |
| Saturn | 0.69 | 2.15 R | 2.62 R | 1.11 R |
| Uranus | 1.27 | 2.72 R | 3.30 R | 1.40 R |
| Neptune | 1.64 | 2.96 R | 3.60 R | 1.53 R |

Between the rigid and the fluid limit, small solid bodies survive (Pan
and Daphnis inside Saturn's A ring, Metis and Adrastea in Jupiter's
main ring), but loose material cannot gather into a new moon: tides
pull clumps apart faster than gravity assembles them. That is why the
main rings sit in this zone, and why bright rings end near the fluid
limit.

The Roche limit is a zone, not a sharp line. Accretion becomes
possible, then easy, from about the fluid limit outwards, and the
particles' density, size and spin set where (Canup & Esposito 1995;
Ohtsuki 1993). Quaoar's ring shows it can be pushed further out when
collisions are very elastic (Morgado et al. 2023).

### What the Solar System's rings span

| Planet | Ring | Span (R) | Kind |
| --- | --- | --- | --- |
| Jupiter (R = 71,492 km) | Halo | 1.29–1.71 | dust, thick torus |
| | Main | 1.72–1.81 | dust, bounded by Metis and Adrastea |
| | Amalthea gossamer | 1.81–2.54 | dust from Amalthea |
| | Thebe gossamer | 1.81–3.11 | dust from Thebe |
| Saturn (R = 60,268 km) | D | 1.11–1.24 | faint |
| | C | 1.24–1.53 | dense, dark-ish |
| | B | 1.53–1.95 | densest, brightest |
| | Cassini Division | 1.95–2.03 | low-density gap |
| | A | 2.03–2.27 | dense, with the Encke and Keeler gaps |
| | F | 2.33 | narrow, shepherded |
| | G | 2.75–2.9 | dust, an arc round Aegaeon |
| | E | about 3–8 | dust from Enceladus (3.95) |
| | Phoebe | about 100–270 | dust from Phoebe, tilted, retrograde |
| Uranus (R = 25,559 km) | 6, 5, 4, α, β, η, γ, δ | 1.64–1.90 | narrow, dark |
| | ε | 2.006 | narrow, dense, shepherded by Cordelia and Ophelia |
| | ν | 2.6–2.7 | dust |
| | μ | 3.4–4.0 | dust, round Mab |
| Neptune (R = 24,764 km) | Galle | 1.69 | faint, broad |
| | Le Verrier | 2.15 | narrow |
| | Lassell, Arago | 2.15–2.31 | faint |
| | Adams | 2.54 | narrow, with arcs held by Galatea |

So:

- **Dense rings** (Saturn's B and A, Uranus's ε) stay inside about
  2.3 R.
- **Narrow dark rings** (Uranus, Neptune) lie at 1.6–2.6 R, well inside
  the fluid limit of their dense planets; shepherd moons hold them at
  their places.
- **Dust rings** reach as far as their sources: Thebe at 3.1 R,
  Enceladus at 4 R, Phoebe at 215 R.
- **The inner edge** is set by the planet. Below about 1.1 R, its upper
  atmosphere drags particles in, and inside the synchronous orbit tides
  pull the rings slowly inwards.

### Masses and ages

- **Saturn's rings** weigh about 1.5 × 10¹⁹ kg, about 0.4 of Mimas
  (Iess et al. 2019, from Cassini's last orbits). Their low dust
  darkening suggests an age of 10–100 Myr, or a few hundred Myr at most
  (Iess et al. 2019; Kempf et al. 2023). This is debated: rings can
  clean themselves of dust, so they could be old and still look young
  (Crida et al. 2019; Hyodo et al. 2025).
- **The other giants' rings** are many orders of magnitude lighter:
  dust, and narrow ringlets of dark material.

The likely origins of massive rings are a moon torn apart inside the
Roche limit, the stripped icy mantle of a migrating moon (Canup 2010),
or a passing body torn apart (Dones 1991). Dust rings are short-lived
and continuously resupplied.

### Rings and moons are one system

A ring spreads viscously, through collisions and self-gravity wakes.
What crosses the Roche limit outwards gathers into moonlets, which
migrate outwards by their tides on the ring and on the planet. This
explains why Saturn's small inner moons grow in mass with distance
from the A ring's edge: Pan, Daphnis, Atlas, Prometheus, Pandora,
Epimetheus and Janus (Charnoz et al. 2010; Crida & Charnoz 2012). The
same model, run longer, may have built the regular moons of Uranus and
Neptune from ancient massive rings (Crida & Charnoz 2012). Jupiter's
rings go the other way: they are dust knocked off its four inner moons
(Burns et al. 1999).

### Rings outside the Solar System

- **J1407b** is a candidate ring system about 0.6 AU across round a
  substellar companion, seen once in a 56-day eclipse of its star
  (Mamajek et al. 2012; Kenworthy & Mamajek 2015). If real, it fills a
  good part of its body's Hill sphere. It would be a young
  circumplanetary disk rather than a ring system like Saturn's.
- **No confirmed exoplanet rings.** Several inflated, low-density
  planets have been proposed as ringed instead.

### Is there a maximum? (answer)

- **Bright, dense rings:** the fluid Roche limit for their particles,
  2.15–2.62 R round Saturn, 2.7–3.3 R round Jupiter or Uranus and
  3.0–3.6 R round Neptune. Saturn's rings end at 2.27 R. A generator
  should keep its bright rings inside `2.44 R (ρ_p / ρ_particles)^(1/3)`,
  `ρ_p` taken over the same radius `R`.
- **Faint dust rings:** no physical maximum short of the Hill sphere.
  They sit where a moon feeds them: a gossamer ring out to an inner
  moon, an E-type ring round a geyser moon, a Phoebe-type ring round a
  far irregular moon.

## 2. Moons in and near rings

### Kinds

- **Ring moons and shepherds:** small moons just outside a ring's edge,
  or on both sides of a narrow ring. Their torques push ring material
  away from their orbits and keep the edge sharp (Goldreich & Tremaine
  1979). Examples: Prometheus and Pandora round the F ring, Cordelia
  and Ophelia round Uranus's ε ring, Atlas at the A ring's outer edge.
- **Embedded moonlets that clear gaps:** a moon inside a ring opens a
  gap round its orbit if it is large enough. Pan (about 28 km across)
  keeps the Encke gap, 325 km wide. Daphnis (about 8 km) keeps the
  Keeler gap, about 35–42 km wide, and raises waves on its edges.
- **Propellers:** a moonlet too small to open a full gap (tens of
  metres to about 1 km across in Saturn's A ring) clears only two short
  partial gaps ahead and behind it, shaped like a propeller. Cassini
  found hundreds of them (Tiscareno et al. 2006, 2008). The large ones
  are tracked like named moons (Blériot, Earhart; Seiß et al. 2017).
- **Resonance gaps and edges:** a moon outside the rings shapes them at
  its mean-motion resonances:
  - Mimas's 2:1 inner Lindblad resonance sets the outer edge of the B
    ring and so the inner edge of the Cassini Division.
  - The 7:6 resonance of Janus and Epimetheus sets the outer edge of
    the A ring.
  - Hundreds of weaker resonances raise density and bending waves.
- **Moons that make arcs:** some small moons hold dust or ring material
  in arcs at a corotation resonance. Galatea holds Neptune's Adams ring
  arcs (42:43); Aegaeon, Methone and Anthe sit in arcs of Saturn's G
  ring and in their own.
- **Co-orbital moons** (Telesto and Calypso at Tethys's L4 and L5,
  Helene and Polydeuces at Dione's) are moons, not ring moons: they
  share a larger moon's orbit far outside the rings. Lagrange points
  and co-orbitals are covered in `13-asteroids.md`.
- **Moons that make dust rings:** Metis and Adrastea feed Jupiter's
  main ring, Amalthea and Thebe the gossamer rings, Enceladus the E
  ring, Phoebe its ring, and Mab Uranus's μ ring.

### How large a gap a moon clears

A moon of mass `m` round a planet of mass `M` (`μ = m / M`) at a
distance `a` has a Hill radius `r_H = a (μ / 3)^(1/3)`. In a dense ring:

- **Gap or propeller:** a full circumferential gap opens when the moon
  clears material faster than the ring's viscosity fills it back in.
  Below that size, the moon makes a propeller whose blades are about
  4 r_H apart radially (Seiß et al. 2005; Lewis & Stewart 2009) and
  whose length grows as `r_H³ / ν` (Spahn & Sremčević 2000). In
  Saturn's A ring the transition lies at moonlets a few km across.
- **Width of a full gap:** balancing the moon's torque against the
  viscous torque gives a gap that widens faster than `r_H`, roughly as
  `μ^(2/3) ν^(-1/3)` (Lissauer, Shu & Cuzzi 1981). The two cases in
  Saturn's A ring:

| Moon | Mass (kg) | `a` (km) | `r_H` (km) | Gap (km) | Gap / `r_H` |
| --- | --- | --- | --- | --- | --- |
| Pan | ≈ 5.0 × 10¹⁵ | 133,584 | ≈ 19 | 325 | ≈ 17 |
| Daphnis | ≈ 8 × 10¹³ | 136,505 | ≈ 5 | 35–42 | ≈ 7–8 |

  Pan is about 60 times as massive as Daphnis. Its gap is about 9 times
  as wide, between the `r_H` ratio (4) and the `μ^(2/3)` ratio (16).
  For a generator, a gap of 7–17 Hill radii of its moon, wider for the
  larger moon, matches what we see.

### How many there are

Moons in or at the edge of each giant's rings:

- **Jupiter, 4:** Metis, Adrastea (in the main ring), Amalthea, Thebe
  (gossamer rings).
- **Saturn, about 10, plus propellers:** Pan, Daphnis (embedded); Atlas,
  Prometheus, Pandora (edges); Janus, Epimetheus (A ring edge); Aegaeon,
  Methone, Anthe (arcs); hundreds of propellers.
- **Uranus, 3 at least:** Cordelia, Ophelia (ε shepherds), Mab (μ ring);
  about ten more small inner moons near the rings.
- **Neptune, 4:** Naiad, Thalassa, Despina (inside or among the rings),
  Galatea (Adams arcs).

So **no giant of the Solar System has rings without ring moons.** These
moons are small, 1 km to about 200 km (Janus is 180 km across, Amalthea
170 km), far smaller than the regular moons (Mimas, 400 km, is the
smallest of Saturn's round moons). Mimas, the innermost round moon,
still sculpts the rings from 3.1 R through its resonance.

## 3. What the generator does today

### Two ring draws that do not agree

- **Giant worlds** (`crates/mapgen/src/giant_world.rs:437–471`, the
  `Look` of a giant world) draw a `RingSystem` from the preset's
  `[giant] rings` odds:
  - gas giants 25 % bright, 50 % faint (`presets/gas-giant.toml`);
  - ice giants 0 % bright, 70 % faint (`presets/ice-giant.toml`);
  - hot Jupiters 0 % bright, 5 % faint (`presets/hot-jupiter.toml`).

  A bright ring runs from 1.2–1.5 R to 2.1–2.5 R, with 1 to 3 gaps of
  0.015–0.06 R at random places. A faint ring starts at 1.5–1.9 R and is
  0.05–0.3 R wide, with no gaps. This is what the planet view draws
  (`web/src/lib/viewer/PlanetView.ts`, `ringShaders.ts`, up to 4 gaps).
- **Giant neighbours of a rocky world** (`crates/mapgen/src/giants.rs:
  464–475`, `describe`) draw a `Rings` enum of their own:
  - 15 % bright, 60 % faint, 25 % none;
  - for hot giants, 10 % faint.

  The star view of that world draws these (`StarView.ts:820`).

The two draws use different streams and different odds, so the same
kind of giant gets rings at different rates depending on how it is
shown. The two never describe the same planet: a neighbour giant in a
rocky world's `giants` list is never generated as a world of its own.
The odds disagree, but no planet shows two different ring sets.

In planetary systems (`crates/mapgen/src/system.rs`), a giant body has
rings only once it is generated (its world's `RingSystem`). The star
view's preview of a giant not generated yet has none
(`web/src/lib/systemStar.ts:121`, "the draw does not give rings yet").
A system therefore shows its rings only after each giant has been
generated.

### Ring extent

- **Bright rings:** the outer edge (2.1–2.5 R) matches Saturn's (2.27 R)
  and lies near the fluid Roche limit, but it does not depend on the
  giant's density. Inflated giants (low density, a lower limit) and
  dense ones get the same range. For a giant of Saturn's density the
  limit for ice is 2.15–2.62 R; for one of Jupiter's, 2.72–3.30 R. The
  current range stays inside both but for its lowest edges (2.1 against
  2.15), so this is about right, only blind to the planet.
- **Faint rings:** 1.5–2.2 R matches Uranus's and Neptune's narrow
  rings. There are no dust rings further out: no gossamer rings, no
  E-type or Phoebe-type ring.

### Moons

- **Planetary systems** (`regular_moons`, `system.rs:1084–1146`): two to
  four regular moons on a 1:2 resonant chain (`SIBLING_SPACING`
  1.587), the first at 6–12 R, all within a third of the Hill radius.
  None are near the rings, and none are small ring moons.
- **Giant worlds** (`MoonSystem::of_giant`, `moons.rs:625–700`): two to
  five regular moons from 4–8 R outwards, or a captured Triton at
  12–16 R.
- **A bug in giant worlds:** when a Triton is captured (35 % of ice
  giants), the regular moons' distances are halved, so the first can sit
  at about 1.9–4.2 R. `of_giant` applies no Roche check, unlike the
  rocky planets' draw (`moons.rs:429–442`). An ice giant can therefore
  get a round moon at about 2 R. That is inside its fluid Roche limit
  (about 2.2–2.3 R for a moon of 1.9 g/cm³ round a planet of
  1.3–1.6 g/cm³), and possibly inside its faint ring (1.5–2.2 R).
- **The ring gaps are tied to nothing:** no moon sits in a gap, no gap
  sits at a moon's resonance, and the rings have no edge moons, no
  shepherds and no propellers. The rings and the moons are drawn from
  different streams and never see each other.
- **The hosts of exomoons** (`exomoon.rs`, `HostGiant`) have no rings
  at all: an Earth-like moon's sky never shows its giant's rings.

## 4. A first proposal

The proposal goes in steps. Each step says whether it changes existing
output. Only realism fixes change it.

### Data model

- **`RingSystem`** (`giant_world.rs`) gains:
  - the particle density its edge was set from;
  - for each gap, a `moon` (an index into the giant's moons, or none
    for a resonance gap) and a `kind` (`Embedded`, `Resonance`).

  It stays in `world.json` under `giant.rings`. Older worlds read
  without these fields, all optional.
- **A ring draw at the system level:** the system's body for a giant
  gets an optional `rings` (`RingSystem`), drawn when the system is
  drawn. The giant world then reads it instead of drawing its own. The
  star view and the moons' skies then have the rings before the giant
  is generated.
- **Ring moons as bodies:** they go into `system.json` as small bodies
  of a new type, `ring-moon`. They use the irregular-moon shape and
  preview of `small_body_params` and carry a `role` (`shepherd`,
  `embedded`, `edge`, `source`). Like every body, they can be generated
  on demand. They count in the moon list and in naming (roman
  numerals, innermost first).

### Generation rules

1. **One ring draw per giant, from the system's seed.**
   - *Change:* a giant world inside a system takes the system's draw,
     and the neighbour-giant odds (`giants.rs`) align with the presets'
     (25/50 for gas, 0/70 for ice, 0/5 for hot).
   - *Output impact:* rings change on giants generated afterwards, and
     on the `giants` list of rocky worlds generated afterwards. It is a
     consistency fix, not a realism one: make it opt-in by the system's
     `revision`, so stored worlds keep theirs.
2. **Bright rings end at the Roche limit.**
   - *Change:* the outer edge becomes
     `min(2.5, 2.44 (ρ_planet / ρ_particles)^(1/3))` with
     `ρ_particles` drawn in 0.5–0.9 g/cm³, kept at 2.0 R at least.
   - *Output impact:* small. Most giants keep a similar edge, and dense
     giants could reach a little further. It is a realism fix: flag it
     in the changelog and apply it to new draws only.
3. **Gaps belong to moons.**
   - *Change:* each bright ring gets:
     - 0–2 embedded moonlets of 2–30 km at random radii inside it, each
       with a gap of 7–17 of its Hill radii (larger for the larger
       moon). This replaces the random gaps.
     - One resonance gap or edge where the innermost regular moon's 2:1
       resonance falls inside the ring, at `a_moon × 2^(-2/3)` (0.63
       of its distance; Mimas at 3.08 R puts it at 1.95 R). A sharp
       drop in opacity outside it, the Cassini Division.
   - *Output impact:* gap positions and widths change on new giants. It
     is a realism fix.
4. **Edge moons and shepherds.**
   - *Change:*
     - **Bright rings:** 1–3 small moons of 20–200 km between the outer
       edge and 1.3 × the outer edge (Atlas, Prometheus, Pandora,
       Janus).
     - **Narrow faint rings:** a pair of shepherds of 20–50 km on
       either side of the ring.
   - *Output impact:* new bodies in new systems; regular moons keep
     their draws (separate stream).
5. **Dust rings fed by moons** (optional, later).
   - *Change:*
     - a gossamer ring out to the innermost small moon;
     - on a giant with an active icy moon (a heated one, already known
       from tides: `HostGiant` computes tidal heating), an E-type ring
       round its orbit;
     - on a giant with captured irregulars, a faint, wide Phoebe ring.
   - *Output impact:* new, faint layers; drawing only.
6. **Fix the captured-Triton regular moons.**
   - *Change:* apply the fluid Roche check of the rocky planets' draw to
     `of_giant`, and keep the regular moons outside the rings' outer
     edge plus a margin.
   - *Output impact:* the few ice giant worlds where a moon fell inside
     about 2.3 R lose it or see it pushed outwards. It is a realism fix
     and a bug fix.

### The viewers

- **Planet view of a giant:** draw the gap's moon as a small dot at its
  place on its orbit (moving with the clock), the shepherds as dots
  outside the edge, and the gaps from the moon list rather than from a
  random array (`uGaps[4]` is already there).
- **Star view:** the giant's ring moons join its moon markers and the
  *Body* menu, under the giant, marked *ring moon*. Rings show before
  generation (step 1).
- **Moon view:** from a ring moon or a close regular moon, the rings
  cross the sky nearly edge-on, a line through the giant. Today the
  rings are not drawn in the moons' skies at all (`systemSky.ts`,
  `skyBodies.ts`); with step 1 they can be.
- **Cards and sheets:** a ring moon's card says what it does:
  *shepherd of the outer edge*, *clears a 40 km gap*, *source of the
  dust ring*.

### Proposed order

1. Step 6 (a bug fix, small).
2. Step 1 (one draw; the rings in the star view and the skies).
3. Steps 2 to 4 together (ring moons and the gaps they clear).
4. Step 5 later, if wanted.

## 5. Since the study (2026-11-07)

Steps 2 to 5 are in (`rings.rs`, `system.rs`), and follow the physics
further than proposed:

- **The Roche limit alone sets a bright ring's edge.** No cap at 2.5 R:
  the edge is the fluid limit for particles of 0.5–0.9 g/cm³ from the
  giant's density over its equatorial radius, 2.15–2.62 R round a
  Saturn, past 3 R round a denser giant. Only physics bounds it: the
  rings stay Hill stable inside the giant's innermost other moon
  (Gladman 1993) and above its atmosphere (1.11 R, Saturn's D ring;
  Mitchell et al. 2018). The giant asked for (`--planet gas-giant`) and
  the giant a world of the request orbits are shaped as the others; the
  first now has its own mass in its system, not Earth's density.
- **The resonance cuts an edge.** The 2:1 inner Lindblad resonance of the
  resonant moon, found with the giant's `J2` (from its flattening,
  `f = 3/2 J2 + q/2`, Murray & Dermott 1999), is the inner edge of a
  division 0.04–0.09 R wide: the B ring's edge and the Cassini Division.
  Saturn's `J2` puts Mimas's 2:1 650 km outward of Kepler's, at 117,532
  km (Spitale & Porco 2010, with the field's higher terms: 117,556).
- **Ring moons grow.** A moonlet grows from a core of solid ice (1.5 to
  10 km) under a mantle of ring particles with 65 % voids until it fills
  its Roche lobe (Porco et al. 2007, eq. 3): it ends at the critical
  density of its distance, `3M / (γ a³)`, in the lobe's 3 : 2 : 2 shape.
  Edge moons and shepherds are no less dense than the critical density,
  filling their lobes when raised to it; edge moons are larger the
  farther out (Charnoz et al. 2010). The resonant moon is irregular below
  200 km in radius (Lineweaver & Norman 2010).
- **Dust-only ring systems.** A giant with faint rings gets one or two
  small moons at 2–3.2 R (Amalthea, Thebe) whose dust spreads from the
  narrow ring out to their orbit (Burns et al. 1999), and on half of them
  a small moon at 3.4–4.2 R wrapped in a faint belt (Mab and the μ ring,
  de Pater et al. 2006). A gossamer ring reaches 1 to 1.26 times its
  moon's distance (Thebe's extension).
- **The A ring's edge (2026-11-08).** The largest edge moon, if at least
  40 km in radius, may sit where its 7:6 inner Lindblad resonance falls on
  the rings' outer edge, which then lies at 0.88 to 0.99 of the Roche limit
  (odds 50 %, a calibration on Saturn alone). The resonance is
  `inner_lindblad(a, 7, J2)`; its Kepler form is `a (6/7)^(2/3)`. Checked
  against Saturn: Janus at 151,460 km gives 136,668 km by Kepler alone and
  about 136,780 km with `J2` = 0.0163, against an edge at 136,770 km and a
  Janus 7:6 resonance at 136,785 km (El Moutamid et al. 2016; Spitale &
  Porco 2009). Embedded moonlets left outside the new edge are dropped.
  The moon is one of the edge moons moved, not an extra body; the ids of
  moons past it can shift. Own stream `RING_EDGE`.
- **A faint D-like ring (2026-11-08).** Half the bright rings get a smooth
  ring of dust, `rings.d_ring`, from 1.11 to 1.15 radii (Saturn's D ring
  starts at 1.11, Hedman et al. 2007; Mitchell et al. 2018) to the bright
  rings' inner edge, at least 0.05 wide, at an opacity of 0.01 to 0.05
  (the real optical depth is about 10⁻³, so the drawn one is a visible
  hint). Own stream `D_RING`. Seen in the giant's view and in the sky of
  its moons (`uRingFaint` in `skyRingDensityGlsl`).
- **Equatorial ridges (2026-11-08).** An embedded moonlet, and an edge moon
  of 30 km in radius or less, has a ridge of ring material it swept up
  (Charnoz et al. 2007; Pan, Atlas and Daphnis, Buratti et al. 2019): its
  shape layer gets `1 + h exp(-(sin φ / 0.3)²)` with `h` 0.12 to 0.25 from
  the moon's seed, over the cube root of the volume it adds so the moon
  keeps its volume. `WorldParams::ridge` carries it (and `meta.ridge`).
- **Not done:** a plume moon chosen by its tidal heating, and a Phoebe ring
  (systems have no irregular moons yet), both still in backlog item 18.

## Sources

- Roche limit, classical and for rubble piles: Murray & Dermott 1999,
  *Solar System Dynamics*, Cambridge University Press, §4.7; Canup &
  Esposito 1995, "Accretion in the Roche zone: coexistence of rings and
  ringmoons", Icarus 113, 331; Ohtsuki 1993, "Capture probability of
  colliding planetesimals: dynamical constraints on accretion of
  planets, satellites, and ring particles", Icarus 106, 228.
- Ring extents: NASA planetary ring fact sheets
  (<https://nssdc.gsfc.nasa.gov/planetary/planetfact.html>); de Pater &
  Lissauer 2015, *Planetary Sciences*, 2nd ed., chapter 13; Tiscareno &
  Murray (eds.) 2018, *Planetary Ring Systems*, Cambridge University
  Press.
- Gazetteer of ring features (Encke 325 km, Keeler 35 km): USGS
  Gazetteer of Planetary Nomenclature,
  <https://planetarynames.wr.usgs.gov/>.
- Phoebe ring: Verbiscer, Skrutskie & Hamilton 2009, "Saturn's largest
  ring", Nature 461, 1098; Hamilton et al. 2015, "Small particles
  dominate Saturn's Phoebe ring to surprisingly large distances",
  Nature 522, 185.
- Quaoar: Morgado et al. 2023, "A dense ring of the trans-Neptunian
  object Quaoar outside its Roche limit", Nature 614, 239,
  <https://doi.org/10.1038/s41586-022-05629-6>.
- Ring mass and age: Iess et al. 2019, "Measurement and implications of
  Saturn's gravity field and ring mass", Science 364, eaat2965; Kempf
  et al. 2023, "Micrometeoroid infall onto Saturn's rings constrains
  their age to no more than a few hundred million years", Science
  Advances 9, eadf8537; Crida et al. 2019, "Are Saturn's rings actually
  young?", Nature Astronomy 3, 967; Hyodo, Genda & Madeira 2025,
  "Pollution resistance of Saturn's ring particles during
  micrometeoroid impact", Nature Geoscience 18, 25.
- Ring origins: Canup 2010, "Origin of Saturn's rings and inner moons
  by mass removal from a lost Titan-sized satellite", Nature 468, 943;
  Dones 1991, "A recent cometary origin for Saturn's rings?", Icarus
  92, 194.
- Rings and moons as one system: Charnoz, Salmon & Crida 2010, "The
  recent formation of Saturn's moonlets from viscous spreading of the
  main rings", Nature 465, 752; Crida & Charnoz 2012, "Formation of
  regular satellites from ancient massive rings in the Solar System",
  Science 338, 1196.
- Jupiter's rings from its inner moons: Burns et al. 1999, "The
  formation of Jupiter's faint rings", Science 284, 1146.
- Uranus's μ ring: de Pater, Hammel, Gibbard & Showalter 2006, "New dust
  belts of Uranus: one ring, two ring, red ring, blue ring", Science 312,
  92, <https://doi.org/10.1126/science.1125110>.
- The B ring's edge and Mimas's 2:1: Spitale & Porco 2010, "Free unstable
  modes and massive bodies in Saturn's outer B ring", AJ 140, 1747,
  <https://doi.org/10.1088/0004-6256/140/6/1747>.
- The D ring's inner edge: Mitchell et al. 2018, "Dust grains fall from
  Saturn's D-ring into its equatorial upper atmosphere", Science 362,
  eaat2236, <https://doi.org/10.1126/science.aat2236>.
- Hill stability: Gladman 1993, "Dynamics of systems of two close
  planets", Icarus 106, 247, <https://doi.org/10.1006/icar.1993.1169>,
  in the form of Obertas, Van Laerhoven & Tamayo 2017, Icarus 293, 52,
  <https://doi.org/10.1016/j.icarus.2017.04.010>.
- Round moons: Lineweaver & Norman 2010, "The potato radius: a lower
  minimum size for dwarf planets", <https://arxiv.org/abs/1004.1091>.
- Shepherding and ring edges: Goldreich & Tremaine 1979, "Towards a
  theory for the Uranian rings", Nature 277, 97; Goldreich & Tremaine
  1982, "The dynamics of planetary rings", ARA&A 20, 249.
- Gaps of embedded moons: Lissauer, Shu & Cuzzi 1981, "Moonlets in
  Saturn's rings?", Nature 292, 707; Porco et al. 2007, "Saturn's small
  inner satellites: clues to their origins", Science 318, 1602 (masses
  of Pan and Daphnis); Weiss et al. 2009, "Ring edge waves and the
  masses of nearby satellites", AJ 138, 272.
- Propellers: Spahn & Sremčević 2000, "Density patterns induced by
  small moonlets in Saturn's rings?", A&A 358, 368; Seiß et al. 2005,
  "Perturbations of the planetary rings by moonlets", GRL 32, L11205;
  Tiscareno et al. 2006, "100-metre-diameter moonlets in Saturn's A
  ring from observations of 'propeller' structures", Nature 440, 648;
  Tiscareno et al. 2008, AJ 135, 1083; Lewis & Stewart 2009, Icarus
  199, 387; Seiß et al. 2017, "Hydrodynamic simulations of
  moonlet-induced propellers in Saturn's rings: application to
  Blériot", AJ 154, 1, <https://arxiv.org/abs/1701.04641>.
- Resonances and the Cassini Division: Goldreich & Tremaine 1978, "The
  formation of the Cassini division in Saturn's rings", Icarus 34, 240;
  Spitale & Porco 2009, "Time variability in the outer edge of Saturn's
  A-ring revealed by Cassini imaging", AJ 138, 1520 (the 7:6 Janus and
  Epimetheus resonance).
- Equatorial ridges of Pan and Atlas: Charnoz, Brahic, Thomas & Porco 2007,
  "The equatorial ridges of Pan and Atlas: terminal accretionary
  ornaments?", Science 318, 1622, <https://doi.org/10.1126/science.1148631>
  (ridges of 10 to 25 % of the volume: read in a secondary summary, the
  paper itself was not read); Buratti et al. 2019, "Close Cassini flybys
  of Saturn's ring moons Pan, Daphnis, Atlas, Pandora, and Epimetheus",
  Science 364, eaat2349, <https://doi.org/10.1126/science.aat2349> (cited,
  not read).
- The D ring: Hedman et al. 2007, "Saturn's dynamic D ring", Icarus 188,
  89, <https://doi.org/10.1016/j.icarus.2006.11.017> (extent about 66,000
  to 74,500 km and optical depth about 10⁻³ from a review table and the
  abstract; the paper's text could not be read).
- The 7:6 resonance: El Moutamid et al. 2016, "How Janus' orbital swap
  affects the edge of Saturn's A ring?", Icarus 279, 125,
  <https://arxiv.org/abs/1510.00434> (Janus's 7:6 resonance at 136,785
  km, 15 km outside the mean edge).
- Neptune's arcs: Porco 1991, "An explanation for Neptune's ring
  arcs", Science 253, 995; Namouni & Porco 2002, Nature 417, 45.
- J1407b: Mamajek et al. 2012, AJ 143, 72; Kenworthy & Mamajek 2015,
  "Modeling giant extrasolar ring systems in eclipse and the case of
  J1407b", ApJ 800, 126.
