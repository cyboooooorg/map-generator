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
  outdated_reason: null,
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
      onRegenerate: () => {},
      onToggle: () => {},
    },
  }).body;

describe('SystemCard', () => {
  it('shows an outdated system with its label and a delete button, and no open link', () => {
    const out = html(system({ outdated: true, outdated_reason: 'system_format' }));
    expect(out).toContain('Outdated: made by an older generator');
    expect(out).toMatch(/class="delete[ "]/);
    expect(out).not.toMatch(/<button class="open[ "]/);
    expect(out).not.toContain('data-rename');
    expect(out).not.toContain('data-export');
  });

  it('puts a regenerate button next to delete on an outdated system', () => {
    const out = html(system({ outdated: true, outdated_reason: 'system_format' }));
    expect(out).toContain('data-regenerate="mgs-1-7"');
    expect(out).toContain('Draw this system again from its seed');
  });

  it('keeps a system with outdated worlds openable, labelled, with a regenerate button', () => {
    const out = html(system({ outdated: true, outdated_reason: 'world_format' }));
    expect(out).toMatch(/<button class="open[ "]/);
    expect(out).toContain('Outdated worlds: made by an older generator');
    expect(out).toContain('data-regenerate="mgs-1-7"');
    expect(out).toContain('data-rename');
    expect(out).toContain('Generate its outdated worlds again');
  });

  it('opens a current system', () => {
    const out = html(system());
    expect(out).toMatch(/<button class="open[ "]/);
    expect(out).not.toContain('Outdated');
    expect(out).not.toContain('data-regenerate');
  });
});
