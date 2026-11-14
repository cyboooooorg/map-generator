<script lang="ts">
  // The star view of a system: its star alone, close up (StarBodyView), with
  // its facts and its activity, on the same day as the system view
  // (StarPane, see clockDays). Mounted only while it shows: closed, it
  // runs nothing.
  import { onMount, untrack } from 'svelte';
  import type { StarActivity } from '../lib/api/types/StarActivity';
  import type { StarInfo } from '../lib/api/types/StarInfo';
  import { formatInt, formatShare, formatUpTo, formatYear, withUnit } from '../lib/format';
  import { t } from '../lib/i18n/index.svelte';
  import { activeRegions, activityRelativeToSun, angularVelocity } from '../lib/starActivity';
  import { clampSpeed, cycleJumps, cyclePhase, defaultStarSpeed, STAR_SPEED_MAX, STAR_SPEED_MIN } from '../lib/starClock';
  import { currentProfile } from '../lib/viewer/quality';
  import { StarBodyView } from '../lib/viewer/StarBodyView';
  import type { SkyChoice } from '../lib/viewer/skyBackdrop';
  import type { SkyExposure } from '../lib/viewer/skyClouds';
  import { skyToggles, type SkySetting } from '../lib/viewer/skyToggles';
  import { FIXED_SKY, type SkyLook } from '../lib/viewer/skyLook';
  import Dock, { type DockPanel } from './dock/Dock.svelte';
  import PlaybackBar from './dock/PlaybackBar.svelte';
  import ToggleBar, { type Toggle } from './dock/ToggleBar.svelte';

  interface Props {
    star: StarInfo;
    /** The star's activity (its living surface); null shows its limb and granules only. */
    activity?: StarActivity | null;
    /**
     * The day shown, Earth days from the system's t = 0, shared with the
     * system view: written as the clock runs, applied when set from outside.
     */
    clockDays?: number;
    /** Whether the clock runs (thirty frames a second at most). */
    animated: boolean;
    /** Whether the coronal loops (and eruptions) are drawn. */
    loops: boolean;
    /** Whether the star is drawn through the warm display filter rather than in its true colour. */
    filter: boolean;
    /** Back to the system view (the System button, a double click on the star). */
    onOpenSystem: () => void;
    /** The system's own sky (see lib/viewer/skyLook.ts). */
    look?: SkyLook;
    /** Which sky is drawn: the system's own, the fixed one or none. */
    sky?: SkyChoice;
    /** How its clouds and nebulae are rendered: as the eye sees them or as a long exposure. */
    skyExposure?: SkyExposure;
    /** The sky's setting, shared with the planet view (Viewer.svelte), for its switches; null shows none. */
    skySetting?: SkySetting | null;
  }

  let {
    star,
    activity = null,
    clockDays = $bindable(0),
    animated = $bindable(),
    loops = $bindable(),
    filter = $bindable(),
    onOpenSystem,
    look = FIXED_SKY,
    sky = 'system',
    skyExposure = 'photo',
    skySetting = null,
  }: Props = $props();

  const DAYS_PER_YEAR = 365.25;
  /** The time bar's span without a cycle, days. */
  const NO_CYCLE_SPAN_DAYS = 30;
  /** Least time between two updates of the regions counted in the panel, ms (counting them walks the timeline). */
  const STATS_MS = 250;

  let canvas: HTMLCanvasElement;
  let view = $state<StarBodyView | null>(null);
  /** The last day the view reported: any other value of clockDays was set from outside. */
  let reported = Number.NaN;
  /** The day the panel's counts are for, updated a few times a second while the clock runs. */
  let statsDay = $state(untrack(() => clockDays));
  let statsAt = 0;
  /** How fast the clock runs, log2 of days per second: the star view opens with a granule living about 10 s. */
  let speed = $state(untrack(() => defaultStarSpeed(activity)));

  onMount(() => {
    const created = new StarBodyView(canvas, currentProfile());
    created.onClock = (days) => {
      reported = days;
      clockDays = days;
      const now = performance.now();
      if (!animated || now - statsAt >= STATS_MS) {
        statsAt = now;
        statsDay = days;
      }
    };
    created.onOpenSystem = () => onOpenSystem();
    view = created;
    return () => {
      created.dispose();
      view = null;
    };
  });

  $effect(() => view?.setSkyLook(look));
  $effect(() => view?.setSkyChoice(sky));
  $effect(() => view?.setSkyExposure(skyExposure));
  $effect(() => view?.setStar(star, activity));
  // The day set from outside (on opening, the system view's), after setStar.
  $effect(() => {
    const days = clockDays;
    if (!view || days === reported) return;
    untrack(() => view?.setDay(days));
  });
  $effect(() => view?.setSpeed(2 ** speed));
  $effect(() => view?.setAnimated(animated));
  $effect(() => view?.setLoops(loops));
  $effect(() => view?.setFilter(filter));

  /** Back to the starting view of the star. */
  export function resetView(): void {
    view?.resetView();
  }

  /** The time bar's span, days: one activity cycle, or 30 days without one. */
  const span = $derived(activity && activity.cycle_years > 0 ? activity.cycle_years * DAYS_PER_YEAR : NO_CYCLE_SPAN_DAYS);
  /** Where the time bar's window starts: the span holding the day shown. */
  const windowStart = $derived(Math.floor(clockDays / span) * span);
  const goToDay = (days: number) => view?.setDay(days);

  /** How fast time runs, in the unit that reads best. */
  const rateText = $derived.by(() => {
    const seconds = 2 ** speed * 86_400;
    if (seconds < 60) return t.starSurface.rateSeconds({ n: formatUpTo(seconds, seconds < 10 ? 1 : 0) });
    if (seconds < 3600) return t.starSurface.rateMinutes({ n: formatUpTo(seconds / 60, seconds < 600 ? 1 : 0) });
    if (seconds < 86_400) return t.starSurface.rateHours({ n: formatUpTo(seconds / 3600, seconds < 36_000 ? 1 : 0) });
    return t.starSurface.rateDays({ n: formatInt(seconds / 86_400) });
  });
  const dayText = $derived(t.starSurface.dayValue({ day: formatInt(Math.floor(clockDays) + 1) }));

  /** Rotation period at latitude `degrees`, days. */
  const rotationAt = (a: StarActivity, degrees: number) => (2 * Math.PI) / angularVelocity(a, (degrees * Math.PI) / 180);
  const daysText = (days: number) => t.worldInfo.yearDays({ days: formatUpTo(days, days < 10 ? 2 : 1) });
  /** The regions alive at the panel's day, on the whole star, and the spotted share of the visible hemisphere. */
  const now = $derived.by(() => {
    if (!activity || !(activity.region_rate > 0)) return { regions: 0, coverage: 0 };
    const regions = activeRegions(activity, statsDay, Number.MAX_SAFE_INTEGER);
    // The whole star's spots, half of them facing the viewer on average.
    const coverage = regions.reduce((sum, r) => sum + r.area, 0) / 2;
    return { regions: regions.length, coverage };
  });
  const phase = $derived(activity ? cyclePhase(activity, statsDay) : null);

  const toggleGroups = $derived<Toggle[][]>([
    [
      ...(activity !== null && activity.envelope !== 'radiative'
        ? [{ key: 'loops', label: t.starView.prominences, help: t.starView.prominencesHelp, icon: 'prominence' as const, on: loops, set: (v: boolean) => (loops = v) }]
        : []),
      { key: 'filter', label: t.starSurface.filter, help: t.starSurface.filterHelp, icon: 'filter', on: filter, set: (v: boolean) => (filter = v) },
    ],
    // The sky behind the star: the same switches as the planet view's, on the same setting.
    skySetting ? skyToggles(skySetting) : [],
  ]);
  const dockPanels = $derived<DockPanel[]>([
    { id: 'star', title: t.starSurface.starSection, icon: 'star', content: starPanel },
    ...(activity ? [{ id: 'activity', title: t.starSurface.activitySection, icon: 'sun' as const, content: activityPanel }] : []),
  ]);
  const jumps = $derived.by(() => {
    if (!activity || !(activity.cycle_years > 0)) return [];
    const a = activity;
    const day = () => view?.day ?? clockDays;
    return [
      {
        key: 'maximum',
        label: t.starSurface.toMaximum,
        help: t.starSurface.toMaximumHelp,
        icon: 'peak' as const,
        run: () => {
          const next = cycleJumps(a, day());
          if (next) goToDay(next.maximum);
        },
      },
      {
        key: 'minimum',
        label: t.starSurface.toMinimum,
        help: t.starSurface.toMinimumHelp,
        icon: 'trough' as const,
        run: () => {
          const next = cycleJumps(a, day());
          if (next) goToDay(next.minimum);
        },
      },
    ];
  });
