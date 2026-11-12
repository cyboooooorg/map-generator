# Planet atmospheres, climate and surface (part 2)

Part 2 of the physical planet model (issue #423) derives what a planet's
interior does not fix: whether it keeps its envelope, its tectonic
regime, atmosphere, climate, liquid and life. It is written section by
section as the tasks land; this file starts with the XUV fluence. The
interiors are in [planet-interiors.md](planet-interiors.md).

## XUV fluence

`planet::xuv::xuv_fluence` gives the XUV energy per unit area a planet
has received over its system's age, relative to Earth's (the Sun at
1 AU at 4.567 Gyr is 1.0). It is the budget that drives photoevaporation.

The star emits `L_XUV / L_bol = 3.71e-4` (1 to 1180 Å) while
saturated: the Sun's own, from the fit `F = 29.7 τ^-1.23` erg s⁻¹ cm⁻²
at 1 AU of Ribas et al. 2005 (doi:10.1086/427977; τ in Gyr, saturated
before 0.1 Gyr), 504 erg s⁻¹ cm⁻² at 0.1 Gyr over the 1361 W/m² solar
constant (fit and band as cited in the model's review, not re-read). It
is below the X-ray saturation of the fastest rotators, `L_X / L_bol ≈
10^-3` (Wright et al. 2011, doi:10.1088/0004-637X/743/1/48). It lasts
`t_sat = 0.1 Gyr · M^-2` (M in solar masses; 0.1 Gyr for the Sun from Jackson, Davis &
Wheatley 2012, doi:10.1111/j.1365-2966.2012.21384.x, the mass exponent is
a calibration giving 1.1 Gyr at 0.3 M☉ and 2.5 Gyr at 0.2 M☉), then
decays as `t^-1.23` (Ribas et al. 2005, doi:10.1086/427977). The
integral over the age is analytic and is multiplied by the insolation.
The luminosity is taken constant over the age.

Calibration targets: Earth is 1.0; its absolute value
`EARTH_XUV_FLUENCE_J_M2` = 5.64e15 J/m² is Ribas et al.'s fit integrated
over 4.567 Gyr, and today's flux 4.6 mW/m² is theirs, both within 1 %;
5 to 10 Gyr adds under 30 %; a 0.2 M☉ star at Earth's insolation delivers more than 10 times Earth's
fluence.

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

**Regime.** A labelled calibration, since the literature disagrees on the
mass dependence (see [plate tectonics](plate-tectonics.md)):

- heat pipe above 2 W/m² of internal heat (Io; Moore & Webb 2013,
  doi:10.1038/nature12424, value not checked against the paper; Earth's
  mean is 0.087 W/m²);
- plates from 0.3 M⊕ (`geology::PLATES_MIN_MASS_EARTH`, shared with the
  generator) when liquid water is kept, never in a runaway (its steam
  does not weaken the lithosphere);
- episodic when the mass allows plates but the water is gone or is steam
  (Venus);
- stagnant otherwise (Mars, Mercury, the Moon).

Calibration targets: Earth keeps 0.9 to 1.1 oceans and has plates; Venus
is in runaway, loses even an Earth ocean and is episodic; Mars and
Mercury are stagnant; Io with 1e14 W of tidal heat is a heat pipe; a dry
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
1). A share of it is the dipole at the core's surface: 0.067 for a
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
and O₂ (`Gases`, with `total()`), in four steps, called in this order:
`outgassed`, `atmosphere`, `cold_trap`, `photochemistry`. The carbonate
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
| Plates | 0.95, of which subduction returns 80 % of the nitrogen (N₂ and NH₃) and 63 % of the rest | Earth's N₂ once its NH₃ is photolysed (Goldblatt et al. 2009, doi:10.1038/ngeo692, on the mantle's large nitrogen store, not checked) |
| Episodic lid | 0.95 | Venus's 90 bar of CO₂ |
| Heat pipe | 0.95 | none (Io) |
| Stagnant lid | 0.2 | Mars, Titan |

