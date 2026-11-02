import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { cliCommand, formatAgo, formatArea, formatDuration, formatShare, requestFromWorld } from './format';

const meta = {
  planet_type: 'frozen',
  seed: 2590618090,
  sea_level: 0.121345,
  volcanic_intensity: 0.61071,
  circumference_km: 51823.43,
  axial_tilt_deg: 18.7234,
  erosion: 1,
  width: 1920,
  height: 1080,
} as WorldMetadata;

describe('format', () => {
  it('rebuilds the request and the command line of a world', () => {
    expect(requestFromWorld(meta)).toEqual({
      planet: 'frozen',
      seed: 2590618090,
      sea_level: 0.1213,
      volcanic: 0.6107,
      circumference_km: 51823.4,
      axial_tilt_deg: 18.72,
      erosion: 1,
      width: 1920,
      height: 1080,
    });
    expect(cliCommand(meta)).toBe(
      'mise run gen -- --planet frozen --seed 2590618090 --sea-level 0.1213 --volcanic 0.6107 ' +
        '--circumference 51823.4 --axial-tilt 18.72 --erosion 1 --width 1920 --height 1080',
    );
  });

  it('formats shares, areas, durations and ages', () => {
    expect(formatShare(0.25)).toBe('25 %');
    expect(formatShare(0.0123)).toBe('1.2 %');
    expect(formatShare(0.0004)).toBe('<0.1 %');
    expect(formatArea(510_000_000)).toBe('510 M km²');
    expect(formatArea(12_345)).toBe('12,300 km²');
    expect(formatArea(1_130_000_000)).toBe('1,130 M km²');
    expect(formatArea(304.4)).toBe('304 km²');
    expect(formatDuration(42.4)).toBe('42 s');
    expect(formatDuration(185)).toBe('3 min 05 s');
    expect(formatAgo(1_000_000, 1_000_000 + 2 * 3_600_000)).toBe('2 hours ago');
    expect(formatAgo(1_000_000, 1_000_000 + 5_000)).toBe('just now');
  });
});
