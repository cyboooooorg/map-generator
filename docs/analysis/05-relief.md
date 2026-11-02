# Analysis: relief everywhere, no smooth plains

Backlog item 5. User report: erosion seems applied everywhere whatever the
biome; there are no smooth plains and no visually fluid deserts.

Worlds measured: terran and barren, seed 3, sea level 0.1, Earth
circumference, 4096 × 2048, erosion 1 and 0. Land pixels below 60° of
latitude. Slope is what the hillshade uses, `|∇(e × 1500)|` per km;
"fine-scale slope" is the slope of `e − box_blur(e, 13 px)`, features under
about 130 km, which is what reads as rough rather than rolling.

## 1. Measurements

**Base map** (`world.png`, also the globe texture):

| Land type                    | Slope median (erosion 1 / 0) | Fine-scale slope median (1 / 0) |
| ---------------------------- | ---------------------------- | ------------------------------- |
| Desert                       | 0.173 / 0.215                | **0.046 / 0.0069**              |
| Plain                        | 0.238 / 0.238                | 0.062 / 0.0057                  |
| Flat land (no mountain term) | 0.150 / 0.228                | 0.045 / 0.0064                  |
| Mountainous land             | 0.388 / 0.351                | 0.083 / 0.0114                  |

- Erosion multiplies the fine-scale slope by 6–10 on every land type, flat
  land included; mountains end up only 1.8× rougher than plains.
- Without erosion plains are nearly as steep as mountains (0.228 against
  0.351): the noise has no plains either.

**Detail tiles** (`detail.rs`):

| Level | km per pixel | Slope of the added detail on plains (median / p90) |
| ----- | ------------ | -------------------------------------------------- |
| z3    | 9.8          | 0.17 / 0.31                                        |
| z5    | 2.4          | 0.42 / 0.74                                        |
| z7    | 0.6          | 0.67 / 1.13                                        |

From z5 the detail added to plains is steeper than the base-map mountains,
erosion makes no difference any more, and desert tiles look rougher than
mountain tiles because the shading saturates at its [0.55, 1.3] clamp.

## 2. Root causes

1. **The continent noise is as steep on plains as on mountains.** Each
   octave has half the height and half the size, so every octave adds the
   same slope (0.09–0.10 each on flat land), at full strength everywhere. The
   mountain term is a single smooth ridged octave adding only 0.06–0.16.
2. **Erosion ignores slope.** `min_slope = 0.01` floors the sediment
   capacity; the terrain drops only 0.005–0.008 per erosion cell, so for most
   land the capacity is that floor, the same on a desert as on a ridge. The
   constants come from Beyer's method, which assumes heights of about 1 per
   cell; this terrain is about 100× flatter per cell.
3. **The zoom detail gets rougher at every level**: same halving rule as the
   noise, plains at a third of the mountain amplitude, no link to biome or
   slope, and a hard shading clamp.
4. **Shading treats every land type the same** (1500× exaggeration), and
   the viewer's Relief slider shades a second time on top of `world.png`.

## 3. Prototypes (base map re-shaded only)

| Variant                                                | Desert fine slope | Mountain / flat |
| ------------------------------------------------------ | ----------------- | --------------- |
| Current, erosion 1                                     | 0.049             | 1.8×            |
| A1. Erosion run on heights × 10                        | 0.032             | 3.2×            |
| A2. Erosion faded where the cell slope is low          | 0.010             | 5.7×            |
| B. Finest 3 continent octaves × 0.3 inland, no erosion | 0.003             | 3.2×            |

A1 gives branching valleys in mountains and calmer deserts. B gives smooth
deserts but soft mountains, which need more ridged octaves. Turning erosion
off is not a fix: erosion also fills pits that would otherwise become lakes.

## 4. Options and decision

- **A. Slope-aware erosion:** run the droplets on heights scaled to real
  slopes (or lower `min_slope` and retune), optionally faded on flat cells.
  A few lines; changes every seed slightly.
- **B. Terrain-dependent roughness in the noise:** fine continent octaves at
  full strength near coasts (so coasts keep their detail), 0.2–0.3 on low
  ground inland, back with the mountain weight; 2–3 ridged octaves for
  mountains. The structural fix; changes every seed.
- **C. Terrain-aware tiles and shading:** plains detail fading faster with
  zoom (gain 0.35) and smaller, a soft shading curve instead of the clamp,
  no double shading in the viewer. Required for deep zoom.

Decision: B with the noise rework (backlog item 5, step 2), A with the
erosion retune (step 3), C with the tile rework (step 4).
