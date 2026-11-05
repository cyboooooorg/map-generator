import { describe, expect, it } from 'vitest';
import { ApiFailure } from '../api/client';
import { detailText, errorText } from './errors';

describe('error messages', () => {
  it('words a detail from its code and values', () => {
    expect(detailText({ code: 'out_of_range', field: 'sea_level', min: -1, max: 1 })).toBe(
      'Sea level must be between -1 and 1.',
    );
    expect(detailText({ code: 'out_of_range', field: 'pressure_bar', min: 0, max: 1000 })).toBe(
      'Pressure must be between 0 and 1,000.',
    );
    expect(detailText({ code: 'out_of_range', field: 'mystery', min: 0.25, max: 2 })).toBe(
      'mystery must be between 0.25 and 2.',
    );
    expect(detailText({ code: 'busy' })).toMatch(/generation is running/);
  });

  it('falls back to the server text, then to the error itself', () => {
    expect(errorText(new ApiFailure(500, 'disk full'))).toBe('disk full');
    expect(errorText(new ApiFailure(409, 'x', { code: 'system_exists', system: 'mgs-1-2' }))).toMatch(/^System mgs-1-2 exists/);
    expect(errorText(new Error('boom'))).toBe('Error: boom');
  });
});
