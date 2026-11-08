// The rocks of a giant's rings round a ring moon (see lib/ringRocks.ts):
// one instance of a low rock per slot of the window, placed, sized and
// lit here from its row and cell; beyond them, the ring's grain, a point
// a pixel wide per slot of a coarser window. Positions are in radii of
// the moon.
import { CELL_KM, GRAIN_CELL_KM, GRAIN_REACH_KM, GRAIN_ROWS, GRAINS_PER_CELL, ROCK_REACH_KM, ROCKS_PER_CELL, ROWS } from '../ringRocks';
import { ringletsGlsl, skyRingDensityGlsl } from './ringShaders';

// What the rocks and the grain share: a window of a lattice fixed in the
// ring (`rows` rows), the hash of a slot, the gaps, and the place of a
// slot from the foot of the moon's orbit.
const latticeGlsl = (rows: number) => /* glsl */ `
  // Its slot: its row in the window (0 innermost), its cell from the
  // moon's and its rank in the cell.
  attribute vec3 aSlot;
  // Each row: its middle's offset from the moon's orbit (km), the cell the
  // moon is in, how far into it (0 to 1), the cells round its circle.
  uniform vec4 uRows[${rows}];
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
  uniform float uPixel;        // radians a drawn pixel spans
  uniform float uFar;          // the camera's far plane
  uniform float uFade;         // the globe's morph: rocks belong to the globe
  ${skyRingDensityGlsl}
  ${ringletsGlsl}

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

  // The same slot whenever its cell comes back: a hash of its row, cell
  // and rank in the lattice (salted, so that two lattices differ).
  uint slotSeed(vec4 row, uint salt) {
    float cell = mod(row.y + aSlot.y, row.w);
    return pcg(uint(uFirstRow + aSlot.x) ^ pcg(uint(cell) ^ pcg(uint(aSlot.z) ^ salt)));
  }

  // Whether radius r (giant radii) lies within a gap. A gap's edges are
  // sharp (as Encke's and Keeler's are): ringDensity's soft edges suit
  // the sheet seen from afar, but would leave rocks a few km from the
  // moon at this scale.
  bool inGap(float r) {
    for (int k = 0; k < 4; k++) {
      if (float(k) >= uRingGapCount) break;
      if (abs(r - uRingGaps[k].x) < 0.5 * uRingGaps[k].y) return true;
    }
    return false;
  }

  // The ring's optical depth at r, out of its gaps, with the ringlets of
  // the giant's own view (ringlets, at the exact radius: a rock is
  // resolved where it is drawn), over their mean (0.675) so that the
  // rocks are as many on the whole as before.
  float opticalDepth(float r) {
    if (inGap(r)) return 0.0;
    return -log(1.0 - min(ringDensity(r) * ringlets(r, 0.0) / 0.675, 0.95));
  }

  // The place of a slot at rKm from the giant, \`cells\` cells ahead of the
  // moon along its row, upKm off the plane: from the foot of the moon's
  // orbit, so that the giant's distance cancels out before 32-bit floats
  // see it (see rockPlace).
  vec3 slotPlace(vec4 row, float rKm, float cells, float upKm) {
    float along = cells * 6.2831853 / row.w;
    float halfSine = sin(0.5 * along);
    float outKm = (rKm - uAKm) * cos(along) - 2.0 * uAKm * halfSine * halfSine;
    float aheadKm = rKm * sin(along);
    return uFoot + (outKm * uOut + aheadKm * uAhead + upKm * uNormal) / uMoonKm;
  }

  // Sunlit unless the giant stands between it and the sun.
  float sunlit(vec3 centre) {
    vec3 toGiant = uGiantCentre - centre;
    float b = dot(toGiant, uSunDir);
    float miss = dot(toGiant, toGiant) - b * b;
    return b > 0.0 && miss < uGiantRadius * uGiantRadius ? 0.0 : 1.0;
  }

  // Beyond the far plane (which bounds the globe's depth) its depth is
  // kept just short of it, nearer ones in front of farther ones.
  void holdDepth(float dist) {
    if (dist > 0.9 * uFar) {
      gl_Position.z = (1.0 - 1e-5 * uFar / dist) * gl_Position.w;
    }
  }
`;

