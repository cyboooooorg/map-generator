<script lang="ts">
  // The live preview beside the generate form: what the form would draw, the
  // seed it draws it with, and a quiet state while the server answers. A lone
  // world shows the six body cards of its resolved body (the viewer's own); a
  // whole system, its star and planets. While the request is refused (or the
  // server silent) what is shown is the last answer, not the current request:
  // it is greyed under a notice that says so.
  import { t } from '../../lib/i18n/index.svelte';
  import { detailText } from '../../lib/i18n/errors';
  import type { InputField } from '../../lib/api/types/InputField';
  import { previewSource } from '../../lib/cardSource';
  import { flashLines } from '../../lib/flashLines';
  import type { Preview } from '../../lib/preview.svelte';
  import AtmosphereCard from '../body/AtmosphereCard.svelte';
  import BodyHeader from '../body/BodyHeader.svelte';
  import InteriorCard from '../body/InteriorCard.svelte';
  import MoonsCard from '../body/MoonsCard.svelte';
  import OrbitCard from '../body/OrbitCard.svelte';
  import SurfaceCard from '../body/SurfaceCard.svelte';
  import SystemPreview from './SystemPreview.svelte';

  interface Props {
    preview: Preview;
    /** A lone world (its cards) or a whole system (its star and planets). */
    kind?: 'around' | 'system';
    /** The panel sits under a folded "Preview" summary: no title of its own. */
    folded?: boolean;
    /** A leaf of a cause tree was pressed: the form field it comes from. */
    onjump?: (field: InputField) => void;
    /** "New draw": the form picks another seed. */
    onreroll: () => void;
  }

  let { preview, kind = 'around', folded = false, onjump = () => {}, onreroll }: Props = $props();

  /** The last answer, when it is of the kind of request shown (a lone world has a focus body, a whole system none). */
  const shown = $derived(preview.result && (kind === 'around') === (preview.result.system.focus !== null) ? preview.result : null);
  const world = $derived(shown && kind === 'around' ? previewSource(shown) : null);
  /** What is shown is not the current request: it was refused, or the server did not answer. */
  const mismatch = $derived(preview.refusal !== null || preview.unavailable);
  // A preview has no name to give nor maps to queue.
  const never = () => {};
</script>

<section class="preview" aria-label={t.form.preview.title} aria-busy={preview.pending}>
  <header>
    {#if !folded}<h3>{t.form.preview.title}</h3>{/if}
    {#if shown}
      <span class="seed">{t.form.preview.seed} <code>{shown.system.seed}</code></span>
      <button type="button" onclick={onreroll} title={t.form.preview.newDrawHelp}>{t.form.preview.newDraw}</button>
    {/if}
    <span class="status" role="status">
      {#if preview.unavailable}<span class="warn">{t.form.preview.unavailable}</span>{:else if preview.pending}{t.form.preview.updating}{/if}
    </span>
  </header>
  {#if shown}
    <div class="stage">
      {#if mismatch}
        <div class="notice">
          <strong>{t.form.preview.mismatch}</strong>
          <span>{preview.refusal ? detailText(preview.refusal) : t.form.preview.mismatchUnavailable}</span>
        </div>
      {/if}
      <div class="content" class:stale={preview.pending && !mismatch} class:mismatch inert={mismatch}>
        {#if world}
          <div class="cards" use:flashLines={{ answers: preview.answers, continued: preview.continued }}>
            <BodyHeader {world} onRenamed={never} onGenerate={never} oninput={onjump} />
            <OrbitCard {world} />
            <InteriorCard {world} oninput={onjump} />
            <AtmosphereCard {world} oninput={onjump} />
            <SurfaceCard {world} />
            <MoonsCard {world} />
          </div>
        {:else if kind === 'system'}
          <SystemPreview system={shown.system} />
        {/if}
      </div>
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
  .stage {
    position: relative;
    display: grid;
  }
  .content {
    transition: opacity 0.2s;
  }
  /* Quiet while the answer comes: the values stay, a little dimmer. */
  .content.stale {
    opacity: 0.75;
  }
  /* Not the current request: what is under the notice is old and cannot be used. */
  .content.mismatch {
    opacity: 0.25;
    filter: grayscale(1);
    user-select: none;
  }
  .notice {
    position: absolute;
    inset: 0 0 auto 0;
    z-index: 1;
    display: grid;
    gap: 2px;
    margin: 8px;
    padding: 8px 10px;
    border: 1px solid var(--danger);
    border-radius: 8px;
    background: var(--panel-solid);
    color: var(--danger);
    font-size: 0.9rem;
  }
  .notice span {
    color: var(--text);
  }
  .cards {
    display: grid;
    gap: 10px;
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
    .content {
      transition: none;
    }
  }
</style>
