---
id: R-005
titre: "v0.4 — Visualisation et analyse (jalon 4)"
resume: "Rendu en volume, coloration, avancement et coupe en Z, inspection du segment survolé, image de la vue et estimation du temps."
statut: livree
jalon: v0.4
cree: 2026-10-24
maj: 2026-11-12
livree: 2026-10-30
---
*Validée le 30/10/2026.*

Commencée le 28/10/2026 pendant les impressions de validation de la v0.3.

Étapes et points d'arrêt :

1. **Rendu en volume** (VIEW-06) : boudins à leur largeur et leur hauteur, éclairage simple, bascule volume / lignes mémorisée. *Fait le 28/10/2026 : 200 195 segments en 12,3 ms par image (M4).*
2. **Coloration et légende** (VIEW-07). *Fait le 28/10/2026 : huit grandeurs, rampe violette validée dans les deux thèmes.*
3. **Curseur de progression**, buse simulée, coupe en Z (VIEW-08). *Fait le 29/10/2026 : barre de lecture en bas de la vue, coupe en Z par plan de coupe à droite, déplacements affichables ; plages de dessin, rien de reconstruit en avançant.*
4. **Inspection du segment survolé** (VIEW-09). *Fait le 29/10/2026 : choix à l'écran sans lancer de rayons, fiche à côté du pointeur ; le moteur transmet vitesse demandée, plafond et durée de chaque mouvement.*
5. **Export PNG** de la vue (VIEW-10). *Fait le 29/10/2026 : vue rendue à plus haute définition (≈ 3 000 px), sans les étiquettes.*
6. **Temps estimé** comparé à PrusaSlicer sur le catalogue. *Préparé le 29/10/2026 (`.tmp/essais-v0.4/`), comparaison non relevée : critère de ± 5 % non vérifié, reste ouvert.*
7. ⏸ Validation de jalon. *Validée par l'utilisateur le 30/10/2026.*

- **Rendu volumétrique** : boudins à section oblongue W × H, éclairage simple ; mode lignes pour les très grands parcours.
- **Coloration** au choix : type de mouvement (impression, pont, déplacement), vitesse obtenue, débit volumétrique, largeur W, hauteur H, ventilateur, Z, ordre d'impression ; légende graduée.
- **Curseur de progression** (couche, pourcentage, temps), simulation de la buse, déplacements affichables, coupe en Z.
- **Inspection** du segment survolé : W, H, vitesse demandée et obtenue, débit, raison d'un plafonnement.
- Estimation du temps avec accélérations (planificateur trapézoïdal simplifié).
- Export PNG de la vue.

Critères d'acceptation : 200 000 segments affichés en volumétrique à 60 images/s sur un ordinateur d'atelier ; temps estimé à ± 5 % de PrusaSlicer sur le catalogue.
