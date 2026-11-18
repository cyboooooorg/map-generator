import { describe, expect, it } from 'vitest';
import { popoverAlign } from './popoverAlign';

const bounds = { left: 0, right: 300 };

describe('popoverAlign', () => {
  it('anchors at the start when the popover fits', () => {
    expect(popoverAlign({ left: 10, right: 30 }, bounds, 200)).toBe('start');
  });
  it('anchors at the end when it would cross the right edge', () => {
    expect(popoverAlign({ left: 270, right: 290 }, bounds, 200)).toBe('end');
  });
  it('keeps the start when neither side has room', () => {
    expect(popoverAlign({ left: 100, right: 120 }, bounds, 400)).toBe('start');
  });
});
