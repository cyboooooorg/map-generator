<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import GenerateForm from './components/GenerateForm.svelte';
  import JobList from './components/JobList.svelte';
  import LanguageSelect from './components/LanguageSelect.svelte';
  import Thumbnail from './components/Thumbnail.svelte';
  import Toasts from './components/Toasts.svelte';
  import WorldInfo from './components/WorldInfo.svelte';
  import WorldList from './components/WorldList.svelte';
  import WorldPane from './components/WorldPane.svelte';
  import { ApiFailure, api } from './lib/api/client';
  import type { FormSpec } from './lib/api/types/FormSpec';
  import type { JobCreated } from './lib/api/types/JobCreated';
  import type { PresetInfo } from './lib/api/types/PresetInfo';
  import type { WorldRequest } from './lib/api/types/WorldRequest';
  import type { WorldSummary } from './lib/api/types/WorldSummary';
  import { requestFromWorld, rerollRequest } from './lib/format';
  import { getLocale, t } from './lib/i18n/index.svelte';
  import { jobs, type TrackedJob } from './lib/jobs.svelte';
  import { decodeView, decodeWorlds, linkHash, type ViewLink } from './lib/link';
  import { loadSetting, saveSetting } from './lib/settings';
  import { stepWorld } from './lib/shortcuts';
  import { toasts } from './lib/toasts.svelte';
  import type { LinkedDisplay } from './lib/display';
  import type { CameraState } from './lib/viewer/PlanetView';
  import { LoadedWorld } from './lib/world.svelte';

  /** How long a deleted world can be restored, in ms. */
  const UNDO_MS = 6000;
  /** Shortest wait between two list refreshes on focus, in ms. */
  const REFRESH_EVERY_MS = 5000;

  let worlds = $state<WorldSummary[]>([]);
  let presets = $state<PresetInfo[]>([]);
  let spec = $state<FormSpec | null>(null);
  let selectedId = $state<string | null>(null);
  let compareId = $state<string | null>(null);
  let fatal = $state<string | null>(null);
  /** Set once the address has been read: until then, writing it would erase it. */
  let ready = $state(false);
  /** The generation form, when open: blank, or varying a world. */
  let form = $state<{ initial: WorldRequest | null; variedFrom: string | null } | null>(null);
  /** Worlds deleted but still restorable: hidden, and removed on the server once the undo delay ends. */
  let hidden = $state<string[]>([]);
  /** World ids in the order the list shows them (sorted, filtered). */
  let listOrder = $state<string[]>([]);
  let linkCameras = $state(loadSetting('linkCameras', true));
  $effect(() => saveSetting('linkCameras', linkCameras));
  /** Whether the compare pane mirrors the main pane's display settings. */
  let linkDisplay = $state(loadSetting('linkDisplay', false));
  $effect(() => saveSetting('linkDisplay', linkDisplay));
  /** The main pane's linked display settings, as it reports them. */
  let mainDisplay = $state<LinkedDisplay | null>(null);
  /**
   * Which pane fills the screen on narrow layouts (the CSS ignores it on wide
   * ones, where the sidebar and the viewer sit side by side).
   */
  let pane = $state<'sidebar' | 'viewer'>('sidebar');

  const primary = new LoadedWorld();
  const secondary = new LoadedWorld();
  let primaryPane = $state<WorldPane | null>(null);
  let secondaryPane = $state<WorldPane | null>(null);
  /** The job whose world opens when ready, if the selection has not changed meanwhile. */
  let autoOpen: { jobId: string; selection: string | null } | null = null;
  /** Deletions waiting for their undo delay, flushed if the page closes. */
  const pendingDeletes = new Set<string>();

  const visible = $derived(worlds.filter((w) => !hidden.includes(w.id)));
  const existing = $derived(new Set(worlds.map((w) => w.id)));
  const revision = (id: string | null) => worlds.find((w) => w.id === id)?.modified_ms ?? null;

  /** When the list was last asked for (ms), to throttle the focus refreshes. */
  let lastRefresh = 0;

  async function refresh(): Promise<void> {
    lastRefresh = Date.now();
    worlds = await api.worlds();
  }

  /**
   * The page came back to the front: list the worlds again, so those made
   * meanwhile (`mise run gen`, another tab) show up. At most once every
   * REFRESH_EVERY_MS; a failure is left for the next one.
   */
  function refreshOnReturn(): void {
    if (!ready || document.visibilityState !== 'visible' || Date.now() - lastRefresh < REFRESH_EVERY_MS) {
      return;
    }
    refresh().catch((e: unknown) => console.warn('Could not refresh the world list', e));
  }

  /**
   * `#world=<id>&compare=<id>`: the worlds shown, so a reload or a shared
   * link opens them. A copied link also carries the view (see lib/link.ts),
   * applied once to the main pane; the address then names only the worlds.
   */
  function applyHash(): void {
    const params = new URLSearchParams(location.hash.slice(1));
    const { world, compare } = decodeWorlds(params);
    const known = (id: string | null) => (id && worlds.some((w) => w.id === id) ? id : null);
    if (known(world)) {
      selectedId = world;
      compareId = known(compare);
      pane = 'viewer';
      pendingView = decodeView(params);
    }
  }

  /** The view of a link, until the panes can take it (the compared one follows a linked camera). */
  let pendingView = $state.raw<ViewLink | null>(null);
  $effect(() => {
    const target = primaryPane;
    const other = compareId ? secondaryPane : null;
    const view = pendingView;
    if (target && view && (!compareId || other)) {
      pendingView = null;
      untrack(() => {
        target.applyView(view);
        if (view.camera && linkCameras) other?.applyCamera(view.camera);
      });
    }
  });

  /** A link to the worlds shown and the main pane's view (Copy link). */
  function shareLink(): string {
    const hash = linkHash(selectedId, compareId, primaryPane?.viewState() ?? null);
    return `${location.origin}${location.pathname}#${hash}`;
  }

  let starting = $state(false);

  /** Loads the presets, the form limits and the worlds; run again by Retry. */
  async function start(): Promise<void> {
    fatal = null;
    starting = true;
    try {
      if (!spec) {
        [presets, spec] = await Promise.all([api.presets(), api.form()]);
        jobs.defaultSize = [spec.default_width, spec.default_height];
      }
      await refresh();
      if (!ready) {
        applyHash();
        if (!selectedId) {
          selectedId = worlds[0]?.id ?? null;
          if (selectedId) {
            pane = 'viewer';
          }
        }
        ready = true;
      }
      await jobs.reattach();
    } catch (e) {
      fatal = String(e);
    } finally {
      starting = false;
    }
  }

  // The page's language follows the interface's, for screen readers,
  // hyphenation and the browser's own controls.
  $effect(() => {
    document.documentElement.lang = getLocale();
    document.title = t.app.title;
  });

  onMount(() => {
    void start();
    window.addEventListener('hashchange', applyHash);
    window.addEventListener('pagehide', flushDeletes);
    window.addEventListener('focus', refreshOnReturn);
    document.addEventListener('visibilitychange', refreshOnReturn);
    return () => {
      window.removeEventListener('hashchange', applyHash);
      window.removeEventListener('pagehide', flushDeletes);
      window.removeEventListener('focus', refreshOnReturn);
      document.removeEventListener('visibilitychange', refreshOnReturn);
    };
  });

  // Keep the address in step with the worlds shown, without filling the history.
  $effect(() => {
    if (!ready) {
      return;
    }
    const params = new URLSearchParams();
    if (selectedId) params.set('world', selectedId);
    if (selectedId && compareId) params.set('compare', compareId);
    const hash = params.toString();
    if (location.hash.slice(1) !== hash) {
      history.replaceState(null, '', hash ? `#${hash}` : location.pathname + location.search);
    }
  });

  $effect(() => {
    const id = selectedId;
    untrack(() => primary.load(id));
  });
  $effect(() => {
    const id = selectedId ? compareId : null;
    untrack(() => secondary.load(id));
  });

  jobs.onFinished = (job: TrackedJob) => {
    const status = job.status;
    void refresh().then(() => {
      if (status.status === 'done') {
        // A world replaced in place keeps its id: selecting it again would
        // change nothing, so reload the panes that show it.
        if (status.world_id === selectedId) {
          primary.load(status.world_id);
        }
        if (selectedId && status.world_id === compareId) {
          secondary.load(status.world_id);
        }
        if (autoOpen?.jobId === job.jobId && autoOpen.selection === selectedId) {
          select(status.world_id);
          toasts.show(t.app.ready({ id: status.world_id }));
        } else {
          toasts.show(t.app.ready({ id: status.world_id }), {
            action: { label: t.app.open, run: () => select(status.world_id) },
            ms: 8000,
          });
        }
        // Finished jobs leave the list after a while.
        setTimeout(() => jobs.dismiss(job.jobId), 8000);
      } else if (status.status === 'failed') {
        toasts.error(t.app.failed({ id: job.worldId, error: status.error }));
      } else if (status.status === 'cancelled') {
        toasts.show(t.app.cancelled({ id: job.worldId }));
        setTimeout(() => jobs.dismiss(job.jobId), 4000);
      }
    });
  };

  function select(id: string): void {
    if (id === compareId) {
      compareId = null;
    }
    selectedId = id;
    pane = 'viewer';
  }

  /** `[` / `]` in the viewer: the previous or next world of the list. */
  function step(direction: 1 | -1): void {
    const next = stepWorld(listOrder, selectedId, direction);
    if (next) {
      select(next);
    }
  }

  function compare(a: string, b: string): void {
    selectedId = a;
    compareId = b;
    pane = 'viewer';
  }

  function swap(): void {
    [selectedId, compareId] = [compareId, selectedId];
  }

  function removeWorlds(ids: string[]): void {
    if (ids.length === 0) {
      return;
    }
    hidden = [...hidden, ...ids];
    for (const id of ids) {
      pendingDeletes.add(id);
    }
    if (compareId && ids.includes(compareId)) {
      compareId = null;
    }
    if (selectedId && ids.includes(selectedId)) {
      selectedId = visible[0]?.id ?? null;
      compareId = null;
    }
    const timer = setTimeout(() => void commitDeletes(ids), UNDO_MS);
    toasts.show(t.app.deleted({ ids }), {
      ms: UNDO_MS,
      action: {
        label: t.app.undo,
        run: () => {
          clearTimeout(timer);
          for (const id of ids) {
            pendingDeletes.delete(id);
          }
          hidden = hidden.filter((id) => !ids.includes(id));
        },
      },
    });
  }

  async function commitDeletes(ids: string[]): Promise<void> {
    const results = await Promise.allSettled(
      ids.map((id) => {
        pendingDeletes.delete(id);
        return api.deleteWorld(id);
      }),
    );
    for (const result of results) {
      if (result.status === 'rejected') {
        toasts.error(t.app.deleteFailed({ error: String(result.reason) }));
      }
    }
    await refresh();
    hidden = hidden.filter((id) => !ids.includes(id));
  }

  /** The page is closing: send the deletions still waiting for their undo delay. */
  function flushDeletes(): void {
    for (const id of pendingDeletes) {
      void fetch(`/api/worlds/${encodeURIComponent(id)}`, { method: 'DELETE', keepalive: true });
    }
    pendingDeletes.clear();
  }

  function openForm(initial: WorldRequest | null = null, variedFrom: string | null = null): void {
    form = { initial, variedFrom };
    pane = 'sidebar';
  }

  function submitted(job: JobCreated, request: WorldRequest): void {
    form = null;
    autoOpen = { jobId: job.job_id, selection: selectedId };
    jobs.track(job.job_id, job.world_id, request, { status: 'queued' }, true);
  }

  /** Queues the selected world again with a new random seed. */
  async function reroll(): Promise<void> {
    if (!primary.meta) {
      return;
    }
    const request = rerollRequest(primary.meta);
    try {
      const job = await api.createWorld(request);
      submitted(job, request);
    } catch (e) {
      toasts.error(t.app.rerollFailed({ error: e instanceof ApiFailure ? e.message : String(e) }));
    }
  }

  function cameraFrom(target: () => WorldPane | null) {
    return (state: CameraState) => {
      if (linkCameras && compareId) {
        target()?.applyCamera(state);
      }
    };
  }
