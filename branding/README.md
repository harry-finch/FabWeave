# Identité visuelle

**Retenue le 24/10/2026** (v0.1, étape 5) : `fabweave-logo.svg`.

- **Tuile** commune à la famille Fab : carré arrondi graphite `#181f2a`, bord `#354759`, viewBox 128 × 128.
- **Symbole** : deux cercles ondulés entrelacés, 3 ondes chacun en opposition de phase, dessus-dessous alternés aux 6 croisements. Deux couches du vase à ondulations vues de dessus : le parcours qui se tisse. Variante « G — Entrelacs, 3 ondes » de [explorations/planche.html](explorations/planche.html), générée par `python3 branding/explorations/generate.py` (formes pleines, coordonnées arrondies au centième).
- **Couleur d'identité** : bleu FabCAM `#58B5F5` (FabCAM et FabWeave génèrent du G-code), `#438ABA` en thème clair (3:1 sur la barre claire). Contraste du symbole sur la tuile : 7,4:1. Proche du bleu d'action `--accent` (`#4b9cf5`, ΔE 18) : acceptable parce que l'identité ne sert qu'au logo et au liseré de 2 px, jamais à un état, une sélection ou une action.

| Fichier | Usage |
| --- | --- |
| `fabweave-logo.svg` | Source |
| `ui/public/logo.svg` | Barre d'application (24 px) |
| `ui/public/favicon.svg`, `favicon.ico` (16, 32, 48), `favicon-32.png`, `apple-touch-icon.png` (180) | Onglets et raccourcis ; PNG rendus directement à chaque taille depuis le SVG (ImageMagick, densité 0,75 × taille) |
| `--identity` dans `ui/src/app.css` | Liseré supérieur, deux thèmes |

Premier tour d'explorations (ondes, spirale, treillis, buse ; violet, rose corail, or pâle, turquoise) : historique Git, commit `189f292`. Icônes des installeurs : générées depuis le SVG avec l'application de bureau (v0.6).
