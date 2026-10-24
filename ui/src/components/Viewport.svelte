<script lang="ts">
  // Vue 3D du parcours (ADR-0003) : plateau de la machine à sa taille, grille
  // de 10 mm, parcours en lignes dégradées selon la hauteur, déplacements en
  // retrait. Orbite (glisser), déplacement (clic droit, Maj + glisser), zoom
  // vers le curseur (molette), cadrage `F`. La vue se cadre seule sur chaque
  // calcul tant que l'utilisateur ne l'a pas bougée ; changer de modèle
  // (`fitKey`) rétablit le cadrage automatique. Rendu à la demande seulement.
  // Couleurs : jetons `--view-*` et `--path-*`, relus au changement de thème.
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
  import { Focus } from '@lucide/svelte';
  import { theme } from '../lib/theme.svelte';
  import { tip } from '../lib/tip';
  import { t } from '../lib/i18n.svelte';
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
  const printMat = new THREE.LineBasicMaterial({ vertexColors: true });
  const travelMat = new THREE.LineBasicMaterial();
  const printLines = new THREE.LineSegments(new THREE.BufferGeometry(), printMat);
  const travelLines = new THREE.LineSegments(new THREE.BufferGeometry(), travelMat);
  scene.add(bed, printLines, travelLines);

  const TOKENS = ['view-bg', 'view-bed', 'view-bed-edge', 'view-grid', 'path-low', 'path-high', 'path-travel'] as const;
  let colors: Record<(typeof TOKENS)[number], THREE.Color> = Object.fromEntries(TOKENS.map((k) => [k, new THREE.Color()])) as never;

  function readColors() {
    const cs = getComputedStyle(document.documentElement);
    for (const k of TOKENS) colors[k] = new THREE.Color(cs.getPropertyValue(`--${k}`).trim() || '#808080');
    scene.background = colors['view-bg'];
    bedFill.color = colors['view-bed'];
    bedEdge.color = colors['view-bed-edge'];
    grid.color = colors['view-grid'];
    travelMat.color = colors['path-travel'];
  }

  function render() {
    if (!renderer || pending) return;
    pending = requestAnimationFrame(() => {
      pending = 0;
      renderer?.render(scene, camera);
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
    const plane = new THREE.Mesh(new THREE.PlaneGeometry(w, h), bedFill);
    plane.position.set(x0 + w / 2, y0 + h / 2, -0.05);
    const lines: number[] = [];
    for (let x = 10; x < w; x += 10) lines.push(x0 + x, y0, 0, x0 + x, y0 + h, 0);
    for (let y = 10; y < h; y += 10) lines.push(x0, y0 + y, 0, x0 + w, y0 + y, 0);
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(lines, 3));
    const edge = new THREE.BufferGeometry();
    edge.setAttribute('position', new THREE.Float32BufferAttribute([x0, y0, 0, x0 + w, y0, 0, x0 + w, y0 + h, 0, x0, y0 + h, 0, x0, y0, 0], 3));
    bed.add(plane, new THREE.LineSegments(g, grid), new THREE.Line(edge, bedEdge));
  }

  function buildPath(f: FrameData | null) {
    printLines.geometry.dispose();
    travelLines.geometry.dispose();
    const pg = new THREE.BufferGeometry();
    const tg = new THREE.BufferGeometry();
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
      for (let i = 1; i < n; i++) (f.kinds[i] === 0 ? travels : prints).push(i - 1, i);
      pg.setAttribute('position', pos);
      pg.setAttribute('color', new THREE.BufferAttribute(col, 3));
      pg.setIndex(prints);
      tg.setAttribute('position', pos);
      tg.setIndex(travels);
    }
    printLines.geometry = pg;
    travelLines.geometry = tg;
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
  <div class="tools">
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
