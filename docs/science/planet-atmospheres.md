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
the law to the formed body and returns the trial before and after the loss
(the water step below needs the first), each value with its cause node.

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
(`after_steam`). The regime, the seafloor's hold on the outgassing
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
and `cmf` is the core mass fraction (the core keeps its sulphur). Each is
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
partial pressures at the surface, in bar, of N₂, CO₂, H₂O, CH₄, NH₃, SO₂
and O₂ (`Gases`, with `total()`), in six steps, called in this order:
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
`condense`, the fallback and the climate wait for #659. The carbonate
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
| Plates | 0.95, of which subduction returns 81.45 % of the nitrogen (N₂ and NH₃) and 63 % of the rest | Earth's N₂ once its NH₃ is photolysed (Goldblatt et al. 2009, doi:10.1038/ngeo692: about 0.5 times the air's nitrogen in the crust and more than 1.4 times in the mantle; abstract only). Legacy fitted 80 % on 0.78 bar, a share by volume; by weight Earth's N₂ is 0.746 bar (*Earth's N₂ target*), and the share is refitted on that weight. Earth's N₂ in the chain, the outgassed N₂ plus what its photolysed NH₃ makes, is proportional to the share kept: 0.804 bar at 80 %, so 1 − 0.2 × 0.746 / 0.804, kept to four digits, 81.45 %, gives 0.7458 bar (0.475 bar before the photochemistry). The 63 % has no source: Kelemen & Manning 2015 (doi:10.1073/pnas.1507889112, as quoted) find that "relatively little carbon" is recycled into the mantle, where older work put about half there, so it is high |
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
paper, which the model's 150 ppm sits inside, and washed out of a wet or
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
trial's mass, and the runaway's steam loss reads the same mass; both take
the bulk radius, the condensed planet's for a water beyond the laws, the
only radius there is until PR 2d's steam envelope.

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
is not reducing is kept (Europa's surface shows some); Titan's N₂ still
comes from its ammonia, though ammonia dissolves in an ocean as well.
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
rewrite has no climate yet: the test on the climate's result (the
fallback to the air with CO₂, `titan_like_air`) is left to 2d-3 (#659),
and until then a reducing air stays without CO₂. The frozen CO₂ is
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
126.19 K, through its normal boiling point, 77.355 K, NIST Chemistry
WebBook). The CO₂ the cold trap froze out stays on the ground as a
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
star), which the reducing test and the methane line read too; the step records `planet.air.trapped.co2` and
`planet.air.trapped.nh3`, each citing the kept gas and the equilibria,
and `planet.air.frozen-co2`, citing the kept and the trapped CO₂. What
the trap leaves is what the photochemistry reads. Where it departs from
legacy: the frost curves of CO₂ and ammonia are the sourced ones above;
`condense` (the climate's condensation of water, methane and nitrogen at
the surface temperature) is left to 2d-3, so the rewrite's air keeps
them whole.

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
(0.8224, not 28/34), and the rewrite's Earth has 0.7458 bar of N₂ (0.475
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

**Calibration of `NITROGEN_RECYCLED_UNDER_PLATES`** (0.8145; study 23,
part 5). Legacy's 0.8 gave the rewrite's Earth 0.804 bar of N₂, 7.8 %
over the target. The chain's N₂ on Earth is proportional to the share
kept, `1 − x` (0.804 bar at 0.8, 0.748 at 0.814, 0.746 at 0.8145, by
running the chain on the reference Earth), so `x = 1 − 0.2 × 0.746 /
0.804`; four digits, 0.8145, give 0.7458 bar, 0.746 to the three digits
of the target. Earth's total moves from 52.83 to 52.77 bar (its CO₂,
52.03 bar before the thermostat, does not move); no other reference
body has plates. Searched and not found: a measured share of the
outgassed nitrogen that subduction returns. Goldblatt et al. 2009
(doi:10.1038/ngeo692, abstract) and Johnson & Goldblatt 2015
(doi:10.1016/j.earscirev.2015.05.006, abstract) put several times the
air's nitrogen in the mantle, which supports a large return without
giving one (study 21, part 5). The test keeps Earth's N₂ within 10 % of
0.746 bar: the target is sourced, the 10 % is the test's own tolerance.

Calibration targets, through the whole chain from the reference bodies'
inputs (volatiles of the rule, no scatter) round the Sun at 4.6 Gyr (the
Earth row and the rewrite's values below it are at 4.567 Gyr):

| Body | Result | Target | Observed |
|---|---|---|---|
| Venus | 87 bar, 98 % CO₂, 1.7 bar N₂ | 60 to 120 bar, CO₂ over 90 %, N₂ 1 to 6 bar | 92 bar, 96.5 % CO₂, about 3 bar N₂ |
| Earth | 0.7458 bar N₂, no CH₄ or NH₃ (52 bar CO₂ before the thermostat) | N₂ 0.746 bar within 10 %; CH₄ and NH₃ under 1e-3 bar | 0.746 bar N₂ by weight (0.78 by volume) |
| Mars | 0.006 bar (its CO₂ scaled to the real 6.1 mbar, `MARS_CO2_FACTOR`; 0.014 bar from the rule alone) | 0.002 to 0.05 bar | 0.006 bar |
| Titan | 2.2 bar, 95 % N₂, 0.11 bar CH₄ after the climate (6.3 bar before it condenses) | 0.5 to 5 bar; CH₄ over 0.01 bar | 1.5 bar, about 5 % CH₄ |
| Ganymede, Callisto | no CH₄ (0.15 and 0.47 bar before photolysis) | CH₄ under 1e-3 bar | airless |
| Mercury, Moon | under 1e-8 bar | under 1e-4 bar | exospheres |

The Earth row is the rewrite's; the other results are `legacy-final`'s,
after its climate where the row says so. The rewrite's chain, before
the climate (2d-3), gives Venus 89.5 bar, 98 % CO₂ and 1.75 bar N₂;
Titan 4.99 bar, of which 2.88 bar N₂ and 2.11 bar CH₄; Callisto
0.060 bar of N₂ and no CH₄.

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
  which give Titan its nitrogen.
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
38 K; both as Sicardy et al. 2024, arXiv:2402.02476, quote them, read), and the emissivity 0.89 centres the
`triton-like` archetype between them: 37.2 to 37.6 K and 1.40e-5 to
1.77e-5 bar of N₂ over seeds 1 to 40 (median 37.4 K and 1.57e-5 bar).
Before, with N₂ ice a black body and the curve 6.3 kJ/mol from memory,
it ranged from 35.4 to 37.0 K and 1.04e-5 to 2.52e-5 bar. That range
also needed the light of its Neptune narrowed: the N₂ vapour about
doubles every kelvin, and the `neptune-like` archetype drew ±9 % of
insolation (0.0010 to 0.0012 of the Earth's), ±0.75 K on Triton, which
no emissivity fits into 1e-5 to 2e-5 bar. It now draws what Neptune
itself receives over its orbit, 0.00109 to 0.00113 (perihelion 29.80
and aphelion 30.34 AU).

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
surface, its vapour capped by its water, on its gravity), so that a
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

