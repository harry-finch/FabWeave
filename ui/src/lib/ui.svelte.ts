// État de l'interface commun à toutes les applications : unités d'affichage,
// aide, messages passagers. L'état métier vit dans le store de l'application.

import { storageKey } from './app';
import type { UnitSystem } from './format';

const UNITS_KEY = storageKey('units.v1');

function readUnits(): UnitSystem {
  try {
    return localStorage.getItem(UNITS_KEY) === 'in' ? 'in' : 'mm';
  } catch {
    return 'mm';
  }
}

export interface Toast {
  id: number;
  text: string;
  /** Action proposée dans le message (« Annuler » après un retrait). */
  action?: { label: string; run: () => void };
}

class Ui {
  #units = $state<UnitSystem>(readUnits());
  help = $state(false);
  toasts = $state<Toast[]>([]);
  #next = 0;

  get units() {
    return this.#units;
  }

  set units(u: UnitSystem) {
    this.#units = u;
    try {
      localStorage.setItem(UNITS_KEY, u);
    } catch {
      /* ignoré */
    }
  }

  /** Message passager en bas de l'écran (confirmation d'une action) ;
   * avec une action, il reste 6 s et disparaît dès qu'elle est choisie. */
  toast(text: string, action?: Toast['action']) {
    const id = ++this.#next;
    const close = () => (this.toasts = this.toasts.filter((x) => x.id !== id));
    const wrapped = action && { label: action.label, run: () => (close(), action.run()) };
    this.toasts = [...this.toasts, { id, text, action: wrapped }];
    setTimeout(close, action ? 6000 : 2600);
  }
}

export const ui = new Ui();
