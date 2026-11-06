// The relief's true scale. Elevations come in units of the generator's
// (`craters::ELEVATION_KM` in crates/mapgen): one unit is 25 km, so an
// Earth-like world's highest peaks (0.3–0.4 above the coastline) stand
// 8–10 km high. The viewer's relief slider is an exaggeration of that:
// 1 is the true relief, the default and the slider's marked point.

/** Kilometres per elevation unit, as the generator calibrates them. */
export const ELEVATION_KM = 25;

/** A body's radii per elevation unit at the true relief, from its circumference in km. */
export function reliefScale(circumferenceKm: number): number {
  return circumferenceKm > 0 ? (ELEVATION_KM * 2 * Math.PI) / circumferenceKm : 0;
}
