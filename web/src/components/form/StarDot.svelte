<script lang="ts">
  // A small disc in the colour of a star's light, beside the menu that picks
  // it. Without a colour (the star is left to the draw and none is drawn yet)
  // the disc is an empty ring.
  import { cssColour } from '../../lib/orrery';

  interface Props {
    /** Linear RGB of the star's light, null when there is none to show. */
    light: [number, number, number] | null;
    /** The star's type, for the screen reader. */
    label: string;
  }

  let { light, label }: Props = $props();
</script>

<span class="dot" class:empty={light === null} style:background={light ? cssColour(light) : undefined} role="img" aria-label={label}></span>

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
