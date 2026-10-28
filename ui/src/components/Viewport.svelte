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
  // dégradé que le parcours.
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
  import { Focus, MessageSquareText } from '@lucide/svelte';
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
    showTravel = true,
  }: {
    frame: FrameData | null;
    printer: PrinterProfile | null;
    fitKey: string;
    showTravel?: boolean;
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
  // Gouttes : une sphère de diamètre 1 par goutte, mise à l'échelle.
  const blobGeom = new THREE.IcosahedronGeometry(0.5, 1);
  const blobMat = new THREE.MeshBasicMaterial();
  let blobs: THREE.InstancedMesh | null = null;

  const TOKENS = ['view-bg', 'view-bed', 'view-bed-edge', 'view-grid', 'view-bed-exclude', 'path-low', 'path-high', 'path-travel', 'danger'] as const;
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
    printLines.geometry.dispose();
    travelLines.geometry.dispose();
    outsideLines.geometry.dispose();
    const pg = new THREE.BufferGeometry();
    const tg = new THREE.BufferGeometry();
    const og = new THREE.BufferGeometry();
    const n = f ? f.kinds.length : 0;
    if (f && n > 1) {
      const pos = new THREE.BufferAttribute(f.positions, 3);
      // Dégradé selon Z, de --path-low à --path-high.
      let zmin = Infinity;
      let zmax = -Infinity;
      for (let i = 0; i < n; i++) {
        const z = f.positions[i * 3 + 2];
        zmin = Math.min(zmin, z);
        zmax = Math.max(zmax, z);
      }
      const span = zmax - zmin || 1;
      const col = new Float32Array(n * 3);
      const lo = colors['path-low'];
      const hi = colors['path-high'];
      for (let i = 0; i < n; i++) {
        const k = (f.positions[i * 3 + 2] - zmin) / span;
        col[i * 3] = lo.r + (hi.r - lo.r) * k;
        col[i * 3 + 1] = lo.g + (hi.g - lo.g) * k;
        col[i * 3 + 2] = lo.b + (hi.b - lo.b) * k;
      }
      const prints: number[] = [];
      const travels: number[] = [];
      const outside: number[] = [];
      for (let i = 1; i < n; i++) (f.kinds[i] === 0 ? travels : f.outside[i] ? outside : prints).push(i - 1, i);
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
    buildBlobs(f);
  }

  function buildBlobs(f: FrameData | null) {
    if (blobs) {
      scene.remove(blobs);
      blobs.dispose();
      blobs = null;
    }
    if (!f) return;
    const idx: number[] = [];
    for (let i = 1; i < f.kinds.length; i++) if (f.kinds[i] === 3) idx.push(i);
    if (!idx.length) return;
    let zmin = Infinity;
    let zmax = -Infinity;
    for (let i = 0; i < f.kinds.length; i++) {
      zmin = Math.min(zmin, f.positions[i * 3 + 2]);
      zmax = Math.max(zmax, f.positions[i * 3 + 2]);
    }
    const span = zmax - zmin || 1;
    blobs = new THREE.InstancedMesh(blobGeom, blobMat, idx.length);
    const m = new THREE.Matrix4();
    const c = new THREE.Color();
    idx.forEach((i, k) => {
      const d = f.widths[i] || 1;
      // La goutte s'étale sous la buse : centre à une demi-goutte sous le point.
      m.makeScale(d, d, d).setPosition(f.positions[i * 3], f.positions[i * 3 + 1], f.positions[i * 3 + 2] - d / 2);
      blobs!.setMatrixAt(k, m);
      if (f.outside[i]) c.copy(colors['danger']);
      else c.copy(colors['path-low']).lerp(colors['path-high'], (f.positions[i * 3 + 2] - zmin) / span);
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

  export function refit() {
    userMoved = false;
    fit();
  }

  onMount(() => {
    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    host.appendChild(renderer.domElement);
    controls = new OrbitControls(camera, renderer.domElement);
    controls.screenSpacePanning = true;
    controls.zoomToCursor = true;
    controls.addEventListener('change', render);
    controls.addEventListener('start', () => (userMoved = true));
    // Mesure du coût d'un rendu en développement (pas dans le build).
    if (import.meta.env.DEV) (window as unknown as Record<string, unknown>).__fabweaveView = { renderer, scene, camera, controls };
    const ro = new ResizeObserver(() => {
      const { clientWidth: w, clientHeight: h } = host;
      if (!w || !h) return;
      renderer!.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      if (!userMoved) fit();
      render();
    });
    ro.observe(host);
    return () => {
      ro.disconnect();
      controls?.dispose();
      renderer?.dispose();
      renderer?.domElement.remove();
      renderer = null;
    };
  });

  $effect(() => {
    void theme.applied;
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
