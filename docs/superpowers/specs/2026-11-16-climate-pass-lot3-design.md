# Climate pass lot 3: subsurface oceans and geysers

Refs #423 (track A), climate pass lot 3 (see #427). Closes #463 and #304.

## Intent

The user's criterion for the climate pass is the most physical realism
the model can afford. Lot 3 makes the oceans under the ice of icy moons
come out of the physics that really separates them, and gives the moons
that vent their ocean visible geysers.

Today (`main` at fa620b1, see #463):

| Moon | Model today | Reference |
|---|---|---|
| Enceladus | no ocean; tidal heat 2.8 GW (`ICY_K2_OVER_Q` 2.5e-3) | global ocean under 20-25 km of ice on average, about 5 km at the south pole (Čadek et al. 2016, doi:10.1002/2016GL068634; Thomas et al. 2016, doi:10.1016/j.icarus.2015.08.037); 15.8 GW from the south polar terrain (Howett et al. 2011, doi:10.1029/2010JE003718) |
| Europa | shell 5-40 km over an ocean (test passes) | shell about 10-30 km over a 60-150 km ocean |
| Ganymede | shell 180-490 km conductive; ocean depends on the radiogenic draw | ocean under about 150 km of ice, over high-pressure ice (Saur et al. 2015, doi:10.1002/2014JA020778) |
| Titan | same as Ganymede | ocean under 50-200 km of ice, from its tidal Love number k2 about 0.6 (Iess et al. 2012, doi:10.1126/science.1219631) |
| Callisto | thick shell; ocean depends on the radiogenic draw | partly differentiated (moment of inertia 0.355, Anderson et al. 2001, doi:10.1006/icar.2001.6664); no ocean or a thin one deep down (Khurana et al. 1998, doi:10.1038/27394) |
| Plume ring (E ring) | its moon drawn at odds 0.25: the innermost icy regular moon | fed by the geysers of a moon heated by its resonance |

Ganymede's and Callisto's shell-base pressures agree within 2 % today,
because the conductive scaling goes as g/q: the model has nothing that
separates them.

## Goals

1. The ocean under the ice of an icy body comes from cited physics, each
   relation with its own source:
   - a convective ice shell (stagnant conductive lid over a convecting
     layer) when the shell is thick enough to convect (Spohn & Schubert
     2003, doi:10.1016/S0019-1035(02)00048-9; Hussmann et al. 2006,
     doi:10.1016/j.icarus.2006.06.005; Barr & McKinnon 2007,
     doi:10.1029/2006JE002781);
   - equilibrium tidal heating of a moon held in a mean-motion
     resonance, set by its host's own tidal dissipation (Meyer & Wisdom
     2007, doi:10.1016/j.icarus.2007.03.001; Lainey et al. 2012,
     doi:10.1088/0004-637X/752/1/14 for Saturn's Q), so Enceladus's
     heat comes from its orbit, not from a parameter of its archetype;
   - partial differentiation of a moon that never heated enough to
     separate its rock from its ice (Callisto; e.g. Barr & Canup 2008,
     doi:10.1016/j.icarus.2008.07.004, impacts differentiating Ganymede
     but not Callisto);
   - the ammonia already carried in the ices lowering the ocean's
     melting point as it concentrates when the ocean freezes.
2. The same rules hold for every body: Solar System archetypes come out
   close to the real moon on every seed (user's rule since lot 2a),
   drawn moons follow the same relations, and icy planets with a shell
   follow them too.
3. Geysers are decided by physics, not by odds (#304): a body vents when
   it has an ocean, a shell thin enough at a pole to crack through, and
   enough tidal heating. Enceladus-like always does. Europa-like may on
   some seeds. Ganymede-, Titan- and Callisto-like never do.
4. Geysers sit at a logical place on the surface map: parallel fractures
   near one pole, where tidal heating thins the shell most (as the
   "tiger stripes" at Enceladus's south pole; polar thinning: Ojakangas
   & Stevenson 1989, doi:10.1016/0019-1035(89)90052-3; Čadek et al.
   2016). The seed picks which pole.
5. The user sees the geysers:
   - from the host: a moon in the sky of its host (`PlanetView`
     `setSkyBodies`) shows a pale plume above its active pole;
   - in the close-up of the moon: distinct jets above the fractures;
   - both brighter when back-lit (fine grains scatter forward), as in
     Cassini's high-phase images.
6. The plume ring (E ring) is fed by a moon with geysers, and only by
   one.
7. The card says what the user needs to judge the other moons until the
   cross-section view exists: ice shell thickness, ocean depth, tidal
   heating (W), whether the shell convects, whether the body is fully
   differentiated, and "active geysers". Each value has its cause in the
   card's explanation tree, like the other model fields.

## Non-goals

- The cross-section view of the interior (a later track).
- New icy biomes (#424). The fractures are relief and fresh ice on the
  existing icy biomes.
- Plumes of other kinds: Io's volcanic plumes, Triton's nitrogen geysers
  (solar-driven, Soderblom et al. 1990).
- Plumes in the system overview (`StarView`), where moons are marker
  spheres.
- A full orbital evolution of resonances: resonances come from the
  existing chain spacing of `moons.rs`, not from a migration history.
- Salts in the melting point (only ammonia, which the model already
  carries).

## Anti-goals

- No per-archetype heat, shell or ocean values: an archetype gives its
  orbit and composition, the physics does the rest. Calibrated constants
  are allowed only where a cited relation needs one, each stated.
- No geysers drawn at odds, anywhere.
- No `FORMAT_VERSION` gate keeping the old shell rule (user's policy).
- Raising the shared icy k2/Q to heat Enceladus (it would thin Europa's
  shell under 5 km and change icy planets; see #463).
- A plume in the viewer that does not match the fractures on the map.

## Design

### Part A: oceans and the card (first branch)

- Ice shell: when the conductive shell is thicker than the onset of
  convection, the shell is a stagnant lid over a convecting layer, and
  the heat it carries follows a cited scaling of the convective flux.
  `IceShell` gains the fields the card needs (at least: convecting or
  not, lid thickness, polar shell thickness).
- Tidal heating: a moon in a 2:1 (or other first-order) resonance of the
  chain gets the equilibrium heating of Meyer & Wisdom 2007, set by its
  host's k2/Q (`ENVELOPED_K2_OVER_Q` 1.1e-5 is the giants' value today)
  and the pair's orbits. Other moons keep `tidal_power_w` with their own
  eccentricity. The model's context must then carry the resonance
  partner; today `HostTide` holds only the host's mass, the moon's a and
  e (`planet/inputs.rs:47`).
- Differentiation: a body that did not differentiate fully keeps rock
  mixed in its ice, which changes its layers, its shell and its ocean.
  The criterion (accretional heating, impacts, or both) is the
  builder's to choose from the literature, stated with its source.
- Ammonia: the ocean's liquidus follows its NH3 concentration
  (`Volatiles.nh3`, 1 % of the ice today), with a cited phase diagram.
- Geysers: `PlanetModel` gains the geysers, if any: active pole, and the
  fractures' positions, so that part B's map and viewer agree with each
  other and with the card. The criterion that a shell cracks through at
  the pole is cited.
- Plume ring: `rings::dust_rings` takes the geyser moon from the model
  instead of `PLUME_ODDS`.
- Reference: Enceladus joins `planet/reference.rs` so its archetype
  checks against a real body.
- Card: new fields and their cause trees, French and English.
- Formats: system format 9 (to agree with the peer sessions before
  merge). Stored systems are marked outdated.

### Part B: geysers on the map and in the viewer (second branch)

- Surface map: the active pole's young terrain carries parallel
  fractures (troughs flanked by ridges, as the tiger stripes: about 130
  km long, about 35 km apart, Porco et al. 2006,
  doi:10.1126/science.1123013) and fewer craters (resurfaced), at the
  positions part A stored. World format 25 or the next free number.
- Viewer, host view: a plume above the active pole of a moon in the sky,
  visible at the scale moons are drawn there, brightest when back-lit.
- Viewer, close-up: jets rising from the fractures, matching the map.
- `RENDER_VERSION` / `TILE_RENDER_VERSION` bumped if the tiles change.

## Order of work

1. Census first: a sweep of the five moon archetypes over many seeds and
   of drawn icy moons in whole systems, before and after, on the
   fields of the table above.
2. Part A physics, test first, one relation at a time.
3. Part A card, then the PR for part A, tested on `develop`.
4. Part B on its own branch from `main` after part A is merged.

## Acceptance

On every seed of its archetype:

| Moon | Shell | Ocean | Other |
|---|---|---|---|
| Enceladus-like | mean 15-40 km, polar under 10 km | yes | tidal heat 5-30 GW; geysers |
| Europa-like | 5-40 km | 50-200 km | geysers allowed, not required |
| Ganymede-like | 80-250 km | yes, over high-pressure ice | no geysers |
| Titan-like | 50-200 km | yes | no geysers |
| Callisto-like | thick | none, or under 50 km | partly differentiated; no geysers |

- Across whole systems, the share of icy moons with geysers is small and
  every one of them is in a resonance or has a high forced
  eccentricity; the census reports it.
- Icy planets and drawn icy moons stay coherent: the census compares
  shell and ocean distributions before and after, and the PR states
  the change.
- Part B: on the map, the fractures sit at the stored positions; in both
  views, the plume or jets sit above them (checked by
  `viewer-visual-check`).

## Calls made while writing this spec

These were settled by the main session after the builder check, not by
the user; the user reviews them with the spec.

- **Resonance partner.** Neighbouring moons of the `moons.rs` chain
  (`CHAIN_STEP`, periods doubling) are in a 2:1 resonance. An archetype
  moon declares its partner in `[moon]` (the outer moon's mass and the
  period ratio, e.g. Dione for Enceladus, Ganymede for Europa's
  Laplace chain): it is an orbit fact, like `host`, not a heat value.
  The model's context carries the partner next to `HostTide`.
- **Heat of a resonant moon.** In equilibrium, its tidal heat is set by
  the host's dissipation and the orbits (Meyer & Wisdom 2007), not by
  the moon's own k2/Q, so the shell and the heat are not circular. The
  moon's eccentricity stays an input shown on the card. The giants'
  single k2/Q (`ENVELOPED_K2_OVER_Q` 1.1e-5, Jupiter's) gives Enceladus
  about 1 GW only; Saturn's measured dissipation is about 20 times
  higher (Lainey et al. 2012, 2017). The host's dissipation must
  therefore come from a cited rule that separates them, such as
  resonance locking (Fuller, Luan & Quataert 2016,
  doi:10.1093/mnras/stw609), not from a per-planet value.
- **Differentiation driver.** It comes from what the model already
  knows of the moon: its mass, its distance from the host and the
  impact flux there (Zahnle 2003 focusing, #475), as in Barr & Canup
  2008. No new archetype input.
- **Partly differentiated body.** A mixed rock-and-ice layer under the
  shell, with its moment of inertia computed and shown on the card.
- **Geyser data.** The active pole and the fractures (centre latitude
  and longitude, azimuth, length, spacing) are stored in `PlanetModel`
  (system format). Part B passes them to world generation through
  `WorldParams`, the way `liquid_poleward_deg` is passed today, and the
  viewer reads them from the model.
- **Europa's geysers.** No odds and no threshold of their own: they
  appear when the drawn mass, ices and heat give a polar shell that
  passes the cracking criterion.
- **Ammonia.** It changes the liquidus of every icy body's ocean, planets
  included.
- **Coherence.** The census reports before/after distributions; the main
  session judges them (repo rule), with no fixed tolerance.
- **Formats.** Part A takes the next free system format, part B the
  next free world format, agreed with the peer sessions before each
  merge.

## Left to the builder

- The exact convective scaling, ice rheology and their constants, cited.
- The differentiation criterion, cited.
- The equilibrium heating formula's form for chains of more than two
  moons.
- The cracking criterion for geysers and the number of fractures.
- The look of plumes and jets (shader, particles or sprites), within the
  goals above.
