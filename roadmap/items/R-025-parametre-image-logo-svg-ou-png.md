---
id: R-025
titre: "Paramètre image : logo SVG ou PNG"
resume: "Nouveau type de paramètre .weave : un dessin SVG ou PNG transformé en contours, en relief ou ajouré, enregistré dans le projet."
statut: specifiee
jalon: v0.7
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Un fablab veut marquer ses pièces : logo sur un dessous de verre, nom sur une boîte, motif découpé dans un abat-jour. Aujourd'hui, un contour ne peut venir que du script. Le paramètre « image » transforme un dessin SVG ou PNG en contours que le modèle remplit, ajoure ou met en relief.

## Inclus

- Nouveau type de paramètre `image` dans l'en-tête `.weave` : taille cible (mm), et un dessin par défaut facultatif.
- Le moteur lit un SVG (chemins, polygones, rectangles, cercles) ou un PNG (fond transparent ou clair, seuil réglable) et rend des contours fermés, trous compris, centrés et mis à l'échelle ; lecture pure et déterministe, dans le moteur.
- Limites : taille du fichier, nombre de contours et de points ; un fichier illisible, vide ou trop gros est refusé avec un code stable.
- Dans l'interface : dépôt ou choix du fichier dans le panneau Paramètres, vignette du contour, ↺ pour revenir au dessin du modèle.
- Le fichier est enregistré dans le projet `.fabweave.json` et le projet embarqué au G-code (même G-code à la réouverture).
- Deux modèles : « Dessous de verre au logo » (dessin en relief, pause de changement de couleur à sa base) et « Plaque ajourée » (dessin découpé).
- Version du format `.weave` incrémentée, ancien format relu, WEAVE_FORMAT et prompt IA mis à jour.

## Hors périmètre

- Relief en niveaux de gris (lithophanie) ; vectorisation de photos.
- Texte saisi au clavier (polices) : plus tard.
- Courbes de Bézier de SVG hors chemins standard (filtres, masques, textes SVG).

## Dépendances

Aucune. Le contour obtenu peut servir de section aux formes de R-023.

## Critères d'acceptation

- Un SVG d'un carré avec un trou rond rend deux contours de bonne taille (±0,05 mm) et le trou reste vide dans le parcours (test).
- Un PNG du même dessin rend des contours équivalents à 0,2 mm près (test).
- Fichier corrompu, SVG sans forme, PNG entièrement vide : refus avec leur code, sans plantage.
- Projet enregistré avec un logo puis rouvert : même G-code, octet pour octet.
- `cargo test --workspace`, Clippy et `npm run check` sans erreur.

## Vérifications de l'utilisateur

- Imprimer un dessous de verre avec le logo du fablab, en deux couleurs.
- Essayer un SVG et un PNG issus des outils du fablab (Inkscape, Illustrator) et signaler ceux qui sont mal lus.
