<script lang="ts">
  // Import de profils depuis un trancheur (PrusaSlicer `.ini`, Bambu Studio
  // ou OrcaSlicer `.json`) : pour chaque profil trouvé, ce qui a été repris,
  // ce qui manquait (et d'où vient la valeur gardée), ce qui est ignoré, et
  // les gabarits à relire. Rien n'est ajouté avant « Ajouter » ; un profil
  // aux valeurs fautives s'ajoute quand même, pour être corrigé dans
  // l'éditeur (il ne se charge pas avant).
  import { untrack } from 'svelte';
  import { Disc3, Printer } from '@lucide/svelte';
  import Dialog from './Dialog.svelte';
  import Alerts from './Alerts.svelte';
  import Disclosure from './Disclosure.svelte';
  import { store } from '../lib/store.svelte';
  import { t, type Key } from '../lib/i18n.svelte';
  import { text } from '../lib/text';
  import { profileFieldLabel, toAlerts } from '../lib/issues';
  import { ui } from '../lib/ui.svelte';
  import type { FilamentProfile, ImportedItem, PrinterProfile } from '../lib/types';

  let { file, items, onclose }: { file: string; items: ImportedItem[]; onclose: () => void } = $props();

  let chosen = $state(untrack(() => items.map(() => true)));
  const count = $derived(chosen.filter(Boolean).length);

  const nameOf = (it: ImportedItem) => text(it.kind === 'printer' ? (it.profile as PrinterProfile).printer.name : (it.profile as FilamentProfile).filament.name);
  const field = profileFieldLabel;
  function baseName(it: ImportedItem) {
    const list = it.kind === 'printer' ? store.printers : store.filaments;
    const p = list.find((e) => e.id === it.report.base)?.profile;
    return p ? text(it.kind === 'printer' ? (p as PrinterProfile).printer.name : (p as FilamentProfile).filament.name) : it.report.base;
  }

  async function add() {
    const picked = items.filter((_, i) => chosen[i]);
    await store.addImported(file, picked);
    ui.toast(t(picked.length > 1 ? 'imp.addedMany' : 'imp.added', { n: picked.length, name: picked[0] ? nameOf(picked[0]) : '' }));
    onclose();
  }
</script>

<Dialog title={t('imp.title', { file })} width={560} {onclose}>
  <p class="note">{t('imp.intro')}</p>
  {#each items as it, i (i)}
    {@const r = it.report}
    {@const errors = it.issues.filter((x) => x.severity === 'error')}
    <section class="item">
      <label class="head">
        <input type="checkbox" bind:checked={chosen[i]} />
        {#if it.kind === 'printer'}<Printer size={14} />{:else}<Disc3 size={14} />{/if}
        <span class="name">{nameOf(it)}</span>
        <span class="kind">{t(it.kind === 'printer' ? 'imp.printer' : 'imp.filament')}</span>
      </label>
      <p class="summary">
        {t('imp.summary', { slicer: r.slicer, taken: r.taken.length, missing: r.missing.length, ignored: r.ignored.length })}
      </p>
      <p class="note">{t('imp.base', { base: baseName(it) })}</p>
      {#if r.inherits}
        <p class="warn">{t('imp.inherits', { name: r.inherits })}</p>
      {/if}
      {#if it.kind === 'printer'}
        <p class="warn">{t('imp.review')}</p>
      {/if}
      {#if r.untranslated.length}
        <p class="warn">{t('imp.untranslated', { n: r.untranslated.length })}</p>
        <ul class="list mono">
          {#each r.untranslated as u, k (k)}<li>{field(u.field)}, {t('issues.line', { line: u.line })} : <code>{u.text}</code> — {u.reason}</li>{/each}
        </ul>
      {/if}
      {#if r.ignored_gcode.length}
        <p class="note">{t('imp.ignoredGcode', { list: r.ignored_gcode.join(', ') })}</p>
      {/if}
      {#if errors.length}
        <p class="warn">{t('imp.errors')}</p>
        <Alerts alerts={toAlerts(errors, null)} />
      {/if}
      <Disclosure id={`import.detail`} label={t('imp.detail')}>
        <h3>{t('imp.taken')}</h3>
        <ul class="list">
          {#each r.taken as x, k (k)}<li>{field(x.field)} ← <code>{x.key}</code> = {x.value}</li>{/each}
        </ul>
        {#if r.missing.length}
          <h3>{t('imp.missing')}</h3>
          <ul class="list">
            {#each r.missing as m, k (k)}<li>{field(m.field)} : {t(`imp.fallback.${m.fallback}` as Key, { base: baseName(it) })}</li>{/each}
          </ul>
        {/if}
        {#if r.constants.length}
          <h3>{t('imp.constants')}</h3>
          <ul class="list">
            {#each r.constants as c, k (k)}<li><code>{c.name}</code> = « {c.value} » ({field(c.field)})</li>{/each}
          </ul>
        {/if}
        {#if r.ignored.length}
          <h3>{t('imp.ignored', { n: r.ignored.length })}</h3>
          <p class="keys">{r.ignored.join(', ')}</p>
        {/if}
      </Disclosure>
    </section>
  {/each}

  {#snippet footer({ close })}
    <button class="btn" type="button" onclick={close}>{t('common.cancel')}</button>
    <button class="btn primary" type="button" disabled={count === 0} onclick={add}>{t('imp.add', { n: count })}</button>
  {/snippet}
</Dialog>

<style>
  .item {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    padding: var(--s2) 0;
    border-top: 1px solid var(--divider);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--s2);
    color: var(--text-primary);
    cursor: pointer;
  }

  .name {
    font-weight: 600;
  }

  .kind {
    margin-left: auto;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .summary {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .warn {
    margin: 0;
    padding: 4px var(--s2);
    border-radius: var(--radius);
    background: var(--warning-tint);
    color: var(--text-primary);
    font-size: var(--fs-sm);
  }

  .list {
    margin: 0;
    padding-left: var(--s4);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  code,
  .keys {
    font-family: var(--mono);
    font-size: var(--fs-xs);
  }

  .keys {
    margin: 0;
    color: var(--text-muted);
    word-break: break-word;
  }
</style>
