"""Planche de variantes du logo FabWeave (skill fab-design-system, identite-et-logo.md).

Produit, dans ce dossier :
- `fabweave-<variante>.svg` : tuile commune de la famille + symbole d'une couleur ;
- `planche.html` : variantes à 128, 64, 48, 32, 24 et 16 px (rendu vectoriel natif à
  chaque taille), sur fond sombre et clair, en monochrome, couleurs candidates
  comparées aux couleurs de sens et aux identités de la famille, barre
  d'application en situation.

Les symboles sont des formes pleines : bandes dont le contour est fait d'arcs
SVG exacts (`A`), coordonnées arrondies au millième. Lancer :
    python3 branding/explorations/generate.py
"""

from __future__ import annotations

import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
TILE = (
    'M31 4 L97 4 C111.912 4 124 16.088 124 31 L124 97 C124 111.912 111.912 124 97 124 '
    'L31 124 C16.088 124 4 111.912 4 97 L4 31 C4 16.088 16.088 4 31 4 Z'
)


def f(v: float) -> str:
    s = f"{v:.3f}".rstrip('0').rstrip('.')
    return '0' if s == '-0' else s


# --- Bandes à contour d'arcs exacts -------------------------------------------
# Repère SVG (y vers le bas). Un arc : centre (cx, cy), rayon r, angles phi0 → phi1
# en degrés, mesurés depuis +x vers +y (sens horaire à l'écran). phi croissant :
# sens horaire, drapeau `sweep` = 1.


class Arc:
    def __init__(self, cx: float, cy: float, r: float, a0: float, a1: float):
        self.cx, self.cy, self.r, self.a0, self.a1 = cx, cy, r, a0, a1

    def point(self, a: float, r: float | None = None) -> tuple[float, float]:
        r = self.r if r is None else r
        return (self.cx + r * math.cos(math.radians(a)), self.cy + r * math.sin(math.radians(a)))

    @property
    def cw(self) -> bool:
        return self.a1 > self.a0


def arc_cmd(arc: Arc, r: float, forward: bool) -> str:
    a0, a1 = (arc.a0, arc.a1) if forward else (arc.a1, arc.a0)
    sweep = 1 if a1 > a0 else 0
    out = []
    # Arcs de plus de 179° coupés en deux (drapeau grand arc sans ambiguïté).
    steps = max(1, math.ceil(abs(a1 - a0) / 179))
    for k in range(1, steps + 1):
        a = a0 + (a1 - a0) * k / steps
        x, y = arc.point(a, r)
        out.append(f"A {f(r)} {f(r)} 0 0 {sweep} {f(x)} {f(y)}")
    return ' '.join(out)


def band(arcs: list[Arc], t: float) -> str:
    """Contour plein d'un trait d'épaisseur `t` qui suit une chaîne d'arcs
    tangents, extrémités rondes."""
    h = t / 2
    left = [a.r + h if a.cw else a.r - h for a in arcs]
    right = [a.r - h if a.cw else a.r + h for a in arcs]
    first, last = arcs[0], arcs[-1]
    x, y = first.point(first.a0, left[0])
    d = [f"M {f(x)} {f(y)}"]
    for a, r in zip(arcs, left):
        d.append(arc_cmd(a, r, True))
    ex, ey = last.point(last.a1, right[-1])
    d.append(f"A {f(h)} {f(h)} 0 0 1 {f(ex)} {f(ey)}")
    for a, r in zip(reversed(arcs), reversed(right)):
        d.append(arc_cmd(a, r, False))
    sx, sy = first.point(first.a0, left[0])
    d.append(f"A {f(h)} {f(h)} 0 0 1 {f(sx)} {f(sy)}")
    d.append('Z')
    return ' '.join(d)


