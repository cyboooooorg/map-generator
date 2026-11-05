// The interface's themes: each a set of CSS custom properties written on
// the document root (see applyTheme), which app.css and the components
// read. Default is the original look; the others are game-like HUDs.
// Colours that carry meaning (the habitable zone, the frost line, the
// velocity and pull arrows, the map layers) are not tokens: they keep
// their hue in every theme.
import { loadSetting, saveSetting } from './settings';

export type ThemeName = 'default' | 'holo' | 'station' | 'cyberpunk' | 'military';

/** The themes, in the picker's order. */
export const THEME_NAMES: readonly ThemeName[] = ['default', 'holo', 'station', 'cyberpunk', 'military'];

/** Every token a theme sets, without its `--` prefix. */
export const TOKENS = [
  'bg',
  'panel',
  'panel-solid',
  'border',
  'text',
  'muted',
  'accent',
  'accent-text',
  'link',
  'danger',
  'control-bg',
  'font-ui',
  'font-display',
  'radius',
  'frame-clip',
  'frame-deco',
  'glow',
  'label-bg',
  'label-text',
  'overlay-orbit',
  'overlay-selected',
  'overlay-line',
  'overlay-muted',
] as const;

export type Token = (typeof TOKENS)[number];
export type ThemeTokens = Record<Token, string>;

const SYSTEM_FONT = "system-ui, -apple-system, 'Segoe UI', sans-serif";
/** A panel's corners cut at 45°, `size` px. */
const cut = (size: number) =>
  `polygon(${size}px 0, calc(100% - ${size}px) 0, 100% ${size}px, 100% calc(100% - ${size}px), calc(100% - ${size}px) 100%, ${size}px 100%, 0 calc(100% - ${size}px), 0 ${size}px)`;
/** Bracket corners in `colour`, drawn over a panel (see .hud in app.css). */
const brackets = (colour: string, length = 14, width = 2) =>
  [
    `linear-gradient(to right, ${colour} ${length}px, transparent ${length}px) top left / 100% ${width}px no-repeat`,
    `linear-gradient(to bottom, ${colour} ${length}px, transparent ${length}px) top left / ${width}px 100% no-repeat`,
    `linear-gradient(to left, ${colour} ${length}px, transparent ${length}px) bottom right / 100% ${width}px no-repeat`,
    `linear-gradient(to top, ${colour} ${length}px, transparent ${length}px) bottom right / ${width}px 100% no-repeat`,
  ].join(', ');

