// Formatting shared by the components, and the requests and commands that
// reproduce a world.
import type { BiomeInfo } from './api/types/BiomeInfo';
import type { Formation } from './api/types/Formation';
import type { StarInfo } from './api/types/StarInfo';
import type { System } from './api/types/System';
import type { WorldMetadata } from './api/types/WorldMetadata';
import type { WorldRequest } from './api/types/WorldRequest';
import { getLocale, t } from './i18n/index.svelte';
import { type SurfaceModel, surfaceTemperatureK } from './planetClass';

/** Number formats of the active locale, built once per locale and options. */
const numberFormats = new Map<string, Intl.NumberFormat>();

function numberFormat(options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const locale = getLocale();
  const key = `${locale} ${JSON.stringify(options)}`;
  let format = numberFormats.get(key);
  if (!format) {
    // signDisplay 'negative' prints a negative number rounded to zero (-0.2 as
    // 0 digits, or -0) as a plain 0, never "-0".
    format = new Intl.NumberFormat(locale, { signDisplay: 'negative', ...options });
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

/**
 * The request with the lone-planet flag of the system it comes from: a
 * world's metadata does not say whether its system is a lone planet, the
 * system's stored request does. Varying, rerolling or moving to another
 * epoch a lone planet gives a lone planet.
 */
export function keepAlone(request: WorldRequest, system: System | null | undefined): WorldRequest {
  const asked = system?.request;
  return asked && 'around' in asked && asked.around.alone === true ? { ...request, alone: true } : request;
}

/** Rounds a parameter as the CLI prints it, so commands stay readable. */
function round(value: number, digits: number): number {
  return Number(value.toFixed(digits));
}

/**
 * The model inputs a world formed with, as a request sets them: world.json's
 * `formation` (`PlanetModel::formation` in crates/mapgen), exact. The drawn
 * mass before photoevaporation and the envelope at formation, so that escape
 * runs once; the water in Earth oceans; the CO₂ and N₂ factors; the core
 * share and radiogenic abundance; whether life arose. A moon has none.
 */
export function formationRequest(formation: Formation | null | undefined): WorldRequest {
  if (!formation) {
    return {};
  }
  const o = formation.overrides;
  const set = <K extends keyof WorldRequest>(key: K, value: WorldRequest[K] | null): WorldRequest =>
    value === null || value === undefined ? {} : { [key]: value };
  return {
    mass_earth: formation.mass_earth,
    ...set('core_mass_fraction', o.core_mass_fraction),
    ...set('water_oceans', o.water_oceans),
    ...set('envelope_fraction', o.envelope_fraction),
    ...set('co2', o.co2),
    ...set('n2', o.n2),
    ...set('radiogenic', o.radiogenic),
    ...set('life', o.life),
  };
}

/**
 * The generation request that reproduces a world: the archetype it was
 * asked as, the inputs its planet formed with pinned as overrides
 * ({@link formationRequest}, so that a new seed, a drift or a variation keeps
 * the planet), and its orbit and star.
 */
export function requestFromWorld(meta: WorldMetadata): WorldRequest {
  return {
    ...(meta.archetype ? { planet: meta.archetype } : {}),
    seed: meta.seed,
    ...formationRequest(meta.formation),
    axial_tilt_deg: round(meta.axial_tilt_deg, 2),
    star: meta.star.type,
    orbit_au: round(meta.star.orbit_au, 5),
    eccentricity: round(meta.star.eccentricity, 4),
    // The star's age, as stored (not rounded: one rounded up past a short-lived
    // star's life is refused). A planet of a system keeps its system's age, which
    // its own seed would not draw again.
    age_gyr: meta.star.age_gyr,
    // A world away from the present of its plates (0 is left out).
    ...(meta.drift_ma ? { drift_ma: meta.drift_ma } : {}),
    // A world whose moons were turned off.
    ...(meta.moons == null ? { moons: 'none' as const } : {}),
    // A number of moons asked for.
    ...(meta.moons?.count_asked != null ? { moons: { count: meta.moons.count_asked } } : {}),
    // The star's metallicity, and giants turned off.
    metallicity: round(meta.star.metallicity, 4),
    ...(meta.giants == null && !meta.giant ? { giants: 'none' as const } : {}),
    // A giant's moon stays one.
    ...(meta.host ? { host: 'giant' as const } : {}),
    width: meta.width,
    height: meta.height,
  };
}

/**
 * The command line that regenerates a world with the CLI. An archetype read
 * from a file is named by its name: pass its path instead if it is not in
 * the built-in list.
 */
export function cliCommand(meta: WorldMetadata): string {
  const r = requestFromWorld(meta);
  const args = [
    ...(r.planet === undefined ? [] : [`--planet ${r.planet}`]),
    `--seed ${r.seed}`,
    ...(r.mass_earth === undefined ? [] : [`--mass ${r.mass_earth}`]),
    ...(r.core_mass_fraction === undefined ? [] : [`--core-fraction ${r.core_mass_fraction}`]),
    ...(r.water_oceans === undefined ? [] : [`--water ${r.water_oceans}`]),
    ...(r.envelope_fraction === undefined ? [] : [`--envelope ${r.envelope_fraction}`]),
    ...(r.co2 === undefined ? [] : [`--co2 ${r.co2}`]),
    ...(r.n2 === undefined ? [] : [`--n2 ${r.n2}`]),
    ...(r.radiogenic === undefined ? [] : [`--radiogenic ${r.radiogenic}`]),
    ...(r.life === undefined ? [] : [`--life ${r.life ? 'yes' : 'no'}`]),
    `--axial-tilt ${r.axial_tilt_deg}`,
    `--star ${r.star}`,
    `--orbit-au ${r.orbit_au}`,
    `--eccentricity ${r.eccentricity}`,
    ...(r.age_gyr === undefined ? [] : [`--age ${r.age_gyr}`]),
    ...(r.drift_ma === undefined ? [] : [`--drift=${r.drift_ma}`]),
    ...(r.moons === undefined ? [] : [`--moons ${typeof r.moons === 'object' ? r.moons.count : r.moons}`]),
    ...(r.metallicity === undefined ? [] : [`--metallicity=${r.metallicity}`]),
    ...(r.giants === undefined ? [] : [`--giants ${r.giants}`]),
    ...(r.host === undefined ? [] : [`--host ${r.host}`]),
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
  const clouds = meta.look.atmosphere.clouds;
  return clouds === 'none'
    ? t.worldInfo.pressureNoClouds({ pressure: bar })
    : t.worldInfo.pressureClouds({ pressure: bar, clouds: t.clouds[clouds] });
}

/**
 * The tectonic regime and surface age of a world, e.g. "stagnant lid,
 * surface 3.8 billion years", in the active locale.
 */
export function formatGeology(meta: WorldMetadata): string {
  const regime = t.regimes[meta.look.geology.regime];
  return t.worldInfo.geologySurface({ regime, age: formatSurfaceAge(meta.surface_age_ga) });
}

/**
 * A duration in Ga spelled out: "8.12 billion years (8.12 Ga)", singular or plural by the
 * language's rule, "600 million years (600 Ma)" under 1 Ga. Billions keep two
 * decimals at most, millions three significant digits.
 */
function spellAge(ga: number): string {
  const words = t.format.ageWords;
  const billions = Number(ga.toFixed(2));
  if (billions >= 1) {
    const unit = billions < 2 ? words.billionOne : words.billionMany;
    const n = formatUpTo(billions, 2);
    return `${withUnit(n, unit)} (${withUnit(n, 'Ga')})`;
  }
  const millions = Number((ga * 1000).toPrecision(3));
  if (millions >= 1000) {
    const n = formatUpTo(1, 2);
    return `${withUnit(n, words.billionOne)} (${withUnit(n, 'Ga')})`;
  }
  const shown = millions >= 1 ? Math.round(millions) : millions;
  const unit = shown >= 2 ? words.millionMany : words.millionOne;
  const n = formatUpTo(shown, 2);
  return `${withUnit(n, unit)} (${withUnit(n, 'Ma')})`;
}

/** A surface age, spelled out in billions or millions of years. */
export function formatSurfaceAge(ga: number): string {
  return spellAge(ga);
}

/** The age of a star or of the bodies of its system, spelled out in billions or millions of years. */
export function formatAge(gyr: number): string {
  return spellAge(gyr);
}

/** The star line of the world card: type, temperature and luminosity. */
export function formatStar(star: StarInfo): string {
  const luminosity = star.luminosity < 0.01 ? formatUpTo(star.luminosity, 4) : formatUpTo(star.luminosity, 2);
  return t.worldInfo.starValue({
    type: star.type,
    teff: withUnit(formatInt(Math.round(star.teff_k / 10) * 10), 'K'),
    luminosity: withUnit(luminosity, 'L☉'),
  });
}

/** A distance in AU, to three significant digits. */
export function formatAu(au: number): string {
  return withUnit(numberFormat({ maximumSignificantDigits: 3 }).format(au), t.format.au);
}

/** The orbit line: distance and place relative to the habitable zone. */
export function formatOrbit(star: StarInfo): string {
  const p = star.hz_position;
  const zone =
    p < 0 ? t.worldInfo.hzInner : p > 1 ? t.worldInfo.hzOuter : t.worldInfo.hzInside({ position: formatNumber(p, 2) });
  return t.worldInfo.orbitValue({ distance: formatAu(star.orbit_au), zone });
}

/** The eccentricity line: its value and the closest and farthest distances. */
export function formatEccentricity(star: StarInfo): string {
  const e = star.eccentricity ?? 0;
  if (e < 0.0005) {
    return t.worldInfo.circular;
  }
  return t.worldInfo.eccentricityValue({
    e: formatUpTo(e, 3),
    periapsis: formatAu(star.periapsis_au),
    apoapsis: formatAu(star.apoapsis_au),
  });
}

/** The light line: flux relative to Earth's and equilibrium temperature. */
export function formatStarlight(star: StarInfo): string {
  return t.worldInfo.lightValue({
    flux: `${formatUpTo(star.insolation, star.insolation < 0.1 ? 3 : 2)}×`,
    equilibrium: withUnit(formatInt(star.equilibrium_k - 273.15), '°C'),
  });
}

/**
 * A body's mean surface temperature from its `model`, as the equilibrium
 * one is shown (whole °C); null for a body without a surface (a giant, no
 * model).
 */
export function formatSurfaceTemperature(model: SurfaceModel): string | null {
  const k = surfaceTemperatureK(model);
  return k === null ? null : formatCelsius(k);
}

/** A temperature in kelvin as whole °C, as the equilibrium one is shown. */
export function formatCelsius(k: number): string {
  return withUnit(formatInt(Math.round(k - 273.15)), '°C');
}

/** The year: in days under two Earth years, in Earth years above. */
export function formatYear(days: number): string {
  return days < 730
    ? t.worldInfo.yearDays({ days: formatUpTo(days, days < 10 ? 1 : 0) })
    : t.worldInfo.yearYears({ years: formatUpTo(days / 365.25, 1) });
}

/**
 * Colour a biome has on the hillshaded biome map: its surface colour, or its
 * classification colour for worlds written before surface colours existed.
 */
export function surfaceColour(biome: Pick<BiomeInfo, 'color' | 'surface_color'>): string {
  return biome.surface_color || biome.color;
}
