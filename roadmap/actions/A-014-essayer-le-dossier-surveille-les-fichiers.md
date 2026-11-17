---
id: A-014
type: tester
titre: "Essayer le dossier surveillé, les fichiers récents et Enregistrer sous dans l'application macOS"
element: R-007
statut: a-faire
cree: 2026-11-17
par: agent
fait:
---

Étape 2 de la v0.6, faite le 17/11/2026 : construite, jamais ouverte par l'agent. Application : crates/fabweave-desktop/target/release/bundle/macos/FabWeave.app (ou le DMG du même dossier bundle/dmg/). Gestes : Fichier › Choisir le dossier de modèles… (un dossier avec quelques .weave, dont un dans un sous-dossier) ; vérifier le groupe « Dossier … » du catalogue ; modifier un .weave dans un éditeur et enregistrer, en ajouter un, en supprimer un, pendant que l'un d'eux est affiché. Puis ⌘S sur un projet neuf (dialogue natif), modifier une valeur et ⌘S (réenregistré sans dialogue), ⇧⌘S (Enregistrer sous), Fichier › Récents pour rouvrir, ⌘O par le dialogue natif (projet, G-code, .weave, profil), quitter et relancer (dossier et récents gardés). Observer : catalogue et vue suivent le disque en moins d'une seconde, sans redémarrage ; aucun fichier .tmp laissé à côté du projet ; récent introuvable retiré avec un message. Répondre : ce qui marche, ce qui manque ou gêne.
