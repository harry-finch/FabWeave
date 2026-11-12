# Migrer une ancienne feuille de route

Pour un projet dont FabBoard signale `roadmap.legacy` : un `ROADMAP.md` écrit à la main, dans un format propre au projet. Le but est de retrouver le même contenu dans `roadmap/`, sans rien perdre et sans rien inventer.

## Avant de commencer

1. `git status` : travailler sur une branche dédiée (`fabboard/migration-roadmap`) si l'utilisateur ne dit pas autrement. Ne pas pousser.
2. Si `roadmap/` n'existe pas, s'arrêter : c'est FabBoard qui le crée (« Mettre d'équerre »).
3. Lire **tout** l'ancien `ROADMAP.md`, puis `STATUS.md`, `DEVLOG.md` et les documents qu'il cite, pour connaître l'état réel : la feuille de route peut être en retard sur le code.

## Correspondances

| Dans l'ancien fichier | Devient |
| --- | --- |
| Introduction, but du projet, principes | `roadmap/intro.md`, repris tel quel (les liens relatifs restent valides : `ROADMAP.md` reste à la racine) |
| Versions ou jalons (« v0.7 », « M2b ») | `[[jalons]]` de `projet.toml`, dans l'ordre, avec leur titre |
| Un jalon ou un lot **terminé et validé** | Un élément `livree` par jalon ou par lot, corps = le texte d'origine, `livree` = la date écrite dans le document (à défaut, celle du commit qui l'a marqué terminé : `git log -S`) |
| Un jalon **livré mais pas encore validé** par l'utilisateur | Élément `a-tester`, et une action `tester` ou `valider` qui dit ce qui reste à vérifier |
| Un travail **en cours** | Élément `en-cours` |
| Un travail **à venir**, décrit avec but, contenu et critères | Élément `specifiee` : répartir le texte existant dans les six sections, sans le réécrire ; une section sans matière reçoit « À préciser. » et l'élément reste `idee` |
| Un travail à venir en une ligne, une piste, une « suite différée » | Élément `idee` |
| Un point d'arrêt en attente (« ⏸ validation », « à confirmer par l'utilisateur ») | Une action du bon type, rattachée à son élément |
| Un travail explicitement écarté | Élément `abandonnee`, avec la raison dans le corps |
| Un tableau de constats, de risques, de mesures | Reste dans le document d'origine (voir « L'ancien fichier ») et se cite depuis l'élément concerné |

Granularité : **un élément par chose qu'on peut livrer et vérifier séparément**. Un jalon déjà livré peut rester un seul élément ; un jalon à venir se découpe selon ses lots s'ils sont nommés dans le texte. Dans le doute, garder le découpage de l'auteur.

L'ordre de `ordre.txt` reprend l'ordre du document, sauf priorité explicite (« à la demande de l'utilisateur, X précède Y »).

## Règles

- **Ne pas inventer.** Pas de date, de critère ni de périmètre qui ne soit dans les documents du projet. Un statut incertain se décide avec l'utilisateur (action `decider`), pas au jugé.
- **Ne pas résumer** le texte des jalons livrés : il fait partie de l'histoire du projet. Le recopier.
- **Garder les identifiants du projet** (« v0.5 », « M2b », « AUD-017 », « CAT-01 ») dans les titres ou les corps : d'autres documents les citent.
- **Ne rien changer d'autre** : ni code, ni autre document, sauf les liens vers `ROADMAP.md` qui viseraient une ancre disparue.

## L'ancien fichier

Il n'est pas supprimé : `git mv ROADMAP.md docs/ROADMAP-avant-fabboard.md` (créer `docs/` au besoin), avec en tête une ligne qui dit qu'il est remplacé par `roadmap/` depuis telle date et conservé pour l'histoire. Corriger les liens relatifs qu'il contient si le déplacement les casse. Ensuite seulement : `fabboard roadmap render`, qui écrit le nouveau `ROADMAP.md`.

Mettre à jour les consignes d'agent du projet (`AGENTS.md`) : là où elles parlent de `ROADMAP.md`, dire que la feuille de route est dans `roadmap/` et renvoyer à la skill `fab-roadmap`.

## Contrôle

1. `fabboard roadmap lint` : aucune erreur ; les avertissements restants sont expliqués dans le rapport.
2. **Rien de perdu** : relire l'ancien fichier section par section et pointer, pour chacune, où elle se trouve maintenant (élément, `intro.md`, document conservé). Donner ce tableau à l'utilisateur.
3. `fabboard show .` : l'avancement et le jalon en cours correspondent à ce que dit `STATUS.md`.
4. Lire le `ROADMAP.md` généré comme le lira quelqu'un sur GitHub.

## Livrer

Un commit sur la branche (`roadmap: migration de l'ancienne feuille de route`), puis une action `valider` : « Valider la migration de la feuille de route », avec dans son corps le nombre d'éléments par statut, les choix faits là où le document était ambigu, et ce qui demande son avis. Ne pas fusionner ni pousser : c'est sa décision.
