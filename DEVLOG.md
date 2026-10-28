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

## 2026-10-27 — v0.2, étape 3 : interface du catalogue

**Choix.**
- Catalogue en sélecteur riche (`CatalogPicker`, `Popover` du kit) au lieu de la liste déroulante : vignette et nom du modèle affiché ; panneau avec recherche (tous les mots, sans accents, dans le nom, la description, la catégorie, les étiquettes et le nom de fichier), groupes par catégorie puis « Mes modèles ».
- Vignettes rendues par l'application : `Engine.thumbnail` calcule le modèle aux valeurs par défaut sur un poste séparé (le modèle courant reste chargé) et rend au plus 6 000 points ; dessin Canvas 2D en vue oblique, dégradé `--path-low` → `--path-high` en huit tranches de Z. Traits fins et semi-transparents : à 44 px, des traits opaques saturaient la silhouette en blanc ; le recouvrement des tours donne maintenant le relief. Calcul une à une, sans doublon (un appel pendant le calcul relance un tour), recalculées au changement de thème.
- Modèles de l'utilisateur : `⌘O`, menu du catalogue, ou dépôt n'importe où dans la fenêtre (voile pointillé) ; texte complet mémorisé sur le poste, identifiant tiré du nom de fichier (redéposer le même fichier le remplace et le relit). Retrait sans confirmation, avec « Annuler » dans le message (règle de la skill) ; les messages passagers acceptent désormais une action.
- Fiche du modèle : étiquettes, buses conseillées (en orange si la buse de l'imprimante n'en fait pas partie), fichier d'origine pour « Mes modèles ».
- Annotations (`path.note`) : pastille et point dans la vue 3D, bouton pour les masquer (mémorisé), texte dans la langue de l'interface. Vase à ondulations annoté (départ, fin de la spirale).

**Vérification.** 45 tests, clippy et `npm run check` sans erreur. Navigateur : voir STATUS.

**Défauts trouvés et corrigés.** Vignettes jamais calculées avant la première ouverture du catalogue (l'effet partait avant le chargement du profil imprimante) ; annotations restées en français après le passage en anglais (rendu seulement au calcul suivant) ; modèle dont le script échoue affiché avec une vignette vide sans marque.

**Non fait.** Fiche avec temps et matière estimés pour chaque modèle du catalogue (le temps et la matière du modèle affiché sont dans « Mesures ») ; vérifié avec un seul modèle livré : catégories multiples à revoir à l'étape 4.

## 2026-10-27 — v0.2, étape 4 : catalogue de départ

Étape 3 validée par l'utilisateur.

**Entrées.** Notebooks de FullControl (dépôt `FullControlXYZ/fullcontrol`, dossier `models/`) : entretoise non planaire, adaptateur hexagonal, treillis en polygones étoilés, moteur de motifs polaires, impression par gouttes ; abat-jour (gist de FullControl). Le défi porte-à-faux, le défi des picots et le porte-savon en serpentin ne sont publiés que dans l'application web de FullControl : seul le principe du porte-à-faux (arcs concentriques qui s'accrochent les uns aux autres, repris par ArcOverhangs) est documenté ; ces trois modèles sont écrits pour FabWeave « d'après » FullControl, sans prétendre au port.

