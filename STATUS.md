# État du projet

**Jalon** : **v0.1 — tranche verticale**, toutes les étapes faites ; **en validation de jalon**. Étapes 1 et 2 et logo validés (24/10/2026), vase à ondulations **imprimé avec succès sur la CORE One**. Voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur (`fabweave-core`) | Lecture stricte des `.weave` (erreurs positionnées), paramètres FR / EN, Rhai en bac à sable (`libm`, variables strictes, limites), plan physique (centrage, `E` en stade, plafonds comptés), G-code (gabarits en expressions Rhai, `M83`, machine à états : axes, `F`, ventilateur, températures, rétraction et saut en Z) |
| Profils | Prusa MK4S, Prusa CORE One (gabarits PrusaSlicer 2.5.13 réécrits), PLA et PETG génériques (noms FR / EN) |
| Catalogue | Vase à ondulations (port FullControl) |
| Interface | Kit `fab-design-system` ; catalogue, paramètres générés (↺ par valeur modifiée, bornes du modèle signalées et non bridées), vue 3D three.js, machine, mesures (mm / pouces), messages, export ; annuler / rétablir (`⌘Z` / `⇧⌘Z`), calcul au relâchement au-delà de 60 ms ; FR / EN, thèmes |
| Identité | Deux cercles ondulés entrelacés, bleu FabCAM `#58B5F5` / `#438ABA` ([branding/README.md](branding/README.md)) ; favicons 16 à 180 px |
| Tests | `cargo test --workspace` : 33 verts ; clippy : 0 avertissement ; `npm run check` : 0 erreur, 0 avertissement |
| Mesures | Moteur ([ADR-0002](docs/adr/0002-format-weave-toml-et-rhai.md)) : vase 54 ms, 50 000 points 81 ms, 200 000 points 324 ms (WebAssembly). Vue 3D ([ADR-0003](docs/adr/0003-vue-3d-webgl-three-js.md)) : 200 195 segments rendus en 1,9 ms |
| Vérifié dans le navigateur | Flèches fusionnées en une étape d'annulation, ↺, valeur hors bornes refusée (souligné, message, export désactivé), « Calcul au relâchement » pendant un défilement lent puis calcul au relâchement, un geste = une étape, mesures en pouces, libellés mesurés dans les deux langues |
| Validé par l'utilisateur | G-code relu dans PrusaSlicer ; vase imprimé sur la CORE One (PLA) |
| Non vérifié | Impression sur la MK4S ; temps estimé comparé à PrusaSlicer (critère ± 15 %) ; défilement réel à la souris (testé par événements simulés) |

## Points ouverts

- 50 000 points en pur script : 81 ms (fonctions natives en v0.2).
- Taille du WebAssembly (2,6 Mo non compressé).

## Prochaine action

Validation du jalon v0.1 par l'utilisateur, puis v0.2 (catalogue et format complet : fonctions natives, paramètres `choice`, annotations, ouverture de `.weave`, CLI de vérification, prompt pour IA, une dizaine de modèles).