</script>

<div class="layout" class:show-viewer={pane === 'viewer'}>
  <aside class="sidebar">
    <header>
      <h1>{t.app.title}</h1>
      <!-- Only shown on narrow screens, where one pane is visible at a time. -->
      <div class="tabs">
        <button class:active={pane === 'sidebar'} aria-pressed={pane === 'sidebar'} onclick={() => (pane = 'sidebar')}
          >{t.app.tabWorlds}</button
        >
        <button class:active={pane === 'viewer'} aria-pressed={pane === 'viewer'} onclick={() => (pane = 'viewer')}
          >{t.app.tabViewer}</button
        >
      </div>
      <LanguageSelect />
      <button class="primary" onclick={() => openForm()} disabled={!spec}>{t.app.newWorld}</button>
    </header>
    {#if fatal}
      <p class="error" role="alert">
        {t.app.unreachable({ error: fatal })}
        <button onclick={() => void start()} disabled={starting}>{starting ? t.app.retrying : t.app.retry}</button>
        <button onclick={() => (fatal = null)}>{t.app.dismiss}</button>
      </p>
    {/if}
    <div class="content">
      <JobList onOpen={select} />
      {#if form && spec}
        <GenerateForm
          {presets}
          {spec}
          {existing}
          initial={form.initial}
          variedFrom={form.variedFrom}
          onSubmitted={submitted}
          onCancel={() => (form = null)}
        />
      {:else}
        <WorldInfo
          world={primary}
          summary={worlds.find((w) => w.id === selectedId)}
          worlds={visible}
          {compareId}
          onVary={() => primary.meta && openForm(requestFromWorld(primary.meta), primary.id)}
          onReroll={reroll}
          onCompare={(id) => (compareId = id)}
          link={shareLink}
          onDelete={() => selectedId && removeWorlds([selectedId])}
        />
        <WorldList
          worlds={visible}
          {selectedId}
          {compareId}
          bind:order={listOrder}
          onSelect={select}
          onDelete={removeWorlds}
          onCompare={compare}
        />
      {/if}
    </div>
  </aside>

  <main class:comparing={compareId !== null}>
    {#if compareId}
      <div class="compare-bar">
        <span class="names">
          <Thumbnail id={selectedId ?? ''} revision={revision(selectedId)} width={36} height={20} radius={3} />
          <strong>{selectedId}</strong>
          <span class="muted">{t.app.versus}</span>
          <Thumbnail id={compareId} revision={revision(compareId)} width={36} height={20} radius={3} />
          <strong>{compareId}</strong>
        </span>
        <label class="check"><input type="checkbox" bind:checked={linkCameras} /> {t.app.linkCameras}</label>
        <label
          class="check"
          title={t.app.linkDisplayHelp}
          ><input type="checkbox" bind:checked={linkDisplay} /> {t.app.linkDisplay}</label
        >
        <button onclick={swap}>{t.app.swap}</button>
        <button onclick={() => (compareId = null)}>{t.app.close}</button>
      </div>
    {/if}
    <div class="panes">
      <WorldPane
        bind:this={primaryPane}
        world={primary}
        revision={revision(selectedId)}
        keyboard={true}
        label={compareId ? selectedId : null}
        onStepWorld={step}
        onDisplayChange={(display) => (mainDisplay = display)}
        onCameraChange={cameraFrom(() => secondaryPane)}
      />
      {#if compareId}
        <WorldPane
          bind:this={secondaryPane}
          world={secondary}
          revision={revision(compareId)}
          keyboard={false}
          persistDisplay={false}
          linkedDisplay={linkDisplay ? mainDisplay : null}
          label={compareId}
          onCameraChange={cameraFrom(() => primaryPane)}
        />
      {/if}
    </div>
  </main>
</div>

<Toasts />

<style>
  .layout {
    display: grid;
    grid-template-columns: 340px minmax(0, 1fr);
    height: 100%;
  }
  .sidebar {
    border-right: 1px solid var(--border);
    padding: 14px;
    overflow-y: auto;
    min-width: 0;
    display: grid;
    /* Keep wide content (a form control's intrinsic width) inside the column. */
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: 12px;
  }
  .content {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 12px;
    align-content: start;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h1 {
    margin: 0;
    font-size: 1.3rem;
  }
  main {
    position: relative;
    min-width: 0;
    min-height: 0;
    display: grid;
    /* The compare bar's names must not widen the viewer past the screen. */
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
  }
  main.comparing {
    grid-template-rows: auto minmax(0, 1fr);
  }
  .panes {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    gap: 1px;
    background: var(--border);
    min-height: 0;
  }
  .compare-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .compare-bar .names {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }
  .compare-bar button {
    padding: 3px 10px;
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .muted {
    color: var(--muted);
  }
  .error {
    color: var(--danger);
    margin: 0;
  }
  .tabs {
    display: none;
  }
  /* Narrow screens: one pane at a time, chosen with the tabs in the header,
     instead of a short sidebar band above the viewer. The sidebar pane is the
     only scroll area and fills the screen; the viewer pane gets all the height
     below the header, so dragging the globe never fights a page scroll. */
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr);
    }
    .layout.show-viewer {
      grid-template-rows: auto minmax(0, 1fr);
    }
    .layout:not(.show-viewer) main,
    .layout.show-viewer .content {
      display: none;
    }
    .sidebar {
      border-right: none;
      /* The header pads itself, so that it can stick to the very top. */
      padding-top: 0;
      padding-bottom: 0;
    }
    .layout.show-viewer .sidebar {
      border-bottom: 1px solid var(--border);
      padding-bottom: 10px;
    }
    header {
      position: sticky;
      top: 0;
      z-index: 1;
      gap: 8px;
      padding-top: 10px;
      background: var(--bg);
    }
    .layout:not(.show-viewer) header {
      padding-bottom: 10px;
      border-bottom: 1px solid var(--border);
      margin: 0 -14px;
      padding-left: 14px;
      padding-right: 14px;
    }
    .content {
      padding-bottom: 14px;
    }
    .tabs {
      display: flex;
      border: 1px solid var(--border);
      border-radius: 6px;
      overflow: hidden;
      margin-left: auto;
    }
    .tabs button {
      border: none;
      border-radius: 0;
    }
    .tabs button.active {
      background: var(--accent);
      color: var(--accent-text);
    }
    /* Two worlds stack vertically on a phone. */
    .panes {
      grid-auto-flow: row;
      grid-auto-rows: minmax(0, 1fr);
    }
  }
</style>
