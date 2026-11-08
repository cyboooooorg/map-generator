import { describe, expect, it } from 'vitest';
import type { WorldMetadata } from './api/types/WorldMetadata';
import { skyFromWorld } from './sky';
import { eclipse, halo, inFrontOfGlobe, pickShadowCasters, planetLook, skyOccluders } from './skyBodies';

describe('eclipses', () => {
  it('darkens the ground in the umbra, less in the penumbra, not elsewhere', () => {
    const sun: [number, number, number] = [0, 0, 1];
    const sunRadius = (0.533 / 2) * (Math.PI / 180);
    // The Moon between the Sun and Earth, over the point (0, 0, 1), near
    // its perigee (at 60 Earth radii its umbra falls just short: annular).
    const caster = { position: [0, 0, 56] as [number, number, number], radius: 0.2727 };
    const annular = eclipse([0, 0, 1], sun, { ...caster, position: [0, 0, 63] }, sunRadius);
    expect(annular).toBeGreaterThan(0.8);
    expect(annular).toBeLessThan(1);
    expect(eclipse([0, 0, 1], sun, caster, sunRadius)).toBeCloseTo(1, 6);
    expect(eclipse([0.3, 0, Math.sqrt(1 - 0.09)], sun, caster, sunRadius)).toBeLessThan(1);
    expect(eclipse([0.9, 0, Math.sqrt(1 - 0.81)], sun, caster, sunRadius)).toBe(0);
    // Behind the planet, the moon casts no shadow on the day side.
    expect(eclipse([0, 0, 1], sun, { ...caster, position: [0, 0, -56] }, sunRadius)).toBe(0);
  });
});

describe('halo', () => {
  const world = (pressure: number, atmosphere?: object) =>
    ({ pressure_bar: pressure, preset: { atmosphere } }) as unknown as WorldMetadata;

  it('gives an airless planet no halo', () => {
    expect(halo(skyFromWorld(world(0, { haze: 0 })))).toBeNull();
  });

  it("gives Earth's air a blue halo, a thin one", () => {
    const earth = halo(skyFromWorld(world(1)))!;
    expect(earth.colour[2]).toBe(1);
    expect(earth.colour[0]).toBeLessThan(0.7);
    expect(earth.thickness).toBeGreaterThan(0.02);
    expect(earth.thickness).toBeLessThan(0.06);
    expect(earth.strength).toBeGreaterThan(0.1);
    expect(earth.strength).toBeLessThanOrEqual(1);
  });

  it('brightens the halo of a denser air', () => {
    const thin = halo(skyFromWorld(world(0.006)))!;
    const dense = halo(skyFromWorld(world(92)))!;
    expect(dense.strength).toBeGreaterThan(thin.strength);
  });
});

describe('planetLook', () => {
  const world = (pressure: number, layers: object, clouds = 'water') =>
    ({ pressure_bar: pressure, layers, preset: { atmosphere: { rayleigh: [0.17, 0.41, 1], haze: 0.1, haze_colour: [1, 1, 1], greenhouse: 0, clouds, cloud_cover: 0.5 } } }) as unknown as WorldMetadata;
  const url = (file: string) => `/w/raw_data/${file}`;

  it('gives a cloudy world with air its halo, its cloud map and its cloud colour', () => {
    const look = planetLook(world(1, { clouds: { file: 'clouds.png' } }, 'sulfuric'), url);
    expect(look.halo).not.toBeNull();
    expect(look.clouds).toBe('/w/raw_data/clouds.png');
    expect(look.cloudTint).toEqual([0.9, 0.82, 0.55]);
  });

  it('gives a world without a cloud layer (a giant) no clouds, and an airless one no halo', () => {
    expect(planetLook(world(1, {}), url).clouds).toBeNull();
    expect(planetLook(world(0, { clouds: { file: 'clouds.png' } }), url).halo).toBeNull();
  });
});

describe('planetLook of a giant', () => {
  it('gives a giant no halo, whatever its nominal pressure', () => {
    const giant = { pressure_bar: 0.3, giant: {}, layers: {}, preset: { atmosphere: { rayleigh: [0.55, 0.5, 0.42], haze: 0.15, haze_colour: [0.9, 0.9, 0.7], greenhouse: 0, clouds: 'none', cloud_cover: 0 } } } as unknown as WorldMetadata;
    const look = planetLook(giant, (f) => f);
    expect(look.halo).toBeNull();
    expect(look.clouds).toBeNull();
  });
});

describe('pickShadowCasters', () => {
  const sun: [number, number, number] = [1, 0, 0];

  it('keeps a small moon between the body and the sun over far giants that cannot shadow it', () => {
    const moon = { position: [60, 0.5, 0] as [number, number, number], radius: 0.27 };
    const giants = [1, 2, 3, 4].map((k) => ({ position: [0, 5000 * k, 0] as [number, number, number], radius: 11 }));
    expect(pickShadowCasters([...giants, moon], sun, 0.0047)).toEqual([moon]);
  });

  it('leaves out bodies behind the body shown, and keeps the largest in its sky first', () => {
    const behind = { position: [-60, 0, 0] as [number, number, number], radius: 0.27 };
    const near = { position: [30, 0, 0] as [number, number, number], radius: 3 };
    const far = { position: [300, 0, 0] as [number, number, number], radius: 3 };
    expect(pickShadowCasters([far, behind, near], sun, 0.0047)).toEqual([near, far]);
  });
});

describe('inFrontOfGlobe', () => {
  const camera: [number, number, number] = [0, 0, 5];

  it('puts a moon between the camera and the body shown in front of it', () => {
    expect(inFrontOfGlobe(camera, [0, 0, 3], 0.2)).toBe(true);
  });

  it('keeps a moon beyond the body shown behind it', () => {
    expect(inFrontOfGlobe(camera, [0, 0, -60], 0.27)).toBe(false);
    // Off to the side, past the horizon's distance.
    expect(inFrontOfGlobe(camera, [30, 0, -10], 0.27)).toBe(false);
  });

  it('puts a near body off the line of sight in front, when closer than the horizon', () => {
    expect(inFrontOfGlobe(camera, [1.2, 0, 3.5], 0.1)).toBe(true);
  });
});

describe('skyOccluders', () => {
  const camera: [number, number, number] = [0, 0, 5];

  it('gives the bodies drawn at their size, the largest from the camera first', () => {
    const giant = { position: [0, 0, -100] as [number, number, number], radius: 40 };
    const moon = { position: [10, 0, 0] as [number, number, number], radius: 1 };
    const far = { position: [0, 0, -1000] as [number, number, number], radius: 20 };
    // About 0.39, 0.09 and 0.02 radians across.
    expect(skyOccluders(camera, [moon, far, giant], 1e-3, 4)).toEqual([giant, moon, far]);
    // At most as many as asked.
    expect(skyOccluders(camera, [moon, far, giant], 1e-3, 2)).toEqual([giant, moon]);
  });

  it('leaves out a body drawn as a marker, and one without a place', () => {
    const speck = { position: [0, 0, -1e6] as [number, number, number], radius: 1 };
    const moon = { position: [10, 0, 0] as [number, number, number], radius: 1 };
    // The speck's angular radius is about 1e-6, under the marker's 1e-3.
    expect(skyOccluders(camera, [speck, {}, moon], 1e-3, 4)).toEqual([moon]);
  });
});
