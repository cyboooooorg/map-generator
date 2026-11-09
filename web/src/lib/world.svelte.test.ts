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

  it('opens the planet the system was drawn around when asked, from the star view', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: true, json: async () => ({ system: files.a }) }));
    const world = new LoadedWorld();
    world.loadSystem('a');
    await flush();
    expect(world.subject).toBe('star');
    world.loadSystem('a', null, true);
    await flush();
    expect(world.subject).toBe('planet');
    expect(world.shownSystemBody?.id).toBe('d');
  });

  it('opens the star view from body=star', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: true, json: async () => ({ system: files.a }) }));
    const world = new LoadedWorld();
    world.loadSystem('x', 'star');
    expect(world.subject).toBe('star');
    expect(world.starMode).toBe('star');
    await flush();
    expect(world.subject).toBe('star');
    expect(world.starMode).toBe('star');
    // 'star' is never a body id: the system's planet is the body behind the view.
    expect(world.systemBody).not.toBe('star');
    expect(world.systemBody).toBe('d');
    expect(world.linkBody).toBe('star');
  });

  it('opens the system view without body', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: true, json: async () => ({ system: files.a }) }));
    const world = new LoadedWorld();
    world.loadSystem('x');
    await flush();
    expect(world.subject).toBe('star');
    expect(world.starMode).toBe('system');
    expect(world.linkBody).toBeNull();
  });

  it('resets the star mode with another system', async () => {
    vi.stubGlobal('fetch', async () => ({ ok: true, json: async () => ({ system: files.a }) }));
    const world = new LoadedWorld();
    world.loadSystem('x', 'star');
    await flush();
    expect(world.starMode).toBe('star');
    world.loadSystem('y');
    expect(world.starMode).toBe('system');
    await flush();
    expect(world.subject).toBe('star');
    expect(world.starMode).toBe('system');
    // A body asked for opens the planet view, in system mode once back at the star.
    world.loadSystem('z', 'd-1');
    expect(world.subject).toBe('planet');
    expect(world.starMode).toBe('system');
    await flush();
    expect(world.linkBody).toBe('d-1');
  });
});
