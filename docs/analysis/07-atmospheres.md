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

## 1. The atmosphere of a world

A new `[atmosphere]` table in presets, stored in `world.json` with the
resolved preset:

```toml
[atmosphere]
pressure = 1.0                     # surface pressure in bar; 0 = airless
rayleigh = [0.17, 0.40, 1.00]      # relative scattering per channel (sky colour)
haze = 0.1                         # aerosol (Mie) density, 0 to 1
haze_colour = [1.0, 0.95, 0.9]     # aerosol tint
greenhouse = 0.0                   # added to temperature, [0, 1] scale
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

The temperature scale stays `[0, 1]` (0 = Earth's poles, 1 = its equator).

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

## 3. Clouds (generation)

A cloud cover in `[0, 1]` per pixel, exported as `raw_data/clouds.png`
(8-bit, `format_version` 4; older worlds have no clouds):

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
- **Airless worlds** draw no shell: a hard limb against space.

The viewer only draws when something changes, so the shader's cost is
paid per frame of interaction only.

## 5. Order

Each step is a pull request:

| Step | Change                                                                                       | Seeds  |
| ---- | -------------------------------------------------------------------------------------------- | ------ |
| 1    | `[atmosphere]` in presets and `world.json`, `--pressure`, form field; no effect yet          | Same   |
| 2    | Viewer: scattering sky shell and aerial perspective from the world's atmosphere              | Same   |
| 3    | Climate: heat transport, greenhouse, no liquid water below 0.01 bar; presets recalibrated    | Change |
| 4    | Cloud layer: generation, `raw_data/clouds.png`, `format_version` 4                           | Same   |
| 5    | Viewer: cloud sphere, cloud shadows, drift, flat-map overlay, toggle                         | Same   |

Steps 1 and 2 already replace the identical blue rim by each planet's own
sky; step 3 is the only one that changes worlds.

## 6. Open points

- The `[0, 1]` temperature scale has no unit, so "greenhouse" values are
  tuned by eye per preset; a calibration in kelvin would be a separate
  change.
- Scattering at the exaggerated shell thickness is tuned for looks: real
  coefficients would give a halo invisible at planet scale.
- Cloud shadows in detail tiles: the tiles are rendered by the server,
  so shadows would be applied by the viewer on top, like the base map.
