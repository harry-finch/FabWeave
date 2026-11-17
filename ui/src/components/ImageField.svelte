<script lang="ts">
  // Paramètre `image` : un dessin SVG ou PNG que le modèle remplit, ajoure
  // ou met en relief. Aperçu de la silhouette (masque teinté par le thème :
  // la forme compte, pas ses couleurs), choix d'un fichier, ↺ pour revenir
  // au dessin du modèle. Le moteur lit le fichier et dit ce qui ne va pas.
  import { ImageUp, Info, RotateCcw } from '@lucide/svelte';
  import { tip } from '../lib/tip';
  import { t } from '../lib/i18n.svelte';
  import { text } from '../lib/text';
  import type { ParamDef } from '../lib/types';

  type ImageDef = Extract<ParamDef, { type: 'image' }>;

  let {
    def,
    value,
    invalid = false,
    modified = false,
    onchange,
    onreset,
  }: {
    def: ImageDef;
    value: string;
    invalid?: boolean;
    modified?: boolean;
    onchange: (v: string) => void;
    onreset: () => void;
  } = $props();

  let input = $state<HTMLInputElement>();

  /** URL de l'aperçu : le PNG tel quel, le SVG encodé. */
  const url = $derived(value.startsWith('data:') ? value : value.trim() ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(value)}` : '');

  async function pick(f: File | undefined) {
    if (!f) return;
    if (/\.svg$/i.test(f.name) || f.type === 'image/svg+xml') {
      onchange(await f.text());
      return;
    }
    const reader = new FileReader();
    reader.onload = () => onchange(String(reader.result));
    reader.readAsDataURL(f);
  }
</script>

<div class="image" class:invalid>
  <div class="head">
    <span class="label">
      {text(def.label)}
      {#if def.help}<span class="hint" use:tip={text(def.help)} tabindex="-1"><Info size={11} /></span>{/if}
    </span>
    {#if modified}
      <button class="btn quiet icon" type="button" aria-label={t('image.reset')} use:tip={t('image.reset')} onclick={onreset}><RotateCcw size={12} /></button>
    {/if}
    <button class="btn" type="button" onclick={() => input?.click()}><ImageUp size={13} />{t('image.choose')}</button>
  </div>
  <div class="preview">
    {#if url}
      <div class="shape" style:mask-image="url(&quot;{url}&quot;)" style:-webkit-mask-image="url(&quot;{url}&quot;)" role="img" aria-label={text(def.label)}></div>
    {:else}
      <p class="none">{t('image.none')}</p>
    {/if}
  </div>
  <p class="note">{t('image.size', { size: def.size })}</p>
  <input
    bind:this={input}
    class="sr-only"
    type="file"
    accept=".svg,.png,image/svg+xml,image/png"
    tabindex="-1"
    aria-hidden="true"
    onchange={(e) => {
      const el = e.currentTarget as HTMLInputElement;
      void pick(el.files?.[0]);
      el.value = '';
    }}
  />
</div>

<style>
  .image {
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

  .preview {
    display: grid;
    place-items: center;
    height: 120px;
    padding: var(--s2);
    background: var(--bg-workspace);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .invalid .preview {
    border-color: var(--danger);
  }

  /* Silhouette du dessin : sa transparence découpe un aplat du thème. */
  .shape {
    width: 100%;
    height: 100%;
    background: var(--text-primary);
    mask-size: contain;
    mask-repeat: no-repeat;
    mask-position: center;
    -webkit-mask-size: contain;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-position: center;
  }

  .none,
  .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }
</style>
