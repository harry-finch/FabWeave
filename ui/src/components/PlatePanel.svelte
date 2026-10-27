<script lang="ts">
  // Pièce sur le plateau, choisie par l'utilisateur et non par le modèle :
  // position du centre (centre du plateau par défaut), rotation, copies en
  // grille imprimées pièce par pièce (espacées du dégagement de la tête), et
  // adhésion (bordure ou radeau, réglages repliés sous le choix). Chaque
  // valeur modifiée a son ↺ ; un défilement du libellé est une seule étape
  // d'annulation. Le conseil d'adhésion du modèle est rappelé, jamais appliqué.
  import NumField from './NumField.svelte';
  import Segmented from './Segmented.svelte';
  import { BRIM, RAFT, store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { fmtLength, lengthUnit } from '../lib/format';
  import { ui } from '../lib/ui.svelte';
  import type { Adhesion, PrinterProfile } from '../lib/types';

  let { printer }: { printer: PrinterProfile } = $props();

  const p = $derived(store.doc.placement);
  const a = $derived(store.doc.adhesion);
  const centre = $derived<[number, number]>(
    printer.bed.origin === 'center' ? [0, 0] : [printer.bed.size[0] / 2, printer.bed.size[1] / 2],
  );
  const lo = $derived<[number, number]>([centre[0] - printer.bed.size[0] / 2, centre[1] - printer.bed.size[1] / 2]);
  const hi = $derived<[number, number]>([centre[0] + printer.bed.size[0] / 2, centre[1] + printer.bed.size[1] / 2]);
  const copies = $derived(p.copies[0] * p.copies[1]);
  const clearance = $derived(printer.nozzle.clearance_radius);
  const recommended = $derived(store.model?.meta.recommended_adhesion);
  const len = (v: number) => `${fmtLength(v, ui.units, 1)} ${lengthUnit(ui.units)}`;

  const begin = () => store.begin();
  const end = () => store.end();

  function setAdhesion(type: Adhesion['type']) {
    store.setAdhesion(type === 'brim' ? BRIM : type === 'raft' ? RAFT : { type: 'none' });
  }
</script>

<NumField
  label={t('plate.x')}
  value={p.x ?? centre[0]}
  min={lo[0]}
  max={hi[0]}
  step={1}
  decimals={1}
  onchange={(v) => store.setPlacement({ x: v }, 'x')}
  modified={p.x !== undefined}
  onreset={() => store.setPlacement({ x: undefined })}
  resetHint={t('plate.centreHint', { v: len(centre[0]) })}
  onbegin={begin}
  onend={end}
/>
<NumField
  label={t('plate.y')}
  value={p.y ?? centre[1]}
  min={lo[1]}
  max={hi[1]}
  step={1}
  decimals={1}
  onchange={(v) => store.setPlacement({ y: v }, 'y')}
  modified={p.y !== undefined}
  onreset={() => store.setPlacement({ y: undefined })}
  resetHint={t('plate.centreHint', { v: len(centre[1]) })}
  onbegin={begin}
  onend={end}
/>
<NumField
  label={t('plate.rotation')}
  kind="angle"
  value={p.rotation}
  min={-360}
  max={360}
  step={15}
  decimals={1}
  onchange={(v) => store.setPlacement({ rotation: v }, 'rotation')}
  modified={p.rotation !== 0}
  onreset={() => store.setPlacement({ rotation: 0 })}
  resetHint="0°"
  onbegin={begin}
  onend={end}
/>
<NumField
  label={t('plate.columns')}
  hint={t('plate.copiesHint')}
  kind="count"
  value={p.copies[0]}
  min={1}
  max={10}
  step={1}
  decimals={0}
  onchange={(v) => store.setPlacement({ copies: [Math.round(v), p.copies[1]] }, 'columns')}
  modified={p.copies[0] !== 1}
  onreset={() => store.setPlacement({ copies: [1, p.copies[1]] })}
  resetHint="1"
/>
<NumField
  label={t('plate.rows')}
  hint={t('plate.copiesHint')}
  kind="count"
  value={p.copies[1]}
  min={1}
  max={10}
  step={1}
  decimals={0}
  onchange={(v) => store.setPlacement({ copies: [p.copies[0], Math.round(v)] }, 'rows')}
  modified={p.copies[1] !== 1}
  onreset={() => store.setPlacement({ copies: [p.copies[0], 1] })}
  resetHint="1"
/>
{#if copies > 1}
  <NumField
    label={t('plate.spacing')}
    hint={clearance ? t('plate.spacingHint', { v: len(clearance) }) : undefined}
    value={p.spacing ?? clearance ?? 10}
    min={0}
    max={500}
    step={1}
    decimals={1}
    invalid={clearance !== undefined && (p.spacing ?? clearance) < clearance}
    onchange={(v) => store.setPlacement({ spacing: v }, 'spacing')}
    modified={p.spacing !== undefined}
    onreset={() => store.setPlacement({ spacing: undefined })}
    resetHint={clearance ? len(clearance) : undefined}
    onbegin={begin}
    onend={end}
  />
{/if}

<p class="sub">{t('plate.adhesion')}</p>
<Segmented
  label={t('plate.adhesion')}
  value={a.type}
  onchange={setAdhesion}
  options={[
    { id: 'none', label: t('plate.none') },
    { id: 'brim', label: t('plate.brim'), hint: t('plate.brimHint') },
    { id: 'raft', label: t('plate.raft'), hint: t('plate.raftHint') },
  ]}
/>
{#if recommended && recommended !== a.type}
  <p class="note">{t('plate.recommended', { what: t(`plate.${recommended}`) })}</p>
{/if}
{#if a.type === 'brim'}
  <NumField
    label={t('plate.brimWidth')}
    value={a.width}
    min={0.1}
    max={50}
    step={0.5}
    decimals={1}
    onchange={(v) => store.setAdhesion({ ...a, width: v }, 'width')}
    modified={a.width !== BRIM.width}
    onreset={() => store.setAdhesion({ ...a, width: BRIM.width })}
    onbegin={begin}
    onend={end}
  />
  <NumField
    label={t('plate.brimGap')}
    hint={t('plate.brimGapHint')}
    value={a.gap}
    min={0}
    max={5}
    step={0.05}
    decimals={2}
    onchange={(v) => store.setAdhesion({ ...a, gap: v }, 'gap')}
    modified={a.gap !== BRIM.gap}
    onreset={() => store.setAdhesion({ ...a, gap: BRIM.gap })}
    onbegin={begin}
    onend={end}
  />
{:else if a.type === 'raft'}
  {@const d = RAFT}
  <NumField
    label={t('plate.raftLayers')}
    kind="count"
    value={a.layers}
    min={1}
    max={10}
    step={1}
    decimals={0}
    onchange={(v) => store.setAdhesion({ ...a, layers: Math.round(v) }, 'layers')}
    modified={a.layers !== d.layers}
    onreset={() => store.setAdhesion({ ...a, layers: d.layers })}
  />
  <NumField
    label={t('plate.raftGap')}
    hint={t('plate.raftGapHint')}
    value={a.gap}
    min={0}
    max={1}
    step={0.05}
    decimals={2}
    onchange={(v) => store.setAdhesion({ ...a, gap: v }, 'gap')}
    modified={a.gap !== d.gap}
    onreset={() => store.setAdhesion({ ...a, gap: d.gap })}
    onbegin={begin}
    onend={end}
  />
  <NumField
    label={t('plate.raftMargin')}
    hint={t('plate.raftMarginHint')}
    value={a.margin}
    min={0}
    max={20}
    step={0.5}
    decimals={1}
    onchange={(v) => store.setAdhesion({ ...a, margin: v }, 'margin')}
    modified={a.margin !== d.margin}
    onreset={() => store.setAdhesion({ ...a, margin: d.margin })}
    onbegin={begin}
    onend={end}
  />
{/if}

<style>
  .sub {
    margin: var(--s1) 0 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }
</style>
