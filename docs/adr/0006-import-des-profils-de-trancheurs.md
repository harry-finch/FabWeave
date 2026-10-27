# ADR-0006 : Import des profils de trancheurs

Status: Proposed (à valider avec l'étape 4 de la v0.3)
Date: 2026-10-27

## Context

Les makers ont déjà leurs réglages dans PrusaSlicer (`.ini`), Bambu Studio ou OrcaSlicer (`.json`). FabWeave doit les reprendre sans rien compléter en silence, et sans exposer la machine.

Trois difficultés :

- **Les fichiers sont incomplets.** Un préréglage de l'utilisateur dans Bambu Studio ne contient que ses écarts à un préréglage du système (`inherits`), absent du fichier. Un profil de filament ne contient ni vitesse nominale ni température de palpage (dans le profil d'impression ou dans le gabarit de démarrage). PrusaSlicer exporte à plat imprimante, filament et impression mêlés.
- **Les gabarits de G-code n'ont pas notre syntaxe.** Les trancheurs emploient `[var]`, `{expr}` et des blocs `{if}…{elsif}…{else}…{endif}`, des noms de variables différents (`nozzle_temperature_initial_layer`, `first_layer_print_min[0]`), des réglages du fichier (`printer_notes`, `z_offset`), des expressions régulières (`=~ /.*PET.*/`) et des variables d'état (`layer_num`) qui n'existent pas au démarrage dans FabWeave.
- **Un gabarit mal traduit est dangereux.** Une ligne de démarrage sautée peut laisser le plateau froid, une buse non palpée ou une purge au mauvais endroit.

## Decision (proposée)

1. **Import dans le moteur** (`fabweave_core::import`, pur) : lecture du `.ini` (à plat ou en paquet, héritage résolu dans le fichier) et du `.json` ; un profil par préréglage d'imprimante ou de filament ; réglages d'impression refusés (`import.process_profile`).
2. **Profil de base** : chaque profil importé part d'un profil livré (même firmware, même matière). Les valeurs absentes en viennent, **sauf** ce qui est propre à une machine ou à un filament (zones exclues, buses proposées, couches par buse, cône, dégagement de la tête, vignettes, débits par buse), laissé vide. Un rapport liste chaque valeur reprise avec sa clé, chaque valeur absente avec ce qu'elle devient, les clés ignorées et les gabarits du trancheur sans équivalent.
3. **Gabarits traduits** vers les expressions Rhai de FabWeave (ADR-0004), avec quatre règles :
   - les variables connues sont renommées, l'extrudeur est toujours le 0 ;
   - un réglage présent dans le fichier est remplacé par sa valeur (il est constant pendant l'impression) et listé ;
   - les expressions régulières simples deviennent `contains`, `starts_with`, `ends_with` ou une égalité ;
   - ce qui ne se traduit pas reste **tel quel** et est listé avec sa ligne : le profil est alors en erreur (`profile.template_error`), ajouté quand même pour être corrigé dans l'éditeur, mais il ne se charge pas.

   Jamais de ligne commentée ou retirée en silence.
4. **Relecture exigée** : une imprimante importée porte `review = ["gcode.start", "gcode.end"]` dans son TOML. Tant qu'il en reste, le parcours se calcule mais le G-code n'est pas exporté (`profile.review_pending`). L'utilisateur lève chaque marque dans l'éditeur (« J'ai relu »). La marque voyage avec le fichier exporté.
5. **Profils de l'utilisateur** : TOML écrit par le moteur, mémorisés sur le poste à côté des profils livrés, jamais écrasés ; l'éditeur montre l'origine de chaque valeur et n'enregistre pas un profil en erreur.

## Alternatives considered

- **Garder les gabarits de la base** et ignorer ceux du fichier : sûr pour une machine connue, faux pour toute autre (un gabarit de MK4S sur une Ender-3). Rejeté.
- **Commenter les lignes non traduites** pour que le profil se charge : l'imprimante sauterait une commande sans que personne ne le voie. Rejeté.
- **Interpréter la syntaxe des trancheurs** dans le moteur de gabarits : deux langages à maintenir, et les variables d'état restent sans valeur. Rejeté : une traduction vers un seul langage, relue par l'utilisateur.
- **Refuser tout profil incomplet** : la plupart des préréglages de Bambu Studio le sont (`inherits`). Rejeté au profit du profil de base, dit dans le rapport.

## Consequences

- Les profils officiels reconstitués (MK4S + Generic PLA de PrusaSlicer, X1E de Bambu Studio) se traduisent entièrement ; un G-code exporté avec le profil PrusaSlicer importé, après relecture, reproduit les commandes du gabarit d'origine (tests `tests/import.rs`).
- Les accélérations d'un profil machine de Bambu Studio sont des maxima (20 000 mm/s² sur la X1E) : le temps estimé est optimiste tant qu'on ne les corrige pas ; le rapport donne la clé d'origine.
- Les gabarits de changement de couche, de filament, de pause des trancheurs ne sont pas repris (listés) : FabWeave n'en a pas l'équivalent.
- À confirmer : des fichiers réels exportés par des utilisateurs (OrcaSlicer en particulier), et une impression avec un profil importé.