export const ringRockVertex = /* glsl */ `
  ${latticeGlsl(ROWS)}
  uniform float uBoulderM;     // the largest boulders' diameter, m
  uniform float uScale;        // drawn size over true size (1: true)
  varying vec3 vNormal;
  varying float vAlpha;
  varying float vLight;
  varying float vAlbedo;

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
    uint seed = slotSeed(row, 0u);
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
    if (keep >= opticalDepth(rKm / uGiantKm) / 1.5) {
      hide();
      return;
    }
    // Ahead of the moon by this many cells; the ring's thickness: tens of metres.
    float cells = aSlot.y + place - row.z;
    vec3 centre = slotPlace(row, rKm, cells, (height - 0.5) * 0.02);
    // The window is round: fading out towards its reach from the moon.
    float fromMoonKm = length(centre) * uMoonKm;
    float edge = 1.0 - smoothstep(${(0.7 * ROCK_REACH_KM).toFixed(1)}, ${ROCK_REACH_KM.toFixed(1)}, fromMoonKm);

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

    vLight = sunlit(centre);
    vNormal = turn * (normal / shape);
    vec3 world = centre + turn * (position * shape) * drawn;
    gl_Position = projectionMatrix * viewMatrix * vec4(world, 1.0);
    holdDepth(dist);
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

// The mean cross-section of a rock, over the square of the largest one's
// diameter: pi/4 E[D^2] on the power law of their sizes (D from a fifth
// of the largest to it, N(>D) ~ D^-2), 2 ln 5 / 24 of it, times the
// share of its circumscribed disk a rock covers on average: an
// icosahedron's mean shadow (a quarter of its area) is 0.76 of that
// disk, flattened by its shape (see ringRockVertex) to about 0.63 of it.
const ROCK_SECTION = (Math.PI / 4) * ((2 * Math.log(5)) / 24) * 0.76 * 0.63;
// The grain's cross-section per point, over a rock's: the rocks' slots
// per km^2 over the grain's.
const GRAIN_SECTION = (ROCKS_PER_CELL / CELL_KM ** 2 / (GRAINS_PER_CELL / GRAIN_CELL_KM ** 2)) * ROCK_SECTION;

// The ring's grain beyond the rocks: a point a pixel wide per slot of the
// coarser window, as many as the ring's optical depth, fading in where
// the rocks fade out, thinning out with distance (as many to a pixel as
// at the rocks' reach) and fading out into the ring's sheet. Each point
// stands for the rocks' cross-section over its share of the ring, as
// faint as that over its pixel: the light per km^2 is the rocks' at
// their reach, whatever the camera's distance, and as bright as a rock on
// average, so that boulders give way to grain and grain to the sheet
// with no edge.
export const ringGrainVertex = /* glsl */ `
  ${latticeGlsl(GRAIN_ROWS)}
  uniform float uPointSize;    // a CSS pixel, in drawn pixels
  uniform vec3 uColour;        // linear colour of the ring particles
  uniform float uBoulderM;     // the largest boulders' diameter, m
  uniform float uScale;        // drawn size over true size (1: true)
  varying vec3 vColour;
  varying float vAlpha;

  void hide() {
    vAlpha = 0.0;
    gl_PointSize = 0.0;
    gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
  }

  void main() {
    vec4 row = uRows[int(aSlot.x)];
    uint seed = slotSeed(row, 0x9e3779b9u);
    float across = unit(seed);
    float keep = unit(seed);
    float place = unit(seed);
    float height = unit(seed);
    float thin = unit(seed);

    float rKm = uAKm + row.x + (across - 0.5) * uCellKm;
    if (keep >= opticalDepth(rKm / uGiantKm) / 1.5) {
      hide();
      return;
    }
    float cells = aSlot.y + place - row.z;
    vec3 centre = slotPlace(row, rKm, cells, (height - 0.5) * 0.02);
    float fromMoonKm = length(centre) * uMoonKm;
    // Beyond the rocks, as many to the pixel as at their reach.
    float reach = ${ROCK_REACH_KM.toFixed(1)};
    float kept = min(1.0, (reach * reach) / (fromMoonKm * fromMoonKm));
    if (thin >= kept) {
      hide();
      return;
    }
    // Its cross-section (km^2, the thinned-out ones' too) over its pixel's.
    float largestKm = uBoulderM * uScale / 1000.0;
    float section = ${GRAIN_SECTION.toFixed(5)} * largestKm * largestKm / kept;
    float dist = length(centre - cameraPosition);
    float pointKm = uPointSize * uPixel * dist * uMoonKm;
    vAlpha = uFade
      * min(1.0, section / (pointKm * pointKm))
      * smoothstep(${(0.7 * ROCK_REACH_KM).toFixed(1)}, reach, fromMoonKm)
      * (1.0 - smoothstep(${(0.6 * GRAIN_REACH_KM).toFixed(1)}, ${GRAIN_REACH_KM.toFixed(1)}, fromMoonKm));
    if (vAlpha < 0.01) {
      hide();
      return;
    }
    // A rock's light on average (see ringRockFragment): its mean albedo,
    // and a Lambert sphere's light over its disk at the phase angle
    // between the sun and the camera, 2/3 (sin a + (pi - a) cos a) / pi;
    // dark in the giant's shadow.
    float phase = acos(clamp(dot(normalize(cameraPosition - centre), uSunDir), -1.0, 1.0));
    float lambert = (2.0 / 3.0) * (sin(phase) + (3.1415927 - phase) * cos(phase)) / 3.1415927;
    vColour = uColour * 0.95 * (0.03 + 0.97 * lambert * sunlit(centre));
    gl_Position = projectionMatrix * viewMatrix * vec4(centre, 1.0);
    gl_PointSize = uPointSize;
    holdDepth(dist);
  }
`;

export const ringGrainFragment = /* glsl */ `
  varying vec3 vColour;
  varying float vAlpha;
  void main() {
    gl_FragColor = vec4(vColour, vAlpha);
    #include <colorspace_fragment>
  }
`;

/** Points drawn: every slot of the grain's window. */
export const RING_GRAIN_SLOTS = GRAIN_ROWS * GRAIN_ROWS * GRAINS_PER_CELL;
