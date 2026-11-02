# Analysis 03: rivers are too straight and too thin

Backlog item 3. This note measures the problem, compares four families of
fixes on a prototype, and recommends a combination with a commit plan.

## Method

- Worlds: `--planet terran --sea-level 0.2 --volcanic 0.3 --circumference
  40000 --axial-tilt 23.44`, seeds 3, 7 and 42, default 1920 × 1080 (one
  pixel is about 20.8 × 18.5 km at the equator).
- Prototype: a scratchpad copy of the repository with a throw-away module
  (`crates/mapgen/src/rivers_proto.rs`) driven by environment variables. It
  traces river polylines from the D8 receivers, computes the metrics below,
  smooths and meanders the polylines, draws them anti-aliased with a
  flow-scaled width, writes a `rivers.json` and renders 6× zoomed crops.
  Nothing in this repository was changed except this file. The copy is in
  `/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/rivers/repo/`.
- Metrics, per world:
  - **heading histogram** and share of length within 5° of a D8 direction;
  - **mean straight run**: consecutive steps in the same 45° bin;
  - **whole-line sinuosity**: path length / great-circle chord, per traced
    line with a chord of at least 300 km;
  - **local sinuosity**: same ratio on 8 px (about 160 km) windows, plus the
    share of windows that are straight (below 1.02);
  - **uphill share**: samples whose bilinear filled height rises more than
    0.002 above the running upstream minimum;
  - **crossings** between polylines and inside one polyline.

Images (absolute paths, base-resolution crops are 600 × 400 px of seed 3,
zooms are 150 × 100 px upscaled 6×), all under
`/private/tmp/claude-501/-Users-cyborg-Developer-cyboooooorg/db9e0dd5-d68f-44f9-adc0-ba2d4971c606/scratchpad/agents/rivers/final/`:

| File                              | Content                          |
| --------------------------------- | -------------------------------- |
| `base_d8_crop.png`                | Current, base resolution         |
| `vector_crop.png`                 | Option C + D                     |
| `combo_crop.png`                  | Recommended (A on flats + C + D) |
| `base_d8_zoom6x.png`              | Current, 6× zoom                 |
| `routing_noise_only_zoom6x.png`   | Option A alone, 6×               |
| `vector_zoom6x.png`               | Option C + D, 6×                 |
| `combo_zoom6x.png`                | Recommended, 6×                  |
| `base_d8_coast_zoom6x.png`        | Current, coast and lake, 6×      |
| `combo_coast_zoom6x.png`          | Recommended, same place          |
| `artifact_confluence_hairpin.png` | Kink before the guards           |

The "recommended" images and the A + C column below apply the routing noise
everywhere (amplitude 0.0015, frequency 150), not only on flats: the
flats-only variant was not prototyped.

## Root cause

| Metric (seeds 3 / 7 / 42)    | Current D8          | Real rivers |
| ---------------------------- | ------------------- | ----------- |
| Length on a D8 direction     | 100 %               | n/a         |
| Cardinal share (E, S, W, N)  | 56 % (seed 3)       | isotropic   |
| Mean straight run            | 3.4 / 3.5 / 3.6 px  | n/a         |
| Whole-line sinuosity, median | 1.22 / 1.22 / 1.22  | 1.2 to 2+   |
| Same after Chaikin smoothing | 1.19 (seed 3)       |             |
| Local sinuosity, median      | 1.06 / 1.06 / 1.06  | 1.3 to 2    |
| Straight 160 km windows      | 23 % (Chaikin 28 %) | rare        |
| River pixels on filled flats | 41 / 43 / 45 %      |             |
| Drawn width                  | 1 px for any flow   | ∝ Q^0.5     |

Four causes add up:

1. **D8 quantisation.** Every step is one of 8 headings. A river crossing the
   grid at 20° becomes a staircase of straight and diagonal runs. The
   staircase inflates the measured sinuosity (a straight line at 22.5° in D8
   measures 1.08) without looking sinuous: once Chaikin removes the
   staircase, sinuosity drops to 1.19 and the share of straight windows
   rises to 28 %. The 1.22 headline number is therefore mostly grid noise.
