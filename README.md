# FabWeave

Générateur de **G-code direct** pour l'impression 3D FDM, à partir de **modèles paramétriques**, pour makers et fablabs. On choisit un modèle dans le catalogue (vase à ondulations, tamis à ponts tendus, ressort non planaire, panier tressé…), on règle ses curseurs, on voit le parcours de la buse se mettre à jour en direct en 3D, et on exporte un G-code prêt pour **son imprimante et son filament** : centrage sur le plateau, extrusion calculée, débit et vitesse Z plafonnés selon les profils, G-code de démarrage et de fin de la machine.

Inspiré de [FullControl](https://fullcontrol.xyz) (Andrew Gleadall, Loughborough University), dans le même écosystème que [FabSpeed](../FabSpeed) et [FabVents](../FabVents) : même langage visuel, mêmes conventions, sans compte ni serveur.

Les modèles sont de simples fichiers texte `.weave` : un en-tête TOML (description, curseurs) et un script [Rhai](https://rhai.rs) qui trace le parcours. Un modèle s'écrit à la main ou se fait générer par une IA à partir d'une spécification courte, puis se glisse dans l'application.

> État : **v0.1 en cours** (moteur, premier G-code, interface de base). Voir [STATUS.md](STATUS.md) et [ROADMAP.md](ROADMAP.md).

## Lancer

Prérequis : Rust 1.88 (installé automatiquement par `rust-toolchain.toml`) avec la cible `wasm32-unknown-unknown`, [wasm-pack](https://rustwasm.github.io/wasm-pack/), Node 20+.

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cd ui && npm install && npm run wasm && npm run dev
```

Puis ouvrir http://localhost:5175. Après une modification du moteur Rust, relancer `npm run wasm`. Un modèle `.weave` ajouté ou modifié dans `models/` apparaît aussitôt.

Exporter un G-code sans interface :

```bash
cargo run --release -p fabweave-core --example export -- models/vases/ripple_vase.weave profiles/printers/prusa_mk4s.toml profiles/filaments/generic_pla.toml vase.gcode star_tips=6
```

## Développer

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cd ui && npm run check
```

Mesures de performance : `cargo run --release -p fabweave-core --example bench` (natif) et `cd ui && npm run bench` (WebAssembly). Écrire un modèle : [docs/WEAVE_FORMAT.md](docs/WEAVE_FORMAT.md).

## Organisation

| Dossier | Rôle |
| --- | --- |
| `crates/fabweave-core` | Moteur pur : lecture des `.weave`, exécution Rhai en bac à sable, parcours, physique de l'extrusion, contrôles, G-code |
| `crates/fabweave-wasm` | Le moteur pour l'interface (WebAssembly, appelé depuis un Web Worker) |
| `crates/fabweave-cli` | (v0.2) Vérifier, prévisualiser et exporter un `.weave` sans interface (utile pour valider un modèle généré par IA) |
| `models/` | Le catalogue de modèles `.weave`, par catégorie |
| `profiles/` | Profils imprimantes et filaments (TOML) |
| `ui/` | Interface Svelte 5 (skill `fab-design-system`) |
| `docs/` | [Fonctions](docs/FEATURES.md), [format des modèles et des profils](docs/WEAVE_FORMAT.md), [décisions](docs/adr) |

## Licence

GPL-3.0-or-later, comme FabSpeed, FabVents, FabCAM et FullControl. Voir [LICENSE](LICENSE).
