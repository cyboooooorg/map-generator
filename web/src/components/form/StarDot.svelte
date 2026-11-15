<script lang="ts">
  // A small disc in the conventional colour of a star's class, beside the menu that picks
  // it. Without a colour (the star is left to the draw and none is drawn yet)
  // the disc is an empty ring.
  interface Props {
    /** CSS colour of the star's spectral class, null when there is none to show. */
    colour: string | null;
    /** The star's type, for the screen reader. */
    label: string;
  }

  let { colour, label }: Props = $props();
</script>

<span class="dot" class:empty={colour === null} style:background={colour ?? undefined} role="img" aria-label={label}></span>

<style>
  .dot {
    display: inline-block;
    flex: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid var(--border);
  }
  .dot.empty {
    background: transparent;
    border-style: dashed;
  }
</style>