def wave(x0: float, y0: float, half: float, amp: float, count: int, up_first: bool) -> list[Arc]:
    """Onde faite d'arcs tangents alternés : `count` demi-ondes de corde `half`
    et de flèche `amp`, à partir de (x0, y0) sur la ligne de base."""
    r = (half * half / 4 + amp * amp) / (2 * amp)
    theta = math.degrees(2 * math.asin(half / 2 / r))
    arcs = []
    up = up_first
    for i in range(count):
        mx = x0 + half * (i + 0.5)
        if up:  # bosse vers le haut : centre en dessous, sens horaire
            cy = y0 + (r - amp)
            arcs.append(Arc(mx, cy, r, -90 - theta / 2, -90 + theta / 2))
        else:  # creux vers le bas : centre au-dessus, sens antihoraire
            cy = y0 - (r - amp)
            arcs.append(Arc(mx, cy, r, 90 + theta / 2, 90 - theta / 2))
        up = not up
    return arcs


def spiral(cx: float, cy: float, r0: float, e: float, halves: int) -> list[Arc]:
    """Spirale de demi-cercles à deux centres : le rayon croît de `e` à chaque
    demi-tour (pas de 2e par tour)."""
    arcs = []
    for k in range(halves):
        r = r0 + k * e
        if k % 2 == 0:
            arcs.append(Arc(cx, cy, r, 180, 360))  # moitié haute, de gauche à droite
        else:
            arcs.append(Arc(cx - e, cy, r, 0, 180))  # moitié basse, de droite à gauche
    return arcs


def capsule(x0: float, y0: float, x1: float, y1: float, t: float) -> str:
    """Segment épais à bouts ronds de (x0, y0) à (x1, y1)."""
    dx, dy = x1 - x0, y1 - y0
    n = math.hypot(dx, dy)
    nx, ny = -dy / n * t / 2, dx / n * t / 2
    h = t / 2
    return (
        f"M {f(x0 + nx)} {f(y0 + ny)} L {f(x1 + nx)} {f(y1 + ny)} "
        f"A {f(h)} {f(h)} 0 0 0 {f(x1 - nx)} {f(y1 - ny)} L {f(x0 - nx)} {f(y0 - ny)} "
        f"A {f(h)} {f(h)} 0 0 0 {f(x0 + nx)} {f(y0 + ny)} Z"
    )


# --- Variantes ----------------------------------------------------------------


def v_ondes() -> list[tuple[str, str]]:
    """Trois couches ondulées décalées d'une demi-onde, dont les largeurs
    dessinent un vase : la paroi du vase à ondulations, qui se tisse d'elle-même."""
    t, half, amp, s = 9.0, 13.0, 4.0, 23.0
    out = []
    for i, (n, up) in enumerate([(4, True), (6, False), (4, True)]):
        y = 64 + (i - 1) * s
        width = n * half
        out.append((f"layer-{i + 1}", band(wave(64 - width / 2, y, half, amp, n, up), t)))
    return out


def v_spirale() -> list[tuple[str, str]]:
    """Parcours continu du mode vase : spirale de demi-cercles."""
    t, e, r0 = 9.0, 9.0, 5.0
    arcs = spiral(0, 0, r0, e, 5)
    # Centrage de l'encombrement sur (64, 64).
    xs = [a.cx + s * (a.r + t / 2) for a in arcs for s in (-1, 1)]
    ys = [a.cy + s * (a.r + t / 2) for a in arcs for s in (-1, 1)]
    dx = 64 - (min(xs) + max(xs)) / 2
    dy = 64 - (min(ys) + max(ys)) / 2
    for a in arcs:
        a.cx += dx
        a.cy += dy
    return [('spiral', band(arcs, t))]


