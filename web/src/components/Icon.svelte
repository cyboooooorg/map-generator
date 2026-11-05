<script lang="ts">
  // An icon of lib/icons.ts, in the current colour; decorative (its button names it).
  import { ICONS, type IconName } from '../lib/icons';

  let { name, size = 18 }: { name: IconName; size?: number } = $props();
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 24 24" aria-hidden="true"
  ><path d={ICONS[name]} fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg
>

<style>
  .icon {
    display: block;
    flex: none;
  }
</style>
