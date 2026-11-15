<script lang="ts">
  // Every input of the planet model a lone world takes, in folded groups. A
  // field is drawn (the value the seed drew is shown, greyed) or pinned (a
  // slider and a number box on its scale, with the Solar System's values to
  // click). Each says what it drives, and what the server refused of it.
  import { setPinned } from '../../lib/modelRequest';
  import { t } from '../../lib/i18n/index.svelte';
  import { detailText, errorField } from '../../lib/i18n/errors';
  import { formatUpTo } from '../../lib/format';
  import type { ArchetypeInfo } from '../../lib/api/types/ArchetypeInfo';
  import type { ErrorDetail } from '../../lib/api/types/ErrorDetail';
  import type { FormSpec } from '../../lib/api/types/FormSpec';
  import type { Quantity } from '../../lib/api/types/Quantity';
  import { limitsOf, marks, MODEL_GROUPS, type ModelFieldKey } from '../../lib/modelFields';
  import { type DrawnValues, type ModelValues, PIN_DEFAULTS, pinnedCount } from '../../lib/modelRequest';
  import { fromPosition, fromShown, SLIDER_STEPS, toPosition, toShown } from '../../lib/modelScale';

  interface Props {
    spec: FormSpec;
    archetypes: ArchetypeInfo[];
    values: ModelValues;
    /** What the seed drew, from the preview; null before the first answer. */
    drawn: DrawnValues | null;
    /** The values of the body a field sets (the preview's trees). */
    driven: (key: ModelFieldKey) => Quantity[];
    /** The request fields the planet has no use for. */
    ignored: string[];
    /** Whether a preview answered: the cause trees then say which fields the planet uses. */
    explained: boolean;
    /** The server's refusal of the form, shown at the field it is about. */
    refusal: ErrorDetail | null;
    /** The ids of the groups that are unfolded. */
    open: Set<string>;
  }

  let { spec, archetypes, values = $bindable(), drawn, driven, ignored, explained, refusal, open }: Props = $props();

  const refusedField = $derived(refusal ? errorField(refusal) : null);
  const numeric = (key: Exclude<ModelFieldKey, 'life'>) => values[key].value as number;

  /** Pins a field, starting from what the seed drew. */
  function pin(key: ModelFieldKey, on: boolean): void {
    const entry = values[key];
    entry.random = !on;
    if (on) entry.value = drawn?.[key] ?? PIN_DEFAULTS[key];
  }
  /** A field the planet has no use for: the server said so, or no cause tree of its body reads it. */
  const isUnused = (key: ModelFieldKey) => ignored.includes(key) || (explained && driven(key).length === 0);
  function setValue(key: Exclude<ModelFieldKey, 'life'>, value: number): void {
    setPinned(values, key, value);
  }
  function shownText(key: Exclude<ModelFieldKey, 'life'>, value: number): string {
    return `${formatUpTo(toShown(key, value), 3)} ${t.form.model[key].unit}`.trim();
  }
  const driveText = (key: ModelFieldKey) => {
    const roots = driven(key);
    return roots.length > 0 ? roots.map((q) => t.causes.quantities[q]).join(', ') : t.form.model[key].drives;
  };
  function toggleGroup(id: string, opened: boolean): void {
    if (opened) open.add(id);
    else open.delete(id);
  }
  const lifeChoices = [
    ['drawn', () => t.form.model.lifeDrawn],
    ['with', () => t.form.model.lifeWith],
    ['without', () => t.form.model.lifeWithout],
  ] as const;
</script>

