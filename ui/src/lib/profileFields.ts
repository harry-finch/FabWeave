// Champs des profils dans l'éditeur, par groupe. Le chemin (`path`) est
// celui du profil TOML et des messages du moteur (`temperature.nozzle`) ;
// `key` donne le libellé (`pf.<key>`). Un même chemin peut avoir plusieurs
// lignes (`bed.size` : X et Y, par `index`).

import type { ProfileKind } from './types';

export type Column =
  | { key: string; type: 'num'; optional?: boolean; unit?: string }
  | { key: string; type: 'text' }
  | { key: string; type: 'tri' };

export type FieldDef = { path: string; key: string } & (
  | { type: 'num'; unit: string; step?: number; decimals?: number; min?: number; index?: number }
  | { type: 'opt'; unit: string; step?: number; decimals?: number; min?: number }
  | { type: 'text' }
  | { type: 'name' }
  | { type: 'bool' }
  | { type: 'choice'; options: string[] }
  | { type: 'numlist'; unit: string }
  | { type: 'strlist' }
  | { type: 'table'; columns: Column[] }
  | { type: 'template' }
);

export interface Group {
  id: string;
  fields: FieldDef[];
}

const temp = (path: string, key: string): FieldDef => ({ path, key, type: 'num', unit: '°C', step: 5, decimals: 0, min: 0 });
const mm = (path: string, key: string, step = 0.1, decimals = 2): FieldDef => ({ path, key, type: 'num', unit: 'mm', step, decimals, min: 0 });
const speed = (path: string, key: string): FieldDef => ({ path, key, type: 'num', unit: 'mm/s', step: 5, decimals: 0, min: 0 });
const opt = (path: string, key: string, unit: string, step = 100, decimals = 0): FieldDef => ({ path, key, type: 'opt', unit, step, decimals, min: 0 });

export const PRINTER_GROUPS: Group[] = [
  {
    id: 'printer',
    fields: [
      { path: 'printer.name', key: 'printer.name', type: 'name' },
      { path: 'printer.model', key: 'printer.model', type: 'text' },
      { path: 'printer.firmware', key: 'printer.firmware', type: 'choice', options: ['prusa', 'marlin', 'klipper', 'bambu'] },
    ],
  },
  {
    id: 'bed',
    fields: [
      { path: 'bed.size', key: 'bed.sizeX', type: 'num', unit: 'mm', step: 1, decimals: 1, min: 0, index: 0 },
      { path: 'bed.size', key: 'bed.sizeY', type: 'num', unit: 'mm', step: 1, decimals: 1, min: 0, index: 1 },
      mm('bed.max_height', 'bed.max_height', 1, 1),
      { path: 'bed.shape', key: 'bed.shape', type: 'choice', options: ['rectangle', 'circle'] },
      { path: 'bed.origin', key: 'bed.origin', type: 'choice', options: ['corner', 'center'] },
      { path: 'bed.plate', key: 'bed.plate', type: 'text' },
      {
        path: 'bed.exclude',
        key: 'bed.exclude',
        type: 'table',
        columns: [
          { key: 'name', type: 'text' },
          { key: 'min.0', type: 'num' },
          { key: 'min.1', type: 'num' },
          { key: 'max.0', type: 'num' },
          { key: 'max.1', type: 'num' },
        ],
      },
    ],
  },
  {
    id: 'nozzle',
    fields: [
      mm('nozzle.diameter', 'nozzle.diameter', 0.05, 2),
      { path: 'nozzle.choices', key: 'nozzle.choices', type: 'numlist', unit: 'mm' },
      { path: 'nozzle.high_flow', key: 'nozzle.high_flow', type: 'bool' },
      { path: 'nozzle.abrasive', key: 'nozzle.abrasive', type: 'bool' },
      { path: 'nozzle.high_flow_available', key: 'nozzle.high_flow_available', type: 'bool' },
      { path: 'nozzle.abrasive_available', key: 'nozzle.abrasive_available', type: 'bool' },
      opt('nozzle.cone_angle', 'nozzle.cone_angle', '°', 5, 0),
      opt('nozzle.cone_height', 'nozzle.cone_height', 'mm', 0.5, 1),
      opt('nozzle.clearance_radius', 'nozzle.clearance_radius', 'mm', 1, 0),
      opt('nozzle.clearance_height', 'nozzle.clearance_height', 'mm', 1, 0),
    ],
  },
  {
    id: 'limits',
    fields: [
      speed('limits.max_print_speed', 'limits.max_print_speed'),
      speed('limits.travel_speed', 'limits.travel_speed'),
      { path: 'limits.max_z_speed', key: 'limits.max_z_speed', type: 'num', unit: 'mm/s', step: 1, decimals: 1, min: 0 },
      opt('limits.acceleration', 'limits.acceleration', 'mm/s²'),
      opt('limits.travel_acceleration', 'limits.travel_acceleration', 'mm/s²'),
      opt('limits.first_layer_acceleration', 'limits.first_layer_acceleration', 'mm/s²'),
      opt('limits.jerk', 'limits.jerk', 'mm/s', 1, 1),
    ],
  },
  {
    id: 'layers',
    fields: [
      mm('layers.layer_height', 'layers.layer_height', 0.01, 3),
      mm('layers.first_layer_height', 'layers.first_layer_height', 0.01, 3),
      {
        path: 'layers.by_nozzle',
        key: 'layers.by_nozzle',
        type: 'table',
        columns: [
          { key: 'diameter', type: 'num' },
          { key: 'layer_height', type: 'num' },
          { key: 'first_layer_height', type: 'num', optional: true },
        ],
      },
    ],
  },
  {
    id: 'gcode',
    fields: [
      { path: 'gcode.thumbnails', key: 'gcode.thumbnails', type: 'strlist' },
      { path: 'gcode.start', key: 'gcode.start', type: 'template' },
      { path: 'gcode.end', key: 'gcode.end', type: 'template' },
    ],
  },
];

