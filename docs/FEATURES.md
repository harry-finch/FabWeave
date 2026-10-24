# Inventaire des fonctions

Périmètre fonctionnel de FabWeave, établi à partir de la conversation de recherche du 23/10/2026 (`docs/research/`, hors dépôt), de l'étude de fullcontrol.xyz (24/10/2026, voir plus bas) et des besoins d'un fablab. Chaque fonction a un identifiant stable, un jalon ([ROADMAP.md](../ROADMAP.md)) et un état.

États : **fait** · **en cours** · **prévu** · **à décider** · **hors périmètre**. *Ajout* = fonction absente de FullControl.

## Format de modèle `.weave` (FMT)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| FMT-01 | Fichier texte : en-tête TOML, séparateur `---`, corps Rhai | v0.1 | fait |
| FMT-02 | `[meta]` : nom, catégorie, description, auteur, étiquettes, buse recommandée, `format` et `version` | v0.1 | fait |
| FMT-03 | Paramètres `float`, `int`, `bool` : libellé, unité, défaut, min, max, pas ; ordre du fichier conservé | v0.1 | fait |
| FMT-04 | Paramètre `choice` (liste de valeurs nommées) | v0.2 | fait |
| FMT-05 | `advanced = true` (« ▸ Avancé ») fait en v0.1 ; groupes de paramètres | v0.2 | en cours |
| FMT-06 | Libellés, descriptions et infobulles FR / EN (`label = { fr = "…", en = "…" }` ou chaîne simple) | v0.1 | fait |
| FMT-07 | `source` et `license` pour les modèles repris d'ailleurs (FullControl) | v0.1 | fait |
| FMT-08 | Erreurs de lecture et de script avec ligne et colonne, code stable | v0.1 | fait |
| FMT-09 | Spécification `docs/WEAVE_FORMAT.md` (premier jet fait) et prompt système pour IA `docs/PROMPT_MODELE_IA.md` | v0.1 / v0.2 | en cours |
| FMT-10 | Versionnement et migration des anciens formats | v0.2 | prévu |

## API du script (API)

