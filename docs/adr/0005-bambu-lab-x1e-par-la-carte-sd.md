# ADR-0005 : Bambu Lab X1E par la carte SD

Status: Proposed (en attente de l'essai sur la X1E du fablab)
Date: 2026-10-27

## Context

Le fablab imprime sur une **Bambu Lab X1E** en passant par la carte SD (ni réseau, ni Bambu Studio). Deux formats sont possibles :

- le **G-code brut** (`.gcode`) ;
- le **paquet `.gcode.3mf`** que produit Bambu Studio (« Exporter le fichier tranché de la plaque ») : une archive ZIP au format 3MF qui contient le G-code de la plaque (`Metadata/plate_1.gcode`), sa somme MD5 (`plate_1.gcode.md5`), les métadonnées de la plaque (`slice_info.config` : modèle d'imprimante, buse, durée, poids, filament), `model_settings.config`, `plate_1.json` et des vignettes PNG.

Ce que l'on sait :

- Bambu Studio et les retours d'utilisateurs : le paquet `.gcode.3mf` exporté s'imprime depuis la carte SD.
- L'outil `bambox` (estampo, MIT), qui produit ces paquets sans Bambu Studio, indique que le firmware contrôle la somme MD5, et ajoute un `project_settings.config` de 544 clés que le firmware « valide » ; il n'est éprouvé que sur une P1S, en impression réseau (Bambu Connect).
- Le firmware lit dans le G-code un en-tête `; HEADER_BLOCK_START … ; HEADER_BLOCK_END` (durée, nombre de couches) et, à chaque couche, `M73 L` et `M991` (compteur de couches à l'écran, time-lapse) ; sans eux, l'écran affiche 0/0 couche.
- Aucune source ne dit si l'écran de la X1E propose un `.gcode` brut posé sur la carte SD, ni si un paquet sans `project_settings.config` est accepté hors réseau.
- Identifiant de la X1E : `C13` dans les profils de Bambu Studio (`Bambu Lab X1E.json`, `model_id`) ; `bambox` donne `BL-P003`, que rien ne confirme. Retenu : `C13`.

## Decision (proposée)

1. **Dialecte Bambu** pour tout G-code d'un profil `firmware = "bambu"` : en-tête `HEADER_BLOCK`, couches au sens du compteur de l'imprimante (une couche commence quand l'impression dépasse d'une hauteur de couche du profil le début de la précédente : paliers de hauteur pour les parcours continus ou non planaires), `M73 L` + `M991` + `M73 P R` à chaque couche.
2. **Deux formats d'export** pour les Bambu Lab, le paquet en premier : `.gcode.3mf` (G-code, MD5, `slice_info.config`, `model_settings.config`, `plate_1.json`, vignettes 512 et 128 px rendues par le moteur) et `.gcode`. Paquet écrit par le moteur (Rust pur, ZIP déterministe ; dépendances `md-5` et `miniz_oxide`), **sans** `project_settings.config`.
3. **Profil X1E** : gabarits de démarrage et de fin de Bambu Studio (« Bambu Lab X1E 0.4 nozzle », 02.08) réécrits en expressions Rhai (ADR-0004), source citée. Écarts assumés : scan lidar de la première couche et détection d'impression dans le vide retirés (un parcours FabWeave n'est pas une suite de couches planes : fausses alertes) ; `outer_wall_volumetric_speed` des lignes de purge et de calibration remplacé par le débit volumétrique maximal du filament ; accélération de calibration fixée à 5 000 mm/s².

**À trancher par l'essai** : si le paquet sans `project_settings.config` est refusé, l'ajouter (réglages de Bambu Studio aplatis pour la X1E) ; si le `.gcode` brut n'apparaît pas à l'écran, ne proposer que le paquet.

## Alternatives considered

- **G-code brut seul** : le plus simple, mais incertain à l'écran de la X1E, sans vignette, sans durée ni filament dans la liste des fichiers.
- **Paquet complet à la `bambox`** (544 réglages de trancheur) : lourd à produire et à maintenir pour une imprimante, et la plupart des réglages n'ont pas de sens pour un parcours FabWeave ; gardé en réserve si l'essai l'exige.
- **Appeler `bambox`** : dépendance Python hors du moteur, incompatible avec un moteur pur en WebAssembly.

## Consequences

Les profils Bambu Lab réutilisent les gabarits de Bambu Studio presque ligne à ligne : une nouvelle version du firmware peut demander de les reprendre. Le paquet ne déclare qu'un filament (emplacement 1 de l'AMS ou bobine externe). Les gabarits de Bambu Studio sont sous AGPL-3.0 ; comme pour les profils Prusa (PrusaSlicer, AGPL-3.0), ils sont réécrits et cités.

## Validation

À faire : essai sur la X1E par la carte SD des deux fichiers (`.gcode.3mf` et `.gcode`) : visibles à l'écran, vignette, durée, compteur de couches, préparation de la machine (nivellement limité à la zone de la pièce, purge), impression, fin (plateau descendu). Tests automatiques : paquet relu (CRC, MD5, métadonnées), G-code de la X1E pour tout le catalogue.
