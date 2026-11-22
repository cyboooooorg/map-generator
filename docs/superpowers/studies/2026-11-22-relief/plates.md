# Relief study: plates and the relief of their boundaries

Part of the [relief study](README.md); tags as in
[topography](topography.md#relief-study-the-statistical-shape-of-the-topography).
Legacy numbers are those of `legacy-final:crates/mapgen/src/plates.rs`
(constants at lines 40-70, 476-491, 934; `relief_of`), whose elevation
unit is 25 km and whose profiles are `bump(d, mu, sigma) =
exp(-((d - mu) / sigma)^2)` (full width at half maximum 1.665 sigma).

## Plate count and sizes

- **Bird 2003** (PB2002): "the 14 large plates whose motion was described
  by the NUVEL-1A poles", plus 38 small plates, "for a total of 52
  plates"; the cumulative number against area follows a power law between
  0.002 and 1 steradian [V, abstract].
- **Morra et al. 2013**: eight large plates "cover most of the surface of
  the Earth" (section 1); 31 plates at present in the
  Seton et al. 2012 reconstruction, falling to 10 at 200 Ma; the large and
  small populations part at about 10^7.5 km² and a cumulative count of 8
  (Fig. 2 caption and text) [V].
- **Sornette & Pisarenko 2003**: on Bird's 42 plates of PB2001, the
  complementary cumulative distribution of areas is a power law of
  exponent µ = 0.25 ± 0.05 (abstract and section 3) [V,
  arXiv:cond-mat/0202320].

Retained: **8 to 14 plates** (Morra's large plates to Bird's), the area
law **µ = 0.25** as the target the plate draw is fitted to. The legacy
draws 10 to 15 (`PLATES = (10, 5)`); 15 has no source. No source scales
the count with the planet's size (searched in the drafts: van Heck &
Tackley 2011, Korenaga 2010, Valencia et al. 2007 discuss whether plates
exist, not how many) [S]: the count stays independent of size.

`PLATE_WEIGHT` (0.35, the spread of the weighted Voronoi centres) has no
source: a calibration to fit the area law above. The 300 subplates and
their warp are a discretisation of the plate outlines (Definition).

## Plate speeds and slab pull

Zahirovic et al. 2015, 3952 plate-time samples over 200 Myr: median RMS
speed 4.2 cm/yr over all samples; plates with no subducting edge 2.6
cm/yr; under 25 % of the perimeter subducting 4.4; any subduction 7.1;
more than 25 % subducting 7.9 (section 3.5, p. 45, Fig. 5A) [V].

Retained: **2.6 to 7.9 cm/yr**, the medians of the two ends (legacy 2 to
10), and **25 %** of the perimeter subducting as the share for the fast
end (legacy 0.3). The paper gives classes, not a saturation: how the speed
rises between 0 and 25 % stays a calibration, with 4.4 cm/yr under 25 %
as its check point. The figures are RMS speeds over a plate; the legacy
sets the speed at the equator of the Euler pole, so the generator converts
through its own plate geometry rather than copying them.

`STAGE_MA` (50 Ma between drift stages) only serves the drift, which W2
does not port: not needed.

## Trenches

Zhang et al. 2018, flexural fits to the global array of trenches: the
trench relief W0 "is in the range of 0.9-4.7 km", the trench width "in
the range of 31-93 km" (sections 3.2-3.3, Table 3) [V]. W0 is the
deflection at the axis relative to the undeformed plate; X0, the width,
is the distance from the axis to where the deflection crosses zero
(Fig. 4c caption) [V].

Retained: relief **0.9-4.7 km**, width **31-93 km** (axis to zero
crossing). Caldwell et al. 1976 add an outer rise of 0.28-0.50 km about
100 km seaward [V, open copy]; not retained (the legacy has none).

## Arcs: slab depth and slab dip

- **Depth of the slab under the arc.** Syracuse & Abers 2006: 839
  volcanic centres; averaged over 500 km arc segments, the depth to the
  top of the slab "ranges from 72 to 173 km with a global average of
  105 km" [V, abstract]. England & Katz 2010: "between about 80 and about
  150 km" [V, author copy, p. 1].