Variables injectées : `params` (valeurs des curseurs), `printer` (diamètre de buse, hauteur de première couche, hauteur de couche conseillée), `PI`.

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| API-01 | `new_path()`, `print_to(x, y, z, w, h)`, `travel_to(x, y, z)` | v0.1 | fait |
| API-02 | `print_polar(r, θ, z, w, h)` (coordonnées cylindriques) | v0.1 | fait |
| API-03 | `set_speed_factor(f)`, `set_fan(%)` | v0.1 | fait |
| API-04 | Fonctions mathématiques identiques en natif et en WebAssembly (crate `libm`) | v0.1 | fait |
| API-05 | `bridge_to(x, y, z, w, h, flux)` : pont tendu, ventilateur forcé, facteur d'étirement | v0.2 | fait |
| API-06 | `warn(texte)` / `fail(texte)` : messages du modèle, texte par langue, ligne du script | v0.2 | fait |
| API-07 | `path.note(texte)`, `path.note_at(x, y, z, texte)` : annotations (moteur fait ; affichage dans la vue 3D à l'étape 3) | v0.2 | en cours |
| API-08 | Listes de points natives : `line`, `arc`, `circle`, `polygon`, `rectangle`, `zigzag`, `sine`, `spiral`, `helix` | v0.2 | fait |
| API-09 | Transformations natives : translation, rotation, miroir, répétition en Z, copie tournée, inversion de sens | v0.2 | fait |
| API-10 | `pts.modulate_z(f)`, `pts.modulate_r(cx, cy, f)` : déformation par une fonction du script | v0.2 | fait |
| API-11 | Subdivision (`pts.subdivide`, faite en v0.2) ; discrétisation adaptative (tolérance de corde) | v0.2 / v0.5 | en cours |
| API-12 | Essuyage (*wipe*), pause (insertion d'objet, ruban de renfort) | v0.3 | prévu |
| API-13 | Remplissage natif d'un contour fermé (zigzag croisé d'une couche à l'autre, ou concentrique) : fonds pleins | v0.2 | prévu |

## Moteur : physique et G-code (ENG)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| ENG-01 | Bac à sable Rhai : limites d'opérations, de profondeur, de points ; aucun accès fichier ni réseau | v0.1 | fait |
| ENG-02 | Section du boudin (stade si W > H, rectangle sinon) × longueur 3D → volume → `E` selon le diamètre et le multiplicateur de débit | v0.1 | fait |
| ENG-03 | Extrusion relative (`M83`) | v0.1 | fait |
| ENG-04 | Centrage automatique sur le plateau (origine au coin ou au centre) ; dépassement bloquant | v0.1 | fait |
| ENG-05 | Vitesse = vitesse nominale du filament × facteur du script ; première couche ralentie | v0.1 | fait |
| ENG-06 | Plafond de débit volumétrique (mm³/s) : vitesse réduite, segments comptés et signalés | v0.1 | fait |
| ENG-07 | Plafond de vitesse de l'axe Z (non planaire) : vitesse réduite, comptée et signalée | v0.1 | fait |
| ENG-08 | Ventilateur : coupé en première couche, valeur du filament, surcharge du script, forcé sur les ponts | v0.1 | fait |
| ENG-09 | Déplacements : rétraction, saut en Z, distance minimale de rétraction | v0.1 | fait |
| ENG-10 | Machine à états : `F`, `M106` émis seulement au changement ; points en double ignorés (comptés) | v0.1 | fait |
| ENG-11 | Gabarits de démarrage et de fin : expressions Rhai `{ … }` sur les variables de l'impression (ADR-0004) | v0.1 | fait |
| ENG-12 | Métadonnées en commentaire : temps estimé, filament (mm, cm³, g), type, buse, modèle d'imprimante | v0.1 | fait |
| ENG-13 | Dialectes : Marlin, Prusa Buddy, Klipper ; avance de pression | v0.3 | prévu |
| ENG-14 | Vignette intégrée au G-code (PNG / QOI pour les écrans Prusa) | v0.3 | prévu |
| ENG-15 | Estimation du temps avec accélérations | v0.4 | prévu |
| ENG-16 | Mesures : nombre de points et de segments, longueur imprimée, déplacements, temps, filament, débit max, Z max, encombrement | v0.1 | fait |
| ENG-17 | Bambu Lab : G-code brut pour la carte SD ou paquet `.gcode.3mf` (MD5, métadonnées de plaque) | v0.3 | à décider |
| ENG-18 | Adhésion au plateau générée par le moteur : bordure (*brim*) autour de l'empreinte de la première couche, ou radeau (*raft*) sous la pièce ; emprise incluse dans le contrôle du volume | v0.3 | prévu |

## Profils imprimante et filament (PRF)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| PRF-01 | Profil imprimante TOML : plateau, origine, buse, vitesses max, vitesse Z max, gabarits | v0.1 | fait |
| PRF-02 | Profil filament TOML : diamètre, débit, températures, refroidissement, rétraction, débit volumétrique max, vitesse nominale | v0.1 | fait |
| PRF-03 | Profils Prusa MK4S et Core One | v0.1 | fait |
| PRF-09 | Profils Bambu Lab A1, P1, X1 et Marlin générique | v0.3 | prévu |
| PRF-04 | Bibliothèque de filaments : PLA, PETG (faits en v0.1), TPU, ASA, PETG-CF | v0.3 | en cours |
| PRF-05 | Cône de dégagement de la buse (angle, hauteur) | v0.3 | prévu |
| PRF-06 | Facteur d'étirement des ponts par filament | v0.2 | prévu |
| PRF-07 | Édition, duplication, import / export des profils (filaments et imprimantes) dans l'application ; origine de chaque valeur ; profils livrés jamais écrasés | v0.3 | prévu |
| PRF-08 | Plateau rond (delta), origine au centre | v0.3 | prévu |
| PRF-10 | Import de profils filament et imprimante depuis PrusaSlicer (`.ini`), OrcaSlicer et Bambu Studio (`.json`) ; rapport des valeurs reprises, ignorées, manquantes ; gabarits « à relire » | v0.3 | prévu |
| PRF-11 | Sélecteur de buse à côté de l'imprimante (diamètre courant ou libre, HF, abrasive) sans dupliquer le profil ; recalcul immédiat ; réglages par buse (première couche, débit max) ; mémorisé par imprimante | v0.3 | prévu |

## Catalogue de modèles (CAT)

Les ports de FullControl (GPL-3.0) citent leur source dans `[meta]`. L'ordre dans chaque jalon est indicatif.

| Id | Modèle | Origine | Ce qu'il teste | Jalon | État |
| --- | --- | --- | --- | --- | --- |
| CAT-01 | Vase à ondulations (texture *ripple*) | FullControl « Ripple Texture Demo » | Spirale continue, largeur variable | v0.1 | fait |
| CAT-23 | *Ajout* : option « Fond » des vases (aucun, 1 ou 2 couches pleines sous la spirale, API-13), d'abord sur CAT-01 | Fablab | Remplissage d'un contour, passage du fond à la spirale | v0.2 | prévu |
| CAT-02 | Tamis micro-maillage (ponts tendus) | Recherche FabWeave | Déplacements, rétractions, ponts | v0.2 | prévu |
| CAT-03 | Entretoise non planaire | FullControl « Nonplanar Spacer » | Z variable, plafond de vitesse Z | v0.2 | prévu |
| CAT-04 | Ressort hélicoïdal non planaire | Recherche FabWeave | Z continu, oscillation radiale | v0.2 | prévu |
| CAT-05 | Défi porte-à-faux | FullControl « Overhang Challenge » | Porte-à-faux extrêmes | v0.2 | prévu |
| CAT-06 | Défi support par picots | FullControl « Pin-Support Challenge » | Picots verticaux, ponts | v0.2 | prévu |
| CAT-07 | Abat-jour | FullControl « Lampshade » | Paroi fine texturée | v0.2 | prévu |
| CAT-08 | Porte-savon en serpentin | FullControl « Snake-Mode Soapdish » | Tracé continu en serpentin | v0.2 | prévu |
| CAT-09 | Treillis en polygones étoilés | FullControl « Star-Polygon Lattice » | Treillis, croisements | v0.2 | prévu |
| CAT-10 | Adaptateur hexagonal | FullControl « Hex Adapter » | Polygones, transitions | v0.2 | prévu |
| CAT-11 | Moteur de motifs polaires | FullControl « Fractional Design Engine (Polar) » | Motifs polaires paramétriques | v0.2 | prévu |
| CAT-12 | Impression par gouttes | FullControl « Blob Printing » | Extrusion à l'arrêt, points isolés | v0.2 | prévu |
| CAT-13 | Écrous et vis | FullControl « Nuts and Bolts » | Filetage | v0.3 | prévu |
| CAT-14 | Support de téléphone à angle libre | FullControl « AnyAngle Phone Stand » | Objet utilitaire complet | v0.3 | prévu |
| CAT-15 | Test des 2 000 rétractions | FullControl « 2000-Retractions Test » | Calibration de la rétraction | v0.3 | prévu |
| CAT-16 | Défi porte-à-faux plus | FullControl « Overhang Challenge Plus » | Porte-à-faux, variante | v0.3 | prévu |
| CAT-17 | Défi glaçage libre | FullControl « Freeform Frosting Challenge » | Dépôt libre en 3D | v0.4 | prévu |
| CAT-18 | Renfort par ruban | FullControl « Tape-Reinforcement Research Demo » | Pause et insertion (API-12) | v0.4 | prévu |
| CAT-19 | Panier tressé | Recherche FabWeave | Variations de vitesse et de débit, décalage d'un demi-pas | v0.3 | prévu |
| CAT-20 | Boîte à visser en mode vase | Recherche FabWeave | Extrusion large (W = 0,9 mm), filet sinusoïdal | v0.3 | prévu |
| CAT-21 | Charnière souple monolithique | Recherche FabWeave | Lames continues sans couture | v0.4 | prévu |
| CAT-22 | *Ajout* : tour de température et test de débit | Fablab | Calibration d'un nouveau filament | v0.3 | prévu |

## Visualisation (VIEW)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| VIEW-01 | Vue 3D WebGL : orbite, déplacement, zoom centré sur le curseur, cadrer, vues standard | v0.1 | fait |
| VIEW-02 | Plateau dessiné (taille, origine, grille) | v0.1 | fait |
| VIEW-03 | Rendu en lignes, couleur par ordre d'impression ; déplacements affichables | v0.1 | fait |
| VIEW-04 | Annotations 3D du modèle (API-07) | v0.2 | prévu |
| VIEW-05 | Vignettes du catalogue rendues par l'application | v0.2 | prévu |
| VIEW-06 | Rendu volumétrique W × H | v0.4 | prévu |
| VIEW-07 | Coloration : type, vitesse, débit, W, H, ventilateur, Z ; légende | v0.4 | prévu |
| VIEW-08 | Curseur de progression, simulation de la buse, coupe en Z | v0.4 | prévu |
| VIEW-09 | Inspection du segment survolé | v0.4 | prévu |
| VIEW-10 | Export PNG de la vue | v0.4 | prévu |

## Sécurité et qualité (SAFE)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| SAFE-01 | Export refusé si le script échoue ou si la pièce sort du volume | v0.1 | fait |
| SAFE-02 | Plafonds du profil jamais dépassés (débit, Z, températures) | v0.1 | fait |
| SAFE-03 | Garde-buse : carte de hauteur, collisions du cône surlignées | v0.5 | prévu |
| SAFE-04 | Segments trop courts / trop nombreux par seconde (planificateur) | v0.5 | prévu |
| SAFE-05 | Porte-à-faux, ponts trop longs, largeur incohérente avec la buse | v0.5 | prévu |
| SAFE-06 | Avertissement si la buse diffère de la buse recommandée par le modèle | v0.3 | prévu |

## Interface et fichiers (UI)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| UI-01 | Curseurs générés depuis l'en-tête, calcul en direct (le dernier gagne) | v0.1 | fait |
| UI-02 | Mesures et messages à droite, champ fautif souligné | v0.1 | fait |
| UI-03 | Export G-code nommé d'après le modèle, l'imprimante et le filament | v0.1 | fait |
| UI-04 | Reprise du dernier modèle et des valeurs, FR / EN, thèmes ; annuler / rétablir (même champ fusionné, geste = une étape) | v0.1 | fait |
| UI-05 | Catalogue : catégories, recherche, étiquettes, fiche du modèle | v0.2 | prévu |
| UI-06 | Ouvrir / glisser-déposer un `.weave` ; « Mes modèles » | v0.2 | prévu |
| UI-07 | Revenir aux valeurs par défaut du modèle : toutes, ou une par son ↺ | v0.1 | fait |
| UI-08 | Placement sur le plateau, copies multiples | v0.3 | prévu |
| UI-09 | Calcul au relâchement quand le premier calcul d'un geste dépasse 60 ms (barre d'état orange) | v0.1 | fait |
| UI-10 | Mesures et dimensions dans l'unité d'affichage (mm, pouces) | v0.1 | fait |

## Bureau, outils et projets (DESK)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| DESK-01 | `fabweave-cli check | export | stats` | v0.2 | prévu |
| DESK-02 | Rechargement à chaud de `models/` en développement | v0.2 | fait |
| DESK-03 | Application Tauri, dossier de modèles surveillé | v0.6 | prévu |
| DESK-04 | Projet `.fabweave.json`, projet embarqué dans le G-code | v0.6 | prévu |
| DESK-05 | Envoi à l'imprimante (PrusaLink, OctoPrint, Moonraker) | v0.6 | prévu |
| DESK-06 | Serveur MCP (stdio) pour un agent IA | v0.6 | prévu |

## Atelier de formes (PLAY)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| PLAY-01 | Éditeur de code intégré, évaluation en direct, erreurs en ligne | v2 | prévu |
| PLAY-02 | Complétion de l'API et aide intégrée | v2 | prévu |
| PLAY-03 | Éditeur d'équations ou de nœuds | v2 | à décider |
| PLAY-04 | Tracés importés (SVG → parcours), texte | v2 | à décider |

---

## Référence : fullcontrol.xyz (étudié le 24/10/2026)

- **Bibliothèque** : 16 modèles en grille de vignettes 3D (liste `CAT` ci-dessus), chacun avec une infobulle ⓘ.
- **Fiche d'un modèle** (exemple « Nonplanar Spacer ») : titre ; à gauche *Design Parameters* (3 curseurs : vagues, épaisseur, diamètre du trou) et *Advanced Design Parameters* (rapport de diamètres, épaisseur de matière, rapport d'aspect du boudin, recouvrement %, facteur de contraction des vagues, quantité) ; bouton **Regenerate Design** ; au centre une vue 3D en lignes avec des **annotations** (« Start », « End », conseils comme « A pointy nozzle is best ») ; à droite *Visual Controls* (annotations, couleur : ordre d'impression, gradient en Z, aléatoire ; épaisseur des lignes, recentrer la vue) et *Printer Parameters* (imprimante : Generic, Prusa i3, Ender 3, Ultimaker 2+, Bambu Lab X1, changeur d'outils E3D T0–T3, CR-10 ; température buse et plateau, ventilateur %, débit matière %, vitesse %) ; **Download GCode** ; lien vers le code Python du modèle.
- **Limites relevées** : recalcul manuel (bouton), profils réduits à un nom de machine et quatre pourcentages, aucune mesure (temps, matière, débit), aucun contrôle de sécurité affiché.
- **Bibliothèque Python** (`FullControlXYZ/fullcontrol`, GPL-3.0) : les modèles sont des notebooks (`models/*.ipynb`), utiles comme source pour les ports.
