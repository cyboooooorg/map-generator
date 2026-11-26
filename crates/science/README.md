# `science`: laws and typed units

Laws of physics and typed units, shared by both engines: radiative balance,
escape, interiors, orbital mechanics, stellar evolution, and so on. Functions
take and return typed quantities. Anything that knows a body, a system, a seed
or an identifier lives in an engine; nothing here draws at random.

Every constant has a doc comment starting with `Source:` (a citation),
`Calibration:` (what it was fitted against, and how) or `Definition:` (an exact
definition or a unit conversion); a test fails otherwise. Physics is ported from
the tag `legacy-final` with its citation and tests, never rewritten from memory.

## Units

`science::units` holds one newtype per unit, over a public `f64` (`Kelvin`,
`Metre`, `AstronomicalUnit`, `SolarMass`, `Gyr`, ...). Each serialises as a bare
number, adds and subtracts within its own type, multiplies and divides by an
`f64`, and carries its `SYMBOL`. `Fraction` (0 to 1 inclusive) keeps its field
private and rejects out-of-range values, also when deserialised. A field stores
the unit the domain thinks in (au for an orbit, solar masses for a star); `From`
converts both ways between such a unit and its SI one. `EarthRadius` is the IAU
nominal equatorial radius (6378.1 km): a law fitted against the mean radius
(6371 km) converts explicitly.

## Constants

`science::constants` holds the sourced values the conversions use: the IAU 2012
astronomical unit and IAU 2015 B3 nominal values (Prša et al. 2016) as
`Definition:`; the CODATA 2018 constants (Tiesinga et al. 2021) as `Source:`;
the solar and Earth masses as the nominal mass parameter over `G`.
`EARTH_MEAN_RADIUS` is the IUGG mean radius R1 of GRS80 (6 371 008.7714 m,
Moritz 2000); `EarthRadius` stays the IAU equatorial.

## Equations of state

`science::eos` gives density against pressure at the reference temperature (for
water, along a path through the phase diagram): `Eos` (`Vinet`,
`BirchMurnaghan3`, `BirchMurnaghan2`) with `pressure(rho)` and `density(p)`
(safeguarded Newton, bracketed between `rho0` and `100 rho0`, or the pressure
maximum of a third-order form with `K0' < 4`); `Mix` (mass shares added by
volume at equal pressure); `MaterialEos`, phases ordered by pressure, the last
unbounded, built by `MaterialEos::new` so that a run-time material fits. The
materials are `core_alloy()` (Valencia et al. 2006), `mantle()` (upper and
lower, split at 23.83 GPa), `ice_vii()` (Seager et al. 2007), and for water:

- `water(t)`, for 273.31 K <= t < 355 K: liquid (IAPWS-95), ice VI, ice VII
  along the isotherm. Outside the range, and in the last 0.1 mK below
  355 K where ice VI has no range: `EosError::TemperatureOutOfRange`.
- `water_on_melting_curve()`, for colder bodies (Zeng and Sasselov 2013):
  ice Ih, III, V, VI, VII, switching at the IAPWS 2011 triple points.

