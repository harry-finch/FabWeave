<script lang="ts">
  // Éditeur d'un profil imprimante ou filament (fenêtre). Un profil livré
  // n'est jamais modifié : l'enregistrement crée une copie dans les profils
  // de l'utilisateur. Chaque valeur dit son origine : ↺ si elle a changé
  // depuis la création (copie, import), étiquette « importé », « base »,
  // « estimé » ou « vide » pour un profil importé. Le moteur contrôle le
  // profil à chaque modification ; l'enregistrement attend qu'il n'ait plus
  // d'erreur. Les gabarits importés se relisent avant d'imprimer.
  import { onMount, untrack } from 'svelte';
  import Dialog from './Dialog.svelte';
  import Field from './Field.svelte';
  import NumField from './NumField.svelte';
  import Segmented from './Segmented.svelte';
  import Alerts from './Alerts.svelte';
  import Tag from './Tag.svelte';
  import ProfileTable from './ProfileTable.svelte';
  import { store, type ProfileEntry } from '../lib/store.svelte';
  import { i18n, intlLocale, t, type Key } from '../lib/i18n.svelte';
  import { text } from '../lib/text';
  import { toAlerts } from '../lib/issues';
  import { ui } from '../lib/ui.svelte';
  import { GROUPS, getPath, same, setPath, type FieldDef } from '../lib/profileFields';
  import type { AnyProfile, FilamentProfile, Issue, PrinterProfile, ProfileKind } from '../lib/types';

  let {
    kind,
    entry,
    copy = false,
    onclose,
  }: {
    kind: ProfileKind;
    entry: ProfileEntry<AnyProfile>;
    /** Nouveau profil d'après celui-ci (toujours le cas d'un profil livré). */
    copy?: boolean;
    onclose: () => void;
  } = $props();

  const info = (p: AnyProfile) => (kind === 'printer' ? (p as PrinterProfile).printer : (p as FilamentProfile).filament);

  // La fenêtre travaille sur l'état du profil à son ouverture.
  const start = untrack(() => {
    const asCopy = copy || entry.source === 'builtin';
    const original = $state.snapshot(entry.profile!) as AnyProfile;
    return { asCopy, original, user: asCopy ? undefined : entry.user, id: entry.id };
  });
  const builtin = start.asCopy;
  const user = start.user;
  const originalName = text(info(start.original).name);

  /** Brouillon : copie du profil (nom « (copie) » pour un nouveau profil). */
  let draft = $state<AnyProfile>(structuredClone(start.original));
  if (builtin) info(draft).name = t('pe.copyName', { name: originalName });
  /** Valeurs de départ : le profil copié, ou celles de la création (copie, import). */
  const reference = (user?.initial ?? start.original) as AnyProfile;
  const report = user?.origin.type === 'import' ? user.origin.report : undefined;

  let issues = $state<Issue[]>([]);
  let saving = $state(false);
  const errors = $derived(issues.filter((i) => i.severity === 'error'));
  const fieldIssue = (path: string) => issues.find((i) => i.field === path || i.field?.startsWith(`${path}[`) || i.field?.startsWith(`${path}.`));

  // Contrôle par le moteur, 150 ms après la dernière modification.
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const json = JSON.stringify($state.snapshot(draft));
    clearTimeout(timer);
    timer = setTimeout(async () => {
      const r = JSON.parse(await store.engine.checkProfile(kind, json)) as { issues: Issue[] };
      issues = r.issues ?? [];
    }, 150);
    return () => clearTimeout(timer);
  });

  onMount(() => () => clearTimeout(timer));

  const get = (path: string) => getPath(draft, path);
  function set(path: string, v: unknown) {
    setPath(draft, path, v);
  }
  const modified = (path: string) => !same(getPath(draft, path), getPath(reference, path));
  const reset = (path: string) => set(path, structuredClone($state.snapshot(getPath(reference, path) as object)));

  /** Origine d'une valeur importée et non modifiée. */
  function origin(path: string): { label: string; hint: string; tone: 'info' | 'neutral' | 'warning' } | null {
    if (!report || modified(path)) return null;
    const taken = report.taken.find((x) => x.field === path);
    if (taken) return { label: t('pe.origin.taken'), hint: t('pe.origin.takenHint', { key: taken.key, value: taken.value }), tone: 'info' };
    const m = report.missing.find((x) => x.field === path);
    if (!m) return null;
    if (m.fallback === 'base') return { label: t('pe.origin.base'), hint: t('pe.origin.baseHint', { base: baseName }), tone: 'neutral' };
    if (m.fallback === 'estimated') return { label: t('pe.origin.estimated'), hint: t('pe.origin.estimatedHint'), tone: 'warning' };
    return { label: t('pe.origin.empty'), hint: t('pe.origin.emptyHint'), tone: 'neutral' };
  }

  const baseName = $derived.by(() => {
    if (!report) return '';
    const list = kind === 'printer' ? store.printers : store.filaments;
    const b = list.find((e) => e.id === report.base)?.profile;
    return b ? text(info(b).name) : report.base;
  });

  const label = (f: FieldDef) => t(`pf.${f.key}` as Key);
  const hint = (f: FieldDef) => {
    const k = `pfh.${f.key}` as Key;
    const s = t(k);
    return s === k ? undefined : s;
  };

  const fmtNum = (v: number) => new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: 3, useGrouping: false }).format(v);
  function parseList(raw: string): number[] | null {
    const parts = raw.split(/[;·\s]+/).filter(Boolean);
    const nums = parts.map((p) => Number(i18n.locale === 'fr' ? p.replace(',', '.') : p));
    return nums.every(Number.isFinite) ? nums : null;
  }

  const review = $derived((draft as PrinterProfile).printer?.review ?? []);
  function reviewed(path: string) {
    const p = draft as PrinterProfile;
    const left = (p.printer.review ?? []).filter((r) => r !== path);
    if (left.length) p.printer.review = left;
    else delete p.printer.review;
  }

  const title = $derived(builtin ? t('pe.titleCopy', { name: originalName }) : t('pe.title', { name: originalName }));

  async function save() {
    saving = true;
    const r = await store.saveProfile(
      kind,
      draft,
      builtin ? { origin: { type: 'copy', from: start.id, name: originalName }, initial: start.original } : { id: start.id },
    );
    saving = false;
    if (!r.ok) {
      issues = r.issues;
      return;
    }
    ui.toast(t('pe.saved', { name: text(info(draft).name) }));
    onclose();
  }

  const lines = (s: string) => Math.min(18, Math.max(6, s.split('\n').length + 1));
