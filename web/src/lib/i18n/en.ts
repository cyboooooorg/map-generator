// English messages: the source catalog. Every other locale is typed after
// this one (see `Messages`): add a message here first.
//
// Messages are plain strings, or functions of named parameters when they
// embed values, so each language can order the words its own way.

import type { Stage } from '../api/types/Stage';

export const en = {
  /** Language selector. */
  language: {
    label: 'Language',
  },
  /** Number, unit and time formatting. */
  format: {
    /** Between a number and its unit. */
    unitSpace: ' ',
    millionKm2: 'M km²',
    justNow: 'just now',
  },
  /** Page frame: header, errors, notifications, compare bar. */
  app: {
    title: 'Planets',
    tabWorlds: 'Worlds',
    tabViewer: 'Viewer',
    newWorld: 'New world',
    unreachable: (p: { error: string }) => `Cannot reach the server: ${p.error}`,
    retry: 'Retry',
    retrying: 'Retrying…',
    dismiss: 'Dismiss',
    ready: (p: { id: string }) => `${p.id} is ready.`,
    open: 'Open',
    failed: (p: { id: string; error: string }) => `${p.id} failed: ${p.error}`,
    cancelled: (p: { id: string }) => `${p.id} cancelled.`,
    deleted: (p: { ids: readonly string[] }) =>
      p.ids.length === 1 ? `Deleted ${p.ids[0]}.` : `Deleted ${p.ids.length} worlds.`,
    undo: 'Undo',
    deleteFailed: (p: { error: string }) => `Could not delete: ${p.error}`,
    rerollFailed: (p: { error: string }) => `Could not reroll: ${p.error}`,
    versus: 'vs',
    linkCameras: 'Link cameras',
    linkDisplay: 'Link display',
    linkDisplayHelp:
      'The right (or lower) pane mirrors the layer, relief, sunlight and season, atmosphere, clouds and grid of the main one',
    swap: 'Swap',
    close: 'Close',
  },
  /** World list in the sidebar. */
  worldList: {
    empty: 'No world yet.',
    noMatch: 'No world matches.',
    search: 'Search id or seed',
    searchLabel: 'Search worlds',
    filterLabel: 'Filter by planet',
    allPlanets: 'All planets',
    sortLabel: 'Sort',
    sorts: {
      newest: 'Newest first',
      oldest: 'Oldest first',
      planet: 'Planet',
      seed: 'Seed',
      size: 'Largest map',
    },
    select: 'Select',
    done: 'Done',
    selected: (p: { count: number }) => `${p.count} selected`,
    all: 'all',
    none: 'none',
    compare: 'Compare',
    compareHelp: 'Select exactly two worlds',
    delete: 'Delete',
    selectOne: (p: { id: string }) => `Select ${p.id}`,
    deleteOne: (p: { id: string }) => `Delete ${p.id}`,
    seedSize: (p: { seed: number; width: number; height: number }) => `seed ${p.seed} · ${p.width}×${p.height}`,
    details: (p: { ago: string; sea: string; tilt: string }) => `${p.ago} · sea ${p.sea} · tilt ${p.tilt}°`,
  },
  /** Generation jobs. */
  jobs: {
    title: 'Generations',
    cancel: 'Cancel',
    cancelling: 'Cancelling…',
    open: 'Open',
    dismiss: 'Dismiss',
    waiting: 'Waiting for the generator…',
    stageStep: (p: { stage: string; step: number; steps: number }) => `${p.stage} (${p.step}/${p.steps})`,
    left: (p: { time: string }) => `about ${p.time} left`,
    stageFirst: 'The current stage finishes first.',
    ready: (p: { seed: string }) => `Ready · seed ${p.seed}`,
    cancelled: 'Cancelled',
  },
  /** Generation stages, as the progress shows them. */
  stages: {
    noise: 'Shaping continents',
    erosion: 'Eroding the terrain',
    temperature: 'Computing temperatures',
    coast: 'Measuring coastlines',
    moisture: 'Blowing winds and rain',
    rivers: 'Routing rivers and lakes',
    biomes: 'Choosing biomes',
    export: 'Writing the maps',
  } satisfies Record<Stage, string>,
  /** Distance measurement. */
  measure: {
    label: 'Distance measurement',
    start: 'Click points on the planet',
    legs: (p: { legs: number }) => `${p.legs} ${p.legs === 1 ? 'leg' : 'legs'}, along great circles`,
    undo: 'Undo',
    clear: 'Clear',
    done: 'Done',
  },
  /** Notifications. */
  toasts: {
    dismiss: 'Dismiss',
  },
};
