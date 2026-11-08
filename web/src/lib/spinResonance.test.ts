import { afterEach, describe, expect, it } from 'vitest';
import { setLocale, t } from './i18n/index.svelte';
import { resonanceCycle, resonanceRatio, resonanceTurns } from './spinResonance';

describe('spin-orbit resonances', () => {
  it('reads the turns per orbit, the legacy flag and a locked body', () => {
    expect(resonanceTurns({})).toBe(0);
    expect(resonanceTurns({ spin_orbit_3_2: true })).toBe(1.5);
    expect(resonanceTurns({ spin_orbit_3_2: true, resonance_turns: 1.5 })).toBe(1.5);
    expect(resonanceTurns({ resonance_turns: 2 })).toBe(2);
    expect(resonanceTurns({ locked: true, resonance_turns: 2 })).toBe(0);
    expect(resonanceTurns({ locked: true, spin_orbit_3_2: true })).toBe(0);
  });

  it('names the ratio and splits it into turns and orbits', () => {
    expect(resonanceRatio(1.5)).toBe('3:2');
    expect(resonanceRatio(2)).toBe('2:1');
    expect(resonanceCycle('3:2')).toEqual({ turns: 3, orbits: 2 });
    expect(resonanceCycle('2:1')).toEqual({ turns: 2, orbits: 1 });
  });

  describe('help text', () => {
    afterEach(() => setLocale('en'));

    it('is grammatical for each ratio, in both languages', () => {
      expect(t.worldInfo.resonantHelp({ ratio: '3:2' })).toContain('turning 3 times every 2 orbits');
      expect(t.worldInfo.resonantHelp({ ratio: '2:1' })).toContain('turning 2 times every orbit ');
      setLocale('fr');
      expect(t.worldInfo.resonantHelp({ ratio: '3:2' })).toContain('3 tours toutes les 2 orbites');
      expect(t.worldInfo.resonantHelp({ ratio: '2:1' })).toContain('2 tours à chaque orbite');
      expect(t.viewer.dialResonant({ angle: '1°', time: '12:00', ratio: '2:1' })).toContain('2:1');
    });
  });
});
