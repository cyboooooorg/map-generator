# Why do some stars have spots, and how fast do they turn?

Seen from: the star's surface, its spots, faculae, granulation and limb
darkening, as the living star (issue #343) draws them. The values are
`star.activity` in `system.json` and in `GET /api/systems/{id}`.

## Short answer

A star like the Sun has a boiling outer layer, the convective envelope.
Its rotation twists the magnetic field carried by that boiling, and the
field breaks through the surface as dark spots and bright faculae. The
faster the star turns compared with how fast its envelope churns, the
stronger this dynamo, up to a ceiling. Stars spin down as they age, so old
stars are quieter. The dynamo runs in cycles of years, and spots appear in
belts that slide towards the equator during each cycle. Stars hotter than
about 7,200 K have no convective envelope: no dynamo, no spots, no
granules, and they keep spinning fast.

## The science

### Rotation slows with age

A star with a convective envelope loses angular momentum through its
magnetised wind, so its rotation period grows with age. Gyrochronology
fits this as a function of the colour $B-V$ and the age $t$ in Myr
(Mamajek & Hillenbrand 2008, table 10):

```math
P = a\,(B-V-c)^{b}\,t^{n},\qquad a=0.407,\ b=0.325,\ c=0.495,\ n=0.566
```

- **In words**: at a given colour the period grows as the square root of
  the age, roughly; redder (cooler) stars turn more slowly at the same
  age.
- **Orders of magnitude**: the Sun ($B-V = 0.65$, 4.6 Gyr) gets 26 days,
  against its measured 25.4 days.

The fit was calibrated for $0.5 < B-V < 0.9$, F7 to K2 dwarfs. The colour
comes from the temperature by inverting the relation of Ballesteros
(2012), $T = 4600\,(1/(0.92\,BV+1.7) + 1/(0.92\,BV+0.62))$.

Stars hotter than the Kraft break, about 6,250 K, have envelopes too thin
to brake them (Kraft 1967): they keep rotating fast. A stars, with no
convective envelope, have $v \sin i$ of 100 to 200 km/s (Royer et al.
2007).

Fully convective M dwarfs spin down on their own law. Engle & Guinan
(2018, eq. 2) fit the ages of M2.5 to M6 dwarfs as
$\text{age (Gyr)} = 0.012 + 0.061\,P\,(\text{days})$.

- **Orders of magnitude**: a 4.6 Gyr old mid-M dwarf turns in about 75
  days; Proxima Centauri, about 4.8 Gyr old, turns in about 83 days.

### The Rossby number sets the activity

The convective turnover time $\tau_c$ is how long a convective cell takes
to rise and sink. Wright et al. (2018, eq. 6) calibrate it from the mass
$M$ in solar masses, for $0.08 < M < 1.36$:

```math
\log_{10}\tau_c = 2.33 - 1.50\,M + 0.31\,M^2 \quad\text{(days)}
```

The Rossby number is $Ro = P/\tau_c$. The ratio of the X-ray to the
bolometric luminosity, a measure of the dynamo, is flat below a
saturation value and falls as a power of $Ro$ above it (Wright et al.
2011, unbiased sample):

```math
\frac{L_X/L_{bol}}{(L_X/L_{bol})_{sat}} = \min\left(1,\ \left(\frac{Ro}{0.13}\right)^{-2.70}\right)
```

- **In words**: below $Ro = 0.13$ the dynamo is as strong as it gets; a
  star that turns ten times slower than that is 500 times quieter.
- **Orders of magnitude**: the Sun has $\tau_c = 13.8$ days and
  $Ro \approx 1.8$, so its activity is about $7.8\times10^{-4}$ of the
  saturated value. Fully convective stars follow the same law (Wright et
  al. 2016, 2018).

### Spots and faculae

At its cycle maximum the Sun has about 0.3 % of its visible hemisphere
under spots, 3,000 millionths of a hemisphere (MSH). Faculae, bright
magnetic patches, cover several times the spotted area on the Sun, which
is why the Sun is brighter at activity maximum. The most active stars are
spot-dominated: they are darker at maximum (Lockwood et al. 2007; Shapiro
et al. 2014), and can have about 30 % of their surface spotted
(Strassmeier 2009).

A single active region's peak spot area follows a log-normal law, first
found for umbral areas by Bogdan et al. (1988). For whole sunspot groups
at their maximum development, Baumann & Solanki (2005, table 1) fit
$\langle A\rangle = 62.2$ MSH and $\sigma_A = 2.45$ in the form
$\ln dN/dA \propto -(\ln A - \ln\langle A\rangle)^2 / (2\ln\sigma_A)$,
which is a median peak area of 152 MSH and $\sigma(\ln A) = 0.95$.

Spots decay at a steady rate in area: the Gnevyshev–Waldmeier rule
$A_{peak} = W\,T$, with $W \approx 10$ MSH per day (Petrovay & van
Driel-Gesztelyi 1997). Giles et al. (2017) find longer lifetimes for
larger spots on other stars, consistent with a constant $W$.

### Inside an active region

An active region emerges as a bipole: a leading spot of one magnetic
polarity and a following one of the other, joined by coronal loops.

- **Joy's law.** The axis of the bipole is tilted to the east–west line,
  the leader nearer the equator, and the tilt grows with latitude
  $\lambda$: $\text{tilt} = 32.1°\,\sin|\lambda|$ over 160,079 bipolar
  regions of the SOHO/MDI magnetograms (Stenflo & Kosovichev 2012, as
  given by Stenflo 2013). Hale's law sets the polarities: the leader's
  polarity is opposite in the two hemispheres and flips each cycle.
- **Separation and loops.** Larger regions have their two polarities
  farther apart: about 10° of heliographic distance for a region of
  1,000 MSH (Wang & Sheeley 1989). A coronal loop over the bipole is
  close to a semicircle, so its height is half the separation of its
  footpoints (Aschwanden 2004, ch. 3).
- **Umbra and penumbra.** A spot has a dark core, the umbra, inside a
  greyer penumbra; Solanki (2003) reviews their brightness, about 0.3
  and 0.75 of the quiet photosphere in the visible, and the penumbra's
  area, about four times the umbra's. Later counts give 4.7 (Nicholson
  1933, as quoted by Jha et al. 2019) to 5.5 (Jha et al. 2019) for that
  ratio.
