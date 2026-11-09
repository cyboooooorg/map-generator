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
- **Penumbral filaments.** A penumbra is not uniform: it is made of thin
  bright and dark radial filaments, a few hundred kilometres wide, which
  fray its outer edge (Solanki 2003).
- **The Evershed flow.** Gas flows outward along the penumbral filaments
  at about 1–2 km/s; Evershed (1909) found it from the Doppler shifts of
  the spectral lines across a spot.
- **Umbral dots and light bridges.** The umbra holds small bright dots,
  a few hundred kilometres across, which live minutes to about an hour,
  and a larger spot is often crossed by a bright lane, a light bridge,
  most often while it develops or breaks up (Borrero & Ichimoto 2011).
- **Pores and the moat.** Small dark pores, spots without a penumbra,
  appear round a spot, and magnetic features stream away from it through
  an annular "moat" (Sheeley 1969; Solanki 2003).
- **Decay.** A spot decays by losing flux at its edge and by breaking up
  into fragments, often along its light bridges (Solanki 2003).
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

Granules are organised on a larger scale. The supergranules, cells about
30 Mm across, about twenty-three granules, are horizontal flows that
sweep the magnetic field to their edges (Leighton, Noyes & Simon 1962;
Rieutord & Rincon 2010). They live about a day (Hart 1956; Rieutord &
Rincon 2010). The field gathered along their edges forms the
chromospheric network, whose small faculae make it bright in white
light near the limb and nearly invisible at the disc centre, as plages
are (Solanki 2003). Between the two scales, the mesogranulation is a
weaker pattern of 5 to 10 Mm (November et al. 1981).

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
  on any disc a planet's sky shows;
- the supergranule network not under 16 px either, and in practice
  only on a disc hundreds of pixels across (from 3 px per network cell,
  see below).

While the orbits play, the clock's speed fades the regions out as in the
star views (a long exposure).

