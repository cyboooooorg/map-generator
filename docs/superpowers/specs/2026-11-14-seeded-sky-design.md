# Seeded sky with gas and dust clouds: design

Issue #429. Decided with the user on 2026-11-11 (issue comment) and
2026-11-14 (this conversation).

## Intent

Every view draws the same sky today: `starField()` in
`web/src/lib/viewer/stars.ts:49` defaults to seed `7` and none of its three
callers passes one. Moving to another system should feel like being
somewhere else. For the user, two things carry that feeling:

1. **The gas clouds and nebulae differ** in shape and colour from one
   system to another. This is the main signal.
2. **The sky is brighter or darker** with the system's place in the
   galaxy: brighter toward the galactic core, darker in the outskirts.

The sky must not conflict with the future galactic map (#420, track E
#421). That map places real nearby stars, and drawn systems will be given
a place on it later. So the sky is not a function of the seed alone: it is
a function of the system's **galactic position**, stored in the system,
and the seed only drives what is really random (the fine shape of the
clouds, the individual stars).

## Goals

- Each system has a galactic position, stored in `system.json`, drawn at
  creation. The galactic map will read the same field later. A system from
  a catalogue will write its real position there.
- The viewer's sky derives from that position plus the system seed:
  - the brightness and density of the Milky Way band, and the visible
    bulge toward the galactic centre, from the position;
  - the band's orientation, from the system's orientation relative to the
    galactic plane (also stored);
  - the shapes and colour mix of the clouds and the individual stars,
    from the seed.
- Faint gas and dust clouds: emission nebulae (red H-alpha, blue-green
  O III), blue reflection nebulae, dark dust lanes that dim the band and
  the stars behind them, concentrated along the band like the real
  galactic plane.
- Every view of a system (planet and moons in `PlanetView`, the system in
  `StarView`, the star close-up in `StarBodyView`) shows the same sky.
- A viewer control with three choices: the system's sky (default), the
  current fixed sky, no sky.

## Non-goals

- The galactic map itself (#420), and assigning drawn systems a place on
  it.
- Real catalogue positions (#419, #420) — only the field they will fill.
- Old stored systems: no care is needed for them (user, 2026-11-14). See
  "Format".
- A sky reroll or seed override in the viewer (may come later).
- Real constellations or a real Milky Way photograph for the Solar System.

## Anti-goals

- Clouds that compete with the planet: they are a backdrop, faint behind
  it.
- Clouds that fight the star close-up's photo filter (#415).
- A noise shader evaluated every frame (decided 2026-11-11: bake once).
- A drawn position that the galactic map would have to recompute or
  contradict: one stored field, one source.

## Decisions

### Galactic position (generator, Rust)

A new block on `System` (`crates/mapgen/src/system.rs:136`), for example
`galaxy: GalacticPlace`, in a new module `crates/mapgen/src/galaxy.rs`:

- **Position**: galactocentric Cartesian coordinates in kpc, the convention
  the galactic map will use (Sun at about R = 8.2 kpc, z = +0.02 kpc).
- **Orientation**: how the system's own frame sits in the galactic frame:
  enough to give, in the system frame, the direction of the galactic north
  pole and the direction of the galactic centre. **The system frame is the
  star's equatorial frame**, the one planets' orbit inclinations are given
  against (`Orbit.inclination_deg`, system.rs:247), with the reference
  direction of the nodes (`node_deg`) as its x axis (builder's call
  noted below).
- **Sky seed**: a `u32` drawn from the same stream, which the web uses for
  the clouds and the individual stars, so that systems with neighbouring
  seeds do not look alike and the web needs no hash of its own.
- **Source**: `drawn` now; `observed` for a catalogue system later (the
  provenance principle of #421, kept to this one block).

How it is drawn, for a whole system (`place_whole`, system.rs:821) and a
system around a planet (`WorldRequest::resolve`, request.rs:558):

- From its own rng stream (a new constant in `noise::stream`, next to
  `METALLICITY = 27`; 43 is free), seeded by `stream_seed(seed, stream)`
  like the metallicity's (giants.rs:195) and shared with no other draw, so
  that no existing draw shifts: every other value of a system stays
  identical. All the draws of the block (radius scatter, height, azimuth,
  orientation, sky seed) come from this one stream, in an order the
  builder picks. It needs the metallicity, so it runs once that is known.
- **Galactocentric radius from the metallicity** (user's choice): invert
  the radial metallicity gradient of the disc, about −0.06 dex/kpc
  (Cepheids, Genovali et al. 2014), plus a scatter (the spread of [Fe/H] at
  a given radius, about 0.1 dex), clamped to the disc (about 2 to 16 kpc).
  The generator's mean metallicity (−0.1, giants.rs:90, the solar
  neighbourhood's) maps to the Sun's radius, 8.2 kpc, so a typical system
  has a sky like ours and markedly brighter or darker skies are each about
  one system in six (radius within about 5 kpc, or beyond about 11 kpc).
  A metal-rich star tends to sit closer to the core, so its sky tends to be
  brighter. The metallicity used is the system's, drawn or set in the
  request (`metallicity` of a request sets the radius too).
- **Height above the plane**: drawn from the thin disc's vertical
  profile (exponential, scale height about 300 pc).
- **Azimuth** around the centre: uniform.
- **Orientation**: uniform random rotation.

### Format

- The block is always written. No `SYSTEM_FORMAT_VERSION` bump: on read,
  a system without it gets one drawn from its seed and its stored
  metallicity (`star.metallicity`), with the same stream, as
  `read_system` already fills a missing `star.activity` (system.rs:394).
  This keeps clear of the bump to 5 in #465 (planet model part 4).
- ts-rs exports the type to `web/src/lib/api/types/`; the web reads it
  from `SystemDetail` (`fetchSystem`, `world.svelte.ts:520`).

### Sky (web)

- **Frames**: the cube map and the star directions are built once in the
  system frame (the star's equator). Each view rotates them into its own
  scene frame, as `PlanetView` already turns its stars about the spin axis
  (`setStarsTurn`, PlanetView.ts:1902): a planet's view applies its
  obliquity to the star's equator, a moon's view also its orbit's tilt to
  its planet; the system and star views use the star's equator directly.
  The sky stays fixed against the stars, never turning with the globe.
- **One sky per system, baked once** into a cube map (decided
  2026-11-11) when the system opens, and shared by its views. The clouds
  are in the cube map. The stars stay points over it (sharp at any
  resolution), drawn by `starField`, which now takes the seed and the
  galactic place.
- **Brightness from the position**: the band's surface brightness and
  star density follow the disc's density at the system's radius
  (exponential disc, scale length about 2.6 kpc) relative to the Sun's;
  the bulge toward the galactic centre grows as the radius shrinks; far
  from the plane, the band narrows and the sky outside it empties.
- **Band orientation** from the stored orientation, replacing the fixed
  `BAND_TILT` = 60°.
- **The seed** of the clouds and stars is the block's sky seed, not
  `system.seed`.
- **Dust dims what is behind it**: the dark lanes darken the band in the
  cube map and also the star points behind them.
- **Faint**: the clouds stay well below the band's brightness and below
  the planet. The photo filter (#415) is a CPU change of the star's own
  light (`StarBodyView.applyLight`, StarBodyView.ts:304) and leaves the
  sky alone; the clouds must simply stay faint there too.
- **Hidden by the host star's disc** in `PlanetView` as the stars are
  (`HIDE_HOST_STAR`, shaders.ts:775): drawn behind the stars (render
  order below −3) and behind the disc.
- **Compared worlds each keep their own sky**, even with linked cameras.
  The comment in `stars.ts` saying they share the same stars is updated.
- **A world without a system** (standalone, `world.system` null) shows
  the fixed sky.

### Sky control

- The stars toggle (`Viewer.svelte:1904`, display key `stars`, shortcut
  `b`, linked display) becomes a three-way choice: the system's sky
  (default), the fixed sky (today's), no sky. A stored boolean migrates
  (`true` to the system's sky, `false` to none) in `migrateDisplay`.
- It applies to all three views; `StarView` and `StarBodyView` have no sky
  toggle today and get one.

## Calls made by the author (not discussed with the user)

- The system frame is the star's equator, x along the nodes' reference.
- A stored sky seed rather than a hash of `system.seed` in the web.
- The mean metallicity maps to the Sun's radius (a typical sky is ours).
- A request's explicit metallicity also sets the radius.
- A stored stars toggle migrates to the system's sky or to none.

## Left to the builder

- The cube map's resolution and whether it is baked on the GPU (render to
  cube target) or the CPU; how long the bake may take as long as opening a
  system does not visibly stall.
- The cloud noise (fBm, domain warping…), the exact colours, the cloud
  amount and how much each colour family varies from seed to seed, within
  "faint" and "different shape and colour per system".
- The exact numbers of the gradient, scatter, clamps and brightness laws
  above (starting points given), and the exact serialization of the
  orientation (quaternion, or two unit vectors).
- How the views receive the sky (a new setter, or a shared object built in
  `Viewer.svelte`).
- How dust dims the stars (sampling the cube map in the stars' shader, or
  computing it in `starField`).

## Done when

- Two systems with different seeds show different clouds; the same system
  always shows the same sky, in every view.
- A system near the core has a visibly brighter band and bulge than one in
  the outskirts.
- Clouds are visible but discreet; dust lanes darken the band.
- `system.json` stores the galactic place; every other value of a
  regenerated system is unchanged (world-coherence check).
- The viewer offers the sky choice in every view.
- Tests: Rust (deterministic draw, radius tied to metallicity, no shifted
  draws, missing block filled on read); web (`stars.test.ts` covers the
  seeding, the position's effect on brightness, the band orientation).
- Documentation: README (system file and viewer sky), `docs/system-spec.md`
  (the new block), the viewer's help text.

## Addendum, 2026-11-14: galactic core, nebulae, two renderings

After a first look on develop, the user found no visible galactic core,
clouds too alike from one system to the next, and no nebula with a
recognisable shape (they cited the Helix, Carina and Eagle nebulae). The
user asked for two renderings with a switch: a realistic one and a
photographic one.

### Galactic core

A real bulge, not a denser patch of stars: an elongated glow centred on
`centre`, flattened along the band (axis ratio about 0.6), crossed by a
dark dust lane along the plane, as Sagittarius seen from Earth. Its
angular size and brightness follow the distance to the centre (bigger
and brighter closer in, `(8.2 / R)²` as a starting law), and it stays
clearly visible from the Sun's distance.

### Nebulae as objects

Each system draws a catalogue of nebulae from its sky seed: families of
real nebulae, never copies of real ones.

| Kind | Look | Angular size | Where |
|---|---|---|---|
| Planetary | ring or bipolar, blue-green core (O III), red rim (H-alpha) — the Helix's family | 0.3–2° | anywhere, mild band bias |
| Emission complex | red-pink glow torn by dark lanes — Carina's family | 3–12° | in the band |
| Pillars | dark columns with bright rims against emission — the Eagle's family | 1–4° | in the band |
| Supernova remnant | filamentary shell, red and blue-green | 2–8° | anywhere, band bias |
| Reflection | blue haze round a bright star | 0.5–3° | in the band |
| Dark | black patch on the band, dims the stars behind | 1–6° | on the band |

The count of each kind, the positions, sizes, orientations, shapes and
colour mixes vary with the seed; more nebulae toward the core (with the
look's brightness). Each nebula has its own texture, baked at a
resolution fitting its size, drawn as a patch of the sky sphere fixed in
the sky (not facing the camera), between the cloud cube and the stars.
Dark nebulae and pillar silhouettes also dim the stars behind them.

### Two renderings

- **Naked eye** (realistic): what the eye sees — the faint ceiling of
  the first version, colours mostly washed out, nebulae barely there
  except the brightest.
- **Long exposure** (photographic): what an astrophotograph shows —
  saturated H-alpha reds and O III blue-greens, clear structure, a bright
  bulge, more stars. Still a backdrop: drawn behind the planet, never in
  front, and the star view's photo filter (#415) stays unaffected.

A switch next to the sky choice picks the rendering for the system's sky
in every view (saved with the display settings, linked between compared
worlds like the sky choice). The fixed sky stays today's sky in both.

Calls by the author: long exposure is the default (the user's references
are photographs); the switch is a separate on/off "Long exposure" toggle
rather than two more sky choices.

### Tuning after the second visual check, 2026-11-14

Found in a real GPU browser, fixed on the same branch:

- **Sharp, bold nebulae.** A texture's resolution follows the planet view's
  field of view (40 degrees over a 1800 px buffer, a texel no wider than a
  screen pixel, a power of two from 128 to 1024). Planetary rings are
  thicker and brighter with a faint outer arc ring; emission complexes have
  broad dark bays with lit edges and thin lanes; pillars are fat columns
  with domed tips and lit rims; remnants are thin bright filamentary
  shells. Sizes grew for the planetary (0.5 to 3 degrees), pillars (1.5 to
  5) and dark (1.5 to 7) kinds.
- **Dark nebulae** are elongated, bent, with warped ragged edges and a few
  tendrils, and lie along the band; the texture and the star dimming read
  the same `darkOpacity`.
- **The core** is a warm yellow-white glow (a cuspy profile, a soft curve
  rather than a clip, its size growing as the square root of the distance
  ratio, at most 0.6 rad) with its own dust lane applied after the curve,
  so the lane shows at every radius.
- **Long exposure** pushes colour saturation (1.5), uses a dimmer warm
  dust glow in clumps with dark gaps, deeper H-alpha and O III, fewer
  reflection patches and sharper emission patches; the naked eye is as
  before. A long exposure draws 1.6 times the sky and band stars (the
  fixed sky is the same in both).
- **The band's edge** is bent by slow and medium noise (about 0.6 and 0.25
  of the glow's spread), so it is never a smooth great circle. The cloud
  cube is baked at 512 px.

### Tuning after the third visual check, 2026-11-14

Reference for the look only (not copied): a wide-field Milky Way
photograph, in which the galaxy is made mainly of stars: countless faint
ones, a milky glow of unresolved stars, a dense yellowish bulge, thick dark
rifts with many branching filaments and only a few small pink emission
spots.

- **Nebulae spread over the sphere.** The band-drawn share is 50 to 80% per
  kind with a latitude spread of 8 to 20 degrees (standard deviation); the
  rest is uniform on the sphere (the dark kind keeps 80% within 30 degrees,
  the others up to 60 to 90). Centres are drawn by dart throwing: two
  nebulae are at least the sum of their radii plus `NEBULA_MARGIN_DEG`
  (3 degrees) apart, the later tries ignoring the band. Each kind keeps
  its own sub-stream; a nebula that finds no room is dropped.
- **Star density follows the galaxy.** A system's star field
  (`seededStarCounts`: 6000 sky stars times the root of the brightness, up
  to 48000 band stars with the brightness clamped to 0.4 to 1.6, 28000
  core-swarm stars times the root of the core brightness, 6000 bulge
  stars times the bulge, capped at 150 000 in total, the long exposure's
  1.6 applying to the sky and the band) has a band thicker on the
  centre's side with a wider halo, an elongated warm swarm on the core, and
  dense populations that are fainter and mostly 1 px. The density per
  steradian at the core is well over four times that 90 degrees along the
  band at the Sun's place. The fixed sky keeps its old stars unchanged.
  The glow is lower (the stars carry the band) and has a fine grain
  (a 0.5 degree value noise) so it reads as unresolved stars.
- **Branching rifts.** `dustAt` takes its band dust from `rifts`: a warped
  ridged noise (trunk), finer ridges that grow only beside a trunk (side
  branches and twigs) and a faint haze, wider in latitude and stronger
  toward the core. The core's own lane is three strands that wander about
  the plane and fork instead of one line. Dust dims the glow and the
  stars behind it.
- **Colour mostly in round nebulae.** The band's own coloured patches are
  18% of what they were; instead each non-dark nebula has a faint
  coloured halo (2.4 radii, cloudy, H-alpha red for emission, pillars and
  remnants, O III teal for planetary, blue for reflection) baked into the
  cube wherever it lies on the sphere. The band's glow is warmer and whiter.

### Tuning after the fourth visual check, 2026-11-14

At the planet view's scale a pillars nebula is about 400 px across, and the
soft oval columns and the single dark lane at the core looked crude.

- **Pillars (the Eagle's family).** `pillarColumns` draws two or three main
  columns that taper from a wide foot to a narrow, bent, wavering finger
  tip (`tip` half width 0.035 to 0.06), one or two thin side fingers per
  column, and one to three small detached globules above each tip.
  `pillarDistance` is a signed distance to that union with the outline
  torn by a warp at three scales; `pillarMask` is its sharp-edged
  opacity (a few texels of softness, a slightly mottled inside) and drives
  both the texture and the extinction. The texture shades the outline with
  the direction of the light (up and a little right): a thin white-hot
  line just inside the edge, a red front around it, an O III teal glow
  beyond, all strongest where the outline faces the light; compressed
  bright gas on the lit side; a dim, wispy inside (ridged noise) fading
  toward the foot; and a brighter background with filaments and grain.
- **Other kinds, finer detail.** Planetary: knots at two scales with fine
  filaments. Emission: grain and thin bright filaments on top of the
  lanes. Remnant: thin extra filaments and grain. Reflection: wispy
  streaks in the haze. Dark: edge torn at three scales, clumps with holes,
  a fibre network and a fringe of hair-like wisps, so it is no longer a
  smooth oval.
- **The core has no straight lane.** `coreLane` no longer follows the
  plane. It is a network of domain-warped ridged noise on the sphere (about
  two rift spacings across the core, widths varying along each lane,
  side filaments growing only next to a trunk), so several irregular lanes
  cross the bulge at different angles. Its envelope is the core's own
  ellipse (a flat-topped fall to zero): the rifts end with the core.
- Bake (worker, whole sky): R 4.5 about 7 s, R 8.2 about 5 s (9.9 and 7.5
  Mpx of textures).
