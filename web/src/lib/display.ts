// The display settings a compare pane can mirror from the main pane
// ("Link display"): what is drawn and how it is lit, not the camera (that is
// "Link cameras") nor the hex grid (computed per world by the server).

/** The settings mirrored by "Link display". */
export interface LinkedDisplay {
  layer: string;
  relief: number;
  sun: boolean;
  season: number;
  sunLongitude: number;
  atmosphere: boolean;
  stars: boolean;
  starLight: boolean;
  clouds: boolean;
  graticule: boolean;
}

export const LINKED_KEYS: readonly (keyof LinkedDisplay)[] = [
  'layer',
  'relief',
  'sun',
  'season',
  'sunLongitude',
  'atmosphere',
  'stars',
  'starLight',
  'clouds',
  'graticule',
];

/** The linked settings of a fuller display state. */
export function linkedPart(display: LinkedDisplay): LinkedDisplay {
  const part = {} as Record<keyof LinkedDisplay, unknown>;
  for (const key of LINKED_KEYS) {
    part[key] = display[key];
  }
  return part as unknown as LinkedDisplay;
}

/** The settings of `source` that differ from `current`: what a linked pane must change. */
export function linkedChanges(current: LinkedDisplay, source: LinkedDisplay): Partial<LinkedDisplay> {
  const changes: Partial<Record<keyof LinkedDisplay, unknown>> = {};
  for (const key of LINKED_KEYS) {
    if (current[key] !== source[key]) {
      changes[key] = source[key];
    }
  }
  return changes as Partial<LinkedDisplay>;
}
