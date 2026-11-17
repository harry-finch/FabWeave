---
id: R-022
titre: "Luminaires : suspension, rosace, applique, lampe à poser"
resume: "Rubrique Luminaires du catalogue : pièces prêtes à monter avec alésage de douille E27 / E14, rosace, platine murale et emboîtement."
statut: specifiee
jalon: v0.7
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Les luminaires imprimés sont l'usage phare de ColiWorks et un projet fréquent en fablab. FabWeave sait déjà imprimer un abat-jour, mais pas la pièce qui le rend utilisable : passage de la douille, rosace, fixation murale, emboîtement de deux pièces. Une famille « Luminaires » du catalogue donne des lampes complètes, prêtes à monter.

## Inclus

- Nouvelle rubrique « Luminaires » du catalogue avec quatre modèles :
  - **suspension** : alésage de douille au sommet, au choix E27 ou E14 (cotes fournies par l'utilisateur), collier d'appui ;
  - **rosace** : coupelle qui cache le boîtier du plafond, trou du câble, deux lumières de vis, même motif que la suspension ;
  - **applique** : platine murale (trou du câble, deux trous en serrure) et diffuseur qui s'y emboîte ;
  - **lampe à poser** en deux pièces : pied (avec passage de câble) et abat-jour, emboîtement par collier avec un paramètre de jeu (0,1 à 0,8 mm, 0,3 par défaut).
- Chaque modèle reprend un motif existant (ondulations, écailles, coussins) par un paramètre `choice`.
- Description de chaque modèle : ampoule LED conseillée, filament conseillé (PETG plutôt que PLA près d'une ampoule), et le rappel que l'installation électrique relève d'un électricien.

## Hors périmètre

- Composants électriques, câblage, normes : FabWeave ne fournit que des pièces imprimées.
- Rendu de la lampe allumée et montage de l'ampoule dans la vue : idée n°14 de l’analyse de ColiWorks du 17/11/2026, non retenue.
- Éprouvettes de jeu (idée n°9 de la même analyse) : non retenues ; le jeu reste un paramètre du modèle.
- Lampes modulaires (bagues de jonction, barres LED).

## Dépendances

- **Cotes des douilles du fablab** (E27 et E14 : diamètre du corps, de la bague, filetage, hauteur) : action `fournir` pour l'utilisateur. Sans elles, aucune valeur n'est inventée.
- R-020 (courbe) donnera des silhouettes libres à ces modèles s'il est livré avant ; sinon, curseurs.

## Critères d'acceptation

- Les quatre modèles se calculent aux valeurs par défaut et aux bornes sur les sept imprimantes en PLA et en PETG (test du catalogue).
- L'alésage E27 et l'alésage E14 ont, dans le parcours, le diamètre fourni à ±0,05 mm (test).
- Pied et abat-jour de la lampe à poser : diamètres d'emboîtement qui diffèrent exactement du jeu choisi (test).
- Aucun heurt du garde-buse aux valeurs par défaut.

## Vérifications de l'utilisateur

- Fournir les cotes des douilles E27 et E14 utilisées au fablab.
- Imprimer la suspension et y monter une douille E27.
- Imprimer la lampe à poser et juger l'emboîtement au jeu par défaut.
