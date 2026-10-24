// Messages du moteur (codes stables) rendus dans la langue de l'interface :
// texte traduit, ligne du fichier `.weave` le cas échéant, détail technique
// (message de TOML ou de Rhai) en second.

import { i18n, labelled, t, type Key } from './i18n.svelte';
import { numTrim } from './format';
import { text } from './text';
import type { Issue, ModelInfo } from './types';
import type { Alert } from '../components/Alerts.svelte';

/** Libellé d'un champ : nom du paramètre pour `params.<id>`, sinon le chemin. */
function fieldLabel(field: string | undefined, model: ModelInfo | null): string {
  if (!field) return '';
  const id = field.startsWith('params.') ? field.slice(7) : null;
  const def = id ? model?.params[id] : undefined;
  if (!def) return field;
  return i18n.locale === 'fr' ? `« ${text(def.label)} »` : `“${text(def.label)}”`;
}

export function issueText(i: Issue, model: ModelInfo | null): string {
  const key = `issues.${i.code}` as Key;
  const params = {
    field: fieldLabel(i.field, model),
    value: i.value == null ? '' : numTrim(i.value, 2),
    count: i.count ?? '',
    detail: i.detail ?? '',
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
