# Analysis: the host star

Backlog item 11. User request: research stars (types, composition,
colours, habitable zones), take the star into account when a planet is
generated, show the star in the viewer, aligned with the **Sunlight**
direction and moving with the star field when the planet is turned, and
offer a view of the planet in the colours it has under its own star next
to the usual ones. New data such as the distance to the star come with
it.

Today every world orbits an implicit Sun at 1 AU: the temperature
profile is Earth's (`climate::row_temperatures`, fitted to Earth's
annual zonal mean), shifted by the preset (`[climate] temperature`) and
the atmosphere (`greenhouse`). Mars-like and Venus-like worlds get their
cold and heat from those shifts, not from their distance to the Sun. The
viewer's sunlight is white, and no star is drawn: the star field
(`stars.ts`) is a fixed backdrop.

## Decisions (2026-11-04)

Taken with the user:

- **Random star:** drawn with the frequencies of the solar
  neighbourhood (about 76 % M, 12 % K, 8 % G, 3 % F, under 1 % A, B,
  O); the form lets the user pick any type instead.
- **Orbit:** each planet type sits where it makes sense around its
  star. Terran worlds are in the habitable zone; frozen worlds far
  beyond it, so their cold comes from the distance; volcanic worlds can
  be anywhere, far ones included, as their heat comes from internal
  activity rather than the star; the other types have their own ranges
  (section 2).
- **Viewer:** the star is drawn when **Sunlight** is on, at its true
  angular size (with a minimum of a few pixels) and a glow growing with
  its brightness.
- **Optional steps kept:** tidal locking of close-in planets, and
  vegetation colours adapted to the star (in the "Star light" view).

## 1. Research

### Spectral types (main sequence)

Stars are classed by their surface temperature (O B A F G K M, each
split 0–9; the Sun is G2V, `V` meaning a main-sequence dwarf). On the
main sequence, mass sets everything else: luminosity grows roughly as
`M^3.5–4`, the lifetime as `M / L ≈ 10 Gyr × M^−2.5`.

| Type | Teff (K)      | Mass (M☉) | Radius (R☉) | Luminosity (L☉) | Lifetime   | Share of stars | Colour (seen)        |
| ---- | ------------- | --------- | ----------- | --------------- | ---------- | -------------- | -------------------- |
| O    | > 30 000      | 16–90     | > 6.6       | > 30 000        | < 10 Myr   | ~0.00003 %     | blue                 |
| B    | 10 000–30 000 | 2.1–16    | 1.8–6.6     | 25–30 000       | 10–400 Myr | ~0.1 %         | blue-white           |
| A    | 7 300–10 000  | 1.4–2.1   | 1.4–1.8     | 5–25            | 0.4–2 Gyr  | ~0.6 %         | white                |
| F    | 6 000–7 300   | 1.04–1.4  | 1.15–1.4    | 1.5–5           | 2–7 Gyr    | ~3 %           | yellow-white         |
| G    | 5 300–6 000   | 0.8–1.04  | 0.96–1.15   | 0.6–1.5         | 7–15 Gyr   | ~7.6 %         | white (Sun: 5 772 K) |
| K    | 3 900–5 300   | 0.45–0.8  | 0.7–0.96    | 0.08–0.6        | 15–50 Gyr  | ~12 %          | pale orange          |
| M    | 2 300–3 900   | 0.08–0.45 | 0.1–0.7     | 0.0001–0.08     | > 50 Gyr   | ~75–78 %       | orange-red           |

Per-subtype values (Teff, mass, radius, luminosity) come from Pecaut &
Mamajek (2013) and Mamajek's maintained table, which the star model
interpolates. The shares are those of the solar neighbourhood (RECONS,
Mamajek): about three quarters of stars are M dwarfs, FGK stars about a
fifth, OBA under 1 %.

- **Composition.** By mass about 73 % hydrogen, 25 % helium, and 1–2 %
  heavier elements ("metals", `[Fe/H]` relative to the Sun). Metal-rich
  stars host giant planets more often; for rocky planets the effect is
  weak. Metallicity changes the colour and luminosity of a star of a
  given mass only slightly: it is worth a stored value, not a model.
- **Colour.** A star radiates nearly as a black body at its effective
  temperature. Seen by an eye adapted to daylight, a 5 800 K star looks
  white, 3 000 K deep orange (`#FFB46B`), 4 000 K pale orange, 7 000 K
  faintly blue, 10 000 K clearly blue (`#CAD7FF`). The colour comes from
  integrating Planck's law against the CIE 1931 colour matching functions
  into linear sRGB.
