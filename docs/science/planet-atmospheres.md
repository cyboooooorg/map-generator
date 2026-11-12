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

The star emits `L_XUV / L_bol = 10^-3` while saturated (Wright et al.
2011, doi:10.1088/0004-637X/743/1/48), for a time `t_sat = 0.1 Gyr ·
M^-2` (M in solar masses; 0.1 Gyr for the Sun from Jackson, Davis &
Wheatley 2012, doi:10.1111/j.1365-2966.2012.21384.x, the mass exponent is
a calibration giving 1.1 Gyr at 0.3 M☉ and 2.5 Gyr at 0.2 M☉), then
decays as `t^-1.23` (Ribas et al. 2005, doi:10.1086/427977). The
integral over the age is analytic and is multiplied by the insolation.
The luminosity is taken constant over the age.

Calibration targets: Earth is 1.0 (absolute value
`EARTH_XUV_FLUENCE_J_M2` = 1.5e16 J/m²); 5 to 10 Gyr adds under 30 %;
a 0.2 M☉ star at Earth's insolation delivers more than 10 times Earth's
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
the Sun is stripped and its radius falls under 1.8 R⊕; Jupiter at
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
`kept = max(0, water - 9 H lost)`. Without a runaway the planet keeps its
water. `oceans` counts it in units of 1.4e21 kg.

**Regime.** A labelled calibration, since the literature disagrees on the
mass dependence (see [plate tectonics](plate-tectonics.md)):

- heat pipe above 2 W/m² of internal heat (Io; Moore & Webb 2013,
  doi:10.1038/nature12424, value not checked against the paper; Earth's
  mean is 0.087 W/m²);
- plates from 0.3 M⊕ (`geology::PLATES_MIN_MASS_EARTH`, shared with the
  generator) when water is kept;
- episodic when the mass allows plates but the water is gone (Venus);
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
