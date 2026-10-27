// État de l'application : moteur, catalogues (modèles, imprimantes,
// filaments), document (modèle et profils choisis, valeurs des paramètres par
// modèle), dernier calcul.
//
// - Toute modification du document passe par `edit()` qui empile un
//   instantané pour annuler / rétablir ; les modifications successives d'un
//   même champ fusionnent, un geste (`begin()` / `end()`) fait une étape.
// - Calcul en direct ; si le premier calcul d'un geste dépasse 60 ms, les
//   suivants attendent le relâchement (`deferred`, signalé dans la barre d'état).
// - Le document est mémorisé sur le poste 400 ms après chaque modification.

import { EngineClient } from './engine';
import { FILAMENT_FILES, MODEL_FILES, PRINTER_FILES } from './catalog';
import { storageKey } from './app';
import { renderThumb } from './thumbs';
import { theme } from './theme.svelte';
import type { FilamentProfile, FrameData, Issue, LoadResult, ModelInfo, PrinterProfile, Values } from './types';

const DOC_KEY = storageKey('document.v1');
/** Modèles ouverts par l'utilisateur (texte complet), mémorisés sur le poste. */
const USER_KEY = storageKey('user-models.v1');
/** Taille des vignettes (px CSS). */
const THUMB = 44;

interface UserFile {
  id: string;
  file: string;
  text: string;
}

function readUserFiles(): UserFile[] {
  try {
    return JSON.parse(localStorage.getItem(USER_KEY) ?? '[]') as UserFile[];
  } catch {
    return [];
  }
}

/** Identifiant d'un modèle de l'utilisateur d'après son nom de fichier. */
const userId = (file: string) =>
  'user/' + file.replace(/\.weave$/i, '').toLowerCase().normalize('NFD').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
/** Au-delà, un geste passe en calcul au relâchement (ms). */
const SLOW_MS = 60;
const MAX_HISTORY = 100;

interface Doc {
  model: string;
  printer: string;
  filament: string;
  /** Valeurs modifiées par l'utilisateur, par modèle. */
  values: Record<string, Values>;
}

function readSaved(): Partial<Doc> {
  try {
    return JSON.parse(localStorage.getItem(DOC_KEY) ?? '{}') as Partial<Doc>;
  } catch {
    return {};
  }
}

/** Formats d'export : G-code seul, ou paquet Bambu Lab. */
export type ExportFormat = 'gcode' | '3mf';

export interface CatalogEntry {
  id: string;
  /** Livré avec FabWeave, ou ouvert par l'utilisateur (« Mes modèles »). */
  source: 'builtin' | 'user';
  /** Nom du fichier d'origine (modèles de l'utilisateur). */
  file?: string;
  text: string;
  info?: ModelInfo;
  issues?: Issue[];
}

export interface ProfileEntry<T> {
  id: string;
  profile?: T;
  issues?: Issue[];
}

class Store {
  engine = new EngineClient();
  status = $state<'loading' | 'ready' | 'failed'>('loading');
  version = $state('');

  catalog = $state<CatalogEntry[]>([]);
  printers = $state<ProfileEntry<PrinterProfile>[]>([]);
  filaments = $state<ProfileEntry<FilamentProfile>[]>([]);

  doc = $state<Doc>({ model: '', printer: 'prusa_mk4s', filament: 'generic_pla', values: {} });

  model = $state<ModelInfo | null>(null);
  printer = $state<PrinterProfile | null>(null);
  filament = $state<FilamentProfile | null>(null);
  /** Messages de chargement (modèle ou profil illisible). */
  loadIssues = $state<Issue[]>([]);

  /** Vignettes par identifiant de modèle (data URL ; vide : le modèle ne se calcule pas), pour le thème courant. */
  thumbs = $state<Record<string, string>>({});
  #thumbTheme = '';
  #thumbRun: Promise<void> | null = null;
  #thumbAgain = false;
  #userFiles: UserFile[] = readUserFiles();

