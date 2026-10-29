<script lang="ts">
  // Vue 3D du parcours (ADR-0003) : plateau de la machine à sa taille, grille
  // de 10 mm, parcours en lignes dégradées selon la hauteur, déplacements en
  // retrait. Orbite (glisser), déplacement (clic droit, Maj + glisser), zoom
  // vers le curseur (molette), cadrage `F`. La vue se cadre seule sur chaque
  // calcul tant que l'utilisateur ne l'a pas bougée ; changer de modèle
  // (`fitKey`) rétablit le cadrage automatique. Rendu à la demande seulement.
  // Couleurs : jetons `--view-*` et `--path-*`, relus au changement de thème.
  // Annotations du modèle (`path.note`) : pastilles d'étiquette (canevas.md),
  // décalées en haut à droite de leur point, masquables, choix mémorisé.
  // Gouttes (extrusion à l'arrêt) : sphères du diamètre de la goutte, même
  // dégradé que le parcours. Boudins en volume (VIEW-06, `lib/beads.ts`) ou en
  // lignes, au choix, mémorisé ; les déplacements restent des lignes.
  // Coloration au choix (VIEW-07, `lib/coloring.ts`), mémorisée, avec sa
  // légende en bas à gauche.
  // Avancement (VIEW-08) : curseur au mouvement près en bas (pourcentage et
  // temps estimés par le moteur, Z de la buse), lecture, buse simulée
  // au point courant ; coupe en Z à droite (plan de coupe, boudins compris) ;
  // déplacements affichables, mémorisé. Le parcours se montre jusqu'au point
  // courant par plages de dessin : rien n'est reconstruit en avançant.
  // Inspection (VIEW-09) : le segment le plus proche du pointeur (à 8 px,
  // le plus près de la caméra s'il y en a plusieurs) est surligné et ses
  // valeurs s'affichent dans une fiche à côté du pointeur.
  import { onMount, untrack } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
  import { CircleX, Cylinder, Focus, Gauge, MessageSquareText, Pause, Play, Route } from '@lucide/svelte';
  import { beadMaterial, beadMesh } from '../lib/beads';
  import { COLOR_MODES, MODE_UNIT, colorize, type ColorMode, type Legend } from '../lib/coloring';
  import { fmtDuration, fmtLength, lengthUnit, num, pct } from '../lib/format';
  import { ui } from '../lib/ui.svelte';
  import { storageKey } from '../lib/app';
  import { text } from '../lib/text';
  import { theme } from '../lib/theme.svelte';
  import { tip } from '../lib/tip';
  import { i18n, t } from '../lib/i18n.svelte';
  import type { FrameData, PrinterProfile } from '../lib/types';

  let {
    frame,
    printer,
    fitKey,
  }: {
    frame: FrameData | null;
    printer: PrinterProfile | null;
    fitKey: string;
  } = $props();

  let host: HTMLDivElement;
  const NOTES_KEY = storageKey('view.notes.v1');
  let showNotes = $state((() => {
    try {
      return localStorage.getItem(NOTES_KEY) !== '0';
    } catch {
      return true;
    }
  })());
  let labels = $state<{ x: number; y: number; text: string }[]>([]);
  const COLOR_KEY = storageKey('view.color.v1');
  let colorMode = $state<ColorMode>((() => {
    try {
      const v = localStorage.getItem(COLOR_KEY) as ColorMode | null;
      return v && COLOR_MODES.includes(v) ? v : 'z';
    } catch {
      return 'z';
    }
  })());
  let legend = $state<Legend | null>(null);
  const TRAVEL_KEY = storageKey('view.travel.v1');
  let showTravel = $state((() => {
    try {
      return localStorage.getItem(TRAVEL_KEY) !== '0';
    } catch {
      return true;
    }
  })());
  const MODE_KEY = storageKey('view.volume.v1');
  let volume = $state((() => {
    try {
      return localStorage.getItem(MODE_KEY) !== '0';
    } catch {
      return true;
    }
  })());

  /** Positions à l'écran des annotations (après chaque rendu). */
  function placeLabels() {
    const notes = frame?.report.notes ?? [];
    if (!showNotes || !notes.length || !host) {
      if (labels.length) labels = [];
      return;
    }
    const w = host.clientWidth;
    const h = host.clientHeight;
    const out: { x: number; y: number; text: string }[] = [];
    for (const n of notes) {
      const v = new THREE.Vector3(n.x, n.y, n.z).project(camera);
      if (v.z > 1 || v.z < -1) continue;
      out.push({ x: ((v.x + 1) / 2) * w, y: ((1 - v.y) / 2) * h, text: text(n.text) });
    }
    labels = out;
  }
  let renderer: THREE.WebGLRenderer | null = null;
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(35, 1, 0.5, 10000);
  camera.up.set(0, 0, 1);
  let controls: OrbitControls | null = null;
  let pending = 0;
  let userMoved = false;
  let lastFitKey = '';

  const bed = new THREE.Group();
  const bedFill = new THREE.MeshBasicMaterial();
  const bedEdge = new THREE.LineBasicMaterial();
  const grid = new THREE.LineBasicMaterial();
  const exclude = new THREE.LineBasicMaterial();
  const printMat = new THREE.LineBasicMaterial({ vertexColors: true });
  const travelMat = new THREE.LineBasicMaterial();
  const printLines = new THREE.LineSegments(new THREE.BufferGeometry(), printMat);
  const travelLines = new THREE.LineSegments(new THREE.BufferGeometry(), travelMat);
  // Mouvements imprimés hors du volume ou dans une zone exclue : surlignés.
  const outsideMat = new THREE.LineBasicMaterial();
  const outsideLines = new THREE.LineSegments(new THREE.BufferGeometry(), outsideMat);
  scene.add(bed, printLines, travelLines, outsideLines);

  // Avancement (VIEW-08) : point courant `last` (n − 1 : tout), temps cumulé
  // jusqu'à chaque point, points d'arrivée de chaque famille dans l'ordre (pour
  // les plages de dessin).
  let count = $state(0);
  let last = $state(0);
  let cum = new Float64Array(0);
  let printEnds = new Int32Array(0);
  let travelEnds = new Int32Array(0);
  let outsideEnds = new Int32Array(0);
  let beadEnds: Int32Array[] = [];
  let blobEnds = new Int32Array(0);
  let playing = $state(false);
  let playRaf = 0;
  /** Durée d'une lecture complète (ms). */
  const PLAY_MS = 30000;
  // Coupe en Z : on garde z ≤ zCut (plan de normale −Z).
  let zMin = $state(0);
  let zMax = $state(0);
  let zCut = $state(Infinity);
  const cutPlane = new THREE.Plane(new THREE.Vector3(0, 0, -1), 1e9);
  for (const m of [printMat, travelMat, outsideMat, beadMaterial]) m.clippingPlanes = [cutPlane];

  // Buse simulée : cône (pointe au point courant) et bloc de chauffe.
  const nozzle = new THREE.Group();
  const nozzleMat = new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.85, depthWrite: false });
  const nozzleEdge = new THREE.LineBasicMaterial();
  {
    const cone = new THREE.CylinderGeometry(3, 0.4, 5, 24).rotateX(Math.PI / 2).translate(0, 0, 2.5);
    const block = new THREE.BoxGeometry(16, 16, 9).translate(3, 0, 5 + 4.5);
    for (const g of [cone, block]) nozzle.add(new THREE.Mesh(g, nozzleMat), new THREE.LineSegments(new THREE.EdgesGeometry(g, 30), nozzleEdge));
  }
  nozzle.visible = false;
  nozzle.renderOrder = 10;
  scene.add(nozzle);

  // Inspection : segment survolé, par-dessus tout (sans test de profondeur).
  const hoverMat = new THREE.LineBasicMaterial({ depthTest: false, transparent: true });
  const hoverDots = new THREE.PointsMaterial({ size: 6, sizeAttenuation: false, depthTest: false, transparent: true });
  const hoverGeom = new THREE.BufferGeometry();
  hoverGeom.setAttribute('position', new THREE.Float32BufferAttribute(new Float32Array(6), 3));
  const hoverLine = new THREE.Line(hoverGeom, hoverMat);
  const hoverEnds = new THREE.Points(hoverGeom, hoverDots);
  hoverLine.renderOrder = hoverEnds.renderOrder = 11;
  hoverLine.visible = hoverEnds.visible = false;
  hoverLine.frustumCulled = hoverEnds.frustumCulled = false;
  scene.add(hoverLine, hoverEnds);
  let hover = $state<{ x: number; y: number; i: number } | null>(null);
  let viewW = $state(0);
  let viewH = $state(0);
  let hoverRaf = 0;
  // Gouttes : une sphère de diamètre 1 par goutte, mise à l'échelle.
  const blobGeom = new THREE.IcosahedronGeometry(0.5, 1);
  const blobMat = new THREE.MeshBasicMaterial({ clippingPlanes: [cutPlane] });
  let blobs: THREE.InstancedMesh | null = null;
  // Boudins en volume : imprimés, et hors du volume (surlignés).
  let beads: THREE.Mesh[] = [];

  const TOKENS = [
    'view-bg',
    'view-bed',
    'view-bed-edge',
    'view-grid',
    'view-bed-exclude',
    'path-low',
    'path-high',
    'path-travel',
    'danger',
    'scale-1',
    'scale-2',
    'scale-3',
    'scale-4',
    'scale-5',
    'path-bridge',
    'path-blob',
    'view-nozzle',
    'view-nozzle-edge',
    'accent',
  ] as const;
  let colors: Record<(typeof TOKENS)[number], THREE.Color> = Object.fromEntries(TOKENS.map((k) => [k, new THREE.Color()])) as never;

  function readColors() {
    const cs = getComputedStyle(document.documentElement);
    for (const k of TOKENS) colors[k] = new THREE.Color(cs.getPropertyValue(`--${k}`).trim() || '#808080');
    scene.background = colors['view-bg'];
    bedFill.color = colors['view-bed'];
    bedEdge.color = colors['view-bed-edge'];
    grid.color = colors['view-grid'];
    exclude.color = colors['view-bed-exclude'];
    travelMat.color = colors['path-travel'];
    outsideMat.color = colors['danger'];
    nozzleMat.color = colors['view-nozzle'];
    nozzleEdge.color = colors['view-nozzle-edge'];
    hoverMat.color = colors['accent'];
    hoverDots.color = colors['accent'];
  }

  function render() {
    if (!renderer || pending) return;
    pending = requestAnimationFrame(() => {
      pending = 0;
      renderer?.render(scene, camera);
      placeLabels();
    });
  }

  function buildBed(p: PrinterProfile | null) {
    for (const c of [...bed.children]) {
      bed.remove(c);
      (c as THREE.Mesh).geometry.dispose();
    }
    if (!p) return;
    const [w, h] = p.bed.size;
    const [x0, y0] = p.bed.origin === 'center' ? [-w / 2, -h / 2] : [0, 0];
    const [cx, cy] = [x0 + w / 2, y0 + h / 2];
    const round = p.bed.shape === 'circle';
    const r = w / 2;
    const plane = new THREE.Mesh(round ? new THREE.CircleGeometry(r, 128) : new THREE.PlaneGeometry(w, h), bedFill);
    plane.position.set(cx, cy, -0.05);
    // Grille tous les 10 mm, coupée au bord d'un plateau rond.
    const lines: number[] = [];
    for (let x = 10; x < w; x += 10) {
      const half = round ? Math.sqrt(Math.max(0, r * r - (x0 + x - cx) ** 2)) : h / 2;
      lines.push(x0 + x, cy - half, 0, x0 + x, cy + half, 0);
    }
    for (let y = 10; y < h; y += 10) {
      const half = round ? Math.sqrt(Math.max(0, r * r - (y0 + y - cy) ** 2)) : w / 2;
      lines.push(cx - half, y0 + y, 0, cx + half, y0 + y, 0);
    }
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(lines, 3));
    const edgePts: number[] = [];
    if (round) {
      for (let k = 0; k <= 128; k++) {
        const a = (k / 128) * Math.PI * 2;
        edgePts.push(cx + r * Math.cos(a), cy + r * Math.sin(a), 0);
      }
    } else {
      edgePts.push(x0, y0, 0, x0 + w, y0, 0, x0 + w, y0 + h, 0, x0, y0 + h, 0, x0, y0, 0);
    }
    const edge = new THREE.BufferGeometry();
    edge.setAttribute('position', new THREE.Float32BufferAttribute(edgePts, 3));
    bed.add(plane, new THREE.LineSegments(g, grid), new THREE.Line(edge, bedEdge));
    // Zones exclues (repère machine) : contour et hachures à 45°, tous les 3 mm.
    const hatch: number[] = [];
    for (const z of p.bed.exclude ?? []) {
      const [ax, ay] = [Math.max(z.min[0], x0), Math.max(z.min[1], y0)];
      const [bx, by] = [Math.min(z.max[0], x0 + w), Math.min(z.max[1], y0 + h)];
      if (ax >= bx || ay >= by) continue;
      const z0 = 0.02;
      hatch.push(ax, ay, z0, bx, ay, z0, bx, ay, z0, bx, by, z0, bx, by, z0, ax, by, z0, ax, by, z0, ax, ay, z0);
      // Droites x − y = c coupées par le rectangle.
      for (let c = ax - by + 3; c < bx - ay; c += 3) {
        const p0: [number, number] = c >= ax - ay ? [ax + (c - (ax - ay)), ay] : [ax, ax - c];
        const p1: [number, number] = c <= bx - by ? [c + by, by] : [bx, bx - c];
        hatch.push(p0[0], p0[1], z0, p1[0], p1[1], z0);
      }
    }
    if (hatch.length) {
      const hg = new THREE.BufferGeometry();
      hg.setAttribute('position', new THREE.Float32BufferAttribute(hatch, 3));
      bed.add(new THREE.LineSegments(hg, exclude));
    }
  }

  function buildPath(f: FrameData | null) {
    for (const b of beads) {
      scene.remove(b);
      b.geometry.dispose();
    }
    beads = [];
    printLines.geometry.dispose();
    travelLines.geometry.dispose();
    outsideLines.geometry.dispose();
    const pg = new THREE.BufferGeometry();
    const tg = new THREE.BufferGeometry();
    const og = new THREE.BufferGeometry();
    const n = f ? f.kinds.length : 0;
    if (f && n > 1) {
      const pos = new THREE.BufferAttribute(f.positions, 3);
      const { colors: col, legend: lg } = colorize(f, colorMode, {
        low: colors['path-low'],
        high: colors['path-high'],
        ramp: [colors['scale-1'], colors['scale-2'], colors['scale-3'], colors['scale-4'], colors['scale-5']],
        print: colors['path-high'],
        bridge: colors['path-bridge'],
        blob: colors['path-blob'],
      });
      legend = lg;
      const prints: number[] = [];
      const travels: number[] = [];
      const outside: number[] = [];
      // Déplacements, essuyages (4) et pauses (5) en retrait.
      for (let i = 1; i < n; i++) (f.kinds[i] === 0 || f.kinds[i] >= 4 ? travels : f.outside[i] ? outside : prints).push(i - 1, i);
      const arrivals = (list: number[]) => Int32Array.from(list.filter((_, k) => k % 2 === 1));
      travelEnds = arrivals(travels);
      if (volume) {
        // Segments imprimés (pas les gouttes, dessinées en sphères).
        const ends = (list: number[]) => list.filter((_, k) => k % 2 === 1).filter((i) => f.kinds[i] !== 3);
        const danger = colors['danger'];
        const pe = ends(prints);
        const oe = ends(outside);
        beadEnds = [Int32Array.from(pe), Int32Array.from(oe)];
        beads.push(
          beadMesh(f.positions, f.widths, f.heights, pe, (i, c) => c.setRGB(col[i * 3], col[i * 3 + 1], col[i * 3 + 2])),
          beadMesh(f.positions, f.widths, f.heights, oe, (_, c) => c.copy(danger)),
        );
        scene.add(...beads);
        prints.length = 0;
        outside.length = 0;
      } else {
        beadEnds = [];
      }
      printEnds = arrivals(prints);
      outsideEnds = arrivals(outside);
      pg.setAttribute('position', pos);
      pg.setAttribute('color', new THREE.BufferAttribute(col, 3));
      pg.setIndex(prints);
      tg.setAttribute('position', pos);
      tg.setIndex(travels);
      og.setAttribute('position', pos);
      og.setIndex(outside);
    }
    printLines.geometry = pg;
    travelLines.geometry = tg;
    outsideLines.geometry = og;
    if (!f || n < 2) legend = null;
    buildBlobs(f, pg.getAttribute('color') as THREE.BufferAttribute | undefined);
    applyProgress();
  }

  /** Temps cumulé et hauteurs d'un nouveau calcul ; garde l'avancement et la
   * coupe là où l'utilisateur les a mis (au bout : au nouveau bout). */
  function measure(f: FrameData | null) {
    stop();
    const n = f ? f.kinds.length : 0;
    const wasEnd = last >= count - 1;
    const ratio = count > 1 ? last / (count - 1) : 1;
    const wasTop = zCut >= zMax - 1e-6;
    // Durée de chaque mouvement estimée par le moteur (accélérations,
    // rétractions comprises) : le temps affiché est celui de l'imprimante.
    cum = new Float64Array(n);
    let lo = Infinity;
    let hi = -Infinity;
    for (let i = 0; i < n; i++) {
      const z = f!.positions[i * 3 + 2];
      if (i > 0) cum[i] = cum[i - 1] + f!.times[i];
      if (f!.kinds[i] !== 0) {
        lo = Math.min(lo, z);
        hi = Math.max(hi, z);
      }
    }
    count = n;
    last = n > 0 ? (wasEnd ? n - 1 : Math.round(ratio * (n - 1))) : 0;
    zMin = Number.isFinite(lo) ? lo : 0;
    zMax = Number.isFinite(hi) ? hi : 0;
    zCut = wasTop ? zMax : Math.min(Math.max(zCut, zMin), zMax);
  }

  /** Nombre de points d'arrivée ≤ k (liste croissante). */
  function upto(ends: Int32Array, k: number): number {
    let lo = 0;
    let hi = ends.length;
    while (lo < hi) {
      const m = (lo + hi) >> 1;
      if (ends[m] <= k) lo = m + 1;
      else hi = m;
    }
    return lo;
  }

  /** Montre le parcours jusqu'au point `last`, place la buse, règle la coupe.
   * Sans suivi : les effets qui l'appellent ne dépendent pas du curseur. */
  function applyProgress() {
    untrack(drawProgress);
  }

  function drawProgress() {
    clearHover();
    const k = last;
    printLines.geometry.setDrawRange(0, 2 * upto(printEnds, k));
    travelLines.geometry.setDrawRange(0, 2 * upto(travelEnds, k));
    outsideLines.geometry.setDrawRange(0, 2 * upto(outsideEnds, k));
    beads.forEach((b, j) => ((b.geometry as THREE.InstancedBufferGeometry).instanceCount = upto(beadEnds[j] ?? new Int32Array(0), k)));
    if (blobs) blobs.count = upto(blobEnds, k);
    const f = frame;
    nozzle.visible = !!f && count > 1 && k < count - 1;
    if (nozzle.visible && f) nozzle.position.set(f.positions[k * 3], f.positions[k * 3 + 1], f.positions[k * 3 + 2]);
    cutPlane.constant = zCut >= zMax - 1e-6 ? 1e9 : zCut;
    render();
  }

  function setLast(k: number) {
    last = Math.max(0, Math.min(count - 1, Math.round(k)));
    applyProgress();
  }

  function stop() {
    playing = false;
    if (playRaf) cancelAnimationFrame(playRaf);
    playRaf = 0;
  }

  /** Lecture : la buse parcourt l'impression en PLAY_MS, à sa vitesse relative. */
  function play() {
    if (count < 2) return;
    if (last >= count - 1) setLast(0);
    playing = true;
    const total = cum[count - 1] || 1;
    const t0 = performance.now();
    const from = cum[last];
    const step = () => {
      const t = from + ((performance.now() - t0) / PLAY_MS) * total;
      // Premier point dont le temps cumulé atteint t.
      let lo = last;
      let hi = count - 1;
      while (lo < hi) {
        const m = (lo + hi) >> 1;
        if (cum[m] < t) lo = m + 1;
        else hi = m;
      }
      setLast(lo);
      if (lo >= count - 1) stop();
      else playRaf = requestAnimationFrame(step);
    };
    playRaf = requestAnimationFrame(step);
  }

  function clearHover() {
    if (hoverRaf) cancelAnimationFrame(hoverRaf);
    hoverRaf = 0;
    if (hover) hover = null;
    if (hoverLine.visible) {
      hoverLine.visible = hoverEnds.visible = false;
      render();
    }
  }

  /** Segment imprimé ou déplacement visible le plus proche de (mx, my), en
   * pixels de la vue ; -1 si aucun à moins de 8 px. Chaque point n'est
   * projeté qu'une fois (les segments se suivent). */
  function pick(mx: number, my: number): number {
    const f = frame;
    if (!f || count < 2 || !host) return -1;
    const m = new THREE.Matrix4().multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse).elements;
    const w2 = host.clientWidth / 2;
    const h2 = host.clientHeight / 2;
    const cut = zCut >= zMax - 1e-6 ? Infinity : zCut;
    const P = f.positions;
    const R2 = 8 * 8;
    let best = -1;
    let bestDepth = Infinity;
    let ax = 0;
    let ay = 0;
    let az = 0;
    let aok = false;
    for (let i = 0; i <= last; i++) {
      const x = P[i * 3];
      const y = P[i * 3 + 1];
      const z = P[i * 3 + 2];
      const cw = m[3] * x + m[7] * y + m[11] * z + m[15];
      const ok = cw > 1e-6;
      const sx = ok ? w2 + ((m[0] * x + m[4] * y + m[8] * z + m[12]) / cw) * w2 : 0;
      const sy = ok ? h2 - ((m[1] * x + m[5] * y + m[9] * z + m[13]) / cw) * h2 : 0;
      const sz = ok ? (m[2] * x + m[6] * y + m[10] * z + m[14]) / cw : 0;
      const kind = f.kinds[i];
      const shown = i > 0 && (kind === 0 || kind >= 4 ? showTravel : true) && (z <= cut || P[i * 3 - 1] <= cut);
      if (shown && ok && aok) {
        // Distance du pointeur au segment (a → b) à l'écran.
        const dx = sx - ax;
        const dy = sy - ay;
        const l2 = dx * dx + dy * dy;
        const t = l2 > 0 ? Math.max(0, Math.min(1, ((mx - ax) * dx + (my - ay) * dy) / l2)) : 0;
        const ex = ax + t * dx - mx;
        const ey = ay + t * dy - my;
        if (ex * ex + ey * ey <= R2) {
          const depth = az + t * (sz - az);
          if (depth < bestDepth) {
            bestDepth = depth;
            best = i;
          }
        }
      }
      ax = sx;
      ay = sy;
      az = sz;
      aok = ok;
    }
    return best;
  }

  function onHover(e: PointerEvent) {
    if (e.buttons || playing) return clearHover();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const mx = e.clientX - r.left;
    const my = e.clientY - r.top;
    if (hoverRaf) cancelAnimationFrame(hoverRaf);
    hoverRaf = requestAnimationFrame(() => {
      hoverRaf = 0;
      const i = pick(mx, my);
      if (i < 1 || !frame) return clearHover();
      hover = { x: mx, y: my, i };
      const a = hoverGeom.getAttribute('position') as THREE.BufferAttribute;
      a.copyArray(frame.positions.subarray((i - 1) * 3, i * 3 + 3));
      a.needsUpdate = true;
      hoverLine.visible = hoverEnds.visible = true;
      render();
    });
  }

  /** Fiche du segment survolé : lignes « libellé, valeur ». */
  let card = $derived.by(() => {
    void i18n.locale;
    void ui.units;
    const f = frame;
    if (!hover || !f || hover.i >= f.kinds.length) return null;
    const i = hover.i;
    const kind = f.kinds[i];
    const len = (v: number) => `${fmtLength(v, ui.units, 2)} ${lengthUnit(ui.units)}`;
    const rows: [string, string][] = [];
    if (kind !== 0 && kind < 4 && f.widths[i] > 0) {
      rows.push([t('view.inspect.bead'), kind === 3 ? `⌀ ${len(f.widths[i])}` : `${fmtLength(f.widths[i], ui.units, 2)} × ${len(f.heights[i])}`]);
    }
    rows.push([t('view.inspect.speed'), `${num(f.speeds[i], 1)} mm/s`]);
    const limit = f.limits[i];
    if (limit > 0) rows.push([t('view.inspect.requested'), `${num(f.requested[i], 1)} mm/s`]);
    if (kind !== 0 && kind < 4) rows.push([t('view.inspect.flow'), `${num(f.flows[i], 1)} mm³/s`]);
    rows.push([t('view.inspect.fan'), pct(f.fans[i])]);
    rows.push(['Z', len(f.positions[i * 3 + 2])]);
    rows.push([t('view.inspect.at'), fmtDuration(cum[i])]);
    const kinds = ['travel', 'print', 'bridge', 'blob', 'wipe', 'pause'] as const;
    const limits = ['first_layer', 'machine', 'flow', 'z_axis'] as const;
    return {
      title: `${t(`view.inspect.kind.${kinds[kind] ?? 'print'}`)} · ${t('view.inspect.move', { n: num(i) })}`,
      rows,
      limit: limit > 0 && limit <= limits.length ? t(`view.inspect.limit.${limits[limit - 1]}`) : null,
      outside: f.outside[i] === 1,
    };
  });

  /** Avancement : « 62 % · 3 h 12 min / 7 h 27 min · Z 48,2 mm ». */
  let readout = $derived.by(() => {
    void i18n.locale;
    void ui.units;
    if (count < 2 || !frame) return '';
    const total = cum[count - 1];
    const z = frame.positions[last * 3 + 2];
    return t('view.readout', {
      pct: pct(total > 0 ? (100 * cum[last]) / total : 100),
      time: fmtDuration(cum[last]),
      total: fmtDuration(total),
      z: `${fmtLength(z, ui.units, 1)} ${lengthUnit(ui.units)}`,
    });
  });

  function buildBlobs(f: FrameData | null, col?: THREE.BufferAttribute) {
    if (blobs) {
      scene.remove(blobs);
      blobs.dispose();
      blobs = null;
    }
    blobEnds = new Int32Array(0);
    if (!f) return;
    const idx: number[] = [];
    for (let i = 1; i < f.kinds.length; i++) if (f.kinds[i] === 3) idx.push(i);
    blobEnds = Int32Array.from(idx);
    if (!idx.length) return;
    blobs = new THREE.InstancedMesh(blobGeom, blobMat, idx.length);
    const m = new THREE.Matrix4();
    const c = new THREE.Color();
    idx.forEach((i, k) => {
      const d = f.widths[i] || 1;
      // La goutte s'étale sous la buse : centre à une demi-goutte sous le point.
      m.makeScale(d, d, d).setPosition(f.positions[i * 3], f.positions[i * 3 + 1], f.positions[i * 3 + 2] - d / 2);
      blobs!.setMatrixAt(k, m);
      if (f.outside[i] || !col) c.copy(colors['danger']);
      else c.setRGB(col.getX(i), col.getY(i), col.getZ(i));
      blobs!.setColorAt(k, c);
    });
    scene.add(blobs);
  }

  /** Cadre la pièce (ou le plateau) vue de devant, à gauche, en hauteur. */
  function fit() {
    const b = frame?.report.stats?.bounds;
    const box = new THREE.Box3();
    if (b) box.set(new THREE.Vector3(...b.min), new THREE.Vector3(...b.max));
    else if (printer) {
      const [w, h] = printer.bed.size;
      box.set(new THREE.Vector3(0, 0, 0), new THREE.Vector3(w, h, 10));
    } else return;
    const sphere = box.getBoundingSphere(new THREE.Sphere());
    const r = Math.max(sphere.radius, 5);
    const dist = r / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2)) * 1.05;
    const dir = new THREE.Vector3(-0.45, -1, 0.75).normalize();
    camera.position.copy(sphere.center).addScaledVector(dir, dist);
    camera.near = dist / 100;
    camera.far = dist * 20;
    camera.updateProjectionMatrix();
    controls?.target.copy(sphere.center);
    controls?.update();
    render();
  }

  /** Valeur de la légende, dans l'unité de la grandeur. */
  function legendValue(mode: ColorMode, v: number): string {
    switch (MODE_UNIT[mode]) {
      case 'length':
        return `${fmtLength(v, ui.units, 2)} ${lengthUnit(ui.units)}`;
      case '%':
        return pct(v);
      case 'mm/s':
        return `${num(v)} mm/s`;
      case 'mm³/s':
        return `${num(v, 1)} mm³/s`;
      default:
        return num(v);
    }
  }

  function setColorMode(m: ColorMode) {
    colorMode = m;
    try {
      localStorage.setItem(COLOR_KEY, m);
    } catch {
      /* ignoré */
    }
  }

  export function refit() {
    userMoved = false;
    fit();
  }

  onMount(() => {
    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.localClippingEnabled = true;
    host.appendChild(renderer.domElement);
    controls = new OrbitControls(camera, renderer.domElement);
    controls.screenSpacePanning = true;
    controls.zoomToCursor = true;
    controls.addEventListener('change', render);
    controls.addEventListener('start', () => {
      userMoved = true;
      clearHover();
    });
    renderer.domElement.addEventListener('pointermove', onHover);
    renderer.domElement.addEventListener('pointerleave', clearHover);
    // Mesure du coût d'un rendu en développement (pas dans le build).
    if (import.meta.env.DEV) (window as unknown as Record<string, unknown>).__fabweaveView = { renderer, scene, camera, controls };
    const ro = new ResizeObserver(() => {
      const { clientWidth: w, clientHeight: h } = host;
      if (!w || !h) return;
      viewW = w;
      viewH = h;
      renderer!.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      if (!userMoved) fit();
      render();
    });
    ro.observe(host);
    return () => {
      stop();
      clearHover();
      ro.disconnect();
      controls?.dispose();
      renderer?.dispose();
      renderer?.domElement.remove();
      renderer = null;
    };
  });

  // Nouveau calcul : temps et hauteurs (avant la construction du parcours).
  $effect.pre(() => {
    const f = frame;
    untrack(() => measure(f));
  });

  $effect(() => {
    void theme.applied;
    void volume;
    void colorMode;
    readColors();
    buildPath(frame);
    render();
  });

  $effect(() => {
    buildBed(printer);
    render();
  });

  // Textes des annotations dans la langue de l'interface.
  $effect(() => {
    void i18n.locale;
    render();
  });

  $effect(() => {
    void frame;
    if (fitKey !== lastFitKey) {
      lastFitKey = fitKey;
      userMoved = false;
    }
    if (!userMoved) fit();
  });

  $effect(() => {
    travelLines.visible = showTravel;
    render();
  });
