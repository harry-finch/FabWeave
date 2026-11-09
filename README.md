# FabWeave

Générateur de **G-code direct** pour l'impression 3D FDM, à partir de **modèles paramétriques**, pour makers et fablabs. On choisit un modèle dans le catalogue (vase à ondulations, abat-jour, tamis à ponts tendus, ressort-soufflet, entretoise non planaire, défis de porte-à-faux…), on règle ses curseurs, on voit le parcours de la buse se mettre à jour en direct en 3D, et on exporte un G-code prêt pour **son imprimante et son filament** : centrage sur le plateau, extrusion calculée, débit et vitesse Z plafonnés selon les profils, G-code de démarrage et de fin de la machine.

Inspiré de [FullControl](https://fullcontrol.xyz) (Andrew Gleadall, Loughborough University), dans le même écosystème que [FabSpeed](../FabSpeed) et [FabVents](../FabVents) : même langage visuel, mêmes conventions, sans compte ni serveur.

Les modèles sont de simples fichiers texte `.weave` : un en-tête TOML (description, curseurs) et un script [Rhai](https://rhai.rs) qui trace le parcours. Un modèle s'écrit à la main ou se fait générer par une IA à partir d'une spécification courte, puis se glisse dans l'application.

> État : **0.9.0-beta.1**, bêta avec une application macOS. Vingt-quatre modèles, sept imprimantes (Prusa MK4S et CORE One, Bambu Lab X1E, A1 et P1S, Marlin et Klipper génériques), cinq filaments, import des profils de PrusaSlicer, Bambu Studio et OrcaSlicer, vue 3D en volume, garde-buse. Validé par des impressions : la MK4S ; à valider pendant la bêta : la CORE One, la X1E et plusieurs modèles. Détail dans [STATUS.md](STATUS.md) et [ROADMAP.md](ROADMAP.md).

## Lancer

Prérequis : Rust 1.88 (installé automatiquement par `rust-toolchain.toml`) avec la cible `wasm32-unknown-unknown`, [wasm-pack](https://rustwasm.github.io/wasm-pack/), Node `^20.19.0` ou `>=22.12.0` (exigé par Vite).

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cd ui && npm install && npm run wasm && npm run dev
```

Puis ouvrir http://localhost:5175. Après une modification du moteur Rust, relancer `npm run wasm`. Un modèle `.weave` ajouté ou modifié dans `models/` apparaît aussitôt.

Vérifier, mesurer ou exporter un modèle sans interface (outil `fabweave`) :

```bash
cargo run --release -p fabweave-cli -- check models/vases/ripple_vase.weave
```

```bash
cargo run --release -p fabweave-cli -- export models/vases/ripple_vase.weave -o vase.gcode --printer profiles/printers/prusa_core_one.toml --set star_tips=6
```

Pour une Bambu Lab (carte SD), une sortie en `.gcode.3mf` donne le paquet que lit l'écran de l'imprimante :

```bash
cargo run --release -p fabweave-cli -- export models/vases/ripple_vase.weave -o vase.gcode.3mf --printer profiles/printers/bambu_x1e.toml
```

La préparation de l'impression se choisit aussi en ligne de commande (buse, placement, copies, adhésion) :

```bash
cargo run --release -p fabweave-cli -- export models/mecanique/hex_adapter.weave -o hex.gcode --nozzle 0.6 --high-flow --brim 5 --copies 2x1 --rotate 30
```

Application macOS : le DMG de la bêta est dans les [versions publiées](https://github.com/harry-finch/FabWeave/releases) (Apple Silicon seulement). Elle n'est pas notarisée : au premier lancement, macOS refuse de l'ouvrir ; aller dans Réglages Système › Confidentialité et sécurité, puis « Ouvrir quand même ». Si macOS la dit « endommagée », lancer `xattr -dr com.apple.quarantine /Applications/FabWeave.app` dans le Terminal.

Application Windows : l'installeur (`.exe`, installation sans droits administrateur) se construit sur GitHub, onglet Actions › « Application Windows » (`.github/workflows/windows.yml`), et s'ajoute à la version publiée. Il n'est pas signé : SmartScreen affiche « Windows a protégé votre ordinateur » ; choisir « Informations complémentaires » puis « Exécuter quand même ». Non encore essayé sur un poste Windows.

Pour construire celle de macOS :

```bash
cd ui && npm run desktop
```

L'application et le DMG sont dans `crates/fabweave-desktop/target/release/bundle/`. Les exports vont dans Téléchargements ; préférences et « Mes modèles » restent propres à l'application, séparés de ceux du navigateur.

Faire écrire un modèle par une IA : donner [docs/PROMPT_MODELE_IA.md](docs/PROMPT_MODELE_IA.md) comme instructions, puis vérifier le fichier obtenu avec `fabweave check`.

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
| `crates/fabweave-cli` | Outil `fabweave` : vérifier (`check`), mesurer (`stats`) et exporter (`export`) un `.weave` sans interface |
| `crates/fabweave-desktop` | Application de bureau (Tauri) : l'interface dans une fenêtre ; hors du workspace Rust |
| `models/` | Le catalogue de modèles `.weave`, par catégorie |
| `profiles/` | Profils imprimantes et filaments (TOML) |
| `ui/` | Interface Svelte 5 (skill `fab-design-system`) |
| `docs/` | [Fonctions](docs/FEATURES.md), [format des modèles et des profils](docs/WEAVE_FORMAT.md), [décisions](docs/adr) |

## Licence

GPL-3.0-or-later, comme FabSpeed, FabVents, FabCAM et FullControl. Voir [LICENSE](LICENSE).
