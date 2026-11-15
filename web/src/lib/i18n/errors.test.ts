import { describe, expect, it } from 'vitest';
import { ApiFailure } from '../api/client';
import { detailText, errorField, errorText } from './errors';
import { setLocale } from './index.svelte';

describe('error messages', () => {
  it('words a detail from its code and values', () => {
    expect(detailText({ code: 'out_of_range', field: 'water_oceans', min: 0, max: 1000 })).toBe(
      'Water must be between 0 and 1,000.',
    );
    expect(detailText({ code: 'out_of_range', field: 'axial_tilt_deg', min: 0, max: 90 })).toBe(
      'Axial tilt must be between 0 and 90.',
    );
    expect(detailText({ code: 'unknown_archetype', archetype: 'terran' })).toBe('There is no planet archetype named terran.');
    expect(detailText({ code: 'out_of_range', field: 'mystery', min: 0.25, max: 2 })).toBe(
      'mystery must be between 0.25 and 2.',
    );
    expect(detailText({ code: 'busy' })).toMatch(/generation is running/);
    expect(detailText({ code: 'water_heavier_than_planet', water_oceans: 5000, mass_earth: 0.1 })).toMatch(
      /^5,000 Earth oceans of water weigh more than a planet of 0\.1 Earth masses/,
    );
    expect(detailText({ code: 'size_not_offered', width: 1000, height: 500 })).toBe(
      '1000 × 500 is not one of the offered map sizes.',
    );
    expect(detailText({ code: 'bad_age', age_gyr: 12, lifetime_gyr: 3.456 })).toBe(
      'The star is 12 Gyr old, but it lives 3.46 Gyr at most: lower the age, or pick a star that lives longer (a Solar System archetype sets the age to 4.57 Gyr).',
    );
    expect(detailText({ code: 'bad_archetype', archetype: 'titan-like' })).toMatch(/Titan.* is a giant's moon.*whole system/i);
    expect(detailText({ code: 'original_size' })).toMatch(/original size/);
    expect(detailText({ code: 'size_not_generated', width: 4096, height: 2048 })).toMatch(/no 4096 × 2048 map/);
  });

  it('words the age and archetype refusals in French too', () => {
    setLocale('fr');
    try {
      expect(detailText({ code: 'bad_age', age_gyr: 12, lifetime_gyr: 3.456 })).toMatch(/12 Gyr.*3,46 Gyr/);
      expect(detailText({ code: 'bad_archetype', archetype: 'titan-like' })).toMatch(/système entier/);
    } finally {
      setLocale('en');
    }
  });

  it('falls back to the server text, then to the error itself', () => {
    expect(errorText(new ApiFailure(500, 'disk full'))).toBe('disk full');
    expect(errorText(new ApiFailure(409, 'x', { code: 'system_exists', system: 'mgs-1-2' }))).toMatch(/^System mgs-1-2 exists/);
    expect(errorText(new Error('boom'))).toBe('Error: boom');
  });
});

describe('errorField', () => {
  it('names the field a refusal belongs to', () => {
    expect(errorField({ code: 'archetype_ages', archetypes: ['mars-like'] })).toBe('age_gyr');
    expect(errorField({ code: 'out_of_range', field: 'co2', min: 0, max: 1 })).toBe('co2');
    expect(errorField({ code: 'out_of_range', field: 'nh3', min: 0, max: 2 })).toBe('nh3');
    expect(errorField({ code: 'out_of_range', field: 's', min: 0, max: 2 })).toBe('s');
    expect(errorField({ code: 'out_of_range', field: 'hz_position', min: 0, max: 1 })).toBe('orbit');
    expect(errorField({ code: 'out_of_range', field: 'mystery', min: 0, max: 1 })).toBeNull();
    expect(errorField({ code: 'water_heavier_than_planet', water_oceans: 5, mass_earth: 0.1 })).toBe('water_oceans');
    expect(errorField({ code: 'bad_age', age_gyr: 12, lifetime_gyr: 3 })).toBe('age_gyr');
    expect(errorField({ code: 'bad_archetype', archetype: 'x' })).toBe('planet');
    expect(errorField({ code: 'unknown_archetype', archetype: 'x' })).toBe('planet');
    expect(errorField({ code: 'orbit_and_position' })).toBe('orbit');
    expect(errorField({ code: 'too_many_planets', asked: 9, max: 8 })).toBe('planets');
    expect(errorField({ code: 'zone_full', body_type: 'x', asked: 2, fits: 1 })).toBe('planets');
    expect(errorField({ code: 'type_impossible', body_type: 'x' })).toBe('planets');
    expect(errorField({ code: 'size_out_of_range', field: 'width', value: 1, min: 2, max: 3 })).toBe('size');
    expect(errorField({ code: 'too_many_pixels', width: 1, height: 1, max_pixels: 1 })).toBe('size');
    expect(errorField({ code: 'busy' })).toBeNull();
  });
});