<div class="model">
  {#each MODEL_GROUPS as group (group.id)}
    {@const count = pinnedCount(values, group.fields)}
    <details open={open.has(group.id)} ontoggle={(e) => toggleGroup(group.id, e.currentTarget.open)} class="group" data-group={group.id}>
      <summary>
        <span class="title">{t.form.model.groups[group.id].title}</span>
        <span class="count" class:some={count > 0}>{count > 0 ? t.form.model.groupSet({ count }) : t.form.model.groupDrawn}</span>
      </summary>
      <p class="help">{t.form.model.groups[group.id].help}</p>
      {#each group.fields as key (key)}
        {@const text = t.form.model[key]}
        {@const unused = isUnused(key)}
        {@const entry = values[key]}
        <div class="field" id={`model-${key}`} class:unused class:pinned={!entry.random}>
          <div class="head">
            <span class="name"><strong>{text.label}</strong> <span class="term">{text.term}</span></span>
            {#if key !== 'life'}
              <label class="pin" title={t.form.model.setHelp}>
                <input type="checkbox" checked={!entry.random} onchange={(e) => pin(key, e.currentTarget.checked)} aria-label={`${text.label}: ${t.form.model.set}`} />
                {t.form.model.set}
              </label>
            {/if}
          </div>
          {#if unused}
            <p class="note">{t.form.model.notUsed}</p>
          {:else}
            <p class="drives"><span aria-hidden="true">→</span> <span class="visually-hidden">{t.form.model.driveLabel}:</span> {driveText(key)}</p>
          {/if}

          {#if key === 'life'}
            <div class="life" role="radiogroup" aria-label={t.form.model.lifeLabel}>
              {#each lifeChoices as [choice, label] (choice)}
                {@const selected = choice === 'drawn' ? entry.random : !entry.random && entry.value === (choice === 'with')}
                <label class="choice" class:selected>
                  <input
                    type="radio"
                    name="model-life"
                    checked={selected}
                    onchange={() => (values.life = choice === 'drawn' ? { random: true, value: entry.value } : { random: false, value: choice === 'with' })}
                  />
                  {label()}
                </label>
              {/each}
            </div>
            {#if entry.random && typeof drawn?.life === 'boolean'}
              <p class="drawn">{t.form.model.lifeDrawnValue({ arose: drawn.life })}</p>
            {/if}
          {:else}
            {@const limits = limitsOf(key, spec)}
            {#if entry.random}
              <p class="drawn">
                {#if typeof drawn?.[key] === 'number'}
                  {t.form.model.drawnValue({ value: shownText(key, drawn[key] as number) })}
                {:else}
                  {t.form.model.drawn}: {t.form.model.drawnUnknown}
                {/if}
              </p>
            {:else}
              <div class="slider">
                <input
                  type="range"
                  min="0"
                  max={SLIDER_STEPS}
                  step="1"
                  value={toPosition(key, numeric(key), limits)}
                  oninput={(e) => setValue(key, fromPosition(key, Number(e.currentTarget.value), limits))}
                  aria-label={text.label}
                />
                <input
                  type="number"
                  min={toShown(key, limits.min)}
                  max={toShown(key, limits.max)}
                  step="any"
                  value={toShown(key, numeric(key))}
                  oninput={(e) => setValue(key, fromShown(key, e.currentTarget.valueAsNumber))}
                  aria-label={`${text.label} (${text.unit})`}
                />
                <span class="unit">{text.unit}</span>
              </div>
              {@const found = marks(key, archetypes, limits)}
              {#if found.length > 0}
                <div class="marks" aria-label={t.form.model.marksLabel}>
                  <div class="track" aria-hidden="true">
                    {#each found as mark (mark.body)}
                      <span class="tick {mark.body}" style:left={`${(toPosition(key, mark.value, limits) / SLIDER_STEPS) * 100}%`}></span>
                    {/each}
                  </div>
                  <div class="chips">
                    {#each found as mark (mark.body)}
                      <button type="button" class="chip" onclick={() => setValue(key, mark.value)} title={t.form.model.marksLabel}
                        >{t.form.model.marks[mark.body]} {formatUpTo(toShown(key, mark.value), 3)}</button
                      >
                    {/each}
                  </div>
                </div>
              {/if}
            {/if}
          {/if}
          <p class="unit-help">{text.unitHelp}</p>
          {#if refusal && refusedField === key}
            <p class="error" role="alert">{detailText(refusal)}</p>
          {/if}
        </div>
      {/each}
    </details>
  {/each}
</div>

<style>
  .model {
    display: grid;
    gap: 8px;
  }
  .group {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 10px 8px;
    min-width: 0;
  }
  summary {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 10px;
    cursor: pointer;
  }
  .title {
    font-weight: 600;
  }
  .count {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .count.some {
    color: var(--link);
  }
  .help,
  .unit-help,
  .note {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .field {
    display: grid;
    gap: 3px;
    padding: 8px 0 0;
    border-top: 1px solid var(--border);
    margin-top: 8px;
    min-width: 0;
  }
  .field.unused {
    opacity: 0.6;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: baseline;
    gap: 2px 10px;
  }
  .term {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .pin {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .field.pinned .pin {
    color: var(--text);
  }
  .drives {
    margin: 0;
    font-size: 0.9rem;
    color: var(--link);
  }
  .drawn {
    margin: 0;
    color: var(--muted);
    font-style: italic;
  }
  .slider {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(5em, 7em) auto;
    gap: 8px;
    align-items: center;
  }
  .slider > input {
    min-width: 0;
    width: 100%;
  }
  .unit {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .marks {
    display: grid;
    gap: 4px;
  }
  .track {
    position: relative;
    height: 6px;
    margin: 0 8px;
  }
  .tick {
    position: absolute;
    top: 0;
    width: 2px;
    height: 6px;
    background: var(--muted);
    transform: translateX(-1px);
  }
  .tick.earth {
    background: var(--link);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    padding: 0 8px;
    font-size: 0.8rem;
    border-radius: 10px;
    color: var(--muted);
  }
  .chip:hover {
    color: var(--link);
  }
  .life {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .choice {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    padding: 2px 10px;
    border: 1px solid var(--border);
    border-radius: 14px;
    cursor: pointer;
  }
  .choice.selected {
    border-color: var(--accent);
    outline: 1px solid var(--accent);
  }
  .error {
    margin: 0;
    color: var(--danger);
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
