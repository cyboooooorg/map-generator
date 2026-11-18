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
and O₂ (`Gases`, with `total()`), in six steps, called in this order:
`outgassed`, `atmosphere`, `non_thermal_loss`, `methane_photolysis`,
`cold_trap`, `photochemistry`. The carbonate
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

Sulphur comes up as SO₂, of which 5e-5 stays in the air (the rest ends in
the crust; Venus's SO₂ is buffered by its surface, Fegley & Prinn 1989,
doi:10.1038/337055a0, the share not checked; calibrated on Venus's
150 ppm, and washed out of a wet or oxygen-rich air, see **Sulphur and
SO₂** below). Water is vapour only in a
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

**Non-thermal loss** (`non_thermal_loss`, after the shoreline, on the
whole stock, frosts to come included). A thin air is an atmosphere only
if the body supplies it faster than it loses it. Inside a giant's
magnetosphere (a moon of a host of 10 M⊕ to 13 Jupiter masses, the
heaviest giant drawn, `LossContext.host`; a heavier host is the star
whose tides a planet's model reads),
each molecule at the top of the air meets a loss per m² that does not
depend on how much air there is:

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
them (`climate::frost` reads what the cold trap froze out of what is
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
`exosphere` under a gas the step exhausted.

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
  reads it as a frost (`climate::frost`, where the surface keeps it under
  SO₂'s sublimation pressure), with its own albedo, `SO2_FROST_ALBEDO`
  = 0.63, **calibration** on Io's Bond albedo (Simonelli & Veverka
  1988, from memory, not checked) for a disc nearly all frost: darker
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
  doi:10.1016/j.pss.2009.09.011) review SO₂'s sublimation pressure too;
  their table was not read.
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
(the supply overtakes the destruction at about 1.2 Gyr), of which the
climate leaves 0.11 bar in its air, the vapour of its seas (see
**Methane** under the climate below). Ganymede and Callisto get 3.4
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
0.0606 bar, 29 kJ/mol; CH₄ 90.7 K, 0.117 bar, 9.1 kJ/mol; values from
the NIST WebBook from memory, not checked). N₂'s curve is measured
(issue #529): triple point 63.14 K and 0.1253 bar (Angus, de Reuck et
al. 1979 and Jacobsen, Stewart et al. 1986, through the NIST WebBook,
checked), and below it the polynomial fits `ln p = Σ Aᵢ / Tⁱ` of Fray &
Schmitt 2009 (*Planet. Space Sci.* 57, 2053,
doi:10.1016/j.pss.2009.09.011), as Lisse et al. 2021 (*Icarus* 356,
114072, doi:10.1016/j.icarus.2020.114072, supplementary Table 2)
reproduce them (`N2_ALPHA_FIT`, `N2_BETA_FIT`): β-N₂ from the triple
point down to the α–β transition at 35.61 K (Scott 1976, from memory),
α-N₂ below. The phase of each fit is read off its values (the β fit
gives 0.1215 bar at the triple point, the α fit 0.317 bar), the paper
itself not read. Each fit is pinned to the curve above it (the β fit to
NIST's triple point, 3 % higher; the α fit to the β curve at the
transition, 2.6 % apart), and under 10 K, where the α polynomial turns
over, the curve goes on with its slope there. It gives 1.4e-5 bar at
37.2 K: Triton's N₂ ice, at 37 to 38 K, is β-N₂. This freezes Titan's
CO₂ and NH₃. Above the triple point the
gas stays; whether it rains out depends on the surface temperature,
which the climate sets. Water is left to the climate, and so are methane
(issue #443) and nitrogen (issue #464): their air moves the surface
temperature they condense at, so the climate condenses them at the
temperature it solves for (`condense`; nitrogen over its own ice, with
activity 1, its liquid above 63.14 K up to its critical point,
126.19 K, through its normal boiling point, 77.34 K). The CO₂ the cold trap froze out stays on the ground as a
frost wherever the surface is cold enough to keep it (below CO₂'s
triple point, beyond what the air can take back at the surface's
sublimation pressure), and the frosts of N₂, CH₄ and CO₂ brighten the
body's Bond albedo (`albedo::cover`: 0.75, covering the share
`frost / 0.02 bar` of the disc), as does a heat pipe's volcanic SO₂
frost (0.63, see *Volcanic SO₂*).

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
| Titan | 2.2 bar, 95 % N₂, 0.11 bar CH₄ after the climate (6.3 bar before it condenses) | 0.5 to 5 bar; CH₄ over 0.01 bar | 1.5 bar, about 5 % CH₄ |
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
| Titan | hydrocarbon world, 93.5 K, 1.43 bar, 94 % N₂, 0.083 bar CH₄ (5.8 %) | 93.7 K, 1.47 bar, 5.7 % CH₄ |
| Ganymede, Callisto | exospheres of 3e-12 bar (CO₂), 116.6 K, Bond albedo 0.19 (alone, out of resonance, Ganymede keeps one old unit, Callisto's; in its chain, 0.45 and 102 to 108 K, see "Icy albedo") | exospheres of 1e-12 (O₂) and 7.5e-12 bar (CO₂); Bond albedo about 0.4 and 0.2 |
| Io | heat pipe, 9.3e-10 bar of SO₂ over its SO₂ frost (N₂ and CO₂ an exosphere of 3e-12 bar), 105.2 K, Bond albedo 0.63 | heat pipe, SO₂ of about 1e-9 bar, Bond albedo 0.63 under SO₂ frost |
| Europa | exosphere of 3e-12 bar (CO₂), 96.1 K, Bond albedo 0.63 | exosphere of about 1e-12 bar (O₂), Bond albedo about 0.62 to 0.68 |
| Triton | 1.7e-5 bar of N₂ over its frost at 37.5 K, Bond albedo 0.75 | about 1.4e-5 bar of N₂ at 38 K, Bond albedo about 0.75 |
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
a kelvin of the observed 38 K, and the emissivity 0.89 centres the
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

**Radiative–convective grey column** (`surface_temperature_k`,
`planet::column`, issue #441). The surface temperature is the one at
which the outgoing longwave radiation (OLR) of a grey column over the
surface balances what it absorbs, `σ T_eq⁴ + q`, with `T_eq` the
equilibrium temperature under the body's derived Bond albedo
(`planet::albedo`, issue #510: its oceans, land, ice, frost, clouds,
Rayleigh scattering, methane haze and sulphuric deck, solved with the
temperature as a fixed point) and `q` the internal heat flux. The
column:

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
287 W/m², the column's OLR over a saturated surface at 380 K, under the
stratosphere the surface solves give a body at that limit, skin
`T_eq / 2^¼` with `T_eq = (F_plateau / σ)^¼`) exactly at its star's
runaway limit `S_rg` (`star::habitable_fluxes`), clamped to 0 to 0.9:
`A_rg = 1 − 4 F_plateau / (S_rg S₀)`, with `S₀` the solar constant
behind `T_eq` (255 K at 1 S⊕ with albedo 0.3). The albedo moves from
the body's derived albedo `A_d` to `A_rg` with the steam column,
`A = A_d + (A_rg − A_d) (1 − exp(−p_steam / 1 bar))` (`STEAM_ALBEDO_BAR`,
a calibration: the moist atmospheres of Kopparapu et al. hold a fraction
of a bar to about one bar of vapour at the limit), so a thick column
takes `A_rg` whole and a runaway losing the last of its water comes back
to its derived albedo without a jump. Kopparapu et al.
2013 (doi:10.1088/0004-637X/765/2/131) and 2014 place the runaway limit
where the light a moist atmosphere absorbs, with the albedo its own
spectrum gives under that star, meets its outgoing limit. With Earth's
0.3, a planet round a red star that runs away by that limit
(0.91 S⊕ for an M9V) would absorb under the plateau and sit on water's
critical point. With `A_rg` (0.08 for an M9V, 0.24 for the Sun) it
balances past the plateau. Wet bodies keep their derived albedo (Earth's
about 0.3), and so does a runaway whose escape took all its water
(Mercury, Venus), which has no moist air left.

`p_i` are the partial pressures of CO₂, H₂O and CH₄ and `P_b,i` the
pressure that broadens each gas's lines (below). The square roots are the
strong-line limit of a pressure-broadened band, which absorbs as
`sqrt(S u α)` with the path `u ∝ p_i` and the width `α` growing with the
pressure round the lines (Pierrehumbert 2010, §4.4; Goody & Yung 1989).
CO₂'s power is smaller, 0.19: its 15 µm band is saturated at the centre,
so its forcing grows with the logarithm of its amount (Myhre et al. 1998,
doi:10.1029/98GL01908). With Venus pinning its opacity (737 K under 85
bar at its derived Bond albedo 0.76, its sulphuric deck's), the power
sets how strong small amounts of CO₂ are: the late Archean Earth (below)
must stay above 270 K and warm by 3 to 8 K when its nitrogen doubles;
0.19 gives 272 K and +7.0 K (0.27 left it at 259 K, 0.25 at 268 K). Under 1e-6 bar (`CO2_WEAK_BAR`) CO₂ absorbs in
proportion to its amount, the weak-line limit; without it Titan's 3e-10
bar of CO₂ warmed it by about 3 K. The opacities are calibrations with no
source:

| Gas | `k_i` | `a_i` | Calibrated on |
|---|---|---|---|
| CO₂ | 1.45 (0.74 under the fixed albedo 0.3, 1.66 before #441) | 0.19 (0.07, 0.25 before #441) | Venus's 737 K under 85 bar at its Bond albedo 0.76, and the late Archean Earth |
| H₂O | 111 (5.9) | 0.5 | the moist plateau at about 287 W/m² |
| CH₄ | 7.75 (6.0 before #530, 3.3 before #443, 3.4 before #441) | 0.5, saturating past 0.01 bar | Titan's 93.7 K and 3 to 6 % of methane, with the anti-greenhouse |

Titan's real greenhouse is mostly collision-induced absorption by N₂,
CH₄ and H₂, partly undone by its haze (McKay, Pollack & Courtin 1991,
doi:10.1126/science.11538492, not checked); the methane opacity carries
all of it here but the near-infrared share its methane takes (below).

**Methane** (issue #443). The `sqrt(p_CH₄ P)` law, calibrated on Titan,
was stretched 500-fold past it: a planet at an equilibrium temperature of
114 K under 26 bar of methane (seed 182, planet d) came out at 351 K with
liquid water. Three rules bound it:

- *Condensation at the surface.* Inside the climate's fixed point
  (`atmosphere::condense`, applied at each step to the temperature the
  last one found, from the black body's up to the coldest balance), the
  air keeps at most the vapour of a Titan sea, `x p_sat(T_s)`, or of
  methane ice, `p_sat(T_s)`, under the sea's freezing point
  (`sea_freezes_k`; activity 1 over a pure solid, issue #530): `p_sat`
  the saturation pressure of pure methane
  (`atmosphere::methane_saturation_bar`: over its ice below its triple
  point, 90.7 K and 0.117 bar, over its liquid above with the mean latent
  heat joining the triple point to its normal boiling point, 111.67 K,
  and none past its critical point, 190.56 K) and `x` = 0.5 the methane
  share of a sea of methane, nitrogen and ethane (`SEA_METHANE_SHARE`,
  Raoult's law; Glein & Shock 2013 and Tan, Kargel & Marion 2013 find
  0.5 to 0.7, and Titan's near-surface air holds 45 to 50 % of pure
  methane's saturation, Niemann et al. 2005). The rest lies on the
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
  `τ = 7.75 (u (P − p + u))^½`. Past Titan's amounts its bands are opaque
  at their centres, and more methane only widens their wings (the
  logarithmic forcing of a saturated band, Pierrehumbert 2010, §4.4); a
  body at 90 to 120 K radiates at 25 to 60 µm, where methane has no
  strong band of its own. With it, raising Titan's methane from 1 to
  10 bar holds back under a third of the outgoing radiation that 0.01
  to 0.1 bar holds back (with the opacity 6.0, 0.6 K of warming against
  2.6 K).
- *Anti-greenhouse* (issue #530). The methane above the tropopause
  absorbs starlight in its near-infrared bands and radiates it back to
  space, so the surface never gets it (McKay, Pollack & Courtin 1991,
  *Science* 253, 1118, describe it on Titan together with its haze's;
  not checked beyond the bibliographic data). `climate::absorbed_w_m2`
  takes the share `0.3 m / (m + 0.005 bar)` off the light the surface
  absorbs (`CH4_NEAR_IR_MAX`, `CH4_NEAR_IR_HALF_BAR`), `m` the methane
  above the 0.1 bar tropopause that the cold trap lets through
  (`column::stratospheric_methane_bar`); the internal heat is not
  touched, and the stratosphere that absorbs it is not warmed in the
  column. Both constants are calibrations, no source, with the opacity
  7.75: the reference Titan loses 11 % of its light, and since a warmer
  tropopause lets more methane through, a Titan nearer its star loses
  more, which narrows the archetype's spread of temperature and of the
  methane its seas keep in its air. A world of 26 bar of methane takes
  nearly all of the 0.3. The share of the light in methane's bands
  depends on the star's spectrum; one value serves every star.

Titan at Saturn's orbit (`titan-like`, seeds 1 to 40 of
`archetype_census`) comes out at 90.6 to 93.0 K (median 92.1 K), 1.28 to
1.54 bar and 3.8 to 5.7 % methane (median 4.8 %), against Titan's 93.7 K,
1.47 bar and 5.65 % (90.4 to 94.5 K and 3.7 to 6.8 % before the
anti-greenhouse); the seed under 1.3 bar is one drawn with 1.21 bar of
N₂. Seed 182's planet d of `planet_census` is a hydrocarbon world at
135 K (149 K before), with 2.6 of its 26 bar of methane in its air and
the rest in its seas, under the 150 K line.

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

Methane keeps the whole pressure, its own counted as the saturated
amount `u` (see **Methane** above): its opacity stands for Titan's
collision-induced greenhouse, which grows with the nitrogen faster than
line broadening.

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
so that a surface at `THERMOSTAT_K`, 288 K (a calibration on Earth), the
vapour taken at that temperature, sends out what it absorbs (solved in
`ln CO₂`). It can build the CO₂ up past what the air held before the
climate (what escape and the cold trap left of it), since weathering
slows and the volcanoes keep giving: up to the planet's carbon inventory
(all it outgassed, `CarbonCycle::inventory_bar`), capped where CO₂ frosts
out at its equilibrium temperature (`co2_frost_bar`). If that most is
not enough, the planet keeps the CO₂ of its maximum greenhouse and is
colder: below 273 K its water freezes. If no CO₂ at all is still too
warm, the planet is hotter. Only a temperature reached is `regulated`.
The grid's `carbon_cycle` is unchanged; part 3 aligns them.

**Bond albedo** (`albedo::cover`, `albedo::bond_albedo`, issue #510). The
albedo is no longer Earth's 0.3 for everyone: it follows what covers the
disc, and the climate and the albedo are solved together as a fixed point
(`Context::with_albedo` turns an albedo into the equilibrium temperature
the column absorbs at). The disc is split, planet-wide, into open ocean
(0.06), bare land (0.25 under an air, the regolith's 0.11 on an airless
body, shielded from space weathering by `1 - exp(-p / 1e-4 bar)`), water
ice and snow (0.6, from the ice line at -5 °C and the equator-to-pole
contrast of an Earth-like planet, `ice_line_share`), frosts of N₂, CH₄ and
CO₂ (0.75, the share `frost / 0.02 bar` of the disc, all frosts together)
and of volcanic SO₂ (0.63, its part of that share), under a water cloud
cover that grows with the vapour (`1 - exp(-p_H₂O / 1.15e-2 bar)`, Earth's
0.67 at its 1.26e-2 bar, each cloud 0.19), Rayleigh scattering of the air's
column, a methane haze and, from 1e-3 bar of SO₂ (what the washing out
leaves) in a surface above 273 K, a sulphuric acid deck that hides all
below it (0.76), over a share of the disc that grows linearly from none
at 273 K to all of it at 313 K (`DECK_ONSET_K`, a calibration: a deck
whole at one temperature left the fixed point no balance between a warm
clear sky and a cold decked one). The model stores the air washed as the
albedo washed it, and the look draws the same deck over the same share. Each constant is
calibrated on several bodies at once and carries its source (several are
from memory, marked in `albedo.rs`). The model gives Earth 0.29, Venus
0.76, Mars 0.25, a frozen Earth-like planet 0.62 to 0.64, Triton 0.75 and
Io 0.63.

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
  a little dust darkens ice much. `0.006` is a calibration on Europa
  (`ρ` 9 to 16).
- **A dark lag** covers patches of it: a share
  `0.88 · min(1, rock / 0.5) · (1 − exp(−N / 0.004))` of the unit,
  `N` the craters of 1 km or more per km² of the lunar chronology on a
  surface that old (`craters::crater_density`, Neukum, Ivanov &
  Hartmann 2001), slow at today's flux and fast under the late heavy
  bombardment. Sublimation and impact gardening leave the rock of an old
  ice-rock crust as a lag and the ice as bright frost on cold crests
  (Spencer 1987; Moore et al. 1999, Callisto; from memory). `0.88` is a
  calibration on Callisto (about 0.2), whose rock share, 0.5, is the
  reference under which the lag covers less in proportion (an old crust
  of 94 % ice keeps 0.77); `0.004` is a calibration on Ganymede (its
  2 Gyr renewed terrain keeps 70 % of its ice uncovered).
- **A plume's fallout** keeps `0.6 × v` of the ice under fresh frost,
  `v` the strength the body vents with (`geysers::vent_strength`): 0
  unless it passes the conditions of its geysers other than the crack,
  then rising linearly with its polar shell's overburden from 0 at the
  crack threshold to 1 at 0.6 of it (the model's Enceladus carries 0.4
  to 0.55 of it). 0.6 is a calibration on Enceladus, 0.81 (Howett et al.
  2010; the plume and E-ring fallout: Verbiscer et al. 2007, Kempf et
  al. 2010; from memory). The fallout brightens and cools the surface,
  which thickens the shell and weakens the vent: the strength the
  climate takes is the fixed point of that decreasing map, bisected in
  12 steps (`planet::bare`). It is 0 exactly where the stored geysers
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
test asserts 0.08 to 0.35.

Census, seeds 1 to 40 (min, median, max):

| Body | Bond albedo | T, K | Observed |
|---|---|---|---|
| Enceladus | 0.769, 0.782, 0.787 | 60.3, 62.2, 63.9 | 0.81 ± 0.04 (Howett et al. 2010) |
| Europa | 0.587, 0.623, 0.660 | 91.9, 95.8, 100.1 | about 0.62 to 0.68 |
| Ganymede | 0.451, 0.455, 0.458 | 102.2, 105.1, 107.7 | about 0.4 |
| Callisto | 0.192, 0.203, 0.250 | 110.8, 115.3, 118.6 | about 0.2 |

Enceladus took the regolith's 0.11 before (85 to 89 K): its 0.027
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
past 38 K. One value serves α- and β-N₂.

**Branches and snowballs** (`climate::climate`, `snowball_chance`). A body
that keeps water has two climates when the ice-albedo feedback makes it
bistable: the temperate one, started from a warm planet with no ice, and
the frozen one, started from a body covered in ice with the CO₂ the
temperate one held when it froze. Weathering stops under the ice, so
volcanic CO₂ builds up in the frozen air until it thaws the planet (the
snowball Earth of Hoffman et al. 2017, doi:10.1126/sciadv.1600983); the
frozen branch's CO₂ is the amount that would be there, half way
(`SNOWBALL_BUILT_UP`) from the CO₂ it froze with to the CO₂ that thaws it.
Where both exist the planet is a snowball (`Climate::snowball`) when its
formation draw `PlanetInputs::snowball_draw`, uniform in `[0, 1]`, is under
`d / (d + 0.8 bar × v / v⊕)`: the share of its history it spends frozen,
with `d` the CO₂ that thaws it and `v / v⊕` its volcanism against Earth's
(0.8 bar gives Earth, which spent about a tenth of its history frozen, a
chance of 0.1). Where no CO₂ within the planet's inventory and its frost
cap thaws it (the chance is then 1, from about 1.6 AU for an Earth-like
planet round the Sun), the frozen state is for good: the planet is
frozen but not a snowball, and its card says so (the `frozen` note). So snowballs are rare near S = 1, likelier towards the
outer edge of the habitable zone and on planets that outgas little, and
`--snowball yes|no` pins the state. A snowball is drawn with the `frozen`
look and keeps its relief and coastlines; no sulphuric cloud forms in its
cold air. On the sweep of an Earth-like planet round the Sun the warm
branch runs from the runaway at about 0.95 AU to 1.6 AU (288 K, the
thermostat's, with 1.6 bar of CO₂ at 1.6 AU) and ends near 1.67 AU, beyond
which the frozen branch is the only one (187 K at 1.8 AU).

**Sulphur and SO₂.** Outgassed sulphur stays in the air as SO₂ only in
share `SULPHUR_IN_AIR` (5e-5); over liquid water or with free oxygen above
0.01 bar it is washed out to sulphate in about a day and a half
(`WET_SO2_SHARE`, 4e-9 of the dry lifetime), and in a dry, oxygen-free air
its lifetime grows with the column (`DRY_COLUMN_BAR`, 10 bar). That gives
Earth under 1e-6 bar of SO₂ (2e-11), Venus about 1e-2 bar (150 ppm of
92 bar) and Mars under its 0.3 ppb limit. The constants are calibrated on
Venus and Mars together; the lifetimes are from memory and marked as such
in `atmosphere.rs`.

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
- `hydrocarbons` between the freezing point of the sea
  (`sea_freezes_k`) and ethane's
  boiling point at the surface pressure (Clausius–Clapeyron from 184.6 K
  at 1 atm with 14.7 kJ/mol, capped at its critical 305 K; NIST values
  from memory, not checked), where there is enough methane to condense:
  the air's methane reaches the vapour of a Titan sea at the surface
  temperature, half of pure methane's saturation pressure
  (`methane_sea_vapour_bar`), so that a trace of methane makes no lake
  and none forms past methane's critical point (190.56 K). The sea
  freezes at methane's freezing point lowered by the nitrogen of the air
  dissolved in it and the ethane methane's photolysis makes
  (`methane_freezes_k`: about 76 K with no N₂, about 69 K under Titan's
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
  triple point it is a frost (`climate::frost`). The look draws no
  nitrogen sea: the map of such a body is its icy surface;
- `none` otherwise.

Calibration targets, through the whole chain from the reference bodies'
inputs round the Sun at 4.6 Gyr:

| Body | Result | Target | Observed |
|---|---|---|---|
| Earth | 288 K, water, regulated, 3e-4 bar of CO₂ | 288 ± 6 K, water, regulated, under 0.01 bar of CO₂ | 288 K, 280 ppm before industry |
| Venus | 737 K, none | 737 ± 50 K, none | 737 K |
| Mars | 209 K, none | 210 ± 12 K, none | about 210 K |
| Titan | 96 K, hydrocarbons | 94 ± 8 K, hydrocarbons | 94 K, methane and ethane lakes |
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
