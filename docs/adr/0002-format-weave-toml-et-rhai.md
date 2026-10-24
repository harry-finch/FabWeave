# ADR-0002 : Format de modèle `.weave` (en-tête TOML + script Rhai)

Status: Accepted
Date: 2026-10-24

## Context

Le catalogue doit s'enrichir sans recompiler l'application, et un modèle doit pouvoir être **généré par une IA externe** à partir d'une spécification courte (demande de l'utilisateur). L'application doit construire automatiquement les curseurs d'un modèle. FullControl décrit ses modèles en Python (notebooks), lourd à exécuter dans un navigateur et sans description déclarative des paramètres. La conversation de recherche propose un fichier hybride, validé par l'utilisateur (« ça me paraît parfait »).

## Decision

- Un modèle est un fichier texte `.weave` : un **en-tête TOML** (`[meta]`, `[params.<id>]` dans l'ordre du fichier) puis une ligne `---` puis un **corps [Rhai](https://rhai.rs)** qui construit et retourne un parcours.
- Le script ne connaît que la géométrie et l'intention : positions (mm, repère centré sur (0, 0)), largeur W et hauteur H du boudin, facteur de vitesse, ventilateur, nature du mouvement (impression, pont, déplacement). Jamais de G-code, de `E`, de températures, de rétraction, de coordonnées du plateau.
- Variables injectées : `params`, `printer` (diamètre de buse, hauteurs de couche), `PI`. Fonctions mathématiques réenregistrées depuis la crate `libm` pour un résultat identique en natif et en WebAssembly (déterminisme, comparaison CLI ↔ interface).
- Les calculs lourds et répétitifs (primitives, copies, modulation en Z, discrétisation) sont des **fonctions natives Rust** exposées au script (v0.2) : la boucle tourne en Rust, le script décrit l'intention.
- Bac à sable : `Engine::set_max_operations`, profondeur d'appel, taille des tableaux et nombre de points plafonnés ; aucun module de fichier ni de réseau. Un dépassement est une erreur à code stable, pas un arrêt muet.
- L'en-tête est lu avec `toml` + `serde` (`indexmap` pour l'ordre des curseurs). Les valeurs hors bornes envoyées par l'interface sont refusées par le moteur avec `param.out_of_range` (aucun bridage silencieux).
- Le format porte `format = "fabweave.model"` et `version` ; spécification dans `docs/WEAVE_FORMAT.md` (v0.1) et prompt système pour IA dans `docs/PROMPT_MODELE_IA.md` (v0.2).

## Alternatives considered

- **Modèles compilés en Rust** (un trait par modèle) : rapides et typés, mais chaque ajout demande une recompilation, et une IA externe ne peut pas livrer un modèle utilisable directement. Gardé pour les primitives natives.
- **Lua (`mlua`)** : dépend d'une bibliothèque C, compilation WebAssembly plus délicate.
- **JavaScript évalué dans le navigateur** : rapide, mais le moteur Rust (CLI, tests, déterminisme) ne pourrait pas l'exécuter, et le bac à sable est plus difficile à garantir.
- **Python via Pyodide** (réutiliser FullControl tel quel) : ≈ 10 Mo à charger, démarrage lent, et on hérite de l'absence de description déclarative des paramètres.
- **JSON ou YAML pour l'en-tête** : virgules (JSON) et indentation (YAML) sont des sources d'erreur fréquentes pour une IA ; TOML est plus robuste et lisible.

## Consequences

Rhai est un interpréteur : nettement plus lent que du Rust natif. Un modèle qui calcule 200 000 points en pur script risque de dépasser le budget du calcul en direct ; d'où les fonctions natives (v0.2) et, si besoin, le calcul au relâchement du curseur signalé dans la barre d'état. Le format devient un contrat public : toute évolution incompatible demande une migration.

## Validation

Mesure à la v0.1 (étape 2) : temps d'exécution en WebAssembly d'un modèle en pur script de 50 000 et 200 000 points (objectif < 60 ms pour 50 000). Test de génération par IA à la v0.2 : trois modèles écrits à partir du seul prompt passent `fabweave-cli check` en deux essais au plus. Revoir le choix de Rhai si la mesure est hors d'atteinte même avec les fonctions natives.

### Mesures (24/10/2026, Apple M4, Rhai 1.27, médiane de 7 à 9 exécutions)

Script seul, puis écriture du G-code. Natif : `cargo run --release -p fabweave-core --example bench`. WebAssembly : `cd ui && npm run bench` (Node 26, moteur V8 de Chrome).

| Cas | Points | Script natif | Script WebAssembly | G-code natif / WebAssembly |
| --- | ---: | ---: | ---: | ---: |
| Vase à ondulations (défaut) | 16 766 | 31 ms | 54 ms | 8 / 11 ms (0,6 Mo) |
| Spirale de mesure, 50 000 points | 50 000 | 46 ms | 81 ms | 23 / 30 ms (1,8 Mo) |
| Spirale de mesure, 200 000 points | 200 000 | 183 ms | 324 ms | 87 / 114 ms (6,1 Mo) |
| Vase, 12 points par ondulation | 100 596 | 190 ms | 321 ms | 45 / 57 ms (3,2 Mo) |

Dans l'application (vue web Chromium) : vase par défaut recalculé en 61 à 67 ms, premier calcul 104 ms. Le plan (physique) coûte 0,1 à 2 ms ; l'essentiel est l'interprétation du script.

Constats :

- WebAssembly ≈ 1,75 × le natif ; ≈ 1,6 µs par point pour un calcul de trois fonctions trigonométriques et un appel à l'API.
- L'objectif de 60 ms pour 50 000 points en pur script **n'est pas atteint** (81 ms). Il l'est jusqu'à ≈ 30 000 points.
- Répartition pour 50 000 points en natif : boucle seule 6 ms, calculs 25 ms, appels `print_polar` 22 ms. Des surcharges typées (arguments flottants au lieu de `Dynamic`) n'apportent rien : le coût est celui de l'interpréteur. Copier les paramètres dans des variables locales avant la boucle : − 16 %.
- Le mode `unchecked` de Rhai retire les limites du bac à sable : écarté.

Suite proposée : fonctions natives (v0.2) pour les formes courantes (spirale, onde polaire, répétition), et calcul au relâchement du curseur, signalé dans la barre d'état, quand un calcul dépasse ≈ 60 ms (principe 5 de la famille Fab).
