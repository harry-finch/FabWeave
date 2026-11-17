// Ce que l'application de bureau ajoute à la version web (commandes de
// `crates/fabweave-desktop/src/main.rs`) : dialogues natifs, lecture et
// écriture de fichiers, dossier de modèles surveillé. Dans un navigateur,
// `desktop` vaut `null` et l'interface garde ses téléchargements.
//
// Appel direct de l'IPC de Tauri, sans dépendance : les commandes sont
// celles de l'application, aucune permission de plugin n'est demandée.

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

const internals = (globalThis as unknown as { __TAURI_INTERNALS__?: { invoke: Invoke } }).__TAURI_INTERNALS__;

/** Modèle du dossier surveillé : chemin relatif (`/`) et texte. */
export interface FolderFile {
  path: string;
  text: string;
}

/** Contenu du dossier envoyé à chaque modification, ou l'erreur qui l'empêche. */
export type FolderUpdate = { files: FolderFile[] } | { error: string };

function bridge(invoke: Invoke) {
  return {
    /** Dialogue Ouvrir ; chemin choisi ou `null`. */
    pickOpen: () => invoke<string | null>('pick_open'),
    /** Dialogue Enregistrer sous, avec un nom proposé. */
    pickSave: (name: string) => invoke<string | null>('pick_save', { name }),
    pickFolder: () => invoke<string | null>('pick_folder'),
    /** Fichier, ou ses `tail` derniers octets. */
    async readFile(path: string, tail?: number): Promise<Uint8Array> {
      return new Uint8Array(await invoke<ArrayBuffer>('read_file', { path, tail: tail ?? null }));
    },
    /** Écriture atomique (fichier temporaire puis renommage). */
    writeFile: (path: string, text: string) => invoke<void>('write_file', { path, text }),
    /** Surveille un dossier (ou plus aucun) ; rend ses modèles du moment. */
    watchFolder: (path: string | null) => invoke<FolderFile[]>('watch_folder', { path }),
    /** Modifications du dossier surveillé ; rend de quoi arrêter l'écoute. */
    onFolder(fn: (u: FolderUpdate) => void): () => void {
      const h = (e: Event) => fn((e as CustomEvent<FolderUpdate>).detail);
      window.addEventListener('fabweave:folder', h);
      return () => window.removeEventListener('fabweave:folder', h);
    },
  };
}

export const desktop = internals ? bridge(internals.invoke) : null;

/** Nom de fichier d'un chemin (séparateurs macOS, Linux et Windows). */
export const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;
