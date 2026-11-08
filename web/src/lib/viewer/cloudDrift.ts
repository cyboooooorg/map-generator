// The clock of the drifting clouds: where the cloud map and, on a giant, its
// bands and storms stand, as a pure state so that it can be tested without
// WebGL (PlanetView keeps it in its uniforms).

/** Where the drift has brought the clouds. */
export interface CloudDrift {
  /** Turn of the cloud map round the planet, `[0, 1)` (uCloudShift). */
  shift: number;
  /** Seconds the giant's bands and storms have drifted (uBandTime). */
  bandTime: number;
}

/** The clouds as generated: no drift. */
export const NO_DRIFT: CloudDrift = { shift: 0, bandTime: 0 };

/** The drift after `seconds` more, a turn of the cloud map taking `period` seconds. */
export function advanceDrift(drift: CloudDrift, seconds: number, period: number): CloudDrift {
  return { shift: (drift.shift + seconds / period) % 1, bandTime: drift.bandTime + seconds };
}
