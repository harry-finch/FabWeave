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
| FMT-09 | Spécification `docs/WEAVE_FORMAT.md` et prompt système pour IA `docs/PROMPT_MODELE_IA.md` | v0.1 / v0.2 | fait |
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
| API-07 | `path.note(texte)`, `path.note_at(x, y, z, texte)` : annotations, affichées dans la vue 3D (VIEW-04) | v0.2 | fait |
| API-08 | Listes de points natives : `line`, `arc`, `circle`, `polygon`, `rectangle`, `zigzag`, `sine`, `spiral`, `helix` | v0.2 | fait |
| API-09 | Transformations natives : translation, rotation, miroir, répétition en Z, copie tournée, inversion de sens | v0.2 | fait |
| API-10 | `pts.modulate_z(f)`, `pts.modulate_r(cx, cy, f)` : déformation par une fonction du script | v0.2 | fait |
| API-11 | Subdivision (`pts.subdivide`, faite en v0.2) ; discrétisation adaptative (tolérance de corde) | v0.2 / v0.5 | en cours |
| API-12 | Essuyage (`path.wipe`, rétraction répartie sur le retour), pause (`path.pause(texte)`, gabarit `pause` du profil, répétée par copie) : insertion d'objet, ruban de renfort | v0.3 | fait |
| API-13 | Remplissage natif d'un contour fermé en lignes parallèles reliées en zigzag (`fill_lines`, contour quelconque, retrait au bord) : fonds pleins, grilles | v0.2 | fait |
| API-14 | Remplissage en anneaux concentriques (`fill_rings`, décalage d'un contour vers l'intérieur, îlots, coutures alignées) | v0.3 | fait |
| API-15 | Extrusion à l'arrêt (`path.blob(volume, durée)` : goutte sans mouvement, débit plafonné), pour CAT-12 | v0.3 | fait |
| API-16 | `pts.slice(from, to)` ; `floor`, `round`, `ceiling` acceptent un entier | v0.2 | fait |
| API-18 | Réglages d'essai des modèles d'étalonnage (`[meta] calibration = true`) : décalage de température ±30 °C, longueur et vitesse de rétraction, saut en Z, bornés, signalés (ADR-0007) | v0.3 | fait |
| API-17 | Bruit de gradient 3D natif `noise(x, y, z)` (−1 à 1, motifs de taille ≈ 1, déterministe, sans couture sur un cylindre pris en 3D) : textures aléatoires continues | v0.3 | fait |

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
| ENG-13 | Dialectes : Marlin, Prusa Buddy, Klipper (accélérations `M204 P T` ou `M204 S`, couches `;LAYER_CHANGE` ou `SET_PRINT_STATS_INFO`, progression `M73 P R` pour tous) ; avance de pression par les gabarits (`M572`, `M900`, `SET_PRESSURE_ADVANCE`) | v0.3 | fait |
| ENG-14 | Vignettes intégrées au G-code (PNG / QOI, tailles du profil, format de PrusaSlicer ; bloc `THUMBNAIL_BLOCK` pour Bambu Lab) | v0.3 | fait |
| ENG-15 | Estimation du temps avec accélérations (trapèzes de vitesse, angles limités par le *jerk*, passes avant et arrière) ; avancée de la v0.4 pour la progression `M73` | v0.3 | fait |
| ENG-20 | Zones exclues du plateau (purge, pinces) : pièce refusée si un mouvement imprimé y passe (`path.excluded_zone`), zones hachurées sur le plateau | v0.3 | fait |
| ENG-16 | Mesures : nombre de points et de segments, longueur imprimée, déplacements, temps, filament, débit max, Z max, encombrement | v0.1 | fait |
| ENG-17 | Bambu Lab : paquet `.gcode.3mf` (G-code, MD5, `slice_info.config`, `model_settings.config`, `plate_1.json`, vignettes PNG) et G-code brut, tous deux pour la carte SD (ADR-0005, proposée) | v0.3 | fait, à valider sur la X1E |
| ENG-19 | Dialecte Bambu : en-tête `HEADER_BLOCK`, `M73 L` + `M991` à chaque couche (paliers de hauteur pour les parcours continus), progression `M73 P R` | v0.3 | fait, à valider sur la X1E |
| ENG-18 | Adhésion au plateau générée par le moteur : bordure (*brim*) autour de l'empreinte de la première couche (premier tour entier d'un vase en spirale ; extérieur seulement, boucles de la plus éloignée à la plus proche), ou radeau (*raft*) sous la pièce (base et contact pleins, intermédiaires espacés, écart de décollage) ; empreinte en grille et carte des distances, contours par carrés marchants ; emprise incluse dans le contrôle du volume ; le modèle peut seulement conseiller (`recommended_adhesion`) | v0.3 | fait |
| ENG-21 | Préparation de l'impression hors du modèle (`Setup`) : position, rotation, copies en grille pièce par pièce (écart ≥ rayon de dégagement de la tête, pièce ≤ hauteur libre sous le portique), rappelée en tête du G-code ; pièce hors plateau montrée, segments en cause surlignés, export refusé | v0.3 | fait |

