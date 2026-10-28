# Format `.weave` et profils — spécification

> **v0.2 (27/10/2026)** : premier jet validé à la v0.1 ; ajouts de la v0.2 (paramètres `choice`, messages du modèle, annotations, ponts, listes de points natives) validés le 24/10 ; ajoutés à l'étape 4 : remplissage d'un contour (`fill_lines`), `slice`, arrondis acceptant un entier. Format `fabweave.model`, version 1 (les ajouts sont compatibles). Décisions : [ADR-0002](adr/0002-format-weave-toml-et-rhai.md) (format), [ADR-0004](adr/0004-gabarits-g-code-en-expressions-rhai.md) (gabarits des profils).

Un modèle FabWeave est un fichier texte `.weave` qui décrit **où passe la buse** et **quelle section de matière elle dépose**. Il ne parle jamais de G-code, de valeur `E`, de températures, de rétraction ni de coordonnées du plateau : tout cela vient des profils imprimante et filament choisis dans l'application. Le même modèle s'imprime donc sur une Prusa MK4S en PLA comme sur une CORE One en PETG, sans retouche.

## 1. Structure du fichier

```text
format = "fabweave.model"      ← facultatif
version = 1                    ← facultatif (absent : 1)

[meta]                         ← description du modèle
…
[params.rayon]                 ← un curseur par paramètre, dans l'ordre voulu
…
---                            ← ligne de séparation (seule sur sa ligne)
let path = new_path();         ← script Rhai qui trace le parcours
…
path                           ← le script se termine par le parcours
```

- L'en-tête est en [TOML](https://toml.io). Il est lu **strictement** : un champ inconnu (faute de frappe comme `defualt`) est une erreur, avec la ligne et la colonne.
- La première ligne composée uniquement de `---` sépare l'en-tête du script. Une ligne `---` en tout début de fichier (style *frontmatter*) est acceptée et ignorée.
- Encodage UTF-8. Les textes affichés peuvent être donnés dans une langue (`"Rayon"`) ou par langue (`{ fr = "Rayon", en = "Radius" }`) ; le français est la référence.

### Exemple minimal complet

```toml
[meta]
name = { fr = "Cylindre en spirale", en = "Spiral cylinder" }
category = { fr = "Essais", en = "Tests" }
description = { fr = "Paroi simple imprimée en spirale continue.", en = "Single wall printed as one continuous spiral." }

[params.radius]
type = "float"
label = { fr = "Rayon", en = "Radius" }
unit = "mm"
default = 20
min = 5
max = 60
step = 0.5

[params.height]
type = "float"
label = { fr = "Hauteur", en = "Height" }
unit = "mm"
default = 30
min = 2
max = 150
---
let w = printer.nozzle_diameter * 1.2;          // largeur du boudin
let h = printer.layer_height;                    // montée par tour
let turns = floor(params.height / h).to_int();
let steps = 120;                                 // points par tour
let path = new_path();
for i in 0..turns * steps {
    let t = i / steps.to_float();                // tours écoulés (flottant !)
    path.print_polar(params.radius, t * TAU, printer.first_layer_height + t * h, w, h);
}
path
```

## 2. En-tête

### Racine

| Clé | Type | Rôle |
| --- | --- | --- |
| `format` | texte | `"fabweave.model"` ; autre valeur : erreur `model.wrong_format`. Facultatif. |
| `version` | entier | Version du format ; absent = 1. Une version plus récente que le moteur : `model.unsupported_version`. |

### `[meta]`

| Clé | Type | Obligatoire | Rôle |
| --- | --- | --- | --- |
| `name` | texte | oui | Nom dans le catalogue. |
| `category` | texte | oui | Rubrique du catalogue (« Vases et luminaires », « Mécanique »…). |
| `description` | texte | non | Deux à quatre phrases : ce que fait le modèle, ce qui le rend intéressant, ses limites. |
| `author` | chaîne | non | Auteur ; pour un port : « Auteur original ; port FabWeave ». |
| `tags` | liste de chaînes | non | Mots-clés (`["vase", "spirale"]`). |
| `source` | chaîne | non | URL d'origine (obligatoire pour un modèle repris d'ailleurs). |
| `license` | chaîne | non | Licence d'origine (`"GPL-3.0"` pour FullControl). |
| `recommended_nozzle` | liste de nombres | non | Diamètres de buse conseillés (mm). Une autre buse produit l'avertissement `nozzle.not_recommended`. |
| `recommended_adhesion` | `"none"`, `"brim"` ou `"raft"` | non | Adhésion conseillée, rappelée à l'utilisateur ; jamais appliquée d'office (le choix est le sien). |

« texte » = chaîne ou table par langue.

### `[params.<id>]`

`<id>` : minuscules, chiffres et `_`, commençant par une lettre ou `_` (`inner_radius`). Il devient `params.<id>` dans le script. L'ordre du fichier est l'ordre des curseurs.

