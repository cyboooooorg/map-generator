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
/// Water lost as steam in a runaway: 9 kg of water for each kg of hydrogen
/// the energy-limited escape takes, capped by the bulk water. Recorded only
/// in a runaway.
pub(crate) const WATER_STEAM_LOST: &str = "planet.water.steam-lost";
/// Water the planet keeps: its bulk water `w (1 - f) M`, less the steam
/// lost in a runaway.
pub(crate) const WATER_INVENTORY: &str = "planet.water.inventory";
/// Whether the climate boiled the ocean: false, imposed, until the
/// climate's boiling loop of the next pull request.
pub(crate) const WATER_BOILED: &str = "planet.water.boiled";
/// Mass of the planet once the steam is gone: its dry mass `(1 - w) M`
/// plus the water kept.
pub(crate) const MASS_AFTER_STEAM: &str = "planet.mass.after-steam";
/// Water mass fraction of the planet once the steam is gone: the water
/// kept over the mass after the steam.
pub(crate) const WATER_KEPT_FRACTION: &str = "planet.water.kept-fraction";
/// Tectonic regime: heat pipe, stagnant lid, plates or episodic, from the
/// internal heat flux, the mass and the water kept.
pub(crate) const REGIME: &str = "planet.regime";
