<script lang="ts">
  // The live preview beside the generate form: the six body cards of what the
  // form would draw, the seed it draws it with, and a quiet state while the
  // server answers. The cards are the viewer's, fed by the resolved body.
  import { t } from '../../lib/i18n/index.svelte';
  import type { InputField } from '../../lib/api/types/InputField';
  import { previewSource } from '../../lib/cardSource';
  import type { Preview } from '../../lib/preview.svelte';
  import AtmosphereCard from '../body/AtmosphereCard.svelte';
  import BodyHeader from '../body/BodyHeader.svelte';
  import InteriorCard from '../body/InteriorCard.svelte';
  import MoonsCard from '../body/MoonsCard.svelte';
  import OrbitCard from '../body/OrbitCard.svelte';
  import SurfaceCard from '../body/SurfaceCard.svelte';

  interface Props {
    preview: Preview;
    /** A leaf of a cause tree was pressed: the form field it comes from. */
    onjump: (field: InputField) => void;
    /** "New draw": the form picks another seed. */
    onreroll: () => void;
  }

  let { preview, onjump, onreroll }: Props = $props();

  const world = $derived(preview.result ? previewSource(preview.result, preview.changed) : null);
  // A preview has no name to give nor maps to queue.
  const never = () => {};
</script>

<section class="preview" aria-label={t.form.preview.title} aria-busy={preview.pending}>
  <header>
    <h3>{t.form.preview.title}</h3>
    {#if preview.result}
      <span class="seed">{t.form.preview.seed} <code>{preview.result.system.seed}</code></span>
      <button type="button" onclick={onreroll} title={t.form.preview.newDrawHelp}>{t.form.preview.newDraw}</button>
    {/if}
    <span class="status" role="status">
      {#if preview.unavailable}<span class="warn">{t.form.preview.unavailable}</span>{:else if preview.pending}{t.form.preview.updating}{/if}
    </span>
  </header>
  {#if world}
    <div class="cards" class:stale={preview.pending}>
      <BodyHeader {world} onRenamed={never} onGenerate={never} oninput={onjump} />
      <OrbitCard {world} />
      <InteriorCard {world} oninput={onjump} />
      <AtmosphereCard {world} oninput={onjump} />
      <SurfaceCard {world} />
      <MoonsCard {world} />
    </div>
  {:else if !preview.unavailable}
    <p class="muted">{t.form.preview.waiting}</p>
  {/if}
</section>

<style>
  .preview {
    display: grid;
    gap: 8px;
    min-width: 0;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
  }
  h3 {
    margin: 0;
    font-size: 1rem;
  }
  .seed {
    color: var(--muted);
  }
  header button {
    padding: 2px 8px;
  }
  .status {
    flex-basis: 100%;
    min-height: 1.2em;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .warn {
    color: var(--danger);
  }
  .cards {
    display: grid;
    gap: 10px;
    transition: opacity 0.2s;
  }
  /* Quiet while the answer comes: the values stay, a little dimmer. */
  .cards.stale {
    opacity: 0.75;
  }
  .cards > :global(*) {
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }
  .muted {
    color: var(--muted);
    margin: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .cards {
      transition: none;
    }
  }
</style>
