# Why do gas giants have bands?

Seen from: the planet view of a giant world (the `gas-giant`, `ice-giant`
and `hot-jupiter` presets): its banded globe, the **Clouds** and **Winds**
rows of its **Info** panel, the **Wind** and **Cloud tops** layers, and the
**Band drift** switch.

## Short answer

A giant planet spins fast, and on a fast-spinning sphere large eddies cannot
grow freely in the north–south direction: the turbulence organises itself
into east–west jets. The faster and larger the planet, and the slower its
winds, the more jets it has. Between the jets the air rises in some bands
and sinks in others. Where it rises, it carries ammonia up into high white
clouds (the zones); where it sinks, the air is dry and we see deeper, darker
clouds (the belts). Each gas condenses at its own depth, so the colour of a
giant depends on its temperature: cream and brown with ammonia clouds, blue
when methane absorbs the red light, white, azure or nearly black on hotter
giants.

## The science

![Jupiter-like belts and zones on a globe, with a jet on every band edge, and the northern bands seen edge-on, air rising in the zones and sinking in the belts](images/gas-giant-bands-circulation.svg)

### Jets: what a fast spin does to turbulence

On a rotating planet, the Coriolis effect depends on latitude. Its change
with latitude $\varphi$, for a planet of radius $a$ turning at the angular
rate $\Omega$, is $\beta = 2\Omega\cos\varphi / a$. Eddies smaller than a
certain size behave like ordinary turbulence. Larger ones turn into waves
that travel east and west, and their energy piles up in zonal jets. That
size is the Rhines scale, for winds of speed $U$ (Rhines 1975; Ingersoll
et al. 2004):

```math
L_\beta = \left(\frac{U}{\beta}\right)^{1/2},
\qquad
\beta = \frac{2\Omega\cos\varphi}{a}
```

**In words.** Jets are about as wide as the Rhines scale. A faster spin or
a smaller planet (a larger $\beta$) makes the jets narrower; faster winds
make them wider. Rhines (1975) showed that zonal jets grow out of decaying
turbulence this way. In shallow-water models of the giants on a sphere,
zonal jets "are verified to scale closely with the Rhines scale" (Scott &
Polvani 2007).

**Orders of magnitude.** Jupiter turns in 9.925 hours and has an
equatorial radius of 71,492 km (NASA fact sheet). With $U = 100$ m/s, a
typical speed of its jets, many of which "exceed 100 m s⁻¹" (Ingersoll
et al. 2004), $L_\beta$ is about 4,500 km at the equator, 3.6° of latitude.

Counting how many jets of that width fit from pole to pole gives the number
of jets (Showman et al. 2010, eq. 39):

```math
N_\text{jet} \sim \left(\frac{2\Omega a}{U}\right)^{1/2}
```

**In words.** The number of jets grows as the square root of the spin and
of the size, and falls as the square root of the wind speed. Rapidly
rotating planets "will tend to exhibit numerous strongly banded jets,
whereas slowly rotating planets will exhibit fewer jets with weaker
banding" (Showman et al. 2010).

**Orders of magnitude.** For Jupiter, the formula gives about 16 jets; for
Neptune (16.11 hours, 24,764 km, winds of 400 m/s), about 4. Jupiter and
Saturn each have about 20 jets, Uranus and Neptune three broad ones, with
peak speeds from about 150 m/s on Jupiter to over 400 m/s on Saturn and
Neptune (Showman et al. 2010). On the ice giants the equatorial jet blows
westward, against the spin, and the two high-latitude jets eastward; in the
shallow-water models, equatorial jets become "increasingly and consistently
retrograde" as the deformation radius shrinks (Scott & Polvani 2007).

### Belts and zones: where the air rises and sinks

On Jupiter, "the white bands are called zones and the dark bands are called
belts", and the jets are strongest on the boundaries between them
(Ingersoll et al. 2004). The zones are anticyclonic: in the northern
hemisphere they have an eastward jet on their poleward edge and a westward
jet on their equatorward edge. The belts turn the other way. The
Equatorial Zone lies between the eastward jets at ±7° of latitude.

The traditional view is that the air in the zones is slowly rising and the
air in the belts slowly sinking (Ingersoll et al. 2004). Several
observations agree with it:

- the visible cloud deck is higher in the zones and lower in the belts
  (Ingersoll et al. 2004), or at least the zones carry thicker high clouds
  and haze (Sudarsky et al. 2000);
