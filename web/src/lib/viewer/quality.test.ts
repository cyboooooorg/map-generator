import { describe, expect, it } from 'vitest';
import { fullPixelRatio, pixelRatio } from './pixelRatio';
import { STRANDS } from './prominenceGeometry';
import { isSoftwareRenderer, PROFILES, resolveQuality } from './quality';

describe('quality', () => {
  it('tells the renderers that run WebGL on the CPU', () => {
    expect(isSoftwareRenderer('ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)')).toBe(true);
    expect(isSoftwareRenderer('llvmpipe (LLVM 15.0.7, 256 bits)')).toBe(true);
    expect(isSoftwareRenderer('ANGLE (Microsoft, Microsoft Basic Render Driver Direct3D11)')).toBe(true);
    expect(isSoftwareRenderer('ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Pro, Unspecified Version)')).toBe(false);
    expect(isSoftwareRenderer('ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Direct3D11)')).toBe(false);
  });

  it('draws light on a CPU renderer unless asked otherwise', () => {
    expect(resolveQuality('auto', true)).toBe('light');
    expect(resolveQuality('auto', false)).toBe('high');
    expect(resolveQuality('high', true)).toBe('high');
    expect(resolveQuality('light', false)).toBe('light');
  });

  it('draws fewer pixels in the light profile, whatever the screen', () => {
    const light = PROFILES.light;
    expect(fullPixelRatio(2, light)).toBe(0.75);
    expect(fullPixelRatio(0.5, light)).toBe(0.5);
    expect(pixelRatio(2, 0, light)).toBe(0.5);
    expect(fullPixelRatio(3, PROFILES.high)).toBe(2);
  });

  it('keeps the full drawing in the high profile and lightens the costly passes in the light one', () => {
    const { high, light } = PROFILES;
    expect(high).toMatchObject({ surfaceSegments: 720, atmosphere: 'scattering', tileBudget: 1, cloudShadows: true, starSegments: 128, prominenceStrands: STRANDS, starRegions: 32, loopRegions: 8, ringRocks: true });
    expect(light).toMatchObject({ surfaceSegments: 240, atmosphere: 'halo', tileBudget: 0.25, cloudShadows: false, starSegments: 48, prominenceStrands: 4, starRegions: 12, loopRegions: 3, ringRocks: false });
    // Meshes have half as many rows as columns.
    expect(light.surfaceSegments % 2).toBe(0);
    expect(light.starSegments % 2).toBe(0);
  });
});
