<script lang="ts">
  // Écran principal : à gauche le modèle (catalogue) et ses paramètres, au
  // centre la vue 3D du parcours, à droite la machine (imprimante, filament),
  // les mesures et les messages du moteur. Tout se recalcule à chaque
  // modification ; l'action principale exporte le G-code.
  import { RotateCcw } from '@lucide/svelte';
  import AppBar from './components/AppBar.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import Section from './components/Section.svelte';
  import Field from './components/Field.svelte';
  import Dialog from './components/Dialog.svelte';
  import Alerts from './components/Alerts.svelte';
  import ParamsPanel from './components/ParamsPanel.svelte';
  import Viewport from './components/Viewport.svelte';
  import { store } from './lib/store.svelte';
  import { ui } from './lib/ui.svelte';
  import { labelled, t } from './lib/i18n.svelte';
  import { text } from './lib/text';
  import { toAlerts } from './lib/issues';
  import { fmtDuration, fmtLength, lengthUnit, num, numTrim } from './lib/format';
  import { tip } from './lib/tip';

  let viewport = $state<ReturnType<typeof Viewport>>();

  const report = $derived(store.frame?.report ?? null);
  const stats = $derived(report?.stats ?? null);
  const issues = $derived([...store.loadIssues, ...(report?.issues ?? [])]);
  const alerts = $derived(toAlerts(issues, store.model));
  const blocked = $derived(issues.some((i) => i.severity === 'error'));
  const canExport = $derived(!!stats && !blocked && store.status === 'ready');
  const modified = $derived(Object.keys(store.current).length > 0);

  /** Modèles du catalogue groupés par catégorie, dans l'ordre des fichiers. */
  const groups = $derived.by(() => {
    const out = new Map<string, { id: string; name: string }[]>();
    for (const e of store.catalog) {
      const cat = e.info ? text(e.info.meta.category) : t('model.unreadable');
      const list = out.get(cat) ?? [];
      list.push({ id: e.id, name: e.info ? text(e.info.meta.name) : e.id });
      out.set(cat, list);
    }
    return [...out.entries()];
  });

  async function exportGcode() {
    const r = await store.gcode();
    if (!r.ok) {
      ui.toast(t('export.blocked'));
      return;
    }
    const base = store.modelId.split('/').pop() ?? 'fabweave';
    const name = `${base}_${store.printer?.printer.model ?? 'printer'}_${store.filament?.filament.type ?? 'filament'}.gcode`;
    const url = URL.createObjectURL(new Blob([r.text], { type: 'text/plain' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    ui.toast(t('export.done', { name }));
  }

  function keydown(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable)) return;
    if (ui.help) return;
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'z') {
      e.preventDefault();
      if (e.shiftKey) store.redo();
      else store.undo();
      return;
    }
    if (mod && e.key.toLowerCase() === 'y') {
      e.preventDefault();
      store.redo();
      return;
    }
    if (mod || e.altKey) return;
    if (e.key === '?') ui.help = true;
    if (e.key === 'f' || e.key === 'F') viewport?.refit();
  }

  /** Dimensions « L × l × h » dans l'unité d'affichage. */
  const dims = (v: number[], mmDecimals: number) => `${v.map((x) => fmtLength(x, ui.units, mmDecimals)).join(' × ')} ${lengthUnit(ui.units)}`;
</script>

<svelte:window onkeydown={keydown} />

