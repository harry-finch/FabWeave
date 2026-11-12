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