- **Faculae** are brightest near the limb, where we see the hot walls of
  small magnetic flux tubes side-on (Solanki 2003).

### The cycle and the butterfly

Böhm-Vitense (2007) found that stars with known cycles fall on two
sequences in the plane of cycle period against rotation period, along
each of which the number of rotations per cycle is nearly the same: about
90 on the inactive sequence and about 300 to 500 on the active ones
(as summarised by Distefano et al. 2017). The Sun, with about 160 (an
11-year cycle of 25.4-day rotations), lies between the two.

During a solar cycle, spots emerge at about 28° of latitude at its start
and drift towards the equator, stopping near 7°: the mean latitude
follows $\lambda = 28°\,e^{-t/90\ \text{months}}$ (Hathaway 2011). Rapid
rotators push the band to higher latitudes, because the Coriolis force
deflects rising flux tubes towards the poles (Schüssler & Solanki 1992).
Doppler images of fully convective M dwarfs show spots at all latitudes
(Morin et al. 2008).

The shape of a cycle in time is fitted by Hathaway, Wilson & Reichmann
(1994) as $f(x) = x^3/(e^{x^2} - c)$; Hathaway (2015) gives $c = 0.8$ for
the average cycle. The browser's timeline (`starActivity.ts`) uses
$c = 0.71$, the value taken from the 1994 paper (not checked
against its text), normalises $f$ to a peak of 1 and maps one cycle onto
$x \in [0, 2.6]$, where $f$ is back to 4 % of its peak (the 2.6 is a
**calibration**). The latitude drift is $e^{-1.5\,u}$ over the cycle
fraction $u$ (Hathaway 2011: 132 months / 90 months = 1.47), rescaled to
end exactly at the band's end. A region's growth time is
$\min(0.1\,T,\ 3\ \text{d})$ times a factor within $\pm 20\ \%$
(a **calibration**, so that regions born in one slot do not all peak
together).

### Differential rotation

The Sun's equator turns faster than its poles,
$\Omega(\lambda) = \Omega_{eq}(1 - k\sin^2\lambda)$. Its equator-to-pole
difference is about $\Delta\Omega = 0.055$ rad/day (Snodgrass & Ulrich
1990). Doppler imaging of young rapid rotators shows $\Delta\Omega$
growing steeply with temperature, $\Delta\Omega \propto T^{8.92\pm0.31}$,
and only weakly with rotation, $\propto \Omega^{0.15}$ (Barnes et al.
2005). Reinhold et al. (2013) find the same trend in Kepler light curves.

### Granulation and limb darkening

Granules, the tops of convective cells, scale with the pressure scale
height $H_p \propto T/g$ (Trampedach et al. 2013; Magic et al. 2013). On
the Sun they are about 1.3 Mm across and live minutes: Title et al.
(1989) find exponential lifetime distributions with time constants of 2.5
to 6 minutes, depending on how granules are tracked. Seen from above,
granulation is a network of cells: each granule is a hot, bright
upflow, ringed by narrow, cooler and darker intergranular lanes where
the gas sinks back (Nordlund, Stein & Asplund 2009). Small magnetic
bright points sit in the lanes; in the quiet Sun they last a few
minutes and reach 0.8 to 1.8 times the mean photosphere (Sánchez
Almeida et al. 2004).

