<script lang="ts">
  // Paramètre `curve` : la silhouette (rayon à l'horizontale, hauteur à la
  // verticale, à la même échelle) dans le cadre des bornes du modèle ; le
  // rayon 0, s'il est dans le cadre, est marqué en pointillés.
  // - Glisser un point le déplace (un geste = une étape d'annulation) ; il
  //   reste dans le cadre et entre ses voisins en hauteur.
  // - Clic près du trait : ajoute un point ; double-clic sur un point : le
  //   retire ; flèches sur le point choisi : ± 0,5 mm (Maj × 10).
  // - Le point choisi se règle aussi au chiffre ; une valeur hors bornes saisie
  //   au clavier n'est pas corrigée : le moteur la signale.
  // - Galerie : silhouettes prêtes du modèle ; ↺ revient à celle du modèle.
  import { Info, RotateCcw } from '@lucide/svelte';
  import NumField from './NumField.svelte';
  import { sample, type Pt } from '../lib/curve';
  import { tip } from '../lib/tip';
  import { t } from '../lib/i18n.svelte';
  import { text } from '../lib/text';
  import type { ParamDef } from '../lib/types';

  type CurveDef = Extract<ParamDef, { type: 'curve' }>;

  let {
    def,
    value,
    invalid = false,
    modified = false,
    onchange,
    onreset,
    onbegin,
    onend,
  }: {
    def: CurveDef;
    value: Pt[];
    invalid?: boolean;
    modified?: boolean;
    onchange: (v: Pt[]) => void;
    onreset: () => void;
    onbegin?: () => void;
    onend?: () => void;
  } = $props();

  /** Largeur utile du tracé (px) et hauteur au plus. */
  const W = 248;
  const H_MAX = 300;
  const PAD = 8;
  /** Écart minimal entre deux hauteurs voisines lors d'un glisser (mm). */
  const GAP = 0.5;

  const [r0, z0] = $derived(def.min);
  const [r1, z1] = $derived(def.max);
  /** px par mm, la même dans les deux sens. */
  const scale = $derived(Math.min(W / (r1 - r0), H_MAX / (z1 - z0)));
  const w = $derived((r1 - r0) * scale);
  const h = $derived((z1 - z0) * scale);
  const sx = (r: number) => PAD + (r - r0) * scale;
  const sy = (z: number) => PAD + h - (z - z0) * scale;

  let selected = $state(0);
  const sel = $derived(Math.min(selected, value.length - 1));
  let svg = $state<SVGSVGElement>();
  let dragging = -1;

  const line = (pts: Pt[]) => pts.map((p, i) => `${i ? 'L' : 'M'}${sx(p[0]).toFixed(1)},${sy(p[1]).toFixed(1)}`).join('');
  const drawn = $derived(sample(value, def.smooth));

  /** Pas de la grille (mm) : environ 6 à 12 lignes. */
  const grid = $derived([5, 10, 20, 50, 100].find((g) => Math.max(r1 - r0, z1 - z0) / g <= 12) ?? 100);
  const ticks = (a: number, b: number) => Array.from({ length: Math.floor(b / grid) - Math.ceil(a / grid) + 1 }, (_, i) => (Math.ceil(a / grid) + i) * grid);

  const round = (v: number) => Math.round(v * 10) / 10;

  function toMm(e: PointerEvent | MouseEvent): Pt {
    const b = svg!.getBoundingClientRect();
    const x = ((e.clientX - b.left) / b.width) * (w + 2 * PAD);
    const y = ((e.clientY - b.top) / b.height) * (h + 2 * PAD);
    return [r0 + (x - PAD) / scale, z0 + (h + PAD - y) / scale];
  }

  /** Point `i` déplacé en `p`, gardé dans le cadre et entre ses voisins. */
  function moved(i: number, [r, z]: Pt): Pt[] {
    const lo = i > 0 ? value[i - 1][1] + GAP : z0;
    const hi = i < value.length - 1 ? value[i + 1][1] - GAP : z1;
    const next = value.map((q) => [...q] as Pt);
    next[i] = [round(Math.min(r1, Math.max(r0, r))), round(Math.min(hi, Math.max(lo, z)))];
    return next;
  }

  function down(e: PointerEvent, i: number) {
    e.stopPropagation();
    selected = i;
    dragging = i;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    onbegin?.();
  }

  function move(e: PointerEvent) {
    if (dragging < 0) return;
    onchange(moved(dragging, toMm(e)));
  }

  function up() {
    if (dragging < 0) return;
    dragging = -1;
    onend?.();
  }

  /** Clic près du trait : un point de plus, sur la courbe, à cette hauteur. */
  function add(e: MouseEvent) {
    // Fin d'un glisser sur un point : pas un clic sur le trait.
    if (value.length >= def.points[1] || (e.target as Element).tagName === 'circle') return;
    const [, z] = toMm(e);
    const [r] = toMm(e);
    const near = drawn.reduce((best, p) => Math.min(best, Math.hypot(sx(p[0]) - sx(r), sy(p[1]) - sy(z))), Infinity);
    if (near > 10 || z <= value[0][1] || z >= value[value.length - 1][1]) return;
    const on = drawn.reduce((a, b) => (Math.abs(b[1] - z) < Math.abs(a[1] - z) ? b : a))[0];
    const i = value.findIndex((q) => q[1] > z);
    if (z - value[i - 1][1] < GAP || value[i][1] - z < GAP) return;
    onchange([...value.slice(0, i), [round(on), round(z)], ...value.slice(i)]);
    selected = i;
  }

  function remove(i: number) {
    if (value.length <= def.points[0]) return;
    onchange(value.filter((_, k) => k !== i));
    selected = Math.max(0, i - 1);
  }

  const ARROWS: Record<string, Pt> = { ArrowRight: [1, 0], ArrowLeft: [-1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] };

  function key(e: KeyboardEvent, i: number) {
    const d = ARROWS[e.key];
    if (d) {
      e.preventDefault();
      const k = e.shiftKey ? 5 : 0.5;
      onchange(moved(i, [value[i][0] + d[0] * k, value[i][1] + d[1] * k]));
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      remove(i);
    }
  }

  function setPoint(axis: 0 | 1, v: number) {
    const next = value.map((q) => [...q] as Pt);
    next[sel][axis] = v;
    onchange(next);
  }
