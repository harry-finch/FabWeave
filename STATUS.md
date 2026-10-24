# État du projet

**Jalon** : **v0.1 validée** le 24/10/2026 (vase imprimé sur la CORE One). **v0.2 en cours** (catalogue et format complet), étapes 1 (format et API, validée) et 2 (CLI, prompt pour IA, essai de génération) faites. Prochaine : étape 3, interface du catalogue. Voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur (`fabweave-core`) | Lecture stricte des `.weave` (erreurs positionnées), paramètres FR / EN, Rhai en bac à sable (`libm`, variables strictes, limites), plan physique (centrage, `E` en stade, plafonds comptés), G-code (gabarits en expressions Rhai, `M83`, machine à états : axes, `F`, ventilateur, températures, rétraction et saut en Z) |
| Profils | Prusa MK4S, Prusa CORE One (gabarits PrusaSlicer 2.5.13 réécrits), PLA et PETG génériques (noms FR / EN) |
| Catalogue | Vase à ondulations (port FullControl) |
| Outils | `fabweave check | stats | export` (calcul aux valeurs par défaut et aux bornes, codes de sortie, `--json`) ; [prompt pour IA](docs/PROMPT_MODELE_IA.md) : 3 modèles générés sur 3 valides au premier essai |
| API v0.2 | Paramètres `choice` ; `warn` / `fail` (texte par langue, ligne) ; `path.note`, `note_at` ; `bridge_to` ; listes de points natives (`line`, `arc`, `circle`, `polygon`, `rectangle`, `spiral`, `helix`, `zigzag`, `sine`), transformations (`translate`, `rotate`, `scale`, `mirror_*`, `copies`, `repeat_z`, `polar_copies`, `subdivide`, `reverse`, `+`), modulations (`modulate_z`, `modulate_r`) ; `helix()` 50 000 points en 1,2 ms dans le navigateur |
| Interface | Kit `fab-design-system` ; catalogue, paramètres générés (↺ par valeur modifiée, bornes du modèle signalées et non bridées), vue 3D three.js, machine, mesures (mm / pouces), messages, export ; annuler / rétablir (`⌘Z` / `⇧⌘Z`), calcul au relâchement au-delà de 60 ms ; FR / EN, thèmes |
| Identité | Deux cercles ondulés entrelacés, bleu FabCAM `#58B5F5` / `#438ABA` ([branding/README.md](branding/README.md)) ; favicons 16 à 180 px |
| Tests | `cargo test --workspace` : 45 verts (31 unitaires, 9 API dont les exemples de la spécification et du prompt et les modèles écrits par IA, 3 catalogue, 2 CLI) ; clippy : 0 avertissement ; `npm run check` : 0 erreur |
| Mesures | Moteur ([ADR-0002](docs/adr/0002-format-weave-toml-et-rhai.md)) : vase 54 ms, 50 000 points 81 ms, 200 000 points 324 ms (WebAssembly). Vue 3D ([ADR-0003](docs/adr/0003-vue-3d-webgl-three-js.md)) : 200 195 segments rendus en 1,9 ms |
| Vérifié dans le navigateur | Flèches fusionnées en une étape d'annulation, ↺, valeur hors bornes refusée (souligné, message, export désactivé), « Calcul au relâchement » pendant un défilement lent puis calcul au relâchement, un geste = une étape, mesures en pouces, libellés mesurés dans les deux langues |
| Validé par l'utilisateur | G-code relu dans PrusaSlicer ; vase imprimé sur la CORE One (PLA) |
| Non vérifié | Impression sur la MK4S ; temps estimé comparé à PrusaSlicer (critère ± 15 %) ; défilement réel à la souris (testé par événements simulés) |

## Points ouverts

- 50 000 points en pur script : 81 ms (fonctions natives en v0.2).
- Taille du WebAssembly (2,6 Mo non compressé).

## Prochaine action

Validation du jalon v0.1 par l'utilisateur, puis v0.2 (catalogue et format complet : fonctions natives, paramètres `choice`, annotations, ouverture de `.weave`, CLI de vérification, prompt pour IA, une dizaine de modèles).
