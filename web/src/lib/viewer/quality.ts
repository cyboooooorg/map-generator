// How much the views draw. Without a GPU (a VM, a server, a remote
// desktop), the browser runs WebGL on the CPU (SwiftShader, llvmpipe, the
// Microsoft Basic Render Driver): every pixel of every shader is computed
// there, and the full views crawl. The light profile draws less (fewer
// pixels, no antialiasing, a coarser globe, a cheaper sky, fewer frames);
// the worlds themselves do not change, only how they are drawn.
//
// A view takes its profile when it is made and keeps it (antialiasing is
// fixed with the WebGL context): switching the quality reloads the page
// (components/QualitySelect.svelte).
import { loadSetting, saveSetting } from '../settings';

/** What the user picked: the detected quality, or one forced. */
export type QualityChoice = 'auto' | 'high' | 'light';

/** The quality the views draw at. */
export type Quality = 'high' | 'light';

/** What a view draws, by quality. */
export interface QualityProfile {
  quality: Quality;
  /** Highest pixel ratio drawn at while the view is still (see pixelRatio.ts). */
  maxPixelRatio: number;
  /** Highest pixel ratio while it moves. */
  movingPixelRatio: number;
  /** Multisampled antialiasing (fixed with the WebGL context). */
  antialias: boolean;
  /** Frames per second at most while something moves; null for the display's rate. */
  maxFps: number | null;
  /** The camera's inertia, which keeps drawing frames after the pointer stops. */
  damping: boolean;
  /** Vertices across the planet view's base surface (720: 2 per degree). */
  surfaceSegments: number;
  /** The planet view's sky: single scattering through the shell, or a plain halo. */
  atmosphere: 'scattering' | 'halo';
  /** Share of the detail tile texture budget kept (see balanceTileBudgets). */
  tileBudget: number;
  /** Cloud shadows on the ground. */
  cloudShadows: boolean;
}

export const PROFILES: Record<Quality, QualityProfile> = {
  high: {
    quality: 'high',
    maxPixelRatio: 2,
    movingPixelRatio: 1.25,
    antialias: true,
    maxFps: null,
    damping: true,
    surfaceSegments: 720,
    atmosphere: 'scattering',
    tileBudget: 1,
    cloudShadows: true,
  },
  light: {
    quality: 'light',
    // Fewer than one drawn pixel per CSS pixel, scaled up by the browser:
    // about seven times fewer pixels than at 2.
    maxPixelRatio: 0.75,
    movingPixelRatio: 0.5,
    antialias: false,
    maxFps: 20,
    damping: false,
    surfaceSegments: 240,
    atmosphere: 'halo',
    tileBudget: 0.25,
    cloudShadows: false,
  },
};

/** Names of the renderers that run WebGL on the CPU, lower case. */
const SOFTWARE_RENDERERS = ['swiftshader', 'llvmpipe', 'softpipe', 'basic render driver', 'software'];

/** Whether a WebGL renderer's name is one that runs on the CPU. */
export function isSoftwareRenderer(name: string): boolean {
  const lower = name.toLowerCase();
  return SOFTWARE_RENDERERS.some((s) => lower.includes(s));
}

/** The WebGL renderer of this browser, and whether it runs on the CPU. */
export interface RendererInfo {
  /** Its name as the driver gives it (unmasked when the browser allows), '' when unknown. */
  name: string;
  /** Whether it runs on the CPU (or WebGL refuses a context that would be fast). */
  software: boolean;
}

let detected: RendererInfo | null = null;

/**
 * Probes WebGL once, on a canvas of its own: the renderer's name, and
 * whether the browser would only give a context with a major performance
 * caveat (its own way of saying "no GPU").
 */
export function detectRenderer(): RendererInfo {
  if (detected) return detected;
  let name = '';
  let caveat = false;
  try {
    const canvas = document.createElement('canvas');
    const fast = canvas.getContext('webgl2', { failIfMajorPerformanceCaveat: true });
    caveat = fast === null;
    const gl = fast ?? document.createElement('canvas').getContext('webgl2');
    if (gl) {
      const debug = gl.getExtension('WEBGL_debug_renderer_info');
      name = String(gl.getParameter(debug ? debug.UNMASKED_RENDERER_WEBGL : gl.RENDERER) ?? '');
      gl.getExtension('WEBGL_lose_context')?.loseContext();
    }
  } catch {
    // No WebGL at all: the views will fail on their own.
  }
  detected = { name, software: caveat || isSoftwareRenderer(name) };
  return detected;
}

/** The quality a choice gives, `software` saying whether WebGL runs on the CPU. */
export function resolveQuality(choice: QualityChoice, software: boolean): Quality {
  if (choice === 'auto') return software ? 'light' : 'high';
  return choice;
}

const CHOICE_KEY = 'quality';
const CHOICES: readonly QualityChoice[] = ['auto', 'high', 'light'];

/** The quality the user picked (`mapgen.quality`), auto by default. */
export function loadQualityChoice(): QualityChoice {
  const stored = loadSetting<string>(CHOICE_KEY, 'auto');
  return (CHOICES as readonly string[]).includes(stored) ? (stored as QualityChoice) : 'auto';
}

export function saveQualityChoice(choice: QualityChoice): void {
  saveSetting(CHOICE_KEY, choice);
}

/** The profile the views of this page draw with: the user's choice, else the detected one. */
export function currentProfile(): QualityProfile {
  return PROFILES[resolveQuality(loadQualityChoice(), detectRenderer().software)];
}
