// Coloration du parcours (VIEW-07) : une grandeur par point (celle du
// mouvement qui y arrive), ramenée à son étendue sur les mouvements qui
// déposent de la matière, puis placée dans une rampe de jetons du thème.
// Hauteur (Z) : dégradé neutre `--path-low` → `--path-high` (par défaut, la
// vue reste calme) ; grandeurs : rampe violette `--scale-1…5` ; type de
// mouvement : impression neutre, pont et goutte en `--path-bridge`,
// `--path-blob`. Le hors-volume reste en couleur d'erreur, quel que soit le
// mode (posé par la vue).

import * as THREE from 'three';
import type { FrameData } from './types';

export type ColorMode = 'z' | 'order' | 'type' | 'speed' | 'flow' | 'width' | 'height' | 'fan';

export const COLOR_MODES: ColorMode[] = ['z', 'order', 'type', 'speed', 'flow', 'width', 'height', 'fan'];

/** Unité de la grandeur affichée (longueurs converties selon l'unité choisie). */
export const MODE_UNIT: Record<ColorMode, 'length' | 'mm/s' | 'mm³/s' | '%' | ''> = {
  z: 'length',
  order: '%',
  type: '',
  speed: 'mm/s',
  flow: 'mm³/s',
  width: 'length',
  height: 'length',
  fan: '%',
};

export type MoveType = 'print' | 'bridge' | 'blob';

export interface Legend {
  mode: ColorMode;
  /** Étendue des valeurs (grandeurs) ; égales : valeur unique. */
  min: number;
  max: number;
  /** Types présents (mode « type »). */
  types: MoveType[];
}

export interface Palette {
  low: THREE.Color;
  high: THREE.Color;
  ramp: THREE.Color[];
  print: THREE.Color;
  bridge: THREE.Color;
  blob: THREE.Color;
}

/** Mouvement qui dépose de la matière : impression, pont, goutte. */
export const deposits = (kind: number) => kind >= 1 && kind <= 3;

function value(f: FrameData, mode: ColorMode, i: number, n: number): number {
  switch (mode) {
    case 'z':
      return f.positions[i * 3 + 2];
    case 'order':
      return n > 1 ? (100 * i) / (n - 1) : 0;
    case 'speed':
      return f.speeds[i];
    case 'flow':
      return f.flows[i];
    case 'width':
      return f.widths[i];
    case 'height':
      return f.heights[i];
    case 'fan':
      return f.fans[i];
    default:
      return 0;
  }
}

/** Couleur de la rampe `stops` à la position `t` (0 à 1). */
function ramp(stops: THREE.Color[], t: number, out: THREE.Color) {
  const x = Math.min(Math.max(t, 0), 1) * (stops.length - 1);
  const k = Math.min(Math.floor(x), stops.length - 2);
  return out.copy(stops[k]).lerp(stops[k + 1], x - k);
}

/** Couleur de chaque point (r, g, b à la suite) et légende. */
export function colorize(f: FrameData, mode: ColorMode, p: Palette): { colors: Float32Array; legend: Legend } {
  const n = f.kinds.length;
  const colors = new Float32Array(n * 3);
  const legend: Legend = { mode, min: 0, max: 0, types: [] };
  const c = new THREE.Color();
  if (mode === 'type') {
    const seen = new Set<MoveType>();
    for (let i = 0; i < n; i++) {
      const k = f.kinds[i];
      const type: MoveType = k === 2 ? 'bridge' : k === 3 ? 'blob' : 'print';
      if (i > 0 && deposits(k)) seen.add(type);
      c.copy(type === 'bridge' ? p.bridge : type === 'blob' ? p.blob : p.print);
      colors.set([c.r, c.g, c.b], i * 3);
    }
    legend.types = (['print', 'bridge', 'blob'] as MoveType[]).filter((t) => seen.has(t));
    return { colors, legend };
  }
  let lo = Infinity;
  let hi = -Infinity;
  for (let i = 1; i < n; i++) {
    if (!deposits(f.kinds[i])) continue;
    const v = value(f, mode, i, n);
    lo = Math.min(lo, v);
    hi = Math.max(hi, v);
  }
  if (!Number.isFinite(lo)) lo = hi = 0;
  legend.min = lo;
  legend.max = hi;
  const span = hi - lo;
  const stops = mode === 'z' ? [p.low, p.high] : p.ramp;
  for (let i = 0; i < n; i++) {
    // Point d'arrivée d'un déplacement : il prend la valeur du segment
    // imprimé qui en part (en lignes, le segment garde une seule couleur).
    const own = deposits(f.kinds[i]) || mode === 'z' || mode === 'order';
    const next = i + 1 < n && deposits(f.kinds[i + 1]);
    const v = own ? value(f, mode, i, n) : next ? value(f, mode, i + 1, n) : lo;
    ramp(stops, span > 0 ? (v - lo) / span : 1, c);
    colors.set([c.r, c.g, c.b], i * 3);
  }
  return { colors, legend };
}
