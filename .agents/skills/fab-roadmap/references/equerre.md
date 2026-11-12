# Terminer la mise d'équerre d'un projet

FabBoard met un projet d'équerre en deux temps. Le premier est automatique : il crée `roadmap/`, remplit `projet.toml` de ce qui se détecte, installe cette skill. Le second demande du jugement et l'avis de l'utilisateur : c'est cette session, lancée depuis FabBoard par « Terminer la mise d'équerre avec l'agent ».

À la fin, le projet a une feuille de route au format, une documentation conforme, des skills à jour, une figure pour sa carte, et FabBoard n'a plus rien à signaler que l'utilisateur ne sache.

## Avant de commencer

1. `fabboard doctor .` : la liste des écarts est le programme de la session. S'il n'y a pas de dossier `roadmap/`, s'arrêter : c'est le bouton « Mettre d'équerre » de FabBoard qui le crée.
2. `git status` : si l'arbre de travail a des modifications en cours, le dire et demander s'il faut continuer ; ne jamais les embarquer dans un commit.
3. Travailler sur une branche dédiée (`fabboard/mise-d-equerre`) sauf demande contraire. Ne pas pousser.
4. Annoncer à l'utilisateur les étapes qui le concernent, pour qu'il sache quand on aura besoin de lui : les skills à remplacer, la documentation à valider, le concept de la figure à choisir.

Les étapes sont indépendantes : en sauter une déjà faite, et faire **un commit par étape**.

## 1. Feuille de route

Si `fabboard doctor` signale `roadmap.legacy` : suivre [migration.md](migration.md) en entier.

Sinon, vérifier que `projet.toml` dit vrai : sous-titre, couleur, logo, jalons, et surtout les **versions à tester** (`[[test]]`) : l'adresse locale et la commande qui la sert, le motif des builds, la release. Demander à l'utilisateur comment il essaie le projet si rien ne se déduit des consignes d'agent. Renseigner `agent` (`claude` ou `codex`) s'il le dit.

## 2. Skills

`fabboard skills status` donne l'état de chaque skill du projet par rapport à la bibliothèque.

- `en-retard` : `fabboard skills install <nom>` les met à jour, sans risque.
- `differente` (copie d'avant FabBoard) ou `modifiee` : montrer à l'utilisateur ce qui diffère (`diff -r` entre la copie du projet et celle de la bibliothèque, résumé en quelques lignes par fichier), et dire si ce que le projet a en propre existe déjà dans la bibliothèque. Avec son accord : `fabboard skills install <nom> --force`. Si le projet a une retouche que la bibliothèque n'a pas, ne pas l'écraser : la signaler, c'est à lui de décider où elle va.
- `hors-bibliotheque` : ne pas y toucher.
- Une skill que les consignes d'agent citent et qui n'est pas installée : proposer de l'installer.

## 3. Documentation

Suivre `references/mise-en-conformite.md` de la skill `project-documentation` (l'installer si elle manque : `fabboard skills install project-documentation`). En bref : `README.md`, `AGENTS.md` repris par `CLAUDE.md`, `STATUS.md` et `roadmap/` sont attendus ; ce qui manque se crée à partir du code et des documents existants, sans rien inventer ; ce qui est bon ne se réécrit pas ; la « prochaine action » d'un `STATUS.md` devient des actions de l'utilisateur ; les consignes d'agent renvoient à cette skill pour la feuille de route.

Montrer les changements et obtenir l'accord de l'utilisateur avant de commiter.

## 4. Figure de la carte

Si `fabboard doctor` signale `project.no_figure`, ou si `projet.toml` porte une figure marquée provisoire : le projet n'a pas encore sa **figure du métier**, l'illustration au trait de sa carte dans FabBoard.

1. Lire la skill `fab-illustration-system` (l'installer si elle manque), en particulier `references/creer-et-adopter.md` et le tableau « Ce que le catalogue n'a pas » de `references/catalogue.md` : il donne un point de départ pour plusieurs applications.
2. Installer `hairline-create` dans le projet si elle n'y est pas, comme le dit `fab-illustration-system`.
3. **Proposer deux ou trois concepts** : pour chacun, l'objet du métier dessiné, ce que le pointeur y règle, et pourquoi il dit ce que fait l'application. **Le choix revient à l'utilisateur**, comme pour un logo. Ne rien dessiner avant.
4. Dessiner la figure choisie avec `hairline-create`, en suivant ses règles et ses contrôles (validation du fichier, regard dans un navigateur). Ne jamais dessiner une figure à la main ni modifier le moteur.
5. La ranger dans `branding/illustrations/<nom>.js`, avec le `README.md` que `fab-illustration-system` demande. FabBoard la lit à cet endroit : rien à copier ailleurs.
6. `figure = "<nom>"` dans `roadmap/projet.toml` (retirer le commentaire « provisoire »).
7. Demander à l'utilisateur s'il veut aussi la figure **dans l'application** (état vide, premier lancement, À propos) : c'est un autre travail, décrit par `fab-illustration-system`. S'il le veut, l'ajouter à la feuille de route (`fabboard roadmap add`), ne pas le faire dans cette session.

Si l'utilisateur n'est pas là pour choisir : créer une action `valider` (« Choisir le concept de la figure de <projet> ») avec les concepts dans son corps, et laisser la figure provisoire.

## 5. Bilan

1. `fabboard doctor .` et `fabboard roadmap lint` : ce qui reste est soit expliqué, soit devenu une action.
2. Une action `valider` : « Valider la mise d'équerre de <projet> », avec dans son corps ce qui a été fait à chaque étape, ce qui a été laissé et pourquoi, et la branche à fusionner.
3. Dire à l'utilisateur, en quelques lignes, ce qui a changé et ce qu'il lui reste à faire. Ne pas fusionner ni pousser : c'est sa décision.
