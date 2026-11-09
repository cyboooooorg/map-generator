<script lang="ts">
  // The resolutions of the planet shown, as a row of chips under the view
  // (the "Resolution" switch of the icon bar): a chip on disk shows it, a
  // dashed one generates it, the × of an added one deletes it.
  import type { BodySizes } from '../lib/api/types/BodySizes';
  import { t } from '../lib/i18n/index.svelte';
  import { chipStates } from '../lib/sizes';

  let {
    sizes,
    shown,
    generating,
    onPick,
    onGenerate,
    onDelete,
  }: {
    sizes: BodySizes;
    /** The resolution shown, `WxH`. */
    shown: string;
    /** The resolution this view is generating, `WxH`. */
    generating: string | null;
    onPick: (size: string) => void;
    onGenerate: (size: string) => void;
    onDelete: (size: string) => void;
  } = $props();

  const chips = $derived(chipStates(sizes, shown, generating));

  function remove(size: string, width: number): void {
    if (confirm(t.viewer.deleteSizeConfirm({ size: String(width) }))) onDelete(size);
  }
</script>

<div class="size-chips" role="group" aria-label={t.viewer.sizes}>
  {#each chips as chip (chip.size)}
    {@const width = String(chip.width)}
    <span class="size" class:present={chip.present} class:shown={chip.shown}>
      {#if chip.present}
        <button aria-pressed={chip.shown} onclick={() => onPick(chip.size)}>{width}</button>
        {#if chip.deletable}
          <button
            class="delete"
            onclick={() => remove(chip.size, chip.width)}
            aria-label={t.viewer.deleteSize({ size: width })}
            title={t.viewer.deleteSize({ size: width })}>×</button
          >
        {/if}
      {:else if chip.generating}
        <button disabled aria-busy="true" title={t.viewer.sizeGenerating({ size: width })}>{width}</button>
      {:else}
        <!-- Too large: focusable, its reason read out with its name (aria-disabled, not disabled). -->
        <button
          aria-disabled={chip.tooLarge ? 'true' : undefined}
          onclick={() => {
            if (!chip.tooLarge) onGenerate(chip.size);
          }}
          title={chip.tooLarge ? t.viewer.sizeTooLarge : t.viewer.generateSize({ size: width })}
          >{width} +{#if chip.tooLarge}<span class="visually-hidden">{t.viewer.sizeTooLarge}</span>{/if}</button
        >
      {/if}
    </span>
  {/each}
</div>

<style>
  .size-chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 6px;
    pointer-events: auto;
  }
  .size {
    display: inline-flex;
    align-items: center;
    border: 1px dashed var(--border);
    border-radius: 999px;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--panel);
    box-shadow: var(--glow);
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
  }
  .size.present {
    border-style: solid;
  }
  .size.shown {
    background: var(--accent);
    border-color: var(--accent);
  }
  .size.shown button {
    color: var(--accent-text);
  }
  button {
    border: none;
    background: none;
    color: inherit;
    padding: 3px 10px;
    border-radius: 999px;
    font: inherit;
    cursor: pointer;
  }
  .size:not(.present) button {
    color: var(--muted);
  }
  button:disabled,
  button[aria-disabled='true'] {
    cursor: default;
    opacity: 0.55;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  button[aria-busy='true'] {
    opacity: 1;
    animation: busy 1.2s ease-in-out infinite alternate;
  }
  .delete {
    padding: 3px 8px 3px 2px;
    color: var(--muted);
  }
  .delete:hover {
    color: var(--danger);
  }
  @keyframes busy {
    to {
      opacity: 0.4;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    button[aria-busy='true'] {
      animation: none;
    }
  }
</style>
