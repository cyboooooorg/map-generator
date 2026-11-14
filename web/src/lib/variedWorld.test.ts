import { describe, expect, it } from 'vitest';
import type { WorldRequest } from './api/types/WorldRequest';
import { isGiantWorld, keepModelInputs, keepsModelInputs, startsDrawn } from './variedWorld';

/** What `requestFromWorld` gives for planet c of a drawn system: no archetype, its inputs pinned. */
const drawnPlanet: WorldRequest = {
  seed: 12,
  mass_earth: 0.62,
  core_mass_fraction: 0.31,
  water_oceans: 0.002,
  envelope_fraction: 0,
  co2: 1.3,
  n2: 0.8,
  radiogenic: 1.1,
  life: false,
  star: 'K3V',
  orbit_au: 0.41,
  drift_ma: -100,
};

describe('a varied world', () => {
  it('a_varied_drawn_world_starts_drawn_from_the_star', () => {
    expect(startsDrawn(drawnPlanet, false)).toBe(true);
    expect(startsDrawn({ ...drawnPlanet, planet: 'earth-like' }, true)).toBe(false);
    // A blank form follows its memory.
    expect(startsDrawn(null, true)).toBe(true);
    expect(startsDrawn(null, undefined)).toBe(false);
  });

  it('a_varied_world_keeps_its_model_inputs', () => {
    // The form left on "drawn from the star": every input of planet c is sent.
    const request: WorldRequest = { seed: 99, star: 'K3V', orbit_au: 0.41, drift_ma: -100 };
    keepModelInputs(request, drawnPlanet, undefined);
    expect(request).toMatchObject({
      mass_earth: 0.62,
      core_mass_fraction: 0.31,
      water_oceans: 0.002,
      envelope_fraction: 0,
      co2: 1.3,
      n2: 0.8,
      radiogenic: 1.1,
      life: false,
    });
    // An archetype varied as itself keeps them too; a mass typed wins.
    const earth = { ...drawnPlanet, planet: 'earth-like' };
    const typed: WorldRequest = { planet: 'earth-like', mass_earth: 1.3 };
    keepModelInputs(typed, earth, 'earth-like');
    expect(typed).toMatchObject({ planet: 'earth-like', mass_earth: 1.3, co2: 1.3, life: false });
  });

  it('another_archetype_drops_the_varied_inputs', () => {
    const request: WorldRequest = { planet: 'jupiter-like' };
    keepModelInputs(request, drawnPlanet, undefined);
    expect(request).toEqual({ planet: 'jupiter-like' });
    const blank: WorldRequest = { planet: 'earth-like' };
    keepModelInputs(blank, null, undefined);
    expect(blank).toEqual({ planet: 'earth-like' });
    // An archetype this server no longer offers, then "drawn from the star":
    // not the varied planet either.
    const gone = { ...drawnPlanet, planet: 'old-one' };
    const drawn: WorldRequest = {};
    keepModelInputs(drawn, gone, undefined);
    expect(drawn).toEqual({});
    expect(keepsModelInputs(undefined, drawnPlanet, undefined)).toBe(true);
    expect(keepsModelInputs('earth-like', drawnPlanet, undefined)).toBe(false);
  });

  it('an_archetype_file_is_matched_by_its_catalog_spec', () => {
    // world.json names the archetype; the form sends its catalog spec.
    const mine = { ...drawnPlanet, planet: 'heavy-rock' };
    const request: WorldRequest = { planet: 'mine/heavy-rock.toml' };
    keepModelInputs(request, mine, 'mine/heavy-rock.toml');
    expect(request.mass_earth).toBe(0.62);
  });
});

describe('a giant world in the form', () => {
  /** What `requestFromWorld` gives for a gas giant of a drawn system: no archetype, its mass pinned. */
  const drawnGiant: WorldRequest = { seed: 4, mass_earth: 318, envelope_fraction: 0.9, star: 'G2V', orbit_au: 5.2 };
  const giantArchetype = { spec: 'jupiter-like', giant: true };
  const rockyArchetype = { spec: 'earth-like', giant: false };

  it('follows the archetype picked', () => {
    expect(isGiantWorld({ randomPlanet: false, archetype: giantArchetype, initial: null, initialSpec: undefined, variedGiant: false })).toBe(true);
    expect(isGiantWorld({ randomPlanet: false, archetype: rockyArchetype, initial: null, initialSpec: undefined, variedGiant: false })).toBe(false);
  });

  it('is a varied giant without archetype while its inputs are kept', () => {
    expect(isGiantWorld({ randomPlanet: true, archetype: rockyArchetype, initial: drawnGiant, initialSpec: undefined, variedGiant: true })).toBe(true);
  });

  it('is not a giant once another archetype is picked, or a planet drawn without the varied one', () => {
    expect(isGiantWorld({ randomPlanet: false, archetype: rockyArchetype, initial: drawnGiant, initialSpec: undefined, variedGiant: true })).toBe(false);
    expect(isGiantWorld({ randomPlanet: true, archetype: giantArchetype, initial: null, initialSpec: undefined, variedGiant: false })).toBe(false);
    expect(isGiantWorld({ randomPlanet: true, archetype: undefined, initial: drawnPlanet, initialSpec: undefined, variedGiant: false })).toBe(false);
  });
});