export const THEMES: Record<ThemeName, ThemeTokens> = {
  // The original look: these values are app.css's own.
  default: {
    bg: '#0b0e14',
    panel: '#141924ee',
    'panel-solid': '#141924',
    border: '#2a3242',
    text: '#e6e9ef',
    muted: '#8b95a7',
    accent: '#336de0',
    'accent-text': '#ffffff',
    link: '#4f8cff',
    danger: '#ff6b6b',
    'control-bg': '#1b2130',
    'font-ui': SYSTEM_FONT,
    'font-display': SYSTEM_FONT,
    radius: '6px',
    'frame-clip': 'none',
    'frame-deco': 'none',
    // No glow (a shadow list item, so components can add theirs to it).
    glow: '0 0 0 transparent',
    'label-bg': 'rgb(5 7 13 / 0.7)',
    'label-text': '#e8dcc4',
    'overlay-orbit': '#c8d2e6',
    'overlay-selected': '#e6ecf8',
    'overlay-line': '#ffe3a8',
    'overlay-muted': '#8a93a8',
  },
  // Cyan holographic lines on near black, cut corners, a technical font.
  holo: {
    bg: '#02080d',
    panel: '#04141cdd',
    'panel-solid': '#04141c',
    border: '#1f7f99',
    text: '#d9fbff',
    muted: '#7fc4d4',
    accent: '#00b8d9',
    'accent-text': '#001218',
    link: '#5ee7ff',
    danger: '#ff7a88',
    'control-bg': '#062330',
    'font-ui': `'Rajdhani', ${SYSTEM_FONT}`,
    'font-display': `'Orbitron', 'Rajdhani', ${SYSTEM_FONT}`,
    radius: '0',
    'frame-clip': cut(10),
    'frame-deco': brackets('#5ee7ff', 18, 1),
    glow: 'inset 0 0 18px #00d4ff2e',
    'label-bg': 'rgb(2 16 24 / 0.78)',
    'label-text': '#d9fbff',
    'overlay-orbit': '#4fd6f0',
    'overlay-selected': '#bff6ff',
    'overlay-line': '#7fe9ff',
    'overlay-muted': '#3f8ea0',
  },
  // Amber and orange, a station's cockpit: bracket frames, reticle-like corners.
  station: {
    bg: '#0d0804',
    panel: '#1a1006e6',
    'panel-solid': '#1a1006',
    border: '#8a5a1c',
    text: '#ffe9c7',
    muted: '#d9a865',
    accent: '#ff9a1f',
    'accent-text': '#1a0d00',
    link: '#ffbe5c',
    danger: '#ff6a5a',
    'control-bg': '#24170a',
    'font-ui': `'Exo 2', ${SYSTEM_FONT}`,
    'font-display': `'Exo 2', ${SYSTEM_FONT}`,
    radius: '2px',
    'frame-clip': cut(6),
    'frame-deco': brackets('#ffb347', 16, 2),
    glow: 'inset 0 0 16px #ff9a1f24',
    'label-bg': 'rgb(22 13 4 / 0.8)',
    'label-text': '#ffe9c7',
    'overlay-orbit': '#d9a865',
    'overlay-selected': '#ffd59a',
    'overlay-line': '#ffc777',
    'overlay-muted': '#8a6a3c',
  },
  // Neon magenta and yellow, diagonal cuts, wide type, a glitch on hover.
  cyberpunk: {
    bg: '#0a0410',
    panel: '#160a20e6',
    'panel-solid': '#160a20',
    border: '#ff2bd6',
    text: '#fdf6ff',
    muted: '#d2b3e8',
    accent: '#fcee0a',
    'accent-text': '#1a0f00',
    link: '#00f0ff',
    danger: '#ff5577',
    'control-bg': '#22102e',
    'font-ui': `'Chakra Petch', ${SYSTEM_FONT}`,
    'font-display': `'Audiowide', 'Chakra Petch', ${SYSTEM_FONT}`,
    radius: '0',
    'frame-clip': 'polygon(16px 0, 100% 0, 100% calc(100% - 16px), calc(100% - 16px) 100%, 0 100%, 0 16px)',
    'frame-deco':
      'repeating-linear-gradient(-45deg, #fcee0a 0 6px, transparent 6px 12px) top right / 48px 4px no-repeat, linear-gradient(#ff2bd6, #ff2bd6) bottom left / 40% 2px no-repeat',
    glow: 'inset 0 0 18px #ff2bd630',
    'label-bg': 'rgb(18 6 26 / 0.82)',
    'label-text': '#fdf6ff',
    'overlay-orbit': '#c46bff',
    'overlay-selected': '#fcee0a',
    'overlay-line': '#ff7ae6',
    'overlay-muted': '#7a4f99',
  },
  // Phosphor green, a radar's: monospaced, scan lines, very legible.
  military: {
    bg: '#020a04',
    panel: '#06140ae6',
    'panel-solid': '#06140a',
    border: '#2f7a3c',
    text: '#c8ffc8',
    muted: '#86d491',
    accent: '#3ddc5a',
    'accent-text': '#011004',
    link: '#7dff8f',
    danger: '#ff7a5a',
    'control-bg': '#0a1f10',
    'font-ui': "'Share Tech Mono', ui-monospace, 'SFMono-Regular', monospace",
    'font-display': "'Share Tech Mono', ui-monospace, 'SFMono-Regular', monospace",
    radius: '0',
    'frame-clip': 'none',
    'frame-deco':
      'repeating-linear-gradient(to bottom, #3ddc5a0d 0 1px, transparent 1px 3px), ' + brackets('#3ddc5a', 12, 2),
    glow: 'inset 0 0 14px #3ddc5a1f',
    'label-bg': 'rgb(2 12 5 / 0.82)',
    'label-text': '#c8ffc8',
    'overlay-orbit': '#4fbf62',
    'overlay-selected': '#b8ffc2',
    'overlay-line': '#9dffad',
    'overlay-muted': '#2f6b3a',
  },
};

/** The relative luminance of a colour (`#rgb`, `#rrggbb` or `#rrggbbaa`, alpha ignored), per WCAG. */
function luminance(hex: string): number {
  const h = hex.replace('#', '');
  const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h.slice(0, 6);
  const channel = (i: number) => {
    const c = parseInt(full.slice(i, i + 2), 16) / 255;
    return c <= 0.040_45 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4);
}

/** The WCAG contrast ratio of two colours, 1 to 21. */
export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

const KEY = 'theme';

/** The theme the browser remembers, else Default (an unknown name included). */
export function loadTheme(): ThemeName {
  const stored = loadSetting<string>(KEY, 'default');
  return (THEME_NAMES as readonly string[]).includes(stored) ? (stored as ThemeName) : 'default';
}

export function saveTheme(name: ThemeName): void {
  saveSetting(KEY, name);
}

const listeners = new Set<(name: ThemeName) => void>();
let current: ThemeName = 'default';

/** The theme applied. */
export function currentTheme(): ThemeName {
  return current;
}

/** Writes theme `name`'s tokens on the document root, sets `data-theme`, and tells the listeners. */
export function applyTheme(name: ThemeName): void {
  current = name;
  if (typeof document !== 'undefined') {
    const root = document.documentElement;
    root.dataset['theme'] = name;
    for (const token of TOKENS) root.style.setProperty(`--${token}`, THEMES[name][token]);
  }
  for (const listener of listeners) listener(name);
}

/** Calls `listener` on every theme change; returns what stops it. */
export function onThemeChange(listener: (name: ThemeName) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The neutral colours the 3D overlays take from the theme applied. */
export function overlayColours(name: ThemeName = current): { orbit: string; selected: string; line: string; muted: string } {
  const tokens = THEMES[name];
  return {
    orbit: tokens['overlay-orbit'],
    selected: tokens['overlay-selected'],
    line: tokens['overlay-line'],
    muted: tokens['overlay-muted'],
  };
}
