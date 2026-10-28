# ADR-0007 : Réglages d'essai des modèles d'étalonnage

Status: Accepted (choix de l'utilisateur, 28/10/2026)
Date: 2026-10-28

## Context

Un modèle `.weave` décrit la géométrie et l'intention ; il ne parle jamais de températures ni de rétraction, qui viennent des profils (règle du projet, AGENTS.md). Or deux modèles d'étalonnage de la v0.3 ne se font qu'en changeant ces réglages pendant l'impression :

- la **tour de température** (CAT-22) change la température de buse d'un étage à l'autre ;
- le **test des rétractions** (CAT-15, d'après FullControl) fait varier la longueur ou la vitesse de rétraction, ou le saut en Z, d'une colonne à l'autre.

Le **test de débit** (CAT-22) n'en a pas besoin : il ne joue que sur la vitesse demandée, et le moteur plafonne toujours au débit maximal du profil.

## Decision

Une exception encadrée, choisie par l'utilisateur parmi trois propositions :

1. Un modèle se déclare **modèle d'essai** : `[meta] calibration = true`. Lui seul peut appeler les réglages d'essai ; ailleurs, le calcul est refusé (`model.calibration_only`).
2. **Réglages d'essai**, valables à partir du point suivant, chacun borné (erreur à code stable, jamais de correction) :
   - `path.set_temperature_offset(°C)` : décalage de la consigne de buse par rapport à celle du filament (première couche ou suivantes), ±30 °C au plus ; `path.reset_temperature()` ;
   - `path.set_retraction_length(mm)` (0 à 8), `path.set_retraction_speed(mm/s)` (5 à 100), `path.set_z_hop(mm)` (0 à 2) : chacun remplace la valeur du filament, les autres restent celles du filament (le modèle ne les connaît pas) ; `path.reset_retraction()`.
3. **Visibles** : un modèle d'essai porte toujours l'avertissement `model.calibration` ; chaque changement de température est écrit `M104 S… ; essai : ±n °C` ; le modèle annote ses étages ou ses colonnes.
4. **Sûrs** : le filament retiré est toujours remis à la même longueur, même si la rétraction d'essai change entre les deux ; la durée estimée compte la rétraction d'essai de chaque déplacement ; les plafonds de vitesse et de débit du profil restent appliqués.

## Alternatives considered

- **Outil de calibration dans l'application**, hors des modèles : la règle reste intacte, mais c'est un second moyen de produire du G-code, avec son interface à part. Écarté pour l'instant.
- **Reporter** les deux modèles : la v0.3 doit servir à régler un nouveau filament au fablab.
- **Laisser tout modèle changer température et rétraction** : un modèle pourrait alors contredire le profil sans que l'utilisateur le voie. Rejeté.

## Consequences

- Les réglages d'essai sont des valeurs par point du parcours (`WayPoint::temp_offset`, `retraction`), lues par le planificateur et l'écriture du G-code.
- Le prompt pour IA ne les présente pas : un modèle ordinaire n'en a pas l'usage.
- Les bornes sont fixes, pas tirées du profil (qui n'a ni température ni rétraction maximales) ; à revoir si un filament demande plus de ±30 °C.
