# Interface redesign

## Intent

The web app has grown one panel and one checkbox at a time. Its screens
now show a great deal of data at once, it reads less and less well, and
fixed widths break when one language's words are longer than the
other's. The user wants an app with:

- a real main menu;
- viewers that show their data by theme instead of all at once;
- controls that feel like a media player;
- a freer camera;
- a choice of visual themes, some of them "video game" HUD styles.

## Goals

### 1. Main menu

- The app opens on a main menu, not on a viewer. No viewer starts until a
  system is opened.
- The menu is a grid of cards, one per existing system. Each card shows:
  - the orrery thumbnail (`SystemOrrery.svelte`);
  - the system's name or designation;
  - its star;
  - its planets by type;
  - its date.

  The library's search, sorting, selection, delete and compare stay,
  restyled.
- **Two entry points create a system:**
  - **Random system**, today's *A whole system*;
  - **Around a planet**, today's *Around a planet*.

  Each opens a step-by-step wizard (the user's choice):
  1. the kind of creation;
  2. the star;
  3. the planets, or the planet;
  4. the advanced options, folded;
  5. a summary.

  An **All random** button on the first step creates a system at once.
  The wizards keep every option today's form has, with its validation
  and its estimates.
- Opening a card opens the system's viewer. A **Menu** button in the
  viewer goes back.
- **Systems only.** A world is always created with its whole system around
  it, so the menu lists systems only. The viewers no longer show the
  library list, and the space it took goes back to the view.

### 2. Viewer panels: a dock, with detachable panels

This is the user's choice, for both the star and the planet viewer.

- The data is split into **panels by theme**. The planet viewer's panels
  are the builder's call; the star viewer's could be:
  - *Star*;
  - *Planet*, the selected one;
  - *Planets*, the list;
  - *Orbits & forces*;
  - *Habitable zone & structure*;
  - *Display*.
- The panels live in a **dock**: a side bar with one tab per panel,
  foldable to its tabs alone.
- Any panel can be **detached** into a floating, draggable card over the
  view, and docked again. Floating cards stay on screen. The layout, that
  is which panels are open, docked or floating and where, is remembered
  per viewer in the browser.
- **The display toggles move into a compact icon bar** in the view's
  corner, each with its name in a tooltip, grouped by kind. This covers
  orbits, zone, distances, lines, velocities, pull, frost line, Hill
  spheres, to scale, labels and the like.

### 3. Playback bar

- Both viewers get a bar at the bottom, like a video player:
  - play / pause;
  - a speed slider, with its value;
  - the time bar (the date, today's *Date* slider);
  - jumps, such as *Day 1*, *to periapsis* and the like.
- It replaces today's *Animate* checkbox and the separate speed and date
  controls. In the planet viewer, it drives what animates today: the
  moons' orbits and their speed.

### 4. Camera

- **Star viewer.** The right button frees the camera: dragging with it pans
  on the two screen axes, the target moving with it. The left button
  still turns and the wheel zooms. **Reset view** brings the target back
  to the star.
- **Axes gizmo**, in both viewers. A corner widget shows the classic three
  arrows, x red, y green and z blue, turning with the camera. Clicking an
  axis looks along it; this is the builder's call if it costs little.
- **Travelling** (the user's choice: there and back):
  - opening a body from the star viewer flies the camera from where it
    is to that body, about one second with easing, then switches to the
    body's view;
  - going back to the system flies out from the body to the system's
    framing.

  A click, a key or the wheel during a flight ends it at once.

### 5. Fixes

- **Star viewer: pull labels.** The star's pull value is written along the
  line to the star, beside the body, at the end of the pull arrow. It no
  longer sits over the body.
- **Planet viewer: occlusion.** A sky body that lies between the camera
  and the shown body must be drawn in front of it. Today it is drawn
  behind.
- **Planet viewer: shadows.** That body casts its shadow on the shown body
  when it passes between it and the star: a moon's shadow on its planet,
  and a planet's shadow on its moon.

### 6. Themes

The user picked five themes: the current one and four new ones.

| Theme | Look |
| ----- | ---- |
| **Default** | Today's look, unchanged. |
| **Holo HUD** | Cyan, JARVIS-like: thin cyan strokes on near-black, cut-corner frames, small data glyphs, a technical font. |
| **Orbital station** | Amber and orange, as the HUDs of a station round Mars: reticles, bracket frames, a cockpit mood. |
| **Cyberpunk** | Neon magenta and yellow, diagonal stripes, wide type, a light glitch on hover. |
| **Military** | Phosphor green, like a radar: grids, a slow sweep, very legible. |

- A theme restyles the interface, its fonts included. The overlays drawn
  over the 3D view are restyled too: labels, orbit and line colours,
  vectors and the gizmo. The bodies keep their true colours (the user's
  choice).
- **Colours that carry meaning** (our call) keep a fixed hue in every
  theme, so the data stays readable:
  - the habitable zone (green);
  - the frost line (blue);
  - velocity (cyan) and pull (orange) arrows;
  - the map layers, colormaps and biomes, which are the bodies' data.

  A theme may adjust only their lightness or saturation. On Military,
  the habitable zone stays green but is told apart from the chrome by
  its fill.
- The theme is picked in the menu, and in the viewer's settings, and
  remembered in the browser.

The user's reference images, for the builder:

- [Holo HUD dashboard](https://jarvis-ui-dashboard-1vypazel.blinkusercontent.com/?__bpt=v1.jarvis-ui-dashboard-1vypazel.1794516279.K3unDmVtPCRBrUA0AW-g3ev3kkHo7qI8WTmBzkG9-7U)
- [Sci-fi HUD 1](https://t4.ftcdn.net/jpg/04/96/70/65/360_F_496706535_KHfBRjKHI4JerhbnJvYilXZPULp5vCSC.jpg)
- [Sci-fi HUD 2](https://i.pinimg.com/736x/b5/9e/09/b59e09ae0d01fa6303f092c2dd888b5d.jpg)
- [Mars station HUD](https://media.istockphoto.com/id/1285985366/photo/space-station-in-planet-mars-orbit-space-exploration-ui-interface-hud-mars-high-resolution.jpg?s=612x612&w=0&k=20&c=cEm3WMUPnGgMBomuwU9_nT36Z4BFz9Pbf1k8W5fkETo=)
- [HUD kit](https://market-resized.envatousercontent.com/previews/files/254255786/Preview.jpg?w=590&h=590&cf_fit=crop&crop=top&format=auto&q=85&s=a20cbfa8f23c6e3bb8f741a8924a35bea2664a3952cf6dc353993fd80841648a)
- [Cyberpunk HUD elements](https://thumbs.dreamstime.com/b/cyberpunk-futuristic-user-interface-hud-design-elements-detailed-digital-including-progress-bars-data-charts-maps-466494908.jpg)

### 7. Compare, jobs, links and the other entries

- **Compare stays** (the user's choice). Two viewers sit side by side,
  each with its own dock and floating cards; the floating cards belong
  to their pane. A **single playback bar** at the bottom drives both
  (play, speed, date). Cameras stay linkable as today, and linking
  cameras does not link flights. The remembered dock layout is per
  viewer kind (star or planet) and shared by both panes.
- **The planet viewer's clock** (the user's choice). Its playback bar
  drives one clock in days, which moves both the seasons (the sun over
  the year) and the moons' orbits, with the speed and the date. The
  geological epoch slider (Ma) stays a separate control, in a panel.
- **After a wizard** (our call). Submitting opens the new system's viewer
  at once, as today, its generation showing in progress.
- **Running jobs** (our call). They show in a small global indicator,
  visible in the menu and in the viewer, that opens the job list, with
  its cancel buttons.
- **Vary and Reroll** (our call) stay. *Vary…* opens the wizard over the
  viewer, prefilled from the body, on its summary step. Closing it
  returns to the viewer.
- **Links** (our call). The address says what is shown: no hash, or
  `#menu`, is the menu, and `#system=…` with today's parameters opens the
  viewer directly. The browser's Back goes from a viewer to the menu, and
  no system is auto-selected any more.
- **Owners of today's sidebar** (our call):
  - D moves WorldInfo and BodyCard into dock panels;
  - M removes the library from the viewer and adds the global jobs
    indicator.

### 8. Languages

- Layouts are fluid. No fixed label column such as today's `9.5em`, and
  text wraps instead of overflowing.
- Buttons size to their words.
- Every screen is checked in English and French.

## Non-goals

- No change to the engine, the server API (unless the menu needs a field
  the summaries lack) or the generated worlds.
- No full mobile design (the user's choice: **desktop first**). On a
  phone the app stays usable: panels open full screen, nothing floats.
- No new data in the viewers. This round is about showing the existing
  data better.

## Anti-goals

- A theme that hurts legibility: every theme keeps the text contrast
  readable, at least WCAG AA for body text.
- Losing a feature on the way. Everything the viewers and the form can
  do today stays reachable.
- Effects that cost frame rate. The HUD decorations are CSS, not extra
  render passes, and a sweep or glitch never runs on the 3D canvas.
- Layout jumps when switching language.

## Constraints

- Svelte 5 runes, TypeScript, the existing Three.js views (`StarView.ts`,
  `PlanetView.ts`).
- Every string goes in `lib/i18n` (en, fr).
- Settings live in `lib/settings.ts` or the existing browser storage
  pattern, wrapped in try/catch.
- `npm run check` and the tests pass.
- Docs in the same change: `web/README.md` and `docs/system-spec.md`
  (Interface).
- Fonts load from Google Fonts or are bundled. No other CDN.

## Plan of work, and the split between the two sessions

*Our call.* I (map-generator-d2) merge every PR, one at a time, the
other session's included. The other session opens its PRs and says when
each is ready.

| PR | Content | Who | After |
| -- | ------- | --- | ----- |
| T | Theme system: design tokens (colours, fonts, radii, frames) as CSS variables on `:root[data-theme]`, the five themes, the picker, the overlay colours read from the tokens | d2 | first |
| M | App shell and main menu: menu ↔ viewer, the card grid, the two wizards; the viewers lose the library list | 6a | T |
| D | Dock and detachable panels: the framework and both viewers' content split into panels, the icon bar of display toggles, fluid layouts | d2 | T |
| P | Playback bar in both viewers | d2 | D |
| C | Camera: right-click pan, axes gizmo in both viewers, travelling in and out | d2 | D |
| F | Fixes: pull labels beside the body (star viewer); occlusion and shadows of sky bodies (planet viewer) | 6a, except the pull labels (d2, in P or C) | any time |

M and D run in parallel. Each PR updates the docs it touches.

## Left to the builder

- The exact panel split of the planet viewer.
- The icons.
- The fonts of each theme.
- The flight curve and its duration.
- How a floating card snaps back to the dock.
- The wizard's step layout.
- Whether clicking a gizmo axis turns the view.