- the belts hold less ammonia and thinner clouds, and the holes in the
  clouds that let heat through from below (the 5-µm hot spots) lie in the
  belts (Ingersoll et al. 2004);
- "the belts represent relatively dry, downwelling regions" (Atreya &
  Wong 2005).

The colours do not come from the height alone. The colour differences of
belts and zones are thought to come mostly from "chromophores", coloured
compounds probably made by sunlight in the upper atmosphere, within the
ammonia cloud deck and seen better where the clouds and haze are thinner
(Sudarsky et al. 2000). What they are is not known. For
the Great Red Spot, the products of ammonia and acetylene under ultraviolet
light match its colour in the laboratory (Carlson et al. 2016).

### Clouds at different depths

![The base of each cloud deck of Jupiter, Saturn and Neptune by pressure, on a log scale](images/gas-giant-bands-clouds.svg)

Rising air cools. Each gas condenses where its partial pressure reaches its
saturation pressure, which sets the base of its cloud. For a giant of the
Sun's composition the decks stack, from the top: ammonia ice, ammonium
hydrosulphide (NH₄SH, from ammonia and hydrogen sulphide) and water. In
Jupiter's equilibrium cloud models, ammonia condenses at 0.72 bar (0.84 bar
for three times the Sun's nitrogen), NH₄SH near 2 bar and water at 5.7 bar
for the Sun's oxygen, 5 to 8 bar for one to three times it (Atreya & Wong
2005).

**In words.** On a colder planet each gas reaches its condensation
temperature deeper down, so the same clouds form at higher pressures.

**Orders of magnitude.** Saturn is colder than Jupiter, so its decks sit
deeper: NH₄SH near 5 bar, water at 12.6 bar for the Sun's oxygen, 21 bar
for five times it (Atreya & Wong 2005). On Neptune, colder still, the top
cloud at about 1 bar is methane ice, and water condenses only near 40 bar.
The winds measured by tracking clouds come from the levels at 0.5 to 4 bar
(Showman et al. 2010).

The clouds end at the tropopause, the coldest level of the atmosphere. On
Jupiter the Voyager radio occultations put it near 140 mbar, at 110 K, with
165 ± 5 K at 1 bar (Lindal et al. 1981); the Galileo probe found it at
260 mbar (Seiff et al. 1998).

### Why ice giants are blue

Uranus and Neptune hold far more methane than Jupiter. Methane absorbs red
light. In the words of Atreya & Wong (2005), "the aqua-marine color of
Uranus and Neptune ... is a direct consequence of the scattering of the
Sun's bluish-green light by their atmospheres in which the Sun's red light
is absorbed effectively by the large quantities of methane", and their white
clouds are methane ice.

Neptune is often shown deep blue and Uranus pale cyan. The difference is
smaller than those images suggest: Uranus is less blue, because of a
thicker haze, but "it is much less significant than is commonly believed due
to a long-standing misperception of Neptune's 'true' colour" (Irwin et al.
2024).

### Storms

Long-lived vortices sit in the bands. The Great Red Spot is an anticyclone,
a high-pressure centre, from 17°S to 27.5°S. Its oval shape appears in
drawings from 1831. It was 39,000 km long from east to west in 1880 and
17,000 km around 2000 (Ingersoll et al. 2004), and it keeps shrinking: by
0.194° of longitude a year in length and 0.048° a year in width (Simon et
al. 2018). Its 2017 length in their fit, about 14° of longitude, is still
some 16,000 km at its latitude, more than Earth's diameter. It drifts
westward, by about 0.36° of longitude a day (Simon et al. 2018). Many
smaller
anticyclones, the white ovals, sit in the zones (Ingersoll et al. 2004).

### Hotter giants: the cloud classes

Close to a star, a giant is too warm for some of these clouds. Sudarsky,
Burrows & Pinto (2000) sorted giant planets by their effective temperature
$T_\text{eff}$, the temperature at which they radiate, into five classes by
the clouds that form high in their atmosphere:

| Class | $T_\text{eff}$ | Top clouds | The light |
| --- | --- | --- | --- |
| I, "Jovian" | ≲ 150 K | ammonia ice | methane absorption |
| II, "water cloud" | ~250 K | water ice | highest visible albedo |
| III, "clear" | ≳ 350 K | none high up | sodium and potassium absorb |
| IV | 900–1500 K | silicates, deep | lowest albedo |
| V | ≳ 1500 K | silicates, high | far brighter than IV |

