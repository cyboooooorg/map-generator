import { describe, expect, it } from 'vitest';
import { SphereGeometry } from 'three';
import type { System } from './api/types/System';
import fixture from './systemSky.fixture.json';
import { libration, librationAmplitude, placeSystem } from './systemSky';
import { systemStarView } from './systemStar';
import type { Axes, Vec3 } from './skyBodies';
import { globeTurn, keplerPosition, orbitDirection, type SystemGiant } from './viewer/starSystem';

// The star view's planet globes are three.js SphereGeometry meshes wrapped
// in their maps, turned by globeTurn; the planet view turns the same
// bodies with placeSystem. Both must show the same face to the star.

const TWO_PI = 2 * Math.PI;
const DEG = Math.PI / 180;
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const unit = (a: Vec3): Vec3 => {
  const l = Math.hypot(...a);
  return [a[0] / l, a[1] / l, a[2] / l];
};
const angle = (a: Vec3, b: Vec3) => Math.acos(Math.min(1, Math.max(-1, dot(unit(a), unit(b)))));
const wrap = (x: number) => Math.atan2(Math.sin(x), Math.cos(x));

/** A geometry-frame vector turned into the star view's frame. */
const turned = (turn: Axes, v: Vec3): Vec3 => [0, 1, 2].map((i) => turn[0][i]! * v[0] + turn[1][i]! * v[1] + turn[2][i]! * v[2]) as Vec3;

/** The point of a SphereGeometry at texture coordinates (u, v), as the marker's geometry has it. */
function geometryPoint(u: number, v: number): Vec3 {
  const geometry = new SphereGeometry(1, 32, 16);
  const uv = geometry.getAttribute('uv');
  const position = geometry.getAttribute('position');
  for (let i = 0; i < uv.count; i += 1) {
    if (Math.abs(uv.getX(i) - u) < 1e-9 && Math.abs(uv.getY(i) - v) < 1e-9) {
      return [position.getX(i), position.getY(i), position.getZ(i)];
    }
  }
  throw new Error(`no vertex at uv ${u}, ${v}`);
}

/** Where giant `giant` stands seen from the star after `days`, in the star view's frame (as StarView.placeGiants). */
function fromStar(giant: SystemGiant, days: number): Vec3 {
  const mean = TWO_PI * (giant.phase + days / giant.year_days);
  return orbitDirection(giant, keplerPosition(giant.eccentricity, mean).trueAnomaly);
}

/** The map longitude and latitude under the star, in the star view (globe turned by globeTurn). */
function subStellarInStarView(giant: SystemGiant, days: number): { lon: number; lat: number } {
  const turn = globeTurn(giant.axes!(days));
  const toStar = unit(fromStar(giant, days).map((c) => -c) as Vec3);
  // Back into the geometry's frame (the turn is a rotation: its transpose).
  const local: Vec3 = [dot(turn[0], toStar), dot(turn[1], toStar), dot(turn[2], toStar)];
  // SphereGeometry: x = -cos(phi) sin(theta), z = sin(phi) sin(theta), u = phi / 2pi.
  const u = Math.atan2(local[2], -local[0]) / TWO_PI;
  return { lon: wrap(u * TWO_PI), lat: Math.asin(local[1]) };
}

/** The same in the planet view: from placeSystem, in the map's frame (geo.ts, lon = atan2(x, z)). */
function subStellarInPlanetView(system: System, id: string, days: number): { lon: number; lat: number } {
  const place = placeSystem(system, days).get(id)!;
  const toStar = unit(place.centre.map((c) => -c) as Vec3);
  const [x, y, z] = place.axes.map((a) => dot(a, toStar));
  return { lon: wrap(Math.atan2(x!, z!)), lat: Math.asin(y!) };
}

/** The fixture with planet b locked on an eccentric orbit, untilted, as planet c of mgs-58761-841956177. */
function lockedSystem(): System {
  const system = structuredClone(fixture) as unknown as System;
  const b = system.bodies.find((x) => x.id === 'b')!;
  b.spin.locked = true;
  b.spin.axial_tilt_deg = 0;
  b.orbit.eccentricity = 0.128;
  b.orbit.period_days = 39.8;
  b.spin.period_hours = 39.8 * 24;
  return system;
}

describe("the star view's planet globes", () => {
  it('keep a locked planet\'s longitude 180° towards the star all year round', () => {
    const system = lockedSystem();
    const b = system.bodies.find((x) => x.id === 'b')!;
    const view = systemStarView(system, b);
    const giant = view.giants.giants[view.ids.indexOf('b')]!;
    // Longitude 180°, the centre column of the map, on the equator.
    const centre = geometryPoint(0.5, 0.5);
    const amplitude = librationAmplitude(b.orbit.eccentricity);
    expect(amplitude).toBeGreaterThan(10 * DEG);
    for (const share of [0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875]) {
      const days = share * b.orbit.period_days;
      const facing = turned(globeTurn(giant.axes!(days)), centre);
      const toStar = fromStar(giant, days).map((c) => -c) as Vec3;
      const off = angle(facing, toStar);
      // Off the star by its libration only.
      expect(off, `day ${days}`).toBeLessThanOrEqual(amplitude + 0.5 * DEG);
      expect(off, `day ${days}`).toBeCloseTo(Math.abs(libration(b.orbit, days)), 3);
    }
  });

  it('show the star the same point of the map as the planet view, locked or turning', () => {
    const system = lockedSystem();
    const view = systemStarView(system, system.bodies.find((x) => x.id === 'b')!);
    for (const id of ['b', 'c']) {
      const giant = view.giants.giants[view.ids.indexOf(id)]!;
      for (const days of [0, 3.3, 10, 19.9, 29.85, 1234.5]) {
        const star = subStellarInStarView(giant, days);
        const planet = subStellarInPlanetView(system, id, days);
        expect(Math.abs(wrap(star.lon - planet.lon)), `${id} day ${days}`).toBeLessThan(0.1 * DEG);
        expect(Math.abs(star.lat - planet.lat), `${id} day ${days}`).toBeLessThan(0.1 * DEG);
      }
    }
  });

  it('turn a free planet with its day', () => {
    const system = lockedSystem();
    const c = system.bodies.find((x) => x.id === 'c')!;
    const view = systemStarView(system, c);
    const giant = view.giants.giants[view.ids.indexOf('c')]!;
    // A quarter of its day later, the star is over a point 90° further west.
    const quarter = c.spin.period_hours / 24 / 4;
    const before = subStellarInStarView(giant, 100);
    const after = subStellarInStarView(giant, 100 + quarter);
    // Its year is long: the star barely moves meanwhile.
    expect(Math.abs(wrap(before.lon - after.lon - Math.PI / 2))).toBeLessThan(0.5 * DEG);
    // Its spin axis, the globe's north, leans from its orbit's normal by its tilt.
    const north = turned(globeTurn(giant.axes!(100)), [0, 1, 0]);
    const normal = unit(cross(fromStar(giant, 100), fromStar(giant, 101)));
    expect(angle(north, normal)).toBeCloseTo(c.spin.axial_tilt_deg * DEG, 3);
  });
});

function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}
