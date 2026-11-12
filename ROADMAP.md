<!-- Généré par FabBoard depuis roadmap/ : ne pas modifier à la main. -->

**But de la v1.0** : une application de bureau (et une version web sans installation) qui reprend le principe de FullControl pour les makers et les fablabs : un **catalogue de modèles paramétriques** dont on règle les curseurs, un **aperçu 3D en direct** du parcours de la buse, et un **G-code direct** adapté à l'imprimante et au filament choisis, sûr pour la machine. Les modèles sont des fichiers `.weave` (en-tête TOML + script Rhai) faciles à écrire à la main ou à faire générer par une IA.

Ce que FabWeave apporte par rapport à FullControl (site et bibliothèque Python) :

- **calcul immédiat** : pas de bouton « Regenerate », le parcours suit le curseur ;
- **profils imprimante et filament séparés du modèle** : G-code de démarrage et de fin, dialecte du firmware, plafonds de débit et de vitesse Z, rétraction ; le même modèle passe d'une Ender 3 en PLA à une Core One en PETG sans retoucher le modèle ;
- **un moteur honnête** : chaque plafonnement, fusion ou collision est compté et signalé ;
- **un format de modèle déclaratif** qu'une IA peut produire sans erreur, vérifiable hors de l'application (CLI).

**Principes** (détail dans [AGENTS.md](AGENTS.md)) :

- moteur Rust pur et déterministe (`fabweave-core`), testé, sans IO ni interface ;
- même écosystème que FabSpeed et FabVents : Svelte 5, skill `fab-design-system`, français et anglais, thème sombre de référence et thème clair, Tauri pour le bureau ;
- les valeurs ne sont jamais corrigées en silence : le moteur signale, l'interface montre ;
- millimètres en interne ; extrusion relative ;
- un jalon = une version testable, livrée avec sa liste de vérifications, **imprimée à l'atelier** avant d'être validée.

La source fonctionnelle est l'inventaire de [docs/FEATURES.md](docs/FEATURES.md) (chaque fonction y a son identifiant et son jalon). Les points d'arrêt **⏸** sont les moments où le travail s'interrompt pour une validation par l'utilisateur.

**Avancement : 4 livrés sur 15** (27 %)

## Restant à faire

1. **[R-004 v0.3 — Imprimantes et filaments (jalon 3)](roadmap/items/R-004-v0-3-imprimantes-et-filaments-jalon-3.md)** · À tester · jalon v0.3
   Chaque machine du fablab a un profil fiable, et l'utilisateur peut créer le sien.
2. **[R-006 v0.5 — Sécurité et qualité du parcours (jalon 5)](roadmap/items/R-006-v0-5-securite-et-qualite-du-parcours-jalon-5.md)** · À tester · jalon v0.5
   Garde-buse, qualité des segments et contrôles du parcours, avec la gravité de l'ADR-0008 à valider.
3. **[R-007 v0.6 — Bureau et projets (jalon 6)](roadmap/items/R-007-v0-6-bureau-et-projets-jalon-6.md)** · En cours · jalon v0.6
   Application de bureau Tauri 2 avec dossier de modèles surveillé, projet .fabweave.json et CLI complète.
4. **[R-008 v0.9-beta1 — Bêta pour macOS](roadmap/items/R-008-v0-9-beta1-beta-pour-macos.md)** · À tester · jalon v0.9-beta1
   Publier une bêta avec l'application macOS pour tester et valider tout ce qui reste ouvert (v0.3, v0.4, v0.5).
5. **[R-009 Audit technique, lot 2](roadmap/items/R-009-audit-technique-lot-2.md)** · Idée
   Budgets de calcul, CI, sauvegardes, identifiants des modèles personnels, export CLI atomique.
6. **[R-010 Audit technique, lot 3](roadmap/items/R-010-audit-technique-lot-3.md)** · Idée
   Moteur relançable, CSP, tests de l'interface, notarisation.
7. **[R-011 v1.0 — Publication](roadmap/items/R-011-v1-0-publication.md)** · Idée · jalon v1.0
   Installeurs signés, version web statique, documentation utilisateur FR / EN et catalogue d'au moins 15 modèles imprimés au fablab.
8. **[R-012 v1.1 — Habiller un volume (jalon 7)](roadmap/items/R-012-v1-1-habiller-un-volume-jalon-7.md)** · Spécifiée · jalon v1.1
   Importer un volume 3D (STL, 3MF) et l'habiller d'un motif du catalogue imprimé en une spirale continue.
