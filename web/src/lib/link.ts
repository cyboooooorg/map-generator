// Links to a view: the worlds shown, and optionally the camera, the layer,
// the relief and the sun, in the address's fragment:
//
//   #world=<id>&compare=<id>&view=g,<lon>,<lat>,<altitude>&layer=<id>&relief=<0..10>&sun=<season>,<lon>&orbit=<days>&epoch=<Ma>
//
// `view` is the camera: `g` on the globe (the point under the camera in
// degrees, and its altitude in planet radii), `m` on the flat map (the
// point at the centre of the screen, and the camera's height above the
// map, in the map's units: the map is 2π × π). `sun` is `off`, or the
// season (fraction of the year) and the longitude under the sun in
// degrees. `orbit` is the time of the moon orbits slider, in days past
// the season, absent at 0. `epoch` is the epoch of the time slider, in whole Ma before
// present (negative in the future), absent at the present; each world
// shows it within its own range. Reading is defensive: a missing, malformed or out-of-range value
// is ignored, and the rest of the link still applies.
import { RELIEF_MAX } from './shortcuts';
import { lonLatToSphere, sphereToLonLat } from './geo';
import type { CameraState } from './viewer/PlanetView';
import { LAYER_IDS, type Layer } from './viewer/shaders';
import { mapPoint } from './viewer/viewTransfer';

/** What a link sets in the viewer, beyond the worlds; every part is optional. */
export interface ViewLink {
  camera?: CameraState;
  layer?: Layer;
  relief?: number;
  sun?: { on: false } | { on: true; season: number; longitudeDeg: number };
  /** Days of the moon orbits slider past the season. */
  orbitDays?: number;
  /** Epoch of the time slider, Ma before present. */
  epoch?: number;
}

/** Epochs a link may name, Ma: 1 Ga ahead to the age of the Solar System. */
const EPOCHS = [-1000, 4600] as const;

/** Limits a link's camera is kept within (those of the viewer's controls). */
const GLOBE_ALTITUDE = [0.0003, 7] as const;
const MAP_HEIGHT = [0.002, 12] as const;
/** Latitude the globe camera stops short of the poles at, degrees (the camera's up is north). */
const POLE_DEG = 89.9;

const DEG = 180 / Math.PI;

/** A number with at most `digits` decimals, without trailing zeros. */
function fixed(value: number, digits: number): string {
  return String(Number(value.toFixed(digits)));
}

/** A positive number to four significant digits. */
function significant(value: number): string {
  return String(Number(value.toPrecision(4)));
}

/** Longitude in degrees, in [-180, 180). */
function wrapDegrees(lon: number): number {
  return ((((lon + 180) % 360) + 360) % 360) - 180;
}

/** The `view` value of a camera. */
export function encodeCamera(camera: CameraState): string {
  const [px, py, pz] = camera.position;
  if (camera.mode === 'globe') {
    const { lon, lat } = sphereToLonLat([px, py, pz]);
    const altitude = Math.hypot(px, py, pz) - 1;
    return `g,${fixed(wrapDegrees(lon * DEG), 4)},${fixed(lat * DEG, 4)},${significant(altitude)}`;
  }
  const [tx, ty, tz] = camera.target;
  const lon = (tx + Math.PI) * DEG;
  const lat = ty * DEG;
  return `m,${fixed(wrapDegrees(lon), 4)},${fixed(lat, 4)},${significant(pz - tz)}`;
}

/** Parses a finite number, or null. */
function number(text: string | undefined): number | null {
  if (text === undefined || text.trim() === '') {
    return null;
  }
  const value = Number(text);
  return Number.isFinite(value) ? value : null;
}

const clamp = (value: number, [low, high]: readonly [number, number]) => Math.min(high, Math.max(low, value));

/** The camera of a `view` value, or null when it is malformed. */
export function decodeCamera(text: string): CameraState | null {
  const [kind, ...rest] = text.split(',');
  if (rest.length !== 3 || (kind !== 'g' && kind !== 'm')) {
    return null;
  }
  const [lonDeg, latDeg, distance] = rest.map(number);
  if (lonDeg == null || latDeg == null || distance == null || Math.abs(latDeg) > 90 || distance <= 0) {
    return null;
  }
  const lon = (wrapDegrees(lonDeg) * Math.PI) / 180;
  if (kind === 'g') {
    const lat = (clamp(latDeg, [-POLE_DEG, POLE_DEG]) * Math.PI) / 180;
    const radius = 1 + clamp(distance, GLOBE_ALTITUDE);
    const [x, y, z] = lonLatToSphere({ lon, lat });
    return { mode: 'globe', position: [x * radius, y * radius, z * radius], target: [0, 0, 0], up: [0, 1, 0] };
  }
  const [x, y] = mapPoint({ lon: lon < 0 ? lon + 2 * Math.PI : lon, lat: (latDeg * Math.PI) / 180 });
  return { mode: 'map', position: [x, y, clamp(distance, MAP_HEIGHT)], target: [x, y, 0], up: [0, 1, 0] };
}

