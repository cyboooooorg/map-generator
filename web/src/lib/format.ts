// Formatting shared by the components, and the requests and commands that
// reproduce a world.
import type { BiomeInfo } from './api/types/BiomeInfo';
import type { Regime } from './api/types/Regime';
import type { WorldMetadata } from './api/types/WorldMetadata';
import type { WorldRequest } from './api/types/WorldRequest';

const integer = new Intl.NumberFormat('en', { maximumFractionDigits: 0 });

/** A whole number with thousands separators. */
export function formatInt(value: number): string {
  return integer.format(value);
}

const significant = new Intl.NumberFormat('en', { maximumSignificantDigits: 3 });

/** A large area, in km², rounded to three significant digits. */
export function formatArea(km2: number): string {
  if (km2 >= 1e6) {
    return `${significant.format(km2 / 1e6)} M km²`;
  }
  return `${significant.format(km2)} km²`;
}

/** A distance in km, with a decimal under 10 km. */
export function formatKm(km: number): string {
  return km < 10 ? `${km.toFixed(1)} km` : `${formatInt(km)} km`;
}

/** A fraction as a percentage; small non-zero values stay visible. */
export function formatShare(share: number): string {
  if (share > 0 && share < 0.001) {
    return '<0.1 %';
  }
  return `${(share * 100).toFixed(share < 0.1 ? 1 : 0)} %`;
}

/** A duration in seconds, as "42 s" or "3 min 05 s". */
export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  if (s < 60) {
    return `${s} s`;
  }
  return `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, '0')} s`;
}

const relative = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

/** How long ago a timestamp (ms since the epoch) was, as "3 hours ago". */
export function formatAgo(ms: number, now = Date.now()): string {
  const seconds = (ms - now) / 1000;
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 31_536_000],
    ['month', 2_592_000],
    ['week', 604_800],
    ['day', 86_400],
    ['hour', 3_600],
    ['minute', 60],
  ];
  for (const [unit, size] of units) {
    if (Math.abs(seconds) >= size) {
      return relative.format(Math.round(seconds / size), unit);
    }
  }
  return 'just now';
}

/** Largest seed the generator takes (a u32). */
export const MAX_SEED = 4_294_967_295;

/** A random seed, drawn from the browser's cryptographic generator. */
export function randomSeed(): number {
  const values = new Uint32Array(1);
  crypto.getRandomValues(values);
  return values[0] ?? 0;
}

/**
 * The request that generates a world again with every parameter the same
 * but a new seed: a sibling of the world, under a new id.
 */
export function rerollRequest(meta: WorldMetadata, seed: number = randomSeed()): WorldRequest {
  return { ...requestFromWorld(meta), seed };
}

/** Rounds a parameter as the CLI prints it, so commands stay readable. */
function round(value: number, digits: number): number {
  return Number(value.toFixed(digits));
}

/** The generation request that reproduces a world exactly. */
export function requestFromWorld(meta: WorldMetadata): WorldRequest {
  return {
    planet: meta.planet_type,
    seed: meta.seed,
    sea_level: round(meta.sea_level, 4),
    volcanic: round(meta.volcanic_intensity, 4),
    circumference_km: round(meta.circumference_km, 1),
    axial_tilt_deg: round(meta.axial_tilt_deg, 2),
    erosion: round(meta.erosion, 2),
    // Worlds made before the atmosphere had Earth's pressure.
    pressure_bar: round(meta.pressure_bar ?? 1, 4),
    // Worlds made before the surface age leave it to the preset.
    ...(meta.surface_age_ga == null ? {} : { surface_age_ga: round(meta.surface_age_ga, 3) }),
    width: meta.width,
    height: meta.height,
  };
}

/**
 * The command line that regenerates a world with the CLI. A preset read from
 * a file is named by its preset name: pass its path instead if it is not in
 * the built-in list.
 */
export function cliCommand(meta: WorldMetadata): string {
  const r = requestFromWorld(meta);
  const args = [
    `--planet ${r.planet}`,
    `--seed ${r.seed}`,
    `--sea-level ${r.sea_level}`,
    `--volcanic ${r.volcanic}`,
    `--circumference ${r.circumference_km}`,
    `--axial-tilt ${r.axial_tilt_deg}`,
    `--erosion ${r.erosion}`,
    `--pressure ${r.pressure_bar}`,
    ...(r.surface_age_ga === undefined ? [] : [`--surface-age ${r.surface_age_ga}`]),
    `--width ${r.width}`,
    `--height ${r.height}`,
  ];
  return `mise run gen -- ${args.join(' ')}`;
}

/** Copies text to the clipboard; resolves false when the browser refuses. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** Offers a blob to the user as a file download. */
export function saveBlob(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  // Give the browser time to start the download before revoking.
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** The pressure and clouds of a world, e.g. "1.00 bar, water clouds". */
export function formatAtmosphere(meta: WorldMetadata): string {
  const pressure = meta.pressure_bar ?? 1;
  if (pressure === 0) {
    return 'none';
  }
  const bar = pressure < 0.1 ? pressure.toPrecision(2) : pressure.toFixed(2);
  const clouds = meta.preset.atmosphere?.clouds ?? 'water';
  return clouds === 'none' ? `${bar} bar, no clouds` : `${bar} bar, ${clouds} clouds`;
}

const REGIMES: Record<Regime, string> = {
  plates: 'plate tectonics',
  stagnant: 'stagnant lid',
  episodic: 'episodic lid',
  heat_pipe: 'heat pipe',
};

/**
 * The tectonic regime and surface age of a world, e.g. "stagnant lid,
 * surface 3.8 Ga". Worlds made before geology get Earth's regime and no age.
 */
export function formatGeology(meta: WorldMetadata): string {
  const regime = REGIMES[meta.preset.geology?.regime ?? 'plates'];
  const age = meta.surface_age_ga;
  if (age == null) {
    return regime;
  }
  const shown = age >= 1 ? `${age.toFixed(1)} Ga` : `${Math.round(age * 1000)} Ma`;
  return `${regime}, surface ${shown}`;
}

/**
 * Colour a biome has on the hillshaded biome map: its surface colour, or its
 * classification colour for worlds written before surface colours existed.
 */
export function surfaceColour(biome: Pick<BiomeInfo, 'color' | 'surface_color'>): string {
  return biome.surface_color || biome.color;
}