- **Activity.** M dwarfs flare often and strongly, and their young years
  are very active (UV and X-rays that can strip atmospheres). Not
  modelled; a note on the world card at most.
- **Ages.** A planet's surface cannot be older than its star; O, B and
  early A stars live too briefly for surfaces billions of years old (the
  geology's `surface_age`).

### Reference values (the star model)

From the star model (`crates/mapgen/src/star.rs`, step 0): the Pecaut &
Mamajek sequence, the Kopparapu zone and a planet in the middle of it
(position 0.5).

| Type | Teff (K) | L (L☉) | M (M☉) | Zone (AU)   | Middle (AU) | Year (days) | Star seen (°) | Locking (Gyr) | Light (RGB, Sun = 1) |
| ---- | -------- | ------ | ------ | ----------- | ----------- | ----------- | ------------- | ------------- | -------------------- |
| M5V  | 3 060    | 0.0030 | 0.162  | 0.057–0.113 | 0.081       | 21          | 1.30          | 0.002         | 1.00, 0.56, 0.20     |
| M3V  | 3 430    | 0.016  | 0.370  | 0.132–0.257 | 0.184       | 48          | 1.04          | 0.05          | 1.00, 0.64, 0.30     |
| M0V  | 3 850    | 0.069  | 0.570  | 0.271–0.517 | 0.374       | 111         | 0.84          | 1.4           | 1.00, 0.72, 0.42     |
| K5V  | 4 440    | 0.17   | 0.700  | 0.423–0.785 | 0.576       | 191         | 0.65          | 12            | 1.00, 0.82, 0.59     |
| K2V  | 5 100    | 0.37   | 0.820  | 0.602–1.085 | 0.808       | 293         | 0.52          | 69            | 1.00, 0.92, 0.80     |
| G2V  | 5 770    | 1.02   | 1.000  | 0.962–1.697 | 1.278       | 528         | 0.42          | 720           | 1.00, 1.00, 1.00     |
| F5V  | 6 550    | 3.6    | 1.330  | 1.729–3.002 | 2.278       | 1 089       | 0.34          | 13 000        | 0.82, 0.88, 1.00     |
| A0V  | 9 700    | 38     | 2.180  | 5.415–9.342 | 7.113       | 4 693       | 0.16          | huge          | 0.52, 0.66, 1.00     |

The light colour is the black body's, through the CIE 1931 colour
matching functions (the analytic fit of Wyman, Sloan & Shirley 2013)
into linear sRGB, divided by the Sun's: an eye adapted to sunlight sees
the Sun white. Planets of M dwarfs (and of early K dwarfs over billions
of years) lock; those of G and hotter stars do not.

### Habitable zone

The flux a planet receives, relative to Earth's, is `S = L / a²` (`L`
in L☉, `a` in AU). The conservative habitable zone of Kopparapu et al.
(2013, 2014) bounds `S` by stellar temperature, for 2 600 ≤ Teff ≤
7 200 K:

```text
Seff = Seff☉ + a·T + b·T² + c·T³ + d·T⁴,   T = Teff − 5 780 K
distance = sqrt(L / Seff) AU
```

| Boundary                   | Seff☉ | a        | b        | c          | d          |
| -------------------------- | ----- | -------- | -------- | ---------- | ---------- |
| Inner (runaway greenhouse) | 1.107 | 1.332e−4 | 1.580e−8 | −8.308e−12 | −1.931e−15 |
| Outer (maximum greenhouse) | 0.356 | 6.171e−5 | 1.698e−9 | −3.198e−12 | −5.575e−16 |

For the Sun this gives 0.95–1.68 AU; for a 3 000 K, 0.001 L☉ M dwarf,
0.033–0.066 AU. Cooler stars shift the zone to lower fluxes (their red
light warms planets more for the same flux: less Rayleigh scattering,
more absorption by water and CO₂). Outside 2 600–7 200 K the formula is
clamped and the world card marks the value as extrapolated.

### Consequences for a planet

- **Equilibrium temperature:** `T_eq ∝ S^(1/4)` (Earth: 255 K without
  greenhouse, 288 K mean surface with it). Mars (1.52 AU, S = 0.43) is
  ×0.81 colder in kelvin before its thin atmosphere; Venus (0.72 AU,
  S = 1.91) ×1.18 warmer before its greenhouse.
- **Year:** Kepler, `P = sqrt(a³ / M)` years. A habitable planet of an
  M dwarf has a year of days to weeks (11 days at 0.05 AU of a 0.12 M☉
  star).
