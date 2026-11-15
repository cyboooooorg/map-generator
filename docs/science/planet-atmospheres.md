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
the planet keeps its water. `oceans` counts it in units of 1.4e21 kg.
The climate can find a runaway this limit misses: when a wet surface
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

`planet::core::core` (called by `planet_model`) gives the
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

`planet::atmosphere` (called by `planet_model`) gives the
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
| Titan | hydrocarbon world, 1.5 bar, 98 % N₂, 0.038 bar CH₄ | 1.5 bar, about 5 % CH₄ |
| Ganymede, Callisto | 4e-5 and 9e-6 bar | airless |
| Io | heat pipe, 1.3e-4 bar | heat pipe, SO₂ of about 1e-9 bar |
| Europa | 8e-6 bar | airless |
| Triton | 1.9e-3 bar of N₂ (frost at 47 K) | about 1.4e-5 bar of N₂ |
| Moon | 1e-12 bar | airless |

Over five seeds Titan ranges from 1.1 to 8.7 bar (its volatiles scatter
by 0.3 dex), Io's air up to 8e-4 bar. Triton's N₂ is its frost's vapour
pressure at the equilibrium temperature of 47 K, where Triton's surface,
brighter, is at 38 K: a hundred times the observed pressure.

## Climate and surface liquid

`planet::climate` (called by `planet_model`) gives the mean
surface temperature, the liquid on the surface (`SurfaceLiquid`: `none`,
`water`, `hydrocarbons` or `lava`) and whether the carbonate thermostat
holds the temperature (`Climate::regulated`). It takes the gases after
`photochemistry` and returns them with the CO₂ the thermostat leaves and
the water vapour the climate sets.

