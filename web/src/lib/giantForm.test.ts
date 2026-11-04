import { describe, expect, it } from 'vitest';
import { applyGiantChoice, initialGiantChoice } from './giantForm';
import type { WorldRequest } from './api/types/WorldRequest';

describe('giant form fields', () => {
  it('sends only what was asked for, and a giant world only its mass', () => {
    const drawn = initialGiantChoice(null);
    const empty: WorldRequest = {};
    applyGiantChoice(empty, drawn, false);
    expect(empty).toEqual({});

    const asked = { ...drawn, metallicityRandom: false, metallicity: 0.3, giants: 'none' as const, host: 'giant' as const, massRandom: false, mass: 95 };
    const rocky: WorldRequest = {};
    applyGiantChoice(rocky, asked, false);
    expect(rocky).toEqual({ metallicity: 0.3, giants: 'none', host: 'giant' });
    const giant: WorldRequest = {};
    applyGiantChoice(giant, asked, true);
    expect(giant).toEqual({ metallicity: 0.3, mass_earth: 95 });
  });

  it('starts from a varied world', () => {
    const choice = initialGiantChoice({ metallicity: -0.2, host: 'giant', mass_earth: 17 });
    expect(choice).toMatchObject({ metallicityRandom: false, metallicity: -0.2, host: 'giant', massRandom: false, mass: 17 });
  });
});
