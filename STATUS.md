# État du projet

**Jalon** : bêta **0.9.0-beta.1** publiée le 02/11/2026 pour macOS Apple Silicon ([R-008](roadmap/items/R-008-v0-9-beta1-beta-pour-macos.md)), en attente des essais de l'utilisateur. v0.0, v0.1, v0.2 et v0.4 validées ; v0.3 et v0.5 à tester. Le jalon bureau v0.6 est en cours. Statuts confirmés lors de la migration (A-009, 12/11). Voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur et catalogue | G-code direct déterministe, profils, préparation du plateau, garde-buse et contrôles de qualité ; 24 modèles, 7 imprimantes, 5 filaments. Inventaire : [FEATURES](docs/FEATURES.md). |
| Interface | Svelte 5 / WebAssembly ; catalogue et profils personnels, vue 3D en volume, inspection et export PNG, FR / EN, thèmes. |
| Bureau | Application macOS construite ; workflow Windows présent mais jamais exécuté selon les derniers retours. Linux, dossier surveillé, fichiers récents et projets restent à développer ([R-007](roadmap/items/R-007-v0-6-bureau-et-projets-jalon-6.md)). |
| Tests | Contrôles du 12/11/2026 : `cargo clippy --workspace --all-targets` sans avertissement ; `npm run check` : 0 erreur, 0 avertissement. `cargo test --workspace` : 127 tests réussis. Le crate bureau est hors workspace. |
| Vérifié | G-code relu dans PrusaSlicer ; vase imprimé sur CORE One, trois modèles de v0.2 et plusieurs sur MK4S validés par l'utilisateur. Parcours de l'interface vérifiés dans le navigateur jusqu'à v0.5 ; auto-test de la build macOS : moteur, WebGL et exports dans Téléchargements. Détails datés dans [DEVLOG](DEVLOG.md). |
| Non vérifié | X1E et autres profils sur machine, modèles restants, adhésion et copies, import de fichiers réels (OrcaSlicer notamment), comparaison des temps à PrusaSlicer, seuils du garde-buse et cône des buses. Parcours macOS manuels et installation sur un autre poste ; Windows et Linux. Voir les actions A-001 à A-008. |

## Points ouverts

- Profils livrés sans cône de buse mesuré : seuls le bout et la tête sont vérifiés ; gravité du garde-buse encore à valider ([ADR-0008](docs/adr/0008-garde-buse-et-gravite-des-collisions.md)).
- Application macOS non notarisée ; installeur Windows non signé. Retours de bêta attendus.
- Mesures historiques : 50 000 points en pur script en 81 ms ; WebAssembly de 3,6 Mo (1,1 Mo gzip). Le fond en lignes peut multiplier les rétractions ; l'option anneaux l'évite.
- Mise d'équerre du 12/11 : documentation et critères bureau validés ; mises à jour de skills et commits bloqués par les droits sur `.agents/` et `.git/` (A-010). Figure choisie et construite, contrôle visuel restant (A-011), bilan à valider (A-012). `fab-design-system` conservée par choix de l'utilisateur.

## À suivre

Essais et mesures : actions A-001 à A-008 dans [ROADMAP.md](ROADMAP.md). Suite technique : [R-009 — audit, lot 2](roadmap/items/R-009-audit-technique-lot-2.md). Historique détaillé des essais et limites conservé dans [DEVLOG.md](DEVLOG.md).
