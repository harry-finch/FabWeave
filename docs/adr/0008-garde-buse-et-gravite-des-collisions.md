# ADR-0008 : Garde-buse et gravité des collisions

Status: Proposed (à valider avec le jalon v0.5)
Date: 2026-10-30

## Context

En non planaire, ou quand un parcours redescend (lampe à coussins, entretoise, pièces imprimées par morceaux), la buse et la tête peuvent toucher la matière déjà posée. La feuille de route demande de surligner ces mouvements, de les compter et de bloquer l'export, en laissant la gravité à cette décision. Les profils livrés ne donnent pas encore le cône de la buse (à mesurer au fablab) ; ils donnent le dégagement de la tête (`clearance_radius`, `clearance_height`, repris des trancheurs).

Un blocage à tort empêcherait d'imprimer une pièce valable : la lampe à coussins s'imprime alors que sa buse frôle le tour précédent sous les cols (0,28 mm au-delà de la marge).

## Decision

1. **Carte des hauteurs** dans le moteur, sur le parcours préparé (adhésion et copies comprises) : mailles de 0,25 mm (2 048 au plus par côté), le dépôt des deux dernières largeurs de boudin n'y entre qu'une fois la buse passée. Un parcours dont Z ne redescend jamais n'est pas examiné.
2. **Formes vérifiées** : le bout (méplat de rayon égal au diamètre de la buse, élargi du dessus plat du boudin voisin), le cône si le profil le donne, la tête dans son rayon. Les déplacements sont vérifiés à leur vraie hauteur, saut en Z compris ; pas les essuyages.
3. **Gravité** (marge = hauteur du boudin, ou de couche pour un déplacement) :
   - sous la marge : rien, le plastique chaud s'écarte ;
   - excès jusqu'à deux marges : `guard.nozzle_rub`, **avertissement** compté ;
   - au-delà : `guard.nozzle_collision`, **erreur** : mouvements surlignés, export refusé ;
   - tête : `guard.head_collision`, **erreur** dès qu'elle touche ;
   - cône inconnu et matière à portée : `guard.cone_unknown`, **info**.
4. Pas de dérogation pour forcer l'export. Le modèle ou le profil se corrige.

## Alternatives considered

- **Tout contact bloquant** : la lampe à coussins, en cours d'impression réussie, serait refusée. Rejeté.
- **Tout en avertissement** : contraire à la règle de sécurité machine pour un vrai heurt. Rejeté.
- **Cône par défaut pour les profils sans valeur** : un chiffre inventé donnerait de fausses certitudes. Rejeté : le message dit ce qui n'est pas vérifié.
- **Maillage 3D de la matière** au lieu d'une carte 2,5D : plus juste sous les surplombs, trop coûteux pour un calcul en direct. Écarté.

## Consequences

- Aucun coût pour les couches planes et les spirales ; lampe à coussins (170 000 points) : 55 ms sur 600 ms de calcul en natif.
- Limites : matière vue par le dessus seulement, pas d'affaissement, bloc de chauffe non modelé au-delà du cône, tête traitée comme un cylindre (prudent : peut refuser une pièce qui passerait).
- Aux bornes de certains curseurs, des modèles sont désormais refusés (lampe à coussins, descente de 50° ; porte-savon, bord de 40 mm ; entretoise, trou de 3 mm).
- Les seuils sont à confirmer par des impressions, et le cône à mesurer sur les machines du fablab.
