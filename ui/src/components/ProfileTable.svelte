<script lang="ts">
  // Petite table d'un profil (zones exclues, couches ou débits par buse) :
  // une ligne par entrée, en-têtes en petites capitales, + pour ajouter, −
  // pour retirer. Nombres saisis selon la langue (virgule en français),
  // validés à la sortie du champ ; une cellule facultative vide retire la
  // valeur. Les erreurs viennent du contrôle du profil par le moteur.
  import { Minus, Plus } from '@lucide/svelte';
  import { i18n, intlLocale, t, type Key } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import { getPath, setPath, type Column } from '../lib/profileFields';
  import { text as textOf } from '../lib/text';

  let {
    label,
    prefix,
    columns,
    rows,
    onchange,
    invalid = false,
  }: {
    label: string;
    /** Préfixe des clés de traduction des colonnes (`pf.bed.exclude`). */
    prefix: string;
    columns: Column[];
    rows: Record<string, unknown>[];
    onchange: (rows: Record<string, unknown>[]) => void;
    invalid?: boolean;
  } = $props();

  const fmt = (v: unknown) =>
    typeof v === 'number' ? new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: 4, useGrouping: false }).format(v) : '';

  function parse(raw: string): number | undefined {
    const s = raw.trim().replace(/\s/g, '');
    if (s === '') return undefined;
    const n = Number(i18n.locale === 'fr' ? s.replace(',', '.') : s.replace(/,/g, ''));
    return Number.isFinite(n) ? n : NaN;
  }

  function cell(r: number, c: Column, raw: string, input: HTMLInputElement | HTMLSelectElement) {
    const next = rows.map((x) => structuredClone($state.snapshot(x)) as Record<string, unknown>);
    if (c.type === 'text') setPath(next[r], c.key, raw);
    else if (c.type === 'tri') setPath(next[r], c.key, raw === '' ? undefined : raw === 'yes');
    else {
      const v = parse(raw);
      if (Number.isNaN(v) || (v === undefined && !c.optional)) {
        input.value = fmt(getPath(rows[r], c.key));
        return;
      }
      setPath(next[r], c.key, v);
    }
    onchange(next);
  }

  function add() {
    const row: Record<string, unknown> = {};
    for (const c of columns) {
      if (c.type === 'text') setPath(row, c.key, `${t('pe.zone')} ${rows.length + 1}`);
      else if (c.type === 'num' && !c.optional) setPath(row, c.key, 0);
    }
    onchange([...rows.map((x) => $state.snapshot(x) as Record<string, unknown>), row]);
  }

  const remove = (r: number) => onchange(rows.filter((_, i) => i !== r).map((x) => $state.snapshot(x) as Record<string, unknown>));
  const head = (c: Column) => t(`${prefix}.${c.key.replace('.', '_')}` as Key);
</script>

<div class="table" class:bad={invalid}>
  <div class="top">
    <span class="title">{label}</span>
    <button class="btn quiet icon" type="button" aria-label={t('pe.addRow')} use:tip={t('pe.addRow')} onclick={add}><Plus size={13} /></button>
  </div>
  {#if rows.length}
    <div class="grid" style:grid-template-columns={`${columns.map((c) => (c.type === 'text' ? 'minmax(0, 2fr)' : 'minmax(0, 1fr)')).join(' ')} 24px`}>
      {#each columns as c (c.key)}<span class="h">{head(c)}</span>{/each}
      <span></span>
      {#each rows as row, r (r)}
        {#each columns as c (c.key)}
          {#if c.type === 'tri'}
            {@const v = getPath(row, c.key)}
            <select class="input" aria-label={head(c)} value={v === undefined ? '' : v ? 'yes' : 'no'} onchange={(e) => cell(r, c, e.currentTarget.value, e.currentTarget)}>
              <option value="">{t('pe.any')}</option>
              <option value="yes">{t('model.on')}</option>
              <option value="no">{t('model.off')}</option>
            </select>
          {:else if c.type === 'text'}
            {@const v = getPath(row, c.key)}
            <input class="input" aria-label={head(c)} value={v === undefined ? '' : textOf(v as string)} onchange={(e) => cell(r, c, e.currentTarget.value, e.currentTarget)} />
          {:else}
            <input
              class="input tabular num"
              aria-label={head(c)}
              inputmode="decimal"
              placeholder={c.optional ? '—' : ''}
              value={fmt(getPath(row, c.key))}
              onchange={(e) => cell(r, c, e.currentTarget.value, e.currentTarget)}
            />
          {/if}
        {/each}
        <button class="btn quiet icon" type="button" aria-label={t('pe.removeRow')} use:tip={t('pe.removeRow')} onclick={() => remove(r)}><Minus size={13} /></button>
      {/each}
    </div>
  {:else}
    <p class="empty">{t('pe.noRow')}</p>
  {/if}
</div>

<style>
  .table {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    padding-left: 0;
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: var(--control-h);
  }

  .title {
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .bad .title {
    color: var(--danger-text);
  }

  .grid {
    display: grid;
    gap: 4px var(--s1);
    align-items: center;
  }

  .h {
    color: var(--text-muted);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .input {
    width: 100%;
    min-width: 0;
  }

  .num {
    text-align: right;
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
</style>
