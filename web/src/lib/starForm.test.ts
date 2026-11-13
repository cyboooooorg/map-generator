import { describe, expect, it } from 'vitest';
import type { ArchetypeInfo } from './api/types/ArchetypeInfo';
import type { WorldRequest } from './api/types/WorldRequest';
import { applyStarChoice, initialStarChoice, parseStarChoice, parseStarType, starType } from './starForm';

const archetype = (orbit: Partial<ArchetypeInfo['orbit']>) => ({ orbit }) as ArchetypeInfo;

describe('star form', () => {
  it('reads and writes spectral types', () => {
    expect(parseStarType('G2V')).toEqual({ klass: 'G', subtype: 2 });
    expect(parseStarType('m4.5')).toEqual({ klass: 'M', subtype: 4.5 });
    expect(parseStarType('K')).toEqual({ klass: 'K', subtype: 5 });
    expect(parseStarType('O1V')).toBeNull();
    expect(parseStarType('X')).toBeNull();
    expect(starType('M', 4.5)).toBe('M4.5V');
  });

  it('starts from the varied world, then the archetype, then the fallback', () => {
    const mars = archetype({ star: 'G2V', insolation: [0.39, 0.47] });
    const fromWorld = initialStarChoice(mars, { star: 'K3V', orbit_au: 0.7 }, [0.1, 0.9]);
    expect(fromWorld).toMatchObject({ klass: 'K', subtype: 3, orbitBy: 'au', orbitRandom: false, au: 0.7 });
    // sqrt(1.023 / 0.43) = 1.542 AU, as the server places it.
    const fromMars = initialStarChoice(mars, null, [0.1, 0.9]);
    expect(fromMars).toMatchObject({ klass: 'G', subtype: 2, orbitBy: 'au', orbitRandom: false });
    expect(fromMars.au).toBeCloseTo(1.542, 2);
    const ocean = initialStarChoice(archetype({ insolation: [0.9, 1.1] }), null, [0.1, 0.9]);
    expect(ocean).toMatchObject({ klass: '', orbitBy: 'position', orbitRandom: true, position: 0.5 });
  });

  it('the_archetype_chooses_the_star', () => {
    const venus = initialStarChoice(archetype({ star: 'G2V', insolation: [1.7, 2.1] }), null, [0.1, 0.9]);
    expect(starType(venus.klass, venus.subtype)).toBe('G2V');
    expect(venus.orbitRandom).toBe(false);
    expect(venus.au).toBeCloseTo(0.733, 2);
  });

  it('puts only what is fixed into the request', () => {
    const request: WorldRequest = {};
    applyStarChoice(request, { klass: '', subtype: 2, orbitBy: 'position', orbitRandom: true, position: 0.5, au: 1, eccentricityRandom: true, eccentricity: 0.1 });
    expect(request).toEqual({});
    applyStarChoice(request, { klass: 'M', subtype: 3, orbitBy: 'position', orbitRandom: false, position: 0.4, au: 1, eccentricityRandom: false, eccentricity: 0.2 });
    expect(request).toEqual({ star: 'M3V', hz_position: 0.4, eccentricity: 0.2 });
  });

  it('refuses malformed remembered choices', () => {
    expect(parseStarChoice({ klass: 'Q', subtype: 1 })).toBeNull();
    expect(parseStarChoice(null)).toBeNull();
  });
});
