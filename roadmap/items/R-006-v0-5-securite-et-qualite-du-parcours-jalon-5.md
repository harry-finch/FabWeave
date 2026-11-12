---
id: R-006
titre: "v0.5 — Sécurité et qualité du parcours (jalon 5)"
resume: "Garde-buse, qualité des segments et contrôles du parcours, avec la gravité de l'ADR-0008 à valider."
statut: a-tester
jalon: v0.5
cree: 2026-10-24
maj: 2026-11-12
livree:
---
*Étapes de code faites, en attente de validation (STATUS.md).*

Commencée le 30/10/2026.

Étapes et points d'arrêt :

1. **Garde-buse** (SAFE-03, ADR-0008). *Fait le 30/10/2026 : bout, cône et tête ; heurt bloquant, frôlement signalé ; aucun heurt sur le catalogue aux valeurs par défaut, sans cône dans les profils livrés tant qu'il n'est pas mesuré.*
2. **Qualité des segments** (SAFE-04, API-11). *Fait le 30/10/2026 : `pts.simplify(tolérance)`, choisi par le modèle et jamais appliqué d'office ; segments trop courts et saturation du planificateur signalés.*
3. **Contrôles du parcours** (SAFE-05). *Fait le 30/10/2026 : porte-à-faux, ponts, section du boudin, première couche ; avertissements comptés, seuils d'atelier fixes. « Trop rapide » en première couche existait déjà : vitesse plafonnée et comptée.*
4. ⏸ Validation de jalon, dont la gravité retenue dans l'ADR-0008.

- **Garde-buse** : carte de hauteur de la matière déposée, détection des collisions du cône de la buse et du carter en non planaire ; segments fautifs surlignés, comptés, export bloqué (ADR à venir sur la gravité).
- **Discrétisation adaptative** (tolérance de corde) dans l'API haut niveau ; fusion des points alignés ; longueur minimale de segment et nombre de segments par seconde maximal (saturation du planificateur, « bégaiement »), signalés.
- **Contrôles** : porte-à-faux au-delà d'un angle, ponts trop longs pour le filament, largeur incohérente avec la buse, première couche trop fine ou trop rapide.

Critères d'acceptation : cas de collision connus détectés (tests) ; aucun faux positif sur le catalogue.
