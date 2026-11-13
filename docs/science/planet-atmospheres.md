# Planet atmospheres, climate and surface (part 2)

Part 2 of the physical planet model (issue #423) derives what a planet's
interior does not fix: whether it keeps its envelope, its tectonic
regime, atmosphere, climate, liquid and life, and reads the surface
classes off them. The sections follow the order of the derivation, from
the XUV fluence to the classes, and end with a census of drawn systems.
The interiors are in [planet-interiors.md](planet-interiors.md).

## XUV fluence

`planet::xuv::xuv_fluence` gives the XUV energy per unit area a planet
has received over its system's age, relative to Earth's (the Sun at
1 AU at 4.567 Gyr is 1.0). It is the budget that drives photoevaporation.

The star emits `L_XUV / L_bol = 3.71e-4` (1 to 1180 Å) while saturated:
the Sun's own, from the fit `F = 29.7 τ^-1.23` erg s⁻¹ cm⁻² at 1 AU of
Ribas et al. 2005 (doi:10.1086/427977; τ in Gyr, saturated before 0.1
Gyr; their Table 5), 504 erg s⁻¹ cm⁻² at 0.1 Gyr over the 1361 W/m²
solar constant (fit and band checked against the paper; the value before
0.1 Gyr is an extrapolation, Ribas et al. use no star that young). It is
below the X-ray saturation of the fastest rotators, `L_X / L_bol ≈
10^-3` (Wright et al. 2011, doi:10.1088/0004-637X/743/1/48, mean
`log R_X = -3.13`; checked).

The saturation lasts `t_sat`, a labelled calibration interpolated in
log-log through three anchors (`SATURATION_GYR`), not a published
relation:

| Star mass | `t_sat` | Anchor |
|---|---|---|
| 1 M☉ | 0.1 Gyr | Jackson, Davis & Wheatley 2012 (MNRAS 422, 2024, doi:10.1111/j.1365-2966.2012.20657.x): `log τ_sat = 7.84` (70 Myr) for the Sun's colour bin, turn-off around 100 Myr for FGK stars (they cover no M dwarfs); 0.1 Gyr keeps Ribas et al.'s fit, which starts there |
| 0.5 M☉ | 0.2 Gyr | Johnstone et al. 2021 (A&A 649, A96, doi:10.1051/0004-6361/202038407): slow and fast rotators stay on identical, saturated tracks until about 200 Myr |
| 0.25 M☉ | 2 Gyr | the same, about 2 Gyr |

Between the anchors `t_sat` goes as `M^-1` above 0.5 M☉ and `M^-3.3`
below, extended past the ends: 0.42 Gyr at 0.4 M☉, 4.2 Gyr at 0.2 M☉,
42 Gyr at 0.1 M☉ (saturated all its life, as Johnstone et al. find the
smallest stars still are at 5 Gyr). Then the XUV decays as `t^-1.23`
(Ribas et al. 2005, doi:10.1086/427977). The integral over the age is
analytic and is multiplied by the insolation. The luminosity is taken
constant over the age.

Calibration targets: Earth is 1.0; its absolute value
`EARTH_XUV_FLUENCE_J_M2` = 5.64e15 J/m² is Ribas et al.'s fit integrated
over 4.567 Gyr, and today's flux 4.6 mW/m² is theirs, both within 1 %; 5
to 10 Gyr adds under 30 %; a 0.2 M☉ star at Earth's insolation delivers
more than 10 times Earth's fluence.

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
order 0.1 for low-mass planets", checked against the paper). The
efficiency really falls for deep potential wells (their eq. 31); the
model ignores that, since a giant loses next to nothing at `η = 0.1`
already. `energy_limited_loss_kg` gives `ΔM`; the water loss of the next
section reuses it.

`envelope_after_escape` caps the loss at the envelope, lowers the mass by
what is lost, and sets the envelope to zero once it falls under 0.1 % of
the mass (`MIN_ENVELOPE`): what is left is a thin atmosphere, not a
hydrogen shell. The radius is the one before the loss, an upper bound.
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

`planet::regime::water` and `planet::regime::regime` (not yet chained into
`planet_model`).

**Runaway.** A planet whose insolation exceeds the runaway limit of
`star::habitable_fluxes` (Kopparapu et al. 2014,
doi:10.1088/2041-8205/787/2/L29, for a 1 M⊕ planet) cannot keep liquid
water: the oceans sit in the air as vapour. The mass correction of that
paper is not applied (not checked). There the XUV splits the water and
the hydrogen escapes at the energy-limited rate of the previous section
(Owen & Wu 2017), 9 kg of water for each kg of hydrogen:
`kept = max(0, water - 9 H lost)` (`regime::steam`). Without a runaway
the planet keeps its water. `oceans` counts it in units of 1.4e21 kg.
The climate can find a runaway this limit misses: when the vapour
feedback carries a wet surface past water's critical point (647 K,
`climate::boils`), the ocean is steam and the chain derives the body
again from `regime::steam` (see [the chain](#the-chain-and-the-surface-classes)).
`Water::boiled` records that a runaway came from such a boil rather than
from the insolation limit. In a runaway all the water kept is steam in
the air: `Water::kept_kg` is what escape to space leaves of it, and the
air's H₂O is that water's weight.

**Regime.** A labelled calibration, since the literature disagrees on the
mass dependence (see [plate tectonics](plate-tectonics.md)):

- heat pipe above 1 W/m² of internal heat, a calibration: no paper sets
  a threshold. Io is the heat-pipe body of today, carrying about
  2.5 W/m², some 40 times Earth's mean (Moore & Webb 2013, "Heat-pipe
  Earth", doi:10.1038/nature12473, from Veeder et al. 2004; checked);
  Lainey et al. 2009 (doi:10.1038/nature08108, not checked) measure
  2.24 ± 0.45 W/m². 1 W/m² keeps Io a heat pipe even at the low end of
  that range;
- plates from 0.3 M⊕ (`geology::PLATES_MIN_MASS_EARTH`, shared with the
  generator) when liquid water is kept, never in a runaway (its steam
  does not weaken the lithosphere);
- episodic when the mass allows plates but the water is gone or is steam
  (Venus);
- stagnant otherwise (Mars, Mercury, the Moon).

Calibration targets: Earth keeps 0.9 to 1.1 oceans and has plates; Venus
is in runaway, loses even an Earth ocean and is episodic; Mars and
Mercury are stagnant; Io with 1e14 W of tidal heat is a heat pipe, and
still is with 7.5e13 W (1.79 W/m², the low end of Lainey et al.); a dry
5 M⊕ planet outside the runaway has no plates.

## Core and dynamo

`planet::core::core` (not yet chained into `planet_model`) gives the
state of the iron core (`none`, `liquid` or `frozen`), whether a dynamo
runs and the surface equatorial dipole field in µT. It is a parametrised
thermal history in the style of Stevenson, Spohn & Schubert 1983
(doi:10.1016/0019-1035(83)90241-5) and Driscoll & Barnes 2015
(doi:10.1089/ast.2015.1325), cut down to one cooling time per body, and
a calibration: the targets below are the test, the constants are not
checked against their papers. The present regime is taken for the whole
history.

**Cooling time.** The core's excess heat leaves across the mantle above
it, so it decays as `e^(-t/τ)` with

```text
τ = ρ_core c r_core d / (3 k Nu)
```

where `d = R - r_core` is the mantle's thickness, `c = 800 J/(kg K)` and
`k = 4 W/(m K)` (Stevenson et al. 1983, from memory), and `Nu` the
convective speed-up by regime: 50 for plates, 10 for episodic, 5 for a
stagnant lid or a heat pipe (calibration). The three levers:

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
| Flux below the adiabat | 0.075 τ | none |
| Inner core growing | 1 τ | deep, compositional |
| Frozen | 2.25 τ | none |

**Field.** The heat flux out of the core is
`q = ρ c r_core ΔT₀ e^(-t/τ) / (3τ)`, with `ΔT₀ = 2800 K` set so that
Earth's core loses about 10 TW today (Lay, Hernlund & Buffett 2008,
doi:10.1038/ngeo.2007.44). The energy-flux scaling of Christensen 2010
(doi:10.1007/s11214-009-9553-2) gives the rms field in the core,
`B² / 2µ₀ = 0.63 ρ^(1/3) q^(2/3)` (efficiency and ohmic share taken as
1), with the prefactor 0.63 of a dipolar dynamo (Christensen, Holzwarth
& Reiners 2009, doi:10.1038/nature07626, p. 168; checked; a multipolar
one has 0.23). A share of it is the dipole at the core's surface: 0.067 for a
dynamo driven from the top of the core, 3e-4 for a deep dynamo under a
stably stratified top, whose field is multipolar and screened
(Christensen 2006, doi:10.1038/nature05342). The factor `(r_core / R)³`
carries it to the surface.

Calibration targets, with the regimes of the previous section:

| Body | Age | Result | Observed |
|---|---|---|---|
| Earth | 4.567 Gyr | dynamo, 29 µT; inner core from 4.0 Gyr | about 30 µT; inner core about 0.5 Gyr old |
| Mercury | 4.567 Gyr | deep dynamo, 0.25 µT | about 0.2 µT (Anderson et al. 2011, doi:10.1126/science.1211001) |
| Mars | 0.3 Gyr | dynamo, 15 µT | magnetised old crust |
| Mars | 4.567 Gyr | liquid core, no dynamo (ends near 0.6 Gyr) | none since about 4.1 Ga |
| Venus | 4.567 Gyr | liquid core, no dynamo (ends near 1.3 Gyr) | none |
| Moon | 4.567 Gyr | frozen core (near 3.3 Gyr), no dynamo | none since 1 to 2.5 Ga |

The Moon's core is in fact still partly molten (Weber et al. 2011); the
model freezes it, which only matters for its state, not its dynamo.

## Atmosphere

`planet::atmosphere` (not yet chained into `planet_model`) gives the
partial pressures at the surface, in bar, of N₂, CO₂, H₂O, CH₄, NH₃, SO₂
and O₂ (`Gases`, with `total()`), in five steps, called in this order:
`outgassed`, `atmosphere`, `methane_photolysis`, `cold_trap`,
`photochemistry`. The carbonate
thermostat of the climate takes Earth's CO₂ away afterwards, and life
adds the oxygen.

**Outgassing** (`outgassed`). Each volatile the planet holds (the
`Volatiles` of its inputs, kg per kg of planet) times its mass, times the
share its regime brings to the surface over its life, weighs
`P = m g / (4π R²)` on the surface. The shares are a calibration within
the picture of Kite, Manga & Gaidos 2009 (doi:10.1088/0004-637X/700/2/1732,
lids outgas less than plates):

| Regime | Share outgassed | Calibrated on |
|---|---|---|
| Plates | 0.95, of which subduction returns 80 % of the nitrogen (N₂ and NH₃) and 63 % of the rest | Earth's N₂ once its NH₃ is photolysed (Goldblatt et al. 2009, doi:10.1038/ngeo692: about 0.5 times the air's nitrogen in the crust and more than 1.4 times in the mantle; checked in their abstract) |
| Episodic lid | 0.95 | Venus's 90 bar of CO₂ |
| Heat pipe | 0.95 | none (Io) |
| Stagnant lid | 0.2 | Mars, Titan |

