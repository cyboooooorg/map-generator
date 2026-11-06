<script lang="ts">
  // How much the 3D views draw (see lib/viewer/quality.ts): auto (light
  // when WebGL runs on the CPU), high or light, remembered in the browser.
  // The views take their quality when they are made, so a change reloads
  // the page, the address keeping the system and body shown.
  import { t } from '../lib/i18n/index.svelte';
  import { detectRenderer, loadQualityChoice, resolveQuality, saveQualityChoice, type QualityChoice } from '../lib/viewer/quality';

  const choice = loadQualityChoice();
  const renderer = detectRenderer();
  const detected = resolveQuality('auto', renderer.software);
  const help = $derived(t.quality.help({ renderer: renderer.name || t.quality.unknownRenderer, detected: t.quality.names[detected] }));
</script>

<label class="quality">
  <span class="visually-hidden">{t.quality.label}</span>
  <select
    value={choice}
    title={`${t.quality.label}. ${help}`}
    onchange={(event) => {
      saveQualityChoice(event.currentTarget.value as QualityChoice);
      location.reload();
    }}
  >
    <option value="auto">{t.quality.auto({ detected: t.quality.names[detected] })}</option>
    <option value="high">{t.quality.names.high}</option>
    <option value="light">{t.quality.names.light}</option>
  </select>
</label>

<style>
  .quality select {
    padding: 4px 6px;
    max-width: 9em;
  }
  /* Phones have a GPU (auto picks high quality there): the bar keeps one row. */
  @media (max-width: 480px) {
    .quality {
      display: none;
    }
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
