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

describe('LoadedWorld sizes', () => {
  const generated = { focus: 'd', bodies: [{ id: 'd', kind: 'planet', generated: true }] };
  const sizes = (displayed: [number, number], present: [number, number, number][]) => ({
    original: [2048, 1024],
    displayed,
    present: present.map(([width, height, revision]) => ({ width, height, revision })),
    pending: [],
    presets: [],
    max_size: 8192,
    max_pixels: 8192 * 4096,
  });
  const reply = (status: number, body?: unknown) => ({
    ok: status < 300,
    status,
    statusText: String(status),
    json: async () => body,
    text: async () => (body === undefined ? '' : JSON.stringify(body)),
  });
  /** A server with system `a`, its generated planet `d`, and the sizes `listing()` gives. */
  function serve(listing: () => ReturnType<typeof sizes>, requests: string[]) {
    vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
      requests.push(`${init?.method ?? 'GET'} ${url}${init?.body ? ` ${String(init.body)}` : ''}`);
      if (url === '/api/systems/a') return reply(200, { system: generated });
      if (url === '/api/systems/a/bodies/d/sizes') return reply(200, listing());
      if (url === '/api/systems/a/bodies/d/displayed-size') return reply(204);
      const file = /^\/systems\/a\/bodies\/d\/sizes\/(\d+)x(\d+)\/raw_data\/world\.json/.exec(url);
      if (file) {
        const [width, height] = [Number(file[1]), Number(file[2])];
        const there = listing().present.some((p) => p.width === width && p.height === height);
        return there ? reply(200, { format_version: 1e6, width, height }) : reply(404, { error: 'not found' });
      }
      return reply(404, { error: 'not found' });
    });
  }
  const settle = async () => {
    for (let i = 0; i < 20; i++) await flush();
  };

  it('loads a body at its displayed size', async () => {
    const requests: string[] = [];
    serve(() => sizes([4096, 2048], [[2048, 1024, 11], [4096, 2048, 22]]), requests);
    const world = new LoadedWorld();
    world.loadSystem('a', 'd');
    await settle();
    expect(world.size).toBe('4096x2048');
    expect(world.revision).toBe(22);
    expect(world.sizes?.displayed).toEqual([4096, 2048]);
    expect(world.meta?.width).toBe(4096);
    expect(requests.some((r) => r.startsWith('GET /systems/a/bodies/d/sizes/4096x2048/raw_data/world.json'))).toBe(true);
  });

  it('loads the displayed size again when the one shown was deleted elsewhere', async () => {
    const requests: string[] = [];
    let listed = 0;
    // The first listing still names 4096x2048, deleted before its files are read.
    serve(() => (listed++ === 0 ? sizes([4096, 2048], [[2048, 1024, 11], [4096, 2048, 22]]) : sizes([2048, 1024], [[2048, 1024, 11]])), requests);
    const world = new LoadedWorld();
    world.loadSystem('a', 'd');
    await settle();
    expect(world.size).toBe('2048x1024');
    expect(world.revision).toBe(11);
    expect(world.meta?.width).toBe(2048);
  });

  it('switches to another size and remembers it', async () => {
    const requests: string[] = [];
    serve(() => sizes([2048, 1024], [[2048, 1024, 11], [4096, 2048, 22]]), requests);
    const world = new LoadedWorld();
    world.loadSystem('a', 'd');
    await settle();
    expect(world.size).toBe('2048x1024');
    await world.switchSize('4096x2048');
    await settle();
    expect(world.size).toBe('4096x2048');
    expect(world.revision).toBe(22);
    expect(world.meta?.width).toBe(4096);
    expect(requests).toContain('PUT /api/systems/a/bodies/d/displayed-size {"width":4096,"height":2048}');
  });

  /** A request held until `release()`. */
  function gate() {
    let release = () => {};
    const opened = new Promise<void>((r) => (release = r));
    return { opened, release: () => release() };
  }

  it('never lets an older listing undo a newer switch', async () => {
    const requests: string[] = [];
    const both = sizes([2048, 1024], [[2048, 1024, 11], [4096, 2048, 22]]);
    // The listing taken before 4096x2048 was on disk, answered late.
    const stale = sizes([2048, 1024], [[2048, 1024, 11]]);
    let held: ReturnType<typeof gate> | null = null;
    serve(() => both, requests);
    const world = new LoadedWorld();
    world.loadSystem('a', 'd');
    await settle();
    const live = globalThis.fetch;
    vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
      if (url === '/api/systems/a/bodies/d/sizes' && held) {
        const wait = held;
        held = null;
        await wait.opened;
        return reply(200, stale);
      }
      return live(url, init);
    });
    held = gate();
    const late = held;
    const refreshed = world.refreshSizes();
    await flush();
    await world.switchSize('4096x2048');
    await settle();
    expect(world.size).toBe('4096x2048');
    late.release();
    await refreshed;
    await settle();
    expect(world.size).toBe('4096x2048');
    expect(world.sizes?.present.map((p) => p.width)).toEqual([2048, 4096]);
  });

  it('remembers the last of two quick switches', async () => {
    const requests: string[] = [];
    serve(() => sizes([2048, 1024], [[1024, 512, 5], [2048, 1024, 11], [4096, 2048, 22]]), requests);
    const world = new LoadedWorld();
    world.loadSystem('a', 'd');
    await settle();
    const live = globalThis.fetch;
    const puts: { body: string; done: ReturnType<typeof gate> }[] = [];
    vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
      if (init?.method === 'PUT') {
        const done = gate();
        puts.push({ body: String(init.body), done });
        await done.opened;
      }
      return live(url, init);
    });
    const first = world.switchSize('4096x2048');
    await flush();
    const second = world.switchSize('1024x512');
    const third = world.switchSize('2048x1024');
    await settle();
    // The second and third wait for the first PUT; the second is dropped.
    expect(puts.map((p) => p.body)).toEqual(['{"width":4096,"height":2048}']);
    puts[0]!.done.release();
    await settle();
    expect(puts.map((p) => p.body)).toEqual(['{"width":4096,"height":2048}', '{"width":2048,"height":1024}']);
    puts[1]!.done.release();
    await Promise.all([first, second, third]);
    expect(world.size).toBe('2048x1024');
  });

  it('lists the sizes again, once, when a file of the size shown is missing', async () => {
    const requests: string[] = [];
    let deleted = false;
    serve(
      () =>
        deleted
          ? sizes([2048, 1024], [[2048, 1024, 11]])
          : sizes([4096, 2048], [[2048, 1024, 11], [4096, 2048, 22]]),
      requests,
    );
    const world = new LoadedWorld();
    world.loadSystem('a', 'd');
    await settle();
    expect(world.size).toBe('4096x2048');
    const listings = () => requests.filter((r) => r === 'GET /api/systems/a/bodies/d/sizes').length;
    const before = listings();
    // Deleted from another tab: a tile answers 404, then another.
    deleted = true;
    world.fileMissing();
    world.fileMissing();
    expect(listings()).toBe(before + 1);
    await settle();
    expect(world.size).toBe('2048x1024');
    // Still there: a missing file of a size on disk is not listed again and again.
    const after = listings();
    world.fileMissing();
    world.fileMissing();
    await settle();
    world.fileMissing();
    await settle();
    expect(listings()).toBe(after + 1);
    expect(world.size).toBe('2048x1024');
  });
});