<div class="app">
  <AppBar onexport={exportGcode} {canExport} />
  <div class="main">
    <aside class="panel left">
      <Section id="model" title={t('model.section')} step={1}>
        {#if store.catalog.length === 0 && store.status === 'ready'}
          <p class="note">{t('model.none')}</p>
        {:else}
          <!-- Liste du catalogue : sous-libellé puis toute la largeur (noms longs). -->
          <p class="sub">{t('model.catalog')}</p>
          <select class="input select" aria-label={t('model.catalog')} value={store.modelId} onchange={(e) => store.select('model', (e.currentTarget as HTMLSelectElement).value)}>
            {#each groups as [cat, list] (cat)}
              <optgroup label={cat}>
                {#each list as m (m.id)}
                  <option value={m.id}>{m.name}</option>
                {/each}
              </optgroup>
            {/each}
          </select>
        {/if}
        {#if store.model}
          {#if store.model.meta.description}<p class="note">{text(store.model.meta.description)}</p>{/if}
          {#if store.model.meta.author}<p class="credit">{labelled(t('model.author'), store.model.meta.author)}</p>{/if}
          {#if store.model.meta.source}
            <p class="credit"><a href={store.model.meta.source} target="_blank" rel="noreferrer">{t('model.source')}</a>{store.model.meta.license ? ` · ${store.model.meta.license}` : ''}</p>
          {/if}
        {/if}
      </Section>
      {#if store.model}
        <Section id="params" title={t('model.params')} step={2}>
          {#snippet actions()}
            <button class="btn quiet icon" type="button" disabled={!modified} aria-label={t('model.reset')} use:tip={t('model.reset')} onclick={() => store.reset()}>
              <RotateCcw size={14} />
            </button>
          {/snippet}
          <ParamsPanel
            params={store.model.params}
            values={store.current}
            {issues}
            onchange={(id, v) => store.setValue(id, v)}
            onreset={(id) => store.reset(id)}
            onbegin={() => store.begin()}
            onend={() => store.end()}
          />
        </Section>
      {/if}
    </aside>

    <main class="workspace">
      <Viewport bind:this={viewport} frame={store.frame} printer={store.printer} fitKey={store.modelId} />
    </main>

    <aside class="panel right">
      <Section id="machine" title={t('machine.section')} step={3}>
        <Field label={t('machine.printer')}>
          <select class="input select" value={store.printerId} onchange={(e) => store.select('printer', (e.currentTarget as HTMLSelectElement).value)}>
            {#each store.printers as p (p.id)}<option value={p.id}>{p.profile?.printer.name ?? p.id}</option>{/each}
          </select>
        </Field>
        <Field label={t('machine.filament')}>
          <select class="input select" value={store.filamentId} onchange={(e) => store.select('filament', (e.currentTarget as HTMLSelectElement).value)}>
            {#each store.filaments as f (f.id)}<option value={f.id}>{f.profile ? text(f.profile.filament.name) : f.id}</option>{/each}
          </select>
        </Field>
        {#if store.printer}
          <dl class="metrics">
            <dt>{t('machine.nozzle')}</dt>
            <dd class="tabular">{numTrim(store.printer.nozzle.diameter, 2)} mm</dd>
            <dt>{t('machine.bed')}</dt>
            <dd class="tabular">{dims([store.printer.bed.size[0], store.printer.bed.size[1], store.printer.bed.max_height], 0)}</dd>
          </dl>
        {/if}
      </Section>
      <Section id="metrics" title={t('metrics.section')}>
        <dl class="metrics">
          <dt><span use:tip={t('metrics.timeHint')}>{t('metrics.time')}</span></dt>
          <dd class="strong tabular">{fmtDuration(stats?.time)}</dd>
          <dt>{t('metrics.filament')}</dt>
          <dd class="tabular">{stats ? `${num(stats.filament_length / 1000, 2)} m · ${num(stats.filament_mass, 1)} g` : '—'}</dd>
          <dt>{t('metrics.size')}</dt>
          <dd class="tabular">{stats ? dims([stats.bounds.max[0] - stats.bounds.min[0], stats.bounds.max[1] - stats.bounds.min[1], stats.bounds.max[2]], 1) : '—'}</dd>
          <dt><span use:tip={t('metrics.printLengthHint')}>{t('metrics.printLength')}</span></dt>
          <dd class="tabular">{stats ? `${num(stats.print_length / 1000, 1)} m` : '—'}</dd>
          <dt><span use:tip={t('metrics.maxFlowHint')}>{t('metrics.maxFlow')}</span></dt>
          <dd class="tabular">{stats ? `${num(stats.max_flow, 1)} mm³/s` : '—'}</dd>
          <dt>{t('metrics.points')}</dt>
          <dd class="tabular">{report ? num(report.points) : '—'}</dd>
          <dt>{t('metrics.retractions')}</dt>
          <dd class="tabular">{stats ? num(stats.retractions) : '—'}</dd>
        </dl>
      </Section>
      <Section id="issues" title={t('issues.section')}>
        {#if alerts.length}
          <Alerts {alerts} />
        {:else}
          <p class="note">{t('issues.none')}</p>
        {/if}
      </Section>
    </aside>
  </div>
  <StatusBar state={store.status} version={store.version}>
    {#if store.status === 'ready'}
      <span class="sep"></span>
      {#if store.deferred}
        <span class="deferred">{t('status.deferred')}</span>
      {:else}
        <span class="tabular">{store.computing ? t('status.computing') : store.frame ? t('status.computed', { ms: num(store.frame.ms) }) : ''}</span>
      {/if}
    {/if}
    {#snippet right()}
      {#if store.printer && store.filament}
        <span>{store.printer.printer.name} · {text(store.filament.filament.name)}</span>
      {/if}
    {/snippet}
  </StatusBar>
</div>

{#if ui.help}
  <Dialog title={t('help.title')} onclose={() => (ui.help = false)}>
    <ol class="steps">
      <li>{t('help.steps.one')}</li>
      <li>{t('help.steps.two')}</li>
      <li>{t('help.steps.three')}</li>
      <li>{t('help.steps.four')}</li>
    </ol>
    <h3>{t('help.shortcuts')}</h3>
    <dl class="keys">
      <dt><kbd>?</kbd></dt>
      <dd>{t('help.keys.help')}</dd>
      <dt><kbd>F</kbd></dt>
      <dd>{t('help.keys.fit')}</dd>
      <dt><kbd>⌘Z / ⇧⌘Z</kbd></dt>
      <dd>{t('help.keys.undo')}</dd>
      <dt><kbd>🖱</kbd></dt>
      <dd>{t('help.keys.orbit')}</dd>
    </dl>
  </Dialog>
{/if}

<div class="toasts" aria-live="polite">
  {#each ui.toasts as toast (toast.id)}
    <div class="toast">{toast.text}</div>
  {/each}
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    min-width: 0;
  }

  .main {
    display: grid;
    grid-template-columns: 296px minmax(0, 1fr) 296px;
    min-height: 0;
  }

  @media (max-width: 1100px) {
    .main {
      grid-template-columns: 264px minmax(0, 1fr) 264px;
    }
  }

  .panel {
    background: var(--bg-panel);
    overflow-y: auto;
    min-height: 0;
  }

  .left {
    border-right: 1px solid var(--border);
  }

  .right {
    border-left: 1px solid var(--border);
  }

  .workspace {
    min-width: 0;
    min-height: 0;
    background: var(--bg-workspace);
  }

  .select {
    width: 100%;
  }

  .deferred {
    color: var(--warning);
  }

  .sub {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .note {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .credit {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .metrics {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px var(--s2);
    margin: 0;
  }

  .metrics dt {
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .metrics dd {
    margin: 0;
    text-align: right;
  }

  .metrics dd.strong {
    font-weight: 600;
    font-size: var(--fs-lg);
  }

  .steps {
    margin: 0 0 var(--s3);
    padding-left: var(--s4);
    color: var(--text-secondary);
  }

  .keys {
    display: grid;
    grid-template-columns: 60px 1fr;
    gap: 6px var(--s3);
    margin: 0;
  }

  .keys dd {
    margin: 0;
  }

  kbd {
    display: inline-block;
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--bg-control);
    font: 500 var(--fs-sm) var(--font);
    color: var(--text-primary);
  }

  .toasts {
    position: fixed;
    bottom: 36px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 900;
    pointer-events: none;
  }

  .toast {
    padding: 8px 14px;
    border-radius: var(--radius-lg);
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    font-size: var(--fs-sm);
    box-shadow: var(--shadow-pop);
    animation: rise 160ms ease-out;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
