import { describe, expect, it } from 'vitest';
import { opaqueContext } from './opaqueContext';

/** A canvas that records the attributes it is asked for. */
function fakeCanvas(gives: unknown): { canvas: HTMLCanvasElement; asked: unknown[] } {
  const asked: unknown[] = [];
  const canvas = {
    getContext: (name: string, attributes: unknown) => {
      asked.push([name, attributes]);
      return gives;
    },
  } as unknown as HTMLCanvasElement;
  return { canvas, asked };
}

describe('opaqueContext', () => {
  it('asks for a WebGL2 drawing buffer without alpha, with the views’ options', () => {
    const context = {};
    const { canvas, asked } = fakeCanvas(context);
    expect(opaqueContext(canvas, { antialias: true, stencil: true })).toBe(context);
    expect(asked).toEqual([['webgl2', expect.objectContaining({ alpha: false, antialias: true, stencil: true, depth: true })]]);
  });

  it('throws when the browser gives no context', () => {
    expect(() => opaqueContext(fakeCanvas(null).canvas, { antialias: false })).toThrow(/WebGL2/);
  });
});
