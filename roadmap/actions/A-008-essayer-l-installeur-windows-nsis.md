---
id: A-008
type: tester
titre: "Essayer l'installeur Windows (NSIS)"
element: R-007
statut: a-faire
cree: 2026-11-12
par: agent
fait:
---

La v0.6 note l'application Windows « à essayer » : l'installeur NSIS (non signé) est construit par le workflow GitHub Actions windows.yml, lancé à la main, mais n'a jamais été exécuté (ni la construction, ni l'installeur). Lancer le workflow, installer sur un poste Windows. Observer : installation par utilisateur, chemin Téléchargements, WebGL, dépôt d'un .weave sous WebView2. Répondre : ce qui fonctionne.
