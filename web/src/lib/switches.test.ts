import { describe, expect, it } from 'vitest';
import { inScope, localGroup, migrateDisplay, pressBody, pressScope, scopeOn } from './switches';

describe('pressBody', () => {
  it('sets both bodies to the opposite of the body shown', () => {
    expect(pressBody({ body: false, host: false })).toEqual({ body: true, host: true });
    expect(pressBody({ body: true, host: true })).toEqual({ body: false, host: false });
  });

  it('goes by the body shown, not by the host', () => {
    expect(pressBody({ body: true, host: false })).toEqual({ body: false, host: false });
    expect(pressBody({ body: false, host: true })).toEqual({ body: true, host: true });
  });

  it('leaves the squares independent: a square flips one field only', () => {
    const pair = { body: true, host: true };
    expect({ ...pair, host: !pair.host }).toEqual({ body: true, host: false });
    expect({ ...pair, body: !pair.body }).toEqual({ body: false, host: true });
  });
});

describe('pressScope', () => {
  it('sets both scopes to the opposite of what the button displays', () => {
    expect(pressScope({ local: false, stellar: false })).toEqual({ local: true, stellar: true });
    expect(pressScope({ local: true, stellar: true })).toEqual({ local: false, stellar: false });
  });

  it('counts a half-lit switch as on, so that a press turns it off', () => {
    expect(scopeOn({ local: true, stellar: false })).toBe(true);
    expect(pressScope({ local: false, stellar: true })).toEqual({ local: false, stellar: false });
  });
});

const bodies = [
  { id: 'a', parent: null },
  { id: 'a-1', parent: 'a' },
  { id: 'a-2', parent: 'a' },
  { id: 'b', parent: null },
  { id: 'b-1', parent: 'b' },
  { id: 'c', parent: null },
];

describe('localGroup', () => {
  it('is the host and the other moons seen from a moon', () => {
    expect([...localGroup(bodies, 'a-1')].sort()).toEqual(['a', 'a-2']);
  });

  it('is the moons seen from their planet', () => {
    expect([...localGroup(bodies, 'a')].sort()).toEqual(['a-1', 'a-2']);
  });

  it('is empty for a planet without moons, and for an unknown body', () => {
    expect(localGroup(bodies, 'c').size).toBe(0);
    expect(localGroup(bodies, 'zz').size).toBe(0);
  });
});

describe('inScope', () => {
  const group = localGroup(bodies, 'a-1');

  it('filters the local group by local and the rest by stellar', () => {
    expect(inScope({ local: true, stellar: false }, group, 'a')).toBe(true);
    expect(inScope({ local: true, stellar: false }, group, 'b')).toBe(false);
    expect(inScope({ local: false, stellar: true }, group, 'a-2')).toBe(false);
    expect(inScope({ local: false, stellar: true }, group, 'c')).toBe(true);
  });
});

describe('migrateDisplay', () => {
  const current = {
    spinAxes: false,
    spinDirection: false,
    hostLink: false,
    hostSpinAxes: false,
    hostSpinDirection: false,
    skyLabels: true,
    skyLabelsLocal: true,
    orbitLines: false,
    orbitLinesLocal: false,
    smallBodies: false,
    smallBodiesLocal: false,
    cloudDrift: false,
    hostCloudDrift: false,
  };

  it('gives the three spin switches the saved spinAxes', () => {
    const out = migrateDisplay({ spinAxes: true }, { ...current, spinAxes: true });
    expect(out).toMatchObject({ spinAxes: true, spinDirection: true, hostLink: true, hostSpinAxes: true, hostSpinDirection: true });
  });

  it('keeps the system-wide switches and the drift as they were', () => {
    const out = migrateDisplay({ skyLabels: false, orbitLines: true, smallBodies: true, cloudDrift: true }, current);
    expect(out).toMatchObject({ skyLabelsLocal: false, orbitLinesLocal: true, smallBodiesLocal: true, hostCloudDrift: true });
  });

  it('leaves a value saved under the new keys alone', () => {
    const out = migrateDisplay({ spinAxes: true, spinDirection: false }, { ...current, spinDirection: false });
    expect(out.spinDirection).toBe(false);
  });

  it('keeps the defaults when nothing was saved', () => {
    expect(migrateDisplay(null, current)).toEqual(current);
  });
});
