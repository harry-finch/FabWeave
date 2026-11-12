---
id: R-007
titre: "v0.6 — Bureau et projets (jalon 6)"
resume: "Application de bureau Tauri 2 avec dossier de modèles surveillé, projet .fabweave.json et CLI complète."
statut: en-cours
jalon: v0.6
cree: 2026-10-24
maj: 2026-11-12
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

## État et source

Périmètre repris du jalon historique, dont le statut « en cours » a été confirmé dans A-009. Les critères ci-dessus explicitent ce périmètre ; leur remise en forme a été validée par l'utilisateur le 12/11/2026. Application macOS réalisée ; workflow Windows présent, construction et essai non vérifiés. Dossier surveillé dans l'application de bureau, fichiers récents et format de projet restent à développer. La surveillance de `models/` par Vite en développement ne vaut pas livraison du dossier surveillé de l'application.
