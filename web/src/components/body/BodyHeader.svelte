<script lang="ts">
  // The header of a body's details: its name (renamed here), its class, the
  // archetype it resembles, a button to its host for a moon, and whether its
  // maps exist.
  import { t } from '../../lib/i18n/index.svelte';
  import { archetypeName, classLabel } from '../../lib/i18n/names.svelte';
  import type { SystemSummary } from '../../lib/api/types/SystemSummary';
  import { jobs } from '../../lib/jobs.svelte';
  import type { LoadedWorld } from '../../lib/world.svelte';
  import CauseTree from './CauseTree.svelte';
  import { treeFor } from '../../lib/causes';
  import NameEditor from './NameEditor.svelte';

  interface Props {
    world: LoadedWorld;
    onRenamed: (summary: SystemSummary) => void;
    /** Queues the generation of a body and its neighbourhood. */
    onGenerate: (system: string, body: string) => void;
  }

  let { world, onRenamed, onGenerate }: Props = $props();

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
  const similar = $derived(world.shownExtras?.similar ?? null);
</script>

{#if system && body}
  {@const title = nameOf(body.id, body.designation)}
  <section class="header" aria-label={t.card.body}>
    <header>
      <NameEditor {world} id={body.id} designation={body.designation} name={title} {onRenamed} />
      {#if host}
        <button class="host" onclick={() => world.showSystemBody(host.id)} title={t.cards.hostHelp}
          >{t.cards.host({ name: nameOf(host.id, host.designation) })}</button
        >
      {/if}
      <small
        >{title === body.designation ? '' : `${body.designation} · `}{classLabel(body.type, body.model, body.kind === 'moon')} · {host
          ? t.card.moonOf({ planet: nameOf(host.id, host.designation) })
          : t.card.planetOf({ star: starName })}</small
      >
      <div class="why">
        <CauseTree tree={treeFor(explanation, 'class')} open={classOpen} ontoggle={() => (classOpen = !classOpen)} />
      </div>
      {#if similar}
        <small class="similar">{t.cards.similar({ name: archetypeName({ name: similar }) })}</small>
      {/if}
    </header>
    {#if !body.generated}
      <p class="muted">{busy ? t.card.generating : world.systemEvicted.includes(body.id) ? t.card.evicted : t.card.notGenerated}</p>
      {#if !busy}
        <button class="primary" onclick={() => world.systemId && onGenerate(world.systemId, body.id)}>{t.card.generate}</button>
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
  .why {
    flex-basis: 100%;
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