A star's disc is darker at its edge, because there we see higher, cooler
layers. The quadratic law $I(\mu) = 1 - u_1(1-\mu) - u_2(1-\mu)^2$, with
$\mu$ the cosine of the angle from the disc centre, fits model
atmospheres well; Claret & Bloemen (2011) tabulate $u_1, u_2$ for every
temperature, gravity and filter.

### What we do not know

Whether the two cycle sequences are real is debated (Reinhold et al.
2017; Boro Saikia et al. 2018), and the Sun fits neither. The dynamo of
fully convective stars works without the tachocline the solar dynamo is
thought to need, and their cycles are poorly known. The spot coverage of
stars between the Sun and the saturated stars is measured for few stars.

## In the app

### What you see

Each system's star carries its activity: its rotation, its cycle, its
active band, how much of it is spotted at cycle maximum and how often new
regions appear, and the look of its photosphere. A Sun-like star turns in
about 25 days and has a few active regions at a time; an A star turns in
less than a day and shows a smooth disc; an old M dwarf turns in about 75
days, is twenty times more active than the Sun, and spots at every
latitude.

### Coronal loops

Above the spots stand coronal loops: arches of plasma that follow the
field lines joining a region's leader to its follower
(`lib/viewer/coronalLoops.ts`). A loop is drawn between points on the
bipole's axis, so its feet sit on the two spots, and its height is half
the distance between its feet, as for the semicircular loops of
Aschwanden (2004, *Physics of the Solar Corona*), within a deterministic
scatter of 20 %. A region is drawn with 3 loops in the high quality
profile (the 8 largest regions) and 1 in the light one (the 3 largest);
they turn with the region's longitude, grow dim as it decays (opacity
proportional to the square root of its area over its peak area) and go
with it. A star with no activity, or with a radiative envelope, has no
loops. The loops are drawn in the red of H-alpha, which is a departure
(see below). The 3 loops, the 20 % scatter and the side-by-side spacing are
**calibration**: they draw the picture, not a measure.

### In a planet's sky

The star's disc in a planet's sky is the same living star: the planet
view draws it with the star views' surface (`starSurface.ts`), at the
system's day, so it shows the spots the star view shows at that day and
the same quadratic limb darkening, Claret & Bloemen's for the star. The
disc is a flat sprite on screen; `lib/viewer/hostStarSurface.ts` turns
each point of it into a point of the star's visible hemisphere, in the
star's own frame (its spin axis the system's +y, longitude 0 the
system's +x at day 0), so the spots sit where they are on the star, seen
from the planet, whatever the camera does.

What the disc shows depends on its size on screen (**calibration**, for
the frame rate):

- under 16 CSS pixels across, the limb darkening only. The Sun from
  Earth is about 14 px at 1080p, so the usual disc skips the region loop
  entirely, behind a uniform branch, and costs what the old disc cost;
- from 16 to 24 px, the active regions fade in, and from 24 px they are
  all drawn, as from a close-in planet;
- never the granulation, compiled out of this shader: a granule is
  about a five-hundredth of a Sun-like star's radius, a pixel or less
  on any disc a planet's sky shows.

While the orbits play, the clock's speed fades the regions out as in the
star views (a long exposure).

The disc takes the star view's exposure, not the plain disc's gain (the
linear colour times 1.4), which clips the quiet photosphere to white over most of a large disc (up to 0.85 of its
radius on G2V b), and the penumbrae (0.75 of the photosphere, 1.05 after
the gain) and the faculae disappear: only the umbrae showed, as grey
dots. The disc now draws `starSurfaceExposed`, at the same gain as the
star view: the soft curve $1 - e^{-1.6\,b}$ on the brightness, with the
limb law and the light linear. For the G2V star of the test system, its
sRGB values at 0, 0.5, 0.9 and 0.98 of the radius are 231, 224, 192 and
166, against 231, 223, 194 and 174 in the star view seen from 50 radii.
The plain disc keeps its 1.4.

The disc is drawn on a square that the vertex shader places round the
star's direction, not on a point sprite. A point is clamped to the GPU's
largest point size (`ALIASED_POINT_SIZE_RANGE`, often 511 px), and the
square was also capped at 512 px, while the star of a close-in planet
spans several hundred pixels: the disc filled the whole square, an
opaque white square behind the globe. The square now grows to twice the
disc and a margin, so that the glow round it, which fades out between a
quarter and half of the square's side, always starts outside the disc.

### How it computes them

[`activity.rs`](../../crates/mapgen/src/activity.rs) derives a
`StarActivity` from the star's temperature, mass, radius and age
(`giants::system_age_gyr`) and the system's seed. The seed draws, from
its own stream (`STAR_ACTIVITY`) and in a fixed order: a scatter of the
rotation period, a scatter of the cycle period, the cycle's phase at day
0, and the seed of the region timeline the browser draws.

