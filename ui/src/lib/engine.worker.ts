// Web Worker du moteur : charge le WebAssembly de `fabweave-wasm` et répond
// aux demandes du fil principal. Le calcul ne bloque jamais l'interface.

import init, { Engine, engine_version, read_model, read_profile } from '../wasm/fabweave.js';

type Request =
  | { kind: 'compute'; id: number; values: string }
  | { kind: 'load'; id: number; target: 'model' | 'printer' | 'filament'; text: string }
  | { kind: 'read'; id: number; text: string }
  | { kind: 'read-profile'; id: number; profile: 'printer' | 'filament'; text: string }
  | { kind: 'gcode'; id: number; values: string }
  | { kind: 'setup'; id: number; setup: string }
  | { kind: 'gcode-3mf'; id: number; values: string }
  | { kind: 'thumbnail'; id: number; model: string; printer: string; filament: string };

let engine: Engine | null = null;

/** `postMessage` du worker (les types DOM ne connaissent que celui de `window`). */
const post = (msg: unknown, transfer: Transferable[] = []) => (self as unknown as Worker).postMessage(msg, transfer);

const ready = init().then(() => {
  engine = new Engine();
  post({ kind: 'ready', version: engine_version() });
});
ready.catch((e) => post({ kind: 'failed', error: String(e) }));

/** Message d'un `JsError` (JSON des messages bloquants pour l'export). */
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

onmessage = async (ev: MessageEvent<Request>) => {
  await ready;
  const req = ev.data;
  const e = engine!;
  try {
    if (req.kind === 'compute') {
      const t0 = performance.now();
      const f = e.evaluate(req.values);
      const positions = f.take_positions();
      const kinds = f.take_kinds();
      const speeds = f.take_speeds();
      const outside = f.take_outside();
      const report = JSON.parse(f.report());
      f.free();
      const ms = performance.now() - t0;
      post({ kind: 'frame', id: req.id, frame: { positions, kinds, speeds, outside, report, ms } }, [
        positions.buffer as ArrayBuffer,
        kinds.buffer as ArrayBuffer,
        speeds.buffer as ArrayBuffer,
        outside.buffer as ArrayBuffer,
      ]);
    } else if (req.kind === 'load') {
      const fn = { model: e.load_model, printer: e.set_printer, filament: e.set_filament }[req.target];
      post({ kind: 'result', id: req.id, ok: true, value: fn.call(e, req.text) });
    } else if (req.kind === 'setup') {
      post({ kind: 'result', id: req.id, ok: true, value: e.set_setup(req.setup) });
    } else if (req.kind === 'read') {
      post({ kind: 'result', id: req.id, ok: true, value: read_model(req.text) });
    } else if (req.kind === 'read-profile') {
      post({ kind: 'result', id: req.id, ok: true, value: read_profile(req.profile, req.text) });
    } else if (req.kind === 'thumbnail') {
      const pts = e.thumbnail(req.model, req.printer, req.filament, 6000);
      post({ kind: 'result', id: req.id, ok: true, value: pts }, [pts.buffer as ArrayBuffer]);
    } else if (req.kind === 'gcode-3mf') {
      const bytes = e.gcode_3mf(req.values);
      post({ kind: 'result', id: req.id, ok: true, value: bytes }, [bytes.buffer as ArrayBuffer]);
    } else if (req.kind === 'gcode') {
      post({ kind: 'result', id: req.id, ok: true, value: e.gcode(req.values) });
    }
  } catch (err) {
    post({ kind: req.kind === 'compute' ? 'frame-error' : 'result', id: req.id, ok: false, error: message(err) });
  }
};
