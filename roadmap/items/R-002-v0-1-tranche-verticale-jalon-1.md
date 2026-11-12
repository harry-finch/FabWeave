---
id: R-002
titre: "v0.1 — Tranche verticale (jalon 1)"
resume: "Choisir un modèle, bouger ses curseurs, voir le parcours en 3D et imprimer le G-code exporté sur une machine du fablab, de bout en bout."
statut: livree
jalon: v0.1
cree: 2026-10-24
maj: 2026-11-12
livree: 2026-10-24
---
*Validée le 24/10/2026 ; vase imprimé sur la CORE One.*

**But** : choisir un modèle, bouger ses curseurs, voir le parcours en 3D et imprimer le G-code exporté sur une machine du fablab, de bout en bout.

Étapes et points d'arrêt :

1. **Squelette** : workspace Rust (`fabweave-core`, `fabweave-wasm`), interface depuis le kit `fab-design-system`, moteur dans un Web Worker (ADR-0001).
2. **Format et moteur de script** : lecture du `.weave` (en-tête TOML, séparateur `---`, corps Rhai), paramètres `float` / `int` / `bool`, API minimale du parcours, bac à sable (limites d'opérations et de points), erreurs avec ligne et colonne. **Mesure de performance** de Rhai en WebAssembly sur un parcours de 50 000 et 200 000 points (ADR-0002).
   ⏸ Validation : spécification du format (`docs/WEAVE_FORMAT.md`, premier jet), chiffres de performance, premier G-code brut relu dans le visualiseur de PrusaSlicer. *Validé le 24/10/2026.*
3. **Physique et G-code** : section de boudin (stade), `E` relatif, centrage sur le plateau, vitesse nominale × facteur, plafonds de débit volumétrique et de vitesse Z (comptés), première couche ralentie, ventilateur, rétraction et saut en Z sur les déplacements, gabarits de démarrage et de fin avec variables, machine à états (pas de commande redondante), commentaires de métadonnées (temps, filament en mm / g). Profils **Prusa MK4S** et **Prusa Core One** (gabarits de démarrage et de fin d'après les profils officiels, source citée), deux profils filament (PLA, PETG).
4. **Interface** (squelette déjà en place : kit, worker, curseurs, vue 3D en lignes, mesures, messages, export) : liste de modèles simple ; à gauche le modèle et ses curseurs générés depuis l'en-tête ; au centre la vue 3D en lignes colorées (orbite, déplacement, zoom, cadrer `F`, plateau dessiné) ; à droite l'imprimante, le filament, les mesures (points, longueur, temps estimé, filament, débit max atteint) et les messages ; **Exporter le G-code** (action principale). Reprise du dernier modèle et des valeurs ; FR / EN ; thème clair.
5. **Identité** : couleur et symbole du logo, planche de variantes.
   ⏸ Choix du logo par l'utilisateur. *Retenu le 24/10/2026 : deux cercles ondulés entrelacés, bleu FabCAM (`branding/README.md`).*
6. **Modèle étalon** : le **vase à ondulations** (port de « Ripple Texture Demo » de FullControl, CAT-01) : spirale continue, Z continu, largeur variable.

Critères d'acceptation :

- `cargo test --workspace`, `cargo clippy --workspace --all-targets` et `npm run check` sans erreur.
- Le G-code exporté s'ouvre dans le visualiseur G-code de PrusaSlicer : dimensions et position justes, extrusion cohérente (largeur affichée ≈ largeur demandée), temps estimé à ± 15 % de celui de PrusaSlicer.
- Un modèle de 50 000 points se recalcule pendant le déplacement d'un curseur sans saccade visible (ou passe en calcul au relâchement, signalé dans la barre d'état).
- ⏸ **Validation de jalon** : le vase imprimé sur la MK4S ou la Core One du fablab.
