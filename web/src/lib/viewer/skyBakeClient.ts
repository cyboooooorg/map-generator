// The main thread's side of the sky bake: asks the worker for a sky's parts
// (skyBake.worker.ts), keeps what came back per (look, exposure), and hands
// the parts to whoever subscribed, at once when they are already there.
//
// One module-level worker serves every backdrop (the three views may show
// the same sky): `acquireSkyBaker` / `releaseSkyBaker` count the live
// backdrops and the worker ends, with its cache, when the last one goes.
// Where there is no Worker (vitest), the same parts are baked synchronously
// inside `subscribeSkyBake`, so the code path and the results are the same.
//
// The cache holds the last four skies' parts (the eye and the photo bake of
// two skies): switching the exposure back and forth is instant. Parts that
// came back are never touched again here; the backdrop builds its textures
// from them.

import type { SkyExposure } from './skyClouds';
import type { SkyLook } from './skyLook';
import { skyBakeParts, type SkyPart } from './skyBake';
import type { BakeRequest } from './skyBake.worker';

type Listener = (part: SkyPart) => void;

interface Entry {
  id: number;
  /** The parts that arrived, in order, replayed to a late subscriber. */
  parts: SkyPart[];
  done: boolean;
  requested: boolean;
  listeners: Set<Listener>;
}

const MAX_ENTRIES = 4;
const entries = new Map<string, Entry>();
let worker: Worker | null = null;
let users = 0;
let nextId = 1;

function keyOf(look: SkyLook, exposure: SkyExposure): string {
  return [exposure, look.seed, look.brightness, look.bandWidth, look.bulge, look.coreSize, look.coreBrightness, ...look.north, ...look.centre].join('|');
}

function post(request: BakeRequest): void {
  worker?.postMessage(request);
}

function onMessage(event: MessageEvent): void {
  const message = event.data as { id: number; part: string };
  const entry = [...entries.values()].find((e) => e.id === message.id);
  if (!entry) return;
  if (message.part === 'done') {
    entry.done = true;
    return;
  }
  const part = message as unknown as SkyPart;
  entry.parts.push(part);
  for (const listener of [...entry.listeners]) listener(part);
}

function ensureWorker(): Worker | null {
  if (worker || typeof Worker === 'undefined') return worker;
  worker = new Worker(new URL('./skyBake.worker.ts', import.meta.url), { type: 'module' });
  worker.onmessage = onMessage;
  return worker;
}

function trim(): void {
  for (const [key, entry] of entries) {
    if (entries.size <= MAX_ENTRIES) return;
    if (entry.listeners.size === 0 && entry.done) entries.delete(key);
  }
}

/** A backdrop is alive: the worker may start. */
export function acquireSkyBaker(): void {
  users++;
}

/** A backdrop is gone; the last one takes the worker and the cache with it. */
export function releaseSkyBaker(): void {
  users = Math.max(0, users - 1);
  if (users > 0) return;
  worker?.terminate();
  worker = null;
  entries.clear();
}

/**
 * Calls `listener` with each part of the look's bake in an exposure (the
 * ones already baked first, at once). Returns the unsubscribe function: the
 * last subscriber of an unfinished bake cancels it.
 */
export function subscribeSkyBake(look: SkyLook, exposure: SkyExposure, listener: Listener): () => void {
  const key = keyOf(look, exposure);
  let entry = entries.get(key);
  if (entry) {
    // Most recently used last.
    entries.delete(key);
    entries.set(key, entry);
  } else {
    entry = { id: nextId++, parts: [], done: false, requested: false, listeners: new Set() };
    entries.set(key, entry);
    trim();
  }
  const own = entry;
  own.listeners.add(listener);
  for (const part of [...own.parts]) listener(part);

  if (!own.requested) {
    own.requested = true;
    if (ensureWorker()) {
      post({ id: own.id, look, exposure });
    } else {
      // No worker: the same parts, baked here.
      for (const part of skyBakeParts(look, exposure)) {
        own.parts.push(part);
        for (const l of [...own.listeners]) l(part);
      }
      own.done = true;
    }
  }

  return () => {
    own.listeners.delete(listener);
    if (own.listeners.size === 0 && !own.done) {
      post({ cancel: own.id });
      if (entries.get(key) === own) entries.delete(key);
    }
  };
}
