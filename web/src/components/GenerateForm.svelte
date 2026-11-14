<script lang="ts">
  import { hostsAdded, planetsRequest } from '../lib/systemPlanets';
  import { errorText } from '../lib/i18n/errors';
  import { t } from '../lib/i18n/index.svelte';
  import { archetypeDescription, archetypeName } from '../lib/i18n/names.svelte';
  import { untrack } from 'svelte';
  import { api } from '../lib/api/client';
  import type { FormSpec } from '../lib/api/types/FormSpec';
  import type { SystemCreated } from '../lib/api/types/SystemCreated';
  import type { SystemRequest } from '../lib/api/types/SystemRequest';
  import type { WholeSystemRequest } from '../lib/api/types/WholeSystemRequest';
  import { ApiFailure } from '../lib/api/client';
  import type { ArchetypeInfo } from '../lib/api/types/ArchetypeInfo';
  import type { WorldRequest } from '../lib/api/types/WorldRequest';
  import { formatInt, formatUpTo, MAX_SEED, randomSeed } from '../lib/format';
  import { formatDrift } from '../lib/drift';
  import { formatBytes, formatEstimate, serverPeakBytes, viewerBytes } from '../lib/estimate';
  import { initialFormSize } from '../lib/sizes';
  import { clearFormMemory, FALLBACK_ARCHETYPE, loadFormMemory, saveFormMemory } from '../lib/formMemory';
  import { jobs } from '../lib/jobs.svelte';
  import { applyStarChoice, initialStarChoice, STAR_CLASSES, starType, type StarChoice } from '../lib/starForm';
  import { loadSetting, saveSetting } from '../lib/settings';
  import { type MoonChoice, moonChoiceOf, moonsRequest } from '../lib/moons';
  import { applyGiantChoice, type GiantChoice, initialGiantChoice } from '../lib/giantForm';
  import { keepModelInputs, keepsModelInputs, startsDrawn } from '../lib/variedWorld';

  interface Props {
    archetypes: ArchetypeInfo[];
    spec: FormSpec;
    /**
     * Parameters to start from (varying an existing world), or null for the
     * last form submitted in this browser (the archetype's values without one).
     */
    initial?: WorldRequest | null;
    /** Id of the world being varied, for the title. */
    variedFrom?: string | null;
    /** Called once the server drew the system; the form closes. */
    onSubmitted: (created: SystemCreated, request: WorldRequest | null) => void;
    onCancel: () => void;
    /** The kind of creation the wizard starts on (the menu's two entries); the remembered one without it. */
    entry?: 'around' | 'system' | null;
    /** Opens on the summary step (Vary…, prefilled from a world). */
    atSummary?: boolean;
  }

  let { archetypes, spec, initial = null, variedFrom = null, onSubmitted, onCancel, entry = null, atSummary = false }: Props = $props();

  /**
   * What the form asks for (docs/system-spec.md): a system around the
   * planet it describes, or a whole system, its planets by archetype. A varied
   * world is always a planet.
   */
  type Mode = 'around' | 'system';
  let mode = $state<Mode>(
    untrack(() => (initial ? 'around' : (entry ?? (loadSetting<Mode>('formMode', 'around') === 'system' ? 'system' : 'around')))),
  );
  /**
   * The wizard's steps (docs/system-spec.md, *Interface*): the kind of
   * creation, the star, the planets (or the planet), the advanced options,
   * folded, and a summary.
   */
  const STEPS = ['kind', 'star', 'planets', 'advanced', 'summary'] as const;
  type Step = (typeof STEPS)[number];
  let step = $state<Step>(untrack(() => (atSummary ? 'summary' : 'kind')));
  const stepIndex = $derived(STEPS.indexOf(step));
  /** Whether a step's entries are valid, so that the wizard may go past it. */
  const stepValid = (s: Step) => (s === 'planets' ? planetsValid : s === 'advanced' ? seedValid : true);
  function go(to: Step): void {
    error = null;
    step = to;
  }
  $effect(() => saveSetting('formMode', mode));
  /** A whole system: its star (a class and subtype, or drawn) and its planets by archetype (or all drawn). */
  let systemStarRandom = $state(true);
  let systemClass = $state<string>('G');
  let systemSubtype = $state(2);
  let planetsDrawn = $state(true);
  let planetCounts = $state<Record<string, number>>(untrack(() => Object.fromEntries(spec.archetypes.map((name) => [name, name === FALLBACK_ARCHETYPE ? 1 : 0]))));
  let moonCounts = $state<Record<string, number>>(untrack(() => Object.fromEntries(spec.moon_archetypes.map((name) => [name, 0]))));
  /** The built-in giant each moon archetype orbits, as the catalog says. */
  const moonHosts = $derived(Object.fromEntries(archetypes.flatMap((a) => (a.moon_host ? [[a.spec, a.moon_host]] : []))));
  const countOf = (counts: Record<string, number>) => Object.values(counts).reduce((sum, n) => sum + (Number.isFinite(n) ? n : 0), 0);
  /** Planets and moons asked, plus the giants the moons bring along. */
  const planetTotal = $derived(countOf(planetCounts) + countOf(moonCounts) + hostsAdded(moonCounts, planetCounts, moonHosts));
  const planetsValid = $derived(planetsDrawn || (planetTotal >= 1 && planetTotal <= spec.max_planets));
  /** The system the server said exists already, which the next submission may replace. */
  let existsId = $state<string | null>(null);

  /** The inputs the form sets itself; the planet's model inputs wait for their own fields. */
  type Field = 'axial_tilt_deg';

  interface FieldInfo {
    /** Slider range; the number input accepts the full server limits. */
    slider: [number, number];
    step: number;
  }

  const FIELDS: Record<Field, FieldInfo> = {
    axial_tilt_deg: {
      slider: [0, 90],
      step: 0.1,
    },
  };
  const FIELD_NAMES = Object.keys(FIELDS) as Field[];
  /** Label and help of a field, in the active locale. */
  const fieldText = (field: Field) => t.form.fields[field];

  /** Each parameter: random (left to the archetype or the server) or a fixed value. */
  type FieldState = Record<Field, { random: boolean; value: number }>;

  /** Earth's axial tilt (23.44 degrees), what a drawn field shows once the user fixes it. */
  const EARTH_TILT = 23.44;

  function initialState(archetype: ArchetypeInfo | undefined, from: WorldRequest | null): FieldState {
    const state = {} as FieldState;
    for (const field of FIELD_NAMES) {
      const given = from?.[field];
      if (given !== undefined) {
        state[field] = { random: false, value: given };
        continue;
      }
      const d = archetype?.orbit.axial_tilt ?? null;
      state[field] =
        typeof d === 'number'
          ? { random: false, value: d }
          : { random: true, value: d ? (d[0] + d[1]) / 2 : EARTH_TILT };
    }
    return state;
  }

  /** The archetype a request names: by spec (built-in name or file) or by name. */
  function findArchetype(name: string | undefined): ArchetypeInfo | undefined {
    return name === undefined ? undefined : archetypes.find((a) => a.spec === name || a.name === name);
  }

  /** Lower end of the giant mass slider, Earth masses: a mini-Neptune's (`mini-neptune` starts at 5). */
  const GIANT_SLIDER_MIN = 5;
  /** Whether an archetype's world takes the giant path, as the server's model derives it. */
  function isGiantArchetype(archetype: ArchetypeInfo | undefined): boolean {
    return archetype?.giant === true;
  }

  // The form starts from the props as they are when it opens; later prop
  // changes must not reset what the user typed, hence untrack.
  const start = untrack(() => ({ initial, archetype: findArchetype(initial?.planet) ?? findArchetype(FALLBACK_ARCHETYPE) ?? archetypes[0] }));
  const missingArchetype = start.initial?.planet !== undefined && !findArchetype(start.initial.planet);
  /** The catalog spec of the varied world's archetype (a request sends the spec, world.json the name). */
  const initialSpec = findArchetype(start.initial?.planet)?.spec;
  /** A blank form starts from the last one submitted (an archetype no longer offered becomes earth-like). */
  const memory = untrack(() => {
    return initial === null ? loadFormMemory(
      FIELD_NAMES,
      archetypes.filter((a) => !a.moon).map((a) => a.spec),
    ) : null;
  });
  const memoryArchetype = untrack(() => (memory ? archetypes.find((a) => a.spec === memory.planet) : undefined));
  let planet = $state(memoryArchetype?.spec ?? start.archetype?.spec ?? '');
  /** A varied world asked as no archetype starts on "drawn from the star", its inputs kept (see variedWorld.ts). */
  let randomPlanet = $state(startsDrawn(start.initial, memory?.randomPlanet));
  let seed = $state(start.initial?.seed !== undefined ? String(start.initial.seed) : '');
  let size = $state(
    untrack(() =>
      initialFormSize(
        start.initial?.width && start.initial.height ? [start.initial.width, start.initial.height] : null,
        memory?.size,
        spec,
      ),
    ),
  );
  let fields = $state<FieldState>(
    untrack(() =>
      memory
        ? { ...initialState(memoryArchetype, null), ...(memory.fields as Partial<FieldState>) }
        : initialState(start.archetype, start.initial),
    ),
  );
  /** The moons asked for: drawn by the seed, none, or a number. */
  let moons = $state<MoonChoice>(untrack(() => memory?.moons ?? moonChoiceOf(start.initial?.moons)));
  /** Metallicity, giant neighbours, a giant's moon, a giant's mass (see giantForm.ts). */
  let giantChoice = $state<GiantChoice>(
    untrack(() => memory?.giant ?? initialGiantChoice(start.initial, !randomPlanet && isGiantArchetype(start.archetype))),
  );
  /** The star and the orbit (see starForm.ts). */
  let star = $state<StarChoice>(
    untrack(() => memory?.star ?? initialStarChoice(start.archetype, start.initial, spec.hz_position.fallback)),
  );
  /** Subtypes of a class, by half steps: O starts at O3, the sequence ends at M9.5. */
  function subtypes(klass: string): number[] {
    const first = klass === 'O' ? 6 : 0;
    return Array.from({ length: 20 - first }, (_, i) => (first + i) / 2);
  }
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let replace = $state(false);

  const selectedArchetype = $derived(archetypes.find((a) => a.spec === planet));
  /** A giant planet: no ground, so no drift, moons or giant neighbours to set. */
  const giantWorld = $derived(!randomPlanet && isGiantArchetype(selectedArchetype));
  /** The fields a world of the chosen kind has. */
  const shownFields = FIELD_NAMES;
  const sizes = $derived.by(() => {
    const list = spec.size_presets.map(([w, h]): [number, number] => [w, h]);
    for (const extra of [size, `${spec.default_width}x${spec.default_height}`]) {
      const [w, h] = extra.split('x').map(Number);
      if (w && h && !list.some(([lw, lh]) => lw === w && lh === h)) {
        list.unshift([w, h]);
      }
    }
    return list;
  });

  function chooseArchetype(archetypeSpec: string): void {
    planet = archetypeSpec;
    const archetype = archetypes.find((a) => a.spec === archetypeSpec);
    fields = initialState(archetype, null);
    star = initialStarChoice(archetype, null, spec.hz_position.fallback);
  }

  /**
   * Back to the defaults: the first archetype's (or the varied world's)
   * parameters, the server's map size and a random seed, and the
   * remembered form forgotten.
   */
  function resetForm(): void {
    clearFormMemory();
    const archetype = start.archetype;
    chooseArchetype(archetype?.spec ?? '');
    if (start.initial) {
      fields = initialState(archetype, start.initial);
      star = initialStarChoice(archetype, start.initial, spec.hz_position.fallback);
    }
    moons = moonChoiceOf(start.initial?.moons);
    randomPlanet = startsDrawn(start.initial, false);
    giantChoice = initialGiantChoice(start.initial, !randomPlanet && isGiantArchetype(archetype));
    seed = start.initial?.seed !== undefined ? String(start.initial.seed) : '';
    size =
      start.initial?.width && start.initial.height
        ? `${start.initial.width}x${start.initial.height}`
        : `${spec.default_width}x${spec.default_height}`;
    replace = false;
    error = null;
  }

  /** The request of a whole system. */
  function buildSystemRequest(): WholeSystemRequest {
    const [width, height] = size.split('x').map(Number);
    return {
      star: systemStarRandom ? null : starType(systemClass, systemSubtype),
      metallicity: giantChoice.metallicityRandom || !Number.isFinite(giantChoice.metallicity) ? null : giantChoice.metallicity,
      planets: planetsDrawn ? null : planetsRequest(planetCounts, moonCounts),
      seed: seed.trim() === '' ? null : Number(seed),
      width: width ?? null,
      height: height ?? null,
    };
  }

  function rollSeed(): void {
    seed = String(randomSeed());
  }

  /** All random: a whole system with every choice left to the draw, at the map size chosen, created at once. */
  async function allRandom(): Promise<void> {
    error = null;
    submitting = true;
    try {
      const [width, height] = size.split('x').map(Number);
      const created = await api.createSystem({
        system: { star: null, metallicity: null, planets: null, seed: null, width: width ?? null, height: height ?? null },
      });
      onSubmitted(created, null);
    } catch (e) {
      error = errorText(e);
    } finally {
      submitting = false;
    }
  }

  /** The summary step: each choice as a line, or random. */
  const summary = $derived.by((): { label: string; value: string }[] => {
    const random = t.form.random;
    const lines: { label: string; value: string }[] = [{ label: t.form.mode, value: mode === 'around' ? t.form.modeAround : t.form.modeSystem }];
    if (mode === 'system') {
      lines.push({ label: t.form.systemStar, value: systemStarRandom ? random : starType(systemClass, systemSubtype) });
      lines.push({
        label: t.form.systemPlanets,
        value: planetsDrawn
          ? t.form.systemPlanetsDrawn
          : Object.entries({ ...planetCounts, ...moonCounts })
              .filter(([, n]) => Number.isFinite(n) && n > 0)
              .map(([type, n]) => `${n} ${archetypeName({ name: type })}`)
              .join(' · '),
      });
    } else {
      lines.push({ label: t.form.planet, value: randomPlanet ? random : selectedArchetype ? archetypeName(selectedArchetype) : planet });
      if (variedFrom && keepsModelInputs(randomPlanet ? undefined : planet, start.initial, initialSpec)) {
        lines.push({ label: t.form.modelInputs, value: t.form.modelInputsKept({ id: variedFrom }) });
      }
      lines.push({ label: t.form.star, value: star.klass === '' ? t.form.starAuto : starType(star.klass, star.subtype) });
      // Each parameter fixed, with its value; the others are drawn.
      const fixed: { label: string; value: string }[] = [];
      for (const f of shownFields) {
        if (!fields[f].random) fixed.push({ label: fieldText(f).label, value: formatUpTo(fields[f].value, 3) });
      }
      if (driftMa !== 0 && !giantWorld) fixed.push({ label: t.form.drift, value: formatDrift(driftMa) });
      if (!star.orbitRandom) {
        fixed.push(
          star.orbitBy === 'position'
            ? { label: t.form.orbitPosition, value: formatUpTo(star.position, 2) }
            : { label: t.form.orbitAu, value: formatUpTo(star.au, 3) },
        );
      }
      if (!star.eccentricityRandom) fixed.push({ label: t.form.eccentricity, value: formatUpTo(star.eccentricity, 3) });
      if (!giantWorld && giantChoice.host !== 'giant' && moons !== 'auto') {
        fixed.push({ label: t.form.moons, value: moons === 'none' ? t.form.moonsNone : t.form.moonsCount({ count: moons }) });
      }
      if (giantWorld && !giantChoice.massRandom) fixed.push({ label: t.form.giantMass, value: formatInt(giantChoice.mass) });
      if (giantChoice.alone) fixed.push({ label: t.form.aloneSummary, value: t.form.aloneNone });
      if (!giantWorld && !giantChoice.alone && giantChoice.giants === 'none') fixed.push({ label: t.form.giants, value: t.form.giantsNone });
      if (!giantWorld && giantChoice.host !== 'auto') {
        fixed.push({ label: t.form.host, value: giantChoice.host === 'giant' ? t.form.hostGiant : t.form.hostNone });
      }
      lines.push(...(fixed.length > 0 ? fixed : [{ label: t.form.summaryFixed, value: t.form.summaryNone }]));
    }
    lines.push({
      label: t.form.metallicity,
      value: giantChoice.metallicityRandom ? random : formatUpTo(giantChoice.metallicity, 2),
    });
    lines.push({ label: t.form.seed, value: seed.trim() === '' ? random : seed.trim() });
    lines.push({ label: t.form.size, value: size.replace('x', ' × ') });
    return lines;
  });

  /** The epoch of the plates a varied world was shown at (the drift slider), kept as it is: the form has no field for it. */
  const driftMa = start.initial?.drift_ma ?? 0;
  /** The age of the star of a varied world. */
  const startAge = start.initial?.age_gyr;

  function buildRequest(): WorldRequest {
    const [width, height] = size.split('x').map(Number);
    const request: WorldRequest = {};
    if (driftMa !== 0 && !giantWorld) {
      request.drift_ma = driftMa;
    }
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
    // A varied world keeps the age of its star (the form has no field for it),
    // unless the star was changed: the old age might outlive the new one.
    if (startAge !== undefined && request.star === start.initial?.star) {
      request.age_gyr = startAge;
    }
    const asked = moonsRequest(moons);
    // A giant's moon keeps no moons of its own (its Hill sphere is too small).
    if (asked !== undefined && !giantWorld && giantChoice.host !== 'giant') {
      request.moons = asked;
    }
    applyGiantChoice(request, giantChoice, giantWorld);
    // The varied world's inputs, while its planet is still the one asked for.
    keepModelInputs(request, start.initial, initialSpec);
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

  const conflict = $derived(existsId !== null);
  const targetId = $derived(existsId);
  // Another request is another system: ask again whether to replace it.
  $effect(() => {
    void [mode, seed, planet, randomPlanet];
    untrack(() => {
      existsId = null;
      replace = false;
    });
  });

  const seedValid = $derived(seed.trim() === '' || (/^\d+$/.test(seed.trim()) && Number(seed) <= MAX_SEED));

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    error = null;
    submitting = true;
    try {
      const request = mode === 'around' ? buildRequest() : null;
      const asked: SystemRequest = request ? { around: request } : { system: buildSystemRequest() };
      const created = await api.createSystem(asked, { replace: conflict && replace });
      saveFormMemory({
        planet,
        randomPlanet,
        size,
        fields,
        star,
        moons: giantChoice.host === 'giant' ? 'auto' : moons,
        giant: giantChoice,
      });
      onSubmitted(created, request);
    } catch (e) {
      if (e instanceof ApiFailure && e.detail?.code === 'system_exists') {
        existsId = e.detail.system;
        return;
      }
      error = errorText(e);
    } finally {
      submitting = false;
    }
  }