A class III giant reflects much of the blue light, with a spherical albedo
up to about 0.6, but under a tenth of the red (Sudarsky et al. 2000): it
would look blue. Class IV giants are as dark as the darkest bodies of the
Solar System. In class V, or in giants of low surface gravity (below
10 m/s²), the silicate cloud forms high enough to reflect much of the light
before the alkali metals absorb it.

### The heat a giant gives off

A giant radiates both the starlight it absorbs and heat from its interior,
left over from its formation. Jupiter absorbs $6.613 \pm 0.160$ W/m² and
makes $7.485 \pm 0.160$ W/m² of its own (Li et al. 2018), so it radiates
about 14.1 W/m², an effective temperature of about 125.6 K. The interior
cools with time and a heavier giant keeps more heat. In the evolution tables
of Baraffe et al. (2003), an object of 0.01 solar masses (10.5 Jupiter
masses) has an effective temperature of 953 K at 0.1 Gyr, 491 K at 1 Gyr
and 322 K at 5 Gyr; one of 0.001 solar masses (1.05 Jupiter masses) has
160 K at 1 Gyr.

**In words.** A heavy, young giant glows from within, wherever it orbits.
For one far from its star, this internal heat alone decides the cloud
class.

### What we do not know

- **How deep the bands go.** Juno's microwave radiometer sees the banded
  structure throughout the troposphere, but the contrast between belts and
  zones changes sign between 4 and 10 bar, near the water clouds (Fletcher
  et al. 2021). The simple picture of rising zones and sinking belts may
  hold only above that level. In the upper troposphere the observed
  circulation, with cold air rising, must itself be driven mechanically
  (Ingersoll et al. 2004).
- **How high the zones' clouds stand.** Some analyses find no appreciable
  difference in altitude between belts and zones, only thicker high clouds
  and haze over the zones (Sudarsky et al. 2000, citing Chanover et al.
  1997).
- **Which wind speed sets the scale.** The jet count depends on $U$, and a
  giant's winds span a wide range. With Saturn's 400 m/s equatorial jet the
  formula gives about 7 jets, a third of what Saturn shows. Nor do the winds
  follow the energy: Neptune "has
  significantly stronger winds than Jupiter ... despite absorbing only 4%
  the solar flux absorbed by Jupiter", and "This is not well understood"
  (Showman et al. 2010).
- **Why the equatorial jet goes one way or the other.** It blows eastward
  on Jupiter and Saturn, westward on Uranus and Neptune. Shallow-water
  models get both, depending on their parameters (Scott & Polvani 2007).
- **What colours the clouds.** The chromophores are still unidentified
  (Sudarsky et al. 2000; Carlson et al. 2016).

## In the app

### What you see