- **Slab dip.** Lallemand, Heuret & Boutelier 2005, 159 transects: mean
  shallow dip (0-125 km) 32° ± 11°, deep dip 58° ± 14° (section 4,
  paragraph 18, p. 7); medians of the shallow dip 42.5° ± 9.5° under
  oceanic upper plates (42 transects) and 27.5° ± 8.5° under continental
  ones (117 transects) (section 4.4, paragraph 29) [V, HAL open copy
  hal-01261567]. The 32 ± 9° quoted in reviews is their mean without the
  near-edge transects (29° ± 9°, 114 transects) mixed with the overall
  value [S]; the overall 32° ± 11° is retained.
- **Rule.** The arc lies at the horizontal distance `depth / tan(dip)`
  from the trench, the relation England & Katz use (their Fig. 1) [V];
  Jarrard 1986 finds the arc-trench gap correlated with 1/tan(dip)
  (R = 0.907) and the arcs "nearly always" above the slab at about
  100-125 km [V].

Retained: **105 km** and **32°**, so 168 km on average [D]; with the
upper-plate medians, 115 km for an island arc (42.5°) and 202 km for an
Andean margin (27.5°) [D], against the legacy's 120 and 220 km.

Gill 1981 (*Orogenic Andesites and Plate Tectonics*) supports no number
in this study: dropped.

## Rifts

Hopper & Buck 1996: the Baikal and East African rifts are "less than
100 km wide" (introduction, p. 20175) [V]. Buck 2007: narrow rifts are
"on the order of 100 km wide"; a flexural model gives a shoulder of about
3 km, "on the high-end of the range of uplifts seen" [V, proof copy].

Retained: rift width **under 100 km**. The shoulder and axis amplitudes
(legacy 0.5 and 1.5 km) have no typical value: calibrations bounded by
0-3 km.

## Orogens

- **Collision (Tibet).** Royden, Burchfiel & van der Hilst 2008: the
  plateau "stands 5 km high over approximately 3 million km²", crust
  thickened to about 80 km (p. 1054) [V].
- **Andean (Altiplano-Puna).** Allmendinger et al. 1997: 1800 km long, a
  width of 350-400 km (abstract) and a mean elevation of 3.65 km between
  13 and 29°S (after Isacks 1988; p. 142 of the review) [V]. DOI
  10.1146/annurev.earth.25.1.139 checked on Crossref (the drafts flagged
  it as unchecked).
- The Alps, about 2 km mean [S, Kissling et al. 2017 snippet]: not
  retained.

Retained: collision height **5 km** (Tibet, an upper bound);
Andean uplift **3.65 km** over **350-400 km**. The width of a collision
orogen has no single published number: a calibration bounded by the
Altiplano (350-400 km) and Tibet (about 1000 km across [S]).

## Ridges

Parsons & Sclater 1977: "Between 0 and 70 m.y. B.P. the depth can be
represented by the relation d(t) = 2500 + 350 t^½ m" [V, abstract].
Retained as the legacy has it.

## Island arcs

Not found: a mean uplift profile of island arcs (searched arc heights,
island-arc edifice elevations, forearc topography statistics in the
drafts). The legacy's 3 km at 120 km stays a calibration.

## Convergence thresholds

Not found: no paper gives a convergence rate below which a trench,
orogen or arc does not form (Jarrard 1986's 39 zones and Heuret &
Lallemand 2005 hold the data, not a threshold). `RELIEF_SPEED`
(0.5-5 cm/yr) and `ARC_SPEED` (2 cm/yr) stay calibrations.

## Not found

- Plate count against planet size or mass.
- Island-arc uplift; rift shoulder and axis amplitudes; collision orogen
  width; convergence thresholds.
- Trench and arc geometry against gravity or plate thickness (the flexure
  lead, trench width as `g^-1/4` at fixed rigidity, is a derivation for a
  later study, not a value).

## Sources

