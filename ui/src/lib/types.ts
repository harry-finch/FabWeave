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
  | { type: 'choice'; default: string; options: { value: string; label?: Text }[] }
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
  /** Adhésion conseillée par le modèle (l'utilisateur choisit). */
  recommended_adhesion?: 'none' | 'brim' | 'raft';
}

export interface ModelInfo {
  meta: ModelMeta;
  params: Record<string, ParamDef>;
}

export type ParamValue = number | boolean | string;
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
  /** Texte écrit par l'auteur du modèle (`warn`, `fail`). */
  text?: Text;
  /** Nom de l'élément du profil en cause (zone exclue du plateau). */
  name?: Text;
}

export interface Zone {
  name: Text;
  min: [number, number];
  max: [number, number];
}

export type Firmware = 'prusa' | 'marlin' | 'klipper' | 'bambu';

export interface NozzleLayers {
  diameter: number;
  layer_height: number;
  first_layer_height?: number;
}

export interface NozzleFlow {
  diameter?: number;
  high_flow?: boolean;
  max_volumetric: number;
}

/** Profil imprimante (`fabweave_core::profile::PrinterProfile`), complet pour l'éditeur. */
export interface PrinterProfile {
  format?: string;
  version?: number;
  printer: { id: string; name: Text; model: string; firmware: Firmware; source?: string; review?: string[] };
  bed: { size: [number, number]; max_height: number; origin: 'corner' | 'center'; shape?: 'rectangle' | 'circle'; plate?: string; exclude?: Zone[] };
  nozzle: {
    diameter: number;
    high_flow?: boolean;
    abrasive?: boolean;
    cone_angle?: number;
    cone_height?: number;
    choices?: number[];
    high_flow_available?: boolean;
    abrasive_available?: boolean;
    clearance_radius?: number;
    clearance_height?: number;
  };
  limits: {
    max_print_speed: number;
    travel_speed: number;
    max_z_speed: number;
    acceleration?: number;
    travel_acceleration?: number;
    first_layer_acceleration?: number;
    jerk?: number;
  };
  layers: { layer_height: number; first_layer_height: number; by_nozzle?: NozzleLayers[] };
  gcode: { thumbnails?: string[]; start: string; end: string };
}

/** Profil filament (`fabweave_core::profile::FilamentProfile`). */
export interface FilamentProfile {
  format?: string;
  version?: number;
  filament: { id: string; name: Text; type: string; diameter: number; density: number; flow_multiplier: number; pressure_advance: number; abrasive: boolean };
  temperature: { first_layer: number; nozzle: number; bed_first_layer: number; bed: number; probe: number; idle?: number | null; chamber: number };
  speed: { nominal: number; first_layer: number; max_volumetric: number; by_nozzle?: NozzleFlow[] };
  cooling: { fan: number; first_layer_fan: number; bridge_fan: number };
  retraction: { length: number; speed: number; z_hop: number; min_travel: number };
}

export type ProfileKind = 'printer' | 'filament';
export type AnyProfile = PrinterProfile | FilamentProfile;

/** Rapport d'import d'un profil de trancheur (`fabweave_core::import::Report`). */
export interface ImportReport {
  slicer: string;
  name: string;
  inherits?: string;
  base: string;
  taken: { field: string; key: string; value: string }[];
  missing: { field: string; fallback: 'base' | 'empty' | 'estimated' }[];
  ignored: string[];
  ignored_gcode: string[];
  untranslated: { field: string; line: number; text: string; reason: string }[];
  constants: { field: string; name: string; value: string }[];
}

/** Profil trouvé dans un fichier de trancheur. */
export interface ImportedItem {
  kind: ProfileKind;
  profile: AnyProfile;
  report: ImportReport;
  toml: string;
  issues: Issue[];
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

export interface Note {
  x: number;
  y: number;
  z: number;
  text: Text;
}

/** Préparation de l'impression (`fabweave_core::setup::Setup`). */
export interface NozzleSetup {
  diameter?: number;
  high_flow?: boolean;
  abrasive?: boolean;
}

export interface Placement {
  x?: number;
  y?: number;
  rotation: number;
  copies: [number, number];
  spacing?: number;
}

export type Adhesion =
  | { type: 'none' }
  | { type: 'brim'; width: number; gap: number }
  | { type: 'raft'; layers: number; gap: number; margin: number };

export interface Setup {
  nozzle: NozzleSetup;
  placement: Placement;
  adhesion: Adhesion;
}

/** Buse montée et ce qu'elle change (profils effectifs). */
export interface NozzleReport {
  diameter: number;
  high_flow: boolean;
  abrasive: boolean;
  layer_height: number;
  first_layer_height: number;
  max_volumetric: number;
}

export interface Report {
  notes: Note[];
  nozzle: NozzleReport | null;
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
  /** 1 : mouvement imprimé hors du volume ou dans une zone exclue. */
  outside: Uint8Array;
  report: Report;
  /** Durée du calcul dans le worker (ms). */
  ms: number;
}

/** Réponse d'un chargement (modèle, profil). Un profil lisible mais aux
 * valeurs fautives garde `profile` (lecture pour l'éditeur). */
export type LoadResult<T> = { ok: true; model?: ModelInfo; profile?: T; issues?: Issue[] } | { ok: false; issues: Issue[]; profile?: T };