9. **[R-013 Envoi à l'imprimante (DESK-05)](roadmap/items/R-013-envoi-a-l-imprimante-desk-05.md)** · Idée
   Envoi du G-code à l'imprimante (PrusaLink, OctoPrint, Moonraker), optionnel, après la v1.0.
10. **[R-014 Serveur MCP (DESK-06)](roadmap/items/R-014-serveur-mcp-desk-06.md)** · Idée
   Serveur MCP (stdio) pour qu'un agent IA crée, vérifie, prévisualise et exporte un modèle, après la v1.0.
11. **[R-015 v2 — Atelier de formes](roadmap/items/R-015-v2-atelier-de-formes.md)** · Idée · jalon v2
   Espace pour jouer et prototyper de nouvelles formes sur le format .weave, avec éditeur de code intégré.

## Actions à faire

- [ ] [A-001](roadmap/actions/A-001-imprimer-un-modele-valide-sur-la-core-one-jalon.md) Imprimer un modèle validé sur la CORE One (jalon v0.3) (tester, R-004)
- [ ] [A-002](roadmap/actions/A-002-essayer-la-bambu-lab-x1e-par-la-carte-sd-en.md) Essayer la Bambu Lab X1E par la carte SD, en .gcode et en .gcode.3mf (jalon v0.3) (tester, R-004)
- [ ] [A-003](roadmap/actions/A-003-relever-la-comparaison-des-temps-estimes-avec.md) Relever la comparaison des temps estimés avec PrusaSlicer (critère v0.4) (tester, R-005)
- [ ] [A-004](roadmap/actions/A-004-valider-le-jalon-v0-5-dont-la-gravite-retenue.md) Valider le jalon v0.5, dont la gravité retenue dans l'ADR-0008 (valider, R-006)
- [ ] [A-005](roadmap/actions/A-005-mesurer-le-cone-des-buses-du-fablab.md) Mesurer le cône des buses du fablab (fournir, R-006)
- [ ] [A-006](roadmap/actions/A-006-imprimer-la-seconde-version-de-la-lampe-a.md) Imprimer la seconde version de la lampe à coussins (tester, R-006)
- [ ] [A-007](roadmap/actions/A-007-essayer-l-application-macos-de-la-beta-v0-9.md) Essayer l'application macOS de la bêta v0.9-beta1 (tester, R-008)
- [ ] [A-008](roadmap/actions/A-008-essayer-l-installeur-windows-nsis.md) Essayer l'installeur Windows (NSIS) (tester, R-007)

## Livré

### v0.0 : Fondations

- [R-001 v0.0 — Fondations](roadmap/items/R-001-v0-0-fondations.md) · livrée le 2026-10-24
  Dépôt Git, consignes, feuille de route, inventaire des fonctions et décisions d'architecture (ADR-0001 à 0003).

### v0.1 : Tranche verticale

- [R-002 v0.1 — Tranche verticale (jalon 1)](roadmap/items/R-002-v0-1-tranche-verticale-jalon-1.md) · livrée le 2026-10-24
  Choisir un modèle, bouger ses curseurs, voir le parcours en 3D et imprimer le G-code exporté sur une machine du fablab, de bout en bout.

### v0.2 : Catalogue et format complet

- [R-003 v0.2 — Catalogue et format complet (jalon 2)](roadmap/items/R-003-v0-2-catalogue-et-format-complet-jalon-2.md) · livrée le 2026-10-27
  Le catalogue devient le cœur de l'application et le format .weave est assez riche et documenté pour qu'une IA produise des modèles valides du premier coup.

### v0.4 : Visualisation et analyse

- [R-005 v0.4 — Visualisation et analyse (jalon 4)](roadmap/items/R-005-v0-4-visualisation-et-analyse-jalon-4.md) · livrée le 2026-10-30
  Rendu en volume, coloration, avancement et coupe en Z, inspection du segment survolé, image de la vue et estimation du temps.

## Écarté

- [R-016 Découpage de modèles STL et supports automatiques](roadmap/items/R-016-decoupage-de-modeles-stl-et-supports.md)
  Hors périmètre : c'est le métier de PrusaSlicer, Orca, Cura.
- [R-017 Comptes, nuage, place de marché](roadmap/items/R-017-comptes-nuage-place-de-marche.md)
  Hors périmètre du projet.
- [R-018 Envoi réseau vers les Bambu Lab et gestion de l'AMS](roadmap/items/R-018-envoi-reseau-vers-les-bambu-lab-et-gestion-de-l.md)
  Hors périmètre : MQTT, compte Bambu et multi-couleurs (AMS).
