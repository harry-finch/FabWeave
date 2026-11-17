---
id: R-009
titre: "Audit technique, lot 2"
resume: "Budgets de calcul avant allocation, CI à chaque push, sauvegardes locales jamais perdues en silence, identifiants des modèles personnels sans collision, export CLI atomique."
statut: specifiee
cree: 2026-11-02
maj: 2026-11-17
livree:
---

## But

L'audit technique du 02/11/2026 (AUDIT-01) a relevé cinq faiblesses qui ne mettent pas la machine en danger, mais peuvent bloquer l'application ou faire perdre le travail d'un utilisateur sans qu'il le sache : un calcul démesuré qui sature la mémoire, des modèles et profils personnels qui ne sont pas réellement enregistrés, un modèle remplacé par un autre de nom voisin, un export en ligne de commande laissé à moitié écrit, et l'absence de contrôles automatiques à chaque modification du code. Ce lot les ferme avant une diffusion plus large.

## Inclus

- **Budgets de calcul (AUD-005)** : les remplissages (`fill_lines`, `fill_rings`, plusieurs contours) et les fonctions natives refusent une demande démesurée **avant** d'allouer (nombre de lignes, d'anneaux ou de points estimé d'abord) ; le parcours préparé (copies, bordure, radeau) est plafonné à **2 millions de points**, refusé avant d'être construit. Code stable (`path.too_many_points`, `setup.too_many_points`), jamais de plantage ni de blocage du calcul.
- **CI (AUD-011)** : workflow GitHub Actions à chaque push et pull request sur `main`, sur un poste Linux : `cargo test --workspace`, `cargo clippy --workspace --all-targets` sans avertissement, construction du WebAssembly, `npm run check`, depuis un clone propre. Les builds de bureau (Windows, Linux, macOS) restent manuels.
- **Sauvegardes locales (AUD-006)** : un échec d'enregistrement (stockage plein, interdit) affiche un bandeau persistant « Non enregistré sur ce poste » ; les données lues sont contrôlées (forme, version) et celles qui sont mal formées sont mises de côté telles quelles, jamais effacées, avec un message ; « Exporter ma bibliothèque » écrit un fichier unique avec les modèles et profils personnels, « Importer une bibliothèque » le relit (sans remplacer en silence : mêmes règles d'identifiant que ci-dessous).
- **Identifiants des modèles personnels (AUD-009)** : un fichier déposé de **même nom exact** met à jour l'ancien (message avec « Annuler ») ; un nom différent qui donnerait le même identifiant (`a+b.weave`, `a b.weave`) reçoit un suffixe (`-2`, `-3`…), jamais un remplacement. Les anciens identifiants mémorisés restent valables.
- **Export CLI atomique (AUD-010)** : `fabweave export` écrit dans un fichier temporaire voisin puis le renomme ; une erreur d'écriture laisse le fichier précédent intact et aucun fichier partiel. L'écrasement d'un fichier existant reste le comportement, documenté dans l'aide.
- Documentation : FEATURES, STATUS, DEVLOG, aide de la CLI, AUDIT-01 annoté (constats fermés).

## Hors périmètre

- Lot 3 (R-010) : cycle de vie du Worker et révisions synchronisées (AUD-007, AUD-008), tests de l'interface, CSP, notarisation.
- Builds de bureau automatiques à chaque push : restent manuels (choix du 17/11/2026).
- Synchronisation de la bibliothèque entre postes ou dans un nuage (R-017, écarté).
- Optimisations de performance proposées par l'audit, faute de mesure qui les justifie.

## Dépendances

Aucune. Les budgets se calibrent sur le catalogue actuel (aucun modèle refusé aux valeurs par défaut).

## Critères d'acceptation

- Un `fill_lines` d'espacement 0,000 001 mm sur un contour de 200 mm, et un modèle d'un million de points copié 100 fois, sont refusés en moins d'une seconde avec leur code, sans allouer la demande (tests, mémoire bornée).
- Tous les modèles du catalogue se calculent toujours aux valeurs par défaut sur les sept imprimantes (test du catalogue).
- Le workflow CI passe sur `main` ; une modification qui casse un test ou ajoute un avertissement Clippy le fait échouer (essai sur une branche).
- Stockage plein simulé : bandeau affiché, modèle gardé pour la session ; données stockées mal formées (`null`, objet au lieu de tableau, JSON tronqué) : démarrage normal, données mises de côté et message.
- Exporter puis importer la bibliothèque sur un stockage vidé redonne les mêmes modèles et profils.
- `a+b.weave` puis `a b.weave` : deux modèles distincts ; redéposer `a b.weave` modifié : une mise à jour, annulable.
- Écriture d'export en échec simulé (dossier en lecture seule, chemin invalide) : fichier précédent intact, aucun fichier partiel ni temporaire laissé.
- `cargo test --workspace`, Clippy et `npm run check` sans erreur.

## Vérifications de l'utilisateur

- Regarder un run du workflow CI sur GitHub (onglet Actions) et juger sa durée.
- Exporter la bibliothèque, vider les données du site ou réinstaller, puis l'importer.
- Déposer deux modèles aux noms voisins et vérifier qu'aucun ne disparaît.
