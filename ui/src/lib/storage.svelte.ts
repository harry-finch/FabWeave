// Stockage du poste (localStorage) sans perte silencieuse (R-009, AUD-006).
// - Lecture contrôlée : une valeur illisible ou mal formée n'est jamais
//   effacée ; elle est recopiée telle quelle sous une clé « .mise-de-cote »
//   datée, et signalée (`persist.setAside`). Les éléments valides d'une liste
//   sont gardés.
// - Écriture : un échec (stockage plein ou interdit) est retenu par clé ;
//   tant qu'une clé n'a pas pu s'écrire, `persist.failed` est vrai et
//   l'interface affiche « Non enregistré sur ce poste ».

export const persist = $state({
  /** Au moins une écriture a échoué et n'a pas réussi depuis. */
  failed: false,
  /** Clés dont la valeur, mal formée, a été mise de côté. */
  setAside: [] as string[],
});

const failedKeys = new Set<string>();

function setAside(key: string, raw: string) {
  const aside = `${key}.mise-de-cote-${new Date().toISOString()}`;
  try {
    localStorage.setItem(aside, raw);
  } catch {
    /* plein : la valeur reste sous sa clé d'origine, qu'on n'écrase pas */
    failedKeys.add(key);
    persist.failed = true;
  }
  persist.setAside = [...persist.setAside, key];
}

function raw(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/** Liste stockée : les éléments valides ; le reste mis de côté. */
export function readList<T>(key: string, valid: (x: unknown) => x is T): T[] {
  const text = raw(key);
  if (text === null) return [];
  try {
    const v: unknown = JSON.parse(text);
    if (Array.isArray(v)) {
      const ok = v.filter(valid);
      if (ok.length !== v.length) setAside(key, text);
      return ok;
    }
  } catch {
    /* JSON tronqué : mis de côté ci-dessous */
  }
  setAside(key, text);
  return [];
}

/** Objet stocké (document) ; autre chose est mis de côté. */
export function readObject(key: string): Record<string, unknown> {
  const text = raw(key);
  if (text === null) return {};
  try {
    const v: unknown = JSON.parse(text);
    if (v && typeof v === 'object' && !Array.isArray(v)) return v as Record<string, unknown>;
  } catch {
    /* mis de côté ci-dessous */
  }
  setAside(key, text);
  return {};
}

/** Écrit une valeur (JSON) ; rend faux et le signale si le poste refuse. */
export function write(key: string, value: unknown): boolean {
  try {
    localStorage.setItem(key, typeof value === 'string' ? value : JSON.stringify(value));
    failedKeys.delete(key);
    persist.failed = failedKeys.size > 0;
    return true;
  } catch {
    failedKeys.add(key);
    persist.failed = true;
    return false;
  }
}

export function remove(key: string) {
  try {
    localStorage.removeItem(key);
  } catch {
    /* rien à perdre */
  }
}

export const isString = (x: unknown): x is string => typeof x === 'string';
export const hasStrings =
  <K extends string>(...keys: K[]) =>
  (x: unknown): x is Record<K, string> =>
    !!x && typeof x === 'object' && keys.every((k) => typeof (x as Record<string, unknown>)[k] === 'string');
