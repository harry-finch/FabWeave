<script lang="ts">
  // Paramètres d'un modèle, générés depuis son en-tête `.weave` : une ligne
  // par paramètre dans l'ordre du fichier, les paramètres `advanced` derrière
  // « ▸ Avancé ». Unité « mm » : longueur (affichée en pouces si demandé),
  // « ° » : angle, entier : compteur, booléen : Oui / Non. Un paramètre visé
  // par un message bloquant du moteur est souligné en rouge.
  // Les bornes du modèle ne bornent pas la saisie : une valeur hors bornes est
  // signalée par le moteur (`param.out_of_range`), jamais ramenée en silence.
  // Une valeur modifiée montre un ↺ qui revient à la valeur du modèle ; un
  // défilement du libellé est un geste (une étape d'annulation).
  import NumField from './NumField.svelte';
  import Field from './Field.svelte';
  import Segmented from './Segmented.svelte';
  import Disclosure from './Disclosure.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';
  import { text } from '../lib/text';
  import { fmtLen, numTrim } from '../lib/format';
  import type { Issue, ParamDef, Values } from '../lib/types';

  let {
    params,
    values,
    issues,
    onchange,
    onreset,
    onbegin,
    onend,
  }: {
    params: Record<string, ParamDef>;
    values: Values;
    issues: Issue[];
    onchange: (id: string, v: number | boolean | string) => void;
    onreset: (id: string) => void;
    onbegin?: () => void;
    onend?: () => void;
  } = $props();

  const entries = $derived(Object.entries(params));
  const basic = $derived(entries.filter(([, d]) => !d.advanced));
  const advanced = $derived(entries.filter(([, d]) => d.advanced));
  const invalid = $derived(new Set(issues.filter((i) => i.severity === 'error' && i.field?.startsWith('params.')).map((i) => i.field!.slice(7))));

  /** Décimales d'un pas (`0.05` → 2). */
  const decimalsOf = (step: number | undefined) => (step == null ? 2 : Math.max(0, (String(step).split('.')[1] ?? '').length));

  /** Valeur du modèle, telle qu'affichée (« 15 mm », « 0,6 in », « 10 % »). */
  function shownDefault(def: ParamDef): string {
    if (def.type === 'bool') return def.default ? t('model.on') : t('model.off');
    if (def.type === 'choice') return text(def.options.find((o) => o.value === def.default)?.label) || def.default;
    if (def.unit === 'mm' && def.type === 'float') return fmtLen(def.default, ui.units, decimalsOf(def.step));
    return `${numTrim(def.default, decimalsOf(def.step))}${def.unit ? ` ${def.unit}` : ''}`;
  }
</script>

{#snippet row(id: string, def: ParamDef)}
  {#if def.type === 'bool'}
    <Field label={text(def.label)} hint={text(def.help) || undefined}>
      <Segmented
        label={text(def.label)}
        value={(values[id] ?? def.default) ? 'on' : 'off'}
        onchange={(v) => onchange(id, v === 'on')}
        options={[
          { id: 'off', label: t('model.off') },
          { id: 'on', label: t('model.on') },
        ]}
      />
    </Field>
  {:else if def.type === 'choice'}
    {@const current = (values[id] as string | undefined) ?? def.default}
    {#if def.options.length <= 2}
      <Field label={text(def.label)} hint={text(def.help) || undefined}>
        <Segmented label={text(def.label)} value={current} onchange={(v) => onchange(id, v)} options={def.options.map((o) => ({ id: o.value, label: text(o.label) || o.value }))} />
      </Field>
    {:else}
      <!-- Trois options ou plus : sous-libellé puis pleine largeur. -->
      <p class="sub">{text(def.label)}</p>
      {#if def.options.length === 3}
        <Segmented label={text(def.label)} value={current} onchange={(v) => onchange(id, v)} options={def.options.map((o) => ({ id: o.value, label: text(o.label) || o.value }))} />
      {:else}
        <select class="input select" aria-label={text(def.label)} value={current} onchange={(e) => onchange(id, (e.currentTarget as HTMLSelectElement).value)}>
          {#each def.options as o (o.value)}<option value={o.value}>{text(o.label) || o.value}</option>{/each}
        </select>
      {/if}
    {/if}
  {:else}
    {@const isLength = def.unit === 'mm' && def.type === 'float'}
    {@const kind = def.type === 'int' ? 'count' : isLength ? 'length' : def.unit === '°' ? 'angle' : 'plain'}
    <NumField
      label={text(def.label)}
      hint={text(def.help) || undefined}
      value={(values[id] as number | undefined) ?? def.default}
      {kind}
      step={isLength && ui.units === 'in' ? undefined : def.step}
      decimals={def.type === 'int' ? 0 : isLength && ui.units === 'in' ? 3 : decimalsOf(def.step)}
      unit={isLength || kind === 'angle' ? undefined : (def.unit ?? '')}
      invalid={invalid.has(id)}
      onchange={(v) => onchange(id, v)}
      modified={values[id] !== undefined && values[id] !== def.default}
      onreset={() => onreset(id)}
      resetHint={t('model.resetOne', { v: shownDefault(def) })}
      {onbegin}
      {onend}
    />
  {/if}
{/snippet}

{#each basic as [id, def] (id)}
  {@render row(id, def)}
{/each}
{#if advanced.length}
  <Disclosure id="params.advanced" label={t('model.advanced')}>
    {#each advanced as [id, def] (id)}
      {@render row(id, def)}
    {/each}
  </Disclosure>
{/if}

<style>
  .sub {
    margin: var(--s1) 0 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .select {
    width: 100%;
  }
</style>
