# État du projet

**Jalon** : **v0.1 — tranche verticale**, étapes 1 (squelette) et 2 (format, moteur de script, mesures) **validées** le 24/10/2026, avec une interface de base. Le vase à ondulations exporté a été **imprimé avec succès sur la CORE One** du fablab. Voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur (`fabweave-core`) | Lecture stricte des `.weave` (erreurs positionnées), paramètres `float` / `int` / `bool` avec libellés FR / EN, Rhai en bac à sable (`libm`, variables strictes, limites), plan physique (centrage, `E` en stade, plafonds première couche / machine / débit / Z comptés), G-code (gabarits en expressions Rhai, `M83`, axes écrits au changement, températures après la première couche, métadonnées Prusa) |
| Profils | Prusa MK4S, Prusa CORE One (gabarits PrusaSlicer 2.5.13 réécrits), PLA et PETG génériques |
| Catalogue | Vase à ondulations (port FullControl) |
| WebAssembly | `fabweave-wasm` dans un Web Worker, « le dernier gagne » ; 2,6 Mo (non compressé) |
| Interface | Kit `fab-design-system` ; catalogue, curseurs générés, vue 3D three.js en lignes (plateau, cadrage `F`), machine, mesures, messages, export ; FR / EN, thèmes, mm / pouces ; identité provisoire (violet) |
| Tests | `cargo test --workspace` : 30 verts (27 unitaires, 3 sur le catalogue) ; clippy : 0 avertissement ; `npm run check` : 0 erreur, 0 avertissement |
| Mesures | [ADR-0002](docs/adr/0002-format-weave-toml-et-rhai.md) : vase 54 ms (WebAssembly), 50 000 points 81 ms, 200 000 points 324 ms |
| Vérifié | Dans le navigateur : vase calculé en 61 à 67 ms, curseur modifié (6 branches), export intercepté (G-code de 621 Ko au bon nom), anglais, thème clair, pouces, 1 100 px. G-code relu par un analyseur indépendant : largeur déduite 1,00 mm (= 2,5 × 0,4), E et durée identiques au moteur |
| Validé par l'utilisateur | G-code relu dans le visualiseur de PrusaSlicer ; vase imprimé sur la CORE One (PLA) |
| Non vérifié | Impression sur la MK4S |

## Points ouverts

- 50 000 points en pur script : 81 ms, au-dessus de l'objectif de 60 ms (fonctions natives en v0.2, calcul au relâchement).
- Mesures du panneau de droite toujours en mm (le mode pouces ne touche que les curseurs).
- Noms des profils non traduits (« PLA générique » en anglais).
- Taille du WebAssembly (2,6 Mo) : réglages de taille à essayer.
- ADR-0003 (three.js) proposée, à confirmer à l'étape 4.

## Prochaine action

Après validation : v0.1 étape 3 (physique et G-code complets : relecture dans PrusaSlicer, ajustements), étape 4 (interface : annuler / rétablir, retour d'un paramètre à sa valeur, calcul au relâchement), étape 5 (logo).
