---
id: A-027
type: tester
titre: "Regarder un passage de la CI « Contrôles » sur GitHub"
element: R-009
statut: a-faire
cree: 2026-11-17
par: agent
fait:
---

R-009 : workflow .github/workflows/ci.yml, lancé à chaque push et pull request sur main. Onglet Actions › Contrôles du dépôt harry-finch/FabWeave : vérifier qu'il est vert et juger sa durée. Pour vérifier qu'il bloque bien : sur une branche, casser un test (ou ajouter un avertissement Clippy), ouvrir une pull request vers main : le passage doit échouer ; puis fermer la PR et supprimer la branche. Répondre : durée acceptable ou non, échec bien constaté.
