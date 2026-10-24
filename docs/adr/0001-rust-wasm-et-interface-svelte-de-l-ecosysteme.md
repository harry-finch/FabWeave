# ADR-0001 : Moteur Rust en WebAssembly et interface Svelte de l'écosystème Fab

Status: Accepted
Date: 2026-10-24

## Context

FabWeave génère du G-code à partir de modèles paramétriques et doit montrer le parcours en 3D en direct pendant qu'on bouge un curseur. La conversation de recherche (23/10/2026) proposait une application `egui` / `eframe` tout en Rust, calcul parallèle avec `rayon`. La demande impose l'interface de la famille Fab (skill `fab-design-system`) : FabSpeed et FabVents utilisent Rust pour le calcul et Svelte 5 pour l'interface, et FabVents fait tourner son moteur en WebAssembly dans un Web Worker (FabVents ADR-0001 et 0002), avec Tauri 2 pour le bureau.

## Decision

- Workspace Rust : `fabweave-core` (moteur pur : format `.weave`, Rhai, parcours, physique, G-code ; sans IO ni interface) et `fabweave-wasm` (pont `wasm-bindgen`). `fabweave-cli` s'ajoute à la v0.2.
- `fabweave-core` est compilé en WebAssembly (`wasm-pack --target web`) et exécuté dans un **Web Worker module** : le fil principal envoie modèle + valeurs + profils en JSON ; le worker renvoie le parcours en tableaux typés **transférés** (`Float32Array` positions, attributs W, H, vitesse, type) et un rapport JSON (mesures, messages). Un seul calcul en cours, **le dernier gagne**.
- Interface Svelte 5 + Vite + TypeScript dans `ui/`, partie du kit `fab-design-system` (jetons, composants, i18n) ; seule la couleur d'identité et le logo changent.
- Application de bureau Tauri 2 au jalon v0.6, sur la même interface.

## Alternatives considered

- `egui` (proposition de la recherche) : un seul langage et un rendu GPU, mais widgets, typographie et comportement différents de la famille : l'impression d'écosystème commun serait perdue et aucun composant partagé.
- Serveur local (comme FabSpeed) : aller-retour réseau à chaque geste et pas de version web sans installation.
- WebAssembly sur le fil principal : un parcours de 200 000 points figerait l'interface pendant le calcul.

## Consequences

Deux langages (Rust, TypeScript). Pas de `rayon` en WebAssembly sans configuration spéciale : le calcul est mono-thread (le parallélisme reste possible en natif pour la CLI). Le moteur doit être reconstruit (`npm run wasm`) après une modification Rust ; `ui/src/wasm/` est généré et hors Git. Le G-code est produit dans le worker et téléchargé par l'interface.

## Validation

Mesure dans le navigateur à la v0.1 : temps de calcul d'un parcours de 50 000 et 200 000 points dans le worker, fil principal fluide pendant l'orbite de la vue. À revoir si le calcul dépasse ≈ 60 ms pour les modèles du catalogue malgré les fonctions natives (ADR-0002).
