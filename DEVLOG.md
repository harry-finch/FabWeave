# Journal de développement

## 2026-10-24 — v0.0 : fondations

**Entrées.** Conversation de recherche avec Gemini du 23/10/2026 (`docs/research/2026-10-23-conversation-gemini-fabweave.md`, hors dépôt) : format `.weave` (TOML + Rhai), structure `WayPoint`, physique de l'extrusion, profils imprimante et filament, post-processeur G-code, modèles de départ. Étude de fullcontrol.xyz dans le navigateur (bibliothèque de 16 modèles, fiche « Nonplanar Spacer » : paramètres, avancés, annotations 3D, réglages imprimante, téléchargement). FabVents comme référence d'architecture et de documentation ; skill `fab-design-system` pour l'interface.

**Choix.**
- Interface Svelte 5 de la famille Fab au lieu de l'`egui` proposé par la recherche ; moteur Rust en WebAssembly dans un Web Worker, comme FabVents (ADR-0001).
- Format `.weave` repris de la recherche, avec en plus : fonctions mathématiques `libm` (même résultat en natif et en WebAssembly), bac à sable Rhai, fonctions natives pour les boucles lourdes, versionnement (ADR-0002).
- Vue 3D WebGL avec three.js, proposée (ADR-0003).
- Licence GPL-3.0-or-later, compatible avec FullControl (GPL-3.0) dont des modèles seront portés en citant la source.
- Le catalogue (v1) passe avant l'atelier de formes (v2), à la demande de l'utilisateur.
- Réponses de l'utilisateur : Prusa MK4S / Core One en v0.1 et Bambu Lab A1 / P1 / X1 en v0.3 ; vase à ondulations seul pour la v0.1 ; web d'abord, Tauri en v0.6 ; dépôt GitHub public.
- Bambu Lab : le G-code brut depuis la carte SD ignore l'AMS et les contrôles du firmware d'après les forums ; le format `.gcode.3mf` porte des sommes MD5 et des métadonnées de plaque (outil `bambox`). Étude au début de la v0.3, l'envoi réseau reste hors périmètre.

**Non fait.** Aucun code : la feuille de route attend sa validation.

## 2026-10-24 — v0.1, étapes 1 et 2 : squelette, format, moteur, premier G-code

v0.0 validée par l'utilisateur.

**Entrées.** Notebook `ripple_texture.ipynb` de FullControl (port du vase) ; profils « Original Prusa MK4S 0.4 nozzle », « Prusa CORE One 0.4 nozzle », « Generic PLA / PETG @PG, @PGIS, @COREONE » de PrusaSlicer (PrusaResearch 2.5.13).

**Choix.**
- Workspace `fabweave-core` + `fabweave-wasm`, Rhai 1.27 sans modules ni horloge, `toml` 0.9 (`preserve_order`), `libm`.
- En-tête lu par serde avec `deny_unknown_fields` et `Spanned` : toute faute est positionnée (ligne, colonne). Textes par langue dès la v0.1 (`{ fr, en }`), `advanced`, `help`.
- Script compilé une fois par modèle (`compile_with_scope`, variables strictes) ; `params` et `printer` déclarés non constants à la compilation pour que l'optimiseur ne fige pas leurs valeurs.
- API à arguments `Dynamic` : entiers et flottants acceptés partout (erreur fréquente des IA). Erreurs de l'API levées avec un code stable, retrouvé derrière les enveloppes de Rhai.
- Vitesse : `set_speed` (mm/s) en plus de `set_speed_factor` (le vase de FullControl impose 8 mm/s).
- Plan : premier point rejoint par un déplacement ; première couche = Z < Z minimal + hauteur de première couche (vase en spirale compris) ; plafonds appliqués dans l'ordre première couche, machine, débit, axe Z, le dernier qui réduit est retenu et compté.
- Gabarits des profils en expressions Rhai (ADR-0004) : les macros de PrusaSlicer (`M555` par zone, températures selon la matière, parcage) se réécrivent sans précalcul propre à une machine.
- Interface : kit copié tel quel, en-tête de barre simplifié (pas de projet avant la v0.6), catalogue et profils importés par `import.meta.glob` (rechargement à chaud de `models/` en développement), vue three.js en lignes (ADR-0003, proposée).

**Vérification.**
- 30 tests ; catalogue calculé sur MK4S et CORE One, en PLA et PETG, déterministe, aux bornes de chaque curseur.
- G-code du vase : 16 766 points, 55 min 46 s, 7,5 g ; relu par un analyseur indépendant (largeur déduite 1,00 mm, `E` et durée identiques) ; `M555 X108.32 Y84.361 W33.339 H37.351` conforme à la formule de PrusaSlicer.
- Mesures natives et WebAssembly : voir ADR-0002.
- Navigateur : calcul en direct, export, langues, thèmes, pouces, 1 100 px.

**Défauts trouvés et corrigés.**
- Banc d'essai : `i / n` entre entiers donnait 0 (division entière de Rhai) ; 49 999 points confondus étaient signalés en simple info. Désormais : parcours vide après fusion = erreur `path.empty`, plus d'un point sur dix confondu = avertissement ; piège documenté dans la spécification.
- Paramètres affichés par ordre alphabétique : `serde_json` triait les clés (`preserve_order` activé).
- Libellés trop longs pour la colonne de 100 px : libellés du vase raccourcis, règle de 16 caractères dans la spécification.

**Non fait.** Visualiseur de PrusaSlicer, impression ; annuler / rétablir ; logo.

**Validation (24/10/2026).** Spécification, mesures et premier G-code validés par l'utilisateur ; choix confirmés : `set_speed` en mm/s, libellés de 16 caractères au plus, gabarits en expressions Rhai (ADR-0004 acceptée), division entière de Rhai documentée et signalée ; parades de performance retenues (fonctions natives en v0.2, calcul au relâchement au-delà de ≈ 60 ms). **Vase à ondulations imprimé sur la CORE One : « ça marche très bien ».**