**Radiative–convective grey column** (`surface_temperature_k`,
`planet::column`, issue #441). The surface temperature is the one at
which the outgoing longwave radiation (OLR) of a grey column over the
surface balances what it absorbs, `σ T_eq⁴ + q`, with `T_eq` the
context's equilibrium temperature (Bond albedo 0.3) and `q` the internal
heat flux. The column:

- runs from the surface up a log-pressure grid (20 levels per decade,
  to a thousandth of the surface pressure and at least to 1e-4 bar);
- follows the dry adiabat of its mixture until its water vapour
  saturates, then the moist pseudo-adiabat (Pierrehumbert 2010,
  *Principles of Planetary Climate*, §2.7, eq. 2.33); past water's
  critical point the vapour is a gas like the others. The heat
  capacities are NIST's at 298 K, the steam's from the NIST Shomate
  equation (it grows from 34 to 52 J/(mol·K) between 300 and 2000 K);
- stops cooling at the skin temperature `T_eq / 2^¼`, a stratosphere
  where the vapour keeps the mixing ratio of its cold trap;
- integrates the grey optical depth from the top down with the per-gas
  laws below, written per unit `ln p` so that a uniform column gives back
  their totals (`τ_i ∝ p^(a_i + ½)`, so `dτ_i / d ln p = (a_i + ½) τ_i`);
- sends out the grey two-stream integral of `σ T⁴` over it with the
  diffusivity factor 3/2 (the transmission falls to `1/e` at `τ` = 2/3,
  the Eddington emission level), plus the surface's `σ T_s⁴ e^(−3τ_s/2)`.

```text
τ_i = k_i (p_i / 1 bar)^a_i (P_b,i / 1 bar)^½        (a uniform column)
```

**Albedo of a water runaway** (`climate::absorbing`, issue #492). A body
in a water runaway (by the insolation limit or a boil) that keeps steam
in its air absorbs under
the Bond albedo that puts the column's moist plateau (`F_plateau`, about
287 W/m², the column's OLR over a saturated surface at 380 K) exactly at
its star's runaway limit `S_rg` (`star::habitable_fluxes`), clamped to
0 to 0.9: `A_rg = 1 − 4 F_plateau / (S_rg S₀)`, with `S₀` the solar
constant behind `T_eq` (255 K at 1 S⊕ with albedo 0.3). Kopparapu et al.
2013 (doi:10.1088/0004-637X/765/2/131) and 2014 place the runaway limit
where the light a moist atmosphere absorbs, with the albedo its own
spectrum gives under that star, meets its outgoing limit. With Earth's
0.3, a planet round a red star that runs away by that limit
(0.91 S⊕ for an M9V) would absorb under the plateau and sit on water's
critical point. With `A_rg` (0.08 for an M9V, 0.24 for the Sun) it
balances past the plateau. Wet bodies keep 0.3, so Earth's calibration
does not move. So does a runaway whose escape took all its water
(Mercury, Venus), which has no moist air left. Albedo in general is issue #510.

`p_i` are the partial pressures of CO₂, H₂O and CH₄ and `P_b,i` the
pressure that broadens each gas's lines (below). The square roots are the
strong-line limit of a pressure-broadened band, which absorbs as
`sqrt(S u α)` with the path `u ∝ p_i` and the width `α` growing with the
pressure round the lines (Pierrehumbert 2010, §4.4; Goody & Yung 1989).
CO₂'s power is smaller, 0.07: its 15 µm band is saturated at the centre,
so its forcing grows with the logarithm of its amount (Myhre et al. 1998,
doi:10.1029/98GL01908). With Venus pinning its opacity, the power sets
how strong small amounts of CO₂ are: 0.27 left the late Archean Earth
(below) at 259 K. Under 1e-6 bar (`CO2_WEAK_BAR`) CO₂ absorbs in
proportion to its amount, the weak-line limit; without it Titan's 3e-10
bar of CO₂ warmed it by about 3 K. The opacities are calibrations with no
source:

| Gas | `k_i` | `a_i` | Calibrated on |
|---|---|---|---|
| CO₂ | 0.74 (1.66 before #441) | 0.07 (0.25) | Venus's 737 K under 85 bar |
| H₂O | 111 (5.9) | 0.5 | the moist plateau at about 287 W/m² |
| CH₄ | 3.3 (3.4) | 0.5 | Titan's 94 K |

Titan's real greenhouse is mostly collision-induced absorption by N₂,
CH₄ and H₂, partly undone by its haze (McKay, Pollack & Courtin 1991,
doi:10.1126/science.11538492, not checked); the methane opacity carries
all of it here.

**The moist plateau.** On a saturated column the levels that radiate to
space sit on water's saturation curve whatever the surface temperature,
so the OLR flattens: the Simpson–Nakajima limit (Nakajima, Hayashi & Abe
1992, doi:10.1175/1520-0469(1992)049<2256:ASOTGE>2.0.CO;2). Goldblatt,
Robinson, Zahnle & Crisp 2013 (doi:10.1038/ngeo1892, abstract checked)
find it at 282 W/m² line by line, Boukrouche, Lichtenberg & Pierrehumbert
2021 (doi:10.3847/1538-4357/ac1345, §III.1 read in arXiv 2107.14150) at
283 W/m² and Hamano et al. 2015 (doi:10.1088/0004-637X/806/2/216) at
280 W/m². With Earth's air saturated at 77 %, the column sends out 279,
287, 288 and 288 W/m² from a surface at 340, 360, 380 and 400 K, and
255 W/m² at 320 K, where it still approaches the limit (the moist OLR
reaches it only asymptotically, Goldblatt et al. 2013); at 380 K, 287.6,
287.7 and 287.5 W/m² with one, two and three times Earth's nitrogen. The
plateau sits a few W/m² above the line-by-line limit so that Earth
without CO₂ and the late Archean (below) stay within their targets. The old Eddington core
(`T⁴ = T_eq⁴ (1 + ¾ τ) + q/σ`) had no plateau: its moist OLR peaked at
253 W/m² at Earth's composition and fell with nitrogen.

**Broadening** (`broadening_bar`, `water_broadening_bar`,
`foreign_share`, issues #440 and #441). For CO₂, `P_b` is the pressure of
CO₂ and water vapour in full plus the pressure `P_o` of the other gases
(N₂, O₂, CH₄, the rest), in proportion up to 2.6 bar of them
(`BROADENING_KNEE_BAR`) and no further past it (`THICK_AIR_EXPONENT` 0):
the column applies to every level the share `K / P_o` the whole column
has past the knee. For water vapour, `P_b` is the vapour's own pressure
plus 0.05 (`WATER_FOREIGN_BROADENING`) times that of the rest: water
broadens its own lines about five times better than air does (HITRAN's
self- against air-broadened half-widths, Gordon et al. 2017,
doi:10.1016/j.jqsrt.2017.06.038, from memory), and its self-continuum
dominates the windows of a moist air (Shine, Ptashnik & Rädel 2012,
doi:10.1007/s10712-011-9170-y, from memory).

Both numbers come from the plateau. More nitrogen lifts it through the
adiabat (a column of more nitrogen holds less vapour per kelvin of its
upper levels) and lowers it through broadening: without foreign
broadening of water the plateau rises by several per cent from one to
three times Earth's nitrogen; with the old knee
at 1 bar the tempered broadening of a 3x column lifted it by 4 %. The
knee sits above three times Earth's air (2.55 bar of N₂ and O₂).

At each level the foreign pressure that broadens CO₂'s and water's lines
also saturates past 0.18 bar (`LOCAL_KNEE_BAR`), growing as
`K (p/K)^0.05` beyond it (`LOCAL_KNEE_EXPONENT`): lines overlap and their
absorption saturates as they widen (Vladilo et al. 2013,
doi:10.1088/0004-637X/767/1/65, App. A.3). At 288 K the column radiates
from levels with a few tenths of a bar of nitrogen, past this knee, so
thick nitrogen warms a temperate planet little; at the 380 K plateau the
radiating levels hold a few hundredths of a bar, under it, so the plateau
keeps its independence from the nitrogen.

Methane keeps the whole pressure, `P_b = P`: its opacity stands for
Titan's collision-induced greenhouse, which grows with the nitrogen
faster than line broadening.

CO₂ and water vapour count in full for each other's lines: water
broadens CO₂'s much more than N₂ and O₂ do (Tan, Kochanov, Rothman &
Gordon 2019, doi:10.1029/2019JD030929, abstract checked), and CO₂
broadens water's 1.33 to 1.45 times as much as air (Yang et al. 2022,
doi:10.7498/aps.71.20220700, abstract checked); in water's foreign share
it counts as air does.

**Thick nitrogen.** At 1 AU with modern CO₂ (4e-4 bar) and no
thermostat, an Earth keeps a moist balance at 307, 310 and 307 K under 2,
5 and 11 bar of N₂; on the old core 2 bar already ran away. Doubled
nitrogen warms as the 1-D and 3-D models find (Goldblatt et al. 2009,
doi:10.1038/ngeo692: +4.4 °C for doubled nitrogen on the late Archean
Earth; Charnay et al. 2013, doi:10.1002/jgrd.50808: +7 °C in a GCM;
Wolf & Toon 2014, doi:10.1089/ast.2013.1112: 3 to 7 K): in Charnay's
setting (0.82 of today's Sun, 10 mbar of CO₂, no thermostat) the column
goes from 271.1 K to 278.4 K (+7.3 K; thin margins against the test's
270 K floor and +8 K ceiling). The thermostat holds an Earth under 2 or
3 bar of N₂ at 288 K with 8e-6 and 6e-6 bar of CO₂, and an Earth with
no CO₂ under 1 bar of N₂ falls to 263 K, well below 288 K (Lacis et al.
2010, doi:10.1126/science.1190653). The OLR rises slowly with the
surface temperature below the plateau (about 1 W/m² per kelvin), so a
forcing warms the grey column more than a line-by-line one.

**Steam.** A steam runaway's gases are fixed, its surface no colder than
the saturation temperature of its steam. Pure steam follows the plateau
too: 10 bar of it send out 282 W/m² from 700 to 1000 K and leave the
plateau at about 1290 K, where the dry adiabat from the surface reaches
the emitting levels (Boukrouche et al. 2021 find about 1050 K under
10 bar, Goldblatt 2015, doi:10.1089/ast.2014.1268, a rise past 1500 K
through the 4 µm window; the old grey law sent out the limit from about
680 K). A grey column has no window, so a deeper steam column leaves the
plateau ever later (about 2600 K under 300 bar, 20 000 K under the
1.6e5 bar of a 30 %-water Earth), where Boukrouche et al. find the end
of the plateau reaching a constant of about 1800 K as the steam thickens
to 295 bar. The column therefore holds at most 70 bar of steam
(`STEAM_MAX_BAR`, calibrated so that a thick steam column leaves the
plateau at 1800 K); the steam under it changes neither the radiation
nor the broadening of the CO₂ lines. The 30 %-water planet at 0.7 AU
(489 W/m² absorbed, 1.6e5 bar of steam; 8805 K under the plain grey law,
2076 K under the old cap) settles at 2099 K. The cap is a surrogate: it
stands for the non-grey 4 µm window (Boukrouche et al. 2021; Goldblatt
2015) through which a real hot steam atmosphere radiates and which the
grey column cannot see.

**Water vapour.** Where water is kept and there is no runaway, the air
above the ocean or the ice holds 77 % of the saturation pressure (the
fixed relative humidity of Manabe & Wetherald 1967,
doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2, not checked). The
saturation is Clausius–Clapeyron from water's triple point (273.16 K,
611.657 Pa) with a mean latent heat of 2.4e6 J/kg, capped at the
critical point (647 K), and never more than the body's whole water
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
between none and all that is outgassed (never more) so that a surface at
`THERMOSTAT_K`, 288 K (a calibration on Earth), the vapour taken at that
temperature, sends out what it absorbs (solved in `ln CO₂`). If all the
CO₂ is not enough, it all stays in the air and the planet is colder:
below 273 K its water freezes (weathering stops, so the CO₂
accumulates). If no CO₂ at all is still too warm, the planet is hotter.
Only a temperature reached is `regulated`. The CO₂ the thermostat can
use is what the cold trap left. The grid's `carbon_cycle` is unchanged;
part 3 aligns them.

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
- `hydrocarbons` between methane's freezing point, lowered by the
  nitrogen of the air dissolved in it (`methane_freezes_k`: 90.67 K with
  no N₂, about 78 K under Titan's 1.45 bar, see
  [atmospheres.md](atmospheres.md)), and ethane's
  boiling point at the surface pressure (Clausius–Clapeyron from 184.6 K
  at 1 atm with 14.7 kJ/mol, capped at its critical 305 K; NIST values
  from memory, not checked), with methane in the air;
- `none` otherwise.

Calibration targets, through the whole chain from the reference bodies'
inputs round the Sun at 4.6 Gyr:

| Body | Result | Target | Observed |
|---|---|---|---|
| Earth | 288 K, water, regulated, 3e-4 bar of CO₂ | 288 ± 6 K, water, regulated, under 0.01 bar of CO₂ | 288 K, 280 ppm before industry |
| Venus | 737 K, none | 737 ± 50 K, none | 737 K |
| Mars | 209 K, none | 210 ± 12 K, none | about 210 K |
| Titan | 94 K, hydrocarbons | 94 ± 8 K, hydrocarbons | 94 K, methane and ethane lakes |
| Mercury | 410 K, its equilibrium temperature | within 1 K of it | airless |

Earth's thermostat needs 3.0e-4 bar of CO₂ on the column, about the
pre-industrial 280 ppm (270 ppm on the old core). An Earth moved round
the Sun keeps 288 K out to 1.4 AU (0.7 bar of CO₂ at 1.2 AU, 2.1 bar at
1.4 AU), still has its ocean at 1.5 AU (286 K, all its 2.9 bar of CO₂ in
the air) and freezes beyond (254 K at 1.6 AU with all its 1.8 bar of
CO₂, 235 K at 1.7 AU). The column itself puts the edge farther than the
maximum-greenhouse limit of Kopparapu et al. 2013
(doi:10.1088/0004-637X/765/2/131: 273 K at about 1.67 AU with the best
CO₂, up to 8 bar), as a grey column without Rayleigh scattering should:
at 1.6 AU it reaches 274 K under 3 bar of CO₂ and 288 K under 4 bar. The
Earth track runs short of CO₂ there, and Venus's albedo of 0.3, which
pins CO₂'s strength, is issue #510. At 0.9 AU it has run away (717 K, no
water); at 0.01 AU it is a lava world.

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
surface liquid, and life with the oxygen it makes. If a wet surface
has no wet balance (`climate::boils`: its absorbed light and internal
heat exceed the moist limit; Goldblatt et al. 2013,
doi:10.1038/ngeo1892), the body has run away after all, in one pass
with no loop: the steps from the regime on run once more from its
steam (`regime::steam`, the water left after energy-limited escape, in
the air), flagged `boiled`. If escape leaves water and that leftover has
a wet balance of its own (too little vapour to hold the moist limit),
it condenses again and a third run keeps it as an ocean (`boiled`, no
runaway); otherwise the steam run stands, dry when escape took all the
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
| 4 | `lava-world` | surface liquid is lava (at least 1400 K, no water kept) |
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

Measured, not tuned: `cargo run --release -p mapgen --example planet_census
-- 500` draws systems of seeds 1 to 500 and derives every planet through
`body_model`. The medians of temperature and pressure are over the
planets with a surface (`NaN` for the classes that have none). The output
below is the one of the climate pass, lot 1 (issues #441, #442 and #492):
the radiative–convective column with its moist plateau, the runaway
decided by that plateau, and the steam worlds. The census counts the
runaways by their source (`Water::boiled`) and by decade of their steam
column, with the steam worlds apart, and the wet planets left past 647 K
because their boil was not kept.

```text
500 systems, 1552 planets

base preset of the worlds (planets)
barren           293   18.9 %
caustic           53    3.4 %
frozen           567   36.5 %
gas-giant         65    4.2 %
hot-jupiter       45    2.9 %
ice-giant        453   29.2 %
steam-world        2    0.1 %
terran            27    1.7 %
volcanic          47    3.0 %

planets per radius bin (Fulton et al. 2017 bins, R_earth)
    <1    40.9 %  (635)
 1-1.5    19.3 %  (300)
 1.5-2     6.3 %  (98)
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
hydrocarbon-world         8     2.10    0.125   1.736  0.316   0.2609    0.0000      84      3.440
ice-giant               160     5.63   14.366   0.700  0.300   0.2496    0.1805     NaN        NaN
ice-world               448     5.51    0.389   0.369  0.311   0.3465    0.0000      91      0.056
lava-world               18     2.25    2.828   0.041  0.299   0.0006    0.0000    1708      0.000
mini-neptune            293     5.26    5.298   0.315  0.307   0.2427    0.0193     NaN        NaN
ocean-world               6     2.99    1.841   0.136  0.324   0.0019    0.0000     288     14.595
rocky                   397     5.41    0.282   0.048  0.310   0.0005    0.0000     322      0.000
runaway-greenhouse       60     5.63    1.574   0.144  0.320   0.0006    0.0000     842     49.685
steam-world               2     6.60    3.189   0.134  0.492   0.4052    0.0000    2032 623182.205
super-earth              29     4.60    2.756   0.042  0.304   0.0006    0.0000     620      0.109
volcanic-world           21     3.74    0.194   0.059  0.327   0.0009    0.0000     425      0.000

class mix (1552 planets)
ice-world             28.9 %  (448)
rocky                 25.6 %  (397)
mini-neptune          18.9 %  (293)
ice-giant             10.3 %  (160)
gas-giant              4.2 %  (65)
runaway-greenhouse     3.9 %  (60)
hot-neptune            2.8 %  (43)
super-earth            1.9 %  (29)
volcanic-world         1.4 %  (21)
lava-world             1.2 %  (18)
hydrocarbon-world      0.5 %  (8)
ocean-world            0.4 %  (6)
hot-jupiter            0.1 %  (2)
steam-world            0.1 %  (2)

planets with a surface: 983
no giant, surface hidden by an envelope: 6 (0.4 %)
  ice-world              2
  super-earth            4

surface liquid (983 planets with a surface)
Hydrocarbons           0.8 %  (8)
Lava                   1.8 %  (18)
None                  94.4 %  (928)
Water                  3.0 %  (29)

tectonic regime (983 planets with a surface)
Episodic              21.3 %  (209)
HeatPipe               3.2 %  (31)
Plates                34.1 %  (335)
Stagnant              41.5 %  (408)

surface age, Ga, by class and regime (planets with a surface)
class               regime         n  median    mean
hydrocarbon-world   Stagnant       8   0.269   1.167
ice-world           Plates       258   0.403   0.407
ice-world           Stagnant     188   3.375   2.697
lava-world          Episodic      12   0.471   0.492
lava-world          HeatPipe       2   0.001   0.001
lava-world          Stagnant       4   4.126   3.353
ocean-world         HeatPipe       1   0.000   0.000
ocean-world         Plates         4   0.376   0.378
ocean-world         Stagnant       1   1.022   1.022
rocky               Episodic     127   0.540   0.541
rocky               Plates        63   0.376   0.379
rocky               Stagnant     207   4.574   3.629
runaway-greenhouse  Episodic      53   0.521   0.520
runaway-greenhouse  HeatPipe       7   0.001   0.001
steam-world         Episodic       2   0.370   0.370
super-earth         Episodic      15   0.605   0.592
super-earth         Plates        10   0.504   0.450
volcanic-world      HeatPipe      21   0.000   0.000
all                              983           1.555

runaways: 383 by the insolation limit, 0 boiled; of the runaway-greenhouse class: 60 by the limit, 0 boiled
boils whose leftover water condensed again: 0
wet, not in runaway and past 647 K: 0
runaways by steam column (other classes / steam worlds, median K), steam-world line 2206.3999999999996 bar:
        <1 bar:  374 /    0  500 K
      1-10 bar:    3 /    0  1368 K
    10-1e2 bar:    3 /    0  2617 K
   1e2-1e3 bar:    1 /    0  1605 K
   1e3-1e4 bar:    0 /    0  NaN K
   1e4-1e5 bar:    0 /    0  NaN K
     >=1e5 bar:    0 /    2  2032 K

life: 1.1 % of planets with a surface (11), 0.7 % oxygenated (7)

radius histogram, period under 100 days (818 planets)
 0.00-0.25      0.0 %  (0)
 0.25-0.50      7.6 %  (62)
 0.50-0.75     21.8 %  (178)
 0.75-1.00     19.3 %  (158)
 1.00-1.25     13.2 %  (108)
 1.25-1.50      7.0 %  (57)
 1.50-1.75      3.4 %  (28)
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
ice-world             14.2 %  (116)
lava-world             2.2 %  (18)
mini-neptune          16.0 %  (131)
ocean-world            0.6 %  (5)
rocky                 42.9 %  (351)
runaway-greenhouse     6.2 %  (51)
steam-world            0.2 %  (2)
super-earth            2.9 %  (24)
volcanic-world         2.6 %  (21)
```

What it shows, plainly:

- **Steam worlds** are 0.1 % of the planets (2 of 1552), both far over
  100 000 bar of steam (median 2032 K at the surface, 623 182 bar). They
  are the only runaways above 1e5 bar; the 381 other runaways hold
  under 1e4 bar (374 of them under 1 bar of steam) and stay
  `runaway-greenhouse` or another class.
- **Runaway greenhouses** are 3.9 % (60), all by the insolation limit
  (none boiled): median 842 K under 49.7 bar. A boil whose water
  condensed again once the planet was dry is counted apart (none here),
  and no wet planet is left past 647 K out of a runaway.
- **Radius valley.** With the close-in planets (818 with a period under
  100 days) the histogram now dips at 1.50-2.00 R⊕ (3.4 % and 2.0 %)
  between the rocky peak at 0.5-1.0 R⊕ and a second hump at 2.0-3.0 R⊕
  (2.8, 2.7 and 3.5 % in the bins from 2.00 to 2.75), and 39.8 % of all
  planets are above 1.5 R⊕. The model was not tuned to produce a valley.
- **Mini-Neptunes** are 18.9 % (293) and hot Neptunes 2.8 % (43): the
  class mix counts the envelope classes the radius histogram reads.
- **Lava worlds** are 1.2 % (18), airless dry rocks at a median 1708 K,
  and volcanic worlds 1.4 % (21), all in the heat-pipe regime.
- **Hydrocarbon worlds** are 0.5 % (8), small icy bodies (median
  0.125 M⊕) far from their stars (median 1.7 AU) under 3.4 bar at 84 K.
- **Ocean worlds** are 0.4 % (6), held at 288 K by their thermostat under
  a median 14.6 bar.
- **Surfaces hidden by an envelope**: 6 planets that are no giant (0.4 %),
  2 ice worlds and 4 super-Earths.
- **Tectonic regimes** over the 983 planets with a surface: plates
  34.1 %, stagnant lid 41.5 %, episodic 21.3 %, heat pipe 3.2 %.
- **Surface liquid**: none for 94.4 %, water 3.0 % (29), lava 1.8 % and
  hydrocarbons 0.8 %.
- **Life** arises on 1.1 % of the planets with a surface (11), 0.7 %
  with oxygen (7).