**Under a deep ocean** (`released_under_ocean`), only a share of that
reaches the air: `1 / (1 + (P_floor / 3 GPa)^4)`, with `P_floor` the
weight of the water kept on the seafloor, `m_water g / (4π R²)`. The
weight of a deep ocean curtails the exchange of volatiles between mantle
and ocean (Kite & Ford 2018, ApJ 864, 75, doi:10.3847/1538-4357/aad6e0,
for waterworlds of 10 to 1000 times Earth's water, whose pCO₂ often lands
between 0.2 and 20 bar), high-pressure ice at its base seals the rock
(Noack et al. 2016, Icarus 277, 215, doi:10.1016/j.icarus.2016.05.009;
the pressure at which they find outgassing stops is not checked against
the paper), and the ices' own volatiles stay dissolved in the ocean or
locked in clathrates. The 3 GPa and the power 4 are calibrations with no
source: Earth's ocean (0.03 GPa) changes nothing, Titan (1.0 GPa in the
model), Ganymede and Callisto keep 98 % or more of their air, and a 1 M⊕
planet of 30 % water (17 GPa) lets out about 1e-3 of it, a few bar of
CO₂ before the thermostat. A runaway's steam weighs on the rock as its
ocean did, so the share holds in a runaway too; the steam itself is not
held back.

