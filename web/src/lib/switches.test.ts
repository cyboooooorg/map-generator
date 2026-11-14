import { describe, expect, it } from 'vitest';
import { inScope, localGroup, migrateDisplay, nextSky, parseExposure, partsOn, pressBody, pressParts, pressScope, scopeOn, skyGroups } from './switches';

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

describe('skyGroups', () => {
  it('splits a moon\'s sky into its host and the other moons', () => {
    const groups = skyGroups(bodies, 'a-1');
    expect(groups.host).toBe('a');
    expect([...groups.moons]).toEqual(['a-2']);
  });

  it('gives a planet no host and all its moons', () => {
    const groups = skyGroups(bodies, 'a');
    expect(groups.host).toBeNull();
    expect([...groups.moons].sort()).toEqual(['a-1', 'a-2']);
  });

  it('is empty for a planet without moons, and for an unknown body', () => {
    expect(skyGroups(bodies, 'c')).toEqual({ host: null, moons: new Set() });
    expect(skyGroups(bodies, 'zz')).toEqual({ host: null, moons: new Set() });
  });
});

describe('partsOn and pressParts', () => {
  const on = (v: boolean, shown = true) => ({ on: v, shown });

  it('reads a lead switch by its first part, and presses all to its opposite', () => {
    expect(partsOn([on(false), on(true), on(true)], true)).toBe(false);
    expect(pressParts([on(false), on(true), on(true)], true)).toBe(true);
    expect(pressParts([on(true), on(false), on(false)], true)).toBe(false);
  });

  it('reads a scoped switch as on when any part shown is', () => {
    expect(partsOn([on(false), on(true), on(false)], false)).toBe(true);
    expect(pressParts([on(false), on(true), on(false)], false)).toBe(false);
    expect(pressParts([on(false), on(false), on(false)], false)).toBe(true);
  });

  it('ignores a part with nothing to draw', () => {
    expect(partsOn([on(false), on(true, false)], false)).toBe(false);
    expect(pressParts([on(false), on(true, false)], false)).toBe(true);
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

  it('splits the local labels, the small bodies and the spin switches again (#430)', () => {
    const out = migrateDisplay(
      { skyLabels: false, skyLabelsLocal: true, smallBodies: true, smallBodiesLocal: false, spinAxes: true, spinDirection: false },
      current,
    );
    expect(out).toMatchObject({
      skyLabelsMoons: true,
      skyLabelsHost: true,
      trojansOwn: false,
      trojansOther: true,
      skyBelts: true,
      otherSpinAxes: true,
      otherSpinDirection: false,
    });
  });

  it('gives the vectors of the body shown the link to the host they came with', () => {
    expect(migrateDisplay({ hostLink: true }, current)).toMatchObject({ vectors: true });
    expect(migrateDisplay({ spinAxes: true }, current)).toMatchObject({ vectors: true });
  });

  it('carries a value through both splits', () => {
    const out = migrateDisplay({ skyLabels: false, smallBodies: true, spinAxes: true }, current);
    expect(out).toMatchObject({ skyLabelsMoons: false, skyLabelsHost: false, trojansOwn: true, trojansOther: true, otherSpinDirection: true });
  });

  it('keeps the defaults when nothing was saved', () => {
    expect(migrateDisplay(null, current)).toEqual(current);
  });
});

describe('the sky choice', () => {
  const current = { sky: 'system', spinAxes: false };

  it('migrates the stars switch: on to the system’s sky, off to none', () => {
    expect(migrateDisplay({ stars: true }, current)).toMatchObject({ sky: 'system' });
    expect(migrateDisplay({ stars: false }, current)).toMatchObject({ sky: 'off' });
  });

  it('keeps a saved choice and falls back to the system’s sky for an unknown one', () => {
    expect(migrateDisplay({ sky: 'fixed' }, current)).toMatchObject({ sky: 'fixed' });
    expect(migrateDisplay({ sky: 'galaxy' }, current)).toMatchObject({ sky: 'system' });
    expect(migrateDisplay({}, current)).toMatchObject({ sky: 'system' });
  });

  it('cycles system, fixed, none', () => {
    expect(nextSky('system')).toBe('fixed');
    expect(nextSky('fixed')).toBe('off');
    expect(nextSky('off')).toBe('system');
  });
});

describe('parseExposure', () => {
  it('gives a stored display without the key the long exposure', () => {
    expect(parseExposure(undefined)).toBe('photo');
    expect(parseExposure('photo')).toBe('photo');
    expect(parseExposure('eye')).toBe('eye');
    expect(parseExposure(42)).toBe('photo');
  });
});
