import { describe, expect, it } from 'vitest';
import { applyGiantChoice, initialGiantChoice, parseGiantChoice, rememberedGiantChoice } from './giantForm';
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
    const giant = initialGiantChoice({ metallicity: -0.2, host: 'giant', mass_earth: 17 }, true);
    expect(giant).toMatchObject({ metallicityRandom: false, metallicity: -0.2, host: 'giant', massRandom: false, mass: 17 });
  });

  it('a_rocky_worlds_mass_never_reaches_a_giant', () => {
    // Varying an Earth: its mass is not a giant's mass to offer.
    const earth = initialGiantChoice({ mass_earth: 1 }, false);
    expect(earth).toMatchObject({ massRandom: true, mass: 318 });
    const jupiter: WorldRequest = { planet: 'jupiter-like' };
    applyGiantChoice(jupiter, earth, true);
    expect(jupiter).not.toHaveProperty('mass_earth');
    // Nor through the form memory: a giant's mass set by hand is not kept.
    const asked = { ...initialGiantChoice(null), massRandom: false, mass: 95, metallicityRandom: false, metallicity: 0.2 };
    expect(rememberedGiantChoice(asked)).toMatchObject({ massRandom: true, mass: 318, metallicityRandom: false, metallicity: 0.2 });
    expect(parseGiantChoice({ massRandom: false, mass: 1 })).toMatchObject({ massRandom: true, mass: 318 });
  });

  it('draws a field left empty', () => {
    const request: WorldRequest = {};
    // An emptied number input binds null.
    const emptied = { ...initialGiantChoice(null), massRandom: false, mass: null as unknown as number };
    applyGiantChoice(request, emptied, true);
    expect(request).toEqual({});
  });

  it('reads a remembered choice, drawing what is malformed', () => {
    expect(parseGiantChoice({ metallicityRandom: false, metallicity: -0.2, giants: 'none', host: 'giant', massRandom: false, mass: 17 })).toEqual({
      metallicityRandom: false,
      metallicity: -0.2,
      giants: 'none',
      alone: false,
      host: 'giant',
      massRandom: true,
      mass: 318,
    });
    expect(parseGiantChoice({ metallicity: 'high', giants: 'many', host: 'moon', mass: -3 })).toEqual(initialGiantChoice(null));
    expect(parseGiantChoice(undefined)).toEqual(initialGiantChoice(null));
  });

  it('asks for a lone planet, which overrides the giant neighbours', () => {
    expect(initialGiantChoice(null).alone).toBe(false);
    expect(initialGiantChoice({ alone: true }).alone).toBe(true);
    const alone = { ...initialGiantChoice(null), alone: true, giants: 'none' as const };
    const rocky: WorldRequest = {};
    applyGiantChoice(rocky, alone, false);
    expect(rocky).toEqual({ alone: true });
    const giant: WorldRequest = {};
    applyGiantChoice(giant, alone, true);
    expect(giant).toEqual({ alone: true });
    const normal: WorldRequest = {};
    applyGiantChoice(normal, { ...alone, alone: false }, false);
    expect(normal).toEqual({ giants: 'none' });
    expect(parseGiantChoice({ alone: true }).alone).toBe(true);
    expect(parseGiantChoice({ alone: 'yes' }).alone).toBe(false);
  });
});
