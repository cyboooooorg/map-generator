// The bodies of the sky as the planet view draws them (see systemSky.ts,
// which places those of a planetary system): their direction, size, map or
// preview, axes, air and display switches, and the shadows they cast on
// the body shown (eclipses).
//
// Directions are in the frame of the body shown, as the viewer's sphere is
// (`[cos lat sin lon, sin lat, cos lat cos lon]`, y along the spin axis);
// positions and radii are in radii of that body.
import type { Preview } from './api/types/Preview';
import type { Sky } from './sky';

export type Vec3 = [number, number, number];

/** A body's own x, y (spin axis) and z axes, in another body's frame. */
export type Axes = [Vec3, Vec3, Vec3];

/** The axes of the frame itself. */
export const IDENTITY_AXES: Axes = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1],
];



/** A body drawn in the sky of the body shown. */
export interface SkyBody {
  direction: Vec3;
  /** Apparent diameter from the centre of the body shown, degrees. */
  angularDiameterDeg: number;
  /**
   * Its centre and radius, in radii of the body shown, when it is near
   * enough for the view to place it from the camera rather than at
   * infinity from the centre (where it would slip off its orbit line as
   * the camera turns round the globe).
   */
  position?: Vec3;
  radius?: number;
  /** Colour of its surface, linear RGB, when it has no map. */
  colour: Vec3;
  /** Its surface map (`surface.png`), or null for a plain colour. */
  texture: string | null;
  /**
   * Its own axes (its x, its spin axis, its z: its map's frame) in the
   * frame of the body shown, as it turns on itself; null for the frame of
   * the body shown.
   */
  axes: Axes | null;
  /** Its cloud map (`clouds.png`, grey), or null for none. */
  clouds?: string | null;
  /** Linear colour of its clouds. */
  cloudTint?: Vec3;
  /** Its atmosphere, seen from afar: a halo at the limb (see `halo`). */
  halo?: Halo | null;
  /** Axial tilt, degrees, for its tropics and polar circles. */
  axialTiltDeg?: number;
  /** What is drawn of it: all of it when absent; its grid and tropics only when asked. */
  show?: SkyBodyShow;
  /** How it is drawn while it has no map (a body of a system not generated yet). */
  preview?: Preview;
}

/** How the planet looks in a moon's sky: its clouds, its air, its marks and their switches. */
export type PlanetLook = Pick<SkyBody, 'clouds' | 'cloudTint' | 'halo' | 'axialTiltDeg' | 'show'>;

/** The display switches of a body in the sky (its own, not the body shown's). */
export interface SkyBodyShow {
  atmosphere: boolean;
  clouds: boolean;
  grid: boolean;
  tropics: boolean;
}

/** An atmosphere seen from afar. */
export interface Halo {
  /** Linear colour of the light it scatters, brightest channel 1. */
  colour: Vec3;
  /** Height of the halo, in radii of the body. */
  thickness: number;
  /** Brightness of the halo at the limb, 0 to 1. */
  strength: number;
}

/**
 * The halo of an atmosphere (see sky.ts), null for an airless body: the
 * colour of the light its gas and aerosols scatter (their columns), and a
 * brightness growing with their optical depth.
 */
export function halo(sky: Sky | null): Halo | null {
  if (!sky) return null;
  const column = sky.betaR.map((b, i) => b * sky.heightR + sky.betaM * sky.heightM * sky.mieAlbedo[i]!) as Vec3;
  const brightest = Math.max(...column, 1e-9);
  const depth = (column[0] + column[1] + column[2]) / 3;
  return {
    colour: column.map((c) => c / brightest) as Vec3,
    thickness: sky.top - 1,
    strength: Math.min(1, 0.9 * (1 - Math.exp(-4 * depth))),
  };
}

/** A body whose shadow can fall on the body shown (an eclipse). */
export interface ShadowCaster {
  /** Its centre, in radii of the body shown. */
  position: Vec3;
  /** Its radius, in radii of the body shown. */
  radius: number;
}

/** Most shadow casters the surface shader takes. */
export const MAX_CASTERS = 4;

/**
 * Share of the sunlight a shadow caster hides at a point of the unit
 * sphere (0 to 1), the sun of angular radius `sunRadius` (radians) in
 * direction `sun`: as the surface shader computes it (umbra inside, a
 * penumbra growing with the distance behind the caster).
 */
export function eclipse(point: Vec3, sun: Vec3, caster: ShadowCaster, sunRadius: number): number {
  const c: Vec3 = [caster.position[0] - point[0], caster.position[1] - point[1], caster.position[2] - point[2]];
  const t = c[0] * sun[0] + c[1] * sun[1] + c[2] * sun[2];
  if (t <= 0) return 0;
  const d = Math.hypot(c[0] - t * sun[0], c[1] - t * sun[1], c[2] - t * sun[2]);
  const spread = t * Math.tan(sunRadius);
  const umbra = Math.max(caster.radius - spread, 0);
  const penumbra = caster.radius + spread;
  // Smoothstep from the penumbra's edge to the umbra's (or the centre).
  const x = Math.min(Math.max((penumbra - d) / Math.max(penumbra - umbra, 1e-6), 0), 1);
  const covered = x * x * (3 - 2 * x);
  // An annular eclipse never hides the whole sun.
  const most = caster.radius >= spread ? 1 : (caster.radius / spread) ** 2;
  return covered * most;
}