export const FILAMENT_GROUPS: Group[] = [
  {
    id: 'filament',
    fields: [
      { path: 'filament.name', key: 'filament.name', type: 'name' },
      { path: 'filament.type', key: 'filament.type', type: 'text' },
      mm('filament.diameter', 'filament.diameter', 0.05, 2),
      { path: 'filament.density', key: 'filament.density', type: 'num', unit: 'g/cm³', step: 0.01, decimals: 2, min: 0 },
      { path: 'filament.flow_multiplier', key: 'filament.flow_multiplier', type: 'num', unit: '×', step: 0.01, decimals: 3, min: 0 },
      { path: 'filament.pressure_advance', key: 'filament.pressure_advance', type: 'num', unit: 's', step: 0.005, decimals: 3, min: 0 },
      { path: 'filament.abrasive', key: 'filament.abrasive', type: 'bool' },
    ],
  },
  {
    id: 'temperature',
    fields: [
      temp('temperature.first_layer', 'temperature.first_layer'),
      temp('temperature.nozzle', 'temperature.nozzle'),
      temp('temperature.bed_first_layer', 'temperature.bed_first_layer'),
      temp('temperature.bed', 'temperature.bed'),
      temp('temperature.probe', 'temperature.probe'),
      opt('temperature.idle', 'temperature.idle', '°C', 5, 0),
      temp('temperature.chamber', 'temperature.chamber'),
    ],
  },
  {
    id: 'speed',
    fields: [
      speed('speed.nominal', 'speed.nominal'),
      speed('speed.first_layer', 'speed.first_layer'),
      { path: 'speed.max_volumetric', key: 'speed.max_volumetric', type: 'num', unit: 'mm³/s', step: 1, decimals: 1, min: 0 },
      {
        path: 'speed.by_nozzle',
        key: 'speed.by_nozzle',
        type: 'table',
        columns: [
          { key: 'diameter', type: 'num', optional: true },
          { key: 'high_flow', type: 'tri' },
          { key: 'max_volumetric', type: 'num' },
        ],
      },
    ],
  },
  {
    id: 'cooling',
    fields: [
      { path: 'cooling.fan', key: 'cooling.fan', type: 'num', unit: '%', step: 5, decimals: 0, min: 0 },
      { path: 'cooling.first_layer_fan', key: 'cooling.first_layer_fan', type: 'num', unit: '%', step: 5, decimals: 0, min: 0 },
      { path: 'cooling.bridge_fan', key: 'cooling.bridge_fan', type: 'num', unit: '%', step: 5, decimals: 0, min: 0 },
    ],
  },
  {
    id: 'retraction',
    fields: [
      mm('retraction.length', 'retraction.length', 0.1, 2),
      speed('retraction.speed', 'retraction.speed'),
      mm('retraction.z_hop', 'retraction.z_hop', 0.05, 2),
      mm('retraction.min_travel', 'retraction.min_travel', 0.5, 1),
    ],
  },
];

export const GROUPS: Record<ProfileKind, Group[]> = { printer: PRINTER_GROUPS, filament: FILAMENT_GROUPS };

/** Valeur au chemin `a.b.0` (indices compris). */
export function getPath(obj: unknown, path: string): unknown {
  let cur: unknown = obj;
  for (const k of path.split('.')) {
    if (cur == null || typeof cur !== 'object') return undefined;
    cur = (cur as Record<string, unknown>)[k];
  }
  return cur;
}

/** Écrit la valeur au chemin (`undefined` retire le champ). */
export function setPath(obj: unknown, path: string, value: unknown) {
  const keys = path.split('.');
  let cur = obj as Record<string, unknown>;
  for (const k of keys.slice(0, -1)) {
    if (cur[k] == null || typeof cur[k] !== 'object') cur[k] = {};
    cur = cur[k] as Record<string, unknown>;
  }
  const last = keys[keys.length - 1];
  if (value === undefined) delete cur[last];
  else cur[last] = value;
}

/** Égalité de deux valeurs de profil (listes, tables comprises). */
export const same = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