1. **Envelope**: radiative at or above 7,200 K (the granulation boundary
   of Gray & Nagel 1989, near F0V), fully convective below 0.35 M☉
   (Chabrier & Baraffe 1997), convective otherwise.
2. **Rotation**: gyrochronology below 6,250 K, with $B-V$ clamped to
   [0.45, 1.6]; from 6,250 to 7,200 K the equatorial velocity ramps
   linearly from the gyrochronology value at 6,250 K to 100 km/s; 150
   km/s for a radiative star; Engle & Guinan's law, clamped to [0.2, 150]
   days, for a fully convective one. The period is then multiplied by
   $e^{N(0,\,0.15)}$.
3. **Activity**: $Ro = P/\tau_c$, then Wright et al.'s law. Radiative
   stars get 0 for the turnover time, the Rossby number and the activity.
4. **Spots**: with $a_\odot$ the model's activity for the Sun (1 M☉, 25.4
   days), `spot_coverage` $= \min(0.3,\ 0.003\,(a/a_\odot)^{0.64})$ and
   `facula_ratio` $= 10\,(a/a_\odot)^{-0.32}$ within [1, 15].
5. **Cycle** (**calibration** on the Sun, with the shape of
   Böhm-Vitense's sequences): `cycle_years` is 160 rotations for
   $Ro \ge 1$ (the Sun: 11.1 years), rising linearly in $\log_{10} Ro$
   to 300 at $Ro = 0.13$ and held at 300 below, times
   $e^{N(0,\,0.15)}$. `cycle_depth` is 0.3 at $Ro \le 0.13$ and 0.9 at
   $Ro \ge 1$, linear in $\log Ro$ between.
6. **Band**: [28°, 7°], shifted poleward by up to 40° as $Ro$ falls from
   the Sun's to 0.13; [75°, 0°] for a fully convective star.
7. **Regions**: the median peak area is 152 MSH times
   $\sqrt{\text{spot\_coverage}/0.003}$ (**calibration**),
   $\sigma(\ln A) = 0.95$, and $W = 10$ MSH/day. A region of peak area
   $A$ lives $T = \min(A/W,\ 120\ \text{days})$ (the cap is a
   **calibration**); its area rises and falls linearly, a triangle of
   base $T$ and height $A$, so it contributes $A\,T/2$ area-days
   ($A^2/(2W)$ when the cap does not bind). The rate of new regions at
   cycle maximum, over the whole star, is
   $2\,$`spot_coverage` $/\,E[A\,T/2]$. The factor 2 is because
   `spot_coverage` is a share of the *visible* hemisphere, the way the
   Sun's 0.3 % is measured, while regions spread evenly in longitude over
   the whole star: their total area is twice the visible hemisphere's
   spotted area, half of it facing the viewer at any time. The
   expectation over the
   log-normal is a fixed-step sum over $\ln A$ within $\pm 6\sigma$
   (`mean_area_days`); the browser's timeline uses the same lifetime
   rule.
8. **Surface**: `shear` $= \Delta\Omega/\Omega_{eq}$ within [0, 0.5],
   with $\Delta\Omega = 0.055\,(T/5772)^{8.92}$ rad/day; granule size
   $1.3\ \text{Mm}\,(T/5772)/(g/g_\odot)$ in star radii, contrast
   $0.15\,(T/5772)^2$ capped at 0.25 and fading to 0 from 6,800 to
   7,200 K, lifetime 8 minutes times the granule size over the Sun's;
   limb darkening interpolated linearly in Teff between seven rows of
   Claret & Bloemen's table (VizieR J/A+A/529/A75/table-af: Johnson V,
   $\log g = 4.5$, solar metallicity, $\xi = 2$ km/s, least squares,
   ATLAS), held beyond 3,500 and 10,000 K. G2V gets $u_1 = 0.462$,
   $u_2 = 0.258$.
