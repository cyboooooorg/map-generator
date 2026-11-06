// The rocks of a giant's rings round a ring moon (see lib/ringRocks.ts):
// one instance of a low rock per slot of the window, placed, sized and
// lit here from its row and cell. Positions are in radii of the moon.
import { ROCKS_PER_CELL, ROWS, WINDOW } from '../ringRocks';
import { skyRingDensityGlsl } from './ringShaders';

export const ringRockVertex = /* glsl */ `
  // Its slot: its row in the window (0 innermost), its cell from the
  // moon's (-WINDOW to WINDOW) and its rank in the cell.
  attribute vec3 aSlot;
  // Each row: its middle's offset from the moon's orbit (km), the cell the
  // moon is in, how far into it (0 to 1), the cells round its circle.
  uniform vec4 uRows[${ROWS}];
  uniform float uFirstRow;     // lattice number of row 0
  uniform float uCellKm;
  uniform float uAKm;          // the moon's semi-major axis
  uniform float uGiantKm;      // the giant's radius (the rings' unit)
  uniform float uMoonKm;       // the moon's radius (the scene's unit)
  uniform vec3 uFoot;          // the ring plane under the moon
  uniform vec3 uOut;
  uniform vec3 uAhead;
  uniform vec3 uNormal;
  uniform vec3 uGiantCentre;
  uniform float uGiantRadius;
  uniform vec3 uSunDir;
  uniform float uBoulderM;     // the largest boulders' diameter, m
  uniform float uScale;        // drawn size over true size (1: true)
  uniform float uPixel;        // radians a drawn pixel spans
  uniform float uFar;          // the camera's far plane
  uniform float uFade;         // the globe's morph: rocks belong to the globe
  ${skyRingDensityGlsl}
  varying vec3 vNormal;
  varying float vAlpha;
  varying float vLight;
  varying float vAlbedo;

  // A 32-bit integer hash (PCG, Jarzynski and Olano 2020), as a float in [0, 1).
  uint pcg(uint v) {
    uint state = v * 747796405u + 2891336453u;
    uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
  }
  float unit(inout uint seed) {
    seed = pcg(seed);
    return float(seed) / 4294967296.0;
  }

  // A rotation from three uniform numbers (Shoemake's uniform quaternion).
  mat3 rotation(float u1, float u2, float u3) {
    float a = sqrt(1.0 - u1);
    float b = sqrt(u1);
    vec4 q = vec4(a * sin(6.2831853 * u2), a * cos(6.2831853 * u2), b * sin(6.2831853 * u3), b * cos(6.2831853 * u3));
    float x = q.x, y = q.y, z = q.z, w = q.w;
    return mat3(
      1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + z * w), 2.0 * (x * z - y * w),
      2.0 * (x * y - z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + x * w),
      2.0 * (x * z + y * w), 2.0 * (y * z - x * w), 1.0 - 2.0 * (x * x + y * y)
    );
  }

  void hide() {
    vAlpha = 0.0;
    gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
  }

  void main() {
    vec4 row = uRows[int(aSlot.x)];
    float cell = mod(row.y + aSlot.y, row.w);
    // The same rock whenever its cell comes back: a hash of its row,
    // cell and rank in the lattice.
    uint seed = pcg(uint(uFirstRow + aSlot.x) ^ pcg(uint(cell) ^ pcg(uint(aSlot.z))));
    float across = unit(seed);
    float keep = unit(seed);
    float place = unit(seed);
    float size = unit(seed);
    float height = unit(seed);
    vec3 shape = vec3(1.0, mix(0.55, 1.0, unit(seed)), mix(0.4, 0.85, unit(seed)));
    mat3 turn = rotation(unit(seed), unit(seed), unit(seed));
    vAlbedo = mix(0.75, 1.15, unit(seed));

    // As many as the ring's optical depth there, none in its gaps.
    float rKm = uAKm + row.x + (across - 0.5) * uCellKm;
    float r = rKm / uGiantKm;
    // A gap's edges are sharp (as Encke's and Keeler's are): none inside
    // its width. ringDensity's soft edges suit the sheet seen from afar,
    // but would leave rocks a few km from the moon at this scale.
    for (int k = 0; k < 4; k++) {
      if (float(k) >= uRingGapCount) break;
      if (abs(r - uRingGaps[k].x) < 0.5 * uRingGaps[k].y) {
        hide();
        return;
      }
    }
    float density = ringDensity(r);
    float depth = -log(1.0 - min(density, 0.95));
    if (keep >= depth / 1.5) {
      hide();
      return;
    }
    // Ahead of the moon by this many cells (fading out at the window's ends).
    float cells = aSlot.y + place - row.z;
    float edge = 1.0 - smoothstep(${WINDOW - 3}.0, ${WINDOW}.0, abs(cells));
    float along = cells * 6.2831853 / row.w;
    // From the foot of the moon's orbit, so that the giant's distance
    // cancels out before 32-bit floats see it (see rockPlace).
    float halfSine = sin(0.5 * along);
    float outKm = (rKm - uAKm) * cos(along) - 2.0 * uAKm * halfSine * halfSine;
    float aheadKm = rKm * sin(along);
    // The ring's thickness: tens of metres.
    float upKm = (height - 0.5) * 0.02;
    vec3 centre = uFoot + (outKm * uOut + aheadKm * uAhead + upKm * uNormal) / uMoonKm;

    // Its size on the power law of ring particles (N(>s) ~ s^-2), among
    // the largest of its cell: a fifth of the largest boulders' to theirs.
    float lo = 0.2 * uBoulderM;
    float hi = uBoulderM;
    float diameterM = 1.0 / sqrt(1.0 / (lo * lo) - size * (1.0 / (lo * lo) - 1.0 / (hi * hi)));
    float radius = 0.5 * diameterM * uScale / 1000.0 / uMoonKm;
    // Half a pixel across at least, faint as its true size makes it.
    float dist = length(centre - cameraPosition);
    float drawn = max(radius, 0.5 * uPixel * dist);
    vAlpha = uFade * edge * min(1.0, (radius / drawn) * (radius / drawn));
    if (vAlpha < 0.01) {
      hide();
      return;
    }

    // Sunlit unless the giant stands between it and the sun.
    vec3 toGiant = uGiantCentre - centre;
    float b = dot(toGiant, uSunDir);
    float miss = dot(toGiant, toGiant) - b * b;
    vLight = b > 0.0 && miss < uGiantRadius * uGiantRadius ? 0.0 : 1.0;
    vNormal = turn * (normal / shape);
    vec3 world = centre + turn * (position * shape) * drawn;
    gl_Position = projectionMatrix * viewMatrix * vec4(world, 1.0);
    // Beyond the far plane (which bounds the globe's depth) its depth is
    // kept just short of it, nearer rocks in front of farther ones.
    if (dist > 0.9 * uFar) {
      gl_Position.z = (1.0 - 1e-5 * uFar / dist) * gl_Position.w;
    }
  }
`;

export const ringRockFragment = /* glsl */ `
  uniform vec3 uColour;        // linear colour of the ring particles
  uniform vec3 uSunDir;
  varying vec3 vNormal;
  varying float vAlpha;
  varying float vLight;
  varying float vAlbedo;
  void main() {
    float lambert = max(dot(normalize(vNormal), uSunDir), 0.0);
    vec3 colour = uColour * vAlbedo * (0.03 + 0.97 * lambert * vLight);
    gl_FragColor = vec4(colour, vAlpha);
    #include <colorspace_fragment>
  }
`;

/** Instances drawn: every slot of the window. */
export const RING_ROCK_SLOTS = ROWS * ROWS * ROCKS_PER_CELL;
