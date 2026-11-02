<script lang="ts">
  import { untrack } from 'svelte';
  import { ApiFailure, api, followJob } from '../lib/api/client';
  import type { Defaults } from '../lib/api/types/Defaults';
  import type { FormSpec } from '../lib/api/types/FormSpec';
  import type { JobStatus } from '../lib/api/types/JobStatus';
  import type { ParamDefault } from '../lib/api/types/ParamDefault';
  import type { PresetInfo } from '../lib/api/types/PresetInfo';
  import type { Stage } from '../lib/api/types/Stage';
  import type { WorldRequest } from '../lib/api/types/WorldRequest';

  interface Props {
    presets: PresetInfo[];
    spec: FormSpec;
    /** Called once the world is generated. */
    onDone: (worldId: string) => void;
    onCancel: () => void;
  }

  let { presets, spec, onDone, onCancel }: Props = $props();

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

  const STAGES: Record<Stage, string> = {
    noise: 'Shaping continents',
    erosion: 'Eroding the terrain',
    temperature: 'Computing temperatures',
    coast: 'Measuring coastlines',
    moisture: 'Blowing winds and rain',
    rivers: 'Routing rivers and lakes',
    biomes: 'Choosing biomes',
    export: 'Writing the maps',
  };

  /** Each parameter: random (left to the preset or the server) or a fixed value. */
  type FieldState = Record<Field, { random: boolean; value: number }>;

  function initialState(preset: PresetInfo | undefined): FieldState {
    const state = {} as FieldState;
    for (const field of FIELD_NAMES) {
      const fromPreset: ParamDefault | null = preset?.defaults[FIELDS[field].defaultsKey] ?? null;
      const d = fromPreset ?? spec[field].fallback;
      state[field] = typeof d === 'number' ? { random: false, value: d } : { random: true, value: (d[0] + d[1]) / 2 };
    }
    return state;
  }

  // The form starts from the props as they are when it opens; later prop
  // changes must not reset what the user typed, hence untrack.
  let planet = $state(untrack(() => presets[0]?.spec ?? ''));
  let randomPlanet = $state(false);
  let seed = $state('');
  let size = $state(untrack(() => `${spec.default_width}x${spec.default_height}`));
  let fields = $state<FieldState>(untrack(() => initialState(presets[0])));
  let status = $state<JobStatus | null>(null);
  let error = $state<string | null>(null);
  let running = $derived(status?.status === 'queued' || status?.status === 'running');

  const selectedPreset = $derived(presets.find((p) => p.spec === planet));
  const sizes = $derived(
    SIZES.some(([w, h]) => w === spec.default_width && h === spec.default_height)
      ? SIZES
      : [[spec.default_width, spec.default_height] as [number, number], ...SIZES],
  );

  function choosePreset(spec: string): void {
    planet = spec;
    fields = initialState(presets.find((p) => p.spec === spec));
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

  const seedValid = $derived(seed.trim() === '' || (/^\d+$/.test(seed.trim()) && Number(seed) <= 4_294_967_295));

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    error = null;
    status = { status: 'queued' };
    try {
      const job = await api.createWorld(buildRequest());
      const final = await followJob(job.job_id, (s) => (status = s)).done;
      if (final.status === 'done') {
        status = null;
        onDone(final.world_id);
      } else if (final.status === 'failed') {
        error = final.error;
        status = null;
      }
    } catch (e) {
      error = e instanceof ApiFailure ? e.message : String(e);
      status = null;
    }
  }
</script>

<form onsubmit={submit}>
  <h2>New world</h2>

  <label class="row">
    <span>Planet</span>
    <select value={planet} onchange={(e) => choosePreset(e.currentTarget.value)} disabled={randomPlanet || running}>
      {#each presets as preset (preset.spec)}
        <option value={preset.spec}>{preset.name}{preset.built_in ? '' : ` (${preset.spec})`}</option>
      {/each}
    </select>
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={randomPlanet} disabled={running} />
    Random built-in planet
  </label>
  {#if selectedPreset && !randomPlanet}
    <p class="help">{selectedPreset.description}</p>
  {/if}

  {#each FIELD_NAMES as field (field)}
    {@const info = FIELDS[field]}
    {@const limits = spec[field].limits}
    <fieldset disabled={running}>
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
          <input type="number" min={limits.min} max={limits.max} step={info.step} bind:value={fields[field].value} />
        </div>
      {/if}
      <p class="help">{info.help}</p>
    </fieldset>
  {/each}

  <label class="row">
    <span>Seed</span>
    <input type="text" inputmode="numeric" placeholder="random" bind:value={seed} disabled={running} />
  </label>
  {#if !seedValid}
    <p class="error">The seed is a whole number between 0 and 4 294 967 295.</p>
  {/if}

  <label class="row">
    <span>Size</span>
    <select bind:value={size} disabled={running}>
      {#each sizes as [w, h] (`${w}x${h}`)}
        <option value={`${w}x${h}`} disabled={w > spec.max_size || h > spec.max_size || w * h > spec.max_pixels}
          >{w} × {h}{w * h > spec.max_pixels ? ' (too large for this server)' : ''}</option
        >
      {/each}
    </select>
  </label>

  <!-- Sticks to the bottom of the screen on narrow layouts, so the progress
       and the buttons stay in view while the parameters scroll. -->
  <div class="footer">
    {#if status?.status === 'queued'}
      <p class="progress">Waiting for the generator…</p>
    {:else if status?.status === 'running'}
      <div class="progress">
        <progress max={status.steps} value={status.step - 1}></progress>
        <span>{STAGES[status.stage]} ({status.step}/{status.steps})</span>
      </div>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}

    <div class="actions">
      <button type="button" onclick={onCancel} disabled={running}>Close</button>
      <button type="submit" class="primary" disabled={running || !seedValid}>Generate</button>
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
  }
  .row {
    display: grid;
    grid-template-columns: 90px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
  }
  .row > select,
  .row > input,
  .slider > input {
    width: 100%;
    min-width: 0;
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
  .progress {
    display: grid;
    gap: 4px;
    color: var(--muted);
  }
  progress {
    width: 100%;
  }
  .error {
    color: var(--danger);
    margin: 0;
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
