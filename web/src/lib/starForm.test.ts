import { describe, expect, it } from 'vitest';
import type { PresetInfo } from './api/types/PresetInfo';
import type { WorldRequest } from './api/types/WorldRequest';
import { applyStarChoice, initialStarChoice, parseStarChoice, parseStarType, starType } from './starForm';

const preset = (star: PresetInfo['star']) => ({ star }) as PresetInfo;

describe('star form', () => {
  it('reads and writes spectral types', () => {
    expect(parseStarType('G2V')).toEqual({ klass: 'G', subtype: 2 });
    expect(parseStarType('m4.5')).toEqual({ klass: 'M', subtype: 4.5 });
    expect(parseStarType('K')).toEqual({ klass: 'K', subtype: 5 });
    expect(parseStarType('O1V')).toBeNull();
    expect(parseStarType('X')).toBeNull();
    expect(starType('M', 4.5)).toBe('M4.5V');
  });

  it('starts from the varied world, then the preset, then the fallback', () => {
    const fromWorld = initialStarChoice(preset({ type: 'G2V', orbit_au: 1.524 }), { star: 'K3V', orbit_au: 0.7 }, [0.1, 0.9]);
    expect(fromWorld).toMatchObject({ klass: 'K', subtype: 3, orbitBy: 'au', orbitRandom: false, au: 0.7 });
    const mars = initialStarChoice(preset({ type: 'G2V', orbit_au: 1.524 }), null, [0.1, 0.9]);
    expect(mars).toMatchObject({ klass: 'G', subtype: 2, orbitBy: 'au', au: 1.524 });
    const terran = initialStarChoice(preset({ position: [0.1, 0.9] }), null, [0.1, 0.9]);
    expect(terran).toMatchObject({ klass: '', orbitBy: 'position', orbitRandom: true, position: 0.5 });
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
