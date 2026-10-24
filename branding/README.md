# Identité visuelle

**En cours de choix** (v0.1, étape 5). Le logo actuel de l'interface (`ui/public/logo.svg` : tuile de la famille et disque violet) et la couleur d'identité violette (`--identity` : `#A27DFF`, `#7A52D9` en thème clair) sont **provisoires**.

## Explorations

[explorations/planche.html](explorations/planche.html) : quatre variantes de symbole sur la tuile commune de la famille (graphite `#181f2a`, bord `#354759`), de 128 à 16 px, sur fond sombre et clair, en monochrome et dans la barre d'application, et quatre couleurs candidates comparées aux couleurs de sens et aux identités de la famille. Généré par `python3 branding/explorations/generate.py` (formes pleines, contours en arcs SVG exacts).

| Variante | Idée |
| --- | --- |
| A — Ondes tissées | Trois couches ondulées décalées d'une demi-onde, en silhouette de vase |
| B — Spirale | Le parcours continu du mode vase, d'un seul trait |
| C — Treillis | Bandes croisées dessus-dessous : le tamis, le tissage |
| D — Buse | Une buse qui trace une onde : le G-code direct |

| Couleur | Sombre / clair | Contraste sur la tuile | Couleurs les plus proches (ΔE) |
| --- | --- | --- | --- |
| Violet | `#A27DFF` / `#8F6EE0` | 5,5:1 | FabSpeed 30, accent 40 |
| Rose corail | `#FF8BA0` / `#BF6878` | 7,5:1 | erreur 27, FabSpeed 60 |
| Or pâle | `#E6CF73` / `#918248` | 10,7:1 | KiPartManager 14, attention 26 |
| Turquoise profond | `#2FB8A6` / `#269385` | 6,7:1 | succès 23, information 25 |

Une fois le choix fait : `branding/fabweave-logo.svg` (source), `ui/public/logo.svg` et `favicon.svg`, `--identity` dans les deux thèmes de `ui/src/app.css`, et ce fichier mis à jour (variante, date, couleurs).
