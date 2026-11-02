// Generation jobs followed by the page: progress, cancellation, time
// estimates, and jobs picked up again after a reload.
import { api, followJob, isFinished } from './api/client';
import type { JobStatus } from './api/types/JobStatus';
import type { Stage } from './api/types/Stage';
import type { WorldRequest } from './api/types/WorldRequest';
import { loadNumbers, saveSetting } from './settings';

/** Stages in generation order (mapgen::generation::Stage::ALL). */
export const STAGES: readonly Stage[] = [
  'noise',
  'erosion',
  'temperature',
  'coast',
  'moisture',
  'rivers',
  'biomes',
  'export',
];

export const STAGE_LABELS: Record<Stage, string> = {
  noise: 'Shaping continents',
  erosion: 'Eroding the terrain',
  temperature: 'Computing temperatures',
  coast: 'Measuring coastlines',
  moisture: 'Blowing winds and rain',
  rivers: 'Routing rivers and lakes',
  biomes: 'Choosing biomes',
  export: 'Writing the maps',
};

export interface TrackedJob {
  jobId: string;
  worldId: string;
  request: WorldRequest;
  /** Map size, in millions of pixels: stage durations scale with it. */
  megapixels: number;
  status: JobStatus;
  /** When the current stage started (ms), as seen by this page. */
  stageStart: number | null;
  /** When the generation started running (ms), as seen by this page. */
  runStart: number | null;
  /** Whether this page submitted the job (not picked up after a reload). */
  submittedHere: boolean;
  cancelling: boolean;
}

/** Seconds per megapixel of each stage, averaged over past generations. */
type StageTimes = Partial<Record<Stage, number>>;

const TIMES_KEY = 'stageSecondsPerMegapixel';

class Jobs {
  list = $state<TrackedJob[]>([]);
  /** Map size used when a request does not set one (the server default). */
  defaultSize: [number, number] = [1920, 1080];
  /** Called once per job when it finishes. */
  onFinished: (job: TrackedJob) => void = () => {};
  private times: StageTimes = loadNumbers(TIMES_KEY);
  private followed = new Set<string>();

  get active(): TrackedJob[] {
    return this.list.filter((j) => !isFinished(j.status));
  }

  /** Starts following a job. */
  track(jobId: string, worldId: string, request: WorldRequest, status: JobStatus, submittedHere: boolean): void {
    if (this.followed.has(jobId)) {
      return;
    }
    this.followed.add(jobId);
    const width = request.width ?? this.defaultSize[0];
    const height = request.height ?? this.defaultSize[1];
    this.list = [
      {
        jobId,
        worldId,
        request,
        megapixels: (width * height) / 1e6,
        status,
        // A job picked up mid-stage: that stage's start is unknown.
        stageStart: null,
        runStart: null,
        submittedHere,
        cancelling: false,
      },
      ...this.list,
    ];
    followJob(jobId, (s) => this.update(jobId, s)).done.then((final) => {
      this.update(jobId, final);
      const job = this.find(jobId);
      if (job) {
        this.onFinished(job);
      }
    });
  }

  /** Follows the jobs still running on the server (after a page reload). */
  async reattach(): Promise<void> {
    const jobs = await api.jobs();
    for (const job of jobs.reverse()) {
      if (!isFinished(job.status)) {
        // The server writes unset fields as null: drop them, as the type expects.
        const request = Object.fromEntries(
          Object.entries(job.request).filter(([, v]) => v !== null && v !== undefined),
        ) as WorldRequest;
        this.track(job.job_id, job.world_id, request, job.status, false);
      }
    }
  }

  async cancel(jobId: string): Promise<void> {
    this.patch(jobId, { cancelling: true });
    try {
      await api.cancelJob(jobId);
    } catch (e) {
      this.patch(jobId, { cancelling: false });
      throw e;
    }
  }

  dismiss(jobId: string): void {
    this.list = this.list.filter((j) => j.jobId !== jobId);
  }

  /** Estimated seconds left, or null without enough history. */
  remaining(job: TrackedJob, now: number): number | null {
    if (job.status.status !== 'running' || job.stageStart === null) {
      return null;
    }
    const current = STAGES.indexOf(job.status.stage);
    let total = 0;
    for (let i = current; i < STAGES.length; i += 1) {
      const perMp = this.times[STAGES[i]!];
      if (perMp === undefined) {
        return null;
      }
      total += perMp * job.megapixels;
    }
    return Math.max(0, total - (now - job.stageStart) / 1000);
  }

  private find(jobId: string): TrackedJob | undefined {
    return this.list.find((j) => j.jobId === jobId);
  }

  private patch(jobId: string, changes: Partial<TrackedJob>): void {
    this.list = this.list.map((j) => (j.jobId === jobId ? { ...j, ...changes } : j));
  }

  private update(jobId: string, status: JobStatus): void {
    const job = this.find(jobId);
    if (!job) {
      return;
    }
    const now = Date.now();
    const changes: Partial<TrackedJob> = { status };
    const previous = job.status;
    const stageChanged =
      previous.status === 'running' && (status.status !== 'running' || status.stage !== previous.stage);
    // Only stages seen from their start are timed.
    if (stageChanged && job.stageStart !== null && status.status !== 'cancelled' && status.status !== 'failed') {
      this.learn(previous.stage, (now - job.stageStart) / 1000 / job.megapixels);
    }
    if (status.status === 'running' && (previous.status !== 'running' || stageChanged)) {
      changes.stageStart = now;
      if (job.runStart === null && previous.status === 'queued') {
        changes.runStart = now;
      }
    }
    this.patch(jobId, changes);
  }

  private learn(stage: Stage, secondsPerMp: number): void {
    const old = this.times[stage];
    this.times = { ...this.times, [stage]: old === undefined ? secondsPerMp : 0.6 * old + 0.4 * secondsPerMp };
    saveSetting(TIMES_KEY, this.times);
  }
}

export const jobs = new Jobs();