**Methane's stand-in.** Methane keeps one grey optical depth, over 0-650
cm⁻¹ only (Courtin, Gautier & McKay 1995 place Titan's N₂–N₂ and CH₄–N₂
collision-induced bands there), `τ = k (u B)^½` with `u` its saturated
amount and `B` the air it collides with (see **Methane** below), and
`k` = 13.4 (`CH4_OPACITY`), calibrated on Titan's 93.7 K (14.3 on the grey
column, 7.75 before #563, 6.0 before #530, 3.3 before #443, 3.4 before
#441). A spectral Titan is left for later.

Titan's real greenhouse is mostly collision-induced absorption by N₂,
CH₄ and H₂, which warms it by 21 K, while its haze cools it by 9 K
(McKay, Pollack & Courtin 1991, doi:10.1126/science.11538492, abstract
checked); the methane opacity carries all of the greenhouse here, the
nitrogen's own included, and the anti-greenhouse (below) the cooling.

**Methane** (issue #443). The `sqrt(p_CH₄ P)` law, calibrated on Titan,
was stretched 500-fold past it: a planet at an equilibrium temperature of
114 K under 26 bar of methane (seed 182, planet d) came out at 351 K with
liquid water. Three rules bound it:

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
  Titan's skin at 70 K lets about 3 % through.
- *Saturation.* The amount the opacity counts is
  `u = p / (1 + p / 0.01 bar)` (`CH4_SATURATION_BAR`), and the pressure
  it collides with counts the methane as `u`:
  `τ = 13.4 (u (P − p + u))^½`. Past Titan's amounts its bands are opaque
  at their centres, and more methane only widens their wings (the
  logarithmic forcing of a saturated band, Pierrehumbert 2010, §4.4); a
  body at 90 to 120 K radiates at 25 to 60 µm, where methane has no
  strong band of its own. With it, raising Titan's methane from 1 to
  10 bar holds back under a third of the outgoing radiation that 0.01
  to 0.1 bar holds back (with the opacity 6.0, 0.6 K of warming against
  2.6 K).
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
a few per cent to the asymptote (Goldblatt et al. 2013, Fig. 2). The
spectral column does the same over Earth's air, saturated: 294.6 W/m² at
360 K, then 283.4 to 283.9 W/m² over 500-640 K. `MOIST_PLATEAU_W_M2` is
that asymptote, 283.5 W/m², computed once on a finer grid (80 levels per
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
Methane's stand-in keeps the whole pressure, its own counted as the
saturated amount `u` (see **Methane** above): it stands for Titan's
collision-induced greenhouse, which grows with the nitrogen faster than
line broadening.

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
24, 241, doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2; the
abstract read, the full text refused to scripts), as the codes that
implement their profile take it (konrad's `Manabe67`, `rh_surface=0.77`;
climlab's `FixedRelativeHumidity`, `relative_humidity=0.77`). The
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
a placeholder the chain replaces when it boils the ocean. A runaway's steam is already in the gases and is kept as it is.

**Carbonate thermostat.** On a planet with plates and water, weathering
draws CO₂ down faster when it is warm and volcanoes give it back (Walker,
Hays & Kasting 1981, doi:10.1029/JC086iC10p09776). The model sets the CO₂
so that a surface at `THERMOSTAT_K`, 288 K (a calibration on Earth), the
vapour taken at that temperature, sends out what it absorbs (solved in
`ln CO₂`). It can build the CO₂ up past what the air held before the
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
  (`atmosphere::co2_saturation_bar`, its frost below 216.6 K, its liquid
  above) it condenses, the air follows CO₂'s pseudo-adiabat
  (`column::co2_lapse`, Kasting 1991 Appendix B) and the tropopause
  (`climate::tropopause_k`, the skin temperature `T_eff 2^-¼` of the
  climate's own albedo) cold-traps it;
- its Rayleigh scattering keeps growing with the column, averaged over
  the star's black-body spectrum (`albedo::spectrum`), while its
  near-infrared absorption (`planet::shortwave`) takes back part of what
  it reflects. A red star's light scatters less and falls more in the
  absorption bands, so the maximum holds at a lower flux.

Kopparapu et al. 2013 (doi:10.1088/0004-637X/765/2/131) find the same
mechanism and put its edge at 1.70 AU round the Sun (0.343 S⊕). The model
lands closer in: for an Earth the outer edge, the last orbit with liquid
water, is 1.535 AU (0.424 S⊕, +24 %), and round 4500 K and 3300 K stars
its flux is 25 % and 20 % above their fit (the edge was 1.47 AU before
the near-infrared absorption, 1.78 AU on the grey column). Two measured
parts make the gap: the thick CO₂ column sends out too much, 73.5 W/m²
under 8 bar at 1.70 AU against Kopparapu's 67.4, so it needs less light
to stay warm; and the albedo at their maximum greenhouse is 0.455
against their 0.417. Neither is tuned away: the edge tests pin what the
model reaches (1.535 ± 0.005 AU, +25 % and +20 % ± 1 %), so that a
change shows, and issue #595 holds the search for the missing physics
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
- *Limits* (left to 2d-3). The shape holds for a spin period much
  shorter than the year; eccentricity only scales the magnitude, by
  `(1 − e²)^(−1/2)`, and leaves the distribution `s(x, β)` unchanged
  (Nadeau & McGehee 2017, eqs. 1 and 2). A slow or tidally locked
  rotator (Mercury and Venus count as slow) has no zonal annual mean
  and needs another weighting. Between 47° and 63° of obliquity (117°
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
`N2_ICE_EMISSIVITY` (0.89), which is left as it is until the climate of
2d-3 can refit both on Triton. Searched and not found: a bolometric
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

The outer edge moved from 1.47 to 1.535 AU round the Sun; the K and M
edges' fluxes from +43 % and +47 % to +25 % and +20 % of Kopparapu's.
The edge tests are frozen there; issue #595 holds the rest of the gap
(see **Carbonate thermostat** above).

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
radiates with an emissivity of 0.89 over the share of the disc its own
amount covers (`frost / 0.02 bar` of N₂ alone: the most volatile frost
condenses last and lies over the others, as Triton's N₂ lies over its
CO₂). A grey surface sends out `ε σ T⁴`: `surface_temperature_k` and the
wet balance take the column's outgoing radiation times `ε`, as through a
transparent air, which holds over a nitrogen frost (its vapour is at
most 0.125 bar). The emissivity is solved in the same fixed point as the
albedo, damped the same way, and stored with the climate so that it
reproduces (worlds stored before it read 1). 0.89 is a calibration on
Triton (above); Stansberry, Pisano & Yelle 1996 (*Planet. Space Sci.*
44, 945, doi:10.1016/0032-0633(96)00001-3, abstract checked) compute
0.85 for β-N₂ of 1 cm grains and 0.40 for 1 mm grains (α-N₂ 0.30 and
0.11), and Bertrand et al. 2022 (*Icarus* 373, 114764) model Triton
with 0.8: the calibration sits just above the coarse-grained end, a
lower value warming the model's Triton, under its frost albedo of 0.75,
past 38 K. One value serves α- and β-N₂. The rewrite takes 0.80 and
refits the pair on Triton's temperature (study 25; see "The climate
solver of the rewrite" below).

**Phase changes and convergence** (`Climate::converged`,
`climate::PHASE_BAND_K`, issue #562). Two phase changes were steps
inside the fixed points: nitrogen frost (emissivity 0.89) becomes a
black-body liquid at its triple point, 63.14 K, and methane's air halves
when its ice melts into a sea (`surface::sea_freezes`). A surface just under
such a point warmed over it and one just over it cooled under it, so the
fixed point had no solution there and cycled until its step limit,
silently. Both now pass from one phase to the other over the last 1 K
under the point (`frozen_share`): the frost's cover retreats and
methane's activity falls from 1 to 0.5 across that band. The band is a
numerical smoothing, not physics; 1 K is the narrowest over which the
damped fixed point settles at the triple point (over a band `w` the
emissivity's loop gain is about `1.8 K / w`). The condensation's fixed
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
frosts out (issue #522).
Where both exist the planet is a snowball (`Climate::snowball`) when its
formation draw `PlanetInputs::snowball_draw`, uniform in `[0, 1]`, is under
`d / (d + 0.8 bar × v / v⊕)`: the share of its history it spends frozen,
with `d` the CO₂ that thaws it and `v / v⊕` its volcanism against Earth's
(legacy's calibration; study 25 found the dated snowballs are 1.4 % of
Earth's history, not a tenth, and refits it below; 0.8 bar gives
Earth, which legacy remembered frozen about a tenth of its history, a
chance of 0.1: on the spectral column Earth at 1 AU needs 0.091 bar more
CO₂ to thaw, a chance of 0.102; 0.031 bar on the grey column). Where no
CO₂ up to its maximum greenhouse thaws it (the chance is then 1, from
about 1.4 AU for an Earth-like planet round the Sun: an icy body's cold column condenses and scatters its CO₂ before it
warms the ice to a thaw), the frozen state is for good: the planet is
frozen but not a snowball, and its card says so (the `frozen` note). So snowballs are rare near S = 1, likelier towards the
outer edge of the habitable zone and on planets that outgas little, and
`--snowball yes|no` pins the state. A snowball is drawn with the `frozen`
look and keeps its relief and coastlines; no sulphuric cloud forms in its
cold air. On the sweep of an Earth-like planet round the Sun (`climate_sweep
sweep`) the warm branch runs from the runaway at about 0.95 AU to the
outer edge at 1.535 AU (288 K, the thermostat's, with 0.47 bar of CO₂ at
1.2 AU, 2.8 bar at 1.4 AU and 5.7 bar at 1.5 AU), beyond which the frozen
branch is the only one (165 K at 1.6 AU, its CO₂ frosted out down to
0.06 bar; 152 K at 1.8 AU).

**Sulphur and SO₂.** Outgassed sulphur stays in the air as SO₂ only in
share `SULPHUR_IN_AIR` (5e-5); over liquid water or with free oxygen above
0.01 bar it is washed out to sulphate in about a day and a half
(`WET_SO2_SHARE`, 2.2e-9 of the dry lifetime), and in a dry, oxygen-free air
its lifetime grows with the column (`DRY_COLUMN_BAR`, 10 bar). That gives
Earth under 1e-6 bar of SO₂ (2e-11), Venus about 1e-2 bar (150 ppm of
92 bar, inside the 130 ± 50 ppmv below the clouds) and Mars under its 0.3 ppb
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
- `water` between the melting temperature of ice Ih and water's boiling
  point at the surface pressure (`water::ice_ih_melting_temperature` and
  `water::boiling_temperature`, both IAPWS, 373.1 K at 1 atm, the
  boiling point meeting the critical point), above the triple-point
  pressure, with water kept out of a runaway; legacy's 273.15 K
  (`ICE_MELTS_K`, uncited) and its Clausius–Clapeyron boiling point are
  not ported, and a pressure under the triple point has no boiling
  point, so no liquid water. Limit: above 208.6 MPa (about 2 086 bar)
  the melting temperature stays at the Ih–III–liquid point, 251.165 K,
  the floor of ice Ih, while the denser ices III, V and VI melt higher
  (ice VI at 273 to 355 K): water between 252 and 273 K under that
  pressure reads liquid where a denser ice is stable. Issue #671 reads
  the melting curve of the stable ice (2d-3a, study 25): IAPWS
  R14-08(2011) gives ice III `π = 1 − 0.299948 (1 − θ⁶⁰)` (eq. 2,
  `T*` 251.165 K, `p*` 208.566 MPa, to 256.164 K) and ice V
  `π = 1 − 1.18721 (1 − θ⁸)` (eq. 3, `T*` 256.164 K, `p*` 350.1 MPa,
  to 273.31 K), 3 % in pressure, checked against its Table 3 (268.685
  MPa at 254 K, 479.640 MPa at 265 K); the ice in equilibrium with the
  liquid is Ih to 208.566 MPa, III to 350.1 MPa, V to 632.4 MPa, VI to
  2216 MPa (Table 1), and eqs. 2 to 4 invert in closed form,
  `θ = (1 − (1 − π)/a)^(1/b)`;
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
`nitrogen_vapour_pressure`; study 24). The frost is the N₂-1 / N₂-2
polynomial fit of Fray & Schmitt 2009 as Lisse et al. 2021 give it in
their Table 2, pinned to the triple point (63.14 K, 12 530 Pa) and
switching phase at 35.61 K (Scott 1976, via Tegler et al. 2019), with a
10 K floor (`Definition:`); it gives Triton's 37.2 K a pressure of
1.40e-5 bar against the 1.4e-5 bar observed. The liquid is the Antoine
equation of Edejer & Thodos 1967 (via the NIST WebBook), whose fit
range stops below the triple point: it is 4.5 % low at the normal
boiling point (77.355 K, 0.955 atm) and 12.5 % low at the triple point.
That is the accuracy of the fit, kept by design, and no pressure that
the model asks for needs better. The SO₂ frost
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
test-only `MANABE_HUMIDITY` of `radiation`. `science::surface::liquids`
holds the surface liquid and the frosts. The cloud cover stays out of
`Cover`: the caller builds `Underneath { ground, clouds, deck }` from
`ground_albedo`, `radiation::clouds::cloud_cover` and
`veils::sulphuric_deck`. `system::planet::volcanism` holds the
volcanic intensity.

Calibration targets, through the whole chain from the reference bodies'
inputs round the Sun at 4.6 Gyr:

| Body | Result | Target | Observed |
|---|---|---|---|
| Earth | 288 K, water, regulated, 1.2e-4 bar of CO₂; 289.5 K at 280 ppm | 288 ± 6 K, water, regulated; 286-290 K at 280 ppm, and the thermostat's CO₂ within the CO₂ the model maps 286-290 K to (3.7e-5 to 3.7e-4 bar) | 288 K, 280 ppm before industry |
| Venus | 577 K, none | measured: pinned at 577 ± 5 K, none | 737 K |
| Mars | 214.5 K (T_eq 210.3 K) under 6.4 mbar, none | 210 ± 5 K, none | about 210 K |
| Titan | 99 K, hydrocarbons (2.2 bar); `titan-like` 93.7 K | 94 ± 8 K, hydrocarbons; `titan-like` 93.7 ± 1 K | 94 K, methane and ethane lakes |
| Mercury | 435 K, its equilibrium temperature | within 1 K of it | airless |

On the spectral column (issue #523) Earth's thermostat needs 1.2e-4 bar
of CO₂ (2.3e-3 bar on the grey column of commit 3c47979), under the
real 280 ppm: Earth is 1.5 K warm. 280 ppm under Earth's own albedo give
289.5 K, 1.5 K over the thermostat's 288 K, and the model's CO₂ moves
Earth by only about 1.8 K per factor e, so the thermostat goes down a
factor 2.3 to shed it. The cause is the clouds' longwave effect: 34
W/m² in the column against 28 W/m² measured (CERES), while its clear
sky is 4 W/m² over CERES's, so its all-sky OLR is 3 W/m² under the
measured 240 W/m². The check bounds Earth at 280 ppm (286-290 K) and
asks the thermostat's CO₂ to lie within what the model's own CO₂
sensitivity maps 286-290 K to (3.7e-5 to 3.7e-4 bar); nothing is tuned
to close the bias.
Venus reaches 577 K under its 85 bar of CO₂ and its Bond albedo 0.76
(739 K on the grey column, whose CO₂ opacity was calibrated on it): the
infrared opacity of its H₂SO₄ clouds and its SO₂ are not modelled, and
the hot bands of the line list above 700 K are unchecked (issue #594;
the spectral column before the CO₂ continuum gave 451 K). The reference Mars takes the real 6.1 mbar of CO₂
(`MARS_CO2_FACTOR`; the volatile rule alone gives it 14 mbar) and
reaches 214.5 K over an equilibrium temperature of 210.3 K: any
greenhouse puts it over the 210 K mean surface temperature observed
(legacy's figure; the NASA fact sheet gives ~214 K, see study 25),
which a uniform 1-D surface cannot reach (the real surface radiates more
than `σ T_mean⁴` for its uneven temperatures); `mars-like`, with 4.8
mbar, reaches 215 K too on the distance sweep (`climate_sweep sweep`),
and 213.3 to 214.1 K under 5.7 to 5.9 mbar over seeds 1 to 40 of
`archetype_census`, where `venus-like` reaches 552 to 601 K (median
575 K) under 75 to 98 bar.

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
minimum near 8 bar), digitised by legacy; the test keeps it with
± 0.04. The model's edge (1.535 AU, frozen by the user on #523) is
+19.2 % in flux over the 2014 fit round the Sun, +21.0 % at 4500 K and
+16.4 % at 3300 K (derived; +23 %, +25 % and +20 % over the 2013 v2
set).

#### Condensation at the surface

`condense` keeps in the air at most `a · p_sat(T_s)` of each condensing
gas and puts the rest on the ground, nitrogen first, then methane:
`p_sat` the pure gas's saturation pressure at the surface temperature
(`science::volatiles`), `a` the activity, 1 for nitrogen and for
methane ice, `SEA_METHANE_SHARE` over a sea, ramped over `PHASE_BAND`.
A `Definition:` (phase equilibrium and Raoult's law); its one number
is `SEA_METHANE_SHARE`. CO₂ is lowered to its saturation by the
collapse, and NH₃ is frozen by the cold trap at the equilibrium
temperature only, as in legacy. Titan's methane, 0.0829 ± 0.0026 bar,
sits 0.6 σ under the cap at 93.65 K, 0.0845 bar (derived): a Titan
whose inventory exceeds the cap gets seas and an air of 0.0845 bar,
inside the measurement; the tests give Titan an inventory, not its
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
al. 1994's 40 ± 2 K. Searched and not found: a bolometric albedo and
an emissivity of Triton measured apart from this balance.

#### The runaway branch

- `A_rg = 1 − 4 F/(S_rg S₀)` is a `Definition:`, with `S₀` the
  rewrite's `SOLAR_IRRADIANCE` (1361 W/m², IAU 2015 B3); legacy's
  `REFERENCE_ALBEDO` (0.3, which rebuilt `S₀` from Earth's 255 K as
  1369.9 W/m²) is not ported. Round the Sun `A_rg` is 0.247 (derived).
- `RUNAWAY_ALBEDO` (0, 0.9) is a guard (`Definition:`): over the fit's
  2600 to 7200 K, `A_rg` runs from 0.087 to 0.357 on the 283.5 W/m²
  plateau (derived).
- **Calibration of `STEAM_ALBEDO_BAR`** (1 bar, kept). Kopparapu et al.
  2013 reach the moist limit "at a surface temperature of 340 K" and
  the albedo's minimum "at a surface temperature of 400 K" (§ III.1);
  water's saturation there is 0.27188 bar and 2.4577 bar (NIST
  Chemistry WebBook, read): the blend's e-folding sits between.
  Searched and not found: the vapour column at the runaway limit as a
  number, in Kopparapu et al. 2013 and 2014 and Goldblatt et al. 2013.
- `RUNAWAY_PROBE_K` (380 K) is a shortcut that changes no result, a
  `Definition:` near the column's own maximum (294.6 W/m² at 360 K);
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
times 0.8 bar. A tier-1 test holds Earth's chance in 0.013 to 0.016.
Hoffman et al. quote Walker et al. 1981: "Even the present rate of
release would yield 1 bar of carbon dioxide in only 20 million years",
so 6.4 bar is about 130 My of today's outgassing per temperate phase.
Searched and not found: a snowball frequency for another planet, and
the length of the Paleoproterozoic snowballs. Where no CO₂ thaws the
body, `thaw_bar` is infinite and the trace takes no non-finite value:
the chance 1 is recorded without it.

#### Calibration of `CH4_OPACITY` (issue #646, refit)

Target: Titan's surface, 93.65 ± 0.25 K at 1467 hPa (Fulchignoni et
al. 2005), on the sourced column of `radiation::test_points` (1.467
bar, 5.65 % methane, effective temperature 82 K), with the new methane
curve and the anti-greenhouse of `CH4_NEAR_IR_HALF_BAR` (0.0013 bar,
already refitted); McKay, Pollack & Courtin 1991's 82 K + 21 K − 9 K
make the same 94 K. Method: bisect the opacity until that column
gives 93.65 K, three figures; a tier-1 test holds the column at
93.65 ± 0.25 K, and the generated Titan through the chain is a
validation (94 ± 8 K), no longer the target.

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
| `COLLAPSE_ITERATIONS` | 100 | Definition | a collapsing Earth at 1.6 AU settles under it | 1 |
| `COLLAPSE_TOLERANCE` | 1e-4 | Calibration | at 1e-5 the temperature moves under 0.01 K | 3 |
| `PHASE_BAND` | 1 K | Calibration | Titan-like and Triton-like sweeps settle across their points | 3 |
| `THAW_STEPS` | 12 | Calibration | at 16 steps Earth's thaw CO₂ moves under 0.1 % | 3 |
| `ALBEDO_ITERATIONS` | 200 | Definition | the reference climates settle under it | 1 |
| `ALBEDO_TOLERANCE_K` | 0.1 K | Calibration | at 0.01 K the reference climates move under 0.1 K | 1 |
| `ALBEDO_DAMPING` | 0.5 | Definition | at 0.25 the converged climates agree | 3 |
| `CONDENSE_ITERATIONS` | 100 | Definition | Titan and Triton settle under it | 1 |
| `THERMOSTAT_TOLERANCE` | 1e-3 in ln CO₂ | Calibration | at 1e-4 Earth moves under 0.01 K | 1 |
| `GREENHOUSE_DECADES` | 4 | Definition | the reference maxima lie inside the range | 1 |
| `GREENHOUSE_STEPS` | 18 | Calibration | at 30 steps the outer edge moves under 0.001 AU | 3 |

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
  0.165 mbar, O₂ 0.010 mbar, derived; argon has no field). Legacy's
  `MARS_CO2_FACTOR` is not ported. The fact sheet gives "Average
  temperature: ~214 K"; no source for the legacy 210 K was found (it is
  Mars's equilibrium temperature): the bound is a question for the
  user (214 ± 5 K proposed).
- **Earth** 288 K (fact sheet), **Titan** 93.65 ± 0.25 K (Fulchignoni
  et al. 2005), **Venus** pinned at 577 ± 5 K (#594).

#### The reducing-air fallback

A reducing body keeps the air solved with its CO₂ held back only if
its climate leaves methane and a total pressure over methane's
triple-point pressure (0.11696 bar, Setzmann & Wagner 1991 as `volatiles_tests.rs` cites
it, read as `volatiles::methane_saturation_pressure` at the triple-point
temperature: a `Definition:`, one code path); otherwise it is solved
again with its CO₂. No new constant.

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
again. A known simplification; on Earth the model's abiotic air has no
methane anyway, so Earth's 288 K does not move.

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
`system::planet::bare::bare`: composition, interior, XUV fluence, escape,
heat, then for a body that has lost its envelope the water, the regime, the
core and the air, before any climate. The model is private until the
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
| 3 | `steam-world` | in water runaway, at least 2206 bar of steam (ten times water's critical pressure) and a surface at 647 K or more (`climate::hides_surface`) |
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
of ten times the critical pressure (`STEAM_WORLD_MIN_BAR`) is a
calibration: in the census no runaway holds between 1e3 and 1e5 bar of
steam, so it cuts through an empty band. A wet planet is never one.
Once the surface is known, the chain derives the interior again
(`interior::steam_interior`): the water layer gives way to a `Steam`
layer of the water kept, on top of the iron and rock. Its thickness
comes from Turbet et al. 2020 (A&A 638, A41, arXiv:1911.08878): Eq. 2
in the form of their Appendix C, for an isothermal envelope in
hydrostatic balance. The printed Eq. 2 has `RT/(Mg)` upside down.

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
steam world below its condensed twin. The paper's own example comes back
1.17 R⊕ (a 0.3 M⊕ silicate core with 5 % water; "around 1.2 R⊕").
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

- Anderson et al. 2011, doi:10.1126/science.1211001, Mercury's
  dipole.
- Angus, de Reuck et al. 1979, nitrogen's triple point, through
  the NIST Chemistry WebBook.
- Atake and Chihara 1976, ethane's triple point (90.341 K) and
  enthalpy of fusion (0.583 kJ/mol), through the NIST Chemistry
  WebBook.
- Atreya, Donahue and Kuhn 1978, *Science* 201, 611,
  doi:10.1126/science.201.4356.611, Titan's N₂ from NH₃.
- Bailey, Ahlsved and Meadows 2011, arXiv:1102.2554, Titan's surface
  albedo in the methane windows (read).
- Baraffe et al. 2015, the luminosity of low-mass stars at the ZAMS.
- Bertrand et al. 2022, *Icarus* 373, 114764, Triton.
- Biver et al. 2022, *Comets III*, arXiv:2207.04800, Table 3.
- Blakley et al. 2024, *Planet. Space Sci.*,
  doi:10.1016/j.pss.2024.105863, arXiv:2403.09917, NH₃'s sublimation
  pressure at 80 to 120 K.
- Bono et al. 2019, the age of Earth's inner core.
- Boukrouche, Lichtenberg and Pierrehumbert 2021,
  doi:10.3847/1538-4357/ac1345.
- Budyko 1969, *Tellus* 21, 611, doi:10.3402/tellusa.v21i5.10109, the
  ice cover's edge (read).
- Bullock and Grinspoon 2001, doi:10.1006/icar.2000.6570,
  Venus's SO₂.
- Buratti et al. 2017, *Icarus* 287, 207, doi:10.1016/j.icarus.2016.11.012,
  Pluto's Bond albedo (read in arXiv:1604.06129).
- Carlson 1999, *Science* 283, 820,
  doi:10.1126/science.283.5403.820.
- Charette and Smith 2010, *Oceanography* 23(2), 112,
  doi:10.5670/oceanog.2010.51, Table 1, the volume and area of Earth's
  ocean (read).
- Charnay et al. 2013, doi:10.1002/jgrd.50808.
- Christensen 2006, doi:10.1038/nature05342, the deep dynamo.
- Christensen 2010, doi:10.1007/s11214-009-9553-2, the energy-flux
  scaling of the field.
- Christensen, Holzwarth and Reiners 2009,
  doi:10.1038/nature07626.
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
- Driscoll and Barnes 2015, doi:10.1089/ast.2015.1325.
- Edejer and Thodos 1967, *J. Chem. Eng. Data* 12(2), 206,
  doi:10.1021/je60033a014, the Antoine fit of liquid nitrogen's vapour
  pressure, 63.14 to 126 K, through the NIST Chemistry WebBook.
- Erkaev et al. 2007, energy-limited escape.
- Fegley and Prinn 1989, doi:10.1038/337055a0, Venus's SO₂ and
  calcite.
- Feistel and Wagner 2007, *Geochim. Cosmochim. Acta* 71, 36, the
  sublimation pressure of water ice, as Lisse et al. 2021 reproduce it.
- Foley, Bercovici and Landuyt 2012, tectonics against
  planet size.
- Fonseca and Lobo 1989, NH₃'s triple-point pressure, through the
  NIST Chemistry WebBook.
- Fray and Schmitt 2009, *Planet. Space Sci.* 57, 2053,
  doi:10.1016/j.pss.2009.09.011, sublimation pressures (closed; its
  Table 2 read as Lisse et al. 2021 reproduce it).
- Fretwell et al. 2013, *The Cryosphere* 7, 375,
  doi:10.5194/tc-7-375-2013, Bedmap2 (read).
- Fulchignoni et al. 2005, *Nature* 438, 785, doi:10.1038/nature04314,
  Titan's surface temperature and pressure.
- Fulton et al. 2017, the radius bins of the census.
- Gao et al. 2023, *J. Phys. Chem. Ref. Data* 52,
  doi:10.1063/5.0128269, NH₃'s triple-point temperature.
- Giauque and Stephenson 1938, *J. Am. Chem. Soc.* 60, 1389,
  through the NIST Chemistry WebBook.
- Giles et al. 2024, arXiv:2405.19253, Io's atmosphere from IRTF/TEXES
  spectra.
- Glein and Shock 2013, *Geochim. Cosmochim. Acta* 115, 217,
  doi:10.1016/j.gca.2013.03.030, Titan's seas (full text read).
- Goldblatt 2015, doi:10.1089/ast.2014.1268.
- Goldblatt et al. 2009,
  doi:10.1038/ngeo692, nitrogen and Earth's temperature.
- Goldblatt, Robinson, Zahnle and Crisp 2013,
  doi:10.1038/ngeo1892, the moist limit.
- Gray and Nagel 1989, the limit of the convective envelope.
- Gurrola 1995, PhD thesis, Stanford University, Triton's surface
  pressure from the Voyager 2 radio occultation, 14 ± 1 µbar (not
  reached; as Sicardy et al. 2024 quote it).
- Hall et al. 1995 and 1998, the O₂ of Europa and Ganymede.
- Hamano, Abe and Genda 2013, *Nature* 497, 607, doi:10.1038/nature12163,
  the steam over a magma ocean (abstract read).
- Hamano et al. 2015, doi:10.1088/0004-637X/806/2/216.
- Hapke 1981, *JGR* 86, 3039.
- Henderson-Sellers and Wilson 1983, *Rev. Geophys.* 21, 1743,
  doi:10.1029/RG021i008p01743, surface albedos (abstract read).
- Hillier et al. 1990, *Science* 250, 419,
  doi:10.1126/science.250.4979.419, Triton's Bond albedo (abstract read).
- Hoffman et al. 2017, doi:10.1126/sciadv.1600983.
- Hörst, Vuitton and Yelle 2008, doi:10.1029/2008JE003135.
- Howett et al. 2010, Enceladus's albedo.
- Huebner, Keady and Lyon 1992, *Astrophys. Space Sci.*
  195, 1, photo-ionisation rates.
- Jackson, Davis and Wheatley 2012, *MNRAS* 422, 2024,
  Table 2.
- Jin et al. 2004, *Geophys. Res. Lett.* 31(22), doi:10.1029/2004GL021180,
  ocean surface albedo (abstract read).
- Johnson 1990, *Energetic Charged-Particle Interactions with
  Atmospheres and Surfaces*.
- Johnson et al. 2004, "Radiation effects on the surfaces of the
  Galilean satellites", in *Jupiter: The Planet, Satellites and
  Magnetosphere*, ch. 20.
- Johnson and Goldblatt 2015, doi:10.1016/j.earscirev.2015.05.006,
  Earth's nitrogen.
- Johnstone et al. 2021, *A&A* 649, A96,
  doi:10.1051/0004-6361/202038407.
- Karkoschka 2001, albedos of the Uranian moons.
- Karwat 1924, NH₃'s sublimation pressure at 162.39 K, through Fray
  and Schmitt 2009 as Blakley et al. 2024 quote it.
- Kasting 1991, Appendix B, CO₂'s pseudo-adiabat.
- Kelemen and Manning 2015, doi:10.1073/pnas.1507889112,
  carbon recycling.
- Kempf et al. 2010, Enceladus's plume fallout.
- Kite and Ford 2018, *ApJ* 864, 75,
  doi:10.3847/1538-4357/aad6e0, waterworlds.
- Kite, Fegley, Schaefer and Gaidos 2016, *ApJ* 828, 80,
  doi:10.3847/0004-637X/828/2/80, arXiv:1606.06740 (read).
- Kite, Manga and Gaidos 2009,
  doi:10.1088/0004-637X/700/2/1732, the outgassing of lids and plates.
- Kivelson et al. 2004, in *Jupiter: The Planet, Satellites and
  Magnetosphere*, ch. 21.
- Kopparapu et al. 2013, doi:10.1088/0004-637X/765/2/131.
- Kopparapu et al. 2014, doi:10.1088/2041-8205/787/2/L29, the
  runaway limit.
- Korenaga 2010, plate tectonics against planet size.
- Korenaga, Planavsky and Evans 2017, *Phil. Trans. R. Soc. A* 375,
  20150393, doi:10.1098/rsta.2015.0393, §2a: Earth's water by
  reservoir.
- Krasnopolsky 2012, doi:10.1016/j.icarus.2011.10.019, Mars's SO₂
  limit.
- Lacis et al. 2010, doi:10.1126/science.1190653.
- Lainey et al. 2009, doi:10.1038/nature08108, Io's tidal
  dissipation.
- Lay, Hernlund and Buffett 2008, doi:10.1038/ngeo.2007.44,
  the core's heat flow.
- Lecavelier des Etangs 2007, energy-limited escape.
- Lemmon et al. 2013, ethane's triple point (90.4 K), as Richardson,
  Hartwig and Leachman 2019 quote it (not re-read).
- Lillis et al. 2013, Mars's dynamo.
- Lipiński et al. 2007, *Int. J. Thermophys.* 28, 1904,
  doi:10.1007/s10765-007-0267-y, nitrogen's alpha-beta transition
  (closed).
- Lisse et al. 2021, *Icarus* 356, 114072,
  doi:10.1016/j.icarus.2020.114072, supplementary Table 2 (read, and
  its appendix Table 2 in arXiv:2009.02277).
- Lora and Ádámkovics 2017, arXiv:1610.06277, Titan's near-surface
  methane humidity (read).
- Lyons, Reinhard and Planavsky 2014,
  doi:10.1038/nature13068, the Great Oxidation Event.
- Mandt, Mousis, Lunine and Gautier 2014, *ApJ Letters*
  788, L24, doi:10.1088/2041-8205/788/2/L24.
- Manabe and Wetherald 1967, *J. Atmos. Sci.* 24, 241,
  doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2 (abstract read;
  the surface humidity 0.77 through the konrad and climlab codes).
- Marcq et al. 2008, Venus's SO₂ below the clouds.
- Marty 2012, *Earth Planet. Sci. Lett.* 313-314, 56-66,
  doi:10.1016/j.epsl.2011.10.040, carbon in the mantle; Earth's bulk
  water, p. 18 of the arXiv author copy (arXiv:1405.6336).
- McDonough 2003, *Treatise on Geochemistry* 2.15.
- McEwen et al. 1998, *Science* 281, 87, Io's lavas.
- McGehee and Lehman 2012, *SIAM J. Appl. Dyn. Syst.* 11(2), 684,
  the annual insolation integral (cited by Nadeau and McGehee).
- McKay, Pollack and Courtin 1991,
  doi:10.1126/science.11538492, Titan's greenhouse.
- Mittelholz et al. 2020, a late field on Mars.
- Moore et al. 1999, Callisto.
- Moore and Webb 2013, doi:10.1038/nature12473, "Heat-pipe
  Earth".
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
  (read).
- Neukum, Ivanov and Hartmann 2001, the crater
  chronology.
- Niemann et al. 2010, *J. Geophys. Res.* 115, E12006,
  doi:10.1029/2010JE003659, Titan's methane at the Huygens site
  (abstract read; supersedes their 2005 results).
- Nimmo 2015, the core's heat flow.
- Nimmo and Stevenson 2000, doi:10.1029/1999JE001216.
- NIST Atomic Spectra Database, the H I Lyman α multiplet.
- NIST CODATA, the Planck constant (exact).
- Noack et al. 2016, *Icarus* 277, 215,
  doi:10.1016/j.icarus.2016.05.009.
- North, Cahalan and Coakley 1981, *Rev. Geophys. Space Phys.* 19, 91,
  doi:10.1029/RG019i001p00091, energy balance climate models (read).
- NSIDC, the sea ice pages (quick facts; sea ice), read 2026-11-24.
- Ohtani 2021, *Annu. Rev. Earth Planet. Sci.* 49, 253,
  doi:10.1146/annurev-earth-080320-062509, the large and small water
  budgets (abstract, through study 22).
- O'Reilly and Davies 1981, *Geophys. Res. Lett.* 8, 313.
- Overstreet and Giauque 1937, NH₃'s sublimation enthalpy, through
  the NIST Chemistry WebBook.
- Owen and Wu 2017, doi:10.3847/1538-4357/aa890a, eq. 19.
- Payne 1972, *J. Atmos. Sci.* 29, 959,
  doi:10.1175/1520-0469(1972)029<0959:AOTSS>2.0.CO;2, the albedo of
  the sea (abstract read).
- Perovich et al. 2002, *J. Geophys. Res.* 107(C10),
  doi:10.1029/2000JC000438, the albedo of Arctic sea ice (abstract
  read).
- Pierrehumbert 2010, section 4.4.
- Plaut et al. 2007, *Science* 316, 92, doi:10.1126/science.1139672,
  Mars's south polar layered deposits (abstract read).
- Quirico et al. 1999, *Icarus* 139, 159, Triton's CO₂ ice.
- Ribas et al. 2005, *ApJ* 622, 680, doi:10.1086/427977, Table 5.
- Robinson and Catling 2012, *ApJ* 757, 104, section 4.3.
- Robinson and Catling 2014, doi:10.1038/ngeo2020, the
  tropopause.
- Rubin et al. 2019, doi:10.1093/mnras/stz2086, Table 2, comet 67P.
- Schröder and Keller 2008, *Planet. Space Sci.* 56, 753,
  doi:10.1016/j.pss.2007.10.011, the Huygens landing site's reflectance
  (read in arXiv:1702.00653).
- Schröder and Schmitt 2007, X-rays of A stars.
- Scott 1976, *Phys. Rep.* 27, 89, doi:10.1016/0370-1573(76)90032-6,
  nitrogen's alpha-beta transition (closed; through Tegler et al. 2019).
- Sicardy et al. 2024, arXiv:2402.02476 (accepted by *A&A*), Triton's
  air from occultations 1989 to 2022 (read).
- Simonelli and Veverka 1988, Io's Bond albedo.
- Spencer 1987, dark moons.
- Stansberry, Pisano and Yelle 1996,
  *Planet. Space Sci.* 44, 945, doi:10.1016/0032-0633(96)00001-3.
- Stevenson, Spohn and Schubert 1983,
  doi:10.1016/0019-1035(83)90241-5, the thermal history of the core.
- Stewart, Jacobsen et al. 1986, nitrogen's properties
  through the NIST Chemistry WebBook.
- Tan, Kargel and Marion 2013, *Icarus* 222, 53,
  doi:10.1016/j.icarus.2012.10.032, Titan's seas (closed; through Glein
  and Shock 2013, Table 4).
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
- Trenberth and Smith 2005, *J. Climate* 18, 864,
  doi:10.1175/JCLI-3299.1, the mass of Earth's dry air.
- Tu et al. 2015, the saturation of a star's X-ray emission.
- Turbet et al. 2020, *A&A* 638, A41, arXiv:1911.08878, Eq. 2 and
  Appendix C.
- Tyler et al. 1989, *Science* 246, 1466,
  doi:10.1126/science.246.4936.1466, Voyager 2's radio occultation of
  Triton: 1.6 ± 0.3 Pa and 38 K (abstract read).
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
  pressure of ice Ih, as `crates/science/src/water.rs` cites it.
- Walker, Hays and Kasting 1981,
  doi:10.1029/JC086iC10p09776, the carbonate thermostat.
- Wang, Lineweaver and Ireland 2018,
  doi:10.1016/j.icarus.2017.08.024, Table 1.
- Ward 1974, *J. Geophys. Res.* 79(24), 3375, the annual insolation
  of a rapid rotator (cited by Nadeau and McGehee).
- Warren 1982, *Rev. Geophys.* 20, 67, doi:10.1029/RG020i001p00067,
  the optical properties of snow (abstract read).
- Weber et al. 2011, the Moon's core.
- Weiss and Tikoo 2014, the lunar dynamo.
- Witt and Kemp 1937, *J. Am. Chem. Soc.* 59, 273, ethane's enthalpy
  of vaporisation, through the NIST Chemistry WebBook.
- Wolf and Toon 2014, doi:10.1089/ast.2013.1112.
- Wright et al. 2011, *ApJ* 743, 48.
- Yung, Allen and Pinto 1984, *ApJS* 55, 465,
  doi:10.1086/190963, Titan's photochemistry, Tables 2A and 5B.
- Zahnle and Catling 2017, *ApJ* 843, 122,
  doi:10.3847/1538-4357/aa7846, the cosmic shoreline.
- Zahnle et al. 2003, doi:10.1016/S0019-1035(03)00048-4.
