---
id: R-023
titre: "Formes : sections vues de dessus, torsion et cage"
resume: "Fonctions de forme natives (sections pétale, étoile, cannelé…, torsion rythmée, cage de déformation) et un modèle « Forme libre »."
statut: a-tester
jalon: v0.7
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Dans ColiWorks, une pièce se façonne en trois gestes : une section vue de dessus (pétale, étoile, cannelé…), une torsion, et une **cage** dont on tire les points pour déformer la paroi. FabWeave n'offre que des fonctions de bas niveau (`polygon`, `modulate_r`). Des fonctions de forme natives donnent ces gestes à tous les modèles, et un modèle « Forme libre » les expose en curseurs.

## Inclus

- Sections vues de dessus, en fonctions natives : rond, facetté, pétale, étoile, cannelé, fleur, dents, goutte ; nombre de côtés (3 à 60) et profondeur, angle de départ.
- Torsion avec rythme : régulière, en S, aller-retour ; resserrement ou évasement vers le haut.
- Révolution : une section qui suit le profil (le paramètre courbe de R-020 s'il est livré, sinon un profil donné par le script), en spirale continue.
- **Cage** : grille de points autour de la pièce (2 à 8 anneaux, 3 à 24 points par anneau) dont chaque point déplace la paroi, avec une douceur des bords réglable ; ses déplacements sont des paramètres du modèle.
- Modèle « Forme libre » : section, torsion, resserrement, cage, motif d'ondulation.
- Code stable pour toute valeur hors domaine ; WEAVE_FORMAT et prompt IA mis à jour.

## Hors périmètre

- Tirer les points de la cage directement dans la vue 3D : plus tard (décision du 17/11/2026).
- Formes importées (STL) : R-012.
- Section dessinée à main levée : R-025 (paramètre image) pour un contour importé.

## Dépendances

- Décidé par l'utilisateur le 17/11/2026 : la cage se règle par des curseurs (un déplacement par point, regroupés par anneau) ; la manipulation dans la vue 3D viendra après.
- Recouvre en partie R-012, étape 2 (« formes paramétriques : révolution d'un profil, polygone extrudé et torsadé ») : les mêmes fonctions servent aux deux, R-012 sera mis à jour.

## Critères d'acceptation

- Chaque section a son test : nombre de lobes et profondeur mesurés sur le contour.
- Une torsion de 90° fait tourner le dernier tour de 90° (±0,1°) par rapport au premier.
- Cage au repos (aucun déplacement) : même parcours qu'un modèle sans cage, octet pour octet.
- « Forme libre » se calcule aux valeurs par défaut et aux bornes sur les sept imprimantes.
- `cargo test --workspace`, Clippy et `npm run check` sans erreur.

## Vérifications de l'utilisateur

- Composer trois pièces avec « Forme libre » et juger la richesse des formes et la prise en main.
- Imprimer une pièce torsadée et déformée par la cage.
