---
id: R-003
titre: "v0.2 — Catalogue et format complet (jalon 2)"
resume: "Le catalogue devient le cœur de l'application et le format .weave est assez riche et documenté pour qu'une IA produise des modèles valides du premier coup."
statut: livree
jalon: v0.2
cree: 2026-10-24
maj: 2026-11-12
livree: 2026-10-27
---
*Validée le 27/10/2026 ; trois modèles imprimés.*

**But** : le catalogue devient le cœur de l'application, et le format `.weave` est assez riche et assez documenté pour qu'une IA externe produise des modèles valides du premier coup.

Étapes et points d'arrêt :

1. **Format et API** : paramètres `choice`, messages du modèle (`warn`, `fail`), annotations, ponts, API native de points inspirée de FullControl (formes, transformations, modulation), mesure du gain.
   ⏸ Validation de l'API et de la spécification (avant d'écrire le catalogue et le prompt pour IA).
2. **Pour l'IA** : `fabweave-cli check | export | stats`, `docs/PROMPT_MODELE_IA.md`, essai de génération de trois modèles à partir du seul prompt. *Fait le 24/10/2026 : trois modèles sur trois valides au premier essai.*
3. **Interface du catalogue** : catégories, recherche, vignettes, fiche du modèle, annotations dans la vue 3D, ouverture et glisser-déposer de `.weave`, « Mes modèles ». *Fait le 27/10/2026.*
4. **Catalogue de départ** d'environ dix modèles. *Fait le 27/10/2026 : onze modèles (CAT-01 à CAT-11, fond des vases CAT-23) ; l'impression par gouttes (CAT-12) passe en v0.3, faute d'extrusion à l'arrêt dans l'API (API-15).*
5. ⏸ Validation de jalon : trois modèles imprimés. *Validé le 27/10/2026.*

- **Catalogue** : arbre par catégories, recherche, étiquettes, vignettes rendues par l'application, fiche du modèle (description, auteur, source et licence, buse recommandée, temps et matière estimés avec les profils courants).
- **Format v1 complet** : paramètres `choice` (liste), unités, pas, groupes et `advanced = true` (« ▸ Avancé »), libellés et infobulles FR / EN, avertissements et erreurs émis par le script avec un code (`warn("model.thin_wall", …)`), annotations 3D (`note(x, y, z, "texte")`, comme les repères « Start », « End » de FullControl), points de départ et de fin.
- **API haut niveau en Rust natif** (rapide, et moins d'erreurs pour une IA) : segments, arcs, cercles, polygones, spirales et hélices, ondes polaires, zigzags, copies (translation, rotation, miroir, répétition en Z), modulation en Z d'un tracé plat, subdivision, `bridge_to`, essuyage.
- **Modèles de l'utilisateur** : ouvrir ou glisser-déposer un `.weave`, « Mes modèles » mémorisés localement, erreurs du script montrées avec la ligne ; rechargement à chaud du dossier `models/` en développement.
- **Pour l'IA** : `docs/WEAVE_FORMAT.md` (spécification), `docs/PROMPT_MODELE_IA.md` (prompt système prêt à copier, avec deux exemples complets) et `fabweave-cli check | export | stats` pour qu'un agent vérifie son fichier sans ouvrir l'application.
- **Fond des vases** : option « Fond » (aucun, 1 ou 2 couches) sur les vases du catalogue, à commencer par le vase à ondulations : couches pleines qui épousent le contour de la base (branches, ondulations comprises), puis la spirale démarre au-dessus sans couture. Repose sur une primitive native de **remplissage d'un contour fermé** (lignes en zigzag croisées d'une couche à l'autre, faites ; anneaux concentriques en v0.3), réutilisable par les autres modèles.
- **Catalogue de départ** d'environ dix modèles (liste `CAT` de [FEATURES.md](../../docs/FEATURES.md)), dont le tamis à ponts tendus, l'entretoise et le ressort non planaires ; ports de FullControl cités avec leur source.

Critères d'acceptation :

- Chaque modèle du catalogue a un test moteur : déterminisme, pièce dans le plateau, aucun message bloquant aux valeurs par défaut, aucun plantage aux bornes de chaque curseur.
- Trois modèles nouveaux générés par une IA à partir du seul prompt passent `fabweave-cli check` au premier ou au deuxième essai.
- ⏸ Validation de jalon : trois modèles du catalogue imprimés.
