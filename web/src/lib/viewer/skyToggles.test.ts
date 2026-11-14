import { describe, expect, it } from 'vitest';
import type { SkyChoice } from './skyBackdrop';
import type { SkyExposure } from './skyClouds';
import { skyToggles } from './skyToggles';

/** A shared setting as Viewer.svelte holds it, with the choice brought back after 'off'. */
function setting(sky: SkyChoice = 'system', exposure: SkyExposure = 'photo') {
  const s = { sky, exposure, remembered: 'system' as SkyChoice };
  const toggles = () =>
    skyToggles({
      sky: s.sky,
      exposure: s.exposure,
      setSky: (c) => {
        if (s.sky !== 'off') s.remembered = s.sky;
        s.sky = c;
      },
      setExposure: (e) => (s.exposure = e),
      restore: () => s.remembered,
    });
  return { s, toggles };
}

describe('skyToggles', () => {
  it('gives the sky switch with its three squares and the long exposure switch', () => {
    const { toggles } = setting();
    const [sky, exposure] = toggles();
    expect(sky.key).toBe('sky');
    expect(sky.on).toBe(true);
    expect(sky.subs!.map((x) => [x.key, x.on])).toEqual([
      ['system', true],
      ['fixed', false],
      ['off', false],
    ]);
    expect(exposure.key).toBe('skyExposure');
    expect(exposure.on).toBe(true);
  });

  it('picks a sky from the squares and brings the last one back from the main button', () => {
    const { s, toggles } = setting();
    toggles()[0].subs!.find((x) => x.key === 'fixed')!.set(true);
    expect(s.sky).toBe('fixed');
    toggles()[0].set(false);
    expect(s.sky).toBe('off');
    expect(toggles()[0].on).toBe(false);
    toggles()[0].set(true);
    expect(s.sky).toBe('fixed');
  });

  it('switches the long exposure', () => {
    const { s, toggles } = setting();
    toggles()[1].set(false);
    expect(s.exposure).toBe('eye');
    expect(toggles()[1].on).toBe(false);
    toggles()[1].set(true);
    expect(s.exposure).toBe('photo');
  });
});
