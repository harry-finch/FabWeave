---
id: R-020
titre: "Paramètre courbe : silhouette dessinée"
resume: "Nouveau type de paramètre .weave : une silhouette modifiable en tirant des points, avec une galerie de silhouettes prêtes."
statut: a-tester
jalon: v0.7
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Aujourd'hui, la silhouette d'un vase ou d'une lampe ne se règle que par des curseurs chiffrés (rayon, renflement, resserrement). Un maker veut **dessiner** la silhouette en tirant des points, comme dans ColiWorks, sans écrire de code. Le paramètre « courbe » le permet dans n'importe quel modèle `.weave`, avec une galerie de silhouettes prêtes.

## Inclus

- Nouveau type de paramètre `curve` dans l'en-tête `.weave` : points par défaut (rayon, hauteur en mm), bornes de chaque axe, nombre de points minimal et maximal, raccord lisse ou droit.
- Galerie de silhouettes nommées (FR / EN) déclarées par le modèle ; en choisir une remplace la courbe en une étape d'annulation.
- Dans le script : la courbe comme liste de points, et une fonction native qui donne le rayon à une hauteur (interpolation lisse ou droite, déterministe).
- Éditeur dans le panneau Paramètres : tracé de la courbe à l'échelle, points déplacés à la souris, ajout par clic près du trait, retrait par double-clic, saisie chiffrée du point choisi, ↺ pour revenir à la courbe du modèle ; un geste = une étape d'annulation ; calcul au relâchement au-delà de 60 ms, comme les curseurs.
- Point hors bornes, courbe qui revient sur elle-même en hauteur, nombre de points hors limites : refusés avec un code stable (`param.out_of_range`, `param.curve_*`), jamais corrigés.
- La courbe est enregistrée dans le projet `.fabweave.json` et dans le projet embarqué au G-code.
- Nouveau modèle « Vase à silhouette libre » et vase à ondulations repris avec une courbe (anciens projets relus).
- Version du format `.weave` incrémentée, ancien format relu (test), WEAVE_FORMAT et prompt IA mis à jour avec un exemple vérifié.

## Hors périmètre

- Tracé à main levée (pinceau) : seuls les points se déplacent.
- Contour fermé vu de dessus : R-023 (formes) et R-025 (paramètre image).
- Courbes en 3D, plusieurs courbes liées (profil + épaisseur) : plus tard.

## Dépendances

Aucune. R-023 (vue de dessus) s'appuie sur cette courbe pour faire tourner une section autour du profil.

## Critères d'acceptation

- Un modèle avec un paramètre `curve` se lit, se calcule et s'exporte ; tous les `.weave` existants se lisent sans changement de G-code (test sur le catalogue).
- Un point hors bornes est souligné et bloque l'export avec `param.out_of_range` ; une courbe non monotone en hauteur est refusée avec son code.
- Projet enregistré puis rouvert : même courbe, même G-code octet pour octet.
- Choisir une silhouette de la galerie puis ⌘Z revient à la courbe précédente.
- `cargo test --workspace`, Clippy et `npm run check` sans erreur.

## Vérifications de l'utilisateur

- Dessiner une silhouette dans l'éditeur et juger la prise en main (souris, saisie chiffrée, galerie).
- Imprimer un vase à silhouette libre.
