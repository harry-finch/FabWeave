# ADR-0003 : Vue 3D en WebGL avec three.js

Status: Accepted
Date: 2026-10-24

## Context

Le centre de l'application montre le parcours de la buse en 3D : de quelques milliers à quelques centaines de milliers de segments, mis à jour à chaque geste sur un curseur, avec orbite, déplacement, zoom, puis (v0.4) un rendu volumétrique des boudins (section W × H variable), une coloration par grandeur et l'inspection du segment survolé. FabVents dessine en Canvas 2D, insuffisant ici. La recherche proposait soit une projection 3D → 2D au CPU, soit un rendu GPU.

## Decision

- Rendu **WebGL 2 avec three.js**, dans un composant Svelte du centre de l'écran.
- v0.1 : `LineSegments` avec couleur par sommet, tampons remplacés à chaque calcul à partir des tableaux transférés par le worker (aucune copie intermédiaire).
- v0.4 : géométrie **instanciée** (une instance par segment : extrémités, W, H, valeur de coloration) et `ShaderMaterial` qui construit la section oblongue dans le *vertex shader*.
- Les couleurs de la scène (fond, plateau, grille, palette de coloration) sont lues dans les jetons CSS de `app.css` au chargement et au changement de thème : aucune couleur en dur.
- Caméra orbitale (`OrbitControls`), zoom centré sur le curseur, cadrage `F`, vues standard.

## Alternatives considered

- **Projection au CPU sur Canvas 2D** : simple, mais tri en profondeur et dessin de 200 000 segments trop lents pour 60 images/s, pas de rendu volumétrique.
- **WebGL 2 sans bibliothèque** : aucune dépendance, mais caméra, contrôles, sélection et gestion des tampons à réécrire.
- **wgpu en Rust (WebGPU)** : un seul langage pour le rendu, mais intégration lourde avec l'interface Svelte, et WebGPU encore inégal selon les navigateurs et les vues web de Tauri.

## Consequences

Dépendance d'environ 150 Ko compressés (après élagage). three.js est très répandu, bien documenté et connu des assistants de code. Le passage au volumétrique (v0.4) se fait sans changer le protocole avec le worker.

## Validation

v0.1 : orbite fluide (60 images/s) sur un parcours de 200 000 segments en lignes, sur l'ordinateur d'atelier. v0.4 : même objectif en volumétrique. À revoir si le volumétrique n'atteint pas 30 images/s.

### Mesure (24/10/2026, Apple M4, vue web Chromium de l'application)

Vase à ondulations réglé à 200 196 points (12 points par ondulation, 100 ondulations) : **200 195 segments** en `LineSegments` à couleur par sommet. Rendu synchrone (`render` puis lecture d'un pixel pour attendre le GPU) sur 60 images en orbite : **1,9 ms** en médiane, 4,7 ms au plus, très loin des 16,7 ms d'une image à 60 images/s. Mesure faite panneau masqué (canevas 600 × 300) : le coût des lignes dépend surtout du nombre de sommets, pas de la taille du canevas. Accès de mesure : `window.__fabweaveView` en développement seulement.