**Choix.**
- API-13 `contour.fill_lines(spacing, angle[, inset])` (module `fill.rs`) : lignes de balayage dans le repère tourné, intervalles par règle pair-impair (contour quelconque), retrait mesuré perpendiculairement au bord rencontré (plafonné pour un bord presque parallèle), chaînage en zigzag tant que le lien est court (3 × écart) et ne coupe aucun bord ; résultat en tableau de chaînes, que le script rejoint par des déplacements. Anneaux concentriques reportés (API-14, v0.3) : un décalage de polygone non convexe sûr demande une bibliothèque ou un travail à part.
- `pts.slice(from, to)` : sans elle, `a + b` répétait un point (« points confondus ignorés » dans les messages). `floor`, `round`, `ceiling` acceptent un entier : l'écriture `floor(200 / n)` (division entière voulue) échouait ; piège rencontré deux fois en écrivant le catalogue.
- Fond du vase à ondulations (CAT-23) : contour de la couche j = tour (j − nb) de la formule de la spirale, ce qui continue le tissage ; dernière couche : remplissage puis contour, qui s'achève au point de départ de la spirale (aucune couture, aucun déplacement) ; sur le fond, la hauteur du boudin du premier tour part de 0,1 × et grandit avec l'écart réel. Vitesse nominale du filament pour le fond (à la vitesse du vase, 4 mm/s, une couche prenait 9 min). Défaut : « Aucun », comme l'original imprimé et validé.
- `fabweave check` et le test du catalogue tolèrent un `fail` du modèle **aux bornes** (combinaison impossible avec les autres valeurs par défaut : trou de 40 mm dans un hexagone de 16) ; jamais aux valeurs par défaut. `--set` garde en texte la valeur d'un `choice`.
- Ressort (CAT-04) : la proposition de la recherche (hélice de 4,5 mm de pas, en l'air) n'est pas imprimable ; remplacée par un ressort-soufflet en spirale continue, plis en dents de scie (pente constante, avertissement au-delà de 50°), en anneaux ou en hélice.
- Abat-jour : la cellule entre deux branches est symétrique ; elle est évaluée directement (même résultat que la réflexion de l'original) puis copiée en rotation en natif : 50 évaluations de script par couche au lieu de 300, 232 000 points en 80 ms en natif, 170 ms dans le navigateur.
- Motifs polaires : les listes d'angles et de rayons de l'original deviennent branches, rayon intérieur (négatif : double étoile) et saut (polygones étoilés) ; treillis : sous-famille M1 seulement.
- Picots verticaux extrudés par tronçons de 1 mm, tous les picots à chaque passage, pour que le bloc de chauffe ne redescende jamais près d'un picot déjà haut (pas encore de garde-buse, v0.5).
- Porte-savon : serpentin = `fill_lines` sur une ellipse ; une couche sur deux à l'envers, donc aucun déplacement sur toute la pièce ; épaisseur de couche variable (non planaire) pour creuser le dessus.
- Rubriques : Défis d'impression, Grilles et treillis, Maison, Mécanique, Motifs, Vases et luminaires.
- Développement : Vite ne surveillait que `ui/` ; un `.weave` ajouté dans un nouveau dossier de `models/` n'apparaissait qu'au redémarrage. `models/` et `profiles/` ajoutés à la surveillance.

**Vérification.** 51 tests (dont 5 sur le remplissage et un sur l'API au niveau du script ; tous les blocs `rhai` de la spécification sont désormais exécutés) ; chaque modèle passe `fabweave check` et le test du catalogue (MK4S et CORE One, PLA et PETG, bornes). Navigateur : les onze modèles et leurs vignettes, fond du vase vu de dessous.

**Défauts trouvés et corrigés.** Point du fond confondu avec le premier de la spirale ; déplacement inutile au premier picot de chaque passage ; avertissement du porte-savon déclenché aux valeurs par défaut (nombre de couches désormais calculé sur la hauteur moyenne) ; annotations trop longues pour la vue.

**Non fait.** Impression par gouttes (CAT-12, extrusion à l'arrêt : API-15, v0.3) ; anneaux concentriques (API-14) ; sous-familles M2 à M4 du treillis ; impressions de validation.

**Validation (27/10/2026).** Étape 4 et jalon v0.2 validés par l'utilisateur : trois modèles du catalogue imprimés, « bien passés ». Pour la v0.3, le fablab dispose d'une **Bambu Lab X1E**, alimentée par la carte SD.

## 2026-10-27 — v0.3, étape 1 : Bambu Lab X1E par la carte SD

**Entrées.** Profils de Bambu Studio (`bambulab/BambuStudio`, `resources/profiles/BBL/machine` : « Bambu Lab X1E 0.4 nozzle », gabarits de démarrage du 31/10/2025, de fin du 02/04/2024, de changement de couche) ; outil `bambox` (estampo, MIT) pour la structure du paquet `.gcode.3mf` et les marqueurs que le firmware attend ; retours d'utilisateurs (export « fichier tranché de la plaque » lu depuis la carte SD).

**Choix** (ADR-0005, proposée).
- Dialecte Bambu : en-tête `HEADER_BLOCK` en tête de fichier, `M73 L` + `M991` à chaque couche, progression `M73 P R`. Une « couche » = palier de hauteur atteinte (dépassement d'une hauteur de couche du profil), seule définition qui tienne pour les spirales et le non planaire. Le plan garde désormais la durée de chaque mouvement.
- Paquet `.gcode.3mf` écrit par le moteur (module `package.rs`) : ZIP déterministe (date fixe), CRC-32 et PNG écrits à la main, `md-5` et `miniz_oxide` en dépendances (Rust pur) ; vignettes 512 et 128 px du parcours, fond transparent. Sans `project_settings.config` (544 clés chez `bambox`) : l'essai dira s'il faut l'ajouter.
- Identifiant de la X1E : `C13` (profil de Bambu Studio) et non `BL-P003` (table de `bambox`, non confirmée).
- Profil X1E : 645 lignes de gabarit converties par script (variables) puis blocs `{if}…{endif}` réécrits à la main en expressions Rhai ; chaînes multi-lignes entre accents graves, profil en chaînes TOML littérales `'''`. Nouvelle fonction de gabarit `num()` et variables `bed_type`, `max_volumetric_speed`. Retirés : scan lidar de la première couche et détection d'impression dans le vide (fausses alertes sur un parcours non planaire). Conservés tels quels : purge à 290 °C, refroidissement de l'enceinte pour le PLA et le PETG, nivellement limité à la zone de la première couche, abaissement de 0,04 mm sur plaque texturée, calibrations conditionnées par le firmware (`M1002 judge_flag`).
- Interface : l'export devient un menu d'une ligne par format (règle de la skill, repris de FabVents) quand l'imprimante en propose plusieurs ; sur les Prusa, le bouton reste simple.

**Vérification.** 55 tests (paquet relu : CRC, MD5, métadonnées, vignette ; G-code de la X1E : en-tête, autant de `M73 L` que de couches annoncées, gabarits rendus sans macro restante) ; le catalogue entier passe sur la X1E. Paquet relu indépendamment par `zipfile` de Python (CRC et MD5 justes). Navigateur : menu d'export sur la X1E, les deux fichiers produits (téléchargements interceptés, rien d'écrit sur le poste), bouton simple sur la MK4S.

**Défauts trouvés et corrigés.** Infobulle du bouton qui masquait le menu ; volume de filament de l'en-tête écrit en mm³ sous une étiquette en cm³ (d'après un souvenir non vérifié du format de Bambu Studio : corrigé en cm³). En nettoyant après les essais du catalogue, le fond du vase était resté sur « 2 couches » dans le navigateur de l'utilisateur : remis sur « Aucun ».

**Non fait.** Essai sur la machine (⏸) ; zone de purge à l'avant du plateau non protégée par le moteur (Y < 15 mm, atteinte seulement par une pièce de plus de 226 mm de profondeur ; zones exclues à l'étape 2) ; profils A1, P1, X1.

**Suite (27/10/2026).** Essai sur la X1E reporté par l'utilisateur, non bloquant : il se fera avec l'impression de validation de l'étape 6.

## 2026-10-27 — v0.3, étape 2 : profils complets et dialectes

**Entrées.** Profils PrusaSlicer (PrusaResearch 2.5.13) : vignettes `16x16/QOI, 313x173/QOI, 480x240/QOI, 380x285/PNG` de la MK4S et de la CORE One, accélérations des profils 0.20mm STRUCTURAL (parois extérieures 1 500 mm/s², première couche 500, déplacements 4 000 / 7 000), jerk 8 / 10. Bambu Studio (X1E) : `bed_exclude_area` (coin avant gauche 18 × 28 mm), jerk 9, déplacements plafonnés à 9 000 mm/s², parois extérieures 5 000. Spécification QOI (qoiformat.org) ; documentation de Klipper (`SET_PRINT_STATS_INFO`, `SET_PRESSURE_ADVANCE`, `M73`).

**Choix.**
- Profil imprimante **v2**, champs ajoutés tous facultatifs : un profil v1 se lit tel quel (la « migration » n'a rien à changer), et un lecteur v1 refuse un profil v2 sur son numéro de version plutôt que sur un champ inconnu. Profil MK4S de la v0.2 gardé comme fixture de lecture. `firmware` devient une énumération (nom inconnu refusé) ; `name` accepte une table par langue.
- **Zones exclues** (`[bed] exclude`) : erreur `path.excluded_zone` (export bloqué) si un mouvement imprimé y passe (Liang–Barsky, bords tolérés) ; nom de la zone par langue porté par le message (nouveau champ `name` des messages). « Ligne de purge hors pièce » réglée ainsi : chaque gabarit purge dans une zone déclarée exclue (bord gauche pour Marlin et Klipper, bande avant Y < 13 mm et coin de Bambu Studio pour la X1E ; les Prusa purgent hors plateau, Y < 0). Zones hachurées sur le plateau de la vue 3D (jeton `--view-bed-exclude`).
- **Temps avec accélérations** (ENG-15, avancé de la v0.4 parce que la progression `M73 R` en dépend) : trapèze de vitesse par mouvement, vitesse de passage d'un angle limitée par le jerk (écart des vecteurs vitesse, modèle de Marlin), passes arrière puis avant ; un déplacement rétracté part et finit à l'arrêt. Sans accélérations dans le profil, estimation inchangée.
- **Dialectes** : progression `M73 P R` pour tous, écrite quand le pourcentage ou les minutes changent ; couches `;LAYER_CHANGE` / `;Z:` / `;HEIGHT:` (Prusa, Marlin), `SET_PRINT_STATS_INFO` (Klipper), `M73 L` / `M991` (Bambu). Accélérations : `M204 P T` (Prusa, Marlin), `M204 S` réécrit à chaque passage impression ↔ déplacement (Klipper, Bambu). L'avance de pression reste dans les gabarits, un par dialecte.
- **Vignettes** (`[gcode] thumbnails`, syntaxe de PrusaSlicer) : rendu oblique du module `image.rs` (sorti de `package.rs`) à toute taille, encodeurs PNG et QOI et base64 écrits à la main (pas de dépendance) ; bloc `THUMBNAIL_BLOCK` de Bambu Studio pour la X1E.
- **Cône de la buse** : champs `cone_angle` (pente maximale depuis l'horizontale) et `cone_height`, transmis au script (`printer.nozzle_cone_angle`, 0 si inconnu). Aucune valeur mise dans les profils livrés : les fabricants ne la publient pas, elle est à mesurer.
- Profils **Marlin générique 220 × 220** (démarrage des profils Cura de l'Ender-3) et **Klipper générique 235 × 235** (gabarit explicite, appel d'une macro `PRINT_START` indiqué en commentaire) ; Bambu Lab A1, P1, X1 laissés de côté (pas de machine pour les essayer).

**Vérification.** 65 tests (lecture du profil v1, contrôles du format v2, aller-retour QOI avec un décodeur écrit d'après la spécification, base64 de la RFC 4648, trapèzes et angles, zones exclues, progression de 0 à 100 sur quatre dialectes, couches et accélérations par dialecte, vignettes) ; le catalogue entier passe sur les cinq imprimantes. Vignettes d'un G-code de la MK4S relues par un décodeur Python indépendant (QOI et PNG, longueurs annoncées justes) et regardées. Navigateur : noms des profils en FR et EN, zones de la X1E hachurées, modèle d'essai traversant la bande de purge refusé avec le nom de la zone en FR et EN, export désactivé ; état remis (MK4S, vase, français) et modèle d'essai supprimé.

**Défauts trouvés et corrigés.** Hachures mal coupées sur un côté de la zone ; message « 1 segments » (compte retiré du texte).

**Non fait.** Valeurs du cône des machines du fablab ; temps estimé comparé à PrusaSlicer (critère ± 15 %) ; vignettes vues sur l'écran d'une Prusa ; profils A1, P1, X1.

## 2026-10-27 — v0.3, étape 3 : buse, placement, adhésion

**Entrées.** PrusaSlicer (PrusaResearch 2.5.13) : buses de la MK4S et de la CORE One (0,25 à 0,8 mm, HF 0,4 à 0,8), hauteur de couche du profil d'impression par défaut de chaque buse, débit maximal des filaments génériques par buse (`@PG 0.6 / 0.8`, `@COREONE HF0.4` à `HF0.8`), dégagement de la tête (`extruder_clearance_radius` / `_height` : 45 / 13 mm MK4S, 75 / 33 mm CORE One). Bambu Studio : buses 0,2 à 0,8 de la X1E et leurs profils « Standard », dégagement 57 / 34 mm. Transformée de distance exacte de Felzenszwalb et Huttenlocher ; carrés marchants ; Douglas–Peucker.

**Choix.**
- **Préparation hors du modèle** (`setup.rs`) : buse, placement, adhésion forment un `Setup` choisi par l'utilisateur, envoyé au moteur à part du modèle et des profils, contrôlé sans correction (`setup.out_of_range`), rappelé en tête du G-code. Le `.weave` n'en sait que `printer.*`, qui suit la buse montée ; il peut seulement conseiller une adhésion (`recommended_adhesion`).
- **Buse** : profils « effectifs » dérivés des profils livrés, jamais modifiés. Couches par buse (`layers.by_nozzle`) ; diamètre libre : interpolation entre les buses connues, dite (`nozzle.custom`). Débit maximal du filament par buse (`speed.by_nozzle`, l'entrée la plus précise l'emporte). Filament abrasif dans une buse qui ne l'est pas : avertissement. Choix mémorisé par imprimante dans le document (donc annulable).
- **Copies** pièce par pièce, seule façon générale pour des parcours continus ou non planaires (impossible de les entrelacer couche par couche) : écart d'au moins le rayon de dégagement de la tête (par défaut égal), pièce pas plus haute que l'espace libre sous le portique, sinon erreur. Grille en serpentin. Le compteur de couches du firmware repart du plateau à chaque copie, sauf pour la première pièce posée à la hauteur de la bordure (défaut trouvé par un test : une couche comptée deux fois).
- **Bordure et radeau** (`adhesion.rs`) sans bibliothèque de polygones : empreinte de la première couche tracée en grille (0,1 mm, agrandie au-delà de 2 millions de cellules), trous bouchés par remplissage depuis le bord (bordure à l'extérieur seulement, radeau plein), carte des distances exacte, contours par carrés marchants, simplifiés à 0,01 mm. Bordure de la boucle la plus éloignée à la plus proche ; radeau rempli par `fill_lines` (base et contact pleins, intermédiaires à 2 mm, sens alterné), pièce rehaussée de l'épaisseur et de l'écart de décollage. Ajoutés avant le placement : leur emprise compte dans les contrôles du plateau et des zones.
- **Placement** : centre de l'encombrement au point choisi. Une pièce hors plateau (ou dans une zone exclue) n'est plus escamotée : le plan est calculé, les segments en cause marqués et dessinés en rouge (jeton `--danger`), l'export reste refusé. Plateau rond (`shape = "circle"`), dépassement radial, dessiné rond.
- **Interface** : buse sous l'imprimante (liste du profil + « Autre… », HF et abrasive quand l'imprimante les propose), couches et débit maximal sous le filament ; section « 4 Plateau » : position, rotation, copies, écart (souligné sous le dégagement), adhésion en sélecteur segmenté avec ses réglages, conseil du modèle rappelé. `.metrics` et `.note` passés dans `app.css` (partagés). Ligne de commande : `--nozzle`, `--high-flow`, `--rotate`, `--copies NxM`, `--at x,y`, `--brim`, `--raft`.

**Vérification.** 77 tests (transformée de distance comparée à la force brute, bordure à la bonne distance d'un anneau et fermée, deux îlots séparés ou réunis, radeau couvrant l'empreinte plus son débord, simplification ; buse effective, rotation, copies et leurs refus, radeau qui rehausse, bordure en tête, valeurs refusées et non bridées ; placement demandé, plateau rond ; couches par copie et rappel de la préparation dans le G-code ; chaque modèle du catalogue avec bordure et avec radeau). Ligne de commande : `M862.1 P0.6 A0 F1` pour une buse 0,6 HF. Navigateur : buse 0,6 → couches de 0,25 mm, HF → 28 mm³/s, choix mémorisé pour la MK4S, ⌘Z ; deux copies du vase refusées (40 mm > 13 mm sous le portique de la MK4S), export désactivé ; bordure qui épouse les ondulations, radeau sous la pièce ; pièce poussée hors du plateau surlignée en rouge ; thème clair ; état remis à l'identique par annulation.

**Défauts trouvés et corrigés.** Couche de la bordure comptée deux fois au début de la première copie ; mesures de la buse d'abord placées avant le filament dont elles dépendent.

**Non fait.** Placement à la souris dans la vue (valeurs numériques seulement) ; ordre des copies selon la forme de la tête ; bordure intérieure (dans les trous) ; aucun modèle du catalogue ne conseille encore d'adhésion ; impressions d'essai de la bordure et du radeau.

## 2026-10-27 — v0.3, étape 4 : édition et import des profils

Étape 3 validée par l'utilisateur.

**Entrées.** `PrusaResearch.ini` de PrusaSlicer (prusa3d/PrusaSlicer-settings-prusa-fff) : préréglages « Original Prusa MK4S 0.4 nozzle », « Generic PLA @PG », « 0.20mm SPEED @MK4S 0.4 », aplatis comme le fait « Exporter la configuration ». Bambu Studio (`resources/profiles/BBL`) : `fdm_machine_common`, `fdm_bbl_3dp_001_common`, « Bambu Lab X1E 0.4 nozzle », `fdm_filament_common`, `fdm_filament_pla` (noms des clés, valeurs `nil`, listes par extrudeur, `bed_exclude_area` en plusieurs rectangles). Fichiers d'essai reconstitués dans `crates/fabweave-core/tests/fixtures/import/`, sources citées.

**Choix** (ADR-0006, proposée).
- Module `import.rs` (moteur pur) : `.ini` à plat ou en paquet (héritage multiple résolu dans le fichier, préréglages abstraits `*…*` ignorés), `.json` de Bambu Studio et d'OrcaSlicer ; un profil par préréglage. Chaque profil part d'un **profil livré de base** (même firmware, même matière, le profil choisi d'abord) ; ce qui est propre à une machine ou à un filament est laissé vide plutôt que repris. Rapport : valeurs reprises (clé, valeur), absentes (base, vide, estimée), clés ignorées, gabarits sans équivalent, parent `inherits` absent. Pour un export à plat, les clés ignorées sont comptées une fois, sur l'imprimante.
- **Traduction des gabarits** : lexique et analyseur des expressions des trancheurs vers Rhai ; blocs `{if}` en chaînes `` `…` `` (Rhai saute le saut de ligne qui suit l'accent grave ouvrant : doublé) ; conditions en chaîne `a ? b : c ? d : e` aplaties en `if … else if …` et profondeur des expressions de Rhai relevée à 128 (température de palpage des Prusa : six cas) ; réglages du fichier remplacés par leur valeur ; `layer_z` du gabarit de fin = `max_z`. Le découpage des gabarits suit désormais les chaînes `` `…` ``. Le non-traduisible reste tel quel et met le profil en erreur.
- **Relecture** : champ `[printer] review` (profil v2, facultatif), erreur `profile.review_pending` posée après le calcul (le parcours se voit, l'export attend).
- Profils : lecture séparée du contrôle (`read_printer` / `check_printer`) pour ouvrir dans l'éditeur un profil lisible mais faux ; écriture TOML (`printer_toml`, `filament_toml`, fonction `display` de `toml`) ; vignettes avant les gabarits dans `[gcode]`.
- Interface : menu « ⋯ » à côté de l'imprimante et du filament (modifier ou copier et modifier, dupliquer, importer, exporter en TOML, supprimer avec « Annuler ») ; listes « Livrés » / « Mes profils » ; éditeur en fenêtre décrit par un schéma de champs (`profileFields.ts`), contrôle par le moteur 150 ms après chaque saisie, ↺ par valeur, étiquette d'origine pour un import, tables (zones, couches et débits par buse), gabarits en zone de texte avec « J'ai relu » ; fenêtre d'import avec rapport et détail ; dépôt d'un `.toml`, `.ini` ou `.json` n'importe où. Profils de l'utilisateur mémorisés sur le poste ; un profil se recharge dans le moteur quand son texte change. `NumField` accepte une valeur facultative (`onclear`, `placeholder`). Champs des profils nommés dans les messages (« Filament › Débit (coef.) »).

**Vérification.** 86 tests : traduction rendue et comparée à la sortie attendue, morceaux non traduits signalés à leur ligne, avance de pression lue dans le gabarit de filament de PrusaSlicer, déséchappement du `.ini` ; export PrusaSlicer → imprimante Prusa et filament PLA conformes, gabarits entièrement traduits et valides ; G-code exporté avec ces profils refusé avant relecture, puis contenant `M862.3 P "MK4S"`, `M104 T0 S170` (palpage du PLA calculé par l'expression de PrusaSlicer), `M106 S100` ; machine X1E et filament de l'utilisateur de Bambu Studio ; fichiers refusés ; aller-retour TOML des profils livrés. Navigateur : voir STATUS.

**Défauts trouvés et corrigés.** Firmware des Prusa récentes non reconnu (`printer_notes` sans `PRINTER_VENDOR_PRUSA3D` : reconnu au modèle `MK4S`, `COREONE`…) ; « Expression exceeds maximum complexity » de Rhai sur la température de palpage ; `binary_gcode = 1` listé comme gabarit ; saut de ligne perdu en tête des blocs ; en-têtes de la table des zones non traduits ; libellés tronqués ; après suppression, retour au premier profil livré au lieu de la base de l'import.

**Non fait.** Fichiers réels d'utilisateurs (OrcaSlicer surtout) ; paquets `.bbscfg` / `.orca_printer` (archives ZIP) ; gabarits de changement de couche ; profils de l'utilisateur partagés entre postes (fichier TOML exporté seulement) ; impression avec un profil importé.

## 2026-10-28 — v0.3, étape 5 : anneaux concentriques et gouttes

Étape 4 validée par l'utilisateur ; ADR-0006 acceptée.

**Entrées.** Notebook `blob_printing.ipynb` de FullControl (tube de gouttes : taille 1,6 mm, hauteur moitié, chevauchement 33 %, volume = hauteur × taille², buse à 95 % d'une hauteur de goutte, deux rangées pleines en haut et en bas, une goutte sur deux entre les deux, 0,5 à 1 s par goutte conseillées). `i_overlay` 9.1, déjà employé par FabVents pour décaler les contours.

**Choix.**
- **Anneaux concentriques** (`fill_rings(spacing[, inset])`, API-14) : décalages de `i_overlay` en arithmétique entière (même résultat en natif et en WebAssembly), jonctions arrondies. Chaque anneau est décalé du précédent (érosion par un disque : deux décalages valent un décalage de leur somme) et simplifié à 0,01 mm près. Décaler le contour d'origine d'une grande distance coûte cher (29 ms pour 9,6 mm sur 400 points ; 250 ms le fond entier) ; décaler sans simplifier multiplie les points (800 000 au 15e anneau). Pas à pas avec simplification : 31 anneaux en 9 ms. Îlots d'un même niveau dans l'ordre du plus proche ; chaque anneau dans le sens du contour, commencé au sommet le plus proche du départ du précédent (coutures alignées).
- Vase à ondulations : paramètre « Motif du fond » (lignes ou anneaux, lignes par défaut : G-code inchangé) ; anneaux de la dernière couche du centre vers le bord, pour que le contour puis la spirale suivent.
- **Goutte** (`path.blob(volume, durée)`, API-15) : mouvement `Blob` au point courant, `G1 E… F…` sans coordonnées. `E` = volume × multiplicateur de débit ÷ section du filament. Une goutte plus rapide que le débit max dure plus longtemps (comptée dans `speed.limited_flow`) ; sous `F1`, elle est ralentie et signalée (`blob.too_slow`). Dans l'estimation du temps avec accélérations, une goutte est un arrêt complet. Volume de 0 à 1 000 mm³, durée de 0 à 60 s ; goutte sans position refusée. Aperçu : sphère de même volume, centrée une demi-goutte sous la buse (vue 3D en `InstancedMesh`, vignettes du moteur en disques) ; le WebAssembly transmet la largeur de chaque point.
- **Tube de gouttes** (CAT-12) : port du notebook, durée par goutte en secondes au lieu de la vitesse d'extrusion propre à chaque imprimante ; amorçage par la ligne de purge du profil au lieu des gouttes d'amorçage de l'original ; ventilateur à 100 % comme l'original.

**Vérification.** 93 tests : anneaux d'un carré (dix, fermés, dans le sens du contour, coutures alignées), contour horaire, col qui sépare en deux îlots, limite de points, déterminisme sur un contour ondulé ; `fill_rings` depuis le script ; goutte (position, diamètre, `E`, durée, débit plafonné, `G1 E… F…` dans le G-code, erreurs) ; parcours fait seulement de gouttes. Tube de gouttes : 406 gouttes, 5 min, 1,0 g ; G-code relu (`G1 E0.85146 F73` par goutte, rétraction entre les gouttes des rangées ajourées, plus espacées que `min_travel`). Navigateur : voir STATUS.

**Défauts trouvés et corrigés.** Décalages lents puis explosion du nombre de points (voir Choix) ; boucle signalée par clippy après l'ajout des arrêts sur goutte ; compte de calculs de `fabweave check` du vase (un paramètre de plus).

**Non fait.** Impression des gouttes et d'un fond en anneaux ; pas de dégradé d'éclairage sur les sphères (couleur à plat, comme les lignes) ; la vignette du catalogue dessine les gouttes à une taille fixe (elle ne reçoit pas leur diamètre).

## 2026-10-28 — Deux modèles d'après photos : photophore vermiculé, vase à écailles

En attendant les impressions de l'étape 6, à la demande de l'utilisateur (photo `tmp/nouveaux-modeles.png`, hors dépôt, deux pièces à reproduire).

**Lecture des photos.** À gauche, un cylindre couvert de petits reliefs en C, courts et serrés, éclairé de l'intérieur. À droite, un vase hexagonal aux faces creusées et à la taille resserrée, couvert d'écailles en quinconce : la texture de « ripple texture » de FullControl (un point sur deux sorti, nombre impair de points par tour).

**Choix.**
- **`noise(x, y, z)`** (API-17), bruit de gradient 3D natif : gradients tirés d'un hachage entier des nœuds (pas de table copiée), courbe de degré 5 ; pur et déterministe. Pris sur le cylindre en 3D, il n'a pas de couture.
- **Photophore vermiculé** (CAT-24) : paroi en spirale, rayon + relief. Relief = lignes de niveau zéro du bruit, épaissies (`1 − |n| / épaisseur`, profil adouci), coupées en tronçons là où un second bruit, 1,6 fois plus fin, est bas. Réglages comparés sur des cartes de hauteur de la paroi déroulée : un masque aussi large que le motif retire des zones entières ; 1,6 fois plus fin, il donne des vermicelles courts répartis partout, comme sur la photo. Fond en anneaux (2 couches par défaut). Prévu pour une bougie LED.
- **Vase à écailles** (CAT-25) : contour polygonal aux faces creusées en parabole, points répartis à pas égal le long du contour (écailles de même taille sur les faces et aux arêtes), sortie selon la normale à la paroi, taille en sablier, torsion facultative ; source FullControl citée pour la texture.
- Dans les deux scripts, le calcul de la boucle principale est écrit en ligne : appeler une fermeture qui renvoie un tableau à chaque point est nettement plus lent dans le navigateur (vase à écailles : 1,1 s → 0,75 s au premier calcul). Conseil ajouté à la spécification.

**Vérification.** `fabweave check` des deux modèles aux bornes de chaque paramètre ; catalogue sur cinq imprimantes ; test de `noise` (borné, centré, continu, valeur de référence) et depuis un script. Navigateur : formes comparées aux photos ; 380 ms (112 000 points) et 200 ms (76 000 points) par calcul, moteur chaud ; le premier calcul après chargement est 2 à 4 fois plus lent.

**Non fait.** Impression ; le rendu en lignes de la vue 3D ne montre pas bien un relief (pas d'ombrage).

## 2026-10-28 — Fond des vases : au contour sans texture

**Retour de l'utilisateur.** Sur les modèles avec fond, celui-ci « dépasse du modèle, on dirait un radeau ».

**Cause.** Le fond suivait le contour du premier tour, texture comprise (pointes des ondulations, écailles sorties, vermicelles du bas). Il ne sortait pas de la ligne médiane de la paroi, mais il formait une couche pleine aussi large que les pointes, tout autour, alors que la paroi au-dessus alterne creux et pointes. Sur le photophore, il reproduisait même les vermicelles de la base, là où la paroi au-dessus n'en a pas.

**Choix.** Fond au contour **sans texture** : creux des ondulations (vase à ondulations), contour sans écailles (vase à écailles), cercle de base (photophore). La texture naît sur les 6 premiers tours au-dessus du fond (environ 1,5 mm, profil adouci), sinon ses premières pointes déborderaient du fond dans le vide. Sans fond, rien ne change. Règle ajoutée à la spécification et au prompt pour IA.

**Vérification.** Rayon maximal du fond comparé à la paroi juste au-dessus : vase à ondulations 20,00 / 20,98 mm, photophore 35,01 / 35,95 mm, vase à écailles 35,00 / 35,16 mm (le fond ne dépasse plus). Pied du vase à écailles vu de face dans l'application. Fichiers d'essai régénérés (CORE One, photophore, vase à écailles).
