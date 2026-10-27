<script lang="ts">
  // Catalogue des modèles : sélecteur riche (Popover). Déclencheur : vignette
  // et nom du modèle affiché. Panneau : recherche (nom, description,
  // catégorie, étiquettes), modèles groupés par catégorie, puis « Mes
  // modèles » (fichiers ouverts, retirables) ; pied : ouvrir un fichier
  // `.weave` (⌘O) ou le déposer dans la fenêtre. Un modèle illisible reste
  // listé, marqué, pour que ses erreurs s'affichent avec leur ligne.
  import { Check, ChevronDown, CircleX, FolderOpen, Search, Trash2 } from '@lucide/svelte';
  import Popover from './Popover.svelte';
  import { store, type CatalogEntry } from '../lib/store.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';
  import { text } from '../lib/text';
  import { tip } from '../lib/tip';
  import { theme } from '../lib/theme.svelte';
  import { untrack } from 'svelte';

  let { onopen }: { onopen: () => void } = $props();

  let query = $state('');
  let search = $state<HTMLInputElement>();

  // Vignettes dès que le moteur est prêt, puis à chaque thème et à chaque modèle ajouté.
  $effect(() => {
    void [theme.applied, store.status, store.printer, store.catalog.length];
    untrack(() => void store.loadThumbs());
  });

  function remove(e: CatalogEntry) {
    const back = store.removeFile(e.id);
    if (back) ui.toast(t('catalog.removed', { name: name(e) }), { label: t('catalog.undo'), run: () => void store.openFile(back.file, back.text) });
  }

  const current = $derived(store.catalog.find((e) => e.id === store.modelId));

  const name = (e: CatalogEntry) => (e.info ? text(e.info.meta.name) : (e.file ?? e.id));

  function matches(e: CatalogEntry, q: string): boolean {
    if (!q) return true;
    const m = e.info?.meta;
    const hay = [name(e), m ? text(m.category) : '', m?.description ? text(m.description) : '', ...(m?.tags ?? []), e.file ?? '']
      .join(' ')
      .toLowerCase()
      .normalize('NFD')
      .replace(/\p{Diacritic}/gu, '');
    return q
      .toLowerCase()
      .normalize('NFD')
      .replace(/\p{Diacritic}/gu, '')
      .split(/\s+/)
      .every((w) => hay.includes(w));
  }

  /** Groupes affichés : catégories des modèles livrés, puis « Mes modèles ». */
  const groups = $derived.by(() => {
    const q = query.trim();
    const out = new Map<string, CatalogEntry[]>();
    for (const e of store.catalog.filter((e) => e.source === 'builtin' && matches(e, q))) {
      const cat = e.info ? text(e.info.meta.category) : t('model.unreadable');
      out.set(cat, [...(out.get(cat) ?? []), e]);
    }
    const mine = store.catalog.filter((e) => e.source === 'user' && matches(e, q));
    const list = [...out.entries()];
    if (mine.length) list.push([t('catalog.mine'), mine]);
    return list;
  });
  const empty = $derived(groups.length === 0);
</script>

<Popover label={t('model.catalog')} width={330}>
  {#snippet trigger({ open, toggle })}
    <button
      class="pick input"
      class:open
      type="button"
      aria-haspopup="menu"
      aria-expanded={open}
      onclick={async () => {
        toggle();
        query = '';
        setTimeout(() => search?.focus(), 30);
      }}
    >
      {#if store.thumbs[store.modelId]}
        <img class="thumb small" src={store.thumbs[store.modelId]} alt="" />
      {:else}
        <span class="thumb small blank">{#if (current && !current.info) || store.thumbs[store.modelId] === ''}<CircleX size={12} />{/if}</span>
      {/if}
      <span class="name">{current ? name(current) : '—'}</span>
      <ChevronDown size={13} />
    </button>
  {/snippet}
  {#snippet content({ close })}
    <label class="search">
      <Search size={13} />
      <input bind:this={search} bind:value={query} type="search" placeholder={t('catalog.search')} aria-label={t('catalog.search')} />
    </label>
    {#if empty}
      <p class="none">{t('catalog.noMatch')}</p>
    {/if}
    {#each groups as [group, list] (group)}
      <div class="group">{group}</div>
      {#each list as e (e.id)}
        <div class="line" class:selected={e.id === store.modelId}>
          <button
            class="row"
            type="button"
            onclick={() => {
              store.select('model', e.id);
              close();
            }}
          >
            {#if store.thumbs[e.id]}
              <img class="thumb" src={store.thumbs[e.id]} alt="" />
            {:else}
              <span class="thumb blank">{#if !e.info || store.thumbs[e.id] === ''}<CircleX size={14} />{/if}</span>
            {/if}
            <span class="txt">
              <span class="rname">{name(e)}</span>
              <span class="desc">{e.info?.meta.description ? text(e.info.meta.description) : !e.info ? t('model.unreadable') : ''}</span>
            </span>
            {#if e.id === store.modelId}<Check size={14} />{/if}
          </button>
          {#if e.source === 'user'}
            <button class="btn quiet icon remove" type="button" aria-label={t('catalog.remove')} use:tip={t('catalog.remove')} onclick={() => remove(e)}>
              <Trash2 size={13} />
            </button>
          {/if}
        </div>
      {/each}
    {/each}
    <div class="divider"></div>
    <button
      class="row menu"
      type="button"
      onclick={() => {
        close();
        onopen();
      }}
    >
      <FolderOpen size={14} />
      <span class="mname">{t('catalog.open')}</span>
      <span class="kbd">⌘O</span>
    </button>
    <p class="foot">{t('catalog.drop')}</p>
  {/snippet}
</Popover>

<style>
  .pick {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    height: 36px;
    padding: 0 var(--s2) 0 4px;
    color: var(--text-primary);
    text-align: left;
  }

  .pick.open {
    border-color: var(--accent);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .thumb {
    flex: none;
    width: 44px;
    height: 44px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    display: grid;
    place-items: center;
    color: var(--danger);
  }

  .thumb.small {
    width: 28px;
    height: 28px;
  }

  .blank {
    background: var(--view-bg);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    height: var(--control-h);
    margin: 0 0 4px;
    padding: 0 var(--s2);
    background: var(--bg-control);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-muted);
  }

  .search:focus-within {
    border-color: var(--accent);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text-primary);
  }

  .group {
    margin: 8px 8px 4px;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .line {
    display: flex;
    align-items: center;
    border-radius: var(--radius);
  }

  .line.selected {
    background: var(--bg-selected);
  }

  .row {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: 4px var(--s2) 4px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
    color: var(--accent-text);
  }

  .row:hover {
    background: var(--bg-hover);
  }

  .txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .rname {
    color: var(--text-primary);
    font-weight: 500;
  }

  .desc {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .remove {
    flex: none;
    margin-right: 2px;
  }

  .menu {
    color: var(--text-secondary);
    padding: 6px var(--s2);
  }

  .mname {
    flex: 1;
    color: var(--text-primary);
    font-weight: 500;
  }

  .kbd,
  .foot,
  .none {
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .foot,
  .none {
    margin: 2px 8px 4px;
  }

  .divider {
    height: 1px;
    margin: 4px 0;
    background: var(--divider);
  }
</style>
