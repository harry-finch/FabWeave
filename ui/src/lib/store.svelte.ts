// État de l'application : moteur, catalogue, modèle et profils choisis,
// valeurs des curseurs (par modèle), dernier calcul. Le choix et les valeurs
// sont mémorisés sur le poste 400 ms après chaque modification et repris au
// lancement.

import { EngineClient } from './engine';
import { FILAMENT_FILES, MODEL_FILES, PRINTER_FILES } from './catalog';
import { storageKey } from './app';
import type { FilamentProfile, FrameData, Issue, LoadResult, ModelInfo, PrinterProfile, Values } from './types';

const DOC_KEY = storageKey('document.v1');

interface Saved {
  model?: string;
  printer?: string;
  filament?: string;
  /** Valeurs modifiées par l'utilisateur, par modèle. */
  values?: Record<string, Values>;
}

function readSaved(): Saved {
  try {
    return JSON.parse(localStorage.getItem(DOC_KEY) ?? '{}') as Saved;
  } catch {
    return {};
  }
}

export interface CatalogEntry {
  id: string;
  info?: ModelInfo;
  issues?: Issue[];
}

class Store {
  engine = new EngineClient();
  status = $state<'loading' | 'ready' | 'failed'>('loading');
  version = $state('');

  catalog = $state<CatalogEntry[]>([]);
  modelId = $state('');
  printerId = $state('prusa_mk4s');
  filamentId = $state('generic_pla');
  values = $state<Record<string, Values>>({});

  model = $state<ModelInfo | null>(null);
  printer = $state<PrinterProfile | null>(null);
  filament = $state<FilamentProfile | null>(null);
  /** Messages de chargement (modèle ou profil illisible). */
  loadIssues = $state<Issue[]>([]);

  frame = $state.raw<FrameData | null>(null);
  computing = $state(false);
  #saveTimer: ReturnType<typeof setTimeout> | undefined;

  constructor() {
    const s = readSaved();
    this.modelId = s.model ?? '';
    this.printerId = s.printer ?? this.printerId;
    this.filamentId = s.filament ?? this.filamentId;
    this.values = s.values ?? {};
    this.engine.onReady = (v) => {
      this.version = v;
      this.status = 'ready';
      void this.#start();
    };
    this.engine.onFailed = () => (this.status = 'failed');
    this.engine.onFrame = (f) => {
      this.frame = f;
      this.computing = false;
    };
    this.engine.onFrameError = () => (this.computing = false);
  }

  /** Valeurs courantes du modèle affiché (modifiées seulement). */
  get current(): Values {
    return this.values[this.modelId] ?? {};
  }

  async #start() {
    this.catalog = await Promise.all(
      MODEL_FILES.map(async (f) => {
        const r = JSON.parse(await this.engine.read(f.text)) as LoadResult<never>;
        return r.ok ? { id: f.id, info: r.model } : { id: f.id, issues: r.issues };
      }),
    );
    if (!MODEL_FILES.some((f) => f.id === this.modelId)) this.modelId = MODEL_FILES[0]?.id ?? '';
    if (!PRINTER_FILES.some((f) => f.id === this.printerId)) this.printerId = PRINTER_FILES[0]?.id ?? '';
    if (!FILAMENT_FILES.some((f) => f.id === this.filamentId)) this.filamentId = FILAMENT_FILES[0]?.id ?? '';
    await this.#loadAll();
  }

  async #load<T>(target: 'model' | 'printer' | 'filament', text: string | undefined): Promise<LoadResult<T> | null> {
    if (text === undefined) return null;
    return JSON.parse(await this.engine.load(target, text)) as LoadResult<T>;
  }

  async #loadAll() {
    const [m, p, f] = await Promise.all([
      this.#load<never>('model', MODEL_FILES.find((x) => x.id === this.modelId)?.text),
      this.#load<PrinterProfile>('printer', PRINTER_FILES.find((x) => x.id === this.printerId)?.text),
      this.#load<FilamentProfile>('filament', FILAMENT_FILES.find((x) => x.id === this.filamentId)?.text),
    ]);
    const issues: Issue[] = [];
    this.model = m?.ok ? (m.model ?? null) : null;
    if (m && !m.ok) issues.push(...m.issues);
    this.printer = p?.ok ? (p.profile ?? null) : null;
    if (p && !p.ok) issues.push(...p.issues);
    this.filament = f?.ok ? (f.profile ?? null) : null;
    if (f && !f.ok) issues.push(...f.issues);
    this.loadIssues = issues;
    this.compute();
  }

  compute() {
    this.computing = true;
    this.engine.compute(JSON.stringify(this.current));
  }

  #save() {
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => {
      try {
        const doc: Saved = { model: this.modelId, printer: this.printerId, filament: this.filamentId, values: this.values };
        localStorage.setItem(DOC_KEY, JSON.stringify(doc));
      } catch {
        /* stockage indisponible */
      }
    }, 400);
  }

  setValue(id: string, v: number | boolean) {
    this.values = { ...this.values, [this.modelId]: { ...this.current, [id]: v } };
    this.compute();
    this.#save();
  }

  /** Revient aux valeurs par défaut du modèle (toutes, ou un paramètre). */
  reset(id?: string) {
    const next = { ...this.current };
    if (id) delete next[id];
    this.values = { ...this.values, [this.modelId]: id ? next : {} };
    this.compute();
    this.#save();
  }

  async select(kind: 'model' | 'printer' | 'filament', id: string) {
    if (kind === 'model') this.modelId = id;
    if (kind === 'printer') this.printerId = id;
    if (kind === 'filament') this.filamentId = id;
    this.#save();
    await this.#loadAll();
  }

  /** G-code des valeurs courantes, ou les messages qui le bloquent. */
  async gcode(): Promise<{ ok: true; text: string } | { ok: false; issues: Issue[] }> {
    try {
      return { ok: true, text: await this.engine.gcode(JSON.stringify(this.current)) };
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
