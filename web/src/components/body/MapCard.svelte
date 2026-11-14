<script lang="ts">
  // The generated map: its seed and archetype, when it was made, and what to
  // do with it: generate it again, vary it, copy its command or link, delete
  // it, download its files.
  import './facts.css';
  import { getLocale, t } from '../../lib/i18n/index.svelte';
  import { worldFile } from '../../lib/api/client';
  import { downloadRawData, downloadSvg } from '../../lib/download';
  import { cliCommand, copyText, formatAgo } from '../../lib/format';
  import { toasts } from '../../lib/toasts.svelte';
  import type { LoadedWorld } from '../../lib/world.svelte';

  interface Props {
    world: LoadedWorld;
    /** When the body's maps were made (ms), for the card and the download links. */
    revision?: number | null;
    onVary: () => void;
    /** Generates the same world again with a new random seed. */
    onReroll: () => Promise<void>;
    /** The link Copy link copies: the worlds shown and the view. */
    link: () => string;
    onDelete: () => void;
  }

  let { world, revision = null, onVary, onReroll, link, onDelete }: Props = $props();

  let zipping = $state(false);
  let drawingSvg = $state(false);
  let rerolling = $state(false);

  const meta = $derived(world.meta);
  /** The body shown, whose maps these are. */
  const id = $derived(world.id);
  const size = $derived(meta ? world.size : null);

  async function copy(text: string, what: string): Promise<void> {
    const ok = await copyText(text);
    toasts.show(ok ? t.worldInfo.copied({ what }) : t.worldInfo.copyRefused, { kind: ok ? 'info' : 'error' });
  }
  async function reroll(): Promise<void> {
    rerolling = true;
    try {
      await onReroll();
    } finally {
      rerolling = false;
    }
  }
  async function exportSvg(): Promise<void> {
    if (!id || !size) return;
    drawingSvg = true;
    try {
      await downloadSvg(id, size);
    } catch (e) {
      toasts.error(t.worldInfo.svgFailed({ error: String(e) }));
    } finally {
      drawingSvg = false;
    }
  }
  async function zip(): Promise<void> {
    if (!id || !meta || !size) return;
    zipping = true;
    try {
      await downloadRawData(id, size, meta);
    } catch (e) {
      toasts.error(t.worldInfo.archiveFailed({ error: String(e) }));
    } finally {
      zipping = false;
    }
  }
</script>

{#if meta}
  <dl class="facts">
    <dt>{t.worldInfo.seed}</dt>
    <dd>{meta.seed}</dd>
    {#if revision}
      <dt>{t.worldInfo.generated}</dt>
      <dd title={new Date(revision).toLocaleString(getLocale())}>{formatAgo(revision)}</dd>
    {/if}
  </dl>
  <div class="actions">
    <button onclick={reroll} disabled={rerolling} title={t.worldInfo.rerollHelp}>{rerolling ? t.worldInfo.sending : t.worldInfo.reroll}</button>
    <button onclick={onVary} title={t.worldInfo.varyHelp}>{t.worldInfo.vary}</button>
    <button onclick={() => copy(cliCommand(meta), t.worldInfo.command)} title={t.worldInfo.copyCommandHelp}>{t.worldInfo.copyCommand}</button>
    <button onclick={() => copy(link(), t.worldInfo.link)} title={t.worldInfo.copyLinkHelp}>{t.worldInfo.copyLink}</button>
    <button class="danger" onclick={onDelete}>{t.worldInfo.delete}</button>
  </div>
  <div class="downloads">
    <span>{t.worldInfo.download}</span>
    {#if id && size}
      <a href={worldFile(id, size, 'world.png', world.revision)} download={`${id}.png`}>{t.worldInfo.mapPng}</a>
      <a href={worldFile(id, size, 'legend.png', world.revision)} download={`${id}-legend.png`}>{t.worldInfo.legend}</a>
      <button class="link" onclick={exportSvg} disabled={drawingSvg} title={t.worldInfo.exportSvgHelp}
        >{drawingSvg ? t.worldInfo.drawingSvg : t.worldInfo.exportSvg}</button
      >
    {/if}
    <button class="link" onclick={zip} disabled={zipping}>{zipping ? t.worldInfo.zipping : t.worldInfo.rawData}</button>
  </div>
{/if}

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 8px 0;
  }
  .actions button {
    padding: 4px 10px;
  }
  .danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }
  .downloads {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    align-items: baseline;
    font-size: 0.9rem;
  }
  .downloads span {
    color: var(--muted);
  }
  .downloads a,
  .link {
    color: var(--link);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
  }
</style>
