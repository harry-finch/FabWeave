---
id: R-024
titre: "Débit en porte-à-faux et couches variables"
resume: "Le moteur compense le débit des parois inclinées (compté et signalé) et les modèles choisissent une hauteur de couche selon la pente."
statut: specifiee
jalon: v0.7
cree: 2026-11-17
maj: 2026-11-17
livree:
---
## But

Quand une paroi penche, le boudin déborde dans le vide : vu perpendiculairement à la paroi, la couche s'amincit, la matière manque et la paroi se creuse ou s'affaisse. Des couches fixes gaspillent aussi du temps sur les parois droites et marquent les courbes en escalier. Le moteur compense le débit dans les porte-à-faux et laisse les modèles choisir une hauteur de couche variable selon la pente.

## Inclus

- **Compensation du débit en porte-à-faux** : réglage du profil filament de 0 à 100 % (0 % : comportement actuel). Le débit est corrigé selon l'inclinaison de la paroi, dans les plafonds de débit volumétrique qui ne sont jamais dépassés ; chaque mouvement compensé est **compté et signalé** (message d'information à code stable).
- **Hauteur de couche variable** : fonction de l'API qui donne les hauteurs successives d'une spirale selon la pente du profil, entre une couche minimale (courbes) et maximale (parois droites), bornées par la buse ; les modèles vases du catalogue l'offrent en option.
- Coloration du parcours par hauteur de couche (mode existant) pour voir le résultat.
- Profils filament : nouvelle version, ancien format relu (test), éditeur et import mis à jour.

## Hors périmètre

- Temps de couche minimal et ralentissement des petites couches (idée n°2 de l’analyse de ColiWorks du 17/11/2026, non retenue).
- Compensation de l'affaissement par la forme (abaisser la buse) : propre à chaque effet (R-021).
- Couches variables pour les pièces non vase (remplissage, couches pleines).

## Dépendances

Aucune. Les paramètres par défaut de compensation restent à 0 % tant que l'utilisateur ne les a pas validés par une impression.

## Critères d'acceptation

- À 0 %, le G-code de tout le catalogue est identique, octet pour octet, à celui d'avant (test).
- Paroi inclinée à 45°, compensation à 100 % : débit multiplié par le facteur attendu (test), dans la limite du débit volumétrique ; le message compte les mouvements.
- Hauteur variable : jamais sous la couche minimale ni au-dessus de 0,8 fois la buse ; parois verticales à la couche maximale (test).
- `cargo test --workspace`, Clippy et `npm run check` sans erreur.

## Vérifications de l'utilisateur

- Imprimer une même pièce en porte-à-faux (lampe à coussins ou défi porte-à-faux) avec 0 % puis 100 % de compensation, et comparer.
- Imprimer un vase en couches variables et juger les courbes et le gain de temps.
