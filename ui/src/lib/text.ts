// Textes des modèles : chaîne simple ou table par langue (`{ fr, en }`).

import { i18n } from './i18n.svelte';
import type { Text } from './types';

/** Texte dans la langue de l'interface, sinon le français, sinon le premier. */
export function text(t: Text | undefined): string {
  if (t == null) return '';
  if (typeof t === 'string') return t;
  return t[i18n.locale] ?? t.fr ?? Object.values(t)[0] ?? '';
}
