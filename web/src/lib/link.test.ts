import { describe, expect, it } from 'vitest';
import { lonLatToSphere } from './geo';
import { decodeCamera, decodeSystems, decodeView, decodeWorlds, encodeCamera, encodeView, linkHash, systemHash, type ViewLink } from './link';
import type { CameraState } from './viewer/PlanetView';

const RAD = Math.PI / 180;

function globeAt(lonDeg: number, latDeg: number, altitude: number): CameraState {
  const [x, y, z] = lonLatToSphere({ lon: lonDeg * RAD, lat: latDeg * RAD });
  const r = 1 + altitude;
  return { mode: 'globe', position: [x * r, y * r, z * r], target: [0, 0, 0], up: [0, 1, 0] };
}

function expectClose(a: readonly number[], b: readonly number[], digits = 4): void {
  expect(a.length).toBe(b.length);
  a.forEach((value, i) => expect(value).toBeCloseTo(b[i]!, digits));
}

const params = (hash: string) => new URLSearchParams(hash);

describe('encodeCamera / decodeCamera', () => {
  it('round-trips a globe camera as the point below it and its altitude', () => {
    const camera = globeAt(-71.06, 42.36, 0.05);
    const text = encodeCamera(camera);
    expect(text).toBe('g,-71.06,42.36,0.05');
    const back = decodeCamera(text)!;
    expect(back.mode).toBe('globe');
    expectClose(back.position, camera.position);
    expect(back.target).toEqual([0, 0, 0]);
    expect(back.up).toEqual([0, 1, 0]);
  });

  it('round-trips a map camera as the centre of the screen and the height', () => {
    const camera: CameraState = { mode: 'map', position: [1.2, -0.4, 0.75], target: [1.2, -0.4, 0], up: [0, 1, 0] };
    const back = decodeCamera(encodeCamera(camera))!;
    expect(back.mode).toBe('map');
    expectClose(back.position, camera.position, 3);
    expectClose(back.target, camera.target, 3);
  });

  it('keeps the map centre on the map across the antimeridian', () => {
    const centre: CameraState = { mode: 'map', position: [0, 0, 5.5], target: [0, 0, 0], up: [0, 1, 0] };
    expect(encodeCamera(centre)).toBe('m,-180,0,5.5');
    expectClose(decodeCamera('m,-180,0,5.5')!.target, [0, 0, 0]);
    expectClose(decodeCamera('m,180,0,5.5')!.target, [0, 0, 0]);
    expectClose(decodeCamera('m,0,0,5.5')!.target, [-Math.PI, 0, 0]);
  });

  it('wraps longitudes and keeps the camera within the controls', () => {
    expectClose(decodeCamera('g,540,0,1')!.position, globeAt(180, 0, 1).position);
    // Too close to the ground, too far, too close to the pole.
    expect(Math.hypot(...decodeCamera('g,0,0,0.000001')!.position)).toBeCloseTo(1.0003, 6);
    expect(Math.hypot(...decodeCamera('g,0,0,100')!.position)).toBeCloseTo(8);
    expect(decodeCamera('g,0,90,1')!.position[1]).toBeLessThan(2);
    expect(decodeCamera('m,0,0,99')!.position[2]).toBe(12);
  });

  it('rejects malformed values', () => {
    const malformed = ['', 'g', 'x,1,2,3', 'g,1,2', 'g,1,2,3,4', 'g,a,2,3', 'g,1,,3', 'g,1,91,3', 'g,1,2,0', 'g,1,2,-1'];
    for (const text of [...malformed, 'm,NaN,0,1', 'g,1,2,Infinity']) {
      expect(decodeCamera(text)).toBeNull();
    }
  });
});

