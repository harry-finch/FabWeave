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
// - Préparation de l'impression : buse mémorisée par imprimante, placement et
//   adhésion dans le document (annulables comme le reste).
// - Profils de l'utilisateur (copies modifiées, imports) mémorisés sur le
//   poste à côté des profils livrés, jamais écrasés.

import { EngineClient } from './engine';
import { FILAMENT_FILES, MODEL_FILES, PRINTER_FILES } from './catalog';
import { storageKey } from './app';
import { renderThumb } from './thumbs';
import { theme } from './theme.svelte';
import { text as textOf } from './text';
import type {
  Adhesion,
  AnyProfile,
  FilamentProfile,
  FrameData,
  ImportedItem,
  ImportReport,
  Issue,
  LoadResult,
  ModelInfo,
  NozzleSetup,
  Placement,
  PrinterProfile,
  ProfileKind,
  Setup,
  Values,
} from './types';

const DOC_KEY = storageKey('document.v1');
/** Modèles ouverts par l'utilisateur (texte complet), mémorisés sur le poste. */
const USER_KEY = storageKey('user-models.v1');
/** Profils de l'utilisateur (TOML), mémorisés sur le poste. */
const PROFILES_KEY = storageKey('user-profiles.v1');
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

/** Identifiant d'un profil d'après son nom (minuscules ASCII et `_`, comme l'import du moteur). */
const profileSlug = (name: string) =>
  name
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '');

/** Profil de l'utilisateur : texte TOML et d'où il vient. */
export interface UserProfile {
  id: string;
  kind: ProfileKind;
  text: string;
  origin:
    | { type: 'copy'; from: string; name: string }
    | { type: 'import'; file: string; report: ImportReport }
    | { type: 'file'; file: string };
  /** Valeurs à la création (copie, import) : chaque valeur modifiée depuis a son ↺. */
  initial: AnyProfile;
}

function readUserProfiles(): UserProfile[] {
  try {
    return JSON.parse(localStorage.getItem(PROFILES_KEY) ?? '[]') as UserProfile[];
  } catch {
    return [];
  }
}

const BUILTIN: Record<ProfileKind, { id: string; text: string }[]> = { printer: PRINTER_FILES, filament: FILAMENT_FILES };
/** Au-delà, un geste passe en calcul au relâchement (ms). */
const SLOW_MS = 60;
const MAX_HISTORY = 100;

interface Doc {
  model: string;
  printer: string;
  filament: string;
  /** Valeurs modifiées par l'utilisateur, par modèle. */
  values: Record<string, Values>;
  /** Buse montée, par imprimante (vide : celle du profil). */
  nozzles: Record<string, NozzleSetup>;
  placement: Placement;
  adhesion: Adhesion;
}

export const DEFAULT_PLACEMENT: Placement = { rotation: 0, copies: [1, 1] };
/** Réglages proposés à la première activation (valeurs par défaut du moteur). */
export const BRIM: Extract<Adhesion, { type: 'brim' }> = { type: 'brim', width: 5, gap: 0.1 };
export const RAFT: Extract<Adhesion, { type: 'raft' }> = { type: 'raft', layers: 3, gap: 0.2, margin: 3 };

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
  source: 'builtin' | 'user';
  text: string;
  /** Profil lu (même aux valeurs fautives, pour l'éditeur). */
  profile?: T;
  /** Erreurs de lecture ou de valeurs : le profil ne se charge pas. */
  issues?: Issue[];
  user?: UserProfile;
}

class Store {
  engine = new EngineClient();
  status = $state<'loading' | 'ready' | 'failed'>('loading');
  version = $state('');

  catalog = $state<CatalogEntry[]>([]);
  printers = $state<ProfileEntry<PrinterProfile>[]>([]);
  filaments = $state<ProfileEntry<FilamentProfile>[]>([]);

  doc = $state<Doc>({ model: '', printer: 'prusa_mk4s', filament: 'generic_pla', values: {}, nozzles: {}, placement: DEFAULT_PLACEMENT, adhesion: { type: 'none' } });

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
  #userProfiles: UserProfile[] = readUserProfiles();

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
  /** Ce qui est chargé dans le moteur : identifiant du modèle, texte des profils. */
  #loaded = { model: '', printer: '', filament: '', setup: '' };
  /** Messages de la préparation refusée par le moteur. */
  #setupIssues: Issue[] = [];