`science::water` holds the laws (`liquid_pressure`, `ice_ih_density`, the melting,
saturation and sublimation pressures, the ice VI / VII boundary), each checked
against its IAPWS table; values and ranges: [What are planets made of?](../../docs/science/planet-interiors.md#from-layers-to-radius).
Limits: the laws are cold (no thermal pressure), the mantle has no
transition-zone phases (-7.5 % / +3.5 % at 23.83 GPa), ices III, V and VI are
0 K curves, and water has nothing above 355 K.

## Hydrostatic structure

`science::structure::integrate(layers, surface_pressure)` builds a planet in
hydrostatic equilibrium from its layers, centre out (`LayerSpec`: a mass and a
borrowed `MaterialEos`; zero-mass layers are skipped). It returns a `Structure`:
the surface radius, the normalised moment of inertia `C/MR^2`, the `Shell`s (one
per layer and phase present, so a phase change inside a layer is a shell
boundary) and the `Sample`s (radius, enclosed mass, density, pressure at the
centre and at the end of every step). Errors: `NoMass`, `NotFinite`,
`NoConvergence`, `Eos`.

The method is the planet-interior study's prototype (`solve.py`): fourth order
Runge-Kutta in `t = (m/M)^(1/3)`, shooting on the central pressure (Brent, 1e-10
relative); a phase transition ends a step exactly. `STEPS_PER_LAYER` (20, a
calibration) keeps R and C/MR^2 within 1e-4 of the converged answer. Below zero
pressure the density is the zero-pressure one.

```rust
use science::{constants::EARTH_MASS, eos::{core_alloy, mantle}, structure::{LayerSpec, integrate}, units::Pascal};

// Earth: core mass fraction 0.325 (PREM), 1 bar at the surface.
let (core, rock) = (core_alloy(), mantle());
let layers = [
    LayerSpec { mass: EARTH_MASS * 0.325, material: &core },
    LayerSpec { mass: EARTH_MASS * 0.675, material: &rock },
];
let earth = integrate(&layers, Pascal(1e5)).expect("Earth integrates");
println!("R = {} m, C/MR^2 = {}", earth.radius.0, earth.moment_of_inertia);
```

Against measured values (table on the page above): Earth R -0.29 %, C/MR^2
-0.24 %; the other bodies within 2 %, Titan's C/MR^2 not reproduced. The core
alloy (PREM, 364 GPa) is used to 7.85 TPa: converged, not validated.

## Internal heat

`science::heat`: `radiogenic`, `secular` and `tidal` power, and
`k2_over_q(TidalKind)` (rocky, icy, enveloped; the system engine picks the
kind). Radiogenic power comes from the abundances of U, Th and K and the Dye
2012 half-lives and heats (K-40's half-life is DDEP 2025's, which the air's
argon shares, `potassium_40_decayed`, and potassium's heat per kilogram is
derived from DDEP 2025's decay data); secular cooling scales Earth's 26.4 TW
by the silicate mass;
tidal heat is Peale, Cassen and Reynolds (1979). The `Calibration:` constants (U
and Th abundances, cooling time and mass exponent, rocky and icy k2/Q) say what
was searched and not found; the page [The heat
inside](../../docs/science/planet-interiors.md#the-heat-inside) lists them.

## Stars, light and orbits

`science::stellar`: `life` (ZAMS age, TAMS or `Beyond(20 Gyr)`), `main_sequence`
(R, L, T_eff) and `profile` (layers; ρ, P, T, fusion share) of a star of 0.08-8
M☉ and [Fe/H] −1 to +0.5, from the tables of `data/`; see
[STELLAR.md](data/STELLAR.md#reading-the-tables). `science::xuv` (`history`): a
star's XUV and Lyman α history; see [XUV
fluence](../../docs/science/planet-atmospheres.md#xuv-fluence).
`science::orbit`: Kepler, flux, ice line, Roche period, Hill spacing floors.
`science::outgassing` also makes the air's argon (`radiogenic_argon`,
`argon_degassed_share`).
`science::{escape, dynamo, outgassing, volatiles, habitable, tectonics,
photochemistry, snowball}`: [planet atmospheres](../../docs/science/planet-atmospheres.md).

## Giants and envelopes

`science::giants`: `giant` (uninflated radius at 1 bar, internal luminosity)
and `core_bounds` read the Fortney et al. 2007 tables; `ice_rock_core_radius`
is their Eq. 7. `science::envelope`: `envelope_thickness` is Lopez & Fortney
2014 (1 to 20 M⊕, 1e-4 to 0.2 of H/He, 20 mbar) minus their core; the 1 M⊕ and
1e-4 rules are calibrated. `science::hydrogen`: `polytrope_radius_at` places
the 100 GPa metallic boundary of a giant. `science::steam`: a runaway's
steam envelope, Turbet et al. 2020's Eqs. 2 and 3 (`envelope`,
`base_pressure`, `steam_temperature`; the thickness cap is calibrated),
and the steam world line (`hides_surface`). Data: [GIANTS.md](data/GIANTS.md);
laws, tests and calibrations: [Mini-Neptunes](../../docs/science/planet-interiors.md#mini-neptunes-and-their-envelopes),
[Giants](../../docs/science/planet-interiors.md#giants-and-the-metallic-hydrogen-boundary),
[Steam](../../docs/science/planet-atmospheres.md#boiling-and-the-steam-envelope-of-the-rewrite-study-26).

## Relief

`science::relief`: a rocky body's height. `spectrum` (Hurst exponent, Haar
amplitude per province, gravity's relief scale), `crust` (Airy), `plates`
(boundary profiles, ridge depth), `highlands`, `volcanoes`. Values: [relief
study](../../docs/superpowers/studies/2026-11-22-relief/README.md). Laws:
[mountains and gravity](../../docs/science/mountains-and-gravity.md),
[plate tectonics](../../docs/science/plate-tectonics.md), [volcanoes](../../docs/science/volcanoes.md).

## Surface

`science::surface`: `cover` and `ground_albedo` (ocean, land, ice, frost),
`RELATIVE_HUMIDITY`, and `liquid`, `frost`, `melting` (the freezing of methane
and nitrogen seas included); `surface::condense`: `condense` keeps in the air
what a gas's saturation allows at a temperature (nitrogen, methane, then
argon) and
returns the condensate. Values:
[planet atmospheres](../../docs/science/planet-atmospheres.md#climate-and-surface-liquid).

## Radiative balance

`science::balance`: the radiative balance of one column (`surface_temperature`
and its root finders), the moist plateau of Earth's air and the runaway
albedo. `science::snowball`: `SNOWBALL_WARM_BAR` and the snowball chance. The
solver that uses them is `system::planet::climate`.

## Radiation

`science::radiation::column`: `outgoing_longwave` (a column's infrared: lines
of `data/opacity.bin`, continua, clouds; the caller owns the `ColumnCache`) and
`skin_temperature`; `radiation::clouds::cloud_cover`; `science::volatiles`: CO₂,
methane, nitrogen and argon saturation. Shortwave: `albedo::shortwave_budget`
(Bond albedo and absorbed light, for a star's temperature and metallicity; the
caller owns the `ShortwaveCache`), `starlight` (the star's spectrum, from
`data/stellar-spectra.tsv`), `shortwave` (`data/shortwave.bin`), `ozone`
(`ozone_column`, an oxygen air's ozone under its star, and its
cross-sections from `data/ozone.tsv`), `rayleigh`,
`veils`, `methane::stratospheric_share`.
`radiation::gases::PartialPressures` holds the eight gases of a column (N₂,
CO₂, H₂O, CH₄, NH₃, SO₂, O₂, Ar); argon absorbs nothing, weighs, scatters,
has a monatomic heat capacity and broadens the lines as air does.
See [OPACITY.md](data/OPACITY.md), [SPECTRA.md](data/SPECTRA.md),
[RFMIP.md](data/RFMIP.md) (the field the shortwave is tested on), [radiative
transfer](../../docs/science/radiative-transfer.md), [stellar
spectra](../../docs/science/stellar-spectra.md).

## Dependencies

`serde` (units serialise as bare numbers), and `serde_json` in tests. Nothing
else. The engines, `catalogue`, `adapters`, `cli` and `regression` may depend
on it; `tools/guardrails` checks both. Design: [Architecture > Science](../../docs/superpowers/specs/2026-11-20-architecture-rewrite-design.md).
