<script lang="ts">
  // A text holding mass units (`1 M⊕ (5,97×10²¹ t)`), each unit abbreviation
  // wrapped in an <abbr> that says what it stands for.
  import { t } from '../lib/i18n/index.svelte';

  interface Props {
    text: string;
  }

  let { text }: Props = $props();
  const parts = $derived(text.split(/(M⊕|M♃)/));
  const titles: Record<string, () => string> = {
    'M⊕': () => t.form.model.massUnits.earth,
    'M♃': () => t.form.model.massUnits.jupiter,
  };
</script>

{#each parts as part, i (i)}{#if titles[part]}<abbr title={titles[part]()}>{part}</abbr>{:else}{part}{/if}{/each}
