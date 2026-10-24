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

## 2026-10-24 — v0.1, étapes 3 à 5 : G-code complété, interface, planche de logos

**Choix.**
- Étape 3 : l'essentiel était en place ; tests ajoutés pour la machine à états (déplacement court sans rétraction, déplacement long avec rétraction et saut au-dessus du plus haut des deux Z, détente avant l'impression, ventilateur coupé en première couche puis du script puis du filament, température de croisière écrite une fois, `F` seulement au changement) et pour les gabarits de la CORE One (enceinte, attente, parcage). Consigne de ventilateur en entiers : 50 % → `M106 S128` (le calcul flottant donnait 127).
- Étape 4 : historique par instantanés du document (`edit(fn, mergeKey)`, fusion des modifications d'un même champ, `begin` / `end` pour un défilement de libellé), comme prescrit par la skill ; `NumField` ne borne plus aux bornes du modèle (règle de la skill : les limites métier sont signalées par le moteur) ; ↺ à côté du libellé d'une valeur modifiée (extension de `NumField` : `modified`, `onreset`, `resetHint`) ; calcul au relâchement si le premier calcul d'un geste dépasse 60 ms ; dimensions en mm ou en pouces ; listes d'imprimantes et de filaments lues par le moteur (`read_profile`), noms de filament par langue (`name = { fr, en }`).
- Libellés de paramètres : 14 caractères au plus (au lieu de 16), la colonne de 100 px accueillant aussi ⓘ et ↺ ; libellés du vase raccourcis (« Rayon de base », « Points/onde », « Pas vertical », « Surplus départ »).
- ADR-0003 acceptée : 200 195 segments rendus en 1,9 ms par image.
- Étape 5 : planche de 4 symboles × 4 couleurs générée par script (arcs exacts) ; deuxième passe pour la lisibilité à 16 px (traits de 9 à 10 unités, vides ≥ 5, spirale ramenée à 2,5 tours, vase à 3 couches).

**Vérification.** 33 tests ; navigateur : voir STATUS. La première mesure d'orbite (rAF) était faussée par le panneau masqué (bridage à 1 image/s) : mesure refaite en rendu synchrone.

**Logo (24/10/2026).** L'utilisateur écarte le premier tour et demande « deux cercles ondulés entrelacés » en bleu FabCAM (« les deux génèrent du G-code »). Deuxième tour : maillons, couronne tressée à 4 ondes, entrelacs à 3 ondes ; anneaux r(θ) = R + A·sin(nθ + φ), brin du dessous coupé net à 4,5–5 unités du brin du dessus, dessus-dessous alternés ; premières versions trop rasantes (coupes longues, effet « icône de chargement »), corrigées par des ondes plus marquées. **Retenu : entrelacs à 3 ondes.** Bleu `#58B5F5`, variante claire `#438ABA` ; ΔE 18 avec le bleu d'action, accepté (identité réservée au logo et au liseré). Favicons rendus directement à chaque taille.

**Jalon v0.1 validé par l'utilisateur (24/10/2026).** Skill `fab-design-system` complétée par une session annexe (identité de FabWeave, ↺ de `NumField`, libellés de 14 caractères, vue 3D WebGL) ; commitée dans FabWeave seulement, à la demande de l'utilisateur (la copie de FabVents reste à sa main).

## 2026-10-24 — v0.2, étape 1 : format et API

**Choix.**
- Paramètre `choice` (`options` en chaînes ou en `{ value, label }`) ; le script reçoit la chaîne ; interface : sélecteur à côté du libellé (2 options), pleine largeur (3), liste (plus).
- `warn(texte)` et `fail(texte)` : texte libre de l'auteur, chaîne ou `#{ fr, en }`, porté par le champ `text` des messages (codes `model.warning`, `model.failure`), avec la ligne du script.
- `path.note` / `note_at` (50 au plus) ; `bridge_to` et `bridge_points` (débit 0 à 2, ventilateur des ponts du filament).
- Listes de points natives sur le modèle des fonctions géométriques de FullControl (`circleXY`, `arcXY`, `helixZ`, `move` avec copies) pour porter ses modèles presque ligne à ligne ; angles en radians comme `sin` et `print_polar`. Les méthodes du parcours s'appellent `print_points` / `bridge_points` : `print` est réservé par Rhai.
- Mesures : `helix()` ≈ 70 × plus rapide qu'une boucle de script ; `modulate_r()` limité par les rappels du script (ADR-0002).

**Vérification.** 41 tests, dont l'exemple de la spécification exécuté tel quel ; l'exemple a été corrigé à la relecture (grille carrée qui débordait d'un anneau rond : tamis carré à fils ancrés sur le cadre).

**Non fait.** Affichage des annotations dans la vue 3D (étape 3) ; vérification de `choice` dans le navigateur (aucun modèle ne l'utilise encore).

## 2026-10-24 — v0.2, étape 2 : CLI, prompt pour IA, essai de génération

**Choix.**
- `fabweave-cli` (binaire `fabweave`) sans dépendance de lecture d'arguments : `check` calcule aux valeurs par défaut puis aux bornes de chaque paramètre (pièce trop grande aux bornes tolérée), explique les codes fréquents, sort en 1 s'il reste une erreur ; `stats`, `export`, `--json` ; profils MK4S et PLA embarqués par défaut.
- `docs/PROMPT_MODELE_IA.md` : prompt système autonome (format, API, pièges, règles d'impression, deux modèles complets, liste de vérification) ; ses exemples sont exécutés par les tests.
- `print_points` / `bridge_points` sautent un premier point confondu avec la position courante (`travel_to(pts)` puis `print_points(pts)` ne crée plus de point en double).

**Essai.** Trois agents (Sonnet), avec le seul prompt, sans terminal : abat-jour à côtes torsadées, ressort non planaire, panier tressé avec fond au choix (`choice`). **Trois sur trois valides au premier essai** (`fabweave check`). Constats : l'abat-jour et le panier sont plausibles (warn de porte-à-faux, deux fonds) ; le ressort est valide mais n'est pas un ressort (montée de 0,3 mm par tour : 1,7 mm de haut) : `check` vérifie la cohérence, pas l'intention. Prompt complété d'après leurs questions et un essai dans le moteur : « chaque passage s'appuie sur le précédent ; une pièce haute = beaucoup de tours », `z` = dessus du boudin, `0..=n`, réaffectation, fermetures, et le piège des `fn` qui ne voient ni `params` ni `printer`. Les trois modèles sont gardés en fixtures de test.
