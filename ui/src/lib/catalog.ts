// Catalogue livré : modèles `.weave` du dossier `models/` et profils TOML de
// `profiles/`, importés en texte au moment de la construction. En
// développement, un fichier ajouté ou modifié recharge l'application.

const raw = (files: Record<string, unknown>) =>
  Object.entries(files)
    .map(([path, text]) => ({ path, text: text as string }))
    .sort((a, b) => a.path.localeCompare(b.path));

const id = (path: string, ext: string) => path.replace(/^.*\/(models|profiles)\//, '').replace(ext, '');

export interface SourceFile {
  /** Identifiant stable : chemin relatif sans extension (`vases/ripple_vase`). */
  id: string;
  text: string;
}

/** Nom affiché d'un profil : premier `name = "…"` du fichier (le moteur fait foi). */
export const profileName = (f: SourceFile) => /^name\s*=\s*"([^"]*)"/m.exec(f.text)?.[1] ?? f.id;

export const MODEL_FILES: SourceFile[] = raw(
  import.meta.glob('../../../models/**/*.weave', { query: '?raw', import: 'default', eager: true }),
).map((f) => ({ id: id(f.path, '.weave'), text: f.text }));

export const PRINTER_FILES: SourceFile[] = raw(
  import.meta.glob('../../../profiles/printers/*.toml', { query: '?raw', import: 'default', eager: true }),
).map((f) => ({ id: id(f.path, '.toml').replace('printers/', ''), text: f.text }));

export const FILAMENT_FILES: SourceFile[] = raw(
  import.meta.glob('../../../profiles/filaments/*.toml', { query: '?raw', import: 'default', eager: true }),
).map((f) => ({ id: id(f.path, '.toml').replace('filaments/', ''), text: f.text }));
