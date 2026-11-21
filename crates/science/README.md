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
