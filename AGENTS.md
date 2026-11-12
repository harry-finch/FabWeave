# Consignes pour les agents

> Ce fichier est repris dans CLAUDE.md.

FabWeave : générateur de **G-code direct** à partir de **modèles paramétriques** (catalogue de fichiers `.weave`), pour makers et fablabs, inspiré de FullControl (fullcontrol.xyz). Workspace Rust (`fabweave-core` pur, `fabweave-wasm` pour l'interface) + interface Svelte 5 dans `ui/`, même écosystème que FabSpeed et FabVents. Lire [README.md](README.md), [STATUS.md](STATUS.md), [ROADMAP.md](ROADMAP.md) (vue générée de `roadmap/`) et [docs/FEATURES.md](docs/FEATURES.md) avant de modifier.

## Skills à utiliser (`.agents/skills/`)

- `fab-design-system` : toute modification de l'interface ; jetons, composants, mise en page, textes et identité de la famille Fab, avec un kit de démarrage. Ses valeurs chiffrées font foi.
- `professional-workshop-ui` : les mêmes principes en termes généraux.
- `fab-roadmap` : la feuille de route est dans `roadmap/` (format FabBoard : un fichier par élément et par action de l'utilisateur) ; `ROADMAP.md` est généré, ne jamais le modifier à la main. Spécifier, tenir les statuts, créer une action quand on attend l'utilisateur, `fabboard roadmap lint` avant chaque commit. L'ancienne feuille de route écrite à la main est conservée dans `docs/ROADMAP-avant-fabboard.md`.
- `project-documentation` : STATUS, DEVLOG, ADR, FEATURES.

## Règles

- `fabweave-core` reste pur : aucune IO, aucun réseau, aucune dépendance d'interface ; déterministe (même modèle, mêmes paramètres, mêmes profils → même G-code, octet pour octet).
- Séparation stricte : un modèle `.weave` décrit **la géométrie et l'intention** (où passe la buse, quelle section de matière, quel facteur de vitesse) ; il ne parle jamais de `G0/G1`, de `E`, de températures, de rétraction ni de coordonnées du plateau. La physique et le dialecte viennent des profils imprimante et filament. Seule exception : un modèle d'étalonnage (`[meta] calibration = true`) peut décaler la température et changer la rétraction, dans des bornes fixes et toujours signalé ([ADR-0007](docs/adr/0007-reglages-d-essai-des-modeles-d-etalonnage.md)).
- Millimètres, degrés, secondes dans le moteur et dans les fichiers ; le repère du modèle est centré sur (0, 0), Z vers le haut ; le moteur place la pièce sur le plateau.
- Extrusion **relative** (`M83`) dans tout G-code produit.
- Les valeurs ne sont jamais corrigées en silence : une valeur hors domaine produit une erreur ou un avertissement à code stable (`param.out_of_range`, `flow.clamped`…). Une vitesse plafonnée (débit, axe Z), un segment fusionné, une collision sont **comptés et signalés**.
- Sécurité machine : aucun G-code n'est exporté si le parcours sort du volume d'impression ou si le script a échoué ; les plafonds du profil (débit volumétrique, vitesse Z, températures) ne sont jamais dépassés.
- Les scripts Rhai s'exécutent en bac à sable : limites d'opérations, de profondeur et de nombre de points ; aucun accès fichier ni réseau.
- Format `.weave` et profils versionnés (`format`, `version`) : toute évolution incompatible ajoute une migration et un test de lecture de l'ancien format. La spécification ([docs/WEAVE_FORMAT.md](docs/WEAVE_FORMAT.md)) et le prompt de génération par IA ([docs/PROMPT_MODELE_IA.md](docs/PROMPT_MODELE_IA.md), exemples vérifiés par les tests) sont mis à jour dans le même changement que le moteur.
- Modèles repris de FullControl (GPL-3.0) : citer la source (`[meta] source`, `license`) dans le fichier `.weave`.
- Les identifiants de `docs/FEATURES.md` sont stables ; mettre à jour l'état d'une fonction dans le même changement que son code.
- Interface : aucune couleur en dur dans les composants ni dans le rendu 3D, tout passe par les jetons de `ui/src/app.css` ; textes par `t('clé')`, `fr.ts` est la référence et `en.ts` a les mêmes clés.
- Mettre à jour la documentation dans le même changement que le comportement.

## Commandes

- Tests : `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cd ui && npm run check`.
- Moteur pour l'interface : `cd ui && npm run wasm` (wasm-pack, sortie dans `ui/src/wasm/`, hors Git).
- Développement : `cd ui && npm run dev` → http://localhost:5175 (relancer `npm run wasm` après une modification du moteur).
- Mesures : `cargo run --release -p fabweave-core --example bench`, `cd ui && npm run bench`.
- Vérifier un modèle : `cargo run --release -p fabweave-cli -- check <modèle.weave>` (aussi `stats`, `export -o <sortie.gcode>`, options `--printer`, `--filament`, `--set id=valeur`, `--json`).
- Build : `cd ui && npm run build` (moteur compris), puis `npm run preview`.
- Application macOS : `cd ui && npm run desktop` (sortie dans `crates/fabweave-desktop/target/release/bundle/`). Le crate est hors du workspace : les tests ci-dessus ne le compilent pas.
