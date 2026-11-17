---
id: R-007
titre: "v0.6 — Bureau et projets (jalon 6)"
resume: "Application de bureau Tauri 2 avec dossier de modèles surveillé, projet .fabweave.json et CLI complète."
statut: en-cours
jalon: v0.6
cree: 2026-10-24
maj: 2026-11-17
livree:
---

## But

Utiliser FabWeave dans une application de bureau et reprendre une préparation d'impression sans la ressaisir. Les modèles personnels doivent pouvoir rester dans un dossier du poste et être retrouvés dans l'application.

## Inclus

- Application Tauri 2 pour macOS, Windows et Linux, fenêtres natives et fichiers récents.
- Dossier de modèles surveillé : un `.weave` déposé ou modifié apparaît aussitôt.
- Projet `.fabweave.json` contenant le modèle, les valeurs et les profils.
- Projet embarqué en commentaire dans le G-code : déposer un G-code FabWeave rouvre la préparation.
- CLI complète pour vérifier, mesurer et exporter les modèles.

## Hors périmètre

- Envoi à l'imprimante : R-013, reporté après la v1.0 le 30/10/2026.
- Serveur MCP : R-014, même report.
- Signature des installeurs et publication de la v1.0 : R-011.

## Dépendances

- Réutiliser le moteur, le catalogue et les profils existants ; conserver les garanties de sécurité de l'export décrites dans AGENTS.md.
- Bêta macOS déjà réalisée dans R-008 ; essai Windows suivi par A-008.
- La remise en forme des critères a été validée par l'utilisateur le 12/11/2026.

## Critères d'acceptation

- L'application de bureau permet de choisir un modèle, de régler ses paramètres, de voir le parcours et d'exporter sur les trois plateformes.
- Ajouter ou modifier un `.weave` dans le dossier surveillé actualise le catalogue sans redémarrage.
- Enregistrer puis rouvrir un `.fabweave.json` retrouve le modèle embarqué, ses valeurs et ses profils.
- Déposer le G-code d'un projet exporté retrouve la même préparation.
- Les fichiers récents permettent de rouvrir une préparation sauvegardée.
- La CLI conserve ses commandes `check`, `stats` et `export` ; les vérifications du workspace et de l'interface passent.

## Vérifications de l'utilisateur

- Essayer l'application de bureau sur macOS, Windows et Linux : ouverture, calcul et export.
- Ajouter puis modifier un modèle dans le dossier surveillé et vérifier sa mise à jour dans le catalogue.
- Enregistrer une préparation, la rouvrir depuis les fichiers récents et comparer ses valeurs et ses profils.
- Déposer son G-code et vérifier que la préparation est retrouvée.

## Étapes

Ordre choisi par l'utilisateur le 17/11/2026 :

1. **Projet** (DESK-04, ADR-0009) : `.fabweave.json` tout embarqué (modèle et profils recopiés), toujours embarqué dans le G-code et le `.gcode.3mf`, menu Fichier (Nouveau, Ouvrir, Enregistrer), nom du projet ; dépôt d'un projet ou d'un G-code de FabWeave. *Fait le 17/11/2026 : moteur testé (même projet → même G-code, octet pour octet) ; interface vérifiée par `npm run check` seulement, navigateur indisponible.* ⏸ Essai par l'utilisateur (A-013).
2. **Bureau** : fichiers récents, dossier de modèles surveillé, Ouvrir / Enregistrer natifs. Choix de l'utilisateur du 17/11/2026 : un seul dossier, choisi dans le menu Fichier (aucun par défaut) ; récents et Enregistrer sous dans le menu Fichier de l'application, pas de menu natif. *Fait le 17/11/2026 : commandes Tauri (dialogues, lecture, écriture atomique, surveillance par `notify`), groupe « Dossier … » du catalogue, 10 fichiers récents ; construit, non essayé.* ⏸ Essai par l'utilisateur (A-014).
3. **Linux** dans la CI, essai Windows (A-008). *Workflow `linux.yml` (AppImage et .deb) écrit le 17/11/2026, jamais lancé* ⏸ (A-015).

## État et source

Périmètre repris du jalon historique, dont le statut « en cours » a été confirmé dans A-009. Les critères ci-dessus explicitent ce périmètre ; leur remise en forme a été validée par l'utilisateur le 12/11/2026. Application macOS réalisée ; workflow Windows présent, construction et essai non vérifiés. Format de projet, dossier surveillé et fichiers récents développés le 17/11/2026, à essayer. La surveillance de `models/` par Vite en développement ne vaut pas livraison du dossier surveillé de l'application.
