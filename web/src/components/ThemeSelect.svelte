<script lang="ts">
  // The interface's theme (see lib/themes.ts), remembered in the browser.
  import { t } from '../lib/i18n/index.svelte';
  import { applyTheme, currentTheme, saveTheme, THEME_NAMES, type ThemeName } from '../lib/themes';

  let theme = $state<ThemeName>(currentTheme());
</script>

<label class="theme">
  <span class="visually-hidden">{t.theme.label}</span>
  <select
    value={theme}
    title={t.theme.label}
    onchange={(event) => {
      theme = event.currentTarget.value as ThemeName;
      applyTheme(theme);
      saveTheme(theme);
    }}
  >
    {#each THEME_NAMES as name (name)}
      <option value={name}>{t.theme.names[name]}</option>
    {/each}
  </select>
</label>

<style>
  .theme select {
    padding: 4px 6px;
    max-width: 14em;
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
