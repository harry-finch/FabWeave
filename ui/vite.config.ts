import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Le moteur (WebAssembly, `npm run wasm`) tourne dans un Web Worker module :
// aucune API à lancer, l'application est un site statique. Les modèles
// (`../models`) et les profils (`../profiles`) sont importés tels quels : en
// développement, un fichier modifié recharge l'application.
export default defineConfig({
  plugins: [svelte()],
  server: { port: 5175, strictPort: true, fs: { allow: ['..'] } },
  preview: { port: 4175 },
  worker: { format: 'es' },
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
});
