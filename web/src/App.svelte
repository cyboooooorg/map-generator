<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import DriftPanel from './components/DriftPanel.svelte';
  import GenerateForm from './components/GenerateForm.svelte';
  import JobsIndicator from './components/JobsIndicator.svelte';
  import LanguageSelect from './components/LanguageSelect.svelte';
  import QualitySelect from './components/QualitySelect.svelte';
  import ThemeSelect from './components/ThemeSelect.svelte';
  import SystemOrrery from './components/SystemOrrery.svelte';
  import Toasts from './components/Toasts.svelte';
  import WorldInfo from './components/WorldInfo.svelte';
  import BodyCard from './components/BodyCard.svelte';
  import PinButton from './components/PinButton.svelte';
  import RenameForm from './components/RenameForm.svelte';
  import SystemList from './components/SystemList.svelte';
  import WorldPane from './components/WorldPane.svelte';
  import { api } from './lib/api/client';
  import type { FormSpec } from './lib/api/types/FormSpec';
  import type { SystemCreated } from './lib/api/types/SystemCreated';
  import type { SystemSummary } from './lib/api/types/SystemSummary';
  import type { PresetInfo } from './lib/api/types/PresetInfo';
  import type { WorldRequest } from './lib/api/types/WorldRequest';
  import { driftRange, driftRequest, formatDrift } from './lib/drift';
  import { keepAlone, requestFromWorld, rerollRequest } from './lib/format';
  import { errorText } from './lib/i18n/errors';
  import { setPresetLocales } from './lib/i18n/names.svelte';
  import { getLocale, t } from './lib/i18n/index.svelte';
  import { jobs, type TrackedJob } from './lib/jobs.svelte';
  import { decodeSystems, decodeView, systemHash, type ViewLink } from './lib/link';
  import { loadSetting, saveSetting } from './lib/settings';
  import { detectRenderer, loadQualityChoice } from './lib/viewer/quality';
  import { stepWorld } from './lib/shortcuts';
  import { toasts } from './lib/toasts.svelte';
  import type { LinkedDisplay } from './lib/display';
  import type { CameraState } from './lib/viewer/PlanetView';
  import { LoadedWorld } from './lib/world.svelte';

  /** How long a deleted world can be restored, in ms. */
  const UNDO_MS = 6000;
  /** Shortest wait between two list refreshes on focus, in ms. */
  const REFRESH_EVERY_MS = 5000;

  let systems = $state<SystemSummary[]>([]);
  let presets = $state<PresetInfo[]>([]);
  let spec = $state<FormSpec | null>(null);
  let selectedId = $state<string | null>(null);
  let compareId = $state<string | null>(null);
  let fatal = $state<string | null>(null);
  /** Set once the address has been read: until then, writing it would erase it. */
  let ready = $state(false);
  /**
   * The creation wizard, when open: from one of the menu's entries, or
   * varying a world (over the viewer, on its summary).
   */
  let form = $state<{
    initial: WorldRequest | null;
    variedFrom: string | null;
    entry: 'around' | 'system' | null;
    atSummary: boolean;
    /** Opened from the Drift panel: the view to restore on the new world's planet once it is made. */
    fromDrift?: { view: ViewLink | null; epoch: string };
  } | null>(null);
  /** Systems deleted but still restorable: hidden, and removed on the server once the undo delay ends. */
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
   * Set when a system is opened from the menu: the address then gets a new
   * history entry, so that the browser's Back returns to the menu.
   */
  let pushNext = false;

  const primary = new LoadedWorld();
  const secondary = new LoadedWorld();
  let primaryPane = $state<WorldPane | null>(null);
  let secondaryPane = $state<WorldPane | null>(null);
  /** The job whose world opens when ready, if the selection has not changed meanwhile. */
  let autoOpen: { jobId: string; selection: string | null } | null = null;
  /**
   * A world being generated from the Drift panel: the planet shown stays, dimmed
   * under a notice, until the job ends, then the new world's planet opens in the
   * view the panel left (no detour through the new system's star view).
   */
  let driftJob = $state<{ jobId: string; from: string; epoch: string; view: ViewLink | null } | null>(null);
  const driftStatus = $derived(driftJob ? jobs.list.find((j) => j.jobId === driftJob!.jobId)?.status : undefined);
  /** Opens the planet view on the system's focus planet, for a world made from the Drift panel. */
  let pendingFocus = false;
  /** Deletions waiting for their undo delay, flushed if the page closes. */
  const pendingDeletes = new Set<string>();

  const visible = $derived(systems.filter((s) => !hidden.includes(s.id)));
  /** The viewer bar's name is being edited. */
  let renamingTitle = $state(false);
  /** Closes the bar's rename form, the focus going back to its pencil. */
  async function closeTitleEditor(): Promise<void> {
    renamingTitle = false;
    await tick();
    document.querySelector<HTMLElement>('.viewer-bar .rename-title')?.focus();
  }
  $effect(() => {
    void selectedId;
    renamingTitle = false;
  });
  /** A system was renamed or (un)pinned, here or in the menu: the viewer's star card reads the names again. */
  const changed = (summary: SystemSummary) => {
    systems = systems.map((s) => (s.id === summary.id ? summary : s));
    if (primary.systemId === summary.id) primary.loadSystem(summary.id);
  };
  const summaryOf = (id: string | null) => systems.find((s) => s.id === id) ?? null;
  /** A system's revision: when its file last changed (a body generated, its names). */
  const revision = (id: string | null) => summaryOf(id)?.modified_ms ?? null;
  /** A system's name in the compare bar: the one given to its star, else its designation. */
  const titleOf = (id: string | null) => {
    const summary = summaryOf(id);
    return summary ? (summary.names['star'] ?? summary.designation) : id;
  };

  /** When the list was last asked for (ms), to throttle the focus refreshes. */
  let lastRefresh = 0;

  async function refresh(): Promise<void> {
    lastRefresh = Date.now();
    systems = await api.systems();
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
   * The address says what is shown: no hash, or `#menu`, is the menu, and
   * `#system=<id>&body=<id>&compare=<id>&pins=…` a system's viewer
   * (`body=star` its star view, no body its system view), so a reload, a
   * shared link or the browser's Back and Forward show it. A copied link
   * also carries the view (see lib/link.ts), applied once to the main
   * pane; the address then names only the systems.
   */
  function applyHash(): void {
    const params = new URLSearchParams(location.hash.slice(1));
    const { system, body, compare, pins = [] } = decodeSystems(params);
    const known = (id: string | null) => (id && systems.some((s) => s.id === id) ? id : null);
    if (known(system)) {
      // The system shown already: its pins change at once; another one takes them once it loads.
      if (system === selectedId) primary.pinnedPlanets = [...pins];
      else pendingPins = [...pins];
      pendingBody = body;
      selectedId = system;
      compareId = known(compare);
      pendingView = decodeView(params);
    } else {
      // The menu (or a system that is no more); a world's variation goes with its viewer.
      selectedId = null;
      compareId = null;
      if (form?.variedFrom) form = null;
    }
  }
  /** The body a link names, shown once its system loads. */
  let pendingBody: string | null = null;
  /** The pinned cards a link names, set once its system loads. */
  let pendingPins: string[] = [];

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
    const hash = systemHash(
      { system: selectedId, body: primary.linkBody, compare: compareId, pins: primary.pinnedPlanets },
      primaryPane?.viewState() ?? null,
    );
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
        setPresetLocales(presets);
        jobs.defaultSize = [spec.default_width, spec.default_height];
      }
      await refresh();
      if (!ready) {
        // No system is opened unless the address names one: the app opens on its menu.
        applyHash();
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
    // Light quality picked for a browser drawing on its processor: said
    // once, the quality menu telling it again (see lib/viewer/quality.ts).
    if (loadQualityChoice() === 'auto' && detectRenderer().software && !loadSetting('qualityNotice', false)) {
      toasts.show(t.quality.lightNotice, { ms: 12000 });
      saveSetting('qualityNotice', true);
    }
    window.addEventListener('hashchange', applyHash);
    window.addEventListener('popstate', applyHash);
    window.addEventListener('pagehide', flushDeletes);
    window.addEventListener('focus', refreshOnReturn);
    document.addEventListener('visibilitychange', refreshOnReturn);
    return () => {
      window.removeEventListener('hashchange', applyHash);
      window.removeEventListener('popstate', applyHash);
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
    const hash =
      systemHash({
        system: selectedId,
        body: primary.linkBody,
        compare: compareId,
        pins: primary.pinnedPlanets,
      }) || 'menu';
    if (location.hash.slice(1) !== hash) {
      // Opening a system from the menu is a step Back undoes; the rest
      // (a body, a pin) only keeps the address in step.
      if (untrack(() => pushNext)) history.pushState({ fromMenu: true }, '', `#${hash}`);
      else history.replaceState(history.state, '', `#${hash}`);
    }
    pushNext = false;
  });

  $effect(() => {
    const id = selectedId;
    untrack(() => {
      if (id) {
        primary.loadSystem(id, pendingBody, pendingFocus);
        pendingFocus = false;
        primary.pinnedPlanets = pendingPins;
        pendingBody = null;
        pendingPins = [];
      } else {
        primary.load(null);
      }
    });
  });
  $effect(() => {
    const id = selectedId ? compareId : null;
    untrack(() => {
      if (id) {
        // The comparison is of maps: its pane opens on a body, not the star.
        secondary.loadSystem(id);
        secondary.subject = 'planet';
      } else {
        secondary.load(null);
      }
    });
  });

  jobs.onFinished = (job: TrackedJob) => {
    const status = job.status;
    void refresh().then(() => {
      if (status.status === 'done') {
        // A body generated in the system shown: load the system again,
        // which keeps the body shown and loads its maps.
        if (status.world_id === selectedId) {
          primary.loadSystem(status.world_id);
        }
        if (selectedId && status.world_id === compareId) {
          secondary.loadSystem(status.world_id);
        }
        if (driftJob?.jobId === job.jobId) {
          // Still on the world it was made from: swap to the new planet, in the recorded view.
          const open = driftJob;
          driftJob = null;
          if (selectedId === open.from) {
            pendingFocus = true;
            pendingView = open.view;
            select(status.world_id);
          }
          toasts.show(t.app.ready({ id: titleOf(status.world_id) ?? status.world_id }));
        } else if (autoOpen?.jobId === job.jobId && autoOpen.selection === selectedId) {
          select(status.world_id);
          toasts.show(t.app.ready({ id: titleOf(status.world_id) ?? status.world_id }));
        } else {
          toasts.show(t.app.ready({ id: titleOf(status.world_id) ?? status.world_id }), {
            action: { label: t.app.open, run: () => select(status.world_id) },
            ms: 8000,
          });
        }
        // Finished jobs leave the list after a while.
        setTimeout(() => jobs.dismiss(job.jobId), 8000);
      } else if (status.status === 'failed') {
        if (driftJob?.jobId === job.jobId) driftJob = null;
        toasts.error(t.app.failed({ id: job.worldId, error: status.error }));
      } else if (status.status === 'cancelled') {
        if (driftJob?.jobId === job.jobId) driftJob = null;
        toasts.show(t.app.cancelled({ id: job.worldId }));
        setTimeout(() => jobs.dismiss(job.jobId), 4000);
      }
    });
  };

  function select(id: string): void {
    if (id === compareId) {
      compareId = null;
    }
    if (selectedId === null) pushNext = true;
    selectedId = id;
  }

  /** Back to the menu: the browser's Back when the menu opened the viewer, so that history stays linear. */
  function toMenu(): void {
    form = null;
    if (history.state?.fromMenu) {
      history.back();
    } else {
      selectedId = null;
      compareId = null;
    }
  }

  /** `[` / `]` in the viewer: the previous or next world of the list. */
  function step(direction: 1 | -1): void {
    const next = stepWorld(listOrder, selectedId, direction);
    if (next) {
      select(next);
    }
  }

  function compare(a: string, b: string): void {
    if (selectedId === null) pushNext = true;
    selectedId = a;
    compareId = b;
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
    // The system shown deleted: back to the menu.
    if (selectedId && ids.includes(selectedId)) {
      selectedId = null;
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
        return api.deleteSystem(id);
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
      void fetch(`/api/systems/${encodeURIComponent(id)}`, { method: 'DELETE', keepalive: true });
    }
    pendingDeletes.clear();
  }

  /** Opens the creation wizard: on an entry of the menu, or on the summary of a varied world. */
  function openForm(entry: 'around' | 'system' | null, initial: WorldRequest | null = null, variedFrom: string | null = null): void {
    form = { initial, variedFrom, entry, atSummary: variedFrom !== null };
  }

  /** A system drawn: shown at once (its bodies as spheres), its planet's maps following its job. */
  function submitted(created: SystemCreated, request: WorldRequest | null): void {
    const fromDrift = form?.fromDrift;
    form = null;
    // From the Drift panel the current planet stays shown until its job ends.
    void refresh().then(() => {
      if (!fromDrift || !created.job_id) select(created.system_id);
    });
    if (created.ignored.length > 0) {
      toasts.show(t.app.ignored({ fields: created.ignored.join(', ') }));
    }
    if (created.job_id) {
      if (fromDrift && selectedId) {
        driftJob = { jobId: created.job_id, from: selectedId, epoch: fromDrift.epoch, view: fromDrift.view };
      } else {
        autoOpen = { jobId: created.job_id, selection: created.system_id };
      }
      jobs.track(created.job_id, created.system_id, request ?? {}, { status: 'queued' }, true);
    }
  }

  /** Queues a body of a system and its neighbourhood, from its card. */
  async function generateBody(system: string, body: string): Promise<void> {
    try {
      const started = await api.generateBody(system, body);
      if (started.job_id) {
        jobs.track(started.job_id, system, {}, { status: 'queued' }, true);
      }
    } catch (e) {
      toasts.error(t.app.failed({ id: titleOf(system) ?? system, error: errorText(e) }));
    }
  }

  /** Queues the selected world again with a new random seed. */
  async function reroll(): Promise<void> {
    if (!primary.meta) {
      return;
    }
    const request = keepAlone(rerollRequest(primary.meta), primary.system);
    try {
      submitted(await api.createSystem({ around: request }), request);
    } catch (e) {
      toasts.error(t.app.rerollFailed({ error: errorText(e) }));
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

{#snippet info()}
  {#if primary.system}
    <BodyCard
      world={primary}
      onRenamed={(renamed) => (systems = systems.map((s) => (s.id === renamed.id ? renamed : s)))}
      onGenerate={generateBody}
    />
  {/if}
  {#if !primary.system || primary.shownSystemBody?.generated}
    <WorldInfo
      world={primary}
      title={primary.shownSystemBody ? (primary.systemNames[primary.shownSystemBody.id] ?? primary.shownSystemBody.designation) : null}
      revision={revision(selectedId)}
      onVary={() => primary.meta && openForm('around', keepAlone(requestFromWorld(primary.meta), primary.system), primary.id)}
      onReroll={reroll}
      link={shareLink}
      onDelete={() => selectedId && removeWorlds([selectedId])}
    />
  {/if}
{/snippet}

{#snippet drift()}
  {#if primary.meta && primary.id && primary.size}
    {#key `${primary.id} ${primary.size}`}
      <DriftPanel
        id={primary.id}
        size={primary.size}
        revision={primary.revision}
        current={primary.meta.drift_ma ?? 0}
        onGenerate={(ma) => {
          if (!primary.meta) return;
          openForm('around', keepAlone(driftRequest(primary.meta, ma), primary.system), primary.id);
          if (form) form.fromDrift = { view: primaryPane?.viewState() ?? null, epoch: formatDrift(ma) };
        }}
      />
    {/key}
  {/if}
{/snippet}

{#snippet fatalError()}
  {#if fatal}
    <p class="error" role="alert">
      {t.app.unreachable({ error: fatal })}
      <button onclick={() => void start()} disabled={starting}>{starting ? t.app.retrying : t.app.retry}</button>
      <button onclick={() => (fatal = null)}>{t.app.dismiss}</button>
    </p>
  {/if}
{/snippet}

{#if selectedId === null}
  <!-- The main menu: the systems as cards, and the two ways to create one. -->
  <div class="menu">
    <header class="menu-bar">
      <h1>{t.app.title}</h1>
      <span class="spacer"></span>
      <JobsIndicator onOpen={select} />
      <QualitySelect />
      <ThemeSelect />
      <LanguageSelect />
    </header>
    {@render fatalError()}
    <section class="create" aria-label={t.menu.create}>
      <button class="entry" onclick={() => openForm('system')} disabled={!spec}>
        <strong>{t.menu.randomSystem}</strong>
        <small>{t.menu.randomSystemHelp}</small>
      </button>
      <button class="entry" onclick={() => openForm('around')} disabled={!spec}>
        <strong>{t.menu.aroundPlanet}</strong>
        <small>{t.menu.aroundPlanetHelp}</small>
      </button>
    </section>
    <section class="systems" aria-label={t.menu.systems}>
      <h2>{t.menu.systems}</h2>
      <SystemList
        systems={visible}
        {selectedId}
        {compareId}
        bind:order={listOrder}
        onSelect={select}
        onDelete={removeWorlds}
        onCompare={compare}
        onChanged={changed}
      />
    </section>
  </div>
{:else}
  <div class="viewer-screen">
    <header class="viewer-bar">
      <button onclick={toMenu} title={t.menu.backHelp}>← {t.menu.back}</button>
      <span class="names">
        {#if summaryOf(selectedId)}<SystemOrrery system={summaryOf(selectedId)!} width={40} height={22} radius={3} />{/if}
        {#if summaryOf(selectedId) && renamingTitle}
          <RenameForm system={summaryOf(selectedId)!} onChanged={changed} onClose={closeTitleEditor} />
        {:else}
          <strong title={selectedId ?? ''}>{titleOf(selectedId)}</strong>
          {#if summaryOf(selectedId)}
            <PinButton system={summaryOf(selectedId)!} onChanged={changed} />
            <button
              class="rename-title"
              onclick={() => (renamingTitle = true)}
              aria-label={t.systemList.rename({ name: titleOf(selectedId) ?? '' })}
              title={t.systemList.rename({ name: titleOf(selectedId) ?? '' })}>✎</button
            >
          {/if}
        {/if}
        {#if compareId}
          <span class="muted">{t.app.versus}</span>
          {#if summaryOf(compareId)}<SystemOrrery system={summaryOf(compareId)!} width={40} height={22} radius={3} />{/if}
          <strong title={compareId}>{titleOf(compareId)}</strong>
        {/if}
      </span>
      {#if compareId}
        <label class="check"><input type="checkbox" bind:checked={linkCameras} /> {t.app.linkCameras}</label>
        <label class="check" title={t.app.linkDisplayHelp}><input type="checkbox" bind:checked={linkDisplay} /> {t.app.linkDisplay}</label>
        <button onclick={swap}>{t.app.swap}</button>
        <button onclick={() => (compareId = null)}>{t.app.close}</button>
      {/if}
      <JobsIndicator onOpen={select} />
      <QualitySelect />
      <ThemeSelect />
      <LanguageSelect />
    </header>
    {@render fatalError()}
    <main class="panes" class:drifting={driftJob !== null}>
      <WorldPane
        bind:this={primaryPane}
        world={primary}
        revision={revision(selectedId)}
        keyboard={form === null}
        label={compareId ? titleOf(selectedId) : null}
        {info}
        drift={primary.meta && driftRange(primary.meta) ? drift : undefined}
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
          label={titleOf(compareId)}
          onCameraChange={cameraFrom(() => primaryPane)}
        />
      {/if}
      {#if driftJob}
        <div class="drift-busy" role="status" aria-live="polite">
          <p>{t.drift.generating({ epoch: driftJob.epoch })}</p>
          {#if driftStatus?.status === 'running'}
            <progress max={driftStatus.steps} value={driftStatus.step - 1}></progress>
            <small>{t.jobs.stageStep({ stage: t.stages[driftStatus.stage], step: driftStatus.step, steps: driftStatus.steps })}</small>
          {:else}
            <small>{t.jobs.waiting}</small>
          {/if}
        </div>
      {/if}
    </main>
  </div>
{/if}

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape' && form) {
      form = null;
      event.stopPropagation();
    }
  }}
/>

{#if form && spec}
  <!-- The creation wizard, over the menu or the viewer. -->
  <div class="wizard-backdrop">
    <div class="wizard" role="dialog" aria-modal="true" aria-label={form.variedFrom ? t.form.vary({ id: form.variedFrom }) : t.form.newSystem}>
      <GenerateForm
        {presets}
        {spec}
        initial={form.initial}
        variedFrom={form.variedFrom}
        entry={form.entry}
        atSummary={form.atSummary}
        onSubmitted={submitted}
        onCancel={() => (form = null)}
      />
    </div>
  </div>
{/if}

<Toasts />

<style>
  .menu {
    height: 100%;
    overflow-y: auto;
    padding: 16px clamp(16px, 4vw, 48px) 32px;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: 20px;
  }
  .menu-bar,
  .viewer-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
  }
  .spacer {
    flex: 1;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.6rem;
  }
  h2 {
    margin: 0 0 10px;
    font-family: var(--font-display);
    font-size: 1.1rem;
  }
  .create {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 18em), 1fr));
    gap: 14px;
  }
  .entry {
    display: grid;
    gap: 6px;
    padding: 18px 20px;
    text-align: left;
    white-space: normal;
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--glow);
  }
  .entry strong {
    font-family: var(--font-display);
    font-size: 1.15rem;
    color: var(--accent);
  }
  .entry small {
    color: var(--muted);
  }
  .entry:hover:not(:disabled),
  .entry:focus-visible {
    border-color: var(--accent);
  }
  .viewer-screen {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .viewer-screen > .panes {
    flex: 1;
  }
  .viewer-bar {
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
  }
  .viewer-bar .names {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }
  .viewer-bar .names strong {
    font-family: var(--font-display);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .viewer-bar button {
    padding: 3px 10px;
  }
  .viewer-bar .names :global(.pin),
  .viewer-bar .rename-title {
    padding: 3px 8px;
  }
  .viewer-bar .names :global(.rename) {
    flex: 1;
    overflow: visible;
  }
  /* A world is being made from the Drift panel: the one shown waits, dimmed, under a notice. */
  .panes {
    position: relative;
  }
  .drifting :global(.pane) {
    filter: blur(4px) brightness(0.55);
    pointer-events: none;
  }
  .drift-busy {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    z-index: 5;
    pointer-events: none;
    text-align: center;
  }
  .drift-busy p {
    margin: 0;
    font-size: 1.2em;
  }
  .drift-busy progress {
    width: min(320px, 70%);
  }
  .panes {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    gap: 1px;
    background: var(--border);
    min-height: 0;
  }
  .wizard-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 16px;
    background: color-mix(in srgb, var(--bg) 70%, transparent);
  }
  .wizard {
    width: min(720px, 100%);
    max-height: 100%;
    overflow-y: auto;
    padding: 16px 18px 0;
    background: var(--frame-deco), var(--panel-solid);
    clip-path: var(--frame-clip);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 12px 40px color-mix(in srgb, var(--bg) 80%, transparent), var(--glow);
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
  /* A phone: the two compared worlds stack, and the wizard fills the screen. */
  @media (max-width: 760px) {
    .panes {
      grid-auto-flow: row;
      grid-auto-rows: minmax(0, 1fr);
    }
    .wizard-backdrop {
      padding: 0;
    }
    .wizard {
      height: 100%;
      border-radius: 0;
    }
  }
</style>
