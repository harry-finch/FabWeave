# ADR-0004 : Gabarits de G-code des profils en expressions Rhai

Status: Accepted
Date: 2026-10-24

## Context

Les G-code de démarrage et de fin des imprimantes ne sont pas du texte fixe. Celui de la Prusa MK4S (profil officiel de PrusaSlicer) calcule la zone de palpage d'après l'encombrement de la première couche (`M555 X{…} Y{…} W{…} H{…}`), choisit une température de palpage et une rétraction selon la matière, n'allume le ventilateur que si le plateau est sous 60 °C, et place la tête en fin d'impression à `min(max_layer_z + 1, max_print_height)`. PrusaSlicer a pour cela son propre langage de macros (`[var]`, `{if …}{endif}`, `min`, tableaux par extrudeur). Un simple remplacement de `{variable}` ne suffit pas.

## Decision

- Un gabarit est du texte libre où chaque `{ … }` (accolades équilibrées, hors chaînes) est une **expression Rhai**, évaluée avec les variables de l'impression (températures, encombrement de la première couche et de la pièce, buse, matière…) en constantes. `{{` / `}}` écrivent une accolade.
- Valeurs écrites : nombres à 3 décimales au plus (zéros retirés), entiers tels quels, booléens en `1` / `0`, rien pour `()` (un `if` sans `else` faux).
- Moteur Rhai séparé de celui des modèles, limité (100 000 opérations), variables strictes : chaque gabarit est **vérifié à la lecture du profil** avec des valeurs d'exemple ; une variable inconnue ou une syntaxe invalide est l'erreur `profile.template_error` (champ, ligne dans le gabarit, message).
- Les gabarits Prusa livrés reprennent ceux de PrusaSlicer (PrusaResearch 2.5.13) ligne à ligne, macros réécrites, source citée dans le profil.

## Alternatives considered

- **Remplacement de `{variable}` seul** (proposition de la recherche) : impossible de reproduire le palpage par zone ni les choix selon la matière ; il faudrait des variables précalculées propres à chaque machine (`m555_x`…), dans le moteur.
- **Réimplémenter le langage de macros de PrusaSlicer** : copier les profils tels quels, mais un langage entier à maintenir, lié aux tableaux par extrudeur de PrusaSlicer.
- **Mini-langage arithmétique maison** : à écrire, tester et documenter, moins expressif que Rhai, déjà embarqué.

## Consequences

Une seule syntaxe d'expression à apprendre dans FabWeave (celle des modèles). Les profils PrusaSlicer, OrcaSlicer ou Bambu Studio se portent à la main (macros réécrites) ; un convertisseur pourra venir plus tard. Le rendu d'un gabarit coûte quelques millisecondes à l'export, négligeable.

## Validation

Gabarits MK4S et CORE One vérifiés au chargement (tests) ; `M555` calculé pour le vase à ondulations égal à la formule de PrusaSlicer (relu à la main) ; impression de validation de la v0.1. Revoir si un firmware demande une construction impossible à exprimer en une expression (boucles sur plusieurs extrudeurs, par exemple pour l'AMS de Bambu).
