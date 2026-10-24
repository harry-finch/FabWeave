# Feuille de route

**But de la v1.0** : une application de bureau (et une version web sans installation) qui reprend le principe de FullControl pour les makers et les fablabs : un **catalogue de modèles paramétriques** dont on règle les curseurs, un **aperçu 3D en direct** du parcours de la buse, et un **G-code direct** adapté à l'imprimante et au filament choisis, sûr pour la machine. Les modèles sont des fichiers `.weave` (en-tête TOML + script Rhai) faciles à écrire à la main ou à faire générer par une IA.

Ce que FabWeave apporte par rapport à FullControl (site et bibliothèque Python) :

- **calcul immédiat** : pas de bouton « Regenerate », le parcours suit le curseur ;
- **profils imprimante et filament séparés du modèle** : G-code de démarrage et de fin, dialecte du firmware, plafonds de débit et de vitesse Z, rétraction ; le même modèle passe d'une Ender 3 en PLA à une Core One en PETG sans retoucher le modèle ;
- **un moteur honnête** : chaque plafonnement, fusion ou collision est compté et signalé ;
- **un format de modèle déclaratif** qu'une IA peut produire sans erreur, vérifiable hors de l'application (CLI).

**Principes** (détail dans [AGENTS.md](AGENTS.md)) :

- moteur Rust pur et déterministe (`fabweave-core`), testé, sans IO ni interface ;
- même écosystème que FabSpeed et FabVents : Svelte 5, skill `fab-design-system`, français et anglais, thème sombre de référence et thème clair, Tauri pour le bureau ;
- les valeurs ne sont jamais corrigées en silence : le moteur signale, l'interface montre ;
- millimètres en interne ; extrusion relative ;
- un jalon = une version testable, livrée avec sa liste de vérifications, **imprimée à l'atelier** avant d'être validée.

La source fonctionnelle est l'inventaire de [docs/FEATURES.md](docs/FEATURES.md) (chaque fonction y a son identifiant et son jalon). Les points d'arrêt **⏸** sont les moments où le travail s'interrompt pour une validation par l'utilisateur.

---

## v0.0 — Fondations · *validée* (24/10/2026)

Dépôt Git, consignes, feuille de route, inventaire des fonctions, décisions d'architecture (ADR-0001 à 0003).

## v0.1 — Tranche verticale (jalon 1) · *faite, en validation de jalon* (étapes 1, 2 et logo validés le 24/10/2026 ; vase imprimé sur la CORE One)

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

## v0.2 — Catalogue et format complet (jalon 2)

**But** : le catalogue devient le cœur de l'application, et le format `.weave` est assez riche et assez documenté pour qu'une IA externe produise des modèles valides du premier coup.

