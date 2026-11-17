# État du projet

**Jalon** : bêta **0.9.0-beta.1** publiée le 02/11/2026 pour macOS Apple Silicon ([R-008](roadmap/items/R-008-v0-9-beta1-beta-pour-macos.md)), en attente des essais de l'utilisateur. v0.0, v0.1, v0.2 et v0.4 validées ; v0.3 et v0.5 à tester. Le jalon bureau v0.6 est en cours : projet `.fabweave.json` et projet embarqué dans le G-code (ADR-0009), dossier de modèles surveillé, fichiers récents et dialogues natifs faits le 17/11, à essayer (A-013, A-014) ; workflow Linux à lancer (A-015). Statuts confirmés lors de la migration (A-009, 12/11). Voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur et catalogue | G-code direct déterministe, profils, préparation du plateau, garde-buse et contrôles de qualité ; 24 modèles, 7 imprimantes, 5 filaments. Inventaire : [FEATURES](docs/FEATURES.md). |
| Interface | Svelte 5 / WebAssembly ; catalogue et profils personnels, vue 3D en volume, inspection et export PNG, FR / EN, thèmes. |
| Bureau | Application macOS construite ; workflow Windows présent mais jamais exécuté selon les derniers retours. Projet, dossier surveillé, fichiers récents et dialogues natifs faits le 17/11, non essayés ; workflow Linux écrit, jamais lancé ([R-007](roadmap/items/R-007-v0-6-bureau-et-projets-jalon-6.md)). |
| Tests | Contrôles du 17/11/2026 : `cargo clippy --workspace --all-targets` sans avertissement ; `npm run check` : 0 erreur, 0 avertissement. `cargo test --workspace` : 167 tests réussis ; CI « Contrôles » à chaque push sur main. Le crate bureau est hors workspace. |
| Vérifié | G-code relu dans PrusaSlicer ; vase imprimé sur CORE One, trois modèles de v0.2, plusieurs sur MK4S (28/10) et la CORE One pour la v0.3 (A-001, 13/11) validés par l’utilisateur. Parcours de l'interface vérifiés dans le navigateur jusqu'à v0.5 ; auto-test de la build macOS : moteur, WebGL et exports dans Téléchargements. Détails datés dans [DEVLOG](DEVLOG.md). |
| Non vérifié | X1E et autres profils sur machine, modèles restants, adhésion et copies, import de fichiers réels (OrcaSlicer notamment), comparaison des temps à PrusaSlicer, seuils du garde-buse et cône des buses. Parcours macOS manuels et installation sur un autre poste ; Windows et Linux. Voir les actions A-002 à A-008. |

## Points ouverts

- Profils livrés sans cône de buse mesuré : seuls le bout et la tête sont vérifiés ; gravité du garde-buse encore à valider ([ADR-0008](docs/adr/0008-garde-buse-et-gravite-des-collisions.md)).
- Application macOS non notarisée ; installeur Windows non signé. Retours de bêta attendus.
- Mesures historiques : 50 000 points en pur script en 81 ms ; WebAssembly de 3,6 Mo (1,1 Mo gzip). Le fond en lignes peut multiplier les rétractions ; l'option anneaux l'évite.
- Mise d'équerre du 12/11 : documentation et critères bureau validés, travail précédent enregistré dans `1175867` sur `main` ; mises à jour de skills et création de la branche de reprise encore bloquées par les droits sur `.agents/` et `.git/` (A-010). Figure choisie et construite, contrôle visuel restant (A-011), bilan à valider (A-012). `fab-design-system` conservée par choix de l'utilisateur.

## À suivre

Essais et mesures : actions A-002 à A-008 (v0.3 à v0.6), A-013 à A-015 (bureau et projets), A-017 à A-026 (jalon v0.7 « Formes et effets », développé le 17/11/2026 : R-020 paramètre courbe, R-023 formes, R-021 effets de parcours, R-024 débit en porte-à-faux et couches variables, R-025 paramètre image, R-022 luminaires). Voir [ROADMAP.md](ROADMAP.md). Lot 2 de l'audit ([R-009](roadmap/items/R-009-audit-technique-lot-2.md)) fait le 17/11/2026, à essayer (A-027 à A-029). Suite technique : [R-010 — audit, lot 3](roadmap/items/R-010-audit-technique-lot-3.md), à spécifier. Historique détaillé des essais et limites conservé dans [DEVLOG.md](DEVLOG.md).