9. **Each region, drawn in the browser** (from `seed`, in
   `web/src/lib/starActivity.ts`, not in Rust): it grows over
   $\min(0.1\,T,\ 3\ \text{days})$ (**calibration**), then decays
   linearly; its tilt follows Joy's law, $32.1°\,\sin|\lambda|$, with
   the leader equatorward, and Hale's law sets its polarity; its two
   polarities are $4\sqrt{2A}$ radians apart (**calibration**, scaled to
   Wang & Sheeley's 10° at 1,000 MSH), and its loops rise to half that
   (Aschwanden 2004). The leader spot carries 0.6 of the area
   (**calibration**). The shader draws umbrae at 0.3 and penumbrae at
   0.75 of the photosphere, the penumbra four times the umbra's area
   (Solanki 2003), and faculae with a contrast of $0.25\,(1-\mu)$
   (**calibration**, brightest at the limb as Solanki 2003 describes).
   The plage is a disc of three times the region's spot radius round its
   centre, which covers nine spot areas; it is filled at
   `facula_ratio` / 9, at most 1, so that a Sun-like ratio of about 10
   fills it and a spot-dominated star's plages are fainter
   (**calibration**). The granulation is moving cellular noise at the
   granule size, as prototype D drew it: one cell centre per unit cube,
   in a space warped by one octave of value noise. A cell is bright in
   its middle, $\mathrm{smoothstep}(0.02, 0.30, F_2 - F_1)\,(1.15 -
   0.55\,F_1)$, and dark in the lanes where $F_2 - F_1$ falls to 0.
   $F_1$ is the distance to the nearest centre. $F_2 - F_1$ is twice
   the distance along the surface to the edge of that centre's cell,
   not the 3D second-nearest distance: a sphere cutting a 3D cell
   pattern runs nearly parallel to some faces between centres stacked
   in depth, where the 3D $F_2 - F_1$ stays small over wide flat grey
   patches with hard edges. About a fifth of the lanes carry a soft
   bright point where the lane crosses the line between its two
   centres, weighted 0.35 against the cells' 0.62 (prototype D;
   Nordlund, Stein & Asplund 2009 for the cells and lanes, Sánchez
   Almeida et al. 2004 for the points; the shapes and weights are
   **calibrations**). Each centre wanders
   about its cell's middle by 0.3 of a cell along each axis, at 97, 103
   and 109 turns per 289 granule lifetimes (2.1 to 2.4 rad per
   lifetime, **calibration**), so that cells deform, split and merge
   over a few lifetimes; the warp drifts round a circle at 2 turns per
   289 lifetimes. Every motion turns a whole number of times in the 289
   lifetimes at which the phase wraps, so the surface does not jump
   there. The result is brought
   to zero mean and unit rms (measured over the disc), then multiplied
   by a gain of 1.6 (**calibration**): prototype D, the approved look,
   drew an rms contrast of about 0.24 on the Sun, whose
   `granule_contrast` is 0.15. The drawn rms contrast of the brightness
   is therefore 1.6 times `granule_contrast`, before the soft exposure
   below compresses it on screen.
   The shader (`web/src/lib/viewer/starSurface.ts`) fades the
   regions and the granules out as the star shrinks on screen, and as a
   frame of the clock outlasts a twelfth of a rotation or a granule
   lifetime (as $\sqrt{\text{timescale}/\text{frame}}$, a long
   exposure's averaging). Its soft exposure, $1 - e^{-1.6\,b}$, acts on
   the brightness $b$ of granules, spots and faculae only; the limb law
   and the star's colour multiply the result linearly, so the limb keeps
   Claret & Bloemen's darkening and the disc its colour.

```rust
// crates/mapgen/src/activity.rs (trimmed)
let turnover_days = turnover_days(mass);
let rossby = rotation_days / turnover_days;
let activity = rossby_activity(rossby);
let relative = activity / sun_activity();
let spot_coverage = (SUN_SPOT_COVERAGE * relative.powf(SPOT_EXPONENT)).min(MAX_SPOT_COVERAGE);
// A region of peak area A lives T = min(A / W, 120 d): A T / 2 area-days.
// The whole star carries twice the visible hemisphere's spot coverage.
let region_rate =
    2.0 * spot_coverage / mean_area_days(region_area, region_sigma, decay_per_day);
```

A system drawn before this existed has no `activity` in its file:
[`read_system`](../../crates/mapgen/src/system.rs) derives it on read,
the same value a new draw stores, so the API always serves it.

For the Sun's type (G2V) the medians over seeds are a rotation of 25.8
days, $Ro = 1.87$, a cycle of 11.1 years, 0.29 % of the visible
hemisphere spotted at maximum and 1.03 new regions per day over the whole
star, about half of them on the visible side.

### Where it departs from the science

- **Coronal loops are red.** The hot loops that dominate the Sun's corona
  glow in the EUV and X-rays (millions of kelvin), invisible to the eye;
  only cool loops, such as the post-flare loops of a decaying region, show
  in H-alpha. The view draws every loop in H-alpha red, as an eye-catching
  convention that keeps the old prominences' look, not as what a camera
  would record. Whatever their temperature, they are red.

- **The cycle is anchored on the Sun, not on a stellar sequence.** Every
  star with $Ro \ge 1$ runs 160 rotations per cycle, the Sun's, which
  lies between Böhm-Vitense's two sequences; real inactive-sequence stars
  run about 90. The ramp to 300 at saturation only keeps the trend that
  active stars need more rotations per cycle, without a jump. The
  sequences themselves are disputed.
- **The spot coverage law is a calibration.** Its exponent, 0.64, only
  joins the Sun (0.3 %) to the saturated stars (30 %); the facula ratio
  law, its exponent and bounds are calibrations too.
- **Active stars have larger regions, barely more of them.** Spot
  coverage scales the median region area by its square root, and without
  the lifetime cap the rate would go as the coverage over the area
  squared, the same on every star (0.86 a day over the whole star). The
  120-day cap shortens the largest regions, so the rate rises somewhat
  with activity: 1.03 a day on the Sun, 1.57 on an old M5 dwarf.
- **The lifetime cap is a calibration.** Real large spots can outlive
  several rotations; the cap keeps regions of a saturated star (median
  peak area about 1,500 MSH, 150 days at $W$) from lasting for half a
  year.
- **Solar region statistics everywhere.** The log-normal width and $W$
  are the Sun's on every star.
- **The poleward shift of the band (40°) and the M-dwarf band
  ([75°, 0°]) are calibrations**, as are the cycle depths and the
  scatters of the rotation and cycle periods (0.15 in the log).
- **The Kraft-break ramp and the 150 km/s of A stars are fixed values**
  where real stars spread widely, and young stars, which turn faster,
  are not modelled: every star has the age `system_age_gyr` gives it.
- **$\Delta\Omega \propto T^{8.92}$ was measured on young G to M rapid
  rotators.** Applied to hotter stars it overshoots: the shear is
  clamped to 0.5, which radiative stars all reach.
- **Granule lifetime**: the 8 minutes is a calibration a little above
  the time constants Title et al. (1989) report.
- **The drawn granulation is a picture of convection, not a model of
  it.** Its cells, lanes, bright points, wander speeds and the gain of
  1.6 on `granule_contrast` are calibrations matched to prototype D;
  the gain makes the drawn contrast (0.24 rms on the Sun) higher than
  the 0.15 the activity gives, so that the surface reads as agitated
  up close.

- **Timeline and region shapes are calibrations.** One cycle maps onto
  $x \in [0, 2.6]$ of the Hathaway fit; a region grows over
  $\min(0.1\,T,\ 3\ \text{d})$ times a factor within $\pm 20\ \%$; its
  polarities are $4\sqrt{2A}$ radians apart; the leader spot carries 0.6
  of the area.
- **Region size and look are calibrations.** The median peak area
  scaling, $152\,\text{MSH}\sqrt{\text{spot\_coverage}/0.003}$; the
  faculae's $0.25\,(1-\mu)$ contrast; the plage's disc of three spot
  radii filled at `facula_ratio` / 9.
- **The loops' 3 per region, 20 % scatter and spacing are calibrations**:
  they draw the picture, not a measure.
- **The sky disc's detail thresholds are calibrations**, chosen for the
  frame rate: limb darkening only under 16 px, regions fading in from 16
  to 24 px (not in one step), all of them from 24 px, no granulation.
- **The sky disc uses the star view's exposure**, not the plain disc's
  gain of 1.4, which clips the photosphere and hides the penumbrae and
  faculae.
- **Region ids are 32-bit hashes** of the star's seed, the slot and the
  index in the slot, so a region keeps its look and its loops from one
  run to the next; two regions can in principle share an id.

### Values not checked against their source

These values could not be checked against their source; they need
checking:

- the Sun's 0.3 % spotted area at maximum (Hathaway 2015);
- the solar $\Delta\Omega = 0.055$ rad/day (Snodgrass & Ulrich 1990);
- the 1.3 Mm solar granule size (Trampedach et al. 2013);
- the rotations per cycle of Böhm-Vitense (2007), read from the summary
  of Distefano et al. (2017);
- the typical $v \sin i$ of A stars (Royer et al. 2007), the M-dwarf
  spots at all latitudes (Morin et al. 2008), and $c = 0.71$ of Hathaway,
  Wilson & Reichmann (1994);
- the umbral and penumbral brightness (0.3, 0.75) and the penumbra's four
  umbral areas in Solanki (2003), and the 10° separation of a 1,000 MSH
  region in Wang & Sheeley (1989).

Joy's 32.1° was checked against Stenflo (2013), the limb-darkening rows
against the VizieR catalogue.

## Sources

1. Mamajek, E. E. & Hillenbrand, L. A. 2008, "Improved age estimation for
   solar-type dwarfs using activity-rotation diagnostics", *The
   Astrophysical Journal* 687, 1264, preprint
   [arXiv:0807.1686](https://arxiv.org/abs/0807.1686).
2. Ballesteros, F. J. 2012, "New insights into black bodies", *EPL* 97,
   34008.
3. Kraft, R. P. 1967, "Studies of stellar rotation. V. The dependence of
   rotation on age among solar-type stars", *The Astrophysical Journal*
   150, 551.
4. Royer, F., Zorec, J. & Gómez, A. E. 2007, "Rotational velocities of
   A-type stars. III. Velocity distributions", *Astronomy & Astrophysics*
   463, 671.
5. Engle, S. G. & Guinan, E. F. 2018, "Red dwarf stars: ages, rotation,
   magnetic dynamo activity and the habitability of hosted planets",
   *Research Notes of the AAS* 2, 34,
   [doi:10.3847/2515-5172/aab1f8](https://doi.org/10.3847/2515-5172/aab1f8).
6. Barnes, S. A. 2007, "Ages for illustrative field stars using
   gyrochronology", *The Astrophysical Journal* 669, 1167.
7. Wright, N. J., Newton, E. R., Williams, P. K. G., Drake, J. J. &
   Yadav, R. K. 2018, "The stellar rotation–activity relationship in
   fully convective M dwarfs", *Monthly Notices of the RAS* 479, 2351,
   [doi:10.1093/mnras/sty1670](https://doi.org/10.1093/mnras/sty1670),
   preprint [arXiv:1807.03304](https://arxiv.org/abs/1807.03304).
8. Wright, N. J., Drake, J. J., Mamajek, E. E. & Henry, G. W. 2011, "The
   stellar-activity–rotation relationship and the evolution of stellar
   dynamos", *The Astrophysical Journal* 743, 48, preprint
   [arXiv:1109.4634](https://arxiv.org/abs/1109.4634).
9. Wright, N. J. & Drake, J. J. 2016, "Solar-type dynamo behaviour in
   fully convective stars without a tachocline", *Nature* 535, 526.
10. Shapiro, A. I. et al. 2014, "Variability of Sun-like stars: reproducing
    observed photometric trends", *Astronomy & Astrophysics* 569, A38.
11. Lockwood, G. W. et al. 2007, "Patterns of photometric and chromospheric
    variation among Sun-like stars", *The Astrophysical Journal* 664,
    1234.
12. Strassmeier, K. G. 2009, "Starspots", *Astronomy and Astrophysics
    Review* 17, 251.
13. Bogdan, T. J., Gilman, P. A., Lerche, I. & Howard, R. 1988,
    "Distribution of sunspot umbral areas: 1917–1982", *The Astrophysical
    Journal* 327, 451.
14. Baumann, I. & Solanki, S. K. 2005, "On the size distribution of
    sunspot groups in the Greenwich sunspot record 1874–1976", *Astronomy
    & Astrophysics* 443, 1061, preprint
    [arXiv:astro-ph/0510516](https://arxiv.org/abs/astro-ph/0510516).
15. Petrovay, K. & van Driel-Gesztelyi, L. 1997, "Making sense of sunspot
    decay. I. Parabolic decay law and Gnevyshev–Waldmeier relation",
    *Solar Physics* 176, 249, preprint
    [arXiv:astro-ph/9706029](https://arxiv.org/abs/astro-ph/9706029).
16. Giles, H. A. C., Collier Cameron, A. & Haywood, R. D. 2017, "A Kepler
    study of starspot lifetimes with respect to light-curve amplitude and
    spectral type", *Monthly Notices of the RAS* 472, 1618.
17. Böhm-Vitense, E. 2007, "Chromospheric activity in G and K
    main-sequence stars, and what it tells us about stellar dynamos", *The
    Astrophysical Journal* 657, 486.
18. Distefano, E., Lanzafame, A. C., Lanza, A. F., Messina, S. & Spada, F.
    2017, "Activity cycles in members of young loose stellar
    associations", *Astronomy & Astrophysics* 606, A58, preprint
    [arXiv:1706.07938](https://arxiv.org/abs/1706.07938).
19. Hathaway, D. H. 2011, "A standard law for the equatorward drift of the
    sunspot zones", *Solar Physics* 273, 221, preprint
    [arXiv:1108.1722](https://arxiv.org/abs/1108.1722).
20. Hathaway, D. H. 2015, "The solar cycle", *Living Reviews in Solar
    Physics* 12, 4,
    [doi:10.1007/lrsp-2015-4](https://doi.org/10.1007/lrsp-2015-4),
    preprint [arXiv:1502.07020](https://arxiv.org/abs/1502.07020).
21. Hathaway, D. H., Wilson, R. M. & Reichmann, E. J. 1994, "The shape of
    the sunspot cycle", *Solar Physics* 151, 177.
22. Schüssler, M. & Solanki, S. K. 1992, "Why rapid rotators have polar
    spots", *Astronomy & Astrophysics* 264, L13.
23. Morin, J. et al. 2008, "Large-scale magnetic topologies of mid M
    dwarfs", *Monthly Notices of the RAS* 390, 567.
24. Snodgrass, H. B. & Ulrich, R. K. 1990, "Rotation of Doppler features in
    the solar photosphere", *The Astrophysical Journal* 351, 309.
25. Barnes, J. R., Collier Cameron, A., Donati, J.-F., James, D. J.,
    Marsden, S. C. & Petit, P. 2005, "The dependence of differential
    rotation on temperature and rotation", *Monthly Notices of the RAS*
    357, L1,
    [doi:10.1111/j.1745-3933.2005.08587.x](https://doi.org/10.1111/j.1745-3933.2005.08587.x),
    preprint [arXiv:astro-ph/0410575](https://arxiv.org/abs/astro-ph/0410575).
26. Reinhold, T., Reiners, A. & Basri, G. 2013, "Rotation and differential
    rotation of active Kepler stars", *Astronomy & Astrophysics* 560, A4.
27. Trampedach, R. et al. 2013, "A grid of three-dimensional stellar
    atmosphere models of solar metallicity. I. General properties,
    granulation, and atmospheric expansion", *The Astrophysical Journal*
    769, 18,
    [doi:10.1088/0004-637X/769/1/18](https://doi.org/10.1088/0004-637X/769/1/18).
28. Magic, Z., Collet, R., Asplund, M. et al. 2013, "The Stagger-grid: a
    grid of 3D stellar atmosphere models. I. Methods and general
    properties", *Astronomy & Astrophysics* 557, A26.
29. Title, A. M. et al. 1989, "Statistical properties of solar granulation
    derived from the SOUP instrument on Spacelab 2", *The Astrophysical
    Journal* 336, 475,
    [doi:10.1086/167026](https://doi.org/10.1086/167026).
30. Claret, A. & Bloemen, S. 2011, "Gravity and limb-darkening
    coefficients for the Kepler, CoRoT, Spitzer, uvby, UBVRIJHK, and
    Sloan photometric systems", *Astronomy & Astrophysics* 529, A75.
31. Gray, D. F. & Nagel, T. 1989, "The granulation boundary in the HR
    diagram", *The Astrophysical Journal* 341, 421.
32. Chabrier, G. & Baraffe, I. 1997, "Structure and evolution of
    low-mass stars", *Astronomy & Astrophysics* 327, 1039.
33. Schmitt, J. H. M. M. 1997, "Coronae on solar-like stars", *Astronomy &
    Astrophysics* 318, 215.
34. Reinhold, T., Cameron, R. H. & Gizon, L. 2017, "Evidence for
    photometric activity cycles in 3203 Kepler stars", *Astronomy &
    Astrophysics* 603, A52.
35. Boro Saikia, S. et al. 2018, "Chromospheric activity catalogue of
    4454 cool stars", *Astronomy & Astrophysics* 616, A108.

36. Stenflo, J. O. & Kosovichev, A. G. 2012, "Bipolar magnetic regions on
    the Sun: global analysis of the SOHO/MDI data set", *The Astrophysical
    Journal* 745, 129, preprint
    [arXiv:1112.5226](https://arxiv.org/abs/1112.5226).
37. Stenflo, J. O. 2013, "Solar magnetic fields as revealed by Stokes
    polarimetry", *Astronomy and Astrophysics Review* 21, 66, preprint
    [arXiv:1309.5454](https://arxiv.org/abs/1309.5454).
38. Wang, Y.-M. & Sheeley, N. R. Jr. 1989, *The Astrophysical Journal*
    347, 529 (the reference the plan gives; not checked).
39. Aschwanden, M. J. 2004, *Physics of the Solar Corona: An
    Introduction*, Springer, ch. 3.
40. Solanki, S. K. 2003, "Sunspots: an overview", *Astronomy and
    Astrophysics Review* 11, 153,
    [doi:10.1007/s00159-003-0018-4](https://doi.org/10.1007/s00159-003-0018-4).
41. Jha, B. K. et al. 2019, "Study of sunspot penumbra to umbra area ratio
    using Kodaikanal white-light digitized data", preprint
    [arXiv:1905.06888](https://arxiv.org/abs/1905.06888).
42. Nordlund, Å., Stein, R. F. & Asplund, M. 2009, "Solar surface
    convection", *Living Reviews in Solar Physics* 6, 2,
    [doi:10.12942/lrsp-2009-2](https://doi.org/10.12942/lrsp-2009-2).
43. Sánchez Almeida, J., Márquez, I., Bonet, J. A., Domínguez Cerdeña,
    I. & Muller, R. 2004, "Bright points in the internetwork quiet Sun",
    *The Astrophysical Journal* 609, L91, preprint
    [arXiv:astro-ph/0405515](https://arxiv.org/abs/astro-ph/0405515).

## See also

- [What does a G-type star mean?](star-types.md): the star's type,
  temperature, mass and radius this article starts from.
- [docs/system-spec.md](../system-spec.md): where the star's activity is
  stored in a system.
- Issue #343: the living star, of which this model is part 1.
