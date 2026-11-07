// How much of each biome is covered by plants, for the vegetation colours
// of "Star light" (see the surface shader).
import type { Biome } from './api/types/Biome';
import type { BiomeInfo } from './api/types/BiomeInfo';

/** Share of the ground covered by plants, by biome; biomes left out have none. */
const SHARES: Partial<Record<Biome, number>> = {
  Forest: 1,
  Jungle: 1,
  Taiga: 0.9,
  Wetland: 0.8,
  Savanna: 0.6,
  Shrubland: 0.6,
  Plain: 0.6,
  Tundra: 0.35,
};

/** Share of vegetation of each biome id of a world, by id. */
export function vegetationShares(biomes: readonly Pick<BiomeInfo, 'id' | 'key'>[]): number[] {
  const shares: number[] = [];
  for (const biome of biomes) {
    shares[biome.id] = SHARES[biome.key] ?? 0;
  }
  return Array.from(shares, (share) => share ?? 0);
}

/**
 * The colour a biome's swatch takes to match the globe under "Star light",
 * as the surface shader colours it: `hex` (sRGB), the share of it that is
 * `plants` recoloured by `shift` (the cool and hot shares of
 * `vegetationShift` in viewer/PlanetView.ts: towards a dark purple a third
 * as bright, then towards a gold as bright), then lit by `tint`, the
 * star's light in linear RGB (the Sun white). The shader's lighting and
 * hillshading are left out: the swatch shows the colour in full light.
 */
export function starLitColour(
  hex: string,
  plants: number,
  shift: readonly [number, number],
  tint: readonly [number, number, number],
): string {
  const value = Number.parseInt(hex.replace('#', '').slice(0, 6), 16);
  if (!Number.isFinite(value)) return hex;
  // The shader's own conversion (toLinear in shaders.ts), and back.
  const linear = [value >> 16, (value >> 8) & 255, value & 255].map((c) => (c / 255) ** 2.2);
  const lum = 0.2126 * linear[0]! + 0.7152 * linear[1]! + 0.0722 * linear[2]!;
  const dark = [0.55, 0.28, 0.42].map((c) => c * lum);
  const gold = [1.35, 1.05, 0.3].map((c) => c * lum);
  const mix = (a: number, b: number, t: number) => a + (b - a) * t;
  const out = linear.map((c, i) => {
    const adapted = mix(mix(c, dark[i]!, shift[0]), gold[i]!, shift[1]);
    return mix(c, adapted, plants) * tint[i]!;
  });
  const byte = (c: number) => Math.round(255 * Math.min(1, Math.max(0, c)) ** (1 / 2.2));
  return `#${out.map((c) => byte(c).toString(16).padStart(2, '0')).join('')}`;
}
