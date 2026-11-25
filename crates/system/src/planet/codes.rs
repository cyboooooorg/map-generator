//! The cause codes of the planet's composition, interior and heat:
//! dotted lowercase, one constant per node, read with `Code::of`.

/// Core mass fraction of the planet without its water, drawn.
pub(crate) const CORE_MASS_FRACTION: &str = "planet.core-mass-fraction";
/// Whether the planet formed at or beyond the ice line, from the
/// semi-major axis and the ice line.
pub(crate) const FORMED_BEYOND_ICE_LINE: &str = "planet.formed-beyond-ice-line";
/// Whether the planet migrated in from beyond the ice line, drawn.
pub(crate) const MIGRATED: &str = "planet.migrated";
/// Water mass fraction of the planet without its envelope (bulk water),
/// drawn: an icy fraction for a water world, a dry one otherwise. A planet
/// with an envelope fraction `f` holds `w (1 - f) M` of water.
pub(crate) const WATER_FRACTION: &str = "planet.water-fraction";
/// Radiogenic abundance of the planet in dex of Earth's, drawn.
pub(crate) const RADIOGENIC_ABUNDANCE: &str = "planet.radiogenic-abundance";
/// Primordial hydrogen-helium envelope mass fraction of the planet: drawn
/// by mass regime below 20 M_E; a gas giant's is derived from its mass and
/// its heavy-element mass, `1 - Mz / M`.
pub(crate) const ENVELOPE_FRACTION: &str = "planet.envelope-fraction";
/// Total heavy-element mass of a gas giant, drawn around the Thorngren et
/// al. 2016 fit within the tabulated cores.
pub(crate) const HEAVY_MASS: &str = "planet.heavy-element-mass";
/// Temperature of a water world's water: the equilibrium temperature of
/// the trial, until the climate of the planet interior plan's second pull
/// request gives the surface temperature.
pub(crate) const WATER_TEMPERATURE: &str = "planet.water-temperature";
/// Mass of the iron core, `cmf (1 - w) M`.
pub(crate) const CORE_MASS: &str = "planet.core-mass";
/// Mass of the silicate mantle: `(1 - cmf)(1 - w) M` for a water world,
/// plus the water `w M` for a dry planet, whose water is bound in it.
pub(crate) const MANTLE_MASS: &str = "planet.mantle-mass";
/// Mass of a water world's water layer, `w M`.
pub(crate) const WATER_LAYER_MASS: &str = "planet.water-layer-mass";
/// Mass of a hot water world's water, `w (1 - f) M` (`w M` without an
/// envelope), recorded apart from the condensed planet: too hot for the water laws (355 K and above), it
/// becomes the steam envelope of the planet interior plan's second pull
/// request.
pub(crate) const WATER_BEYOND_LAW: &str = "planet.water-beyond-law";
/// Dry mass of a planet whose water is beyond the water laws: the mass of
/// the condensed planet that its bulk describes, `(1 - w) M` without an
/// envelope, the core's dry mass plus the envelope for a mini-Neptune.
pub(crate) const DRY_MASS: &str = "planet.dry-mass";
/// Mass of an enveloped planet without its envelope, `(1 - f) M`: the
/// condensed core a mini-Neptune's structure is built for, its water
/// included as a share of it.
pub(crate) const MASS_WITHOUT_ENVELOPE: &str = "planet.mass-without-envelope";
/// Radius of a mini-Neptune's core, the planet without its envelope, from
/// its integrated structure.
pub(crate) const RADIUS_WITHOUT_ENVELOPE: &str = "planet.radius-without-envelope";
/// The age the giant tables were read at, when it differs from the age
/// asked: clamped to the tables' 1 Myr to 10 Gyr, or an older age where a
/// corner of the interpolation has no row. Derived from the age asked.
pub(crate) const TABLE_AGE_USED: &str = "planet.table-age-used";
/// The mass the giant tables were read at, when it differs from the mass
/// asked: their top mass, 3 593.81 M_E, for a heavier giant. Derived from
/// the mass asked.
pub(crate) const TABLE_MASS_USED: &str = "planet.table-mass-used";
/// Bolometric flux the planet receives from all its stars, the sum of
/// every source's flux: the flux the envelope and giant tables read.
pub(crate) const BOLOMETRIC_FLUX: &str = "planet.bolometric-flux";
/// Mass of a mini-Neptune's hydrogen-helium envelope, `f M`.
pub(crate) const ENVELOPE_MASS: &str = "planet.envelope.mass";
/// Mass of the envelope lost to the XUV light of the stars over the
/// planet's age, energy-limited (Owen & Wu 2017, eq. 19) on the formed
/// body's radius and mass, capped by the envelope `f M`. Recorded only
/// when it is not zero.
pub(crate) const ENVELOPE_LOST: &str = "planet.envelope.lost";
/// Envelope mass fraction of the planet once the escape has taken its
/// share: `(f M - lost) / (M - lost)` for a mini-Neptune, 0 below
/// `science::escape::MIN_ENVELOPE`; `1 - Mz / M'` for a giant.
pub(crate) const ENVELOPE_KEPT_FRACTION: &str = "planet.envelope.kept-fraction";
/// Mass of the planet once the escape has taken its share: `M - lost`, or
/// `M - f M` for a mini-Neptune stripped of its whole envelope.
pub(crate) const MASS_KEPT: &str = "planet.mass.kept";
/// Thickness of a mini-Neptune's envelope, read in the Lopez & Fortney 2014
/// tables, added to the radius of its core.
pub(crate) const ENVELOPE_THICKNESS: &str = "planet.envelope.thickness";
/// Radius of a giant's ice-rock core, Fortney et al. 2007 Eq. 7.
pub(crate) const GIANT_CORE_RADIUS: &str = "planet.giant.core-radius";
/// Internal luminosity of a giant, read with its radius in the Fortney et
/// al. 2007 tables.
pub(crate) const GIANT_LUMINOSITY: &str = "planet.giant.internal-luminosity";
/// Radius where a giant's hydrogen turns metallic: where the index-one
/// polytrope of the planet reaches the metallic pressure.
pub(crate) const METALLIC_RADIUS: &str = "planet.giant.metallic-radius";
/// The structure node: the central pressure that the integration finds
/// for the layers, which every radius and shell of the structure follows.
pub(crate) const CENTRAL_PRESSURE: &str = "planet.structure.central-pressure";
/// Radius of the planet: from the integrated structure (a rocky planet or
/// a mini-Neptune's core), the core plus the envelope's thickness (a
/// mini-Neptune), or the giant tables.
pub(crate) const RADIUS: &str = "planet.radius";
/// Surface gravity, `G M / R^2`.
pub(crate) const SURFACE_GRAVITY: &str = "planet.surface-gravity";
/// Mean density, `M / (4/3 pi R^3)`.
pub(crate) const MEAN_DENSITY: &str = "planet.mean-density";
/// Outer radius of a shell.
pub(crate) const LAYER_OUTER_RADIUS: &str = "planet.layer.outer-radius";
/// Mass of a shell.
pub(crate) const LAYER_MASS: &str = "planet.layer.mass";
/// Material of a shell.
pub(crate) const LAYER_MATERIAL: &str = "planet.layer.material";
/// State of the matter of a shell, set by the engine until the
/// temperatures of the planet interior plan's later pull requests: the
/// mantle and a giant's ice-rock core are solid, an envelope without a
/// metallic layer is gas. A giant's core is a calibration, a placeholder:
/// the state of a dilute ice-rock core at tens of Mbar is not modelled
/// until the temperature profile arrives (`enveloped::GIANT_CORE_STATE`).
pub(crate) const LAYER_STATE: &str = "planet.layer.state";
/// State of the matter of a shell read from the phase of its equation of
/// state, which pressure alone selects: the core's liquid outer and solid
/// inner phases switch at PREM's inner-core boundary pressure (the
/// Valencia et al. 2006 fit), and the water's liquid and ices at their
/// phase boundaries on its isotherm or melting curve, and a giant's
/// hydrogen liquid (metallic) below the metallic pressure and gas above
/// it. Not a thermal state:
/// the core's comes with the temperatures of a later pull request.
pub(crate) const LAYER_STATE_BY_PRESSURE: &str = "planet.layer.state-by-pressure";
/// Mass of a steam world with its steam envelope: the whole mass of the
/// body it was made from (the bulk's plus a water beyond the laws), which
/// the steam, the water the planet kept, makes up with its rock.
pub(crate) const MASS_WITH_STEAM: &str = "planet.mass.with-steam";
/// Temperature of a steam envelope, Turbet et al. 2020 Eq. 3
/// (`science::steam::envelope`), from the core's shells, the steam and
/// the fluxes of the sources.
pub(crate) const STEAM_TEMPERATURE: &str = "planet.steam.temperature";
/// Thickness of a steam envelope up to the transit radius, Turbet et al.
/// 2020 Eq. 2 (`science::steam::envelope`), from the same inputs.
pub(crate) const STEAM_THICKNESS: &str = "planet.steam.thickness";
/// Radius of a steam world: its core's radius plus the steam's thickness.
pub(crate) const RADIUS_STEAM: &str = "planet.radius.steam";
/// Radiogenic heating power.
pub(crate) const HEAT_RADIOGENIC: &str = "planet.heat.radiogenic";
/// Tidal heating power.
pub(crate) const HEAT_TIDAL: &str = "planet.heat.tidal";
/// Secular cooling power.
pub(crate) const HEAT_SECULAR: &str = "planet.heat.secular";
/// Scatter of the CO2 budget around its rule, dex (drawn).
pub(crate) const VOLATILE_SCATTER_CO2: &str = "planet.volatiles.scatter.co2";
/// Scatter of the N2 budget around its rule, dex (drawn).
pub(crate) const VOLATILE_SCATTER_N2: &str = "planet.volatiles.scatter.n2";
/// Scatter of the sulfur budget around its rule, dex (drawn).
pub(crate) const VOLATILE_SCATTER_S: &str = "planet.volatiles.scatter.s";
/// Scatter of the CH4 budget around its rule, dex (drawn).
pub(crate) const VOLATILE_SCATTER_CH4: &str = "planet.volatiles.scatter.ch4";
/// Scatter of the NH3 budget around its rule, dex (drawn).
pub(crate) const VOLATILE_SCATTER_NH3: &str = "planet.volatiles.scatter.nh3";
/// Whether the planet's water ran away by insolation: the sum over its
/// stars of each flux over the runaway greenhouse limit at that star's
/// effective temperature exceeds 1 (#592). Derived from every source's
/// flux and temperature.
pub(crate) const WATER_RUNAWAY: &str = "planet.water.runaway";
/// Water at the surface of a planet whose water is bound in its mantle:
/// `science::habitable::SURFACE_WATER_SHARE` of its bulk water
/// `w (1 - f) M`, derived from the mass and the two fractions. A water
/// world's surface water is its water layer (or its water beyond the
/// laws), already recorded: no node of this code then.
pub(crate) const WATER_SURFACE: &str = "planet.water.surface";
/// Water lost as steam in a runaway: 9 kg of water for each kg of hydrogen
/// the energy-limited escape takes at the whole mass (the bulk's plus a
/// water beyond the laws), capped by the surface water. Recorded only in
/// a runaway.
pub(crate) const WATER_STEAM_LOST: &str = "planet.water.steam-lost";
/// Water the planet keeps at its surface: its surface water, less the
/// steam lost in a runaway.
pub(crate) const WATER_INVENTORY: &str = "planet.water.inventory";
/// Whether the climate boiled the ocean: false, imposed, before the
/// climate (`water`); derived from the boil loop's verdict once settled
/// (`water::boiled`, `settled`).
pub(crate) const WATER_BOILED: &str = "planet.water.boiled";
/// Mass of the planet once the steam is gone: its mass less the surface
/// water lost, `M - (surface - kept)`; the mantle's water stays.
pub(crate) const MASS_AFTER_STEAM: &str = "planet.mass.after-steam";
/// Water mass fraction of the planet once the steam is gone: the water
/// left, `w M - (surface - kept)`, over the mass after the steam.
pub(crate) const WATER_KEPT_FRACTION: &str = "planet.water.kept-fraction";
/// Tectonic regime: heat pipe, stagnant lid, plates or episodic, from the
/// internal heat flux, the mass and the water kept.
pub(crate) const REGIME: &str = "planet.regime";
/// Volcanic intensity the snowball chance reads, from 0 to 1: the law of
/// `science::relief::volcanoes` at the internal heat flux, scaled by
/// `LID_SHARE / MOBILE_SHARE` under a stagnant lid. Derived from the
/// three heat sources, the radius and the regime.
pub(crate) const SURFACE_VOLCANISM: &str = "planet.surface.volcanism";
/// Cooling time of the iron core through the mantle, `ρ c r_c d / (3 k
/// Nu)`, from the core's mass and outer radius, the planet's radius and
/// the regime's Nusselt number (`science::dynamo::cooling_time`).
pub(crate) const CORE_COOLING_TIME: &str = "planet.core.cooling-time";
/// State of the iron core: none (no iron shell or no core mass), liquid,
/// or frozen past `science::dynamo::CORE_FROZEN` cooling times.
pub(crate) const CORE_STATE: &str = "planet.core.state";
/// Whether a dynamo runs in the core, from its cooling time, the age and
/// the regime (plates keep the thermal dynamo running).
pub(crate) const CORE_DYNAMO: &str = "planet.core.dynamo";
/// Surface equatorial dipole field of the core's dynamo; 0 T without one.
pub(crate) const CORE_FIELD: &str = "planet.core.field";
/// Midplane temperature of the disc the planet's ice formed in, from its
/// semi-major axis and its host's luminosity
/// (`science::volatiles::disc_temperature`).
pub(crate) const VOLATILE_DISC_TEMPERATURE: &str = "planet.volatiles.disc-temperature";
/// Whether the planet's ice trapped N2 when it formed: a disc colder than
/// `science::volatiles::NITROGEN_TRAPPING_TEMPERATURE`.
pub(crate) const VOLATILE_ICE_TRAPS_N2: &str = "planet.volatiles.ice-traps-n2";
/// Partial pressure of N2 the interior brought up: the inventory at
/// formation times its scatter (the ice's N2 only where it trapped some),
/// plus the N2 of the NH3 the interior decomposed
/// (`science::outgassing::decomposed`), times the regime's share, the
/// share the water lets through and, under plates, what subduction
/// leaves.
pub(crate) const AIR_OUTGASSED_N2: &str = "planet.air.outgassed.n2";
/// Partial pressure of CO2 the interior brought up (see
/// [`AIR_OUTGASSED_N2`]).
pub(crate) const AIR_OUTGASSED_CO2: &str = "planet.air.outgassed.co2";
/// Partial pressure of water vapour before the escape: a runaway's whole
/// water inventory, 0 bar otherwise (the oceans hold it).
pub(crate) const AIR_OUTGASSED_H2O: &str = "planet.air.outgassed.h2o";
/// Partial pressure of CH4 the interior brought up (see
/// [`AIR_OUTGASSED_N2`]).
pub(crate) const AIR_OUTGASSED_CH4: &str = "planet.air.outgassed.ch4";
/// Partial pressure of NH3 the interior brought up: the NH3 left once
/// the share the interior decomposes has come up as N2
/// (`science::outgassing::decomposed`), then as [`AIR_OUTGASSED_N2`].
pub(crate) const AIR_OUTGASSED_NH3: &str = "planet.air.outgassed.nh3";
/// Partial pressure of SO2 the interior brought up: its sulphur as SO2,
/// the share that stays in the air, then as [`AIR_OUTGASSED_N2`].
pub(crate) const AIR_OUTGASSED_SO2: &str = "planet.air.outgassed.so2";
/// Partial pressure of O2 outgassed: 0 bar, imposed (life makes it).
pub(crate) const AIR_OUTGASSED_O2: &str = "planet.air.outgassed.o2";
/// Share of its air the planet keeps on the cosmic shoreline, from its
/// escape velocity (the whole mass, the bulk's plus a water beyond the
/// laws, and the bulk radius), the XUV fluence of its stars and its
/// dynamo.
pub(crate) const AIR_SHORELINE_KEPT: &str = "planet.air.shoreline-kept";
/// Share of the SO2 the air keeps against its wash-out: wet or oxidising,
/// a day and a half's worth; dry, in proportion to its column.
pub(crate) const AIR_SO2_KEPT: &str = "planet.air.so2-kept";
/// Partial pressure of N2 the cosmic shoreline kept: the outgassed one
/// times the shoreline's share.
pub(crate) const AIR_KEPT_N2: &str = "planet.air.kept.n2";
/// Partial pressure of CO2 the cosmic shoreline kept (see [`AIR_KEPT_N2`]).
pub(crate) const AIR_KEPT_CO2: &str = "planet.air.kept.co2";
/// Partial pressure of water vapour the cosmic shoreline kept (see [`AIR_KEPT_N2`]).
pub(crate) const AIR_KEPT_H2O: &str = "planet.air.kept.h2o";
/// Partial pressure of CH4 the cosmic shoreline kept (see [`AIR_KEPT_N2`]).
pub(crate) const AIR_KEPT_CH4: &str = "planet.air.kept.ch4";
/// Partial pressure of NH3 the cosmic shoreline kept (see [`AIR_KEPT_N2`]).
pub(crate) const AIR_KEPT_NH3: &str = "planet.air.kept.nh3";
/// Partial pressure of SO2 in the air: the outgassed one times the
/// shoreline's share and the share the wash-out leaves; the air's, no
/// later step changing it.
pub(crate) const AIR_KEPT_SO2: &str = "planet.air.kept.so2";
/// Partial pressure of O2 the cosmic shoreline kept (see [`AIR_KEPT_N2`]).
pub(crate) const AIR_KEPT_O2: &str = "planet.air.kept.o2";
/// Partial pressure of CH4 the stars' ultraviolet leaves of the kept one
/// (`science::photochemistry::methane_left`), from the surface gravity,
/// each star's flux and history (its mass, the metallicity and the age).
pub(crate) const AIR_PHOTOLYSED_CH4: &str = "planet.air.photolysed.ch4";
/// Whether the air is reducing, Titan's: methane left by the photolysis
/// and an equilibrium temperature cold enough for it to survive
/// (`science::photochemistry::reducing`); always false when the chain
/// does not hold a reducing air's CO2 back. A reducing air outgasses no
/// CO2 ([`AIR_OUTGASSED_CO2`] is then 0 and derives from it).
pub(crate) const AIR_REDUCING: &str = "planet.air.reducing";
/// Partial pressure of CO2 the cold trap leaves as gas: the kept one,
/// capped by the CO2 frost pressure at the equilibrium temperature below
/// CO2's triple point (`science::volatiles::co2_frost_pressure`).
pub(crate) const AIR_TRAPPED_CO2: &str = "planet.air.trapped.co2";
/// Partial pressure of NH3 the cold trap leaves as gas (see
/// [`AIR_TRAPPED_CO2`], `science::volatiles::ammonia_frost_pressure`).
pub(crate) const AIR_TRAPPED_NH3: &str = "planet.air.trapped.nh3";
/// CO2 the cold trap froze out, bar: the photolysed CO2 (the photolysis
/// leaves it as kept) less the trapped one. Legacy books all of it as a
/// frost, which overstates it on small icy bodies (#590).
pub(crate) const AIR_FROZEN_CO2: &str = "planet.air.frozen-co2";
/// Partial pressure of N2 once the photochemistry has run: the kept N2
/// plus the N2 the trapped NH3 leaves once photolysed
/// (`science::photochemistry::nitrogen_from_ammonia`).
pub(crate) const AIR_PHOTOCHEMISTRY_N2: &str = "planet.air.photochemistry.n2";
/// Partial pressure of NH3 once the photochemistry has run: 0 bar, all of
/// the trapped NH3 photolysed to N2.
pub(crate) const AIR_PHOTOCHEMISTRY_NH3: &str = "planet.air.photochemistry.nh3";
/// Partial pressure of CH4 once the photochemistry has run: the photolysed
/// CH4 where it survives the OH of water vapour, under the equilibrium
/// temperature of `science::photochemistry::METHANE_SURVIVES`; 0 bar
/// above it.
pub(crate) const AIR_PHOTOCHEMISTRY_CH4: &str = "planet.air.photochemistry.ch4";
/// Surface pressure: the sum of the partial pressures.
pub(crate) const AIR_PRESSURE: &str = "planet.air.pressure";

