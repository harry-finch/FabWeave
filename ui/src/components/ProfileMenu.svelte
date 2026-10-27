<script lang="ts">
  // Menu « ⋯ » d'un profil (imprimante ou filament), à côté de sa liste :
  // modifier (un profil livré se copie d'abord), dupliquer, importer un
  // profil (FabWeave, PrusaSlicer, Bambu Studio, OrcaSlicer), exporter en
  // TOML, supprimer un profil de l'utilisateur (annulable).
  import { Copy, Download, Ellipsis, FileUp, Pencil, Trash2 } from '@lucide/svelte';
  import Popover from './Popover.svelte';
  import { store, type ProfileEntry } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import { ui } from '../lib/ui.svelte';
  import { text } from '../lib/text';
  import type { AnyProfile, FilamentProfile, PrinterProfile, ProfileKind } from '../lib/types';

  let {
    kind,
    onedit,
    onimport,
  }: {
    kind: ProfileKind;
    onedit: (entry: ProfileEntry<AnyProfile>, copy: boolean) => void;
    onimport: () => void;
  } = $props();

  const entry = $derived(
    ((kind === 'printer' ? store.printers : store.filaments) as ProfileEntry<AnyProfile>[]).find((e) => e.id === store.doc[kind]),
  );
  const user = $derived(entry?.source === 'user');
  const name = (e: ProfileEntry<AnyProfile>) =>
    e.profile ? text(kind === 'printer' ? (e.profile as PrinterProfile).printer.name : (e.profile as FilamentProfile).filament.name) : e.id;

  function exportToml() {
    const f = entry && store.exportProfile(kind, entry.id);
    if (!f) return;
    const url = URL.createObjectURL(new Blob([f.text], { type: 'application/toml' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = f.name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    ui.toast(t('export.done', { name: f.name }));
  }

  async function remove() {
    if (!entry) return;
    const label = name(entry);
    const removed = await store.deleteProfile(kind, entry.id);
    if (removed) ui.toast(t('pm.removed', { name: label }), { label: t('catalog.undo'), run: () => void store.restoreProfile(removed) });
  }
</script>

<Popover label={t('pm.menu')} width={250}>
  {#snippet trigger({ open, toggle })}
    <button class="btn quiet icon" class:open type="button" aria-label={t('pm.menu')} aria-haspopup="menu" aria-expanded={open} use:tip={t('pm.menu')} onclick={toggle}>
      <Ellipsis size={14} />
    </button>
  {/snippet}
  {#snippet content({ close })}
    <div role="menu">
      <button
        class="row"
        role="menuitem"
        type="button"
        disabled={!entry?.profile}
        onclick={() => {
          close();
          if (entry) onedit(entry, false);
        }}><Pencil size={14} /><span>{user ? t('pm.edit') : t('pm.copyEdit')}</span></button
      >
      {#if user}
        <button
          class="row"
          role="menuitem"
          type="button"
          disabled={!entry?.profile}
          onclick={() => {
            close();
            if (entry) onedit(entry, true);
          }}><Copy size={14} /><span>{t('pm.duplicate')}</span></button
        >
      {/if}
      <button
        class="row"
        role="menuitem"
        type="button"
        onclick={() => {
          close();
          onimport();
        }}><FileUp size={14} /><span>{t('pm.import')}</span></button
      >
      <button
        class="row"
        role="menuitem"
        type="button"
        disabled={!entry}
        onclick={() => {
          close();
          exportToml();
        }}><Download size={14} /><span>{t('pm.export')}</span></button
      >
      {#if user}
        <div class="divider"></div>
        <button
          class="row"
          role="menuitem"
          type="button"
          onclick={() => {
            close();
            void remove();
          }}><Trash2 size={14} /><span>{t('pm.remove')}</span></button
        >
      {/if}
    </div>
  {/snippet}
</Popover>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    padding: 6px var(--s2);
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text-secondary);
    text-align: left;
  }

  .row:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .row:disabled {
    opacity: 0.45;
  }

  .divider {
    height: 1px;
    margin: var(--s1) 0;
    background: var(--divider);
  }
</style>
