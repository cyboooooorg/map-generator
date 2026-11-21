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
2012 astronomical unit and the IAU 2015 Resolution B3 nominal values (Prša
et al. 2016, AJ 152, 41) as `Definition:`; the CODATA 2018 gravitational and
Stefan-Boltzmann constants (Tiesinga et al. 2021, Rev. Mod. Phys. 93,
025010) as `Source:`; the solar and Earth masses as the nominal mass
parameter divided by `GRAVITATIONAL_CONSTANT`.

`EARTH_MEAN_RADIUS` is the IUGG mean radius R1 of GRS80 (6 371 008.7714 m,
Moritz 2000, J. Geodesy 74, 128). `EarthRadius` stays the IAU equatorial
radius.

## Equations of state

`science::eos` gives density against pressure at the reference
temperature (or, for water, along a path through the phase diagram):
`Eos` (`Vinet`, `BirchMurnaghan3`, `BirchMurnaghan2`) with
`pressure(rho)` and `density(p)` (safeguarded Newton, bracketed between
`rho0` and `100 rho0`, or the pressure maximum of a third-order form with
`K0' < 4`); `Mix` (mass shares added by volume at equal pressure);
`MaterialEos`, phases ordered by pressure, the last one unbounded, built by
`MaterialEos::new` so that a later task can build one at run time. The
materials are `core_alloy()` (Valencia et al. 2006 fits to PREM, liquid
outer and solid inner core), `mantle()` (50/50 forsterite and enstatite
below 23.83 GPa; 90 % (Mg0.88 Fe0.12) SiO3 bridgmanite and 10 % periclase
above) and `ice_vii()` (Seager et al. 2007 Vinet fit). Values are in the
study `docs/superpowers/studies/2026-11-21-planet-interior/equations-of-state.md`.
The mantle at the 670 km boundary is 7.5 % below PREM above it and 3.5 %
above PREM below it (no mineral phase transitions, no temperature).

Water below 2.2 GPa has two materials, since the boundaries between ices
III, V and VI away from the melting curve were not found in a source
(study section 2.7):

- `water(t)`, for 273.31 K <= t < 355 K: liquid water at `t` (IAPWS-95)
  below the melting pressure of ice VI (IAPWS 2011), ice VI up to the
  ice VI / ice VII boundary (AQUA, Haldemann et al. 2020), ice VII above.
  Outside that range it returns `EosError::TemperatureOutOfRange`.
- `water_on_melting_curve()`, for cold icy bodies (Zeng and Sasselov 2013):
  ice Ih at its melting temperature (Feistel and Wagner 2006), then ices
  III, V, VI and VII, switching at the IAPWS 2011 triple points with the
  liquid (208.566, 350.1, 632.4 and 2216 MPa).

Ices III, V and VI are the Journaux et al. 2020 cold (0 K) curves, so
their densities are 1 to 4 % above those at the real temperature.
`science::water` holds the laws themselves: `liquid_pressure` (IAPWS-95),
`ice_ih_density` (IAPWS R10-06), the melting pressures of ices Ih and VI
with the inverse for ice Ih, the ice VI / ice VII boundary and the triple
points; each is checked against its release's verification table.

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
