# Engine performance review, 2026-11-05

A review of the generator (`crates/mapgen`) for possible optimisations,
made by the two sessions, one half of the stages each. It lists
candidates with estimated gains; the [Results](#results) section records
which were done and what they measured.

## Setup

`map-generator --planet terran --seed 4242 --width 8192 --height 4096
--no-noise-maps --moons 0 --host none`, release build, `main` at
`10d2087`, 10 cores (Apple silicon), on an otherwise idle machine. CPU
profiles come from macOS `sample` (1 ms), memory from the RSS polled every
0.2 s.

| Stage       | Wall time | Peak RSS |
| ----------- | --------- | -------- |
| Noise       | 4.3 s     | 1.10 GB  |
| Erosion     | 1.9 s     | 1.24 GB  |
| Temperature | 0.1 s     |          |
| Coast       | 1.5–1.9 s | 1.40 GB  |
| Moisture    | 1.7 s     | 1.62 GB  |
| Rivers      | 2.8–4.2 s | 1.66 GB  |
| Biomes      | 0.1 s     | 1.81 GB  |
| Export      | 3.8–4.6 s | 2.62 GB  |
| **Total**   | **≈17 s** |          |

Two facts frame the findings:

- **Perlin noise is about 65 % of the busy CPU time** (noise-rs
  `Perlin::get`): `Sampler::sample` 33 CPU-s, the clouds 12, the surface
  variation 7, the hydrology's flat and routing noises 4.
- **The cores are idle about half the time**: serial sections (two
  Dijkstra-like heaps, the flow accumulation, PNG encodes one after
  another) leave nine cores waiting.

Fat LTO with one codegen unit was measured too: no gain (Rivers slower),
so the release profile stays as it is.

Each finding says whether the output stays **bit-identical** (ID: every
seed reproduces the same world) or **changes** (worlds generated before
and after differ slightly, including bodies of a system generated on
either side of the change).

## Results

Measured as the minimum of three interleaved runs at 8192 × 4096 (the
machine is noisy: other work ran at the same time). The rivers and export
items were timed on a river-heavy world (25.3 s on `main`), the others on
the setup above (20.3 s on `main` for that run). Every item kept is
bit-identical: the same output bytes as before for every test world.

| Item | Status     | Gain                                                                                                                                    |
| ---- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| N1   | Dropped    | 1.4–1.5× per call in a micro-benchmark, but about 3 % on the Noise stage, within the noise of the measurement: not worth its own Perlin |
| C1   | Done       | Coast 2.32 → 1.55 s (bucket queue)                                                                                                      |
| C2   | Done       | −1.8 s (coast and moisture together)                                                                                                    |
| L1   | Done       | −0.5 s                                                                                                                                  |
| M1   | Dropped    | No wall time left to gain: after C2 the moisture pass runs beside the longer coast distance                                             |
| E1   | Done       | Erosion 1.60 → 1.30 s                                                                                                                   |
| R1   | Dropped    | The flood is bound by random memory accesses: even a hole-based 4-ary heap was no faster than the standard binary heap                  |
| R2   | Done       | With R3: Rivers 8.68 → 8.04 s                                                                                                           |
| R3   | Done       | See R2                                                                                                                                  |
| R4   | Done       | Rivers 8.68 → 7.17 s                                                                                                                    |
| X1   | Done       | With X2a: Export 5.32 → 4.13 s, four PNGs written at a time at most, peak memory unchanged                                              |
| X2a  | Done       | See X1                                                                                                                                  |
| P2   | Dropped    | The clone is the `world.png` buffer itself                                                                                              |
| P3   | Dropped    | Covered by X1's cap of four files at a time                                                                                             |
| N2   | Done       | Noise 3.29 → 1.90 s at 4096 × 2048 (−15 % in total), 9.37 → 7.15 s at 8192 × 4096 (−10 %), on 4 threads (see below)                    |
| S1   | Not needed | Already in place: moons are sized by their radius (at least 512 × 256, `MOON_MIN_WIDTH`), siblings at 512 × 256                         |

All the items together, `main` at `50aa128` (before) against `6c8acdf`
(after), same command at 8192 × 4096, minimum of three interleaved runs:

| World                             | Before  | After   | Gain  |
| --------------------------------- | ------- | ------- | ----- |
| The setup above (seed 4242)       | 17.09 s | 14.22 s | −17 % |
| River-heavy (seed 1777467217)     | 22.91 s | 18.90 s | −18 % |

The output files are byte-identical for both worlds. The coast distance
now runs inside the Moisture stage, and the clouds inside Rivers, so the
per-stage times shift between those stages.

N1 shows that the Perlin share of the profile overstates what faster noise
alone gains: N2 and X3 stay to be measured before they are kept. P1, E2
and X2 (b) are left for later (P1 means reordering the export, E2 and
X2 (b) change the output).

N2 was done on 2026-11-08 (format version 13), on the five climate fields
of the map point only: moisture, temperature anomaly, the two variability
fields and the highland jitter (21 Perlin calls a pixel). The domain warp
and the volcanic zones stay per pixel: they are read at the plates'
origin of the crust, which jumps across plate boundaries, and the volcanic
zones place the cones. Each field takes its own step, about a sixth of
its finest octave's wavelength (`coarse::step_for`), and Catmull-Rom
splines instead of bilinear interpolation, so biome borders stay smooth
curves. On 9 worlds at 4096 × 2048 (terran ×5, volcanic, frozen,
caustic, a small barren body): land unchanged, every biome's share of the
land within 0.1 point, 0.00–0.10 % of land pixels changing biome,
temperature within 0.07 °C. Which fields go coarse depends on the map:
at 1920 × 1080 the moisture and the anomaly (steps 4 and 10); at 4096 ×
2048 the highland jitter too; from about 7 300 pixels of width the two
variability fields as well (step 2 at 8192).

## Findings

### Noise, coast, moisture, clouds

- **N1. A faster Perlin, bit-identical** (`noise.rs`, every caller). A
  hand-written 3D Perlin with noise-rs's permutation table (read through
  `PermutationTable::hash`), its floor quirk and its arithmetic order, and
  the gradient picked from a table instead of a 16-way `match`: **1.5×
  faster per call, 0 mismatches** over 6 million points and 3 seeds in a
  micro-benchmark. Since noise is 65 % of the CPU time, about −1.5 to
  −2 s overall (Noise, clouds, surface variation, hydrology noises). ID.
  Low risk: a test compares it with noise-rs bit for bit.
