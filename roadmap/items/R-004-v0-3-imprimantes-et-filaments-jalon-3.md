---
id: R-004
titre: "v0.3 — Imprimantes et filaments (jalon 3)"
resume: "Chaque machine du fablab a un profil fiable, et l'utilisateur peut créer le sien."
statut: a-tester
jalon: v0.3
cree: 2026-10-24
maj: 2026-11-12
livree:
---
*En cours dans l'ancienne feuille de route ; étapes de code toutes faites, validation de jalon partielle (MK4S validée le 28/10/2026).*

**But** : chaque machine du fablab a un profil fiable, et l'utilisateur peut créer le sien.

Étapes et points d'arrêt (machines du fablab : Prusa MK4S, Prusa CORE One, **Bambu Lab X1E** alimentée par la carte SD) :

1. **Bambu Lab X1E** : profil (gabarits de Bambu Studio réécrits), dialecte Bambu (en-tête lu par le firmware, couches et progression à l'écran), paquet `.gcode.3mf` (G-code, somme MD5, métadonnées de plaque, vignettes), export au choix `.gcode` ou `.gcode.3mf`.
   ⏸ Essai sur la X1E par la carte SD des deux formats ; choix consigné dans l'ADR-0005. *Essai reporté (non bloquant) : fait avec l'impression de validation de l'étape 6.*
2. **Profils complets et dialectes** : schéma de profil enrichi (zones exclues du plateau, accélérations, cône de la buse), migration et test de lecture de l'ancien format ; progression `M73` et vignette pour les Prusa ; Marlin générique, Klipper ; avance de pression ; ligne de purge hors pièce. *Fait le 27/10/2026 : profil imprimante v2 (v1 lu tel quel), estimation du temps avec accélérations (ENG-15, avancée de la v0.4), vignettes PNG et QOI dans le G-code, profils Marlin et Klipper génériques, purges en zones exclues ; valeurs du cône à mesurer sur les machines.*
3. **Buse, placement, adhésion** : changer de buse en un clic, position et rotation sur le plateau, copies, bordure et radeau. *Fait le 27/10/2026 : buse mémorisée par imprimante (couches et débit par buse), placement, copies pièce par pièce, bordure et radeau générés par le moteur, plateau rond, pièce hors plateau surlignée.*
4. **Édition et import des profils** dans l'application ; import depuis PrusaSlicer, OrcaSlicer et Bambu Studio. *Fait le 27/10/2026 : éditeur (copie d'un profil livré, origine de chaque valeur, contrôle en direct), profils de l'utilisateur sur le poste, export TOML, import avec rapport et gabarits traduits, relecture exigée avant l'export (ADR-0006, acceptée). Validé le 28/10/2026.*
5. **Reports de la v0.2** : remplissage en anneaux concentriques (API-14), extrusion à l'arrêt (API-15), impression par gouttes (CAT-12). *Fait le 28/10/2026 : `fill_rings` (décalages de `i_overlay`), option « Anneaux » du fond du vase à ondulations, `path.blob`, modèle « Tube de gouttes ».*
6. ⏸ Validation de jalon : une impression validée par machine du fablab. *MK4S validée le 28/10/2026 (plusieurs modèles, vignette à l'écran) ; restent la CORE One et la X1E.*

Étapes ajoutées le 28/10/2026 pour terminer la v0.3, menées pendant les impressions de l'étape 6 :

7. **Filaments** : TPU, ASA, PETG-CF (PRF-04), préchauffe de l'enceinte de la CORE One, facteur des ponts par filament (PRF-06). *Fait le 28/10/2026.*
8. **Bambu Lab A1 et P1S** (PRF-09) : profils et gabarits de Bambu Studio réécrits. *Fait le 28/10/2026 : gabarits traduits par l'import (nouvelles correspondances des variables de purge Bambu), non essayés sur machine.*
9. **Essuyage et pause** (API-12). *Fait le 28/10/2026 : `path.wipe`, `path.pause`, gabarit de pause dans les sept profils et à l'import.*
10. **Catalogue** : tour de température et test de débit (CAT-22), 2 000 rétractions (CAT-15), défi porte-à-faux plus (CAT-16), écrous et vis (CAT-13), support de téléphone (CAT-14), panier tressé (CAT-19), boîte à visser (CAT-20). *Fait le 28/10/2026 : les cinq derniers, puis, après le choix de l'utilisateur, réglages d'essai des modèles d'étalonnage (ADR-0007) : tour de température, test de débit et test des 2 000 rétractions, nouvelle rubrique « Étalonnage ».*

- **Profils versionnés** (TOML) : plateau (taille, forme rectangulaire ou ronde, origine), buse, cinématique (vitesses max, vitesse Z max), cône de dégagement de la buse, dialecte du firmware, gabarits de démarrage et de fin ; filaments : températures (première couche, suivantes, plateau), refroidissement, rétraction, débit volumétrique max, vitesse nominale, facteur d'étirement des ponts.
- **Bibliothèque** : Prusa MK4S et Core One (livrées en v0.1), **Bambu Lab A1, P1 et X1**, plus « Marlin générique ».
- **Dialectes** : Prusa (Buddy), Bambu Lab, Marlin, Klipper ; avance de pression (`M572`, `M900`, `SET_PRESSURE_ADVANCE`), contrôles de modèle et de buse (`M862.x`), ligne de purge hors pièce, vignette intégrée au G-code (écran des Prusa).
- **Bambu Lab** (étude en début de jalon) : G-code brut lancé depuis la carte SD, ou paquet `.gcode.3mf` (G-code de plaque, sommes MD5, métadonnées de plaque et de filament, vignette) comme le fait l'outil `bambox`, que le firmware contrôle avant d'imprimer. Choix consigné dans une ADR.
- **Édition dans l'application** : dupliquer un profil, modifier, importer / exporter, mémorisé par poste ; chaque valeur dit son origine (profil livré, modifié, importé). Les profils livrés ne sont jamais écrasés : une modification crée une copie.
  - **Filaments** : modifier chaque paramètre (températures, refroidissement, rétraction, débit volumétrique max, vitesse nominale, multiplicateur de débit, densité, facteur des ponts), créer un filament à partir d'un autre, importer un fichier de filament.
  - **Imprimantes** : importer un profil d'imprimante (plateau, buse, vitesses, gabarits de démarrage et de fin), le vérifier et le corriger avant usage.
  - **Import depuis les trancheurs** : profils filament et imprimante de PrusaSlicer (`.ini`), OrcaSlicer et Bambu Studio (`.json`) convertis en TOML FabWeave ; les valeurs reprises, ignorées ou manquantes sont listées, et les gabarits de G-code sont signalés « à relire » (leurs variables diffèrent des nôtres).
  - Valeurs vérifiées à l'import comme à l'édition : hors domaine → erreur ou avertissement à code stable, jamais corrigées en silence.
- **API** : remplissage en anneaux concentriques (API-14), extrusion à l'arrêt (API-15) et le modèle d'impression par gouttes (CAT-12).
- **Placement** : position et rotation sur le plateau, copies multiples en grille, plateau dessiné à sa forme réelle, dépassement surligné et bloquant.
- **Changer de buse en un clic** : sélecteur du diamètre de buse à côté de l'imprimante (0,25 · 0,4 · 0,5 · 0,6 · 0,8 mm, ou valeur libre), sans dupliquer le profil ; option buse haut débit (HF) et buse abrasive quand l'imprimante les propose. Le parcours se recalcule aussitôt (les largeurs et hauteurs des modèles sont en multiples de `printer.nozzle_diameter`) ; le diamètre est transmis aux gabarits (contrôle `M862.1` des Prusa) ; le profil peut ajuster par buse la hauteur de première couche, et le filament le débit volumétrique max (buse HF). Choix mémorisé par imprimante.
- Diamètre de buse : avertissement si le modèle recommande une autre buse.
- **Adhésion au plateau** : bordure (*brim*) ou radeau (*raft*) ajoutés par le moteur, au choix de l'utilisateur et non du modèle (le `.weave` reste muet là-dessus). Bordure : boucles décalées autour de l'empreinte de la première couche (nombre de tours ou largeur, écart à la pièce) ; radeau : couches de base et d'interface sous la pièce, qui la rehausse d'autant, avec écart en Z pour le décollage. Emprise et hauteur ajoutées prises en compte dans le contrôle du volume d'impression ; le modèle peut seulement recommander une adhésion (`[meta]`).

Critères d'acceptation : G-code de chaque profil livré relu (démarrage, fin, unités, températures) ; ⏸ une impression validée par machine du fablab.
