<script lang="ts">
  import { untrack } from 'svelte';
  import { ApiFailure, api } from '../lib/api/client';
  import type { Defaults } from '../lib/api/types/Defaults';
  import type { FormSpec } from '../lib/api/types/FormSpec';
  import type { JobCreated } from '../lib/api/types/JobCreated';
  import type { ParamDefault } from '../lib/api/types/ParamDefault';
  import type { PresetInfo } from '../lib/api/types/PresetInfo';
  import type { WorldRequest } from '../lib/api/types/WorldRequest';

  interface Props {
    presets: PresetInfo[];
    spec: FormSpec;
    /** Parameters to start from (varying an existing world), or null for the preset defaults. */
    initial?: WorldRequest | null;
    /** Id of the world being varied, for the title. */
    variedFrom?: string | null;
    /** Ids of the existing worlds: the server refuses to overwrite one. */
    existing: ReadonlySet<string>;
    /** Called once the server accepted the job; the form closes. */
    onSubmitted: (job: JobCreated, request: WorldRequest) => void;
    onCancel: () => void;
  }

  let { presets, spec, initial = null, variedFrom = null, existing, onSubmitted, onCancel }: Props = $props();

  type Field = 'sea_level' | 'volcanic' | 'circumference_km' | 'axial_tilt_deg' | 'erosion';

  interface FieldInfo {
    label: string;
    help: string;
    /** Key of the preset `[defaults]` table. */
    defaultsKey: keyof Defaults;
    /** Slider range; the number input accepts the full server limits. */
    slider: [number, number];
    step: number;
  }

  const FIELDS: Record<Field, FieldInfo> = {
    sea_level: {
      label: 'Sea level',
      help: 'Positive: more ocean',
      defaultsKey: 'sea_level',
      slider: [-1, 1],
      step: 0.01,
    },
    volcanic: {
      label: 'Volcanism',
      help: 'Share of mountain chains that erupt',
      defaultsKey: 'volcanic',
      slider: [0, 1],
      step: 0.01,
    },
    circumference_km: {
      label: 'Circumference (km)',
      help: 'Earth: 40 075 km',
      defaultsKey: 'circumference',
      slider: [5_000, 120_000],
      step: 100,
    },
    axial_tilt_deg: {
      label: 'Axial tilt (°)',
      help: 'Earth: 23.4°',
      defaultsKey: 'axial_tilt',
      slider: [0, 90],
      step: 0.1,
    },
    erosion: {
      label: 'Erosion',
      help: '0 disables it',
      defaultsKey: 'erosion',
      slider: [0, 5],
      step: 0.1,
    },
  };
  const FIELD_NAMES = Object.keys(FIELDS) as Field[];

  const SIZES: [number, number][] = [
    [960, 540],
    [1920, 1080],
    [2048, 1024],
    [4096, 2048],
    [8192, 4096],
  ];
  const MAX_SEED = 4_294_967_295;

  /** Each parameter: random (left to the preset or the server) or a fixed value. */
  type FieldState = Record<Field, { random: boolean; value: number }>;

  function initialState(preset: PresetInfo | undefined, from: WorldRequest | null): FieldState {
    const state = {} as FieldState;
    for (const field of FIELD_NAMES) {
      const given = from?.[field];
      if (given !== undefined) {
        state[field] = { random: false, value: given };
        continue;
      }
      const fromPreset: ParamDefault | null = preset?.defaults[FIELDS[field].defaultsKey] ?? null;
      const d = fromPreset ?? spec[field].fallback;
      state[field] = typeof d === 'number' ? { random: false, value: d } : { random: true, value: (d[0] + d[1]) / 2 };
    }
    return state;
  }

  /** The preset a request names: by spec (built-in name or file) or by preset name. */
  function findPreset(name: string | undefined): PresetInfo | undefined {
    return name === undefined ? undefined : presets.find((p) => p.spec === name || p.name === name);
  }

  // The form starts from the props as they are when it opens; later prop
  // changes must not reset what the user typed, hence untrack.
  const start = untrack(() => ({ initial, preset: findPreset(initial?.planet) ?? presets[0] }));
  const missingPreset = start.initial?.planet !== undefined && !findPreset(start.initial.planet);
  let planet = $state(start.preset?.spec ?? '');
  let randomPlanet = $state(false);
  let seed = $state(start.initial?.seed !== undefined ? String(start.initial.seed) : '');
  let size = $state(
    untrack(() =>
      start.initial?.width && start.initial.height
        ? `${start.initial.width}x${start.initial.height}`
        : `${spec.default_width}x${spec.default_height}`,
    ),
  );
  let fields = $state<FieldState>(untrack(() => initialState(start.preset, start.initial)));
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let replace = $state(false);

  const selectedPreset = $derived(presets.find((p) => p.spec === planet));
  const sizes = $derived.by(() => {
    const list = [...SIZES];
    for (const extra of [size, `${spec.default_width}x${spec.default_height}`]) {
      const [w, h] = extra.split('x').map(Number);
      if (w && h && !list.some(([lw, lh]) => lw === w && lh === h)) {
        list.unshift([w, h]);
      }
    }
    return list;
  });

  function choosePreset(spec: string): void {
    planet = spec;
    fields = initialState(
      presets.find((p) => p.spec === spec),
      null,
    );
  }

  function rollSeed(): void {
    const values = new Uint32Array(1);
    crypto.getRandomValues(values);
    seed = String(values[0] ?? 0);
  }

  function buildRequest(): WorldRequest {
    const [width, height] = size.split('x').map(Number);
    const request: WorldRequest = {};
    if (!randomPlanet) {
      request.planet = planet;
    }
    if (seed.trim() !== '') {
      request.seed = Number(seed);
    }
    if (width && height) {
      request.width = width;
      request.height = height;
    }
    for (const field of FIELD_NAMES) {
      if (!fields[field].random) {
        request[field] = fields[field].value;
      }
    }
    return request;
  }

  /** Id the world will get (planet-seed), when both are fixed. */
  const targetId = $derived(
    !randomPlanet && selectedPreset && /^\d+$/.test(seed.trim()) ? `${selectedPreset.name}-${Number(seed)}` : null,
  );
  const conflict = $derived(targetId !== null && existing.has(targetId));

  const seedValid = $derived(seed.trim() === '' || (/^\d+$/.test(seed.trim()) && Number(seed) <= MAX_SEED));

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    error = null;
    submitting = true;
    try {
      const request = buildRequest();
      if (conflict && replace && targetId) {
        await api.deleteWorld(targetId);
      }
      const job = await api.createWorld(request);
      onSubmitted(job, request);
    } catch (e) {
      error = e instanceof ApiFailure ? e.message : String(e);
    } finally {
      submitting = false;
    }
  }
