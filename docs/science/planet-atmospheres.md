# Planet atmospheres, climate and surface (part 2)

Part 2 of the physical planet model (issue #423) derives what a planet's
interior does not fix: whether it keeps its envelope, its tectonic
regime, atmosphere, climate, liquid and life, and reads the surface
classes off them. The sections follow the order of the derivation, from
the XUV fluence to the classes, and end with a census of drawn systems.
The interiors are in [planet-interiors.md](planet-interiors.md).

## XUV fluence

`science::xuv::history` reads one star's high-energy history once;
`History::exposure` then gives the XUV flux a planet receives today and
the XUV energy per unit area it has received over the star's age, both as
multiples of the star's present bolometric flux at the planet, and
`History::integrated_between` the energy between two ages. It is the
budget that drives photoevaporation. The same history serves the Lyman α
line (`LYMAN_ALPHA`), which photolyses methane. The sources were checked
in the study `docs/superpowers/studies/2026-11-21-xuv-history.md`. The
sections after this one still describe the `legacy-final` planet model
(`xuv_fluence`, `EARTH_XUV_FLUENCE_J_M2`) until it is ported (#602).

**The Sun.** Ribas et al. 2005 (ApJ 622, 680, doi:10.1086/427977, Table 5)
measure the Sun's XUV flux (1 to 1180 Å) at 1 au against its age from five
solar analogues aged 0.1 to 6.7 Gyr: `F = 29.7 τ^-1.23` erg s⁻¹ cm⁻², τ in
Gyr, and `F = 19.2 τ^-0.72` for Lyman α (their eq. 2, "preliminary").
That is a power law in absolute flux, while the Sun's bolometric
luminosity rose by 40 % since its zero-age main sequence (ZAMS). So the
band luminosity is a share of the star's ZAMS luminosity: 5.220e-4 for
the XUV while saturated (1.043e-4 for Lyman α), which makes the 1 M☉,
[Fe/H] 0 track of the stellar tables follow Ribas et al.'s fit at every
age, within 0.2 %. The `legacy-final` code took 3.71e-4 of the present
luminosity: exact for the Sun, but for a star whose luminosity changed
more or less than the Sun's at its age, the history scaled with today's
luminosity instead of its ZAMS one.

**Saturation.** The share holds for `t_sat`, a labelled calibration
interpolated in log-log through three anchors, not a published relation:

| Star mass | `t_sat` | Anchor |
| --- | --- | --- |
| 1 M☉ | 0.1 Gyr | Jackson, Davis & Wheatley 2012 (MNRAS 422, 2024, Table 2): `log τ_sat = 7.84` (70 Myr) for the B−V bin that holds the Sun, 10^7.8 to 10^8.3 yr over FGK stars; 0.1 Gyr is where Ribas et al.'s fit starts |
| 0.5 M☉ | 0.2 Gyr | Johnstone et al. 2021 (A&A 649, A96, doi:10.1051/0004-6361/202038407): slow and fast rotator tracks "almost identical until 200 Myr", read as the end of saturation |
| 0.25 M☉ | 2 Gyr | the same, "2 Gyr" |

Between the anchors `t_sat` goes as `M^-1` above 0.5 M☉ and `M^-3.3`
below, extended past the ends: 4.2 Gyr at 0.2 M☉, 42 Gyr at 0.1 M☉,
saturated all its life. Then the band luminosity decays as Ribas et al.'s
power of age. One saturated share serves every convective star: the
saturated X-ray share is about flat with spectral type (Wright et al.
2011, ApJ 743, 48, `log R_X = -3.13`).

**Stars without a corona.** A star at or above 7200 K has no convective
envelope (Gray & Nagel 1989, the limit the legacy star activity used),
hence no dynamo and no coronal XUV: A stars show no X-rays of their own
(Schröder & Schmitt 2007), B stars about `L_X/L_bol ≈ 1e-7` from wind
shocks. `history` reads the star's effective temperature along its track
and counts the XUV only at the ages below the limit: a 1.6 M☉ star born
hotter gains its corona as it cools. The decay runs on the star's age, so
such a star starts its corona at the decayed level, while a star above
the Kraft break has not spun down: its XUV is a lower bound. The Lyman α
line is chromospheric, and photospheric in hot stars: it is not cut, and
an A or B star gets the solar law, a lower bound too.

**Before the ZAMS** `science::stellar::main_sequence` gives no values
(the star's age must lie on its main sequence, as every drawn star's
does): the star is taken at its ZAMS temperature, and the band at its
saturated share of the ZAMS luminosity, from age 0. A contracting star is
brighter (0.08 M☉ is 11 times its ZAMS luminosity at 0.05 Gyr in Baraffe
et al. 2015), so this is a lower bound, large for late M dwarfs, whose
ZAMS comes after 1 Gyr.

Calibration targets: the Sun of the tables follows Ribas et al.'s fit
from 0.1 to 6.7 Gyr within 0.2 %; Earth's fluence over 4.567 Gyr is
`EARTH_XUV_FLUENCE` = 5.64e15 J/m², the fit integrated (held at its
0.1 Gyr value before), and today's flux 4.6 mW/m²; Lyman α today is
about 3.9e11 photons cm⁻² s⁻¹ (Ribas et al.).

Where it departs from the science: the share is the Sun's for every
convective star, while Jackson et al. measure the saturated X-ray share of
early F stars about 10 times below the Sun's; the spread from the initial
rotation (saturation from about 6 to 230 Myr at 1 M☉, Tu et al. 2015) is
not drawn; the larger Lyman α share of an M dwarf's light, the
photospheric EUV and Lyman α of hot stars and the brighter contracting
star before the ZAMS are not modelled; the orbit is taken as it is today.

## Envelope escape

`planet::escape` removes the hydrogen and helium a planet loses to the
star's XUV light. It is the energy-limited escape of Owen & Wu 2017
(doi:10.3847/1538-4357/aa890a, eq. 19, after Lecavelier des Etangs 2007
and Erkaev et al. 2007), integrated over the age:

```text
ΔM = η π R³ F / (G M)
```

with `F` the XUV fluence in J/m² (the relative `xuv_fluence` times
`EARTH_XUV_FLUENCE_J_M2`), `R` the planet's radius with its envelope, `M`
its mass and `η = 0.1`, the constant efficiency Owen & Wu adopt ("of
order 0.1 for low-mass planets", section 2.2, read; the paper's `Ṁ = η π R³
L_HE / (4 π a² G M)` is the same law, integrated here over the fluence). The
efficiency really falls for deep potential wells (their eq. 31, `η = 0.1 (v_esc
/ 15 km/s)^-2`, which their own footnote says is "only suitable in a narrow
range of parameter space"); the
model ignores that, since a giant loses next to nothing at `η = 0.1`
already. `energy_limited_loss_kg` gives `ΔM`; the water loss of the next
section reuses it.

The rewrite ports the law to `science::escape::energy_limited_loss`
(typed radius, mass and fluence in J/m², uncapped), with `EFFICIENCY`,
`MIN_ENVELOPE` and `WATER_PER_HYDROGEN`, the 9 kg of water per kg of
hydrogen of the next section. The system engine's step is
`system::planet::escape::escaped`: it reads the star's XUV history, applies
the law to the body the boil-off left (next section) and returns the trial
before and after the loss (the water step below needs the first), each
value with its cause node.

`envelope_after_escape` caps the loss at the envelope, lowers the mass by
what is lost, and sets the envelope to zero once it falls under 0.1 % of
the mass (`MIN_ENVELOPE`): what is left is a thin atmosphere, not a
hydrogen shell. The 0.1 % is a calibration: Owen & Wu 2017 start their
envelopes at 1 % or more (`X_min = 0.01`, section 3.3) and find the planets
that keep one near 1 %, with no threshold under which it counts as gone, and no
other paper read gives one ([study
21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit)).
The radius is the one before the loss, an upper bound.
`planet_model` then derives the interior again from what is left, and
records `xuv_fluence` and `envelope_lost`, the share of the initial
envelope lost. Nothing is tuned to the radius valley's position; the
census of the valley is measured at the end of part 2.

Calibration targets: a 5 M⊕ planet with a 1 % envelope at 0.05 AU round
the Sun is stripped and its radius falls under 1.8 R⊕ (with a 2 %
envelope it is stripped at 0.05 AU and keeps about a third of it at
0.1 AU, where the fluence of before Ribas et al.'s normalisation, 2.7
times higher, stripped it); Jupiter at
0.05 AU keeps over 99 % of its envelope; 5 M⊕ with 1 % at 0.5 AU keeps at
least half; Earth, Uranus and Neptune are unchanged.

### Boil-off and core-powered mass loss (study 30)

Before the XUV law above, a nebular envelope loses what the star's
bolometric light boils off. Two losses are driven by that light, not
the XUV, so they act round a host without a corona too (an A star). This
section is the study of issue #697
([study 30](https://github.com/cyboooooorg/map-generator/wiki/30-core-powered-loss-audit))
and the law the rewrite implements from it:
`science::escape::boil_off_ceiling`, chained by
`system::planet::boil_off::boiled_off`.

**Model.** When the disc goes, a young planet that fills its Bondi
sphere loses its outer layers as an isothermal Parker wind, cools and
shrinks to about a tenth of the Bondi radius within about a million
years: the *boil-off* (Owen & Wu 2016; Ginzburg, Schlichting & Sari
2016). What is left can keep losing gas for gigayears, lifted by the
heat of the core and the envelope: *core-powered mass loss* (Ginzburg,
Schlichting & Sari 2018; Gupta & Schlichting 2019, 2020). The rate is
the smaller of an energy-limited and a Bondi-limited one (Gupta &
Schlichting 2019, eqs. 5, 6 and 8):

```text
Ṁ_E = L / (g R_c)
Ṁ_B = 4π R_s² c_s ρ_rcb exp(−G M_p / (c_s² R_rcb)),   R_s = G M_p / (2 c_s²)
L   = 64π σ T_eq⁴ R'_B / (3 κ ρ_rcb),   R'_B = ((γ−1)/γ) G M_c µ / (k_B T_eq)
```

with `c_s = (k_B T_eq / µ)^(1/2)`, `κ` the opacity at the
radiative-convective boundary (Freedman et al. 2008, 500 to 2 000 K) and
`T_eq` the zero-albedo equilibrium temperature. Integrated over the age,
it puts the largest stripped core at `R ∝ P^(−1/9)`, the radius valley's
slope (study 30 reproduces it: −0.110 from 8 to 64 days round the Sun).

The two are one outflow, bolometrically heated above the
radiative-convective boundary, with different starting states (Owen &
Schlichting 2024, § 2). Tang, Fortney & Murray-Clay 2024 run both in one
evolution model with a self-consistent start, and find that after the
boil-off the core-powered stage removes at most 0.1 % of the H/He mass
fraction over 3 Gyr, against about 10 % in the boil-off (abstract and
§ 3.5). Their Table 2, the largest H/He fraction a planet keeps after the
boil-off, whatever it started with below 20 %:

| Flux (F⊕) | 1.5 M⊕ | 2.4 | 3.6 | 5.5 | 8.5 | 13 | 20 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1.18 % | 3.38 % | 8.62 % | − | − | − | − |
| 3 | 0.49 % | 1.79 % | 4.93 % | 14.64 %\*\* | − | − | − |
| 10 | 0.14 % | 0.77 % | 2.56 % | 8.85 % | − | − | − |
| 30 | 0.02 % | 0.25 % | 1.17 % | 4.44 % | 16.75 %\*\* | − | − |
| 100 | 2 × 10⁻⁶\* | 0.03 % | 0.30 % | 1.67 % | 7.21 % | 23.49 %\*\* | − |
| 300 | 0 | 6 × 10⁻⁶\* | 0.04 % | 0.53 % | 3.14 % | 11.13 %\*\* | − |
| 1000 | 0 | 0 | 4 × 10⁻⁶\* | 0.04 % | 0.73 % | 4.04 % | 13.50 %\*\* |

"−" is "zero or very slight mass loss". The starred values (\*:
2 × 10⁻⁶, 6 × 10⁻⁶ and 4 × 10⁻⁶) are planets that lost the whole
envelope, the value a radiative layer alone. The double-starred ones
(\*\*, above 10 %) were started from a larger envelope than the
default 10 % to find the largest one kept. Photoevaporation "should
start right after boil-off around 1 Myr" (Tang et al. 2024, § 3.3).

**The law.** One bolometric law, then the XUV law, never two
bolometric ones (the same outflow would be counted twice): Gupta &
Schlichting's slow loss is not added on top of the ceiling.

1. `science::escape::boil_off_ceiling(core_mass, flux)`: Table 2,
   bilinear in `log M_c` and `log F` of `log f`, so it reads the table's
   cells back exactly:

   ```text
   log c = Σ w_ij log f_ij,   w bilinear in (log M_c, log F)
   ```

2. In the chain (`system::planet::bare::formed`), after the body is
   formed on its drawn envelope `f` and before `escaped`: when `f > c`
   the fraction left is `c`, the mass `M_c / (1 − c)`, and the body is
   built again. The ceiling (`planet.envelope.boil-off-ceiling`, from the
   mass without the envelope and the bolometric flux), the fraction left
   (`planet.envelope.after-boil-off`), the mass left
   (`planet.mass.after-boil-off`) and the envelope boiled off
   (`planet.envelope.boiled-off`) are recorded with their causes; when
   `f ≤ c` nothing is recorded. The planet carries the envelope boiled
   off (`envelope_boiled`) next to the one the XUV escape took
   (`envelope_lost`), so that an explanation reaches both. Only a
   mini-Neptune is read: not from 20 M⊕ (`GAS_GIANT_MIN_MASS`, the
   grid's end; the giants are #497).
3. Then the XUV escape and `MIN_ENVELOPE` (kept at 1e-3), as above, on
   what is left.

The core mass is the mass without the envelope; the flux is the present
bolometric flux of all stars, the only one the timeline holds (the
boil-off happens about 1 Myr after the disc, before the ZAMS the stellar
tracks start at; #700 tracks the flux at disc dispersal). The
alternative, Gupta & Schlichting 2020's evolution over the age in place
of the ceiling, strips the same reference planets and gives the share of
super-Earths growing with age that the ceiling lacks, but starts from a
state Tang et al. show overestimates the later loss, and its rates are
disputed by orders of magnitude (Misener et al. 2025; Modirrousta-Galian
& Korenaga 2026).

**Calibration.** Two values no source gives, searched in study 30 (no
published ceiling off Tang et al.'s grid, no value for their "−"):

- The off-grid extension (`science::escape`'s `log_log_read`, study 30's
  new calibration 1): off the grid (below 1.5 M⊕, below 1 F⊕, above
  20 M⊕ or 1 000 F⊕), the end segment of each axis is extended in log-log
  from the grid's edge, as `science::xuv`'s saturation. The physics fixes
  the direction (lighter and hotter planets lose more, Owen & Wu 2016,
  conclusion 5), not the slope. Off both axes at once the two axis
  extensions add, without the bilinear cross term: a corner rule chosen
  at review (2026-11-25), part of this calibration and not a new one.
  With the cross term, which grows with both distances, a 0.3 M⊕ core
  would be given back 3.6 % of envelope at 3 000 F⊕ (and a 0.2 M⊕ core
  1.9 % at 2 300 F⊕), next to a cell (1.5 to 2.4 M⊕, 300 to 1 000 F⊕)
  that keeps nothing at its four corners; without it, the ceiling there
  stays at the corner's 1e-7. Clamping at the edges instead changes none
  of the 22 reference outcomes; it keeps more on Earth (1.18 % against
  0.48 %) and on ref 8's hot Neptunes (2.6 % and 7.3 % against 0.28 %
  and 2.6 %).
- `NO_BOIL_OFF`: a "−" cell reads 0.20, the highest envelope drawn below
  20 M⊕ (Lopez & Fortney 2014's last column): a ceiling there never cuts
  a draw.

`NOTHING_LEFT`: a "0" cell reads 1e-7, a definition: every neighbour is
under `MIN_ENVELOPE` (the starred cells are 2 to 6 × 10⁻⁶), so the value
decides nothing. A ceiling read above one is one.

**Validation** (study 30, derived from the table; the tests of
`science::escape` and `system::planet::boil_off` check each line):

- The 32 numeric cells of Table 2 read back to 1e-12; the 14 "−" cells
  are at least 0.20; the 3 "0" cells and the three starred ones are under
  `MIN_ENVELOPE`. Wherever the ceiling lies between 1e-5 and 0.15, it
  falls with the flux and rises with the core mass, on and off the grid.
- The largest core the ceiling strips (under `MIN_ENVELOPE`) is 1.37 M⊕
  at 10 F⊕, 2.02 M⊕ at 30 F⊕, 2.97 M⊕ at 100 F⊕ and 6.31 M⊕ at
  1 000 F⊕: with `R ∝ M^(1/4)` (Lopez & Fortney 2014, eq. 1),
  `R ∝ F^0.081` from 30 to 1 000 F⊕.
  Measured: `α = 0.069 (+0.019/−0.023)` (Berger et al. 2023), 0.08
  predicted for core-powered loss (Rogers et al. 2021). Round one star,
  −0.108 in period against −0.09 (+0.02/−0.04) (Van Eylen et al. 2018)
  and −0.10 ± 0.03 (Petigura et al. 2022). The line sits at 1.1 R⊕
  (10 F⊕) to 1.6 R⊕ (1 000 F⊕), under the observed valley (about 1.9 R⊕
  at 10 days): the boil-off "cannot by itself form a pronounced radius
  gap", the XUV law adds to it.
- Earth (1 M⊕, 1 F⊕): ceiling 0.48 %, so a nebular envelope above it is
  trimmed but not removed; Earth's bareness comes from capturing little
  (Lammer et al. 2014), not from this law. In the chain, an Earth at
  1 au with Lammer et al.'s largest envelope, 2.5 %, keeps 0.40 % after
  the boil-off and the XUV escape (2.29 % before the boil-off was
  chained).
- HD 56414 b, a Neptune of an 8 500 K A star at 274 F⊕ (Giacalone et al.
  2022), keeps 2.6 % at 8 M⊕ and 13 % at 14 M⊕: consistent with its
  existence. With Lopez & Fortney 2014's envelopes at 420 Myr, the
  ceiling's envelope reaches its 3.71 ± 0.20 R⊕ from a core of about
  10 M⊕ (3.74 R⊕), 5.1 R⊕ at 14 M⊕ and 3.1 R⊕ at 8 M⊕: the law puts its
  core in the upper half of Giacalone et al.'s range. The test checks
  both ends: the 8 M⊕ core is not above 3.91 R⊕, the 14 M⊕ core not
  below 3.51 R⊕.
- A planet of an A star without XUV, a 1.92 M⊕ core with 2.74 % at
  802 F⊕ (ref 8 d), is rocky after the chain; before the boil-off was
  chained it kept its 2.74 %.
- The reference systems: four placements lose their envelope, ref 6 c, d
  and ref 8 d, e (cores of 0.57 to 3.7 M⊕ at 228 to 802 F⊕ round A
  stars, envelopes of 0.1 to 3.9 %, ceilings under 2e-5); ref 8's hot
  Neptunes and ref 7's earth-like planet keep theirs.

**Limitations.** Tang et al.'s grid is for rocky cores round a Sun-like
star at constant flux; icy cores are more fragile, and the model reads
the mass without the envelope, water included, as the rocky core. Tang
et al. do not say whether their fraction is of the planet's mass or of
the core's; the model reads it as of the planet's (the mass left is
`M_c / (1 − c)`). Next to the "−" cells the calibration sits under a
published value (13 M⊕: "−" at 30 F⊕, 23.49 % at 100 F⊕), so in the
cell from 8.5 to 13 M⊕ and 30 to 100 F⊕ the ceiling rises with the
flux for cores from 12.15 to 13 M⊕, against the physics' direction. It
is lowest there at 12.15 M⊕ and 30 F⊕, 0.194, where it trims a 20 %
draw by about 2.8 % of itself; the flux where it reaches 0.20 depends
on the core (about 40 F⊕ at 12.5 M⊕, 32 F⊕ at 12.8 M⊕; under about
12.2 M⊕ it stays under 0.20 up to 100 F⊕). The present flux stands for
the flux at disc dispersal. The near-ultraviolet escape of hot stars
(Balmer-driven, García Muñoz & Schneider 2019), which strips Neptunes of
A stars within 0.1 AU (Giacalone et al. 2022), is a separate channel the
model lacks (#701). The energy-limited rate of Owen & Schlichting 2024 (eq. 17)
divides the XUV law by 4 for the same `η` (#702).

## Runaway greenhouse, water kept and tectonic regime

`planet::regime::water` and `planet::regime::regime` (called by
`planet_model`).

**Runaway.** A planet whose insolation exceeds the runaway limit of
`star::habitable_fluxes` (Kopparapu et al. 2014,
doi:10.1088/2041-8205/787/2/L29, for a 1 M⊕ planet) cannot keep liquid
water: the oceans sit in the air as vapour. The mass correction of that
paper is not applied (not checked). There the XUV splits the water and
the hydrogen escapes at the energy-limited rate of the previous section
(Owen & Wu 2017), 9 kg of water for each kg of hydrogen:
`kept = max(0, water - 9 H lost)` (`regime::steam`). Without a runaway
the planet keeps its water. `oceans` counts it in units of 1.4e21 kg (a
definition: Earth's ocean is 1.332e9 km³, Charette & Smith 2010,
doi:10.5670/oceanog.2010.51, Table 1, which is 1.37e21 kg at the usual 1027
kg/m³, a density not read here; the unit is rounded up). The climate can find a
runaway this limit misses: when a wet surface
has no balance under the moist limit (its absorbed light and internal
heat exceed what its moist column sends out, `climate::boils`), the
ocean is steam and the chain derives the body again from
`regime::steam`; the water its escape leaves condenses again only if it
has a wet balance of its own (see
[the chain](#the-chain-and-the-surface-classes)). `Water::boiled`
records such a boil rather than the insolation limit: with `runaway`,
the steam or dry air that stands; without, the leftover that condensed
again. In a runaway all the water kept is steam in
the air: `Water::kept_kg` is what escape to space leaves of it, and the
air's H₂O is that water's weight.

The water escape took leaves the planet's mass too (#513,
`planet::after_steam`): `PlanetModel.inputs` holds the mass less that
water, the water share of what is left, and the envelope and volatiles
at the same mass (their shares grow). The layers, gravity and internal
heat are derived again from it before the steam pass runs, so the
climate's column sees the lighter body. That pass's own escape (the
cosmic shoreline and the non-thermal loss, below) then takes more of
the steam: the mass and the layers are derived once more from the water
it kept (`Water::kept_kg`, what the card shows), without running the
climate again, so the mass, the water card and both cause trees agree;
the heat stays the one the climate ran on, and the explanation finds
that body again from the water the pass started with
(`explain::surface_inputs`). `PlanetModel.before_steam`
keeps the inputs from before the runaway, when escape took water: the
request's formation is
read from them, and `drawn_mass_earth` stays the mass drawn. The cause
tree shows the mass from the mass formed and the water kept, with the
water formed and the water lost.

The rewrite splits this section in two. `system::planet::water::water`
sums the runaway greenhouse over every light source (#592: the threshold is
`science::habitable::runaway_flux` at each star's effective temperature,
`OCEAN_MASS` the ocean's unit), takes the steam's loss from
`science::escape::energy_limited_loss` and records the water kept;
`after_steam` derives the lighter body again. `system::planet::regime::regime`
reads `science::tectonics::{HEAT_PIPE_ONSET_FLUX, PLATES_MIN_MASS}`. Neither
recomputes a value to explain it: the cause tree is the computation's.

**Regime.** A labelled calibration, since the literature disagrees on the
mass dependence (see [plate tectonics](plate-tectonics.md)):

- heat pipe above 1 W/m² of internal heat (`HEAT_PIPE_ONSET_FLUX`, the
  regime's onset; `HEAT_PIPE_FLUX` before issue #670), a calibration:
  no paper sets a threshold. Io is the heat-pipe body of today, carrying about
  2.5 W/m², some 40 times Earth's mean (Moore & Webb 2013, "Heat-pipe
  Earth", doi:10.1038/nature12473, from Veeder et al. 2004; checked);
  Lainey et al. 2009 (doi:10.1038/nature08108) measure
  Io's tidal dissipation from its orbit, with 2.24 ± 0.45 W/m² as the legacy
  had it; the paper was reached only through its abstract record, so that
  figure is not checked. 1 W/m² keeps Io a heat pipe even at the low end of
  that range;
- plates from 0.3 M⊕ (`geology::PLATES_MIN_MASS_EARTH`, shared with the
  generator; the app's choice, the geometric mean of Mars's 0.107 M⊕ and
  Venus's 0.815 M⊕, `√(0.107 × 0.815) = 0.295`, where Foley, Bercovici &
  Landuyt 2012 favour larger planets and Korenaga 2010 finds size does not
  matter) when liquid water is kept, never in a runaway (its steam
  does not weaken the lithosphere);
- episodic when the mass allows plates but the water is gone or is steam
  (Venus);
- stagnant otherwise (Mars, Mercury, the Moon).

Calibration targets: Earth keeps 0.9 to 1.1 oceans and has plates; Venus
is in runaway, loses even an Earth ocean and is episodic; Mars and
Mercury are stagnant; Io with 1e14 W of tidal heat is a heat pipe, and
still is with 7.5e13 W (1.79 W/m², the low end of Lainey et al.); a dry
5 M⊕ planet outside the runaway has no plates.

### Surface water

Legacy drew a planet's ocean directly (Earth's `water_fraction` was
2.3e-4, its ocean). The rewrite draws the **bulk** water, the ocean plus
the water bound in the mantle (log-uniform from 1e-5 to 1e-2 inside the
ice line; Earth's is 2700 ppm, Marty 2012, the reference of the share,
not the centre of the draw). The ocean is then a share of
it (#663, `system::planet::water::surface_water`, node
`planet.water.surface`):

```text
surface = s · w (1 - f) M      for a dry planet, its water in the mantle
surface = its water layer      for a water world (or its water beyond the laws)
s = ω_o / ω_⊕ = 2.3e-4 / 2.7e-3 = 0.085   (SURFACE_WATER_SHARE)
```

A water world's water is a layer of its structure, not hydrated rock
([planet interiors](planet-interiors.md), "Water inside the ice line"):
all of it is at its surface, and its node (`planet.water-layer-mass` or
`planet.water-beyond-law`) is the surface water as it is. The runaway's
steam is the surface water: `min(9 H, surface)` is lost, the mantle's
water stays, and the mass after the steam is `M - (surface - kept)`
(`after_steam`). A boil the climate forces (`water::boiled`) loses the
same steam as the light's (one `steam_lost`) and derives `runaway` and
`boiled` from the boil loop's verdict; a leftover that condenses back
(`water::recondensed`) keeps the inventory and is no longer a runaway. The
regime, the seafloor's hold on the outgassing
(`released_under_ocean`) and the air's water read the water kept of the
surface water, never the bulk.

**Calibration.** `science::habitable::SURFACE_WATER_SHARE` is a
`Calibration:`, the decision of #663 (option A of
[study 22](https://github.com/cyboooooorg/map-generator/wiki/22-surface-water)):
Earth's ocean as a share of its mass, $\omega_o = 2.3 \times 10^{-4}$
(Cowan & Abbot 2014, Table 1; 1.37e21 kg, the ocean alone, without the
0.19 ocean of the continental crust, Korenaga et al. 2017, §2a), over
Earth's bulk water, 2700 ppm (Marty 2012, arXiv author copy, p. 18:
2700 ± 1350 ppm,
"10 ± 5 ocean masses"). The same share for every mass and regime. Both
numbers are `Source:` constants; the ratio is computed from them.
Searched and not found (study 22): a measured or modelled surface share
for any planet other than Earth with plates; a stagnant-lid water
outgassing law as a function of mass (Dorn et al. 2018 is for CO₂, Tosi
et al. 2017 for 1 M⊕ only); a share for heat-pipe or episodic planets.

**Validation.** A planet drawn at Earth's 2700 ppm gets 0.98 ocean
(units of 1.4e21 kg; the ocean's 1.37e21 kg exactly), the 0.9 to 1.1 of
the targets above, and keeps plates; Venus given that water runs away and
loses it; the reference Earth's air keeps 0.51 bar of N₂.

**What the draw gives.** The draw is not centred on Earth: a dry 1 M⊕
planet, its bulk water log-uniform from 1e-5 to 1e-2, keeps 0.004 to 3.6
oceans at its surface, a median of about 0.12, and about 19 % of them
hold at least one ocean ($w \ge 2.75 \times 10^{-3}$). Legacy drew the
ocean itself, $10^{U(-4, -2.7)}$ of the mass: 0.43 to 8.5 oceans, a
median of about 1.9. The median dry planet is some sixteen times drier
at its surface than legacy's; the user confirmed this choice, the share
and the draw as they are, on 2026-11-23.

**Limitations.** The share rides on Marty's large bulk: other budgets
give Earth 0.07 to about 0.6 (Ohtani 2021's small budget of 1 to 4
oceans, Korenaga et al. 2017's mantle of 0.56 to 1.3 oceans). A stagnant
lid outgasses less (about 0.02 to 0.05, derived from Tosi et al. 2017),
and the share changes over a planet's life as the mantle degasses and
regasses: both are #665, with the keyframes of T4. Whether a runaway
outgasses more of its mantle than the share is left open.

## Core and dynamo

`planet::core::core` (called by `planet_model`) gives the
state of the iron core (`none`, `liquid` or `frozen`), whether a dynamo
runs and the surface equatorial dipole field in µT. It is a parametrised
thermal history in the style of Stevenson, Spohn & Schubert 1983
(doi:10.1016/0019-1035(83)90241-5) and Driscoll & Barnes 2015
(doi:10.1089/ast.2015.1325), cut down to one cooling time per body, and
a calibration: the targets below are the test. The constants of Stevenson et
al. 1983 could not
be reached (paywalled), so none of them is checked against that paper ([study
21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit)).
The present regime is taken for the whole
history.

The rewrite ports the law to `science::dynamo`: `cooling_time`,
`core_phase` (the regime enters as `plates`, true under plate tectonics),
which gives the dynamo's kind (`Thermal`, which wins under plates when an
inner core grows, or `Compositional`), `rms_field` (the paper's eq. 2 with
the efficiency `F` as an argument) and `surface_field`, which takes that
kind. The system engine's core step (`system`, `planet/core.rs`) reads the
iron shells of a rocky planet's structure and records the cooling time, the
state, whether a dynamo runs and the field, each with its causes. The dipole share
is the four factors below (`DYNAMO_EFFICIENCY`, `DIPOLE_OVER_FIELD`,
`RMS_TO_EQUATORIAL`, `DIPOLE_REMAINDER`); the deep one stays
`DEEP_DIPOLE_SHARE`, on the field taken with `F = 1`.

**Cooling time.** The core's excess heat leaves across the mantle above
it, so it decays as `e^(-t/τ)` with

```text
τ = ρ_core c r_core d / (3 k Nu)
```

where `d = R - r_core` is the mantle's thickness, `c = 840 J/(kg K)` (Earth's
core, printed in Christensen, Holzwarth & Reiners 2009) and `k = 4 W/(m K)` (a
calibration of the order of the mantle's value: Stevenson et al. 1983 were not
reached). The legacy set the calibrated `τ` thresholds below and the Nusselt
numbers with `c = 800` and its own interior. With 840 and the system engine's
structure, `τ` is 4.88 Gyr for Earth (plates), 20.9 for Venus (episodic), 9.90
for Mars, 3.18 for Mercury and 1.58 for a rocky body of the Moon's mass and 2 %
core (all three under a lid), about a fifth longer than the legacy's for
Earth and Mars: two of the three thresholds were refitted on their targets (below),
the Nusselt numbers kept. `Nu` is the convective speed-up by regime: 50 for plates,
10 for episodic, 5 for a
stagnant lid or a heat pipe (calibrations on the bodies of the table below:
Earth's inner core, Venus without a dynamo, Mars, Mercury and the Moon). The
three levers:

- the core's size: a big core holds its heat longer;
- the mantle's thickness: Mercury's thin mantle lets its core cool about
  four times faster than Mars's, which is why Mercury grows an inner
  core today and Mars does not yet;
- the regime: plates cool the core ten times faster than a lid.

Sulphur (`Volatiles::s`) is not used: it would lower the core's liquidus
and delay every phase (Stevenson et al. 1983), but no target needs it.

**Phases.** Under plates the core-mantle heat flux stays above the
adiabat and a thermal dynamo runs until the core freezes (Nimmo &
Stevenson 2000, doi:10.1029/1999JE001216). Under a lid:

| Phase | From | Dynamo |
|---|---|---|
| Thermal | 0 | dipolar |
| Flux below the adiabat | 0.055 τ | none |
| Inner core growing | 0.82 τ | deep, compositional |
| Frozen | 2.25 τ | none |

**Calibration of the thresholds.** Each threshold is a share of `τ` fitted
on one body, checked with the cooling times above (`c = 840`):

- `THERMAL_DYNAMO_END` 0.055 (legacy 0.075): Mars's dynamo ends at 0.545 Gyr,
  4.02 Ga ago, inside the 4.0 to 4.1 Ga of Lillis et al. 2013; the legacy
  value ended it at 0.74 Gyr (3.82 Ga ago), only within Mittelholz et al.
  2020's late field.
- `INNER_CORE_START` 0.82 (legacy 1.0): Earth's inner core starts at
  4.00 Gyr, 0.57 Ga ago (Bono et al. 2019: 565 Ma), Mercury's at 2.61 Gyr,
  Mars's and Venus's after today; the legacy value started Earth's at
  4.88 Gyr, after today.
- `CORE_FROZEN` 2.25 (the legacy's, kept): the Moon's core freezes at
  3.55 Gyr, 1.02 Ga ago, inside the 1 to 2.5 Ga window of Tikoo et al. 2017
  (near its recent edge), Mercury's at 7.16 Gyr. Its target holds, so it
  was not refitted.

What was searched is in study 21, part 4; no paper gives these thresholds
for a lumped law of this form. The system engine's test
`the_phases_meet_their_calibration_targets` checks the three targets.

**Field.** The heat flux out of the core is
`q = ρ c r_core ΔT₀ e^(-t/τ) / (3τ)`, with `ΔT₀ = 2800 K` set so that
Earth's core loses about 10 TW today, the middle of the 5 to 15 TW of Lay,
Hernlund & Buffett 2008 (doi:10.1038/ngeo.2007.44) and Nimmo 2015, both reached
only as quoted by other papers. The energy-flux scaling of Christensen 2010
(doi:10.1007/s11214-009-9553-2) gives the rms field in the core,
`B² / 2µ₀ = 0.63 ρ^(1/3) q^(2/3)` (efficiency and ohmic share taken as
1), with the prefactor 0.63 of a dipolar dynamo (Christensen, Holzwarth
& Reiners 2009, doi:10.1038/nature07626, p. 168, "the constant of
proportionality as c = 0.63"; a multipolar
one has 0.23, Christensen 2010, section 3.4). The paper's own form,
`B² / 2µ₀ = c f_ohm ρ^(1/3) (F q)^(2/3)`, carries an efficiency `F`, 0.52 for
Earth's core with a constant flux (0.27 with none at the top, p. 168), so the
field goes as `F^(1/3)` = 0.80. The dipole share of a dynamo driven from the
top of the core is the product of four factors:

- `F^(1/3)` = 0.804 (`Source:` Christensen et al. 2009);
- the dipole's share of the internal field, 1/7 = 0.143: "we find a typical
  ratio B/B_dip of around seven for dynamos with an Earth-like magnetic power
  spectrum" (same paper, pp. 168-169, the paragraph on the planetary data;
  `Source:`);
- 1/√2 = 0.707, from the rms of a dipole over a sphere to its equatorial value
  (`Definition:`; the paper's 0.26 mT for Earth's core is the rms:
  `0.26 mT × (3480 / 6371)³ / √2` = 30.0 µT);
- a remainder of 0.82, a calibration fitted on Earth's 30 µT surface dipole
  so that the product is the 0.067 the legacy used: `0.067 / (0.804 × 0.143 ×
  0.707)` = 0.825. It holds what the legacy simplifies (the core's heat flow
  against the paper's reference flux, the ohmic share).

Using `F` itself instead of `F^(1/3)` would give 0.90, which is not the
paper's law. The deep dynamo under a stably stratified top, whose field is
multipolar and screened (Christensen 2006, doi:10.1038/nature05342), keeps
one calibration, 3e-4, on Mercury's dipole: the same paper says its scaling
law is "either hard to test or not applicable" at Mercury (a slow rotator
that may be non-dipolar, below a stably stratified layer), so `F`, 1/7 and
`c = 0.63` do not carry over. The factor `(r_core / R)³` carries the field to
the surface.

Calibration targets, with the regimes of the previous section:

| Body | Age | Result | Observed |
|---|---|---|---|
| Earth | 4.567 Gyr | dynamo, 28 µT, core heat flow 11.6 TW; inner core from 4.0 Gyr | about 30 µT (the surface field runs 25 to 60 µT); inner core about 0.5 Gyr old (Bono et al. 2019: 565 Ma; older, to about 1 Ga, in other work: not reached) |
| Mercury | 4.567 Gyr | deep dynamo, 0.25 µT | about 0.2 µT (Anderson et al. 2011, doi:10.1126/science.1211001, p. 1861: dipole moment 195 ± 10 nT R_M³, offset about 0.2 R_M north; read) |
| Mars | 0.3 Gyr | dynamo, 14 µT | magnetised old crust |
| Mars | 4.567 Gyr | liquid core, no dynamo (ends at 0.55 Gyr) | none since 4.0 to 4.1 Ga (Lillis et al. 2013); a late field near 3.7 Ga from one crater (Mittelholz et al. 2020); both reached as abstracts only |
| Venus | 4.567 Gyr | liquid core, no dynamo (ends near 1.15 Gyr) | none |
| Moon | 4.567 Gyr | frozen core (at 3.55 Gyr), no dynamo | dynamo from 4.25 to at least 3.56 Ga, weak (about 5 µT) at 1 to 2.5 Ga (Weiss & Tikoo 2014; Tikoo et al. 2017; abstracts as quoted) |

The Moon's core is in fact still partly molten (Weber et al. 2011); the
model freezes it, which only matters for its state, not its dynamo.

## Volatile inventory at formation

`planet::inputs` gives a body the volatiles it starts with, in kg per kg
of planet, from its mass in rock and iron (`refractory`) and in ice
(`water`); the draw scatters each by a log-normal factor. They are the
`Volatiles` that `outgassed` (below) brings to the air. The rule is placed
here, with the part that uses it, rather than in
[planet-interiors.md](planet-interiors.md), which stops at the layers.

```text
co2 = 1e-4 refractory + 0.12 water        n2  = 2e-6 refractory + 1.4e-3 water
s   = 2.5e-4 refractory (1 - cmf)         ch4 = 7e-3 water,  nh3 = 7e-3 water
```

`co2` and `n2` are masses of CO₂ and N₂, `s` of sulphur (SO₂ is twice it),
and `cmf` is the core mass fraction (the core keeps its sulphur). In the
rewrite the ice's N₂ term counts only for ice that formed under 30 K
(`traps_nitrogen`, see *Titan's nitrogen*): around the Sun, beyond
21.3 au. Each is
multiplied by `10^N(0, 0.3)` (`VOLATILE_SPREAD_DEX`). The rewrite ports
the rule without its scatter to `science::volatiles::inventory`, on the
masses of rock and iron, of silicate and of ice (`VOLATILE_SPREAD` is the
spread the engine draws), called on the composition before any steam loss, and the
engine draws the scatter once per body from its own sub-seed
(`Drawn::volatiles`), so adding a draw never moves another. The
rewrite gives a stripped mini-Neptune the volatiles of its whole formed
composition, the condensed planet it kept, where legacy gave too few:
it multiplied the shares per kg of the formed planet, envelope included,
by the mass the escape left, taking the envelope off twice.

The rewrite's ice is the **surface water** (`planet.water.surface`, see
[Surface water](#surface-water)): a dry planet's ocean, the surface share
of its bulk water, and a water world's whole water. It carries the
volatiles delivered with the water that is at the surface. Legacy fed
the ice term its drawn water, which was the ocean; the rewrite draws the
bulk water, ten times Earth's ocean, and fed the whole of it to the ice
term until #663, which raised the reference Earth's N₂ from 0.51 to
1.28 bar and its CO₂ from 52 to 173 bar. Reading the surface water
restores legacy's meaning: a port-fidelity fix, not a retune, and none
of the constants above moved.

**Rock.** The bulk silicate Earth, in Wang, Lineweaver & Ireland 2018
(doi:10.1016/j.icarus.2017.08.024, Table 1, a concordance of three
compilations), holds C 109 (+77/-39) ppm, N 2.01 (+1.42/-0.71) ppm and S
235 ± 38 ppm; the silicate is 0.675 of Earth's rock and iron.

- Sulphur: 250 ppm of the silicate (`ROCK_S`) is McDonough 2003's value
  (Treatise on Geochemistry 2.15, not reached; consistent with the
  235 ± 38 above).
- Nitrogen: 2e-6 per kg of rock and iron lies between the 1.4e-6 of that
  concordance and the 4.5e-6 of Johnson & Goldblatt 2015
  (doi:10.1016/j.earscirev.2015.05.006, abstract: 27 ± 16e18 kg in the
  silicate Earth, about seven times the air's 4e18 kg), a calibration
  inside the spread.
- Carbon: 1e-4 kg of CO₂ per kg is a calibration on Venus, whose 92 bar
  (radius and pressure as in Johnson & Goldblatt, section 3.1; mass not
  reread) is about 1e-4 of the planet once 95 % is outgassed, and not on
  the bulk silicate Earth, which holds 2.7 times more as CO₂ (109 ppm of
  carbon) and up to 500 ppm of carbon in Marty 2012's abstract
  (doi:10.1016/j.epsl.2011.10.040): the carbon that never degasses is
  inside the outgassed share below.

**Ice.** Per kg of water, by mass: CO₂ 12 %, N₂ 0.14 %, CH₄ 0.7 %, NH₃ 0.7 %
(`ICE_CO2` 0.12, `ICE_N2` 1.4e-3, `ICE_CH4` 7e-3, `ICE_NH3` 7e-3), that is 4.9,
0.09, 0.79 and 0.74 % of the moles of water. The legacy had 5, 0.1, 1 and 1 %
by mass, which mixed mass and mole ratios: comet abundances are printed in
moles of water, and the legacy's 5 % of CO₂ by mass is 2 % of the moles,
under the whole printed range. They are converted with M / 18 here.

Read: Biver et al. 2022 (*Comets III*, arXiv:2207.04800, Table 3, updated
from Dello Russo et al. 2016 and Rubin et al. 2019) print, as moles of
water: CO₂ 4 to 30 %, "typically 5 to 30 %" (AKARI); CH₄ 0.15 to 3 %,
"typical abundances ~0.8 %"; NH₃ 0.1 to 5 %, "typical ~0.5 to 1 %"; N₂ only
for 67P. Comet 67P's bulk ices (Rubin et al. 2019, doi:10.1093/mnras/stz2086,
Table 2) are 4.7 ± 1.4, 0.34 ± 0.07, 0.67 ± 0.20 and 0.089 ± 0.024 %
(CO₂, CH₄, NH₃, N₂), 11.5, 0.30, 0.71 and 0.14 % by mass.

- `ICE_CH4` is `Source:` the typical 0.8 mole % (0.71 % by mass).
- `ICE_NH3` is `Source:` the middle of the typical 0.5 to 1 % (0.75 mole %,
  0.71 % by mass, also 67P's).
- `ICE_CO2` is a `Calibration:`: no median is printed; 5 mole %, the low end
  of the AKARI range and 67P's 4.7, is 12 % by mass.
- `ICE_N2` is a `Calibration:` on 67P alone (0.089 mole %, 0.14 % by mass),
  the only comet with a measured N₂.

Mumma & Charnley 2011 (doi:10.1146/annurev-astro-081309-130811) could not be
reached beyond its abstract; the review the values were checked in is Biver et
al. ([study 21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit)).

**Spread.** 0.3 dex is of the order of the uncertainty on Earth's own
carbon and nitrogen budgets (-0.44/+0.23 dex for each in Wang et al.; the
two nitrogen budgets differ by 0.5 dex); nothing read sets a spread
between planets. A calibration.

The audit behind this section and the next ones is
[study 21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit).

## Atmosphere

`planet::atmosphere` (called by `planet_model`) gives the
partial pressures at the surface, in bar, of N₂, CO₂, H₂O, CH₄, NH₃, SO₂,
O₂ and Ar (`Gases`, with `total()`; the argon since study 29, see
*Argon (study 29)* below), in six steps, called in this order:
`outgassed`, `atmosphere`, `non_thermal_loss`, `methane_photolysis`,
`cold_trap`, `photochemistry`. The rewrite has so far `outgassed`, the
shoreline of `atmosphere`, the SO₂ wash-out, `methane_photolysis`,
`cold_trap` and `photochemistry` (with the reducing air's CO₂ hold-back):
`system::planet::air::air` chains the outgassing (`science::outgassing`, the shares `MOBILE_SHARE`,
`LID_SHARE`, `RECYCLED_UNDER_PLATES`, `NITROGEN_RECYCLED_UNDER_PLATES`), the
cosmic shoreline (`science::escape::shoreline_kept`), the runaway's steam and
the wash-out (`science::outgassing::so2_kept`) on the pre-steam trial and
the post-steam body, then the photolysis, the reducing test, the cold trap
and the photochemistry. The non-thermal loss waits for #658;
the fallback and the climate are the solver's (2d-3a, see "The climate
solver of the rewrite" below), and the climate is solved again after
oxygenation in 2d-4 (#659). The carbonate
thermostat of the climate takes Earth's CO₂ away afterwards, and life
adds the oxygen.

**Outgassing** (`outgassed`). Each volatile the planet holds (the
`Volatiles` of its inputs, kg per kg of planet) times its mass, times the
share its regime brings to the surface over its life, weighs
`P = m g / (4π R²)` on the surface. The shares are a calibration within
the picture of Kite, Manga & Gaidos 2009 (doi:10.1088/0004-637X/700/2/1732,
lids outgas less than plates; the paper gives no share, only that a lid melts
early and "die[s] young" and that melting is "necessary but insufficient" for
degassing, so 0.95 and 0.2 are calibrations):

| Regime | Share outgassed | Calibrated on |
|---|---|---|
| Plates | 0.95, of which subduction returns 79.65 % of the nitrogen (N₂ and NH₃) and 63 % of the rest | Earth's N₂ once its NH₃ is photolysed (Goldblatt et al. 2009, doi:10.1038/ngeo692: about 0.5 times the air's nitrogen in the crust and more than 1.4 times in the mantle; abstract only). Legacy fitted 80 % on 0.78 bar, a share by volume; by weight Earth's N₂ is 0.746 bar (*Earth's N₂ target*), and the share is refitted on that weight. Earth's N₂ in the chain, the outgassed N₂ plus what its photolysed NH₃ makes, is proportional to the share kept: 0.804 bar at 80 %, so 81.45 % (study 23); once Earth's ocean carried no N₂ ice (study 28) 81.45 % gave 0.6799 bar, so 1 − 0.1855 × 0.746 / 0.6799, kept to four digits, 79.65 %, gives 0.7458 bar (0.483 bar before the photochemistry). The 63 % has no source: Kelemen & Manning 2015 (doi:10.1073/pnas.1507889112, as quoted) find that "relatively little carbon" is recycled into the mantle, where older work put about half there, so it is high |
| Episodic lid | 0.95 | Venus's 90 bar of CO₂ |
| Heat pipe | 0.95 | none (Io) |
| Stagnant lid | 0.2 | Mars, Titan |

**Under a deep ocean** (`released_under_ocean`), only a share of that
reaches the air: `1 / (1 + (P_floor / 3.7 GPa)^4.1)`, with `P_floor` the
weight of the water kept on the seafloor, `m_water g / (4π R²)`. The
weight of a deep ocean curtails the decompression melting that outgasses
the mantle (Kite & Ford 2018, ApJ 864, 75, doi:10.3847/1538-4357/aad6e0,
for waterworlds of 10 to 1000 times Earth's water, whose pCO₂ often lands
between 0.2 and 20 bar; checked), high-pressure ice at its base seals the
rock (Noack et al. 2016, Icarus 277, 215, doi:10.1016/j.icarus.2016.05.009;
the paper was not read, its companion abstract gives no single pressure),
and the ices' own volatiles stay dissolved in the ocean or locked in
clathrates. The 3.7 GPa and the power 4.1 are calibrations with no source:
Earth's ocean (0.03 GPa) changes nothing, Titan (1.0 GPa in the model),
Ganymede (1.1 GPa) and Callisto keep 98 % or more of their air (the power
4 left Ganymede 97.9 %), and a 1 M⊕ planet of 30 % water lets out about
1e-3 of it, a few bar of CO₂ before the thermostat. Legacy set the half
point at 3 GPa, for a seafloor of 17 GPa under that planet; the rewrite's
interior makes it smaller (1.143 R⊕ against 1.187) and its seafloor
deeper (20.2 GPa), so the half point was retuned to keep the target:
`P½ = 20.2 GPa / 999^(1/4.1) = 3.74 GPa`, rounded to 3.7 GPa (the planet
then lets out 9.6e-4, Ganymede keeps 99.3 %). A
runaway's steam weighs on the rock as its ocean did, so the share holds
in a runaway too; the steam itself is not held back, by this share or by
the regime's: it is the ocean, not a volatile of the mantle. The rewrite
ports the weight to `science::outgassing::column_pressure` and the share
to `released_under_ocean` (`SEAFLOOR_HALF_PRESSURE`, `SEAFLOOR_STEEPNESS`);
the regime's share is the constant the system picks, `MOBILE_SHARE` or
`LID_SHARE`.

A known departure: Kite & Ford (Fig. 5) find that a stagnant-lid planet
under a 3 GPa ocean does not melt at all, so 3 GPa is their full
shutoff, not a half point; the retuned half point, 3.7 GPa, sits further
past it than legacy's 3 GPa. A shutoff there (tried with
`1 - (P / 3 GPa)^5`) keeps the icy moons and Earth but leaves the
30 %-water planet no air at all, frozen at 257 K at 1 AU instead of an
ocean world: its air would have to come from the volatiles dissolved in
its ocean, which the model does not carry.

Sulphur comes up as SO₂, of which 5e-5 stays in the air (the rest ends in
the crust; Venus's SO₂ is lost to its surface rocks, Fegley & Prinn 1989,
doi:10.1038/337055a0, abstract: the share is not in it; a calibration on
Venus's
SO₂ below the clouds, 130 ± 50 ppmv, Marcq et al. 2008 as quoted by a later
paper, which the model's 115 ppm by volume sits inside, and washed out of a wet or
oxygen-rich air, see **Sulphur and
SO₂** below). Water is vapour only in a
runaway greenhouse, and then all that the runaway left
(`Water::kept_kg`); otherwise it is an ocean or ice, and the climate sets
its vapour. The escape of the cosmic shoreline (below) takes the same
share of the steam as of every gas, and the chain lowers
`Water::kept_kg` to what is left, so the water kept and the H₂O in the
air always agree.

**Escape** (`atmosphere`). The cosmic shoreline of Zahnle & Catling 2017
(ApJ 843, 122, doi:10.3847/1538-4357/aa7846) splits the bodies with and
without air by `I_XUV ∝ v_esc⁴` (the exponent is in their abstract; the fluence
is the cumulative XUV relative to Earth's, their eq. 26-27, read in full);
their line is "drawn by eye" on
their Fig. 1 and 2, and no equation, constant or crossing velocity appears in
the text, nor a width ("we do not know if the shoreline is broad or narrow",
section 10). Here the line is `I_line = (v_esc / 6.5 km/s)⁴`, in
units of Earth's XUV fluence (`xuv_fluence`), and the share of air kept
falls smoothly across it:

```text
kept = 1 / (1 + (s I / I_line)^6)
```

with `s = 2` for a body without a dynamo, whose upper air the stellar wind
strips (`UNMAGNETISED_ESCAPE`, a calibration with no source, tied to issue
#344). The normalisation 6.5 km/s, the steepness 6 and `s` are calibrations on
the Solar System bodies ([study
21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit)):
the line passes close to Mars, as in the paper, Titan keeps a bar, and `s` is
small enough that Venus, which has no dynamo, keeps its air. Every gas is kept
in the same share. `escape_velocity_km_s` gives `sqrt(2 G M / R)`. The
rewrite ports both to `science::escape::{escape_velocity, shoreline_kept}`,
the fluence in J/m² divided by `EARTH_XUV_FLUENCE`, with
`SHORELINE_VELOCITY`, `SHORELINE_STEEPNESS` and `UNMAGNETISED_ESCAPE`;
the exponent 4 (`SHORELINE_EXPONENT`) is the paper's. The rewrite's
`v_esc` reads the whole mass (`Body::whole_mass`: the bulk mass plus a
water beyond the laws, which the bulk leaves out), as legacy read the
trial's mass, and the runaway's steam loss reads the same mass; both read
them through `Body::exposed` (an `Exposed`: the radius and the whole mass),
the bulk radius before the climate and the steam envelope's once the boil
loop has one (PR 2d).

**Non-thermal loss** (`non_thermal_loss`, after the shoreline, on the whole
stock, frosts to come included). A thin air is an atmosphere only if the body
supplies it faster than it loses it. Inside a giant's magnetosphere (a moon of a
host of 10 M⊕ to 13 Jupiter masses, the heaviest giant drawn,
`HOST_PLASMA_MAX_MASS_EARTH`, `LossContext.host`; a heavier host is the star
whose tides a planet's model reads), each molecule at the top of the air meets a
loss per m² that does not depend on how much air there is:

```text
L = (J N_exo + f Φ) t
J = 5e-7 s⁻¹ / d²                  photo-ionisation at d AU
N_exo = 2.5e18 m⁻²                 the exobase column, 1/σ
Φ = 1e13 m⁻² s⁻¹ (M / M_J)³ (a / 26.3 R_host)^-5.5, at most 2.2e14
f = U (2x + U) / (x + U)²          share fast enough to escape
```

- *Photo-ionisation.* The Sun ionises a molecule about every 2e6 s at
  1 AU (N₂ 3.5e-7, CO₂ 6e-7 s⁻¹ under the quiet Sun: Huebner, Keady &
  Lyon 1992, *Astrophys. Space Sci.* 195, 1; from memory, not checked;
  one order for every gas, `PHOTO_IONISATION_AT_1_AU_S`), scaled as
  `1/d²` with the distance of the body (of its planet) to its star. The
  ions made above the exobase, where a molecule leaves without a
  collision, are swept away by the giant's corotating field: Carlson
  1999 (*Science* 283, 820, doi:10.1126/science.283.5403.820) finds that
  photo-ionisation and Jupiter's field would remove Callisto's CO₂ in a
  few years (about four, from memory), so that it needs a steady supply.
  The exobase column is `1/σ` with the hard-sphere cross-section of N₂,
  O₂ and CO₂, about 4e-19 m² (`EXOBASE_COLUMN_M2`).
- *Plasma sputtering.* The giant's plasma knocks molecules off the moon's
  surface and air (`plasma_flux`). At Callisto's place (Jupiter's mass,
  26.3 Jupiter radii) it is 1e13 m⁻² s⁻¹ (`PLASMA_LOSS_AT_CALLISTO`).
  **Calibration** on Callisto's CO₂: the column the shoreline leaves it,
  about 0.05 bar (5.5e28 m⁻²) and up to 0.2 bar on the archetype's
  richest draws, must be gone within its age, since Callisto has only an
  exosphere. That is about fifty times the water ice Jupiter's ions
  sputter there: Callisto receives about 2e8 keV cm⁻² s⁻¹ of energetic
  ions (Cooper et al. 2001, *Icarus* 149, 133, doi:10.1006/icar.2000.6498,
  read through a secondary table, not checked), at about 0.1 molecule of
  ice per keV (Europa's 2e27 H₂O s⁻¹ over its 6e10 keV cm⁻² s⁻¹; Johnson
  et al. 2004, "Radiation effects on the surfaces of the Galilean
  satellites", in *Jupiter: The Planet, Satellites and Magnetosphere*,
  ch. 20; from memory, not checked), or 2e11 m⁻² s⁻¹. A CO₂ or N₂
  frost, bound less than water ice, sputters several times more per ion;
  the rest of the factor is the calibration's uncertainty. The flux falls
  with the distance in the giant's radii as `r^-5.5`, the fit through
  Cooper et al.'s energy fluxes at Europa (9.4 R_J, 6e10 keV cm⁻² s⁻¹),
  Ganymede (15.0, 5e9) and Callisto (26.3, 2e8)
  (`PLASMA_DISTANCE_EXPONENT`), and grows with the giant's mass as `M³`
  (`PLASMA_MASS_EXPONENT`, **calibration**, no source: Jupiter's dipole
  is about 20 times Saturn's for 3.3 times its mass; the cube keeps Titan
  losing at most 7 % of its N₂ over its age and Triton at most 4 % while
  Callisto loses all its CO₂; with a square, Triton, round a giant 18.5
  times lighter than Jupiter, would lose up to about 80 % of its N₂ and
  Titan up to 23 %). The giant's radius is `giants::giant_radius` of its mass and
  equilibrium temperature. The flux is capped at Io's loss, the largest
  known, about one tonne per second over its 4.2e7 km², 2.2e14 SO₂
  m⁻² s⁻¹ (Thomas et al. 2004, *Jupiter*, ch. 23; from memory, not
  checked; `PLASMA_LOSS_MAX`): the power law would give Io about 170 times
  that, and the moons a few radii from a giant of ten Jupiter masses
  millions of times, while the plasma a giant's moons load is limited by
  what they supply and what its rotation carries.
- *Escape share.* A sputtered molecule leaves only if it is faster than
  the escape velocity. Sputtered energies follow `E / (E + U)³`
  (Sigmund–Thompson) with `U` the binding energy of water ice, 0.45 eV
  (`ICE_BINDING_EV`; Johnson 1990, *Energetic Charged-Particle
  Interactions with Atmospheres and Surfaces*, from memory, not
  checked); the share above the escape energy `x = m v_esc² / 2` is
  `f` above: 0.4 for CO₂ on Callisto, 0.5 for N₂ on Titan, 0.05 for N₂
  on a moon of the Earth's mass, which holds what its plasma knocks up.
  Photo-ions are swept whatever the gravity.

The air loses its gases in proportion to their share of its column
`Σ p / (m g)`: an air whose column is less than its share-weighted loss
over the age `t` is exhausted. It keeps only the exosphere its supply
feeds, `EXOSPHERE_BAR` = 3e-12 bar, which its gases share in proportion
to their supply, none keeping more than it had (an air already thinner
stays as it is). That is the order of one exobase column, 2.5e18 m⁻² of
CO₂ under a Galilean moon's 1.2 to 1.4 m/s², 2 to 3e-12 bar, where the
measured exospheres sit: Callisto's CO₂ at 7.5e-12 bar (Carlson 1999; a
column of 8e14 cm⁻² at about 150 K) and the O₂ of Europa and Ganymede
near 1e-12 bar (Hall et al. 1995 and 1998, from memory, not checked).
The exosphere is an order of magnitude, not a chemistry: the radiolytic
O₂ of an icy surface is not modelled. An air that outlasts its loss
keeps all of it, the loss being then a share of a large column. Since
the step comes before the cold trap, a frost of an exhausted gas is gone
too: Ganymede and Callisto no longer have the CO₂ frost that brightened
them (`surface::frost` reads what the cold trap froze out of what is
left), and the carbon cycle's inventory is what is left of their CO₂.

A moon of a giant meets its giant's plasma in two places. The
shoreline already takes it as unshielded (*Moons* below: its own dynamo
ignored, the star's XUV counted twice as for a body without a field),
a bulk escape driven by the star's light that does not see the plasma's
own energy, the moon's distance or the giant's mass. This step adds
that energy: the plasma's sputtering and its sweeping of photo-ions.
They share a cause, not a process; whatever they overlap is absorbed by
the calibration of `PLASMA_LOSS_AT_CALLISTO`, made on the air the
unshielded shoreline leaves, and the thick airs to keep (Titan, Triton)
do not change.

Outside a giant's magnetosphere the step takes nothing: the stellar
wind's pick-up of photo-ions is already in the shoreline, which Zahnle &
Catling fitted on the planets (Mars, Mercury) that undergo it. Counting
it again would strip Mars, whose 0.006 bar is gone in a few hundred
million years at its measured ion loss; its reservoirs (polar caps,
regolith), which the model does not carry, resupply it. The pressure
tree of the card names the escape (both losses) and adds the note
`exosphere` under a gas the step exhausted. The card writes an air
the step collapsed (at most twice `EXOSPHERE_BAR`, mirrored in
`web/src/lib/format.ts`) as an exosphere (`exosphere, 3×10⁻¹² bar · CO₂
100 %`); a thin air under 1e-3 bar keeps its name and is written in
scientific notation (Triton's N₂ `1.5×10⁻⁵ bar`, Io's SO₂ `3×10⁻⁹ bar`,
a real air over its frost). Io's SO₂ node of
the pressure tree names the volcanic frost (`volcanic_frost`) the air is
the vapour of, in place of an outgassing and an escape it does not have.

Over seeds 1 to 40 of `archetype_census` (loss over the column):
Ganymede's CO₂ 5 to 72, Callisto's 2.8 to 95, so both are exhausted;
Titan's N₂ 0.02 to 0.07 and Triton's 0.001 to 0.04, so both keep their
air, as do every planet and every archetype but the four Galilean moons
(their rows alone change). Through `moon_census 300`, eight drawn icy
moons of giants with thin airs (2e-8 to 1.4e-3 bar) become exospheres;
no class changes, and the thick airs of drawn moons (up to 36 bar round
giants of 4 to 12 Jupiter masses) stay.

**Volcanic SO₂** (`volcanic_so2_supply`, `volcanic_so2_frost_bar`,
`volcanic_so2_vapour`; issue #525). A heat-pipe body erupts the heat its
tides give it as magma, and the magma degasses its sulphur as SO₂,
which falls back as frost and breathes a thin air over it. Io's air is
that: a dayside column of 1e16 to 2e17 cm⁻² of SO₂ (about 1 to 10
nbar) in sublimation equilibrium with SO₂ frost at 106 to 117 K on the
equator (Giles et al. 2024, arXiv:2405.19253, 22 years of IRTF/TEXES
spectra; checked), with N₂ and CO₂ far under it.

```text
supply = 2 s_sil · P_tidal / E_magma           kg/s of SO₂
frost  = (supply − loss) · t_res · g / (4π R² · 1e5)   bar
air    = P_SO₂(2^(1/4) · T_eq)                 at most the frost
```

- *Supply.* In a heat pipe the magma carries the heat up (O'Reilly &
  Davies 1981, *Geophys. Res. Lett.* 8, 313; from memory, not checked):
  the magma erupted is the tidal heat over the heat a kilogram of magma
  carries, its latent heat (about 4e5 J/kg) and its heat capacity
  (about 1200 J/(kg·K)) over the 1400 K from an eruption at about
  1600 K (Io's ultramafic lavas, McEwen et al. 1998, *Science* 281, 87;
  from memory, not checked) to the cold surface, `MAGMA_HEAT_J_KG` =
  2.1e6 J/kg: about 5e7 kg/s for Io's 1e14 W. Its sulphur is the
  silicate's (`s_sil`, the 250 ppm of `ROCK_S` times the body's
  scatter), all degassed as SO₂ (64 for 32) into the vent's vacuum.
  Only in a heat pipe: the other regimes keep the outgassed SO₂ of
  `outgassed` (Venus unchanged). And only the tidal heat, a scope limit:
  in a heat pipe all of the internal heat leaves through the vents, but
  a heat pipe driven by its own young heat (a hot young planet) gets no
  volcanic SO₂ from this rule. The model's Io gives about 1.7e4 kg/s.
- *Loss.* The non-thermal loss takes SO₂ at the rate per m² of the
  step above, photo-ions and escaping sputtered molecules, over the
  whole surface (`so2_loss_kg_s`): about 300 kg/s on Io, the order of the
  tonne per second its torus is fed. A supply under it keeps no frost
  and no air of its own; the frost starts from nothing at that line.
- *Frost.* What the loss leaves accumulates as frost over
  `SO2_FROST_RESIDENCE_S`, 150 000 years. That is not a measured burial
  time (the frost sublimes and condenses again each day and so stays on
  top of the lava and plume deposits that bury Io's surface at about a
  centimetre a year, from memory, not checked): with the frost layer of
  0.02 bar (itself calibrated on Io's old CO₂ frost), it is what sets
  the share of the disc the frost covers. **Calibration** on Io, no
  source: SO₂ frost covers most of Io (Douté et al. 2001, *Icarus* 149,
  107; from memory, not checked), and 150 kyr covers at least 80 % of
  the disc on every draw of `io-like`, whose sulphur scatters by a
  factor of 20 (7.9e3 to 1.5e5 kg/s, 0.017 to 0.33 bar of frost); a
  shorter one leaves its low-sulphur draws half bare, dark and warm
  enough to breathe more than 1e-8 bar. The climate
  reads it as a frost (`surface::frost`, where the surface keeps it under
  SO₂'s sublimation pressure), with its own albedo, `SO2_FROST_ALBEDO`
  = 0.63, **calibration** on Io's Bond albedo (Simonelli & Veverka
  1988, from memory, not checked) for a disc nearly all frost (Giles et
  al. 2024 fit Io's frost with a Bond albedo of 0.565, their Fig. 3: issue
  #669 follows it up): darker
  than the N₂, CH₄ and CO₂ frosts' 0.75, Io's frost being mixed with its
  sulphur and silicate deposits. Under 0.75 the frost's dayside would
  lie near 103 K and breathe 4e-11 bar.
- *Vapour curve.* `SO2_FROST`: triple point 197.64 K and 0.0167 bar
  (Giauque & Stephenson 1938, *J. Am. Chem. Soc.* 60, 1389, through the
  NIST Chemistry WebBook; checked), and the sublimation curve
  `P = 1.516e8 exp(−4510 K / T)` bar of Wagman 1979 (NBS Chemical
  Thermodynamics Data Center; quoted by Giles et al. 2024, checked
  there), so 37.5 kJ/mol from the triple point: 1.3e-9 bar at 115 K.
  Fray & Schmitt 2009 (*Planet. Space Sci.* 57, 2053,
  doi:10.1016/j.pss.2009.09.011; closed, not read) fit SO₂ too; Lisse
  et al. 2021 (arXiv:2009.02277, appendix Table 2) reproduce the fit
  `SO2-1`, `ln(p / 1 bar) = Σ Aᵢ / Tⁱ` with 15.60, −3508, −9.401e4,
  4.152e6, −6.946e7 (the copy prints `A₁` as "−3.5.08x10+3"; −3508 is
  the reading that puts the fit's triple-point pressure at 0.0172 bar,
  within 3 % of NIST's 0.0167, where −3580 would give 0.0119). That fit
  gives 2.8e-9 bar at 115 K, twice Wagman's curve, and 35 to 37 kJ/mol
  over 110 to 190 K (derived). The rewrite keeps Wagman's curve, the one
  Io's atmosphere is read with (study 24, part 2): the factor 2 moves
  Io's frost temperature by about 2 K, under the spread of its observed
  106 to 117 K. The range over which Wagman fitted the curve was not
  reached (an NBS report).
- *Air.* The frost's vapour at the mean temperature of the dayside,
  `2^(1/4)` times the equilibrium temperature under the body's Bond
  albedo (`DAYSIDE_FACTOR`: the starlight over the dayside only; a heat
  pipe's internal heat leaves through its vents and lava, not through
  the cold lid under the frost), at most the frost. The frost holds the
  air at that vapour, resupplying it within hours, so no sink of the air
  applies to it: neither the non-thermal loss, whose rate is already
  taken out of the supply, nor the washing out, whose dry rule scales a
  finite supply by its own pressure over 10 bar (it would cut a nanobar
  air by ten orders of magnitude). The stored air's SO₂ is the vapour,
  or the outgassed SO₂ the washing out leaves when that is more. Under
  `TRACE_SO2_BAR`, 1e-7 bar (ten thousand times under the sulphuric
  deck's 1e-3 bar, a thousandth of the 1e-4 bar that shields a surface,
  and SO₂ has no greenhouse in the climate), the vapour moves neither the
  albedo nor the temperature and joins the air after the climate. Above
  it, on a warmer heat pipe, it leaves the frost and goes into the air
  before the climate, which is solved again once with it (the vapour of
  that second climate differs by 0.4 % at most, for Io round a Jupiter
  at 2.5 AU). The vapour is at most SO₂'s triple-point pressure, 0.0167
  bar, above the deck's 1e-3 bar, but the deck forms only over 273 K
  and the frost only under 197.64 K: no deck forms over a frost, in the
  albedo or in the look. A dayside or a mean surface (warmed by a
  greenhouse) at or above SO₂'s triple point holds no frost: such a body
  keeps no volcanic frost or air beyond the SO₂ its outgassing gives it
  (`outgassed`, which counts the same sulphur over its age). A scope
  limit: its air gets that bulk SO₂, not the tidal supply against a
  chemical lifetime, so a lava world's tides add no SO₂. Io round a
  Jupiter breathes 9.3e-10 bar at 5.2 AU, 1.2e-7 at 4 AU and 1.7e-4
  off 0.037 bar of frost at 2.5 AU, its air growing steadily as it nears
  the star; at 1 AU, or round an Earth-mass host at 1 AU with no plasma,
  its dayside is above the triple point and it keeps only its outgassed
  trace.

Over seeds 1 to 40 of `archetype_census`, `io-like` breathes 2.4e-10 to
5.5e-9 bar of SO₂ (median 8.9e-10), against 1.8e-14 to 1.1e-12 of N₂
and 1.9e-12 to 3.0e-12 of CO₂, its Bond albedo 0.54 to 0.63 (median
0.63; 0.11 before) and its mean surface temperature 101 to 113 K
(median 107; 120 to 130 before, under its bare rock). Every other
archetype's rows are unchanged; `venus-like` keeps 0.0054 to 0.072 bar
of SO₂.

**Methane photolysis** (`methane_photolysis`, before the cold trap, on
the whole stock). The star's ultraviolet destroys methane at a rate set
by its light, not by the amount of methane: any column worth counting
absorbs all the Lyman α (photon-limited). The rate, in molecules per m²
and second over the sphere, is

```text
D = Σ over the stars s of  Y · F_s r_s(t) / 4 / E_γ
```

with `F_s` the present bolometric flux of star `s` at the planet
(`S₀ I` with `S₀ = 1361 W/m²` and `I` the insolation, for one Sun-like
star, as `legacy-final` wrote it), `E_γ` the energy of a Lyman α
photon (`h c / λ` = 1.634e-18 J: the H I multiplet at 1215.6701 Å in
vacuum, NIST Atomic Spectra Database, and the exact Planck constant of
CODATA, 6.626 070 15e-34 J s) and `r_s(t)` the Lyman α share of star
`s`'s light, from its own history: Ribas et al. 2005 (doi:10.1086/427977, their eq. 2,
checked) fit `F = 19.2 τ^-0.72` erg s⁻¹ cm⁻² at 1 AU, 7.40e-5 of the
solar constant at the end of saturation (0.1 Gyr) and 3.9e11 photons
cm⁻² s⁻¹ today. The line saturates with the XUV (`saturation_gyr`, the
same anchors) and decays as `t^-0.72` after. A methane molecule weighs
`CH4_MOLAR_MASS / AVOGADRO` = 2.664e-26 kg. `Y` is the number of methane
molecules destroyed per Lyman α photon, fitted on Titan (below).
Yung, Allen & Pinto 1984 (ApJS 55, 465, doi:10.1086/190963; full text
read, the ADS scan) give Titan's present methane loss in their Table 5B:
"net loss of parent molecules from atmosphere: CH₄ 1.5 × 10¹⁰"
molecules cm⁻² s⁻¹, averaged over the globe. Their abstract's "5.6 ×
10⁴ g cm⁻²", "the equivalent of 8 bars", is that rate held constant: "If
the loss rates given in Table 5B were constant over the age of the solar
system (4.5 billion yr), we would estimate accumulated losses of 5.6 ×
10⁴ … g cm⁻²" (§ VI d, p. 500). `Y` is over one because Lyman α stands
for all the ultraviolet that destroys methane: Lyman α's own photolysis
(their reaction R5) is 2.9e9 of the 1.5e10; the rest is mostly the
dissociation photosensitised by acetylene and diacetylene, which absorb
at 1600 to 2300 Å, where the Sun sends 20 to 200 times more photons
(their Table 2A and § II b: "C₂H₂ and C₄H₂ act as catalysts for
collecting the solar energy needed for breaking CH₄ apart"; § VI e:
"About 96 % of CH₄ dissociation (mostly by photosensitized processes)
leads to production of higher hydrocarbons"). At that rate Titan's air
would lose its methane in a few tens of millions of years, as Tobie,
Lunine & Sotin 2006 (Nature 440, 61, doi:10.1038/nature04497; abstract
checked) state.

**Calibration of `Y`** (`CH4_PER_LYMAN_ALPHA_PHOTON`, 14.1; study 23 of
the [Wiki](https://github.com/cyboooooorg/map-generator/wiki/23-air-chemistry-audit),
part 1). Fitted on Yung et al.'s present rate with the rewrite's Lyman α
history (a share of the ZAMS light times the history's `scale`, #622):
for the 1 M☉, [Fe/H] 0 star at 4.6 Gyr, `scale` = 0.7081 and the
present Lyman α share is 4.690e-6 of the present light, 3.91e11 photons
cm⁻² s⁻¹ at 1 AU and 1.066e9 over Titan's sphere at Saturn's 9.57 AU
(1432.0e6 km, NASA Saturn fact sheet), so `Y = 1.5e10 / 1.066e9 =
14.07`, 14.1 to the two digits of the rate (legacy's Titan cases put it
at 9.58 AU, which gives 14.10). Legacy's 13.9 came from the abstract's
5.6e4 g cm⁻² over 4.5 Gyr (1.48e10) and legacy's ratio, 0.25 % higher
than the rewrite's; the age 4.57 Gyr or Yung et al.'s own distance
(their attenuation 0.011, 9.53 AU) give 14.0, within the two digits of
the rate. Fitting the 5.6e4 g cm⁻² over the whole history instead would
give 5.1; it is not done, since the paper computed that loss from
today's rates. Searched and not found: a measured or compiled yield of
methane destroyed per Lyman α photon for any other body or spectrum;
Titan's methane destruction against the Sun's age (Yung et al. only
extrapolate today's rate). Departure: the destruction is scaled by
Lyman α's decay, whereas most of it is driven by 1600 to 2300 Å light
from the photosphere, which fades far less with age; the law overstates
the destruction round a young star.

The supply is the methane outgassed (after escape), spread evenly over
the system's age: `S = CH₄ / age`. While the destruction outruns it,
nothing builds up; from the age `t*` at which the falling destruction
meets the supply, methane gathers in the air and on the ground:

```text
CH₄ left = ∫ from t* to age of (S − D(t)) dt
```

through the band integral of `xuv.rs`; the onset `t*` is found by
bisection on `[0, age]` (the destruction is constant while the star is
saturated and only falls after). The rewrite's law
(`science::photochemistry`, `methane_left`) sums the destruction of
every star the planet sees, each with its present flux at the planet and
its own Lyman α history; `legacy-final` had one star and an analytic
onset. This is Titan's
resupply: Tobie et al. explain its methane by outgassing from its
interior, in episodes where the model spreads it evenly. In the
rewrite's chain (the reference Titan round the Sun at 4.567 Gyr), Titan
outgasses 14.4 bar of methane, keeps 14.36 bar past the shoreline, and
the photolysis leaves 2.11 bar of it (the supply overtakes the
destruction at about 2.06 Gyr); the climate, which condenses it onto
the ground, comes in 2d-3. Callisto gets 3.4 times Titan's light per
unit area and keeps 0.30 bar past the shoreline: the destruction
outruns it at every age and it keeps none. `legacy-final`, with
`Y` = 13.9 and its own history, had Titan outgas 20.6 bar and keep
6.3 bar on its ground (onset about 1.2 Gyr), of which its climate left
0.11 bar in the air, the vapour of its seas (see **Methane** under the
climate below), and Ganymede and Callisto outgas 0.15 and 0.47 bar. Destroyed
methane is gone: its hydrogen escapes and its carbon ends in heavier
hydrocarbons on the ground, which the model does not carry. Two
departures: the outgassing is spread evenly over the age, and the
larger Lyman α share of an M dwarf's light is not modelled (both
understate the destruction round red dwarfs, if anything).

**How the rewrite computes the photolysis** (`science::photochemistry::methane_left`;
`system::planet::air`). The step reads the methane kept by the shoreline,
the surface gravity and, per star, its flux at the planet and its Lyman α
history, and records `planet.air.photolysed.ch4`. Its cause tree is the
chain's: `planet.air.outgassed.ch4` (outgassing), then the shoreline's
kept node (`planet.air.kept.ch4`, the SO₂ wash-out aside, which reads
the kept air of another gas), then the photolysed node, which cites the
kept methane, the gravity, each source's flux and mass, and the trial's
metallicity and age. Nothing recomputes the destroyed amount to explain
it. Where it departs from legacy: `Y` is fitted again, 14.1 for legacy's
13.9, on the history of #622 (a share of the ZAMS light times the
history's scale); the destruction is summed over every star the planet
sees, where legacy had one; the onset is a 60-step bisection, not an
analytic form; and no non-thermal loss precedes the photolysis (#658).

**Reducing air** (`reducing`, `without_co2_if_reducing`, issue #557).
A body whose methane survives the ultraviolet (the air the photolysis
leaves holds some) under an equilibrium temperature of 150 K (the line
of *Photochemistry* below, a calibration) has a reducing air. The chain
first solves it without CO₂; it keeps that air if the climate then leaves a Titan's
dense, hazy air (`titan_like_air`: methane still in it, over methane's
triple-point pressure of 0.1169 bar, the line of the hydrocarbon-world
class), and is solved again with its CO₂ otherwise. Titan's interior holds CO₂,
likely its most abundant gas (up to 3 % of its mass), but stores it,
mostly dissolved in its ocean, and what reaches the air is a moderate
outgassing of methane (Tobie, Gautier & Hersant 2012, ApJ 752, 125,
doi:10.1088/0004-637X/752/2/125; abstract checked). The CO₂ of its air,
about 15 ppb in the stratosphere (Cassini CIRS: de Kok et al. 2007,
Coustenis et al. 2010, doi:10.1016/j.icarus.2009.11.027), is made there
from oxygen falling in from outside, most likely from Enceladus (Hörst,
Vuitton & Yelle 2008, doi:10.1029/2008JE003135; checked). Before, the
model's Titan froze out 40 to 150 bar of outgassed CO₂, a frost over its
whole disc under its haze, and census seed 387's planet d, a dim Titan
at about 74 K, was bright enough under its CO₂ frost to freeze its
nitrogen out and end an ice world; it now keeps its seas at 95 K. The
pressure tree shows the missing CO₂ with the note `reducing_air`.
Departures: the storage in an ocean would hold for any ice-covered
ocean, but only Titan's case is checked, so the CO₂ of bodies whose air
is not reducing is kept (Europa's surface shows some); Titan's N₂ comes
from the ammonia its interior decomposes (*Titan's nitrogen* below),
though ammonia dissolves in an ocean as well.
Triton's trace of methane makes its air reducing too, but its thin
nitrogen air is no Titan's: it keeps its CO₂ frost, as Triton's surface
shows CO₂ ice (Cruikshank et al. 1993, Science 261, 742; Quirico et al.
1999, Icarus 139, 159). The model's triton-likes freeze 6 to 32 bar of
it under their nitrogen frost, far more than a surface veneer.

**How the rewrite computes the reducing air** (`science::photochemistry::reducing`;
`system::planet::air`). The air is solved in one pass: the step reads the
methane the photolysis leaves and the combined equilibrium temperature
(`StellarSources::equilibrium_k`, as the cold trap below), records `planet.air.reducing`, and the CO₂ is outgassed (or not) after
that node, so a reducing air's `planet.air.outgassed.co2` cites the
reducing node only. Legacy solved the air without CO₂, then solved it
again with CO₂ when the climate did not leave a Titan's air. The
rewrite does the same in `system::planet::settled`: the test on the
climate's result (the fallback to the air with CO₂, `titan_like_air`)
exists, but `settled` is not wired into generation until 2d-4 (#659), so
the planets the engine produces still keep a reducing air without CO₂
until then. The frozen CO₂ is
booked as legacy does, a frost of what the cold trap removes, which
gives Callisto 5 bar of it (#590).

**Cold trap** (`cold_trap`). A gas whose triple point lies above the
body's equilibrium temperature frosts onto the ground until its partial
pressure falls to its sublimation pressure, by Clausius–Clapeyron from
the triple point (CO₂ 216.6 K, 5.185 bar, 26.1 kJ/mol; NH₃ 195.4 K,
0.0606 bar, 29 kJ/mol; CH₄ 90.7 K, 0.117 bar, 9.1 kJ/mol; values from
the NIST WebBook from memory, not checked). This is the legacy law; the
rewrite replaces CO₂'s and methane's with `science::volatiles`, the
equations of state's vapour curves and CO₂'s Span & Wagner sublimation
equation (see [radiative-transfer.md](radiative-transfer.md#carbon-dioxide-and-methane-saturation)),
and NH₃'s with values read (study 23, part 3): the triple point at
195.49 K (Gao et al. 2023, J. Phys. Chem. Ref. Data 52,
doi:10.1063/5.0128269, the reference equation of state; abstract) and
0.06060 bar (Fonseca & Lobo 1989, through the NIST Chemistry WebBook),
and below it Clausius–Clapeyron at the sublimation enthalpy of 31.2
kJ/mol measured from 177 to 195 K (Overstreet & Giauque 1937, through
the NIST WebBook). That curve follows the polynomial fit of Fray &
Schmitt 2009 (NH3-1, as Lisse et al. 2021 reproduce it) within 2 %
from 150 K to the triple point, and meets their data point of 1.27e-3
bar at 162.39 K (Karwat 1924, through Fray & Schmitt 2009, as Blakley et al. 2024, *Planet. Space Sci.*,
doi:10.1016/j.pss.2024.105863, quote it) within 5 %; at 80 to 120 K
Blakley et al. measure pressures 1.5 times below Fray & Schmitt's
extrapolation, and the constant enthalpy lies between the two.
Legacy's 29 kJ/mol overstated the pressure four times at 100 K. N₂'s curve is measured
(issue #529): triple point 63.14 K and 0.1253 bar (Angus, de Reuck et
al. 1979 and Jacobsen, Stewart et al. 1986, through the NIST WebBook,
checked), and below it the polynomial fits `ln p = Σ Aᵢ / Tⁱ` of Fray &
Schmitt 2009 (*Planet. Space Sci.* 57, 2053,
doi:10.1016/j.pss.2009.09.011), as Lisse et al. 2021 (*Icarus* 356,
114072, doi:10.1016/j.icarus.2020.114072, supplementary Table 2)
reproduce them (`NITROGEN_ALPHA_FIT`, `NITROGEN_BETA_FIT`): β-N₂ from the triple
point down to the α–β transition at 35.61 K (`NITROGEN_ALPHA_BETA`),
α-N₂ below. Study 24 of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/24-surface-cover-audit)
(part 1) read the coefficients again in the arXiv copy of Lisse et al.
(arXiv:2009.02277, appendix Table 2, "from Fray & Schmitt 2009"; B.
Schmitt is a co-author): `N2-1` 12.40, −807.4, −3926, 62 970,
−463 300, 1 325 000 and `N2-2` 8.514, −458.4, −19 870, 480 000,
−4 524 000 (`A₀` to `A₅`, K to the power `i`). Fray & Schmitt's own
paper is closed (the HAL record links an ISTEX copy behind a login), so
the validity range of each fit and which phase each one stands for were
not read: the phase is read off the values (the `N2-2` fit gives 0.1215
bar at the triple point, within 3 % of NIST's 0.1253, the `N2-1` fit
0.317 bar; their slopes give 6.7 to 7.0 kJ/mol over 36 to 60 K for
`N2-2` and 7.6 kJ/mol over 20 to 35 K for `N2-1`, derived), and the
two raw fits are 2.8 % apart at 35.61 K (derived). The transition, 35.61 K, is
Scott 1976 (*Phys. Rep.* 27, 89, doi:10.1016/0370-1573(76)90032-6;
closed, not read), as Tegler et al. 2019 (arXiv:1905.13250, read) quote
it: "In pure N2, this α β transition occurs at 35.61 K (Scott 1976)".
The thermometric measurement of Lipiński et al. 2007 (*Int. J.
Thermophys.* 28, 1904, doi:10.1007/s10765-007-0267-y; closed, its
35.620 K seen only in a search summary) agrees to 0.01 K. Each fit is
pinned to the curve above it (the β fit scaled by about 3.1 % to
NIST's triple point, 0.1253 / 0.1215 bar; the α fit pinned to that
scaled β curve at the transition, which moves it by about 6.1 %,
derived), and under 10 K, where the α polynomial turns
over, the curve goes on with its slope there. It gives 1.4e-5 bar at
37.2 K: Triton's N₂ ice, at 37 to 38 K, is β-N₂. This freezes Titan's
NH₃ (its CO₂ is not outgassed: *Reducing air* above). Above the triple point the
gas stays; whether it rains out depends on the surface temperature,
which the climate sets. Water is left to the climate, and so are methane
(issue #443) and nitrogen (issue #464): their air moves the surface
temperature they condense at, so the climate condenses them at the
temperature it solves for (`condense`; nitrogen over its own ice, with
activity 1, its liquid above 63.14 K up to its critical point,
126.192 K, Span et al. 2000; see *The nitrogen vapour curves* below).
The CO₂ the cold trap froze out stays on the ground as a
frost wherever the surface is cold enough to keep it (below CO₂'s
triple point, beyond what the air can take back at the surface's
sublimation pressure), and the frosts of N₂, CH₄ and CO₂ brighten the
body's Bond albedo (`albedo::cover`: 0.75, covering the share
`frost / 0.02 bar` of the disc), as does a heat pipe's volcanic SO₂
frost (0.63, see *Volcanic SO₂*).

**How the rewrite computes the cold trap** (`science::volatiles`,
`co2_frost_pressure`, `ammonia_frost_pressure`; `system::planet::air`).
The CO₂ and the NH₃ of the kept air are each held to their frost pressure at one combined
equilibrium temperature, the fourth root of the sum of every star's
`T_eq⁴` (`StellarSources::equilibrium_k`; one star's own with a single
star), which the reducing test and the methane line read too. The air
keeps weights, so each keeps the weight whose partial pressure, over the
air as it stands (its photolysed methane, its washed SO₂), is its frost
pressure (`science::surface::cold_trapped`, the closed form of *Weights
and partial pressures*, the two in rounds since each raises the other's
share; issue #749: the trap capped the weight itself before, which left
a trace CO₂ in an N₂ air at 28/44 of its frost pressure, Callisto's
4.88e-6 bar becoming 7.66e-6 bar by weight). The step records
`planet.air.trapped.co2` and `planet.air.trapped.nh3`, each citing the
kept gas, the rest of the air and the equilibria, and
`planet.air.frozen-co2`, citing the kept and the trapped CO₂. What
the trap leaves is what the photochemistry reads. Where it departs from
legacy: the frost curves of CO₂ and ammonia are the sourced ones above;
`condense` (the climate's condensation of methane and nitrogen at the
surface temperature, `science::surface::condense`) belongs to the climate
solver of 2d-3a, so the air of this section keeps them whole.

**Photochemistry** (`photochemistry`, after the cold trap, on what is
still gas). The star's ultraviolet photolyses NH₃ into N₂ and hydrogen,
which escapes: `2 NH₃ → N₂ + 3 H₂`, the nitrogen conserved. Atreya,
Donahue & Kuhn 1978 (*Science* 201, 611,
doi:10.1126/science.201.4356.611; abstract read) find that "if NH₃
outgassed from Titan it should have been converted to a dense N₂
atmosphere during the lifetime of the satellite", about 20 bar; Yung et
al. 1984 (§ VI d) find "no compelling evidence" for the warm early
climate that conversion needs, so Titan's N₂ from its NH₃ is a
departure the law keeps. The chain's partial pressures are weight
pressures (`column_pressure`: mass times gravity over area), so the N₂
gained is the NH₃ lost times the mass ratio `N2_PER_NH3` =
`N2_MOLAR_MASS / (2 · NH3_MOLAR_MASS)` = 0.8224, a definition (legacy
wrote 28/34, 0.13 % higher). CH₄ survives only where the equilibrium
temperature is under 150 K: there water is frozen out so deep that none
reaches the upper air (the sublimation pressure of ice is 6.1e-11 bar
at 150 K, by Feistel & Wagner 2007 as Lisse et al. 2021 reproduce it);
above it, OH from water's photolysis oxidises the methane, which is set
to zero (the CO₂ it would make is not added). Life's methane is added
later. Methane's photolysis by the ultraviolet comes before the cold
trap (above).

**How the rewrite computes the photochemistry**
(`science::photochemistry::nitrogen_from_ammonia`, `methane_survives`;
`system::planet::air`). After the trap, the ammonia left becomes N₂ at
`N2_PER_NH3` and methane is kept only under `METHANE_SURVIVES`. The
nodes are `planet.air.photochemistry.n2` (citing the kept N₂ and the
trapped NH₃), `.nh3` (the trapped NH₃) and `.ch4` (the photolysed
methane and the equilibria); the partial pressures of the final air are
these nodes and the trapped CO₂, so each gas's tree reads in
chain order (outgassed, shoreline-kept, then photolysed, trapped or
photochemistry, as the gas goes through them). Where it
departs from legacy: `N2_PER_NH3` comes from the molar masses
(0.8224, not 28/34), and the rewrite's Earth has 0.7458 bar of N₂ (0.483
before the photochemistry), the target below, on which
`NITROGEN_RECYCLED_UNDER_PLATES` is fitted.

**Calibration of `METHANE_SURVIVES`** (150 K; study 23, part 2). Titan
(about 82 K) keeps its methane, Mars (about 207 K) and an abiotic Earth
(255 K) keep none; the ice's sublimation pressure is 2.2e-25 bar at
82 K and 4.6e-6 bar at 207 K, so any line in between keeps the picture,
and 150 K is kept from legacy. Searched and not found: a temperature, or
an equilibrium temperature, at which abiotic methane stops being
oxidised by the OH of water vapour; the studies of methane's lifetime
are about Earth's oxidising air, and Yung et al. 1984 feed Titan's
oxygen chemistry from an external flux of water (6.1e5 cm⁻² s⁻¹, Table
5B), not from its own.

**Earth's N₂ target** (study 23, part 5). The chain's N₂ is a weight,
so its target is the weight of Earth's N₂: 78.08 % by volume with a
mean molecular weight of 28.97 (NASA Earth fact sheet) is 0.7551 of the
mass of the dry air, 5.1352e18 kg (Trenberth & Smith 2005, *J. Climate*
18, 864, doi:10.1175/JCLI-3299.1; abstract), so 3.878e18 kg of N₂,
which weighs 0.746 bar at 9.80665 m/s². The 0.78 bar quoted elsewhere
is its share by volume times a bar.

**Weights and partial pressures** (issue #749). The system keeps an air
as the weights of its gases, `w_i = m_i g / (4 π R²)`
(`science::radiation::gases::Weights`): weights add as masses do, the
dry air's surface pressure is their sum, and a gas that comes or goes
leaves the others' weights as they were. A gas's partial pressure, its
share of the moles times the pressure, is not additive: with `n_i` moles
per area, `p_i = x_i p = n_i g M̄ / A`, which moves with the mean molar
mass `M̄` whenever any other gas comes or goes. The laws that read a
partial pressure (the column, the shortwave budget, the saturation of a
condensing gas, the sea's dissolved nitrogen) read the weights of the air
as it is when they read it (definitions, from `n_i = m_i / M_i` and a
column's weight being its pressure):

```text
p_i = x_i Σ_j w_j,    x_i = (w_i / M_i) / Σ_j (w_j / M_j)    (Weights::partial_pressures)
w_i = p_i M_i / M̄,    M̄ = Σ_j p_j M_j / Σ_j p_j               (PartialPressures::weights)
```

the sums over the dry gases; the water vapour is kept apart, as the
column takes it: a pressure added to the dry air's. Its one meaning, at
every stage, is its weight taken as its pressure at the surface (the
column's convention, `p = Σ w_dry + e`, so the surface pressure stays
the sum of the weights): the chain's runaway steam is the weight of the
water in its air (`planet.air.outgassed.h2o`, `planet.air.kept.h2o`),
and a wet climate sets it to the saturation pressure at its surface
(`surface_vapour`), the vapour weighing what it presses
(`planet.climate.air.h2o`). That is exact for a pure steam air; in a
mixed air the vapour's partial pressure would be `M̄ / M_H₂O` of its
weight (a departure kept from the column; a runaway's 270 bar of steam
over 50 bar of CO₂ would press about 10 % more than it weighs). A measured air given
by its shares by volume becomes weights through the second line: Earth's
78.08 % N₂ and 9340 ppm Ar by volume (NASA Earth fact sheet) are 0.7555
and 1.289 % of the mass among N₂, O₂ and Ar alone, the 0.7551 and 1.288 %
above within the 0.04 % of the air those three leave out.

*A gas capped at its saturation.* A condensing gas keeps the weight `w`
whose partial pressure is its cap (its saturation times its activity),
the other dry gases' weights held. With `A = Σ_j w_j / M_j` and
`B = Σ_j w_j` over the others, its partial pressure is
`(w/M) / (A + w/M) · (B + w)`, which rises with `w`; setting it to the
cap gives

```text
w² + (B − cap) w − cap A M = 0
```

whose positive root is the weight kept (`Weights::weight_at`, taken as
`2 cap A M / ((B − cap) + √((B − cap)² + 4 cap A M))` when `B` exceeds
the cap, so that it does not cancel). A pure gas keeps its cap. Checked
by hand on a Titan: over 1.422 bar of N₂ by weight (`A` = 0.05076,
`B` = 1.422) a methane capped at 0.08 bar keeps 0.0469 bar by weight
(`w² + 1.342 w − 0.06515 = 0`), whose partial pressure reads back
0.0800 bar. Scaling the methane's weight by its partial pressure's share
instead (keeping `cap / p_CH4` of it) would leave it 40 % over its cap
in the air its condensation leaves, the N₂'s share having grown.

*How the climate uses them.* The climate's air is weights end to end:
its inputs are the chain's weights, it records weights
(`planet.climate.air.*`, `timeline::Atmosphere::partial`), and each law
that reads a partial pressure converts the whole air it is given at the
moment it reads it: the longwave column (`outgoing_longwave`,
`surface_temperature`), what the surface absorbs (`absorbed`), the
shortwave budget, the moist plateau, the sulphuric deck and the ozone
column. The partial pressures are never stored and get no node of their
own. A gas that condenses keeps its exact weight through the closed form:
`condense` caps the N₂, the CH₄ and the Ar each over the others held, in
rounds until no weight moves by more than `CONDENSE_SETTLED` (1e-12 of
it; two gases that condense together, as a Triton's N₂ and Ar, each
raise the other's share as it leaves); the frost takes back from the
ground only the CO₂ or SO₂ weight that its sublimation pressure lets
into the air (`frost`); a CO₂ collapse keeps the weight whose partial
pressure is CO₂'s saturation (`collapsed_climate`). The sea's
dissolved nitrogen reads the N₂'s partial pressure, and whether an air
is a Titan's reads its total, a sum of weights.

*How the chain uses them.* The chain's steps add and scale weights. The
two that read a partial pressure convert the same air, as it stands
when they run (its kept gases and its photolysed methane): the wash-out
first, whose oxidising threshold (`OXIDISING_O2`, a twentieth of
Earth's O₂ by volume) reads the O₂'s partial pressure, with the SO₂ the
shoreline kept before it and as its column the surface pressure, the
sum of the weights; then the cold trap (`cold_trapped`, *How the
rewrite computes the cold trap*), with the SO₂ washed. The chain's O₂ is 0 until
life gives it some (2d-4), so the wash-out's reading moves no reference
body today; the trap moves Callisto's CO₂ alone among them.

**Calibration of `NITROGEN_RECYCLED_UNDER_PLATES`** (0.7965; studies 23,
part 5, and 28). Study 23 fitted it first: legacy's 0.8 gave the
rewrite's Earth 0.804 bar of N₂, 7.8 % over the target; the chain's N₂ on
Earth was proportional to the share kept, `1 − x` (0.804 bar at 0.8,
0.748 at 0.814, 0.746 at 0.8145, by running the chain on the reference
Earth), so `x = 1 − 0.2 × 0.746 / 0.804`; four digits, 0.8145, gave
0.7458 bar, 0.746 to the three digits of the target, and Earth's total
moved from 52.83 to 52.77 bar (its CO₂, 52.03 bar before the thermostat,
did not move). The current value, 0.7965, is the refit below; no other
reference body has plates. Searched and not found: a measured share of the
outgassed nitrogen that subduction returns. Goldblatt et al. 2009
(doi:10.1038/ngeo692, abstract) and Johnson & Goldblatt 2015
(doi:10.1016/j.earscirev.2015.05.006, abstract) put several times the
air's nitrogen in the mantle, which supports a large return without
giving one (study 21, part 5). The test keeps Earth's N₂ within 10 % of
0.746 bar: the target is sourced, the 10 % is the test's own tolerance.

**Refit (study 28, #679).** Earth's ocean, formed in a 200 K disc, no
longer carries N₂ ice (*Titan's nitrogen* below), which was 8.85 % of its
N₂: 0.8145 gave 0.6799 bar, still inside the 10 %, but off the target the
share is fitted on. The same method, `x = 1 − 0.1855 × 0.746 / 0.6799`,
kept to four digits, gives 0.7965 and 0.7458 bar again; Earth's NH₃ path
does not move (above NH₃'s triple point the photochemistry already turned
all of it into N₂).

**Titan's nitrogen** (study 28 of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/28-titan-nitrogen-audit),
issue #679). Titan's air is 1.467 bar at the surface (Fulchignoni et al.
2005, doi:10.1038/nature04314), 5.65 % of it CH₄ by volume (Niemann et
al. 2010, doi:10.1029/2010JE003659), so 1.384 bar of N₂ by volume and,
the chain's partials being weights (*Earth's N₂ target*), 1.418 bar by
weight: 1.467 bar times the mass share of N₂, `0.9435 × 28.014 /
(0.9435 × 28.014 + 0.0565 × 16.043)` = 0.9669, the surface's composition
standing for the column's; its CH₄ weighs 0.049 bar. Titan accreted almost
none of it as N₂: its ³⁶Ar/¹⁴N is more than five orders of magnitude
under the Sun's (Niemann et al. 2005, as Mousis et al. 2009, ApJ 691,
1780, doi:10.1088/0004-637X/691/2/1780, arXiv:0810.0308, § 1, quote it),
"an indication that only an unsubstantial amount of N₂ could have been
accreted", argon being trapped in ice at colder temperatures than N₂
(Scherf et al. 2020, *Space Sci. Rev.* 216, 123,
doi:10.1007/s11214-020-00752-0, arXiv:2011.00973, § 3; read). Its
¹⁴N/¹⁵N, 167.7, points to ammonia ice (Mandt et al. 2014, as Scherf et
al. summarise it), with perhaps up to half from organics (Miller et al.
2019, same). The building blocks lost their N₂ in Saturn's subnebula at
about 50 K and kept their CH₄ clathrate and NH₃ hydrate (Mousis et al.
2009, § 3.1; Anderson, Mousis & Ronnet 2021, arXiv:2102.03230,
abstract).

The rewrite's chain gave the reference Titan 2.876 bar of N₂ (4.99 bar
in all, 3.15 after `condense`, and a climate at 104.8 K), 99.8 % of it
from comet 67P's N₂ ice (`ICE_N2` on its 6.05e22 kg of water, times
`LID_SHARE`), 0.005 bar from its rock, and nothing from its NH₃, which
the cold trap freezes at 82 K. Its inputs are not the cause (the core
fraction from 0.10 to 0.40 moves the N₂ by −2.6 % to +5.6 %). Two laws make
Titan's N₂ from its ammonia instead:

- **The ice-N₂ gate** (`science::volatiles::traps_nitrogen`,
  `disc_temperature`). Ice traps N₂ only when it forms under 30 K
  (`NITROGEN_TRAPPING_TEMPERATURE`): comet 67P's N₂/CO "suggests that
  cometary grains formed at low-temperature conditions below ~30 kelvin"
  (Rubin et al. 2015, *Science* 348, 232, doi:10.1126/science.aaa6100,
  abstract), and "Accretion of N₂ can only occur at low temperatures
  (e.g., <30 K)" (Glein 2023, *Icarus*, arXiv:2306.06350, § 3.1; read),
  which Pluto's N₂ needs. The temperature is the disc's where the ice
  formed, not today's equilibrium (which would put the line past 72 au
  and strip Pluto): the average midplane profile `T = 200 K (r / 1 au)^-0.62`
  (Andrews & Williams 2007, *ApJ* 659, 705, as Öberg, Murray-Clay &
  Bergin 2011, *ApJL* 743, L16, arXiv:1110.5567, eq. 1, adopt it;
  `DISC_TEMPERATURE_AT_ONE_AU`, `DISC_TEMPERATURE_INDEX`), for a
  Sun-like star. Another host heats its disc as Hayashi's thin disc
  does, `T ∝ L^¼` at a fixed distance (Hayashi 1981, *Prog. Theor. Phys.
  Suppl.* 70, 35, eq. 2.3, not reached; as Ida & Lin 2005, eq. 4, and
  Kennedy & Kenyon 2008, p. 3, "T⁴_disk ∝ L⋆ a⁻²", quote it, read;
  `DISC_LUMINOSITY_INDEX`): `T = 200 K (L / L☉)^¼ (r / 1 au)^-0.62`.
  Öberg et al. say only that "a more luminous star will heat the disk
  further"; the power is Hayashi's. Around the Sun the N₂ line falls at
  21.3 au: Saturn's ice (49 K) and Jupiter's (72 K) carry no N₂,
  Neptune's (24 K) and Pluto's (20 K) do. It moves with the host as
  `L^(1 / (4 × 0.62))`: 3.3 au round a host of 0.01 L☉, 33 au round one
  of 3 L☉. The gate reads the host's present luminosity (the primary
  source, whose mass places the ice line), the only one the timeline
  holds, as Hayashi took the young Sun's at today's "for simplicity"
  ([giant planet placement](giant-planet-placement.md)). Earth's ocean,
  from a 200 K disc, carries none either: its nitrogen is chondritic
  (Marty 2012, as cited by Scherf et al. 2020, § 1). The chain records
  the disc's temperature (`planet.volatiles.disc-temperature`, from the
  semi-major axis and the host's luminosity) and the gate
  (`planet.volatiles.ice-traps-n2`), from which the outgassed N₂
  derives.

  **Two discs, two epochs.** The model's ice line, 2.7 au round the Sun
  (`orbit::ICE_LINE_OF_THE_SUN`), is where Hayashi's thin disc, lit at
  the main-sequence luminosity, reaches water's 170 K (Ida & Lin 2005,
  eq. 4 and the text of eq. 12, "T = 170(a/a_ice)^-1/2 K"; Kennedy &
  Kenyon 2008, Fig. 1, "a at Tmid = 170 K"). The gate's disc is the
  observed average midplane, colder: 108 K at 2.7 au, its water snowline
  (135 K, Öberg et al. Table 1) at 1.9 au. The two do not contradict
  each other: a disc's snow line moves inward as it ages, and Kennedy &
  Kenyon's crosses "the canonical distance of 2.7 AU at 5 × 10⁵ yr" for
  the Sun, their snow line differing from Ida & Lin's because the latter
  "is based on the main-sequence luminosity [...] and an optically thin
  disk" (p. 3). The ice line places where a giant's core gathers ice;
  the gate asks how cold the ice was when it formed, which the colder,
  observed midplane answers. Anchored on the ice line instead (170 K at
  2.7 au), the N₂ line would fall at 44 au with the profile's slope (86
  au with Hayashi's), and Pluto's and Triton's ice would lose the N₂
  they hold (study 28).
- **The NH₃ the interior decomposes** (`science::outgassing::decomposed`,
  `AMMONIA_DECOMPOSED`). Glein 2015 (*Icarus* 250, 570,
  doi:10.1016/j.icarus.2015.01.001) builds Titan's N₂ from the NH₃ its
  interior decomposes, `2 NH₃ → N₂ + 3 H₂`, outgassed as N₂:
  `n_N2 = ε (ξ/2) (n_NH3/n_H2O) (f_H2O/µ_H2O) M`, with an outgassing
  efficiency `ε` of 0.8 to 4.8 % from Titan's argon and a reaction
  progress `ξ ≤ 1` (Erkaev et al. 2020, MNRAS, arXiv:2010.12342, eqs.
  41-42 and Table 2, read; Glein's paper itself not reached). The model's
  `ε` is the regime's share times the share under the ocean; the new law
  sends `ξ` of the NH₃ the interior brings up to the air as N₂ (at
  `N2_PER_NH3` of its mass), the rest going on as NH₃ to the cold trap
  and the photochemistry. Above NH₃'s triple point the photochemistry
  makes the same N₂ of it, so the law changes nothing there.

**Calibration of `AMMONIA_DECOMPOSED`** (`ξ` = 0.120; study 28). Fitted
on Titan's N₂ by weight, 1.418 bar: the reference Titan's N₂ is its
rock's 0.005 bar plus `ξ` times the 11.807 bar of N₂ its 14.356 bar of
kept NH₃ would make, so `ξ = 1.413 / 11.807 = 0.1197`, kept to three
digits (1.422 bar). The fit absorbs the reference Titan's inputs, its
45 % water among them (Sohl et al. 2003; Fortes 2012's interiors hold 21
to 36 %, as Erkaev et al. 2020, Table 2, tabulate them). The model's
`ε ξ`, 0.2 × 0.120 = 2.4 %, sits inside the 0.8 to 4.8 % Glein's `ε`
gives at `ξ = 1`: a consistency, not a source. Searched and not found: a
measured or bounded `ξ` other than Glein's `ξ ≤ 1` (Scherf et al. 2020,
Erkaev et al. 2020, Glein 2023). Left out of the fit: Titan's escape.
For an atmosphere outgassed after Titan formed, as Glein's is, Erkaev et
al. 2020 (Table 3) find Titan lost 0.48 to 0.62 times its present air
for a slowly rotating young Sun and 0.48 to 2.02 for a moderate or fast
one (2.22 and 15.73 are their losses for an air present from the
start), where the model's shoreline takes 0.4 % of it; with that loss `ξ`
would be 0.18 to 0.36. The non-thermal loss waits for #658.

**Effect on the reference bodies** (the bare air before the climate,
before and after). Titan: N₂ 2.876 to 1.422 bar, CH₄ unchanged (2.11 bar
photolysed), 4.99 to 3.54 bar in all; on the chain's own air its climate
settles at 94.24 K and 1.512 bar, where it settled at 104.84 K and
3.15 bar (measured 93.65 ± 0.25 K and 1.467 bar). Its seas hold
hydrocarbons, and its air kept 0.091 bar of CH₄, then a partial
pressure the climate read as a weight. Since the climate keeps its air
in weights (#749) its CH₄ is a weight too: the climate caps it at the
weight whose partial pressure is its activity times its saturation
pressure (`condense`), and the measured air by weight is 1.418 bar of
N₂ and 0.049 bar of CH₄ (0.083 bar of CH₄ by volume). Earth: its ocean's N₂ ice was 8.85 % of its N₂,
0.7458 to 0.6799 bar; `NITROGEN_RECYCLED_UNDER_PLATES`, fitted on that
target, is refitted (above). Callisto: 0.060 to 0.030 bar. A Triton-like
body (0.0036 M⊕ at 30 au) keeps its ice N₂ and gains its NH₃'s, 0.338 to
0.504 bar; its climate, set by the N₂ frost, does not move (38.17 K).
Venus and Mars carry no ice and do not move.

Departures: the disc profile is an average of observed discs, not the
Sun's. It reads the host's present luminosity, not the one at the
formation of its ice: a young star is brighter (Kennedy & Kenyon 2008
follow it on pre-main-sequence tracks), and a host that has left the
main sequence would heat the disc of its planets' past. A body is placed in the disc at its present
orbit, as the ice line places it, so a water world that migrated in from
beyond the ice line (`Drawn::is_water_world`) has its ice gated at its
present, warmer orbit and carries no N₂ ice. `ξ` is fitted on one body
and absorbs its inputs (its 45 % water). The climate's CH₄ is a partial
pressure where its N₂ is a weight. Titan's organics (Miller et al. 2019)
are not a nitrogen source of the model.

Calibration targets, through the whole chain from the reference bodies'
inputs (volatiles of the rule, no scatter) round the Sun at 4.6 Gyr (the
Earth row and the rewrite's values below it are at 4.567 Gyr):

| Body | Result | Target | Observed |
|---|---|---|---|
| Venus | 87 bar, 98 % CO₂, 1.7 bar N₂ (legacy's, read as weights) | 60 to 120 bar, CO₂ over 90 %, N₂ 1 to 6 bar | 92 bar, 96.5 % CO₂, about 3 bar N₂ (by volume) |
| Earth | 0.7458 bar N₂, no CH₄ or NH₃ (52 bar CO₂ before the thermostat) | N₂ 0.746 bar within 10 %; CH₄ and NH₃ under 1e-3 bar | 0.746 bar N₂ by weight (0.78 by volume) |
| Mars | 0.006 bar (legacy scaled its CO₂ to the real 6.1 mbar with `MARS_CO2_FACTOR`, not ported; 0.014 bar from the rule alone) | 0.002 to 0.05 bar | 0.006 bar |
| Titan | 2.2 bar, 95 % N₂, 0.11 bar CH₄ after the climate (6.3 bar before it condenses) | 0.5 to 5 bar; CH₄ over 0.01 bar | 1.5 bar, about 5 % CH₄ |
| Ganymede, Callisto | no CH₄ (0.15 and 0.47 bar before photolysis) | CH₄ under 1e-3 bar | airless |
| Mercury, Moon | under 1e-8 bar | under 1e-4 bar | exospheres |

The Earth row is the rewrite's; the other results are `legacy-final`'s,
after its climate where the row says so. The rewrite's chain, before
the climate (2d-3), gives Venus 89.5 bar, 98 % CO₂ and 1.75 bar N₂ by
weight, which are 96.9 % CO₂ and 3.05 % N₂ of its moles (a partial
pressure of 2.73 bar of N₂; the tests compare the partial pressures
with the shares by volume, #749);
Titan 3.54 bar, of which 1.422 bar N₂ and 2.11 bar CH₄ (4.99 bar and
2.88 bar of N₂ before #679); Callisto 0.030 bar of N₂ and no CH₄. Titan's
target in the rewrite is its N₂ by weight, 1.418 bar within 10 %, and its bare N₂
and photolysed CH₄ within legacy's 0.5 to 5 bar (*Titan's nitrogen*).

As planets round the Sun, the reference moons above have a shortcoming,
outside the targets: Ganymede keeps 0.016 bar of N₂ and Callisto, to
which the core model gives a weak deep dynamo, 0.047 bar, where both are
airless. Before methane photolysis they also kept 0.15 and 0.47 bar of
CH₄. As moons (below), they come out airless.

**Moons** (`planet::derive_moon_model`). A round moon goes through the
same chain under its star's light at its planet's orbit (the same XUV
fluence and shoreline), with two rules of its own:

- *Born warm, no hypervolatiles.* A moon born in its planet's disk keeps
  the ammonia, methane and nitrogen of its ices only when its planet's
  equilibrium temperature is under 100 K (`HYPERVOLATILES_MAX_K`). Titan's
  nitrogen came from ammonia ice of the protosolar disk (Mandt, Mousis,
  Lunine & Gautier 2014, *ApJ Letters* 788, L24,
  doi:10.1088/2041-8205/788/2/L24), which Saturn's cold subnebula let its
  moons keep (Mousis, Gautier & Bockelée-Morvan 2002, *Icarus* 156, 162,
  doi:10.1006/icar.2001.6782); Jupiter's warmer one did not. Both DOIs
  were checked against Crossref; the papers were not read beyond their
  titles and abstracts. The planet's equilibrium temperature stands for
  its disk's. **Calibration**, no source, for 100 K: it sits between
  Jupiter (112 K, Ganymede and Callisto airless) and Saturn (83 K, Titan
  with its N₂), set there so that these moons come out as themselves. A
  captured moon (a body from beyond the ice line, Triton) keeps these
  ices; a moon born of an impact, or without ice, draws as a planet. The
  amounts kept are the planet rule's (1 % of the ice in NH₃ and in CH₄),
  which give Titan its nitrogen. The rewrite has no moons yet; its Titan,
  a planet, takes its nitrogen from the share of its NH₃ its interior
  decomposes, its ice carrying no N₂ (*Titan's nitrogen* above).
- *Inside a giant's magnetosphere, no shield.* A moon of a giant (a host
  of 10 M⊕ or more) meets the unshielded escape whatever its own dynamo:
  the giant's corotating plasma reaches its upper air (Kivelson et al.
  2004, in *Jupiter: The Planet, Satellites and Magnetosphere*, ch. 21,
  not checked). **Calibration**, no source, for taking every giant's
  magnetosphere as enclosing its moons: it leaves Io airless (the core
  model gives it a dynamo, which the real Io lacks, and with it Io kept
  about 0.01 bar of N₂). The Moon, whose host is the Earth, keeps the
  planet rule.

Through `derive_moon_model` (G2V, 4.57 Gyr, seed 1, the moons' real
masses, ice shares and orbits):

| Moon | Result | Observed |
|---|---|---|
| Titan | hydrocarbon world, 93.5 K, 1.43 bar, 94 % N₂, 0.083 bar CH₄ (5.8 %) | 93.7 K, 1.47 bar, 5.7 % CH₄ |
| Ganymede, Callisto | exospheres of 3e-12 bar (CO₂), 116.6 K, Bond albedo 0.19 (alone, out of resonance, Ganymede keeps one old unit, Callisto's; in its chain, 0.45 and 102 to 108 K, see "Icy albedo") | exospheres of 1e-12 (O₂) and 7.5e-12 bar (CO₂); Bond albedo about 0.4 and 0.2 |
| Io | heat pipe, 9.3e-10 bar of SO₂ over its SO₂ frost (N₂ and CO₂ an exosphere of 3e-12 bar), 105.2 K, Bond albedo 0.63 | heat pipe, SO₂ of about 1e-9 bar, Bond albedo 0.63 under SO₂ frost |
| Europa | exosphere of 3e-12 bar (CO₂), 96.1 K, Bond albedo 0.63 | exosphere of about 1e-12 bar (O₂), Bond albedo about 0.62 to 0.68 |
| Triton | 1.7e-5 bar of N₂ over its frost at 37.5 K, Bond albedo 0.75 | 14 ± 1 µbar (1.4e-5 bar) of N₂ (Gurrola 1995, refining Tyler et al. 1989) at 38 K (Tyler et al. 1989), Bond albedo 0.82 ± 0.05 (Hillier et al. 1990) |
| Moon | 1e-12 bar | airless |

Over five seeds Titan ranges from 1.1 to 8.7 bar (its volatiles scatter
by 0.3 dex). Io's N₂ and CO₂ are an exosphere on every seed (Jupiter's
plasma torus exhausts them, see *Non-thermal loss*); its air is the
SO₂ of its volcanic frost (see *Volcanic SO₂*). Triton's N₂ is its frost's vapour
pressure at its surface temperature, which the frost's own albedo sets
(issue #464; before, the cold trap held it at the equilibrium
temperature under the reference albedo 0.3, 47 K, and kept 1.9e-3 bar),
and its emissivity (`N2_ICE_EMISSIVITY`, see *Climate and surface
liquid*). The curve and the emissivity were set together (issue #529):
Fray & Schmitt's β-N₂ curve puts the observed 14 µbar at 37.2 K, within
a kelvin of the observed 38 K (the 14 ± 1 µbar of the Voyager 2 radio
occultation, Gurrola 1995, a Stanford PhD thesis not reached, refining
the 1.6 ± 0.3 Pa of Tyler et al. 1989, *Science* 246, 1466,
doi:10.1126/science.246.4936.1466, whose abstract, read, also gives the
38 K; both as Sicardy et al. 2024, arXiv:2402.02476, quote them, read), and legacy's emissivity 0.89 centred the
`triton-like` archetype between them (the rewrite takes 0.80, see "Calibration of `FROST_ALBEDO`" below): 37.2 to 37.6 K and 1.40e-5 to
1.77e-5 bar of N₂ over seeds 1 to 40 (median 37.4 K and 1.57e-5 bar).
Before, with N₂ ice a black body and the curve 6.3 kJ/mol from memory,
it ranged from 35.4 to 37.0 K and 1.04e-5 to 2.52e-5 bar. That range
also needed the light of its Neptune narrowed: the N₂ vapour about
doubles every kelvin, and the `neptune-like` archetype drew ±9 % of
insolation (0.0010 to 0.0012 of the Earth's), ±0.75 K on Triton, which
no emissivity fits into 1e-5 to 2e-5 bar. It now draws what Neptune
itself receives over its orbit, 0.00109 to 0.00113 (perihelion 29.80
and aphelion 30.34 AU).

### Argon (study 29)

Study 29 of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/29-argon-audit),
for issue #694, gives the chain its argon: `science::outgassing` makes it,
`system::planet::air` outgasses it (`planet.air.outgassed.ar`, then
`planet.air.kept.ar`), the column carries it (`PartialPressures.ar`) and the
climate condenses it (`planet.climate.air.ar`, `planet.climate.condensate.ar`).
This section is the law, with its sources, its checks and what the chain gives;
the partials stay weights (*Earth's N₂ target*).

**Earth's argon, measured.** Argon is 9340 ppm of the dry air by volume (NASA
Earth fact sheet, read; Picard et al. 2008, *Metrologia* 45, 149,
doi:10.1088/0026-1394/45/2/004, adopt a remeasured argon fraction in the
CIPM-2007 density formula, abstract only). With the air's mean molecular weight,
28.97, and argon's standard atomic weight, 39.95 (CIAAW 2024), argon is 1.288 %
of the dry air's mass, 5.1352e18 kg (Trenberth & Smith 2005): 6.61e16 kg,
1.656e18 mol, which weighs 0.0127 bar at 9.80665 m/s² on a 6371 km sphere
(derived). The air's ⁴⁰Ar/³⁶Ar is 298.6 (Lee et al. 2006, *GCA* 70, 4507,
doi:10.1016/j.gca.2006.06.1563, as Avice & Marty 2020, p. 24 of
arXiv:2003.11431, quote it; its uncertainty was not read) and its ³⁸Ar/³⁶Ar
0.188 (Marty 2012, p. 9 of arXiv:1405.6336). ⁴⁰Ar is then 99.60 % of the air's
argon, 1.649e18 mol (Marty 2012, p. 15, gives 1.6e18 mol after Ozima & Podosek
2002), and ³⁶Ar 5.5e15 mol. The air's argon is radiogenic: the ⁴⁰Ar that the
decay of ⁴⁰K made in the rock.

**The decay of ⁴⁰K.** ⁴⁰K decays to ⁴⁰Ca (β⁻) or to ⁴⁰Ar (electron capture and
β⁺). The evaluations, as the decay constants in 1e-10 per year (Schaen et al.
2020, *GSA Bulletin* 133, 461, doi:10.1130/B35560.1, Table 1, read in the HAL
copy hal-02903663):

| Evaluation | λ to ⁴⁰Ar | λ to ⁴⁰Ca | λ total | Half-life (derived) | Share to ⁴⁰Ar (derived) |
| --- | --- | --- | --- | --- | --- |
| Steiger & Jäger 1977 | 0.581 | 4.962 | 5.543 | 1.2505 Gyr | 10.48 % |
| Min et al. 2000 | 0.580 ± 0.007 | 4.884 ± 0.049 | 5.463 ± 0.107 | 1.2688 Gyr | 10.62 % |
| Renne et al. 2011 | 0.5757 ± 0.016 (as printed) | 4.9548 ± 0.0134 | 5.531 ± 0.0135 | 1.2532 Gyr | 10.41 % |
| DDEP 2025 (Mougeot) | | | | 1.2522 ± 0.0027 Gyr | 10.439 % |

The DDEP evaluation of 2025 (X. Mougeot, ICRM 2025 poster, HAL cea-05087440,
read) gives the half-life 1.2522(27) Gyr and the branches β⁻ 89.56(7) %,
electron capture to ⁴⁰Ar's 1461 keV level 10.34(7) %, to its ground state
0.098(25) %, β⁺ 0.00103(13) %: 10.439 % to ⁴⁰Ar (derived, the sum). It lists
ENSDF 2017's 1.2480(30) Gyr, DDEP 2009's 1.2504(30) and DDEP 1998's 1.265(13),
and takes ⁴⁰K as 0.011668(8) % of potassium's atoms (Naumenko et al. 2013, as
the poster quotes it), so 2.9843e-3 mol of ⁴⁰K per kg of potassium (derived with
potassium's standard atomic weight, 39.0983(1), CIAAW). The law takes the DDEP
2025 values: the newest evaluation, and the one whose ground-state branch, the
source of the old disagreement, was measured since (Kossert et al. 2022, as the
poster cites it). The code's `HALF_LIFE_K40` took 1.277 Gyr (Dye 2012's λ
17.200e-18 per s), longer than every one of them, by 0.6 % (Min et al. 2000) to
2.3 % (ENSDF 2017) and by 2.0 % than DDEP 2025's, until the argon's branch
(#694) set it to DDEP's 1.2522 Gyr (decided 2026-11-25): the heat and the argon
read one value. Potassium's heat per kilogram, Dye's 3.33e-3 µW/kg on his λ, is
derived since from DDEP's decay data on the same λ, 3.400e-3 µW/kg (*The heat
inside* in [planet interiors](planet-interiors.md#the-heat-inside)). Earth's
radiogenic power today moves from 20.18 to 20.25 TW (+0.39 %, potassium's
share); at formation it is 104.1 TW against 100.8 before the branch (+3.2 %),
and 62.2 against 60.6 TW at 1 Gyr (+2.6 %) (from the code's isotopes).

**Earth's potassium.** The heat of the model takes 280 ppm of potassium in
Earth's silicate part (`POTASSIUM_ABUNDANCE`, Arevalo, McDonough & Luong 2009;
280 ± 60 ppm at 1σ as Marty 2012, p. 15, quotes it). Other estimates: Wang,
Lineweaver & Ireland 2018 (*Icarus* 299, 460, doi:10.1016/j.icarus.2017.08.024,
arXiv:1708.08718, Table 1, read) average three primitive-mantle models
(Lyubetskaya & Korenaga 2007, McDonough & Arevalo 2008, Palme & O'Neill 2014)
into 237 ± 25 ppm, and quote the first two as 190 ± 76 and 240 ± 48 ppm (p. 4);
Jochum et al. 1983's K/U of about 12 000 gives 250 ppm and Albarède 1998 allows
a factor of two (both as Marty 2012, p. 15, quotes them); Farcy, Arevalo &
McDonough 2020 (*JGR Solid Earth*, doi:10.1029/2020JB020245, abstract) find a
MORB K/U of 13 900 ± 200. The spread, 190 to 280 ppm, is the main uncertainty of
the law, as large as the share degassed.

**⁴⁰Ar produced.** A body whose silicate part has the mass `M`, the radiogenic
abundance `a` of the heat (Earth = 1, the same draw `radiogenic_abundance` that
scales U, Th and K together) and the age `t` has made, since it formed,

$$
n_{40}(t) = b \, a \, \frac{c_K x_{40}}{m_K} \, M \,
  e^{\lambda t_\oplus} \left(1 - e^{-\lambda t}\right)
$$

moles of ⁴⁰Ar, with `b` = 0.10439 the share to ⁴⁰Ar, `c_K` = 280e-6 the
potassium of Earth's silicate part, `x₄₀/m_K` = 2.9843e-3 mol of ⁴⁰K per kg of
potassium, `λ = ln 2 / 1.2522 Gyr` and `t⊕` = 4.5673 Gyr: the ⁴⁰K a body holds
at its formation is Earth's at its own (the heat's convention,
`science::heat::radiogenic`), and the share `1 − e^{−λt}` of it has decayed. On
Earth (4.03e24 kg of silicate, 4.5673 Gyr) it is 4.053e18 mol, 1.006e-6 mol per
kg of silicate (derived; Marty 2012, p. 15: 3.9 ± 0.9e18 mol over 4.5 Gyr from
280 ± 60 ppm). With the former `HALF_LIFE_K40`, 1.277 Gyr, it would be
3.842e18 mol (−5 %). In the code: `science::heat::potassium_40_decayed` (the
heat's `POTASSIUM_ABUNDANCE`, `K40_ATOM_SHARE` and `POTASSIUM_MOLAR_MASS`)
times `science::outgassing::ARGON_40_BRANCH` is `radiogenic_argon`; the test
`earths_rock_made_martys_argon` holds it within 0.5 % of 4.053e18 mol and
inside Marty's 1σ.

**The share degassed.** Marty 2012 (p. 15) puts 1.6e18 of the 3.9e18 mol in the
air, at most 3.5e17 mol in the continents (Arevalo et al. 2009), and the rest,
about half, still in the mantle (Allègre, Hofmann & O'Nions 1996, *GRL* 23,
3555, doi:10.1029/96GL03373, abstract: about half of the ⁴⁰Ar is retained in the
solid Earth). The share in the air is 1.6 / 3.9 = 0.41, and 1.649 / 4.053 =
0.407 with the law's own production (derived). Argon needs this share of its
own: the mantle's primordial volatiles came out early, most of them in the first
magma ocean and the first Gyr of melting, while ⁴⁰Ar was made over the whole
life of the rock, and a volatile made late comes out only through the melting of
the later Gyr. Venus shows it: more CO₂ and N₂ than Earth, a quarter of its ⁴⁰Ar
(Sasaki & Tajika 1995, AIP Conf. Proc., doi:10.1063/1.48741, abstract: the
long-term degassing of Venus and Mars is 1/4 and 1/20 of Earth's, from the ⁴⁰Ar
of each air over the planet's mass). The models that follow argon degas it with
the melt: the air's ⁴⁰Ar grows as the mantle's ⁴⁰Ar times the share of the
mantle melted per unit time (O'Rourke & Korenaga 2015, as Krissansen-Totton,
Fortney & Nimmo 2021, *PSJ* 2, 216, doi:10.3847/PSJ/ac2580, Appendix A.3, write
it; on Mars, with a crustal production rate and a share of the melt degassed of
0.3 to 1.0, Lichtenegger et al. 2022, arXiv:2105.09789, eq. 32). The model has
no melting history, so it takes the ratio:

| Regime | Share of the ⁴⁰Ar made that reaches the air | Tag |
| --- | --- | --- |
| Plates, heat pipe | 0.41 | Source: Marty 2012, p. 15 (1.6e18 of 3.9e18 mol) |
| Stagnant lid | 0.41 × `LID_SHARE` / `MOBILE_SHARE` = 0.0863 | Definition: the ratio by which the outgassing scales a stagnant lid's other volatiles (`volcanism.rs` scales the intensity the same way); it inherits the calibrations `LID_SHARE` and `MOBILE_SHARE` |
| Episodic lid | 0.0863, as a stagnant lid | A documented choice, not the outgassing's ratio: the episodic lid's other volatiles take `MOBILE_SHARE`, its ⁴⁰Ar the lid's share (below) |

The episodic lid takes the lid's share although its CO₂ and N₂ take the
plates' (*Outgassing*): Venus's 90 bar of CO₂ came out early, its ⁴⁰Ar through
the lid of its later Gyr. This is a choice, and its reason is Venus's ⁴⁰Ar
itself (Sasaki & Tajika 1995 read the quarter of Earth's degassing from it):
with the plates' share Venus would hold 3.4 times its measured ⁴⁰Ar (*Checks*
below); with the lid's it holds 0.69 times, inside the measurement's 1σ. The
Venus check is therefore not independent of the choice. A share fitted on Venus
(0.12 with the model's potassium) would be a new calibration; the ratio adds
none of its own, and inherits `LID_SHARE` and `MOBILE_SHARE`. In the code:
`science::outgassing::argon_degassed_share`, `ARGON_DEGASSED_UNDER_PLATES`.

The ⁴⁰Ar then goes the way of every outgassed gas: its mass, 39.95 g/mol per
mole, weighs `P = m g / (4π R²)` (`column_pressure`), a deep ocean lets out the
share `released_under_ocean`, and the cosmic shoreline keeps the share
`shoreline_kept`. It is not recycled by subduction (the
`*_RECYCLED_UNDER_PLATES` shares are for the volatiles that the crust binds; the
share 0.41 is already what stays in the air), it condenses only as the frost
of a body colder than its triple point (*Argon's frost* below), and the
photochemistry does not touch it.

**Primordial argon.** ³⁶Ar and ³⁸Ar came with the planet, from the solar nebula,
the solar wind or the ices. On Earth they are 0.4 % of the air's argon, on Mars
0.05 % (⁴⁰Ar/³⁶Ar of about 1900, below), on Titan 0.6 % (below); only Venus
holds as much of them as of ⁴⁰Ar (⁴⁰Ar/³⁶Ar 1.11, below), and its whole argon is
6 mbar of its 92 bar. Venus's ³⁶Ar per kg of planet is about 70 times Earth's
(3.3e17 mol over 4.87e24 kg against 5.5e15 over 5.97e24, derived), which no rule
of composition the model knows explains (Avice et al. 2022 review the solar-wind
and cometary hypotheses). The ices are the other source: comet 67P's argon is
5.8 ± 2.2e-6 of its water by number, its ³⁶Ar/N₂ 5.5 ± 1.5e-3 (Rubin et al.
2018, *Sci. Adv.* 4, eaar6297, doi:10.1126/sciadv.aar6297, Table 2, read;
Balsiger et al. 2015, *Sci. Adv.* 1, e1500377, doi:10.1126/sciadv.1500377, read:
³⁶Ar/N₂ 9.1 ± 0.3e-3, the two "closely correlated owing to their similar
volatility", and ³⁶Ar/³⁸Ar 5.4 ± 1.4), so 1.29e-5 kg of argon per kg of ice
(derived). Argon and N₂ leave the same ices together (Mousis et al. 2009, § 3.1:
planetesimals warmed to about 50 K release CO, Ar and N₂), so argon would join
study 28's gate: the same disc profile, the same 30 K (*Titan's nitrogen*).
Scherf et al. 2020 (§ 3) write instead that argon is trapped at colder
temperatures than N₂; the comet's correlated N₂ and argon, read above, disagree.
Titan's ³⁶Ar/¹⁴N, 1.1e-7 (derived from Niemann et al. 2010's ³⁶Ar, below, and
its N₂), is four orders of magnitude under 67P's ³⁶Ar/N₂ (2.75e-3 per nitrogen
atom), as the gate wants: Titan's ice held no N₂ and no argon.

The primordial argon is left out of the first step: it is climatically nothing
(0.05 mbar on Earth, under 3 mbar on Venus) and Venus's cannot be predicted. An
ice argon would condense on the cold bodies by the frost law below, as the
radiogenic argon does.

**Argon's frost** (study 29b of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/29b-argon-frost)). A
body colder than argon's triple point holds its argon as frost wherever the
frost's vapour pressure is under the argon's weight. Its triple point is 83.806
K and 68 891 Pa (Tegeler, Span & Wagner 1999, *J. Phys. Chem. Ref. Data* 28,
779, doi:10.1063/1.556037; the 68 891 Pa as CoolProp's `Argon.json` gives
Tegeler et al.'s triple point in its melting line, `p_0` at `T_0` = 83.8058 K,
read; Xiao et al. 2025 print 0.0689 MPa and 83.806 K; CoolProp's saturated state
at 83.806 K is 68 892 Pa; the NIST WebBook lists 83.8 K and 0.689 bar after
Gosman, McCarty et al. 1969). Two fits of the measured sublimation pressures
were read:

- Xiao, Sriskandaruban & Maynard-Casely 2025 (*Int. J. Thermophys.* 46, 14,
  doi:10.1007/s10765-024-03469-2, eq. 55, read in the Research Square preprint):
  `p = p_t exp[(T_t / T)(e₁ θ + e₂ θ^1.5 + e₃ θ⁵)]`, `θ = 1 − T / T_t`, `e₁ =
  −10.763`, `e₂ = −1.526`, `e₃ = −0.4245`, valid from 26 to 83.806 K. It is
  fitted to the same data as their solid equation of state (their Table 6:
  Ancsin 1973, Boato et al. 1961, Chen et al. 1971 and 1978, Clark et al. 1951,
  Flubacher et al. 1961, Freeman & Halsey 1956, Lee et al. 1970, Leming &
  Pollack 1970 from 26 K, Levenson 1967 from 24 to 31 K, Regnier 1972), whose
  sublimation pressure is good to 1 % above 50 K; under 50 K only Leming &
  Pollack's and Levenson's points exist, the latter read from a plot and 100 %
  off, and the authors call for "more reliable sublimation measurements at T <
  50 K".
- Fray & Schmitt 2009's `Ar-1`, `ln(p / 1 bar) = Σ Aᵢ / Tⁱ` with `A₀` to `A₄` =
  10.69, −893.2, −3567, 65 740, −428 000, as Table 2 of Lisse et al. 2021
  reprints it (read in arXiv:2009.02277), the compilation the model already
  reads for N₂'s frost. It gives 0.688 bar at the triple point; its range was
  not read.

The two agree within 1 % at 70 K and 5 % at 60 K, and part below: the Fray &
Schmitt curve is 18 % higher at 50 K, 58 % at 38 K, 2.4 times at 30 K (derived).
At the 37 to 39 K of Triton's and Pluto's ices argon's frost holds 0.019 to
0.078 Pa (Xiao) or 0.031 to 0.12 Pa (Fray & Schmitt); the 1.6 between them is
the uncertainty there. The law takes Xiao et al.'s: newer, fitted to every data
set with a stated range that covers those ices, and anchored on the triple point
of the fluid's reference equation. Under 26 K it carries on the slope at 26 K,
as N₂'s does under 10 K: the 26 K is a `Source:` (the fit's stated lower bound),
the slope carried under it a `Definition:`. Above the triple point the liquid's
curve is CoolProp's ancillary of Tegeler et al.'s equation (`ANCILLARIES.pS` of
`Argon.json`, read: `p = p_c exp[(T_c / T) Σ nᵢ θᵢ^tᵢ]`, `θ = 1 − T / T_c`,
`T_c` = 150.687 K, `p_c` = 4.863 MPa, its largest error against the equation
0.0077 %), 68 892 Pa at the triple point and 1.011 bar at the normal boiling
point, 87.28 K (Gosman et al. 1969, through the WebBook; 0.23 % under 1 atm);
above `T_c` argon does not condense.

Argon's heat of sublimation is not needed by the law: the fits are its curve.
Their slope, by Clausius-Clapeyron for an ideal vapour, `L = R T² d ln p / dT`,
is 7.9 to 8.1 kJ/mol over 26 to 84 K for Fray & Schmitt's and 7.5 to 8.7 kJ/mol
for Xiao et al.'s (derived). A Clausius-Clapeyron curve from the triple point
with a constant sourced `L` would be a `Definition:` built on `Source:` values,
as NH₃'s and SO₂'s frosts are in the code, and is the fallback where no fit
exists; for argon, with `L` = 7.9 kJ/mol (Fray & Schmitt's slope at the triple
point), it would give 0.080 Pa at 38 K, 1.29 times Fray & Schmitt's curve and
2.04 times Xiao et al.'s there (derived with R = 8.3145 J/(mol·K), `T_t` =
83.806 K, `p_t` = 68 891 Pa). Xiao et al.'s Figure 8 plots Chen et al. 1978's
heats of sublimation as −6.0 to −6.4 kJ/mol, which is not the slope of their own
eq. 55; that figure is not used.

In the code: `science::volatiles::argon_frost_pressure` (eq. 55, the slope
carried on under 26 K) and `argon_saturation_pressure` (the frost, then the
liquid ancillary, infinite from `ARGON_CRITICAL_TEMPERATURE`). Tests: eq. 55
gives the triple pressure at the triple point and 0.0219 Pa at 37.2 K; Fray &
Schmitt's `Ar-1` stays within a factor 1.7 of it from 37 K up and within 2 %
from 65 K up (the largest gap there is 1.7 %, near 75 K: the 2 % is the test's
choice; study 29b's 1 % is the tolerance of `condense` on eq. 55's 0.0219 Pa at
37.2 K, below, not of this agreement); the liquid gives 1.011 bar at 87.28 K
(0.23 % under 1 atm) and meets the frost at the triple point within 0.002 %.

**Argon with the N₂ frost.** In the climate, `condense` caps each condensing gas
at its activity times its saturation pressure: N₂ at activity 1 over its frost,
CH₄ at its share of a sea (Raoult's law) or 1 over its own ice. Argon joins it
as a third gas on the same path. Three pictures of a minor volatile over an N₂
ice are in the literature (Lellouch et al. 2010, *A&A* 512, L8, arXiv:1003.2866,
§ 3, read): an ideal solid solution, where the gas's pressure is its mole
fraction in the ice times its pure vapour pressure (Raoult's law); pure deposits
of the less volatile ice, segregated as the N₂ sublimates, whose vapour is the
pure one (Stansberry et al. 1996 for Pluto's CH₄, as Lellouch et al. quote it);
and a "detailed balancing layer", in which the air's mixing ratio is that of the
ice reservoir (Trafton 1990, Trafton et al. 1998, as quoted). On Triton,
Raoult's law puts CO and CH₄ 1 and 3 orders of magnitude under what the air
holds; the measured CO fits the detailed balancing (Lellouch et al. 2010). Argon
and N₂ form solid solutions, mutually soluble near the solidus and less so as
the temperature falls, with a miscibility gap of two hexagonal phases whose
critical point is near 56 K (Barrett & Meyer 1965, *J. Chem. Phys.* 42, 107,
doi:10.1063/1.1695654; Yantsevich, Prokhvatilov & Krupskii 1986,
doi:10.1063/10.0031462; abstracts). The law takes argon at activity 1 over its
own frost, as CH₄ over its ice: the same rule, no new number, and the most argon
of the three pictures. Raoult's law would leave in the air a few 1e-5 of the
activity-1 argon, the detailed balancing about 2e-3 of it (derived below). In
the code `science::surface::condense` caps argon third, after N₂ and CH₄, at
`argon_saturation_pressure` and lays the rest in `Condensed.condensate`; the
climate's fixed point calls it at each step, so argon follows the temperature
with N₂. The cold trap at the equilibrium temperature does not read argon, and
the frost of the ground (`surface::frost`, hence `frost_cover` and
`frost_emissivity`) leaves it out: a few grams per square metre on Triton,
nothing a disc's albedo or emissivity would show. A liquid argon (it needs,
between 83.8 and 150.7 K, more argon in the air than the liquid's vapour
pressure, 0.689 bar at the triple point rising to 48.6 bar at 150.7 K) is not a
surface liquid of the model either.

**Checks on the cold bodies.** At the N₂ ice temperature that its air sets, by
the N₂ curve of the code, and with the law's argon (8.68e-8 mol per kg of
silicate under a lid at 4.5673 Gyr, silicate 0.5 to 0.7 of the mass, an
assumption for the check):

| Body | Mass, radius (fact sheet) | N₂ air | N₂ ice | Argon made into the air | Argon's frost (Xiao; Fray & Schmitt) | Argon in the air, activity 1 | Raoult; detailed balancing | Observed |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Triton | 2.14e22 kg, 1353.4 km | 14 µbar | 37.2 K | 1.3 to 1.8 Pa | 0.022; 0.036 Pa | 0.22 to 0.36 µbar, 1.6 to 2.5 % | 7e-7 Pa; 4e-5 Pa | no noble gas measured |
| Pluto | 1.303e22 kg, 1188 km | about 13 µbar | 37.1 K | 0.8 to 1.1 Pa | 0.020; 0.033 Pa | 0.20 to 0.33 µbar, 1.5 to 2.5 % | about 5e-7 Pa; 3e-5 Pa | Ar I 1048 Å under 0.14 R (3σ) |
| The model's Triton-like body (study 28) | 0.0036 M⊕ at 30 au | 25 µbar | 38.17 K | about 1.3 to 1.8 Pa | 0.044; 0.070 Pa | 0.44 to 0.70 µbar, 1.8 to 2.8 % | | |

Triton's 14 µbar is Voyager's (see *Moons* above); Pluto's is the fact sheet's
"~13 microbar" (read). The Raoult and detailed-balancing columns take an N₂
reservoir of 0.34 to 0.5 bar, the ice N₂ of study 28's Triton-like body (argon's
mole fraction 2e-5 to 3e-5). In every picture the frost takes almost all the
argon: the air holds at most 0.36 µbar of the 1.3 to 1.8 Pa made. New Horizons'
Alice saw no argon at Pluto: 0.14 R at 1048 Å (3σ) is 1.8e15 cm⁻² of argon above
the 480 km where methane turns the line opaque, but in a Pluto atmosphere of
small eddy mixing (Young et al. 2018) the line stays under that limit "even if
the density of argon and (molecular) nitrogen were equal at Pluto's surface"
(Steffl et al. 2020, *AJ* 159, 274, doi:10.3847/1538-3881/ab8d1c, § 5.2, read):
the limit does not bound the surface's argon. Triton's argon was never measured
(Zhang 2024, arXiv:2410.04595, a review: "the lack of ... data on noble gases
such as argon"); a Voyager ultraviolet limit was not found. Mousis et al. 2013
(*Icarus* 225, 856, doi:10.1016/j.icarus.2013.03.008, arXiv:1303.4595, read)
found with Fray & Schmitt's fits that pure argon ice condenses at about 39.5 K
on Pluto at its protosolar share of the air, and that on Triton, at 38 K,
condensation "would largely remove" the noble gases from the air: the same frost
the law gives.

**Escape.** Argon is too heavy for Jeans escape. Mars lost it to sputtering by
the ions of its own upper air, which takes the lighter ³⁶Ar more than ³⁸Ar:
Curiosity measured ³⁶Ar/³⁸Ar 4.2 ± 0.1 against about 5.3 to 5.5 for the
primordial ratio, which needs at least half and perhaps 85 to 95 % of the argon
lost (Atreya et al. 2013, *GRL* 40, 5605, doi:10.1002/2013GL057763, abstract);
MAVEN's ³⁸Ar/³⁶Ar above the homopause "require that 66% of the atmospheric argon
has been lost to space" (Jakosky et al. 2017, *Science* 355, 1408,
doi:10.1126/science.aai7721, abstract); Hutchins & Jakosky 1996 (*JGR* 101,
14933, doi:10.1029/96JE00860, abstract) found more than 85 % of the outgassed
³⁶Ar sputtered; Lichtenegger et al. 2022 (arXiv:2105.09789, read) model the
sputtering from the pick-up ions. The model's laws carry argon without a new
one: the shoreline takes the same share of every gas, and the non-thermal loss
of the moons (#658) weighs a gas by `m v_esc² / 2`, which counts argon's mass.
The shoreline takes too much of it on Mars: the reference Mars keeps about 0.5 %
of its air (`(2 × 0.4306 / 0.3578)^6` ≈ 194 against 1, derived from the law),
while its argon lost 66 % to 95 % of itself, and its ⁴⁰Ar came out after most of
the early escape (*Departures*).

**Checks on bodies.** The law above, run by hand on the reference bodies' inputs
at 4.5673 Gyr (silicate `(1 − f)(1 − w)(1 − cmf) M`, as `air.rs` builds it; the
ocean's and the shoreline's shares by hand from their laws, at the Sun's present
light, the planets without a dynamo), against the measured ⁴⁰Ar:

| Body | Silicate (kg) | ⁴⁰Ar made (mol) | Share, then ocean and shoreline | ⁴⁰Ar in the air (mol) | Measured (mol) | Model / measured |
| --- | --- | --- | --- | --- | --- | --- |
| Earth | 4.031e24 | 4.053e18 | 0.41 | 1.662e18 | 1.649e18 | 1.01 |
| Venus | 3.358e24 | 3.378e18 | 0.0863, shoreline about 0.96 | 2.8e17 | 4.03 ± 1.35e17 | 0.69 |
| Mars | 4.857e23 | 4.885e17 | 0.0863, shoreline about 0.005 | 2e14 | 1.11e16 | 0.02 |
| Titan | 5.912e22 | 5.947e16 | 0.0863, ocean 0.991, shoreline 0.9964 | 5.07e15 | 1.121e16 | 0.45 |

The measured column, all derived from the sources: Earth above. Venus holds
(1.61 ± 0.54)e16 kg of ⁴⁰Ar (Kaula 1999, *Icarus* 139, 32, as Warren & Kite
2023, *PNAS* 120, e2209751120, doi:10.1073/pnas.2209751120, quote it; read),
4.03e17 mol; Avice et al. 2022 (*Space Sci. Rev.* 218, 60,
doi:10.1007/s11214-022-00929-9, arXiv:2210.03176, Table 1, read) give ⁴⁰Ar 31
(+20/−10) ppm, ³⁶Ar 30 (+20/−10) ppm, ⁴⁰Ar/³⁶Ar 1.11 ± 0.02 and ³⁸Ar/³⁶Ar 0.183
± 0.003, which over the fact sheet's 4.8e20 kg of air at 43.45 g/mol is 3.4 (2.3
to 5.6)e17 mol; its 70 ppm of argon weighs 6.0 mbar. Mars's air is 2.5e16 kg at
43.49 g/mol (NASA Mars fact sheet, read), 1.93 % of it ⁴⁰Ar with ⁴⁰Ar/³⁶Ar 1.9 ±
0.3e3 (Mahaffy et al. 2013, *Science* 341, 263, doi:10.1126/science.1237966,
abstract), so 1.11e16 mol, 0.11 mbar. Titan's ⁴⁰Ar is 3.39 ± 0.12e-5 of its air
and its ³⁶Ar 2.1 ± 0.8e-7 (Niemann et al. 2010, eq. 10 and 7, read in the NASA
author copy), over 9.04e18 kg of air (1.467 bar over its 1.352 m/s² and 2574.7
km) at 27.34 g/mol (*Titan's nitrogen*), so 1.121e16 mol, 0.07 mbar; Niemann et
al. add that a rock like Earth's, as degassed, would have given about 0.05 %
(Owen 1982, as they quote it): Titan degassed less than Earth, as the lid's
share says.

Tolerances the tests can take from the sources: Earth within 10 % of 0.0127 bar
(Marty's ±0.9e18 on 3.9e18 is ±23 %, but the share is fitted on Earth's air, so
the test checks the chain, as the N₂ test does); Venus within a factor of 2 of
4.03e17 mol (Kaula's ±34 % and the spread of the potassium); Titan within a
factor of 3 of 1.12e16 mol (its rock's potassium is not Earth's, and its ocean
and shoreline are the model's); Mars as a departure, under its measured ⁴⁰Ar.

**In the chain.** `system::planet::air` outgasses the argon at step 1 of the
air: `radiogenic_argon` of the interior's mantle mass (the mass the heat's
potassium is in, a dry planet's mantle water included, as the heat takes it),
the drawn radiogenic abundance (`10^dex`) and the trial's age, times
`argon_degassed_share` (a lid for a stagnant or an episodic regime) and
`released_under_ocean`, weighed by `column_pressure`; the shoreline keeps its
share as of every gas, and no later step of the air changes it. Its node cites
the mantle mass, the abundance, the age, the regime, the water and the bulk's
gravity and radius. On the reference bodies (tests `earth_weighs_its_argon`,
`venus_titan_and_mars_hold_their_argon`): Earth 0.01294 bar (+1.9 % on 0.0127
bar), Venus 2.80e17 mol (0.70 of Kaula's), Titan 5.09e15 mol (0.45), Mars
2.08e14 mol (0.019, the departure below): the hand values of the table above
within 2 % (the chain's Earth has its own radius, gravity and mantle). The
climate's Triton-like body (`triton_like`, 1.8 Pa of argon, test
`a_triton_like_body_keeps_its_argon_as_frost`) settles at 37.97 K with 0.38
µbar of argon's partial pressure in its air (its frost pressure; 0.54 µbar by
weight, #749), 1.7 % of its moles, the rest frost.

**In the climate column.** Argon weighs: on Earth 0.0127 bar, 1.29 % of the air.
It joins the column's partial pressures as a gas of 39.95 g/mol that absorbs
nothing, counts as air in the broadening of the lines (measured, it broadens
CO₂'s and water's less than N₂ does, 0.78 of N₂ for CO₂ and about 0.37 of air
for water: a departure below), has the heat capacity of a monatomic gas,
20.786 J/(mol·K) = 5/2 R, and scatters light as `rayleigh.rs` already knows (see
[radiative transfer](radiative-transfer.md#which-gases-absorb-and-where), where
the values and their sources are; the code since study 29). On Earth at 280
ppm, from published numbers only: doubling N₂, about +0.78 bar, warms the late
Archean Earth by 4.4 °C in Goldblatt et al. 2009 and 3 to 7 K in the models
quoted under *Thick nitrogen*;
argon's 0.0127 bar, at most equally efficient, is +0.07 to +0.11 K by
proportion, and less with its weaker broadening (derived). Argon does not close
the gap between the reference Earth's 285.07 K and the 286 to 290 K window: it
is missing from the weight of the air, not from its climate.

The code, on Earth's measured air at 280 ppm in the wet climate of the settled
reference Earth (the air of `earth_has_its_climate`, release build, measured
2026-11-25 with a probe, not a test): its argon, 9340 ppm by volume of 1.014
bar, warms it by +0.125 K, from 287.050 to 287.175 K, a little above the +0.07
to +0.11 K estimated above. Added to the air without argon, 0.0127 bar of argon
warms it by +0.168 K and 0.0127 bar of N₂ by +0.150 K: argon warms 1.12 times as
much as N₂. Both add the same broadening, since the code counts argon as air,
and argon's lower heat capacity (20.786 against N₂'s 29.12 J/(mol·K)) steepens
the dry slope: with N₂'s heat capacity given to argon, the same probe gives
+0.120 K. The measured weaker broadening of argon, left out (a departure below),
would pull the other way; no source was read that orders the two effects. A
tier-1 test, `argon_warms_as_air_broadens_and_its_slope_steepens`
(`science::balance`), keeps the two parts apart on a wet column with fixed
inputs (Earth's measured air at 280 ppm, Earth's equilibrium starlight, a black
surface, the standard gravity; not the settled reference Earth), where it finds
the same +0.150, +0.120 and +0.167 K. It guards that, with argon given N₂'s heat
capacity (a test-only override of the column), argon warms, and at most as much
as N₂ (`Source:` argon broadens CO₂'s and water's lines at most as N₂ does,
Reichle 1969 and Claveau & Valentin 2009; equal here since the code counts it as
air); and that argon's own heat capacity warms it more than N₂'s (`Definition:`
the dry lapse is `g / c_p` per kilogram, and argon's 520 J/(kg·K) is half N₂'s
1040). A change that gave argon a broadening above N₂'s, or lost its heat
capacity, fails it. On that Earth the model's doubled N₂ (+0.78 bar) warms by
+7.49 K, above Goldblatt et al. 2009's 4.4 K and the 3 to 7 K quoted under
*Thick nitrogen* (late Archean Earths, under a fainter Sun and more CO₂, not
this one). Earth's dry column (`EARTH_DRY_AIR`, no water vapour, test
`earths_argon_warms_its_column_little`) warms by only 0.005 K with 0.0127 bar of
argon: without the water's feedback it is about 12 times less sensitive
(doubling its N₂ gives +0.37 K), so that test holds an upper bound only. The
reference Earth, held by the carbonate thermostat at 288 K, does not move.

**Contract.** The timeline's air is a map of the gases (`timeline::planet::Gas`,
`#[non_exhaustive]`, kebab-case) with `surface_pressure` the sum of the
partials. Argon enters it as `Ar` (`"ar"`): the sum of the partials stays the
surface pressure, and the map and the viewer, once they read the air, will find
Earth's and Mars's third gas. No crate outside `system` reads the air today, so
the change is additive. The planet chain is not wired into the system draw yet
(#659): the regression goldens do not move and `system::ENGINE_VERSION` stays;
the draw that wires the chain bumps it.

**Departures.**

- The law has no melting history: the share is Earth's, scaled for a lid by the
  outgassing's own ratio. A Venus or a Mars whose volcanism stopped early would
  hold less.
- An episodic lid degasses its argon as a stagnant lid and its other volatiles
  as plates: a choice drawn from Venus's ⁴⁰Ar, so the Venus test
  (`venus_titan_and_mars_hold_their_argon`) does not check it independently.
- The potassium is Earth's for every body, scaled with U and Th by one
  abundance; Venus's and Mars's K/U, measured by landers and orbiters, were not
  read.
- The shoreline takes the same share of argon as of the early air; on Mars it
  leaves 0.019 of the measured ⁴⁰Ar, where MAVEN's 66 % loss would leave about
  1.3 times it (derived: 0.0863 × 4.885e17 × 0.34 = 1.43e16 mol). The test
  holds Mars under its measured ⁴⁰Ar; a time-resolved escape is issue #699.
- Primordial ³⁶Ar and ³⁸Ar are left out: half of Venus's argon, 6 mbar in all;
  with the argon of the ices (study 28's gate), issue #698.
- Argon's frost takes activity 1, the most argon of the three pictures of
  a minor volatile over N₂ ice; Raoult's law would keep a few 1e-5 of it
  in the air. Under 50 K its curve rests on two data sets, and its two
  fits part by a factor 1.6 at 38 K; under 26 K it carries on the slope.
- The air's argon is a weight, 1.29 % of Earth's by weight; the column
  reads it at its share of the moles, 0.93 %, through
  `Weights::partial_pressures` (issue #749; it read the weight as a share
  of the moles before).
- The column counts argon as air in the lines' broadening (0.78 of N₂ for CO₂,
  about 0.37 of air for water, measured): 0.2 % and 0.6 % too much foreign
  broadening on Earth ([radiative transfer](radiative-transfer.md)).
- An argon frost is not on the ground's frost (`frost_cover`,
  `frost_emissivity`), and a liquid argon is not a surface liquid of the model.
- The published Venus share of 24 % (O'Rourke & Korenaga 2015, "assuming an
  Earth-like K abundance", as Warren & Kite 2023 quote it) is twice what the
  law's production needs (0.12); their potassium and mantle mass were not read.

## Climate and surface liquid

`planet::climate` (called by `planet_model`) gives the mean
surface temperature, the liquid on the surface (`SurfaceLiquid`: `none`,
`water`, `hydrocarbons`, `nitrogen` or `lava`), the emissivity its
surface radiated with (`Climate::emissivity`) and whether the carbonate thermostat
holds the temperature (`Climate::regulated`). It takes the gases after
`photochemistry` and returns them with the CO₂ the thermostat leaves and
the water vapour the climate sets.

**Radiative–convective column** (`surface_temperature_k`,
`planet::column`, issues #441 and #523). The surface temperature is the
one at which the outgoing longwave radiation (OLR) of a column over the
surface balances what it absorbs, `σ T_eq⁴ + q`, with `T_eq` the
equilibrium temperature under the body's derived Bond albedo
(`planet::albedo`, issue #510: its oceans, land, ice, frost, clouds,
Rayleigh scattering, methane haze and sulphuric deck, solved with the
temperature as a fixed point) and `q` the internal heat flux. How the
column computes its OLR (the adiabats, the skin temperature, the
correlated-k spectrum of CO₂ and water, their continua, the CO₂ pairs,
the cloud greenhouse, the falling humidity, the runaway convention and
the checks against Earth, the moist plateau, Mars and Venus) is on its
own page, [How does a planet's air set its outgoing heat and its
albedo?](radiative-transfer.md). The climate prose below relies on
its runaway convention: a wet surface runs away when what it absorbs
exceeds the most its saturated column sends out.

**Albedo of a water runaway** (`climate::absorbing`, issue #492). A body
in a water runaway (by the insolation limit or a boil) that keeps steam
in its air absorbs under
the Bond albedo that puts the column's moist plateau (`F_plateau`, about
287 W/m², the column's OLR over a saturated surface at 380 K, under the
stratosphere the surface solves give a body at that limit, skin
`T_eq / 2^¼` with `T_eq = (F_plateau / σ)^¼`) exactly at its star's
runaway limit `S_rg` (`star::habitable_fluxes`), clamped to 0 to 0.9:
`A_rg = 1 − 4 F_plateau / (S_rg S₀)`, with `S₀` the solar constant
behind `T_eq` (255 K at 1 S⊕ with albedo 0.3). `F_plateau` is the
higher of Earth's air's plateau and the body's own (issue #523,
`moist_plateau_of`: the asymptote of its dry air over a saturated
surface, its vapour capped by its water, on its gravity; in the
rewrite both under the body's own ozone, #761, see
[ozone.md](ozone.md)), so that a
runaway absorbs more than its own steam column can send out: on Earth's
plateau the super-Earth of issue #492 (1.7 g, 0.45 bar of CO₂, its own
plateau 299 W/m²) stayed at water's critical point, and now settles at
1399 K. The albedo moves from
the body's derived albedo `A_d` to `A_rg` with the steam column,
`A = A_d + (A_rg − A_d) (1 − exp(−p_steam / 1 bar))` (`STEAM_ALBEDO_BAR`,
a calibration: the moist atmospheres of Kopparapu et al. hold a fraction
of a bar to about one bar of vapour at the limit), so a thick column
takes `A_rg` whole and a runaway losing the last of its water comes back
to its derived albedo without a jump. The albedo's cause tree shows
that blend as a `steam` part: its weight `1 − exp(−p_steam / 1 bar)`,
with `A_rg` and a note naming the plateau that sets it (`moist_limit`,
Earth's air's; `moist_limit_own`, the body's own;
`climate::steam_albedo`). Kopparapu et al.
2013 (doi:10.1088/0004-637X/765/2/131) and 2014 place the runaway limit
where the light a moist atmosphere absorbs, with the albedo its own
spectrum gives under that star, meets its outgoing limit. With Earth's
0.3, a planet round a red star that runs away by that limit
(0.91 S⊕ for an M9V) would absorb under the plateau and sit on water's
critical point. With `A_rg` (0.08 for an M9V, 0.24 for the Sun) it
balances past the plateau. Wet bodies keep their derived albedo (Earth's
about 0.3), and so does a runaway whose escape took all its water
(Mercury, Venus), which has no moist air left.

**Methane's pairs** (study 36, issue #703). Methane's infrared is the
collision-induced absorption of its pairs with nitrogen and with itself,
HITRAN's `N2-CH4_2024.cia` (Finenko et al. 2022) and `CH4-CH4_2011.cia`
(Borysow & Frommhold 1987), with nitrogen's own N₂–N₂ pairs
(Chistikov et al. 2019), binned into the spectral column
(`spectrum::PAIRS`; see [Which gases does the infrared column still
miss?](longwave-absorbers.md)). They settle the sourced Titan at 92.30 K
against the measured 93.65 ± 0.25 K, without a fitted constant
(N₂–N₂ alone 83.95 K, with N₂–CH₄ 92.23 K, nothing 74.88 K); at 91.81 K
since its dry adiabat condenses N₂ from about 0.65 bar up to its skin
(issue #741, [radiative transfer](radiative-transfer.md), "N₂'s
pseudo-adiabat and its phase step"). They replace
the legacy's grey stand-in, `τ = k (u B)^½` over 0-650 cm⁻¹ with
`k` = 15.1 (`CH4_OPACITY`, calibrated on Titan's 93.65 K: 13.4 before
#646, 14.3 on the grey column, 7.75 before #563, 6.0 before #530, 3.3
before #443, 3.4 before #441) and `u` saturating at 0.01 bar
(`CH4_SATURATION_BAR`), retired on the user's decision of 2026-11-25.

Titan's real greenhouse is mostly collision-induced absorption by N₂,
CH₄ and H₂, which warms it by 21 K, while its haze cools it by 9 K
(McKay, Pollack & Courtin 1991, doi:10.1126/science.11538492, abstract
checked); the column holds the N₂ and CH₄ pairs, and the anti-greenhouse
(below) the cooling. Missing: N₂–H₂ pairs (H₂ is not a gas of the model,
issue #720), methane's lines, the haze's infrared, and the methane the
cold trap's ratio leaves out of the lower troposphere (issue #744,
below), which the tier-1 test allows 1.9 K for (`balance_tests.rs`,
`TITAN_MISSING_WARMING_K`; its floor is 93.65 − 0.25 − 1.9 = 91.50 K,
0.31 K under the column's 91.81 K; 1.4 K and 92.00 K until issue #741,
refitted by the user on 2026-11-27). The column's troposphere, 4 to
8 K colder than Huygens's aloft (issue #750), hides 1.2 to 2.3 K more of
the missing warming and would cool the column once fixed (study 39,
[Nitrogen phases](nitrogen-phases.md#titans-column-against-huygens)):
the bound is one-sided only for the missing absorbers.

**Methane** (issue #443). The legacy's grey `sqrt(p_CH₄ P)` law,
calibrated on Titan, was stretched 500-fold past it: a planet at an
equilibrium temperature of 114 K under 26 bar of methane (seed 182,
planet d) came out at 351 K with liquid water. Three rules bounded it;
since study 36 retired the grey law for HITRAN's pairs, the first two
bound the pairs' methane, and the third is gone:

- *Condensation at the surface.* Inside the climate's fixed point
  (`atmosphere::condense`, applied at each step to the temperature the
  last one found, from the black body's up to the coldest balance), the
  air keeps at most the vapour of a Titan sea, `x p_sat(T_s)`, or of
  methane ice, `p_sat(T_s)`, under the sea's freezing point
  (`surface::sea_freezes`; activity 1 over a pure solid, issue #530): `p_sat`
  the saturation pressure of pure methane
  (`atmosphere::methane_saturation_bar`: over its ice below its triple
  point, 90.7 K and 0.117 bar, over its liquid above with the mean latent
  heat joining the triple point to its normal boiling point, 111.67 K,
  and none past its critical point, 190.56 K; the legacy law, which the
  rewrite replaces with `science::volatiles`, see
  [radiative-transfer.md](radiative-transfer.md#carbon-dioxide-and-methane-saturation)) and `x` = 0.5 the methane
  share of a sea of methane, nitrogen and ethane (`SEA_METHANE_SHARE`,
  Raoult's law). The rest lies on the
  ground. The mechanism is general over the gases listed in
  `atmosphere::CONDENSING`.
- *Cold trap at the tropopause.* The methane that radiates in the column
  (`column::radiating_methane_bar`) has at most the mixing ratio that
  pure methane's saturation pressure at the skin temperature `T_eq / 2^¼`
  allows at a 0.1 bar tropopause (`TROPOPAUSE_BAR`: the common tropopause
  of thick atmospheres, Robinson & Catling 2014, doi:10.1038/ngeo2020):
  Titan's skin at 70 K lets about 3 % through. The column applies that
  ratio at every level, not only above the tropopause, so Titan's lower
  troposphere holds about 40 % of its sourced surface methane (a
  review's estimate, derived, issue #744), likely a large share of the
  1.35 K the pairs leave Titan under its measurement.
- *Saturation* (legacy, retired by study 36). The grey law counted
  `u = p / (1 + p / 0.01 bar)` (`CH4_SATURATION_BAR`), and the pressure
  it collides with counted the methane as `u`:
  `τ = 15.1 (u (P − p + u))^½`, so that raising Titan's methane from 1
  to 10 bar held back under a third of the outgoing radiation that 0.01
  to 0.1 bar held back. The pairs need no such bound: their absorption
  is HITRAN's at every amount, and grows as the product of the two
  gases' densities. On the legacy's seed 182 planet d (4.4 bar of N₂,
  26 bar of methane, a 114 K equilibrium) they give 113.48 K where the
  grey law gave 103.06 K (study 36), far from legacy issue #443's 351 K.
- *Anti-greenhouse* (issues #530, #563). The stratosphere, the methane
  above the tropopause in its near-infrared bands and the organic haze
  its photolysis makes, absorbs starlight and radiates it half back to
  space, so the surface never gets that half. `climate::absorbed_w_m2`
  takes the share `0.5 m / (m + 0.002 bar)` off the light the surface
  absorbs (`CH4_NEAR_IR_MAX`, `CH4_NEAR_IR_HALF_BAR`), `m` the methane
  above the 0.1 bar tropopause that the cold trap lets through
  (`column::stratospheric_methane_bar`); the internal heat is not
  touched, and the stratosphere that absorbs it is not warmed in the
  column. The maximum, 0.5, is the limit of a layer that absorbs all
  the light and radiates half of it down. The half-column is anchored
  on Titan (issue #563): McKay, Pollack & Courtin 1991 (*Science* 253,
  1118, abstract checked) find the anti-greenhouse cools its 94 K
  surface by 9 K, the greenhouse warming it by 21 K, so its surface
  keeps `(94 / 103)⁴` of the light under the same greenhouse, a share of
  0.306; their split of the light, 1.5 W/m² absorbed in the
  stratosphere of 2.6 W/m² (reproduced by Robinson & Catling 2012, ApJ
  757, 104, §4.3), half of it going up, gives 0.29. The reference
  Titan's cold trap lets through 0.0031 bar, which takes 0.304 (0.11
  with the calibration of issue #530, `0.3 m / (m + 0.005 bar)`). Since
  a warmer tropopause lets more methane through, a Titan nearer its
  star loses more. A world of 26 bar of methane takes nearly all of the
  0.5. One value serves every star, though redder stars put more of
  their light in methane's bands.

**The sea's methane share** (`SEA_METHANE_SHARE`, 0.5; study 24, part
3). One number stands for two things: the air's methane over a sea, as
a share of pure methane's saturation (its relative humidity), and the
sea's methane mole fraction (Raoult's law), the rest of the sea being
the nitrogen it dissolves and ethane. The first is measured: the
Huygens GCMS found a methane mole fraction of (5.65 ± 0.18) × 10⁻²
from 6.7 km to the surface (Niemann et al. 2010, *J. Geophys. Res.*
115, E12006, doi:10.1029/2010JE003659; abstract read: it supersedes
the 2005 results), under 1.467 bar at 93.65 K (Fulchignoni et al.
2005), so 0.0829 bar of methane against the 0.1689 bar of pure
methane's saturation there (`science::volatiles`), a relative humidity
of 0.49 ± 0.02 (derived); Lora & Ádámkovics 2017 (arXiv:1610.06277,
read) state that the GCMS "measured a near-surface relative humidity
of roughly 50% near the equator (Niemann et al., 2010)". The value is
`Source:` that measurement. The
second is a model's: the sea compositions published for an air of
about 6 % methane run from 0.32 (Tan, Kargel & Marion 2013, *Icarus*
222, 53, doi:10.1016/j.icarus.2012.10.032, 93.7 K; closed, as Glein &
Shock tabulate it) to 0.68 (Glein & Shock 2013, *Geochim. Cosmochim.
Acta* 115, 217, doi:10.1016/j.gca.2013.03.030, full text read, their
Table 4 at 90.69 K), and Glein & Shock put the polar seas under the
Huygens air at "62.4 % CH₄, 22.9 % C₂H₆, 12.4 % N₂, and 2.3 % C₃H₈"
(§ 5), 63 to 77 % if the lakes buffer the air. Half methane sits
inside that range; the model's Titan sea (half methane, 19 % N₂, 31 %
ethane, `surface::methane_freezes`) holds more ethane than Glein & Shock's.
Departure: Raoult's law ties the two numbers, whereas a non-ideal sea
(Glein & Shock's van Laar model) holds more methane than its air's
relative humidity; and the Huygens site is an equatorial plain, drier
than the polar seas.

Titan at Saturn's orbit (`titan-like`, seeds 1 to 40 of
`archetype_census`) comes out at 92.4 to 94.5 K (mean 93.69 K, median
93.75 K), 1.31 to 1.61 bar (median 1.48 bar, no seed under 1.3 bar
since `titan-like` pins `n2 = 0.82`, see [atmospheres.md](atmospheres.md))
and 5.5 to 5.9 % methane (median 5.7 %), against Titan's 93.7 K, 1.47 bar
and 5.65 % (95.1 K and 6.6 % on the spectral column before the opacity
was re-anchored; 91.0 to 93.3 K and 3.9 to 5.8 % before the anchor of
issue #563; 90.4 to 94.5 K and 3.7 to 6.8 % before the anti-greenhouse).
Seed 182's planet d of `planet_census` is a hydrocarbon world under the
150 K line (129 K on the grey column, 136 K before the anchor, 149 K
before the anti-greenhouse), with 0.9 bar of methane in its air (1.8 bar
on the grey column) and the rest of its methane in its seas.

**The moist plateau.** On a saturated column the levels that radiate to
space sit on water's saturation curve whatever the surface temperature,
so the OLR flattens: the Simpson–Nakajima limit (Nakajima, Hayashi & Abe
1992, doi:10.1175/1520-0469(1992)049<2256:ASOTGE>2.0.CO;2). Goldblatt,
Robinson, Zahnle & Crisp 2013 (doi:10.1038/ngeo1892, abstract checked)
find it at 282 W/m² line by line, Boukrouche, Lichtenberg & Pierrehumbert
2021 (doi:10.3847/1538-4357/ac1345, §III.1 read in arXiv 2107.14150) at
283 W/m² and Hamano et al. 2015 (doi:10.1088/0004-637X/806/2/216) at
280 W/m². Line-by-line curves rise to a maximum near 350-400 K and fall
a few per cent to the asymptote (Goldblatt et al. 2013, Fig. 2). Legacy's
spectral column does the same over Earth's air, saturated: 294.6 W/m² at
360 K, then 283.4 to 283.9 W/m² over 500-640 K (legacy's column; the
rewrite's plateau is 2.2 % higher and its own peak was not measured). `MOIST_PLATEAU_W_M2` is
that asymptote, 283.5 W/m² in legacy (the rewrite's plateau is 289.7 W/m²,
+2.2 %, see "The runaway branch" below), computed once on a finer grid (80 levels per
decade, converged to 0.5 W/m²); it moves by under 2 % with CO₂ up to
1e-2 bar and with one to three times Earth's nitrogen. The old Eddington core
(`T⁴ = T_eq⁴ (1 + ¾ τ) + q/σ`) had no plateau: its moist OLR peaked at
253 W/m² at Earth's composition and fell with nitrogen.

**Broadening.** The lines take the total pressure at each level from the
table, whose Voigt widths grow with it, CO₂'s and water's each broadened
by the air and by the gas itself in proportion to its share; the
continua grow as the product of the absorber and its partner, the CO₂
pairs as the CO₂ density squared. The knees and broadening shares of the
grey laws (`BROADENING_KNEE_BAR`, `LOCAL_KNEE_BAR`,
`WATER_FOREIGN_BROADENING`, issues #440 and #441) are gone with them.
Argon counts as air in the broadening, although it broadens less than N₂
(0.78 of N₂ for CO₂, about 0.37 of air for water, measured): a departure
(*Argon (study 29)* above). Methane's grey stand-in kept the whole
pressure, its own counted as the saturated amount `u` (see **Methane**
above), standing for Titan's collision-induced greenhouse; since study 36
the column holds that greenhouse as HITRAN's N₂–CH₄ and CH₄–CH₄ pairs,
which grow as the product of the two gases' densities.

**Thick nitrogen.** At 1 AU with modern CO₂ (4e-4 bar) and no
thermostat, an Earth keeps a moist balance at 300, 310 and 319 K under 2,
5 and 11 bar of N₂; on the old core 2 bar already ran away. The
thermostat holds an Earth under 2 or 3 bar of N₂ at 288 K with 1.3e-5
bar of CO₂, and an Earth with no CO₂ under its own albedo falls to
277 K, below 288 K (Lacis et al. 2010, doi:10.1126/science.1190653).

Doubled nitrogen warms the late Archean Earth by 4.4 °C in the clear-sky
1-D model of Goldblatt et al. 2009 (doi:10.1038/ngeo692), by 7 °C in
the GCM of Charnay et al. 2013 (doi:10.1002/jgrd.50808), and by 3 to 7 K
in Wolf & Toon 2014 (doi:10.1089/ast.2013.1112). In Charnay's setting
(0.82 of today's Sun, 10 mbar of CO₂, no thermostat, Earth's albedo 0.3
held fixed) the all-sky column goes from 275.4 K to 285.7 K, +10.3 K.
At a fixed 275 K surface the doubled nitrogen holds back 9.1 W/m², about
half of it the lines' wider Lorentz wings and half the colder moist
adiabat and the vapour it keeps; but the cloud cover grows from 0.39 at
275 K to 0.61 at 285 K, and with the albedo held fixed each new cloud
brings its longwave trapping without its reflection: half a cloud.
Goldblatt's 1-D model keeps its clouds in a fixed albedo, so the test
compares the two airs at a fixed cloud cover, that of the 1-bar
climate (0.40 at 275.4 K): +4.9 K, inside the 1-D range of 4 to 5 K
(the clear sky alone gives +4.3 K).

**Steam.** A steam runaway's gases are fixed, its surface no colder than
the saturation temperature of its steam. Pure steam follows the plateau
too: 10 bar of it leave the plateau at about 1100 K, where the dry
adiabat from the surface reaches the emitting levels (Boukrouche et al.
2021 find about 1050 K under 10 bar, Goldblatt 2015,
doi:10.1089/ast.2014.1268, a rise past 1500 K through the 4 µm window).
The column holds at most 70 bar of steam (`STEAM_MAX_BAR`), the steam
under it changing nothing. The cap was calibrated on the grey column,
which had no window: a deeper steam column left its plateau ever later
(about 2600 K under 300 bar, 20 000 K under the 1.6e5 bar of a 30 %-water
Earth), where Boukrouche et al. find the end of the plateau reaching a
constant of about 1800 K as the steam thickens to 295 bar; the cap put
it at 1800 K. The spectral column has the window, and keeps the cap so
that a runaway's 1.6e5 bar of steam stay inside its opacity table, which
ends at 100 bar.

**Water vapour.** Where water is kept and there is no runaway, the air
above the ocean or the ice holds 77 % of the saturation pressure
(`RELATIVE_HUMIDITY`, `Source:`; study 24, part 8): Earth's surface air
holds "a mean value of 75%–80% over most oceans in all seasons" (Dai
2006, *J. Climate* 19, 3589, doi:10.1175/JCLI3816.1; abstract read,
15 000 stations and ships, 1976-2005). 0.77 is the surface value of the
fixed relative humidity of Manabe & Wetherald 1967 (*J. Atmos. Sci.*
24, 241, doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2; read in
study 33 from the scan GFDL hosts: their eq. 2, the surface value fitted
to observed hemispheric means), as the codes that
implement their profile take it (konrad's `Manabe67`, `rh_surface=0.77`;
climlab's `FixedRelativeHumidity`, `relative_humidity=0.77`). How much
water that profile puts in Earth's column, and why one column holds
less than the observed 2.45 to 2.53 cm, is in
[radiative transfer](radiative-transfer.md), *How much water one column
holds* (study 33, issue #654). The
saturation is `water::vapour_saturation_bar`: the sublimation curve of
ice Ih below the triple point (273.16 K, 611.657 Pa; Wagner et al.
2011, IAPWS R14-08(2011), eq. 6), the saturation curve over liquid from
the triple point to the critical point (Wagner & Pruss 1993, IAPWS
SR1-86(1992), eq. 1), and the critical pressure, 22.064 MPa (IAPWS
SR1-86(1992)), above the critical temperature (647.096 K); and never
more than the body's whole water
would weigh as vapour (`m_water g / (4π R²)`): a trace of water cannot
put a hundred bar of steam in the air. The temperature and the vapour
are solved together (`wet_balance`): the coldest surface temperature
whose column, its vapour at that temperature, sends out what the surface
absorbs, scanned up from the black-body temperature in steps of at most
3 % to the critical point. With none, the absorbed light and heat exceed
the moist limit and the water runs away: the climate then puts all the
body's water in the air at the temperature of `surface_temperature_k`,
a stand-in, as in legacy (`climate::wet_climate`, l.1212): a body without a balance boils (`climate::wet::boils`), and the chain runs its climate again on its steam, so this air is never the one a boiled body keeps. A runaway's steam is already in the gases and is kept as it is.

**Carbonate thermostat.** On a planet with plates and water, weathering
draws CO₂ down faster when it is warm and volcanoes give it back (Walker,
Hays & Kasting 1981, doi:10.1029/JC086iC10p09776). The model sets the CO₂
so that a surface at `THERMOSTAT_K`, 288 K (a calibration on Earth), the
vapour taken at that temperature, sends out what it absorbs (solved in
`ln CO₂`; the rewrite weighs each CO₂ on its own absorbed light, see
"The absorbed light of the thermostat and the wet balance (issue #681)"
below). It can build the CO₂ up past what the air held before the
climate (what escape and the cold trap left of it), since weathering
slows and the volcanoes keep giving: up to the planet's carbon inventory
(all it outgassed, `CarbonCycle::inventory_bar`), at most its maximum
greenhouse (`maximum_greenhouse`, `Body::greenhouse_co2`; issue #522).
That maximum is the model's own, not a published edge: a golden-section
search in `ln CO₂` for the amount at which a surface at 288 K gains most,
the light it absorbs under the albedo that CO₂ gives against what the
spectral column sends out. Three things in the model cap it:

- CO₂'s bands saturate: the correlated-k lines, the MT_CKD far wings and
  the CO₂–CO₂ pairs close the 15 µm region, so each bar adds less
  trapping;
- where the CO₂ of a cold column reaches its saturation
  (`volatiles::co2_saturation_bar`, its frost below 216.592 K, its
  liquid above) it condenses, the air follows CO₂'s pseudo-adiabat
  (`column::co2_lapse`, Kasting 1991 Appendix B, over the condensate
  stable where each step starts, the step that crosses the triple point
  split there by `column::co2_step`) and the tropopause
  (`climate::tropopause_k`, the skin temperature `T_eff 2^-¼` of the
  climate's own albedo) cold-traps it;
- its Rayleigh scattering keeps growing with the column, averaged over
  the star's spectrum (the legacy's black body, `albedo::spectrum`; the
  rewrite's table of model spectra, `starlight::star_bands`), while its
  near-infrared absorption (`planet::shortwave`) takes back part of what
  it reflects. A red star's light scatters less and falls more in the
  absorption bands, so the maximum holds at a lower flux.

Kopparapu et al. 2013 (doi:10.1088/0004-637X/765/2/131) find the same
mechanism and put its edge at 1.70 AU round the Sun (0.343 S⊕). The model
lands closer in. Legacy's column put the outer edge of an Earth, the last
orbit with liquid water, at 1.535 AU (0.424 S⊕, +24 % over the 2013
fit), and round 4500 K and 3300 K stars 25 % and 20 % above it (1.47 AU
before the near-infrared absorption, 1.78 AU on the grey column). The
rewrite's solver measures the edge against Kopparapu et al. 2014's fit
(tier-3 test `outer_edge_against_kopparapu_2014`): 1.513 AU round the
Sun, +22.9 % in flux (0.4367 against 0.3555 S⊕) at 5772 K, +24.0 % at
4500 K and +19.9 % at 3300 K, falling with the star's temperature as the
fit does. The gap is a documented departure (#595), not a bound. Two
parts of legacy's gap were measured: the thick CO₂ column sends out too much, 73.5 W/m²
under 8 bar at 1.70 AU against Kopparapu's 67.4, so it needs less light
to stay warm; and the albedo at their maximum greenhouse is 0.455
against their 0.417. Neither is tuned away: legacy's tests pinned what its model reached
(1.535 ± 0.005 AU), the rewrite measures the gap instead, and issue #595 holds the search for the missing physics
(the CO₂ far-wing shape against MT_CKD, CO₂ ice clouds, the 0-D
column's stratosphere, the HITRAN versions). If the maximum is not
enough, the planet keeps the CO₂ of its maximum greenhouse and is
colder: below 273 K its water freezes, and where its surface is too
cold to keep that CO₂ it frosts out until the air keeps what the surface
allows (`collapsed_climate`). If no CO₂ at all is still too warm, the
planet is hotter. Only a temperature reached is `regulated`.
The grid's `carbon_cycle` is unchanged; part 3 aligns them.

**Bond albedo** (`surface::cover`, `radiation::albedo::shortwave_budget`, issue #510). The
albedo is no longer Earth's 0.3 for everyone: it follows what covers the
disc, and the climate and the albedo are solved together as a fixed point
(`Context::with_albedo` turns an albedo into the equilibrium temperature
the column absorbs at). The disc is split, planet-wide, into open ocean
(0.06), bare land (0.25 under an air, the regolith's 0.11 on an airless
body, shielded from space weathering by `1 - exp(-p / 1e-4 bar)`), water
ice and snow (0.6, from the ice line at -5 °C and the equator-to-pole
contrast of an Earth-like planet, `ice_line_share`, as far as the body's
water reaches as ice sheets: see "The ice reach" below), weighted by the
light it receives, not by its area (see "Ice weighted by sunlight"),
frosts of N₂, CH₄ and
CO₂ (0.75, the share `frost / 0.02 bar` of the disc, all frosts together)
and of volcanic SO₂ (0.63, its part of that share). What the air adds over
this ground (the water clouds, Rayleigh scattering, the absorption of the
star's light, the methane haze and the sulphuric acid deck) lives in
[radiative-transfer.md](radiative-transfer.md#the-shortwave-half-the-bond-albedo),
with its sources, calibrations and departures, and in
`science::radiation::albedo`. The surface cover above lives in
`science::surface`; the global climate that solves it with the temperature
is not ported yet.

**The surface cover's values** (study 24 of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/24-surface-cover-audit),
parts 4 to 7, for planet PR 2d-2). Each value, the source read and how
far:

- *Open ocean, 0.06* (`OCEAN_ALBEDO`, `Source:`). Payne 1972 (*J.
  Atmos. Sci.* 29, 959, doi:10.1175/1520-0469(1972)029<0959:AOTSS>2.0.CO;2;
  abstract read) measured "0.061±0.005 for heavily overcast skies …,
  indicating isotropic radiance distribution", the albedo averaged over
  every angle of the light, which a planet-wide cover takes; under a
  clear sky it runs from 0.03 at a high sun to 0.45 under 10°. Jin et
  al. 2004 (*Geophys. Res. Lett.* 31(22), doi:10.1029/2004GL021180;
  abstract read) give a look-up table, no single value.
- *Airless regolith, 0.11* (`REGOLITH_ALBEDO`, `Source:`, was a
  calibration). The Moon's Bond albedo, 0.11 (NASA NSSDC Moon fact
  sheet, read). Departure: Mercury's is 0.068 on its fact sheet (read;
  legacy remembered 0.09), so the model's Mercury is brighter by 0.04.
- *Earth's ocean share, 0.71* (`EARTH_OCEAN_COVER`, `Source:`).
  Charette & Smith 2010 (*Oceanography* 23(2), 112,
  doi:10.5670/oceanog.2010.51; full text read, Table 1) put the ocean
  at 361.84e6 km², 0.709 of Earth's 510.06e6 km² (`4π R²` with the
  volumetric mean radius 6371.0 km of the NASA Earth fact sheet;
  derived).
- *Ice sheet thickness, 1.94 km* (`ICE_SHEET_THICKNESS`, `Source:`, was 1.5 km
  from memory). The mean thickness of Antarctica's ice, shelves
  included, 1937 m (Fretwell et al. 2013, Bedmap2, *The Cryosphere* 7,
  375, doi:10.5194/tc-7-375-2013; full text read, Table 7; 2126 m
  without the shelves). Plaut et al. 2007 (*Science* 316, 92,
  doi:10.1126/science.1139672; abstract read) sound Mars's south polar
  layered deposits to more than 3.7 km, 1.6e6 km³ in all; their mean
  thickness and area were not reached. A thicker sheet covers less: a
  nearly dry body whitens about 16 % less of its disc than at 1.5 km:
  the thickness alone gives 23 % less, and the reach also changes
  density, from legacy's 1000 kg/m³ of liquid to 917 kg/m³ of ice Ih
  (`1500 · 1000 / (1940 · 917) = 0.84`, derived).
- *Pole-to-equator contrast, 42 K* (`ICE_LINE_CONTRAST`, `Source:`).
  North, Cahalan & Coakley 1981 (*Rev. Geophys. Space Phys.* 19, 91,
  doi:10.1029/RG019i001p00091; full text read) write Earth's annual
  mean temperature `T(x) = T₀ + T₂ P₂(x)` and take "the observed value
  for T₂ (−28.0°C)"; with `P₂ = (3x² − 1)/2` that is
  `T₀ − 42 K (x² − 1/3)` (derived). North 1975 (*J. Atmos. Sci.* 32,
  1301 and 2033) was not reached (the publisher's site refuses scripts);
  the review is by the same author. The disc colder than the ice line
  then follows by definition: `1 − x_s`, with
  `x_s² = 1/3 + (T_mean − T_ice) / 42 K` (`ice_line_share`,
  `Definition:`, the disc being uniform in `x`). The whole disc is under
  ice once `x_s` reaches 0, at a mean of `T_ice − 42 K / 3`, about 254 K
  (`ICE_COVERED_TEMPERATURE`, `Definition:`, where 2d-3's frozen branch
  starts).

**Calibration of the ice line** (`ICE_LINE_TEMPERATURE`, −5 °C, kept). Budyko
1969 (*Tellus* 21, 611, doi:10.3402/tellusa.v21i5.10109; full text
read) lets the ice cover extend over the area "with temperature equal
to or lower than the temperature observed now at 72°N", the mean
latitude of the Arctic ice edge; North et al. 1981 write that rule as
the annual-mean isotherm "T(x_s) = T_s = −10°C". With −10 °C and the 42
K contrast, Earth at 288 K keeps 3.8 % of its disc under ice (derived),
the permanent ice; −5 °C keeps 10.2 %, which also counts the seasonal
sea ice and snow the planet-wide albedo has to carry. Target: about a
tenth of Earth's surface, Antarctica's 13.92e6 km² with its shelves
(2.7 %, Fretwell et al. 2013, Table 7) and the 34e6 km² of ocean with
sea ice at some point of the year (6.7 %, the NSIDC sea ice page,
read: "about 9 percent" of the ocean). Searched and not found: a
single annual-mean share of Earth's disc under ice and snow, and an
energy-balance ice line fitted to it.

**Ice weighted by sunlight** (decision of 2026-11-24, study 24 of the
Wiki, planet PR 2d-2). A cap over the polar share `1 − x_s` of the area
receives less than that share of the light, so the ground albedo counts
the ice by the annual-mean insolation `S(x) = 1 + s₂ P₂(x)` (mean 1 over
the disc), not by area. The ice's sunlit share is then
`∫_{x_s}^{1} S dx = (1 − x_s) + (s₂/2)(x_s − x_s³)` (`Definition:`,
the integral of `P₂`; `sunlit_cap`). `Cover` keeps both: `ice`, by area,
which the map will read, and `ice_sunlit`, the share the ground albedo
mixes, times the reach and `(1 − frost)`. Counted by area, Earth's ground came out at 0.165
against the 0.122 of CERES under clear skies, its Bond albedo at 0.314
against 0.29 to 0.31. Counted by sunlight, Earth's ground is 0.1445
(derived, from the code's own cover: the cap receives 6.07 % of the
light) and its Bond albedo 0.304: the ground is still 0.023 over the
CERES value, the Bond albedo inside the 0.29 to 0.31 target.

- *The coefficient* (`Source:`). Nadeau & McGehee 2021 (*J. Math. Anal.
  Appl.*, 125129, doi:10.1016/j.jmaa.2021.125129; full text read in
  arXiv:1810.10081) expand the annual-mean insolation of a rapid
  rotator, normalised to a global mean of 1, as
  `s(x, β) = Σ A₂ₙ P₂ₙ(cos β) P₂ₙ(x)` (Theorem 1.1), with `A₂ = −5/8`
  (eq. 39 writes `1 − (5/8) P₂(cos β) P₂(x)`). So
  `s₂ = −(5/8) P₂(cos β)`: −0.625 at 0°, +0.3125 at 90°. Nadeau &
  McGehee 2017 (*Icarus* 291, 46, doi:10.1016/j.icarus.2017.01.040;
  full text read in arXiv:1510.04542) give the same first term. Both
  start from the integral of Ward 1974 and McGehee & Lehman 2012 (not
  read, cited by them).
- *Earth's numbers* (derived). At β = 23.44°, `s₂ = −0.477`; the −0.482
  of North's energy-balance models is the same term at β = 23°
  (Nadeau & McGehee 2021, § 6.1). At 288 K (`x_s = 0.898`) the cap
  covers 10.2 % of the area and receives 6.07 % of the light, 0.59 of
  its area share.
- *Truncation at `P₂`*. Nadeau & McGehee 2021 find the second degree
  "sufficient for Earth's obliquity", and the series slow to converge
  at the poles, without a figure for a polar cap. Integrating their
  eq. (1) numerically (derived): Earth's cap receives 5.82 % of the
  light, so `P₂` overstates it by 4.4 % (0.0026 of the disc); at β = 0°
  by 24 %, since `P₂` keeps 0.375 at the pole where the exact value is
  0; at 90° by 2 %. Searched and not found: a published error for a
  polar cap share.
- *Limits*. The shape holds for a spin period much
  shorter than the year; eccentricity only scales the magnitude, by
  `(1 − e²)^(−1/2)`, and leaves the distribution `s(x, β)` unchanged
  (Nadeau & McGehee 2017, eqs. 1 and 2). A slow or tidally locked
  rotator (Mercury and Venus count as slow) has no zonal annual mean
  and needs another weighting: a locked one uses `locked_sunlit_ice`
  (see "Tidally locked rotators"); a slow unlocked one is not handled. Between 47° and 63° of obliquity (117°
  to 133°) the insolation takes a "W" shape that the degree two misses;
  the degree six is the first to catch it.

**The ice reach** (`surface::cover`, decision of 2026-11-24). The water
inventory spreads as ice over the whole body, to the share
`inventory / (ρ_ice · 4π R² · ICE_SHEET_THICKNESS)`, capped at 1. The
ice is ice: `ρ_ice` is `water::ice_ih_density` at one standard
atmosphere (101 325 Pa, a `Definition:`) and at the melting temperature
of ice Ih at that pressure (`water::ice_ih_melting_temperature`, IAPWS),
about 917 kg/m³. Legacy spread liquid water at 1000 kg/m³ over Earth's
area scaled by `R⁻²`, a density with no source; the function now reads
the planet's radius in metres, so no Earth-area constant is needed and
legacy's `OCEAN_LAYER_KM` is not ported.

**Calibration of `ocean_fraction`** (kept). `1 − exp(−k oceans)` with
`k = −ln(1 − 0.71)`: the sea level of a body follows its hypsometry,
which the planet model does not draw. Searched and not found: a
published law from a water inventory to the share of a planet under
sea (study 24, part 7; Cowan & Abbot 2014 are 2d-4's study).

**Calibration of `SOIL_ALBEDO`** (0.25, kept): Mars's Bond albedo,
0.250 (NASA Mars fact sheet, read), a thin air over dust. Searched and
not found: a planet-wide broadband albedo of bare weathered soil;
Henderson-Sellers & Wilson 1983 (*Rev. Geophys.* 21, 1743,
doi:10.1029/RG021i008p01743; abstract read) give no value in their
abstract, and the full text is closed.

**Calibration of `ICE_ALBEDO`** (0.6, kept): the middle of "Sea ice
reflects 50 to 70 percent of the incoming energy" (NSIDC, quick facts
on sea ice, read), under snow: Perovich et al. 2002 (*J. Geophys. Res.*
107(C10), doi:10.1029/2000JC000438; abstract read) measure 0.8 to 0.9
on dry snow over sea ice, 0.65 on bare white ice and 0.4 on average
once melt ponds form; Warren 1982 (*Rev. Geophys.* 20, 67,
doi:10.1029/RG020i001p00067; abstract read) gives no broadband value
in its abstract. Searched and not found: a planet-wide broadband albedo
of a mix of sea ice and snow.

**Calibration of `FROST_ALBEDO`** (0.75, kept, flagged by study 24).
Fitted on Pluto, 0.72 ± 0.07, with Triton's gap stated in the doc
comment (below); Triton is not part of the fit. Legacy remembered Triton's
Bond albedo as about 0.75; Hillier et al. 1990 (*Science* 250, 419,
doi:10.1126/science.250.4979.419; abstract read) give 0.82 ± 0.05 in
the violet to green bands, and with it an emissivity of 0.59 ± 0.16 at
37.5 K. Pluto's is 0.72 ± 0.07, "preliminary", in LORRI's band (Buratti
et al. 2017, *Icarus* 287, 207, doi:10.1016/j.icarus.2016.11.012; read
in arXiv:1604.06129). 0.75 lies inside Pluto's range and 1.4 σ under
Triton's; Triton's temperature rests on the pair it forms with
`N2_ICE_EMISSIVITY`, 0.80 since 2d-3a (see "Calibration of
`FROST_ALBEDO` and the N₂ ice emissivity"). Searched and not found: a bolometric
Bond albedo of either body. Study 25 keeps 0.75 with a new target,
Triton's 38 ± 1 K under an N₂ emissivity of 0.80 (issue #673; see "The
climate solver of the rewrite" below).

**Calibration of `FROST_LAYER`** (0.02 bar, kept from legacy, not
studied in 2d-2). The frost that covers a whole disc, as a column of
gas: 0.02 bar is 2 000 Pa, 204 kg/m² under Earth's gravity (about 0.2 m
of ice, derived), 0.7 m of CO₂ frost under Io's gravity and 2.5 m of N₂
frost under Triton's (legacy's figures, checked by the same arithmetic).
Legacy set it on the CO₂ frost its Io then froze out of its air, 0.013
bar covering two thirds of the disc, a Bond albedo of 0.56 (a calibration
"on the moons, no source"); that air is gone and Io's frost is now the
SO₂ of its volcanoes, whose amount is calibrated against this layer (see
the SO₂ frost above). Triton's 0.36 bar of N₂ frost covers all of the
disc with any value under that, and Europa's 2e-4 bar of CO₂ frost is
left to its water ice, so neither constrains it. Not studied: study 24
did not search for a measure of a frost's coverage as a function of its
amount, and the legacy calibration is kept (open gap, listed in the
study).

**Calibration of `SHIELD_PRESSURE`** (1e-4 bar, kept). Searched and not
found: a surface pressure or column above which an air shields a
regolith from the solar wind and micrometeoroids; the space-weathering
studies a search returned (lunar sputtering, nanophase iron) treat
airless bodies only. Any value between the exospheres
(under 1e-12 bar) and Mars's 6 mbar gives the Moon, Mercury and Mars
their albedos.

**Issue #656: Titan's ground** (study 24, part 10). No broadband
ground albedo of Titan was found. The Huygens DISR measured the
landing site's reflectance spectrum: Tomasko et al. 2005 (not reached)
as smoothed by Bailey, Ahlsved & Meadows 2011 (arXiv:1102.2554, read),
"decreasing from about 0.2 at 0.8 µm to 0.08 at 1.7 µm"; Schröder &
Keller 2008 (*Planet. Space Sci.* 56, 753, doi:10.1016/j.pss.2007.10.011;
read in arXiv:1702.00653) find it "significantly higher" under the lamp,
which they put down to the opposition surge at under a degree of
phase. Disc-wide fits through the methane windows give 0.108 ± 0.009 at
1.6 µm and 0.096 in the 2 µm window (Bailey et al., Table 4 and § 7).
These are spectral and local; a broadband value would need the
spectrum of the light that reaches Titan's ground, which the model does
not compute. The model's Titan ground stays the soil's 0.25, above
every one of them, and the haze refit of #656 stays open: the cover now
exists (`surface::ground_albedo`), but it has no sourced Titan
value to take.

**The air's absorption of starlight** is described in
[radiative-transfer.md](radiative-transfer.md#the-shortwave-half-the-bond-albedo)
and ported in `science::radiation::{albedo, shortwave}`.

In legacy the outer edge moved from 1.47 to 1.535 AU round the Sun; the K
and M edges' fluxes from +43 % and +47 % to +25 % and +20 % of
Kopparapu's 2013 fit. The rewrite's own measure against the 2014 fit is
in "The outer edge" below; issue #595 holds the rest of the gap (see
**Carbonate thermostat** above).

**Icy albedo from surface age** (`albedo::icy_unit_albedo`, climate
pass lot 3C-1, goal 4). The exposed water ice of an ice crust darkens
with its age and its rock, and the body's Bond albedo is the
area-weighted mix of its terrain units. The crust is a stagnant-lid ice
crust read before the model exists (`planet::ice_crust`): the ice test
of `generation::model_crust_kind` (5 % of water or more, 0.01 oceans of
it kept) narrowed to a stagnant lid out of a runaway, the bodies whose
terrain units `surface_age::icy_estimate` gives. The ice test's third
condition, no liquid water at the surface, is the climate's: the
crust's albedo replaces only the cover's ice and land, never open
water, and only under an air thinner than `ERODING_BAR` (0.01 bar),
where nothing erodes or buries the ice. The units are those of
`surface_age::icy_estimate`, before the climate: an ice lid, and on a
member of a Laplace-like chain the unit its past tidal episode renewed.
Rocky planets with ice caps, snowballs (water under 5 % of the mass),
Titan (its haze, over 0.01 bar), Triton (its N₂ frost covers its disc)
and Io (a heat pipe, its SO₂ frost) keep their paths. Each unit:

- **Fresh ice** is an intimate mix of clean ice (`PURE_ICE_ALBEDO`
  0.85, calibration: brighter than Enceladus's 0.81) and rock dust at
  the regolith's 0.11, the dust `0.006 ρ` of its grains, `ρ` the body's
  rock-to-ice mass ratio (`1 − water_fraction` over `water_fraction`):
  a mostly-rock body's ice is dirtier (Europa's salts). The grains'
  single-scattering albedos mix and the mix reflects as a half-space of
  isotropic scatterers, `(1 − γ) / (1 + γ)` with `γ = √(1 − w)` (Hapke
  1981, JGR 86, 3039, from memory), taken as approximately its Bond
  albedo (Hapke's spherical albedo is slightly lower for dark grains);
  a little dust darkens ice much. `0.006` (`DUST_PER_ROCK_TO_ICE`) is a
calibration on Europa
  (`ρ` 9 to 16).
- **A dark lag** covers patches of it: a share
  `0.88 · min(1, rock / 0.5) · (1 − exp(−N / 0.004))` of the unit,
  `N` the craters of 1 km or more per km² of the lunar chronology on a
  surface that old (`craters::crater_density`, Neukum, Ivanov &
  Hartmann 2001), slow at today's flux and fast under the late heavy
  bombardment. Sublimation and impact gardening leave the rock of an old
  ice-rock crust as a lag and the ice as bright frost on cold crests
  (Spencer 1987; Moore et al. 1999, Callisto; from memory). `0.88`
  (`LAG_MAX_SHARE`) is a calibration on Callisto (about 0.2), whose
  rock share, 0.5 (`LAG_ROCK_SHARE`), is the reference under which the lag covers less in proportion (an old crust
  of 94 % ice keeps 0.77); `0.004` (`LAG_CRATERS_PER_KM2`) is a
  calibration on Ganymede (its
  2 Gyr renewed terrain keeps 70 % of its ice uncovered).
- **A plume's fallout** keeps `FALLOUT_SHARE × v` of the ice under
  fresh frost, `v` the strength the body vents with
  (`geysers::vent_strength`): 0 unless it passes the conditions of its
  geysers other than the crack, then rising linearly with its polar
  shell's overburden from 0 at the crack threshold to 1 at
  `FULL_VENT_OVERBURDEN` of it. The two constants are both 0.6 but
  unrelated. `FALLOUT_SHARE` (0.6) is a calibration on Enceladus, 0.81
  (Howett et al. 2010; the plume and E-ring fallout: Verbiscer et al.
  2007, Kempf et al. 2010; from memory). `FULL_VENT_OVERBURDEN` (0.6
  of the crack threshold) is not a calibration but a bound above the
  model's Enceladus band, whose polar overburden is 0.4 to 0.55 of the
  threshold over the census seeds: any value from 0.55 up keeps
  Enceladus venting in full on every seed, so that `FALLOUT_SHARE`
  applies to it whole; the value only sets how fast the vent fades on
  thicker shells, which no body constrains. The fallout brightens and cools the surface,
  which thickens the shell and weakens the vent: the strength the
  climate takes is the fixed point of that decreasing map, bisected in
  12 steps (`VENT_STEPS`, `planet::bare`), 14 climate solves
  per venting crust (one without fallout, 12 in the bisection, one at
  the strength taken). It is 0 exactly where the stored geysers
  stop, so over the 300 drawn systems of `drawn_icy_moons_stay_in_bounds`
  the climate takes a fallout on exactly the ice crusts that store
  geysers (36 of them).

**Calibration and validation.** The four bodies of the census below are
the calibration set: each of the four calibrated constants is set on
one of them, so their agreement with observation is by construction.
The validation is the five large moons of Uranus (`uranian_moons_are_dark`),
which were not looked at while setting them: the model gives Miranda
0.31, Ariel 0.18, Umbriel 0.18, Titania 0.19 and Oberon 0.19, against
about 0.1 to 0.25 observed (Ariel about 0.23, Umbriel 0.10, Titania 0.17,
Oberon 0.14; Veverka et al. 1991, Karkoschka 2001, from memory); the
test asserts 0.08 to 0.35. It is a weak validation, which separates
dark old crusts from bright young ones and no more: the per-moon values
miss. Miranda comes out 0.31 against about 0.2, Umbriel 0.18 against
0.10 and Oberon 0.19 against 0.14, while Ariel (0.18 against 0.23) is
too dark; the law has no term for their carbon-rich non-ice material,
nor for Umbriel's unusual darkness, and does not order them.

**How the card explains the albedo** (`explain::albedo_tree`, no
generation change). The albedo is its own tree, `bond_albedo`, on the
Surface card ("Bond albedo", with its "Why?"), and the same node sits under
the equilibrium temperature of the surface temperature's tree. Its
children are the parts of the cover the climate weighed, one
`cover_share` each with its share of the disc, largest first: ocean, bare
land, water ice and snow, the frost (SO₂ frost, with its own albedo, is a
part of its own; the other frosts weigh one albedo together, so they are
one part, named for the one lying on top by the layering of the
condensates, N₂ then CH₄ then CO₂, the most volatile present, and those
beneath it are omitted), or an ice
`crust` where the crust's albedo replaced the land and the ice, and a
sulphuric acid `deck`; the surface's parts and the deck sum to 1. The
water clouds (their cover) and the methane haze (its strength, which is
not a share of the disc, worded as such) are layers over them, listed
after. A part under 1 % of the disc is left out
(`COVER_SHARE_MIN`: it moves the albedo by less than a hundredth of the
gap between two covers' albedos). The cover is replayed from what the
climate used (its air, the water, the frost and the surface
temperature), not drawn again; a test checks that the albedo it weighs
(over the crust's, where it has one) is the stored one within 5e-3: the
climate's fixed point stops when its temperature moves under 0.1 K, so
the replay is exact only to that. Under the crust the tree names the terrain units whose
ages set its albedo and the plumes' fallout; under the SO₂ frost, the
note that the volcanoes lay it (Io). The node nested in the temperature
tree has no room under its parts (the trees stay three levels deep), so
it names them without those details. A body whose albedo is mostly a
frost says so on the card's value line ("0.63 · volcanic SO₂ frost").

Census, seeds 1 to 40 of `archetype_census` (min, median, max; climate
pass lot 3C-2):

| Body | Bond albedo | T, K | Observed |
|---|---|---|---|
| Enceladus | 0.789, 0.803, 0.812 | 58.7, 60.7, 62.5 | 0.81 ± 0.04 (Howett et al. 2010) |
| Europa | 0.586, 0.625, 0.660 | 91.9, 95.8, 100.1 | about 0.62 to 0.68 |
| Ganymede | 0.451, 0.454, 0.457 | 102.2, 105.1, 107.7 | about 0.4 |
| Callisto | 0.196, 0.204, 0.250 | 110.8, 115.1, 118.4 | about 0.2 |

Before lot 3C-2 Enceladus came out at 0.782 (62.2 K) and Callisto at
0.203; the other moons moved by under 0.002. Enceladus once took the
regolith's 0.11 (85 to 89 K): its 0.027
oceans spread as 1.5 km sheets cover 5 % of its disc, the rest counting
as bare rock; the Galilean moons took the ice's 0.6 (95 to 98 K). The
colder Enceladus has a thicker shell (45 to 54 km, 10 to 13 km at the
poles, still venting on every seed), the warmer Callisto a thinner one
(155 to 240 km over 0 to 99 km of ocean).

Where it departs:

- The lag follows the lunar chronology, whose flux falls steeply until
  3.5 Ga; the outer satellites are cratered mostly by comets, whose flux
  falls more slowly (Zahnle et al. 2003,
  doi:10.1016/S0019-1035(03)00048-4), so the model darkens an outer
  moon's 1 to 3 Gyr surfaces less than a comet chronology would.
- The lag has no temperature term: Spencer's thermal segregation needs a
  warm dayside (Callisto's), and would not work at Uranus's 60 K.
- Saturn's mid-sized moons (Tethys, Dione, Rhea, 0.6 to 0.7, from
  memory) are coated by the E ring, which only the venting body's own
  fallout stands for here: a Dione-like moon (half rock, old) comes out
  near Callisto's 0.2.
- The albedo jumps between the crust's and the cover's ice and land (0.6
  and soil) as the air crosses `ERODING_BAR` (0.01 bar) in the fixed
  point.
- An ice crust with plates or episodic overturn (162 bodies over the 300
  drawn systems) keeps the ice albedo of 0.6.

**Emissivity** (`climate::emissivity`, `N2_ICE_EMISSIVITY`, issue #529).
The surface radiates as a black body except under nitrogen frost, which
radiates with an emissivity of `N2_ICE_EMISSIVITY` (0.80, `frost_emissivity`) over the share of the disc its own
amount covers (`frost / 0.02 bar` of N₂ alone: the most volatile frost
condenses last and lies over the others, as Triton's N₂ lies over its
CO₂). A grey surface sends out `ε σ T⁴`: `surface_temperature_k` and the
wet balance take the column's outgoing radiation times `ε`, as through a
transparent air. That held over a nitrogen frost (its vapour is at most
0.125 bar) while the air absorbed nothing there; since the N₂ pairs of
study 36 it absorbs (6 % to 16 % of `σ T⁴` near the triple point, study
38), and `ε × OLR` scales the air's own emission by `ε` too and leaves
out the frost's reflection of the air's downward emission, `(1 − ε)` of
it: about 1 to 3 % of the outgoing radiation over the frost (a review's
estimate, derived, not verified; issue #745). The emissivity is solved in the same fixed point as the
albedo, damped the same way, and stored with the climate so that it
reproduces (worlds stored before it read 1). The legacy's 0.89 was a calibration on
Triton (above); Stansberry, Pisano & Yelle 1996 (*Planet. Space Sci.*
44, 945, doi:10.1016/0032-0633(96)00001-3, abstract checked) compute
0.85 for β-N₂ of 1 cm grains and 0.40 for 1 mm grains (α-N₂ 0.30 and
0.11), and Bertrand et al. 2022 (*Icarus* 373, 114764) model Triton
with 0.8: the calibration sits just above the coarse-grained end, a
lower value warmed legacy's model of Triton, under its frost albedo of
0.75, past 38 K (the rewrite's pair, 0.75 and 0.80, gives 37.97 K without
internal heat, see below). One value serves α- and β-N₂. The rewrite takes 0.80 and
refits the pair on Triton's temperature (study 25; see "The climate
solver of the rewrite" below).

**Phase changes and convergence** (`Climate::converged`,
`climate::PHASE_BAND_K`, issue #562). Two phase changes were steps
inside the fixed points: nitrogen frost (emissivity 0.80; 0.89 in
legacy) becomes a
black-body liquid at its triple point, 63.14 K, and methane's air halves
when its ice melts into a sea (`surface::sea_freezes`). A surface just under
such a point warmed over it and one just over it cooled under it, so the
fixed point had no solution there and cycled until its step limit,
silently. Both now pass from one phase to the other over the last 1 K
under the point (`frozen_share`): the frost's cover retreats and
methane's activity falls from 1 to 0.5 across that band. The band is a
numerical smoothing, not physics; 1 K is the narrowest over which the
damped fixed point settles at the triple point (over a band `w` the
emissivity's loop gain is about `1.8 K / w`). That holds only while the
emissivity's rise outweighs the albedo's fall across the band, over a
ground brighter than about 0.69 for N₂ frost (study 38): over a darker
ground, as the Titan-like body of the nitrogen sweep (water ice, 0.6),
the albedo wins (about `3.3 K / w` of radiative temperature, derived)
and the band holds an unstable balance between the frost and the
liquid, not a solution at the triple point. The condensation's fixed
point solves its temperature in the bracket its last two steps make
when they overshoot, and the albedo's fixed point halves its damping
when a step turns back. Each of its steps goes as far as the secant
through its last two steps puts the fixed point, between half the way
and all the way to the albedo derived (issue #593: about half the steps
of a fixed damping of 0.5, and never further than the undamped step). A climate whose fixed points still do not
settle is stored with `converged: false` and the card says so under its
temperature (worlds stored before read `true`). `cargo run --release
-p mapgen --example climate_sweep phase` sweeps a Titan-like and a
Triton-like body across both points and prints each flux, temperature,
step count and whether it settled. The methane-like sweep still shows
two steps between settled climates, from about 62 to 74 K and over about
1.2 K just above the sea's freezing point: methane darkening the body
(its near-infrared absorption) makes it bistable there, whatever the
phase change.

The rewrite's climate does not smooth that step away: across nitrogen's
triple point its sweep jumps from the frost to the liquid. It is a
bistability, as the runaway edge's step is (#523): the nitrogen frost
(Bond albedo 0.752, emissivity 0.80) and the liquid over the darker
ground under it (0.616, 1.00) are both stable over a range of light, and
the warm start picks the branch. Study 38 ([Can a nitrogen world be
frozen or melted under the same light?](nitrogen-phases.md)) checked it
against the literature and the model's own balance: a bright N₂ frost
and a liquid that shows the darker ground under it are both stable
under the same light (Charnay et al. 2014,
doi:10.1016/j.icarus.2014.07.009), over 0.005918 to 0.007041 Earth
fluxes for this body with the N₂ pairs of #703 (0.006818 to 0.008028
while its air was transparent). The sweep's 0.24 % jump before the
pairs (59.77 K at flux 0.006861 to 63.15 K at 0.006878) was where the
warm start landed, not the width of the bistability, 18 % of flux. The
melting band between the two branches, where `PHASE_BAND` ramps the
frost's cover, is the unstable middle branch of an S curve, not a third
state. Legacy's 1 K band has no source (study
25).

Before #703's first pull request the solver could stop on that
unstable branch: the
bracket's Illinois steps (#729) find a zero of the push, and its stop
test did not ask whether the zero attracts; it also settled one
residual, `(1 − A) / ε`, with two numbers, so the albedo and emissivity
it stored were not the derived ones (0.6552 and 0.9582 against 0.6690
and 0.9197 at 62.739 K, study 38). On #703's branch the warm-start
sweep then jumped twice, 59.31 to 62.78 K and 62.78 to 64.50 K (flux
0.006236 to 0.006252 to 0.006269). Since that pull request `settle`
keeps only a
root that attracts (the push taken again at a radiative equilibrium
`ALBEDO_TOLERANCE_K` colder must fall from there to the root, the
condition the bracket asks of its ends; a root the plain step reached
with its push falling needs no probe), leaves an unstable
one from its colder side (the side #729 stores a jump on), and settles
only once the albedo's and the emissivity's own residuals are within
the tolerance too. The sweep jumps once, 59.361 K (frost) at 0.006252
to 64.503 K (liquid) at 0.006269; Earth, Mars, Titan and the
Triton-like body settle where they did, to the printed digit. The
tier-3 test `the_fixed_point_converges_until_the_nitrogen_frost_melts`
checks that the absorbed light rises across the jump as the column's
own `ε OLR` does (×1.5520 against ×1.5518): `ε T⁴` (×1.7424) was the
premise of a transparent air, which the pairs end (the column sends out
6 % to 16 % less than `σ T⁴`, 0.9432 and 0.8417 of it, study 38). The tier-3 test
`a_nitrogen_world_keeps_its_frost_and_its_liquid_over_a_range_of_light`
follows both branches: the frost holds up to 0.0070506 Earth fluxes and
the liquid down to 0.0059258, within three steps of the folds study 38
measured, every climate converged, on its derived pair, and attracting.
Since issue #741 the column follows N₂'s saturation curve wherever its
air saturates in N₂, from the ground up over N₂ frost or liquid, instead
of a dry adiabat (study 38; it reaches Titan's upper troposphere too,
study 39): over the ground it sends out
2 % to 7 % less than `σ T⁴` (0.9817 and 0.9278 of it at 59.25 and
64.4 K), the sweep jumps once from 59.470 K at 0.006565 to 63.526 K at
0.006581 (absorbed and `ε OLR` both ×1.5614), and the branches hold up
to 0.0076953 and down to 0.0064470 Earth fluxes, about 0.1 % from the
0.007688 and 0.006449 study 38 estimated
([Nitrogen phases](nitrogen-phases.md)).

**Branches and snowballs** (`climate::climate`, `snowball_chance`). A body
that keeps water has two climates when the ice-albedo feedback makes it
bistable: the temperate one, started from a warm planet with no ice, and
the frozen one, started from a body covered in ice with the CO₂ the
temperate one held when it froze. Weathering stops under the ice, so
volcanic CO₂ builds up in the frozen air until it thaws the planet (the
snowball Earth of Hoffman et al. 2017, doi:10.1126/sciadv.1600983); the
frozen branch's CO₂ is what its thermostat reaches: the most that leaves
it frozen, just short of the CO₂ that thaws it (`still_frozen_bar`), or
its maximum greenhouse where none thaws it, less what its cold surface
frosts out (issue #522). An icy start that cycles (no fixed point, see
"A climate with no fixed point" below) holds no frozen balance: it
counts as thawed, and the thaw search stops at the first one (see "A
thaw search that meets a cycle" below).
Where both exist the planet is a snowball (`Climate::snowball`) when its
formation draw `PlanetInputs::snowball_draw`, uniform in `[0, 1]`, is under
`d / (d + W v / v⊕)`, with `W` = `SNOWBALL_WARM_BAR` (13.2 bar by
weight in the rewrite, 0.8 bar in legacy): the share of its history it spends frozen,
with `d` the CO₂ that thaws it and `v / v⊕` its volcanism against Earth's
(legacy's 0.8 bar gave Earth, which it remembered frozen about a tenth
of its history, a chance of 0.1: on the spectral column Earth at 1 AU
needed 0.091 bar more CO₂ to thaw, a chance of 0.102; 0.031 bar on the
grey column. Study 25 found the dated snowballs are 1.4 % of Earth's
history, not a tenth; the rewrite refits `W` to 13.2 bar and Earth's
chance is about 0.014, see "Calibration of `SNOWBALL_WARM_BAR`" below). Where no
CO₂ up to its maximum greenhouse thaws it (the chance is then 1, from
about 1.4 AU for an Earth-like planet round the Sun: an icy body's cold column condenses and scatters its CO₂ before it
warms the ice to a thaw), the frozen state is for good: the planet is
frozen but not a snowball (`ClimateState::Frozen` in the rewrite; the
`frozen` note in legacy). So snowballs are rare near S = 1, likelier towards the
outer edge of the habitable zone and on planets that outgas little, and
`--snowball yes|no` pins the state. A snowball is drawn with the `frozen`
look and keeps its relief and coastlines; no sulphuric cloud forms in its
cold air. On the sweep of an Earth-like planet round the Sun (`climate_sweep
sweep`) the warm branch runs from the runaway at about 0.95 AU to the
outer edge at 1.535 AU in legacy (288 K, the thermostat's, with 0.47 bar
of CO₂ at 1.2 AU, 2.8 bar at 1.4 AU and 5.7 bar at 1.5 AU; the rewrite's
edge is at 1.513 AU), beyond which the frozen
branch is the only one (165 K at 1.6 AU, its CO₂ frosted out down to
0.06 bar; 152 K at 1.8 AU).

**Sulphur and SO₂.** Outgassed sulphur stays in the air as SO₂ only in
share `SULPHUR_IN_AIR` (5e-5); over liquid water or with free oxygen above
0.01 bar it is washed out to sulphate in about a day and a half
(`WET_SO2_SHARE`, 2.2e-9 of the dry lifetime), and in a dry, oxygen-free air
its lifetime grows with the column (`DRY_COLUMN_BAR`, 10 bar). That gives
Earth under 1e-6 bar of SO₂ (2e-11), Venus 0.0151 bar by weight, a
partial pressure of 0.0103 bar, 115 ppm of its air by volume (inside the
130 ± 50 ppmv below the clouds; the test reads the partial pressure, and
the wash-out reads the O₂'s, since #749) and Mars under its 0.3 ppb
limit (Krasnopolsky 2012, doi:10.1016/j.icarus.2011.10.019: the upper limit
itself was not reached). The constants are calibrations on
Venus and Mars together. The wet lifetime of 1 to 2 days (1 to 3 days in a
review of Kīlauea's sulphur chemistry, as quoted; Seinfeld & Pandis not
reached) sets the numerator; the dry lifetime of Venus's SO₂ is 1.9 Myr in
Fegley & Prinn 1989 (their abstract: the reaction with calcite removes all of
it "in 1.9 Myr unless volcanism replenishes" it), not the 1 Myr the legacy
remembered, and ten times that in Bullock & Grinspoon 2001
(doi:10.1006/icar.2000.6570, as quoted). `WET_SO2_SHARE` is 2.2e-9, 1.5 days over the 1.9 Myr read in Fegley & Prinn
(a calibration; with Bullock & Grinspoon's ten times longer lifetime it would
be 2e-10, a factor-10 disagreement of the sources that no target
discriminates); `OXIDISING_O2_BAR` (0.01 bar, a twentieth of Earth's O₂)
is a calibration with no source found ([study
21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit)).
The rewrite ports the wash-out to `science::outgassing::so2_kept`, the share
kept (`OXIDISING_O2`, `DRY_COLUMN`).

**Surface liquid**, from the phase diagrams at the surface temperature
and pressure, in this order:

- `lava` from 1673 K in the rewrite (`LAVA_TEMPERATURE`, `Source:`; 1400 K in
  legacy; study 24, part 9), on a body that has kept no water. Kite,
  Fegley, Schaefer & Gaidos 2016 (*ApJ* 828, 80,
  doi:10.3847/0004-637X/828/2/80; full text read in arXiv:1606.06740)
  count the planets with "substellar temperatures hot enough to melt
  peridotite rock" as those with `T_ss` > 1673 K (their footnote 1),
  and take 1673 K for the edge of a magma pool, where the melt locks up:
  "Katz et al. (2003) recommends a parameterization that yields 1673 K
  for T_lo" on KLB-1 peridotite (their appendix). Legacy's 1400 K, "the
  basalt solidus", is not in the paper: it is the low end of the range
  of their vapour-pressure fit ("1400K < T < 2800K", their eq. 12).
  Kite et al. set 1673 K on the substellar temperature; the model
  compares it with the mean surface temperature, as legacy did, which is
  colder than the substellar one and so counts fewer lava worlds (issue
  #675). A body
  with water on top is never a lava world: Hamano, Abe & Genda 2013
  (*Nature* 497, 607, doi:10.1038/nature12163; abstract read, PubMed)
  find that a planet inside a critical distance keeps "a magma ocean …
  for longer, even with a larger initial amount of water", under its
  steam, until escape dries it; the model shows that planet's steam, not
  its magma;
- `water` between the melting temperature of the stable ice
  (`water::melting_temperature`) and water's boiling point at the
  surface pressure (`water::boiling_temperature`, IAPWS, 373.1 K at
  1 atm, the boiling point meeting the critical point), above the
  triple-point pressure, with water kept out of a runaway; legacy's
  273.15 K (`ICE_MELTS_K`, uncited) and its Clausius–Clapeyron boiling
  point are not ported, and a pressure under the triple point has no
  boiling point, so no liquid water. The melting curve is that of the
  ice in equilibrium with the liquid (issue #671, study 25): IAPWS
  R14-08(2011) Table 1 gives Ih to 208.566 MPa, III to 350.1 MPa, V to
  632.4 MPa, VI to 2216 MPa. Ice III is
  `π = 1 − 0.299948 (1 − θ⁶⁰)` (eq. 2, `T*` 251.165 K, `p*` 208.566 MPa,
  to 256.164 K) and ice V `π = 1 − 1.18721 (1 − θ⁸)` (eq. 3, `T*`
  256.164 K, `p*` 350.1 MPa, to 273.31 K), 3 % in pressure, checked
  against its Table 3 (268.685 MPa at 254 K, 479.640 MPa at 265 K);
  eqs. 2 to 4 invert in closed form, `θ = (1 − (1 − π)/a)^(1/b)`.
  Limit: ice VII (eq. 5) is not ported, a choice: above 2216 MPa the
  melting temperature is held at 355 K, the ice VI - ice VII - liquid
  point;
- `hydrocarbons` between the freezing point of the sea
  (`surface::sea_freezes`) and ethane's
  boiling point at the surface pressure (`volatiles::ethane_boiling_temperature`,
  Clausius–Clapeyron from 184.6 K
  at 1 atm, NIST's average of 23 values, ± 0.6 K; with 14.7 kJ/mol, the
  14.703 kJ/mol of Witt & Kemp 1937 at 184.1 K; capped at its critical
  point, 305.3 ± 0.3 K, NIST's average of 41 values: all read on the
  NIST Chemistry WebBook's ethane phase-change page, study 24, part 9),
  with ethane's triple point at 90.4 K (Lemmon et al. 2013, as
  Richardson, Hartwig & Leachman 2019 quote it, checked in legacy; the
  NIST WebBook's fusion entry gives 90.341 K, Atake & Chihara 1976), the
  enthalpy of melting of the ethane solid a cold liquid freezes into,
  2.79 kJ/mol at 89.5 K (Domalski & Hearing 1996, through NIST; the
  fusion alone is 0.583 kJ/mol, Atake & Chihara 1976) and methane's
  enthalpy of fusion, 0.9392 kJ/mol (Vogt & Pitzer 1976, as NIST gives
  it): none of these papers was re-read here, the values are those of
  legacy and of the NIST pages study 24 read,
  where there is enough methane to condense:
  the air's methane reaches the vapour of a Titan sea at the surface
  temperature, half of pure methane's saturation pressure
  (`surface::methane_sea_vapour`), so that a trace of methane makes no lake
  and none forms past methane's critical point (190.56 K). The sea
  freezes at methane's freezing point lowered by the nitrogen of the air
  dissolved in it and the ethane methane's photolysis makes
  (`surface::methane_freezes`: about 76 K with no N₂, about 69 K under Titan's
  1.45 bar, see [atmospheres.md](atmospheres.md)); where
  nitrogen itself condenses (`condense`), the air holds pure nitrogen's
  saturation and the sea it condenses into is nitrogen-rich, liquid down
  to nitrogen's triple point, 63.14 K, where its methane freezes with it
  (issue #524). That is a simple rule: an ideal solution would stay
  liquid colder (the Schröder–van Laar equations meet at about 48 K and
  two-thirds nitrogen, with nitrogen's enthalpy of fusion, 0.72 kJ/mol,
  from memory), but N₂ and CH₄ freeze into solid solutions an ideal
  liquidus does not describe (not checked), so the rule errs towards
  freezing;
- `nitrogen` where nitrogen condenses above its triple point (63.14 K)
  with no methane liquid: under its boiling point at its partial
  pressure and its critical point, 126.19 K (`condense`); under its
  triple point it is a frost (`surface::frost`). The look draws no
  nitrogen sea: the map of such a body is its icy surface;
- `none` otherwise.

The laws live in `science::surface::liquids` (`liquid`, `frost`,
`melting`, `frozen_share`). The liquidus of methane and nitrogen
(`hydrocarbon_freezes`) is solved in closed form, which is legacy's own
method, not a departure (the plan asked for a bisection): `ln x +
W(1 − x)²/T = −L (1/T − 1/T_m)` is linear in `1/T`. Three departures
from legacy: L/R is taken from the exact gas constant
(`MOLAR_GAS_CONSTANT`, under 0.01 % from legacy's 8.314); the
condensate (the part of N₂ and CH₄ the air cannot hold) is an argument
of `liquid` and `frost`, since `condense` is 2d-3's, so Titan's seas
need the condensate to be given; and the methane triple point is
`volatiles`' 90.6941 K, not legacy's 90.67 K. Richardson et al. 2019's
two freezing points (71.5 K and 74.0 K) hold to 2 K (71.51 K and 74.51
K).

**The nitrogen vapour curves** (`volatiles::nitrogen_frost_pressure`,
`nitrogen_saturation_pressure`; study 24, plan 2d-3a). The frost is the
N₂-1 / N₂-2 polynomial fit of Fray & Schmitt 2009 as Lisse et al. 2021
give it in their Table 2, pinned to the triple point (63.14 K, 12 530
Pa) and switching phase at 35.61 K (Scott 1976, via Tegler et al.
2019), with a 10 K floor (`Definition:`); it gives Triton's 37.2 K a
pressure of 1.40e-5 bar against the 1.4e-5 bar observed. The liquid,
from 63.14 K to the critical point (126.192 K, 3.3958 MPa), is the
ancillary vapour-pressure equation of the nitrogen reference equation
of state (Span et al. 2000, eq. 4, from Nowak et al. 1997b,
`ln(p/p_c) = (T_c/T)(N₁θ + N₂θ^1.5 + N₃θ^2.5 + N₄θ^5)`, `θ = 1 − T/T_c`),
read in the paper. The paper says it represents the measured vapour
pressures "well within their uncertainties" (typically under 0.015 %,
0.075 % at the triple point); against the paper's saturation table it
is within 0.021 % from 63.151 to 120 K (the largest, +0.0201 %, is at 70 K), normal boiling point (77.355 K,
101 325 Pa) included. It replaces the Antoine fit of Edejer & Thodos
1967, 12.5 % under the triple-point pressure, which would have made
`condense` step there; at methane's triple point, the one value
`methane_freezes` reads, the change is +0.09 % (3.8235 bar against
3.8200).

The two branches meet with a step: at 63.14 K the liquid curve gives
12 496 Pa, 0.27 % under the frost's 12 530 Pa (derived). The cause is
the triple point. The frost is pinned at Angus et al. 1979's (63.14 K,
12 530 Pa); Span et al. adopt Nowak et al. 1997b's ITS-90 measurement,
63.151 ± 0.003 K and 12.523 ± 0.010 kPa, and their curve passes there.
The pressures differ by 0.056 %, but the temperatures by 0.011 K, over
which the liquid curve falls 0.20 %; the curve also misses its own
triple pressure by 0.010 %. The switch stays at Angus's temperature,
where the frost fit is pinned, by decision (2026-11-24); the step is a
known departure, and `nitrogen_saturation_joins_its_frost` holds it
under the disagreement of the two sourced triple points (in pressure
and along the curve between the two temperatures, 0.268 %). Just under 63.14 K (about the last hundredth of a kelvin,
derived from the frost's slope) the frost is above the liquid's value
at 63.14 K, so the curve is not monotonic there; read on a 0.1 K grid
it rises. The SO₂ frost
(`sulphur_dioxide_frost_pressure`, Wagman 1979 via Giles 2024, anchored
at Giauque & Stephenson 1938's 197.64 K and 1 670 Pa) is 10 % under
Wagman's value at 115 K.

**Volcanism for the snowball chance** (`planet::volcanism`, option B of
2026-11-24). The intensity the chance reads is
`volcanic_intensity(heat_flux)` of `science::relief::volcanoes`, times
`LID_SHARE / MOBILE_SHARE` (`science::outgassing`, 0.2 / 0.95) under a
stagnant lid, 1 otherwise. It adds no number, and legacy's
`interim::volcanism` is not ported. The bare chain records it as
`SURFACE_VOLCANISM` right after the regime: Earth 0.506, Mars 0.079.
2d-3 divides it by `EARTH_VOLCANIC_INTENSITY` and refits
`SNOWBALL_WARM_BAR`, the 0.8 bar above. A heat pipe has no special case
(legacy set it to 1): the regime classes one from 1 W/m²
(`tectonics::HEAT_PIPE_ONSET_FLUX`, the regime's onset) and the law
reaches 1 at 2.5 W/m² (`volcanoes::FULL_VOLCANISM_FLUX`, Io's flux,
where volcanism is total), so a heat pipe between the two reads 0.83 to
1 (0.90 at 1.5 W/m²). The two fluxes mean different things and were
renamed in 2d-3a, the law unchanged (issue #670; they were both named
`HEAT_PIPE_FLUX`).

**Where the code lives.** `science::surface` holds, in its `cover` module, the cover and
the ground albedo: `cover` (takes the obliquity), `ground_albedo`,
`ice_line_share`, `sunlit_cap`, `ocean_fraction`, `frost_cover`,
`surface_vapour` and the public `RELATIVE_HUMIDITY`, which replaces the
test-only `MANABE_HUMIDITY` of `radiation`. `science::surface::condense`
holds the condensation of nitrogen and methane at the surface temperature.
`science::balance` holds the radiative balance of one column, its root
finders, the moist plateau and the runaway albedo, and `science::snowball`
the warm bar and the chance. The solver itself is
`system::planet::climate`, run by the chain step `planet::settled`.
`science::surface::liquids`
holds the surface liquid and the frosts. The cloud cover stays out of
`Cover`: the caller builds `Underneath { ground, clouds, deck }` from
`ground_albedo`, `radiation::clouds::cloud_cover` and
`veils::sulphuric_deck`. `system::planet::volcanism` holds the
volcanic intensity.

Calibration targets of the rewrite's climate solver (2d-3a), on the
reference bodies round the Sun at 4.6 Gyr; Earth, Mars and Titan on
their measured air (see "Test air and Solar System bounds" below):

| Body | Result | Target | Observed |
|---|---|---|---|
| Earth | 288 K, water, regulated, thermostat CO₂ 0.010605 bar by weight, a partial pressure of 6.819e-3 bar, Bond albedo 0.3066 (no O₂, so no ozone); on the measured air, with its 300 DU of ozone in the light and in the infrared (#704), Bond albedo 0.2907 and 289.38 K at 280 ppm (before #749, which keeps the air in weights: 6.910e-3 bar read as a partial pressure, 0.3064, 0.2906 and 289.39 K; before #704 and the clouds' infrared emissivity of #705: 4.071e-3 bar, 0.3071, 0.2914 and 288.18 K; before the N₂ and O₂ pairs of #703: 4.206e-3 bar and 288.02 K; on `main` after #655, before #705: 2.058e-3 bar, 0.2961 and 287.27 K under the reference Earth's albedo) | 288 ± 6 K, water, regulated; 286-290 K at 280 ppm, and the thermostat's CO₂ between the CO₂ that gives 286 K and 290 K | 288 K, 280 ppm before industry |
| Venus | 577 K, none | Bond albedo 0.76 ± 0.05 and no liquid (tier 1); the 577 K pin, the model's output (#594), moves to the tier-2 net in 2d-4 | 737 K |
| Mars | 214.06 K (T_eq 209.90 K), Bond albedo 0.2487, no liquid, on the imposed 6.36 mbar air | 214 ± 5 K, none | about 214 K |
| Titan | 92.36 K on the measured air, hydrocarbons; the reference Titan settles at 92.39 K on the chain's own air (92.33 K and 92.62 K before #749, whose chain methane, 2.11 bar by weight, the column read as a partial pressure; 92.64 K and 92.97 K before N₂ condensed in the column, #741; 93.86 K and 94.24 K under methane's grey stand-in, before #703; 104.8 K before #679) | 94 ± 8 K, hydrocarbons | 94 K, methane and ethane lakes |
| Mercury | its equilibrium temperature, dry, temperate | within 1 K of it | airless |

The reference Titan settled at 104.8 K because of its upstream nitrogen
inventory (2.876 bar against 1.384 bar measured, +11.2 K; #679), so the
climate is tested on Titan's measured air, not on the reference trial.
Since #679 (*Titan's nitrogen*) the chain gives it 1.422 bar of N₂ and it
settled at 94.24 K, 1.512 bar, with 0.091 bar of CH₄ over its seas under
methane's grey stand-in; since its greenhouse is its pairs only (#703)
it settled at 92.97 K, and since N₂ condenses in its column (#741) at
92.61 K; 2d-4 moves the test onto the chain's own air.
Earth's O₂ comes from life, which the chain adds later: its reference air
has none, and the climate is solved again after oxygenation in 2d-4
(#659), so the 280 ppm check runs on Earth's measured air (N₂ 78.08 %,
O₂ 20.95 % of 1.014 bar), under that air's own Bond albedo with the
ozone its O₂ holds (`ozone_column`, [ozone.md](ozone.md)), taken at
the reference Earth's temperature. Since #705 the cloud albedo is
fitted with that ozone in (0.322), so the reference Earth without O₂
is brighter (0.3070; 0.3071 since the N₂ and O₂ pairs of #703, the
heavy tests' 0.30703 and 0.30708) and its thermostat holds 288 K with
4.071e-3 bar of CO₂, about twice the 2.058e-3 bar it held before #705
(4.206e-3 bar before the pairs). Since the clouds' infrared emissivity
was fitted on CERES (0.848, #705's longwave part, with #704's gases)
its clouds trap less: it holds 288 K with 6.910e-3 bar (Bond albedo
0.3064), and the measured air settles at 289.39 K at 280 ppm. Since the
climate keeps its air in weights (#749) it holds 288 K with 0.010605 bar
of CO₂ by weight, 6.819e-3 bar of partial pressure (Bond albedo
0.3066), and the measured air settles at 289.38 K. Dry bodies are never a runaway: Mercury
and a dry Venus are `Temperate`.

The wet column against CERES (#680, study 32): on Earth's measured air
at 288 K and 280 ppm the clear column sends out 274.72 W/m² on `main`
(274.63 since the phase step of the moist adiabat was split), which
looked 7 W/m² brighter than CERES's 267.9 W/m². The +7 W/m² is not an
excess of the column: the comparison is not like for like. The column
holds water vapour and CO₂ only; the ozone, methane, N₂O and
halocarbons it lacks account for 8 to 14 W/m², and like for like it
should send out 275.5 to 282.9 W/m², 0.85 W/m² above what it does (see
[Earth's longwave against CERES (study 32)](#earths-longwave-against-ceres-study-32)).
Its clouds take out 33.38 W/m² at 288 K since the split (32.91 on
`main`), 5.4 to 7.7 W/m² more than CERES's 25.7 to 28.0 W/m², and
legacy's took out 34.0 W/m². The
reference Earth, whose air lacks oxygen, runs 283.84 K at 280 ppm (+2.7 K
from the missing O₂, +0.5 K from the nitrogen, +1.4 to 2 K from the
IAPWS saturation curve of the column); its cold bias comes mostly from
its Bond albedo (the shortwave), not from the longwave.

Legacy's results, for history: Earth 288 K with 1.2e-4 bar of CO₂ and
289.5 K at 280 ppm (the thermostat needed a factor 2.3 less CO₂ than
Earth has, from the clouds' longwave effect of 34 W/m² against 28 W/m²
measured by CERES), Mars 214.5 K under `MARS_CO2_FACTOR`'s 6.1 mbar
against an observed 210 K it took as target (the NASA fact sheet gives
about 214 K, see study 25), Titan 99 K, `titan-like` 93.7 K. Legacy's
`mars-like` reached 215 K on the distance sweep and 213.3 to 214.1 K
over seeds 1 to 40, and `venus-like` 552 to 601 K (median 575 K) under
75 to 98 bar. Venus reaches 577 K under its 85 bar of CO₂ and its Bond
albedo 0.76 (739 K on the grey column, whose CO₂ opacity was calibrated
on it): the infrared opacity of its H₂SO₄ clouds and its SO₂ are not
modelled, and the hot bands of the line list above 700 K are unchecked
(issue #594; the spectral column before the CO₂ continuum gave 451 K).
A uniform 1-D surface cannot reach Mars's mean temperature below its
equilibrium temperature: any greenhouse puts it over (the real surface
radiates more than `σ T_mean⁴` for its uneven temperatures).

### The climate solver of the rewrite (study 25)

What PR 2d-3a ports, with the values study 25 of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/25-climate-solver-audit)
fixed before the code (2026-11-24). The prose above describes legacy;
where they differ, this sub-section holds for the rewrite.

#### The outer edge

`science::habitable` gains the maximum-greenhouse fit of Kopparapu et
al. 2014 (ApJL 787, L29, arXiv:1404.5292v2, read; the table of their
conclusions, the same for every planetary mass): `S_eff⊙` 0.356,
`a` 6.171e-5, `b` 1.698e-9, `c` −3.198e-12, `d` −5.575e-16, from the
same paper as `runaway_flux` (`Source:` each). Round the Sun it puts
the edge at 1.676 AU (derived). Kopparapu et al. 2013 have two other
sets: their arXiv v2 (0.3438, 5.8942e-5, 1.6558e-9, −3.0045e-12,
−5.2983e-16; 1.70 AU in their § III.3), which legacy's tests used, and
their erratum (ApJ 770, 82, Table 3: 0.3507, 5.9578e-5, 1.6707e-9,
−3.0058e-12, −5.1925e-16; "should be 1.67 AU, instead of 1.70 AU").
The albedo at the maximum greenhouse round the Sun, 0.417, exists only
as their Fig. 5b (planetary albedo against CO₂, a 273 K surface,
minimum near 8 bar), digitised by legacy. Decided with the user on
2026-11-24: the edge is tested against the 2014 fit in a tier-3 heavy
test, and the measured gap is a documented departure, not a bound
frozen on the code's output; legacy's pre-erratum bounds (the 0.42
albedo, 1.535 AU) are not ported as bounds. Legacy's spectral column
put the edge at 1.535 AU, +19.2 % in flux over the 2014 fit round the
Sun, +21.0 % at 4500 K and +16.4 % at 3300 K (derived; +23 %, +25 % and
+20 % over the 2013 v2 set); the rewrite's gap is measured by that
test, and the departure and the search for its cause stay in #595.

#### The maximum greenhouse on a condensing air (#593 lever 2)

The carbonate thermostat searches the maximum greenhouse once per body,
on the inventory air, and keeps it whatever air the condensation loop
then holds (`Numerics::greenhouse_once`, on by default). Legacy searched
it again on each air (`climate.rs` l.1975-1983 rebuilds the body with
the condensed gases and a fresh cache): with the numerics off, the
rewrite does the same (`ClimateBody::greenhouse_co2(air)`, tier-3 test
`one_greenhouse_search_on_a_condensing_air`). The departure is measured
on the reference Earth under Titan's measured 2.11 bar of methane, at
Titan's 9.57 AU, where the methane condenses: the two searches give
14.2 and 52.0 bar (11.0 bar on the condensed air since CO₂'s phase step
was split, radiative transfer page, *CO₂'s phase step*), yet the settled climate is the same, 88.419440 K both
ways (a gap of 2e-11 K), frozen, and the outer edge is 1.4326 AU both
ways (a gap under 0.0001 AU; 1.6465 AU before issue #681, see below;
1.4443 AU once the stellar spectra of #730 came in, 1.4355 AU since
#729, below, and 1.4365 AU since #734). Since methane's greenhouse is its
pairs only (#703) the edge was 1.2588 AU both ways, and 1.2534 AU since
CO₂'s phase step was split (#746), 1.2720 AU since #704's lines and
#705's cloud emissivity; at 9.57 AU the climate settles at
80.52 K (a gap of 3e-12 K; 80.402 K since N₂ condenses, #741), the two
searches giving 23.67 and 23.85 bar (21.87 and 21.84 bar before #746;
21.06 and 23.85 bar since #704 and #705).
No regulated body can have a condensing air: the thermostat holds
288 K, above the critical points of methane
(190.564 K, Setzmann & Wagner 1991) and nitrogen (126.192 K, Span et al.
2000), so the condensation loop's last pass is solved on the inventory
air. A condensed air reaches the thermostat only in the loop's
intermediate steps and where the thermostat fails, far out and frozen,
where the CO₂ collapses to what the frost keeps whatever the cap it
starts from.

#### The absorbed light of the thermostat and the wet balance (issue #681)

The light a surface absorbs depends on its air: the methane
anti-greenhouse (above, `science::balance::absorbed`) takes a share
`0.5 m / (m + h)` of it, `m` the methane above the tropopause, which
falls as the rest of the air dilutes the methane. Legacy took that light
once per solve, on the air it was given, while the solve then varied
the air: the thermostat varies its CO₂ (`climate.rs` l.2279), the wet
balance its vapour (`wet_balance`). The rewrite weighs each candidate
on its own air, with the same function `surface_temperature` reads, so
that the air returned balances its own light:

- `carbon::thermostat`: each CO₂ tried, both ends of the search
  included, is weighed on its own air (the vapour at `THERMOSTAT_K`).
  Legacy weighed them all against the light of the air before the
  thermostat, whose CO₂ is far more (52 bar for the reference Earth,
  distinct from the carbon inventory `CarbonCycle::inventory_bar` it
  is capped by) and dilutes the methane more.
- `wet::wet_balance`: each surface temperature tried is weighed on the
  air with its own vapour. The scan starts from the grey body of the
  dry air, which absorbs least, so that it stays a lower bound.

Without methane the share is nil, the light does not depend on the air,
and nothing moves: Earth's own air gives the same climates to the last
digit (288.000012 K and 2.114e-3 bar of CO₂ at 1 AU).

Measured on the reference Earth under Titan's 2.11 bar of methane
(`solve_with`, warm draw): legacy's thermostat balanced 288 K against
155.7 W/m² absorbed on the 52 bar air, while the regulated air (0.31 bar
of CO₂) absorbed 126.8 W/m² (the diagnosis of issue #681); the surface
then fell to the floor of the surface solver, the dew point of its
vapour, 284.0103 K, at 1, 1.3 and 1.6 AU. Now it sits at 288.0000 K at 1
AU (3.94 bar of CO₂) and 288.0004 K at 1.3 AU (7.73 bar), and is frozen
at 1.6 AU. Its outer edge moves in from 1.6470 to 1.4331 AU (1.4355 AU
since #730 and #729, 1.4365 AU since #734, 1.2588 AU since #703,
1.2534 AU since #746, 1.2720 AU since #704 and #705). The old
edge came from the top end of the search: from about 1.43 to 1.65 AU
even the most CO₂ sends out more from a 288 K surface than the air with
that CO₂ absorbs (at 1.6 AU under a 0.3 albedo, 57.6 against 51.0 W/m²),
so the thermostat should give way to the collapse, but the light of the
52 bar air (58.4 W/m²) still exceeded it and the body was called
regulated; the dew-point floor then hid how cold its regulated air
really was, and kept it wet.

The wet balance's error grows with the vapour and the methane: under
0.05 bar of methane at 0.7 AU (a test input) the balance moves from
296.071 to 296.546 K (22.2 mbar of vapour); with Titan's methane and
1 bar of CO₂ at 0.6 AU, from 325.918 to 326.391 K (0.11 bar of vapour);
on the 52 bar air with Titan's methane at 1 AU, from 436.853 to
438.218 K (5.4 bar of vapour). Before, the air returned sent out 0.93,
0.21 and 2.08 W/m² less than it absorbed at the temperature returned.
Tier-1 tests: `the_thermostat_holds_a_methane_earth_at_its_target`,
`a_wet_methane_balance_is_its_own_airs`.

Limitation: `balance::surface_root` still returns its lower bound (the
grey body or the dew point) without saying so when the column already
sends out more there; a regulated surface off `THERMOSTAT_K` would go
unnoticed (issue #706).

#### Condensation at the surface

`condense` keeps in the air at most the weight whose partial pressure
is `a · p_sat(T_s)` for each of the three gases that condense,
nitrogen, methane and argon, and puts the rest of its weight on the
ground: `p_sat` the pure gas's saturation pressure at the surface
temperature (`science::volatiles`), `a` the activity, 1 for nitrogen,
argon and methane ice, `SEA_METHANE_SHARE` over a sea, ramped over
`PHASE_BAND`. The weight is the closed form of *Weights and partial
pressures* (`Weights::weight_at`) over the other gases of the air the
climate solved with (`condensables`, issue #749); since each gas that
leaves raises the others' shares, the three are capped in turn, in
rounds until no weight moves by more than `CONDENSE_SETTLED` (1e-12 of
it), at most `CONDENSE_ROUNDS` (50, a guard). A `Definition:` (phase
equilibrium and Raoult's law, on partial pressures); its one number is
`SEA_METHANE_SHARE`. CO₂ is lowered to its saturation by the
collapse, and NH₃ is frozen by the cold trap at the equilibrium
temperature only, as in legacy (kept by the user's decision of
2026-11-24: no NH₃ liquid curve is in `science`). Titan's methane, a
partial pressure of 0.0829 ± 0.0026 bar, sits 0.6 σ under the cap at
93.65 K, 0.0845 bar of partial pressure (derived): a Titan whose
inventory exceeds the cap gets seas and an air of 0.0845 bar of
methane's partial pressure, inside the measurement, which is about
0.050 bar of methane by weight over its 1.418 bar of N₂ (0.0496,
`weight_at`, derived); the tests give Titan an inventory, not its
air, and the surface liquid reads the condensate `condense` returns.

#### Calibration of `FROST_ALBEDO` and the N₂ ice emissivity (issue #673)

An isothermal N₂ frost in vapour equilibrium balances
`ε σ T⁴ = (1 − A) F / 4`, which constrains `(1 − A)/ε` only (Meza et
al. 2019, arXiv:1903.02315, § 4, read: the occultations "provide us
with a constraint on (1− A_N2)/ϵ_N2"). Triton's N₂ ice is at 38 ± 1 K
(Tryka et al. 1993, *Science* 261, 751, and 1994, *Icarus* 112, 513;
abstract records), and at its orbit-mean flux (1.508 W/m², NASA Neptune
fact sheet, read) 38 K takes `(1 − A)/ε` = 0.314 (derived). Meza et
al. fix "the emissivity at a relatively high value ϵ_N2 = 0.8 (Lellouch
et al. 2011)" for large grains; Stansberry, Pisano & Yelle 1996
(abstract excerpts) give pure β-N₂ 0.85 at 1 cm grains and 0.40 at
1 mm. So `N2_ICE_EMISSIVITY` becomes 0.80 (`Source:` Lellouch et al.
2011 as Meza et al. adopt it), and `FROST_ALBEDO` stays 0.75, now a
`Calibration:` on Triton's 38 ± 1 K given that emissivity (0.749,
derived): Triton at 37.97 K without its internal heat, against 36.97 K
for legacy's pair (0.75, 0.89). Hillier et al.'s albedos, 0.82 ± 0.05
(1990, *Science* 250, 419) and the bolometric 0.85 ± 0.05 with an
emissivity of 0.46 ± 0.16 at 37.7 K (1991, *J. Geophys. Res.* 96,
doi:10.1029/91JA01736; abstract records), are missed by 1.4 to 2 σ:
Pluto wins the albedo (0.72 ± 0.07, Buratti et al. 2017), Triton the
temperature. Pluto with the pair sits at 36.3 K under its 2015 flux
(1.26 W/m², Meza et al.), against their 37.0 to 37.3 K and Tryka et
al. 1994's 40 ± 2 K. The pair was chosen with the user on
2026-11-24. Searched and not found: a bolometric albedo and
an emissivity of Triton measured apart from this balance.

#### The runaway branch

- `A_rg = 1 − 4 F/(S_rg S₀)` is a `Definition:`, with `S₀` the
  rewrite's `SOLAR_IRRADIANCE` (1361 W/m², IAU 2015 B3); legacy's
  `REFERENCE_ALBEDO` (0.3, which rebuilt `S₀` from Earth's 255 K as
  1369.9 W/m²) is not ported. Round the Sun `A_rg` is about 0.23 on the
  rewrite's plateau, `balance::earth_moist_plateau` (289.7 W/m²; study 25
  derived 0.247 on legacy's 283.5 W/m²). These albedos follow from the
  plateau and are not test targets: the tests hold the binding identity
  and the plateau against Goldblatt et al. 2013 (282 ± 14 W/m²).
- `RUNAWAY_ALBEDO` (0, 0.9) is a guard (`Definition:`): over the fit's
  2600 to 7200 K, `A_rg` runs from about 0.07 to 0.34 on the 289.7 W/m²
  plateau (derived; 0.087 to 0.357 on legacy's 283.5 W/m²).
- **Departure from legacy:** the rewrite's moist plateau, 289.7 W/m², is
  2.2 % above legacy's 283.5 W/m²; both sit inside Goldblatt et al.
  2013's band of 282 ± 14 W/m² (5 %).
- **Calibration of `STEAM_ALBEDO_BAR`** (1 bar, kept). Kopparapu et al.
  2013 reach the moist limit "at a surface temperature of 340 K" and
  the albedo's minimum "at a surface temperature of 400 K" (§ III.1);
  water's saturation there is 0.27188 bar and 2.4577 bar (NIST
  Chemistry WebBook, read): the blend's e-folding sits between.
  Searched and not found: the vapour column at the runaway limit as a
  number, in Kopparapu et al. 2013 and 2014 and Goldblatt et al. 2013.
- `RUNAWAY_PROBE_K` (380 K) is a shortcut that changes no result, a
  `Definition:` near legacy's column maximum (294.6 W/m² at 360 K; the
  rewrite's own peak was not measured, and no result depends on it);
  legacy's citation of Goldblatt et al. 2013 "Fig. 1" was wrong (that
  figure shows spectra; the OLR curves are their Figs. 2 and 4).
- **Calibration of `THERMOSTAT_K`** (288 K, kept): Earth's "Average
  temperature: 288 K" (NASA Earth fact sheet, read). Searched and not
  found: a law for the temperature at which weathering regulates
  another planet.

#### Calibration of `SNOWBALL_WARM_BAR` (refit)

Legacy's target, Earth "about a tenth" of its history frozen, is not
Hoffman et al. 2017's (*Sci. Adv.* 3, e1600983, full text read): "two
long-lived Cryogenian (58 and ≥5 My) glaciations" (their Table 1:
Sturtian 717.5 to 716.3 Ma until 659.3 to 658.5 Ma; Marinoan 649.9 to
639.0 Ma until 636.0 to 634.7 Ma), and "at least one such episode in
the early Paleoproterozoic era", undated in length. The target is the
dated share, `f⊕` = 63 My over 4.6 Gyr, 0.014 (0.013 to 0.016 from
Table 1; derived). With the rewrite's volcanism (Earth 0.506 over
`EARTH_VOLCANIC_INTENSITY` 0.5), `W = d⊕ (1 − f⊕) / (1.012 f⊕)`, `d⊕`
the extra CO₂ the reference Earth at 1 AU needs to thaw, computed by
the code; with legacy's 0.091 bar, about 6.4 bar (derived), eight
times 0.8 bar. Decided with the user on 2026-11-24: the dated episodes
only, so snowballs become about seven times rarer near 1 AU than in
legacy. Refitted in 2d-3a on the rewrite's solver: the reference Earth
at 1 AU freezes with 0.001986 bar of CO₂ and thaws at 0.180748 bar (13
thaw steps), so `d⊕` = 0.1788 bar (0.178762), twice legacy's 0.091 bar;
with its recorded volcanism 0.506064 (`rate⊕` 1.012128), `W` = 12.44,
kept to three figures as 12.4 bar, which gave Earth a chance of
0.01404. That is twice the
6.4 bar expected from legacy's `d⊕`, inside the 3 to 13 bar the plan
allowed. Refitted again with the stellar spectra of issue #655 (study
35), with the cloud albedo refitted on the table's Sun (0.296) and
Earth's argon (#694): the reference Earth then froze with 0.002058 bar
and thawed at 0.148885 bar, so `d⊕` = 0.1468 bar (0.146828) and `W` =
10.22, kept as 10.2 bar, which gave Earth a chance of 0.0140 (it
fell to 0.01168 under 12.4 bar). Refitted again with #705's cloud
albedo (0.322, fitted with ozone over the climate's own ground; the
reference Earth holds no O₂, so no ozone): it froze with 0.004206 bar
and thawed at 0.149970 bar, so `d⊕` = 0.1458 bar (0.145764) and `W` =
10.14, kept as 10.1 bar, which gave Earth a chance of 0.0141 (it
would be 0.01392 under 10.2 bar, inside the window too). Refitted again
with the N₂ and O₂ pairs of #703 (study 36): the reference Earth freezes
with 0.004071 bar and thaws at 0.144919 bar, so `d⊕` = 0.1408 bar
(0.140848) and `W` = 9.80, kept as 9.80 bar, which gave Earth a
chance of 0.0140 (it fell to 0.01359 under 10.1 bar, inside the window
too). Refitted again with the clouds' infrared emissivity of #705
(0.848, fitted on CERES's longwave cloud effect once ozone, methane and
N₂O absorb in the column, #704; after N₂'s condensation of #741): the
reference Earth freezes with 0.006910 bar and thaws at 0.148532 bar, so
`d⊕` = 0.1416 bar (0.141623) and `W` = 9.855, kept as 9.85 bar
(Earth's chance was 0.01408 under 9.80 bar, inside the window).
Refitted again once the climate kept its air in weights (#749): `d` and
`W` are CO₂ weights, and the column reads the CO₂'s partial pressure
from the whole air, which holds less CO₂ by moles than by weight (44.01
against Earth's 28.96 g/mol). The reference Earth freezes with 0.010605
bar of CO₂ by weight and thaws at 0.200121 bar, so `d⊕` = 0.1895 bar
(0.189516) and `W` = 13.19, kept as **13.2 bar** (Earth's chance was
0.01865 under 9.85 bar, out of the window, and is 0.01399 under 13.2
bar). The thermostat's 0.010605 bar by weight is a partial pressure of
6.82e-3 bar, against the 6.910e-3 bar the climate read before: the
weights moved the reading, not the climate. The tier-3 test `earth_snowballs_as_often_as_hoffman_dates`
holds Earth's chance in 0.013 to 0.016 on the full solve (65 s at the
tier-1 opt-level); a tier-1 test of `science::snowball` holds the
arithmetic of the fit on the measured `d⊕` and `rate⊕`.
Hoffman et al. quote Walker et al. 1981: "Even the present rate of
release would yield 1 bar of carbon dioxide in only 20 million years",
so 6.4 bar would be about 130 My of today's outgassing per temperate
phase, and 13.2 bar by weight about 260 My (in the chain Earth's air,
over its 0.746 bar of N₂ and 0.013 bar of argon, 13.2 bar of CO₂ by
weight is a partial pressure of 12.8 bar, `Weights::partial_pressures`,
derived).
Searched and not found: a snowball frequency for another planet, and
the length of the Paleoproterozoic snowballs. Where no CO₂ thaws the
body, `thaw_bar` is infinite and the trace takes no non-finite value:
the chance 1 is recorded without it.

The states of the rewrite (`system::planet::climate::branches`,
`timeline::ClimateState`): `Runaway` when the water ran away;
`Snowball` when the body is bistable, its draw falls under the chance,
and its frozen branch is frozen over with a thaw; `Frozen` for a body
with water, frozen over, which no CO₂ it can hold thaws: bistable with
no thaw, or, with plates, frozen on its only branch past the outer
edge; `Temperate` otherwise. The plates condition is legacy's (its
`frozen` note, `explain.rs`): without a carbonate thermostat no CO₂
builds up to a thaw, and the cold has another cause, so Titan, its
water ice under a methane sea, stays `Temperate`. "Frozen over" reads
the melting curve of the stable ice at the surface pressure
(`water::melting_temperature`), as `liquid` does, not legacy's
273.15 K (#671).

#### Calibration of `CH4_OPACITY` (issue #646, refit)

Retired by study 36 (issue #703, the user's decision of 2026-11-25):
HITRAN's N₂–N₂, N₂–CH₄ and CH₄–CH₄ pairs replace the grey opacity and
`CH4_SATURATION_BAR`, and settle the same column at 92.30 K (92.296 K
in the tier-1 test, now
`titans_pairs_hold_its_column_just_under_its_measured_surface`, which
allowed 1.4 K under the measurement for the N₂–H₂ pairs, methane's lines
and the haze's infrared the column lacks; 1.9 K since issue #741, see
above). The record of the fit
follows.

Target: Titan's surface, 93.65 ± 0.25 K at 1467 hPa (Fulchignoni et
al. 2005), on the sourced column of `radiation::test_points` (1.467
bar, 5.65 % methane, effective temperature 82 K), with the new methane
curve and the anti-greenhouse of `CH4_NEAR_IR_HALF_BAR` (0.0013 bar,
already refitted); McKay, Pollack & Courtin 1991's 82 K + 21 K − 9 K
make the same 94 K. Method: bisect the opacity until that column
gives 93.65 K, three figures; a tier-1 test holds the column at
93.65 ± 0.25 K, and the generated Titan through the chain is a
validation (94 ± 8 K), no longer the target.

Result: 15.1, from 13.4. The bisection, on the column through
`balance::surface_temperature`: 13.4 gave 91.24 K, 16 gave 94.83 K,
14.7 gave 93.06 K, 15.35 gave 93.95 K, 15.0 gave 93.47 K, 15.2 gave
93.74 K, 15.1 gives 93.60 K (15.13 and 15.14 bracket 93.65 K at the
fourth figure). `titans_column_sits_at_its_measured_surface` (tier 1)
held it. Searched and not found: a grey or band opacity of Titan's
collision-induced greenhouse usable as one number; HITRAN's N₂–N₂ and
N₂–CH₄ CIA tables would replace the fit, and were not extracted.

#### A climate with no fixed point (#729)

Just past a methane Earth's outer edge the albedo's fixed point has no
solution. At 1.44 AU, from its warm start, the climate jumps between
three states: the thermostat (288 K, 15.2 bar of CO₂), the warm balance
at the maximum greenhouse (277 K, 15.8 bar) and a collapse (188 K,
0.57 bar). The CO₂ of a warm climate raises the albedo it derives (its
Rayleigh scattering) past the light at which its CO₂ frosts out; the
collapsed climate derives a lower albedo, which brings the CO₂ back.
The collapse is a jump in the light: from the maximum greenhouse its
fixed point either stops at a warm balance or falls to the cold one.
Legacy's step (the secant of #593, between half the way and the whole
way to the albedo derived) kept the full step inside that cycle, so the
fixed point ran to `ALBEDO_ITERATIONS` and kept whichever climate its
last step landed on.

The rewrite keeps the bracket the fixed point has once its push turns
back: the last albedo that pushes up and the last that pushes down. Each
further step goes to the zero of the push between them by the Illinois
rule (Dowell and Jarratt 1971, as `regula_falsi` applies it). The
bracket is kept only while its ends push towards each other: one whose
ends push apart holds an unstable state of a bistable body, which the
plain step leaves as before. Where a fixed point exists inside the
bracket, the push reaches it by those steps; a fixed point that never
brackets takes the same steps as legacy, to the last bit (Earth and
Titan, measured). Where the bracket narrows to `ALBEDO_BRACKET` in both
the albedo and the emissivity without the temperature settling, the
climate has a
jump and no fixed point: it is stored on its colder side, the one a
cooling planet reaches, with `converged: false` and the cause node
`planet.climate.cycles` true (decided with the user, 2026-11-26, after
study 25: the study does not cover it). A bracket that attracts can
still hold three roots, the two stable branches of a bistable body and
the unstable one between them, and its steps can stop on the middle
one: since #703's first pull request a root is kept only where it
attracts, and an unstable
one is left from its cold side (see the nitrogen sweep above and
[nitrogen-phases.md](nitrogen-phases.md)).

**Departure.** Kadoya and Tajika 2014, Menou 2015 and Haqq-Misra et al.
2016 find limit cycles in time near the outer edge: the carbonate-silicate
cycle at low outgassing swings a planet between a warm and a glaciated
state over millions of years. A solver with no fixed point is the static
analogue of those limit cycles, not the same thing. A climate that
spends part of its time on each side has a time-averaged state that a
future law would rest on these papers for. The rewrite draws no share
of time: the cold side and the `cycles` flag say that no stable warm
state is claimed. The edge of a methane Earth moves from 1.4443 to
1.4355 AU with it (measured, `one_greenhouse_search_on_a_condensing_air`;
1.4365 AU since #734), and that test went from 833 s to 74 s (release).
Since methane's grey stand-in is retired (#703) its edge was 1.2588 AU
and the orbits with no fixed point ran from 1.268 to 1.277 AU (probed
every 0.001 AU); the tier-3 test
`a_climate_with_no_fixed_point_stops_on_its_cold_side` moved from 1.44
to 1.272 AU, where the climate cycled for 16 steps and was stored at
227.17 K with 8.01 bar of CO₂ under the plain collapse, then for 17
steps at 232.79 K with 9.91 bar under the collapse of #743. Since CO₂'s
phase step was split (#746) the edge is 1.2534 AU and the orbits with
no fixed point run from 1.261 to 1.267 AU (1.260 settles at its maximum
greenhouse, 1.268 frozen); 1.272 AU now converges (227.31 K, 8.05 bar),
so the test moved to 1.264 AU, where the climate cycles for 21 steps
and is stored at 231.65 K with 9.50 bar. With methane's and ozone's
lines (#704) and the clouds' emissivity of #705 the edge is 1.2720 AU
and the orbits with no fixed point run from 1.280 to 1.285 AU (from
1.268 to 1.272 AU it settles warm and unregulated, from 1.273 to
1.279 AU and from 1.286 AU frozen; the band was swept before #741 and
before the emissivity, which leave the edge and the cycling at 1.282 AU
as they were); the test moved to 1.282 AU, where the climate cycles for
19 steps and is stored at 234.08 K with 10.39 bar (233.92 K and
10.33 bar under the collapse's step rules of 2026-11-27, #756).
Once the climate keeps its air in weights (#749), Titan's 2.11 bar of
methane by weight is 35 % of the moles of the methane Earth's air at
its edge (0.746 bar of N₂, 0.013 of Ar and 9.50 of CO₂ by weight
beside it), a partial pressure of 4.35 bar, where read as a partial
pressure it was 17 % of that air's pressure (derived), and the edge
moves to 1.3276 AU: the thermostat holds 288 K to
1.311 AU, the climate settles warm and unregulated to 1.327 AU, and
frozen beyond, with a jump at 1.333 AU (267.2 to 235.3 K). No orbit
cycles, probed every 0.005 AU from 1.20 to 1.50 AU and every 0.001 AU
from 1.305 to 1.352 AU (at 1.282 AU it now regulates at 288 K with
7.08 bar). Round a 3300 K star the same methane Earth cycles from
1.2950 to 1.3000 AU and at 1.3070 and 1.3080 AU (edge 1.3079 AU), but
on its regulated side, near 288 K: not the frozen, unregulated cold
side the test held. With no methane Earth to stand on, the tier-3 test
`a_climate_with_no_fixed_point_stops_on_its_cold_side` and its orbit
(`CYCLING_AU`) were deleted in #749; issue #767 records its intent and
this orbit history, and rebuilds the stopping rule on constructed
inputs.

#### A collapse from the fold (#731)

A climate past its maximum greenhouse collapses from the CO₂ of that
maximum: the CO₂ frosts out, step by step, until the air keeps what its
surface allows. Its gap `g(c) = c − φ(c)` (the CO₂ of the air less what
its surface keeps) was about zero there on a methane Earth's edge
search (measured before #734): its maximum greenhouse sat at the fold
that ends the warm branch. Below it `φ`
grows with `c`, so the plain step `c ← φ(c)` falls onto the largest
balance below the start without passing it. From the fold it crawls: on
a methane Earth near its outer edge `g` starts at 0.002 to 0.006 bar
and grows by about a tenth a step, so the collapse took up to 58
climates before falling to the cold balance (0.57 bar, 188 K).

The rewrite keeps the plain step as its shortest and lets each step
grow to at most twice the last. While `g` rises or stays flat, a step
is also at most `COLLAPSE_STEP` of the CO₂ (0.05, 0.85 bar at 17 bar).
Once `g` falls by more than its noise, the step goes to the zero of the
secant through the last two points, with no cap but twice the last
step; a fall within the noise is a jitter of the solve and keeps the
rising rule, so that it cannot lengthen a step past the cap. Where `g` falls
below minus its noise, the step has passed a balance: the search
brackets it and closes in by the Illinois rule. A warm balance narrower
than a rising step, or hidden before the secant's zero, can be stepped
over; the plain step finds every one (tier 1, synthetic collapses in
the measured shape).

A long step can also cross part of a warm balance (a pocket of `g`
below zero) and land next to its lower root, the unstable one, where
`g` falls with the CO₂: within the noise above it, or within the
tolerance below it. The search would stop there, on a balance that the
plain step never reaches and that a cooling planet cannot hold (#755).
Since 2026-11-27, a stop reached by a longer step than the plain one,
or inside the bracket, costs one more climate just above it, where the
secant from the stop to the point above it reaches five times the noise
(`POCKET_PROBE_NOISES`), at least `COLLAPSE_TOLERANCE` of the CO₂ above
the stop and at most halfway to that point: if `g` there is below minus
its noise, the stop sits under a pocket, and the search closes in on the
pocket's upper root, the plain step's balance. On 4287 synthetic pockets
deeper than twice the noise, cut into the traced 3300 K gap with `φ`
growing with the CO₂, the stops on an unstable root went from 26 to
none, for one more evaluation a collapse on average (19.8 to 20.8); at
1.5, 2, 3 and 8 noises they are 5, 3, 1 and 0. On the climates
of the heavy climate tests (1601 stops traced on main 6fc12ab7) no stop
sat on an unstable root, and on main 22e37f31 the check never fires
there: those climates do not change.

Until 2026-11-27 every step was capped at 0.005 of the CO₂. Round cool
stars that made the thaw search of a methane Earth near its outer edge
slow (#731, main a97c1ca1): a frozen climate collapses its CO₂ again
at every albedo step, and its gap is a single hump, from about zero to
0.3 to 3 bar and back down to its root over 12 to 44 bar of CO₂. The
cap kept the rise short and the secant short of the root, up to 45
(3300 K) and 85 (4500 K) evaluations of about 33 ms per collapse, and a solve took 261 to 276 s
at 4500 K (1.2828 AU) and 45 to 79 s at 3300 K (1.32 to 1.37 AU). On the
traced gaps the two rules take 22 instead of 83 evaluations at 4500 K
and 22 instead of 44 at 3300 K (simulated). Accepting that a narrow warm
balance can be stepped over was the user's decision on 2026-11-27. The
frozen climates of those solves (`COOL_FROZEN` in
`collapse_step_is_converged`) are held against half the step, which
moves the rising cap, and against the plain step, which finds every
root and so holds the falling rule too:

| Frozen climate | Step rules | Half the step | Plain step | Old step (0.005) |
| --- | --- | --- | --- | --- |
| 4500 K, 19.03 bar | 223.5344 K | 223.4506 K | 223.5353 K | 223.461 K |
| 4500 K, 19.62 bar, cycles | 222.6553 K | 222.6540 K | 222.6571 K | 222.621 K |
| 3300 K, 45.9 bar | 204.7439 K | 204.7578 K | 204.7436 K | 204.758 K |

The rules stay within 0.002 K of the plain step, closer than the old
step did (0.07 K at 19.03 bar). The old step's column was measured with
probes on main a97c1ca1 (#731).

With methane's lines (#704) and the clouds' infrared emissivity (#705)
those three climates thaw (308.6, 309.8 and 381.4 K), and a methane
Earth round a 4500 K star is no longer bistable up to its outer edge
(1.3315 AU; its solves take under 6 s). Round a 3300 K star (edge
1.4147 AU) it is bistable from about 0.94 of its edge, and its solves
still take 42 to 80 s (release) at 0.94 to 0.98 of it. The test holds
frozen climates there, at the geometric mean of the CO₂ each froze with
and the most its thaw search leaves frozen:

| Frozen climate | Step rules | Half the step | Plain step |
| --- | --- | --- | --- |
| 3300 K, 1.3298 AU, 18.27 bar | 215.7167 K | 215.7130 K | 215.7292 K |
| 3300 K, 1.3581 AU, 27.96 bar | 210.3725 K | 210.3725 K | 210.4052 K |
| 3300 K, 1.3864 AU, 39.31 bar | 205.4620 K | 205.4620 K | 205.4657 K |

Since the climate keeps its air in weights (#749) the 3300 K methane
Earth's edge is 1.3079 AU, and it is bistable only within 0.5 % of it
(from 1.3030 to 1.3080 AU, probed every 0.001 AU; the solves at 0.94,
0.96 and 0.98 of the edge are temperate with no frozen branch). The
test holds its bistable orbits that settle, at the same geometric
mean: 1.3030 AU and 31.39 bar, 1.3050 AU and 32.24 bar, 1.3060 AU and
32.50 bar (by weight):

| Frozen climate | Step rules | Half the step | Plain step |
| --- | --- | --- | --- |
| 3300 K, 1.3030 AU, 31.39 bar | 251.8200 K | 251.8036 K | 251.8756 K |
| 3300 K, 1.3050 AU, 32.24 bar | 250.1125 K | 250.1121 K | 250.1172 K |
| 3300 K, 1.3060 AU, 32.50 bar | 248.9588 K | 248.9567 K | 249.0257 K |

Before #749, at the most CO₂ each thaw search leaves frozen, within 0.004 bar of its
thaw, the branch flipped with the step: at 1.3298 AU and 20.9095 bar the
rules land frozen (215.73 K) and the plain step warm (316.14 K); at
1.3864 AU and 51.2841 bar the rules frozen (205.47 K) and half the step
warm (384.96 K). The frozen branch a snowball is stored on sits at that
CO₂.

Before #703's pairs, at
half the step the methane Earth's climates at 1.42875, 1.4355 and
1.44 AU, around its outer edge, did not move (tier 3,
`collapse_step_is_converged`). With the pairs and the split phase step
of #746 the edge is 1.2534 AU and the test held 1.25, 1.2534 and
1.264 AU (`STEP_ORBITS`); each moved under `ALBEDO_TOLERANCE_K` at half
the step (at 1.264 AU, 231.5440 against 231.5523 K with the rules of
2026-11-27; 231.6489 against 231.6492 K before them). That cycling
climate moved by 0.105 K with the rules (231.6489 to 231.5440 K), the
size of the step's effect on a cycle's cold side measured below. Since
#704 and #705 the edge is 1.2720 AU and the test holds 1.265, 1.272 and
1.282 AU, and at half the step 288.0001, 283.8828 and 233.9173 K
become 288.0001, 283.8828 and 233.9140 K (under the step rules of
2026-11-27). Since #749 the edge is 1.3276 AU and no orbit past it
cycles: the test holds 1.321, 1.3276 and 1.333 AU, the last where the
climate falls onto its frozen branch in the most albedo steps (7):

| Orbit | Step rules | Half the step | CO₂ by weight |
| --- | --- | --- | --- |
| 1.321 AU | 280.0386 K | 280.0386 K | 9.8338 bar |
| 1.3276 AU | 273.4592 K | 273.4592 K | 9.4980 bar |
| 1.333 AU | 235.2849 K | 235.2849 K | 9.2392 bar |

Measured on 2026-11-26 at the old
`COLLAPSE_STEP` (0.005), half, a quarter and the plain step, the
climates just past the edge stay within:

- 0.11 K at 1.268 AU, a converged climate (230.92 to 231.03 K, 9.25 to
  9.28 bar of CO₂);
- 0.08 K at 1.264 AU, 0.22 K at 1.266 AU and 0.33 K at 1.262 AU, on the
  cold side where a cycling climate is stored (231.39 to 231.77 K, 9.41
  to 9.54 bar): where a climate cycles, the step can change where the
  cycle stops, by a few tenths of a kelvin.

Before #747 they moved by up to 5.6 K (9.26 to 11.42 bar at 1.268 AU),
and before #746 they landed on 9.91 or 8.00 bar. The cause was the cap
on a collapse's steps, not the step: just past the edge the CO₂ falls
from 14.2 to about 9.3 bar on a flat gap and needs up to 207 plain
steps, and stopped at 100 the collapse kept a climate that depended on
the step. `COLLAPSE_ITERATIONS` is 300 since #747, and a collapse
stopped at it is not converged.

The gap's noise is how much `φ` moves within the temperature tolerance
of the climate solve, `φ(T + TOLERANCE_K) − φ(T)`: 0.005 bar at 248 K
and 17 bar, the size of the oscillation measured at the fold. Legacy
stopped wherever `g` fell under `COLLAPSE_TOLERANCE`, and at the fold
that happened on its noise: whether a collapse stayed warm (248 K, at
the fold) or fell cold depended on the solver's last digits. The
rewrite counts a balance only once `g` has risen above its noise since
the start: a zero within the noise is not a balance, and the collapse
steps on. A collapse whose air already holds at its start does not
step at all, as in legacy. This follows the decision of #729 (at a
discontinuity, the cold side; decided with the user, 2026-11-26). Before
#734, on the methane Earth's edge search, 28 of 280 collapses that
legacy stopped warm on noise fell cold, and the search went from 74 s to
59 s (release), its crawls from the fold gone. Before #703's pairs, and
since #734, the maximum
greenhouse sits on the fold less often: 12 of 272 collapses (from 16.14
and 16.23 bar, about 247 K) fall cold, at 16 to 18 climates each where
the stops on noise took 2 to 6; the others keep their balance within
0.001 K. The outer edge stayed at 1.4365 AU, and
`one_greenhouse_search_on_a_condensing_air` took 40 s against 37 s
with the plain step (these figures date from before #703's pairs, which
moved the edge to 1.2588 AU; not measured again): the rule costs that,
for a result that no longer
depends on the solver's last digits. Within the noise the steps stay
at least the noise, and inside a bracket a point below the balance
within its noise ends the search: the solve's 0.01 K tolerance lets the
gap jump across a balance by up to that much.

A collapse can also run out of steps. With #703's pairs, a methane
Earth just past its outer edge collapses from 14.2 to about 9.3 bar
with a gap of only 0.02 to 0.09 bar all the way: the step is about the
gap, and the collapse needed up to 91 climates at the old
`COLLAPSE_STEP` (0.005), 140 at half of it and 207 with the plain step (measured at 1.25 to
1.268 AU). Legacy's cap of 100 cut the half step short, and the
climate it kept depended on the step (232.36 K against 230.96 K at
1.268 AU); with the cap lifted the step is converged (230.92 to
231.03 K at every step). The cap is now 300, and a collapse that stops
at it no longer passes for a settled one: the climate it belongs to is
stored not converged (`planet.climate.converged` false), and
`collapse_step_is_converged` fails on it. Crossing that flat gap in
fewer climates is left to a search of its own.

#### A thaw search that meets a cycle (#731)

The thaw search bisects `ln CO₂` between the CO₂ the temperate branch
froze with and its maximum greenhouse, from an icy start at each CO₂
(13 steps, `THAW_STEPS`). Between the CO₂ where that start settles
frozen and the CO₂ where it thaws, a methane Earth near its outer edge
round a 4500 K star has a band where the start has no fixed point and
cycles: at 1.2828 AU (0.9 of the way from its runaway orbit to its
edge) it stays frozen up to about 19.40 bar, cycles from there to
19.64 bar, and thaws at 19.647 bar (measured on main f6462ba2,
release, with probes on each call of the search, 4 rayon threads, on a
loaded shared machine: the times below compare runs of that setup, and
#731's figures, 4 cycling calls and a 110 s solve, came from another
run of it).
Legacy's bisection, which the rewrite kept until 2026-11-27, counted a
cycling start as frozen, since it is stored on its cold side (#729): it
converged on the top of the band, spent 6 of its 13 calls there at 19
to 22 albedo steps each (99 of 138 s), and stored the snowball as a
cycle's cold side (222.578 K, `cycles` true), with a thaw at 19.6473
bar and a snowball chance of 0.1099.

A snowball whose CO₂ has entered the band has no frozen balance left:
its climate swings to the warm side, the static analogue of the limit
cycles of Kadoya and Tajika 2014 and Menou 2015. The thaw is therefore
where the band starts, and since 2026-11-27 (decided with the user,
#731) the search stops at its first cycling call: that CO₂ is the
thaw, and the last CO₂ that stayed frozen is the snowball's. On that
methane Earth the thaw goes to 19.4984 bar (−0.76 %), the chance to
0.0979, and the snowball is a frozen balance (223.102 K at 19.3409 bar,
`cycles` false); the search takes about 56 s instead of 138 s. The
bottom of the band is then known only within the bracket the search
had reached when it met the band (19.34 to 19.50 bar here, 0.8 %), not
within `THAW_STEPS`: continuing the search with a cycle counted as
thawed would place it within 0.013 % (19.3974 bar) but costs as much as
before (130 s), the frozen calls just under the band taking 11 to 13
albedo steps each. An icy start that cycles at the frozen CO₂ or at the
maximum greenhouse counts as thawed too. Round the Sun (1.25, 1.2534
and 1.264 AU) and round a 3300 K star (1.3231, 1.3586 and 1.3657 AU),
no call of the search cycles, and the plain Earth's never does (#731):
those climates do not change. The tier-3 test
`a_snowball_thaws_where_its_icy_start_starts_cycling` held the 4500 K
case until methane's lines (#704, merged with #759 on 2026-11-27):
with them that methane Earth is no longer bistable up to its edge
(1.3315 AU), and round a 3300 K star (edge 1.4147 AU) no search from
0.94 to 0.99 of the edge meets a cycling icy start (1.3298, 1.344,
1.3581 and 1.3864 AU thaw at 20.9125, 29.5120, 37.5607 and
51.2875 bar on a warm icy start; 1.393, 1.3975 and 1.4006 AU stay
frozen to the maximum greenhouse). No cycling thaw was found to hold,
so the test was dropped; the ends of the search stay tested in tier 1.
`collapse_step_is_converged` checks that the collapse's step does not
flip its frozen climates between settling and cycling.

The search starts from the frozen CO₂, or from `THAW_FLOOR` (1e-12) of
the maximum greenhouse where the temperate branch froze with less (no
CO₂, or the thermostat's trace). Since #758 the icy start at that floor
is read too, since the bisection holds only if its low end stays frozen:
if it does not, the floor is the thaw. At that share the CO₂ has no
measurable greenhouse, so it is a guard: none of the 34 searches of the
heavy climate tests, nor an Earth from 0.80 to 1.00 AU, starts from the
floor (main 6fc12ab7).

#### Solver constants (rule of #648)

A guard (a cap on steps, a search range, a start or a damping that
does not change the converged result) is a `Definition:`; a
discretisation is a `Calibration:` held by a convergence test.

| Constant | Value | Label | Held by | Tier |
| --- | --- | --- | --- | --- |
| `TOLERANCE_K` | 0.01 K | Calibration | the reference climates at a tenth of it move under 0.01 K | 1 |
| `BRACKET_TOLERANCE_K` | `TOLERANCE_K` / 100 | Calibration | with `TOLERANCE_K` | 1 |
| `ITERATIONS` | 60 | Definition | the reference solves stop by tolerance under it | 1 |
| `SCAN_RATIO` | 1.03 | Calibration | at 1.01 the same first balance, Earth and a sweep across `S_rg` | 3 |
| `RUNAWAY_PROBE_K` | 380 K | Definition | the wet balance agrees with and without the probe | 1 |
| `PLATEAU_SURFACES_K` | 500 to 640 K by 20 K | Calibration | every 10 K and to 700 K, the plateau moves under 0.5 W/m² | 3 |
| `COLLAPSE_ITERATIONS` | 300 | Definition | a collapsing Earth at 1.6 AU settles under it (tier 1); a methane Earth just past its outer edge with #703's pairs needs up to 207 steps, plain; a collapse stopped at it leaves its climate not converged | 1, 3 |
| `COLLAPSE_TOLERANCE` | 1e-4 | Calibration | at 1e-5 the temperature moves under 0.01 K | 3 |
| `COLLAPSE_STEP` | 0.05 of the CO₂, while the gap rises (0.005 for every step before 2026-11-27) | Calibration | at 0.025, and against the plain step for the falling rule, a methane Earth's frozen climates round 4500 and 3300 K stars move by at most 0.084 K (#731); at the old 0.0025 its climates at 1.42875, 1.4355 and 1.44 AU did not move before #703's pairs; since them and #746, 1.25, 1.2534 and 1.264 AU do not move, and from 1.262 to 1.268 AU full, half, quarter and plain steps stay within 0.11 K on a converged climate and 0.33 K on a cycle's cold side (5.6 K before the cap of #747) | 3 |
| `POCKET_PROBE_NOISES` | 5 noises | Calibration | the synthetic pockets of #755: 5, 3, 1, 0 and 0 stops on an unstable root at 1.5, 2, 3, 5 and 8 noises, 26 unchecked; a stop on a stable root does not fire it | 1 |
| `POCKET_PROBE_REACH` | 0.5 of the way to the point above | Definition | the probe stays inside the bracket | 1 |
| `PHASE_BAND` | 1 K | Calibration | Titan-like and Triton-like sweeps settle across their points | 3 |
| `THAW_FLOOR` | 1e-12 of the maximum greenhouse | Definition | keeps `ln CO₂` finite; the icy start is read there when the search starts from it (#758) | 1 |
| `THAW_STEPS` | 13 | Calibration | at 16 steps Earth's thaw CO₂ moves under 0.1 % (measured: +0.165 % at 12 steps, +0.063 % at 13, 0.180932, 0.180748 and 0.180634 bar at 12, 13 and 16); fewer steps run when the search meets a cycling icy start (#731) | 3 |
| `ALBEDO_ITERATIONS` | 200 | Definition | the reference climates settle under it | 1 |
| `ALBEDO_TOLERANCE_K` | 0.1 K | Calibration | at 0.01 K the reference climates move under 0.1 K | 1 |
| `ALBEDO_DAMPING` | 0.5 | Definition | at 0.25 the converged climates agree | 3 |
| `ALBEDO_BRACKET` | 1e-6 in albedo | Definition | a cycling climate stopped on its cold side within 40 steps (#729); the test was deleted in #749, no methane Earth cycling any more (#767) | 3 |
| `CONDENSE_ITERATIONS` | 100 | Definition | Titan and Triton settle under it | 1 |
| `THERMOSTAT_TOLERANCE` | 1e-3 in ln CO₂ | Calibration | at 1e-4 Earth moves under 0.01 K | 1 |
| `GREENHOUSE_DECADES` | 4 | Definition | the reference maxima lie inside the range | 1 |
| `GREENHOUSE_STEPS` | 18 | Calibration | at 30 steps the outer edge moves under 0.001 AU | 3 |

The study's criterion for `THERMOSTAT_TOLERANCE` also asked that Earth's
CO₂ move under 0.1 % at a tenth of it. The test holds the temperature
only (under 0.01 K); the tolerance is itself 1e-3 in `ln CO₂`, so the
criterion on the CO₂ is the tolerance's own definition, and the
temperature is what the test can show.

#### Tidally locked rotators (issues #672 and #675)

The solver takes a "locked" flag with the obliquity. A locked rotator
receives `4μ` of the mean light on its day side, `μ` the cosine of the
angle from the substellar point (Lambert's law, normalised to a global
mean of 1), none at night; its ice forms from the antistellar point,
so an ice area share `a` receives `max(0, 2a − 1)²` of the light
(`Definition:`, derived). Its lava test reads the substellar
temperature of Kite et al. 2016 (arXiv:1606.06740, eq. 1, read):
`T_ss = T⋆ (1 − α)^¼ √(r⋆/2a) 4^¼`, `√2` times the equilibrium
temperature under the same albedo (derived), against 1673 K, with the
body's own albedo (Kite et al. assume 0.1). A fast rotator keeps the
zonal weighting and the mean. Searched and not found: a rotation
period past which a non-locked rotator should leave the zonal form.
Showman et al. 2013 (arXiv:1306.2418, § 2) put "Ro≳1 everywhere when
the rotation period exceeds ∼10 Earth days"; Yang et al. 2014
(arXiv:1404.4992) find rotators of about 100 days or more behave like
locked ones through their clouds; Way et al. 2016 (abstract) a
habitable Venus under a rotation "slower than about 16 Earth days".
These are circulation results; the insolation averaged over whole
solar days is zonal for any non-locked rotator, so the flag is
"locked" only.

#### Test air and Solar System bounds

- **Mars**: "6.36 mb at mean radius (variable from 4.0 to 8.7 mb
  depending on season)", 95.1 % CO₂, 2.59 % N₂, 0.16 % O₂ (NASA Mars
  fact sheet, read, May 2025), imposed in the test (CO₂ 6.05 mbar, N₂
  0.165 mbar, O₂ 0.010 mbar, derived), with its argon, 1.93 % by volume
  (Mahaffy et al. 2013), 0.123 mbar: partial pressures, which the test
  turns into the weights the climate reads (`PartialPressures::weights`,
  issue #749; the *Argon (study 29)* section's 0.11 mbar is that argon's
  weight).
  Legacy's `MARS_CO2_FACTOR` is not ported. The fact sheet gives "Average
  temperature: ~214 K"; no source for the legacy 210 K was found (it is
  Mars's equilibrium temperature), so the bound is 214 ± 5 K, `Source:`
  the fact sheet (decided with the user on 2026-11-24).
- **Earth** 288 K (fact sheet), **Titan** 93.65 ± 0.25 K (Fulchignoni
  et al. 2005), **Venus** Bond albedo 0.76 ± 0.05 (NASA Venus fact sheet) and no
  liquid, held in tier 1. The model's own 577 K is not a source: it is
  pinned as a regression net in tier 2 once 2d-4 makes the chain
  reachable from `crates/regression` (#594).

#### The reducing-air fallback

A reducing body keeps the air solved with its CO₂ held back only if
its climate leaves methane and a total pressure over methane's
triple-point pressure (0.11696 bar, Setzmann & Wagner 1991 as `volatiles_tests.rs` cites
it, read as `volatiles::methane_saturation_pressure` at the triple-point
temperature: a `Definition:`, one code path); otherwise it is solved
again with its CO₂. No new constant.

#### Earth's longwave against CERES (study 32)

Study 32 of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/32-longwave-against-ceres-audit)
(2026-11-25, issue #680) asked why the climate's column sends out
7 W/m² more than CERES's clear sky, and whether that makes the
reference Earth (285.82 K at 280 ppm in planet PR 2d-4) too cold. The details
and the measurements are in
[the radiative transfer page](radiative-transfer.md#where-it-departs-from-the-science);
for the climate:

- **The column's water is not too dry.** Without its clouds the column
  sends out 274.63 W/m² at 288 K and 280 ppm (274.72 before the phase
  step was split), under the 214.51 K skin of CERES's all-sky emission.
  With the gases it lacks (ozone 4 to 10 W/m², Schmidt et al. 2010 and
  Kiehl & Trenberth 1997; methane, N₂O and halocarbons 3.6 W/m², Hogan
  & Matricardi 2020), the CO₂ of CERES's years (1.75 W/m²) and the
  clear sky of a column model (Loeb et al. 2020's 265.9 W/m²), the
  like-for-like level is 275.5 to 282.9 W/m²: the column lies 0.85 W/m²
  under it, inside only the window widened by CERES's uncertainty
  (272.1 to 286.3 W/m²). On the 100 RFMIP reanalysis profiles the
  column is 1.0 to 1.9 W/m² darker than the mean of their own outgoing
  radiation with the same two gases. Moistening the upper air to bring
  the clear sky to 267.9 W/m² would hide the missing gases in the
  water, and is not recommended.
- **Its clouds are too dark in the infrared**: 33.38 W/m² of longwave
  effect at 288 K and 280 ppm (32.91 before the split) against CERES's
  25.7 (the same air without its clouds; Loeb et al. 2020) to 28.0 W/m²
  (cloud-free scenes), 5.4 to 7.7 W/m² too much. At 389 ppm the two errors cancel: the missing
  gases make the cloudy column send out about 7.6 W/m² too much, its
  clouds (32.98 W/m² at 389 ppm) 7.3 W/m² too little, and it sends out
  239.90 W/m² (240.46 before the split) against CERES's 240.1 W/m².
- **The cold reference Earth** is then not the longwave's doing. Its
  Bond albedo, 0.304 to 0.306, against the observed 0.2915 to 0.293
  (CERES), lets in 3.7 to 4.9 W/m² less, about 1.9 to 2.5 K at the
  column's cloudy-sky slope, 2.0 W/m²/K (the table below); the
  shortwave's missing ozone and aerosols are #653.
- **A numerical step, now split.** The moist adiabat's latent heat
  jumps at 273.16 K inside a step of the grid, so the cloudy column's
  outgoing radiation ran as a sawtooth in the surface temperature, up
  to 1.2 W/m² (0.89 W/m² at 288 K against a grid 16 times finer). The
  step is now split at 273.16 K (radiative transfer page, *The phase
  step of the moist adiabat*): the cloudy column at 288 K sends out
  0.57 W/m² less, at 285.82 K (the reference Earth in planet PR 2d-4)
  0.07 W/m² more. It was not the cause of
  #681: the regulated Earth under Titan's methane stays at 284.0103 K
  after the split. That temperature is the dew point of the vapour the
  thermostat sets at 288 K, the floor of the surface's root search,
  reached because the thermostat weighs the light the surface absorbs
  on the air before its CO₂ is regulated (its methane's anti-greenhouse
  then takes less), and the regulated air absorbs 29 W/m² less.

What each change would do to the reference Earth at 280 ppm (285.82 K
in planet PR 2d-4), estimated, not run, from the column in that state
(hence the clouds row starts from its 32.48 W/m² at 285.82 K, not the
33.38 at 288 K), with one slope for every row:
the column's own cloudy-sky slope, 2.0 W/m²/K (1.98 on Earth's column
from 284 to 290 K since the split, 2.02 before). Koll & Cronin 2018's
clear-sky 2.2 W/m²/K would make each change about 10 % smaller.

| Change | Cloudy-sky W/m² | Temperature |
| --- | --- | --- |
| Ozone, methane, N₂O and halocarbons in the column | −7.6 (Schmidt et al. 2010, Table 1: 4.9 % of 155) | about +3.8 K |
| Clouds' longwave effect brought from 32.48 (the column at 285.82 K) to 25.7 W/m² | +6.8 | about −3.4 K |
| Bond albedo from 0.304-0.306 to 0.2915-0.293 | +3.7 to +4.9 absorbed | about +1.9 to +2.5 K |
| The phase step split at 273.16 K (done) | +0.07 at 285.82 K | about −0.03 K |
| Humidity over liquid water below 0 °C (not recommended until the 1967 convention is read) | −2.56 (study 32, at 288 K) | about +1.3 K |

The first two go together: refitting the clouds alone would cool Earth
by 3.4 K. Venus (supercritical, 85 bar of CO₂), Mars (6.36 mbar, below
273.16 K at every level) and Titan (no water) do not cross the phase
step; ozone and methane in the longwave would warm a body only where
its air holds them (Titan's methane is already in the column, through
its collision-induced pairs).

### Boiling and the steam envelope of the rewrite (study 26)

What PR 2d-3b ports, with the values study 26 of the
[Wiki](https://github.com/cyboooooorg/map-generator/wiki/26-boiling-and-steam-audit)
fixed before the code (2026-11-24): `boils` and its loop, the steam
world line (`hides_surface`) and the steam envelope (`steam_interior`).
The prose of [the chain](#the-chain-and-the-surface-classes) describes
legacy; where they differ, this sub-section holds for the rewrite.

#### The boil

- **The criterion** (`Source:`). Kasting 1988 (*Icarus* 74, 472,
  doi:10.1016/0019-1035(88)90116-9; read, Zenodo record 1253896) finds
  the outgoing infrared of a saturated column "levels out at higher
  surface temperatures at a value of about 310 W m⁻²" (§ 3a, Fig. 7a),
  so that the solar constant a surface temperature needs, `S_eff =
  F_IR / F_s`, has a maximum, 1.4 S₀ cloud-free (Fig. 7c): past it no
  balance exists under the critical point and the surface jumps to
  about 1500 K. Goldblatt et al. 2013 (accepted manuscript, read): "If
  the net absorption of solar radiation exceeds this limit, then
  surface temperatures will increase in a 'runaway greenhouse',
  evaporating the entire ocean", the limit 282 W/m². So "no wet balance
  under the moist limit" is the published criterion, with no
  temperature condition (#442).
- **The verdict** (`climate::wet::boils`). A body that keeps water out
  of a runaway boils when `wet_balance` is `None` under the starlight
  its Bond albedo lets in, the air its climate left and its surface's
  emissivity, whether its thermostat holds it or not. The emissivity is
  a departure from legacy: legacy passes an emissivity of 1.0; the
  rewrite passes the emissivity the climate radiated under.
- **Departure: a regulated climate is judged too** (user, 2026-11-24).
  Legacy skipped a climate its carbonate thermostat holds
  (`legacy-final:crates/mapgen/src/planet/mod.rs` l.524-537), taking
  the thermostat's balance for a wet one. It is not one: the thermostat
  (`climate::carbon::thermostat`, as legacy's) balances on the climate's
  column (`Profile::CLIMATE`, cloudy, relative humidity 0.77) at 288 K,
  while `wet_balance` first needs the saturated column (`Profile::SATURATED`)
  to send out what the surface absorbs, the moist limit of Kasting 1988
  and Goldblatt et al. 2013 above. The two part where the absorbed flux
  lies between the saturated column's peak and what the climate's column
  sends out at 288 K. Measured on an Earth with a tenth of its N₂ round a
  7220 K star at 1.2963 S⊕: absorbed 300.785 W/m², the climate's column
  at 288 K 300.785 W/m² (the thermostat's balance, CO₂ 2.6e-5 bar), the
  saturated column 290.7 W/m² at 288 K and at most 299.17 W/m² (at
  308 K), so no wet balance. By the sources such a body runs away; the
  model boils it. The band is narrow, about 0.5 % under that star's
  runaway flux (`runaway_flux(7220 K)`, 1.2964 S⊕); it widens when #680
  lowers the model's threshold. The thermostat is not changed.
- **Departure: the model's threshold is above Kopparapu's.** With
  Earth's air, an Earth ocean, emissivity 1 and Bond albedo 0.306, the
  model's boil threshold is an absorbed 299.43 W/m², 1.268 S⊕, against
  Kopparapu et al. 2014's 1.107 S⊕ (`habitable::runaway_flux`, 1.106).
  Cause: `wet_balance` needs the saturated column's outgoing radiation
  to reach the absorbed flux, and that column peaks at 299.4 W/m² near
  360 K before it falls to its 289.7 W/m² asymptote (the plateau); Kasting
  1988 Fig. 7a puts the level near 310 W/m². About 7 of the 7.8 W/m² gap
  between the threshold and a body at 1.2346 S⊕ (0.9 AU) is the wet
  column's outgoing radiation lying 7 W/m² above CERES (#680), which
  study 32 traces to the absorbers the column lacks (ozone, methane,
  N₂O), not to its water; how much of them a hot, wet column would keep
  was not studied. In the
  chain `water()`'s runaway flag (`runaway_flux`, 1.106 S⊕) already
  catches such planets before `boils` is asked; `boils` decides for the
  bodies it passes. The tests therefore take Venus's orbit, where the
  verdict does not depend on the gap.
- **Internal heat** (`Source:`). Kasting 1988 § 3d treats the
  difference between the outgoing infrared and the absorbed sunlight as
  the accretional heat flux a surface temperature needs; about
  150 W/m² of it runs a young Earth away under 0.7 S₀ (Fig. 13). Barnes
  et al. 2013 (*Astrobiology* 13, 225, doi:10.1089/ast.2012.0851,
  arXiv:1203.5104v2, read in UVic's repository) state the rule for any
  source: if the energy
  from "the Sun (Simpson, 1927), impacts (Abe and Matsui, 1988), or
  tidal heating" exceeds the limit, "runaway heating of the surface
  would ensue, causing evaporation of the entire ocean". The absorbed
  light plus the internal heat is what `boils` weighs.
- **The vapour bounded by the water** (`Definition:`, a mass balance:
  the vapour cannot weigh more than the body's water, `m_w g / 4πR²`).
  Kasting 1988 § 3a: a full ocean (1.4 × 10²⁴ g) gives 270 bar of steam;
  "if the mass of the oceans was somewhat lower, the H₂O pressure at the
  surface would vary smoothly from the saturated to the unsaturated
  region, and the transition from a moist to a runaway greenhouse would
  occur at a lower temperature". Boukrouche et al. 2021 (Fig. 9) find
  the end of the plateau at about 1050 K under 10 bar and 1800 K under
  260 to 295 bar: the thinner the steam, the sooner the column sends out
  more than the limit. A trace of water therefore finds a balance (it
  does not boil), all of it in the air.
- **After the boil.** The steam escapes at the energy-limited rate, 9 kg
  of water per kg of hydrogen (study 21, unchanged). In the rewrite
  (`settled::settled_with`): a first run (`settled::run`: the surface
  pass, the air, the climate and its reducing-air fallback) from the
  water its light leaves (`water::water`), then its verdict
  (`settled::verdict`, the `planet.climate.boils` node, which reads the
  solver's settled state, never a second solve). No boil: that run
  stands, and `planet.water.boiled` is recorded false from the verdict
  (the imposed leaf of `water` is replaced). A boil: a run from the
  steam forced on the formed planet's surface water (`water::boiled`,
  runaway and boiled from the verdict), then `settled::after_boil`: a
  steam run that kept no water stands; otherwise its leftover, condensed
  again (`water::recondensed`, the runaway node from the leftover's
  inventory), is run and judged, kept if it does not boil, the steam run
  standing if it does. Each run's nodes stay in the trace. Hamano et al. 2013
  (abstract read) split planets in two: "type I" keep their water as
  oceans, "type II", inside a critical distance, are "desiccated by
  hydrodynamic escape". Keeping the leftover as an ocean only if its own
  air finds a wet balance follows the same criterion as the boil.
  **Departure (user, 2026-11-24):** Goldblatt et al. 2013 find a cloud-free steam
  atmosphere absorbing more than the limit at today's sunlight, "a
  climate bistability": a body with a wet balance may also hold a stable
  steam state. The model takes the wet branch whenever one exists; no
  source says which branch a body that boiled takes back.
- **No moist-greenhouse loss** (a departure kept by the user on
  2026-11-24, unchanged from legacy):
  Kasting 1988 § 4 finds the stratosphere wet past about 1.1 S₀ (a
  mixing ratio over 10⁻³ loses Earth's oceans in under 4 Gyr); a body
  that keeps a wet balance loses no water to escape in the model.

#### The steam world line

The rewrite's line is `science::steam::hides_surface` and
`science::steam::STEAM_WORLD_MIN_BAR`.

- **Water's critical point** (`Source:`, already in the rewrite):
  `science::water::CRITICAL_PRESSURE`, 22.064 MPa, and
  `CRITICAL_POINT_TEMPERATURE`, 647.096 K (IAPWS SR1-86(1992), section 2,
  "Reference constants", read: identical to the IAPS 1983 statement).
- **Why no surface shows** (`Source:`). Pierrehumbert 2023 (*ApJ* 944,
  20, doi:10.3847/1538-4357/acafdf, arXiv:2212.02644, read § 1-2):
  "once the ocean surface temperature exceeds the critical point, there
  is no longer a distinction between the fluid and gaseous phase", the
  atmosphere's adiabat continuing "seamlessly" into the supercritical
  fluid. The column's base is supercritical when its pressure and its
  temperature are both past the critical point.
- **`STEAM_WORLD_MIN_BAR`: no source sets ten times the critical
  pressure.** Legacy's line (2206.4 bar) rests on its own census (no
  runaway between 1e3 and 1e5 bar), the old code's output, which cannot
  be a calibration's target. Searched and not found: a pressure past
  which a steam column is called a steam world, in Turbet et al. 2020,
  Pierrehumbert 2023, Aguichine et al. 2021, Kasting 1988 and Goldblatt
  et al. 2013. The line that follows from the sources is the critical
  pressure itself, a `Definition:` (the base fluid supercritical); the
  647 K condition then holds by construction, since a steam column's
  surface is never colder than its saturation temperature, and stays
  as a guard. **Decision (user, 2026-11-24):** `STEAM_WORLD_MIN_BAR`
  is `science::water::CRITICAL_PRESSURE` (220.64 bar), a `Definition:`,
  not ten times it. A runaway between 220.64 and 2206.4 bar becomes a
  steam world; a boiled Earth that kept its 270 bar of ocean is one.
- **Turbet's radius for steam worlds only** (user, 2026-11-24, as
  legacy). Turbet et al. inflate the transit radius of every runaway
  with water (on an Earth core, 320 to 370 km under 27 bar, 510 to
  600 km under 270 bar, derived); the model's radius is the surface's,
  so a runaway whose rock shows keeps its rock radius.

#### The steam envelope (Turbet et al. 2020)

Turbet et al. 2020 (*A&A* 638, A41, doi:10.1051/0004-6361/201937151,
arXiv:1911.08878v3, read in full; v3 is the "final version published in
A&A", with "small typos in Tables 1 and 2" corrected):

The rewrite's laws are `science::steam::base_pressure` (C.9, C.10),
`science::steam::steam_temperature` (Eq. 3), `science::steam::envelope`
(Eq. 2, one call that returns the base pressure, the core's gravity, the
temperature and the thickness) and `science::steam::THICKNESS_CAP`.

- **Eq. 2** (`Source:`), printed as `z = (1 / (ln(x/(1−x) · g²/(4πG
  P_t)) · R T/(M g)) − 1/R_core)⁻¹`, the same as Appendix C (C.8):
  `1/z = M g / (R T ln(P_surf/P_t)) − 1/R_core`. It is not upside down
  (v1, v2 and v3 read): the earlier note of this page was wrong.
  `P_surf = x/(1−x) g²/(4πG)` (C.9, C.10), `x` the water's share of the
  whole planet (Appendix D: `M_planet = M_core/(1−x)`), `g` and `R_core`
  the core's. The paper uses `R` = 8.314, `M` = 1.8e-2 kg/mol and `G` =
  6.67e-11; the rewrite's `MOLAR_GAS_CONSTANT`, `H2O_MOLAR_MASS`
  (18.015e-3) and `GRAVITATIONAL_CONSTANT` change `z` by under 0.1 %.
- **`P_transit` 0.1 Pa** (`Source:`, § 2.2: it "varies little across the
  range of parameters we explored and is roughly equal to 10⁻¹ Pa").
- **Eq. 3 and Table 2** (`Source:`), `log₁₀ T = β₁ + β₂x + β₃y + β₄z +
  β₅xy + β₆y² + β₇x³ + β₈x²y + β₉xy² + β₁₀y⁴`, `x = (log₁₀ x_H₂O −
  α₁)/α₂`, `y = (log₁₀ g − α₃)/α₄` (g in m/s²), `z = (log₁₀ S − α₅)/α₆`
  (S in S⊕). Table 2 of v3, read digit by digit on the rendered page:
  α = −3.550, 1.310, 1.099, 4.683e-1, 7.664e-1, 4.224e-1; β = 2.846,
  1.555e-1, 8.777e-2, 6.045e-2, 1.143e-2, 1.736e-2, 1.859e-2, 4.314e-2,
  3.393e-2, −1.034e-2. Legacy's `TURBET_ALPHA` and `TURBET_BETA` match.
  v1 and v2 print α₁ = +3.550 and the α in another order: never use them.
- **The clamps of Eq. 3's inputs** (`Source:` each bound). § 2.2: valid
  "for irradiation from 1 to 30 S⊕ …, surface gravity from 0.2 to
  6 g⊕, and water vapor pressure from 2.7 bar to 27 kbar". § 2.1: the
  structures the fit was made on span 27 bar to 270 kbar of steam. The
  rewrite keeps legacy's clamp of the water share to the shares whose
  steam weighs 27 bar to 270 kbar: clamped at 27 kbar, a 4.78 M⊕ body
  with 12 % water comes out 1.59 R⊕ (Zeng's Earth-like core
  table, 1.48 R⊕), under the 1.64 R⊕ legacy gave it condensed, against
  the paper's main
  result (§ 3.1: steam planets have "a significantly larger radius than
  in the condensed water configuration"). Clamped at 270 kbar it comes
  out 1.68 R⊕ on Zeng's core, over that 1.64 R⊕: the choice restores
  the paper's result there, but not on the rewrite's own core, where
  the body is 1.643 R⊕ against its condensed twin's 1.655 R⊕
  ([planet interiors](planet-interiors.md), Refs #684). Only the
  temperature is clamped; Eq. 2 takes the planet's own share and
  gravity.
- **The stellar spectrum** (a departure, documented). Turbet et al.
  2020 compute their fits with the model of Turbet et al. 2019 (§ 2.1:
  "For more details on the model, we refer the reader to Appendix A of
  Turbet et al. (2019)"). That appendix (*A&A* 628, A12,
  doi:10.1051/0004-6361/201935585, arXiv:1906.03527, read) runs the
  radiative transfer with "the Sun (assumed to be the host star)" at a
  fixed zenith angle of 60°, and defines `S_eff = OLR / ((1 − A_p)
  F⊕)`, `A_p` the planetary albedo under that light. Eq. 3's `S` is a
  flux of Sun-like light: the 2020 paper gives no correction for other
  stars (it applies its relations to TRAPPIST-1 in § 3.2, and says only
  that the runaway limit depends on the host star, Appendix D step 3).
  The rewrite feeds Eq. 3 the plain sum of its sources' fluxes
  (`interior::steam_interior`, the trial's `sources.total_flux()`),
  whatever their spectra, so a steam world round a cool star (the #492
  body's 2400 K M9V) reads a fit made for the Sun's light.
- **Calibration of the thickness cap** (the core's radius, legacy's
  "Calibration, no source", kept). Eq. 2 has no finite `z` once
  `R T ln(P_surf/P_t)/(M g) ≥ R_core`: the atmosphere is unbound (the
  "U-shape" of § 3.1, where "efficient atmospheric escape mechanisms
  should take place"; C.9 "does not hold for inflated atmospheres").
  The cap `z ≤ R_core` binds from `R T ln/(M g) = R_core/2`, so it also
  caps bound envelopes thicker than the core's radius. It binds in the
  paper's own grid: on the worked example's core below (0.3 M⊕,
  0.750 R⊕, 5 % water), Eq. 2 gives a bound `z` of 1.18 `R_core` at
  10 S⊕ and 1.73 `R_core` at 30 S⊕, the cap binding from about 5.7 S⊕
  (derived, with the rewrite's constants). Searched and not found: a published
  thickness or radius past which a steam envelope stops being bound, in
  Turbet et al. 2020, Aguichine et al. 2021 and Pierrehumbert 2023.
- **The worked example** (§ 3.1): "a 0.3 M⊕ pure silicate core planet
  (located just above the runaway greenhouse irradiation threshold) with
  a 5 % water-to-rock ratio … the transit radius lies around 1.2 R⊕",
  with about 2 m/s² there. With Zeng's MgSiO₃ table, the one
  the paper points to (footnote 2; 0.750 R⊕ at 0.3 M⊕, interpolated),
  Eqs. 2 and 3 give 1.23 R⊕ at
  1 S⊕ (1.22 with `x` = 0.05/1.05) and 2.05 m/s² at that radius for
  the whole planet's mass, 0.3/0.95 M⊕ (1.95 for the core alone;
  derived): a tier-1 test with a source, independent of the model's
  interior. Legacy's 1.17 R⊕ came from its own rock radius.
- **Past the fit's water shares** (a departure, documented). Appendix A
  holds `R = R_core + z` while the steam weighs "significantly lower than
  the total mass" and its base pressure stays well under about 1e10 Pa,
  where MgSiO₃ starts to compress. Aguichine et al. 2021 (*ApJ* 914, 84,
  doi:10.3847/1538-4357/abfa99, arXiv:2105.01102, read; fits on Zenodo
  4552188) cover water shares of
  0.1 to 1, 1 to 20 M⊕ and `T_irr` 400 to 1300 K, with `T_irr = T_eff
  √(R★/2a)` (Eq. 9), about 4.3 S⊕ and more round a Sun-like star: none
  of their grid sits near the runaway limit. The #492 body (12 % water
  near 1 S⊕, 5.6e5 bar of steam) is outside both papers. The rewrite
  keeps Turbet's Eqs. 2 and 3 with the clamps above; Aguichine's fits
  are left for later (an issue) for hot planets past 10 % water.
- **The climate under the envelope.** The rewrite's climate reads the
  bulk's gravity and radius (`ClimateInputs::of`). For a water world
  past the laws (formed at 355 K or more, about 3.6 S⊕ and over) that
  bulk is the dry one: Turbet's `g_core` and `R_core`, the gravity and
  radius at the interior-atmosphere boundary where Eqs. 1 to 3 and C.9
  set the surface, so not solving the climate again on the inflated
  body is no departure at the surface. A steam world formed with a
  water layer (under 355 K, as near the runaway limit) has the
  condensed water world for its bulk: its climate reads the condensed
  planet's gravity, not the core's. For the #492 body that is about
  1.78 g⊕ (4.78 M⊕ at legacy's condensed 1.64 R⊕) against 1.93 g⊕ at
  its core (4.22 M⊕ at Zeng's 1.48 R⊕), some 8 % lower (derived): a
  documented departure. The gravity at the radiating level is lower
  still (about 1.65 g⊕ for the #492 body, derived); the limiting
  outgoing radiation is "only weakly dependent on gravity"
  (Pierrehumbert 2023, after Pierrehumbert 2010), and Kopparapu et al.
  2014 move the inner edge by about −10 % to +7 % from 0.1 to 5 M⊕
  ([habitable zone](habitable-zone.md)): a few per cent at most. In
  the rewrite, `settled::steam_world` keeps the kept run's climate
  (`Settled::climate`) and solves none on the envelope.
- **The steam loss on the inflated body** (user, 2026-11-24). The
  envelope's radius depends on the water kept, which depends on the
  loss. One pass, no fixed point: the envelope is computed on the water
  handed to the loss (before it), and the energy-limited loss and the
  cosmic shoreline read that radius and the whole mass. A larger
  envelope than the water left would hold, so the loss is a deliberate
  upper bound. Turbet et al. § 3.1 support reading the transit radius
  (escape "can be very strong" at its low gravity); legacy read the
  condensed body. After that loss the steam world line is read again
  on the water kept (user, 2026-11-24): the envelope is built only if
  the kept steam still weighs at least the critical pressure, otherwise
  the planet is a runaway without an envelope. The climate is not
  solved again, so its air holds more water vapour than the water kept
  (a documented departure). The rewrite's steps, in
  `settled::steam_world` after the boil loop: the line
  (`record_hides_surface`) on the kept run's water, its climate's H₂O
  and temperature; past it, `interior::steam_interior` on the ground's
  surface water (the water handed to the loss); one more pass from the
  same water entry (`water::water` for the light, `water::boiled` with
  the first verdict), its `planet.water.steam-lost` and the cosmic
  shoreline reading the inflated body (`Body::exposed`, through
  `bare::surface_pass` and `bare::bare_air`), its `boiled` the kept
  run's node; the line read again on that pass's water and air, with
  the climate's temperature; past it again, `steam_interior` on the
  water kept, over the rock that pass left. Two
  `planet.climate.hides-surface` nodes, the last in
  `Settled::hides_surface`.

#### Test windows

| Test | Bound | From |
| --- | --- | --- |
| Earth's air, an Earth ocean, 1.0 AU | does not boil | Earth keeps its oceans (observation) |
| Same, at Venus's orbit, 0.723 AU (1.91 S⊕), `a_wet_venus_boils` | boils | Kasting 1988 § 3a, Fig. 7 (a wet Venus runs away); past Kopparapu et al. 2014's limit, 1.107 S⊕ (`habitable::runaway_flux`) |
| A trace of water (1e-4 bar, 1e-9 of the mass) | does not boil | Kasting 1988 § 3a, Boukrouche et al. 2021 Fig. 9 (property) |
| Thick steam (≥ 260 bar) past the limit; 30 % water at 0.7 AU, `a_thick_steam_runaway_settles_past_the_end_of_the_plateau` | 1500 to 2500 K, a steam world | Kasting 1988 (over 1500 K for a full ocean), Goldblatt et al. 2013 (a stable steam state from about 1600 K), Boukrouche et al. 2021 § 4.3 (1835 K under 311 W/m² incoming, 2364 to 2451 K under 1343 W/m², 260 bar) |
| Turbet's worked example | 1.2 R⊕ ± 5 % on Zeng's 0.750 R⊕ core ("around"; the fit errs by up to about 10 %) | Turbet et al. 2020 § 3.1 |
| A steam world's radius | over its condensed radius, inside the structures of Turbet's § 2.1 (27 bar to 270 kbar), past § 2.2's 27 kbar | Turbet et al. 2020 § 3.1 (property) |
| The #492 body, round a 2400 K star at 1 S⊕, `the_492_super_earth_is_a_steam_world` | steam world, over 1e5 bar, over 1000 K, the last shell steam, the radius over its dry core's (outside Turbet's grid its condensed twin is larger, #684), the last steam loss read on the envelope's radius | the class is the model's; 1.61 R⊕ and 1679 K are the old code's output (tier-2 net only) |
| A wet Venus under an XUV fluence of 1e14 J/m², `thin_steam_over_rock_stays_a_runaway` | a runaway under 220.64 bar of steam, no envelope | Pierrehumbert 2023 § 1 (no surface hides under the critical pressure); legacy `thin_steam_over_rock_stays_a_runaway_greenhouse` |
| A regulated Earth at 1.0 AU, `a_regulated_climate_with_a_wet_balance_does_not_boil` | does not boil | Earth keeps its oceans (observation); the thermostat's air has a wet balance |
| F0V (7220 K) at 1.2963 S⊕, a hundredth of the N₂, half Earth's mass, `a_boil_that_escapes_all_its_water_ends_dry` | the first run boils, regulated, under the runaway flux (1.2964 S⊕) | property: absorbed light over the saturated column's peak (at Earth's mass 291.0 over 289.9 W/m²; at a tenth of the N₂, Earth's argon raises the peak to 292.3, #694), Kasting 1988, Goldblatt et al. 2013. Since the cloud albedo's refit (#705, 0.322) an Earth-mass planet there has a Bond albedo of 0.3515 and absorbs 286.0 W/m², under any peak the N₂ cut gives; half Earth's mass, whose lower gravity lowers the peak, boils at 282.7 W/m² (0.7 Earth masses does not); legacy's 1.28 S⊕ lies under the model's threshold; "kept 0" is the model's escape (tier-2 net) |
| Same, under an XUV fluence of 1e14 J/m², water sized to leave 0.03 bar, `a_boil_whose_leftover_recondenses_keeps_its_ocean` | boils, then keeps an ocean under 1e-3 of its water | property: the leftover's own wet balance (Hamano et al. 2013's type I); the fluence is a test input so that the planet stays dry enough to sit in the band (2e14 until the air's argon, #694, took the first run out of it; the N₂ cut to a hundredth keeps it there with the stellar spectra of #655, half Earth's mass since #705) |

The 1500 to 2500 K window extrapolates Boukrouche et al. 2021, who stop
at 295 bar and find the plateau's end constant past it, to columns of
1e5 bar.

## Life

`life` decides from the climate whether life arose, and `oxygenate` gives
the air of a world with oxygenic life its oxygen. Life is possible when
the surface liquid is water now and the system is at least `LIFE_MIN_GYR`
(1.0 Gyr, calibration, no source: the spec's "order of a billion years")
old. It then arises when the last draw of the inputs, `life_draw`, is
below `LIFE_CHANCE` = 1/3.

`LIFE_CHANCE` is a game choice, set with the user on 2026-11-12. No
measured rate exists: the only world known to have life is Earth, and one
case gives no probability. Nothing in the model is calibrated on it, and
it can be changed without touching any physics.

Life has oxygenated the air once the system is `OXYGEN_AFTER_GYR` old,
2.2 Gyr, Earth's Great Oxidation Event about 2.4 Ga (Lyons, Reinhard &
Planavsky 2014, doi:10.1038/nature13068: 2.3 to 2.4 Ga, checked in the
authors' EGU 2014 abstract, the Nature text not read). The oxygenated air holds O₂ at `0.27` of its
N₂ (calibration on Earth: 0.21 over 0.78) and no methane, which the
oxygen oxidises.

`Surface.gases` is that air after life, but `Surface.climate` was solved
with the air before it, methane included: removing a greenhouse gas
would cool the surface a little, and the temperature is not solved
again. This simplification is dropped in 2d-4, which solves the climate
again after oxygenation (spec 2d, #659); until then it holds. Earth's
O₂ comes from life, so the climate of 2d-3a runs on an air without it.

| Case | Result |
|---|---|
| Earth, `life_draw` 0, 4.6 Gyr | life, O₂ between 0.15 and 0.3 bar, no CH₄ |
| Earth at 0.5 Gyr | no life |
| Earth at 1.5 Gyr | life, no oxygen yet |
| Earth, `life_draw` 0.99 | habitable, lifeless |
| Venus, Mars | no life, whatever the draw (no liquid water) |

`life_draw` is appended as the last draw of `draw_inputs`, so every
earlier input of a body keeps its value.

## The chain and the surface classes

The rewrite chains the first part of this derivation in
`system::planet::bare::bare`: composition, interior, the boil-off of a
mini-Neptune's envelope down to its ceiling (new to the rewrite, see
[Boil-off and core-powered mass loss](#boil-off-and-core-powered-mass-loss-study-30)),
XUV fluence, escape, heat, then for a body that has lost its envelope the
water, the regime, the core and the air, before any climate. The model is private until the
climate step (2d) and nothing generated changes. Where it departs from the
legacy, beyond the corrections named in each section: the rewrite's
interior is heavier (cooling times about a fifth longer, a deeper seafloor),
which moved three calibrated values (`THERMAL_DYNAMO_END` 0.075 to 0.055,
`INNER_CORE_START` 1.0 to 0.82, `SEAFLOOR_HALF_PRESSURE` 30 to 37 kbar),
kept `CORE_FROZEN` 2.25, set `CORE_HEAT_CAPACITY` to 840, split the dipole
into two sourced factors (`F^(1/3)`, `1/7`), a definition (`1/√2`) and
a calibrated remainder (0.82), converted the ice volatiles from a mass to a
mole basis and gave `WET_SO2_SHARE` its 2.2e-9.

`planet_model` is the single place that chains the derivation, in this
order: the interior from the drawn inputs, the XUV fluence, the envelope
escape (the interior is derived again from what is left), the internal
heat, then, if the envelope left is under `MIN_ENVELOPE` (0.1 % of the
mass), the water kept and the runaway check, the tectonic regime, the
core and dynamo, the air (`outgassed`, `atmosphere`,
`methane_photolysis`, `cold_trap`, then `photochemistry`, which reads
the cold trap's air), the climate and
surface liquid, and life with the oxygen it makes. If a wet surface
has no wet balance (`climate::boils`: its absorbed light and internal
heat exceed the moist limit; Goldblatt et al. 2013,
doi:10.1038/ngeo1892), the body has run away after all, in one pass
with no loop: the steps from the regime on run once more from its
steam (`regime::steam`, the water left after energy-limited escape, in
the air), flagged `boiled`, on the body that water leaves
(`planet::after_steam`: its mass, layers and heat derived again). If
escape leaves water and that leftover has a wet balance of its own (too
little vapour to hold the moist limit), it condenses again and a third
run keeps it as an ocean (`boiled`, no runaway; the water cause tree
says so in a `boiled_recondensed` note that carries the water lost,
issue #517); otherwise the steam run stands, dry when escape took all the
water. There is no temperature condition, so a boil whose dry air is
under 647 K still ends dry (issue #442). A body
that keeps an envelope has no surface: `PlanetModel.surface` is `None`.
Otherwise the `Surface` holds the water, regime, core, final gases,
climate and life, and `classify` reads the class off it. Nothing derives
from the class.

The first rule that holds wins:

| Order | Class | Rule |
|---|---|---|
| 1 | `hot-jupiter`, `gas-giant`, `ice-giant` | giant rules of part 1 |
| 2 | `mini-neptune` | envelope of at least 0.1 % and radius of at least 1.6 R⊕ |
| 3 | `steam-world` | in water runaway, at least 220.64 bar of steam (water's critical pressure) and a surface at 647 K or more (`climate::hides_surface`) |
| 4 | `lava-world` | surface liquid is lava (at least 1673 K, no water kept) |
| 5 | `runaway-greenhouse` | in runaway (the insolation is over the runaway limit, or the climate boiled the ocean) and the air is over 1 bar |
| 6 | `hydrocarbon-world` | surface liquid is hydrocarbons and the air is over methane's triple-point pressure (0.117 bar) |
| 7 | `ocean-world` | surface liquid is water and at least 10 Earth oceans are kept |
| 8 | `ice-world` | at least 5 % water and ices |
| 9 | `volcanic-world` | a solid surface in the heat-pipe regime, none of the classes above, no ice world |
| 10 | `super-earth`, `rocky` | radius of at least 1.25 R⊕, rocky below |

A `steam-world` (issue #492) is a runaway whose water stands as a deep
supercritical column over the rock. Above water's critical point there
is no liquid–gas boundary, so no surface shows, lava or rock. The line
of ten times the critical pressure (`STEAM_WORLD_MIN_BAR`) was a
calibration: in the census no runaway holds between 1e3 and 1e5 bar of
steam, so it cuts through an empty band. The rewrite takes the critical
pressure itself (study 26). A wet planet is never one.
Once the surface is known, the chain derives the interior again
(`interior::steam_interior`): the water layer gives way to a `Steam`
layer of the water kept, on top of the iron and rock. Its thickness
comes from Turbet et al. 2020 (A&A 638, A41, arXiv:1911.08878): Eq. 2
in the form of their Appendix C, for an isothermal envelope in
hydrostatic balance; the printed Eq. 2 is the same law (study 26).

```text
1/z = M g / (R T ln(P_surf / P_transit)) − 1/R_core
P_surf = f/(1−f) g² / (4πG),  P_transit = 0.1 Pa
```

The core is the iron and rock, and `f` is the steam's share of core and
steam. `T` comes from their Eq. 3 and Table 2, a polynomial in
`log₁₀ f`, `log₁₀ g` and `log₁₀ S`, with its inputs clamped:

- `S` to 1 to 30 S⊕;
- `g` to 0.2 to 6 g⊕;
- the share to the shares whose steam weighs 27 bar to 270 kbar, the
  structures the fit was made on (§2.1).

§2.2 calls Eq. 3 valid only up to 27 kbar, but clamped there it puts a
steam world below its condensed twin on Zeng's core (1.59 against
1.64 R⊕); clamped at 270 kbar it is above it there (1.68 R⊕), though
not on the rewrite's own core (1.643 against 1.655 R⊕, #684). The paper's own example comes back
1.17 R⊕ on legacy's rock (a 0.3 M⊕ silicate core with 5 % water;
"around 1.2 R⊕"), 1.23 R⊕ on the silicate core the paper used (study
26).
Density and gravity count the whole mass. The climate is not solved
again under the new gravity, and the cause trees replay the surface on
the interior from before the steam.

The `volcanic-world` is Io's class: Moore & Webb 2013
(doi:10.1038/nature12424) model its resurfacing as heat-pipe tectonics,
magma rising through the lithosphere's conduits. It takes only a body
that would otherwise be `rocky` or `super-earth`: silicate volcanism is
Io's, while an icy tidally heated body (Europa, Enceladus) would at most
be cryovolcanic, which the model does not describe, so an `ice-world`
stays one whatever its regime. It also comes after the classes that name
what covers the surface (magma, a dense greenhouse air, a sea). Its
interim base look is `volcanic`, as for any heat-pipe body.

Two constants are calibrations. `OCEAN_WORLD_OCEANS` is 10 Earth oceans:
Cowan & Abbot 2014 (doi:10.1088/0004-637X/781/1/27) study how much water
drowns all land; ten is a round figure above the few oceans their basins
hold, not a value from the paper. `DENSE_AIR_BAR` is 1 bar, for the
runaway greenhouse (Venus has 90 bar; a runaway that lost its air is a
bare rock). The hydrocarbon world needs its air above methane's
triple-point pressure (`METHANE_TRIPLE_BAR`, 0.1169 bar, NIST), under
which no liquid methane is stable; it took the 1 bar until part 4, when
Titans drawn with 0.6 to 1 bar kept lakes but were classed ice worlds.
It keeps a cold body with a trace of methane over its ice under a thin
air an ice world. Titan has 2.1 bar in the model.

The case of issue #492 (4.78 M⊕, 12 % water, an M9V star at 0.018 AU)
is a `steam-world`: 4.1e5 bar of steam over a surface at 1679 K, and
1.700 R⊕ (1.641 with its water condensed).

The Solar System classifies as itself: Venus `runaway-greenhouse`, Earth
and Mars `rocky`, Titan `hydrocarbon-world`, Ganymede and Europa
`ice-world`, Callisto `ice-world` with no methane, the giants as in
part 1. A 1 M⊕ planet of 30 % water at
1 AU, with the volatiles `draw_inputs` gives its rock and ice (no
scatter), is an `ocean-world` at 288 K under 0.4 bar. With 1 % water
(about 40 oceans) at 1 AU, under the 20 bar of nitrogen its ice brings,
it keeps its ocean under 330 K: the moist limit does not depend on the
nitrogen. The Earth with 0.03 % of water round an F0V star at 1.24 S⊕
(inside that star's runaway limit of 1.30 S⊕, but 297 W/m² absorbed,
over the plateau) boils, its steam escapes to nothing and it ends a dry
`runaway-greenhouse` (630 K under 65 bar of CO₂, 680 K under 93 bar);
with just enough water that 0.03 bar survives escape, that leftover
condenses again into a small ocean at 306 K. The model is
finite and non-negative on mass 0.01 and 20 M⊕, insolation 1e-3 and 1e4,
age 0.5 and 13 Gyr, M5V and A0V stars.

## From the model to the map

`planet::interim` turns the model's air into the sky and the climate of
the world the planet renders (part 3a; issue #424 replaces it):

- The sky (`sky_of`) gives the air's colour and aerosols, not its
  thickness: the viewer already scales the Rayleigh depth with the
  surface pressure, so counting the column here too made dense airs opaque
  grey. It is Earth's for an air of 0.8 to 1.25 bar; otherwise Earth's
  Rayleigh colour, haze and haze colour yellowed toward Venus's
  (`looks/caustic.toml`) from 10 bar (wholly at 100 bar), with no haze
  under the airless limit. A methane haze of Titan's orange
  appears where CH4 is over 1 % of an air thicker than 0.5 bar. Clouds
  follow the air: none under the airless limit; water where water or
  hydrocarbons are liquid or the vapour reaches 1 mbar; sulfuric where
  SO2 reaches 1 mbar; dust on a dry air under 0.1 bar.
- The pressure the world is drawn with is the model's total, clamped to
  the request's limits; the greenhouse of the preset is zero, because the
  temperature is aimed at the model's instead.
- The sea level is where the model's oceans fill the relief (on a dry
  world, with none at its surface, only the datum of its relief: no open
  water anywhere, `look.rules.dry`; on a hydrocarbon world, the level
  that leaves 1 % of the surface liquid, measured on the relief the
  biomes see, its craters included, issue #482); the
  temperature offset (grid offset) is the shift that brings the grid's
  mean to the model's surface temperature. Both are set when the world is
  generated and are only in `world.json`; the drawn parameters of the
  body hold the values the resolution drew.
- Life: a planet where life did not arise renders with the lifeless
  remap (no vegetation biomes).
- Calibrations, no source: the airless, cloud and dust pressure limits,
  the yellowing range, the methane haze share and the 99 % land share of a
  hydrocarbon world. Sea level and temperature are calibrated against the
  model, not measured on a globe.

## Census

This census was run on legacy, at lot 3C-2, with the lava-world
threshold at 1400 K; the class table (*The chain and the surface
classes*) now takes 1673 K (`LAVA_TEMPERATURE`), so its `lava-world`
count and median are legacy's, not the rewrite's.

Measured, not tuned: `cargo run --release -p mapgen --example planet_census
-- 500` draws systems of seeds 1 to 500 and derives every planet through
`body_model`. The medians of temperature and pressure are over the
planets with a surface (`NaN` for the classes that have none). The output
below is the one of the climate pass, lot 3C-2 (issue #523): the
spectral column (correlated-k lines, MT_CKD continua, CO₂ pairs, cloud
greenhouse, falling humidity), the near-infrared absorption, the outer
edge from the column's own maximum greenhouse and the steam runaway's
mass loss. The census counts the runaways by their source
(`Water::boiled`) and by decade of their steam column, with the steam
worlds apart, the wet planets left past 647 K because their boil was not
kept, the snowballs by insolation and the methane worlds. It took 428 s
on four threads (`RAYON_NUM_THREADS=4`; about 0.85 s a
system, the column's cost is issue #593).

```text
500 systems, 1552 planets

base preset of the worlds (planets)
barren           293   18.9 %
caustic           55    3.5 %
frozen           553   35.6 %
gas-giant         65    4.2 %
hot-jupiter       45    2.9 %
ice-giant        453   29.2 %
steam-world        2    0.1 %
terran            43    2.8 %
volcanic          43    2.8 %

planets per radius bin (Fulton et al. 2017 bins, R_earth)
    <1    41.0 %  (636)
 1-1.5    19.3 %  (300)
 1.5-2     6.2 %  (97)
   2-3    12.6 %  (196)
   3-4     7.6 %  (118)
   4-6    10.0 %  (155)
  6-10     2.1 %  (33)
 10-22     1.1 %  (17)
   >22     0.0 %  (0)

medians per class
class               planets  age Gyr  mass M⊕    a AU    CMF    water  envelope     T K      P bar
gas-giant                65     4.39   48.726   2.165  0.325   0.0267    0.9000     NaN        NaN
hot-jupiter               2     8.72  225.725   0.036  0.432   0.0000    0.9289     NaN        NaN
hot-neptune              43     5.28   13.503   0.030  0.286   0.0007    0.1791     NaN        NaN
hydrocarbon-world         7     1.90    0.137   3.894  0.303   0.2794    0.0000     101      2.500
ice-giant               160     5.63   14.366   0.700  0.300   0.2496    0.1805     NaN        NaN
ice-world               449     5.43    0.380   0.373  0.311   0.3454    0.0000      70      0.018
lava-world               15     3.18    0.674   0.034  0.300   0.0000    0.0000    1777      0.000
mini-neptune            293     5.26    5.298   0.315  0.307   0.2427    0.0193     NaN        NaN
ocean-world               6     3.78    1.841   0.065  0.333   0.0019    0.0000     288     10.009
rocky                   399     5.38    0.283   0.048  0.309   0.0000    0.0000     338      0.000
runaway-greenhouse       63     5.22    1.596   0.141  0.317   0.0000    0.0000     650     52.480
steam-world               2     6.60    3.159   0.134  0.492   0.4002    0.0000    1796 630440.205
super-earth              27     5.12    2.840   0.042  0.316   0.0002    0.0000     596      0.622
volcanic-world           21     3.74    0.189   0.059  0.327   0.0000    0.0000     452      0.000

class mix (1552 planets)
ice-world             28.9 %  (449)
rocky                 25.7 %  (399)
mini-neptune          18.9 %  (293)
ice-giant             10.3 %  (160)
gas-giant              4.2 %  (65)
runaway-greenhouse     4.1 %  (63)
hot-neptune            2.8 %  (43)
super-earth            1.7 %  (27)
volcanic-world         1.4 %  (21)
lava-world             1.0 %  (15)
hydrocarbon-world      0.5 %  (7)
ocean-world            0.4 %  (6)
hot-jupiter            0.1 %  (2)
steam-world            0.1 %  (2)

planets with a surface: 983
no giant, surface hidden by an envelope: 6 (0.4 %)
  ice-world              2
  super-earth            4

surface liquid (983 planets with a surface)
Hydrocarbons           0.7 %  (7)
Lava                   1.5 %  (15)
Nitrogen               4.0 %  (39)
None                  89.4 %  (879)
Water                  4.4 %  (43)

tectonic regime (983 planets with a surface)
Episodic              21.3 %  (209)
HeatPipe               3.2 %  (31)
Plates                34.1 %  (335)
Stagnant              41.5 %  (408)

surface age, Ga, by class and regime (planets with a surface)
class               regime         n  median    mean
hydrocarbon-world   Stagnant       7   0.580   0.591
ice-world           Plates       257   0.351   0.332
ice-world           Stagnant     190   0.268   1.396
lava-world          Episodic       9   0.443   0.480
lava-world          HeatPipe       1   0.000   0.000
lava-world          Stagnant       5   4.600   3.603
ocean-world         HeatPipe       1   0.000   0.000
ocean-world         Plates         5   0.507   0.464
rocky               Episodic     130   0.545   0.544
rocky               Plates        63   0.376   0.379
rocky               Stagnant     206   4.569   3.625
runaway-greenhouse  Episodic      55   0.521   0.520
runaway-greenhouse  HeatPipe       8   0.001   0.001
steam-world         Episodic       2   0.000   0.000
super-earth         Episodic      13   0.555   0.571
super-earth         Plates        10   0.504   0.450
volcanic-world      HeatPipe      21   0.000   0.000
all                              983           1.283

runaways: 383 by the insolation limit, 0 boiled; of the runaway-greenhouse class: 63 by the limit, 0 boiled
boils whose leftover water condensed again: 0
wet, not in runaway and past 647 K: 0
runaways by steam column (other classes / steam worlds, median K), steam-world line 2206.3999999999996 bar:
        <1 bar:  375 /    0  502 K
      1-10 bar:    2 /    0  410 K
    10-1e2 bar:    3 /    0  2148 K
   1e2-1e3 bar:    1 /    0  1741 K
   1e3-1e4 bar:    0 /    0  NaN K
   1e4-1e5 bar:    0 /    0  NaN K
     >=1e5 bar:    0 /    2  1796 K

life: 1.8 % of planets with a surface (18), 1.5 % oxygenated (15)

snowballs / planets with liquid water, by insolation (Earth's = 1)
  S     >1.1:    0 /    0
  S  0.9-1.1:    0 /    4
  S  0.7-0.9:    0 /   11
  S  0.5-0.7:    6 /    9
  S 0.36-0.5:    1 /   11
  S    <0.36:    0 /    8

methane: 0 planets under 150 K of equilibrium warmed past 273 K with methane; 7 hydrocarbon liquids (least CH4 0.0212 bar), 0 under too little methane to condense

radius histogram, period under 100 days (818 planets)
 0.00-0.25      0.0 %  (0)
 0.25-0.50      8.3 %  (68)
 0.50-0.75     21.4 %  (175)
 0.75-1.00     19.1 %  (156)
 1.00-1.25     13.4 %  (110)
 1.25-1.50      6.7 %  (55)
 1.50-1.75      3.3 %  (27)
 1.75-2.00      2.0 %  (16)
 2.00-2.25      2.8 %  (23)
 2.25-2.50      2.7 %  (22)
 2.50-2.75      3.5 %  (29)
 2.75-3.00      1.1 %  (9)
 3.00-3.25      0.9 %  (7)
 3.25-3.50      2.0 %  (16)
 3.50-3.75      1.6 %  (13)
 3.75-4.00      1.5 %  (12)
 4.00-4.25      0.9 %  (7)
 4.25-4.50      1.3 %  (11)
 4.50-4.75      1.3 %  (11)
 4.75-5.00 +    6.2 %  (51)

classes, period under 100 days (818 planets)
gas-giant              0.9 %  (7)
hot-jupiter            0.2 %  (2)
hot-neptune            5.3 %  (43)
ice-giant              5.7 %  (47)
ice-world             14.1 %  (115)
lava-world             1.8 %  (15)
mini-neptune          16.0 %  (131)
ocean-world            0.7 %  (6)
rocky                 43.2 %  (353)
runaway-greenhouse     6.6 %  (54)
steam-world            0.2 %  (2)
super-earth            2.7 %  (22)
volcanic-world         2.6 %  (21)
```

What it shows, plainly (against the census of lot 1, the grey column):

- **Steam worlds** are 0.1 % of the planets (2 of 1552), both far over
  100 000 bar of steam (median 1796 K at the surface, 630 440 bar; 2032 K
  on the grey column, whose steam had no window). They are the only
  runaways above 1e5 bar; the 381 other runaways hold under 1e3 bar (375
  of them under 1 bar of steam) and stay `runaway-greenhouse` or another
  class.
- **Runaway greenhouses** are 4.1 % (63, 60 before), all by the
  insolation limit (none boiled): median 650 K under 52.5 bar (842 K
  on the grey column). No boil condensed again and no wet planet is left past 647 K
  out of a runaway.
- **Water** is the surface liquid of 4.4 % of the planets with a
  surface (43, 29 before), and **nitrogen** (a liquid since lot 3C-1)
  of 4.0 % (39). The ice worlds' median temperature is 70 K (91 K
  before).
- **Snowballs** are 7, all between 0.36 and 0.7 S⊕ (6 of them between
  0.5 and 0.7), against 43 planets with liquid water; none of the 15
  wet planets above 0.7 S⊕ drew one.
- **Radius valley.** With the close-in planets (818 with a period under
  100 days) the histogram dips at 1.50-2.00 R⊕ (3.3 % and 2.0 %)
  between the rocky peak at 0.5-1.0 R⊕ and a second hump at 2.0-3.0 R⊕
  (2.8, 2.7 and 3.5 % in the bins from 2.00 to 2.75), and 39.6 % of all
  planets are above 1.5 R⊕. The model was not tuned to produce a valley.
- **Mini-Neptunes** are 18.9 % (293) and hot Neptunes 2.8 % (43): the
  class mix counts the envelope classes the radius histogram reads.
- **Lava worlds** are 1.0 % (15), airless dry rocks at a median 1777 K,
  and volcanic worlds 1.4 % (21), all in the heat-pipe regime.
- **Hydrocarbon worlds** are 0.5 % (7), small icy bodies (median
  0.137 M⊕) far from their stars (median 3.9 AU) under 2.5 bar at 101 K;
  none is a methane world warmed past 273 K, and none holds too little
  methane to condense (the least, 0.021 bar).
- **Ocean worlds** are 0.4 % (6), held at 288 K by their thermostat under
  a median 10.0 bar.
- **Surfaces hidden by an envelope**: 6 planets that are no giant (0.4 %),
  2 ice worlds and 4 super-Earths.
- **Tectonic regimes** over the 983 planets with a surface: plates
  34.1 %, stagnant lid 41.5 %, episodic 21.3 %, heat pipe 3.2 %
  (unchanged).
- **Surface liquid**: none for 89.4 %, water 4.4 % (43), nitrogen 4.0 %
  (39), lava 1.5 % (15) and hydrocarbons 0.7 % (7).
- **Life** arises on 1.8 % of the planets with a surface (18, 11
  before), 1.5 % with oxygen (15, 7 before): more planets keep water.

## Sources

Every reference cited above, in alphabetical order. The venue and the
DOI are the ones the text gives; an entry without them is cited without
them above. How far each one was read (abstract only, as quoted by
another paper, from memory) is said where it is used.

- Aguichine, Mousis, Deleuil and Marcq 2021, *ApJ* 914, 84,
  doi:10.3847/1538-4357/abfa99, arXiv:2105.01102, irradiated ocean
  planets, their grid and Eq. 9 (read; fits on Zenodo, record
  4552188).
- Albarède 1998, Earth's potassium possibly off by a factor of two (as
  Marty 2012, p. 15, quotes it).
- Allègre, Hofmann and O'Nions 1996, *Geophys. Res. Lett.* 23, 3555,
  doi:10.1029/96GL03373, about half of Earth's ⁴⁰Ar still in the solid
  Earth (abstract read).
- Anderson et al. 2011, doi:10.1126/science.1211001, Mercury's
  dipole.
- Anderson, Mousis and Ronnet 2021, *PSJ*, arXiv:2102.03230, Titan's
  and Enceladus's building blocks in Saturn's circumplanetary disc
  (abstract and § 1-3 read).
- Andrews and Williams 2007, *ApJ* 659, 705, the average disc
  temperature profile (as Öberg et al. 2011 adopt it).
- Angus, de Reuck et al. 1979, nitrogen's triple point, through
  the NIST Chemistry WebBook.
- Arevalo, McDonough and Luong 2009, *Earth Planet. Sci. Lett.* 278,
  361, potassium of the silicate Earth, 280 ± 60 ppm at 1σ (as Marty
  2012, p. 15, quotes it), and the continents' ⁴⁰Ar.
- Atake and Chihara 1976, ethane's triple point (90.341 K) and
  enthalpy of fusion (0.583 kJ/mol), through the NIST Chemistry
  WebBook.
- Atreya, Donahue and Kuhn 1978, *Science* 201, 611,
  doi:10.1126/science.201.4356.611, Titan's N₂ from NH₃.
- Atreya, Trainer, Franz, Wong et al. 2013, *Geophys. Res. Lett.* 40,
  5605, doi:10.1002/2013GL057763, Mars's ³⁶Ar/³⁸Ar and its argon loss
  (abstract read).
- Avice and Marty 2020, *Space Sci. Rev.* 216, 36,
  doi:10.1007/s11214-020-00655-0, arXiv:2003.11431, the air's ⁴⁰Ar/³⁶Ar
  (read).
- Avice, Parai, Jacobson, Labidi et al. 2022, *Space Sci. Rev.* 218,
  60, doi:10.1007/s11214-022-00929-9, arXiv:2210.03176, Venus's argon,
  Table 1 (read).
- Bailey, Ahlsved and Meadows 2011, arXiv:1102.2554, Titan's surface
  albedo in the methane windows (read).
- Balsiger, Altwegg, Bar-Nun, Berthelier et al. 2015, *Sci. Adv.* 1,
  e1500377, doi:10.1126/sciadv.1500377, argon in comet 67P (read in
  PMC4643765).
- Barnes, Mullins, Goldblatt, Meadows, Kasting and Heller 2013,
  *Astrobiology* 13, 225, doi:10.1089/ast.2012.0851, arXiv:1203.5104v2,
  tidal heat as a runaway trigger, § 2 (read).
- Baraffe et al. 2015, the luminosity of low-mass stars at the ZAMS.
- Barrett and Meyer 1965, *J. Chem. Phys.* 42, 107,
  doi:10.1063/1.1695654, the argon-nitrogen phase diagram (abstract
  read).
- Bell, Wronski, Quoilin and Lemort 2014, *Ind. Eng. Chem. Res.* 53,
  2498, doi:10.1021/ie4033999, CoolProp, whose saturation ancillaries
  give argon's liquid vapour pressure.
- Berger, Schlieder, Huber and Barclay 2023, arXiv:2302.00009, the
  radius valley's slope with insolation and stellar mass (abstract read).
- Bertrand et al. 2022, *Icarus* 373, 114764, Triton.
- Biver et al. 2022, *Comets III*, arXiv:2207.04800, Table 3.
- Blakley et al. 2024, *Planet. Space Sci.*,
  doi:10.1016/j.pss.2024.105863, arXiv:2403.09917, NH₃'s sublimation
  pressure at 80 to 120 K.
- Bono et al. 2019, the age of Earth's inner core.
- Borysow and Frommhold 1987, *Astrophys. J.* 318, 940,
  doi:10.1086/165426, the CH₄–CH₄ pairs of HITRAN's `CH4-CH4_2011.cia`
  (through HITRAN's references, not read).
- Boukrouche, Lichtenberg and Pierrehumbert 2021, *ApJ* 919, 130,
  doi:10.3847/1538-4357/ac1345, arXiv:2107.14150, the end of the
  plateau (Fig. 9) and the post-runaway equilibrium (§ 4.3; read).
- Budyko 1969, *Tellus* 21, 611, doi:10.3402/tellusa.v21i5.10109, the
  ice cover's edge (read).
- Bullock and Grinspoon 2001, doi:10.1006/icar.2000.6570,
  Venus's SO₂.
- Buratti et al. 2017, *Icarus* 287, 207, doi:10.1016/j.icarus.2016.11.012,
  Pluto's Bond albedo (read in arXiv:1604.06129).
- Carlson 1999, *Science* 283, 820,
  doi:10.1126/science.283.5403.820.
- CERES Science Team 2018, *CERES_EBAF_Ed4.0 Data Quality Summary*,
  Table 6-1 and § 4.4, Earth's outgoing longwave 2005-2015 and its
  uncertainty (read; study 32).
- Charette and Smith 2010, *Oceanography* 23(2), 112,
  doi:10.5670/oceanog.2010.51, Table 1, the volume and area of Earth's
  ocean (read).
- Charnay et al. 2013, doi:10.1002/jgrd.50808.
- Charnay, Forget, Tobie, Sotin and Wordsworth 2014, "Titan's past and
  future: 3D modeling of a pure nitrogen atmosphere and geological
  implications", *Icarus* 241, 269, doi:10.1016/j.icarus.2014.07.009
  (arXiv:1407.1791, read for study 38: §2, §4.2, §4.3, Table 2).
- Chen, Aziz and Lim 1971, *Can. J. Phys.* 49, 1569,
  doi:10.1139/p71-183, the vapour pressure of solid argon (record only).
- Chistikov et al. 2019, *J. Chem. Phys.* 151 (19),
  doi:10.1063/1.5125756, the N₂–N₂ roto-translational pairs of HITRAN's
  `N2-N2_2021.cia` (through HITRAN's references, not read).
- Christensen 2006, doi:10.1038/nature05342, the deep dynamo.
- Christensen 2010, doi:10.1007/s11214-009-9553-2, the energy-flux
  scaling of the field.
- Christensen, Holzwarth and Reiners 2009,
  doi:10.1038/nature07626.
- CIAAW, *Abridged standard atomic weights* 2024, argon's 39.95 g/mol
  (as radiative-transfer.md reads it), and the potassium page of
  ciaaw.org, potassium's standard atomic weight 39.0983(1) (read for
  study 29).
- CoolProp, `dev/fluids/Argon.json` (read 2026-11-25): Tegeler et al.
  1999's triple and critical points and the ancillary `pS` of argon's
  liquid vapour pressure (its largest error 0.0077 %).
- Cooper et al. 2001, *Icarus* 149, 133, doi:10.1006/icar.2000.6498.
- Courtin, Gautier and McKay 1995, Titan's collision-induced
  bands.
- Coustenis et al. 2010, doi:10.1016/j.icarus.2009.11.027, Titan's CO₂.
- Cowan and Abbot 2014, doi:10.1088/0004-637X/781/1/27, Table 1:
  Earth's ocean as a share of its mass.
- Cruikshank et al. 1993, *Science* 261, 742, Triton's CO₂ ice.
- Dai 2006, *J. Climate* 19, 3589, doi:10.1175/JCLI3816.1, Earth's
  surface relative humidity (abstract read).
- de Kok et al. 2007, Titan's CO₂ from Cassini CIRS.
- Dello Russo et al. 2016, cometary abundances, as updated in Biver et
  al. 2022.
- Domalski and Hearing 1996, the enthalpy of melting of ethane (2.79
  kJ/mol at 89.5 K), through the NIST Chemistry WebBook.
- Dorn, Noack and Rozel 2018, *A&A* 614, A18,
  doi:10.1051/0004-6361/201731513, outgassing on stagnant-lid
  super-Earths (abstract, through study 22).
- Douté et al. 2001, *Icarus* 149, 107, Io's SO₂ frost.
- Drii and Rabinovich 1966, argon's liquid vapour pressure (Antoine fit,
  83.78 to 150.72 K), through the NIST Chemistry WebBook (read).
- Driscoll and Barnes 2015, doi:10.1089/ast.2015.1325.
- Edejer and Thodos 1967, *J. Chem. Eng. Data* 12(2), 206,
  doi:10.1021/je60033a014, the Antoine fit of liquid nitrogen's vapour
  pressure, 63.14 to 126 K, through the NIST Chemistry WebBook; replaced
  by Span et al. 2000, quoted for the comparison.
- Erkaev et al. 2007, energy-limited escape.
- Erkaev, Scherf, Thaller, Lammer, Mezentsev, Ivanov and Mandt 2020,
  *MNRAS*, arXiv:2010.12342, Titan's N₂ escape and Glein 2015's mass
  balance (read).
- Farcy, Arevalo and McDonough 2020, *J. Geophys. Res. Solid Earth*,
  doi:10.1029/2020JB020245, the K/U of the MORB source (abstract read).
- Fegley and Prinn 1989, doi:10.1038/337055a0, Venus's SO₂ and
  calcite.
- Feistel and Wagner 2007, *Geochim. Cosmochim. Acta* 71, 36, the
  sublimation pressure of water ice, as Lisse et al. 2021 reproduce it.
- Finenko et al. 2022, *Astrophys. J. Suppl.* 258, 33,
  doi:10.3847/1538-4365/ac36d3, the N₂–CH₄ pairs of HITRAN's
  `N2-CH4_2024.cia` (through HITRAN's references, not read).
- Foley, Bercovici and Landuyt 2012, tectonics against
  planet size.
- Fonseca and Lobo 1989, NH₃'s triple-point pressure, through the
  NIST Chemistry WebBook.
- Fortes 2012, Titan's interior models (their water shares as Erkaev et
  al. 2020, Table 2, tabulate them).
- Fray and Schmitt 2009, *Planet. Space Sci.* 57, 2053,
  doi:10.1016/j.pss.2009.09.011, sublimation pressures (closed; its
  Table 2 read as Lisse et al. 2021 reproduce it, N₂ and argon's
  `Ar-1`).
- Freedman, Marley and Lodders 2008, *ApJS* 174, 504, the opacity at
  the radiative-convective boundary (as Ginzburg et al. 2018 eq. 9 print
  it).
- Fretwell et al. 2013, *The Cryosphere* 7, 375,
  doi:10.5194/tc-7-375-2013, Bedmap2 (read).
- Fulchignoni et al. 2005, *Nature* 438, 785, doi:10.1038/nature04314,
  Titan's surface temperature and pressure.
- Fulton et al. 2017, the radius bins of the census.
- García Muñoz and Schneider 2019, *ApJL*, doi:10.3847/2041-8213/ab498d,
  arXiv:1910.00267, Balmer-driven escape round hot stars (abstract
  read).
- Gao et al. 2023, *J. Phys. Chem. Ref. Data* 52,
  doi:10.1063/5.0128269, NH₃'s triple-point temperature.
- Giacalone et al. 2022, *ApJL*, doi:10.3847/2041-8213/ac80f4,
  arXiv:2208.06396, HD 56414 b, a Neptune of an A star (read).
- Giauque and Stephenson 1938, *J. Am. Chem. Soc.* 60, 1389,
  through the NIST Chemistry WebBook.
- Giles et al. 2024, arXiv:2405.19253, Io's atmosphere from IRTF/TEXES
  spectra.
- Ginzburg, Schlichting and Sari 2016, *ApJ* 825, 29,
  doi:10.3847/0004-637X/825/1/29, arXiv:1512.07925, the boil-off as
  spontaneous mass loss (read).
- Ginzburg, Schlichting and Sari 2018, *MNRAS* 476, 759,
  doi:10.1093/mnras/sty290, arXiv:1708.01621, core-powered mass loss
  (read).
- Glein 2015, *Icarus* 250, 570, doi:10.1016/j.icarus.2015.01.001,
  Titan's N₂ from the NH₃ its interior decomposes (not reached; its
  equations as Erkaev et al. 2020 print them).
- Glein 2023, *Icarus*, arXiv:2306.06350, the origin of Pluto's N₂ and
  the 30 K N₂ trapping (read).
- Glein and Shock 2013, *Geochim. Cosmochim. Acta* 115, 217,
  doi:10.1016/j.gca.2013.03.030, Titan's seas (full text read).
- Goldblatt 2015, doi:10.1089/ast.2014.1268.
- Goldblatt et al. 2009,
  doi:10.1038/ngeo692, nitrogen and Earth's temperature.
- Goldblatt, Robinson, Zahnle and Crisp 2013,
  doi:10.1038/ngeo1892, the moist limit, the steam state and its
  bistability (accepted manuscript, UVic repository, read); corrigendum
  doi:10.1038/ngeo2296 (read: 286 W/m² absorbed, not 294).
- Gosman, McCarty et al. 1969, argon's triple point, 83.8 K and 0.689
  bar, through the NIST Chemistry WebBook (read).
- Gray and Nagel 1989, the limit of the convective envelope.
- Gupta and Schlichting 2019, *MNRAS* 487, 24, doi:10.1093/mnras/stz1230,
  arXiv:1811.03202, core-powered mass loss and the valley's slope
  (read).
- Gupta and Schlichting 2020, *MNRAS*, doi:10.1093/mnras/staa315,
  arXiv:1907.03732, its dependence on the host (read).
- Gurrola 1995, PhD thesis, Stanford University, Triton's surface
  pressure from the Voyager 2 radio occultation, 14 ± 1 µbar (not
  reached; as Sicardy et al. 2024 quote it).
- Hall et al. 1995 and 1998, the O₂ of Europa and Ganymede.
- Hamano, Abe and Genda 2013, *Nature* 497, 607, doi:10.1038/nature12163,
  the steam over a magma ocean (abstract read).
- Hamano et al. 2015, doi:10.1088/0004-637X/806/2/216.
- Hapke 1981, *JGR* 86, 3039.
- Haqq-Misra, Kopparapu, Batalha, Harman and Kasting 2016, *ApJ* 827,
  120, doi:10.3847/0004-637X/827/2/120, limit cycles near the outer
  edge of the habitable zone (cited for the departure of #729; title
  and venue checked, not read).
- Hayashi 1981, *Prog. Theor. Phys. Suppl.* 70, 35,
  doi:10.1143/PTPS.70.35, eq. 2.3, the thin disc's `T ∝ L^¼` (not
  reached; as Ida & Lin 2005 and Kennedy & Kenyon 2008 quote it).
- Henderson-Sellers and Wilson 1983, *Rev. Geophys.* 21, 1743,
  doi:10.1029/RG021i008p01743, surface albedos (abstract read).
- Hillier et al. 1990, *Science* 250, 419,
  doi:10.1126/science.250.4979.419, Triton's Bond albedo (abstract read).
- Hillier, Helfenstein, Verbiscer and Veverka 1991, "Voyager
  photometry of Triton: haze and surface photometric properties", *J.
  Geophys. Res.* 96, doi:10.1029/91JA01736, Triton's bolometric albedo
  and emissivity (abstract read).
- Hoffman et al. 2017, doi:10.1126/sciadv.1600983.
- Hörst, Vuitton and Yelle 2008, doi:10.1029/2008JE003135.
- Hogan and Matricardi 2020, *Geosci. Model Dev.* 13, 6501,
  doi:10.5194/gmd-13-6501-2020, Table 4, the longwave of each
  minor gas by line-by-line (read; study 32).
- Howett et al. 2010, Enceladus's albedo.
- Huebner, Keady and Lyon 1992, *Astrophys. Space Sci.*
  195, 1, photo-ionisation rates.
- Hutchins and Jakosky 1996, *J. Geophys. Res.* 101, 14933,
  doi:10.1029/96JE00860, Mars's argon sputtered (abstract read).
- Ida and Lin 2005, *ApJ* 626, 1045, doi:10.1086/429953,
  arXiv:astro-ph/0502566, eq. 4 and the text of eq. 12: the ice line of
  Hayashi's thin disc at 170 K.
- Jackson, Davis and Wheatley 2012, *MNRAS* 422, 2024,
  Table 2.
- Jakosky, Slipski, Benna, Mahaffy et al. 2017, *Science* 355, 1408,
  doi:10.1126/science.aai7721, 66 % of Mars's argon lost (abstract
  read).
- Jin et al. 2004, *Geophys. Res. Lett.* 31(22), doi:10.1029/2004GL021180,
  ocean surface albedo (abstract read).
- Jochum et al. 1983, a silicate Earth K/U of about 12 000 (as Marty
  2012, p. 15, quotes it).
- Johnson 1990, *Energetic Charged-Particle Interactions with
  Atmospheres and Surfaces*.
- Johnson et al. 2004, "Radiation effects on the surfaces of the
  Galilean satellites", in *Jupiter: The Planet, Satellites and
  Magnetosphere*, ch. 20.
- Johnson and Goldblatt 2015, doi:10.1016/j.earscirev.2015.05.006,
  Earth's nitrogen.
- Johnstone et al. 2021, *A&A* 649, A96,
  doi:10.1051/0004-6361/202038407.
- Kadoya and Tajika 2014, *ApJ* 790, 107,
  doi:10.1088/0004-637X/790/2/107, the climate of Earth-like planets
  across the habitable zone with volcanic CO₂ degassing (cited for the
  departure of #729 and the thaw search of #731; title and venue
  checked, not read).
- Karkoschka 2001, albedos of the Uranian moons.
- Karwat 1924, NH₃'s sublimation pressure at 162.39 K, through Fray
  and Schmitt 2009 as Blakley et al. 2024 quote it.
- Kasting 1988, *Icarus* 74, 472, doi:10.1016/0019-1035(88)90116-9,
  runaway and moist greenhouses, § 3a, 3d and 4, Figs. 4, 7 and 13
  (read, Zenodo record 1253896).
- Kasting 1991, Appendix B, CO₂'s pseudo-adiabat.
- Kaula 1999, *Icarus* 139, 32, Venus's ⁴⁰Ar, (1.61 ± 0.54)e16 kg (as
  Warren and Kite 2023 quote it).
- Kelemen and Manning 2015, doi:10.1073/pnas.1507889112,
  carbon recycling.
- Kempf et al. 2010, Enceladus's plume fallout.
- Kennedy and Kenyon 2008, *ApJ* 673, doi:10.1086/524130,
  arXiv:0710.1065, p. 3 and Fig. 1: the snow line at 170 K moving inward
  with time, and why it differs from Ida & Lin's.
- Kiehl and Trenberth 1997, *Bull. Am. Meteorol. Soc.* 78, 197,
  doi:10.1175/1520-0477(1997)078<0197:EAGMEB>2.0.CO;2, Table 3, the
  clear-sky longwave by gas (read; study 32).
- Kite and Ford 2018, *ApJ* 864, 75,
  doi:10.3847/1538-4357/aad6e0, waterworlds.
- Kite, Fegley, Schaefer and Gaidos 2016, *ApJ* 828, 80,
  doi:10.3847/0004-637X/828/2/80, arXiv:1606.06740 (read).
- Kite, Manga and Gaidos 2009,
  doi:10.1088/0004-637X/700/2/1732, the outgassing of lids and plates.
- Kivelson et al. 2004, in *Jupiter: The Planet, Satellites and
  Magnetosphere*, ch. 21.
- Koll and Cronin 2018, *PNAS* 115, 10293, doi:10.1073/pnas.1809868115,
  Earth's clear-sky feedback near 2.2 W/m²/K (read; study 32).
- Kopparapu et al. 2013, doi:10.1088/0004-637X/765/2/131.
- Kopparapu et al. 2014, doi:10.1088/2041-8205/787/2/L29, the
  runaway limit and the maximum-greenhouse row (the outer edge).
- Korenaga 2010, plate tectonics against planet size.
- Korenaga, Planavsky and Evans 2017, *Phil. Trans. R. Soc. A* 375,
  20150393, doi:10.1098/rsta.2015.0393, §2a: Earth's water by
  reservoir.
- Kossert et al. 2022, *Appl. Radiat. Isot.* 188, 110362, the
  electron-capture branch of ⁴⁰K (as Mougeot 2025 cites it).
- Krasnopolsky 2012, doi:10.1016/j.icarus.2011.10.019, Mars's SO₂
  limit.
- Krissansen-Totton, Fortney and Nimmo 2021, *Planet. Sci. J.* 2, 216,
  doi:10.3847/PSJ/ac2580, the argon degassing law of their Appendix A.3
  (read, open access).
- Lacis et al. 2010, doi:10.1126/science.1190653.
- Lainey et al. 2009, doi:10.1038/nature08108, Io's tidal
  dissipation.
- Lammer et al. 2014, *MNRAS*, arXiv:1401.2765, nebula-captured
  hydrogen envelopes of 0.1 to 5 M⊕: Tables 1 to 3 (the largest
  envelope of a 1 M⊕ core, 2.5 % of the planet's mass, Table 1) and
  Table 5 (read).
- Lay, Hernlund and Buffett 2008, doi:10.1038/ngeo.2007.44,
  the core's heat flow.
- Lecavelier des Etangs 2007, energy-limited escape.
- Lee, Marti, Severinghaus, Kawamura et al. 2006, *Geochim. Cosmochim.
  Acta* 70, 4507, doi:10.1016/j.gca.2006.06.1563, the air's argon
  isotopes (as Avice and Marty 2020 quote it).
- Lellouch, de Bergh, Sicardy, Ferron and Käufl 2010, *Astron.
  Astrophys.* 512, L8, doi:10.1051/0004-6361/201014339, arXiv:1003.2866,
  Triton's CO and the pictures of surface-atmosphere exchange, § 3
  (read).
- Leming and Pollack 1970, Lee, Fuks and Bigeleisen 1970, Levenson
  1967, Chen et al. 1978, Ancsin 1973, Boato et al. 1961, Clark et al.
  1951, Flubacher et al. 1961, Freeman and Halsey 1956, Regnier 1972:
  argon's sublimation pressures (as Xiao et al. 2025, Table 6, list
  them).
- Lemmon et al. 2013, ethane's triple point (90.4 K), as Richardson,
  Hartwig and Leachman 2019 quote it (not re-read).
- Lichtenegger, Dyadechkin, Scherf, Lammer et al. 2022,
  arXiv:2105.09789, Mars's non-thermal escape and its argon isotopes,
  eq. 32 (read).
- Lillis et al. 2013, Mars's dynamo.
- Lipiński et al. 2007, *Int. J. Thermophys.* 28, 1904,
  doi:10.1007/s10765-007-0267-y, nitrogen's alpha-beta transition
  (closed).
- Lisse et al. 2021, *Icarus* 356, 114072,
  doi:10.1016/j.icarus.2020.114072, supplementary Table 2 (read, and
  its appendix Table 2 in arXiv:2009.02277, argon's row read
  2026-11-25).
- Loeb et al. 2020, *J. Climate* 33, 61, doi:10.1175/JCLI-D-19-0381.1,
  Table 3, CERES's clear sky of the same air without its clouds
  (read; study 32).
- Lopez and Fortney 2014, *ApJ* 792, 1, arXiv:1311.0329, eq. 1 (a
  rocky core's radius, `R ∝ M^(1/4)`) and Tables 2 to 4 (the envelope's
  thickness, through `science::envelope`).
- Lora and Ádámkovics 2017, arXiv:1610.06277, Titan's near-surface
  methane humidity (read).
- Lyons, Reinhard and Planavsky 2014,
  doi:10.1038/nature13068, the Great Oxidation Event.
- Lyubetskaya and Korenaga 2007; McDonough and Arevalo 2008; Palme and
  O'Neill 2014: models of the primitive mantle's potassium (as Wang,
  Lineweaver and Ireland 2018 combine them).
- Mahaffy, Webster, Atreya, Franz et al. 2013, *Science* 341, 263,
  doi:10.1126/science.1237966, Mars's air from Curiosity (abstract
  read).
- Mandt, Mousis, Lunine and Gautier 2014, *ApJ Letters*
  788, L24, doi:10.1088/2041-8205/788/2/L24.
- Manabe and Wetherald 1967, *J. Atmos. Sci.* 24, 241,
  doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2 (read in study 33
  in the scan GFDL hosts: eq. 2, the surface humidity 0.77 fitted to
  observed hemispheric means, Fig. 3).
- Marcq et al. 2008, Venus's SO₂ below the clouds.
- Marty 2012, *Earth Planet. Sci. Lett.* 313-314, 56-66,
  doi:10.1016/j.epsl.2011.10.040, carbon in the mantle; Earth's bulk
  water, p. 18 of the arXiv author copy (arXiv:1405.6336); Earth's
  nitrogen chondritic (as cited by Scherf et al. 2020, § 1); the air's
  ³⁸Ar/³⁶Ar (p. 9) and the ⁴⁰Ar mass balance (p. 15, read).
- Lellouch, Stansberry, Emery, Grundy and Cruikshank 2011, "Thermal
  properties of Pluto's and Charon's surfaces from Spitzer
  observations", *Icarus* 214, 701-716, the N₂ ice emissivity 0.8 (not
  read; cited through Meza et al. 2019).
- McDonough 2003, *Treatise on Geochemistry* 2.15.
- McEwen et al. 1998, *Science* 281, 87, Io's lavas.
- McGehee and Lehman 2012, *SIAM J. Appl. Dyn. Syst.* 11(2), 684,
  the annual insolation integral (cited by Nadeau and McGehee).
- McKay, Pollack and Courtin 1991,
  doi:10.1126/science.11538492, Titan's greenhouse.
- Menou 2015, *Earth Planet. Sci. Lett.* 429, 20,
  doi:10.1016/j.epsl.2015.07.046, climate stability of habitable
  Earth-like planets (cited for the departure of #729 and the thaw
  search of #731; title and venue checked, not read).
- Meza et al. 2019, "Pluto's lower atmosphere and pressure evolution
  from ground-based stellar occultations, 1988-2016", *A&A* 625, A42,
  doi:10.1051/0004-6361/201834281, arXiv:1903.02315 (§ 4 read).
- Min, Mundil, Renne and Ludwig 2000; Renne, Balco, Ludwig, Mundil and
  Min 2011; Steiger and Jäger 1977: the decay constants of ⁴⁰K (as
  Schaen et al. 2020, Table 1, tabulate them).
- Misener, Schulik, Schlichting and Owen 2025, *ApJ*,
  doi:10.3847/1538-4357/ada777, arXiv:2405.15221, the non-isothermal
  core-powered wind (abstract read).
- Mittelholz et al. 2020, a late field on Mars.
- Modirrousta-Galian and Korenaga 2026, *Icarus* 444, 116827,
  doi:10.1016/j.icarus.2025.116827, arXiv:2509.16046, the energy-limited
  cap of core-powered loss (abstract read).
- Moore et al. 1999, Callisto.
- Moore and Webb 2013, doi:10.1038/nature12473, "Heat-pipe
  Earth".
- Miller et al. 2019, organic nitrogen in Titan's N₂ (as Scherf et al.
  2020 summarise it).
- Mougeot 2025, DDEP evaluation of ⁴⁰K, ICRM 2025 poster, HAL
  cea-05087440, the half-life 1.2522(27) Gyr and the branches (read).
- Mougeot et al. 2026, *Metrologia* 63, 019001,
  doi:10.1088/1681-7575/ae3733, the DDEP evaluation of ⁴⁰K as the
  LNE-LNHB table of ⁴⁰K publishes it (read on lnhb.fr; the paper itself
  was not read): the decay energies of potassium's heat.
- Mousis, Lunine, Mandt et al. 2013, *Icarus* 225, 856,
  doi:10.1016/j.icarus.2013.03.008, arXiv:1303.4595, noble gases in
  Pluto's air and their condensation (read).
- Mousis, Lunine, Thomas et al. 2009, *ApJ* 691, 1780,
  doi:10.1088/0004-637X/691/2/1780, arXiv:0810.0308, the clathration of
  Titan's building blocks (read).
- Mousis, Gautier and Bockelée-Morvan 2002, *Icarus* 156, 162,
  doi:10.1006/icar.2001.6782.
- Mumma and Charnley 2011,
  doi:10.1146/annurev-astro-081309-130811.
- Nadeau and McGehee 2017, *Icarus* 291, 46,
  doi:10.1016/j.icarus.2017.01.040, the mean annual insolation by
  latitude (read in arXiv:1510.04542).
- Nadeau and McGehee 2021, *J. Math. Anal. Appl.*, 125129,
  doi:10.1016/j.jmaa.2021.125129, the Legendre series of the annual
  insolation (read in arXiv:1810.10081).
- Nakajima, Hayashi and Abe 1992,
  doi:10.1175/1520-0469(1992)049<2256:ASOTGE>2.0.CO;2, the
  Simpson-Nakajima limit.
- NASA NSSDC Earth fact sheet, the composition of Earth's air and its
  mean radius; the Moon, Mercury and Mars fact sheets, their Bond albedos
  (read); the Mars fact sheet also for its air (6.36 mb, 95.1 % CO₂,
  2.59 % N₂, 0.16 % O₂) and its "Average temperature: ~214 K" (read,
  May 2025); the Venus fact sheet for its Bond albedo; the Neptune fact
  sheet for Triton's orbit-mean flux (1.508 W/m²) (read); the Earth,
  Venus and Mars fact sheets for their argon, the mass and the mean
  molecular weight of their air, the Pluto fact sheet for its mass,
  radius and air, the Neptunian satellite fact sheet for Triton's mass
  and radius (read 2026-11-25).
- Naumenko et al. 2013, *Geochim. Cosmochim. Acta* 122, 353, ⁴⁰K's
  share of potassium (as Mougeot 2025 quotes it).
- Neukum, Ivanov and Hartmann 2001, the crater
  chronology.
- Niemann et al. 2005, *Nature* 438, 779, Titan's ³⁶Ar/¹⁴N (as
  Mousis et al. 2009 quote it).
- Niemann et al. 2010, *J. Geophys. Res.* 115, E12006,
  doi:10.1029/2010JE003659, Titan's methane at the Huygens site
  (abstract read; supersedes their 2005 results); its argon, eq. 7 and
  10 and § 4.5 (read in the NASA author copy, NTRS 20110008599).
- Nimmo 2015, the core's heat flow.
- Nimmo and Stevenson 2000, doi:10.1029/1999JE001216.
- NIST Atomic Spectra Database, the H I Lyman α multiplet.
- NIST CODATA, the Planck constant (exact).
- Noack et al. 2016, *Icarus* 277, 215,
  doi:10.1016/j.icarus.2016.05.009.
- North, Cahalan and Coakley 1981, *Rev. Geophys. Space Phys.* 19, 91,
  doi:10.1029/RG019i001p00091, energy balance climate models (read).
- NSIDC, the sea ice pages (quick facts; sea ice), read 2026-11-24.
- O'Rourke and Korenaga 2015, *Icarus* 260, 128, Venus's argon
  degassing (as Warren and Kite 2023 and Krissansen-Totton et al. 2021
  quote it).
- Ohtani 2021, *Annu. Rev. Earth Planet. Sci.* 49, 253,
  doi:10.1146/annurev-earth-080320-062509, the large and small water
  budgets (abstract, through study 22).
- Öberg, Murray-Clay and Bergin 2011, *ApJL* 743, L16, arXiv:1110.5567,
  the disc temperature profile and its snowlines, eq. 1 and Table 1
  (read).
- O'Reilly and Davies 1981, *Geophys. Res. Lett.* 8, 313.
- Overstreet and Giauque 1937, NH₃'s sublimation enthalpy, through
  the NIST Chemistry WebBook.
- Owen 1982, Titan's ⁴⁰Ar for an Earth-like rock (as Niemann et al.
  2010 quote it).
- Owen and Schlichting 2024, *MNRAS*, arXiv:2308.00020, photoevaporation
  against core-powered loss (§ 1 to 4 read).
- Owen and Wu 2016, *ApJ* 817, 107, doi:10.3847/0004-637X/817/2/107,
  arXiv:1506.02049, the boil-off (read).
- Owen and Wu 2017, doi:10.3847/1538-4357/aa890a, eq. 19.
- Ozima and Podosek 2002, *Noble Gas Geochemistry*, the air's 1.6e18
  mol of ⁴⁰Ar (as Marty 2012, p. 15, quotes it).
- Payne 1972, *J. Atmos. Sci.* 29, 959,
  doi:10.1175/1520-0469(1972)029<0959:AOTSS>2.0.CO;2, the albedo of
  the sea (abstract read).
- Perovich et al. 2002, *J. Geophys. Res.* 107(C10),
  doi:10.1029/2000JC000438, the albedo of Arctic sea ice (abstract
  read).
- Petigura et al. 2022, *AJ*, doi:10.3847/1538-3881/ac51e3,
  arXiv:2201.10020, the radius gap against stellar mass (abstract read).
- Picard, Davis, Gläser and Fujii 2008, *Metrologia* 45, 149,
  doi:10.1088/0026-1394/45/2/004, the CIPM-2007 air density formula and
  its argon (abstract read).
- Pierrehumbert 2010, section 4.4.
- Pierrehumbert 2023, *ApJ* 944, 20, doi:10.3847/1538-4357/acafdf,
  arXiv:2212.02644, the supercritical column of a post-runaway
  waterworld, § 1 and 2 (read).
- Pincus, Forster and Stevens 2016, *Geosci. Model Dev.* 9, 3447,
  doi:10.5194/gmd-9-3447-2016, the 100 RFMIP profiles (read in
  RRTMGP's data; study 32).
- Plaut et al. 2007, *Science* 316, 92, doi:10.1126/science.1139672,
  Mars's south polar layered deposits (abstract read).
- Quirico et al. 1999, *Icarus* 139, 159, Triton's CO₂ ice.
- Ribas et al. 2005, *ApJ* 622, 680, doi:10.1086/427977, Table 5.
- Robinson and Catling 2012, *ApJ* 757, 104, section 4.3.
- Robinson and Catling 2014, doi:10.1038/ngeo2020, the
  tropopause.
- Rogers, Gupta, Owen and Schlichting 2021, *MNRAS* 508, 5886,
  doi:10.1093/mnras/stab2897, arXiv:2105.03443, the valley's slopes
  predicted by each loss (read in part).
- Rubin et al. 2015, *Science* 348, 232, doi:10.1126/science.aaa6100,
  N₂ in comet 67P and its formation under about 30 K (abstract read).
- Rubin et al. 2019, doi:10.1093/mnras/stz2086, Table 2, comet 67P.
- Rubin, Altwegg, Balsiger, Bar-Nun et al. 2018, *Sci. Adv.* 4,
  eaar6297, doi:10.1126/sciadv.aar6297, the noble gases of comet 67P,
  Table 2 (read in PMC6031375).
- Sasaki and Tajika 1995, AIP Conf. Proc., doi:10.1063/1.48741, the
  long-term degassing of Venus and Mars from their ⁴⁰Ar (abstract read).
- Schaen, Jicha, Hodges, Vermeesch et al. 2020, *GSA Bulletin* 133,
  461, doi:10.1130/B35560.1, Table 1, the ⁴⁰K decay constants (read in
  HAL hal-02903663).
- Scherf, Lammer, Erkaev, Mandt, Thaller and Marty 2020, *Space Sci.
  Rev.* 216, 123, doi:10.1007/s11214-020-00752-0, arXiv:2011.00973,
  nitrogen atmospheres of the icy bodies, Titan's § 3 (read).
- Schmidt, Ruedy, Miller and Lacis 2010, *JGR* 115, D20106,
  doi:10.1029/2010JD014287, Table 1, the greenhouse by absorber
  (read; study 32).
- Schröder and Keller 2008, *Planet. Space Sci.* 56, 753,
  doi:10.1016/j.pss.2007.10.011, the Huygens landing site's reflectance
  (read in arXiv:1702.00653).
- Schröder and Schmitt 2007, X-rays of A stars.
- Scott 1976, *Phys. Rep.* 27, 89, doi:10.1016/0370-1573(76)90032-6,
  nitrogen's alpha-beta transition (closed; through Tegler et al. 2019).
- Setzmann and Wagner 1991, "A New Equation of State and Tables of
  Thermodynamic Properties for Methane Covering the Range from the
  Melting Line to 625 K at Pressures up to 1000 MPa", *J. Phys. Chem.
  Ref. Data* 20, 1061-1155, doi:10.1063/1.555898, methane's critical
  point, triple point and saturation curve (as `volatiles.rs` and
  `volatiles_tests.rs` cite it).
- Showman, Wordsworth, Merlis and Kaspi 2013, "Atmospheric circulation
  of terrestrial exoplanets", in *Comparative Climatology of Terrestrial
  Planets* (Arizona Space Science Series), arXiv:1306.2418, § 2 (read).
- Sicardy et al. 2024, arXiv:2402.02476 (accepted by *A&A*), Triton's
  air from occultations 1989 to 2022 (read).
- Simonelli and Veverka 1988, Io's Bond albedo.
- Sohl et al. 2003, *J. Geophys. Res.*
  108, doi:10.1029/2003JE002044, Titan's rock and iron, 50 to 60 wt %
  (as Mosqueira 2026, arXiv:2610.34081, Table A1, tabulates it).
- Span, Lemmon, Jacobsen, Wagner and Yokozeki 2000, "A reference
  equation of state for the thermodynamic properties of nitrogen for
  temperatures from 63.151 to 1000 K and pressures to 2200 MPa", *J.
  Phys. Chem. Ref. Data* 29, 1361, doi:10.1063/1.1349047 (read in full
  in the NIST reprint): the critical point (Sec. 2.1, eq. 2), the triple
  point (Sec. 2.2, eq. 3), the vapour-pressure ancillary (Sec. 2.6,
  eq. 4) and the saturation tables (pp. 1405-1407).
- Spencer 1987, dark moons.
- Stansberry, Pisano and Yelle 1996,
  *Planet. Space Sci.* 44, 945, doi:10.1016/0032-0633(96)00001-3;
  Pluto's pure CH₄ deposits (as Lellouch et al. 2010 quote it).
- Steffl, Young, Strobel et al. 2020, *Astron. J.* 159, 274,
  doi:10.3847/1538-3881/ab8d1c, Pluto's ultraviolet airglow and its
  argon limit, § 5.2 (read).
- Stevenson, Spohn and Schubert 1983,
  doi:10.1016/0019-1035(83)90241-5, the thermal history of the core.
- Stewart, Jacobsen et al. 1986, nitrogen's properties
  through the NIST Chemistry WebBook.
- Tan, Kargel and Marion 2013, *Icarus* 222, 53,
  doi:10.1016/j.icarus.2012.10.032, Titan's seas (closed; through Glein
  and Shock 2013, Table 4).
- Tang, Fortney and Murray-Clay 2024, arXiv:2410.08577, boil-off and
  the core-powered loss after it, Table 2 (read).
- Tegeler, Span and Wagner 1999, *J. Phys. Chem. Ref. Data* 28, 779,
  doi:10.1063/1.556037, argon's reference equation of state, its triple
  point (as Xiao et al. 2025 and CoolProp take it).
- Tegler et al. 2019, arXiv:1905.13250, nitrogen's alpha-beta
  transition as Scott 1976 gives it (read).
- Thomas et al. 2004, *Jupiter*, ch. 23, Io's plasma loss.
- Tikoo et al. 2017, the lunar dynamo.
- Tobie, Gautier and Hersant 2012, *ApJ* 752, 125,
  doi:10.1088/0004-637X/752/2/125.
- Tobie, Lunine and Sotin 2006, *Nature* 440, 61,
  doi:10.1038/nature04497.
- Tosi et al. 2017, *A&A* 605, A71, doi:10.1051/0004-6361/201730728,
  the habitability of a stagnant-lid Earth (through study 22).
- Trafton 1990; Trafton, Matson and Stansberry 1998: the detailed
  balancing layer (as Lellouch et al. 2010 quote them).
- Trenberth and Smith 2005, *J. Climate* 18, 864,
  doi:10.1175/JCLI-3299.1, the mass of Earth's dry air.
- Tu et al. 2015, the saturation of a star's X-ray emission.
- Turbet, Bolmont, Ehrenreich, Gratier, Leconte, Selsis, Hara and
  Lovis 2020, *A&A* 638, A41, doi:10.1051/0004-6361/201937151,
  arXiv:1911.08878v3, Eqs. 2 and 3, Table 2, Appendices A, C and D (read
  in full; v1 and v2 read for Eq. 2 and Table 2).
- Turbet, Ehrenreich, Lovis, Bolmont and Fauchez 2019, *A&A* 628, A12,
  doi:10.1051/0004-6361/201935585, arXiv:1906.03527, Appendix A (read):
  the climate model behind Turbet et al. 2020's fits, run under the
  Sun's light.
- Tryka et al. 1993, "Spectroscopic determination of the phase
  composition and temperature of nitrogen ice on Triton", *Science*
  261, 751, doi:10.1126/science.261.5122.751 (abstract read); Tryka et
  al. 1994, "Temperature of nitrogen ice on Pluto and its implications
  for flux measurements", *Icarus* 112, 513-527 (abstract read).
- Tyler et al. 1989, *Science* 246, 1466,
  doi:10.1126/science.246.4936.1466, Voyager 2's radio occultation of
  Triton: 1.6 ± 0.3 Pa and 38 K (abstract read).
- Van Eylen et al. 2018, *MNRAS*, doi:10.1093/mnras/sty1783,
  arXiv:1710.05398, the valley's slope with period (abstract and
  conclusions read).
- Veeder et al. 2004, Io's heat flow, as quoted by Moore and Webb
  2013.
- Verbiscer et al. 2007, Enceladus's E-ring fallout.
- Veverka et al. 1991, albedos of the Uranian moons.
- Vogt and Pitzer 1976, methane's enthalpy of fusion (0.9392 kJ/mol),
  through the NIST Chemistry WebBook.
- Wagman et al. 1979, NBS Chemical Thermodynamics Data Center, as
  quoted by Giles et al. 2024.
- Wagner and Pruss 1993, IAPWS SR1-86(1992), the vapour pressure over
  liquid water and its critical pressure, as `crates/science/src/water.rs`
  cites it.
- Wagner et al. 2011, IAPWS R14-08(2011), eq. 6, the sublimation
  pressure of ice Ih, and eqs. 2 and 3, the melting curves of ices III
  and V, as `crates/science/src/water.rs` cites it.
- Walker, Hays and Kasting 1981,
  doi:10.1029/JC086iC10p09776, the carbonate thermostat.
- Wang, Lineweaver and Ireland 2018,
  doi:10.1016/j.icarus.2017.08.024, Table 1; its potassium, Table 1
  and p. 4 (read in arXiv:1708.08718).
- Ward 1974, *J. Geophys. Res.* 79(24), 3375, the annual insolation
  of a rapid rotator (cited by Nadeau and McGehee).
- Warren 1982, *Rev. Geophys.* 20, 67, doi:10.1029/RG020i001p00067,
  the optical properties of snow (abstract read).
- Warren and Kite 2023, *Proc. Natl. Acad. Sci.* 120, e2209751120,
  doi:10.1073/pnas.2209751120, Venus's ⁴⁰Ar and its degassing (read in
  PMC10089166).
- Way et al. 2016, "Was Venus the first habitable world of our solar
  system?", *Geophys. Res. Lett.* 43, doi:10.1002/2016GL069790,
  arXiv:1608.00706 (abstract read).
- Weber et al. 2011, the Moon's core.
- Weiss and Tikoo 2014, the lunar dynamo.
- Witt and Kemp 1937, *J. Am. Chem. Soc.* 59, 273, ethane's enthalpy
  of vaporisation, through the NIST Chemistry WebBook.
- Wolf and Toon 2014, doi:10.1089/ast.2013.1112.
- Wright et al. 2011, *ApJ* 743, 48.
- Xiao, Sriskandaruban and Maynard-Casely 2025, *Int. J. Thermophys.*
  46, 14, doi:10.1007/s10765-024-03469-2, solid argon's equation of
  state and its sublimation pressure, eq. 55 (read in the Research
  Square preprint rs-5370077).
- Yang, Boué, Fabrycky and Abbot 2014, "Strong dependence of the inner
  edge of the habitable zone on planetary rotation rate", *ApJ* 787,
  L2, doi:10.1088/2041-8205/787/1/L2, arXiv:1404.4992 (read).
- Yantsevich, Prokhvatilov and Krupskii 1986, doi:10.1063/10.0031462,
  N₂-Ar solid solutions (abstract read).
- Young, Kammer, Steffl et al. 2018, *Icarus* 300, 174,
  doi:10.1016/j.icarus.2017.09.006, Pluto's atmosphere from the solar
  occultation (as Steffl et al. 2020 quote it).
- Yung, Allen and Pinto 1984, *ApJS* 55, 465,
  doi:10.1086/190963, Titan's photochemistry, Tables 2A and 5B.
- Zeng, Sasselov and Jacobsen 2016, *ApJ* 819, 127,
  doi:10.3847/0004-637X/819/2/127, the mass-radius tables of rocky
  planets Turbet et al. build on (the MgSiO₃ and Earth-like tables of
  the author's site, read).
- Zahnle and Catling 2017, *ApJ* 843, 122,
  doi:10.3847/1538-4357/aa7846, the cosmic shoreline.
- Zahnle et al. 2003, doi:10.1016/S0019-1035(03)00048-4.
- Zhang 2024, arXiv:2410.04595, a review of the atmospheres of the
  moons and Pluto, the lack of noble-gas data on Triton (read).
- Ziegler, Mullins et al. 1962, Georgia Institute of Technology,
  Project A-460, Technical Report 2, argon's sublimation below one
  atmosphere (record only, listed by the NIST Chemistry WebBook).