</script>

<div class="curve" class:invalid>
  <div class="head">
    <span class="label">
      {text(def.label)}
      {#if def.help}<span class="hint" use:tip={text(def.help)} tabindex="-1"><Info size={11} /></span>{/if}
    </span>
    {#if modified}
      <button class="btn quiet icon reset" type="button" aria-label={t('curve.reset')} use:tip={t('curve.reset')} onclick={onreset}><RotateCcw size={12} /></button>
    {/if}
    {#if def.presets?.length}
      <select
        class="input select gallery"
        aria-label={t('curve.gallery')}
        value=""
        onchange={(e) => {
          const el = e.currentTarget as HTMLSelectElement;
          const p = def.presets![Number(el.value)];
          el.value = '';
          if (p) onchange(p.points.map((q) => [...q] as Pt));
        }}
      >
        <option value="" disabled>{t('curve.gallery')}</option>
        {#each def.presets as p, i (i)}<option value={i}>{text(p.name)}</option>{/each}
      </select>
    {/if}
  </div>

  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <svg
    bind:this={svg}
    class="plot"
    viewBox="0 0 {w + 2 * PAD} {h + 2 * PAD}"
    width={w + 2 * PAD}
    height={h + 2 * PAD}
    role="img"
    aria-label={text(def.label)}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onclick={add}
  >
    <rect class="frame" x={PAD} y={PAD} width={w} height={h} />
    {#each ticks(r0, r1) as g (g)}<line class={g === 0 ? 'axis' : 'grid'} x1={sx(g)} x2={sx(g)} y1={PAD} y2={PAD + h} />{/each}
    {#each ticks(z0, z1) as g (g)}<line class="grid" x1={PAD} x2={PAD + w} y1={sy(g)} y2={sy(g)} />{/each}
    <path class="trace" d={line(drawn)} />
    {#each value as p, i (i)}
      <circle
        class="pt"
        class:on={i === sel}
        cx={sx(p[0])}
        cy={sy(p[1])}
        r="4.5"
        role="slider"
        tabindex="0"
        aria-label={t('curve.point', { n: i + 1 })}
        aria-valuenow={p[0]}
        aria-valuetext="{p[0]} mm · {p[1]} mm"
        onpointerdown={(e) => down(e, i)}
        ondblclick={(e) => {
          e.stopPropagation();
          remove(i);
        }}
        onfocus={() => (selected = i)}
        onkeydown={(e) => key(e, i)}
      />
    {/each}
  </svg>

  {#if value[sel]}
    <NumField label={t('curve.radius', { n: sel + 1 })} value={value[sel][0]} kind="length" decimals={1} onchange={(v) => setPoint(0, v)} />
    <NumField label={t('curve.height', { n: sel + 1 })} value={value[sel][1]} kind="length" decimals={1} onchange={(v) => setPoint(1, v)} />
  {/if}
  <p class="note">{t('curve.howto', { n: value.length, min: def.points[0], max: def.points[1] })}</p>
</div>

<style>
  .curve {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--s1);
    min-height: var(--control-h);
  }

  .label {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .hint {
    display: inline-flex;
    color: var(--text-muted);
  }

  .gallery {
    width: auto;
    max-width: 150px;
  }

  .plot {
    display: block;
    max-width: 100%;
    height: auto;
    align-self: center;
    background: var(--bg-workspace);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    touch-action: none;
    cursor: crosshair;
  }

  .invalid .plot {
    border-color: var(--danger);
  }

  .frame {
    fill: none;
    stroke: var(--border);
  }

  .grid {
    stroke: var(--divider);
  }

  .axis {
    stroke: var(--border-strong);
    stroke-dasharray: 3 3;
  }

  .trace {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
  }

  .pt {
    fill: var(--bg-control);
    stroke: var(--accent);
    stroke-width: 1.5;
    cursor: grab;
  }

  .pt.on {
    fill: var(--accent);
  }

  .pt:focus-visible {
    outline: none;
    stroke: var(--accent-text);
    stroke-width: 3;
  }

  .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }
</style>
