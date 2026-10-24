# Format `.weave` et profils — spécification

> **v0.2 (24/10/2026)** : premier jet validé à la v0.1 ; ajouts de la v0.2 (paramètres `choice`, messages du modèle, annotations, ponts, listes de points natives) à valider. Format `fabweave.model`, version 1 (les ajouts sont compatibles). Décisions : [ADR-0002](adr/0002-format-weave-toml-et-rhai.md) (format), [ADR-0004](adr/0004-gabarits-g-code-en-expressions-rhai.md) (gabarits des profils).

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
- `w` (largeur) et `h` (hauteur) décrivent la section du boudin déposé : section en stade (bords arrondis), `(max − min) × min + π × min² / 4`. Une largeur de 1,2 à 2,5 fois la buse est courante ; `h` est la montée par couche.

### Variables fournies

| Nom | Contenu |
| --- | --- |
| `params` | Valeurs des paramètres : `params.inner_radius` (flottant), `params.star_tips` (entier), `params.closed` (booléen), `params.pattern` (chaîne d'un `choice`). Constante : ne pas l'affecter. |
| `printer.nozzle_diameter` | Diamètre de buse (mm). |
| `printer.layer_height` | Hauteur de couche conseillée par le profil (mm). |
| `printer.first_layer_height` | Hauteur de première couche du profil (mm). |
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
| `path.print_points(pts, w, h)` | Imprime une liste de points (ci-dessous) d'un coup. |
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
| `pts.len()`, `pts.point(i)` | Nombre de points ; point `i` en tableau `[x, y, z]` (`-1` : le dernier). |

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

`sin cos tan asin acos atan sinh cosh tanh exp ln log sqrt` (un argument), `atan(y, x)` et `atan2(y, x)`, `pow(x, y)`, `hypot(x, y)`, `min(a, b)`, `max(a, b)`, `clamp(x, lo, hi)`, `lerp(a, b, t)`, `rad(degrés)`, `deg(radians)`. Les fonctions de Rhai restent disponibles : `floor(x)`, `ceiling(x)`, `round(x)`, `abs(x)`, `x.to_int()`, `x.to_float()`…

Ces fonctions donnent le même résultat en natif et dans le navigateur (`libm`). Préférer `pow(x, y)` à `x ** y`.

### Pièges de Rhai (à lire avant d'écrire un modèle)

1. **La division de deux entiers est entière** : `i / n` vaut 0 si `i < n`. Écrire `i / n.to_float()` ou `i * 1.0 / n`. Le moteur signale un parcours dont beaucoup de points sont confondus (`path.duplicates_skipped`), voire vide (`path.empty`).
2. Une boucle `for i in 0..n` demande un **entier** `n` : `floor(x).to_int()`.
3. `floor`, `round`, `ceiling` renvoient un **flottant** ; `.to_int()` pour un entier.
4. Une variable doit être déclarée avec `let` avant usage ; une variable inconnue est une erreur dès la lecture du script (`script.unknown_variable`).
5. Une propriété inconnue (`params.radius` au lieu de `params.inner_radius`) est une erreur (`script.unknown_property`).
6. Les blocs `if` peuvent servir d'expression : `let h = if t < 1.0 { 0.3 } else { 0.24 };`.
7. Chaînes : `` `n = ${params.n}` `` interpole ; `print(...)` écrit dans le journal du calcul (aide à la mise au point).

### Limites (bac à sable)

50 millions d'opérations Rhai (au-delà : `script.too_many_operations`), 1 million de points, profondeur d'appel 64. Pas d'accès aux fichiers, au réseau ni à l'horloge ; `eval` est désactivé.

### Performance

Mesurée sur Apple M4 (voir [ADR-0002](adr/0002-format-weave-toml-et-rhai.md)) : de l'ordre de **1 µs par point en natif et 1,6 µs dans le navigateur** pour une boucle de script de trois fonctions trigonométriques ; une liste de points native, ≈ 0,025 µs par point. Une boucle de script reste fluide sous le curseur jusqu'à environ **30 000 points** ; au-delà, l'interface calcule au relâchement du curseur. Pour aller vite :

- préférer les listes de points natives aux boucles de script ;

- copier les paramètres utilisés dans la boucle dans des variables locales avant la boucle (≈ 15 % de gain) ;
- calculer une fois hors de la boucle ce qui ne dépend pas de l'indice ;
- ne pas multiplier les points inutiles (2 points suffisent pour un zigzag).

## 4. Ce que fait le moteur

1. Contrôle les valeurs des paramètres, exécute le script.
2. Centre l'encombrement de la pièce sur le plateau ; refuse une pièce qui dépasse le plateau, descend sous Z = 0 ou dépasse la hauteur d'impression.
3. Pour chaque mouvement : extrusion relative `E` = section × longueur 3D × multiplicateur de débit ÷ section du filament ; vitesse = demande du script, plafonnée (première couche, machine, débit volumétrique, axe Z) ; ventilateur (première couche, filament, script, ponts).
4. Écrit le G-code : gabarit de démarrage du profil, `M83`, mouvements (seuls les axes qui changent), consignes de température après la première couche, gabarit de fin, métadonnées (temps estimé, filament) lues par les imprimantes Prusa.

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
| `path.invalid_position`, `path.invalid_width`, `path.invalid_height`, `path.invalid_speed`, `path.invalid_fan`, `path.invalid_flow`, `path.too_many_points`, `path.too_many_notes` | erreur | Appel de l'API invalide. |
| `points.invalid_count`, `points.invalid_value`, `points.index_out_of_range`, `points.empty` | erreur | Argument invalide d'une fonction de points (nombre de segments, rayon…). |
| `model.warning` | avertissement | `warn()` du modèle (son texte, sa ligne). |
| `model.failure` | erreur | `fail()` du modèle. |
| `param.invalid_options`, `param.duplicate_option`, `param.unknown_option` | erreur | Paramètre `choice` mal défini, ou valeur hors liste. |
| `path.empty` | erreur | Aucun mouvement ne dépose de matière. |
| `path.out_of_bed`, `path.below_bed`, `path.too_high` | erreur | La pièce ne tient pas dans le volume d'impression. |
| `path.duplicates_skipped` | info (avertissement au-delà d'un point sur dix) | Points confondus avec le précédent, ignorés. |
| `speed.limited_first_layer`, `speed.limited_machine`, `speed.limited_flow`, `speed.limited_z` | info | Vitesse plafonnée (nombre de segments, valeur du plafond). |
| `nozzle.not_recommended` | avertissement | Buse hors de `recommended_nozzle`. |

## 6. Profils

Deux fichiers TOML, lus strictement comme l'en-tête des modèles : `profiles/printers/*.toml` (`format = "fabweave.printer"`) et `profiles/filaments/*.toml` (`format = "fabweave.filament"`). Exemples commentés : [prusa_mk4s.toml](../profiles/printers/prusa_mk4s.toml), [generic_pla.toml](../profiles/filaments/generic_pla.toml).

| Imprimante | Contenu |
| --- | --- |
| `[printer]` | `id`, `name`, `model` (annoncé au firmware, `M862.3`), `firmware` (`prusa`, `marlin`, `klipper`, `bambu`), `source` |
| `[bed]` | `size = [x, y]`, `max_height`, `origin = "corner"` ou `"center"` |
| `[nozzle]` | `diameter`, `high_flow`, `abrasive` |
| `[limits]` | `max_print_speed`, `travel_speed`, `max_z_speed` (mm/s) |
| `[layers]` | `layer_height`, `first_layer_height` (transmis au script) |
| `[gcode]` | `start`, `end` : gabarits (ci-dessous) |

| Filament | Contenu |
| --- | --- |
| `[filament]` | `id`, `name`, `type` (`PLA`, `PETG`, `FLEX`…), `diameter`, `density` (g/cm³), `flow_multiplier`, `pressure_advance`, `abrasive` |
| `[temperature]` | `first_layer`, `nozzle`, `bed_first_layer`, `bed`, `probe` (palpage), `idle`, `chamber` |
| `[speed]` | `nominal` (pour `set_speed_factor(1)`), `first_layer` (plafond), `max_volumetric` (mm³/s) |
| `[cooling]` | `fan`, `first_layer_fan`, `bridge_fan` (%) |
| `[retraction]` | `length`, `speed`, `z_hop`, `min_travel` |

### Gabarits de G-code

Texte libre ; chaque `{ … }` est une **expression Rhai** évaluée avec les variables ci-dessous, et remplacée par sa valeur (nombres à 3 décimales au plus, booléens en `1` / `0`, rien pour une expression sans valeur). `{{` et `}}` écrivent une accolade.

```text
M104 S{first_layer_temperature}
G1 Z{min(max_z + 1, max_print_height)} F720
{if filament_type == "FLEX" { "G1 E-4 F2400" } else { "G1 E-2 F2400" }}
{if first_layer_bed_temperature <= 60 { "M106 S100" }}
```

Variables : `printer_model`, `printer_name`, `filament_type`, `filament_name`, `model_name` (textes) ; `nozzle_high_flow`, `nozzle_abrasive`, `filament_abrasive` (booléens) ; `nozzle_diameter`, `bed_size_x`, `bed_size_y`, `max_print_height`, `first_layer_temperature`, `temperature`, `first_layer_bed_temperature`, `bed_temperature`, `probe_temperature`, `idle_temperature`, `chamber_temperature`, `pressure_advance`, `retraction_length`, `retraction_speed`, `first_layer_min_x`, `first_layer_min_y`, `first_layer_max_x`, `first_layer_max_y`, `print_min_x`, `print_min_y`, `print_max_x`, `print_max_y`, `max_z` (nombres, repère de la machine). Une variable inconnue est une erreur dès la lecture du profil (`profile.template_error`).

## 7. Modèle de référence

[models/vases/ripple_vase.weave](../models/vases/ripple_vase.weave) : port du « Ripple Texture Demo » de FullControl, commenté ; il montre un en-tête complet (textes par langue, infobulles, paramètres avancés) et un script typique (spirale continue, première couche renforcée, vitesse réduite au premier tour).