Sulphur comes up as SO₂, of which 1e-4 stays in the air (the rest ends in
the crust; Venus's SO₂ is buffered by its surface, Fegley & Prinn 1989,
not checked; calibrated on Venus's 150 ppm). Water is vapour only in a
runaway greenhouse, and only what the runaway left (`Water::kept_kg`);
otherwise it is an ocean or ice, and the climate sets its vapour.

**Escape** (`atmosphere`). The cosmic shoreline of Zahnle & Catling 2017
(doi:10.3847/2041-8213/aa7846) splits the bodies with and without air by
`I_XUV ∝ v_esc⁴`. Here the line is `I_line = (v_esc / 6.5 km/s)⁴`, in
units of Earth's XUV fluence (`xuv_fluence`), and the share of air kept
falls smoothly across it:

```text
kept = 1 / (1 + (s I / I_line)^6)
```

with `s = 2` for a body without a dynamo, whose upper air the stellar
wind strips (`UNMAGNETISED_ESCAPE`, a calibration with no source, tied to
issue #344). The normalisation 6.5 km/s, the steepness 6 and `s` are
calibrations: the line passes close to Mars, as in the paper, and `s` is
small enough that Venus, which has no dynamo, keeps its air. Every gas is
kept in the same share. `escape_velocity_km_s` gives `sqrt(2 G M / R)`.

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
Life's methane is added later.

Calibration targets, through the whole chain from the reference bodies'
inputs (volatiles of the rule, no scatter) round the Sun at 4.6 Gyr:

| Body | Result | Target | Observed |
|---|---|---|---|
| Venus | 87 bar, 98 % CO₂, 1.7 bar N₂ | 60 to 120 bar, CO₂ over 90 %, N₂ 1 to 6 bar | 92 bar, 96.5 % CO₂, about 3 bar N₂ |
| Earth | 0.89 bar N₂, no CH₄ or NH₃ (45 bar CO₂ before the thermostat) | N₂ 0.5 to 1.2 bar; CH₄ and NH₃ under 1e-3 bar | 0.78 bar N₂ |
| Mars | 0.014 bar | 0.002 to 0.05 bar | 0.006 bar |
| Titan | 2.1 bar, 98 % N₂, 0.035 bar CH₄ | 0.5 to 5 bar; CH₄ over 0.01 bar | 1.5 bar, about 5 % CH₄ |
| Mercury, Moon | under 1e-8 bar | under 1e-4 bar | exospheres |

A shortcoming, outside the targets: at Jupiter's distance (equilibrium
temperature 112 K, above methane's triple point and under 150 K), Ganymede keeps about 0.2 bar of CH₄ and Callisto,
to which the core model gives a weak deep dynamo, about 0.5 bar (both
are airless). Moons are not modelled yet (`body_model` skips them).

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
τ  = (P / 1 bar)^½ · Σ k_i (p_i / 1 bar)^a_i
```

`T_eq` is the context's equilibrium temperature (Bond albedo 0.3), `P`
the surface pressure and `p_i` the partial pressures of CO₂, H₂O and
CH₄. The square roots are the strong-line limit of a pressure-broadened
band, which absorbs as `sqrt(S u α)` with the path `u ∝ p_i` and the
width `α ∝ P` (Pierrehumbert 2010, §4.4; Goody & Yung 1989). CO₂'s
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

**Water vapour.** Where water is kept and there is no runaway, the air
above the ocean or the ice holds 77 % of the saturation pressure (the
fixed relative humidity of Manabe & Wetherald 1967,
doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2, not checked). The
saturation is Clausius–Clapeyron from water's triple point (273.16 K,
611.657 Pa) with a mean latent heat of 2.4e6 J/kg, capped at the
critical point (647 K). The temperature and the vapour are solved
together by iterating from the dry temperature, which climbs to the
coldest balance. A runaway's steam is already in the gases and is kept
as it is.

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
| Venus | 736 K, none | 737 ± 50 K, none | 737 K |
| Mars | 209 K, none | 210 ± 12 K, none | about 210 K |
| Titan | 94 K, hydrocarbons | 94 ± 8 K, hydrocarbons | 94 K, methane and ethane lakes |
| Mercury | 410 K, its equilibrium temperature | within 1 K of it | airless |

An Earth moved round the Sun keeps 288 K out to 1.4 AU (0.17 bar of CO₂
at 1.2 AU, 0.8 bar at 1.4 AU), is just short at 1.6 AU (288 K less 0.2,
all its 1.8 bar of CO₂ in the air, its water still liquid) and freezes
beyond (234 K at 1.8 AU). At 0.9 AU it has run away (705 K, no water);
at 0.01 AU it is a lava world.

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
Planavsky 2014, doi:10.1038/nature13068; the date is from memory, not
checked against the paper). The oxygenated air holds O₂ at `0.27` of its
N₂ (calibration on Earth: 0.21 over 0.78) and no methane, which the
oxygen oxidises.

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
core and dynamo, the air (`outgassed`, `atmosphere`, `cold_trap`, then
`photochemistry`, which reads the cold trap's air), the climate and
surface liquid, and life with the oxygen it makes. If that climate
carries a wet surface past water's critical point (`climate::boils`),
the body has run away after all: the steps from the regime on run once
more from its steam (`regime::steam`, the water left after escape, in
the air), so that a body never keeps a liquid ocean above 647 K. A body that keeps an
envelope has no surface: `PlanetModel.surface` is `None`. Otherwise the
`Surface` holds the water, regime, core, final gases, climate and life,
and `classify` reads the class off it. Nothing derives from the class.

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
runaway greenhouse (Venus has 90 bar; a runaway that lost its air is a bare rock) and for
the hydrocarbon world. The pressure condition on hydrocarbons is a
departure from the plain rule "liquid hydrocarbons": without it the model
makes Ganymede and Callisto hydrocarbon worlds, because it outgasses 0.2
and 0.5 bar of methane from their ices and holds it, which the real
moons do not have. Titan has 2.1 bar in the model. That excess methane is
a weakness of the atmosphere step, not something the classifier should
hide for good.

The Solar System classifies as itself: Venus `runaway-greenhouse`, Earth
and Mars `rocky`, Titan `hydrocarbon-world`, Ganymede and Europa
`ice-world`, the giants as in part 1. A 1 M⊕ planet of 30 % water at
1 AU, with the volatiles `draw_inputs` gives its rock and ice (no
scatter), is an `ocean-world` at 288 K under 0.4 bar. With 1 % water
(about 40 oceans) at 1 AU, the 20 bar of nitrogen its ice brings carry
the vapour feedback past 647 K: it is a `runaway-greenhouse`, never a
lava world. The model is
finite and non-negative on mass 0.01 and 20 M⊕, insolation 1e-3 and 1e4,
age 0.5 and 13 Gyr, M5V and A0V stars.

## Census

Measured, not tuned: `cargo run --release -p mapgen --example planet_census
-- 500` draws systems of seeds 1 to 500 and derives every planet through
`body_model`.

```text
500 systems, 1677 planets

body_type against derived class (planets)
body_type              gas-giant         hot-jupiter   hydrocarbon-world           ice-giant           ice-world          lava-world        mini-neptune         ocean-world               rocky  runaway-greenhouse         super-earth   total
barren                         0                   0                   0                   0                  37                  16                   0                   2                 339                  15                   1     410
caustic                        0                   0                   0                   0                   7                   6                   1                   0                  57                  24                   0      95
frozen                         0                   0                 128                   0                 547                  10                  56                  31                   2                   0                   0     774
gas-giant                     41                   0                   0                   0                   0                   0                   0                   0                   0                   0                   0      41
hot-jupiter                    0                   1                   0                   0                   0                   0                   0                   0                   0                   0                   0       1
ice-giant                     28                   0                   0                  56                   0                   0                   0                   0                   0                   0                   0      84
terran                         0                   0                   0                   0                  11                  10                   4                   7                 131                   3                   8     174
volcanic                       0                   0                   0                   0                   6                   4                   0                   0                  80                   8                   0      98

planets per radius bin (Fulton et al. 2017 bins, R_earth)
    <1    55.9 %  (938)
 1-1.5    31.0 %  (520)
 1.5-2     3.5 %  (58)
   2-3     2.1 %  (36)
   3-4     1.9 %  (32)
   4-6     2.4 %  (41)
  6-10     1.9 %  (32)
 10-22     1.2 %  (20)
   >22     0.0 %  (0)

medians per class
class               planets   age Gyr    CMF    water  envelope
gas-giant                69      4.90  0.332   0.0283    0.9000
hot-jupiter               1      6.33  0.382   0.0000    0.9445
hydrocarbon-world       128      5.75  0.317   0.3514    0.0000
ice-giant                56      5.35  0.313   0.2924    0.1614
ice-world               608      5.51  0.308   0.3441    0.0000
lava-world               46      4.42  0.303   0.3193    0.0000
mini-neptune             61      6.09  0.312   0.3434    0.0089
ocean-world              40      3.65  0.296   0.3007    0.0000
rocky                   609      5.34  0.311   0.0005    0.0000
runaway-greenhouse       50      4.13  0.310   0.0007    0.0000
super-earth               9      3.53  0.279   0.0007    0.0014

class mix (1677 planets)
rocky                 36.3 %  (609)
ice-world             36.3 %  (608)
hydrocarbon-world      7.6 %  (128)
gas-giant              4.1 %  (69)
mini-neptune           3.6 %  (61)
ice-giant              3.3 %  (56)
runaway-greenhouse     3.0 %  (50)
lava-world             2.7 %  (46)
ocean-world            2.4 %  (40)
super-earth            0.5 %  (9)
hot-jupiter            0.1 %  (1)

planets with a surface: 1479

surface liquid (1479 planets with a surface)
Hydrocarbons          11.8 %  (175)
Lava                   3.1 %  (46)
None                  77.6 %  (1148)
Water                  7.4 %  (110)

tectonic regime (1479 planets with a surface)
Episodic              19.5 %  (289)
HeatPipe               2.6 %  (39)
Plates                42.3 %  (625)
Stagnant              35.6 %  (526)

life: 2.4 % of planets with a surface (36), 2.1 % oxygenated (31)

radius histogram, period under 100 days (867 planets)
 0.00-0.25      0.0 %  (0)
 0.25-0.50     10.4 %  (90)
 0.50-0.75     20.2 %  (175)
 0.75-1.00     36.9 %  (320)
 1.00-1.25     23.2 %  (201)
 1.25-1.50      5.8 %  (50)
 1.50-1.75      0.9 %  (8)
 1.75-2.00      0.6 %  (5)
 2.00-2.25      0.5 %  (4)
 2.25-2.50      0.3 %  (3)
 2.50-2.75      0.2 %  (2)
 2.75-3.00      0.0 %  (0)
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
  23.2 % in 1.00-1.25, 5.8 % in 1.25-1.50, 0.9 % and 0.6 % in the two
  bins of the valley, then a thin tail. There is no peak at 1.25-1.5 and
  no second peak at 2-3 R⊕ for a valley to separate. The generator draws
  few planets above 1.5 R⊕ at all (13 % of all planets), most of
  them beyond 100 days, so photoevaporation has no population of
  sub-Neptunes to carve a gap in. The model was not tuned to produce one.
- **Mini-Neptunes** are 3.6 % of the planets (61), against the
  over-representation of part 1 (see [planet-interiors.md](planet-interiors.md)).
  The `frozen` type holds 56 of them; escape strips the ones close in.
- **Surface liquid** over the 1479 planets with a surface: no liquid for
  77.6 %, hydrocarbons 11.8 %, water 7.4 %, lava 3.1 %.
- **Life** arises on 2.4 % of the planets with a surface, 2.1 % with
  oxygen.
- **Hydrocarbon worlds** are 7.6 % of the planets, nearly all of them
  `frozen`: the volatile rule gives ices methane and nitrogen, and the
  model keeps over a bar of it on the wet ones.
  This is likely too many; it is the same excess methane that the
  Ganymede and Callisto remark above points to.
- Ocean worlds are 2.4 %, runaway greenhouses 3.0 %, lava worlds 2.7 %.

