# FabWeave

Générateur de **G-code direct** pour l'impression 3D FDM, à partir de **modèles paramétriques**, pour makers et fablabs. On choisit un modèle dans le catalogue (vase à ondulations, tamis à ponts tendus, ressort non planaire, panier tressé…), on règle ses curseurs, on voit le parcours de la buse se mettre à jour en direct en 3D, et on exporte un G-code prêt pour **son imprimante et son filament** : centrage sur le plateau, extrusion calculée, débit et vitesse Z plafonnés selon les profils, G-code de démarrage et de fin de la machine.

Inspiré de [FullControl](https://fullcontrol.xyz) (Andrew Gleadall, Loughborough University), dans le même écosystème que [FabSpeed](../FabSpeed) et [FabVents](../FabVents) : même langage visuel, mêmes conventions, sans compte ni serveur.

Les modèles sont de simples fichiers texte `.weave` : un en-tête TOML (description, curseurs) et un script [Rhai](https://rhai.rs) qui trace le parcours. Un modèle s'écrit à la main ou se fait générer par une IA à partir d'une spécification courte, puis se glisse dans l'application.

> État : **v0.0 — fondations** (dépôt, feuille de route). Voir [STATUS.md](STATUS.md) et [ROADMAP.md](ROADMAP.md).

## Organisation prévue

| Dossier | Rôle |
| --- | --- |
| `crates/fabweave-core` | Moteur pur : lecture des `.weave`, exécution Rhai en bac à sable, parcours, physique de l'extrusion, contrôles, G-code |
| `crates/fabweave-wasm` | Le moteur pour l'interface (WebAssembly, appelé depuis un Web Worker) |
| `crates/fabweave-cli` | Vérifier, prévisualiser et exporter un `.weave` sans interface (utile pour valider un modèle généré par IA) |
| `models/` | Le catalogue de modèles `.weave`, par catégorie |
| `profiles/` | Profils imprimantes et filaments (TOML) |
| `ui/` | Interface Svelte 5 (skill `fab-design-system`) |
| `docs/` | [Fonctions](docs/FEATURES.md), [décisions](docs/adr), spécification du format (à venir) |

## Licence

GPL-3.0-or-later, comme FabSpeed, FabVents, FabCAM et FullControl. Voir [LICENSE](LICENSE).