describe('encodeView / decodeView', () => {
  it('round-trips every part', () => {
    const view: ViewLink = {
      camera: globeAt(10, 20, 0.5),
      layer: 'temperature',
      relief: 0.35,
      sun: { on: true, season: 0.25, longitudeDeg: 90 },
      orbitDays: 3.25,
      epoch: 3800,
    };
    const hash = encodeView(view).join('&');
    expect(hash).toBe('view=g,10,20,0.5&layer=temperature&relief=0.35&sun=0.25,90&orbit=3.25&epoch=3800');
    const back = decodeView(params(hash));
    expect(back.layer).toBe('temperature');
    expect(back.relief).toBe(0.35);
    expect(back.sun).toEqual({ on: true, season: 0.25, longitudeDeg: 90 });
    expect(back.epoch).toBe(3800);
    expect(back.orbitDays).toBe(3.25);
    // No days past the season is the default: not written.
    expect(encodeView({ orbitDays: 0 })).toEqual([]);
    // The present is the default: not written.
    expect(encodeView({ epoch: 0 })).toEqual([]);
    expect(decodeView(params('epoch=-500'))).toEqual({ epoch: -500 });
    expectClose(back.camera!.position, view.camera!.position);
    expect(decodeView(params(encodeView({ sun: { on: false } }).join('&')))).toEqual({ sun: { on: false } });
  });

  it('ignores the parts that are malformed, and keeps the others', () => {
    const view = decodeView(params('view=g,abc&layer=lava&relief=11&sun=0.5,400&world=x'));
    expect(view).toEqual({});
    const partial = decodeView(params('view=nope&layer=moisture&relief=-1&sun=0.5'));
    expect(partial).toEqual({ layer: 'moisture' });
    expect(decodeView(params('layer=toString&layer2=x'))).toEqual({});
    expect(decodeView(params('epoch=12.5'))).toEqual({});
    expect(decodeView(params('epoch=-2000'))).toEqual({});
    expect(decodeView(params('orbit=-1'))).toEqual({});
    expect(decodeView(params('orbit=abc'))).toEqual({});
    expect(decodeView(params('sun=1,0'))).toEqual({ sun: { on: true, season: 0, longitudeDeg: 0 } });
  });

  it('is empty for a link that names only worlds', () => {
    expect(decodeView(params('world=terran-1&compare=mars-2'))).toEqual({});
  });
});

describe('linkHash / decodeWorlds', () => {
  it('names the worlds, then the view', () => {
    const hash = linkHash('terran-1', 'mars like', { layer: 'elevation' });
    expect(hash).toBe('world=terran-1&compare=mars%20like&layer=elevation');
    expect(decodeWorlds(params(hash))).toEqual({ world: 'terran-1', compare: 'mars like' });
  });

  it('drops the compared world without a main one', () => {
    expect(linkHash(null, 'x')).toBe('');
    expect(linkHash('a', null)).toBe('world=a');
  });
});

describe('systemHash / decodeSystems', () => {
  it('names the system, the body shown and the compared system', () => {
    const hash = systemHash({ system: 'mgs-48213-42', body: 'c-1', compare: 'mgs-1-2' }, { layer: 'elevation' });
    expect(hash.startsWith('system=mgs-48213-42&body=c-1&compare=mgs-1-2&')).toBe(true);
    expect(decodeSystems(new URLSearchParams(hash))).toEqual({ system: 'mgs-48213-42', body: 'c-1', compare: 'mgs-1-2', pins: [] });
    expect(systemHash({ system: null, body: 'c', compare: 'x' })).toBe('');
    expect(decodeSystems(new URLSearchParams('world=terran-1'))).toEqual({ system: null, body: null, compare: null, pins: [] });
  });

  it('carries the pinned cards', () => {
    const hash = systemHash({ system: 'mgs-48213-42', body: null, compare: null, pins: ['c', 'e'] });
    expect(hash).toBe('system=mgs-48213-42&pins=c,e');
    expect(decodeSystems(new URLSearchParams(hash)).pins).toEqual(['c', 'e']);
    expect(systemHash({ system: 'mgs-48213-42', body: null, compare: null, pins: [] })).toBe('system=mgs-48213-42');
  });
});

