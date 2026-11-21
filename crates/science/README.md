# `science`: laws and typed units

Laws of physics and typed units, shared by both engines: radiative balance,
escape, interiors, orbital mechanics, stellar evolution, and so on.
Functions take and return typed quantities. Anything that knows a body, a
system, a seed or an identifier lives in an engine, and nothing here draws
at random.

Every constant has a doc comment starting with `Source:` (a citation),
`Calibration:` (what it was fitted against, and how) or `Definition:` (an
exact definition or a unit conversion). A test fails otherwise. Physics is
ported from the tag `legacy-final` with its citation and its tests, never
rewritten from memory.

## Units

`science::units` holds one newtype per unit, over a public `f64`
(`Kelvin`, `Metre`, `AstronomicalUnit`, `SolarMass`, `Gyr`, ...). Each
serialises as a bare number, adds and subtracts within its own type,
multiplies and divides by an `f64`, and carries its `SYMBOL`. `Fraction`
(0 to 1 inclusive) keeps its field private and rejects out-of-range values,
also when deserialised.

A field stores the unit the domain thinks in (au for an orbit, solar masses
for a star); conversions go through `From`, both ways, for the pairs
`AstronomicalUnit`/`Metre`, `SolarMass`/`Kilogram`, `EarthMass`/`Kilogram`,
`SolarRadius`/`Metre`, `EarthRadius`/`Metre`, `SolarLuminosity`/`Watt`,
`EarthFlux`/`WattPerSquareMetre`, `Bar`/`Pascal`, `Day`/`Second`,
`Year`/`Second`, `Gyr`/`Year` and `Degree`/`Radian`. `EarthRadius` is the
IAU nominal equatorial radius (6378.1 km): a law fitted against the mean
radius (6371 km) converts explicitly.

## Constants

`science::constants` holds the sourced values the conversions use: the IAU
2012 astronomical unit and IAU 2015 B3 nominal values (Prša et al. 2016, AJ
152, 41) as `Definition:`; the CODATA 2018 constants (Tiesinga et al. 2021)
as `Source:`; the solar and Earth masses as the nominal mass parameter over
`GRAVITATIONAL_CONSTANT`. `EARTH_MEAN_RADIUS` is the IUGG mean radius R1 of
GRS80 (6 371 008.7714 m, Moritz 2000); `EarthRadius` stays the IAU equatorial.

## Equations of state

`science::eos` gives density against pressure at the reference
temperature (or, for water, along a path through the phase diagram):
`Eos` (`Vinet`, `BirchMurnaghan3`, `BirchMurnaghan2`) with
`pressure(rho)` and `density(p)` (safeguarded Newton, bracketed between
`rho0` and `100 rho0`, or the pressure maximum of a third-order form with
`K0' < 4`); `Mix` (mass shares added by volume at equal pressure);
`MaterialEos`, phases ordered by pressure, the last one unbounded, built by
`MaterialEos::new` so that a run-time material fits. The materials are
`core_alloy()` (Valencia et al. 2006), `mantle()` (upper and lower, split at
23.83 GPa), `ice_vii()` (Seager et al. 2007), and for water:

- `water(t)`, for 273.31 K <= t < 355 K: liquid (IAPWS-95), ice VI, ice VII
  along the isotherm. Outside the range, and in the last 0.1 mK below
  355 K where ice VI has no range: `EosError::TemperatureOutOfRange`.
- `water_on_melting_curve()`, for colder bodies (Zeng and Sasselov 2013):
  ice Ih, III, V, VI, VII, switching at the IAPWS 2011 triple points.

