// Vignettes du catalogue : parcours d'un modèle aux valeurs par défaut,
// dessiné en Canvas 2D en vue oblique (même direction que la vue 3D),
// dégradé selon la hauteur. Couleurs : jetons `--view-bg`, `--path-low`,
// `--path-high`, relus à chaque thème (le cache est par thème).

const TOKENS = ['view-bg', 'path-low', 'path-high'] as const;

function rgb(css: string): [number, number, number] {
  const c = document.createElement('canvas').getContext('2d')!;
  c.fillStyle = css;
  const hex = c.fillStyle as string;
  if (hex.startsWith('#')) return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [number, number, number];
  const m = hex.match(/\d+/g) ?? ['128', '128', '128'];
  return [+m[0], +m[1], +m[2]];
}

/** Image PNG (data URL) de `size` px CSS ; `pts` : x, y, z, nature à la suite.
 * Chaîne vide si le parcours est vide (le script a échoué). */
export function renderThumb(pts: Float32Array, size: number): string {
  const cs = getComputedStyle(document.documentElement);
  const col = Object.fromEntries(TOKENS.map((k) => [k, cs.getPropertyValue(`--${k}`).trim() || '#808080']));
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = Math.round(size * dpr);
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = col['view-bg'];
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  const n = pts.length / 4;
  if (n < 2) return '';
  // Projection oblique : vue de devant à gauche, en hauteur (comme la vue 3D).
  const proj = (i: number): [number, number] => {
    const x = pts[i * 4], y = pts[i * 4 + 1], z = pts[i * 4 + 2];
    return [x * 0.91 + y * 0.41, -z * 0.8 + (y * 0.91 - x * 0.41) * 0.45];
  };
  let [u0, v0, u1, v1, z0, z1] = [Infinity, Infinity, -Infinity, -Infinity, Infinity, -Infinity];
  for (let i = 0; i < n; i++) {
    const [u, v] = proj(i);
    u0 = Math.min(u0, u); u1 = Math.max(u1, u);
    v0 = Math.min(v0, v); v1 = Math.max(v1, v);
    z0 = Math.min(z0, pts[i * 4 + 2]); z1 = Math.max(z1, pts[i * 4 + 2]);
  }
  const pad = 0.1 * canvas.width;
  const k = (canvas.width - 2 * pad) / Math.max(u1 - u0, v1 - v0, 1e-6);
  const ou = pad + ((canvas.width - 2 * pad) - (u1 - u0) * k) / 2 - u0 * k;
  const ov = pad + ((canvas.height - 2 * pad) - (v1 - v0) * k) / 2 - v0 * k;
  const lo = rgb(col['path-low']), hi = rgb(col['path-high']);
  // Huit tranches de hauteur, une couleur chacune : un tracé par tranche.
  // Traits fins et semi-transparents : sur un parcours dense, c'est le
  // recouvrement des tours qui dessine le relief (sinon la silhouette sature).
  const BUCKETS = 8;
  ctx.lineWidth = dpr * 0.6;
  ctx.globalAlpha = 0.45;
  ctx.lineJoin = 'round';
  for (let b = 0; b < BUCKETS; b++) {
    const t = b / (BUCKETS - 1);
    ctx.strokeStyle = `rgb(${lo.map((c, j) => Math.round(c + (hi[j] - c) * t)).join(',')})`;
    ctx.beginPath();
    for (let i = 1; i < n; i++) {
      if (pts[i * 4 + 3] === 0) continue; // déplacement : rien
      const zb = Math.min(BUCKETS - 1, Math.floor(((pts[i * 4 + 2] - z0) / Math.max(z1 - z0, 1e-6)) * BUCKETS));
      if (zb !== b) continue;
      if (pts[i * 4 + 3] === 3) {
        // Goutte : disque d'environ 1,6 mm (la vignette ne connaît pas sa taille).
        const [u, v] = proj(i);
        const r = Math.max(0.8 * k, dpr);
        ctx.moveTo(ou + u * k + r, ov + v * k);
        ctx.arc(ou + u * k, ov + v * k, r, 0, Math.PI * 2);
        continue;
      }
      const [ua, va] = proj(i - 1);
      const [ub, vb] = proj(i);
      ctx.moveTo(ou + ua * k, ov + va * k);
      ctx.lineTo(ou + ub * k, ov + vb * k);
    }
    ctx.stroke();
  }
  return canvas.toDataURL();
}