- **Tidal locking:** the locking time grows as `a⁶ / M²`. Planets in the
  habitable zone of M dwarfs (and of late K dwarfs) are expected to be
  locked, one face always lit: an "eyeball" climate around the
  substellar point instead of latitude bands.
- **The star in the sky:** angular diameter `0.533° × R / a` (R in R☉,
  a in AU): the Sun 0.53° from Earth, 1.7° for Proxima Centauri from
  Proxima b.
- **Light and colours:** under a cooler star, sunlight is redder and
  dimmer; the sky scatters less blue. Photosynthesis may adapt (Kiang
  et al. 2007, Kiang & Hoehler 2021): around M dwarfs the vegetation's
  red edge would shift to the near infrared and leaves look dark (even
  near-black) to us; around F stars absorption peaks in the blue and
  plants may look yellow to orange. This is speculation, not a model.

## 2. What changes

### Generation

- **New parameters:** the star (a spectral type such as `G2V`, or an
  effective temperature, from which mass, radius and luminosity are
  interpolated) and the orbit (semi-major axis in AU). Defaults: the Sun
  at 1 AU, which keeps every existing world and preset as it is.
- **Orbit position:** a planet's place is given by its position `h`
  in the habitable zone of its star: 0 at the inner edge, 1 at the outer
  edge, below 0 closer in, above 1 farther out. The flux follows
  `S(h) = S_inner · (S_outer / S_inner)^h` (geometric, so it extends
  smoothly beyond the edges), and the distance is `sqrt(L / S)`. The
  same `h` gives the same climate whatever the star, which is what makes
  a planet type "logical" around any star; a distance in AU can still be
  given explicitly (CLI and form), and wins.
- **Presets:** a `[star]` table (type or a list of types to draw from,
  and an orbit range `position = [min, max]` or an explicit `orbit_au`),
  like `[atmosphere]` and `[geology]`:

  | Preset         | Star                 | Orbit                                                          |
  | -------------- | -------------------- | -------------------------------------------------------------- |
  | `terran`       | random (realistic)   | `position = [0.1, 0.9]`: inside the habitable zone             |
  | `frozen`       | random (realistic)   | `position = [1.5, 4.0]`: well beyond its outer edge            |
  | `volcanic`     | random (realistic)   | `position = [-0.5, 4.0]`: anywhere, its heat is internal       |
  | `caustic`      | random (realistic)   | `position = [-0.4, 0.4]`: warm, near the inner edge            |
  | `barren`       | random (realistic)   | `position = [-1.0, 4.0]`: anywhere, no water to care about     |
  | `earth-like`   | `G2V` (the Sun)      | `orbit_au = 1.0`                                               |
  | `mars-like`    | `G2V`                | `orbit_au = 1.524`                                             |
  | `venus-like`   | `G2V`                | `orbit_au = 0.723`                                             |
  | `mercury-like` | `G2V`                | `orbit_au = 0.387`                                             |

  The ranges are a first proposal, to be tuned with the climate step.
- **Internal heat:** volcanic worlds get `[climate] internal_heat`
  (°C), added after the insolation scaling: their surface stays
  scorching far from their star, where a world without it would freeze.
  It replaces the part of their warmth that is not greenhouse.
- **Climate:** the latitude profile is scaled in kelvin by `S^(1/4)`
  (a dimmer star cools the whole planet and narrows the equator-to-pole
  contrast in degrees), then the greenhouse and the preset shift apply
  as today. Presets are recalibrated so their worlds keep their
  temperatures and biome shares within a point or two, as was done for
  the °C change.
- **Stored data:** a `star` section in `world.json` (type, Teff, mass,
  radius, luminosity, metallicity, orbit in AU) and the derived values
  (insolation, habitable zone edges, equilibrium temperature, year,
  angular size, tidally locked or not). `raw_data` is unchanged; old
  worlds read as the Sun at 1 AU.

### Viewer

- **The star:** a disk with a glow at the sun direction of **Sunlight**
  (the direction the planet is lit from, set by the season and the sun
  longitude), drawn at infinity like the star field, so it stays aligned
  with the light and moves with the other stars when the planet is
  turned. Its colour is the star's; its size is its angular diameter,
  with a minimum on screen and a glow growing with its brightness.
- **"Star light" checkbox:** the planet lit by its star's light instead
  of white light. The direct light and the light entering the sky shell
  take the star's colour, white-balanced on the Sun (an Earth eye), so a
  Sun-like star changes nothing and a red dwarf turns the day side
  orange and the sky less blue. The base colours stay those of the
  usual maps (`world.png`, `surface.png`).