- **N2. Low-frequency fields on a coarse grid** (`Sampler::sample`). The
  domain warp (2 calls), volcanic zones (3), moisture (4), temperature
  anomaly (3), highland jitter (4) and the two variability fields (10)
  are 26 of the ~45 Perlin calls of a pixel; their finest wavelengths run
  from about 65 km (variability) to thousands of km, against 5 km pixels.
  Sampled every 2 to 8 pixels by wavelength and interpolated bilinearly,
  they would cost a fraction. About −1.5 s on Noise. Changes the output
  (interpolation error well under the fields' own amplitude). Medium risk:
  the grid step must follow each field's wavelength and the planet's size.
  The surface variation (X3) and the clouds can do the same.
- **C1. A bucket queue for `Grid::distance_to`** (`geo.rs`, the whole
  Coast stage, serial). A Dijkstra over every pixel within 2 000 km of the
  sea with a binary heap. A Dial or radix queue on the quantised distances
  (steps are a few km), or an exact parallel two-pass sweep on rows
  followed by columns, would be 2–3× faster: about −1 s. Near-identical:
  equal-length paths may sum their steps in another order (last-bit
  differences). Low risk.
- **C2. Overlap Coast with Temperature and the moisture inputs.** The
  serial Dijkstra leaves nine cores idle; what does not read the coast
  distance can run beside it in `rayon::join`. Up to −0.5 s. ID.
- **M1. A sliding-window vertical pass in `climate::smooth`.** The
  horizontal pass already slides; the vertical one sums 2r + 1 rows per
  pixel (r ≈ 20 at 100 km). About −0.3 s. Last-bit changes (summation
  order). Low risk.
- **L1. Clouds beside the rivers.** The clouds (12 CPU-s of Perlin, fully
  parallel) are computed before the hydrology, whose priority-flood is
  serial: computed in `rayon::join` with it, they fill idle cores. About
  −0.5 to −1 s. ID. Adds the clouds' 134 MB during the flood, under the
  Export peak.

### Erosion, rivers

- **E1. `erosion::spill_levels`** (serial, both passes, about 0.44 s):
  seed the heap with the coast cells only, as the hydrology does, and run
  it in `rayon::join` with `simulate`, which it does not depend on. About
  −0.3 to −0.45 s. ID. Low risk.
- **E2. More erosion tiles on the coarse pass.** At 1024 × 512 the tiling
  gives 8 tiles per checkerboard phase for 10 cores, and ocean tiles do
  nothing: smaller tiles keep every core busy. About −0.2 to −0.4 s.
  Changes the output (droplet streams depend on the tiles). Medium risk.
- **R1. A 4-ary heap for the priority-flood** (`hydrology.rs`), with the
  same total-order comparator: the same pop order, so `order` and
  `filled` stay the same (the `serial_reference` test checks it). About
  −0.3 to −0.5 s. ID. Low risk.
- **R2. The routing noise beside the flood** (`rayon::join`): about −0.3
  to −0.4 s. ID. Low risk.
- **R3. Per-row neighbour distances** (`Grid::neighbours`, which
  recomputes a cosine for 3 rows and a square root per neighbour on every
  call): a table per row with the same expressions and an unrolled
  8-entry array. About −0.1 to −0.2 s, and it speeds up C1 too. ID. Low
  risk.
- **R4. A fast hasher in `carve::carve_line`** (FxHash or ahash instead of
  SipHash, presized): about −0.2 to −0.3 s. ID (the merge is a min or max,
  whatever the order). Low risk.

### Export

- **X1. Encode the raw-data PNGs in parallel** (`export_raw_data` writes
  about 12 PNGs of 33 M pixels one after another, 1.1 s of the main
  thread). About −1 to −1.5 s. ID (the same bytes). Low to medium risk:
  each concurrent layer holds a 67 MB buffer, so cap the concurrency or
  quantise row by row into the encoder (see P3).
- **X2. Fewer `BiomeRules::choose` calls in the ecotones** (once for the
  dry biome, once for the centre, 8 probes, 128 more near borders): pass
  the biome already computed as the centre (ID, 5–10 % of
  `render_pixels`), or 64 samples instead of 128 (not ID, ±1 in the last
  bit of the colour at borders, about −0.3 s).
- **X3. The surface variation on a coarse grid** (`SurfaceVariation`:
  about 14 Perlin calls per sea pixel, 8 per land pixel, for 900 km
  swirls): see N2. About −0.3 to −0.5 s. Changes the output. Low visual
  risk.

### Memory (peak in Export, 2.6–2.9 GB at 8192 × 4096)

- **P1. Encode early the layers that the render and the SVG never read**
  (`flow_km2`, `elevation_base`, probably `floodplain`), then drop them:
  about −0.3 to −0.4 GB. ID. Medium risk (the export must be reordered).
- **P2. Draw the contours on the `world.png` buffer** once `surface.png`
  is encoded, instead of cloning the surface: −100 MB. ID. Low risk.
- **P3. Quantise the 16-bit layers row by row into the encoder**: no
  67 MB buffer per file, which matters once X1 writes them in parallel.
  ID. Low risk.

### Planetary systems

- **S1. Map sizes by body.** *Wrong at the time of writing: moons are
  already sized by their radius (see Results).* The finding read: every
  body of a system gets the system's map size. Sizing each body by its radius
  (the planet's km per pixel, with a floor such as 1024 × 512 and the
  system's size as a ceiling) would make a moon a fraction of a planet.
  For a terran planet with its moons, roughly −50 % on the neighbourhood;
  more around giants. Changes the moons' maps (smaller; the detail tiles
  still zoom). Low risk, but a visible choice: it is the user's to make.

## Suggested order

1. **Bit-identical, low risk, largest gains:** N1, X1 (with P3), L1, R1,
   R2, E1, R3, R4, C2, P2. Together about −5 to −7 s of the 17 s (a
   third), and −0.1 GB.
2. **Output changes, worth it:** C1, N2 and X3, M1, then S1 for systems.
   About −3 s more on a planet, and much less work per neighbourhood.
3. **Later:** E2, X2 (b), P1.

Each step ships with its benchmark (stage times before and after on the
setup above) and, for the ID items, a test that the output bytes are
unchanged.