**Under a deep ocean** (`released_under_ocean`), only a share of that
reaches the air: `1 / (1 + (P_floor / 3 GPa)^4.1)`, with `P_floor` the
weight of the water kept on the seafloor, `m_water g / (4π R²)`. The
weight of a deep ocean curtails the decompression melting that outgasses
the mantle (Kite & Ford 2018, ApJ 864, 75, doi:10.3847/1538-4357/aad6e0,
for waterworlds of 10 to 1000 times Earth's water, whose pCO₂ often lands
between 0.2 and 20 bar; checked), high-pressure ice at its base seals the
rock (Noack et al. 2016, Icarus 277, 215, doi:10.1016/j.icarus.2016.05.009;
the paper was not read, its companion abstract gives no single pressure),
and the ices' own volatiles stay dissolved in the ocean or locked in
clathrates. The 3 GPa and the power 4.1 are calibrations with no source:
Earth's ocean (0.03 GPa) changes nothing, Titan (1.0 GPa in the model),
Ganymede (1.1 GPa) and Callisto keep 98 % or more of their air (the power
4 left Ganymede 97.9 %), and a 1 M⊕ planet of 30 % water (17 GPa) lets
out about 1e-3 of it, a few bar of CO₂ before the thermostat. A
runaway's steam weighs on the rock as its ocean did, so the share holds
in a runaway too; the steam itself is not held back, by this share or by
the regime's: it is the ocean, not a volatile of the mantle.

