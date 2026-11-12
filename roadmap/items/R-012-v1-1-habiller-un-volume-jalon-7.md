---
id: R-012
titre: "v1.1 — Habiller un volume (jalon 7)"
resume: "Importer un volume 3D (STL, 3MF) et l'habiller d'un motif du catalogue imprimé en une spirale continue."
statut: specifiee
jalon: v1.1
cree: 2026-10-29
maj: 2026-11-12
livree:
---
## But

Importer un volume 3D (STL, 3MF) et l'habiller d'un motif du catalogue (maille, ondulations, gouttes, abat-jour…) imprimé en une spirale continue, comme les modèles paramétriques. Formes visées : révolutions, polygones extrudés ou torsadés, formes organiques peu détaillées ; une seule paroi, jamais de tranchage.

## Inclus

1. **Décision** (ADR) : forme et motif dans le format `.weave` (motif = modèle qui reçoit une forme ; forme importée ou paramétrique), frontière avec le tranchage, version du format. ⏸ validation de l'ADR.
2. **Formes** (FORM-01 à FORM-04) : lecture STL et 3MF dans le moteur (octets donnés par l'interface), tranches, refus nommant la hauteur fautive (anse, branche, îles, maillage ouvert) ; formes paramétriques (révolution d'un profil, polygone extrudé et torsadé) pour les mêmes motifs sans fichier.
3. **Suivre la surface** (FORM-05) : spirale qui avance le long de la paroi et non en Z (porte-à-faux au-delà de 45°, descentes, comme la lampe à coussins), dégagement du cône de la buse vérifié par le garde-buse (SAFE-03).
4. **Motifs** (FORM-06) : maille, ondulations, écailles, gouttes, vermicelles, abat-jour réécrits en motifs ; fond plein ou maillé, bord plein ; angles vifs respectés.
5. **Interface** (FORM-07) : mode « Habiller un volume » : import par dépôt ou `⌘O`, aperçu du volume, choix de l'axe, motif et ses curseurs ; panneaux Machine, Plateau, Messages et Export inchangés.

## Hors périmètre

- Une seule paroi, jamais de tranchage (ancienne feuille de route, but du jalon).
- Découpage de modèles STL et supports automatiques : habiller la paroi d'un volume importé d'un motif n'en est pas, mais ni remplissage, ni couches pleines, ni pièces quelconques (section « Hors périmètre » de l'ancienne feuille de route).

## Dépendances

**Point de départ** : essai du 29/10/2026 sur la branche `essai/habillage` (DEVLOG ; format en section 8 de [WEAVE_FORMAT](../../docs/WEAVE_FORMAT.md)) : STL coupé en tranches, un contour fermé par hauteur, repère (u, z) aligné d'une tranche à l'autre par courbures puis distances (torsion suivie) ; motifs `.weave` avec `[meta] motif = true` qui reçoivent `form` ; maille et ondulations calculées sur quatre volumes d'essai (vase, hexagone, pentagone torsadé, forme organique) en ligne de commande.

- Décision (ADR) de l'étape 1, à valider par l'utilisateur avant le reste.
- Garde-buse (SAFE-03) pour le dégagement du cône de la buse (étape 3).
- Jalon placé après la v1.0 dans l'ancienne feuille de route (DEVLOG du 29/10/2026).

## Critères d'acceptation

- Chaque famille de forme lue et habillée par chaque motif sans message bloquant.
- Volume refusé avec la hauteur fautive quand il n'a pas un contour par hauteur.
- Volume de 200 000 triangles lu et tranché en moins d'une seconde dans le navigateur.
- Mêmes volume, motif, valeurs et profils → même G-code, octet pour octet.
- Une impression validée par famille.

## Vérifications de l'utilisateur

- Valider l'ADR de l'étape 1 (forme et motif dans le format `.weave`).
- Valider le jalon : un volume de chaque famille habillé et imprimé à l'atelier.
