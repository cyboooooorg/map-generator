# Analysis: rounded coastlines, islets and ponds

Backlog item 5. User report, after PR #16 (erosion no longer moves a pixel
across the waterline): continent outlines are too round, with none of the
angles, capes and straight stretches of real coasts; and along the coasts
there are many tiny islands, and tiny lakes or ponds inland.

Worlds measured: terran, volcanic 0.3, tilt 23.44, erosion 1; seeds 3, 7, 42,
1094255522, 1939511125, at 1920, 4096 and 8192 px wide. Metrics: connected
land and water components by size, box-counting dimension of the coastline
(1.0 is a smooth curve), and land/water masks of detail tiles rebuilt from
`detail.rs`.

## 1. Rounded outlines

- The coast is the waterline contour of the 5-octave continent fbm
  (`generation.rs`, continent term). That fbm samples the **unwarped** sphere
  position: the domain warp only feeds the ridged mountains, and the
  mountain term is zero below continent 0.2, so it never reaches the coast
  for sea levels below 0.35.
- Its finest feature is about 500 km on an Earth-sized planet, and about
  1 650 km on a 72 787 km planet (feature size grows with C² through
  `noise_scale`), against a pixel of 10–20 km.
- Coastline box dimension between 4 and 32 px: **0.95–1.01** on every current
  world. Real coasts measure about 1.02–1.25.
- Before PR #16 the erosion artefact made coasts measure 1.22–1.63: that
  roughness was the grid of islets, which is why coasts now look rounder.
- Detail tiles only add ±0.005 of height noise, so a zoomed coast is the
  smooth bicubic line plus speckle.

## 2. Islets and ponds

- **Base data:** worlds generated with the current code have 0–1 land
  components of 64 px or less, and 0–1 small enclosed waters. The saved
  worlds generated before PR #16 have hundreds (the erosion grid artefact);
  regenerating them fixes it.
- **Detail tiles:** thousands. Summed over 40 coastal tiles at the first
  detail level, small land components go from 0–28 in the base to
  2 154–6 653 with detail, small water components from 0–2 to 617–3 426.

Mechanism in the tiles:

1. Droplets dump all their sediment on the first sea cell they reach and
   erode land down to the waterline; the per-pixel waterline clamp of PR #16
   (`WATERLINE_MARGIN`, 0.9) then holds the shore just off the waterline.
   Within 3 px of the coast, 68–99 % of sea pixels are pinned by the clamp.
   Erosion builds a **flat shelf at the waterline**.
2. The detail noise (RMS 0.0015, peak 0.005) is taller than that shelf, so it
   pushes the shelf above water (islets) and coastal land below it (ponds).
   The clamp's step also leaves a scalloped coast.

| Run (z4, land / water components) | seed 3      | seed 42     | 1094255522    |
| --------------------------------- | ----------- | ----------- | ------------- |
| Current                           | 6 291 / 617 | 2 154 / 866 | 3 451 / 3 426 |
| Erosion off                       | 361 / 333   | 78 / 100    | 99 / 96       |
| Erosion tapered within 0.05       | 371 / 301   | 79 / 95     | 101 / 98      |
| Detail warps the coast sideways   | 42 / 7      | 4 / 4       | 2 / 2         |

## 3. Options

### Islets and ponds

- **2A. Tiles warp the coast sideways** instead of adding height at the
  waterline; height detail fades to zero near the waterline. Removes nearly
  all tile islets and gives zoomed coasts a fractal outline. About 30 lines;
  no change to generated data. River mouths can end up 1–2 base pixels off
  the warped coast; the lake mask must be sampled at the warped position.
- **2B. Taper erosion near the waterline** and spread sediment offshore,
  instead of the hard clamp. Removes the shelf and the staircase; changes
  every seed slightly.
- **2C. Scale detail by local slope.** Cheap, but coastal flats lose all
  detail and the coast stays smooth.

### Rounded outlines

Prototypes compared on the base map:

| Option                                            | Coast dimension | Small islets per world |
| ------------------------------------------------- | --------------- | ---------------------- |
| 1A. Multi-octave domain warp of the continent fbm | 1.14–1.28       | 117–196                |
| 1B. 1A plus a low (0.06) Voronoi "plate" step     | 1.15–1.26       | 110–167                |
| 1C. More fbm octaves with a higher gain           | up to 1.36      | 450–1 100              |

1B is the only option giving straight stretches and angular corners. 1A and
1B change every seed and need a cleanup pass removing 1–4 px components. 1C
repeats the speckle mechanism of the tiles and is rejected.

## 4. Decision

- 1A + 1B with the noise rework (backlog item 5, step 2).
- 2B with the erosion retune (step 3).
- 2A with the tile rework (step 4).
