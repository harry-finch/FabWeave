<script lang="ts">
  // Buse montée, à côté de l'imprimante (PRF-11) : diamètres proposés par le
  // profil ou valeur libre (« Autre… »), buse haut débit et buse abrasive
  // quand l'imprimante les propose. Le choix est mémorisé par imprimante et
  // ne duplique pas le profil ; le parcours se recalcule aussitôt. Ce que la
  // buse change (hauteurs de couche, débit maximal) est montré sous le
  // filament, qui en dépend aussi.
  import Field from './Field.svelte';
  import NumField from './NumField.svelte';
  import Segmented from './Segmented.svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { numTrim } from '../lib/format';
  import type { PrinterProfile } from '../lib/types';

  let { printer }: { printer: PrinterProfile } = $props();

  const n = $derived(printer.nozzle);
  const choices = $derived(n.choices?.length ? n.choices : [n.diameter]);
  const diameter = $derived(store.nozzle.diameter ?? n.diameter);
  const listed = $derived(choices.some((d) => Math.abs(d - diameter) < 1e-6));
  /** « Autre… » choisi alors que le diamètre est encore une buse de la liste. */
  let other = $state(false);
  const selected = $derived(other || !listed ? 'other' : String(diameter));
  const highFlow = $derived(store.nozzle.high_flow ?? n.high_flow ?? false);
  const abrasive = $derived(store.nozzle.abrasive ?? n.abrasive ?? false);

  function pick(v: string) {
    if (v === 'other') {
      other = true;
      return;
    }
    other = false;
    const d = Number(v);
    store.setNozzle({ diameter: Math.abs(d - n.diameter) < 1e-6 ? undefined : d });
  }

  $effect(() => {
    // Autre imprimante : la liste repart de son choix mémorisé.
    void printer.printer.id;
    other = false;
  });
</script>

<Field label={t('nozzle.diameter')} hint={t('nozzle.diameterHint')}>
  <select class="input select" value={selected} onchange={(e) => pick((e.currentTarget as HTMLSelectElement).value)}>
    {#each choices as d (d)}<option value={String(d)}>{numTrim(d, 2)} mm</option>{/each}
    <option value="other">{t('nozzle.other')}</option>
  </select>
</Field>
{#if selected === 'other'}
  <NumField
    label={t('nozzle.custom')}
    kind="plain"
    unit="mm"
    value={diameter}
    step={0.05}
    decimals={2}
    min={0.1}
    max={2}
    onchange={(v) => store.setNozzle({ diameter: Math.abs(v - n.diameter) < 1e-6 ? undefined : v })}
  />
{/if}
{#if n.high_flow_available}
  <Field label={t('nozzle.flow')} hint={t('nozzle.flowHint')}>
    <Segmented
      label={t('nozzle.flow')}
      value={highFlow ? 'hf' : 'std'}
      onchange={(v) => store.setNozzle({ high_flow: (v === 'hf') === !!n.high_flow ? undefined : v === 'hf' })}
      options={[
        { id: 'std', label: t('nozzle.standard') },
        { id: 'hf', label: t('nozzle.highFlow') },
      ]}
    />
  </Field>
{/if}
{#if n.abrasive_available}
  <Field label={t('nozzle.abrasive')} hint={t('nozzle.abrasiveHint')}>
    <Segmented
      label={t('nozzle.abrasive')}
      value={abrasive ? 'on' : 'off'}
      onchange={(v) => store.setNozzle({ abrasive: (v === 'on') === !!n.abrasive ? undefined : v === 'on' })}
      options={[
        { id: 'off', label: t('model.off') },
        { id: 'on', label: t('model.on') },
      ]}
    />
  </Field>
{/if}