2. **Routing across filled flats.** 41 to 45 % of river pixels sit on
   surfaces raised by the priority-flood. There the receiver is decided by
   `FLAT_EPSILON` and the flood order, which is a chessboard-distance front
   from the spill point: rivers run in perfectly straight cardinal or
   diagonal lines towards the outlet. These are the longest straight runs on
   the map (`base_d8_zoom6x.png`, the two vertical rivers).
3. **No sub-pixel or meander scale.** Real meander wavelengths are about
   10 to 14 channel widths (Leopold and Wolman, 1960). With real widths of
   0.1 to 10 km, all meanders are below one 20 km pixel: the generator has
   no scale at which they could appear.
4. **Width is a biome, not a line.** `Biome::River` replaces one pixel per
   river cell, so a 6 million km² basin is drawn like a 120 000 km² one.
   Diagonal steps even look thinner (pixels touch only by a corner).
   Hydraulic geometry gives `w = a·Q^b` with `b ≈ 0.5` (Leopold and
   Maddock, 1953). River-pixel flows span 120 000 to 6 million km²
   (6 096 / 4 710 / 3 279 / 1 942 / 789 / 202 pixels in octaves from
   120 k to over 4 M km² on seed 3), so widths should differ about 7×.

The flow raster and the drainage network themselves are sound: every river
reaches the sea or a lake, flow grows downstream, lakes are where pits are.
The problem is the geometry of the drawn line, not the hydrology.

## Options

### A. Perturb the routing surface

Add fractal noise to the routing height before the priority-flood
(prototype: Perlin on the sphere, 3 octaves, base frequency 60 to 200).

| Amplitude @ freq. | Straight run | Whole sin. | Straight win. | Max flow |
| ----------------- | ------------ | ---------- | ------------- | -------- |
| none              | 3.41         | 1.22       | 23.1 %        | 6.03 M   |
| 0.001 @ 200       | 2.07         | 1.28       | 10.2 %        | 5.78 M   |
| 0.003 @ 200       | 1.71         | 1.37       | 3.8 %         | 6.82 M   |
| 0.008 @ 200       | 1.51         | 1.39       | 1.1 %         | 6.92 M   |
| 0.003 @ 60        | 2.36         | 1.29       | 11.1 %        | 8.34 M   |

- Visual gain: breaks the long straight runs and gives rivers a wandering
  course (`routing_noise_only_zoom6x.png`). Still 1 px, still a staircase.
- Plausibility: rivers follow the perturbed surface, so they can climb the
  real one by up to the noise amplitude. Applied everywhere it moves divides:
  the largest basin of seed 3 grows from 6.0 to 7.6–8.3 M km² (river
  capture), and amplitudes near `LAKE_MIN_DEPTH` (0.004) create new pits and
  lakes. Restricted to **filled flats** (the 41–45 % above), it changes only
  the path across the flat, never the divides or the lakes.
- Consistency: flow raster, biomes, hex flags and the viewer all stay
  consistent because they all derive from the same receivers.
- Cost: one noise sample per pixel, negligible. Complexity: low.

### B. Different routing: D-infinity, multiple flow, finer grid

- **D∞ / MFD** (Tarboton 1997, Freeman 1991) spread flow between two or more
  receivers. They improve accumulation on planar slopes, but they produce
  diffuse flow bands, not channels: a single path still has to be extracted
  for drawing, and it is again on the pixel grid. Not prototyped; no visual
  gain for the cost of rewriting accumulation and lakes.
- **Finer routing grid** (2× or 4×) moves the staircase to a smaller scale
  but multiplies the hydrology cost (already about 1 s of 1.8 s, backlog
  item 4) by 4 or 16, and still has 8 headings and no meanders. The deep
  zoom needs detail far below any affordable grid anyway.

Rejected as the main fix.

### C. Vector post-processing

Trace polylines from the receivers (main stem follows the largest donor,
other donors start tributaries that end on the junction pixel), then:

