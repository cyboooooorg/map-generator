import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { SystemSummary } from '../lib/api/types/SystemSummary';
import SystemCard from './SystemCard.svelte';

const system = (extra: Partial<SystemSummary> = {}): SystemSummary => ({
  id: 'mgs-1-7',
  seed: 7,
  width: 64,
  height: 32,
  star: 'G2V',
  designation: 'MGS 1',
  star_light: [1, 1, 1],
  hz_inner_au: 0,
  hz_outer_au: 0,
  planets: {},
  moons: 0,
  bodies: [],
  names: {},
  modified_ms: 0,
  pinned: false,
  outdated: false,
  ...extra,
});

const html = (s: SystemSummary): string =>
  render(SystemCard, {
    props: {
      system: s,
      compact: false,
      selecting: false,
      chosen: false,
      renaming: false,
      dirty: false,
      onOpen: () => {},
      onChanged: () => {},
      onRename: () => {},
      onRenameClose: () => {},
      onDelete: () => {},
      onToggle: () => {},
    },
  }).body;

describe('SystemCard', () => {
  it('shows an outdated system with its label and a delete button, and no open link', () => {
    const out = html(system({ outdated: true }));
    expect(out).toContain('Outdated: made by an older generator');
    expect(out).toMatch(/class="delete[ "]/);
    expect(out).not.toMatch(/<button class="open[ "]/);
    expect(out).not.toContain('data-rename');
    expect(out).not.toContain('data-export');
  });

  it('opens a current system', () => {
    const out = html(system());
    expect(out).toMatch(/<button class="open[ "]/);
    expect(out).not.toContain('Outdated');
  });
});
