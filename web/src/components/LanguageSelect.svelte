<script lang="ts">
  // The language of the interface. Hidden while a single language exists.
  import { LOCALE_NAMES, LOCALES, getLocale, setLocale, t, type Locale } from '../lib/i18n/index.svelte';
</script>

{#if LOCALES.length > 1}
  <label class="language">
    <span class="visually-hidden">{t.language.label}</span>
    <select
      value={getLocale()}
      onchange={(event) => setLocale(event.currentTarget.value as Locale)}
      title={t.language.label}
    >
      {#each LOCALES as locale (locale)}
        <option value={locale}>{LOCALE_NAMES[locale]}</option>
      {/each}
    </select>
  </label>
{/if}

<style>
  .language select {
    padding: 4px 6px;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
