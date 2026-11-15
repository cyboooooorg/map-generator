import { describe, expect, it } from 'vitest';
import type { Body } from './api/types/Body';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { cardsFor } from './bodyCards';

const body = (over: Record<string, unknown> = {}): Body =>
  ({ id: 'c', kind: 'planet', params: {}, model: { surface: {} }, generated: true, ...over }) as unknown as Body;
const meta = (over: Record<string, unknown> = {}): WorldMetadata => ({ ...over }) as unknown as WorldMetadata;

describe('cardsFor', () => {
  it('has no card without a body', () => {
    expect(cardsFor(null, null, false)).toEqual([]);
  });

  it('gives a body not generated its facts but no surface, legend or map card', () => {
    const cards = cardsFor(body({ generated: false }), null, false);
    expect(cards).toEqual(['header', 'orbit', 'interior', 'atmosphere', 'moons']);
  });

  it('gives a generated rocky planet every card, the surface once measured', () => {
    expect(cardsFor(body(), meta(), false)).not.toContain('surface');
    expect(cardsFor(body(), meta(), true)).toEqual(['header', 'orbit', 'interior', 'atmosphere', 'surface', 'legend', 'moons', 'map']);
  });

  it('gives a giant a sky but no surface', () => {
    const cards = cardsFor(body({ model: null }), meta({ giant: {} }), true);
    expect(cards).toContain('atmosphere');
    expect(cards).not.toContain('surface');
  });

  it('gives a moon the moons card only for a part in rings', () => {
    expect(cardsFor(body({ kind: 'moon', parent: 'c' }), meta(), true)).not.toContain('moons');
    expect(cardsFor(body({ kind: 'moon', parent: 'c', ring_role: 'edge' }), meta(), true)).toContain('moons');
  });

  it('gives a moon that feeds a dust ring the moons card, without a ring role', () => {
    const planet = body({ params: { given_rings: { dust: [{ source: 'c-1' }] } } });
    const moon = body({ id: 'c-1', kind: 'moon', parent: 'c' });
    expect(cardsFor(moon, meta(), true, planet)).toContain('moons');
    expect(cardsFor(moon, meta(), true, null)).not.toContain('moons');
  });
});
