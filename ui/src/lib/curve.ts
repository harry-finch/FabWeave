// Courbe d'un paramètre `curve` (silhouette `[rayon, hauteur]`), pour la
// dessiner dans l'éditeur exactement comme le moteur la suit : même calcul
// que `crates/fabweave-core/src/curve.rs` (raccord lisse : interpolation
// cubique monotone de Fritsch et Carlson ; raccord droit : segments).

export type Pt = [number, number];

function slopes(p: Pt[]): number[] {
  const n = p.length;
  if (n < 2) return new Array(n).fill(0);
  const d = p.slice(1).map((q, k) => (q[0] - p[k][0]) / (q[1] - p[k][1]));
  const m = new Array(n).fill(0);
  m[0] = d[0];
  m[n - 1] = d[n - 2];
  for (let k = 1; k < n - 1; k++) m[k] = d[k - 1] * d[k] <= 0 ? 0 : (d[k - 1] + d[k]) / 2;
  for (let k = 0; k < n - 1; k++) {
    if (d[k] === 0) {
      m[k] = 0;
      m[k + 1] = 0;
      continue;
    }
    const a = m[k] / d[k];
    const b = m[k + 1] / d[k];
    const s = a * a + b * b;
    if (s > 9) {
      const tau = 3 / Math.sqrt(s);
      m[k] = tau * a * d[k];
      m[k + 1] = tau * b * d[k];
    }
  }
  return m;
}

/** Rayon à la hauteur `z` (prolongé à la verticale hors des points). */
export function curveAt(p: Pt[], smooth: boolean, z: number, m = smooth ? slopes(p) : []): number {
  if (p.length === 1 || z <= p[0][1]) return p[0][0];
  if (z >= p[p.length - 1][1]) return p[p.length - 1][0];
  let k = 0;
  while (p[k + 1][1] <= z) k++;
  const [r0, z0] = p[k];
  const [r1, z1] = p[k + 1];
  const h = z1 - z0;
  const t = (z - z0) / h;
  if (!smooth) return r0 + (r1 - r0) * t;
  const t2 = t * t;
  const t3 = t2 * t;
  return (2 * t3 - 3 * t2 + 1) * r0 + (t3 - 2 * t2 + t) * h * m[k] + (-2 * t3 + 3 * t2) * r1 + (t3 - t2) * h * m[k + 1];
}

/** Points du tracé, `n` hauteurs régulières du premier au dernier point. */
export function sample(p: Pt[], smooth: boolean, n = 120): Pt[] {
  if (p.length < 2) return p.slice();
  const m = smooth ? slopes(p) : [];
  const [z0, z1] = [p[0][1], p[p.length - 1][1]];
  return Array.from({ length: n + 1 }, (_, i) => {
    const z = z0 + ((z1 - z0) * i) / n;
    return [curveAt(p, smooth, z, m), z] as Pt;
  });
}

/** Deux courbes égales (valeur modifiée ou non). */
export const sameCurve = (a: Pt[], b: Pt[]) => a.length === b.length && a.every((q, i) => q[0] === b[i][0] && q[1] === b[i][1]);
