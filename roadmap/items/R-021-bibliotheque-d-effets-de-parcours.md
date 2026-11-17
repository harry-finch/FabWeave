---
id: R-021
titre: "Bibliothèque d'effets de parcours"
resume: "Effets tracés par la buse (trame, grain, vagues, picots, dentelle, colonnes, gros boudin) en fonctions natives de l'API, et un « Vase à effets »."
statut: specifiee
jalon: v0.7
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Les effets les plus spectaculaires de ColiWorks sont tracés par la buse elle-même (trame, grain, picots, dentelle, colonnes, gros boudin). Dans FabWeave, chaque modèle doit les recoder en Rhai. Une bibliothèque d'effets natifs, applicables à n'importe quel parcours, les met à portée de tous les modèles et des modèles générés par IA.

## Inclus

- Fonctions natives de l'API qui transforment une liste de points : **trame** (décalage alterné d'une couche à l'autre), **grain**, **vagues**, **picots**, **dentelle** (la buse monte et descend, chaque fil ne touche la rangée du dessous qu'à son pied), **colonnes**, **gros boudin** (largeur et hauteur de boudin indépendantes de la buse).
- Réglages communs : intensité (mm), répétitions par tour, plage de hauteur avec transition ; réglages propres à la dentelle : hauteur de crête, vitesse à la crête (facteur de vitesse), recul du pied.
- Les effets ne parlent que de géométrie, de section de matière et de facteur de vitesse : jamais d'`E`, de température ni de rétraction (règle du format).
- Valeur hors domaine refusée avec un code stable ; nombre de points borné comme les autres fonctions.
- Le garde-buse (SAFE-03) et les contrôles de qualité s'appliquent aux parcours obtenus, sans exception.
- Nouveau modèle « Vase à effets » (choix de l'effet et de ses réglages) ; WEAVE_FORMAT et prompt IA mis à jour avec un exemple par effet.

## Hors périmètre

- Textures de maillage (gyroïde, Voronoï…) : FabWeave ne fabrique pas de maillage.
- Recettes de température ou de ventilation par effet : elles restent dans les profils.
- Éditeur graphique de la cellule (crête à tirer) : R-020 (courbe) pourra le permettre plus tard.

## Dépendances

Aucune. R-024 (débit en porte-à-faux) améliore la dentelle et les colonnes mais n'est pas requis.

## Critères d'acceptation

- Chaque effet a son test : amplitude mesurée sur le parcours égale à l'intensité demandée (à 0,01 mm), nombre de motifs par tour égal aux répétitions.
- Une intensité négative ou des répétitions nulles sont refusées avec leur code.
- « Vase à effets » se calcule aux valeurs par défaut et aux bornes de chaque curseur sur les sept imprimantes en PLA (test du catalogue), sans heurt du garde-buse aux valeurs par défaut.
- Un vase de 50 000 points avec un effet se calcule en moins de 100 ms en natif (mesure dans `bench`).
- `cargo test --workspace`, Clippy et `npm run check` sans erreur.

## Vérifications de l'utilisateur

- Imprimer le « Vase à effets » avec au moins trois effets (dont la dentelle) sur la MK4S ou la CORE One, et juger le rendu et la tenue.