</script>

<div class="viewport" bind:this={host}>
  {#if frame && frame.report.points === 0}
    <p class="empty">{t('view.empty')}</p>
  {/if}
  {#each labels as l, i (i)}
    <span class="dot" style:left="{l.x}px" style:top="{l.y}px"></span>
    <span class="label" style:left="{l.x + 10}px" style:top="{l.y - 28}px">{l.text}</span>
  {/each}
  <label class="colorby">
    <span class="sr">{t('view.colorBy')}</span>
    <select class="input" value={colorMode} aria-label={t('view.colorBy')} use:tip={t('view.colorBy')} onchange={(e) => setColorMode(e.currentTarget.value as ColorMode)}>
      {#each COLOR_MODES as m (m)}
        <option value={m}>{t(`view.color.${m}`)}</option>
      {/each}
    </select>
  </label>
  {#if legend}
    <div class="legend">
      {#if legend.mode === 'type'}
        {#each legend.types as ty (ty)}
          <span class="item"><span class="swatch {ty}"></span>{t(`view.type.${ty}`)}</span>
        {/each}
      {:else if legend.max > legend.min}
        <div class="bar" class:neutral={legend.mode === 'z'}></div>
        <div class="ticks tabular">
          <span>{legendValue(legend.mode, legend.min)}</span>
          <span>{legendValue(legend.mode, (legend.min + legend.max) / 2)}</span>
          <span>{legendValue(legend.mode, legend.max)}</span>
        </div>
      {:else}
        <span class="item tabular"><span class="swatch {legend.mode === 'z' ? 'print' : 'scale'}"></span>{legendValue(legend.mode, legend.min)}</span>
      {/if}
    </div>
  {/if}
  {#if card && hover}
    <div class="card" style:left="{hover.x + 14}px" style:top="{hover.y + 14}px" class:flip-x={hover.x > viewW - 240} class:flip-y={hover.y > viewH - 200}>
      <div class="card-title">{card.title}</div>
      <dl>
        {#each card.rows as [k, v] (k)}
          <dt>{k}</dt>
          <dd class="tabular">{v}</dd>
        {/each}
      </dl>
      {#if card.limit}
        <div class="card-note"><Gauge size={12} />{card.limit}</div>
      {/if}
      {#if card.outside}
        <div class="card-note danger"><CircleX size={12} />{t('view.inspect.outside')}</div>
      {/if}
    </div>
  {/if}
  {#if count > 1}
    <div class="timeline">
      <button
        class="btn quiet icon"
        type="button"
        aria-label={playing ? t('view.pause') : t('view.play')}
        use:tip={playing ? t('view.pause') : t('view.play')}
        onclick={() => (playing ? stop() : play())}
        >{#if playing}<Pause size={15} />{:else}<Play size={15} />{/if}</button
      >
      <input
        class="range"
        type="range"
        min="0"
        max={count - 1}
        step="1"
        value={last}
        aria-label={t('view.progress')}
        use:tip={t('view.progressHint')}
        oninput={(e) => {
          stop();
          setLast(Number(e.currentTarget.value));
        }}
      />
      <span class="readout tabular">{readout}</span>
    </div>
    {#if zMax > zMin}
      <div class="zcut" use:tip={t('view.cutHint')}>
        <span class="zval tabular">{num(Math.min(zCut, zMax) / (ui.units === 'mm' ? 1 : 25.4), ui.units === 'mm' ? 1 : 2)}</span>
        <input
          class="range vertical"
          type="range"
          min={zMin}
          max={zMax}
          step="0.05"
          value={Math.min(zCut, zMax)}
          aria-label={t('view.cut')}
          oninput={(e) => {
            zCut = Number(e.currentTarget.value);
            applyProgress();
          }}
        />
        <span class="zunit">{lengthUnit(ui.units)}</span>
      </div>
    {/if}
  {/if}
  <div class="tools">
    <button
      class="btn quiet icon"
      class:on={showNotes}
      type="button"
      aria-pressed={showNotes}
      aria-label={t('view.notes')}
      use:tip={t('view.notes')}
      onclick={() => {
        showNotes = !showNotes;
        try {
          localStorage.setItem(NOTES_KEY, showNotes ? '1' : '0');
        } catch {
          /* ignoré */
        }
        render();
      }}><MessageSquareText size={15} /></button
    >
    <button
      class="btn quiet icon"
      class:on={showTravel}
      type="button"
      aria-pressed={showTravel}
      aria-label={t('view.travel')}
      use:tip={t('view.travel')}
      onclick={() => {
        showTravel = !showTravel;
        try {
          localStorage.setItem(TRAVEL_KEY, showTravel ? '1' : '0');
        } catch {
          /* ignoré */
        }
      }}><Route size={15} /></button
    >
    <button
      class="btn quiet icon"
      class:on={volume}
      type="button"
      aria-pressed={volume}
      aria-label={t('view.volume')}
      use:tip={t('view.volumeHint')}
      onclick={() => {
        volume = !volume;
        try {
          localStorage.setItem(MODE_KEY, volume ? '1' : '0');
        } catch {
          /* ignoré */
        }
      }}><Cylinder size={15} /></button
    >
    <button class="btn quiet icon" type="button" aria-label={t('view.fit')} use:tip={t('view.fit')} onclick={refit}><Focus size={15} /></button>
  </div>
</div>

<style>
  .viewport {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--view-bg);
  }

  .viewport :global(canvas) {
    display: block;
  }

  .tools {
    position: absolute;
    top: var(--s2);
    right: var(--s2);
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .colorby {
    position: absolute;
    top: var(--s2);
    left: var(--s2);
  }

  .colorby .input {
    width: auto;
    min-width: 150px;
    background: var(--bg-panel);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  .legend {
    position: absolute;
    left: var(--s2);
    bottom: calc(var(--s2) * 2 + var(--control-h) + 6px);
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 180px;
    padding: var(--s1) var(--s2);
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    pointer-events: none;
  }

  /* Avancement : barre en bas de la vue, sur toute la largeur. */
  .timeline {
    position: absolute;
    left: var(--s2);
    right: var(--s2);
    bottom: var(--s2);
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: 2px var(--s2) 2px 2px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .range {
    flex: 1;
    min-width: 0;
    accent-color: var(--accent);
  }

  .readout {
    flex: none;
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    white-space: nowrap;
  }

  /* Fiche du segment survolé : pastille d'infobulle, à côté du pointeur. */
  .card {
    position: absolute;
    z-index: 2;
    min-width: 190px;
    padding: 6px 8px;
    border-radius: var(--radius);
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    box-shadow: var(--tooltip-shadow);
    font-size: var(--fs-xs);
    pointer-events: none;
  }

  .card.flip-x {
    transform: translateX(calc(-100% - 28px));
  }

  .card.flip-y {
    translate: 0 calc(-100% - 28px);
  }

  .card-title {
    margin-bottom: 4px;
    font-weight: 600;
  }

  .card dl {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px var(--s2);
    margin: 0;
  }

  .card dt {
    opacity: 0.75;
  }

  .card dd {
    margin: 0;
    text-align: right;
  }

  .card-note {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
    opacity: 0.85;
  }

  .card-note.danger {
    color: var(--danger);
    opacity: 1;
  }

  /* Coupe en Z : curseur vertical à droite, sous les outils. */
  .zcut {
    position: absolute;
    right: var(--s2);
    top: calc(var(--s2) * 2 + var(--control-h) + 6px);
    bottom: calc(var(--s2) * 2 + var(--control-h) + 6px);
    width: 44px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px 0;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }

  .zcut .range.vertical {
    flex: 1;
    min-height: 0;
    width: 20px;
    writing-mode: vertical-lr;
    direction: rtl;
  }

  .zunit {
    color: var(--text-muted);
  }

  .bar {
    height: 8px;
    border-radius: 2px;
    background: linear-gradient(to right, var(--scale-1), var(--scale-2), var(--scale-3), var(--scale-4), var(--scale-5));
  }

  .bar.neutral {
    background: linear-gradient(to right, var(--path-low), var(--path-high));
  }

  .ticks {
    display: flex;
    justify-content: space-between;
    gap: var(--s2);
  }

  .item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    background: var(--path-high);
  }

  .swatch.bridge {
    background: var(--path-bridge);
  }

  .swatch.blob {
    background: var(--path-blob);
  }

  .swatch.scale {
    background: var(--scale-3);
  }

  .btn.on {
    background: var(--bg-selected);
    color: var(--accent-text);
  }

  /* Étiquettes d'annotation : pastille 18 px, 600 11 px, décalée de 10 px. */
  .label {
    position: absolute;
    height: 18px;
    padding: 0 6px;
    border-radius: 3px;
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    font: 600 11px/18px var(--font);
    white-space: nowrap;
    pointer-events: none;
  }

  .dot {
    position: absolute;
    width: 6px;
    height: 6px;
    margin: -3px 0 0 -3px;
    border-radius: 50%;
    background: var(--tooltip-bg);
    box-shadow: 0 0 0 1.5px var(--tooltip-text);
    pointer-events: none;
  }

  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--text-muted);
    pointer-events: none;
  }
</style>