A known departure: Kite & Ford (Fig. 5) find that a stagnant-lid planet
under a 3 GPa ocean does not melt at all, so 3 GPa is their full
shutoff, not a half point. A shutoff there (tried with
`1 - (P / 3 GPa)^5`) keeps the icy moons and Earth but leaves the
30 %-water planet no air at all, frozen at 257 K at 1 AU instead of an
ocean world: its air would have to come from the volatiles dissolved in
its ocean, which the model does not carry.

Sulphur comes up as SO₂, of which 1e-4 stays in the air (the rest ends in
the crust; Venus's SO₂ is buffered by its surface, Fegley & Prinn 1989,
not checked; calibrated on Venus's 150 ppm). Water is vapour only in a
runaway greenhouse, and then all that the runaway left
(`Water::kept_kg`); otherwise it is an ocean or ice, and the climate sets
its vapour. The escape of the cosmic shoreline (below) takes the same
share of the steam as of every gas, and the chain lowers
`Water::kept_kg` to what is left, so the water kept and the H₂O in the
air always agree.

**Escape** (`atmosphere`). The cosmic shoreline of Zahnle & Catling 2017
(ApJ 843, 122, doi:10.3847/1538-4357/aa7846) splits the bodies with and
without air by `I_XUV ∝ v_esc⁴` (checked); their line is drawn by eye on
their Fig. 1 and 2, with no normalisation in the text. Here the line is `I_line = (v_esc / 6.5 km/s)⁴`, in
units of Earth's XUV fluence (`xuv_fluence`), and the share of air kept
falls smoothly across it:

```text
kept = 1 / (1 + (s I / I_line)^6)
```

with `s = 2` for a body without a dynamo, whose upper air the stellar
wind strips (`UNMAGNETISED_ESCAPE`, a calibration with no source, tied to
issue #344). The normalisation 6.5 km/s, the steepness 6 and `s` are
calibrations on the Solar System bodies: the line passes close to Mars,
as in the paper, Titan keeps a bar, and `s` is small enough that Venus,
which has no dynamo, keeps its air. Every gas is
kept in the same share. `escape_velocity_km_s` gives `sqrt(2 G M / R)`.

**Methane photolysis** (`methane_photolysis`, before the cold trap, on
the whole stock). The star's ultraviolet destroys methane at a rate set
by its light, not by the amount of methane: any column worth counting
absorbs all the Lyman α (photon-limited). The rate, in molecules per m²
and second over the sphere, is

```text
D = Y · S₀ I r(t) / 4 / E_γ
```

with `S₀ = 1361 W/m²`, `I` the insolation, `E_γ` the energy of a Lyman α
photon (121.6 nm, 1.634e-18 J) and `r(t)` the Lyman α share of the
star's light: Ribas et al. 2005 (doi:10.1086/427977, their eq. 2,
checked) fit `F = 19.2 τ^-0.72` erg s⁻¹ cm⁻² at 1 AU, 7.40e-5 of the
solar constant at the end of saturation (0.1 Gyr) and 3.9e11 photons
cm⁻² s⁻¹ today. The line saturates with the XUV (`saturation_gyr`, the
same anchors) and decays as `t^-0.72` after. `Y = 13.9` methane
molecules per Lyman α photon is a calibration on Titan: Yung, Allen &
Pinto 1984 (ApJS 55, 465, doi:10.1086/190963; abstract checked) find
that Titan "might have lost 5.6 × 10⁴ g cm⁻²" of methane, "the
equivalent of 8 bars", over geologic time. Read as 4.5 Gyr at today's
rate (an assumption, the full text was not read), that is 1.48e10
molecules cm⁻² s⁻¹ against the 1.07e9 Lyman α photons cm⁻² s⁻¹ Titan
gets on average. `Y` is over one because Lyman α stands for all the
ultraviolet that destroys methane, the photolysis proper below 145 nm
and the dissociation photosensitised by acetylene at longer
wavelengths. At that rate Titan's air would lose its methane in a few
tens of millions of years, as Tobie, Lunine & Sotin 2006 (Nature 440,
61, doi:10.1038/nature04497; abstract checked) state.

The supply is the methane outgassed (after escape), spread evenly over
the system's age: `S = CH₄ / age`. While the destruction outruns it,
nothing builds up; from the age `t*` at which the falling destruction
meets the supply, methane gathers in the air and on the ground:

```text
CH₄ left = ∫ from t* to age of (S − D(t)) dt
```

analytic, through the band integral of `xuv.rs`. This is Titan's
resupply: Tobie et al. explain its methane by outgassing from its
interior, in episodes where the model spreads it evenly. Titan outgasses
20.6 bar of methane in the model and keeps 6.3 bar of it on its ground
(the supply overtakes the destruction at about 1.2 Gyr), from which the
cold trap leaves 0.035 bar in its air. Ganymede and Callisto get 3.4
times Titan's light per unit area and outgas 0.15 and 0.47 bar: the
destruction outruns them at every age and they keep none. Destroyed
methane is gone: its hydrogen escapes and its carbon ends in heavier
hydrocarbons on the ground, which the model does not carry. Two
departures: the outgassing is spread evenly over the age, and the
larger Lyman α share of an M dwarf's light is not modelled (both
understate the destruction round red dwarfs, if anything).

