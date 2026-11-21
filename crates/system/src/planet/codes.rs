//! The cause codes of the planet's composition, interior and heat:
//! dotted lowercase, one constant per node, read with `Code::of`.

/// Core mass fraction of the planet, drawn.
pub(crate) const CORE_MASS_FRACTION: &str = "planet.core-mass-fraction";
/// Water mass fraction of the planet, drawn.
pub(crate) const WATER_FRACTION: &str = "planet.water-fraction";
/// Radiogenic abundance of the planet in dex of Earth's, drawn.
pub(crate) const RADIOGENIC_ABUNDANCE: &str = "planet.radiogenic-abundance";
/// Temperature of the water layer: the equilibrium temperature of the
/// trial, until the climate of the planet interior plan's second pull
/// request gives the surface temperature.
pub(crate) const WATER_TEMPERATURE: &str = "planet.water-temperature";
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
/// State of the matter of a shell.
pub(crate) const LAYER_STATE: &str = "planet.layer.state";
/// Radiogenic heating power.
pub(crate) const HEAT_RADIOGENIC: &str = "planet.heat.radiogenic";
/// Tidal heating power.
pub(crate) const HEAT_TIDAL: &str = "planet.heat.tidal";
/// Secular cooling power.
pub(crate) const HEAT_SECULAR: &str = "planet.heat.secular";