def v_treillis() -> list[tuple[str, str]]:
    """Bandes croisées à 45°, dessus-dessous alternés : le tamis à ponts tendus."""
    t, gap = 10.0, 5.0
    c, s = math.cos(math.radians(45)), math.sin(math.radians(45))

    def rot(x: float, y: float) -> tuple[float, float]:
        x, y = x - 64, y - 64
        return (64 + x * c - y * s, 64 + x * s + y * c)

    lines = [-13.0, 13.0]
    half_len = 36.0
    out = []
    # Croisements : (u, v) ; la bande « horizontale » i passe dessus quand i + j est pair.
    for i, v in enumerate(lines):  # bandes horizontales (avant rotation)
        cuts = [u for j, u in enumerate(lines) if (i + j) % 2 == 1]
        pts = [-half_len] + [x for u in cuts for x in (u - t / 2 - gap, u + t / 2 + gap)] + [half_len]
        for k in range(0, len(pts), 2):
            a, b = rot(64 + pts[k], 64 + v), rot(64 + pts[k + 1], 64 + v)
            out.append((f"h{i + 1}-{k // 2 + 1}", capsule(*a, *b, t)))
    for j, u in enumerate(lines):  # bandes verticales
        cuts = [v for i, v in enumerate(lines) if (i + j) % 2 == 0]
        pts = [-half_len] + [y for v in cuts for y in (v - t / 2 - gap, v + t / 2 + gap)] + [half_len]
        for k in range(0, len(pts), 2):
            a, b = rot(64 + u, 64 + pts[k]), rot(64 + u, 64 + pts[k + 1])
            out.append((f"v{j + 1}-{k // 2 + 1}", capsule(*a, *b, t)))
    return out


def v_buse() -> list[tuple[str, str]]:
    """Une buse qui dépose une onde."""
    # Bloc de chauffe (rectangle arrondi) puis cône jusqu'à la pointe ; décalé
    # de 6 vers le bas pour centrer l'ensemble buse + onde.
    dy = 6
    nozzle = (
        f"M 48 {16 + dy} L 80 {16 + dy} A 3 3 0 0 1 83 {19 + dy} L 83 {33 + dy} A 3 3 0 0 1 80 {36 + dy} L 75.5 {36 + dy} "
        f"L 68.8 {53 + dy} A 2 2 0 0 1 66.9 {54.3 + dy} L 61.1 {54.3 + dy} A 2 2 0 0 1 59.2 {53 + dy} L 52.5 {36 + dy} "
        f"L 48 {36 + dy} A 3 3 0 0 1 45 {33 + dy} L 45 {19 + dy} A 3 3 0 0 1 48 {16 + dy} Z"
    )
    t, half, amp = 8.5, 16.0, 6.0
    n = 5
    bead = band(wave(64 - n * half / 2, 79 + dy, half, amp, n, False), t)
    return [('nozzle', nozzle), ('bead', bead)]


VARIANTS = {
    'ondes': ('A — Ondes tissées', 'Trois couches décalées d’une demi-onde : la paroi du vase à ondulations.', v_ondes),
    'spirale': ('B — Spirale', 'Le parcours continu du mode vase, d’un seul trait.', v_spirale),
    'treillis': ('C — Treillis', 'Bandes croisées dessus-dessous : le tamis à ponts tendus, le tissage.', v_treillis),
    'buse': ('D — Buse', 'Une buse qui trace une onde : le G-code direct.', v_buse),
}

# Couleurs candidates (thème sombre, thème clair) : teintes libres dans la famille.
COLORS = {
    'violet': ('Violet', '#A27DFF'),
    'corail': ('Rose corail', '#FF8BA0'),
    'or': ('Or pâle', '#E6CF73'),
    'turquoise': ('Turquoise profond', '#2FB8A6'),
}
SEMANTIC = {
    'accent (bleu)': '#4b9cf5',
    'succès (vert)': '#3fb67a',
    'attention (orange)': '#e8a33c',
    'erreur (rouge)': '#ef5b5b',
    'information (cyan)': '#4fc3d9',
    'FabSpeed': '#D954DC',
    'FabVents': '#B5D93A',
    'FabCAM': '#58B5F5',
    'KiPartManager': '#E8B954',
}


def svg(variant: str, color: str, size: int | None = 128, mono: bool = False) -> str:
    _, _, fn = VARIANTS[variant]
    tile_fill, tile_stroke, sym = ('#000000', '#ffffff', '#ffffff') if mono else ('#181f2a', '#354759', color)
    attrs = f'width="{size}" height="{size}" ' if size else ''
    shapes = '\n    '.join(f'<path id="{i}" d="{d}"/>' for i, d in fn())
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" {attrs}viewBox="0 0 128 128">\n'
        f'  <title>FabWeave — variante {variant}</title>\n'
        f'  <path d="{TILE}" fill="{tile_fill}" stroke="{tile_stroke}" stroke-width="3.6"/>\n'
        f'  <g id="symbol" fill="{sym}">\n    {shapes}\n  </g>\n</svg>\n'
    )


