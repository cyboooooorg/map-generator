// The WebGL2 context the 3D views draw with, made opaque. three.js always
// asks for an alpha drawing buffer (its `alpha` option only sets the clear
// alpha), so the pixels a shader writes with alpha below 1 (the
// atmosphere's halo, the dial's lines) would let the page behind the canvas
// show through: the theme's colours (see #372). Without an alpha channel,
// the page composites the canvas as it is drawn.

/** The drawing-buffer options the views choose. */
export interface OpaqueContextOptions {
  antialias: boolean;
  stencil?: boolean;
}

/**
 * A WebGL2 context on `canvas` without alpha, to pass to three's
 * `WebGLRenderer` as `context`. Throws, as three does, when the browser
 * gives none.
 */
export function opaqueContext(canvas: HTMLCanvasElement, options: OpaqueContextOptions): WebGL2RenderingContext {
  const gl = canvas.getContext('webgl2', {
    alpha: false,
    depth: true,
    stencil: options.stencil ?? false,
    antialias: options.antialias,
    premultipliedAlpha: true,
    preserveDrawingBuffer: false,
    powerPreference: 'default',
  });
  if (!gl) throw new Error('WebGL2 is not available: no context could be created.');
  return gl;
}