**Cold trap** (`cold_trap`). A gas whose triple point lies above the
body's equilibrium temperature frosts onto the ground until its partial
pressure falls to its sublimation pressure, by Clausius–Clapeyron from
the triple point (CO₂ 216.6 K, 5.185 bar, 26.1 kJ/mol; NH₃ 195.4 K,
0.0606 bar, 29 kJ/mol; CH₄ 90.7 K, 0.117 bar, 9.1 kJ/mol; N₂ 63.15 K,
0.125 bar, 6.3 kJ/mol; values from the NIST WebBook from memory, not
checked). This freezes Titan's CO₂ and NH₃ and most of its CH₄. Above
the triple point the gas stays; whether it rains out depends on the
surface temperature, which the climate sets. Water is left to the
climate.

**Photochemistry** (`photochemistry`, after the cold trap, on what is
still gas). The star's ultraviolet photolyses NH₃ into N₂ and hydrogen,
which escapes: `2 NH₃ → N₂ + 3 H₂`, the nitrogen conserved (Atreya,
Donahue & Kuhn 1978, doi:10.1126/science.201.4356.611, on Titan's N₂ from
NH₃; reference from memory, not checked). CH₄ survives only where the
equilibrium temperature is under 150 K: there water is frozen out so deep
that none reaches the upper air; above it, OH from water's photolysis
oxidises the methane, which is set to zero (the CO₂ it would make is
not added). The 150 K is a calibration with no source, between Titan
(82 K, keeps its methane) and an abiotic Earth (255 K) or Mars (207 K).
Life's methane is added later. Methane's photolysis by the ultraviolet
comes before the cold trap (above).

Calibration targets, through the whole chain from the reference bodies'
inputs (volatiles of the rule, no scatter) round the Sun at 4.6 Gyr:

| Body | Result | Target | Observed |
|---|---|---|---|
| Venus | 87 bar, 98 % CO₂, 1.7 bar N₂ | 60 to 120 bar, CO₂ over 90 %, N₂ 1 to 6 bar | 92 bar, 96.5 % CO₂, about 3 bar N₂ |
| Earth | 0.89 bar N₂, no CH₄ or NH₃ (45 bar CO₂ before the thermostat) | N₂ 0.5 to 1.2 bar; CH₄ and NH₃ under 1e-3 bar | 0.78 bar N₂ |
| Mars | 0.014 bar | 0.002 to 0.05 bar | 0.006 bar |
| Titan | 2.1 bar, 98 % N₂, 0.035 bar CH₄ | 0.5 to 5 bar; CH₄ over 0.01 bar | 1.5 bar, about 5 % CH₄ |
| Ganymede, Callisto | no CH₄ (0.15 and 0.47 bar before photolysis) | CH₄ under 1e-3 bar | airless |
| Mercury, Moon | under 1e-8 bar | under 1e-4 bar | exospheres |

A shortcoming, outside the targets: Ganymede keeps 0.016 bar of N₂ and
Callisto, to which the core model gives a weak deep dynamo, 0.047 bar;
both are airless. Before methane photolysis they also kept 0.15 and
0.47 bar of CH₄. Moons are not modelled yet (`body_model` skips them).

## Climate and surface liquid

`planet::climate` (not yet chained into `planet_model`) gives the mean
surface temperature, the liquid on the surface (`SurfaceLiquid`: `none`,
`water`, `hydrocarbons` or `lava`) and whether the carbonate thermostat
holds the temperature (`Climate::regulated`). It takes the gases after
`photochemistry` and returns them with the CO₂ the thermostat leaves and
the water vapour the climate sets.

**Grey greenhouse** (`surface_temperature_k`). A grey atmosphere in
radiative equilibrium, in the Eddington approximation (Pierrehumbert
2010, *Principles of Planetary Climate*, §4.6), warmed from below by the
internal heat flux `q`:

```text
T⁴ = T_eq⁴ (1 + ¾ τ) + q / σ
τ  = Σ k_i (p_i / 1 bar)^a_i (P_b,i / 1 bar)^½
```

`T_eq` is the context's equilibrium temperature (Bond albedo 0.3), `p_i`
the partial pressures of CO₂, H₂O and CH₄ and `P_b,i` the pressure that
broadens each gas's lines (below). The square roots are the strong-line
limit of a pressure-broadened band, which absorbs as `sqrt(S u α)` with
the path `u ∝ p_i` and the width `α` growing with the pressure round
the lines (Pierrehumbert 2010, §4.4; Goody & Yung 1989). CO₂'s
power is smaller, 0.25: its 15 µm band is saturated at the centre, so its
forcing grows with the logarithm of its amount (Myhre et al. 1998,
doi:10.1029/98GL01908), and the small power stands in for it across the
six decades between Earth and Venus. The opacities are calibrations with
no source:

