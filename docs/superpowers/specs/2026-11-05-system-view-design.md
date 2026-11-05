# System view redesign

## Intent

The star view of a planetary system (`StarPane.svelte` and
`lib/viewer/StarView.ts`, fed by `lib/systemStar.ts` from `Viewer.svelte`)
shows the details of the selected planet only. The other planets are dots
with a name and a distance. The user wants to **compare every visible
planet at a glance**. Selecting a planet must not change the picture of
the system either.

## Goals

1. **Every planet carries its own facts on the view**, always:
   - A **dotted line** from the star to the planet, with the planet's
     current distance (AU) written at the planet's end.
   - A **velocity vector** starting at the planet, tangent to its orbit
     and pointing the way it moves, with its current speed (km/s) written
     above it. Its length is proportional to the speed, on one scale for
     the whole system, so the vectors compare at a glance. This is the
     builder's call: the scale must keep the fastest arrow short enough
     not to cross other orbits often.
   - Both follow the animation: the distance and speed are the live
     values on an eccentric orbit, as the selected planet's label is
     today.
2. **Toggles.** The display section of the panel gets a checkbox for the
   dotted lines with their distances, and one for the vectors with their
   speeds. Both are on by default. *Our call:* like the other star-view
   toggles, they last for the session and are not saved in
   `settings.ts`. The existing **distances** toggle stays, with its
   current meaning: the selected planet's apsis and zone-edge labels and
   its rich label (distance, speed, light, zone). The solid star–planet
   line it draws gives way to the new dotted line.
3. **Hover card.** Hovering a planet's marker:
   - highlights the planet (a ring or a glow, the builder's call);
   - opens a detail card beside it, linked to the planet by a line, with
     a short appear animation (about 150–200 ms, fade and scale or slide);
   - closes the card when the pointer leaves, unless the card is pinned.
   A pin button keeps the card open. Several cards can be pinned at once,
   to compare planets side by side. A pinned card follows its planet
   while the orbits animate; the line keeps linking them. Unpinning, or
   switching system, closes it.
4. **Card contents** (the user agreed to this list):
   - name and type;
   - radius and mass;
   - semi-major axis and eccentricity;
   - current speed, and the year;
   - equilibrium temperature;
   - its place in the habitable zone (inside, or the distance to it);
   - number of moons;
   - whether it is generated, and a button that opens the planet (what a
     click on a marker does today).
5. **One fixed scale per system.** Distances on the view come from one
   mapping computed from the whole system (the star, the habitable zone,
   every planet's apoapsis), not from the selected planet. Selecting
   another planet changes the highlighted planet and the panel, never the
   orbits, the zone, the star or the zoom. The mapping still fits close
   and far planets on one screen (a compression such as today's linear
   then logarithmic part, or a square root or log scale; the builder's
   call). The panel says that distances are not to scale, as it does
   today with `compressedFrom`.
6. **Every planet is drawn alike** (the user's choice). Today
   `systemStarView` makes the selected planet "the world": a fixed blue
   marker (`PLANET_MARKER`), its moons, its apsis markers, and the zone
   edges labelled on its apsides line, while the others are "giants".
   That asymmetry goes. Every planet is one entry of a single list, sized
   by its radius and coloured by its preview or map. Selection only adds
   a highlight, and the selected planet alone shows its moons and its
   periapsis and apoapsis markers. Selecting another planet moves those
   extras, and changes nothing else.
7. **One clock for the system** (the user's choice). The animation runs
   at one pace for the whole system, set by the existing speed control,
   and the clock text counts Earth days. Selecting a planet no longer
   resets the time or changes any planet's speed. *Our call:* the default
   pace makes the innermost planet's year last about 10 s, keeping the
   outer ones visibly moving at the fastest speed setting.
8. **Click and touch** (the user's choice). A click or a tap on a planet
   opens its card and pins it. Hover still shows an unpinned card on
   devices that have a pointer. The card's button opens the planet, and
   so does a double click on the marker. A drag still turns the view, as
   it does today.
9. **Pins survive the planet view** (the user's choice). Pinned cards
    are kept per system for the session. Opening a planet and coming back
    restores them. Switching system closes them. The pin state therefore
    lives above `StarPane`, in `Viewer.svelte` or the `world` store.
10. **Generated planets show their map.** A generated planet's marker
   wraps its map around the sphere instead of its preview colour.
   *Our call:* the export already writes `thumb.png` (512 × 256) for
   every body, served by `worldThumbnail` (`web/src/lib/api/client.ts`).
   That is small enough for a marker, so the user's "very low resolution
   map generated after the planet" already exists and needs no engine
   change. If loading ten 512 × 256 textures proves slow, the builder may
   add a smaller export (for example 128 × 64) instead. That would be an
   engine change, documented in the README.

## Non-goals

- The planet (globe) view. In particular the host star's halo there stays
  exactly as it is. Its disk is already to scale: angular size from the
  star's true radius and distance, with a 3-pixel floor.
- The star stays drawn larger than to scale in the system view: a
  true-size star would be invisible beside the orbits.
- No change to the system draw, the generation or `system.json`.

## Anti-goals

- Labels piling up into an unreadable mass with many planets (up to 15).
  Labels must avoid overlapping each other and the star where possible,
  for example by shrinking or by fading the distance labels of crowded
  inner orbits. This is the builder's call, but it must be checked on a
  15-planet system.
- The view jumping or rescaling when a planet is selected.
- Slowing the view down: the hover and the live labels must keep the
  animation smooth on a 15-planet system.

## Constraints

- `StarPane` also serves a lone world's star view, with giants instead of
  system planets. That view must keep working. The new labels may apply
  to it too (its giants are planets too), but it must not break.
- Front-end conventions of the repo: Svelte 5 runes, TypeScript, every
  user-facing string in `lib/i18n` in English and French, tests next to
  the code (`*.test.ts`) for the pure parts (the scale, the label
  layout), `npm run check` and the tests passing.
- Docs in the same change: `web/README.md` (the system view section)
  and `docs/system-spec.md` (the Interface part) where it describes the
  system view.
- Repo rules: Conventional Commits, English in the repo, markdownlint.

## Split between the two sessions

*Our call:* the work runs in three PRs, because both halves need the same
planet model:

- **0. The shared model, first, one session.** One planet list for the
  system view: every planet with its orbit, size, colour, id, generated
  flag and live position, the selected one included and pickable. The
  fixed system scale (goal 5), the system clock (goal 7), and every
  planet drawn alike (goal 6). The panel and labels keep working as
  today. Merged before A and B start.
- **A. The labels**: the dotted lines, the vectors and their toggles
  (goals 1–2), and the label layout. These live mostly in
  `web/src/lib/viewer/starSystem.ts`, `StarView.ts`, and
  `StarPane.svelte`'s labels and checkboxes.
- **B. The card and the textures**: the click, hover and highlight, the
  card, its pins and animation (goals 3–4, 8–9), and the map textures
  on the markers (goal 10). These live mostly in a new card component, in
  `Viewer.svelte` for the pins, and in the marker code of `StarView.ts`.

A and B run in parallel once PR 0 is merged. Both touch `StarView.ts`, so
the second to merge merges `main` into its branch first.

*Our call:* the hover cards apply to system planets only. A lone world's
star view keeps its giants as they are, with no cards: its giants have
no moons, no generated state and no planet to open.

## Left to the builder

- The exact compression of the scale, the vector scale, the highlight's
  look, the card's look and animation, and its placement (left or right
  of the planet, kept on screen).
- How labels avoid each other.
- Whether the lone world's star view gets the new lines and vectors.
