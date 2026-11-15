import { describe, expect, it } from 'vitest';
import { changedLines } from './flashLines';

describe('changedLines', () => {
  const before = new Map([
    ['0:Radius', '1 R⊕'],
    ['1:Gravity', '1 g'],
    ['line:class', 'Rocky world'],
  ]);

  it('reports the lines whose text changed, whatever their kind', () => {
    const after = new Map([
      ['0:Radius', '0.53 R⊕'],
      ['1:Gravity', '1 g'],
      ['line:class', 'Rocky world, frozen surface'],
    ]);
    expect([...changedLines(before, after)].sort()).toEqual(['0:Radius', 'line:class']);
  });

  it('reports a line that appeared, not one that vanished', () => {
    const after = new Map([...before, ['2:Moons', '2']]);
    expect([...changedLines(before, after)]).toEqual(['2:Moons']);
    expect(changedLines(after, before).size).toBe(0);
  });

  it('reports nothing when nothing changed', () => {
    expect(changedLines(before, new Map(before)).size).toBe(0);
  });
});
