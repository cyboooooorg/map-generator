<script lang="ts">
  // The drift slider: a low-resolution preview of the whole world as its
  // plates carried it at an epoch, and "generate at this epoch", which opens
  // the generation form on the same planet with that epoch (docs/backlog.md,
  // item 9, step 3).
  import { untrack } from 'svelte';
  import { ApiFailure } from '../lib/api/client';
  import {
    DRIFT_LIMIT_MA,
    DRIFT_STEP_MA,
    PREVIEW_WIDTH,
    driftPreviewUrl,
    formatDrift,
    snapDrift,
  } from '../lib/drift';
  import { t } from '../lib/i18n/index.svelte';

  interface Props {
    /** Id of the body shown (`systems/<system>/bodies/<body>`). */
    id: string;
    /** The resolution shown (`WxH`), which the previews are made from. */
    size: string;
    /** That resolution's revision, which keys the cached previews. */
    revision: number | null;
    /** The epoch of the world shown, Ma from the present (negative in the past). */
    current: number;
    /** Opens the generation form for the world at this epoch. */
    onGenerate: (driftMa: number) => void;
  }

  let { id, size, revision, current, onGenerate }: Props = $props();

  /** The epoch chosen, and the one under the slider while it is dragged. */
  let choice = $state(untrack(() => snapDrift(current)));
  let dragging = $state<number | null>(null);
  const shown = $derived(dragging ?? choice);
  let image = $state<string | null>(null);
  let loading = $state(false);
  let message = $state<string | null>(null);

  /** A coarse preview a moment after the slider stops moving, a finer one once it is released. */
  const DRAG_DELAY_MS = 150;

  $effect(() => {
    const ma = shown;
    const width = dragging === null ? PREVIEW_WIDTH.fine : PREVIEW_WIDTH.coarse;
    const controller = new AbortController();
    const timer = setTimeout(
      () => {
        loading = true;
        void load(ma, width, controller.signal);
      },
      dragging === null ? 0 : DRAG_DELAY_MS,
    );
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  });

  async function load(ma: number, width: number, signal: AbortSignal): Promise<void> {
    try {
      const response = await fetch(driftPreviewUrl(id, size, ma, width, revision), { signal });
      if (!response.ok) {
        const body: { error?: string; detail?: { code?: string } } | null = await response.json().catch(() => null);
        throw new ApiFailure(response.status, body?.error ?? response.statusText);
      }
      const url = URL.createObjectURL(await response.blob());
      if (signal.aborted) {
        URL.revokeObjectURL(url);
        return;
      }
      if (image) URL.revokeObjectURL(image);
      image = url;
      message = null;
    } catch (e) {
      if (signal.aborted) return;
      message = e instanceof ApiFailure && e.status === 503 ? t.drift.busy : t.drift.failed({ error: String(e instanceof Error ? e.message : e) });
    } finally {
      if (!signal.aborted) loading = false;
    }
  }

  $effect(() => () => {
    if (image) URL.revokeObjectURL(image);
  });
</script>

<div class="panel-body drift">
  <p class="help">{t.drift.help}</p>
  <div class="row" role="group" aria-label={t.drift.slider} title={t.drift.help}>
    <span class="label" aria-live="polite">{formatDrift(shown)}</span>
    <input
      type="range"
      min={-DRIFT_LIMIT_MA}
      max={DRIFT_LIMIT_MA}
      step={DRIFT_STEP_MA}
      value={shown}
      oninput={(e) => (dragging = Number(e.currentTarget.value))}
      onchange={(e) => {
        choice = Number(e.currentTarget.value);
        dragging = null;
      }}
      aria-label={t.drift.slider}
      aria-valuetext={formatDrift(shown)}
    />
    <button onclick={() => (choice = 0)} disabled={shown === 0}>{t.drift.backToPresent}</button>
    {#if loading}
      <span class="busy" title={t.drift.loading}>…</span>
    {/if}
  </div>
  <div class="preview">
    {#if image}
      <img src={image} alt={t.drift.previewAlt({ epoch: formatDrift(choice) })} />
    {/if}
  </div>
  {#if message}
    <p class="message" role="status">{message}</p>
  {/if}
  <div class="row">
    <button onclick={() => onGenerate(choice)} disabled={choice === snapDrift(current)} title={t.drift.generateHelp}>{t.drift.generate}</button>
    {#if choice === snapDrift(current)}
      <span class="hint">{t.drift.fromHere({ epoch: formatDrift(current) })}</span>
    {/if}
  </div>
</div>

<style>
  .drift {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .help,
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.85em;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .row input[type='range'] {
    flex: 1;
    min-width: 0;
  }
  .label {
    min-width: 8em;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .busy {
    color: var(--muted);
  }
  .preview img {
    display: block;
    width: 100%;
    height: auto;
    image-rendering: auto;
  }
  .message {
    margin: 0;
    color: var(--danger);
  }
</style>
