// Client du moteur (Web Worker). Calcul « le dernier gagne » : un seul
// calcul en cours ; si les valeurs changent pendant ce temps, seules les plus
// récentes sont envoyées ensuite. Chargements et export passent par des
// promesses, dans l'ordre d'envoi.

import type { FrameData } from './types';

type Pending = { resolve: (v: any) => void; reject: (e: Error) => void };

export class EngineClient {
  #worker: Worker;
  #busy = false;
  #queued: string | null = null;
  #seq = 0;
  #pending = new Map<number, Pending>();

  onReady: (version: string) => void = () => {};
  onFailed: (error: string) => void = () => {};
  onFrame: (frame: FrameData) => void = () => {};
  onFrameError: (error: string) => void = () => {};

  constructor() {
    this.#worker = new Worker(new URL('./engine.worker.ts', import.meta.url), { type: 'module' });
    this.#worker.onmessage = (ev) => this.#receive(ev.data);
    this.#worker.onerror = (ev) => this.onFailed(ev.message);
  }

  #receive(msg: any) {
    switch (msg.kind) {
      case 'ready':
        this.onReady(msg.version);
        break;
      case 'failed':
        this.onFailed(msg.error);
        break;
      case 'frame':
      case 'frame-error':
        if (msg.kind === 'frame') this.onFrame(msg.frame as FrameData);
        else this.onFrameError(msg.error);
        this.#busy = false;
        if (this.#queued !== null) {
          const next = this.#queued;
          this.#queued = null;
          this.compute(next);
        }
        break;
      case 'result': {
        const p = this.#pending.get(msg.id);
        this.#pending.delete(msg.id);
        if (msg.ok) p?.resolve(msg.value);
        else p?.reject(new Error(msg.error));
        break;
      }
    }
  }

  /** Calcule le parcours pour ces valeurs (JSON `{ id: valeur }`). */
  compute(values: string) {
    if (this.#busy) {
      this.#queued = values;
      return;
    }
    this.#busy = true;
    this.#worker.postMessage({ kind: 'compute', id: ++this.#seq, values });
  }

  #call<T = string>(msg: Record<string, unknown>): Promise<T> {
    const id = ++this.#seq;
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      this.#worker.postMessage({ ...msg, id });
    });
  }

  /** Charge un modèle ou un profil ; réponse JSON `{ ok, … }`. */
  load(target: 'model' | 'printer' | 'filament', text: string): Promise<string> {
    return this.#call({ kind: 'load', target, text });
  }

  /** Préparation (buse, placement, adhésion) ; réponse JSON `{ ok, issues? }`. */
  setup(setup: string): Promise<string> {
    return this.#call({ kind: 'setup', setup });
  }

  /** Description d'un modèle sans le charger (catalogue). */
  read(text: string): Promise<string> {
    return this.#call({ kind: 'read', text });
  }

  /** Profil modifié (JSON) : contrôlé et écrit en TOML ; réponse `{ ok, issues, toml }`. */
  checkProfile(kind: 'printer' | 'filament', profile: string): Promise<string> {
    return this.#call({ kind: 'check-profile', profile: kind, text: profile });
  }

  /** Profils d'un fichier de trancheur ; `bases` : `{ printers, filaments }` en TOML. */
  importProfiles(text: string, bases: string): Promise<string> {
    return this.#call({ kind: 'import', text, bases });
  }

  /** Profil sans le charger (listes des imprimantes et des filaments). */
  readProfile(kind: 'printer' | 'filament', text: string): Promise<string> {
    return this.#call({ kind: 'read-profile', profile: kind, text });
  }

  /** Parcours d'un modèle aux valeurs par défaut (`x, y, z, nature` par point), vide s'il échoue. */
  thumbnail(model: string, printer: string, filament: string): Promise<Float32Array> {
    return this.#call<Float32Array>({ kind: 'thumbnail', model, printer, filament });
  }

  /** Paquet `.gcode.3mf` (Bambu Lab) ; refus comme `gcode`. */
  gcode3mf(values: string): Promise<Uint8Array> {
    return this.#call<Uint8Array>({ kind: 'gcode-3mf', values });
  }

  /** G-code ; en cas de refus, l'erreur porte le JSON des messages bloquants. */
  gcode(values: string): Promise<string> {
    return this.#call({ kind: 'gcode', values });
  }
}
