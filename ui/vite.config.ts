import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// Le moteur (WebAssembly, `npm run wasm`) tourne dans un Web Worker module :
// aucune API à lancer, l'application est un site statique. Les modèles
// (`../models`) et les profils (`../profiles`) sont importés tels quels : en
// développement, un fichier ajouté ou modifié recharge l'application.

/** Vite ne surveille que `ui/` : sans cela, un `.weave` ajouté dans un
 * nouveau dossier de `models/` n'apparaît qu'au redémarrage. */
const watchCatalogue = (): Plugin => ({
  name: 'fabweave-watch-catalogue',
  configureServer(server) {
    for (const dir of ['../models', '../profiles']) server.watcher.add(fileURLToPath(new URL(dir, import.meta.url)));
  },
});

export default defineConfig({
  plugins: [svelte(), watchCatalogue()],
  server: { port: 5175, strictPort: true, fs: { allow: ['..'] } },
  preview: { port: 4175 },
  worker: { format: 'es' },
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
});