# --- Couleurs : contraste et écart perceptif ----------------------------------


def rgb(hexa: str) -> tuple[float, float, float]:
    h = hexa.lstrip('#')
    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))  # type: ignore[return-value]


def lum(hexa: str) -> float:
    def lin(c: float) -> float:
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(c) for c in rgb(hexa))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a: str, b: str) -> float:
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def lab(hexa: str) -> tuple[float, float, float]:
    def lin(c: float) -> float:
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(c) for c in rgb(hexa))
    x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047
    y = 0.2126 * r + 0.7152 * g + 0.0722 * b
    z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883

    def fl(t: float) -> float:
        return t ** (1 / 3) if t > 0.008856 else 7.787 * t + 16 / 116
    fx, fy, fz = fl(x), fl(y), fl(z)
    return (116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz))


def delta_e(a: str, b: str) -> float:
    return math.dist(lab(a), lab(b))


def darker(hexa: str) -> str:
    """Variante du thème clair : même teinte, assombrie par pas de 1 % jusqu'à
    un contraste de 3:1 sur la barre claire (#e6e6e6), et d'au moins 12 %."""
    r, g, b = rgb(hexa)
    k = 0.88
    while k > 0.2:
        c = '#{:02X}{:02X}{:02X}'.format(round(r * k * 255), round(g * k * 255), round(b * k * 255))
        if contrast(c, '#e6e6e6') >= 3.0:
            return c
        k -= 0.01
    return c


# --- Planche --------------------------------------------------------------------

SIZES = [128, 64, 48, 32, 24, 16]


def tile_row(variant: str, color: str, bg: str, mono: bool = False) -> str:
    cells = ''.join(f'<div class="cell"><div>{svg(variant, color, s, mono)}</div><span>{s}</span></div>' for s in SIZES)
    return f'<div class="row" style="background:{bg}">{cells}</div>'


def appbar(variant: str, color: str, light: bool) -> str:
    bar, text, muted, edge = ('#e6e6e6', '#1e1e1e', '#636363', darker(color)) if light else ('#181818', '#e8e8e8', '#7d7d7d', color)
    return (
        f'<div class="appbar" style="background:{bar};color:{text}"><i style="background:{edge}"></i>'
        f'{svg(variant, color, 24)}<b>FabWeave</b><span style="color:{muted}">G-code direct, modèles paramétriques</span></div>'
    )