- **World card:** star type and temperature, luminosity, distance in AU,
  insolation (× Earth), position in the habitable zone (inside, too
  close, too far), equilibrium temperature, year length, apparent size
  of the star, tidal locking.
- **Form:** a star selector (type, with a "random" option) and the
  distance (AU, or the position in the habitable zone).

## 3. Plan

| Step | Change                                                                                                                 | Seeds  |
| ---- | ---------------------------------------------------------------------------------------------------------------------- | ------ |
| 0    | Star model (`star.rs`): spectral table, interpolation, black-body colour, habitable zone, derived values; tests        | Same   |
| 1    | `[star]` in presets, `--star` and `--orbit`, `star` in `world.json`, form fields and world card; no climate effect yet | Same   |
| 2    | Climate driven by the insolation; Mars-like, Venus-like and Mercury-like at their real distances; presets recalibrated | Change |
| 3    | Viewer: the star disk and glow at the sunlight direction, at infinity with the star field                              | Same   |
| 4    | Viewer: "Star light" checkbox, the planet and its sky lit by the star's colour                                         | Same   |
| 5    | Tidal locking of close-in planets: substellar climate instead of latitude bands, fixed sun in the viewer               | Change |
| 6    | Vegetation colours adapted to the star (Kiang), in the "Star light" view only                                          | Same   |

Steps 0, 1, 3, 4 and 6 change no world; step 2 changes the worlds of
presets whose star or distance differs from the Sun at 1 AU, and only
by the recalibration error for the others; step 5 changes locked worlds.

### Notes per step

- **Step 1:** random draws: the star first (realistic weights), then
  the orbit position from the preset's range, both from the world seed
  so a world reproduces. The CLI takes `--star <type>` and either
  `--orbit-au` or `--hz-position`; the form gets a star selector
  ("random" by default) and the orbit.
- **Step 2:** the frozen, volcanic and caustic shifts are re-derived so
  their worlds stay frozen, scorched and acid at their new distances;
  biome shares compared on the same seeds as for the °C change.
- **Step 5:** a planet is locked when its locking time (Gladman et al.
  1996 scaling, `∝ a⁶ / M²`, with Earth-like rigidity) is shorter than
  the star's age; the temperature then follows the angle to the
  substellar point (a hot day side, a cold night side, a terminator
  ring where water stays liquid), winds and rain follow that geometry,
  and the viewer's sun stops moving with the season.
- **Step 6:** biome colours of vegetated biomes get a tint by star type
  (dark red to near black under M dwarfs, unchanged under G, yellowish
  under F), applied by the "Star light" view only, so the usual maps and
  the exported files stay Earth-coloured.

## Outcome

### Steps 0 and 1

The star model (`crates/mapgen/src/star.rs`) and the star and orbit as
generation parameters: `--star`, `--orbit-au`, `--hz-position`, the
presets' `[star]` tables, `star` in `world.json`, the form and the world
card. Both are drawn after every other parameter, so every output but
`world.json` is unchanged.

### Step 2: climate

The insolation is taken relative to the model Sun at 1 AU (G2V, 1.023
L☉ in the table), so `earth-like` worlds are unchanged to the byte.
Planets with plate tectonics and liquid water are regulated across the
habitable zone (a carbonate-silicate cycle): +30 °C at its inner edge,
Earth's climate at Earth's position (0.068), −15 °C at its outer edge,
then the flux law beyond the edges; the others follow `T × S^(1/4)` in
kelvin; `internal_heat` adds as `T⁴`. Presets recalibrated, mean
temperature and the main biome shares on three seeds (512 × 256, same
parameters, before → after):

| Preset (place)                 | Mean T (°C)      | Biomes (%)                                     |
| ------------------------------ | ---------------- | ---------------------------------------------- |
| `earth-like` (Sun, 1 AU)       | 14.0 → 14.0      | identical                                      |
| `terran` (position 0.05)       | 14.0 → 22.0      | jungle 19 → 24, ocean 45 → 48                  |
| `terran` (0.4, any star)       | 14.0 → 8.6       | jungle 19 → 15, sea ice 3 → 5, ice cap 3 → 5   |
| `terran` (0.75)                | 14.0 → 3.0       | jungle 19 → 10, sea ice 3 → 8, ice cap 3 → 6   |
| `frozen` (1.0 to 1.4)          | −13 → −1 to −32  | ice cap 16 → 8 to 35, taiga 21 → 25 to 0       |
| `caustic` (−0.25 to 0.1)       | 18 → 45 to 15    | unchanged within 1                             |
| `volcanic` (−0.5 to 4)         | 34 → 116 to 31   | unchanged within 2                             |
| `barren` (−1 to 4)             | 14 → 113 to −191 | dust plain 39 → 35 to 39, mountain 7 → 6 to 10 |
| `mars-like` (Sun, 1.524 AU)    | −46 → −51        | identical                                      |
| `venus-like` (Sun, 0.723 AU)   | 464 → 460        | identical                                      |
| `mercury-like` (Sun, 0.387 AU) | 14 → 169         | identical (no water, no ice)                   |

