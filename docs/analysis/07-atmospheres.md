# Analysis: atmospheres

Backlog item 6. User request: generate and show planet atmospheres. Every
planet has the same fixed blue rim glow today (`#5c9dff` in
`PlanetView.ts`), airless worlds included, and the generator has no notion
of an atmosphere: the climate depends on the axial tilt and on the preset's
climate shift only.

## Decisions (2026-11-03)

Taken with the user:

- **Scope:** the atmosphere changes both the look and the climate
  (temperature contrast, greenhouse warming, liquid water). Worlds of
  presets whose atmosphere differs from Earth's change.
- **Clouds:** computed from the climate (rising air, rain, wind), exported
  as a layer, not free procedural noise.
- **Rendering:** physical single scattering (Rayleigh and Mie) in the
  viewer, instead of a stylised rim.
- **Flat map and `world.png`:** clouds are an optional layer (checkbox on
  the flat map, `raw_data/clouds.png`); `world.png` stays a map of the
  ground.
- **Temperatures in °C** (added the same day): the climate model, the
  biome thresholds and the preset shifts move to degrees Celsius, on
  Earth's real mean profile rather than a linear rescale of today's
  scale (step 0).

## 0. Temperatures in °C

Today temperature is a unitless `[0, 1]` scale, linear in annual
insolation: 0 at Earth's poles, 1 at its equator. Biome thresholds were
tuned on it, so biomes sit at the right latitudes (jungle around 17°,
taiga around 59° on `earth-like` worlds), but the scale is not linear in
real degrees: a linear `−18 + 45 t` would put the tropical threshold at
6.75 °C.

The model moves to °C with a curve fitted to Earth's annual zonal mean,
`T = −18 + 66.2 t − 21.2 t²` (−18 °C at the poles, 12 °C at 42°, 27 °C
at the equator), so each threshold keeps its latitude:

| Threshold                    | Today | °C    |
| ---------------------------- | ----- | ----- |
| Sea ice                      | 0.07  | −13.5 |
| Ice cap (`polar`)            | 0.10  | −11.6 |
| Boreal                       | 0.30  | 0     |
| Highland snow                | 0.35  | +2.6  |
| Tropical (above `temperate`) | 0.55  | 12    |

Additive terms become real degrees: the lapse rate, the continental
anomaly and the regional variability (scaled by the mean slope of the
curve, 45 °C per unit), the preset shifts and the erosion thaw band.
Their effect is no longer stronger near the poles than at the equator,
so worlds change slightly; biome shares are compared before and after
on every preset and the presets retuned where they drift.

`raw_data` moves to `format_version` 4 with the temperature layer in
°C (at the time, older worlds were converted on load; the generator no
longer reads worlds of formats before 14). The viewer shows °C.

## 1. The atmosphere of a world

A new `[atmosphere]` table in presets, stored in `world.json` with the
resolved preset:

```toml
[atmosphere]
pressure = 1.0                     # surface pressure in bar; 0 = airless
rayleigh = [0.17, 0.40, 1.00]      # relative scattering per channel (sky colour)
haze = 0.1                         # aerosol (Mie) density, 0 to 1
haze_colour = [1.0, 0.95, 0.9]     # aerosol tint
greenhouse = 0.0                   # added to temperature, °C
clouds = "water"                   # water | sulfuric | dust | none
cloud_cover = 0.0                  # shift of the cloud cover, -1 to 1
```

`pressure` also becomes a generation parameter (`--pressure`, form field),
drawn from a preset range like the sea level, so a frozen world can be thin
or thick. Presets without the table get Earth's values: terran worlds do
not change.

| Preset         | Pressure (bar) | Sky                            | Clouds               |
| -------------- | -------------- | ------------------------------ | -------------------- |
| `terran`       | 0.7–1.5        | blue                           | water                |
| `earth-like`   | 1.0            | blue                           | water                |
| `frozen`       | 0.3–1.0        | pale blue                      | water (thin)         |
| `volcanic`     | 5–20           | orange, dense haze             | sulfuric, patchy     |
| `caustic`      | 2–5            | yellow-green haze              | sulfuric (acid)      |
| `barren`       | 0–0.01         | none or a faint haze           | none                 |
| `mars-like`    | 0.006          | butterscotch dust, blue sunset | dust, thin           |
| `venus-like`   | 92             | yellow-white                   | sulfuric deck, total |
| `mercury-like` | 0              | none                           | none                 |