1. Chaikin corner cutting, 3 iterations, endpoints fixed;
2. resampling every 0.4 px;
3. lateral meander offsets along the normal of a smoothed copy:
   `A·n1(s/λ) + 1.5 px·n2(s/3.5λ)`, with `n1`, `n2` Perlin noise along the
   arc length from the mouth (stable phase), `λ = clamp(7·w, 6, 40)` px and
   `A = clamp(2.5·w, 1, 3)` px where `w` is the drawn width, tapered to zero
   over `λ/2` at both ends;
4. guards: offset limited to half the local bend radius, shrunk while it
   climbs the filled surface by more than 0.001 (**valley constraint**),
   limited to 0.35 × the distance to an unrelated river, offsets smoothed
   over ±4 samples, and any remaining self-loop cut;
5. tributary ends snapped to the displaced parent with an 8 px blend.

| Seeds 3 / 7 / 42        | D8    | Chaikin | C: vector   | A + C      |
| ----------------------- | ----- | ------- | ----------- | ---------- |
| Whole sinuosity, median | 1.22  | 1.19    | 1.38–1.41   | 1.44–1.47  |
| Whole sinuosity, p90    | 1.46  | 1.42    | 1.68–1.73   | 1.77–1.78  |
| Local sinuosity, median | 1.06  | 1.05    | 1.10        | 1.13       |
| Straight 160 km windows | 23 %  | 28 %    | 9–10 %      | 5 %        |
| Length on a D8 heading  | 100 % | 79 %    | 23 %        | 23 %       |
| Uphill samples          | 0 %   | 0 %     | 0.08–0.18 % | n/m        |
| Crossings, unrelated    | 0     | 0       | 0–3         | n/m        |
| Self-crossings          | 0     | 0       | 0           | 0          |
| Time, trace + vectorise |       |         | 12 + 45 ms  | 12 + 55 ms |

n/m: not measured (the prototype checks uphill on the perturbed surface
only, and did not split crossings by relation for A + C).

- Visual gain: large. Rivers are smooth curves at any zoom
  (`vector_zoom6x.png`) and meander with an amplitude that grows with size.
- Plausibility: polylines follow the receivers, so they reach the sea, a lake
  or their parent by construction. The guards matter: without them
  (`A = 2.5w` up to 6 px, no valley constraint) sinuosity reaches 1.82 but
  10 % of samples climb, with loops and 26–113 self-crossings per world;
  with them, 0.1–0.2 % of samples climb by more than 0.002 and self-crossings
  are gone. 7–13 tributary/parent crossings remain where a tributary runs
  alongside its parent on a flat: the proximity clamp must also apply to the
  parent outside the confluence zone.
- Consistency: the drawn line departs from its raster pixel by at most
  `A + 1.5` px (4.5 px, about 90 km), inside the valley by construction. The
  flow raster, hex flags and the viewer flow layer are unchanged. Valleys
  carved by erosion (512 × 256 grid, about 3.75 map px per cell) are wider
  than the displacement.
- Complexity: medium (one new module, about 400 lines).

### D. Drawing and data contract

- **Width law**: `w_px = 0.9 · (Q / 120 000)^0.5`, capped at 4 px at the base
  resolution (reached at about 2.4 M km²). Real widths (about
  `0.002 · √Q` km, 5 km for a 6 M km² basin) are a fraction of a pixel, so
  this is a 10–20× cartographic exaggeration; a zoomed renderer should draw
  `max(real width, exaggerated width in screen px)` so real widths take over
  at deep zoom.
- **Drawing**: anti-aliased variable-width segments with a coverage buffer
  (maximum, not sum, so confluences do not darken), drawn over the biome
  colours, with the longitude seam handled by drawing at `x ± width`.
- **Raw data**: `raw_data/rivers.json` with one entry per polyline:

```json
{
  "version": 1,
  "width_law": { "w0_px": 0.9, "q0_km2": 120000, "exp": 0.5, "max_px": 4 },
  "rivers": [
    {
      "id": 0,
      "parent": null,
      "mouth": "sea",
      "points": [[129.656, -62.275, 158147], [129.802, -62.306, 158175]]
    }
  ]
}
```