| Clé | Types | Obligatoire | Rôle |
| --- | --- | --- | --- |
| `type` | — | oui | `"float"`, `"int"`, `"bool"` ou `"choice"` |
| `label` | texte | oui | Libellé **court : 14 caractères au plus** (la colonne fait 100 px et accueille aussi ⓘ et ↺). Le détail va dans `help`. |
| `unit` | chaîne | non | Unité affichée. `"mm"` : longueur (affichée en pouces si l'utilisateur le demande) ; `"°"` : angle ; sinon texte libre court (`"%"`, `"mm/s"`, `"×"`). |
| `help` | texte | non | Infobulle : une ou deux phrases concrètes, vocabulaire d'atelier. |
| `advanced` | booléen | non | `true` : rangé derrière « ▸ Avancé ». Garder visibles les 5 à 10 réglages qui changent la forme. |
| `default` | nombre / booléen / chaîne | oui | Valeur par défaut, **imprimable telle quelle** sur une buse de 0,4 mm (pour `choice` : la `value` d'une option). |
| `min`, `max` | nombre | oui (float, int) | Bornes du curseur. `min < max`, `min ≤ default ≤ max`. |
| `step` | nombre | non (float, int) | Pas des flèches et nombre de décimales affichées (`0.05` → 2 décimales). |
| `options` | liste | oui (choice) | Options : `["zigzag", "onde"]` ou `[{ value = "zigzag", label = { fr = "Zigzag", en = "Zigzag" } }, …]`. Deux options : sélecteur à côté du libellé ; trois : sélecteur pleine largeur ; davantage : liste déroulante. |

- `float` accepte des valeurs entières (`default = 15`) ; `int` refuse `6.5`. `bool` et `choice` n'ont ni `min`, ni `max`, ni `step`.
- `choice` : le script reçoit la chaîne `value` (`if params.pattern == "onde" { … }`) ; options uniques, non vides ; une valeur hors liste est l'erreur `param.unknown_option`.
- Une valeur hors bornes envoyée au moteur n'est **jamais ramenée** dans l'intervalle : erreur `param.out_of_range`, parcours non calculé.
- Les angles des paramètres sont en **degrés** ; convertir avec `rad()` dans le script.

## 3. Script

Le script est écrit en [Rhai](https://rhai.rs/book/) (syntaxe proche de Rust et de JavaScript). Il construit un parcours avec `new_path()` et **se termine par ce parcours** (dernière expression, ou `return path;`).

### Repère et conventions

- Millimètres, repère du modèle **centré sur (0, 0)**, Z vers le haut, **Z = 0 au plateau**. Le moteur centre la pièce sur le plateau de la machine ; ne jamais écrire de coordonnées de plateau.
- La première couche se pose à `printer.first_layer_height` (ou un peu moins pour écraser le premier tour).
- Le **premier point est rejoint par un déplacement** (sans extrusion) ; chaque point suivant décrit le mouvement depuis le point précédent.
- `z` est la hauteur de la buse, soit le dessus du boudin déposé. Chaque passage doit s'appuyer sur la matière du passage d'en dessous : en spirale continue, la montée par tour vaut `h` ; une pièce haute s'obtient par beaucoup de tours.
- `w` (largeur) et `h` (hauteur) décrivent la section du boudin déposé : section en stade (bords arrondis), `(max − min) × min + π × min² / 4`. Une largeur de 1,2 à 2,5 fois la buse est courante ; `h` est la montée par couche.

### Variables fournies

| Nom | Contenu |
| --- | --- |
| `params` | Valeurs des paramètres : `params.inner_radius` (flottant), `params.star_tips` (entier), `params.closed` (booléen), `params.pattern` (chaîne d'un `choice`). Constante : ne pas l'affecter. |
| `printer.nozzle_diameter` | Diamètre de buse (mm). |
| `printer.layer_height` | Hauteur de couche conseillée par le profil (mm). |
| `printer.first_layer_height` | Hauteur de première couche du profil (mm). |
| `printer.nozzle_cone_angle`, `printer.nozzle_cone_height` | Cône de dégagement de la buse : pente maximale d'une surface que la buse peut longer (degrés depuis l'horizontale) et hauteur du cône (mm) ; **0 si le profil ne les donne pas** (tester `> 0` avant de s'en servir, par exemple pour avertir d'une pente non planaire trop forte). |
| `PI`, `TAU` | π et 2π. |

### API du parcours

| Fonction | Effet |
| --- | --- |
| `new_path()` | Nouveau parcours vide. |
| `path.print_to(x, y, z, w, h)` | Imprime jusqu'au point (x, y, z) un boudin de largeur `w` et de hauteur `h` (mm). |
| `path.print_polar(r, a, z, w, h)` | Idem en coordonnées cylindriques : rayon `r` (mm), angle `a` en **radians**. |
| `path.travel_to(x, y, z)` | Déplacement sans extrusion. Rétraction et saut en Z selon le filament si le déplacement dépasse la distance minimale. |
| `path.set_speed(v)` | Vitesse des mouvements suivants, en mm/s. |
| `path.set_speed_factor(f)` | Vitesse = `f` × vitesse nominale du filament (défaut : 1). |
| `path.set_fan(p)` | Ventilateur des mouvements suivants, en % (0 à 100). |
| `path.reset_fan()` | Revient au ventilateur du filament. |
| `path.len()` | Nombre de points. |
| `path.bridge_to(x, y, z, w, h, flow)` | Pont tendu dans le vide : ventilateur au moins celui des ponts du filament, débit × `flow` (0 à 2 ; 0,85 tend le fil). |
| `path.blob(volume, durée)` | Goutte : la buse reste au point courant et pousse `volume` mm³ (multiplicateur de débit du filament compris) en `durée` secondes (`G1 E… F…` sans coordonnées). Volume de 0 à 1 000 mm³, durée de 0 à 60 s ; une goutte plus rapide que le débit max du filament dure plus longtemps (`speed.limited_flow`). Aperçu : sphère de même volume. |
| `path.print_points(pts, w, h)` | Imprime une liste de points (ci-dessous) d'un coup ; un premier point confondu avec la position courante est sauté (après `travel_to(pts)`). |
| `path.bridge_points(pts, w, h, flow)` | Idem en pont. |
| `path.travel_to(pts)` | Déplacement jusqu'au premier point de la liste. |
| `path.note(texte)` | Annotation dans la vue 3D au dernier point (« Départ », « Pointe fine conseillée »). 50 au plus. |
| `path.note_at(x, y, z, texte)` | Annotation à une position donnée. |

### Messages du modèle

| Fonction | Effet |
| --- | --- |
| `warn(texte)` | Avertissement affiché avec la ligne du script (le parcours est produit) ; 20 au plus. |
| `fail(texte)` | Refus : erreur bloquante, rien n'est produit (« Pores plus petits que la buse »). |

Un texte est une chaîne, ou une table par langue : `#{ fr: "Pont long", en: "Long bridge" }`.

### Listes de points (calcul natif)

Les fonctions ci-dessous calculent leurs points **en Rust** : ≈ 70 fois plus vite qu'une boucle de script (50 000 points en 1,2 ms dans le navigateur, contre 81 ms). À utiliser pour toute forme qu'elles savent produire ; garder les boucles de script pour les formules propres au modèle. Angles en **radians**, sens trigonométrique (antihoraire vu de dessus) ; une forme fermée répète son premier point à la fin. Inspirées des fonctions géométriques de FullControl.

| Fonction | Points |
| --- | --- |
| `points()` | Liste vide ; `pts.push(x, y, z)` ajoute un point. |
| `line(x0, y0, z0, x1, y1, z1, segments)` | Segment découpé en `segments`. |
| `arc(cx, cy, z, r, start, sweep, segments)` | Arc de `sweep` radians (négatif : sens horaire). |
| `circle(cx, cy, z, r, start, segments)` | Cercle fermé. |
| `polygon(cx, cy, z, r, sides, start)` | Polygone régulier fermé, sommets sur le cercle de rayon `r`. |
| `rectangle(x0, y0, z, dx, dy)` | Rectangle fermé depuis le coin (x0, y0). |
| `spiral(cx, cy, z, r_start, r_end, start, turns, segments_per_turn)` | Spirale à plat. |
| `helix(cx, cy, z_start, r_start, r_end, start, turns, pitch, segments_per_turn)` | Hélice : Z monte de `pitch` par tour, en continu (mode vase). |
| `zigzag(x0, y0, z, length, spacing, lines)` | Lignes parallèles à X en aller-retour, espacées de `spacing` en Y. |
| `sine(x0, y0, z, length, amplitude, period, segments_per_period)` | Onde le long de X. |

| Transformation (renvoie une nouvelle liste) | Effet |
| --- | --- |
| `a + b`, `a += b` | Mise bout à bout. |
| `pts.translate(dx, dy, dz)`, `pts.rotate(angle, cx, cy)`, `pts.scale(k, cx, cy)` | Déplacement, rotation autour de Z, mise à l'échelle en XY. |
| `pts.mirror_x(x)`, `pts.mirror_y(y)` | Symétrie par rapport à la droite verticale `x` ou horizontale `y`. |
| `pts.reverse()` | Sens inversé. |
| `pts.copies(n, dx, dy, dz)` | La liste et ses `n − 1` copies décalées, à la suite. |
| `pts.repeat_z(n, dz)` | Couches empilées. |
| `pts.polar_copies(n, cx, cy)` | Copies tournées de 2π / n. |
| `pts.subdivide(max_length)` | Points ajoutés pour qu'aucun segment ne dépasse `max_length`. |
| `pts.modulate_z(\|t, x, y, z\| dz)` | Z += la fonction (t de 0 à 1 le long de la liste). Appelle le script à chaque point : vitesse d'une boucle de script. |
| `pts.modulate_r(cx, cy, \|t, angle, z\| dr)` | Rayon autour de (cx, cy) += la fonction. Même coût. |
| `pts.slice(from, to)` | Points d'indice `from` ≤ i < `to` ; indice négatif compté depuis la fin. `b.slice(1, b.len())` retire le premier point, confondu avec le dernier de `a`, avant `a + b`. |
| `pts.len()`, `pts.point(i)` | Nombre de points ; point `i` en tableau `[x, y, z]` (`-1` : le dernier). |

**Remplissage d'un contour** : `contour.fill_lines(spacing, angle)` ou `contour.fill_lines(spacing, angle, inset)` renvoie un **tableau de listes de points** (des chaînes) qui remplissent le contour fermé `contour` (polygone XY quelconque, même non convexe, ondulé ou en étoile ; le dernier point se relie au premier) de lignes parallèles à la direction `angle` (radians), espacées de `spacing`, à la hauteur du premier point du contour. Les lignes s'arrêtent à `inset` mm du bord (distance perpendiculaire au bord rencontré) et sont reliées en zigzag tant que le lien reste dans la pièce et court (3 × `spacing` au plus) ; sinon une nouvelle chaîne commence. Chaque chaîne s'imprime d'un trait ; exemple, un disque plein de 40 mm entouré d'un contour :

```rhai
let path = new_path();
let disc = circle(0, 0, 0.2, 20, 0, 120);
path.travel_to(disc);
path.print_points(disc, 0.8, 0.2);
for c in disc.fill_lines(0.45, PI / 4, 0.6) {
    path.travel_to(c);
    path.print_points(c, 0.45, 0.2);
}
path
```

**Anneaux concentriques** : `contour.fill_rings(spacing)` ou `contour.fill_rings(spacing, inset)` renvoie un **tableau de listes de points fermées** (des anneaux) : le contour décalé vers l'intérieur de `inset` mm, puis de `spacing` en `spacing` jusqu'au centre (jonctions arrondies aux angles rentrants ; au centre reste un jour d'au plus `spacing`). Un contour étranglé donne plusieurs anneaux au même niveau. Les anneaux vont du bord vers le centre, tournent dans le sens du contour et commencent au point le plus proche du départ du précédent (coutures alignées) ; `rings.reverse()` les donne du centre vers le bord. Même usage que `fill_lines` : `for r in disc.fill_rings(0.45, 0.6) { path.travel_to(r); path.print_points(r, 0.45, 0.2); }`.

Fond plein : contour = ligne médiane de la paroi **sans sa texture** (au creux des ondulations, rayon de base sous un relief) : un fond qui suit les pointes de la texture forme un socle plus large que la paroi, comme un radeau. Faire naître la texture sur les premiers tours au-dessus du fond (environ 1,5 mm), sinon ses pointes débordent du fond dans le vide. Puis `spacing` = largeur des lignes, `inset` ≈ demi-largeur de la paroi + 35 % de la largeur des lignes (elles mordent sur la paroi) ; angle tourné de 90° d'une couche à l'autre. Grille de fils : `spacing` = pore + largeur, chaque ligne imprimée en pont (voir le tamis du catalogue).

Exemple : un tamis carré de 12 couches, fils tendus d'un bord à l'autre du cadre, croisés à 90° d'une couche à l'autre.

```rhai
let path = new_path();
let frame = rectangle(-20, -20, 0, 40, 40);
for k in 0..12 {
    let z = printer.first_layer_height + k * printer.layer_height;
    path.print_points(frame.translate(0, 0, z), 0.5, 0.2);
    let grid = zigzag(-20, -19.2, z, 40, 1.2, 33).rotate(k * PI / 2, 0, 0);
    path.bridge_points(grid, 0.45, 0.2, 0.85);
}
path
```

Les vitesses demandées restent **plafonnées** par l'imprimante, la première couche, le débit volumétrique du filament et la vitesse de l'axe Z ; chaque plafonnement est compté et signalé.

Les arguments numériques acceptent entiers et flottants. Une largeur, une hauteur ou une vitesse nulle ou négative, une position non finie, un ventilateur hors de 0–100 sont des erreurs (`path.invalid_width`, …) positionnées sur la ligne du script.

### Fonctions mathématiques

`sin cos tan asin acos atan sinh cosh tanh exp ln log sqrt` (un argument), `atan(y, x)` et `atan2(y, x)`, `pow(x, y)`, `hypot(x, y)`, `min(a, b)`, `max(a, b)`, `clamp(x, lo, hi)`, `lerp(a, b, t)`, `rad(degrés)`, `deg(radians)`. `noise(x, y, z)` : bruit de gradient 3D (calcul natif), valeur de −1 à 1, continue, nulle sur les points entiers, motifs d'une taille d'environ 1 ; diviser les coordonnées par la taille du motif voulue, les décaler pour un autre tirage. Sur un cylindre, le prendre en `(cos(a) × r, sin(a) × r, z)` : pas de couture (voir le photophore vermiculé du catalogue). Les fonctions de Rhai restent disponibles : `floor(x)`, `ceiling(x)`, `round(x)`, `abs(x)`, `x.to_int()`, `x.to_float()`…

Ces fonctions donnent le même résultat en natif et dans le navigateur (`libm`). Préférer `pow(x, y)` à `x ** y`.

### Pièges de Rhai (à lire avant d'écrire un modèle)

1. **La division de deux entiers est entière** : `i / n` vaut 0 si `i < n`. Écrire `i / n.to_float()` ou `i * 1.0 / n`. Le moteur signale un parcours dont beaucoup de points sont confondus (`path.duplicates_skipped`), voire vide (`path.empty`).
2. Une boucle `for i in 0..n` demande un **entier** `n` : `floor(x).to_int()`.
3. `floor`, `round`, `ceiling` renvoient un **flottant** ; `.to_int()` pour un entier. Ils acceptent aussi un entier : `floor(200 / n)` fait la division entière, puis renvoie un flottant.
4. Une variable doit être déclarée avec `let` avant usage ; une variable inconnue est une erreur dès la lecture du script (`script.unknown_variable`).
5. Une propriété inconnue (`params.radius` au lieu de `params.inner_radius`) est une erreur (`script.unknown_property`).
6. Les blocs `if` peuvent servir d'expression : `let h = if t < 1.0 { 0.3 } else { 0.24 };`.
7. Chaînes : `` `n = ${params.n}` `` interpole ; `print(...)` écrit dans le journal du calcul (aide à la mise au point).
8. Une fonction `fn nom(a, b) { … }` ne voit aucune variable extérieure, ni `params` ni `printer` : les passer en arguments. Les fermetures `|t, a, z| …` (utilisées par `modulate_r`, `modulate_z`), elles, voient les variables du script.
9. Permis : intervalles inclusifs `0..=n`, réaffectation (`x = x + 1`), mélange entiers / flottants dans les calculs (hors division de deux entiers).

### Limites (bac à sable)

50 millions d'opérations Rhai (au-delà : `script.too_many_operations`), 1 million de points, profondeur d'appel 64. Pas d'accès aux fichiers, au réseau ni à l'horloge ; `eval` est désactivé.

### Performance

Mesurée sur Apple M4 (voir [ADR-0002](adr/0002-format-weave-toml-et-rhai.md)) : de l'ordre de **1 µs par point en natif et 1,6 µs dans le navigateur** pour une boucle de script de trois fonctions trigonométriques ; une liste de points native, ≈ 0,025 µs par point. Une boucle de script reste fluide sous le curseur jusqu'à environ **30 000 points** ; au-delà, l'interface calcule au relâchement du curseur. Pour aller vite :

- préférer les listes de points natives aux boucles de script ;

- copier les paramètres utilisés dans la boucle dans des variables locales avant la boucle (≈ 15 % de gain) ;
- dans une boucle de dizaines de milliers de points, écrire le calcul en ligne plutôt que d'appeler une fermeture (`f.call(…)`) qui renvoie un tableau : dans le navigateur, le vase à écailles passe de 1,1 s à 0,75 s au premier calcul ;
- calculer une fois hors de la boucle ce qui ne dépend pas de l'indice ;
- ne pas multiplier les points inutiles (2 points suffisent pour un zigzag).

## 4. Ce que fait le moteur

1. Contrôle les valeurs des paramètres, exécute le script.
2. Prépare le parcours selon les choix de l'utilisateur (section 6, « Préparation ») : rotation, copies, bordure ou radeau. Place le centre de l'encombrement au point choisi (centre du plateau par défaut) ; refuse une pièce qui dépasse le plateau, descend sous Z = 0, dépasse la hauteur d'impression ou passe dans une zone exclue du plateau (purge, pinces).
3. Pour chaque mouvement : extrusion relative `E` = section × longueur 3D × multiplicateur de débit ÷ section du filament (goutte : volume × multiplicateur ÷ section du filament, en la durée demandée, plafonnée par le débit max) ; vitesse = demande du script, plafonnée (première couche, machine, débit volumétrique, axe Z) ; ventilateur (première couche, filament, script, ponts) ; durée avec les accélérations du profil (trapèze de vitesse, passage des angles limité par le *jerk*).
4. Écrit le G-code : vignettes du profil, gabarit de démarrage, `M83`, accélérations, mouvements (seuls les axes qui changent) avec progression `M73 P R` et changements de couche dans le dialecte du profil, consignes de température après la première couche, gabarit de fin, métadonnées (temps estimé, filament) lues par les imprimantes Prusa.

## 5. Messages du moteur

Codes stables, traduits par l'interface. **Erreur** : rien n'est produit ; **avertissement** : produit mais à vérifier ; **info** : adaptation attendue, comptée.

| Code | Gravité | Cause |
| --- | --- | --- |
| `model.missing_separator` | erreur | Pas de ligne `---`. |
| `model.invalid_header` | erreur | TOML invalide ou champ inconnu (détail : message de TOML). |
| `model.wrong_format`, `model.unsupported_version` | erreur | Mauvais `format`, version trop récente. |
| `param.invalid_id`, `param.missing_field`, `param.unknown_type`, `param.not_a_number`, `param.not_an_integer`, `param.not_a_boolean`, `param.unexpected_field`, `param.invalid_bounds`, `param.default_out_of_range`, `param.invalid_step` | erreur | Définition d'un paramètre incorrecte. |
| `param.out_of_range`, `param.wrong_type` | erreur | Valeur envoyée hors bornes ou du mauvais type. |
| `param.unknown` | avertissement | Valeur envoyée pour un paramètre absent du modèle (ignorée). |
| `script.syntax`, `script.unknown_variable`, `script.unknown_property`, `script.unknown_function`, `script.wrong_type`, `script.arithmetic`, `script.index_out_of_range`, `script.constant_assignment`, `script.runtime`, `script.thrown` | erreur | Erreur du script (ligne et colonne dans le fichier). |
| `script.too_many_operations`, `script.too_deep`, `script.data_too_large` | erreur | Limite du bac à sable atteinte. |
| `script.no_path` | erreur | Le script ne se termine pas par le parcours. |
| `path.invalid_position`, `path.invalid_width`, `path.invalid_height`, `path.invalid_speed`, `path.invalid_fan`, `path.invalid_flow`, `path.invalid_blob`, `path.too_many_points`, `path.too_many_notes` | erreur | Appel de l'API invalide. |
| `path.blob_without_position` | erreur | `path.blob` avant tout point : la goutte n'a pas de position. |
| `points.invalid_count`, `points.invalid_value`, `points.index_out_of_range`, `points.empty` | erreur | Argument invalide d'une fonction de points (nombre de segments, rayon…). |
| `model.warning` | avertissement | `warn()` du modèle (son texte, sa ligne). |
| `model.failure` | erreur | `fail()` du modèle. Aux bornes d'un paramètre, `fabweave check` tolère ce refus (combinaison impossible avec les autres valeurs par défaut, par exemple un trou plus grand que la pièce) ; aux valeurs par défaut, jamais. |
| `param.invalid_options`, `param.duplicate_option`, `param.unknown_option` | erreur | Paramètre `choice` mal défini, ou valeur hors liste. |
| `path.empty` | erreur | Aucun mouvement ne dépose de matière. |
| `path.out_of_bed`, `path.below_bed`, `path.too_high` | erreur | La pièce ne tient pas dans le volume d'impression. |
| `path.excluded_zone` | erreur | La pièce passe dans une zone exclue du plateau (`field` : `printer.bed.exclude[i]`, `name` : nom de la zone par langue, `count` : segments en cause). |
| `path.duplicates_skipped` | info (avertissement au-delà d'un point sur dix) | Points confondus avec le précédent, ignorés. |
| `speed.limited_first_layer`, `speed.limited_machine`, `speed.limited_flow`, `speed.limited_z` | info | Vitesse plafonnée (nombre de segments, valeur du plafond) ; `speed.limited_flow` compte aussi les gouttes allongées. |
| `blob.too_slow` | avertissement | Gouttes qui pousseraient le filament sous `F1` (1 mm/min) : elles durent plus longtemps que demandé (nombre de gouttes). |
| `nozzle.not_recommended` | avertissement | Buse montée hors de `recommended_nozzle`. |
| `nozzle.custom` | info | Diamètre libre, hors des buses du profil : hauteurs de couche interpolées. |
| `nozzle.not_abrasive` | avertissement | Filament abrasif dans une buse qui ne l'est pas. |
| `setup.invalid`, `setup.out_of_range`, `setup.too_many_copies`, `setup.high_flow_unavailable`, `setup.abrasive_unavailable` | erreur | Préparation refusée (valeur hors domaine, buse non proposée par l'imprimante). |
| `copies.too_close`, `copies.too_tall`, `copies.clearance_unknown` | erreur | Copies pièce par pièce impossibles : écart sous le rayon de dégagement de la tête, pièce plus haute que l'espace libre sous le portique, dégagement absent du profil. |
| `adhesion.skipped` | avertissement | Bordure ou radeau non ajouté (empreinte de la première couche introuvable). |
| `profile.review_pending` | erreur | Imprimante importée d'un trancheur dont les gabarits n'ont pas été relus (`[printer] review`) : le parcours se montre, le G-code n'est pas exporté. |
| `import.no_profile`, `import.process_profile`, `import.invalid_json` | erreur | Import : aucun profil d'imprimante ni de filament dans le fichier ; réglages d'impression (sans équivalent) ; JSON illisible (ligne, colonne). |

## 6. Profils

Deux fichiers TOML, lus strictement comme l'en-tête des modèles : `profiles/printers/*.toml` (`format = "fabweave.printer"`, `version = 2`) et `profiles/filaments/*.toml` (`format = "fabweave.filament"`, `version = 2`). Exemples commentés : [prusa_mk4s.toml](../profiles/printers/prusa_mk4s.toml), [bambu_x1e.toml](../profiles/printers/bambu_x1e.toml), [marlin_generic.toml](../profiles/printers/marlin_generic.toml), [klipper_generic.toml](../profiles/printers/klipper_generic.toml), [generic_pla.toml](../profiles/filaments/generic_pla.toml).

**Version 2 du profil imprimante** (FabWeave 0.3) : zones exclues, accélérations, cône de la buse, vignettes, nom par langue. Tous ces champs sont facultatifs : un profil `version = 1` se lit sans changement (test de lecture : `crates/fabweave-core/tests/fixtures/prusa_mk4s_v1.toml`), sans zone, sans accélération (temps estimé = longueur ÷ vitesse, aucun `M204`) ni vignette.

| Imprimante | Contenu |
| --- | --- |
| `[printer]` | `id`, `name` (texte, ou table par langue `{ fr = "…", en = "…" }`), `model` (annoncé au firmware : `M862.3` des Prusa, `printer_model_id` du paquet Bambu, `C13` pour la X1E), `firmware` (`prusa`, `marlin`, `klipper`, `bambu` ; tout autre nom est refusé), `source`, `review` (facultatif : `["gcode.start", "gcode.end"]`, gabarits à relire, posé par l'import ; tant qu'il en reste, `profile.review_pending` bloque l'export) |
| `[bed]` | `size = [x, y]` (diamètre deux fois pour un plateau rond), `shape` (`"rectangle"` par défaut, ou `"circle"`), `max_height`, `origin = "corner"` ou `"center"`, `plate` (facultatif : type de plaque, variable `bed_type` ; Bambu Lab : `"Textured PEI Plate"`, `"Cool Plate"`, `"Engineering Plate"`, `"High Temp Plate"`), `exclude` (facultatif : zones interdites à la pièce, `{ name = { fr, en }, min = [x, y], max = [x, y] }` en repère machine ; dessinées hachurées sur le plateau ; erreur `path.excluded_zone` si un mouvement imprimé y passe) |
| `[nozzle]` | `diameter`, `high_flow`, `abrasive` (buse livrée) ; `choices` (diamètres proposés par le sélecteur de buse), `high_flow_available`, `abrasive_available` (l'imprimante accepte ces buses) ; `clearance_radius`, `clearance_height` (dégagement de la tête pour les copies pièce par pièce : rayon autour de la buse, hauteur libre sous le portique ; `extruder_clearance_*` des trancheurs) ; `cone_angle` et `cone_height` (cône de dégagement, pente maximale en degrés de 0 à 90 exclus, hauteur en mm ; transmis au script) ; tous facultatifs |
| `[limits]` | `max_print_speed`, `travel_speed`, `max_z_speed` (mm/s) ; facultatifs : `acceleration` (impression, mm/s²), `travel_acceleration` et `first_layer_acceleration` (sinon `acceleration`, qu'ils exigent), `jerk` (saut de vitesse dans un angle, mm/s, 8 par défaut ; pour l'estimation seulement) |
| `[layers]` | `layer_height`, `first_layer_height` (transmis au script) ; `by_nozzle = [{ diameter, layer_height, first_layer_height? }, …]` : hauteurs pour les autres buses |
| `[gcode]` | `start`, `end` : gabarits (ci-dessous) ; `thumbnails` (facultatif) : vignettes du parcours intégrées au G-code, `["16x16/QOI", "380x285/PNG"]` comme le réglage de PrusaSlicer (PNG ou QOI, 1 024 px de côté au plus ; `profile.invalid_thumbnail` sinon) |

| Filament | Contenu |
| --- | --- |
| `[filament]` | `id`, `name`, `type` (`PLA`, `PETG`, `FLEX`…), `diameter`, `density` (g/cm³), `flow_multiplier`, `pressure_advance`, `abrasive` |
| `[temperature]` | `first_layer`, `nozzle`, `bed_first_layer`, `bed`, `probe` (palpage), `idle`, `chamber` |
| `[speed]` | `nominal` (pour `set_speed_factor(1)`), `first_layer` (plafond), `max_volumetric` (mm³/s) ; `by_nozzle = [{ diameter?, high_flow?, max_volumetric }, …]` (v2) : débit maximal selon la buse, l'entrée qui correspond le plus précisément l'emporte (diamètre et HF, puis l'un des deux ; à égalité, la première) |
| `[cooling]` | `fan`, `first_layer_fan`, `bridge_fan` (%) |
| `[retraction]` | `length`, `speed`, `z_hop`, `min_travel` |

### Profils de l'utilisateur et import

Dans l'application, le menu « ⋯ » à côté de l'imprimante et du filament modifie, duplique, importe, exporte (TOML) et supprime les profils. Un profil livré n'est jamais modifié : « Copier et modifier » crée un profil de l'utilisateur, mémorisé sur le poste (TOML écrit par le moteur, commentaires non gardés). L'éditeur montre l'origine de chaque valeur (↺ si elle a changé depuis la copie ou l'import ; « importé », « base », « estimé », « vide » pour un import) et le moteur contrôle le profil à chaque modification : il ne s'enregistre pas avec une erreur.

**Import** (dépôt du fichier ou « Importer un profil… » ; `fabweave_core::import`) :

- **Fichiers** : PrusaSlicer `.ini`, export à plat (« Exporter la configuration » : une imprimante et un filament) ou paquet de préréglages (`[printer:Nom]`, `[filament:Nom]`, héritage `inherits` résolu dans le fichier) ; Bambu Studio et OrcaSlicer `.json` (`"type": "machine"` ou `"filament"` ; les réglages d'impression `"process"` sont refusés) ; profil FabWeave `.toml`, ajouté tel quel.
- **Profil de base** : chaque profil part d'un profil livré (même firmware pour une imprimante, même matière pour un filament) ; les valeurs du fichier le remplacent. Rapport : valeurs reprises avec leur clé, valeurs absentes (gardées de la base, laissées vides ou estimées), clés ignorées, gabarits du trancheur sans équivalent (changement de couche, de filament…), préréglage parent absent du fichier (`inherits` d'un préréglage du système : ses valeurs manquent).
- **Laissé vide plutôt que repris de la base** (propre à une machine ou à un filament) : zones exclues, buses proposées, couches par buse, cône, dégagement de la tête, vignettes, débits par buse du filament. Hauteur de couche absente et buse différente de la base : moitié du diamètre, dite « estimée ».
- **Correspondances** (première clé présente) : plateau `bed_shape` / `printable_area` (origine au centre si le contour est centré sur zéro, rond s'il a au moins 8 sommets équidistants), `max_print_height` / `printable_height`, `bed_exclude_area` (rectangles de 4 sommets) ; vitesses maximales de la machine en mode normal (`machine_max_feedrate_*` / `machine_max_speed_*`) ; accélération de la paroi extérieure, sinon par défaut, sinon maximale de la machine ; firmware d'après `printer_model` (Bambu Lab, Prusa à firmware Buddy) ou `gcode_flavor` ; filament : températures (Bambu : plaque texturée, sinon PEI lisse), `extrusion_multiplier` / `filament_flow_ratio`, avance de pression (`pressure_advance` si activée, sinon `M572 S` ou `M900 K` du gabarit de filament de PrusaSlicer, buse 0,4 mm), ventilateur maximal, rétraction du filament sinon de l'imprimante. Vitesses nominale et de première couche, température de palpage : jamais dans un profil de filament des trancheurs, gardées de la base.
- **Gabarits traduits** de la syntaxe des trancheurs vers celle de FabWeave : `[var]`, `{var[0]}`, `{expr}`, `{if}…{elsif}…{else}…{endif}` (blocs en chaînes `` `…` ``), `and` / `or` / `not`, `a ? b : c`, `min`, `max`, `int`, `round`, expressions régulières simples (`=~ /.*PET.*/` → `.contains("PET")`). Variables renommées (`nozzle_temperature_initial_layer` → `first_layer_temperature`, `max_layer_z` → `max_z`, `first_layer_print_min[0]` → `first_layer_min_x`, `curr_bed_type` → `bed_type`…), extrudeur unique (`initial_tool` → 0) ; un réglage du fichier sans équivalent (`z_offset`, `printer_notes`) est remplacé par sa valeur, constante pendant l'impression, et listé. Ce qui ne se traduit pas (variable d'état comme `layer_num`, `{local …}`, fonction inconnue) reste tel quel, listé avec sa ligne : le profil est alors en erreur jusqu'à sa réécriture dans l'éditeur.
- **Relecture** : une imprimante importée porte `review = ["gcode.start", "gcode.end"]` ; l'éditeur montre « J'ai relu » au-dessus de chaque gabarit. Choix : [ADR-0006](adr/0006-import-des-profils-de-trancheurs.md).

### Préparation de l'impression

Choisie par l'utilisateur dans l'application (sections « Machine » et « Plateau ») ou en ligne de commande, jamais par le modèle. JSON du moteur (`fabweave_core::setup::Setup`, tous les champs facultatifs) :

```json
{
  "nozzle": { "diameter": 0.6, "high_flow": true, "abrasive": false },
  "placement": { "x": 125, "y": 105, "rotation": 30, "copies": [2, 1], "spacing": 45 },
  "adhesion": { "type": "brim", "width": 5, "gap": 0.1 }
}
```

- **Buse** : sans diamètre, celle du profil. Le moteur dérive des profils « effectifs » (les profils livrés ne changent pas) : diamètre transmis au script et aux gabarits (`M862.1` des Prusa), hauteurs de couche de `layers.by_nozzle` (interpolées entre les buses connues pour un diamètre libre, message `nozzle.custom`), débit maximal de `speed.by_nozzle` du filament.
- **Placement** : `x`, `y` = centre de l'encombrement en repère machine (centre du plateau par défaut) ; `rotation` en degrés autour de ce centre ; `copies = [colonnes, rangées]` en grille centrée, imprimées **pièce par pièce** (la buse finit une copie avant la suivante, le compteur de couches repart du plateau) ; `spacing` = écart entre copies, par défaut le rayon de dégagement de la tête (`clearance_radius`), qu'il ne peut pas descendre sous ; une pièce plus haute que `clearance_height` ne peut pas être copiée. 100 copies au plus.
- **Adhésion** : `{"type": "none"}`, `{"type": "brim", "width": 5, "gap": 0.1}` (bordure de cette largeur, à cet écart de la pièce, boucles de 1,125 × la buse, extérieur seulement) ou `{"type": "raft", "layers": 3, "gap": 0.3, "margin": 3}` (radeau débordant de `margin`, couche de base à la hauteur de première couche, autres à la hauteur de couche, lignes de 1,25 × la buse, couche de contact en lignes de la largeur de la buse ; la pièce est rehaussée de l'épaisseur du radeau plus `gap`). L'emprise compte dans les contrôles du plateau et des zones exclues.

La préparation est rappelée en tête du G-code (`; nozzle = …`, `; copies = …`, `; adhesion = …`).

### Gabarits de G-code

Texte libre ; chaque `{ … }` est une **expression Rhai** évaluée avec les variables ci-dessous, et remplacée par sa valeur (nombres à 3 décimales au plus, booléens en `1` / `0`, rien pour une expression sans valeur). `{{` et `}}` écrivent une accolade.

```text
M104 S{first_layer_temperature}
G1 Z{min(max_z + 1, max_print_height)} F720
{if filament_type == "FLEX" { "G1 E-4 F2400" } else { "G1 E-2 F2400" }}
{if first_layer_bed_temperature <= 60 { "M106 S100" }}
```

Variables : `printer_model`, `printer_name`, `filament_type`, `filament_name`, `model_name` (textes) ; `nozzle_high_flow`, `nozzle_abrasive`, `filament_abrasive` (booléens) ; `nozzle_diameter`, `bed_size_x`, `bed_size_y`, `max_print_height`, `first_layer_temperature`, `temperature`, `first_layer_bed_temperature`, `bed_temperature`, `probe_temperature`, `idle_temperature`, `chamber_temperature`, `pressure_advance`, `retraction_length`, `retraction_speed`, `first_layer_min_x`, `first_layer_min_y`, `first_layer_max_x`, `first_layer_max_y`, `print_min_x`, `print_min_y`, `print_max_x`, `print_max_y`, `max_z` (nombres, repère de la machine), `max_volumetric_speed` (débit volumétrique maximal du filament, mm³/s), `acceleration`, `travel_acceleration`, `first_layer_acceleration` (mm/s², 0 si le profil n'en donne pas), `total_layer_count` (couches au sens du compteur de l'imprimante, voir plus bas), `estimated_time` (s) ; `bed_type` (texte, plaque du profil). Fonction `num(x)` : nombre écrit comme une expression du gabarit (3 décimales au plus), à utiliser dans une chaîne interpolée. Un bloc de plusieurs lignes s'écrit en chaîne entre accents graves : ``{if chamber_temperature >= 40 { `M141 S${num(chamber_temperature)}` }}`` ; dans un profil, mettre alors le gabarit en chaîne TOML littérale `'''…'''` (sans échappements). Une variable inconnue est une erreur dès la lecture du profil (`profile.template_error`).

### Dialectes

Ce que le moteur écrit lui-même, selon `firmware` (le reste vient des gabarits) :

| | Prusa | Marlin | Klipper | Bambu Lab |
| --- | --- | --- | --- | --- |
| En tête de fichier | `; generated by FabWeave`, vignettes | idem | idem | `HEADER_BLOCK` (durée, couches, filament), vignettes entre `; THUMBNAIL_BLOCK_START` et `_END` |
| Accélérations | `M204 P<impression> T<déplacements>` | idem | `M204 S`, réécrit à chaque passage impression ↔ déplacement | idem Klipper |
| Couches | `;LAYER_CHANGE`, `;Z:`, `;HEIGHT:` (comme PrusaSlicer) | idem | `SET_PRINT_STATS_INFO TOTAL_LAYER` puis `CURRENT_LAYER` | `M73 L<n>`, `M991 S0 P<n−1>` (compteur à l'écran) |
| Progression | `M73 P<%> R<minutes>` à chaque changement de l'un des deux | idem | idem (Klipper lit `P`) | idem, et à chaque couche |

L'accélération de la première couche s'applique jusqu'au premier mouvement au-dessus d'elle, puis celle d'impression. L'avance de pression du filament (`pressure_advance`) passe par les gabarits : `M572 S` (Prusa), `M900 K` (Marlin), `SET_PRESSURE_ADVANCE ADVANCE=` (Klipper), écrite seulement si elle est donnée (`{if pressure_advance > 0 { … }}`).

Vignettes (`[gcode] thumbnails`) : rendu du parcours en vue oblique, fond transparent, encodé en PNG ou QOI puis en base64, lignes de 78 caractères au plus, au format de PrusaSlicer (`; thumbnail begin 380x285 <longueur>` … `; thumbnail end`, `; thumbnail_QOI begin …` pour le QOI).

Un parcours FabWeave n'a pas toujours de couches : une couche commence quand l'impression dépasse d'une hauteur de couche du profil le début de la précédente (en spirale ou en non planaire, ce sont des paliers de hauteur atteinte).

### Paquet `.gcode.3mf` (Bambu Lab)

L'export propose alors aussi le **paquet `.gcode.3mf`** (CLI : sortie en `.gcode.3mf`) : archive ZIP qui contient `Metadata/plate_1.gcode`, sa somme MD5, `slice_info.config` (modèle d'imprimante `model`, buse, durée, poids, filament), `model_settings.config`, `plate_1.json` et des vignettes PNG du parcours (512 et 128 px). Choix et essai : [ADR-0005](adr/0005-bambu-lab-x1e-par-la-carte-sd.md).

## 7. Modèle de référence

[models/vases/ripple_vase.weave](../models/vases/ripple_vase.weave) : port du « Ripple Texture Demo » de FullControl, commenté ; il montre un en-tête complet (textes par langue, infobulles, paramètres avancés, `choice`) et un script typique (spirale continue, première couche renforcée, vitesse réduite au premier tour, fond plein par `fill_lines` raccordé à la spirale sans couture).

Autres exemples du catalogue : listes natives et copies tournées ([hex_adapter](../models/mecanique/hex_adapter.weave)), non planaire ([nonplanar_spacer](../models/mecanique/nonplanar_spacer.weave), [snake_soap_dish](../models/maison/snake_soap_dish.weave)), ponts ([mesh_sieve](../models/grilles/mesh_sieve.weave), [pin_support_challenge](../models/defis/pin_support_challenge.weave)), cellule calculée puis copiée en natif sur un grand parcours ([lampshade](../models/vases/lampshade.weave), 232 000 points).