</script>

<form onsubmit={submit}>
  <h2>{variedFrom ? `Vary ${variedFrom}` : 'New world'}</h2>
  {#if variedFrom}
    <p class="help">Starts from every parameter of {variedFrom}: change the ones you want.</p>
  {/if}
  {#if missingPreset}
    <p class="error">The preset {start.initial?.planet} is not offered by this server: pick another one.</p>
  {/if}

  <label class="row">
    <span>Planet</span>
    <select value={planet} onchange={(e) => choosePreset(e.currentTarget.value)} disabled={randomPlanet}>
      {#each presets as preset (preset.spec)}
        <option value={preset.spec}>{preset.name}{preset.built_in ? '' : ` (${preset.spec})`}</option>
      {/each}
    </select>
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={randomPlanet} />
    Random built-in planet
  </label>
  {#if selectedPreset && !randomPlanet}
    <p class="help">{selectedPreset.description}</p>
  {/if}

  {#each FIELD_NAMES as field (field)}
    {@const info = FIELDS[field]}
    {@const limits = spec[field].limits}
    <fieldset>
      <legend>
        {info.label}
        <label class="check inline">
          <input type="checkbox" bind:checked={fields[field].random} />
          random
        </label>
      </legend>
      {#if !fields[field].random}
        <div class="slider">
          <input
            type="range"
            min={info.slider[0]}
            max={info.slider[1]}
            step={info.step}
            bind:value={fields[field].value}
          />
          <!-- step="any": the slider step counts from the server minimum (1 km for
               the circumference), so a stepped input would reject round values
               such as 50000. -->
          <input type="number" min={limits.min} max={limits.max} step="any" bind:value={fields[field].value} />
        </div>
      {/if}
      <p class="help">{info.help}</p>
    </fieldset>
  {/each}

  <div class="row">
    <label for="seed">Seed</label>
    <div class="seed">
      <input id="seed" type="text" inputmode="numeric" placeholder="random" bind:value={seed} />
      <button type="button" onclick={rollSeed} title="Draw a random seed">Roll</button>
      <button type="button" onclick={() => (seed = '')} disabled={seed === ''} title="Let the server draw it"
        >Clear</button
      >
    </div>
  </div>
  {#if !seedValid}
    <p class="error">The seed is a whole number between 0 and 4 294 967 295.</p>
  {:else if conflict}
    <div class="warning">
      <p>
        {targetId} exists already: a world is named after its planet and seed. Roll a new seed, or replace it.
      </p>
      <label class="check">
        <input type="checkbox" bind:checked={replace} />
        Replace {targetId} (deletes its files)
      </label>
    </div>
  {/if}

  <label class="row">
    <span>Size</span>
    <select bind:value={size}>
      {#each sizes as [w, h] (`${w}x${h}`)}
        <option value={`${w}x${h}`} disabled={w > spec.max_size || h > spec.max_size || w * h > spec.max_pixels}
          >{w} × {h}{w * h > spec.max_pixels ? ' (too large for this server)' : ''}</option
        >
      {/each}
    </select>
  </label>

  <!-- Sticks to the bottom of the screen on narrow layouts, so the buttons
       stay in view while the parameters scroll. -->
  <div class="footer">
    {#if error}
      <p class="error">{error}</p>
    {/if}
    <p class="help">The generation runs in the background: keep exploring meanwhile, or queue more worlds.</p>
    <div class="actions">
      <button type="button" onclick={onCancel}>Close</button>
      <button type="submit" class="primary" disabled={submitting || !seedValid || (conflict && !replace)}>
        {submitting ? 'Sending…' : 'Generate'}
      </button>
    </div>
  </div>
</form>

<style>
  /* Every track uses minmax(0, …) and every box min-width: 0 so that the
     intrinsic width of a control (a <select> is as wide as its longest
     option in Firefox and Safari) never widens the form past the sidebar. */
  form {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 10px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 1.1rem;
    overflow-wrap: anywhere;
  }
  .row {
    display: grid;
    grid-template-columns: 90px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
  }
  .row > select,
  .slider > input {
    width: 100%;
    min-width: 0;
  }
  .seed {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 6px;
  }
  .seed input {
    min-width: 0;
  }
  .seed button {
    padding: 6px 8px;
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--muted);
  }
  .check.inline {
    display: inline-flex;
    margin-left: 8px;
    font-weight: normal;
  }
  fieldset {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 10px 8px;
    margin: 0;
    /* Fieldsets default to min-inline-size: min-content. */
    min-inline-size: 0;
  }
  legend {
    padding: 0 4px;
    font-weight: 600;
  }
  .slider {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 90px;
    gap: 8px;
    align-items: center;
  }
  .help {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .error {
    color: var(--danger);
    margin: 0;
  }
  .warning {
    border: 1px solid var(--danger);
    border-radius: 8px;
    padding: 6px 10px;
    display: grid;
    gap: 6px;
  }
  .warning p {
    margin: 0;
    font-size: 0.9rem;
  }
  .footer {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 10px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  @media (max-width: 760px) {
    .footer {
      position: sticky;
      bottom: 0;
      /* Bleed over the sidebar padding so the parameters do not show through. */
      margin: 0 -14px;
      padding: 10px 14px 14px;
      background: var(--bg);
      border-top: 1px solid var(--border);
    }
  }
</style>
