<script lang="ts">
  // Renders a display string with the name of each formula or unit in a
  // tooltip: native `title` on hover, a focus popover (see `.gloss` in
  // app.css) on tap. `inert` is for use inside a button or a link, where a
  // focusable element must not be nested.
  import { glossSegments } from '../lib/glossary';

  let { text, inert = false }: { text: string | null | undefined; inert?: boolean } = $props();
  const segments = $derived(text ? glossSegments(text) : []);
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
{#each segments as seg}{#if 'kind' in seg}{#if inert}<abbr class="gloss" title={seg.name}>{seg.text}</abbr>{:else}<abbr class="gloss" title={seg.name} data-name={seg.name} tabindex="0">{seg.text}</abbr>{/if}{:else}{seg.text}{/if}{/each}
