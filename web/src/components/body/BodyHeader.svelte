<script lang="ts">
  // The header of a body's details: its name (renamed here), its class, the
  // archetype it resembles, a button to its host for a moon, and whether its
  // maps exist.
  import { t } from '../../lib/i18n/index.svelte';
  import { classLabel, similarLine } from '../../lib/i18n/names.svelte';
  import type { SystemSummary } from '../../lib/api/types/SystemSummary';
  import { jobs } from '../../lib/jobs.svelte';
  import type { CardSource } from '../../lib/cardSource';
  import CauseTree from './CauseTree.svelte';
  import type { InputField } from '../../lib/api/types/InputField';
  import { treeFor } from '../../lib/causes';
  import NameEditor from './NameEditor.svelte';

  interface Props {
    world: CardSource;
    onRenamed: (summary: SystemSummary) => void;
    /** Queues the generation of a body and its neighbourhood. */
    onGenerate: (system: string, body: string) => void;
    /** Makes the input leaves of the class tree buttons (the generate form's preview). */
    oninput?: ((field: InputField) => void) | undefined;
  }

  let { world, onRenamed, onGenerate, oninput }: Props = $props();

  const system = $derived(world.system);
  const body = $derived(world.shownSystemBody);
  const names = $derived(world.systemNames);
  const nameOf = (id: string, designation: string) => names[id] ?? designation;
  /** Whether a job of this system is queued or running: its bodies are on their way. */
  const busy = $derived(jobs.active.some((j) => j.worldId === world.systemId));
  const host = $derived(body?.parent ? (system?.bodies.find((b) => b.id === body.parent) ?? null) : null);
  const starName = $derived(system ? nameOf('star', system.star.designation) : '');
  const explanation = $derived(world.shownExtras?.explanation ?? null);
  let classOpen = $state(false);
  // The tree starts closed on every body.
  // A preview answer is a new object for the same body: only the id counts.
  const bodyId = $derived(world.systemBody);
  $effect(() => {
    void bodyId;
    classOpen = false;
  });
  const similar = $derived(world.shownExtras?.similar ? similarLine(world.shownExtras.similar) : null);
</script>

{#if system && body}
  {@const title = nameOf(body.id, body.designation)}
  <section class="header" aria-label={t.card.body}>
    <header>
      <NameEditor {world} id={body.id} designation={body.designation} name={title} {onRenamed} />
      {#if host && world.preview}
        <span class="host">{t.cards.host({ name: nameOf(host.id, host.designation) })}</span>
      {:else if host}
        <button type="button" class="host" onclick={() => world.showSystemBody(host.id)} title={t.cards.hostHelp}
          >{t.cards.host({ name: nameOf(host.id, host.designation) })}</button
        >
      {/if}
      <!-- The class and the "Why?" of its tree are one line: the mark has a value to explain. -->
      <div class="kind" data-line="class">
        <small
          >{title === body.designation ? '' : `${body.designation} · `}{classLabel(body.type, body.model, body.kind === 'moon')} · {host
            ? t.card.moonOf({ planet: nameOf(host.id, host.designation) })
            : t.card.planetOf({ star: starName })}</small
        >
        <CauseTree tree={treeFor(explanation, 'class')} open={classOpen} ontoggle={() => (classOpen = !classOpen)} {oninput} />
      </div>
      {#if similar}
        <small class="similar" data-line="similar" title={similar.hint}>{similar.text}</small>
      {/if}
    </header>
    {#if !body.generated && !world.preview}
      <p class="muted">{busy ? t.card.generating : world.systemEvicted.includes(body.id) ? t.card.evicted : t.card.notGenerated}</p>
      {#if !busy}
        <button type="button" class="primary" onclick={() => world.systemId && onGenerate(world.systemId, body.id)}>{t.card.generate}</button>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .header {
    display: grid;
    gap: 6px;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 8px;
  }
  header small {
    flex-basis: 100%;
    color: var(--muted);
  }
  .kind {
    flex-basis: 100%;
  }
  .kind small {
    flex-basis: auto;
  }
  .host {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 8px;
    color: var(--link);
  }
  .muted {
    color: var(--muted);
    margin: 0;
  }
</style>
