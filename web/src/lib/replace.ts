// Replacing a world in place: the server refuses to overwrite one, so the old
// world has to be deleted before the new one is submitted.
import { ApiFailure, api } from './api/client';
import type { JobCreated } from './api/types/JobCreated';
import type { WorldRequest } from './api/types/WorldRequest';

/** The API calls a replacement needs (injectable for the tests). */
export interface ReplaceCalls {
  createWorld: (request: WorldRequest) => Promise<JobCreated>;
  deleteWorld: (id: string) => Promise<void>;
}

/**
 * Submits `request`, which generates the existing world `id` again, deleting
 * the old world only once the request is known to be valid.
 *
 * The server checks a request fully (preset, parameters, map size) before it
 * looks for an existing world: a 409 answer to a first submission means the
 * request would be accepted, so the old world is deleted only then and the
 * request sent again. Any other failure (an invalid parameter, a map too
 * large, the server unreachable) leaves the old world in place.
 */
export async function replaceWorld(id: string, request: WorldRequest, calls: ReplaceCalls = api): Promise<JobCreated> {
  try {
    // Accepted: the old world was deleted meanwhile, nothing to replace.
    return await calls.createWorld(request);
  } catch (e) {
    if (!(e instanceof ApiFailure && e.status === 409)) {
      throw e;
    }
  }
  await calls.deleteWorld(id);
  return calls.createWorld(request);
}