  frame = $state.raw<FrameData | null>(null);
  computing = $state(false);
  /** Calcul au relâchement : le geste en cours ne recalcule plus. */
  deferred = $state(false);
  canUndo = $state(false);
  canRedo = $state(false);

  #undo: Doc[] = [];
  #redo: Doc[] = [];
  #lastKey: string | null = null;
  #gesture = false;
  /** Premier calcul du geste en cours encore attendu. */
  #probe = false;
  #dirty = false;
  #saveTimer: ReturnType<typeof setTimeout> | undefined;
  #loaded = { model: '', printer: '', filament: '' };

  constructor() {
    const s = readSaved();
    this.doc = { model: s.model ?? '', printer: s.printer ?? 'prusa_mk4s', filament: s.filament ?? 'generic_pla', values: s.values ?? {} };
    this.engine.onReady = (v) => {
      this.version = v;
      this.status = 'ready';
      void this.#start();
    };
    this.engine.onFailed = () => (this.status = 'failed');
    this.engine.onFrame = (f) => {
      this.frame = f;
      this.computing = false;
      if (this.#probe) {
        this.#probe = false;
        if (this.#gesture && f.ms > SLOW_MS) this.deferred = true;
      }
    };
    this.engine.onFrameError = () => (this.computing = false);
  }

  get modelId() {
    return this.doc.model;
  }

  get printerId() {
    return this.doc.printer;
  }

  get filamentId() {
    return this.doc.filament;
  }

  /** Valeurs modifiées du modèle affiché. */
  get current(): Values {
    return this.doc.values[this.doc.model] ?? {};
  }

  async #start() {
    const read = async <T>(kind: 'model' | 'printer' | 'filament', id: string, text: string) => {
      const r = JSON.parse(kind === 'model' ? await this.engine.read(text) : await this.engine.readProfile(kind, text)) as LoadResult<T>;
      return r.ok ? { id, info: r.model, profile: r.profile } : { id, issues: r.issues };
    };
    const [models, printers, filaments] = await Promise.all([
      Promise.all([
        ...MODEL_FILES.map(async (f) => ({ ...(await read<never>('model', f.id, f.text)), source: 'builtin' as const, text: f.text })),
        ...this.#userFiles.map(async (f) => ({ ...(await read<never>('model', f.id, f.text)), source: 'user' as const, file: f.file, text: f.text })),
      ]),
      Promise.all(PRINTER_FILES.map((f) => read<PrinterProfile>('printer', f.id, f.text))),
      Promise.all(FILAMENT_FILES.map((f) => read<FilamentProfile>('filament', f.id, f.text))),
    ]);
    this.catalog = models;
    this.printers = printers;
    this.filaments = filaments;
    const d = this.doc;
    if (!this.catalog.some((f) => f.id === d.model)) d.model = MODEL_FILES[0]?.id ?? '';
    if (!PRINTER_FILES.some((f) => f.id === d.printer)) d.printer = PRINTER_FILES[0]?.id ?? '';
    if (!FILAMENT_FILES.some((f) => f.id === d.filament)) d.filament = FILAMENT_FILES[0]?.id ?? '';
    await this.#sync();
  }