| Gas | `k_i` | `a_i` | Calibrated on |
|---|---|---|---|
| CO₂ | 1.66 | 0.25 | Venus's 737 K under 85 bar |
| H₂O | 5.9 | 0.5 | Earth: the thermostat leaves about 280 ppm of CO₂ |
| CH₄ | 3.4 | 0.5 | Titan's 94 K |

Titan's real greenhouse is mostly collision-induced absorption by N₂,
CH₄ and H₂, partly undone by its haze (McKay, Pollack & Courtin 1991,
doi:10.1126/science.11538492, not checked); the methane opacity carries
all of it here.

**Broadening by the rest of the air** (`broadening_bar`, issue #440).
For CO₂ and water vapour, `P_b` is the gas's own pressure in full plus
the other gases' pressure `P_o`, in proportion up to 1 bar
(`BROADENING_KNEE_BAR`) and as its 0.2 power past it
(`THICK_AIR_EXPONENT`):

```text
P_b = p_i + P_o                    for P_o ≤ 1 bar
P_b = p_i + 1 bar · (P_o / 1 bar)^0.2   past it
```

Below 1 bar of other gases this is the whole pressure, so Earth (0.90
bar of other gases in the climate step), Mars and the pure CO₂ of Venus
keep the calibration of the table, as does a steam atmosphere, whose
water broadens its own lines. Past 1 bar, the other gases' share of an
opacity grows as `P_o^0.1` instead of `P_o^0.5`. With the plain square
root, nitrogen alone ran an Earth away: at 1 AU, with 2 bar of N₂, the
thermostat ran out of CO₂ to remove and the vapour feedback carried the
surface to 1490 K (issue #440). The 0.2 is calibrated on the warming
of doubled nitrogen. Goldblatt et al. 2009 (doi:10.1038/ngeo692,
abstract checked) find 4.4 °C for twice the present nitrogen on the late
Archean Earth (1-D radiative–convective model), Charnay et al. 2013
(doi:10.1002/jgrd.50808, §5.2 read in arXiv 1310.4286) +7 °C in a GCM
for the air going from 1 to 2 bar at 2.5 Ga with 10 mbar of CO₂ and
2 mbar of CH₄, and Wolf & Toon 2014 (doi:10.1089/ast.2013.1112,
abstract checked) 3 to 7 K for plausible nitrogen inventories. In
Charnay's setting without methane (the Sun at 0.82 of its light, 10
mbar of CO₂, no thermostat) the model goes from 280.6 K to 285.6 K
(+5.0 K) at 2 bar and to 289.4 K at 3 bar; the powers 0.1 and 0.3 give
about +3 K and +8.5 K. The real rise is milder than the strong-line law
because lines overlap and saturate (Vladilo et al. 2013,
doi:10.1088/0004-637X/767/1/65, appendix A.3, checked).

Methane keeps the whole pressure, `P_b = P`: its opacity stands for
Titan's collision-induced greenhouse, which grows with the nitrogen
faster than line broadening, not slower. Titan's calibration is
therefore unchanged, and so is Earth's (`H2O_OPACITY`), whose other
gases stay under 1 bar.

Earth at 1 AU now keeps 288 K under its thermostat with 2, 3, 5 and
10 bar of N₂ (the CO₂ it leaves falls from 270 ppm of a bar to 23, 7,
0.7 and a trace); past about 11 bar its vapour still runs away once the
thermostat has no CO₂ left to remove. That remainder is the grey
model's own moist limit: at Earth's composition the outgoing radiation
peaks at 253 W/m² (Goldblatt et al. 2013, doi:10.1038/ngeo1892, find
282 W/m² line by line), barely above the 240 W/m² Earth absorbs, and
the peak still falls with nitrogen (239 W/m² at twice Earth's N₂, 211
with the plain square root), whereas the limit of a real moist
atmosphere is set by the water vapour alone (Nakajima, Hayashi & Abe
1992, doi:10.1175/1520-0469(1992)049<2256:ASOTGE>2.0.CO;2, abstract
checked). Issue #438 bounds the steam side of that limit.

**Water vapour.** Where water is kept and there is no runaway, the air
above the ocean or the ice holds 77 % of the saturation pressure (the
fixed relative humidity of Manabe & Wetherald 1967,
doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2, not checked). The
saturation is Clausius–Clapeyron from water's triple point (273.16 K,
611.657 Pa) with a mean latent heat of 2.4e6 J/kg, capped at the
critical point (647 K), and never more than the body's whole water
would weigh as vapour (`m_water g / (4π R²)`): a trace of water cannot
put a hundred bar of steam in the air. The temperature and the vapour
are solved together by iterating from the dry temperature, which climbs
to the coldest balance. A runaway's steam is already in the gases and is
kept as it is.

**Carbonate thermostat.** On a planet with plates and water, weathering
draws CO₂ down faster when it is warm and volcanoes give it back (Walker,
Hays & Kasting 1981, doi:10.1029/JC086iC10p09776). The model sets the CO₂
by bisection between none and all that is outgassed (never more) so that
the surface is at `THERMOSTAT_K`, 288 K (a calibration on Earth), the
vapour taken at that temperature. If all the CO₂ is not enough, it all
stays in the air and the planet is colder: below 273 K its water freezes
(weathering stops, so the CO₂ accumulates). If no CO₂ at all is still too
warm, the planet is hotter. Only a temperature reached is `regulated`.
The CO₂ the thermostat can use is what the cold trap left: past about
1.6 AU from the Sun, CO₂ frosts out at the equilibrium temperature and
an Earth falls short, close to the maximum-greenhouse limit of the
habitable zone (Kopparapu et al. 2013, about 1.67 AU). The grid's
`carbon_cycle` is unchanged; part 3 aligns them.

