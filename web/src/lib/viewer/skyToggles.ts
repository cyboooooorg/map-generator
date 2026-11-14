// The sky's two switches of the icon bar, shared by the planet view
// (Viewer.svelte) and the system and star views (StarPane.svelte,
// StarSurfacePane.svelte): they drive one setting, which Viewer.svelte
// owns and remembers.

import type { Toggle } from '../../components/dock/ToggleBar.svelte';
import { t } from '../i18n/index.svelte';
import type { SkyChoice } from './skyBackdrop';
import type { SkyExposure } from './skyClouds';

export interface SkySetting {
  /** The sky drawn now. */
  sky: SkyChoice;
  /** Its rendering: as the eye sees it or as a long exposure. */
  exposure: SkyExposure;
  setSky: (choice: SkyChoice) => void;
  setExposure: (exposure: SkyExposure) => void;
  /** The choice the main button brings back after it turned the sky off (see skyToRestore). */
  restore: () => SkyChoice;
}

/**
 * The sky switch (its main button turns the sky off and back on, its
 * three squares pick the system's sky, the fixed one or none) and the
 * long exposure switch.
 */
export function skyToggles(s: SkySetting): [Toggle, Toggle] {
  return [
    {
      key: 'sky',
      label: t.viewer.sky,
      help: t.viewer.skyHelp,
      icon: 'sparkles',
      on: s.sky !== 'off',
      set: (v) => s.setSky(v ? s.restore() : 'off'),
      subs: [
        { key: 'system', label: t.viewer.skySystem, icon: 'sparkles', on: s.sky === 'system', set: () => s.setSky('system') },
        { key: 'fixed', label: t.viewer.skyFixed, icon: 'star', on: s.sky === 'fixed', set: () => s.setSky('fixed') },
        { key: 'off', label: t.viewer.skyOff, icon: 'close', on: s.sky === 'off', set: () => s.setSky('off') },
      ],
    },
    {
      key: 'skyExposure',
      label: t.viewer.longExposure,
      help: t.viewer.longExposureHelp,
      icon: 'sparkles',
      on: s.exposure === 'photo',
      set: (v) => s.setExposure(v ? 'photo' : 'eye'),
    },
  ];
}
