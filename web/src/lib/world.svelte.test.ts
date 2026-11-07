import { afterEach, describe, expect, it, vi } from 'vitest';
import { LoadedWorld } from './world.svelte';

const body = (id: string, parent?: string) => ({ id, kind: parent ? 'moon' : 'planet', parent, generated: false });
const files: Record<string, unknown> = {
  a: { focus: 'd', bodies: [body('d'), body('d-1', 'd'), body('d-2', 'd')] },
  b: { focus: 'd', bodies: [body('d'), body('d-1', 'd')] },
};
const flush = () => new Promise((r) => setTimeout(r, 0));

afterEach(() => vi.unstubAllGlobals());

describe('LoadedWorld.loadSystem', () => {
  it('drops the previous system while the next one loads', async () => {
    vi.stubGlobal('fetch', async (url: string) => {
      const id = decodeURIComponent(url.split('/')[3] ?? '');
      return { ok: true, json: async () => ({ system: files[id], names: { d: id } }) };
    });
    const world = new LoadedWorld();
    world.loadSystem('a', 'd-2');
    await flush();
    expect(world.shownSystemBody?.id).toBe('d-2');

    world.loadSystem('b', 'd');
    // The new id must never stand over the old system's bodies.
    expect(world.systemId).toBe('b');
    expect(world.system).toBeNull();
    expect(world.shownSystemBody).toBeNull();
    expect(world.systemNames).toEqual({});
    await flush();
    expect(world.system?.bodies).toHaveLength(2);
    expect(world.shownSystemBody?.id).toBe('d');
  });

  it('keeps the system when it is loaded again', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: true, json: async () => ({ system: files.a }) }));
    const world = new LoadedWorld();
    world.loadSystem('a', 'd-1');
    await flush();
    world.loadSystem('a');
    expect(world.system).not.toBeNull();
    await flush();
    expect(world.shownSystemBody?.id).toBe('d-1');
  });
});
