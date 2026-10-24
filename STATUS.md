# État du projet

**Jalon** : **v0.0 — fondations**, en validation. Prochain jalon : v0.1 (tranche verticale), voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Dépôt | Git, branche `main`, publié sur GitHub (`harry-finch/FabWeave`, public) |
| Documentation | README, AGENTS, ROADMAP, STATUS, DEVLOG, [FEATURES](docs/FEATURES.md), ADR-0001 à 0003 |
| Code | Aucun |
| Outils vérifiés | Rust 1.88.0 avec la cible `wasm32-unknown-unknown`, wasm-pack, Node 26, gh |

## Décisions de l'utilisateur (24/10/2026)

- Imprimantes prioritaires : Prusa MK4S / Core One (v0.1), Bambu Lab A1 / P1 / X1 (v0.3).
- Modèle étalon de la v0.1 : le vase à ondulations (CAT-01).
- Version web d'abord, application Tauri à la v0.6.
- Dépôt GitHub public.

## Points ouverts

- Validation de la feuille de route.
- ADR-0003 (three.js) proposée, à confirmer à l'étape 4 de la v0.1.
- Bambu Lab : G-code brut ou paquet `.gcode.3mf` (étude au début de la v0.3).

## Prochaine action

Après validation de la feuille de route : v0.1, étape 1 (squelette du workspace et de l'interface).
