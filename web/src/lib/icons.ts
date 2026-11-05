// The interface's icons: SVG path data on a 24 × 24 grid, drawn with a
// stroke of the current colour (see components/Icon.svelte). Plain shapes,
// so that every theme can colour them.
export const ICONS = {
  star: 'M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8z',
  planet: 'M12 6a6 6 0 1 0 0 12a6 6 0 1 0 0-12zM3 15c2-1 5-1.5 9-1.5s7 .5 9 1.5',
  list: 'M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01',
  info: 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18zM12 11v6M12 7.5v.01',
  orbit: 'M12 5c5 0 9 3.1 9 7s-4 7-9 7s-9-3.1-9-7s4-7 9-7zM12 10a2 2 0 1 0 0 4a2 2 0 1 0 0-4z',
  zone: 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18zM12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8z',
  prominence: 'M5 18a7 7 0 0 1 14 0M8 18c0-5 1.5-9 4-9s4 4 4 9',
  giants: 'M12 5a7 7 0 1 0 0 14a7 7 0 1 0 0-14zM5.5 10h13M5.5 14h13M2 13c3 1 17 1 20-1',
  distance: 'M4 18L20 6M4 18v-4M4 18h4M20 6v4M20 6h-4',
  links: 'M4 20l2-2M8 16l2-2M12 12l2-2M16 8l2-2M20 4h.01',
  velocity: 'M4 16c4-6 9-8 15-8M15 4l4 4l-4 4',
  pull: 'M20 12H6M10 8l-4 4l4 4M3 12h.01',
  frost: 'M12 3v18M4.2 7.5l15.6 9M4.2 16.5l15.6-9M9.5 4.5L12 7l2.5-2.5M9.5 19.5L12 17l2.5 2.5',
  hills: 'M12 6a6 6 0 1 0 0 12a6 6 0 1 0 0-12zM12 10.5a1.5 1.5 0 1 0 0 3a1.5 1.5 0 1 0 0-3zM18 3l3 3M3 18l3 3',
  scale: 'M3 15l12-12l6 6l-12 12zM7 11l2 2M10 8l2 2M13 5l2 2',
  play: 'M7 4l13 8l-13 8z',
  pause: 'M7 4h3v16H7zM14 4h3v16h-3z',
  start: 'M6 4v16M20 4L9 12l11 8z',
  periapsis: 'M12 12m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M3 12c0-5 4-8 9-8M14 6l4 1-1 4',
  detach: 'M14 4h6v6M20 4l-8 8M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5',
  dock: 'M15 3h5a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1h-5zM3 12h9M8 8l4 4l-4 4',
  close: 'M6 6l12 12M18 6L6 18',
  fold: 'M9 6l6 6l-6 6',
  unfold: 'M15 6l-6 6l6 6',
} as const;

export type IconName = keyof typeof ICONS;