`science::water` holds the laws (`liquid_pressure`, `ice_ih_density`, the
melting pressures, the ice VI / ice VII boundary), each checked against its
IAPWS verification table. Every value, its source and its fitted range are
in the page [What are planets made of?](../../docs/science/planet-interiors.md#from-layers-to-radius).
Limits to know: the laws are cold (no thermal pressure), the mantle has no
transition-zone phases (-7.5 % / +3.5 % against PREM at 23.83 GPa), ices III,
V and VI are 0 K curves, and water has nothing above 355 K.

## Hydrostatic structure

`science::structure::integrate(layers, surface_pressure)` builds a planet
in hydrostatic equilibrium from its layers, centre out (`LayerSpec`: a mass
and a borrowed `MaterialEos`; zero-mass layers are skipped). It returns a
`Structure`: the surface radius, the normalised moment of inertia
`C/MR^2`, the `Shell`s (one per layer and phase actually present, so a
phase change inside a layer is a shell boundary) and the `Sample`s (radius,
enclosed mass, density, pressure at the centre and at the end of every
step). Errors: `NoMass`, `NotFinite`, `NoConvergence`, `Eos`.

The method is the planet-interior study's prototype (`solve.py`): fourth
order Runge-Kutta in `t = (m/M)^(1/3)`, state `(r, P, I)`, shooting on the central
pressure (Brent, 1e-10 relative); a phase transition ends a step exactly.
`STEPS_PER_LAYER` (20, a calibration) keeps R and C/MR^2 within 1e-4 of the
converged answer. Below zero pressure the density is the zero-pressure one.

```rust
use science::constants::EARTH_MASS;
use science::eos::{core_alloy, mantle};
use science::structure::{LayerSpec, integrate};
use science::units::Pascal;

// Earth: core mass fraction 0.325 (PREM), 1 bar at the surface.
let (core, rock) = (core_alloy(), mantle());
let earth = integrate(
    &[
        LayerSpec { mass: EARTH_MASS * 0.325, material: &core },
        LayerSpec { mass: EARTH_MASS * 0.675, material: &rock },
    ],
    Pascal(1e5),
)
.expect("Earth integrates");
println!("R = {} m, C/MR^2 = {}", earth.radius.0, earth.moment_of_inertia);
```

Against measured values (tests of `structure.rs`; the table is on the
page above): Earth R -0.29 %, C/MR^2 -0.24 %; Venus, Mars, Mercury, the Moon
and Ganymede within 2 %; super-Earths of 5 Earth masses within 2 % of Zeng
et al. (1.1 % in the prototype). Titan's C/MR^2 is not reproduced (partial
differentiation). The core alloy is fitted to PREM (364 GPa) and used to
7.85 TPa at 20 Earth masses: that converges, but is not validated.

## Internal heat

`science::heat`: `radiogenic`, `secular` and `tidal` power, and
`k2_over_q(TidalKind)` (rocky, icy, enveloped; the system engine picks the
kind). Radiogenic power comes from the abundances of U, Th and K and the
Dye 2012 half-lives and heats; secular cooling scales Earth's 26.5 TW by the
silicate mass; tidal heat is Peale, Cassen and Reynolds (1979). The
`Calibration:` constants (U and Th abundances, cooling time and its mass
exponent, rocky and icy k2/Q) say what was searched and not found; the page
[The heat inside](../../docs/science/planet-interiors.md#the-heat-inside)
lists them under "Where it departs from the science".

## Main-sequence stars

`science::stellar`: `life` (ZAMS age, TAMS or `Beyond(20 Gyr)`),
`main_sequence` (R, L, T_eff) and `profile` (layers; ρ, P, T, fusion share)
of a star of 0.08-8 M☉ and [Fe/H] −1 to +0.5, from the tables of `data/`;
see [STELLAR.md](data/STELLAR.md#reading-the-tables).

## Giant and mini-Neptune radii

`data/` also holds the radius tables of Fortney et al. 2007 (giants,
1 bar) and Lopez & Fortney 2014 (planets of 1 to 20 M⊕ with an envelope,
20 mbar), with their provenance and validity limits:
[GIANTS.md](data/GIANTS.md).

## High-energy light

`science::xuv` (`history`): a star's XUV and Lyman α history; see
[XUV fluence](../../docs/science/planet-atmospheres.md#xuv-fluence).

## Ports

None: `science` is a library of pure functions. The laws arrive with the
pull requests of slice 1 that use them (#602).

## Dependencies

`serde` (units serialise as bare numbers), and `serde_json` in tests. Nothing
else: `tools/guardrails` lists the allowed external crates. The engines,
`catalogue`, `adapters`, `cli` and `regression` may depend on it.

The direction of dependencies is checked by `tools/guardrails`
(`mise run test:tier1`). See the [architecture rewrite design](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md),
section Architecture > Science.