/** The fragment parameters of a view, in a stable order: `key=value`, values URL-safe. */
export function encodeView(view: ViewLink): string[] {
  const parts: string[] = [];
  if (view.camera) parts.push(`view=${encodeCamera(view.camera)}`);
  if (view.layer) parts.push(`layer=${view.layer}`);
  if (view.relief !== undefined) parts.push(`relief=${fixed(view.relief, 2)}`);
  if (view.sun) {
    parts.push(view.sun.on ? `sun=${fixed(view.sun.season, 3)},${fixed(view.sun.longitudeDeg, 1)}` : 'sun=off');
  }
  if (view.orbitDays) parts.push(`orbit=${fixed(view.orbitDays, 3)}`);
  if (view.epoch) parts.push(`epoch=${Math.round(view.epoch)}`);
  return parts;
}

/** The view a fragment's parameters describe, without the parts that are missing or malformed. */
export function decodeView(params: URLSearchParams): ViewLink {
  const view: ViewLink = {};
  const camera = params.get('view');
  if (camera !== null) {
    const state = decodeCamera(camera);
    if (state) view.camera = state;
  }
  const layer = params.get('layer');
  if (layer !== null && Object.hasOwn(LAYER_IDS, layer)) {
    view.layer = layer as Layer;
  }
  const relief = number(params.get('relief') ?? undefined);
  if (relief !== null && relief >= 0 && relief <= RELIEF_MAX) {
    view.relief = relief;
  }
  const sun = params.get('sun');
  if (sun === 'off') {
    view.sun = { on: false };
  } else if (sun !== null) {
    const [season, longitude, extra] = sun.split(',');
    const s = number(season);
    const l = number(longitude);
    if (extra === undefined && s !== null && l !== null && s >= 0 && s <= 1 && l >= 0 && l <= 360) {
      view.sun = { on: true, season: s % 1, longitudeDeg: l };
    }
  }
  const orbit = number(params.get('orbit') ?? undefined);
  if (orbit !== null && orbit >= 0) {
    view.orbitDays = orbit;
  }
  const epoch = number(params.get('epoch') ?? undefined);
  if (epoch !== null && Number.isInteger(epoch) && epoch >= EPOCHS[0] && epoch <= EPOCHS[1]) {
    view.epoch = epoch;
  }
  return view;
}

/** The worlds a fragment names: `world`, and `compare` beside it. */
export function decodeWorlds(params: URLSearchParams): { world: string | null; compare: string | null } {
  return { world: params.get('world'), compare: params.get('compare') };
}

/** The fragment (without '#') naming the worlds shown, and the view when given. */
export function linkHash(world: string | null, compare: string | null, view: ViewLink | null = null): string {
  const parts: string[] = [];
  if (world) {
    parts.push(`world=${encodeURIComponent(world)}`);
    if (compare) parts.push(`compare=${encodeURIComponent(compare)}`);
  }
  if (view) parts.push(...encodeView(view));
  return parts.join('&');
}

/** The systems a link names: the one shown, the body shown in it, and the one compared. */
export interface SystemsLink {
  system: string | null;
  body: string | null;
  compare: string | null;
  /** The planets whose cards are pinned in the star view, by body id (none when absent). */
  pins?: readonly string[];
}

/** `#system=<id>&body=<id>&compare=<id>&pins=<id>,<id>`: the systems of a link. */
export function decodeSystems(params: URLSearchParams): SystemsLink {
  const pins = (params.get('pins') ?? '').split(',').filter((id) => id !== '');
  return { system: params.get('system'), body: params.get('body'), compare: params.get('compare'), pins };
}

/** The fragment (without '#') naming the systems shown, and the view when given. */
export function systemHash(link: SystemsLink, view: ViewLink | null = null): string {
  const parts: string[] = [];
  if (link.system) {
    parts.push(`system=${encodeURIComponent(link.system)}`);
    if (link.body) parts.push(`body=${encodeURIComponent(link.body)}`);
    if (link.compare) parts.push(`compare=${encodeURIComponent(link.compare)}`);
    if (link.pins?.length) parts.push(`pins=${link.pins.map(encodeURIComponent).join(',')}`);
  }
  if (view) parts.push(...encodeView(view));
  return parts.join('&');
}
