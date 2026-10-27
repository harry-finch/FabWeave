// Messages du moteur (codes stables) rendus dans la langue de l'interface :
// texte traduit, ligne du fichier `.weave` le cas échéant, détail technique
// (message de TOML ou de Rhai) en second.

import { i18n, labelled, t, type Key } from './i18n.svelte';
import { numTrim } from './format';
import { text } from './text';
import type { Issue, ModelInfo } from './types';
import type { Alert } from '../components/Alerts.svelte';
import { FILAMENT_GROUPS, PRINTER_GROUPS } from './profileFields';

const quote = (s: string) => (i18n.locale === 'fr' ? `« ${s} »` : `“${s}”`);

/** Libellé d'un champ de profil (`temperature.nozzle` → « Températures › Buse »), sinon le chemin. */
export function profileFieldLabel(path: string): string {
  const clean = path.replace(/\[\d+\]$/, '');
  const own = `pf.${clean}` as Key;
  let label = t(own);
  if (label === own) {
    const def = [...PRINTER_GROUPS, ...FILAMENT_GROUPS].flatMap((g) => g.fields).find((f) => f.path === clean);
    if (!def) return path;
    label = t(`pf.${def.key}` as Key);
  }
  const group = `pg.${clean.split('.')[0]}` as Key;
  const g = t(group);
  return g === group ? label : `${g} › ${label}`;
}

/** Libellé d'un champ : nom du paramètre pour `params.<id>`, champ d'un profil, sinon le chemin. */
function fieldLabel(field: string | undefined, model: ModelInfo | null): string {
  if (!field) return '';
  const id = field.startsWith('params.') ? field.slice(7) : null;
  const def = id ? model?.params[id] : undefined;
  if (def) return quote(text(def.label));
  const p = profileFieldLabel(field);
  return p === field ? field : quote(p);
}

export function issueText(i: Issue, model: ModelInfo | null): string {
  const key = `issues.${i.code}` as Key;
  const params = {
    field: fieldLabel(i.field, model),
    value: i.value == null ? '' : numTrim(i.value, 2),
    count: i.count ?? '',
    detail: i.detail ?? '',
    name: i.name ? text(i.name) : '',
  };
  let s = t(key, params);
  if (s === key) s = i.code;
  // Message écrit par l'auteur du modèle (`warn`, `fail`) : son texte, dans la langue.
  if (i.text) s = text(i.text);
  if (i.line != null) s = labelled(t('issues.line', { line: i.line }), s);
  // Détail technique, sauf s'il est déjà repris dans le texte.
  if (i.detail && !s.includes(i.detail)) s = `${s} — ${i.detail}`;
  return s;
}

export function toAlerts(issues: Issue[], model: ModelInfo | null): Alert[] {
  return issues.map((i, n) => ({ code: `${i.code}#${n}`, severity: i.severity, message: issueText(i, model) }));
}
