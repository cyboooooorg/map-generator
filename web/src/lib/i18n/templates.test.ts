// Guard: the Svelte templates show no literal text. Every word a user reads
// comes from the catalogs (`t`), so a new string cannot ship untranslated.
import { describe, expect, it } from 'vitest';

/** Every component's source, read by Vite. */
const SOURCES = import.meta.glob<string>('../../**/*.svelte', { query: '?raw', import: 'default', eager: true });

/** Literal text that is the same in every language: units and symbols. */
const ALLOWED = new Set(['px']);

/** Removes `{…}` expressions, nested ones included. */
function withoutExpressions(text: string): string {
  let previous = '';
  let current = text;
  while (current !== previous) {
    previous = current;
    current = current.replace(/\{[^{}]*\}/g, ' ');
  }
  return current;
}

/** Words of the markup outside expressions: text nodes and user-facing attributes. */
export function literalText(source: string): string[] {
  const markup = source
    .replace(/<script[\s\S]*?<\/script>/g, '')
    .replace(/<style[\s\S]*?<\/style>/g, '')
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/&[a-z]+;/g, ' ');
  const found: string[] = [];
  const plain = withoutExpressions(markup);
  for (const [, text] of plain.matchAll(/>([^<>]*)</g)) {
    found.push(...(text ?? '').split(/\s+/));
  }
  for (const [, text] of markup.matchAll(/\s(?:title|aria-label|placeholder|alt|label)="([^"{]*)"/g)) {
    found.push(...(text ?? '').split(/\s+/));
  }
  return found.filter((word) => /\p{L}{2,}/u.test(word) && !ALLOWED.has(word));
}

describe('templates', () => {
  it('detects literal text', () => {
    expect(literalText('<p>Hello {name}</p><b title="Help">{t.x}</b>')).toEqual(['Hello', 'Help']);
    expect(literalText('<script>const a = "<p>Not markup</p>";</script><p>{t.ok} px&nbsp;(#{n})</p>')).toEqual([]);
  });

  it('take every word from the catalogs', () => {
    const files = Object.entries(SOURCES);
    expect(files.length).toBeGreaterThan(10);
    const offenders = files.flatMap(([file, source]) => literalText(source).map((word) => `${file}: ${word}`));
    expect(offenders).toEqual([]);
  });
});
