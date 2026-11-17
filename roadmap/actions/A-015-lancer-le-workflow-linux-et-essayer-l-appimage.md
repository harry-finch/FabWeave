---
id: A-015
type: tester
titre: "Lancer le workflow Linux et essayer l'AppImage"
element: R-007
statut: a-faire
cree: 2026-11-17
par: agent
fait:
---

Étape 3 de la v0.6 : .github/workflows/linux.yml (AppImage et .deb, Ubuntu 22.04) écrit le 17/11/2026, jamais lancé. Après avoir poussé main, onglet Actions › Application Linux › Run workflow (tag vide pour un simple artefact, ou v0.9.0-beta.1). Observer : la construction aboutit ; sur un poste Linux, l'AppImage démarre, calcule un modèle, exporte dans Téléchargements, ouvre le dialogue natif et suit le dossier de modèles. Répondre : construction réussie ou journal d'erreur, et ce qui fonctionne sur le poste Linux.