A giant world is all clouds. Its globe shows pale zones and dark belts from
an equatorial zone to the polar regions, wavy edges where neighbouring jets
shear, white ovals, and on some a great spot (gas giants) or a dark spot
with bright companion clouds (ice giants). With **Band drift** on, each band
turns at the pace of its own wind, some backwards; the great spot barely
moves. The **Info** panel gives its cloud class (**Clouds**: "ammonia
clouds, banded cream and brown", "water clouds, bright white", and so on)
and **Winds**: the eastward jets in each hemisphere and the strongest
westward and eastward winds. The **Wind** layer shows the eastward wind, and
**Cloud tops** the height of the clouds, "deep belts" to "high storms". The
star view draws giants not yet generated as spheres with bands in the same
colours, between the band edges they will be generated with.

### How it computes them

[`giant_world.rs`](../../crates/mapgen/src/giant_world.rs) draws the look
of a giant (`Look::draw`), then samples it on every pixel:

1. **The cloud class.** The effective temperature combines the starlight
   (its equilibrium temperature) and the heat of the interior:
   $T_\text{eff} = (T_\text{eq}^4 + T_\text{int}^4)^{1/4}$.
   `internal_temperature` in
   [`giants.rs`](../../crates/mapgen/src/giants.rs) cools a gas giant with
   its mass and the age of the system (4.6 Gyr, or the lifetime of a
   shorter-lived star):

   ```math
   T_\text{int} = 107.2\,\text{K}
   \left(\frac{M}{M_J}\right)^{0.5}
   \left(\frac{t}{4.6\,\text{Gyr}}\right)^{-0.29}
   ```

   The exponents are a least-squares fit, in the logs, to the tables of
   Baraffe et al. (2003) for 1 to 10.5 Jupiter masses and 0.5 to 10 Gyr
   (within 10 % of every entry); the constant is Jupiter's 7.485 W/m²
   (Li et al. 2018). Ten Jupiter masses give 340 K at 4.6 Gyr (Baraffe:
   322 K at 5 Gyr), about 1,000 K at 0.1 Gyr (953 K) and 530 K at 1 Gyr
   (491 K). An ice giant keeps 40 K.

   [`giant_clouds.rs`](../../crates/mapgen/src/giant_clouds.rs) turns
   $T_\text{eff}$ into a class (`Clouds::of`). Classes I to III come from
   cloud decks. The temperature profile is a dry adiabat,
   $T = 1.32\,T_\text{eff}\,P^{2/7}$ with $P$ in bar (1.32 is Jupiter's
   165 K at 1 bar over its $T_\text{eff}$). A deck forms where its gas
   reaches saturation: ammonia, ammonium hydrosulphide and water, each with
   the Sun's nitrogen, sulphur and oxygen (Asplund et al. 2009). A deck
   that still reaches the tropopause, taken at 0.085 bar, is a top deck:
   ammonia makes class I (below about 205 K), water class II (to 350 K),
   neither class III (to 900 K). Then IV to 1,500 K and V above, or V above
   900 K when the gravity is under 10 m/s². Near a bound the top deck is
   thin and the palette blends into the next class's, so no bound is a step.
   Each class, and the kind of giant, has its palette of zone, belt, storm
   and polar colours. A cold ice giant gets cyan to deep blue; an ice giant
   of class II or III keeps a tinge of methane (red × 0.85); class V is
   pale.
2. **The jets.** The spin is drawn (9 to 12 hours for a gas giant, 14 to 19
   for an ice giant; a hot giant, or one its star's tides have locked, turns
   once a year). The jets in each hemisphere follow the Rhines count,
   $0.45\sqrt{\Omega R/U}$, between 1 and 8, with an eddy speed $U$ that
   scales with the energy the giant radiates, $F = \sigma T_\text{eff}^4$,
   and with $\beta = 2\Omega/R$:
   $U = U_0\,(F/F_0)^{2/5}\,(\beta/\beta_0)^{-1/5}$. The reference is
   Jupiter's (100 m/s, 13.7 W/m²) for a gas giant and an ice giant's
   (300 m/s, a $T_\text{eff}$ of 60 K, Neptune's $\beta$) for an ice giant.
   A hot giant takes 2,000 m/s scaled by $T_\text{eq}/1500$ K.
3. **The bands.** An equatorial zone (±6° to 11° on a gas giant, ±18° to
   28° on an ice giant), then $2J-1$ bands in each hemisphere for $J$
   jets, belts and zones in turn, narrower towards the poles, up to the
   polar regions (58° to 70° on a gas giant, 62° to 74° on an ice giant).
4. **The winds.** The peak wind is set separately from the count, by a
   calibration on the four giants of the Solar System: the Rossby number
   $U/\Omega R$ of a gas giant is
   $0.0119\,(7.485\,\text{W/m}^2/F_\text{int})^{0.5}$ (Jupiter's 150 m/s;
   the app's Saturn analogue gets 410 to 510 m/s), that of an ice giant is
   0.11 (about 290 m/s on Neptune). A giant warm from its star has at least
   $2000\,(T_\text{eq}/1500\,\text{K})^2$ m/s, and no wind in the whole
   profile passes 0.75 times the speed of sound at $T_\text{eff}$ (the
   profile is scaled down when it would). Then:
   - A gas giant gets a jet on every band edge, eastward on the poleward
     edge of a zone, westward on its equatorward edge, each 0.3 to 0.7 of
     the peak and weaker towards the poles, and an eastward equatorial jet
     of 0.9 to 1.2 of the peak.
   - An ice giant with one jet gets Neptune's profile from Voyager 2,
     $u(\varphi) = -398 + 0.188\,\varphi^2 - 1.2\times10^{-5}\varphi^4$ m/s
     (Sromovsky et al. 1993, as tabulated by Fitzpatrick et al. 2014),
     scaled to the peak wind: westward at the equator, eastward beyond
     about 50°. With two or more jets it gets the band-edge jets and a
     westward equatorial jet.
   - A hot giant gets one broad eastward jet at the equator.
5. **Each pixel.** The latitude it samples is shifted by noise stretched
   along the bands, more on the band edges. Storms swirl the bands around
   them. Zones are high and cool, belts low and 6 K warmer at the cloud
   tops. The height difference is proportional to the scale height times
   the log of the ratio of the base pressures of the top deck and the one
   below, relative to Jupiter's.

The jets, trimmed (from `crates/mapgen/src/giant_world.rs`):

```rust
const JUPITER_WIND: f64 = 100.0; // m/s, Jupiter's eddies
const JUPITER_FLUX: f64 = 13.7; // W/m2 radiated
const JET_FACTOR: f64 = 0.45;
const MAX_MACH: f64 = 0.75;

// Eddy speed, from the energy radiated and the spin.
let flux = 5.670_374e-8 * f64::from(effective_k).powi(4);
let beta = 2.0 * omega / radius_m;
let (u0, f0, b0) = match kind {
    GiantKind::Gas => (JUPITER_WIND, JUPITER_FLUX, JUPITER_BETA),
    GiantKind::Ice => (ICE_WIND, ICE_FLUX, ICE_BETA),
};
let eddies = (u0 * (flux / f0).powf(0.4) * (beta / b0).powf(-0.2))
    .clamp(20.0, 3000.0);
let jets = ((JET_FACTOR * (omega * radius_m / eddies).sqrt()).round() as u32)
    .clamp(1, 8);

// Peak wind: a Rossby number, bounded by the speed of sound.
let law = match kind {
    GiantKind::Gas => {
        let flux = 5.670_374e-8 * f64::from(internal_k).powi(4);
        GAS_ROSSBY * (JUPITER_INTERNAL_FLUX / flux).powf(ROSSBY_EXPONENT)
            * omega * radius_m
    }
    GiantKind::Ice => ICE_ROSSBY * omega * radius_m,
};
let ceiling = MAX_MACH * sound_speed(f64::from(effective_k));
law.max(irradiated).min(ceiling).clamp(20.0, 3000.0)
```

With Jupiter's spin and radius the count gives five jets in each
hemisphere, counted as the eastward ones: the wind profile has nine
alternating jets in each hemisphere and one at the equator, 19 in all, close
to Jupiter's "∼20". A Saturn-like giant (95 Earth masses, 9.5 AU) gets five
to seven eastward jets, 19 to 27 in all, like Saturn's "about 20". Neptune's
spin and size give one, sometimes two.

The storms are drawn with the bands. A great spot on 45 % of gas giants
sits in a zone of the southern hemisphere between 12° and 45° of latitude,
chosen at random (the Great Red Spot lies at 17° to 27.5° S). It is up to
29° of longitude long and is shrunk to fit a narrow zone, never under 0.7
of its size, so that it stays the largest storm. Then two to eight white
ovals between 20° and 50°, and a dark spot with bright companion clouds
at 18° to 45° S on half the ice giants. Hot giants get none.

The viewer moves each band with its wind, read from `wind_profile` every
2° of latitude (`giantColour` in
[`shaders.ts`](../../web/src/lib/viewer/shaders.ts)). The fastest band goes
round in two minutes, the others in proportion. Each storm drifts whole at
the wind of its centre, except the great spot (`world.json` stores its
longitude and latitude in `great_spot_at`, and the viewer takes the storm
nearest to it). It drifts westward at 3.5 % of the fastest band, the ratio
of the real spot's 0.36° a day (Simon et al. 2018) to the 10.4° a day of
Jupiter's 150 m/s. A giant not generated yet is drawn from its
preview ([`previewTexture.ts`](../../web/src/lib/viewer/previewTexture.ts)):
the zone, belt and polar colours between the band edges the generated giant
will have, in linear colours like the globe's. A system saved before the
edges were stored gets evenly spaced bands.

### Where it departs from the science

- **The jet count rests on a scaling derived for the app.** The eddy speed
  $U$ comes from an energy balance (a share of the radiated flux feeds the
  eddies, carried at the Rhines scale), not from a published law. It is
  normalised to Jupiter, with an ice-giant reference that is a calibration,
  no source. It gives the counts of Jupiter and Neptune, and a Saturn-like
  giant a count in line with Saturn's. But the count uses an eddy speed
  of about 100 m/s on Jupiter and the peak wind another, 150 m/s, so
  the two are not the one speed $U$ of the formula above.
- **The peak wind is a calibration, not a result.** Showman et al. (2010)
  call the strong winds of Neptune and Saturn, on less energy than
  Jupiter's, not understood, and the app does not explain them: it fits the
  four giants. The Rossby numbers (0.0119 with the internal flux, 0.11 for
  ice giants), the irradiation floor and the 0.75 sound-speed ceiling have
  no source. With Saturn's real internal flux the law would give about
  230 m/s, a third under its winds. Uranus comes out at about 290 m/s
  against 200. A Saturn-like giant's westward flank (-130 to -170 m/s) is
  stronger than the real one.
- **Every ice giant has Neptune's profile.** Uranus's eastward peak is the
  larger and its westward flow weaker (no readable table of its profile was
  found).
- **Odd corners of the calibration.** A giant that tidal locking stops
  (the rule locks a Jupiter-mass giant at 0.5 AU of a G2V star, which is
  doubtful) loses its Rossby wind and keeps the irradiation floor, a drop
  of several hundred m/s. A super-Jupiter (10 Jupiter masses at 40 AU)
  blows at only 20 to 30 m/s, because its large internal flux gives a tiny
  Rossby number. A hot giant's superrotation, 2,000 m/s scaled by the
  equilibrium temperature, has no source either.
- **The bands are colours, not cloud layers.** Zones and belts are pale and
  dark colours, with cloud tops 0.06 elevation units above or below the deck,
  scaled by the relief of the decks. The decks set the class, the blend of
  colours and the relief, but no cloud is drawn at its depth, nor the thinner
  clouds that let the deeper ones show in the belts. The palettes are chosen
  by eye for each class.
- **The decks are a rough model.** The composition is the Sun's: no
  methane deck on an ice giant, no enrichment, no water near 40 bar. The
  profile is a dry adiabat from a 1-bar temperature fixed at 1.32 times
  $T_\text{eff}$, and the tropopause pressure, 0.085 bar, is a knob set so
  that water leaves at 350 K (the profile puts Jupiter's tropopause at
  81 K, against 110 K). The saturation laws of ammonia and of NH₄SH are
  not read from the original papers: the NH₄SH one is Lewis (1969) as
  quoted by Li, Ingersoll & Oyafuso (2018), extended in a straight line outside
  180 to 300 K, and both are checked only by the bases they give Jupiter.
  The relief of classes IV and V (0.5) and the 100 K blends are
  calibrations.
- **Class boundaries.** The model puts the end of class I at about 205 K,
  between Sudarsky et al.'s "≲ 150 K" and "~250 K"; class II ends at 350 K
  as they have it. A 300 K giant is white and thinning towards clear, not
  a step.
- **Class V below 1,500 K.** Sudarsky et al. (2000) lower the temperature of
  class V for low gravity but give no number; 900 K is the app's.
- **The methane tinge is a share.** An ice giant of class II or III keeps
  the gas giant palettes with red × 0.85 and green × 0.97, a calibration
  that follows the qualitative sources only (Atreya & Wong 2005; Sudarsky
  et al. 2000).
- **The internal heat is a fit.** The cooling law follows Baraffe et al.
  (2003) for 1 to 10.5 Jupiter masses, so Saturn comes out at 59 K, under
  its real internal heat (helium rain is not modelled), and ice giants keep
  a fixed 40 K, a calibration, against the very different heat of Uranus and
  Neptune. The Jupiter references use 13.7 W/m² radiated and a $T_\text{eff}$
  of 124.7 K; adding Li et al.'s absorbed and internal heat gives 14.1 W/m²
  and 125.6 K, 3 % apart.
- **Only the cloud tops.** The app has nothing below them, so the reversal
  Juno sees below 4 to 10 bar never shows. It would matter only in a view of
  the depths, which the app does not have.
- **The Great Red Spot.** It sits in a southern zone, not on a real band,
  and it does not shrink with time (the real one loses 0.19° of longitude a
  year); the viewer has no date. The map's copy of the spot is hidden by a
  hole that still follows the band wind while the spot drifts slowly, so a
  patch of plain band rides along the zone (not checked in a browser). The
  dark spot of an ice giant is not tied to a band either.
- **Band drift is a sight.** The fastest band turns in two minutes; the
  relative speeds are kept, not the true pace. The help text says so.

## Sources

1. Rhines, P. B. 1975, "Waves and turbulence on a beta-plane", *Journal of
   Fluid Mechanics* 69, 417–443,
   [doi:10.1017/S0022112075001504](https://doi.org/10.1017/S0022112075001504).
2. Scott, R. K. & Polvani, L. M. 2007, "Forced-dissipative shallow-water
   turbulence on the sphere and the atmospheric circulation of the giant
   planets", *Journal of the Atmospheric Sciences* 64, 3158–3176,
   [doi:10.1175/JAS4003.1](https://doi.org/10.1175/JAS4003.1).
3. Showman, A. P., Cho, J. Y-K. & Menou, K. 2010, "Atmospheric circulation
   of exoplanets", in Seager, S. (ed.), *Exoplanets*, University of Arizona
   Press, 471–516, preprint
   [arXiv:0911.3170](https://arxiv.org/abs/0911.3170). The jet count
   (eq. 39), the jets and wind speeds of the four giants, the pressures of
   cloud tracking.
4. Ingersoll, A. P. et al. 2004, "Dynamics of Jupiter's atmosphere", in
   Bagenal, F., Dowling, T. E. & McKinnon, W. B. (eds), *Jupiter: The
   Planet, Satellites and Magnetosphere*, Cambridge University Press,
   105–128,
   [CaltechAUTHORS](https://authors.library.caltech.edu/records/8wqax-apm75).
   Belts and zones, the Rhines scale, rising zones and sinking belts, the
   Great Red Spot.
5. Atreya, S. K. & Wong, A. S. 2005, "Coupled clouds and chemistry of the
   giant planets: a case for multiprobes", *Space Science Reviews* 116,
   121–136,
   [doi:10.1007/s11214-005-1951-5](https://doi.org/10.1007/s11214-005-1951-5).
   Cloud bases of Jupiter, Saturn and Neptune; the colour of the ice giants.
6. Sudarsky, D., Burrows, A. & Pinto, P. 2000, "Albedo and reflection
   spectra of extrasolar giant planets", *The Astrophysical Journal* 538,
   885–903, [doi:10.1086/309160](https://doi.org/10.1086/309160), preprint
   [arXiv:astro-ph/9910504](https://arxiv.org/abs/astro-ph/9910504).
7. Fletcher, L. N. et al. 2021, "Jupiter's temperate belt/zone contrasts
   revealed at depth by Juno microwave observations", *Journal of
   Geophysical Research: Planets* 126, e2021JE006858,
   [doi:10.1029/2021JE006858](https://doi.org/10.1029/2021JE006858),
   preprint [arXiv:2110.14620](https://arxiv.org/abs/2110.14620).
8. Irwin, P. G. J. et al. 2024, "Modelling the seasonal cycle of Uranus's
   colour and magnitude, and comparison with Neptune", *Monthly Notices of
   the Royal Astronomical Society* 527, 11521–11538,
   [doi:10.1093/mnras/stad3761](https://doi.org/10.1093/mnras/stad3761).
9. Simon, A. A. et al. 2018, "Historical and contemporary trends in the
   size, drift, and color of Jupiter's Great Red Spot", *The Astronomical
   Journal* 155, 151,
   [doi:10.3847/1538-3881/aaae01](https://doi.org/10.3847/1538-3881/aaae01).
10. Carlson, R. W. et al. 2016, "Chromophores from photolyzed ammonia
    reacting with acetylene: application to Jupiter's Great Red Spot",
    *Icarus* 274, 106–115,
    [doi:10.1016/j.icarus.2016.03.008](https://doi.org/10.1016/j.icarus.2016.03.008).
11. NASA Space Science Data Coordinated Archive, [Jupiter fact
    sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/jupiterfact.html)
    and [Neptune fact
    sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/neptunefact.html)
    (radii, rotation periods).
12. Li, L. et al. 2018, "Less absorbed solar energy and more internal heat
    for Jupiter", *Nature Communications* 9, 3709,
    [doi:10.1038/s41467-018-06107-2](https://doi.org/10.1038/s41467-018-06107-2).
    Absorbed power 6.613 W/m², internal heat 7.485 W/m².
13. Baraffe, I., Chabrier, G., Barman, T. S., Allard, F. & Hauschildt, P. H.
    2003, "Evolutionary models for cool brown dwarfs and extrasolar giant
    planets. The case of HD 209458", *Astronomy & Astrophysics* 402,
    701–712,
    [doi:10.1051/0004-6361:20030252](https://doi.org/10.1051/0004-6361:20030252),
    preprint [arXiv:astro-ph/0302293](https://arxiv.org/abs/astro-ph/0302293).
    The evolution tables (effective temperature by mass and age).
14. Asplund, M., Grevesse, N., Sauval, A. J. & Scott, P. 2009, "The chemical
    composition of the Sun", *Annual Review of Astronomy and Astrophysics*
    47, 481–522,
    [doi:10.1146/annurev.astro.46.060407.145222](https://doi.org/10.1146/annurev.astro.46.060407.145222).
    Solar nitrogen, oxygen and sulphur for the decks.
15. Lewis, J. S. 1969, "The clouds of Jupiter and the NH₃–H₂O and NH₃–H₂S
    systems", *Icarus* 10, 365–378,
    [doi:10.1016/0019-1035(69)90091-8](https://doi.org/10.1016/0019-1035(69)90091-8).
    Not read (paywalled, no open copy found). The NH₄SH law, $\log_{10} K =
    14.82 - 4705/T$ with $K$ in atm², is quoted from equation 30 of Li, C.,
    Ingersoll, A. P. & Oyafuso, F. 2018, "Moist adiabats with multiple
    condensing species: a new theory with application to giant-planet
    atmospheres", *Journal of the Atmospheric Sciences* 75, 1063–1072,
    [doi:10.1175/JAS-D-17-0257.1](https://doi.org/10.1175/JAS-D-17-0257.1),
    preprint [arXiv:1801.05529](https://arxiv.org/abs/1801.05529), read for
    that equation, which attributes it to Lewis. Neither the temperature
    range of the fit nor the ammonia constants are in that preprint.
16. Fitzpatrick, P. J., de Pater, I., Luszcz-Cook, S., Wong, M. H. & Hammel,
    H. B. 2014, "Dispersion in Neptune's zonal wind velocities from NIR
    Keck AO observations in July 2009", *Astrophysics and Space Science*
    350, 65–88,
    [doi:10.1007/s10509-013-1737-2](https://doi.org/10.1007/s10509-013-1737-2),
    preprint [arXiv:1312.2676](https://arxiv.org/abs/1312.2676). Table 1,
    the Voyager fit of Sromovsky et al. (1993).
17. Soyuer, D., Neuenschwander, B. & Helled, R. 2023, "Zonal winds of
    Uranus and Neptune: gravitational harmonics, dynamic self-gravity,
    shape, and rotation", *The Astronomical Journal* 165, 27,
    [doi:10.3847/1538-3881/aca08d](https://doi.org/10.3847/1538-3881/aca08d),
    preprint [arXiv:2210.17389](https://arxiv.org/abs/2210.17389). Winds
    of up to 200 and 400 m/s on Uranus and Neptune.
18. Lindal, G. F. et al. 1981, "The atmosphere of Jupiter: an analysis of
    the Voyager radio occultation measurements", *Journal of Geophysical
    Research* 86, 8721–8727,
    [doi:10.1029/JA086iA10p08721](https://doi.org/10.1029/JA086iA10p08721).
    Read in the abstract: temperature of 165 ± 5 K at 1 bar, and "the
    tropopause, which was detected near the 140-mbar level, had a
    temperature of 110 K".
19. Seiff, A. et al. 1998, "Thermal structure of Jupiter's atmosphere near
    the edge of a 5-μm hot spot in the north equatorial belt", *Journal of
    Geophysical Research: Planets* 103, 22857–22889,
    [doi:10.1029/98JE01766](https://doi.org/10.1029/98JE01766). Read in the
    abstract: the Galileo probe found the tropopause at 260 mbar, with an
    isothermal layer of about 112 K just above it.

## See also

- [docs/analysis/12-gas-giants.md](../analysis/12-gas-giants.md): the study
  behind the app's giant planets and giant worlds.
- [Why are giant planets far from their star, and what is a hot
  Jupiter?](giant-planet-placement.md)
- [How far can rings reach?](ring-extent.md) and [Why are ring moons so
  small?](ring-moons.md)
- The README's **giant worlds** and **giants** paragraphs, for the options
  and the data stored in `world.json`.