</script>

{#snippet originTag(path: string)}
  {@const o = origin(path)}
  {#if o}<Tag label={o.label} hint={o.hint} tone={o.tone} />{/if}
{/snippet}

<Dialog {title} width={560} {onclose}>
  <div class="fields" class:origins={!!report}>
  {#if builtin}
    <p class="note">{t('pe.copyNote')}</p>
  {:else if report}
    <p class="note">{t('pe.importNote', { file: user?.origin.type === 'import' ? user.origin.file : '', slicer: report.slicer, base: baseName })}</p>
  {:else if user?.origin.type === 'copy'}
    <p class="note">{t('pe.fromNote', { name: user.origin.name })}</p>
  {/if}
  {#if errors.length}<Alerts alerts={toAlerts(errors, null)} />{/if}

  {#each GROUPS[kind] as g (g.id)}
    <h3>{t(`pg.${g.id}` as Key)}</h3>
    {#each g.fields as f (f.key)}
      {@const bad = !!fieldIssue(f.path)}
      {#if f.type === 'num'}
        {@const v = get(f.path)}
        {@const value = (f.index !== undefined ? (v as number[])[f.index] : v) as number}
        <div class="row">
          <NumField
            label={label(f)}
            hint={hint(f)}
            kind="plain"
            unit={f.unit}
            step={f.step}
            decimals={f.decimals}
            min={f.min}
            {value}
            invalid={bad}
            onchange={(n) => {
              if (f.index !== undefined) {
                const arr = [...(get(f.path) as number[])];
                arr[f.index] = n;
                set(f.path, arr);
              } else set(f.path, n);
            }}
            modified={f.index !== undefined ? (get(f.path) as number[])[f.index] !== (getPath(reference, f.path) as number[])[f.index] : modified(f.path)}
            onreset={() => {
              if (f.index !== undefined) {
                const arr = [...(get(f.path) as number[])];
                arr[f.index] = (getPath(reference, f.path) as number[])[f.index];
                set(f.path, arr);
              } else reset(f.path);
            }}
            resetHint={t('pe.reset')}
          />
          {@render originTag(f.path)}
        </div>
      {:else if f.type === 'opt'}
        {@const v = get(f.path) as number | null | undefined}
        <div class="row">
          <NumField
            label={label(f)}
            hint={hint(f)}
            kind="plain"
            unit={f.unit}
            step={f.step}
            decimals={f.decimals}
            min={f.min}
            value={v ?? NaN}
            placeholder={t('pe.none')}
            invalid={bad}
            onchange={(n) => set(f.path, n)}
            onclear={() => set(f.path, undefined)}
            modified={modified(f.path)}
            onreset={() => reset(f.path)}
            resetHint={t('pe.reset')}
          />
          {@render originTag(f.path)}
        </div>
      {:else if f.type === 'name' || f.type === 'text'}
        {@const v = get(f.path)}
        <div class="row">
          <Field label={label(f)} hint={hint(f)}>
            <input
              class="input wide"
              class:bad
              aria-label={label(f)}
              value={f.type === 'name' ? text(v as string) : ((v as string) ?? '')}
              onchange={(e) => set(f.path, e.currentTarget.value.trim())}
            />
          </Field>
          {@render originTag(f.path)}
        </div>
      {:else if f.type === 'bool'}
        <div class="row">
          <Field label={label(f)} hint={hint(f)}>
            <Segmented
              label={label(f)}
              value={get(f.path) ? 'on' : 'off'}
              onchange={(v) => set(f.path, v === 'on')}
              options={[
                { id: 'off', label: t('model.off') },
                { id: 'on', label: t('model.on') },
              ]}
            />
          </Field>
          {@render originTag(f.path)}
        </div>
      {:else if f.type === 'choice'}
        <div class="row">
          <Field label={label(f)} hint={hint(f)}>
            <select class="input wide" aria-label={label(f)} value={(get(f.path) as string) ?? f.options[0]} onchange={(e) => set(f.path, e.currentTarget.value)}>
              {#each f.options as o (o)}<option value={o}>{t(`pe.opt.${o}` as Key)}</option>{/each}
            </select>
          </Field>
          {@render originTag(f.path)}
        </div>
      {:else if f.type === 'numlist'}
        {@const v = (get(f.path) as number[] | undefined) ?? []}
        <div class="row">
          <Field label={label(f)} hint={hint(f)}>
            <input
              class="input wide tabular"
              class:bad
              aria-label={label(f)}
              placeholder={t('pe.none')}
              value={v.map(fmtNum).join(' · ')}
              onchange={(e) => {
                const list = parseList(e.currentTarget.value);
                if (list) set(f.path, list.length ? list : undefined);
                else e.currentTarget.value = v.map(fmtNum).join(' · ');
              }}
            />
          </Field>
          {@render originTag(f.path)}
        </div>
      {:else if f.type === 'strlist'}
        {@const v = (get(f.path) as string[] | undefined) ?? []}
        <div class="row">
          <Field label={label(f)} hint={hint(f)}>
            <input
              class="input wide"
              class:bad
              aria-label={label(f)}
              placeholder={t('pe.none')}
              value={v.join(', ')}
              onchange={(e) => {
                const list = e.currentTarget.value
                  .split(',')
                  .map((x) => x.trim())
                  .filter(Boolean);
                set(f.path, list.length ? list : undefined);
              }}
            />
          </Field>
          {@render originTag(f.path)}
        </div>
      {:else if f.type === 'table'}
        <ProfileTable
          label={label(f)}
          prefix={`pe.cols.${f.key.replace('.', '_')}`}
          columns={f.columns}
          rows={(get(f.path) as Record<string, unknown>[] | undefined) ?? []}
          invalid={bad}
          onchange={(rows) => set(f.path, rows.length ? rows : undefined)}
        />
      {:else if f.type === 'template'}
        {@const issue = fieldIssue(f.path)}
        <div class="tpl">
          <div class="tpl-head">
            <span class="tpl-label">{label(f)}</span>
            {@render originTag(f.path)}
            {#if modified(f.path)}
              <button class="btn quiet small" type="button" onclick={() => reset(f.path)}>{t('pe.reset')}</button>
            {/if}
          </div>
          {#if review.includes(f.path)}
            <div class="review">
              <span>{t('pe.review')}</span>
              <button class="btn small" type="button" onclick={() => reviewed(f.path)}>{t('pe.reviewed')}</button>
            </div>
          {/if}
          <textarea
            class="input code"
            class:bad={!!issue}
            spellcheck="false"
            aria-label={label(f)}
            rows={lines(get(f.path) as string)}
            value={get(f.path) as string}
            onchange={(e) => set(f.path, e.currentTarget.value)}
          ></textarea>
          {#if issue}<p class="err">{issue.detail ?? ''}</p>{/if}
        </div>
      {/if}
    {/each}
  {/each}
  </div>

  {#snippet footer({ close })}
    <button class="btn" type="button" onclick={close}>{t('common.cancel')}</button>
    <button class="btn primary" type="button" disabled={errors.length > 0 || saving} onclick={save}>
      {builtin ? t('pe.saveCopy') : t('pe.save')}
    </button>
  {/snippet}
</Dialog>

<style>
  .fields {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-items: center;
    gap: var(--s2);
  }

  /* Profil importé : une colonne pour l'origine de chaque valeur. */
  .origins .row {
    grid-template-columns: minmax(0, 1fr) 58px;
  }

  .wide {
    width: 100%;
  }

  .bad {
    border-color: var(--danger);
  }

  .tpl {
    display: flex;
    flex-direction: column;
    gap: var(--s1);
  }

  .tpl-head {
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-height: var(--control-h);
  }

  .tpl-label {
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    margin-right: auto;
  }

  .code {
    width: 100%;
    height: auto;
    padding: 6px 8px;
    font: var(--fs-sm) / 1.45 var(--mono);
    resize: vertical;
    white-space: pre;
    overflow: auto;
  }

  .review {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2);
    padding: 6px var(--s2);
    border-radius: var(--radius);
    background: var(--warning-tint);
    color: var(--text-primary);
    font-size: var(--fs-sm);
  }

  .err {
    margin: 0;
    color: var(--danger-text);
    font: var(--fs-sm) / 1.4 var(--mono);
  }

  .small {
    height: 22px;
    padding: 0 8px;
    font-size: var(--fs-sm);
  }
</style>
