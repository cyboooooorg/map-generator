<script lang="ts">
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { presetDescription, presetName } from '../lib/i18n/names.svelte';
  import { untrack } from 'svelte';
  import { api } from '../lib/api/client';
  import type { Defaults } from '../lib/api/types/Defaults';
  import type { FormSpec } from '../lib/api/types/FormSpec';
  import type { JobCreated } from '../lib/api/types/JobCreated';
  import type { ParamDefault } from '../lib/api/types/ParamDefault';
  import type { PresetInfo } from '../lib/api/types/PresetInfo';
  import type { WorldRequest } from '../lib/api/types/WorldRequest';
  import { formatInt, formatUpTo, MAX_SEED, randomSeed } from '../lib/format';
  import { formatBytes, formatEstimate, serverPeakBytes, viewerBytes } from '../lib/estimate';
  import { clearFormMemory, loadFormMemory, saveFormMemory } from '../lib/formMemory';
  import { jobs } from '../lib/jobs.svelte';
  import { type Unit, unitTooltip } from '../lib/units';
  import { PRESSURE_STEPS, pressureToSlider, sliderToPressure } from '../lib/pressure';
  import { applyStarChoice, initialStarChoice, STAR_CLASSES, type StarChoice } from '../lib/starForm';
  import { type MoonChoice, moonChoiceOf, moonsRequest } from '../lib/moons';
  import { applyGiantChoice, type GiantChoice, initialGiantChoice } from '../lib/giantForm';

  interface Props {
    presets: PresetInfo[];
    spec: FormSpec;
    /**
     * Parameters to start from (varying an existing world), or null for the
     * last form submitted in this browser (the preset defaults without one).
     */
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

  type Field =
    | 'sea_level'
    | 'volcanic'
    | 'circumference_km'
    | 'axial_tilt_deg'
    | 'erosion'
    | 'pressure_bar'
    | 'surface_age_ga';

  interface FieldInfo {
    /** Key of the preset `[defaults]` table. */
    defaultsKey: keyof Defaults;
    /** Slider range; the number input accepts the full server limits. */
    slider: [number, number];
    step: number;
    /** Less common units of the field, explained in a tooltip. */
    units?: Unit[];
  }

  const FIELDS: Record<Field, FieldInfo> = {
    sea_level: {
      defaultsKey: 'sea_level',
      slider: [-1, 1],
      step: 0.01,
    },
    volcanic: {
      defaultsKey: 'volcanic',
      slider: [0, 1],
      step: 0.01,
    },
    circumference_km: {
      defaultsKey: 'circumference',
      slider: [5_000, 120_000],
      step: 100,
    },
    axial_tilt_deg: {
      defaultsKey: 'axial_tilt',
      slider: [0, 90],
      step: 0.1,
    },
    erosion: {
      defaultsKey: 'erosion',
      slider: [0, 5],
      step: 0.1,
    },
    pressure_bar: {
      units: ['bar'],
      defaultsKey: 'pressure',
      slider: [0, 100],
      step: 0.001,
    },
    surface_age_ga: {
      units: ['Ga'],
      defaultsKey: 'surface_age',
      slider: [0, 4.6],
      step: 0.01,
    },
  };
  const FIELD_NAMES = Object.keys(FIELDS) as Field[];
  /** Label and help of a field other than the sea level, in the active locale. */
  const fieldText = (field: Exclude<Field, 'sea_level'>) => t.form.fields[field];
  /** Every field but the sea level, which has its own fieldset. */
  const OTHER_FIELDS = FIELD_NAMES.filter((field) => field !== 'sea_level');

  /** The sea is set either by its level or by the share of land it leaves. */
  type SeaBy = 'level' | 'land';

  const SIZES: [number, number][] = [
    [960, 540],
    [1920, 1080],
    [2048, 1024],
    [4096, 2048],
    [8192, 4096],
  ];

  /** Each parameter: random (left to the preset or the server) or a fixed value. */
  type FieldState = Record<Field, { random: boolean; value: number }>;

  function initialState(preset: PresetInfo | undefined, from: WorldRequest | null): FieldState {
    const state = {} as FieldState;
    const land = landDefault(preset, from);
    for (const field of FIELD_NAMES) {
      if (field === 'sea_level' && land !== null) {
        // Set by the land share: only its random flag matters here.
        state[field] = { random: land.random, value: 0 };
        continue;
      }
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

  /**
   * The land share a request or preset sets instead of the sea level, in
   * percent, or null when it sets the sea level (or neither).
   */
  function landDefault(
    preset: PresetInfo | undefined,
    from: WorldRequest | null,
  ): { random: boolean; percent: number } | null {
    if (from?.land_fraction !== undefined) {
      return { random: false, percent: toPercent(from.land_fraction) };
    }
    if (from?.sea_level !== undefined) {
      return null;
    }
    const d = preset?.defaults.land_fraction ?? null;
    if (d === null) {
      return null;
    }
    return typeof d === 'number'
      ? { random: false, percent: toPercent(d) }
      : { random: true, percent: toPercent((d[0] + d[1]) / 2) };
  }

  /** A fraction in percent, rounded to 0.01 so 0.29 shows as 29, not 28.999999999999996. */
  function toPercent(fraction: number): number {
    return Math.round(fraction * 10_000) / 100;
  }

  /** The preset a request names: by spec (built-in name or file) or by preset name. */
  function findPreset(name: string | undefined): PresetInfo | undefined {
    return name === undefined ? undefined : presets.find((p) => p.spec === name || p.name === name);
  }

  /** Whether a map size is one this server accepts. */
  function sizeFits(value: string): boolean {
    const [w, h] = value.split('x').map(Number);
    return !!w && !!h && w <= spec.max_size && h <= spec.max_size && w * h <= spec.max_pixels;
  }

  // The form starts from the props as they are when it opens; later prop
  // changes must not reset what the user typed, hence untrack.
  const start = untrack(() => ({ initial, preset: findPreset(initial?.planet) ?? presets[0] }));
  const missingPreset = start.initial?.planet !== undefined && !findPreset(start.initial.planet);
  /** A blank form starts from the last one submitted, if its preset is still offered. */
  const memory = untrack(() => {
    const stored = initial === null ? loadFormMemory(FIELD_NAMES) : null;
    return stored && presets.some((p) => p.spec === stored.planet) ? stored : null;
  });
  const memoryPreset = untrack(() => (memory ? presets.find((p) => p.spec === memory.planet) : undefined));
  let planet = $state(memoryPreset?.spec ?? start.preset?.spec ?? '');
  let randomPlanet = $state(memory?.randomPlanet ?? false);
  let seed = $state(start.initial?.seed !== undefined ? String(start.initial.seed) : '');
  let size = $state(
    untrack(() =>
      start.initial?.width && start.initial.height
        ? `${start.initial.width}x${start.initial.height}`
        : memory && sizeFits(memory.size)
          ? memory.size
          : `${spec.default_width}x${spec.default_height}`,
    ),
  );
  let fields = $state<FieldState>(
    untrack(() =>
      memory
        ? { ...initialState(memoryPreset, null), ...(memory.fields as Partial<FieldState>) }
        : initialState(start.preset, start.initial),
    ),
  );
  const startLand = untrack(() => (memory ? null : landDefault(start.preset, start.initial)));
  let seaBy = $state<SeaBy>(memory?.seaBy ?? (startLand ? 'land' : 'level'));
  /** Land share in percent, used when `seaBy` is 'land'. */
  let landPercent = $state(memory?.landPercent ?? startLand?.percent ?? 29);
  /** The moons asked for: drawn by the seed, none, or a number. */
  let moons = $state<MoonChoice>(untrack(() => memory?.moons ?? moonChoiceOf(start.initial?.moons)));
  /** Metallicity, giant neighbours, a giant's moon, a giant's mass (see giantForm.ts). */
  let giantChoice = $state<GiantChoice>(untrack(() => initialGiantChoice(start.initial)));
  /** The star and the orbit (see starForm.ts). */
  let star = $state<StarChoice>(
    untrack(() => memory?.star ?? initialStarChoice(start.preset, start.initial, spec.hz_position.fallback)),
  );
  /** Subtypes of a class, by half steps: O starts at O3, the sequence ends at M9.5. */
  function subtypes(klass: string): number[] {
    const first = klass === 'O' ? 6 : 0;
    return Array.from({ length: 20 - first }, (_, i) => (first + i) / 2);
  }
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let replace = $state(false);

  const selectedPreset = $derived(presets.find((p) => p.spec === planet));
  /** A giant planet: no ground, so no sea, volcanoes, erosion, air pressure, craters or moons to set. */
  const giantWorld = $derived(!randomPlanet && selectedPreset?.giant !== undefined);
  /** The fields a world of the chosen kind has. */
  const shownFields = $derived(giantWorld ? OTHER_FIELDS.filter((f) => f === 'axial_tilt_deg') : OTHER_FIELDS);
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

  function choosePreset(presetSpec: string): void {
    planet = presetSpec;
    const preset = presets.find((p) => p.spec === presetSpec);
    fields = initialState(preset, null);
    star = initialStarChoice(preset, null, spec.hz_position.fallback);
    const land = landDefault(preset, null);
    seaBy = land ? 'land' : 'level';
    landPercent = land?.percent ?? 29;
  }

  /**
   * Back to the defaults: the first preset's (or the varied world's)
   * parameters, the server's map size and a random seed, and the
   * remembered form forgotten.
   */
  function resetForm(): void {
    clearFormMemory();
    const preset = start.preset;
    randomPlanet = false;
    choosePreset(preset?.spec ?? '');
    if (start.initial) {
      fields = initialState(preset, start.initial);
      star = initialStarChoice(preset, start.initial, spec.hz_position.fallback);
      const land = landDefault(preset, start.initial);
      seaBy = land ? 'land' : 'level';
      landPercent = land?.percent ?? 29;
    }
    seed = start.initial?.seed !== undefined ? String(start.initial.seed) : '';
    size =
      start.initial?.width && start.initial.height
        ? `${start.initial.width}x${start.initial.height}`
        : `${spec.default_width}x${spec.default_height}`;
    replace = false;
    error = null;
  }

  function rollSeed(): void {
    seed = String(randomSeed());
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
    for (const field of shownFields) {
      if (!fields[field].random) {
        request[field] = fields[field].value;
      }
    }
    applyStarChoice(request, star);
    const asked = moonsRequest(moons);
    if (asked !== undefined && !giantWorld) {
      request.moons = asked;
    }
    applyGiantChoice(request, giantChoice, giantWorld);
    if (!fields.sea_level.random && !giantWorld) {
      if (seaBy === 'level') {
        request.sea_level = fields.sea_level.value;
      } else {
        request.land_fraction = landPercent / 100;
      }
    }
    return request;
  }

  /** What the chosen size costs: time (once learnt in this browser) and memory. */
  const cost = $derived.by(() => {
    const [w, h] = size.split('x').map(Number);
    const pixels = (w ?? 0) * (h ?? 0);
    const seconds = jobs.estimate(pixels / 1e6);
    return {
      time: seconds === null ? null : formatEstimate(seconds),
      server: formatBytes(serverPeakBytes(pixels)),
      viewer: formatBytes(viewerBytes(pixels)),
    };
  });

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
      // The server swaps the new world in once it is complete: a failed or
      // cancelled generation keeps the old one.
      const job = await api.createWorld(request, { replace: conflict && replace });
      saveFormMemory({ planet, randomPlanet, size, seaBy, landPercent, fields, star, moons });
      onSubmitted(job, request);
    } catch (e) {
      error = errorText(e);
    } finally {
      submitting = false;
    }
  }
</script>

<form onsubmit={submit}>
  <h2>{variedFrom ? t.form.vary({ id: variedFrom }) : t.form.newWorld}</h2>
  {#if variedFrom}
    <p class="help">{t.form.variedHelp({ id: variedFrom })}</p>
  {:else if memory}
    <p class="help">{t.form.memoryHelpBefore}<strong>{t.form.memoryHelpDefaults}</strong>{t.form.memoryHelpAfter}</p>
  {/if}
  {#if missingPreset}
    <p class="error">{t.form.missingPreset({ preset: start.initial?.planet ?? '' })}</p>
  {/if}

  <label class="row">
    <span>{t.form.planet}</span>
    <select value={planet} onchange={(e) => choosePreset(e.currentTarget.value)} disabled={randomPlanet}>
      {#each presets as preset (preset.spec)}
        <option value={preset.spec}>{presetName(preset)}{preset.built_in ? '' : ` (${preset.spec})`}</option>
      {/each}
    </select>
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={randomPlanet} />
    {t.form.randomPlanet}
  </label>
  {#if selectedPreset && !randomPlanet}
    <p class="help">{presetDescription(selectedPreset)}</p>
  {/if}

  {#if !giantWorld}
  <fieldset>
    <legend>
      <select class="mode" bind:value={seaBy} aria-label={t.form.seaBy}>
        <option value="level">{t.form.seaLevel}</option>
        <option value="land">{t.form.landShare}</option>
      </select>
      <label class="check inline">
        <input type="checkbox" bind:checked={fields.sea_level.random} />
        {t.form.random}
      </label>
    </legend>
    {#if !fields.sea_level.random}
      <div class="slider">
        {#if seaBy === 'level'}
          <input type="range" min={-1} max={1} step={0.01} bind:value={fields.sea_level.value} />
          <input
            type="number"
            min={spec.sea_level.limits.min}
            max={spec.sea_level.limits.max}
            step="any"
            bind:value={fields.sea_level.value}
          />
        {:else}
          <input type="range" min={0} max={100} step={1} bind:value={landPercent} />
          <input
            type="number"
            min={spec.land_fraction.min * 100}
            max={spec.land_fraction.max * 100}
            step="any"
            bind:value={landPercent}
          />
        {/if}
      </div>
    {/if}
    <p class="help">
      {seaBy === 'level' ? t.form.seaLevelHelp : t.form.landShareHelp}
    </p>
  </fieldset>
  {/if}

  {#each shownFields as field (field)}
    {@const info = FIELDS[field]}
    {@const limits = spec[field].limits}
    <fieldset>
      <legend>
        {#if info.units}
          <span class="unit-help" title={unitTooltip(...info.units)}>{fieldText(field).label}</span>
        {:else}
          {fieldText(field).label}
        {/if}
        <label class="check inline">
          <input type="checkbox" bind:checked={fields[field].random} />
          {t.form.random}
        </label>
      </legend>
      {#if !fields[field].random}
        <div class="slider">
          {#if field === 'pressure_bar'}
            <!-- Logarithmic: position 0 is airless, then 0.001 to 1000 bar. -->
            <input
              type="range"
              min={0}
              max={PRESSURE_STEPS}
              step={1}
              value={pressureToSlider(fields[field].value)}
              oninput={(e) => (fields.pressure_bar.value = sliderToPressure(Number(e.currentTarget.value)))}
              aria-label={t.form.pressureSlider}
            />
          {:else}
            <input
              type="range"
              min={info.slider[0]}
              max={info.slider[1]}
              step={info.step}
              bind:value={fields[field].value}
            />
          {/if}
          <!-- step="any": the slider step counts from the server minimum (1 km for
               the circumference), so a stepped input would reject round values
               such as 50000. -->
          <input type="number" min={limits.min} max={limits.max} step="any" bind:value={fields[field].value} />
        </div>
      {/if}
      <p class="help">{fieldText(field).help}</p>
    </fieldset>
  {/each}

  <fieldset>
    <legend>{t.form.star}</legend>
    <div class="star">
      <select
        bind:value={star.klass}
        onchange={() => (star.subtype = Math.max(star.subtype, star.klass === 'O' ? 3 : 0))}
        aria-label={t.form.starClass}
      >
        <option value="">{t.form.starAuto}</option>
        {#each STAR_CLASSES as klass (klass)}
          <option value={klass}>{klass}</option>
        {/each}
      </select>
      <select bind:value={star.subtype} disabled={star.klass === ''} aria-label={t.form.starSubtype}>
        {#each subtypes(star.klass) as subtype (subtype)}
          <option value={subtype}>{formatUpTo(subtype, 1)}</option>
        {/each}
      </select>
    </div>
    <p class="help">{star.klass === '' ? t.form.starAutoHelp : t.form.starClasses[star.klass]}</p>
  </fieldset>
  <fieldset>
    <legend>
      <select class="mode" bind:value={star.orbitBy} aria-label={t.form.orbitBy}>
        <option value="position">{t.form.orbitPosition}</option>
        <option value="au">{t.form.orbitAu}</option>
      </select>
      <label class="check inline">
        <input type="checkbox" bind:checked={star.orbitRandom} />
        {t.form.random}
      </label>
    </legend>
    {#if !star.orbitRandom}
      <div class="slider">
        {#if star.orbitBy === 'position'}
          <input type="range" min={-1} max={4} step={0.05} bind:value={star.position} />
          <input
            type="number"
            min={spec.hz_position.limits.min}
            max={spec.hz_position.limits.max}
            step="any"
            bind:value={star.position}
          />
        {:else}
          <input
            type="number"
            min={spec.orbit_au.min}
            max={spec.orbit_au.max}
            step="any"
            bind:value={star.au}
            aria-label={t.form.orbitAu}
          />
        {/if}
      </div>
    {/if}
    <p class="help">{star.orbitBy === 'position' ? t.form.orbitPositionHelp : t.form.orbitAuHelp}</p>
  </fieldset>
  <fieldset>
    <legend>
      {t.form.eccentricity}
      <label class="check inline">
        <input type="checkbox" bind:checked={star.eccentricityRandom} />
        {t.form.random}
      </label>
    </legend>
    {#if !star.eccentricityRandom}
      <div class="slider">
        <input type="range" min={0} max={0.9} step={0.005} bind:value={star.eccentricity} />
        <input
          type="number"
          min={spec.eccentricity.min}
          max={spec.eccentricity.max}
          step="any"
          bind:value={star.eccentricity}
          aria-label={t.form.eccentricity}
        />
      </div>
    {/if}
    <p class="help">{t.form.eccentricityHelp}</p>
  </fieldset>
  {#if !giantWorld}
  <fieldset>
    <legend>{t.form.moons}</legend>
    <select bind:value={moons} aria-label={t.form.moons}>
      <option value="auto">{t.form.moonsAuto}</option>
      <option value="none">{t.form.moonsNone}</option>
      {#each Array.from({ length: spec.moon_count.max - spec.moon_count.min + 1 }, (_, i) => spec.moon_count.min + i) as count (count)}
        <option value={count}>{t.form.moonsCount({ count })}</option>
      {/each}
    </select>
    <p class="help">{t.form.moonsHelp}</p>
  </fieldset>
  {/if}
  <fieldset>
    <legend>
      {t.form.metallicity}
      <label class="check inline">
        <input type="checkbox" bind:checked={giantChoice.metallicityRandom} />
        {t.form.random}
      </label>
    </legend>
    {#if !giantChoice.metallicityRandom}
      <div class="slider">
        <input type="range" min={spec.metallicity.min} max={spec.metallicity.max} step={0.01} bind:value={giantChoice.metallicity} />
        <input
          type="number"
          min={spec.metallicity.min}
          max={spec.metallicity.max}
          step="any"
          bind:value={giantChoice.metallicity}
          aria-label={t.form.metallicity}
        />
      </div>
    {/if}
    <p class="help">{t.form.metallicityHelp}</p>
  </fieldset>
  {#if giantWorld}
    <fieldset>
      <legend>
        {t.form.giantMass}
        <label class="check inline">
          <input type="checkbox" bind:checked={giantChoice.massRandom} />
          {t.form.random}
        </label>
      </legend>
      {#if !giantChoice.massRandom}
        <div class="slider">
          <!-- Logarithmic: from a small ice giant to 13 Jupiter masses. -->
          <input
            type="range"
            min={Math.log10(spec.giant_mass.min)}
            max={Math.log10(spec.giant_mass.max)}
            step={0.01}
            value={Math.log10(giantChoice.mass)}
            oninput={(e) => (giantChoice.mass = Math.round(10 ** Number(e.currentTarget.value)))}
            aria-label={t.form.giantMass}
          />
          <input type="number" min={spec.giant_mass.min} max={spec.giant_mass.max} step="any" bind:value={giantChoice.mass} />
        </div>
      {/if}
      <p class="help">{t.form.giantMassHelp}</p>
    </fieldset>
  {:else}
    <fieldset>
      <legend>{t.form.giants}</legend>
      <select bind:value={giantChoice.giants} aria-label={t.form.giants}>
        <option value="auto">{t.form.giantsAuto}</option>
        <option value="none">{t.form.giantsNone}</option>
      </select>
      <p class="help">{t.form.giantsHelp}</p>
    </fieldset>
    <fieldset>
      <legend>{t.form.host}</legend>
      <select bind:value={giantChoice.host} aria-label={t.form.host}>
        <option value="auto">{t.form.hostAuto}</option>
        <option value="none">{t.form.hostNone}</option>
        <option value="giant">{t.form.hostGiant}</option>
      </select>
      <p class="help">{t.form.hostHelp}</p>
    </fieldset>
  {/if}

  <div class="row">
    <label for="seed">{t.form.seed}</label>
    <div class="seed">
      <input id="seed" type="text" inputmode="numeric" placeholder={t.form.seedPlaceholder} bind:value={seed} />
      <button type="button" onclick={rollSeed} title={t.form.rollHelp}>{t.form.roll}</button>
      <button type="button" onclick={() => (seed = '')} disabled={seed === ''} title={t.form.clearHelp}
        >{t.form.clear}</button
      >
    </div>
  </div>
  {#if !seedValid}
    <p class="error">{t.form.seedInvalid({ max: formatInt(MAX_SEED) })}</p>
  {:else if conflict}
    <div class="warning">
      <p>
        {t.form.exists({ id: targetId ?? '' })}
      </p>
      <label class="check">
        <input type="checkbox" bind:checked={replace} />
        {t.form.replace({ id: targetId ?? '' })}
      </label>
    </div>
  {/if}

  <label class="row">
    <span>{t.form.size}</span>
    <select bind:value={size}>
      {#each sizes as [w, h] (`${w}x${h}`)}
        <option value={`${w}x${h}`} disabled={w > spec.max_size || h > spec.max_size || w * h > spec.max_pixels}
          >{w} × {h}{w * h > spec.max_pixels ? t.form.tooLarge : ''}</option
        >
      {/each}
    </select>
  </label>
  <p class="help cost">
    {cost.time ? t.form.takes({ time: cost.time }) : t.form.timeUnknown}{t.form.peaks({
      server: cost.server,
      viewer: cost.viewer,
    })}
  </p>

  <!-- Sticks to the bottom of the screen on narrow layouts, so the buttons
       stay in view while the parameters scroll. -->
  <div class="footer">
    {#if error}
      <p class="error">{error}</p>
    {/if}
    <p class="help">{t.form.background}</p>
    <div class="actions">
      <button type="button" class="reset" onclick={resetForm} title={t.form.defaultsHelp}
        >{t.form.defaults}</button
      >
      <button type="button" onclick={onCancel}>{t.form.close}</button>
      <button type="submit" class="primary" disabled={submitting || !seedValid || (conflict && !replace)}>
        {submitting ? t.form.sending : t.form.generate}
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
  .star {
    display: flex;
    gap: 6px;
  }
  legend .mode {
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
  .actions .reset {
    margin-right: auto;
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
