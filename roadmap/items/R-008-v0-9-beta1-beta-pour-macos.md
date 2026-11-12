---
id: R-008
titre: "v0.9-beta1 — Bêta pour macOS"
resume: "Publier une bêta avec l'application macOS pour tester et valider tout ce qui reste ouvert (v0.3, v0.4, v0.5)."
statut: a-tester
jalon: v0.9-beta1
cree: 2026-11-02
maj: 2026-11-12
livree:
---
Décidée par l'utilisateur le 30/10/2026 : publier maintenant une bêta avec l'application macOS, qui sert à tester et valider tout ce qui reste ouvert (v0.3, v0.4, v0.5).

1. **Application macOS** (Tauri, Apple Silicon, signature ad hoc, non notarisée) : interface et moteur WebAssembly dans une fenêtre, exports dans Téléchargements. *Faite le 30/10/2026 : `npm run desktop`.*
2. **Audit technique** (AUDIT-01, 02/11/2026, rapport hors dépôt), lot 1 avant publication : textes jamais transformés en commandes (AUD-001), sauts en Z dans le volume (AUD-002), `G90` / `M83` réaffirmés par le moteur (AUD-003), plafonds de température par imprimante (AUD-004), README à jour (AUD-012). *Fait le 02/11/2026.* Lot 2 ensuite : budgets de calcul, CI, sauvegardes, identifiants des modèles personnels, export CLI atomique. Lot 3 plus tard : moteur relançable, CSP, tests de l'interface, notarisation.
3. ⏸ Essais de l'utilisateur avec cette application.
4. Publication du dépôt et de la bêta. *Faite le 02/11/2026, à la demande de l'utilisateur, avant la fin de ses essais, pour la faire tester par un collègue : pré-version GitHub `v0.9.0-beta.1` avec le DMG.*