  async #load<T>(target: 'model' | 'printer' | 'filament', text: string | undefined): Promise<LoadResult<T> | null> {
    if (text === undefined) return null;
    return JSON.parse(await this.engine.load(target, text)) as LoadResult<T>;
  }

  /** Charge dans le moteur ce qui a changé (modèle, profils), puis calcule. */
  async #sync() {
    const d = $state.snapshot(this.doc);
    const issues: Issue[] = [];
    const todo: Promise<void>[] = [];
    if (d.model !== this.#loaded.model) {
      todo.push(
        this.#load<never>('model', this.catalog.find((x) => x.id === d.model)?.text).then((m) => {
          this.model = m?.ok ? (m.model ?? null) : null;
          if (m && !m.ok) issues.push(...m.issues);
          this.#loaded.model = d.model;
        }),
      );
    }
    if (d.printer !== this.#loaded.printer) {
      todo.push(
        this.#load<PrinterProfile>('printer', PRINTER_FILES.find((x) => x.id === d.printer)?.text).then((p) => {
          this.printer = p?.ok ? (p.profile ?? null) : null;
          if (p && !p.ok) issues.push(...p.issues);
          this.#loaded.printer = d.printer;
        }),
      );
    }
    if (d.filament !== this.#loaded.filament) {
      todo.push(
        this.#load<FilamentProfile>('filament', FILAMENT_FILES.find((x) => x.id === d.filament)?.text).then((f) => {
          this.filament = f?.ok ? (f.profile ?? null) : null;
          if (f && !f.ok) issues.push(...f.issues);
          this.#loaded.filament = d.filament;
        }),
      );
    }
    if (todo.length) {
      await Promise.all(todo);
      this.loadIssues = issues;
    }
    this.compute();
  }

  compute() {
    if (this.deferred) {
      this.#dirty = true;
      return;
    }
    this.computing = true;
    this.engine.compute(JSON.stringify(this.current));
  }

  #save() {
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => {
      try {
        localStorage.setItem(DOC_KEY, JSON.stringify($state.snapshot(this.doc)));
      } catch {
        /* stockage indisponible */
      }
    }, 400);
  }

  #snapshot(): Doc {
    return $state.snapshot(this.doc) as Doc;
  }

  #flags() {
    this.canUndo = this.#undo.length > 0;
    this.canRedo = this.#redo.length > 0;
  }

  /**
   * Modifie le document : une étape d'annulation, sauf pendant un geste ou
   * si `mergeKey` est celle de la modification précédente.
   */
  edit(fn: (d: Doc) => void, mergeKey?: string) {
    if (!this.#gesture && !(mergeKey && mergeKey === this.#lastKey)) {
      this.#undo.push(this.#snapshot());
      if (this.#undo.length > MAX_HISTORY) this.#undo.shift();
      this.#redo = [];
      this.#flags();
    }
    if (!this.#gesture) this.#lastKey = mergeKey ?? null;
    fn(this.doc);
    this.#save();
    void this.#sync();
  }

  /** Début d'un geste continu (défilement d'un libellé) : une seule étape. */
  begin() {
    this.#undo.push(this.#snapshot());
    if (this.#undo.length > MAX_HISTORY) this.#undo.shift();
    this.#redo = [];
    this.#flags();
    this.#gesture = true;
    this.#probe = true;
    this.#lastKey = null;
  }

  /** Fin du geste : calcule ce qui a été retenu pendant le relâchement. */
  end() {
    this.#gesture = false;
    this.#probe = false;
    if (this.deferred) {
      this.deferred = false;
      if (this.#dirty) {
        this.#dirty = false;
        this.compute();
      }
    }
  }

  #restore(d: Doc) {
    this.doc = d;
    this.#lastKey = null;
    this.#flags();
    this.#save();
    void this.#sync();
  }

  undo() {
    const d = this.#undo.pop();
    if (!d) return;
    this.#redo.push(this.#snapshot());
    this.#restore(d);
  }

  redo() {
    const d = this.#redo.pop();
    if (!d) return;
    this.#undo.push(this.#snapshot());
    this.#restore(d);
  }

  setValue(id: string, v: number | boolean | string) {
    this.edit((d) => {
      d.values[d.model] = { ...(d.values[d.model] ?? {}), [id]: v };
    }, `params.${id}`);
  }

  /** Revient aux valeurs du modèle : toutes, ou un paramètre. */
  reset(id?: string) {
    this.edit((d) => {
      const next = { ...(d.values[d.model] ?? {}) };
      if (id) delete next[id];
      d.values[d.model] = id ? next : {};
    });
  }

  select(kind: 'model' | 'printer' | 'filament', id: string) {
    this.edit((d) => {
      d[kind] = id;
    });
  }

  /** Vignettes manquantes (ou de l'autre thème), calculées une à une ;
   * un appel pendant le calcul relance un tour à la fin au lieu d'un second calcul. */
  async loadThumbs() {
    if (this.status !== 'ready' || !this.printer) return;
    if (this.#thumbRun) {
      this.#thumbAgain = true;
      return this.#thumbRun;
    }
    this.#thumbRun = (async () => {
      do {
        this.#thumbAgain = false;
        if (this.#thumbTheme !== theme.applied) {
          this.thumbs = {};
          this.#thumbTheme = theme.applied;
        }
        const printer = PRINTER_FILES.find((x) => x.id === this.doc.printer)?.text ?? '';
        const filament = FILAMENT_FILES.find((x) => x.id === this.doc.filament)?.text ?? '';
        for (const e of this.catalog) {
          if (e.id in this.thumbs || !e.info) continue;
          const pts = await this.engine.thumbnail(e.text, printer, filament);
          if (this.#thumbTheme !== theme.applied) break;
          this.thumbs = { ...this.thumbs, [e.id]: renderThumb(pts, THUMB) };
        }
      } while (this.#thumbAgain || this.#thumbTheme !== theme.applied);
    })().finally(() => (this.#thumbRun = null));
    return this.#thumbRun;
  }

  #saveUserFiles() {
    try {
      localStorage.setItem(USER_KEY, JSON.stringify(this.#userFiles));
    } catch {
      /* stockage indisponible ou plein : le modèle reste ouvert pour la session */
    }
  }

  /** Ajoute (ou remplace, même nom de fichier) un modèle dans « Mes modèles » et l'affiche. */
  async openFile(file: string, text: string) {
    const id = userId(file);
    const r = JSON.parse(await this.engine.read(text)) as LoadResult<never>;
    const entry: CatalogEntry = r.ok ? { id, source: 'user', file, text, info: r.model } : { id, source: 'user', file, text, issues: r.issues };
    this.#userFiles = [...this.#userFiles.filter((f) => f.id !== id), { id, file, text }];
    this.#saveUserFiles();
    this.catalog = [...this.catalog.filter((e) => e.id !== id), entry];
    const { [id]: _, ...rest } = this.thumbs;
    this.thumbs = rest;
    // Forcer la relecture si c'est le modèle affiché (fichier modifié puis redéposé).
    if (this.#loaded.model === id) this.#loaded.model = '';
    this.select('model', id);
    void this.loadThumbs();
    return entry;
  }

  /** Retire un modèle de « Mes modèles » ; rend de quoi le remettre (`openFile`). */
  removeFile(id: string): { file: string; text: string } | undefined {
    const removed = this.#userFiles.find((f) => f.id === id);
    this.#userFiles = this.#userFiles.filter((f) => f.id !== id);
    this.#saveUserFiles();
    this.catalog = this.catalog.filter((e) => e.id !== id);
    if (this.doc.model === id) this.select('model', MODEL_FILES[0]?.id ?? '');
    return removed && { file: removed.file, text: removed.text };
  }

  /** Fichier à exporter pour les valeurs courantes (G-code, ou paquet
   * `.gcode.3mf` des Bambu Lab), ou les messages qui le bloquent. */
  async exportFile(format: ExportFormat): Promise<{ ok: true; data: string | Uint8Array } | { ok: false; issues: Issue[] }> {
    try {
      const values = JSON.stringify(this.current);
      return { ok: true, data: format === '3mf' ? await this.engine.gcode3mf(values) : await this.engine.gcode(values) };
    } catch (e) {
      try {
        return { ok: false, issues: JSON.parse((e as Error).message) as Issue[] };
      } catch {
        return { ok: false, issues: [{ code: 'export.failed', severity: 'error', detail: String(e) }] };
      }
    }
  }
}

export const store = new Store();
