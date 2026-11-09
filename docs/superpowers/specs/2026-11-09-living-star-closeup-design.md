# Living star: the close-up photosphere

## Intent

Living star part 1 (#343, PRs #350, #373, #388) gave every star an activity
and a surface that changes with the system clock. Up close, the result is too
smooth to look like a star. The granulation is so faint that the surface looks
smooth, and the spots are round discs with a plain penumbra.

The user wants the star view, zoomed in, to look like the high-resolution
images of the Sun, such as the DKIST (Inouye telescope) sunspot close-ups. The
surface is agitated, made of bright cells with dark lanes between them. A spot
is torn and irregular, "like an oil slick on the open sea". Its penumbra is
made of filaments that fray into the granulation around it.

The failure the user named: a render that feels like an old video game for
lack of life and detail. That means static, too clean, repetitive, a visible
grid or tiling, or a blurry "digital noise" texture.

## Decisions taken with the user

1. **The look is variant C of the prototype, built the way variant D builds
   it.**
   - The prototype is
     [2026-11-09-living-star-closeup-prototype.html](2026-11-09-living-star-closeup-prototype.html)
     (also published as an artifact during the design). C and D are its third
     and fourth renders.
   - D draws the same image as C. It only computes a detail where that detail
     is visible, and uses lighter noise for the cell warp. On the user's Apple
     Silicon Mac, at 1× pixel ratio, D held 120 frames per second, about 8 ms
     a frame, full screen.
   - The user judged that D keeps C's look.
2. **A slight exaggeration of reality is accepted.** Seen whole, a real star's
   granules are smaller than a pixel, so the disc looks almost smooth. A texture
   at a larger scale shows on the whole disc instead: the supergranule network,
   with faculae brighter towards the limb, a little stronger than reality, plus
   a faint mesogranular mottle. Granules take over as the camera comes closer.
   The exaggeration is labelled a **calibration** in the docs.
3. **Spots change shape over their life.**
   - While a spot grows, it is compact.
   - At its maximum, it is spread out and filamentary.
   - As it declines, it fragments and its pieces dissolve.
   - The shape at a day depends only on the region and its age that day.
4. **One rendering, shared by every view, gated by size on screen.** The star
   view, the system view and the planet's sky disc keep sharing the one surface
   chunk (`web/src/lib/viewer/starSurface.ts`). Each detail switches on only
   above a size on screen, so a small star costs nothing extra. The sky disc
   keeps granulation compiled out (`STAR_NO_GRANULATION`). It may draw the new
   spot structure once its disc is large enough.
5. **Pixel ratio in the star view.** At Retina resolution the prototype stutters:
   4× the pixels make about 30 ms a frame. So:
   - while the star view's clock runs, it draws at the quality profile's
     *moving* pixel ratio (`movingPixelRatio`, 1.25 in high);
   - when it is paused, it draws one sharp frame at the full ratio
     (`maxPixelRatio`), as the other views do when they settle.

   The user accepts a slightly softer image while it animates.

## Goals

- **Granulation** as in C: bright cells with dark intergranular lanes, cells
  brighter at their centre, small bright points in the lanes. The cells move,
  deform, split and merge over a few granule lifetimes, with no repeating
  pattern and no visible grid. The contrast is high enough that the surface
  reads as agitated up close, with real granules about 1–2 Mm across (the
  granule size and lifetime come from `StarActivity`).
- **Network and faculae**: the supergranule network (cells about 30 Mm across)
  is outlined by bright faculae, which strengthen towards the limb. It shows
  from the whole disc down to the close-up. A mesogranular mottle sits between
  the network and the granules.
- **Spots**:
  - the umbra has a torn edge;
  - the penumbra is made of radial filaments and has a frayed outer edge that
    merges into the granulation;
  - an outward drift of the filaments (the Evershed flow);
  - bright umbral dots;
  - light bridges crossing the umbra of large spots, at some stage of their life;
  - small dark pores in the moat around a spot;
  - the shape evolves with age as in decision 3: compact while growing,
    filamentary at maximum, fragmenting while declining.

  The leader/follower spots, plages, tilt, separation and areas of part 1 stay
  as they are. This work changes how one spot looks, not where spots are or
  how big they are.
- **Every relation that comes from the science cites its source.** This covers
  filament widths, the Evershed flow speed, umbral-dot and pore sizes, light
  bridges, network size and contrast. Every invented number is labelled
  **calibration** in the code and in `docs/science/star-activity.md`, as in
  part 1.

## Non-goals

- Magnetic fields drawn as such (#345), ejections and flares (#344).
- Any change to the region timeline, the activity model in Rust, the stored
  format or `SYSTEM_FORMAT_VERSION`. This is a rendering change only.
- Granulation on the planet's sky disc, which keeps `STAR_NO_GRANULATION`.
- Coronal loops. The loop fades of #391 stay in that issue, and this work only
  `Refs #391`.
- Limb features (spicules, the chromosphere seen edge-on).

## Anti-goals

- The "old video game" look described above.
- Any visible frame drop. The bounds are part 1's, measured with the
  `?frames=1` protocol of `web/README.md`:
  - **System and planet views:** each median stays within twice the run-to-run
    spread of a fresh baseline taken on `main` before the change.
  - **Star view at 2560×1440:** the p95 stays at or under 16.7 ms, at the
    default distance and at about 1.1 R.
- Anything that depends on frame history or wall time, or on the path taken to
  reach a day. The surface at a day stays a function of `StarActivity`, the
  day and the view (size on screen, pixel ratio) only. Part 1's "same pixels at
  the same day" checks must keep passing.

## Constraints

- **Code to keep or reuse.** Strict TypeScript, Three.js 0.187, GLSL ES 3.0 as
  the existing shaders use. No new dependency.
- **One clock.** Granule motion runs on the system day, reduced in JS (mod 1e4
  days) before it reaches the shader, as part 1 does. Spot shapes take the
  region's age in days.
- **Each region needs a stable seed in the shader.** Today `uRegionA` packs
  latitude, longitude, tilt and separation, and `uRegionB` packs born time,
  growth, lifetime and peak area. Neither carries the region id. The builder
  adds the id or a seed derived from it (`hash32`), keeping the packing cheap
  and repacked only when the set changes.
- **Size gates.** The same size gates as part 1 (`surfaceDetail`,
  `surfaceRadiusPx`, `hostStarDetail`): no detail is computed below its size
  threshold. Detail is gated by branching on uniforms, plus the cheap paths of
  prototype D.
- **Docs ship with the change**:
  - `docs/science/star-activity.md`: the new relations, their sources and the
    calibrations;
  - `web/README.md`: the star view's pixel-ratio behaviour and the new measured
    frame times;
  - `README.md`, if what the star view shows is described there.
- **Process.** As in part 1:
  - a branch per PR from `main`;
  - a per-task review, plus a real-GPU check of the look and of the anti-goals;
  - the before/after measurement posted on the PR.

## Calls made by Claude (not discussed with the user)

These settle questions the builder check raised. The user can overturn them.

- **Where the frame-time bound is measured.** Measure in two places:
  - the `?frames=1` protocol of part 1: a GPU browser, a window of 2560×1440
    at device pixel ratio 1;
  - the same window on the user's Retina Mac (ratio 2), where the moving
    ratio rule applies.

  Each is measured at three camera distances in the star view:
  - the default distance;
  - about 1.1 R;
  - the closest, 1.02 R (`MIN_DISTANCE`), the worst case for granules and spot
    detail.

  The p95 ≤ 16.7 ms bound applies to all of them.
- **If the look and the bound conflict, the frame time wins.** The user's first
  anti-goal is frame drops. Cut cost in this order:
  1. cheaper noise (fewer octaves, a precomputed tileable noise texture,
     cheaper cells);
  2. a lower moving pixel ratio at close range, down to 1.0;
  3. weaker detail as a last resort.

  Record each cut and its measured gain in the PR.
- **Spot areas.** The darkened area drawn for a region stays within ±20 % of
  the area part 1 draws for it at the same day. That area is the region's area
  in the timeline, which also feeds the spot coverage shown in the star view's
  panel.
  - The torn edges, filaments and light bridges reshape that area. They do not
    add to it or remove from it beyond that margin.
  - Fragmentation in the decline moves area into pieces; it does not delete it.
    The timeline already shrinks the area to zero at the end of the region's
    life, so the decline is not counted twice.
  - A test measures the drawn dark area against the region's area at several
    ages.
  - The ±20 % margin is a **calibration**.

## Left to the builder

- **How the noise is built.**
  - Procedural noise as in the prototype, or a precomputed tileable noise
    texture where it is cheaper. A texture must not show tiling.
  - The number of octaves, the warp strength, how the cells move.
- **Exact thresholds and contrasts**, as long as the look matches prototype D
  and the bounds hold:
  - granule contrast;
  - network contrast and its limb factor;
  - the spot's edge noise;
  - filament frequency and length;
  - the stages of a spot's life at which bridges, fragmentation and pores
    appear.
- **How the pixel-ratio rule hooks into `pixelRatio.ts` / `StarBodyView`.**
- **How many PRs**, and in what order (for example granulation and network
  first, spots second).
- **Whether the system view's star also drops to the moving pixel ratio while
  animated.** Its star is usually small, so the measurement decides.