## 2. Climate effects (generation)

Temperatures are in °C (step 0).

- **Heat transport.** A dense atmosphere carries heat to the poles. The
  row temperature becomes `mean + (row − mean) × contrast`, with
  `contrast = 1 / (1 + k × (pressure − 1))` calibrated to 1 at 1 bar:
  Venus (92 bar) is nearly uniform, Mars (0.006 bar) keeps Earth's
  contrast at most (clamped, its caps already come from the low
  temperature). The mean is the area-weighted mean of the rows, so the
  global temperature does not move.
- **Greenhouse.** `greenhouse` is added to every pixel. The `volcanic`
  and `venus-like` presets move part of their `[climate] temperature`
  shift into it, so their biomes stay as tuned.
- **Lapse rate.** Unchanged: it already gives mountains their snow.
- **Liquid water.** Below about 0.01 bar (the triple point of water is
  0.006 bar) no water stays liquid: moisture is forced to −1, rivers and
  hydraulic erosion are switched off. Today `mercury-like` and
  `venus-like` set `erosion = 0` by hand; this makes it a consequence of
  the atmosphere. Planets with air but no water (`barren`) keep their
  preset remap.

Every seed of a non-Earth preset changes once, in the step that brings
these effects.

**Outcome (step 3).** `contrast = 1 / (1 + 0.1 (p − 1))` above 1 bar
(the steeper `k` first considered flattened 5–20 bar worlds too much).
With temperatures in real degrees, the two planets named after real ones
were set to their real means: `venus-like` gets a greenhouse of 450 °C
(448–475 °C at the surface), `mars-like` a climate shift of −60 °C
(−87 to −21 °C) with its ice thresholds lowered to −70 °C, which keeps
its caps exactly. `volcanic` moves its +20 °C into its greenhouse.
`mars-like` and `barren` set `erosion = 1`. On 18 worlds of the nine
presets, biome shares move by at most 1.4 points (ash land on volcanic
worlds): the dense presets remap almost every biome.

## 3. Clouds (generation)

A cloud cover in `[0, 1]` per pixel, exported as `raw_data/clouds.png`
(8-bit, `format_version` 5; older worlds have no clouds):

- **Rising air** from the moisture model: the circulation band (equatorial
  convergence, storm tracks around 60°), the precipitation rate and the
  orographic lift on windward slopes; **sinking air** clears the desert
  belts around 30° and the lee of mountains.
- **Texture:** a few octaves of noise stretched along the zonal wind
  (cloud streets and bands), with a seed stream of its own.
- **Kinds:** `water` follows the rule above; `sulfuric` adds a uniform
  deck with zonal bands (total on Venus); `dust` is low and patchy, with
  a few regional storms; `none` exports nothing.
- `cloud_cover` shifts the whole field, for drier or cloudier worlds.

**Outcome (step 4).** Two fixes were needed on Earth-like worlds: the
poles came out clear, first because the north-south squeeze of the noise
folded the whole polar cap into a single noise value (the squeeze now
fades towards the poles), then because the circulation band and the low
polar rain both meant sinking dry air there, while the Arctic is about
60 % overcast with low stratus (the rain term now fades out towards the
poles and a polar stratus term is added). Cover then matches Earth's
latitude profile and its two-thirds mean.

**Review (after step 5).** On the globe the water clouds read as too many
and too linear near the equator: wide overcast blobs, and east-west
streaks where the ×3 squeeze was strongest, at the equator. The noise is
now warped by a larger noise (swirls and fronts), squeezed only at
mid-latitudes (×2 at 45°, none at the equator or the poles), and broken
into finer cells, stronger in the tropics (convective clusters); the
equatorial convergence is a narrow band of its own, and the cover ramp is
wider (`smoothstep(0, 0.9)`), so cloud fields have thick cores and thin
edges. The mean cover drops to 0.42–0.49 (about 30 % of the ground hidden
in the viewer, half before): Earth's two-thirds counts thin cirrus that a
satellite image barely shows.

## 4. Rendering (viewer)

- **Sky shell.** The rim glow becomes a single-scattering shader on a
  shell around the globe: per pixel, the view ray is intersected with the
  shell and marched in about 16 steps, each sampling the sunlight
  attenuated along about 8 steps towards the sun; Rayleigh (`rayleigh`
  coefficients, scaled by `pressure`) plus Mie (`haze`, phase
  `g ≈ 0.76`). The shell thickness is exaggerated (a few percent of the
  radius instead of 0.1 %) so the halo reads at planet scale. With the
  headlight the sun follows the camera; with the sun, the terminator gets
  its twilight band.
