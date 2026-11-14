import { describe, expect, it } from 'vitest';
import { hostsAdded, planetsRequest } from './systemPlanets';

const hostOf = { 'titan-like': 'saturn-like', 'io-like': 'jupiter-like' };

describe('hostsAdded', () => {
  it('a_moon_whose_host_is_asked_adds_no_planet', () => {
    expect(hostsAdded({ 'titan-like': 1 }, { 'saturn-like': 1 }, hostOf)).toBe(0);
  });

  it('a_moon_alone_adds_its_host', () => {
    expect(hostsAdded({ 'titan-like': 2 }, {}, hostOf)).toBe(1);
    expect(hostsAdded({ 'titan-like': 1, 'io-like': 1 }, {}, hostOf)).toBe(2);
    expect(hostsAdded({ 'titan-like': 0 }, {}, hostOf)).toBe(0);
  });
});

describe('planetsRequest', () => {
  it('planets_and_moons_merge_into_one_map', () => {
    expect(planetsRequest({ 'earth-like': 1, 'mars-like': 0 }, { 'titan-like': 1, 'io-like': 0 })).toEqual({ 'earth-like': 1, 'titan-like': 1 });
  });

  it('is null when nothing is asked', () => {
    expect(planetsRequest({ 'earth-like': 0 }, {})).toBeNull();
  });
});