</script>

<form onsubmit={submit}>
  <h2>{variedFrom ? t.form.vary({ id: variedFrom }) : t.form.newSystem}</h2>
  <ol class="steps" aria-label={t.form.stepsLabel}>
    {#each STEPS as s, i (s)}
      <li class:current={s === step} class:done={i < stepIndex}>
        <button type="button" onclick={() => go(s)} disabled={i > stepIndex && !STEPS.slice(0, i).every(stepValid)} aria-current={s === step ? 'step' : undefined}
          ><span class="number">{i + 1}</span> {t.form.steps[s]}</button
        >
      </li>
    {/each}
  </ol>
  {#if variedFrom}
    <p class="help">{t.form.variedHelp({ id: variedFrom })}</p>
  {:else if memory}
    <p class="help">{t.form.memoryHelpBefore}<strong>{t.form.memoryHelpDefaults}</strong>{t.form.memoryHelpAfter}</p>
  {/if}
  {#if missingArchetype}
    <p class="error">{t.form.missingArchetype({ archetype: start.initial?.planet ?? '' })}</p>
  {/if}

  {#if step === 'kind'}
    {#if !variedFrom}
      <div class="kinds" role="group" aria-label={t.form.mode}>
        <button type="button" class="kind" class:active={mode === 'system'} aria-pressed={mode === 'system'} onclick={() => (mode = 'system')}>
          <strong>{t.form.modeSystem}</strong>
          <small>{t.form.modeSystemHelp}</small>
        </button>
        <button type="button" class="kind" class:active={mode === 'around'} aria-pressed={mode === 'around'} onclick={() => (mode = 'around')}>
          <strong>{t.form.modeAround}</strong>
          <small>{t.form.modeAroundHelp}</small>
        </button>
      </div>
      <div class="all-random">
        <button type="button" onclick={allRandom} disabled={submitting}>{t.form.allRandom}</button>
        <small class="help">{t.form.allRandomHelp}</small>
      </div>
    {:else}
      <p class="help">{t.form.modeAround}</p>
    {/if}
  {:else if step === 'star'}
    {#if mode === 'system'}
      <fieldset>
        <legend>
          {t.form.systemStar}
          <label class="check inline">
            <input type="checkbox" bind:checked={systemStarRandom} />
            {t.form.random}
          </label>
        </legend>
        {#if !systemStarRandom}
          <div class="star">
            <label class="labelled">
              <span>{t.form.starClass}</span>
              <select bind:value={systemClass}>
                {#each STAR_CLASSES as klass (klass)}
                  <option value={klass}>{klass}</option>
                {/each}
              </select>
            </label>
            <label class="labelled">
              <span>{t.form.starSubtype}</span>
              <input type="number" min={systemClass === 'O' ? 3 : 0} max="9.5" step="0.5" bind:value={systemSubtype} />
            </label>
            <span class="labelled">
              <span>{t.form.starType}</span>
              <strong class="value">{starType(systemClass, systemSubtype)}</strong>
            </span>
          </div>
          <p class="help">{t.form.starClasses[systemClass]}</p>
        {:else}
          <p class="help">{t.form.systemStarRandomHelp}</p>
        {/if}
      </fieldset>
    {:else}
  <fieldset>
    <legend>{t.form.star}</legend>
    <div class="star">
      <label class="labelled">
        <span>{t.form.starClass}</span>
        <select bind:value={star.klass} onchange={() => (star.subtype = Math.max(star.subtype, star.klass === 'O' ? 3 : 0))}>
          <option value="">{t.form.starAuto}</option>
          {#each STAR_CLASSES as klass (klass)}
            <option value={klass}>{klass}</option>
          {/each}
        </select>
      </label>
      <label class="labelled">
        <span>{t.form.starSubtype}</span>
        <select bind:value={star.subtype} disabled={star.klass === ''}>
          {#each subtypes(star.klass) as subtype (subtype)}
            <option value={subtype}>{formatUpTo(subtype, 1)}</option>
          {/each}
        </select>
      </label>
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
    {/if}
  {:else if step === 'planets'}
    {#if mode === 'system'}
      <fieldset>
        <legend>
          {t.form.systemPlanets}
          <label class="check inline">
            <input type="checkbox" bind:checked={planetsDrawn} />
            {t.form.systemPlanetsDrawn}
          </label>
        </legend>
        {#if !planetsDrawn}
          <div class="counts">
            {#each spec.archetypes as type (type)}
              <label>
                <span>{archetypeName({ name: type })}</span>
                <input type="number" min="0" max={spec.max_planets} step="1" bind:value={planetCounts[type]} />
              </label>
            {/each}
          </div>
          <p class="help" class:error={!planetsValid}>{t.form.systemPlanetsTotal({ count: planetTotal, max: spec.max_planets })}</p>
          {#if spec.moon_archetypes.length > 0}
            <h4>{t.form.systemMoons}</h4>
            <div class="counts">
              {#each spec.moon_archetypes as type (type)}
                <label>
                  <span>{archetypeName({ name: type })}</span>
                  <input type="number" min="0" max={spec.max_planets} step="1" bind:value={moonCounts[type]} />
                </label>
              {/each}
            </div>
            <p class="help">{t.form.systemMoonsHelp}</p>
          {/if}
        {/if}
        <p class="help">{t.form.systemPlanetsHelp}</p>
      </fieldset>
    {:else}
  <label class="row">
    <span>{t.form.planet}</span>
    <select value={planet} onchange={(e) => chooseArchetype(e.currentTarget.value)} disabled={randomPlanet}>
      {#each archetypes.filter((a) => !a.moon) as archetype (archetype.spec)}
        <option value={archetype.spec}>{archetypeName(archetype)}{archetype.built_in ? '' : ` (${archetype.spec})`}</option>
      {/each}
    </select>
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={randomPlanet} />
    {t.form.randomPlanet}
  </label>
  {#if selectedArchetype && !randomPlanet}
    <p class="help">{archetypeDescription(selectedArchetype)}</p>
  {/if}

  {#each shownFields as field (field)}
    {@const info = FIELDS[field]}
    {@const limits = spec.inputs[field]}
    <fieldset>
      <legend>
        {fieldText(field).label}
        <label class="check inline">
          <input type="checkbox" bind:checked={fields[field].random} />
          {t.form.random}
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
          <input type="number" min={limits.min} max={limits.max} step="any" bind:value={fields[field].value} />
        </div>
      {/if}
      <p class="help">{fieldText(field).help}</p>
    </fieldset>
  {/each}

  {#if !giantWorld && giantChoice.host === 'giant'}
    <p class="help">{t.form.moonsOfAMoon}</p>
  {:else if !giantWorld}
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
  {#if mode === 'around'}
    <fieldset>
      <legend>{t.form.aloneSummary}</legend>
      <label class="check">
        <input type="checkbox" bind:checked={giantChoice.alone} />
        {t.form.alone}
      </label>
      <p class="help">{t.form.aloneHelp}</p>
    </fieldset>
  {/if}
    {/if}
  {:else if step === 'advanced'}
    <details class="advanced">
      <summary>{t.form.advanced}</summary>
      <div class="advanced-body">
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
      {#if giantWorld && mode === 'around'}
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
                min={Math.log10(Math.max(GIANT_SLIDER_MIN, spec.inputs.mass_earth.min))}
                max={Math.log10(spec.inputs.mass_earth.max)}
                step={0.01}
                value={Math.log10(giantChoice.mass)}
                oninput={(e) => (giantChoice.mass = Math.round(10 ** Number(e.currentTarget.value)))}
                aria-label={t.form.giantMass}
              />
              <input type="number" min={Math.max(GIANT_SLIDER_MIN, spec.inputs.mass_earth.min)} max={spec.inputs.mass_earth.max} step="any" bind:value={giantChoice.mass} />
            </div>
          {/if}
          <p class="help">{t.form.giantMassHelp}</p>
        </fieldset>
      {:else if mode === 'around'}
        <fieldset>
          <legend>{t.form.giants}</legend>
          {#if giantChoice.alone}
            <p class="help">{t.form.giantsAlone}</p>
          {:else}
            <select bind:value={giantChoice.giants} aria-label={t.form.giants}>
              <option value="auto">{t.form.giantsAuto}</option>
              <option value="none">{t.form.giantsNone}</option>
            </select>
            <p class="help">{t.form.giantsHelp}</p>
          {/if}
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

      </div>
    </details>
    {#if !seedValid}
      <p class="error">{t.form.seedInvalid({ max: formatInt(MAX_SEED) })}</p>
    {/if}
  {:else}
    <dl class="summary">
      {#each summary as line (line.label)}
        <dt>{line.label}</dt>
        <dd>{line.value}</dd>
      {/each}
    </dl>
    <p class="help cost">
      {cost.time ? t.form.takes({ time: cost.time }) : t.form.timeUnknown}{t.form.peaks({
        server: cost.server,
        viewer: cost.viewer,
      })}
    </p>
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

  {/if}

  <!-- Sticks to the bottom of the wizard, so that the buttons stay in view
       while a step scrolls. -->
  <div class="footer">
    {#if error}
      <p class="error">{error}</p>
    {/if}
    {#if step === 'summary'}
      <p class="help">{t.form.background}</p>
    {/if}
    <div class="actions">
      <button type="button" class="reset" onclick={resetForm} title={t.form.defaultsHelp}
        >{t.form.defaults}</button
      >
      <button type="button" onclick={onCancel}>{t.form.close}</button>
      {#if stepIndex > 0}
        <button type="button" onclick={() => go(STEPS[stepIndex - 1]!)}>{t.form.back}</button>
      {/if}
      {#if step !== 'summary'}
        <button type="button" class="primary" disabled={!stepValid(step)} onclick={() => go(STEPS[stepIndex + 1]!)}>{t.form.next}</button>
      {:else}
        <button type="submit" class="primary" disabled={submitting || !seedValid || !planetsValid || (conflict && !replace)}>
          {submitting ? t.form.sending : t.form.generate}
        </button>
      {/if}
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
  .steps {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .steps button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    color: var(--muted);
  }
  .steps .number {
    display: inline-grid;
    place-items: center;
    width: 1.4em;
    height: 1.4em;
    border-radius: 50%;
    border: 1px solid currentColor;
    font-size: 0.8em;
  }
  .steps li.done button {
    color: var(--text);
  }
  .steps li.current button {
    color: var(--accent-text);
    background: var(--accent);
    border-color: var(--accent);
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 14em), 1fr));
    gap: 10px;
  }
  .kind {
    display: grid;
    gap: 4px;
    padding: 12px;
    text-align: left;
    white-space: normal;
  }
  .kind small {
    color: var(--muted);
  }
  .kind.active {
    border-color: var(--accent);
    outline: 1px solid var(--accent);
  }
  .all-random {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
  }
  .advanced summary {
    cursor: pointer;
    font-weight: 600;
  }
  .advanced-body {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 10px;
    margin-top: 8px;
  }
  .summary {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 4px 12px;
    margin: 0;
  }
  .summary dt {
    color: var(--muted);
  }
  .summary dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .counts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px 10px;
  }
  .counts label {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
  }
  .counts input {
    width: 4em;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 1.1rem;
    overflow-wrap: anywhere;
  }
  .row {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
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
    flex-wrap: wrap;
    gap: 6px 12px;
    align-items: end;
  }
  .labelled {
    display: grid;
    gap: 2px;
  }
  .labelled > span:first-child {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .labelled input {
    width: 6em;
  }
  legend .mode {
    font-weight: 600;
  }
  .slider {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(5em, 7em);
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
  .footer {
    position: sticky;
    bottom: 0;
    padding: 10px 0 14px;
    background: var(--panel-solid);
    border-top: 1px solid var(--border);
  }
  .actions {
    flex-wrap: wrap;
  }
</style>
