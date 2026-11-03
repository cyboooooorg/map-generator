// Formatting shared by the components, and the requests and commands that
// reproduce a world.
import type { BiomeInfo } from './api/types/BiomeInfo';
import type { WorldMetadata } from './api/types/WorldMetadata';
import type { WorldRequest } from './api/types/WorldRequest';
import { getLocale, t } from './i18n/index.svelte';

/** Number formats of the active locale, built once per locale and options. */
const numberFormats = new Map<string, Intl.NumberFormat>();

function numberFormat(options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const locale = getLocale();
  const key = `${locale} ${JSON.stringify(options)}`;
  let format = numberFormats.get(key);
  if (!format) {
    format = new Intl.NumberFormat(locale, options);
    numberFormats.set(key, format);
  }
  return format;
}

/** A number with `digits` decimals in the active locale (decimal comma in French). */
export function formatNumber(value: number, digits = 0): string {
  return numberFormat({ minimumFractionDigits: digits, maximumFractionDigits: digits }).format(value);
}

/** A number with at most `digits` decimals, trailing zeros dropped (0.5, 100). */
export function formatUpTo(value: number, digits: number): string {
  return numberFormat({ maximumFractionDigits: digits }).format(value);
}

/** A number followed by its unit, with the locale's space between them. */
export function withUnit(value: string, unit: string): string {
  return `${value}${t.format.unitSpace}${unit}`;
}

/** A whole number with thousands separators. */
export function formatInt(value: number): string {
  return numberFormat({ maximumFractionDigits: 0 }).format(value);
}

/** A large area, in km², rounded to three significant digits. */
export function formatArea(km2: number): string {
  const significant = numberFormat({ maximumSignificantDigits: 3 });
  if (km2 >= 1e6) {
    return withUnit(significant.format(km2 / 1e6), t.format.millionKm2);
  }
  return withUnit(significant.format(km2), 'km²');
}

/** A distance in km, with a decimal under 10 km. */
export function formatKm(km: number): string {
  return withUnit(km < 10 ? formatNumber(km, 1) : formatInt(km), 'km');
}

/** A fraction as a percentage; small non-zero values stay visible. */
export function formatShare(share: number): string {
  if (share > 0 && share < 0.001) {
    return withUnit(`<${formatNumber(0.1, 1)}`, '%');
  }
  return withUnit(formatNumber(share * 100, share < 0.1 ? 1 : 0), '%');
}

/** A duration in seconds, as "42 s" or "3 min 05 s". */
export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  if (s < 60) {
    return withUnit(String(s), 's');
  }
  return `${withUnit(String(Math.floor(s / 60)), 'min')} ${withUnit(String(s % 60).padStart(2, '0'), 's')}`;
}

const relativeFormats = new Map<string, Intl.RelativeTimeFormat>();

/** How long ago a timestamp (ms since the epoch) was, as "3 hours ago". */
export function formatAgo(ms: number, now = Date.now()): string {
  const locale = getLocale();
  let relative = relativeFormats.get(locale);
  if (!relative) {
    relative = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
    relativeFormats.set(locale, relative);
  }
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
  return t.format.justNow;
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
    return t.worldInfo.airless;
  }
  const digits = pressure < 0.1 ? Math.max(0, 1 - Math.floor(Math.log10(pressure))) : 2;
  const bar = withUnit(formatNumber(pressure, digits), 'bar');
  const clouds = meta.preset.atmosphere?.clouds ?? 'water';
  return clouds === 'none'
    ? t.worldInfo.pressureNoClouds({ pressure: bar })
    : t.worldInfo.pressureClouds({ pressure: bar, clouds: t.clouds[clouds] });
}

/**
 * The tectonic regime and surface age of a world, e.g. "stagnant lid,
 * surface 3.8 Ga", in the active locale. Worlds made before geology get
 * Earth's regime and no age.
 */
export function formatGeology(meta: WorldMetadata): string {
  const regime = t.regimes[meta.preset.geology?.regime ?? 'plates'];
  const age = meta.surface_age_ga;
  if (age == null) {
    return regime;
  }
  const shown = age >= 1 ? withUnit(formatNumber(age, 1), 'Ga') : withUnit(formatNumber(age * 1000), 'Ma');
  return t.worldInfo.geologySurface({ regime, age: shown });
}

/**
 * Colour a biome has on the hillshaded biome map: its surface colour, or its
 * classification colour for worlds written before surface colours existed.
 */
export function surfaceColour(biome: Pick<BiomeInfo, 'color' | 'surface_color'>): string {
  return biome.surface_color || biome.color;
}