A regulated planet gets the same climate at the same position around
any star (M3V and G2V at 0.4 give the same world). Ranges kept:
`terran` 0.05–0.75, `frozen` 1.0–1.35 (its cold now comes from the
distance, its −27 °C shift is gone), `caustic` −0.25–0.1, `volcanic`
−0.5–4 with `internal_heat = 285`, `barren` −1–4. Mars keeps a −10 °C
shift (thin air, no greenhouse), Venus's greenhouse drops from 450 to
395 °C, Mercury gets −20 °C.

### Steps 3 and 4: the star in the viewer

With Sunlight on, the star is a point at infinity at the sun's
direction, like the star field: a disk of its apparent diameter (at
least 3 px), slightly darker at its limb, in its light's colour, in a
glow growing with the light the planet gets. "Star light" multiplies
the direct light, the clouds, the sea glint and the light entering the
sky shell by the star's colour (the Sun white): under an M3 dwarf the
day side turns ochre and the halo loses its blue.

### Step 5: tidal locking

A planet is locked when its locking time is shorter than its star's age
(4.6 Gyr or its lifetime): at the middle of their habitable zones, M
dwarfs' planets lock (M0V: 1.4 Gyr), K5V's do not (12 Gyr). Its
temperature follows the angle θ from the substellar point, at the map's
centre: `4 cos θ` times the mean light on the day side (none at night),
through Earth's scale and curve (34 °C under the star, −56 °C on the
night side), the atmosphere's heat transport over the sphere, then the
star. An M4V planet at 0.3 in its zone shows an open ocean and green
continents around the substellar point inside a frozen night side. The
viewer keeps its sun above the map's centre. Winds and clouds keep
their latitude bands: a substellar circulation is left for later.

## 4. Open points

- Locked planets keep latitude winds, rain and clouds; a substellar
  circulation (rising air and clouds under the star, winds towards it)
  would make their day sides cloudier and their terminators wetter.

- Multiple stars (binaries are common, especially among massive stars),
  eccentric orbits and their seasons, stellar evolution (a brightening
  star, giants) and flares: not planned.
- Planetary albedo: the scaling uses Earth's; an icy world reflects
  more and would be colder. The preset shift can carry it.

## Sources

- Kopparapu et al. 2013, "Habitable Zones around Main-sequence Stars:
  New Estimates", ApJ 765, 131; Kopparapu et al. 2014, ApJL 787, L29
  (coefficients, as in the authors' `hzcalc.py`:
  <https://personal.ems.psu.edu/~jfk4/ruk15/planets/hzcalc.py>).
- Pecaut & Mamajek 2013, ApJS 208, 9; E. Mamajek, "A Modern Mean Dwarf
  Stellar Color and Effective Temperature Sequence":
  <https://www.pas.rochester.edu/~emamajek/EEM_dwarf_UBVIJHK_colors_Teff.txt>.
- Number densities of stars by type in the solar vicinity (Mamajek):
  <https://www.pas.rochester.edu/~emamajek/memo_star_dens.html>;
  RECONS, "The Solar Neighborhood XLV" (Winters et al. 2019):
  <https://arxiv.org/abs/1901.06364>.
- Kiang et al. 2007, "Spectral Signatures of Photosynthesis II:
  Coevolution with Other Stars and the Atmosphere on Extrasolar
  Worlds", Astrobiology 7, 252: <https://arxiv.org/abs/astro-ph/0701382>.
- Kiang & Hoehler 2021, photon absorption of photosynthetic organisms
  by stellar type (Frontiers in Astronomy and Space Sciences): peak
  absorption in the blue around F stars, in the red or just beyond
  around G, K and early M stars.
- Gladman et al. 1996, "Synchronous Locking of Tidally Evolving
  Satellites", Icarus 122, 166 (the locking time scaling).
- Wyman, Sloan & Shirley 2013, "Simple Analytic Approximations to the
  CIE XYZ Color Matching Functions", JCGT 2(2).
- IAU 2015 Resolution B3 (nominal solar values: Teff = 5 772 K).
