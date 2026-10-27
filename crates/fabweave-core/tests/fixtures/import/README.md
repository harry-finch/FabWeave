# Fichiers d'essai de l'import

Réglages réels des trancheurs, servant aux tests de `tests/import.rs` :

- `prusaslicer_mk4s_pla.ini` : export d'une configuration à plat (« Exporter la configuration ») de PrusaSlicer, reconstitué depuis les préréglages « Original Prusa MK4S 0.4 nozzle », « Generic PLA @PG » et « 0.20mm SPEED @MK4S 0.4 » de `PrusaResearch.ini` (prusa3d/PrusaSlicer-settings-prusa-fff, AGPL-3.0).
- `bambu_x1e_machine.json` : préréglage machine « Bambu Lab X1E 0.4 nozzle » de Bambu Studio (bambulab/BambuStudio, `resources/profiles/BBL`, AGPL-3.0), parents résolus, enregistré comme préréglage de l'utilisateur.
- `bambu_pla_user.json` : préréglage filament de l'utilisateur tel que Bambu Studio l'exporte (écarts seulement, `inherits` vers un préréglage du système).
- `bambu_process.json` : réglages d'impression (sans équivalent dans FabWeave : refusés).
