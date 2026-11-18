export interface Box {
  left: number;
  right: number;
}

/**
 * Which edge of a glossed term its popover is anchored to: `start` grows
 * to the right of the term's left edge, `end` grows to the left of its
 * right edge. `end` when the popover would cross the right edge of
 * `bounds` and has room on the left.
 */
export function popoverAlign(term: Box, bounds: Box, popoverWidth: number): 'start' | 'end' {
  if (term.left + popoverWidth <= bounds.right) return 'start';
  return term.right - popoverWidth >= bounds.left ? 'end' : 'start';
}

/** The box of the nearest ancestor that clips its content, else the viewport. */
export function clippingBounds(el: Element): Box {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const o = getComputedStyle(p).overflowX;
    if (o !== 'visible') {
      const r = p.getBoundingClientRect();
      return { left: r.left, right: r.right };
    }
  }
  return { left: 0, right: document.documentElement.clientWidth };
}