**Surface liquid**, from the phase diagrams at the surface temperature
and pressure, in this order:

- `lava` from 1400 K (`LAVA_K`), the basalt solidus taken as the
  threshold of a magma surface (Kite, Fegley, Schaefer & Gaidos 2016,
  doi:10.3847/0004-637X/828/2/80; value not checked against the paper),
  on a body that has kept no water: under water, steam or supercritical
  fluid covers the rock (Hamano, Abe & Genda 2013, doi:10.1038/nature12163,
  on the steam over a magma ocean; not checked), so a body with water on
  top is never a lava world;
- `water` between 273.15 K and water's boiling point at the surface
  pressure (the same Clausius–Clapeyron, 373 K at 1 atm, capped at the
  critical point), above the triple-point pressure, with water kept out
  of a runaway;
- `hydrocarbons` between methane's triple point (90.7 K) and ethane's
  boiling point at the surface pressure (Clausius–Clapeyron from 184.6 K
  at 1 atm with 14.7 kJ/mol, capped at its critical 305 K; NIST values
  from memory, not checked), with methane in the air;
- `none` otherwise.

Calibration targets, through the whole chain from the reference bodies'
inputs round the Sun at 4.6 Gyr:

| Body | Result | Target | Observed |
|---|---|---|---|
| Earth | 288 K, water, regulated, 270 ppm CO₂ | 288 ± 6 K, water, regulated | 288 K, 280 ppm before industry |
| Venus | 735 K, none | 737 ± 50 K, none | 737 K |
| Mars | 209 K, none | 210 ± 12 K, none | about 210 K |
| Titan | 94 K, hydrocarbons | 94 ± 8 K, hydrocarbons | 94 K, methane and ethane lakes |
| Mercury | 410 K, its equilibrium temperature | within 1 K of it | airless |