## Profils imprimante et filament (PRF)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| PRF-01 | Profil imprimante TOML : plateau, origine, buse, vitesses max, vitesse Z max, gabarits | v0.1 | fait |
| PRF-02 | Profil filament TOML : diamètre, débit, températures, refroidissement, rétraction, débit volumétrique max, vitesse nominale | v0.1 | fait |
| PRF-03 | Profils Prusa MK4S et Core One | v0.1 | fait |
| PRF-09 | Profils Bambu Lab A1 et P1S (gabarits de Bambu Studio traduits par l'import, sans morceau laissé de côté ; identifiants N2S et C12 du paquet) ; X1E : PRF-12 ; P1P, X1 Carbon, A1 mini non livrés (importables) | v0.3 | fait, non essayé sur machine |
| PRF-13 | Profil imprimante v2 : zones exclues, accélérations, cône, vignettes, nom par langue, dialecte contrôlé ; profil v1 lu sans changement (test sur le profil MK4S de la v0.2) | v0.3 | fait |
| PRF-14 | Profils Marlin générique (220 × 220) et Klipper générique (235 × 235) : purge le long du bord gauche en zone exclue, avance de pression si le filament la donne | v0.3 | fait |
| PRF-12 | Profil Bambu Lab X1E (gabarits de Bambu Studio 02.08 réécrits ; scan lidar retiré ; plaque `plate`) ; variables de gabarit `bed_type`, `max_volumetric_speed`, fonction `num()` | v0.3 | fait, à valider sur la X1E |
| PRF-04 | Bibliothèque de filaments : PLA, PETG (faits en v0.1), TPU, ASA, PETG-CF (abrasif) ; enceinte minimale attendue avant d'imprimer (`chamber_min`, préchauffe de la CORE One) | v0.3 | fait |
| PRF-05 | Cône de dégagement de la buse (angle, hauteur) : champs du profil, transmis au script (`printer.nozzle_cone_angle`, `_height`) ; valeurs des machines du fablab à mesurer | v0.3 | en cours |
| PRF-06 | Facteur d'étirement des ponts par filament (`bridge_flow`, multiplie le débit des ponts du modèle) | v0.3 | fait |
| PRF-07 | Édition, duplication, import / export des profils (filaments et imprimantes) dans l'application ; origine de chaque valeur ; profils livrés jamais écrasés | v0.3 | fait |
| PRF-08 | Plateau rond (`shape = "circle"`, delta), origine au centre ; dépassement radial ; dessiné rond | v0.3 | fait |
| PRF-10 | Import de profils filament et imprimante depuis PrusaSlicer (`.ini`), OrcaSlicer et Bambu Studio (`.json`) ; rapport des valeurs reprises, ignorées, manquantes ; gabarits traduits et « à relire » (export bloqué avant relecture) | v0.3 | fait, essayé sur des préréglages officiels reconstitués |
| PRF-11 | Sélecteur de buse à côté de l'imprimante (diamètres du profil ou libre, HF, abrasive) sans dupliquer le profil ; recalcul immédiat ; couches par buse (`layers.by_nozzle`, interpolées pour un diamètre libre et signalé), débit max par buse (`speed.by_nozzle` du filament) ; mémorisé par imprimante ; filament abrasif dans une buse qui ne l'est pas signalé | v0.3 | fait |

## Catalogue de modèles (CAT)

Les ports de FullControl (GPL-3.0) citent leur source dans `[meta]`. L'ordre dans chaque jalon est indicatif.

| Id | Modèle | Origine | Ce qu'il teste | Jalon | État |
| --- | --- | --- | --- | --- | --- |
| CAT-01 | Vase à ondulations (texture *ripple*) | FullControl « Ripple Texture Demo » | Spirale continue, largeur variable | v0.1 | fait |
| CAT-23 | *Ajout* : option « Fond » des vases (aucun, 1 ou 2 couches pleines sous la spirale, en lignes API-13 ou en anneaux API-14, au contour sans texture pour ne pas dépasser de la paroi), d'abord sur CAT-01, puis CAT-24 et CAT-25 | Fablab | Remplissage d'un contour, passage du fond à la spirale | v0.2 | fait |
| CAT-02 | Tamis micro-maillage (ponts tendus) | Recherche FabWeave | Déplacements, rétractions, ponts | v0.2 | fait |
| CAT-03 | Entretoise non planaire | FullControl « Nonplanar Spacer » | Z variable, plafond de vitesse Z | v0.2 | fait |
| CAT-04 | Ressort hélicoïdal non planaire : ressort-soufflet en spirale, plis en dents de scie, en anneaux ou en hélice | Recherche FabWeave | Z continu, oscillation radiale | v0.2 | fait |
| CAT-05 | Défi porte-à-faux | *d'après* FullControl « Overhang Challenge » (principe des arcs ; source non publiée) | Porte-à-faux extrêmes | v0.2 | fait |
| CAT-06 | Défi support par picots | *d'après* FullControl « Pin-Support Challenge » (source non publiée) | Picots verticaux, ponts | v0.2 | fait |
| CAT-07 | Abat-jour | FullControl « Lampshade » | Paroi fine texturée | v0.2 | fait |
| CAT-08 | Porte-savon en serpentin | *d'après* FullControl « Snake-Mode Soapdish » (source non publiée) | Tracé continu en serpentin | v0.2 | fait |
| CAT-09 | Treillis en polygones étoilés (sous-famille M1 ; M2 à M4 plus tard) | FullControl « Star-Polygon Lattice » | Treillis, croisements | v0.2 | fait |
| CAT-10 | Adaptateur hexagonal | FullControl « Hex Adapter » | Polygones, transitions | v0.2 | fait |
| CAT-11 | Moteur de motifs polaires (listes de l'original remplacées par branches, saut, rayon intérieur) | FullControl « Fractional Design Engine (Polar) » | Motifs polaires paramétriques | v0.2 | fait |
| CAT-12 | Impression par gouttes (tube de gouttes) | FullControl « Blob Printing » | Extrusion à l'arrêt, points isolés | v0.3 | fait |
| CAT-13 | Écrous et vis (vis et écrou à ailettes, tube fileté, M6 à M12, parois multiples) | FullControl « Nuts and Bolts » | Filetage en spirale continue, ailettes de Bézier | v0.3 | fait |
| CAT-14 | Support de téléphone multi-angle (mode fâché compris) | FullControl « AnyAngle Phone Stand » | Objet utilitaire complet | v0.3 | fait |
| CAT-15 | Test des 2 000 rétractions (longueur, vitesse ou saut en Z, cinq colonnes) | *d'après* FullControl « 2000-Retractions Test » (source non publiée) | Calibration de la rétraction (réglages d'essai, API-18) | v0.3 | fait |
| CAT-16 | Défi porte-à-faux plus (cercle, hexagone, carré, triangle ; 45 à 135° ; vers l'intérieur) | *d'après* FullControl « Overhang Challenge Plus » (source non publiée) | Porte-à-faux, variante | v0.3 | fait |
| CAT-17 | Défi glaçage libre | FullControl « Freeform Frosting Challenge » | Dépôt libre en 3D | v0.4 | prévu |
| CAT-18 | Renfort par ruban | FullControl « Tape-Reinforcement Research Demo » | Pause et insertion (API-12) | v0.4 | prévu |
| CAT-19 | Panier tressé | Recherche FabWeave | Variations de vitesse et de débit, décalage d'un demi-pas | v0.3 | fait |
| CAT-20 | Boîte à visser en mode vase (boîte et couvercle) | Recherche FabWeave | Extrusion large (W = 0,9 mm), filet sinusoïdal | v0.3 | fait |
| CAT-21 | Charnière souple monolithique | Recherche FabWeave | Lames continues sans couture | v0.4 | prévu |
| CAT-22 | *Ajout* : tour de température (réglages d'essai, API-18) et test de débit | Fablab | Calibration d'un nouveau filament | v0.3 | fait |
| CAT-24 | *Ajout* : photophore vermiculé (relief en vermicelles tiré d'un bruit, fond en anneaux) | Photo de l'utilisateur (28/10/2026) | Bruit 3D (API-17), paroi en spirale | v0.3 | fait |
| CAT-26 | *Ajout* : photophore à vermicelles (paroi fine en spirale d'où la buse sort au hasard en petites boucles, qui retombent dans le vide) | Photo de l'utilisateur (28/10/2026), seconde lecture | Excursions aléatoires reproductibles, affaissement voulu | v0.3 | fait |
| CAT-25 | *Ajout* : vase à écailles (polygone aux faces creusées, taille resserrée, écailles en quinconce) | Photo de l'utilisateur (28/10/2026) ; texture de FullControl « ripple texture » | Points à pas égal le long du contour, sortie selon la normale | v0.3 | fait |

## Visualisation (VIEW)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| VIEW-01 | Vue 3D WebGL : orbite, déplacement, zoom centré sur le curseur, cadrer, vues standard | v0.1 | fait |
| VIEW-02 | Plateau dessiné (taille, origine, grille) | v0.1 | fait |
| VIEW-03 | Rendu en lignes, couleur par ordre d'impression ; déplacements affichables | v0.1 | fait |
| VIEW-04 | Annotations 3D du modèle (API-07), masquables (choix mémorisé), dans la langue de l'interface | v0.2 | fait |
| VIEW-05 | Vignettes du catalogue rendues par l'application (parcours aux valeurs par défaut, vue oblique, couleurs du thème ; échec du script marqué) | v0.2 | fait |
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
| SAFE-06 | Avertissement si la buse montée diffère de la buse recommandée par le modèle | v0.3 | fait |

## Interface et fichiers (UI)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| UI-01 | Curseurs générés depuis l'en-tête, calcul en direct (le dernier gagne) | v0.1 | fait |
| UI-02 | Mesures et messages à droite, champ fautif souligné | v0.1 | fait |
| UI-03 | Export G-code nommé d'après le modèle, l'imprimante et le filament ; menu d'une ligne par format quand l'imprimante en a plusieurs (Bambu Lab : `.gcode.3mf` puis `.gcode`) | v0.1 / v0.3 | fait |
| UI-04 | Reprise du dernier modèle et des valeurs, FR / EN, thèmes ; annuler / rétablir (même champ fusionné, geste = une étape) | v0.1 | fait |
| UI-05 | Catalogue : catégories, recherche (nom, description, catégorie, étiquettes, sans accents), étiquettes, fiche du modèle (buses conseillées signalées si la buse diffère, auteur, source, licence) | v0.2 | fait |
| UI-06 | Ouvrir (`⌘O`) / glisser-déposer un `.weave` ; « Mes modèles » mémorisés sur le poste, retirables (« Annuler » dans le message) ; erreurs du script avec leur ligne | v0.2 | fait |
| UI-07 | Revenir aux valeurs par défaut du modèle : toutes, ou une par son ↺ | v0.1 | fait |
| UI-08 | Section « Plateau » : position, rotation, copies, écart, adhésion (conseil du modèle rappelé) ; buse sous l'imprimante ; segments hors plateau en rouge dans la vue | v0.3 | fait |
| UI-09 | Calcul au relâchement quand le premier calcul d'un geste dépasse 60 ms (barre d'état orange) | v0.1 | fait |
| UI-10 | Mesures et dimensions dans l'unité d'affichage (mm, pouces) | v0.1 | fait |

## Bureau, outils et projets (DESK)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| DESK-01 | `fabweave check | stats | export` (calcul aux valeurs par défaut et aux bornes, `--json` ; aux bornes, pièce hors plateau et `fail` du modèle tolérés ; `--set` garde en texte la valeur d'un `choice`) | v0.2 | fait |
| DESK-02 | Rechargement à chaud de `models/` et `profiles/` en développement (fichier modifié ou ajouté, même dans un nouveau dossier) | v0.2 | fait |
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