/// The gases of the air, each with the code of its outgassed partial
/// pressure and of its partial pressure the shoreline kept, in the
/// contract's order.
pub(crate) const AIR_GASES: [(timeline::Gas, &str, &str); 7] = [
    (timeline::Gas::N2, AIR_OUTGASSED_N2, AIR_KEPT_N2),
    (timeline::Gas::Co2, AIR_OUTGASSED_CO2, AIR_KEPT_CO2),
    (timeline::Gas::H2o, AIR_OUTGASSED_H2O, AIR_KEPT_H2O),
    (timeline::Gas::Ch4, AIR_OUTGASSED_CH4, AIR_KEPT_CH4),
    (timeline::Gas::Nh3, AIR_OUTGASSED_NH3, AIR_KEPT_NH3),
    (timeline::Gas::So2, AIR_OUTGASSED_SO2, AIR_KEPT_SO2),
    (timeline::Gas::O2, AIR_OUTGASSED_O2, AIR_KEPT_O2),
];

/// Surface temperature the climate settles at, from the light of every
/// star, the air, the water, the heat, the body, the rotation and the
/// state.
pub(crate) const CLIMATE_SURFACE_TEMPERATURE: &str = "planet.climate.surface-temperature";
/// Equilibrium temperature of the light the surface took: from the Bond
/// albedo and the flux of every star.
pub(crate) const CLIMATE_EQUILIBRIUM: &str = "planet.climate.equilibrium";
/// Bond albedo the surface absorbed its light under, settled with the
/// temperature.
pub(crate) const CLIMATE_BOND_ALBEDO: &str = "planet.climate.bond-albedo";
/// Emissivity the surface radiated under (the frosts' on its ground),
/// settled with the temperature.
pub(crate) const CLIMATE_EMISSIVITY: &str = "planet.climate.emissivity";
/// Whether the carbonate thermostat holds the temperature.
pub(crate) const CLIMATE_REGULATED: &str = "planet.climate.regulated";
/// Whether the fixed points of the albedo and of the condensation
/// settled within the solver's tolerance.
pub(crate) const CLIMATE_CONVERGED: &str = "planet.climate.converged";
/// The state the climate is found in: temperate, snowball, frozen or
/// runaway. Derived from the inputs and, on a bistable body, from the
/// snowball chance and its draw.
pub(crate) const CLIMATE_STATE: &str = "planet.climate.state";
/// CO2 the temperate branch of a bistable body holds, with which it also
/// freezes.
pub(crate) const CLIMATE_FROZE_CO2: &str = "planet.climate.froze-co2";
/// Whether some CO2 the bistable body can hold thaws its frozen branch.
/// A flag, because a thaw out of reach has no finite CO2 to record.
pub(crate) const CLIMATE_THAWS: &str = "planet.climate.thaws";
/// The least CO2 known to thaw the frozen branch; recorded only when
/// [`CLIMATE_THAWS`] is true.
pub(crate) const CLIMATE_THAW_CO2: &str = "planet.climate.thaw-co2";
/// The share of its history a bistable body spends frozen: from the
/// thaw (when it has one), the CO2 it froze with and the volcanism.
pub(crate) const CLIMATE_SNOWBALL_CHANCE: &str = "planet.climate.snowball-chance";
/// The snowball draw, uniform in (0, 1), from the `planet/snowball`
/// sub-seed; recorded only for a bistable body.
pub(crate) const CLIMATE_SNOWBALL_DRAW: &str = "planet.climate.snowball-draw";
/// Whether the climate's air is a Titan's (`science::surface::titan_like_air`):
/// recorded for a reducing body only, the test of the reducing-air
/// fallback.
pub(crate) const CLIMATE_TITAN_LIKE: &str = "planet.climate.titan-like";
/// Whether the water the climate kept boils: no wet balance exists under
/// its light, its air and its heat, its thermostat holding it or not. A
/// flag.
pub(crate) const CLIMATE_BOILS: &str = "planet.climate.boils";
/// Whether a runaway's steam hides its surface: a runaway whose steam is
/// supercritical at its base (`science::steam::hides_surface`).
pub(crate) const CLIMATE_HIDES_SURFACE: &str = "planet.climate.hides-surface";
/// Partial pressure of N2 in the air the climate leaves.
pub(crate) const CLIMATE_AIR_N2: &str = "planet.climate.air.n2";
/// Partial pressure of CO2 in the air the climate leaves.
pub(crate) const CLIMATE_AIR_CO2: &str = "planet.climate.air.co2";
/// Partial pressure of water vapour in the air the climate leaves.
pub(crate) const CLIMATE_AIR_H2O: &str = "planet.climate.air.h2o";
/// Partial pressure of CH4 in the air the climate leaves.
pub(crate) const CLIMATE_AIR_CH4: &str = "planet.climate.air.ch4";
/// Partial pressure of NH3 in the air the climate leaves.
pub(crate) const CLIMATE_AIR_NH3: &str = "planet.climate.air.nh3";
/// Partial pressure of SO2 in the air the climate leaves.
pub(crate) const CLIMATE_AIR_SO2: &str = "planet.climate.air.so2";
/// Partial pressure of O2 in the air the climate leaves.
pub(crate) const CLIMATE_AIR_O2: &str = "planet.climate.air.o2";
/// Surface pressure of the air the climate leaves: the sum of its
/// partial pressures.
pub(crate) const CLIMATE_AIR_PRESSURE: &str = "planet.climate.air.pressure";
/// N2 on the ground at the surface temperature (liquid or frost);
/// recorded when positive.
pub(crate) const CLIMATE_CONDENSATE_N2: &str = "planet.climate.condensate.n2";
/// CH4 on the ground at the surface temperature (a sea or a frost);
/// recorded when positive.
pub(crate) const CLIMATE_CONDENSATE_CH4: &str = "planet.climate.condensate.ch4";
/// What fills the oceans, from the phase diagrams: recorded when there is
/// a liquid.
pub(crate) const SURFACE_LIQUID: &str = "planet.surface.liquid";

/// The gases of the air the climate leaves, each with the code of its
/// partial pressure, in the contract's order.
pub(crate) const CLIMATE_AIR_GASES: [(timeline::Gas, &str); 7] = [
    (timeline::Gas::N2, CLIMATE_AIR_N2),
    (timeline::Gas::Co2, CLIMATE_AIR_CO2),
    (timeline::Gas::H2o, CLIMATE_AIR_H2O),
    (timeline::Gas::Ch4, CLIMATE_AIR_CH4),
    (timeline::Gas::Nh3, CLIMATE_AIR_NH3),
    (timeline::Gas::So2, CLIMATE_AIR_SO2),
    (timeline::Gas::O2, CLIMATE_AIR_O2),
];