- **Catalogue** : arbre par catégories, recherche, étiquettes, vignettes rendues par l'application, fiche du modèle (description, auteur, source et licence, buse recommandée, temps et matière estimés avec les profils courants).
- **Format v1 complet** : paramètres `choice` (liste), unités, pas, groupes et `advanced = true` (« ▸ Avancé »), libellés et infobulles FR / EN, avertissements et erreurs émis par le script avec un code (`warn("model.thin_wall", …)`), annotations 3D (`note(x, y, z, "texte")`, comme les repères « Start », « End » de FullControl), points de départ et de fin.
- **API haut niveau en Rust natif** (rapide, et moins d'erreurs pour une IA) : segments, arcs, cercles, polygones, spirales et hélices, ondes polaires, zigzags, copies (translation, rotation, miroir, répétition en Z), modulation en Z d'un tracé plat, subdivision, `bridge_to`, essuyage.
- **Modèles de l'utilisateur** : ouvrir ou glisser-déposer un `.weave`, « Mes modèles » mémorisés localement, erreurs du script montrées avec la ligne ; rechargement à chaud du dossier `models/` en développement.
- **Pour l'IA** : `docs/WEAVE_FORMAT.md` (spécification), `docs/PROMPT_MODELE_IA.md` (prompt système prêt à copier, avec deux exemples complets) et `fabweave-cli check | export | stats` pour qu'un agent vérifie son fichier sans ouvrir l'application.
- **Fond des vases** : option « Fond » (aucun, 1 ou 2 couches) sur les vases du catalogue, à commencer par le vase à ondulations : couches pleines qui épousent le contour de la base (branches, ondulations comprises), puis la spirale démarre au-dessus sans couture. Repose sur une primitive native de **remplissage d'un contour fermé** (lignes en zigzag croisées d'une couche à l'autre, ou anneaux concentriques), réutilisable par les autres modèles.
- **Catalogue de départ** d'environ dix modèles (liste `CAT` de [FEATURES.md](docs/FEATURES.md)), dont le tamis à ponts tendus, l'entretoise et le ressort non planaires ; ports de FullControl cités avec leur source.

Critères d'acceptation :

- Chaque modèle du catalogue a un test moteur : déterminisme, pièce dans le plateau, aucun message bloquant aux valeurs par défaut, aucun plantage aux bornes de chaque curseur.
- Trois modèles nouveaux générés par une IA à partir du seul prompt passent `fabweave-cli check` au premier ou au deuxième essai.
- ⏸ Validation de jalon : trois modèles du catalogue imprimés.

## v0.3 — Imprimantes et filaments (jalon 3)

**But** : chaque machine du fablab a un profil fiable, et l'utilisateur peut créer le sien.

- **Profils versionnés** (TOML) : plateau (taille, forme rectangulaire ou ronde, origine), buse, cinématique (vitesses max, vitesse Z max), cône de dégagement de la buse, dialecte du firmware, gabarits de démarrage et de fin ; filaments : températures (première couche, suivantes, plateau), refroidissement, rétraction, débit volumétrique max, vitesse nominale, facteur d'étirement des ponts.
- **Bibliothèque** : Prusa MK4S et Core One (livrées en v0.1), **Bambu Lab A1, P1 et X1**, plus « Marlin générique ».
- **Dialectes** : Prusa (Buddy), Bambu Lab, Marlin, Klipper ; avance de pression (`M572`, `M900`, `SET_PRESSURE_ADVANCE`), contrôles de modèle et de buse (`M862.x`), ligne de purge hors pièce, vignette intégrée au G-code (écran des Prusa).
- **Bambu Lab** (étude en début de jalon) : G-code brut lancé depuis la carte SD, ou paquet `.gcode.3mf` (G-code de plaque, sommes MD5, métadonnées de plaque et de filament, vignette) comme le fait l'outil `bambox`, que le firmware contrôle avant d'imprimer. Choix consigné dans une ADR.
- **Édition dans l'application** : dupliquer un profil, modifier, importer / exporter, mémorisé par poste ; chaque valeur dit son origine (profil livré, modifié, importé). Les profils livrés ne sont jamais écrasés : une modification crée une copie.
  - **Filaments** : modifier chaque paramètre (températures, refroidissement, rétraction, débit volumétrique max, vitesse nominale, multiplicateur de débit, densité, facteur des ponts), créer un filament à partir d'un autre, importer un fichier de filament.
  - **Imprimantes** : importer un profil d'imprimante (plateau, buse, vitesses, gabarits de démarrage et de fin), le vérifier et le corriger avant usage.
  - **Import depuis les trancheurs** : profils filament et imprimante de PrusaSlicer (`.ini`), OrcaSlicer et Bambu Studio (`.json`) convertis en TOML FabWeave ; les valeurs reprises, ignorées ou manquantes sont listées, et les gabarits de G-code sont signalés « à relire » (leurs variables diffèrent des nôtres).
  - Valeurs vérifiées à l'import comme à l'édition : hors domaine → erreur ou avertissement à code stable, jamais corrigées en silence.
- **Placement** : position et rotation sur le plateau, copies multiples en grille, plateau dessiné à sa forme réelle, dépassement surligné et bloquant.
- Diamètre de buse : avertissement si le modèle recommande une autre buse.
- **Adhésion au plateau** : bordure (*brim*) ou radeau (*raft*) ajoutés par le moteur, au choix de l'utilisateur et non du modèle (le `.weave` reste muet là-dessus). Bordure : boucles décalées autour de l'empreinte de la première couche (nombre de tours ou largeur, écart à la pièce) ; radeau : couches de base et d'interface sous la pièce, qui la rehausse d'autant, avec écart en Z pour le décollage. Emprise et hauteur ajoutées prises en compte dans le contrôle du volume d'impression ; le modèle peut seulement recommander une adhésion (`[meta]`).

Critères d'acceptation : G-code de chaque profil livré relu (démarrage, fin, unités, températures) ; ⏸ une impression validée par machine du fablab.

## v0.4 — Visualisation et analyse (jalon 4)

- **Rendu volumétrique** : boudins à section oblongue W × H, éclairage simple ; mode lignes pour les très grands parcours.
- **Coloration** au choix : type de mouvement (impression, pont, déplacement), vitesse obtenue, débit volumétrique, largeur W, hauteur H, ventilateur, Z, ordre d'impression ; légende graduée.
- **Curseur de progression** (couche, pourcentage, temps), simulation de la buse, déplacements affichables, coupe en Z.
- **Inspection** du segment survolé : W, H, vitesse demandée et obtenue, débit, raison d'un plafonnement.
- Estimation du temps avec accélérations (planificateur trapézoïdal simplifié).
- Export PNG de la vue.

Critères d'acceptation : 200 000 segments affichés en volumétrique à 60 images/s sur un ordinateur d'atelier ; temps estimé à ± 5 % de PrusaSlicer sur le catalogue.

## v0.5 — Sécurité et qualité du parcours (jalon 5)

- **Garde-buse** : carte de hauteur de la matière déposée, détection des collisions du cône de la buse et du carter en non planaire ; segments fautifs surlignés, comptés, export bloqué (ADR à venir sur la gravité).
- **Discrétisation adaptative** (tolérance de corde) dans l'API haut niveau ; fusion des points alignés ; longueur minimale de segment et nombre de segments par seconde maximal (saturation du planificateur, « bégaiement »), signalés.
- **Contrôles** : porte-à-faux au-delà d'un angle, ponts trop longs pour le filament, largeur incohérente avec la buse, première couche trop fine ou trop rapide.

Critères d'acceptation : cas de collision connus détectés (tests) ; aucun faux positif sur le catalogue.

## v0.6 — Bureau et projets (jalon 6)

- Application **Tauri 2** (macOS, Windows, Linux) : **dossier de modèles surveillé** (un `.weave` déposé ou modifié apparaît aussitôt), fenêtres natives, fichiers récents.
- **Projet** `.fabweave.json` (modèle embarqué, valeurs, profils) ; projet embarqué en commentaire dans le G-code (glisser un G-code FabWeave le rouvre).
- **Envoi à l'imprimante** (optionnel) : PrusaLink, OctoPrint, Moonraker.
- **CLI complète** et **serveur MCP** (stdio) : un agent IA crée, vérifie, prévisualise (PNG) et exporte un modèle.

## v1.0 — Publication

- Installeurs signés, version web statique, documentation utilisateur FR / EN (dont « écrire un modèle »), tests de bout en bout de l'interface.
- Catalogue d'au moins 15 modèles, chacun **imprimé au fablab** avec ses réglages validés (fiche de validation, photo).

## v2 — Atelier de formes (après la v1.0)

L'espace pour jouer et prototyper de nouvelles formes, sur le même format `.weave` :

- éditeur de code intégré (CodeMirror 6 : coloration Rhai, complétion de l'API, erreurs en ligne), évaluation en direct, curseurs déclarés au fil de l'écriture, « Enregistrer dans Mes modèles » ;
- à évaluer : éditeur d'équations ou de nœuds, tracés importés (SVG → parcours), texte, multi-outils (changeur d'outils, MMU), enchaînement de modèles.

## Hors périmètre

- Découpage de modèles STL (c'est le métier de PrusaSlicer, Orca, Cura) et supports automatiques.
- Comptes, nuage, place de marché.
- Envoi réseau vers les Bambu Lab (MQTT, compte Bambu) et gestion de l'AMS (multi-couleurs).