  constructor() {
    const s = readSaved();
    this.doc = {
      model: s.model ?? '',
      printer: s.printer ?? 'prusa_mk4s',
      filament: s.filament ?? 'generic_pla',
      values: s.values ?? {},
      nozzles: s.nozzles ?? {},
      placement: { ...DEFAULT_PLACEMENT, ...s.placement },
      adhesion: s.adhesion ?? { type: 'none' },
    };
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

  /** Buse choisie pour l'imprimante courante. */
  get nozzle(): NozzleSetup {
    return this.doc.nozzles[this.doc.printer] ?? {};
  }

  /** Préparation envoyée au moteur. */
  get setup(): Setup {
    return { nozzle: this.nozzle, placement: this.doc.placement, adhesion: this.doc.adhesion };
  }

  /** Valeurs modifiées du modèle affiché. */
  get current(): Values {
    return this.doc.values[this.doc.model] ?? {};
  }

  async #start() {
    const read = async (id: string, text: string) => {
      const r = JSON.parse(await this.engine.read(text)) as LoadResult<never>;
      return r.ok ? { id, info: r.model } : { id, issues: r.issues };
    };
    const [models] = await Promise.all([
      Promise.all([
        ...MODEL_FILES.map(async (f) => ({ ...(await read(f.id, f.text)), source: 'builtin' as const, text: f.text })),
        ...this.#userFiles.map(async (f) => ({ ...(await read(f.id, f.text)), source: 'user' as const, file: f.file, text: f.text })),
      ]),
      this.#readProfiles(),
    ]);
    this.catalog = models;
    const d = this.doc;
    if (!this.catalog.some((f) => f.id === d.model)) d.model = MODEL_FILES[0]?.id ?? '';
    if (!this.printers.some((f) => f.id === d.printer)) d.printer = PRINTER_FILES[0]?.id ?? '';
    if (!this.filaments.some((f) => f.id === d.filament)) d.filament = FILAMENT_FILES[0]?.id ?? '';
    await this.#sync();
  }

