// Rendu en volume des boudins (VIEW-06, ADR-0003) : une instance par segment
// imprimé, un tube de section oblongue (stade W × H) construit dans le vertex
// shader à partir des deux extrémités, de la largeur et de la hauteur. Le
// boudin est sous la buse : son centre est une demi-hauteur sous le point.
// Chaque tube dépasse d'une demi-épaisseur à ses deux bouts pour couvrir les
// raccords, et se ferme par un fond plat. Éclairage simple (ambiant et deux lumières fixées à la caméra) :
// des intensités, aucune couleur ; la couleur de chaque instance vient des
// jetons du thème, comme en lignes.

import * as THREE from 'three';

/** Points de la section (cercle déformé en stade). */
const SIDES = 8;

const vertex = /* glsl */ `
attribute vec2 aSide;      // cos, sin de l'angle dans la section
attribute float aEnd;      // 0 : début du segment, 1 : fin
attribute float aCap;      // 0 : flanc ; -1 ou 1 : fond (normale le long du segment)
attribute vec3 iStart;
attribute vec3 iEnd;
attribute vec2 iSize;      // largeur, hauteur
attribute vec3 iColor;
varying vec3 vColor;
varying vec3 vNormal;
#include <clipping_planes_pars_vertex>

void main() {
  vec3 d = iEnd - iStart;
  float len = length(d);
  vec3 dir = len > 1e-6 ? d / len : vec3(1.0, 0.0, 0.0);
  vec3 side = cross(dir, vec3(0.0, 0.0, 1.0));
  side = length(side) > 1e-4 ? normalize(side) : vec3(1.0, 0.0, 0.0);
  vec3 up = normalize(cross(side, dir));
  float w = iSize.x;
  float h = iSize.y;
  float r = 0.5 * min(w, h);
  float straight = max(0.0, 0.5 * (w - h));
  // Stade : demi-cercles de rayon h/2 écartés de (w − h)/2 de part et d'autre.
  vec3 normal = aCap == 0.0 ? side * aSide.x + up * aSide.y : dir * aCap;
  vec3 offset = side * (sign(aSide.x) * straight + aSide.x * r) + up * (aSide.y * r);
  float ext = r;
  vec3 base = mix(iStart - dir * ext, iEnd + dir * ext, aEnd);
  vec3 p = base + offset - vec3(0.0, 0.0, 0.5 * h);
  vColor = iColor;
  vNormal = normalize(normalMatrix * normal);
  vec4 mvPosition = modelViewMatrix * vec4(p, 1.0);
  gl_Position = projectionMatrix * mvPosition;
  #include <clipping_planes_vertex>
}
`;

const fragment = /* glsl */ `
varying vec3 vColor;
varying vec3 vNormal;
#include <clipping_planes_pars_fragment>

void main() {
  #include <clipping_planes_fragment>
  vec3 n = normalize(vNormal);
  // Lumières dans le repère de la caméra : principale en haut à gauche,
  // secondaire en face ; ambiant pour que les faces cachées restent lisibles.
  float key = max(dot(n, normalize(vec3(-0.4, 0.7, 0.6))), 0.0);
  float fill = max(dot(n, vec3(0.0, 0.0, 1.0)), 0.0);
  float light = 0.42 + 0.48 * key + 0.22 * fill;
  gl_FragColor = vec4(vColor * light, 1.0);
  #include <colorspace_fragment>
}
`;

/** Géométrie de base : un tube de SIDES faces, fermé par deux fonds. */
function baseGeometry(): THREE.InstancedBufferGeometry {
  const g = new THREE.InstancedBufferGeometry();
  const side: number[] = [];
  const end: number[] = [];
  const cap: number[] = [];
  const vertex = (c: number, s: number, e: number, k: number) => {
    side.push(c, s);
    end.push(e);
    cap.push(k);
  };
  // Flancs : deux anneaux (début, fin).
  for (let e = 0; e < 2; e++) {
    for (let k = 0; k < SIDES; k++) {
      const a = (k / SIDES) * Math.PI * 2;
      vertex(Math.cos(a), Math.sin(a), e, 0);
    }
  }
  const index: number[] = [];
  for (let k = 0; k < SIDES; k++) {
    const k1 = (k + 1) % SIDES;
    index.push(k, k1, SIDES + k, k1, SIDES + k1, SIDES + k);
  }
  // Fonds : centre et anneau, normale vers l'extérieur du segment.
  for (let e = 0; e < 2; e++) {
    const first = side.length / 2;
    const dir = e === 0 ? -1 : 1;
    vertex(0, 0, e, dir);
    for (let k = 0; k < SIDES; k++) {
      const a = (k / SIDES) * Math.PI * 2;
      vertex(Math.cos(a), Math.sin(a), e, dir);
    }
    for (let k = 0; k < SIDES; k++) {
      const k1 = (k + 1) % SIDES;
      index.push(first, first + 1 + k, first + 1 + k1);
    }
  }
  const count = side.length / 2;
  g.setAttribute('aSide', new THREE.Float32BufferAttribute(side, 2));
  g.setAttribute('aEnd', new THREE.Float32BufferAttribute(end, 1));
  g.setAttribute('aCap', new THREE.Float32BufferAttribute(cap, 1));
  // `position` n'est pas lu par le shader, mais three.js en attend un.
  g.setAttribute('position', new THREE.Float32BufferAttribute(new Float32Array(count * 3), 3));
  g.setIndex(index);
  return g;
}

// `clipping` : coupe en Z de la vue (VIEW-08), plans donnés par la vue.
export const beadMaterial = new THREE.ShaderMaterial({ vertexShader: vertex, fragmentShader: fragment, side: THREE.DoubleSide, clipping: true });

/** Boudins des segments `segments` (indices du point d'arrivée), couleur de chacun. */
export function beadMesh(
  positions: Float32Array,
  widths: Float32Array,
  heights: Float32Array,
  segments: number[],
  color: (i: number, out: THREE.Color) => void,
): THREE.Mesh {
  const n = segments.length;
  const start = new Float32Array(n * 3);
  const end = new Float32Array(n * 3);
  const size = new Float32Array(n * 2);
  const col = new Float32Array(n * 3);
  const c = new THREE.Color();
  for (let k = 0; k < n; k++) {
    const i = segments[k];
    start.set(positions.subarray((i - 1) * 3, i * 3), k * 3);
    end.set(positions.subarray(i * 3, i * 3 + 3), k * 3);
    size[k * 2] = widths[i];
    size[k * 2 + 1] = heights[i];
    color(i, c);
    col[k * 3] = c.r;
    col[k * 3 + 1] = c.g;
    col[k * 3 + 2] = c.b;
  }
  const g = baseGeometry();
  g.instanceCount = n;
  g.setAttribute('iStart', new THREE.InstancedBufferAttribute(start, 3));
  g.setAttribute('iEnd', new THREE.InstancedBufferAttribute(end, 3));
  g.setAttribute('iSize', new THREE.InstancedBufferAttribute(size, 2));
  g.setAttribute('iColor', new THREE.InstancedBufferAttribute(col, 3));
  const mesh = new THREE.Mesh(g, beadMaterial);
  // Les sommets sont placés par le shader : pas d'élimination hors champ.
  mesh.frustumCulled = false;
  return mesh;
}
