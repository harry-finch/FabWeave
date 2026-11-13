# Figure de FabWeave

**Concept retenu après révision par l'utilisateur le 12/11/2026 : un vase ondulé sous une buse ; le pointeur règle l'amplitude des ondulations.** Figure `ripple-vase`, destinée à la carte FabBoard. Son usage dans l'application reste une idée distincte (R-019), sans emplacement choisi.

- Source : [ripple-vase.js](ripple-vase.js).
- Page autonome : [hairline-ripple-vase.html](hairline-ripple-vase.html). Ouvrir depuis le disque ; ajouter `?play=1` pour le parcours, `?theme=light` ou `?theme=dark` pour le thème.
- Métaphore : un vase ouvert et galbé, en couches ondulées, sous une buse à pointe conique. Le mouvement de gauche à droite augmente le relief ; quitter restitue le galbe de repos.
- Règles principales : zone de visée fixe (01), amplitude bornée (03), bord supérieur seul en trait vif (04), repos composé (05), ressort pour une entrée continue (08), solides arrondis (09).
- Parcours : relief faible à gauche, moyen au centre, fort à droite, puis retour au repos. Lecture hors dessin : « relief N », « rest » au repos.
- Fabrication : skill `hairline-create` de la copie locale de FabSpeed ; moteur et page d'essai incorporés à l'identique par `build.mjs`. Installation locale des skills toujours bloquée par les droits sur `.agents/`.

La première proposition [woven-nozzle.js](woven-nozzle.js) et sa [page](hairline-woven-nozzle.html) sont conservées comme exploration. Elles ne sont plus la figure de la carte.

## Contrôles du 12/11/2026

`build.mjs` et `validate.mjs` acceptent la nouvelle page. **La figure n'a pas été regardée dans un navigateur.** Le contrôle `look.mjs` n’a pas été réalisé lors de la session initiale. À la reprise, l’accès navigateur de l’outil est indisponible et le contrôle natif de Vivaldi est refusé ; aucun contrôle visuel supplémentaire n’a donc été effectué. La validation statique a été relancée avec succès. Elle reste provisoire dans `roadmap/projet.toml` ; A-011 porte les essais à reprendre.

Revue de `look.md` à partir du code, sans validation visuelle :

1. Lecture à 240 px non vérifiée ; galbe, ouverture et buse conique sont construits.
2. Repos : relief d'amplitude 3, vase galbé, buse décalée au-dessus du bord, lèvre en trait vif.
3. Réponse continue du vase entier ; amplitude bornée entre 1,2 et 9,2, sans propagation entre objets indépendants.
4. Zone fixe dans le repère du dessin ; aucune mesure des contours déplacés.
5. Anneaux échantillonnés pour la paroi ; corps de buse arrondi ; aucun coin vertical dessiné.
6. Bandes opaques de bas en haut, puis bouche et intérieur ; buse en dernier. Occultations à confirmer visuellement.
7. Seule la lèvre est vive ; aucun aplat ou changement de palette propre.
8. Lecture « relief N » sous le pointeur, « rest » au repos.
9. Cadre centré sur l'enveloppe du vase et la buse ; extrêmes à confirmer par look.mjs.
10. Classes neutres du moteur ; thèmes non regardés.
11. Aucun texte dans le dessin.
12. Validation statique conforme ; console navigateur non contrôlée.
13. État vide : sans objet, figure de métier.
14. Source SVG : sans objet.

Après installation de hairline-create dans ce projet, depuis ce dossier :

```sh
node ../../.agents/skills/hairline-create/look.mjs ripple-vase.js --answer 200,175 --edge 110,205 --edge 290,205
```

Regarder les planches et faire juger le parcours avec `?play=1` avant de retirer le commentaire provisoire.

## Licence

Figure propre au projet : GPL-3.0-or-later. Moteur Hairline et page d'essai : MIT, © Lucas Marques ; notice complète dans [Hairline-MIT.txt](Hairline-MIT.txt), à distribuer avec la page autonome.