- **Aerial perspective.** The surface shader applies the same model
  along the view ray to the ground: distant and grazing ground takes the
  sky colour and loses contrast. On Venus the ground is hidden.
- **Clouds.** A second sphere just above the ground, textured with
  `clouds.png` (opacity = cover, tint by kind), lit by the sun, casting
  shadows on the ground (the surface shader samples the cloud layer
  offset towards the sun), with an optional slow drift by latitude
  following the zonal wind. A "Clouds" toggle and shortcut; they fade out
  when zooming close in the deep zoom.
- **Flat map.** No sky; the cloud layer as an optional overlay.
- **Outcome (step 5).** The clouds are drawn in the surface shader rather
  than on a second sphere: one code path for the globe, the flat map and
  the detail tiles (which share its uniforms), and the sky shell veils
  them like the ground. The cover goes through `smoothstep(0.3, 1)`
  (up to 0.85 opacity) so thin clouds stay translucent; at full opacity a
  two-thirds cover hid the continents. The water cover was also eased
  (narrower equatorial band, 0.75–0.8 instead of 0.9). The drift by
  latitude is not done: it would need a frame every few seconds, against
  the render-on-demand loop.
- **Drift (after step 5).** A solid eastward rotation of the cloud layer
  instead of a drift by latitude: a per-latitude speed would shear the
  layer into streaks within minutes. It is optional (off by default) and
  cheap: the texture lookup is offset by a uniform, and while the clouds
  drift the viewer redraws every 100 ms rather than every frame, since a
  turn takes five minutes (a few pixels a second at the equator).
- **Airless worlds** draw no shell: a hard limb against space.
- **Review (after step 5).** At full relief, mountains rise 0.06 radius
  above the ground, above the 0.036 of Earth's shell, and the shader's
  ground was the unit sphere, drawn without depth test: the halo was
  painted over the peaks at the limb. The shell now rises with the relief
  exaggeration (scale heights stretched, coefficients divided by the same
  factor, so the zenith optical depths are kept), and it is drawn twice
  from its far side, split by the depth test: where the surface is in
  front, the ray stops at the ground (or, for a peak above the unit
  sphere, at its closest point to the planet) and veils it; elsewhere it
  draws the sky, now hidden by the peaks in front of it.
- **Calibration (step 2).** Shell thickness `0.03 + 0.02 log10(1 + p)`
  of the radius; zenith optical depth `0.25 p^0.35` for the gas (blue
  channel) and `2 × haze` for the aerosols, so dense atmospheres take
  the colour of their haze rather than a deep blue; aerosol anisotropy
  from 0.76 (clear) down to 0.26 (thick haze, which scatters many times).
  The preset's `rayleigh` sets the gas colour with its absorbers: Earth
  blue, Venus yellow-white, volcanic orange, caustic yellow-green.

The viewer only draws when something changes, so the shader's cost is
paid per frame of interaction only.

## 5. Order

Each step is a pull request:

| Step | Change                                                                                    | Seeds  |
| ---- | ----------------------------------------------------------------------------------------- | ------ |
| 0    | Temperatures in °C: Earth-fitted curve, thresholds and presets in °C, `format_version` 4  | Change |
| 1    | `[atmosphere]` in presets and `world.json`, `--pressure`, form field; no effect yet       | Same   |
| 2    | Viewer: scattering sky shell and aerial perspective from the world's atmosphere           | Same   |
| 3    | Climate: heat transport, greenhouse, no liquid water below 0.01 bar; presets recalibrated | Change |
| 4    | Cloud layer: generation, `raw_data/clouds.png`, `format_version` 5                        | Same   |
| 5    | Viewer: cloud sphere, cloud shadows, drift, flat-map overlay, toggle                      | Same   |

Steps 1 and 2 already replace the identical blue rim by each planet's own
sky; step 3 is the only one that changes worlds.

## 6. Open points

- Elevation has no unit yet: the lapse rate is in °C per elevation unit,
  not per km.
- Scattering at the exaggerated shell thickness is tuned for looks: real
  coefficients would give a halo invisible at planet scale.
- Cloud shadows in detail tiles: the tiles are rendered by the server,
  so shadows would be applied by the viewer on top, like the base map.
