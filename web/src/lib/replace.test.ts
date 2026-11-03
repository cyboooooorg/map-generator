import { describe, expect, it } from 'vitest';
import { ApiFailure } from './api/client';
import type { JobCreated } from './api/types/JobCreated';
import { replaceWorld, type ReplaceCalls } from './replace';

const JOB: JobCreated = { job_id: '1', world_id: 'terran-7' };

/** Fake API: `answers` are the outcomes of the successive createWorld calls. */
function fake(answers: (JobCreated | ApiFailure)[]): ReplaceCalls & { log: string[] } {
  const log: string[] = [];
  return {
    log,
    createWorld: () => {
      log.push('create');
      const next = answers.shift();
      if (next === undefined) {
        return Promise.reject(new TypeError('Failed to fetch'));
      }
      return next instanceof ApiFailure ? Promise.reject(next) : Promise.resolve(next);
    },
    deleteWorld: (id) => {
      log.push(`delete ${id}`);
      return Promise.resolve();
    },
  };
}

describe('replaceWorld', () => {
  it('deletes the old world only after the server validated the request', async () => {
    const calls = fake([new ApiFailure(409, 'world terran-7 already exists'), JOB]);
    await expect(replaceWorld('terran-7', { seed: 7 }, calls)).resolves.toEqual(JOB);
    expect(calls.log).toEqual(['create', 'delete terran-7', 'create']);
  });

  it('keeps the old world when the request is invalid', async () => {
    const calls = fake([new ApiFailure(400, 'unknown planet preset')]);
    await expect(replaceWorld('terran-7', { seed: 7 }, calls)).rejects.toThrow('unknown planet preset');
    expect(calls.log).toEqual(['create']);
  });

  it('keeps the old world when the server cannot be reached', async () => {
    const calls = fake([]);
    await expect(replaceWorld('terran-7', { seed: 7 }, calls)).rejects.toThrow('Failed to fetch');
    expect(calls.log).toEqual(['create']);
  });

  it('submits once when the old world is already gone', async () => {
    const calls = fake([JOB]);
    await expect(replaceWorld('terran-7', { seed: 7 }, calls)).resolves.toEqual(JOB);
    expect(calls.log).toEqual(['create']);
  });
});