</script>

<canvas bind:this={canvas} class="star-canvas"></canvas>
{#snippet starPanel()}
  <p class="heading"><strong>{t.starView.heading({ type: star.type, kind: t.starView.classNames[star.type[0] ?? 'G'] ?? '' })}</strong></p>
  <dl>
    <dt title={t.starSurface.temperatureHelp}>{t.starSurface.temperature}</dt>
    <dd>{withUnit(formatInt(Math.round(star.teff_k / 10) * 10), 'K')}</dd>
    {#if activity}
      <dt title={t.starSurface.envelopeHelp}>{t.starSurface.envelope}</dt>
      <dd>{t.starSurface.envelopes[activity.envelope] ?? activity.envelope}</dd>
      <dt title={t.starSurface.rotationHelp}>{t.starSurface.rotation}</dt>
      <dd>{t.starSurface.rotationValue({ equator: daysText(rotationAt(activity, 0)), sixty: daysText(rotationAt(activity, 60)) })}</dd>
      <dt title={t.starSurface.ageHelp}>{t.starSurface.age}</dt>
      <dd>{withUnit(formatUpTo(activity.age_gyr, activity.age_gyr < 1 ? 2 : 1), t.starView.gyr)}</dd>
      <dt title={t.starSurface.rossbyHelp}>{t.starSurface.rossby}</dt>
      <dd>{activity.rossby > 0 ? formatUpTo(activity.rossby, 2) : t.starSurface.none}</dd>
    {/if}
  </dl>
  {#if !activity}
    <small class="muted">{t.starSurface.noActivity}</small>
  {/if}
{/snippet}
{#snippet activityPanel()}
  {#if activity}
    <dl>
      <dt title={t.starSurface.activityHelp}>{t.starSurface.activity}</dt>
      <dd>
        {activity.activity > 0
          ? t.starSurface.activityValue({ factor: `${formatUpTo(activityRelativeToSun(activity), activityRelativeToSun(activity) < 10 ? 2 : 0)}×` })
          : t.starSurface.none}
      </dd>
      <dt title={t.starSurface.cycleHelp}>{t.starSurface.cycle}</dt>
      <dd>
        {activity.cycle_years > 0
          ? t.starSurface.cycleValue({ period: formatYear(activity.cycle_years * DAYS_PER_YEAR), phase: phase ? (t.starSurface.phases[phase] ?? phase) : '' })
          : t.starSurface.noCycle}
      </dd>
      <dt title={t.starSurface.regionsHelp}>{t.starSurface.regions}</dt>
      <dd>{formatInt(now.regions)}</dd>
      <dt title={t.starSurface.coverageHelp}>{t.starSurface.coverage}</dt>
      <dd>{formatShare(now.coverage)}</dd>
    </dl>
  {/if}
{/snippet}
<div class="star-chrome">
  <button type="button" class="view-switch" title={t.starSurface.openSystemHelp} onclick={onOpenSystem}>{t.starSurface.openSystem}</button>
  <div class="toggles">
    <ToggleBar label={t.starView.displaySection} groups={toggleGroups} />
  </div>
  <Dock kind="starSurface" panels={dockPanels} defaultActive="star" />
  <PlaybackBar
    bind:playing={animated}
    speed={{
      value: speed,
      min: STAR_SPEED_MIN,
      max: STAR_SPEED_MAX,
      text: rateText,
      help: t.starSurface.speedHelp,
      set: (v) => (speed = clampSpeed(v, 'star')),
    }}
    time={{
      value: clockDays - windowStart,
      max: span,
      text: dayText,
      help: activity && activity.cycle_years > 0 ? t.starSurface.timeBarHelp : t.starSurface.timeBarHelpNoCycle,
      set: (v) => goToDay(windowStart + v),
    }}
    clock={`${rateText} · ${dayText}`}
    {jumps}
  />
</div>

<style>
  .star-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    cursor: grab;
  }
  .star-canvas:active {
    cursor: grabbing;
  }
  .star-chrome {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .toggles {
    position: absolute;
    left: 12px;
    top: 72px;
    pointer-events: auto;
  }
  .view-switch {
    position: absolute;
    left: 12px;
    bottom: 72px;
    pointer-events: auto;
    /* Framed as the theme frames panels (see app.css and lib/themes.ts). */
    background: var(--frame-deco), var(--panel);
    clip-path: var(--frame-clip);
    box-shadow: var(--glow);
    padding: 4px 10px;
  }
  /* The playback bar wraps to two lines on narrow views. */
  @media (max-width: 720px) {
    .view-switch {
      bottom: 128px;
    }
  }
  .heading {
    margin: 0;
  }
  dl {
    display: grid;
    /* As wide as the longest term in the language, never more than half. */
    grid-template-columns: fit-content(50%) 1fr;
    gap: 2px 10px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dt[title] {
    cursor: help;
  }
  dd {
    margin: 0;
  }
  .muted {
    color: var(--muted);
  }
</style>
