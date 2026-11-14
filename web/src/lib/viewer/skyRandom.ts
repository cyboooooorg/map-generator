// Small deterministic random helpers shared by the sky modules.

/** Small deterministic generator (mulberry32), uniform in [0, 1). */
export function random(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let z = state;
    z = Math.imul(z ^ (z >>> 15), z | 1);
    z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
    return ((z ^ (z >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/**
 * A sub-seed of `seed` named by `label` (FNV-1a), so that each stream of a
 * sky (one per nebula kind) is independent: adding a stream reshuffles none
 * of the others.
 */
export function subSeed(seed: number, label: string): number {
  let h = 0x811c9dc5 ^ (seed >>> 0);
  h = Math.imul(h, 0x01000193);
  for (let i = 0; i < label.length; i += 1) {
    h ^= label.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  h ^= h >>> 15;
  h = Math.imul(h, 0x2c1b3c6d);
  h ^= h >>> 12;
  return h >>> 0;
}
