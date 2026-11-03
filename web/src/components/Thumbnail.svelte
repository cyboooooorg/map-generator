<script lang="ts">
  // A world's thumbnail (thumb.png, 512 pixels wide) instead of its
  // full-size world.png: an 8192 × 4096 map would decode to 128 MB in the
  // browser. Loaded lazily, at a fixed size (no layout shift), with a
  // placeholder if it fails.
  import { worldThumbnail } from '../lib/api/client';

  interface Props {
    id: string;
    revision?: number | null;
    width: number;
    height: number;
    radius?: number;
  }

  let { id, revision = null, width, height, radius = 4 }: Props = $props();

  const src = $derived(worldThumbnail(id, revision));
  // The URL that failed to load: another world or revision tries again.
  let failed = $state<string | null>(null);
</script>

{#if failed === src}
  <span
    class="thumb placeholder"
    style:width={`${width}px`}
    style:height={`${height}px`}
    style:border-radius={`${radius}px`}
    aria-hidden="true"
  ></span>
{:else}
  <img
    class="thumb"
    {src}
    alt=""
    {width}
    {height}
    loading="lazy"
    decoding="async"
    style:border-radius={`${radius}px`}
    onerror={() => (failed = src)}
  />
{/if}

<style>
  .thumb {
    display: block;
    flex: none;
    object-fit: cover;
  }
  .placeholder {
    background: repeating-linear-gradient(-45deg, #1b2130 0 6px, #222a3b 6px 12px);
  }
</style>
