import { afterEach, describe, expect, it } from 'vitest';
import type { WorldMetadata } from '../api/types/WorldMetadata';
import { formatAgo, formatArea, formatDuration, formatGeology, formatKm, formatNumber, formatShare } from '../format';
import { formatLonLat } from '../geo';
import { getLocale, setLocale, t } from './index.svelte';
import { initialLocale } from './locale';
import { biomeName } from './names.svelte';

/** Narrow no-break space. */
const S = ' ';

describe('French', () => {
  afterEach(() => setLocale('en'));

  it('is picked from the browser languages', () => {
    expect(initialLocale(null, ['fr-CA', 'en'])).toBe('fr');
  });

  it('switches the messages', () => {
    setLocale('fr');
    expect(getLocale()).toBe('fr');
    expect(t.app.newWorld).toBe('Nouveau monde');
    expect(t.stages.rivers).toBe('Tracé des rivières et des lacs');
    expect(biomeName({ key: 'SeaIce', name: 'Sea Ice' })).toBe('Banquise');
    setLocale('en');
    expect(t.app.newWorld).toBe('New world');
  });

  it('describes the geology in French', () => {
    setLocale('fr');
    const meta = { surface_age_ga: 3.8, preset: { geology: { regime: 'stagnant', wet_until: 3.5 } } };
    expect(formatGeology(meta as unknown as WorldMetadata)).toBe(`couvercle stagnant, surface de 3,8${S}Ga`);
    expect(formatGeology({ ...meta, surface_age_ga: 0.02 } as unknown as WorldMetadata)).toBe(
      `couvercle stagnant, surface de 20${S}Ma`,
    );
  });

    it('formats numbers, units and times the French way', () => {
    setLocale('fr');
    expect(formatNumber(1.5, 1)).toBe('1,5');
    expect(formatKm(2.25)).toBe(`2,3${S}km`);
    expect(formatKm(12_345)).toBe(`12${S}345${S}km`);
    expect(formatArea(3_200_000)).toBe(`3,2${S}M${S}km²`);
    expect(formatShare(0.42)).toBe(`42${S}%`);
    expect(formatDuration(185)).toBe(`3${S}min 05${S}s`);
    expect(formatAgo(Date.now() - 3 * 3_600_000)).toBe('il y a 3 heures');
    expect(formatLonLat({ lat: (12.5 * Math.PI) / 180, lon: (-45 * Math.PI) / 180 })).toBe(
      `12,5°${S}N${S}; 45,0°${S}O`,
    );
  });
});
