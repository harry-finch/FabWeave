# État du projet

**Jalon** : **v0.1 — tranche verticale**. Étapes 1 et 2 validées (24/10/2026), étapes 3 (physique et G-code) et 4 (interface) faites. Le vase à ondulations a été **imprimé avec succès sur la CORE One**. **Point d'arrêt ⏸ étape 5** : choix du logo sur la planche [branding/explorations/planche.html](branding/explorations/planche.html). Voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur (`fabweave-core`) | Lecture stricte des `.weave` (erreurs positionnées), paramètres FR / EN, Rhai en bac à sable (`libm`, variables strictes, limites), plan physique (centrage, `E` en stade, plafonds comptés), G-code (gabarits en expressions Rhai, `M83`, machine à états : axes, `F`, ventilateur, températures, rétraction et saut en Z) |
| Profils | Prusa MK4S, Prusa CORE One (gabarits PrusaSlicer 2.5.13 réécrits), PLA et PETG génériques (noms FR / EN) |
| Catalogue | Vase à ondulations (port FullControl) |
| Interface | Kit `fab-design-system` ; catalogue, paramètres générés (↺ par valeur modifiée, bornes du modèle signalées et non bridées), vue 3D three.js, machine, mesures (mm / pouces), messages, export ; annuler / rétablir (`⌘Z` / `⇧⌘Z`), calcul au relâchement au-delà de 60 ms ; FR / EN, thèmes ; identité provisoire |
| Tests | `cargo test --workspace` : 33 verts ; clippy : 0 avertissement ; `npm run check` : 0 erreur, 0 avertissement |
| Mesures | Moteur ([ADR-0002](docs/adr/0002-format-weave-toml-et-rhai.md)) : vase 54 ms, 50 000 points 81 ms, 200 000 points 324 ms (WebAssembly). Vue 3D ([ADR-0003](docs/adr/0003-vue-3d-webgl-three-js.md)) : 200 195 segments rendus en 1,9 ms |
| Vérifié dans le navigateur | Flèches fusionnées en une étape d'annulation, ↺, valeur hors bornes refusée (souligné, message, export désactivé), « Calcul au relâchement » pendant un défilement lent puis calcul au relâchement, un geste = une étape, mesures en pouces, libellés mesurés dans les deux langues |
| Validé par l'utilisateur | G-code relu dans PrusaSlicer ; vase imprimé sur la CORE One (PLA) |
| Non vérifié | Impression sur la MK4S ; défilement réel à la souris (testé par événements simulés) |

## Points ouverts

- Choix du logo et de la couleur d'identité.
- 50 000 points en pur script : 81 ms (fonctions natives en v0.2).
- Taille du WebAssembly (2,6 Mo non compressé).

## Prochaine action

Après le choix du logo : intégrer le logo et la couleur, puis validation du jalon v0.1 (critères de la feuille de route), et v0.2 (catalogue et format complet).