  async #entry<T>(kind: ProfileKind, id: string, text: string, user?: UserProfile): Promise<ProfileEntry<T>> {
    const r = JSON.parse(await this.engine.readProfile(kind, text)) as LoadResult<T>;
    const issues = r.issues?.filter((i) => i.severity === 'error');
    return { id, source: user ? 'user' : 'builtin', text, profile: r.profile, issues: issues?.length ? issues : undefined, user };
  }

  /** Listes des imprimantes et des filaments : livrés, puis ceux de l'utilisateur. */
  async #readProfiles() {
    const list = <T>(kind: ProfileKind) =>
      Promise.all([
        ...BUILTIN[kind].map((f) => this.#entry<T>(kind, f.id, f.text)),
        ...this.#userProfiles.filter((u) => u.kind === kind).map((u) => this.#entry<T>(kind, u.id, u.text, u)),
      ]);
    const [printers, filaments] = await Promise.all([list<PrinterProfile>('printer'), list<FilamentProfile>('filament')]);
    this.printers = printers;
    this.filaments = filaments;
  }

  /** Texte TOML d'un profil (livré ou de l'utilisateur). */
  profileText(kind: ProfileKind, id: string): string | undefined {
    return BUILTIN[kind].find((f) => f.id === id)?.text ?? this.#userProfiles.find((u) => u.kind === kind && u.id === id)?.text;
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
    // Profils comparés par leur texte : un profil modifié se recharge.
    const printerText = this.profileText('printer', d.printer);
    if (printerText === undefined) issues.push({ code: 'profile.not_found', severity: 'error', field: 'printer' });
    if (printerText !== undefined && printerText !== this.#loaded.printer) {
      todo.push(
        this.#load<PrinterProfile>('printer', printerText).then((p) => {
          this.printer = p?.ok ? (p.profile ?? null) : null;
          if (p && !p.ok) issues.push(...p.issues);
          this.#loaded.printer = printerText;
        }),
      );
    }
    const filamentText = this.profileText('filament', d.filament);
    if (filamentText === undefined) issues.push({ code: 'profile.not_found', severity: 'error', field: 'filament' });
    if (filamentText !== undefined && filamentText !== this.#loaded.filament) {
      todo.push(
        this.#load<FilamentProfile>('filament', filamentText).then((f) => {
          this.filament = f?.ok ? (f.profile ?? null) : null;
          if (f && !f.ok) issues.push(...f.issues);
          this.#loaded.filament = filamentText;
        }),
      );
    }
    if (todo.length || issues.length) {
      await Promise.all(todo);
      // Un profil dont le chargement échoue garde ses messages jusqu'au suivant.
      const kept = todo.length ? [] : this.loadIssues.filter((i) => !this.#setupIssues.includes(i) && i.code !== 'profile.not_found');
      this.loadIssues = [...kept, ...issues, ...this.#setupIssues];
    }
    const setup = JSON.stringify($state.snapshot(this.setup));
    if (setup !== this.#loaded.setup) {
      const r = JSON.parse(await this.engine.setup(setup)) as { ok: boolean; issues?: Issue[] };
      this.#loaded.setup = setup;
      const kept = this.loadIssues.filter((i) => !this.#setupIssues.includes(i));
      this.#setupIssues = r.ok ? [] : (r.issues ?? []);
      this.loadIssues = [...kept, ...this.#setupIssues];
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

  /** Buse de l'imprimante courante (fusionne avec le choix précédent). */
  setNozzle(patch: NozzleSetup) {
    this.edit((d) => {
      const next = { ...(d.nozzles[d.printer] ?? {}), ...patch };
      for (const k of Object.keys(next) as (keyof NozzleSetup)[]) if (next[k] === undefined) delete next[k];
      d.nozzles = { ...d.nozzles, [d.printer]: next };
    });
  }

  /** Placement ; les modifications successives d'un même champ fusionnent. */
  setPlacement(patch: Partial<Placement>, key?: string) {
    this.edit((d) => {
      const next = { ...d.placement, ...patch };
      for (const k of ['x', 'y', 'spacing'] as const) if (next[k] === undefined) delete next[k];
      d.placement = next;
    }, key && `placement.${key}`);
  }

  setAdhesion(a: Adhesion, key?: string) {
    this.edit((d) => {
      d.adhesion = a;
    }, key && `adhesion.${key}`);
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
        const printer = this.profileText('printer', this.doc.printer) ?? '';
        const filament = this.profileText('filament', this.doc.filament) ?? '';
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

  #saveProfiles() {
    try {
      localStorage.setItem(PROFILES_KEY, JSON.stringify(this.#userProfiles));
    } catch {
      /* stockage indisponible ou plein : le profil reste pour la session */
    }
  }

  /** Identifiant libre `user/<nom>` (suffixe -2, -3… si besoin). */
  #freeId(kind: ProfileKind, name: string): string {
    const base = `user/${profileSlug(name) || kind}`;
    const taken = (id: string) => !!this.profileText(kind, id);
    let id = base;
    for (let n = 2; taken(id); n++) id = `${base}-${n}`;
    return id;
  }

  /**
   * Enregistre un profil de l'utilisateur : nouveau (copie, import) ou
   * modifié (même identifiant). Contrôlé par le moteur ; refusé s'il a une
   * erreur. Le profil enregistré devient le profil choisi.
   */
  async saveProfile(
    kind: ProfileKind,
    profile: AnyProfile,
    opts: { id?: string; origin?: UserProfile['origin']; initial?: AnyProfile; select?: boolean } = {},
  ): Promise<{ ok: true; id: string } | { ok: false; issues: Issue[] }> {
    const p = $state.snapshot(profile) as AnyProfile;
    const info = kind === 'printer' ? (p as PrinterProfile).printer : (p as FilamentProfile).filament;
    const id = opts.id ?? this.#freeId(kind, textOf(info.name));
    info.id = id.replace(/^user\//, '');
    const r = JSON.parse(await this.engine.checkProfile(kind, JSON.stringify(p))) as { ok: boolean; issues: Issue[]; toml?: string };
    if (!r.ok || !r.toml) return { ok: false, issues: r.issues };
    const old = this.#userProfiles.find((u) => u.id === id && u.kind === kind);
    const user: UserProfile = {
      id,
      kind,
      text: r.toml,
      origin: opts.origin ?? old?.origin ?? { type: 'file', file: `${info.id}.toml` },
      initial: opts.initial ?? old?.initial ?? p,
    };
    this.#userProfiles = [...this.#userProfiles.filter((u) => !(u.id === id && u.kind === kind)), user];
    this.#saveProfiles();
    await this.#readProfiles();
    if (opts.select !== false && this.doc[kind] !== id) this.select(kind, id);
    else void this.#sync();
    return { ok: true, id };
  }

  /** Ajoute tel quel un profil de l'utilisateur (import, annulation d'une suppression). */
  async #addUser(user: UserProfile) {
    this.#userProfiles = [...this.#userProfiles.filter((u) => !(u.id === user.id && u.kind === user.kind)), user];
    this.#saveProfiles();
    await this.#readProfiles();
  }

  /** Retire un profil de l'utilisateur ; rend de quoi le remettre (`restoreProfile`). */
  async deleteProfile(kind: ProfileKind, id: string): Promise<UserProfile | undefined> {
    const removed = this.#userProfiles.find((u) => u.kind === kind && u.id === id);
    if (!removed) return undefined;
    this.#userProfiles = this.#userProfiles.filter((u) => u !== removed);
    this.#saveProfiles();
    await this.#readProfiles();
    if (this.doc[kind] === id) {
      // Retour au profil d'où il venait (copie, base de l'import), sinon au premier livré.
      const o = removed.origin;
      const from = o.type === 'copy' ? o.from : o.type === 'import' ? o.report.base : '';
      this.select(kind, this.profileText(kind, from) !== undefined ? from : (BUILTIN[kind][0]?.id ?? ''));
    }
    return removed;
  }

  async restoreProfile(user: UserProfile, select = true) {
    await this.#addUser(user);
    if (select) this.select(user.kind, user.id);
  }

  /**
   * Fichier de profil déposé ou ouvert : un profil FabWeave (`.toml`) est
   * ajouté tel quel ; un fichier de trancheur (`.ini`, `.json`) rend les
   * profils trouvés, à confirmer (`addImported`).
   */
  async readProfileFile(file: string, text: string): Promise<{ added: ProfileEntry<AnyProfile>[] } | { items: ImportedItem[] } | { issues: Issue[] }> {
    if (/\.toml$/i.test(file)) {
      const kind: ProfileKind | undefined = /format\s*=\s*"fabweave\.printer"/.test(text)
        ? 'printer'
        : /format\s*=\s*"fabweave\.filament"/.test(text)
          ? 'filament'
          : /^\s*\[printer\]/m.test(text)
            ? 'printer'
            : /^\s*\[filament\]/m.test(text)
              ? 'filament'
              : undefined;
      if (!kind) return { issues: [{ code: 'import.no_profile', severity: 'error' }] };
      const r = JSON.parse(await this.engine.readProfile(kind, text)) as LoadResult<AnyProfile>;
      if (!r.profile) return { issues: r.issues ?? [] };
      const info = kind === 'printer' ? (r.profile as PrinterProfile).printer : (r.profile as FilamentProfile).filament;
      const id = this.#freeId(kind, info.id || textOf(info.name));
      await this.#addUser({ id, kind, text, origin: { type: 'file', file }, initial: r.profile });
      this.select(kind, id);
      const entry = (kind === 'printer' ? this.printers : this.filaments).find((e) => e.id === id) as ProfileEntry<AnyProfile>;
      return { added: [entry] };
    }
    const bases = (kind: ProfileKind) => {
      // Profil choisi en premier : il sert de base s'il convient.
      const first = BUILTIN[kind].find((f) => f.id === this.doc[kind]);
      return [first, ...BUILTIN[kind].filter((f) => f !== first)].filter(Boolean).map((f) => f!.text);
    };
    const r = JSON.parse(await this.engine.importProfiles(text, JSON.stringify({ printers: bases('printer'), filaments: bases('filament') }))) as
      | { ok: true; profiles: ImportedItem[] }
      | { ok: false; issues: Issue[] };
    return r.ok ? { items: r.profiles } : { issues: r.issues };
  }

  /** Ajoute les profils importés choisis ; le dernier de chaque sorte devient le profil choisi. */
  async addImported(file: string, items: ImportedItem[]): Promise<string[]> {
    const ids: string[] = [];
    for (const it of items) {
      const info = it.kind === 'printer' ? (it.profile as PrinterProfile).printer : (it.profile as FilamentProfile).filament;
      const id = this.#freeId(it.kind, textOf(info.name));
      info.id = id.replace(/^user\//, '');
      // Texte du moteur, identifiant mis à jour (le profil peut avoir des
      // erreurs à corriger dans l'éditeur).
      const text = it.toml.replace(/^id = ".*"$/m, `id = "${info.id}"`);
      await this.#addUser({ id, kind: it.kind, text, origin: { type: 'import', file, report: it.report }, initial: it.profile });
      ids.push(id);
    }
    const last = (kind: ProfileKind) => [...items].reverse().findIndex((i) => i.kind === kind);
    this.edit((d) => {
      for (const kind of ['printer', 'filament'] as const) {
        const k = last(kind);
        if (k >= 0) d[kind] = ids[items.length - 1 - k];
      }
    });
    return ids;
  }

  /** Profil en TOML pour l'export (`<identifiant>.toml`). */
  exportProfile(kind: ProfileKind, id: string): { name: string; text: string } | undefined {
    const text = this.profileText(kind, id);
    return text === undefined ? undefined : { name: `${id.replace(/^user\//, '')}.toml`, text };
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