The disc takes the star view's exposure, not the plain disc's gain (the
linear colour times 1.4), which clips the quiet photosphere to white
over most of a large disc (up to 0.85 of its radius on G2V b), and the penumbrae
(0.75 of the photosphere, 1.05 after the gain) and the faculae disappear: only
the umbrae showed, as grey dots. The disc now draws `starSurfaceExposed`, at the
same gain as the star view: the soft curve $1 - e^{-1.6\,b}$ on the brightness,
with the limb law and the light linear. For the G2V star of the test system, its
sRGB values at 0, 0.5, 0.9 and 0.98 of the radius are 231, 224, 192 and 166,
against 231, 223, 194 and 174 in the star view seen from 50 radii. The plain
disc keeps its 1.4.

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
   (**calibration**). Each spot is drawn with the area of a disc of its
   share, its umbra about a fifth of it (the penumbra four times the
   umbra's area, Solanki 2003), and shaped as prototype D drew it (see
   *The spots' shapes* below). Faculae have a contrast of
   $0.25\,(1-\mu)$ (**calibration**, brightest at the limb as Solanki
   2003 describes).
   The plage is a disc of three times the region's spot radius round its
   centre, which covers nine spot areas; it is filled at
   `facula_ratio` / 9, at most 1, so that a Sun-like ratio of about 10
   fills it and a spot-dominated star's plages are fainter
   (**calibration**). The granulation is moving cellular noise at the
   granule size, as prototype D drew it: one cell centre per unit cube,
   in a space warped by one octave of value noise, strong enough to
   curve the cells' edges. Each cell is a dome, bright at its centre and
   falling towards its edge as $1.2 - 0.75\,s^2$, with $s$ the distance
   to the centre over the sum of the distances to the centre and to the
   edge, both measured along the surface. Its tone varies from cell to
   cell (0.75 to 1.2), it is dimmer while its centre lies deep (a
   granule being born or dying), and a quarter of the large cells have
   a darker middle, as exploding granules do; it falls faster near its
   edge ($-0.35\,\text{smoothstep}(0.75, 1, s)$), which rounds its
   corners. The intergranular lanes are dark with soft edges, narrow
   along long edges (0.05 cell) and widening into pools at the
   junctions of three cells (0.26 cell). The distances are taken
   along the surface, not as the 3D second-nearest distance: a sphere
   cutting a 3D cell pattern runs nearly parallel to some faces between
   centres stacked in depth, where the 3D $F_2 - F_1$ stays small over
   wide flat grey patches with hard edges (Nordlund, Stein & Asplund
   2009 for the cells and lanes; the shapes and weights are
   **calibrations**). The magnetic bright points of the lanes (Sánchez
   Almeida et al. 2004) are not drawn: prototype D's, cut by the lane
   they sat in, read as specks on screen. Each centre wanders
   about its cell's middle by 0.3 of a cell along each axis, at 97, 103
   and 109 turns per 289 granule lifetimes (2.1 to 2.4 rad per
   lifetime, **calibration**), so that cells deform, split and merge
   over a few lifetimes; the warp drifts round a circle at 2 turns per
   289 lifetimes. Every motion turns a whole number of times in the 289
   lifetimes at which the phase wraps, and the day clock behind the
   phase wraps at a whole number of those periods (near 10,000 days),
   so the surface does not jump at either wrap. The result is brought
   to zero mean and unit rms (measured over disc patches from the
   centre to 0.6 radii), then multiplied by a gain of 1.6
   (**calibration**): prototype D, the approved look, drew an rms
   contrast of about 0.24 on the Sun, whose `granule_contrast` is 0.15. The
   drawn rms contrast of the brightness is therefore 1.6 times
   `granule_contrast`, before the soft exposure below compresses it on screen.
   The shading follows a granule's size on screen (**calibration**): from 120
   pixels across it is the one above; at 40 pixels or fewer the cells are a sum
   of round blobs, one per centre, each of its cell's tone (0.45 to 1.45) times
   $e^{-d^2/(2\sigma^2)}$ with $\sigma = 0.3$ cell and $d^2$ the squared
   distance along the surface plus 0.4 of the squared depth (**calibrations**).
   The field is continuous everywhere, no edge is drawn, and the dips between
   centres are the lanes, wide where centres are far apart. Between the two it
   blends, and every lane edge is at least about 2.5 pixels soft. At mid zoom,
   about 1.1 radii, thin crisp lanes round flat cells read as cracked glass, and
   any edge drawn among the blobs as a thin seam. Both ends are brought to zero
   mean and the same rms. The supergranule network is a second cell pattern, at
   `SUPERGRANULE_SIZE_MM` = 30 Mm on the Sun (Leighton, Noyes & Simon 1962): its
   cell size is the granule size times 30/1.3, so that it scales with the
   convection on other stars (**calibration**; no survey measures other stars'
   networks). Its cells wander as the granules do, on their own phase, in
   supergranule lifetimes of `SUPERGRANULE_LIFETIME_DAYS` = 1 day (Hart 1956;
   Rieutord & Rincon 2010), wrapped at 289 lifetimes as the granule phase is,
   and turn with the star: the network is sampled at the longitude minus the
   equatorial rotation phase, without differential rotation over its day of
   life. Its lanes, $F_2 - F_1$ with the lane distance measured along the
   surface as for the granules, are drawn as soft, wide ribbons of faculae: $1 -
   \text{smoothstep}(0, w, F_2 - F_1)$ with $w = 0.6$ cells (**calibration**),
   widened up to 1.3 times at the junctions of three cells and of a tone varying
   from lane to lane (**calibration**), blended to a common tone where lanes
   meet. They are brightened by $c\,L(\mu)$, with $L = \max(0.35, \min(0.35 +
   1.4\,(1-\mu),\ 1.33\,(1-\mu)))$: faint at the disc centre, about four times
   brighter at the limb, as faculae are (Solanki 2003). The cap $1.33\,(1-\mu)$
   keeps a ribbon's peak, $0.15 \times 1.33\,(1-\mu) = 0.2\,(1-\mu)$, at 0.8 of
   a full plage's $0.25\,(1-\mu)$ (**calibration**), so that the active regions'
   faculae stay the brightest features near the limb. Its contrast is $c =
   \min(1, \text{granule\_contrast}/0.15) \times 0.15$, 0 on a radiative star.
   The 0.15 (**calibration**) makes the network stronger than the real one in
   white light, so that the whole disc, whose granules are under a pixel, does
   not look smooth: the disc centre is about 2 % brighter on average with a 2.3
   % rms, and near the limb ($\mu \approx 0.4$) about 5 % brighter with a 5 %
   rms. From afar, a mesogranular mottle, one octave of value noise at 0.35 of
   the cell size, adds $\pm c/4$ and varies the ribbons' width and tone along
   their length (**calibration**); it fades out as the granules fade in, since
   they cover its scale up close. The network is searched among 8 cells rather
   than the granules' 20, to keep it cheap over the whole disc; it misses a
   short stub of lane near a junction on about 0.1 % of the points, hidden by
   the soft ribbons. It fades in from 3 to 8 pixels per network cell. The shader
   (`web/src/lib/viewer/starSurface.ts`) fades the regions and the granules out
   as the star shrinks on screen, and as a frame of the clock outlasts a twelfth
   of a rotation, a granule lifetime or, for the network, which turns with the
   star, a supergranule lifetime or a twelfth of a rotation (as
   $\sqrt{\text{timescale}/\text{frame}}$, a long exposure's averaging). Its
   soft exposure, $1 - e^{-1.6\,b}$, acts on the brightness $b$ of granules,
   spots and faculae only; the limb law and the star's colour multiply the
   result linearly, so the limb keeps Claret & Bloemen's darkening and the disc
   its colour, but for the network's faculae: on average they brighten the limb
   by a few percent more than the centre (about 5 % at $\mu \approx 0.4$ against
   2 %), which flattens the drawn limb darkening a little (**calibration**, the
   price of a visible network).

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

### The spots' shapes

Each spot is drawn by `starSpotShaped` in
`web/src/lib/viewer/starSurface.ts`, after prototype D's live path, in
spot radii (so its shape grows and shrinks with the region's area). Its
seed is `hash32(region id, 7) / 2^32`, the follower's that seed plus a
half; its age is the region's age over its life. Every number below is a
**calibration**, matched by eye to prototype D, unless a source is given.

- **The outline.** Its second and third harmonics in angle (amplitude
  0.05 young, 0.12 grown) and two octaves of value noise at 2 cycles per
  spot radius (amplitude 0.35 young, 0.5 grown), both moving slowly with
  age: compact and nearly round while it grows, torn and lobed from about
  a quarter of its life. The harmonics keep the outline curved where a
  contour of lattice noise alone draws straight runs.
- **Under about a pixel** of radius a spot is part 1's round spot (umbra
  0.3, penumbra 0.75), blended into the shaped one from 0.7 to 1.5 px of
  radius: a spot being born, dying or seen small keeps part 1's area and
  does not blink as it moves across pixels.
- **The penumbra** at a mean 0.77 of the quiet photosphere (Solanki
  2003's 0.75), its filaments between 0.7 and 0.88: none as dark as the
  umbra, none as bright as the photosphere (Solanki 2003). They are value
  noise on a ring round the spot, periodic so that it has no seam,
  stretched along the radius. Faint bundles of 88 cells a turn, which
  change only with age and show from afar; filaments of 251 and 565 cells
  a turn (prototype D's circles of radius 40 and 90 noise units) up close,
  drifting outward with the Evershed flow: 1.5 km/s (Evershed 1909
  measured about 1–2 km/s) on a reference spot of 10 Mm, that is 13 spot
  radii a day, whatever the spot's size. Each set is computed only once
  its cells span about two pixels. The bundles shift the filaments round
  the spot, so their width varies; the outline's noise varies their
  length and bends them, by a curvature that changes round the edge; each
  ends at its own radius, so the edge frays into the granules.
- **The umbra** at about 0.3 (Solanki 2003), varying by ±0.03 with a
  smooth noise (a gradient that followed its outline banded into
  terraces on screen), 0.47 of the spot's radius (a little above a fifth
  of its area, so that the dark area on screen matches part 1's). It
  follows the outline at half its distortion, so its area varies little
  from spot to spot.
  Umbral dots (Borrero & Ichimoto 2011): round, soft points on a jittered
  grid of 0.07 spot radii, one in about a third of the cells, each
  twinkling with a 20-minute period, computed only inside the umbra.
- **A light bridge** across the umbra (Borrero & Ichimoto 2011), between
  0.30 and 0.80 of the region's life, at 0.8 of the photosphere, on about
  a third of the spots (a draw of their own) and only on large ones, from
  0.008 to 0.012 star radii (6 to 8 Mm on the Sun). The umbra widens by
  8 % round it, so that it keeps its area.
- **Fragmentation** in the decline (Solanki 2003): from 0.45 of the life,
  a share of the spot growing to 0.35 at its death opens into gaps where a
  breakup noise (two lattices, the second turned by 37°, so that no gap is
  blocky) falls below the level of that share, and the spot spreads by
  $1/\sqrt{1 - 0.78\,\text{share}}$, so its pieces keep its area. The
  umbra is not cut and keeps its size (the penumbra, about 0.78 of the
  area, breaks up around it). The timeline
  already shrinks the area to 0 at the end of the life, so the decline is
  not counted twice. Gaps too small to be seen (under about 1.5 to 5 px)
  are not opened, and the spot does not spread for them.
- **Pores in the moat** (Sheeley 1969; Solanki 2003), from 1.05 to 2.6
  spot radii, from about a fifth of the life: on a jittered grid of half
  spot radii, one in an eighth of the cells, 0.12 to 0.2 cell in radius,
  darkened by half, computed only in the moat. Each lies inside its cell,
  so the grid never cuts one.
- **No granules** inside a spot or a pore: they fade out across its soft
  edge and its brightest filaments (brightness 0.8 to 0.98), and are not
  computed where they are gone.
- **Softness and detail.** Every edge is at least about a pixel soft, and
  each detail fades out before it is smaller than a couple of pixels, so
  a small spot is a soft, irregular blot. The moving details (filaments,
  umbral dots) fade as a frame of the clock outlasts 5 minutes (a quarter
  of a dot's twinkle, `exposureFade`), as a long exposure averages them
  out; the surface at a day stays a function of the activity, the day and
  the view.
- **The area.** The drawn dark area of a region stays within 20 % of
  part 1's round spots at the same day (**calibration**), on two
  measures: the brightness under 0.9 of the quiet photosphere (before the
  limb law), and what reads as dark on screen, the displayed luminance
  (after the exposure curve and sRGB) under 0.9 of the local surface's.
  The second counts the umbra and pores only, since a penumbra of 0.75
  displays at about 0.94 of the photosphere. A first version of these
  shapes had a penumbra of mean 0.68, whose dark filaments crossed that
  threshold: it drew 1.3 to 1.9 times part 1's dark area on an M dwarf.
  The shape's scale (1.02, trimmed by 3 % once grown), the share the pores
  take from the spot (5 %), the penumbra's mean and floor and the umbra's
  radius were set so in a harness that renders a fixture region at ages
  0.1 to 0.9, at four sizes on screen and for 16 seeds, with an M4V's
  colour, limb and granulation. On the first measure the means lie within
  0.93–0.98 and every seed within 0.84–1.08; on the second the means lie
  within 0.87–1.13 (every seed within 0.84–1.20 down to spots of 40 px;
  under 0.8 for a few seeds at 11 px, where the umbra covers a few dozen
  pixels). A threshold between the two, at 0.95 of the photosphere's
  luminance, sits at part 1's penumbra itself and counts 0.7–0.85 of
  part 1's area: no filamentary penumbra matches a flat one there.

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
  it.** Its cells, lanes, blobs, wander speeds and the gain of
  1.6 on `granule_contrast` are calibrations matched to prototype D;
  the gain makes the drawn contrast (0.24 rms on the Sun) higher than
  the 0.15 the activity gives, so that the surface reads as agitated
  up close.
- **The supergranule network is slightly exaggerated** (**calibration**):
  its contrast of 0.15 on the Sun, rising about four times towards the limb,
  and its wide ribbons are above the real network's in white light, so
  that the whole disc shows texture where granules are under a pixel.
  Its size on other stars scales with the granules. It turns with the
  star's equator, without the differential rotation the regions follow.

- **The granules' bright points are dropped.** Prototype D drew the
  magnetic bright points of the lanes; cut by the lane they sat in, they
  read as specks on screen.
- **The cells move on a parabola wave.** Each centre wanders along
  `starWave`, a parabola per half turn that stays within 0.056 of a sine
  (**calibration**), because it is cheaper than a sine.
- **Granules are skipped inside spots.** They fade out across a spot's or
  a pore's edge and are not computed there, where real granulation is
  broken up by the field rather than absent (**calibration** of the fade).
- **The animated close-up is drawn at 0.75 pixel ratio** (**calibration**),
  the star view under 1.1 radii while its clock runs, a softer frame for
  about 44 % fewer pixels; once paused it is drawn sharp.
- **Timeline and region shapes are calibrations.** One cycle maps onto
  $x \in [0, 2.6]$ of the Hathaway fit; a region grows over
  $\min(0.1\,T,\ 3\ \text{d})$ times a factor within $\pm 20\ \%$; its
  polarities are $4\sqrt{2A}$ radians apart; the leader spot carries 0.6
  of the area.
- **The spots' shapes are calibrations.** Their filaments, torn
  outlines, umbral dots, light bridges, fragmentation and pores are a
  picture drawn after prototype D, not a model of a spot's magnetic
  structure; the Evershed flow has the measured speed only on a spot of
  10 Mm, and the pores do not move out with the moat flow. The penumbra's
  filaments keep to 0.7–0.88, a narrower contrast than real ones, so that
  the dark area on screen stays part 1's; the umbra's radius, 0.47 of the
  spot's, is a little above Solanki's fifth of the area for the same
  reason.
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
- the 30 Mm supergranule size (Leighton, Noyes & Simon 1962), its
  lifetime of about a day (Hart 1956; Rieutord & Rincon 2010), and the
  5 to 10 Mm mesogranulation (November et al. 1981);
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
44. Leighton, R. B., Noyes, R. W. & Simon, G. W. 1962, "Velocity fields
    in the solar atmosphere. I. Preliminary report", *The Astrophysical
    Journal* 135, 474.
45. Hart, A. B. 1956, "Motions in the Sun at the photospheric level. VI.
    Large-scale motions in the equatorial region", *Monthly Notices of
    the Royal Astronomical Society* 116, 38.
46. Rieutord, M. & Rincon, F. 2010, "The Sun's supergranulation",
    *Living Reviews in Solar Physics* 7, 2,
    [doi:10.12942/lrsp-2010-2](https://doi.org/10.12942/lrsp-2010-2).
47. November, L. J., Toomre, J., Gebbie, K. B. & Simon, G. W. 1981, "The
    detection of mesogranulation on the Sun", *The Astrophysical Journal*
    245, L123.
48. Evershed, J. 1909, "Radial movement in sun-spots", *Monthly Notices
    of the Royal Astronomical Society* 69, 454,
    [doi:10.1093/mnras/69.5.454](https://doi.org/10.1093/mnras/69.5.454).
49. Borrero, J. M. & Ichimoto, K. 2011, "Magnetic structure of sunspots",
    *Living Reviews in Solar Physics* 8, 4,
    [doi:10.12942/lrsp-2011-4](https://doi.org/10.12942/lrsp-2011-4).
50. Sheeley, N. R. Jr. 1969, "The evolution of the photospheric network",
    *Solar Physics* 9, 347,
    [doi:10.1007/BF02391657](https://doi.org/10.1007/BF02391657).

## See also

- [What does a G-type star mean?](star-types.md): the star's type,
  temperature, mass and radius this article starts from.
- [docs/system-spec.md](../system-spec.md): where the star's activity is
  stored in a system.
- Issue #343: the living star, of which this model is part 1.
