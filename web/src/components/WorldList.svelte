<script lang="ts">
  import { worldFile } from '../lib/api/client';
  import type { WorldSummary } from '../lib/api/types/WorldSummary';

  interface Props {
    worlds: WorldSummary[];
    selectedId: string | null;
    onSelect: (id: string) => void;
    onDelete: (world: WorldSummary) => void;
  }

  let { worlds, selectedId, onSelect, onDelete }: Props = $props();
</script>

{#if worlds.length === 0}
  <p class="empty">No world yet.</p>
{:else}
  <ul>
    {#each worlds as world (world.id)}
      <li class:selected={world.id === selectedId}>
        <button class="pick" onclick={() => onSelect(world.id)}>
          <img src={worldFile(world.id, 'world.png')} alt="" loading="lazy" />
          <span class="text">
            <strong>{world.planet}</strong>
            <small>seed {world.seed} · {world.width}×{world.height}</small>
            <small>tilt {world.axial_tilt_deg.toFixed(1)}° · {Math.round(world.circumference_km)} km</small>
          </span>
        </button>
        <button class="delete" onclick={() => onDelete(world)} aria-label={`Delete ${world.id}`}>×</button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  li {
    display: flex;
    align-items: stretch;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }
  li.selected {
    border-color: var(--accent);
  }
  .pick {
    flex: 1;
    display: flex;
    gap: 10px;
    align-items: center;
    border: none;
    border-radius: 0;
    text-align: left;
    padding: 6px;
    min-width: 0;
  }
  img {
    width: 72px;
    height: 40px;
    object-fit: cover;
    border-radius: 4px;
    flex: none;
  }
  .text {
    display: grid;
    min-width: 0;
  }
  small {
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .delete {
    border: none;
    border-radius: 0;
    padding: 0 10px;
    color: var(--muted);
  }
  .delete:hover {
    color: var(--danger);
  }
  .empty {
    color: var(--muted);
  }
</style>
