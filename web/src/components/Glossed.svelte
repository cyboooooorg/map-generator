<script lang="ts">
  // Renders a display string with the name of each formula or unit in a
  // tooltip: native `title` on hover, a focus popover (see `.gloss` in
  // app.css) on tap. `inert` is for use inside a button or a link, where a
  // focusable element must not be nested.
  import { glossSegments } from '../lib/glossary';
  import { clippingBounds, popoverAlign } from '../lib/popoverAlign';

  let { text, inert = false }: { text: string | null | undefined; inert?: boolean } = $props();
  const segments = $derived(text ? glossSegments(text) : []);

  // The popover is at most 18em wide (see `.gloss` in app.css).
  function align(e: FocusEvent) {
    const el = e.currentTarget as HTMLElement;
    const width = Math.min(18 * parseFloat(getComputedStyle(el).fontSize), 0.8 * window.innerWidth);
    el.dataset.align = popoverAlign(el.getBoundingClientRect(), clippingBounds(el), width);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
{#each segments as seg}{#if 'kind' in seg}{#if inert}<abbr class="gloss" title={seg.name}>{seg.text}</abbr>{:else}<abbr class="gloss" title={seg.name} data-name={seg.name} tabindex="0" onfocus={align}>{seg.text}</abbr>{/if}{:else}{seg.text}{/if}{/each}
