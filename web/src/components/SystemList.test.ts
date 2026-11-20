import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { SystemSummary } from '../lib/api/types/SystemSummary';
import SystemList from './SystemList.svelte';

const lone: SystemSummary = {
  id: 'mgs-9-3',
  seed: 3,
  width: 64,
  height: 32,
  star: 'O5V',
  designation: 'MGS 9',
  star_light: [0.6, 0.7, 1],
  hz_inner_au: 90,
  hz_outer_au: 160,
  planets: {},
  moons: 0,
  bodies: [],
  names: {},
  modified_ms: Date.now(),
  pinned: false,
  outdated: false,
  outdated_reason: null,
};

describe('SystemList', () => {
  it('lists a system with no planet, with its star and a "no planet" chip', () => {
    const out = render(SystemList, {
      props: {
        systems: [lone],
        selectedId: null,
        compareId: null,
        onSelect: () => {},
        onDelete: () => {},
        onRegenerate: () => {},
        onCompare: () => {},
        onChanged: () => {},
      },
    }).body;
    expect(out).toContain('MGS 9');
    expect(out).toContain('<b>O5V</b>');
    expect(out).toContain('No planet');
  });
});
