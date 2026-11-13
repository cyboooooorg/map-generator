/**
 * The classes drawn through the giant path: the giants (hot Neptune included) and the
 * mini-Neptune, whose radius is the giant look's (issue #446). The
 * presets a request around a planet names (`terran`, `barren`, …) are
 * never giants.
 */
export const GIANT_CLASSES: ReadonlySet<string> = new Set(['gas-giant', 'ice-giant', 'hot-jupiter', 'hot-neptune', 'mini-neptune']);

/** Whether a body type (a class key or a preset name) is drawn as a giant. */
export const isGiantType = (type: string): boolean => GIANT_CLASSES.has(type);

/** Earth oceans kept at the surface from which a body counts as holding ice (`ICE_WORLD_MIN_OCEANS`, classify.rs). */
export const ICE_WORLD_MIN_OCEANS = 0.01;

/** Freezing point of water, K. */
const FREEZING_K = 273.15;

/** The part of a planet's model the surface helpers read. */
export type SurfaceModel = {
  surface?: { water: { oceans: number }; climate: { temperature_k: number; liquid: string } } | null;
} | null | undefined;

/**
 * Whether a world of class `cls` with `model` is rocky but holds its
 * water frozen at the surface: at least `ICE_WORLD_MIN_OCEANS` Earth
 * oceans kept, no liquid, under 273.15 K. Never an ice world (that is
 * already its class), an ocean world or a giant; false without a model.
 */
export function hasFrozenSurface(cls: string, model: SurfaceModel): boolean {
  const surface = model?.surface;
  if (!surface || cls === 'ice-world' || cls === 'ocean-world' || isGiantType(cls)) return false;
  return (
    surface.water.oceans >= ICE_WORLD_MIN_OCEANS && surface.climate.liquid === 'none' && surface.climate.temperature_k < FREEZING_K
  );
}

/** The mean surface temperature of a model, K; null for a body without a surface (a giant, no model). */
export function surfaceTemperatureK(model: SurfaceModel): number | null {
  return model?.surface?.climate.temperature_k ?? null;
}