def main() -> None:
    for v in VARIANTS:
        (HERE / f'fabweave-{v}.svg').write_text(svg(v, COLORS['violet'][1], None))

    sections = []
    # 1. Les quatre variantes en violet.
    for v, (title, desc, _) in VARIANTS.items():
        c = COLORS['violet'][1]
        sections.append(
            f'<section><h2>{title}</h2><p>{desc}</p>'
            f'{tile_row(v, c, "#232323")}{tile_row(v, c, "#f8f8f8")}{tile_row(v, c, "#808080", True)}'
            f'<div class="bars">{appbar(v, c, False)}{appbar(v, c, True)}</div></section>'
        )
    # 2. Couleurs candidates.
    rows = []
    for key, (name, c) in COLORS.items():
        light = darker(c)
        dist = sorted(((delta_e(c, s), n) for n, s in SEMANTIC.items()))
        near = ', '.join(f'{n} {d:.0f}' for d, n in dist[:2])
        tiles = ''.join(f'<div class="mini">{svg(v, c, 48)}</div>' for v in VARIANTS)
        rows.append(
            f'<tr><td><span class="sw" style="background:{c}"></span><span class="sw" style="background:{light}"></span></td>'
            f'<td><b>{name}</b><br><code>{c}</code> / <code>{light}</code></td>'
            f'<td>{contrast(c, "#181f2a"):.1f}:1</td><td>{contrast(light, "#e6e6e6"):.1f}:1</td>'
            f'<td>{near}</td><td class="minis">{tiles}</td></tr>'
        )
    sem = ''.join(f'<span class="chip"><span class="sw" style="background:{c}"></span>{n}</span>' for n, c in SEMANTIC.items())
    sections.append(
        '<section><h2>Couleurs candidates</h2>'
        '<p>Teintes laissées libres par la famille : ni bleu, vert, orange, rouge ou cyan (couleurs de sens), '
        'ni magenta (FabSpeed) ni chartreuse (FabVents). Contraste du symbole sur la tuile (≥ 3:1 demandé) et du liseré '
        'clair sur la barre claire ; écart perceptif (ΔE, plus c’est grand mieux c’est, < 25 : risque de confusion) '
        'avec les deux couleurs les plus proches.</p>'
        f'<div class="chips">{sem}</div>'
        '<table><tr><th></th><th>Couleur (sombre / clair)</th><th>Sur la tuile</th><th>Liseré clair</th><th>Plus proches (ΔE)</th><th>Variantes</th></tr>'
        + ''.join(rows) + '</table></section>'
    )
    html = f'''<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>FabWeave — planche de logos</title>
<style>
  body {{ margin: 0; padding: 24px; background: #1c1c1c; color: #e8e8e8; font: 14px/1.45 -apple-system, "Inter", system-ui, sans-serif; }}
  h1 {{ font-size: 20px; margin: 0 0 4px; }} h2 {{ font-size: 15px; margin: 0 0 2px; }}
  p {{ color: #ababab; margin: 0 0 10px; max-width: 760px; }}
  section {{ margin: 0 0 28px; padding: 16px; background: #232323; border: 1px solid #343434; border-radius: 6px; }}
  .row {{ display: flex; align-items: flex-end; gap: 22px; padding: 12px 14px; border-radius: 4px; margin-bottom: 6px; overflow-x: auto; }}
  .cell {{ display: flex; flex-direction: column; align-items: center; gap: 4px; }}
  .cell svg {{ display: block; }} .cell span {{ font-size: 11px; color: #7d7d7d; }}
  .bars {{ display: grid; gap: 6px; margin-top: 8px; }}
  .appbar {{ position: relative; display: flex; align-items: center; gap: 8px; height: 42px; padding: 0 12px; border-radius: 4px; overflow: hidden; }}
  .appbar i {{ position: absolute; inset: 0 0 auto 0; height: 2px; opacity: .85; }}
  .appbar b {{ font-size: 15px; letter-spacing: -0.01em; }} .appbar span {{ font-size: 12px; }}
  table {{ border-collapse: collapse; width: 100%; }} th, td {{ text-align: left; padding: 8px; border-top: 1px solid #343434; vertical-align: middle; }}
  th {{ color: #9a9a9a; font-weight: 600; font-size: 11px; text-transform: uppercase; letter-spacing: .06em; }}
  .sw {{ display: inline-block; width: 18px; height: 18px; border-radius: 4px; margin-right: 4px; vertical-align: middle; }}
  .chips {{ display: flex; flex-wrap: wrap; gap: 6px 14px; margin: 0 0 12px; font-size: 12px; color: #ababab; }}
  .minis {{ display: flex; gap: 8px; }} .mini svg {{ display: block; }}
  code {{ font-size: 12px; color: #ababab; }}
</style></head><body>
<h1>FabWeave — planche de logos</h1>
<p>Tuile graphite commune à la famille, symbole d’une seule couleur. Chaque variante à 128, 64, 48, 32, 24 et 16 px
(rendu vectoriel natif à chaque taille), sur panneau sombre, sur panneau clair, en monochrome, puis dans la barre
d’application (logo 24 px, liseré de 2 px). Généré par <code>branding/explorations/generate.py</code>.</p>
{''.join(sections)}
</body></html>
'''
    (HERE / 'planche.html').write_text(html)
    for key, (name, c) in COLORS.items():
        print(f"{name:18} {c} / {darker(c)}  tuile {contrast(c, '#181f2a'):.1f}:1  "
              + ', '.join(f'{n} {delta_e(c, s):.0f}' for n, s in sorted(SEMANTIC.items(), key=lambda kv: delta_e(c, kv[1]))[:3]))


if __name__ == '__main__':
    main()
