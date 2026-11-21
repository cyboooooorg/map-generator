//! The cause codes of the planet's composition, interior and heat:
//! dotted lowercase, one constant per node, read with `Code::of`.

/// Core mass fraction of the planet without its water, drawn.
pub(crate) const CORE_MASS_FRACTION: &str = "planet.core-mass-fraction";
/// Whether the planet formed at or beyond the ice line, from the
/// semi-major axis and the ice line.
pub(crate) const FORMED_BEYOND_ICE_LINE: &str = "planet.formed-beyond-ice-line";
/// Whether the planet migrated in from beyond the ice line, drawn.
pub(crate) const MIGRATED: &str = "planet.migrated";
/// Water mass fraction of the planet (bulk water), drawn: an icy
/// fraction for a water world, a dry one otherwise.
pub(crate) const WATER_FRACTION: &str = "planet.water-fraction";
/// Radiogenic abundance of the planet in dex of Earth's, drawn.
pub(crate) const RADIOGENIC_ABUNDANCE: &str = "planet.radiogenic-abundance";
/// Primordial hydrogen-helium envelope mass fraction of the planet, drawn
/// by mass regime (a gas giant's is `1 - heavy-element mass / M`).
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
/// Mass of a hot water world's water, `w M`, kept as an inventory outside
/// the structure: too hot for the water laws (355 K and above), it becomes
/// the steam envelope of the planet interior plan's second pull request.
pub(crate) const WATER_INVENTORY: &str = "planet.water-inventory";
/// The structure node: the central pressure that the integration finds
/// for the layers, which every radius and shell of the structure follows.
pub(crate) const CENTRAL_PRESSURE: &str = "planet.structure.central-pressure";
/// Radius of the planet, from the integrated structure.
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
/// State of the matter of a shell, set by the engine: the mantle is solid
/// until the temperatures of the planet interior plan's second pull
/// request.
pub(crate) const LAYER_STATE: &str = "planet.layer.state";
/// State of the matter of a shell read from the phase of its equation of
/// state, which pressure alone selects: the core's liquid outer and solid
/// inner phases switch at PREM's inner-core boundary pressure (the
/// Valencia et al. 2006 fit), and the water's liquid and ices at their
/// phase boundaries on its isotherm or melting curve. Not a thermal state:
/// the core's comes with the temperatures of a later pull request.
pub(crate) const LAYER_STATE_BY_PRESSURE: &str = "planet.layer.state-by-pressure";
/// Radiogenic heating power.
pub(crate) const HEAT_RADIOGENIC: &str = "planet.heat.radiogenic";
/// Tidal heating power.
pub(crate) const HEAT_TIDAL: &str = "planet.heat.tidal";
/// Secular cooling power.
pub(crate) const HEAT_SECULAR: &str = "planet.heat.secular";