An Earth moved round the Sun keeps 288 K out to 1.4 AU (0.17 bar of CO₂
at 1.2 AU, 0.9 bar at 1.4 AU), is short at 1.6 AU (281 K, all its 1.8
bar of CO₂ in the air, its water still liquid; 287.8 K before the
broadening by thick air was tempered, issue #440) and freezes beyond
(234 K at 1.8 AU). At 0.9 AU it has run away (702 K, no water); at
0.01 AU it is a lava world.

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

`planet_model` is the single place that chains the derivation, in this
order: the interior from the drawn inputs, the XUV fluence, the envelope
escape (the interior is derived again from what is left), the internal
heat, then, if the envelope left is under `MIN_ENVELOPE` (0.1 % of the
mass), the water kept and the runaway check, the tectonic regime, the
core and dynamo, the air (`outgassed`, `atmosphere`,
`methane_photolysis`, `cold_trap`, then `photochemistry`, which reads
the cold trap's air), the climate and
surface liquid, and life with the oxygen it makes. If that climate
carries a wet surface past water's critical point (`climate::boils`),
the body has run away after all: the steps from the regime on run once
more from its steam (`regime::steam`, the water left after escape, in
the air), flagged `boiled`, so that a body never keeps a liquid ocean
above 647 K. That second pass is kept only if its steam holds the
surface at or past 647 K too (`climate::past_critical`): a boil whose
steam escapes until the dry re-run is cool has no self-consistent
runaway, and the first pass stands, its vapour bounded by its own
water. A body
that keeps an envelope has no surface: `PlanetModel.surface` is `None`.
Otherwise the `Surface` holds the water, regime, core, final gases,
climate and life, and `classify` reads the class off it. Nothing derives
from the class.

The first rule that holds wins:

| Order | Class | Rule |
|---|---|---|
| 1 | `hot-jupiter`, `gas-giant`, `ice-giant` | giant rules of part 1 |
| 2 | `mini-neptune` | envelope of at least 0.1 % and radius of at least 1.6 R⊕ |
| 3 | `lava-world` | surface liquid is lava (at least 1400 K, no water kept) |
| 4 | `runaway-greenhouse` | in runaway (the insolation is over the runaway limit, or the climate boiled the ocean) and the air is over 1 bar |
| 5 | `hydrocarbon-world` | surface liquid is hydrocarbons and the air is over 1 bar |
| 6 | `ocean-world` | surface liquid is water and at least 10 Earth oceans are kept |
| 7 | `ice-world` | at least 5 % water and ices |
| 8 | `super-earth`, `rocky` | radius of at least 1.25 R⊕, rocky below |

Two constants are calibrations. `OCEAN_WORLD_OCEANS` is 10 Earth oceans:
Cowan & Abbot 2014 (doi:10.1088/0004-637X/781/1/27) study how much water
drowns all land; ten is a round figure above the few oceans their basins
hold, not a value from the paper. `DENSE_AIR_BAR` is 1 bar, for the
runaway greenhouse (Venus has 90 bar; a runaway that lost its air is a
bare rock) and for the hydrocarbon world. The pressure condition on
hydrocarbons is a departure from the plain rule "liquid hydrocarbons".
It was added when the model kept 0.2 and 0.5 bar of methane on
Ganymede and Callisto, which methane photolysis now removes; it still
keeps a cold body with a trace of methane over its ice under a thin air
an ice world. Titan has 2.1 bar in the model.

The Solar System classifies as itself: Venus `runaway-greenhouse`, Earth
and Mars `rocky`, Titan `hydrocarbon-world`, Ganymede and Europa
`ice-world`, Callisto `ice-world` with no methane, the giants as in
part 1. A 1 M⊕ planet of 30 % water at
1 AU, with the volatiles `draw_inputs` gives its rock and ice (no
scatter), is an `ocean-world` at 288 K under 0.4 bar. With 1 % water
(about 40 oceans) at 1 AU, the 20 bar of nitrogen its ice brings carry
the vapour feedback past 647 K (past about 11 bar of N₂, see the
broadening above): it is a `runaway-greenhouse`, never a lava world. The model is
finite and non-negative on mass 0.01 and 20 M⊕, insolation 1e-3 and 1e4,
age 0.5 and 13 Gyr, M5V and A0V stars.

## Census

Measured, not tuned: `cargo run --release -p mapgen --example planet_census
-- 500` draws systems of seeds 1 to 500 and derives every planet through
`body_model`. The medians of temperature and pressure are over the
planets with a surface (`NaN` for the classes that have none). Run after
the fixes of the steam review and the source check: the vapour capped at
the water a body has, a boil kept only if its steam stays past 647 K, a
runaway's water all in its air, the XUV saturation time anchored on
Jackson et al. and Johnstone et al., the heat-pipe threshold at 1 W/m²
and the seafloor power 4.1, then again after methane photolysis (issue
#439) and after the broadening by thick air was tempered (issue #440). The census also counts the runaways by their source
(`Water::boiled`) and the wet planets left past 647 K because their boil
was not kept.

```text
500 systems, 1677 planets

body_type against derived class (planets)
body_type              gas-giant         hot-jupiter   hydrocarbon-world           ice-giant           ice-world          lava-world        mini-neptune         ocean-world               rocky  runaway-greenhouse         super-earth   total
barren                         0                   0                   0                   0                  38                  14                   0                   0                 335                  22                   1     410
caustic                        0                   0                   0                   0                   8                   0                   1                   0                  56                  30                   0      95
frozen                         0                   0                  16                   0                 699                   0                  57                   0                   2                   0                   0     774
gas-giant                     41                   0                   0                   0                   0                   0                   0                   0                   0                   0                   0      41
hot-jupiter                    0                   1                   0                   0                   0                   0                   0                   0                   0                   0                   0       1
ice-giant                     28                   0                   0                  56                   0                   0                   0                   0                   0                   0                   0      84
terran                         0                   0                   0                   0                  15                   0                   9                   8                 124                   5                  13     174
volcanic                       0                   0                   0                   0                   6                   3                   0                   0                  79                  10                   0      98

planets per radius bin (Fulton et al. 2017 bins, R_earth)
    <1    55.9 %  (937)
 1-1.5    30.6 %  (514)
 1.5-2     3.8 %  (63)
   2-3     2.3 %  (38)
   3-4     1.9 %  (32)
   4-6     2.4 %  (41)
  6-10     1.9 %  (32)
 10-22     1.2 %  (20)
   >22     0.0 %  (0)

medians per class
class               planets  age Gyr  mass M⊕    a AU    CMF    water  envelope     T K      P bar
gas-giant                69     4.90   47.361   2.172  0.332   0.0283    0.9000     NaN        NaN
hot-jupiter               1     6.33  191.055   0.024  0.382   0.0000    0.9447     NaN        NaN
hydrocarbon-world        16     4.49    0.107   3.125  0.317   0.3555    0.0000     107      4.240
ice-giant                56     5.35   17.247   2.028  0.313   0.2924    0.1614     NaN        NaN
ice-world               766     5.57    0.418   0.404  0.310   0.3451    0.0000      90      0.046
lava-world               17     3.18    0.428   0.026  0.303   0.0006    0.0000    1593      0.000
mini-neptune             67     5.82    2.129   0.352  0.312   0.3407    0.0082     NaN        NaN
ocean-world               8     5.31    1.356   0.107  0.302   0.1530    0.0000     288      4.826
rocky                   596     5.31    0.474   0.047  0.312   0.0005    0.0000     336      0.000
runaway-greenhouse       67     4.39    1.053   0.141  0.310   0.0008    0.0000     719     55.258
super-earth              14     5.07    1.679   0.151  0.269   0.0008    0.0018     233      2.186

class mix (1677 planets)
ice-world             45.7 %  (766)
rocky                 35.5 %  (596)
gas-giant              4.1 %  (69)
mini-neptune           4.0 %  (67)
runaway-greenhouse     4.0 %  (67)
ice-giant              3.3 %  (56)
lava-world             1.0 %  (17)
hydrocarbon-world      1.0 %  (16)
super-earth            0.8 %  (14)
ocean-world            0.5 %  (8)
hot-jupiter            0.1 %  (1)

planets with a surface: 1467

surface liquid (1467 planets with a surface)
Hydrocarbons           1.1 %  (16)
Lava                   1.2 %  (17)
None                  92.8 %  (1361)
Water                  5.0 %  (73)

tectonic regime (1467 planets with a surface)
Episodic              20.5 %  (301)
HeatPipe               3.5 %  (51)
Plates                40.4 %  (593)
Stagnant              35.6 %  (522)

runaways: 531 by the insolation limit, 0 boiled; of the runaway-greenhouse class: 67 by the limit, 0 boiled
wet, not in runaway and past 647 K (a boil not kept): 1

life: 1.7 % of planets with a surface (25), 1.4 % oxygenated (21)

radius histogram, period under 100 days (867 planets)
 0.00-0.25      0.0 %  (0)
 0.25-0.50     10.4 %  (90)
 0.50-0.75     20.2 %  (175)
 0.75-1.00     36.8 %  (319)
 1.00-1.25     22.6 %  (196)
 1.25-1.50      5.7 %  (49)
 1.50-1.75      1.4 %  (12)
 1.75-2.00      0.7 %  (6)
 2.00-2.25      0.6 %  (5)
 2.25-2.50      0.5 %  (4)
 2.50-2.75      0.1 %  (1)
 2.75-3.00      0.1 %  (1)
 3.00-3.25      0.1 %  (1)
 3.25-3.50      0.0 %  (0)
 3.50-3.75      0.1 %  (1)
 3.75-4.00      0.0 %  (0)
 4.00-4.25      0.0 %  (0)
 4.25-4.50      0.0 %  (0)
 4.50-4.75      0.0 %  (0)
 4.75-5.00 +    0.8 %  (7)
```

What it shows, plainly:

- **Radius valley.** No dip appears between 1.5 and 2 R⊕ (Fulton et al.
  2017, doi:10.3847/1538-3881/aa80eb). Among the 867 planets with a
  period under 100 days the histogram falls steadily from 1.0 R⊕ upward:
  22.6 % in 1.00-1.25, 5.7 % in 1.25-1.50, 1.4 % and 0.7 % in the two
  bins of the valley, then a thin tail. There is no peak at 1.25-1.5 and
  no second peak at 2-3 R⊕ for a valley to separate. The generator draws
  few planets above 1.5 R⊕ at all (13 % of all planets), most of them
  beyond 100 days, so photoevaporation has no population of sub-Neptunes
  to carve a gap in. The model was not tuned to produce one. With the
  XUV fluence of Ribas et al. and the anchored saturation time, 18
  close-in planets lie in 1.50-2.00 R⊕ (13 before Ribas et al., 17
  before the saturation time was anchored).
- **Mini-Neptunes** are 4.0 % of the planets (67), against the
  over-representation of part 1 (see [planet-interiors.md](planet-interiors.md)).
  The `frozen` type holds 57 of them; escape strips the ones close in.
- **Lava worlds** are 1.0 % (17): dry rocks (median water 6e-4 of the
  mass) at a median 0.026 AU, airless, at 1593 K. Before the fixes, 46
  lava worlds had a median water share of 0.32: migrated water worlds
  whose outgassed ices had run their climate past 1400 K under a water
  layer. A body with water on top is never a lava world now.
- **Hydrocarbon worlds** are 1.0 % (16), all `frozen`: small icy bodies
  (median 0.11 M⊕, water 0.36) far from their stars (median 3.1 AU) under
  a median 4.2 bar. Before methane photolysis there were 85 (5.1 %), at
  a median 0.64 AU round dim stars: the ultraviolet of a red dwarf, close
  in and saturated for gigayears, now destroys their methane faster than
  they outgas it, and they are ice worlds. Ten of the sixteen are
  Titans (82 to 88 K at equilibrium, a few hundredths of a bar of
  methane, 94 to 113 K). The other six keep 0.5 to 18 bar of
  methane in their air (their equilibrium
  temperature, 91 to 107 K, is above methane's triple point, so the cold
  trap holds none back), which warms four of them to 225 to 255 K
  (within 1 K of their temperatures before issue #440: methane keeps the
  broadening by the whole air, and their CO₂ and vapour are traces).
- **Ocean worlds** are 0.5 % (8), down from 33: 26 of those were small
  icy bodies (median 0.2 M⊕) at 90 to 150 K at equilibrium, warmed to
  280 to 440 K by their methane (up to 70 bar of it). Photolysis leaves 20 of them
  ice worlds and 6 hydrocarbon worlds. Tempering the broadening by thick
  air (issue #440) made two ocean worlds, a former boiled runaway with
  17 bar of N₂ and a rock frozen at 228 K under 20 bar of N₂, both now
  held at 288 K by their thermostat, and froze one (275 K to 273 K under
  1.65 bar of CO₂).
- **Runaway greenhouses** are 4.0 % (67), all dry rocks over the
  runaway limit of the insolation (median water 8e-4, 55 bar, 719 K).
  Of the 531 runaways among the planets with a surface, all come from
  the limit; most of them have kept under 1 bar of air and are rocks.
  Before methane photolysis 15 had boiled their ocean, 13 of them icy
  bodies whose methane greenhouse carried them there; they are ice
  worlds now. The last two boiled with 2.9 and 17 bar of nitrogen in
  their first pass, whose broadening carried their vapour away; with it
  tempered (issue #440) their thermostats hold them at 288 K, a `rocky`
  planet and an `ocean-world`. The dense CO₂ runaways with several bar
  of N₂ cooled by up to 37 K. The steam temperature is not bounded yet (issue #438). One
  wet planet stays past 647 K out of a runaway (two before #440): its
  boil was not kept, because its steam escaped until the dry re-run was
  cool. The other, under 8.5 bar of N₂, is now a wet `rocky` planet at
  290 K, with life.
- **Tectonic regimes**: heat pipes rose from 2.7 % (39) to 3.5 % (51)
  with the threshold lowered from 2 to 1 W/m². The two planets that
  stopped boiling (#440) moved from episodic to plates.
- **Surface liquid** over the 1467 planets with a surface: no liquid for
  92.8 %, hydrocarbons 1.1 % (14.6 % before methane photolysis), water
  5.0 % (6.7 %), lava 1.2 %. Issue #440 moved eight planets across the
  water line, four each way, and left the count at 73: four gained an
  ocean (the two former boils, the boil not kept and the frozen rock
  under 20 bar of N₂), four cold ones under 1.8 to 3.1 bar of CO₂ and
  N₂ froze (from 273 to 277 K down to 263 to 273 K).
- **Life** arises on 1.7 % of the planets with a surface (25), 1.4 %
  with oxygen (21); 1.8 % and 1.6 % before issue #440, which froze two
  inhabited planets and brought one back from 1073 K; 2.4 % and 2.0 % before methane
  photolysis (nine of the icy ocean worlds warmed by methane had life).
