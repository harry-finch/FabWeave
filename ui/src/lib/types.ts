// Types du modèle, des profils et du rapport, calqués sur `fabweave-core`
// (sérialisation serde). Millimètres, secondes ; repère de la machine pour
// les positions du parcours.

/** Texte d'un modèle : chaîne, ou table par langue. */
export type Text = string | Record<string, string>;

export type ParamDef = {
  label: Text;
  unit?: string;
  help?: Text;
  advanced: boolean;
} & (
  | { type: 'float'; default: number; min: number; max: number; step?: number }
  | { type: 'int'; default: number; min: number; max: number; step?: number }
  | { type: 'bool'; default: boolean }
);

export interface ModelMeta {
  name: Text;
  category: Text;
  description?: Text;
  author?: string;
  tags: string[];
  source?: string;
  license?: string;
  recommended_nozzle: number[];
}

export interface ModelInfo {
  meta: ModelMeta;
  params: Record<string, ParamDef>;
}

export type ParamValue = number | boolean;
export type Values = Record<string, ParamValue>;

export interface Issue {
  code: string;
  severity: 'error' | 'warning' | 'info';
  field?: string;
  value?: number;
  count?: number;
  line?: number;
  column?: number;
  detail?: string;
}

export interface PrinterProfile {
  printer: { id: string; name: string; model: string; firmware: string };
  bed: { size: [number, number]; max_height: number; origin: 'corner' | 'center' };
  nozzle: { diameter: number };
  layers: { layer_height: number; first_layer_height: number };
}

export interface FilamentProfile {
  filament: { id: string; name: Text; type: string };
}

export interface Bounds {
  min: [number, number, number];
  max: [number, number, number];
}

export interface Stats {
  points: number;
  print_moves: number;
  travel_moves: number;
  retractions: number;
  skipped: number;
  print_length: number;
  travel_length: number;
  time: number;
  filament_length: number;
  filament_volume: number;
  filament_mass: number;
  max_flow: number;
  max_requested_flow: number;
  max_speed: number;
  bounds: Bounds;
  first_layer_bounds: Bounds;
  limited: { first_layer: number; machine: number; flow: number; z_axis: number };
}

export interface Report {
  params: Values;
  stats: Stats | null;
  offset: [number, number, number] | null;
  first_layer_top: number | null;
  points: number;
  issues: Issue[];
  log: string[];
}

/** Parcours calculé, transféré par le worker. */
export interface FrameData {
  positions: Float32Array;
  kinds: Uint8Array;
  speeds: Float32Array;
  report: Report;
  /** Durée du calcul dans le worker (ms). */
  ms: number;
}

/** Réponse d'un chargement (modèle, profil). */
export type LoadResult<T> = { ok: true; model?: ModelInfo; profile?: T } | { ok: false; issues: Issue[] };