Points are longitude, latitude (degrees) and flow (km²), about one per base
pixel: 0.5–0.6 MB per world (242 KB gzipped), 262–310 polylines. The flow
raster stays the source for sampling and hexes.

## Recommendation

Combine **A restricted to filled flats**, **C** and **D**:

1. Flats: replace the pure `FLAT_EPSILON` ordering by a priority of
   `z + FLAT_EPSILON · (1 + noise)` (or Barnes' flat resolution plus noise),
   so paths across filled areas wander instead of running straight. Divides
   and lakes do not move.
2. Vector rivers with the prototype parameters: Chaikin ×3, resample
   0.4 px, `λ = clamp(7w, 6, 40)`, `A = clamp(2.5w, 1, 3)`, wander 1.5 px at
   `3.5λ`, valley tolerance 0.001, curvature limit 0.5 R, proximity 0.35 d
   including the parent outside the confluence zone, loop cut, snap blend
   proportional to the parent's `λ`, tributaries shorter than 3 px merged.
3. Width `0.9 · (Q / 120 000)^0.5` px, cap 4 px, tapered from 0.5 px at the
   head so rivers do not start abruptly.
4. `world.png` and `world.svg` draw the polylines; the `River` biome is set
   from the drawn coverage (≥ 0.5) so `biomes.png` matches the picture.
5. `raw_data/rivers.json` as above; `flow.png` unchanged.

### Commit plan

1. `feat(hydrology): break straight paths across filled flats` — noise in the
   flat priority; tests: every land pixel still drains to the sea or a lake,
   total runoff is conserved, divides are unchanged on a fixture, output is
   deterministic per seed. README "Rivers and lakes" step 2.
2. `feat(rivers): trace river polylines from the drainage network` — new
   `rivers.rs`; tests: every river pixel belongs to one polyline, each
   polyline ends at the sea, a lake or a vertex of its parent, flow is
   non-decreasing downstream.
3. `feat(rivers): smooth and meander river polylines` — tests: endpoints and
   junctions kept within 0.5 px, offsets within `A + wander`, zero self or
   unrelated crossings on seeds 3, 7 and 42, uphill share below 0.5 %,
   determinism.
4. `feat(export): draw rivers as lines whose width grows with flow` —
   PNG and SVG, `River` biome from coverage; snapshot test on a small world;
   README map description.
5. `feat(export): write river polylines to raw_data/rivers.json` — serde
   types (with `ts` bindings for the web), README raw data format, a test
   that the JSON round-trips and matches the polylines.
6. `docs: close backlog item 3` — backlog status and README examples.

### Risks

- **Over-meandering** looks artificial on steep mountain rivers: modulate
  `A` by slope (straighter on steep reaches, as in nature) if review asks.
- **Residual crossings** where rivers run side by side on flats; the
  proximity clamp must cover parents, and the test above makes it a hard gate.
- **Confluence kinks** (`artifact_confluence_hairpin.png`): blend length must
  scale with the parent's wavelength.
- **Small lakes** are covered by wide river ends; draw lakes after rivers.
- **Biome rework (parallel)**: rivers become an overlay instead of a biome
  that wins a pixel. Agree on whether `biomes.png` keeps `River` (from the
  coverage, as proposed) or drops it in favour of `rivers.json`.
- **Deep zoom (parallel)**: the renderer should consume `rivers.json`, draw
  width as `max(real, exaggerated)`, and add finer meanders procedurally,
  seeded by river id and arc length from the mouth, so the base-scale shape
  stays put when zooming. The hex river flag keeps using the flow raster;
  the viewer's flow layer and info panel are unaffected.

## References

- Barnes, Lehman and Mulla (2014), Priority-Flood: an optimal
  depression-filling and watershed-labeling algorithm.
- Leopold and Maddock (1953), the hydraulic geometry of stream channels.
- Leopold and Wolman (1960), river meanders.
- Langbein and Leopold (1966), river meanders: theory of minimum variance.
- Tarboton (1997), D-infinity flow directions; Freeman (1991), multiple flow
  directions.
