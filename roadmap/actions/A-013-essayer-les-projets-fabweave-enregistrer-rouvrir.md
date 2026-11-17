---
id: A-013
type: tester
titre: "Essayer les projets FabWeave (enregistrer, rouvrir, G-code)"
element: R-007
statut: a-faire
cree: 2026-11-17
par: agent
fait:
---

Étape 1 de la v0.6 (DESK-04, ADR-0009), faite le 17/11/2026 ; l'interface n'a été contrôlée que par npm run check (navigateur indisponible pour l'agent). Version web : cd ui && npm run wasm && npm run dev, puis http://localhost:5175. Gestes : nommer le projet dans l'encart « Projet » de la barre ; régler un modèle (valeurs, buse, placement, bordure) ; Fichier › Enregistrer ou ⌘S (fichier <nom>.fabweave.json dans Téléchargements) ; Fichier › Nouveau puis Annuler dans le toast ; déposer le .fabweave.json, puis un G-code exporté, puis un .gcode.3mf d'une Bambu Lab (ou ⌘O). Essayer aussi avec un modèle de « Mes modèles » et un profil personnel, et un G-code d'un autre trancheur (message attendu : aucun projet). Observer : même modèle, mêmes valeurs (↺ sur les valeurs modifiées seulement), même imprimante, filament, buse, placement et adhésion ; aucun doublon dans « Mes modèles » ni dans les profils ; menu Fichier et encart conformes au reste de la barre, en thème sombre et clair. Répondre : ce qui marche, ce qui manque ou gêne.
