# État du projet

**Jalon** : **v0.1 validée** le 24/10/2026 (vase imprimé sur la CORE One). **v0.2 validée** le 27/10/2026 (trois modèles du catalogue imprimés). **v0.3 en cours** (imprimantes et filaments) : étape 1 (Bambu Lab X1E) faite, ⏸ en attente de l'essai sur la machine par la carte SD. Voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur (`fabweave-core`) | Lecture stricte des `.weave` (erreurs positionnées), paramètres FR / EN, Rhai en bac à sable (`libm`, variables strictes, limites), plan physique (centrage, `E` en stade, plafonds comptés), G-code (gabarits en expressions Rhai, `M83`, machine à états : axes, `F`, ventilateur, températures, rétraction et saut en Z) |
| Profils | Prusa MK4S, Prusa CORE One (gabarits PrusaSlicer 2.5.13 réécrits), **Bambu Lab X1E** (gabarits Bambu Studio 02.08 réécrits, sans scan lidar), PLA et PETG génériques (noms FR / EN) |
| Catalogue | Onze modèles en six rubriques. Ports de FullControl : vase à ondulations (avec fond de 0 à 2 couches), abat-jour, entretoise non planaire, adaptateur hexagonal, treillis en étoiles (M1), motifs polaires. D'après FullControl : défis porte-à-faux et picots, porte-savon serpentin. FabWeave : tamis micro-maillage, ressort-soufflet. Chacun calculé sur MK4S et CORE One, PLA et PETG, et aux bornes de chaque curseur |
| Export | G-code ; pour une Bambu Lab, menu : paquet `.gcode.3mf` (MD5, métadonnées, vignettes rendues par le moteur) ou G-code seul, tous deux avec l'en-tête et les marqueurs de couche du firmware (ADR-0005, proposée) |
| Outils | `fabweave check | stats | export` (calcul aux valeurs par défaut et aux bornes, codes de sortie, `--json`) ; [prompt pour IA](docs/PROMPT_MODELE_IA.md) : 3 modèles générés sur 3 valides au premier essai |
| API v0.2 | Remplissage d'un contour (`fill_lines`, contour quelconque) ; `slice` ; arrondis acceptant un entier ; paramètres `choice` ; `warn` / `fail` (texte par langue, ligne) ; `path.note`, `note_at` ; `bridge_to` ; listes de points natives (`line`, `arc`, `circle`, `polygon`, `rectangle`, `spiral`, `helix`, `zigzag`, `sine`), transformations (`translate`, `rotate`, `scale`, `mirror_*`, `copies`, `repeat_z`, `polar_copies`, `subdivide`, `reverse`, `+`), modulations (`modulate_z`, `modulate_r`) ; `helix()` 50 000 points en 1,2 ms dans le navigateur |
| Interface | Kit `fab-design-system` ; catalogue en sélecteur riche (vignettes rendues par l'application, catégories, recherche, « Mes modèles »), fiche du modèle (étiquettes, buses conseillées), ouverture et dépôt de `.weave` (`⌘O`), annotations dans la vue 3D ; paramètres générés (↺ par valeur modifiée, bornes du modèle signalées et non bridées), vue 3D three.js, machine, mesures (mm / pouces), messages, export ; annuler / rétablir (`⌘Z` / `⇧⌘Z`), calcul au relâchement au-delà de 60 ms ; FR / EN, thèmes |
| Identité | Deux cercles ondulés entrelacés, bleu FabCAM `#58B5F5` / `#438ABA` ([branding/README.md](branding/README.md)) ; favicons 16 à 180 px |
| Tests | `cargo test --workspace` : 55 verts (40 unitaires dont 5 de remplissage et 4 du paquet Bambu, 10 API, 3 catalogue sur MK4S, CORE One et X1E, 2 CLI) ; clippy : 0 avertissement ; `npm run check` : 0 erreur |
| Mesures | Moteur ([ADR-0002](docs/adr/0002-format-weave-toml-et-rhai.md)) : vase 54 ms, 50 000 points 81 ms, 200 000 points 324 ms (WebAssembly). Vue 3D ([ADR-0003](docs/adr/0003-vue-3d-webgl-three-js.md)) : 200 195 segments rendus en 1,9 ms |
| Vérifié dans le navigateur | Onze modèles affichés (27/10), abat-jour 232 000 points calculé en 170 ms ; fond du vase vu de dessous ; `.weave` ajouté dans un nouveau dossier de `models/` affiché sans redémarrage. Catalogue : vignettes au démarrage et au changement de thème, recherche à plusieurs mots, dépôt d'un modèle valide et d'un modèle en erreur (ligne du script, export bloqué, vignette marquée), retrait puis « Annuler », annotations en FR et EN. v0.1 : flèches fusionnées en une étape d'annulation, ↺, valeur hors bornes refusée (souligné, message, export désactivé), « Calcul au relâchement » pendant un défilement lent puis calcul au relâchement, un geste = une étape, mesures en pouces, libellés mesurés dans les deux langues |
| Validé par l'utilisateur | G-code relu dans PrusaSlicer ; vase imprimé sur la CORE One (PLA) ; v0.2 : trois modèles du catalogue imprimés |
| Non vérifié | Impression des modèles de la v0.2 non retenus pour la validation ; impression sur la MK4S ; temps estimé comparé à PrusaSlicer (critère ± 15 %) ; défilement réel à la souris (testé par événements simulés) |

## Points ouverts

- 50 000 points en pur script : 81 ms ; fonctions natives faites (abat-jour : 232 000 points en 170 ms, cellule en script puis copies natives).
- Remplissage du fond : plusieurs chaînes (donc rétractions) dans les creux d'un contour en étoile ; anneaux concentriques en v0.3.
- Taille du WebAssembly (2,8 Mo non compressé, 2,6 Mo à la v0.1).

## Prochaine action

⏸ Essai sur la X1E par la carte SD des deux fichiers de `.tmp/essais-x1e/` (adaptateur hexagonal, PLA) : `fabweave_essai_hexagone.gcode.3mf` et `fabweave_essai_hexagone.gcode`. Puis ADR-0005 acceptée (ou paquet complété), étape 2 (profils complets et dialectes).
