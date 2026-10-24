// Temps de calcul du moteur WebAssembly (le même que dans le navigateur),
// exécuté par Node (V8, comme Chrome). À lancer après `npm run wasm` :
//   node scripts/bench-wasm.mjs [répétitions]
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { initSync, Engine, engine_version } from '../src/wasm/fabweave.js';

const root = fileURLToPath(new URL('../../', import.meta.url));
const read = (p) => readFileSync(root + p, 'utf8');
initSync({ module: readFileSync(new URL('../src/wasm/fabweave_bg.wasm', import.meta.url)) });

const repeats = Number(process.argv[2] ?? 7);
const median = (v) => v.sort((a, b) => a - b)[Math.floor(v.length / 2)];
const cases = [
  ['vase à ondulations (défaut)', 'models/vases/ripple_vase.weave', {}],
  ['spirale 50 000 points', 'crates/fabweave-core/tests/fixtures/bench_spiral.weave', { points: 50000 }],
  ['spirale 200 000 points', 'crates/fabweave-core/tests/fixtures/bench_spiral.weave', { points: 200000 }],
  ['vase, 12 points par ondulation', 'models/vases/ripple_vase.weave', { ripple_segments: 12 }],
];

console.log(`moteur ${engine_version()}, Node ${process.version}, ${repeats} répétitions (médiane)\n`);
console.log('| Cas | Points | Script | Calcul complet (script + plan + tableaux) | G-code |');
console.log('| --- | ---: | ---: | ---: | ---: |');
for (const [name, file, values] of cases) {
  const e = new Engine();
  for (const [fn, p] of [['set_printer', 'profiles/printers/prusa_mk4s.toml'], ['set_filament', 'profiles/filaments/generic_pla.toml'], ['load_model', file]]) {
    const r = JSON.parse(e[fn](read(p)));
    if (!r.ok) throw new Error(`${p}: ${JSON.stringify(r.issues)}`);
  }
  const v = JSON.stringify(values);
  e.run_script(v); // échauffement
  const script = [], full = [], gcode = [];
  let points = 0;
  for (let i = 0; i < repeats; i++) {
    let t = performance.now();
    points = e.run_script(v);
    script.push(performance.now() - t);
    t = performance.now();
    const f = e.evaluate(v);
    f.take_positions(); f.take_kinds(); f.take_speeds(); JSON.parse(f.report()); f.free();
    full.push(performance.now() - t);
    t = performance.now();
    e.gcode(v);
    gcode.push(performance.now() - t);
  }
  console.log(`| ${name} | ${points} | ${median(script).toFixed(1)} ms | ${median(full).toFixed(1)} ms | ${median(gcode).toFixed(1)} ms |`);
  e.free();
}
