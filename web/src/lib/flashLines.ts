// The live preview highlights each line of its cards whose displayed text
// changed between two answers: not only the values that carry a cause tree,
// but the radius, the day, the moons line, the "frozen surface" tag…
// `changedLines` is the comparison; `flashLines` reads the lines off the
// rendered cards and marks the changed ones with `data-flash`.

const clean = (text: string | null | undefined) => (text ?? '').replace(/\s+/g, ' ').trim();

/** The keys whose text differs from the previous snapshot, or is new. */
export function changedLines(previous: ReadonlyMap<string, string>, next: ReadonlyMap<string, string>): Set<string> {
  const changed = new Set<string>();
  for (const [key, text] of next) if (previous.get(key) !== text) changed.add(key);
  return changed;
}

/**
 * The lines under `root`, by key: a `.facts` row by its card's place and its
 * label (`2:Mass`), a `[data-line]` element by that name. The text is what is
 * rendered, whitespace collapsed.
 */
export function collectLines(root: ParentNode): Map<string, { el: Element; text: string }> {
  const lines = new Map<string, { el: Element; text: string }>();
  root.querySelectorAll('[data-line]').forEach((el) => lines.set(`line:${el.getAttribute('data-line')}`, { el, text: clean(el.textContent) }));
  root.querySelectorAll('dl.facts').forEach((list, card) => {
    list.querySelectorAll(':scope > dd').forEach((dd, row) => {
      const dt = dd.previousElementSibling;
      const label = clean(dt?.tagName === 'DT' ? dt.textContent : null);
      lines.set(`${card}:${label || row}`, { el: dd, text: clean(dd.textContent) });
    });
  });
  return lines;
}

const texts = (lines: ReturnType<typeof collectLines>) => new Map([...lines].map(([key, { text }]) => [key, text] as const));

export interface FlashState {
  /** How many answers came: the highlight replays on each. */
  answers: number;
  /** Whether the answer is the same world as the one before; a new world flashes nothing. */
  continued: boolean;
}

/**
 * Svelte action for the container of the preview's cards: after each answer
 * it highlights the lines whose text changed since the previous one. The
 * highlight alternates between two names (`data-flash` a or b) because a CSS
 * animation restarts only when its name changes.
 */
export function flashLines(node: HTMLElement, state: FlashState) {
  let before = texts(collectLines(node));
  let seen = state.answers;
  return {
    update(next: FlashState) {
      if (next.answers === seen) return;
      seen = next.answers;
      // The cards are up to date by the time an action's update runs.
      const lines = collectLines(node);
      const after = texts(lines);
      const changed = next.continued ? changedLines(before, after) : new Set<string>();
      const name = next.answers % 2 === 0 ? 'a' : 'b';
      for (const [key, { el }] of lines) {
        if (changed.has(key)) el.setAttribute('data-flash', name);
        else el.removeAttribute('data-flash');
      }
      before = after;
    },
  };
}