1. Bird, P. 2003, "An updated digital model of plate boundaries",
   *Geochemistry, Geophysics, Geosystems* 4, 1027,
   [doi:10.1029/2001GC000252](https://doi.org/10.1029/2001GC000252).
2. Morra, G., Seton, M., Quevedo, L. & Müller, R. D. 2013, "Organization
   of the tectonic plates in the last 200 Myr", *Earth and Planetary
   Science Letters* 373, 93-101,
   [doi:10.1016/j.epsl.2013.04.020](https://doi.org/10.1016/j.epsl.2013.04.020).
3. Sornette, D. & Pisarenko, V. 2003, "Fractal plate tectonics",
   *Geophysical Research Letters* 30, 1105,
   [doi:10.1029/2002GL015043](https://doi.org/10.1029/2002GL015043),
   read as [arXiv:cond-mat/0202320](https://arxiv.org/abs/cond-mat/0202320).
4. Zahirovic, S., Müller, R. D., Seton, M. & Flament, N. 2015, "Tectonic
   speed limits from plate kinematic reconstructions", *Earth and
   Planetary Science Letters* 418, 40-52,
   [doi:10.1016/j.epsl.2015.02.037](https://doi.org/10.1016/j.epsl.2015.02.037).
5. Zhang, F., Lin, J., Zhou, Z., Yang, H. & Zhan, W. 2018, "Intra- and
   intertrench variations in flexural bending of the Manila, Mariana and
   global trenches", *Geophysical Journal International* 212, 1429-1449,
   [doi:10.1093/gji/ggx488](https://doi.org/10.1093/gji/ggx488).
6. Caldwell, J. G., Haxby, W. F., Karig, D. E. & Turcotte, D. L. 1976,
   "On the applicability of a universal elastic trench profile", *Earth
   and Planetary Science Letters* 31, 239-246.
7. Syracuse, E. M. & Abers, G. A. 2006, "Global compilation of variations
   in slab depth beneath arc volcanoes and implications", *Geochemistry,
   Geophysics, Geosystems* 7, Q05017,
   [doi:10.1029/2005GC001045](https://doi.org/10.1029/2005GC001045).
8. England, P. & Katz, R. 2010, "Global systematics of arc volcano
   position", *Nature* 468, E6-E7,
   [doi:10.1038/nature09154](https://doi.org/10.1038/nature09154).
9. Lallemand, S., Heuret, A. & Boutelier, D. 2005, "On the relationships
   between slab dip, back-arc stress, upper plate absolute motion, and
   crustal nature in subduction zones", *Geochemistry, Geophysics,
   Geosystems* 6, Q09006,
   [doi:10.1029/2005GC000917](https://doi.org/10.1029/2005GC000917),
   read as [HAL hal-01261567](https://hal.science/hal-01261567).
10. Jarrard, R. D. 1986, "Relations among subduction parameters",
    *Reviews of Geophysics* 24, 217-284,
    [doi:10.1029/RG024i002p00217](https://doi.org/10.1029/RG024i002p00217).
11. Hopper, J. R. & Buck, W. R. 1996, "The effect of lower crustal flow on
    continental extension and passive margin formation", *Journal of
    Geophysical Research* 101, 20175-20194,
    [doi:10.1029/96JB01644](https://doi.org/10.1029/96JB01644).
12. Buck, W. R. 2007, "Dynamic processes in extensional and compressional
    settings: the dynamics of continental breakup and extension",
    *Treatise on Geophysics* 6, 335-376,
    [doi:10.1016/B978-044452748-6.00110-3](https://doi.org/10.1016/B978-044452748-6.00110-3).
13. Royden, L. H., Burchfiel, B. C. & van der Hilst, R. D. 2008, "The
    geological evolution of the Tibetan Plateau", *Science* 321,
    1054-1058,
    [doi:10.1126/science.1155371](https://doi.org/10.1126/science.1155371).
14. Allmendinger, R. W., Jordan, T. E., Kay, S. M. & Isacks, B. L. 1997,
    "The evolution of the Altiplano-Puna plateau of the Central Andes",
    *Annual Review of Earth and Planetary Sciences* 25, 139-174,
    [doi:10.1146/annurev.earth.25.1.139](https://doi.org/10.1146/annurev.earth.25.1.139).
15. Parsons, B. & Sclater, J. G. 1977, "An analysis of the variation of
    ocean floor bathymetry and heat flow with age", *Journal of
    Geophysical Research* 82, 803-827,
    [doi:10.1029/JB082i005p00803](https://doi.org/10.1029/JB082i005p00803).
