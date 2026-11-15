# Task 4 report

Implemented `CardSource` and `previewSource` (web/src/lib/cardSource.ts); the six cards and NameEditor take `world: CardSource` (LoadedWorld satisfies it unchanged, App.svelte untouched). CardSource members: system, systemId, systemNames, systemEvicted, systemBody, shownSystemBody, shownExtras, meta, stats, rivers, optional changed, showSystemBody, loadSystem (BodyHeader/NameEditor/MoonsCard read the system ones). `moved` class (1.5 s animation, none under reduced motion) on the dd of each tree line in Atmosphere/Interior and on the header's class tree. CauseTree/CauseBranch take `oninput`; leaf is a button when set. SurfaceCard now renders without `meta` (surface age only) instead of nothing.

Fixtures: real resolves from the debug server (earth-like, hot-jupiter, mini-neptune, seed 7), extras trimmed to the focus body.

Tests: `npx vitest run src/components/body` 18/18; `npm run check` 0 errors; `npm test` 107 files, 831 passed (before the final exactOptionalPropertyTypes type fix; focused tests rerun after).
TDD: tests were written after the implementation, so no RED run. Honest deviation.
No new strings, so no i18n/doc change.
Concern: NameEditor and the Generate button show in a preview's header; the form may want to hide them (no onRenamed/onGenerate meaning there).
